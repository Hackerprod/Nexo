"""Typed F5 expressions evaluated with mathematical integers.

The rules in this module are transcribed from F5 §§2.1, 2.2, 3.1 and 4.1.
The source serializer and evaluator both consume this AST.  Neither consumes
Nexo's parser, checker, IR, or diagnostic output.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

MIN_I64 = -(1 << 63)
MAX_I64 = (1 << 63) - 1


class ModelFault(Exception):
    def __init__(self, code: str, operation: str, detail: str = "") -> None:
        super().__init__(detail or code)
        self.code = code
        self.operation = operation


@dataclass(frozen=True)
class Expr:
    kind: str
    ty: str
    args: tuple[Expr, ...] = ()
    value: Any = None


@dataclass(frozen=True)
class Stmt:
    kind: str
    name: str = ""
    expr: Expr | None = None
    body: tuple[Stmt, ...] = ()
    otherwise: tuple[Stmt, ...] = ()
    mutable: bool = False
    op: str = ""
    upper: Expr | None = None


@dataclass(frozen=True)
class Program:
    body: tuple[Stmt, ...]


@dataclass
class Array:
    items: list[Any]
    capacity: int

    def copy(self) -> Array:
        return Array([copy_value(item) for item in self.items], len(self.items))


def copy_value(value: Any) -> Any:
    if isinstance(value, Array):
        return value.copy()
    if isinstance(value, tuple):
        return tuple(copy_value(item) for item in value)
    return value


def i64(value: int, operation: str) -> int:
    if type(value) is not int or value < MIN_I64 or value > MAX_I64:
        raise ModelFault("R0101" if operation == "value.add" else
                         "R0103" if operation == "value.sub" else
                         "R0501" if operation == "value.mul" else
                         "R0505", operation)
    return value


def quotient(a: int, b: int) -> int:
    """Truncate toward zero without Python's negative floor division."""
    if b == 0:
        raise ModelFault("R0502", "value.div")
    if a == MIN_I64 and b == -1:
        raise ModelFault("R0503", "value.div")
    magnitude = abs(a) // abs(b)
    return -magnitude if (a < 0) != (b < 0) else magnitude


def remainder(a: int, b: int) -> int:
    if b == 0:
        raise ModelFault("R0504", "value.rem")
    # MIN % -1 is zero; the mathematical quotient need not fit I64.
    magnitude = abs(a) // abs(b)
    q = -magnitude if (a < 0) != (b < 0) else magnitude
    return a - q * b


def binary(op: str, a: Any, b: Any) -> Any:
    if op == "+":
        return i64(a + b, "value.add")
    if op == "-":
        return i64(a - b, "value.sub")
    if op == "*":
        return i64(a * b, "value.mul")
    if op == "/":
        return quotient(a, b)
    if op == "%":
        return remainder(a, b)
    if op == "<":
        return a < b
    if op == "<=":
        return a <= b
    if op == ">":
        return a > b
    if op == ">=":
        return a >= b
    if op == "==":
        return equal(a, b)
    if op == "!=":
        return not equal(a, b)
    raise ValueError(f"unknown operator: {op}")


def equal(a: Any, b: Any) -> bool:
    if isinstance(a, Array) and isinstance(b, Array):
        return len(a.items) == len(b.items) and all(
            equal(x, y) for x, y in zip(a.items, b.items)
        )
    if type(a) is not type(b):
        return False
    return a == b


