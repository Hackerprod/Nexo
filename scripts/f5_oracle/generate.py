"""Typed, seeded F5 source construction independent of the Nexo parser.

The tree is interpreted by model.py before any subprocess is launched.
"""

from __future__ import annotations

from dataclasses import dataclass, replace
import hashlib
import json
import random
import time
from typing import Iterator

from .model import Expr, Program, Stmt, evaluate, execute, parse_source, type_check
from .diagnostics import RuleViolation, SourceChecker

PRECEDENCE = {
    "or": 1, "and": 2,
    "<": 3, "<=": 3, ">": 3, ">=": 3, "==": 3, "!=": 3,
    "..<": 4, "..=": 4,
    "+": 5, "-": 5,
    "*": 6, "/": 6, "%": 6,
}


def render_expr(expr: Expr, parent: int = 0, right: bool = False) -> str:
    kind = expr.kind
    if kind == "int":
        return str(expr.value)
    if kind == "bool":
        return "true" if expr.value else "false"
    if kind == "unit":
        return "()"
    if kind == "name":
        return str(expr.value)
    if kind == "array":
        return "[" + ",".join(render_expr(arg) for arg in expr.args) + "]"
    if kind == "index":
        return render_expr(expr.args[0], 9) + "[" + render_expr(expr.args[1]) + "]"
    if kind == "len":
        return "Len(" + render_expr(expr.args[0]) + ")"
    if kind == "join":
        return "JoinArrays(" + ",".join(render_expr(arg) for arg in expr.args) + ")"
    if kind in ("neg", "not"):
        op = "-" if kind == "neg" else "not "
        result = op + render_expr(expr.args[0], 7)
        return "(" + result + ")" if parent > 7 else result
    if kind == "binary":
        op = str(expr.value)
        prec = PRECEDENCE[op]
        left = render_expr(expr.args[0], prec)
        # Arithmetic/logical operators associate left; preserve a right child
        # of the same precedence by grouping it.
        right_text = render_expr(expr.args[1], prec, True)
        result = left + (" " + op + " " if op in ("and", "or") else op) + right_text
        if parent > prec or (right and parent == prec):
            return "(" + result + ")"
        return result
    raise ValueError(kind)


def render_stmt(stmt: Stmt) -> str:
    kind = stmt.kind
    if kind in ("let", "var"):
        assert stmt.expr is not None
        return f"{kind} {stmt.name}={render_expr(stmt.expr)};"
    if kind in ("assign", "compound"):
        assert stmt.expr is not None
        op = "=" if kind == "assign" else stmt.op + "="
        return f"{stmt.name}{op}{render_expr(stmt.expr)};"
    if kind == "for_range":
        assert stmt.expr is not None and stmt.upper is not None
        return f"for {stmt.name} in {render_expr(stmt.expr)}{stmt.op}{render_expr(stmt.upper)}{{" + \
            "".join(render_stmt(child) for child in stmt.body) + "}"
    if kind == "for_array":
        assert stmt.expr is not None
        return f"for {stmt.name} in {render_expr(stmt.expr)}{{" + \
            "".join(render_stmt(child) for child in stmt.body) + "}"
    if kind == "if":
        assert stmt.expr is not None
        result = "if " + render_expr(stmt.expr) + "{" + \
            "".join(render_stmt(child) for child in stmt.body) + "}"
        if stmt.otherwise:
            result += "else{" + "".join(render_stmt(child) for child in stmt.otherwise) + "}"
        return result
    if kind == "while":
        assert stmt.expr is not None
        return "while " + render_expr(stmt.expr) + "{" + \
            "".join(render_stmt(child) for child in stmt.body) + "}"
    if kind in ("break", "continue", "return"):
        return kind + ";"
    if kind == "expr":
        assert stmt.expr is not None
        return render_expr(stmt.expr) + ";"
    raise ValueError(kind)


def render_program(program: Program) -> str:
    return "fn Main(){" + "".join(render_stmt(stmt) for stmt in program.body) + "}"


@dataclass(frozen=True)
class Case:
    seed: int
    index: int
    lane: str
    family: str
    program: Program
    source: str
    expected_value: object | None
    expected_fault: str | None
    positive_source: str | None = None
    expected_issue: dict[str, object] | None = None

    def record(self) -> dict[str, object]:
        return {
            "seed": self.seed, "index": self.index, "lane": self.lane,
            "family": self.family, "source": self.source,
            "source_sha256": hashlib.sha256(self.source.encode("utf-8")).hexdigest(),
            "expected_value": self.expected_value,
            "expected_fault": self.expected_fault,
            "positive_source": self.positive_source,
            "expected_issue": self.expected_issue,
        }


