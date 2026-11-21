"""Typed, seeded F5 source construction independent of the Nexo parser.

The tree is interpreted by model.py before any subprocess is launched.
"""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
import random
from typing import Iterator

from .model import Expr, Program, Stmt, evaluate, execute, type_check

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

    def record(self) -> dict[str, object]:
        return {
            "seed": self.seed, "index": self.index, "lane": self.lane,
            "family": self.family, "source": self.source,
            "source_sha256": hashlib.sha256(self.source.encode("utf-8")).hexdigest(),
            "expected_value": self.expected_value,
            "expected_fault": self.expected_fault,
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
    ty = "I64" if index % 2 == 0 else "Bool"
    expr = scalar(rng, 1 + index % 3, ty)
    program = Program((Stmt("let", "answer", expr),))
    type_check(expr, {})
    value = evaluate(expr, {}, [])
    return Case(seed, index, "valid", "scalar." + ty,
                program, render_program(program), value, None)


def make_invalid(seed: int, index: int) -> Case:
    rng = random.Random((seed << 32) | index)
    family = index % 4
    if family == 0:
        expr = Expr("binary", "I64", (Expr("int", "I64", value=rng.randint(1, 100)),
                                       Expr("int", "I64", value=0)), "/")
        code, name = "R0502", "div.zero"
    elif family == 1:
        expr = Expr("binary", "I64", (Expr("int", "I64", value=(1 << 63) - 1),
                                       Expr("int", "I64", value=2)), "*")
        code, name = "R0501", "mul.overflow"
    elif family == 2:
        expr = Expr("binary", "I64", (Expr("int", "I64", value=rng.randint(1, 100)),
                                       Expr("int", "I64", value=0)), "%")
        code, name = "R0504", "rem.zero"
    else:
        expr = Expr("neg", "I64", (Expr("int", "I64", value=-(1 << 63)),))
        code, name = "R0505", "neg.overflow"
    program = Program((Stmt("let", "answer", expr),))
    return Case(seed, index, "invalid", name, program, render_program(program), None, code)


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