def type_check(expr: Expr, types: dict[str, str]) -> str:
    """Validate the generated typed tree, including skipped RHS branches."""
    kind = expr.kind
    if kind in ("int", "bool", "unit"):
        expected = {"int": "I64", "bool": "Bool", "unit": "()"}[kind]
        if expr.ty != expected:
            raise TypeError((kind, expr.ty))
        if kind == "int" and not MIN_I64 <= expr.value <= MAX_I64:
            raise ModelFault("NX-E0107", "value.literal")
        return expected
    if kind == "name":
        actual = types[expr.value]
        if actual != expr.ty:
            raise TypeError((actual, expr.ty))
        return actual
    if kind == "array":
        element = expr.ty.removeprefix("Array<").removesuffix(">")
        if not expr.ty.startswith("Array<") or any(
            type_check(item, types) != element for item in expr.args
        ):
            raise TypeError("array element")
        return expr.ty
    if kind in ("neg", "not"):
        need = "I64" if kind == "neg" else "Bool"
        if type_check(expr.args[0], types) != need or expr.ty != need:
            raise TypeError((kind, need))
        return need
    if kind == "binary":
        op = expr.value
        left, right = (type_check(arg, types) for arg in expr.args)
        if op in ("and", "or"):
            need, result = "Bool", "Bool"
        elif op in ("+", "-", "*", "/", "%"):
            need, result = "I64", "I64"
        elif op in ("<", "<=", ">", ">="):
            need, result = "I64", "Bool"
        else:
            need, result = left, "Bool"
        if left != need or right != need or expr.ty != result:
            raise TypeError((op, left, right, need, result))
        return result
    if kind == "index":
        array_ty = type_check(expr.args[0], types)
        if not array_ty.startswith("Array<") or type_check(expr.args[1], types) != "I64":
            raise TypeError("index")
        element = array_ty[6:-1]
        if expr.ty != element:
            raise TypeError((expr.ty, element))
        return element
    if kind == "len":
        if not type_check(expr.args[0], types).startswith("Array<") or expr.ty != "I64":
            raise TypeError("Len")
        return "I64"
    if kind == "join":
        left, right = (type_check(arg, types) for arg in expr.args)
        if left != right or left != expr.ty or not left.startswith("Array<"):
            raise TypeError("JoinArrays")
        return left
    raise ValueError(f"unknown expression: {kind}")


def evaluate(expr: Expr, env: dict[str, Any], journal: list[str]) -> Any:
    kind = expr.kind
    if kind in ("int", "bool"):
        return expr.value
    if kind == "unit":
        return None
    if kind == "name":
        journal.append("read:" + expr.value)
        return copy_value(env[expr.value])
    if kind == "array":
        return Array([evaluate(arg, env, journal) for arg in expr.args], len(expr.args))
    if kind == "neg":
        return i64(-evaluate(expr.args[0], env, journal), "value.neg")
    if kind == "not":
        return not evaluate(expr.args[0], env, journal)
    if kind == "binary":
        left = evaluate(expr.args[0], env, journal)
        if expr.value == "and" and not left:
            return False
        if expr.value == "or" and left:
            return True
        right = evaluate(expr.args[1], env, journal)
        if expr.value == "and":
            return left and right
        if expr.value == "or":
            return left or right
        return binary(expr.value, left, right)
    if kind == "index":
        array = evaluate(expr.args[0], env, journal)
        index = evaluate(expr.args[1], env, journal)
        if index < 0 or index >= len(array.items):
            raise ModelFault("R0102", "array.index")
        return copy_value(array.items[index])
    if kind == "len":
        return len(evaluate(expr.args[0], env, journal).items)
    if kind == "join":
        left = evaluate(expr.args[0], env, journal)
        right = evaluate(expr.args[1], env, journal)
        items = [copy_value(x) for x in left.items + right.items]
        return Array(items, len(items))
    raise ValueError(kind)


@dataclass
class State:
    env: dict[str, Any] = field(default_factory=dict)
    types: dict[str, str] = field(default_factory=dict)
    mutable: set[str] = field(default_factory=set)
    journal: list[str] = field(default_factory=list)
    steps: int = 0
    max_steps: int = 16_000_000

    def tick(self, amount: int = 1) -> None:
        self.steps += amount
        if self.steps > self.max_steps:
            raise ModelFault("R0001", "runtime.limit")


class _Break(Exception):
    pass


class _Continue(Exception):
    pass


class _Return(Exception):
    pass