def scalar(rng: random.Random, depth: int, ty: str) -> Expr:
    """Generate only well-typed bounded trees; all values remain small."""
    if depth == 0:
        return Expr("int", "I64", value=rng.randint(-50, 50)) if ty == "I64" else \
            Expr("bool", "Bool", value=bool(rng.getrandbits(1)))
    if ty == "I64":
        op = rng.choice(("+", "-", "*", "%"))
        if op == "%":
            left = scalar(rng, depth - 1, "I64")
            right = Expr("int", "I64", value=rng.choice((-7, -3, -1, 1, 3, 7)))
        else:
            left, right = scalar(rng, depth - 1, "I64"), scalar(rng, depth - 1, "I64")
        return Expr("binary", "I64", (left, right), op)
    op = rng.choice(("and", "or", "<", "=="))
    if op in ("and", "or"):
        args = (scalar(rng, depth - 1, "Bool"), scalar(rng, depth - 1, "Bool"))
    else:
        args = (scalar(rng, depth - 1, "I64"), scalar(rng, depth - 1, "I64"))
    return Expr("binary", "Bool", args, op)


def make_valid(seed: int, index: int) -> Case:
    rng = random.Random((seed << 32) | index)
    family = index % 10
    integer = lambda value: Expr("int", "I64", value=value)
    name = lambda label: Expr("name", "I64", value=label)
    if family in (0, 1, 9):
        ty = "I64" if family == 0 else "Bool"
        expr = scalar(rng, 1 + index % 3, ty)
        program = Program((Stmt("let", "answer", expr),))
        label = "scalar." + ty
    elif family == 2:
        array = Expr("array", "Array<I64>", tuple(integer(rng.randint(-9, 9))
                                                      for _ in range(rng.randrange(1, 5))))
        program = Program((Stmt("let", "answer", Expr("len", "I64", (array,))),))
        label = "array.len"
    elif family == 3:
        left = Expr("array", "Array<I64>", (integer(rng.randint(0, 9)),))
        right = Expr("array", "Array<I64>", (integer(rng.randint(0, 9)),
                                                 integer(rng.randint(0, 9))))
        program = Program((Stmt("let", "answer", Expr("len", "I64", (
            Expr("join", "Array<I64>", (left, right)),))),))
        label = "array.join_len"
    elif family == 4:
        hi = rng.randrange(5)
        program = Program((Stmt("var", "answer", integer(0)),
                           Stmt("for_range", "i", integer(0),
                                body=(Stmt("compound", "answer", name("i"), op="+"),),
                                op="..<", upper=integer(hi))))
        label = "control.for_range"
    elif family == 5:
        items = tuple(integer(rng.randrange(5)) for _ in range(rng.randrange(1, 4)))
        program = Program((Stmt("var", "answer", integer(0)),
                           Stmt("for_array", "i", Expr("array", "Array<I64>", items),
                                body=(Stmt("compound", "answer", name("i"), op="+"),))))
        label = "control.for_array"
    elif family == 6:
        condition = Expr("bool", "Bool", value=bool(rng.getrandbits(1)))
        program = Program((Stmt("var", "answer", integer(0)),
                           Stmt("if", expr=condition,
                                body=(Stmt("compound", "answer", integer(1), op="+"),),
                                otherwise=(Stmt("compound", "answer", integer(2), op="+"),))))
        label = "control.if"
    elif family == 7:
        amount = rng.randrange(10)
        program = Program((Stmt("var", "answer", integer(0)),
                           Stmt("compound", "answer", integer(amount), op="+")))
        label = "place.compound"
    else:
        hi = rng.randrange(4)
        condition = Expr("binary", "Bool", (name("answer"), integer(hi)), "<")
        program = Program((Stmt("var", "answer", integer(0)),
                           Stmt("while", expr=condition,
                                body=(Stmt("compound", "answer", integer(1), op="+"),))))
        label = "control.while"
    value = execute(program).env["answer"]
    return Case(seed, index, "valid", label,
                program, render_program(program), value, None)


