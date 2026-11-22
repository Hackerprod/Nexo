"""Typed, seeded F5 source construction independent of the Nexo parser.

The tree is interpreted by model.py before any subprocess is launched.
"""

from __future__ import annotations

from dataclasses import dataclass, replace
import hashlib
import json
import random
import re
import time
from typing import Iterator

from .model import Expr, Program, SourceSyntaxFault, SourceVM, Stmt, evaluate, execute, lex_source, parse_source, type_check
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
        if op in ("<", "<=", ">", ">=", "==", "!=") and \
                expr.args[0].kind == "binary" and expr.args[0].value in \
                ("<", "<=", ">", ">=", "==", "!="):
            left = "(" + left + ")"
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
    if kind == "print":
        assert stmt.expr is not None
        return "Print(" + render_expr(stmt.expr) + ");"
    if kind == "block":
        return "{" + "".join(render_stmt(child) for child in stmt.body) + "}"
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
    required_caps: tuple[str, ...] = ()

    def record(self) -> dict[str, object]:
        return {
            "seed": self.seed, "index": self.index, "lane": self.lane,
            "family": self.family, "source": self.source,
            "source_sha256": hashlib.sha256(self.source.encode("utf-8")).hexdigest(),
            "expected_value": self.expected_value,
            "expected_fault": self.expected_fault,
            "positive_source": self.positive_source,
            "expected_issue": self.expected_issue,
            "required_caps": self.required_caps,
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


def _make_extended_valid(seed: int, index: int, unique: int, family: int,
                         rng: random.Random) -> Case:
    """Additional F5 source families interpreted by the independent SourceVM."""
    u = unique
    variants = {
        10: ("array.push_copy", f"fn Main(){{var a=[[1]];var b=[{u}];"
             "Push(edit a,b);Push(edit b,3);Print(a);}", ("L07b",)),
        11: ("control.for_array_order", f"fn Main(){{for x in [{u},{u+1},{u+2}]"
             "{Print(x);}}", ("L06b",)),
        12: ("control.for_range_break", f"fn Main(){{for i in 0..<3{{"
             f"if i==1{{break;}}Print(i+{u});}}}}", ("L05b",)),
        13: ("control.for_range_continue", f"fn Main(){{for i in 0..<3{{"
             f"if i==1{{continue;}}Print(i+{u});}}}}", ("L05b",)),
        14: ("control.short_circuit_effect",
             f"fn Right()->Bool{{Print({u});return true;}}fn Main(){{"
             "AssertEq(false and Right(),false);AssertEq(true or Right(),true);"
             "AssertEq(true and Right(),true);}", ("L02b",)),
        15: ("function.user", f"fn Add(x:view I64)->I64{{return x+{u};}}"
             "fn Main(){Print(Add(3));}", ()),
        16: ("const.scalar", f"const k:I64=comptime {u};fn Main(){{Print(k);}}",
             ("L09", "L10")),
        17: ("array.pop_option", f"fn Main(){{var a=[{u}];"
             "Print(Pop(edit a));Print(Pop(edit a));}", ("L07b",)),
    }
    label, source, caps = variants[family]
    if family == 17 and (index // 18) % 3:
        # F5 §4.1 admits Array/Option payloads transitively inside Option.
        payload = f"[{u}]" if (index // 18) % 3 == 1 else f"Some([{u}])"
        source = f"fn Main(){{var a=[{payload}];Print(Pop(edit a));Print(Pop(edit a));}}"
    parsed = parse_source(source)
    SourceChecker(parsed).check()
    state = SourceVM(parsed).run()
    return Case(seed, index, "valid", label, Program(()), source,
                state.stdout, None, required_caps=caps)


def make_valid(seed: int, index: int) -> Case:
    rng = random.Random((seed << 32) | index)
    family = index % 18
    unique = seed * 100_000 + index + 1
    integer = lambda value: Expr("int", "I64", value=value)
    name = lambda label: Expr("name", "I64", value=label)
    if family >= 10:
        return _make_extended_valid(seed, index, unique, family, rng)
    if family in (0, 1, 9):
        ty = "I64" if family == 0 else "Bool"
        expr = scalar(rng, 1 + (index % 2 if ty == "I64" else 0), ty)
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
    # The extra expression consumes the family's computed answer. Its operand
    # varies per source and remains visible after identifier normalization.
    answer_ty = "Bool" if family in (1, 9) else "I64"
    answer = Expr("name", answer_ty, value="answer")
    sample = (Expr("binary", "I64", (answer, integer(unique)), "+")
              if answer_ty == "I64" else
              Expr("binary", "Bool", (answer, Expr("binary", "Bool",
                  (integer(unique), integer(unique + 1)), "<")), "=="))
    # Three independently selected identity operators make 64 source shapes
    # per family after integer literals and identifiers are abstracted.
    variant = index // 18
    for _ in range(3):
        choice = variant % 4
        variant //= 4
        if answer_ty == "I64":
            op, identity = (("+", 0), ("-", 0), ("*", 1), ("/", 1))[choice]
            sample = Expr("binary", "I64", (sample, integer(identity)), op)
        else:
            op, identity = (("and", True), ("or", False),
                            ("==", True), ("!=", False))[choice]
            sample = Expr("binary", "Bool", (sample, Expr("bool", "Bool", value=identity)), op)
    program = Program(program.body + (
        Stmt("let", "sample", sample),
        Stmt("print", expr=answer),
        Stmt("print", expr=Expr("name", sample.ty, value="sample")),
    ))
    value = execute(program).env["answer"]
    return Case(seed, index, "valid", label,
                program, render_program(program), value, None)


def make_invalid(seed: int, index: int) -> Case:
    rng = random.Random((seed << 32) | index)
    family = index % 14
    if family >= 10:
        return _special_invalid(seed, index, family)
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
        return _contextual_invalid(seed, index, name, code, program, repair_program)
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
    return _contextual_invalid(seed, index, name, code, program, repair_program)


def _contextual_invalid(seed: int, index: int, name: str, code: str,
                        program: Program, repair_program: Program) -> Case:
    # The context uses only inherited arithmetic. A pending types barrier in
    # the context would combine with the deliberately isolated negative.
    prefix = (Stmt("block", body=(Stmt("let", "context", _legacy_context(seed, index)),)),)
    program = Program(prefix + program.body)
    repair_program = Program(prefix + repair_program.body)
    source = render_program(program)
    repaired_source = render_program(repair_program)
    if code.startswith("R"):
        SourceChecker(parse_source(source)).check()
        SourceChecker(parse_source(repaired_source)).check()
        return Case(seed, index, "invalid", name, program, source, None, code,
                    repaired_source)
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
                repaired_source, expected)


def _legacy_context(seed: int, index: int) -> Expr:
    integer = lambda value: Expr("int", "I64", value=value)
    result = integer(seed * 100_000 + index + 1)
    variant = index // 14
    for _ in range(3):
        choice = variant % 4
        variant //= 4
        unit = integer(0) if choice < 2 else Expr("binary", "I64", (integer(1), integer(1)), "-")
        result = Expr("binary", "I64", (result, unit), "+" if choice % 2 == 0 else "-")
    return result


def _special_invalid(seed: int, index: int, family: int) -> Case:
    context = "let context=" + render_expr(_legacy_context(seed, index)) + ";"
    if family == 13 and index >= 27:
        # F5 §2.2: a reachable break restores a normal exit, even for true.
        variant = (index // 14) % 4
        condition = ("false", "true", "b", "((true))")[variant]
        prefix = "let b=false;" if condition == "b" else ""
        body = "break;" if variant in (1, 3) else ""
        loop = f"{prefix}while {condition}{{{body}}}"
        source = f"fn F()->I64{{{loop}}}fn Main(){{{context}}}"
        repaired = f"fn F()->I64{{{loop}return 1;}}fn Main(){{{context}}}"
        try:
            SourceChecker(parse_source(source)).check()
        except RuleViolation as failure:
            issue = failure.issue
            assert issue.code == "NX-E1317", (condition, issue.code)
            expected = {"code": issue.code, "subcause": issue.subcause,
                        "stage": issue.stage, "operation": issue.operation,
                        "primary": [issue.start, issue.end], "data": issue.data,
                        "feature_id": issue.feature_id,
                        "related": [list(item) for item in issue.related]}
        else:
            raise AssertionError((condition, "missing return accepted"))
        SourceChecker(parse_source(repaired)).check()
        return Case(seed, index, "invalid", "control.while_return",
                    Program(()), source, None, "NX-E1317", repaired, expected)
    if family == 10:
        if index >= 100 and (index // 14) % 2:
            # F5-N02 fixes names/NX-E0104 before the pending L09 barrier.
            prefix = "const k:I64=comptime 1;const k:I64=comptime 2;"
            source = prefix + "fn Main(){" + context + "}"
            repaired = prefix.replace("const k:I64=comptime 2;",
                                      "const j:I64=comptime 2;") + "fn Main(){" + context + "}"
            expected = {"code": "NX-E0104", "subcause": None,
                        "stage": "names", "operation": "names.declare",
                        "primary": [29, 30],
                        "data": {"declaration_kind": "constant"},
                        "feature_id": None,
                        "related": [["previous_declaration", 6, 7]]}
            SourceChecker(parse_source(repaired)).check()
            return Case(seed, index, "invalid", "const.duplicate_name",
                        Program(()), source, None, "NX-E0104", repaired, expected)
        label, code = "const.cycle", "NX-E1323"
        prefix = "const a:I64=comptime b;const b:I64=comptime a;"
        repair_prefix = "const a:I64=comptime b;const b:I64=comptime 1;"
        source = prefix + "fn Main(){" + context + "}"
        repaired = repair_prefix + "fn Main(){" + context + "}"
    elif family == 11:
        label, code = "const.impure", "NX-E1316"
        prefix = "const k:()=comptime Print(1);"
        repair_prefix = "const k:()=comptime ();"
        source = prefix + "fn Main(){" + context + "}"
        repaired = repair_prefix + "fn Main(){" + context + "}"
    elif family == 12:
        label, code = "borrow.live", "NX-E1105"
        source = "fn Main(){" + context + "var a=[1];Push(edit a,Len(a));}"
        repaired = "fn Main(){" + context + "var a=[1];let n=Len(a);Push(edit a,n);}"
    else:
        label, code = "syntax.first_truncation", "NX-E0002"
        source = "fn Main(){" + context + "let a="
        repaired = source + "0;}"
        try:
            parse_source(source)
        except SourceSyntaxFault as fault:
            token = fault.token
            expected = {"code": code, "subcause": None, "stage": "parse",
                        "operation": "syntax.parse", "primary": [token.start, token.end],
                        "data": {}, "feature_id": None,
                        "unexpected_token": {"kind": "eof", "text": "",
                                             "span": [token.start, token.end]},
                        "expected_tokens": fault.expected,
                        "expected_tokens_kind": "admissible_set",
                        "parser_context": fault.context,
                        "recognized_prefix": [fault.prefix_start, fault.prefix_end]}
        else:
            raise AssertionError((label, "truncated source accepted"))
        SourceChecker(parse_source(repaired)).check()
        return Case(seed, index, "invalid", label, Program(()), source, None,
                    code, repaired, expected)
    try:
        SourceChecker(parse_source(source)).check()
    except RuleViolation as failure:
        issue = failure.issue
        assert issue.code == code, (label, issue.code)
        expected = {"code": issue.code, "subcause": issue.subcause,
                    "stage": issue.stage, "operation": issue.operation,
                    "primary": [issue.start, issue.end], "data": issue.data,
                    "feature_id": issue.feature_id,
                    "related": [list(item) for item in issue.related]}
    else:
        raise AssertionError((label, "negative accepted by reference checker"))
    SourceChecker(parse_source(repaired)).check()
    return Case(seed, index, "invalid", label, Program(()), source, None, code,
                repaired, expected)


def cases(seed: int, valid_count: int, invalid_count: int) -> Iterator[Case]:
    for lane, count, factory in (("valid", valid_count, make_valid),
                                 ("invalid", invalid_count, make_invalid)):
        seen: set[tuple[str, ...]] = set()
        for index in range(count):
            candidate_index = index
            while True:
                case = factory(seed, candidate_index)
                normal = normalized_source(case.source)
                if normal not in seen:
                    seen.add(normal)
                    yield replace(case, index=index)
                    break
                candidate_index += count
                if candidate_index > index + count * 8:
                    raise AssertionError((seed, lane, index, "distinct corpus exhausted"))


_NAMES_KEPT = {"fn", "Main", "let", "var", "const", "comptime", "I64", "Bool",
               "Array", "Option", "view", "edit", "move", "if", "else", "while",
               "for", "in", "break", "continue", "return", "true", "false",
               "and", "or", "not", "Print", "AssertEq", "Len", "JoinArrays",
               "Push", "Pop", "Some", "None", "with", "no_alloc"}


def normalized_source(source: str) -> tuple[str, ...]:
    """Ignore spacing and alpha-renaming while retaining operators and literals."""
    names: dict[str, str] = {}
    result = []
    for token in lex_source(source)[:-1]:
        word = token.text
        if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", word) and word not in _NAMES_KEPT:
            names.setdefault(word, f"id{len(names)}")
            word = names[word]
        result.append(word)
    return tuple(result)


def structural_source(source: str) -> tuple[str, ...]:
    """Count syntax shapes without obtaining credit for changed integers."""
    return tuple("IntLiteral" if word.isdecimal() else word
                 for word in normalized_source(source))


def distinct_counts(seed: int, valid_count: int, invalid_count: int) -> dict[str, int]:
    forms = {"valid": set(), "invalid": set()}
    for case in cases(seed, valid_count, invalid_count):
        forms[case.lane].add(normalized_source(case.source))
    return {lane: len(values) for lane, values in forms.items()}


def corpus_sha(seed: int, valid_count: int, invalid_count: int) -> str:
    return audit_seed(seed, valid_count, invalid_count)["sha256"]


def audit_seed(seed: int, valid_count: int, invalid_count: int) -> dict[str, object]:
    digest = hashlib.sha256()
    forms: dict[str, set[tuple[str, ...]]] = {"valid": set(), "invalid": set()}
    structures: dict[str, set[tuple[str, ...]]] = {"valid": set(), "invalid": set()}
    max_valid_nodes = 0
    for case in cases(seed, valid_count, invalid_count):
        forms[case.lane].add(normalized_source(case.source))
        structures[case.lane].add(structural_source(case.source))
        if case.lane == "valid":
            parsed = parse_source(case.source)
            def size(node) -> int:
                return 1 + sum(size(child) for child in node.children)
            nodes = sum(size(stmt) for fn in parsed.functions.values()
                        for stmt in fn.body)
            nodes += sum(size(expr) for _, _, expr in parsed.constants)
            max_valid_nodes = max(max_valid_nodes, nodes)
        digest.update(json.dumps(case.record(), ensure_ascii=False,
                                 sort_keys=True, separators=(",", ":")).encode("utf-8"))
        digest.update(b"\n")
    return {"sha256": digest.hexdigest(),
            "valid_distinct": len(forms["valid"]),
            "invalid_distinct": len(forms["invalid"]),
            "valid_structural": len(structures["valid"]),
            "invalid_structural": len(structures["invalid"]),
            "max_valid_nodes": max_valid_nodes}


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