def execute_statement(stmt: Stmt, state: State) -> None:
    if stmt.kind in ("let", "var"):
        assert stmt.expr is not None
        ty = type_check(stmt.expr, state.types)
        value = evaluate(stmt.expr, state.env, state.journal)
        state.env[stmt.name] = value
        state.types[stmt.name] = ty
        if stmt.kind == "var":
            state.mutable.add(stmt.name)
        state.journal.append("bind:" + stmt.name)
        return
    if stmt.kind in ("assign", "compound"):
        if stmt.name not in state.mutable:
            raise ModelFault("NX-E0105", "place.write")
        assert stmt.expr is not None
        if type_check(stmt.expr, state.types) != state.types[stmt.name]:
            raise ModelFault("NX-E0102", "types.check")
        old = state.env[stmt.name]
        rhs = evaluate(stmt.expr, state.env, state.journal)
        state.env[stmt.name] = binary(stmt.op, old, rhs) if stmt.kind == "compound" else rhs
        state.journal.append("write:" + stmt.name)
        return
    if stmt.kind == "for_range":
        assert stmt.expr is not None and stmt.upper is not None
        if type_check(stmt.expr, state.types) != "I64" or type_check(stmt.upper, state.types) != "I64":
            raise ModelFault("NX-E0102", "types.check")
        lo = evaluate(stmt.expr, state.env, state.journal)
        hi = evaluate(stmt.upper, state.env, state.journal)
        current = lo
        while current < hi or (stmt.op == "..=" and current == hi):
            state.tick()
            state.env[stmt.name] = current
            state.types[stmt.name] = "I64"
            state.journal.append("iter:" + str(current))
            try:
                execute_block(stmt.body, state)
            except _Continue:
                pass
            except _Break:
                break
            if stmt.op == "..=" and current == hi:
                break
            current += 1
        state.env.pop(stmt.name, None)
        state.types.pop(stmt.name, None)
        return
    if stmt.kind == "for_array":
        assert stmt.expr is not None
        ty = type_check(stmt.expr, state.types)
        if not ty.startswith("Array<"):
            raise ModelFault("NX-E0102", "types.check")
        array = evaluate(stmt.expr, state.env, state.journal)
        for i in range(len(array.items)):
            state.tick()
            state.env[stmt.name] = array.items[i]
            state.types[stmt.name] = ty[6:-1]
            try:
                execute_block(stmt.body, state)
            except _Continue:
                pass
            except _Break:
                break
        state.env.pop(stmt.name, None)
        state.types.pop(stmt.name, None)
        return
    if stmt.kind == "if":
        assert stmt.expr is not None
        if type_check(stmt.expr, state.types) != "Bool":
            raise ModelFault("NX-E0102", "types.check")
        execute_block(stmt.body if evaluate(stmt.expr, state.env, state.journal) else stmt.otherwise, state)
        return
    if stmt.kind == "while":
        assert stmt.expr is not None
        if type_check(stmt.expr, state.types) != "Bool":
            raise ModelFault("NX-E0102", "types.check")
        while evaluate(stmt.expr, state.env, state.journal):
            state.tick()
            try:
                execute_block(stmt.body, state)
            except _Continue:
                continue
            except _Break:
                break
        return
    if stmt.kind == "break":
        raise _Break()
    if stmt.kind == "continue":
        raise _Continue()
    if stmt.kind == "return":
        raise _Return()
    if stmt.kind == "expr":
        assert stmt.expr is not None
        type_check(stmt.expr, state.types)
        evaluate(stmt.expr, state.env, state.journal)
        return
    raise ValueError(stmt.kind)


def execute_block(body: tuple[Stmt, ...], state: State) -> None:
    for stmt in body:
        state.tick()
        execute_statement(stmt, state)


def execute(program: Program, max_steps: int = 16_000_000) -> State:
    state = State(max_steps=max_steps)
    try:
        execute_block(program.body, state)
    except _Return:
        pass
    return state


# The literal programs in F5 §§9.3 and 11 are parsed by an independent small
# recognizer. Generated cases use the typed tree above; this recognizer exists
# to exercise the published source examples without hand-coding their result.

from collections.abc import Callable
import re


@dataclass(frozen=True)
class Token:
    text: str
    start: int
    end: int


@dataclass(frozen=True)
class SourceNode:
    kind: str
    value: Any = None
    children: tuple[SourceNode, ...] = ()
    start: int = 0
    end: int = 0
    annotation: str = ""


@dataclass(frozen=True)
class SourceFunction:
    name: str
    params: tuple[tuple[str, str, str], ...]
    body: tuple[SourceNode, ...]
    return_type: str = "()"
    contract: str = ""
    end: int = 0
    return_span: tuple[int, int] = (0, 0)
    contract_span: tuple[int, int] = (0, 0)
    name_span: tuple[int, int] = (0, 0)


@dataclass(frozen=True)
class SourceProgram:
    constants: tuple[tuple[str, str, SourceNode], ...]
    functions: dict[str, SourceFunction]
    constant_spans: dict[str, tuple[int, int]] = field(default_factory=dict)
    constant_type_spans: dict[str, tuple[int, int]] = field(default_factory=dict)


_TOKEN = re.compile(
    r"\s+|//[^\r\n]*|[A-Za-z_][A-Za-z_0-9]*|[0-9]+|"
    r"\.\.<|\.\.=|\+=|-=|\*=|/=|%=|==|!=|<=|>=|->|=>|"
    r"[^\s]", re.ASCII,
)
_INFIX = {
    "or": 1, "and": 2,
    "<": 3, "<=": 3, ">": 3, ">=": 3, "==": 3, "!=": 3,
    "..<": 4, "..=": 4,
    "+": 5, "-": 5, "*": 6, "/": 6, "%": 6,
}
_EXPR_START = ["(", "-", "IntLiteral", "Name", "[", "edit", "false", "fn",
               "if", "match", "move", "not", "true"]