def make_invalid(seed: int, index: int) -> Case:
    rng = random.Random((seed << 32) | index)
    family = index % 10
    if family == 0:
        expr = Expr("binary", "I64", (Expr("int", "I64", value=rng.randint(1, 100)),
                                       Expr("int", "I64", value=0)), "/")
        code, name = "R0502", "div.zero"
        repaired = Expr("binary", "I64", (expr.args[0], Expr("int", "I64", value=1)), "/")
    elif family == 1:
        expr = Expr("binary", "I64", (Expr("int", "I64", value=(1 << 63) - 1),
                                       Expr("int", "I64", value=2)), "*")
        code, name = "R0501", "mul.overflow"
        repaired = Expr("binary", "I64", (expr.args[0], Expr("int", "I64", value=1)), "*")
    elif family == 2:
        expr = Expr("binary", "I64", (Expr("int", "I64", value=rng.randint(1, 100)),
                                       Expr("int", "I64", value=0)), "%")
        code, name = "R0504", "rem.zero"
        repaired = Expr("binary", "I64", (expr.args[0], Expr("int", "I64", value=1)), "%")
    elif family == 3:
        expr = Expr("neg", "I64", (Expr("int", "I64", value=-(1 << 63)),))
        code, name = "R0505", "neg.overflow"
        repaired = Expr("int", "I64", value=-(1 << 63))
    if family < 4:
        program = Program((Stmt("let", "answer", expr),))
        repair_program = Program((Stmt("let", "answer", repaired),))
        return Case(seed, index, "invalid", name, program, render_program(program), None,
                    code, render_program(repair_program))
    one = Expr("int", "I64", value=1)
    zero = Expr("int", "I64", value=0)
    if family == 4:
        program = Program((Stmt("let", "answer", Expr("binary", "I64", (
            one, Expr("bool", "Bool", value=True)), "+")),))
        repair_program = Program((Stmt("let", "answer", Expr("binary", "I64",
                                                     (one, one), "+")),))
        name, code = "types.operator", "NX-E0102"
    elif family in (5, 6):
        action = "break" if family == 5 else "continue"
        program = Program((Stmt(action),))
        repair_program = Program((Stmt("for_range", "i", zero, body=(Stmt(action),),
                                       op="..<", upper=one),))
        name, code = "control." + action, "NX-E1326"
    elif family == 7:
        expr = Expr("binary", "Bool", (zero, one), "..<")
        # A range is a legal iteration source, but not a storable value.
        program = Program((Stmt("let", "answer", expr),))
        repair_program = Program((Stmt("for_range", "i", zero, op="..<", upper=one),))
        name, code = "range.stored", "NX-E0006"
    elif family == 8:
        program = Program((Stmt("let", "answer", zero),
                           Stmt("assign", "answer", one)))
        repair_program = Program((Stmt("var", "answer", zero),
                                  Stmt("assign", "answer", one)))
        name, code = "place.immutable", "NX-E0105"
    else:
        program = Program((Stmt("let", "answer", Expr("int", "I64", value=1 << 63)),))
        repair_program = Program((Stmt("let", "answer", Expr("int", "I64",
                                                      value=(1 << 63) - 1)),))
        name, code = "literal.out_of_range", "NX-E0107"
    source = render_program(program)
    try:
        SourceChecker(parse_source(source)).check()
    except RuleViolation as failure:
        issue = failure.issue
        assert issue.code == code, (name, issue.code)
        expected = {"code": issue.code, "subcause": issue.subcause,
                    "stage": issue.stage, "operation": issue.operation,
                    "primary": [issue.start, issue.end], "data": issue.data,
                    "feature_id": issue.feature_id,
                    "related": [list(item) for item in issue.related]}
    else:
        raise AssertionError((name, "negative accepted by reference checker"))
    return Case(seed, index, "invalid", name, program, source, None, code,
                render_program(repair_program), expected)


def cases(seed: int, valid_count: int, invalid_count: int) -> Iterator[Case]:
    for index in range(valid_count):
        yield make_valid(seed, index)
    for index in range(invalid_count):
        yield make_invalid(seed, index)


def corpus_sha(seed: int, valid_count: int, invalid_count: int) -> str:
    digest = hashlib.sha256()
    for case in cases(seed, valid_count, invalid_count):
        digest.update(json.dumps(case.record(), ensure_ascii=False,
                                 sort_keys=True, separators=(",", ":")).encode("utf-8"))
        digest.update(b"\n")
    return digest.hexdigest()


def _rederive(case: Case, program: Program) -> Case | None:
    """Recompute an expected result from the own tree after a reduction."""
    try:
        state = execute(program)
    except Exception as fault:
        if case.expected_fault is None or getattr(fault, "code", None) != case.expected_fault:
            return None
        value = None
    else:
        if case.expected_fault is not None or "answer" not in state.env:
            return None
        value = state.env["answer"]
    return replace(case, program=program, source=render_program(program),
                   expected_value=value, positive_source=None)


def minimize_case(case: Case, discrepancy, *, seconds: float = 60.0) -> tuple[Case, int]:
    """Keep only reductions with the same externally reported discrepancy.

    The caller's signature includes the first different JSON path, diagnostic
    code/subcause/stage and control path. Expected values are always rederived
    before the callback invokes a binary.
    """
    deadline = time.monotonic() + seconds
    original_signature = discrepancy(case)
    best = case
    attempts = 0
    changed = True
    while changed and time.monotonic() < deadline:
        changed = False
        candidates: list[Program] = []
        if len(best.program.body) > 1:
            for index in range(len(best.program.body)):
                candidates.append(Program(best.program.body[:index] +
                                          best.program.body[index + 1:]))
        for index, stmt in enumerate(best.program.body):
            if stmt.expr is None:
                continue
            replacements = (Expr("int", "I64", value=0),
                            Expr("int", "I64", value=1),
                            Expr("bool", "Bool", value=False))
            for expression in replacements:
                candidates.append(Program(best.program.body[:index] +
                                          (replace(stmt, expr=expression),) +
                                          best.program.body[index + 1:]))
        for program in candidates:
            if time.monotonic() >= deadline:
                break
            candidate = _rederive(best, program)
            if candidate is None or len(candidate.source) >= len(best.source):
                continue
            attempts += 1
            if discrepancy(candidate) == original_signature:
                best = candidate
                changed = True
                break
    return best, attempts