_NEG_START = ["(", "-", "IntLiteral", "Name", "[", "edit", "false", "fn",
              "move", "not", "true"]
_BLOCK_NEXT = ["(", "-", "IntLiteral", "Name", "[", "break", "continue",
               "edit", "false", "fn", "for", "if", "let", "match", "move",
               "not", "return", "true", "var", "while", "yield", "{", "}"]
_TYPE_START = ["(", "Array", "Bool", "I64", "MockBlockingResource",
               "MockResource", "Option", "Result"]


def lex_source(source: str) -> list[Token]:
    result = []
    for match in _TOKEN.finditer(source):
        token = match.group()
        if token.isspace() or token.startswith("//"):
            continue
        result.append(Token(token, len(source[:match.start()].encode("utf-8")),
                            len(source[:match.end()].encode("utf-8"))))
    size = len(source.encode("utf-8"))
    result.append(Token("<EOF>", size, size))
    return result


class SourceParser:
    def __init__(self, source: str) -> None:
        self.tokens = lex_source(source)
        self.index = 0

    @property
    def token(self) -> Token:
        return self.tokens[self.index]

    def take(self, expected: str | None = None) -> Token:
        token = self.token
        if expected is not None and token.text != expected:
            raise SyntaxError(f"expected {expected!r}, got {token.text!r} at {token.start}")
        self.index += 1
        return token

    def accept(self, value: str) -> bool:
        if self.token.text == value:
            self.index += 1
            return True
        return False

    def type_until(self, endings: set[str]) -> str:
        start = self.index
        depth = 0
        if self.token.text in (")", "<EOF>"):
            raise SourceSyntaxFault(self.token, "grammar.Type", _TYPE_START,
                                    self.token.start, self.token.start)
        while self.token.text != "<EOF>":
            text = self.token.text
            # Longest-match lexing makes `Array<I64>=[]` one >= token. In a
            # type context that token closes the type and starts assignment.
            if text == ">=" and depth > 0:
                old = self.token
                self.tokens[self.index:self.index + 1] = [
                    Token(">", old.start, old.start + 1),
                    Token("=", old.start + 1, old.end),
                ]
                text = ">"
            if depth == 0 and text in endings:
                break
            if text == "<":
                depth += 1
            elif text == ">":
                depth -= 1
            self.take()
        return "".join(t.text for t in self.tokens[start:self.index])

    def program(self) -> SourceProgram:
        constants: list[tuple[str, str, SourceNode]] = []
        constant_spans: dict[str, tuple[int, int]] = {}
        constant_type_spans: dict[str, tuple[int, int]] = {}
        functions: dict[str, SourceFunction] = {}
        while self.token.text != "<EOF>":
            if self.token.text == "pub":
                token = self.take()
                raise SourceScopeFault("modules_visibility", token.start, token.end,
                                       "parse", "syntax.parse")
            if self.accept("const"):
                if self.token.text == "<EOF>":
                    raise SourceSyntaxFault(self.token, "grammar.ConstDecl", ["Name"],
                                            0, self.token.start)
                name_token = self.take()
                name = name_token.text
                if name in constant_spans:
                    raise SourceDuplicateConstant(name_token, constant_spans[name])
                constant_spans[name] = (name_token.start, name_token.end)
                self.take(":")
                type_start = self.token.start
                annotation = self.type_until({"="})
                constant_type_spans[name] = (type_start, self.tokens[self.index - 1].end)
                self.take("=")
                self.take("comptime")
                expression = self.expression()
                self.take(";")
                constants.append((name, annotation, expression))
                continue
            if self.token.text != "fn":
                raise SourceSyntaxFault(self.token, "module_item",
                                        ["EOF", "const", "fn", "gen"],
                                        0, self.token.start)
            self.take("fn")
            name_token = self.take()
            name = name_token.text
            self.take("(")
            params: list[tuple[str, str, str]] = []
            if self.token.text != ")":
                while True:
                    label = self.take().text
                    self.take(":")
                    mode = self.take().text
                    if mode not in ("view", "edit", "move"):
                        raise SyntaxError(f"bad parameter mode {mode}")
                    annotation = self.type_until({",", ")"})
                    params.append((label, mode, annotation))
                    if not self.accept(","):
                        break
            self.take(")")
            return_span = (0, 0)
            if self.accept("->"):
                return_start = self.token.start
                return_type = self.type_until({"{", "with"})
                return_span = (return_start, self.tokens[self.index - 1].end)
            else:
                return_type = "()"
            contract = ""
            contract_span = (0, 0)
            if self.accept("with"):
                contract_token = self.take()
                contract = contract_token.text
                contract_span = (contract_token.start, contract_token.end)
            body = self.block()
            functions[name] = SourceFunction(name, tuple(params), body, return_type,
                                             contract, self.tokens[self.index - 1].end,
                                             return_span, contract_span,
                                             (name_token.start, name_token.end))
        return SourceProgram(tuple(constants), functions,
                             constant_spans, constant_type_spans)

    def block(self) -> tuple[SourceNode, ...]:
        opener = self.take("{")
        items = []
        while self.token.text != "}":
            if self.token.text == "<EOF>":
                raise SourceSyntaxFault(self.token, "block_item", _BLOCK_NEXT,
                                        opener.start, opener.end)
            items.append(self.statement())
        self.take("}")
        return tuple(items)

    def statement(self) -> SourceNode:
        start = self.token.start
        if self.token.text in ("let", "var"):
            kind = self.take().text
            name = self.take().text
            annotation = ""
            if self.accept(":"):
                annotation = self.type_until({"="})
            self.take("=")
            expr = self.expression(context="binding_initializer", prefix_start=start)
            end = self.take(";").end
            return SourceNode(kind, name, (expr,), start, end, annotation)
        if self.accept("for"):
            edit = self.accept("edit")
            if self.token.text == "<EOF>":
                raise SourceSyntaxFault(self.token, "grammar.ForStmt", ["Name", "edit"],
                                        start, self.token.start)
            name = self.take().text
            self.take("in")
            iterable = self.expression()
            body = self.block()
            return SourceNode("for_edit" if edit else "for", name,
                              (iterable, SourceNode("block", children=body)), start,
                              self.tokens[self.index - 1].end)
        if self.accept("while"):
            condition = self.expression()
            body = self.block()
            return SourceNode("while", children=(condition, SourceNode("block", children=body)),
                              start=start, end=self.tokens[self.index - 1].end)
        if self.accept("if"):
            condition = self.expression()
            body = self.block()
            otherwise = self.block() if self.accept("else") else ()
            return SourceNode("if", children=(condition, SourceNode("block", children=body),
                                              SourceNode("block", children=otherwise)),
                              start=start, end=self.tokens[self.index - 1].end)
        if self.token.text in ("break", "continue"):
            kind = self.take().text
            end = self.take(";").end
            return SourceNode(kind, start=start, end=end)
        if self.accept("return"):
            expr = None if self.token.text == ";" else self.expression()
            end = self.take(";").end
            return SourceNode("return", children=() if expr is None else (expr,),
                              start=start, end=end)
        if self.token.text == "{":
            body = self.block()
            return SourceNode("block", children=body, start=start,
                              end=self.tokens[self.index - 1].end)
        target = self.expression()
        if self.token.text in ("=", "+=", "-=", "*=", "/=", "%="):
            op = self.take().text
            rhs = self.expression(context="grammar.Assignment", prefix_start=start)
            end = self.take(";").end
            return SourceNode("assign", op, (target, rhs), start, end)
        end = self.take(";").end
        return SourceNode("expr", children=(target,), start=start, end=end)

    def expression(self, minimum: int = 0,
                   context: str | None = None,
                   prefix_start: int | None = None) -> SourceNode:
        if self.token.text == "<EOF>" and context is not None:
            expected = _NEG_START if context in ("grammar.NegExpr", "grammar.AddExpr") \
                else _EXPR_START
            raise SourceSyntaxFault(self.token, context, expected,
                                    prefix_start if prefix_start is not None else self.token.start,
                                    self.token.start)
        token = self.take()
        start = token.start
        if token.text.isdecimal():
            node = SourceNode("int", int(token.text), start=start, end=token.end)
        elif token.text in ("true", "false"):
            node = SourceNode("bool", token.text == "true", start=start, end=token.end)
        elif token.text == "None":
            node = SourceNode("none", start=start, end=token.end)
        elif token.text == "(":
            if self.accept(")"):
                node = SourceNode("unit", start=start, end=self.tokens[self.index - 1].end)
            else:
                inner = self.expression()
                end = self.take(")").end
                node = SourceNode("group", children=(inner,), start=start, end=end)
        elif token.text == "[":
            items = []
            if self.token.text != "]":
                while True:
                    items.append(self.expression(context="array_element", prefix_start=start))
                    if not self.accept(","):
                        break
            end = self.take("]").end
            node = SourceNode("array", children=tuple(items), start=start, end=end)
        elif token.text in ("-", "not", "move", "edit"):
            if token.text == "-" and self.token.text.isdecimal():
                number = self.take()
                node = SourceNode("int", -int(number.text), start=start, end=number.end)
            else:
                child = self.expression(7, context="grammar.NegExpr", prefix_start=start)
                node = SourceNode(token.text, children=(child,), start=start, end=child.end)
        elif token.text == "if":
            condition = self.expression()
            self.take("then")
            yes = self.value_block()
            self.take("else")
            no = self.value_block()
            node = SourceNode("ifexpr", children=(condition, yes, no), start=start,
                              end=self.tokens[self.index - 1].end)
        elif token.text == "comptime":
            child = self.expression(7)
            node = SourceNode("comptime", children=(child,), start=start, end=child.end)
        elif token.text == "fn":
            # Closure literal, outside the F5 Array element domain.
            self.take("[")
            while self.token.text != "]":
                self.take()
            self.take("]")
            self.take("(")
            while self.token.text != ")":
                self.take()
            self.take(")")
            body = self.block()
            node = SourceNode("closure", children=(SourceNode("block", children=body),),
                              start=start, end=self.tokens[self.index - 1].end)
        elif token.text[0].isalpha() or token.text[0] == "_":
            node = SourceNode("name", token.text, start=start, end=token.end)
        else:
            raise SyntaxError(f"unexpected {token.text!r} at {token.start}")
        while True:
            if self.token.text == "(" and minimum <= 8:
                self.take("(")
                args = []
                if self.token.text != ")":
                    while True:
                        args.append(self.expression())
                        if not self.accept(","):
                            break
                end = self.take(")").end
                node = SourceNode("call", children=(node, *args), start=node.start, end=end)
                continue
            if self.token.text == "[" and minimum <= 8:
                self.take("[")
                index = self.expression()
                end = self.take("]").end
                node = SourceNode("index", children=(node, index), start=node.start, end=end)
                continue
            if self.token.text == "?" and minimum <= 8:
                end = self.take("?").end
                node = SourceNode("try", children=(node,), start=node.start, end=end)
                continue
            prec = _INFIX.get(self.token.text, -1)
            if prec < minimum:
                break
            if (prec in (3, 4) and node.kind == "binary" and
                    _INFIX.get(node.value) == prec):
                operator = self.token
                context = "grammar.CompareExpr" if prec == 3 else "grammar.RangeExpr"
                expected = [";", "and", "or"] if prec == 3 else \
                    ["!=", ";", "<", "<=", "==", ">", ">=", "and", "or"]
                raise SourceSyntaxFault(operator, context, expected, node.start,
                                        operator.start)
            op = self.take().text
            right = self.expression(prec + 1,
                                    context="grammar.AddExpr" if op == "+" else None,
                                    prefix_start=node.start)
            node = SourceNode("binary", op, (node, right), node.start, right.end)
        return node

    def value_block(self) -> SourceNode:
        self.take("{")
        self.take("value")
        self.take(":")
        expression = self.expression()
        self.take(";")
        self.take("}")
        return expression


def parse_source(source: str) -> SourceProgram:
    return SourceParser(source).program()


class SourceSyntaxFault(SyntaxError):
    def __init__(self, token: Token, context: str, expected: list[str],
                 prefix_start: int, prefix_end: int) -> None:
        super().__init__(f"unexpected {token.text!r} in {context} at {token.start}")
        self.token = token
        self.context = context
        self.expected = expected
        self.prefix_start = prefix_start
        self.prefix_end = prefix_end


class SourceScopeFault(Exception):
    def __init__(self, feature_id: str, start: int, end: int,
                 stage: str, operation: str) -> None:
        super().__init__(feature_id)
        self.feature_id = feature_id
        self.start = start
        self.end = end
        self.stage = stage
        self.operation = operation


class SourceDuplicateConstant(Exception):
    def __init__(self, token: Token, previous: tuple[int, int]) -> None:
        super().__init__(token.text)
        self.token = token
        self.previous = previous


@dataclass
class CellRef:
    read: Callable[[], Any]
    write: Callable[[Any], None]


class SourceReturn(Exception):
    def __init__(self, value: Any) -> None:
        super().__init__()
        self.value = value


class SourceVM:
    """Execute the accepted scalar/Array/control domain of source literals."""

    def __init__(self, program: SourceProgram, *, limit: int = 16_000_000) -> None:
        self.program = program
        self.scopes: list[dict[str, Any]] = [{}]
        self.const_expr = {name: expr for name, _, expr in program.constants}
        self.const_active: set[str] = set()
        self.stdout: list[str] = []
        self.events: list[str] = []
        self.observations: list[dict[str, Any]] = []
        self.steps = 0
        self.limit = limit
        self.resource_id = 0

    def tick(self) -> None:
        self.steps += 1
        if self.steps > self.limit:
            raise ModelFault("R0001", "runtime.limit")

    def get(self, name: str) -> Any:
        for scope in reversed(self.scopes):
            if name in scope:
                value = scope[name]
                return value.read() if isinstance(value, CellRef) else value
        if name in self.const_expr:
            if name in self.const_active:
                raise ModelFault("NX-E1323", "types.check")
            self.const_active.add(name)
            try:
                value = self.eval(self.const_expr[name])
            finally:
                self.const_active.remove(name)
            self.scopes[0][name] = copy_value(value)
            return value
        raise NameError(name)

    def reference(self, node: SourceNode) -> CellRef:
        if node.kind == "edit" or node.kind == "move":
            return self.reference(node.children[0])
        if node.kind == "name":
            name = node.value
            for scope in reversed(self.scopes):
                if name in scope:
                    current = scope[name]
                    if isinstance(current, CellRef):
                        return current
                    return CellRef(lambda scope=scope, name=name: scope[name],
                                   lambda value, scope=scope, name=name: scope.__setitem__(name, value))
            raise ModelFault("NX-E0105", "place.write")
        if node.kind == "index":
            array = self.eval(node.children[0])
            index = self.eval(node.children[1])
            if type(index) is not int or index < 0 or index >= len(array.items):
                raise ModelFault("R0102", "array.index")
            return CellRef(lambda array=array, index=index: array.items[index],
                           lambda value, array=array, index=index: array.items.__setitem__(index, value))
        raise ModelFault("NX-E0105", "place.write")

    def eval(self, node: SourceNode) -> Any:
        try:
            return self._eval(node)
        except ModelFault as fault:
            if not hasattr(fault, "start"):
                fault.start = node.start
                fault.end = node.end
            raise

    def _eval(self, node: SourceNode) -> Any:
        self.tick()
        kind = node.kind
        if kind in ("int", "bool"):
            if kind == "int" and not MIN_I64 <= node.value <= MAX_I64:
                raise ModelFault("NX-E0107", "value.literal")
            return node.value
        if kind == "unit":
            return None
        if kind == "none":
            return ("None",)
        if kind == "closure":
            return ("Closure", node)
        if kind == "name":
            return self.get(node.value)
        if kind == "group":
            return self.eval(node.children[0])
        if kind == "array":
            items = [copy_value(self.eval(child)) for child in node.children]
            return Array(items, len(items))
        if kind == "index":
            return self.reference(node).read()
        if kind == "-":
            return i64(-self.eval(node.children[0]), "value.neg")
        if kind == "not":
            return not self.eval(node.children[0])
        if kind == "move":
            return self.eval(node.children[0])
        if kind == "edit":
            return self.reference(node.children[0])
        if kind == "try":
            value = self.eval(node.children[0])
            if isinstance(value, tuple) and value and value[0] == "Err":
                raise SourceReturn(value)
            return value[1] if isinstance(value, tuple) and value and value[0] == "Ok" else value
        if kind == "ifexpr":
            return self.eval(node.children[1] if self.eval(node.children[0]) else node.children[2])
        if kind == "binary":
            op = node.value
            if op in ("..<", "..="):
                # A range is only evaluated by for; it never becomes a value.
                raise ModelFault("NX-E0006", "types.check")
            left = self.eval(node.children[0])
            if op == "and":
                return bool(left) and bool(self.eval(node.children[1]))
            if op == "or":
                return bool(left) or bool(self.eval(node.children[1]))
            right = self.eval(node.children[1])
            return binary(op, left, right)
        if kind == "call":
            callee = node.children[0]
            if callee.kind != "name":
                raise NameError("indirect call outside F5 subset")
            name = callee.value
            if name == "Push":
                array = self.reference(node.children[1]).read()
                value = self.eval(node.children[2])
                if len(array.items) == array.capacity:
                    array.capacity = max(1, 2 * array.capacity)
                array.items.append(copy_value(value))
                return None
            if name == "Pop":
                array = self.reference(node.children[1]).read()
                return ("Some", array.items.pop()) if array.items else ("None",)
            args = [self.eval(child) for child in node.children[1:]]
            if name == "Len":
                return len(args[0].items)
            if name == "JoinArrays":
                items = [copy_value(x) for x in args[0].items + args[1].items]
                return Array(items, len(items))
            if name == "Some":
                return ("Some", copy_value(args[0]))
            if name == "Err":
                return ("Err", copy_value(args[0]))
            if name == "Ok":
                return ("Ok", copy_value(args[0]))
            if name == "AssertEq":
                if not equal(args[0], args[1]):
                    raise ModelFault("R0201", "assert.eq")
                return None
            if name == "Print":
                line = format_value(args[0])
                self.stdout.append(line)
                return None
            if name == "MockOpen":
                ident = self.resource_id
                self.resource_id += 1
                self.events.append(f"open:{args[0]}:{ident}")
                return ("Resource", args[0], ident)
            return self.call(name, args)
        raise ValueError(kind)

    def call(self, name: str, args: list[Any]) -> Any:
        function = self.program.functions[name]
        if len(args) != len(function.params):
            raise TypeError((name, len(args), len(function.params)))
        scope = {}
        for (param, mode, _annotation), arg in zip(function.params, args):
            scope[param] = arg if mode == "edit" else copy_value(arg)
        self.scopes.append(scope)
        try:
            self.block(function.body)
        except SourceReturn as result:
            return result.value
        finally:
            self.cleanup_scope()
        return None

    def cleanup_scope(self) -> None:
        scope = self.scopes.pop()
        for value in reversed(list(scope.values())):
            if isinstance(value, tuple) and value and value[0] == "Resource":
                self.events.append(f"close:{value[1]}:{value[2]}")

    def block(self, body: tuple[SourceNode, ...]) -> None:
        self.scopes.append({})
        try:
            for statement in body:
                self.statement(statement)
        finally:
            self.cleanup_scope()

    def statement(self, node: SourceNode) -> None:
        self.tick()
        kind = node.kind
        if kind in ("let", "var"):
            self.scopes[-1][node.value] = copy_value(self.eval(node.children[0]))
            return
        if kind == "assign":
            target = self.reference(node.children[0])
            old = target.read() if node.value != "=" else None
            rhs = self.eval(node.children[1])
            try:
                result = binary(node.value[0], old, rhs) if node.value != "=" else copy_value(rhs)
            except ModelFault as fault:
                fault.start, fault.end = node.start, node.end - 1
                raise
            target.write(result)
            return
        if kind == "expr":
            self.eval(node.children[0])
            return
        if kind == "block":
            self.block(node.children)
            return
        if kind == "return":
            raise SourceReturn(self.eval(node.children[0]) if node.children else None)
        if kind == "break":
            raise _Break()
        if kind == "continue":
            raise _Continue()
        if kind == "if":
            self.block(node.children[1].children if self.eval(node.children[0])
                       else node.children[2].children)
            return
        if kind == "while":
            while self.eval(node.children[0]):
                try:
                    self.block(node.children[1].children)
                except _Continue:
                    continue
                except _Break:
                    break
            return
        if kind == "for":
            iterable = node.children[0]
            while iterable.kind == "group":
                iterable = iterable.children[0]
            body = node.children[1].children
            if iterable.kind == "binary" and iterable.value in ("..<", "..="):
                lo = self.eval(iterable.children[0])
                hi = self.eval(iterable.children[1])
                current = lo
                while current < hi or (iterable.value == "..=" and current == hi):
                    self.scopes.append({node.value: current})
                    try:
                        self.block(body)
                    except _Continue:
                        pass
                    except _Break:
                        break
                    finally:
                        self.cleanup_scope()
                    if iterable.value == "..=" and current == hi:
                        break
                    current += 1
                return
            array = self.eval(iterable)
            if not isinstance(array, Array):
                raise ModelFault("NX-E0102", "types.check")
            for item in array.items[:]:
                self.scopes.append({node.value: item})
                try:
                    self.block(body)
                except _Continue:
                    pass
                except _Break:
                    break
                finally:
                    self.cleanup_scope()
            return
        raise ValueError(kind)

    def run(self) -> SourceVM:
        self.call("Main", [])
        return self

    @property
    def observation_bytes(self) -> int:
        return sum(len((line + "\n").encode("utf-8")) for line in self.stdout)


def format_value(value: Any) -> str:
    if isinstance(value, Array):
        return "[" + ", ".join(format_value(item) for item in value.items) + "]"
    if type(value) is bool:
        return "true" if value else "false"
    if value is None:
        return "()"
    if isinstance(value, tuple) and value:
        return value[0] + ("(" + format_value(value[1]) + ")" if len(value) == 2 else "")
    return str(value)


def run_source(source: str) -> SourceVM:
    return SourceVM(parse_source(source)).run()
