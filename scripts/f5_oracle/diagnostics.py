"""F5 diagnostic projections built from the ADR, never from Nexo JSON."""

from __future__ import annotations

from typing import Any
from dataclasses import dataclass

from .model import (ModelFault, SourceDuplicateConstant, SourceNode,
                    SourceProgram, SourceScopeFault, SourceVM, parse_source)

PROFILE = "f0-0005-b-v1"


def span(start: int, end: int, file: str = "case.n") -> dict[str, Any]:
    return {"file": file, "start": start, "end": end}


def semantic(code: str, stage: str, operation: str, start: int, end: int,
             *, subcause: str | None = None, data: dict[str, Any] | None = None,
             related: list[dict[str, Any]] | None = None,
             cause: list[dict[str, Any]] | None = None,
             feature_id: str | None = None, message: str = "") -> dict[str, Any]:
    rule = code + ("/" + subcause if subcause else "")
    return {
        "schema_version": 1,
        "diagnostic_namespace": "F0-0002-sim" if code.startswith("R") else "Nexo-proposed-v1",
        "profile_id": PROFILE,
        "code": code,
        "subcause": subcause,
        "rule": rule,
        "explanation_id": None if code.startswith("R") else rule,
        "severity": "error",
        "stage": stage,
        "message": message,
        "operation": operation,
        "primary_span": span(start, end),
        "related_spans": related or [],
        "cause_chain": cause or [],
        "feature_id": feature_id,
        "unexpected_token": None,
        "expected_tokens": [],
        "expected_tokens_kind": None,
        "parser_context": None,
        "recognized_prefix": None,
        "repair": [],
        "data": data if data is not None else {},
    }


def pending(capability: str, stage: str, start: int, end: int,
            *, feature_id: str = "implementation_limit") -> dict[str, Any]:
    operations = {"parse": "syntax.parse", "types": "types.check",
                  "ownership": "ownership.check", "lower": "ir.lower"}
    return semantic("NX-E0006", stage, operations[stage], start, end,
                    data={"pending_scope": capability}, feature_id=feature_id,
                    message=f"Capacidad pendiente de implementación en {capability}.")


def missing_main(source: str) -> dict[str, Any]:
    """D15 section 7.2 entry rule for `run` of a checked library."""
    offset = len(source.encode("utf-8"))
    return semantic("NX-E0112", "entry", "entry.check", offset, offset,
                    subcause="missing_main", data={"entry": "Main"},
                    message="Invalid name, call or type in the proposed profile.")


def syntax(start: int, end: int, text: str, kind: str,
           expected: list[str], context: str,
           prefix_start: int, prefix_end: int) -> dict[str, Any]:
    result = semantic("NX-E0002", "parse", "syntax.parse", start, end,
                      message="Syntax is not admitted by the grammar.")
    result["unexpected_token"] = {"kind": kind, "text": text, "span": span(start, end)}
    result["expected_tokens"] = expected
    result["expected_tokens_kind"] = "admissible_set"
    result["parser_context"] = context
    result["recognized_prefix"] = span(prefix_start, prefix_end)
    return result


def pending_projection(diagnostic: dict[str, Any], *, literal_span: bool) -> dict[str, Any]:
    """Contract for a pending capability with no published full JSON literal."""
    fields = ("code", "subcause", "stage", "operation", "feature_id")
    result = {key: diagnostic[key] for key in fields}
    result["pending_scope"] = diagnostic["data"]["pending_scope"]
    if literal_span:
        result["primary_span"] = diagnostic["primary_span"]
    return result


@dataclass(frozen=True)
class Issue:
    code: str
    subcause: str | None
    start: int
    end: int
    stage: str
    operation: str
    data: dict[str, Any]
    related: tuple[tuple[str, int, int], ...] = ()
    feature_id: str | None = None
    cause: tuple[dict[str, Any], ...] = ()


class RuleViolation(Exception):
    def __init__(self, issue: Issue) -> None:
        super().__init__(issue.code)
        self.issue = issue


@dataclass
class Binding:
    ty: str
    mutable: bool
    start: int
    end: int
    element_origin: tuple[int, int] | None = None
    constant: bool = False


def mismatch(expected: str, found: str, node: SourceNode,
             *, origin: tuple[int, int] | None = None,
             primary: tuple[int, int] | None = None) -> RuleViolation:
    related = []
    if origin is not None:
        related.append(("expected_type", *origin))
    related.append(("found_type", node.start, node.end))
    a, b = primary if primary is not None else (node.start, node.end)
    return RuleViolation(Issue("NX-E0102", "type_mismatch", a, b,
                               "types", "types.check",
                               {"expected_type": expected, "found_type": found},
                               tuple(related)))


def scope_issue(feature: str, node: SourceNode, stage: str = "types",
                operation: str = "types.check") -> RuleViolation:
    return RuleViolation(Issue("NX-E0006", None, node.start, node.end,
                               stage, operation, {"pending_scope": "later_adr"},
                               feature_id=feature))


def root_name(node: SourceNode) -> str | None:
    while node.kind in ("index", "edit", "move", "group"):
        node = node.children[0]
    return node.value if node.kind == "name" else None


def first_read(node: SourceNode, name: str) -> SourceNode | None:
    if node.kind == "name" and node.value == name:
        return node
    for child in node.children:
        found = first_read(child, name)
        if found is not None:
            return found
    return None


def walk(node: SourceNode):
    yield node
    for child in node.children:
        yield from walk(child)


def body_nodes(body: tuple[SourceNode, ...]):
    for statement in body:
        yield from walk(statement)


def borrow_conflict(read: SourceNode, loan_origin: SourceNode,
                    declaration: Binding, permission: str,
                    requested: str = "read") -> RuleViolation:
    owner = root_name(loan_origin)
    assert owner is not None
    origin = loan_origin.start
    loan = f"loan:{origin}:0"
    binding = f"binding:{declaration.start}"
    return RuleViolation(Issue("NX-E1105", "active_borrow_conflict",
                               read.start, read.end, "ownership", "ownership.access",
                               {"origin": binding, "binding": binding, "loan": loan,
                                "active_permission": permission,
                                "requested_access": requested},
                               (("declaration", declaration.start, declaration.end),
                                ("origin", loan_origin.start, loan_origin.end)),
                               cause=({"kind": "borrow",
                                       "operation": "borrow." + permission,
                                       "span": span(loan_origin.start, loan_origin.end),
                                       "entity": {"kind": "loan", "id": loan},
                                       "evidence": "verified"},)))


class SourceChecker:
    """Static rules for generated and literal sources, using only F5 rules."""

    def __init__(self, program: SourceProgram) -> None:
        self.program = program
        self.consts = {name: Binding(ty, False, *program.constant_spans[name], constant=True)
                       for name, ty, _ in program.constants}
        self.bindings: dict[str, Binding] = {}
        self.active_array_loan: dict[str, SourceNode] = {}

    def find(self, name: str) -> Binding:
        if name in self.bindings:
            return self.bindings[name]
        if name in self.consts:
            return self.consts[name]
        raise NameError(name)

    def infer(self, node: SourceNode) -> str:
        kind = node.kind
        if kind == "int":
            if not -(1 << 63) <= node.value <= (1 << 63) - 1:
                raise RuleViolation(Issue("NX-E0107", None, node.start, node.end,
                                          "types", "value.literal", {}))
            return "I64"
        if kind == "bool":
            return "Bool"
        if kind == "unit":
            return "()"
        if kind == "none":
            return "Option<?>"
        if kind == "group":
            return self.infer(node.children[0])
        if kind == "name":
            return self.find(node.value).ty
        if kind in ("edit", "move"):
            root = node.children[0]
            if root.kind == "name" and self.find(root.value).constant:
                binding = self.find(root.value)
                raise RuleViolation(Issue("NX-E0105", None, root.start, root.end,
                                          "ownership", "place.write", {},
                                          (("declaration", binding.start, binding.end),)))
            return self.infer(root)
        if kind == "comptime":
            raise RuleViolation(Issue("NX-E0006", None, node.start, node.start + 8,
                                      "parse", "syntax.parse",
                                      {"pending_scope": "later_adr"},
                                      feature_id="comptime"))
        if kind == "closure":
            return "Closure"
        if kind == "array":
            if not node.children:
                return "Array<?>"
            element_type = self.infer(node.children[0])
            for child in node.children[1:]:
                found = self.infer(child)
                if found != element_type:
                    raise mismatch(element_type, found, child,
                                   origin=(node.children[0].start, node.children[0].end))
            if element_type == "Closure":
                raise scope_issue("closure", node)
            return "Array<" + element_type + ">"
        if kind == "index":
            if node.children[1].kind == "binary" and node.children[1].value in ("..<", "..="):
                selector = SourceNode("selector", start=node.children[0].end, end=node.end)
                raise scope_issue("range", selector)
            array_type = self.infer(node.children[0])
            index_type = self.infer(node.children[1])
            if index_type != "I64":
                raise mismatch("I64", index_type, node.children[1])
            return array_type[6:-1] if array_type.startswith("Array<") else "?"
        if kind in ("-", "not"):
            child = node.children[0]
            found = self.infer(child)
            expected = "I64" if kind == "-" else "Bool"
            if found != expected:
                raise mismatch(expected, found, child)
            return expected
        if kind == "ifexpr":
            condition = self.infer(node.children[0])
            if condition != "Bool":
                raise mismatch("Bool", condition, node.children[0])
            left = self.infer(node.children[1])
            right = self.infer(node.children[2])
            if left != right:
                raise mismatch(left, right, node.children[2],
                               origin=(node.children[1].start, node.children[1].end))
            return left
        if kind == "binary":
            op = node.value
            if op in ("..<", "..="):
                raise RuleViolation(Issue("NX-E0006", None,
                                          node.children[0].end, node.children[1].start,
                                          "types", "types.check",
                                          {"pending_scope": "later_adr"},
                                          feature_id="range"))
            left_node, right_node = node.children
            left, right = self.infer(left_node), self.infer(right_node)
            if op in ("<", "<=", ">", ">=", "==", "!="):
                if left != right:
                    raise mismatch(left, right, right_node,
                                   origin=(left_node.start, left_node.end))
                expected = "I64" if op in ("<", "<=", ">", ">=") else "domain:copyable_data"
                admissible = left == "I64" if expected == "I64" else \
                    left not in ("Closure", "Resource")
                if not admissible:
                    raise mismatch(expected, left, left_node,
                                   primary=(left_node.end, right_node.start))
                return "Bool"
            expected = "Bool" if op in ("and", "or") else "I64"
            for child, found in ((left_node, left), (right_node, right)):
                if found != expected:
                    raise mismatch(expected, found, child)
            return expected
        if kind == "call":
            callee = node.children[0]
            if callee.kind != "name":
                raise NameError("indirect call")
            name = callee.value
            args = node.children[1:]
            if name in ("Len", "Pop", "Push"):
                array = args[0].children[0] if args[0].kind == "edit" else args[0]
                array_type = self.infer(array)
                if name == "Len":
                    return "I64"
                if array.kind == "name" and not self.find(array.value).mutable:
                    binding = self.find(array.value)
                    raise RuleViolation(Issue("NX-E0105", None, array.start, array.end,
                                              "ownership", "place.write", {},
                                              (("declaration", binding.start, binding.end),)))
                if name == "Pop":
                    return "Option<" + array_type[6:-1] + ">"
                value_type = self.infer(args[1])
                owner = root_name(array)
                read = first_read(args[1], owner) if owner else None
                if read is not None:
                    raise borrow_conflict(read, args[0], self.find(owner), "inout")
                if array_type == "Array<?>" and array.kind == "name":
                    binding = self.find(array.value)
                    binding.ty = "Array<" + value_type + ">"
                    binding.element_origin = (args[1].start, args[1].end)
                elif array_type.startswith("Array<"):
                    expected = array_type[6:-1]
                    if value_type != expected:
                        binding = self.find(array.value) if array.kind == "name" else None
                        raise mismatch(expected, value_type, args[1],
                                       origin=binding.element_origin if binding else None)
                return "()"
            if name == "JoinArrays":
                left, right = self.infer(args[0]), self.infer(args[1])
                if left != right:
                    raise mismatch(left, right, args[1],
                                   origin=(args[0].start, args[0].end))
                return left
            if name == "Some":
                return "Option<" + self.infer(args[0]) + ">"
            if name in ("Print", "AssertEq", "MockOpen"):
                for arg in args:
                    self.infer(arg)
                return "Resource" if name == "MockOpen" else "()"
            if name in self.program.functions:
                function = self.program.functions[name]
                for arg in args:
                    self.infer(arg)
                return function.return_type
            raise NameError(name)
        if kind == "try":
            return self.infer(node.children[0])
        raise ValueError(kind)

    def statement(self, node: SourceNode, loop_depth: int = 0,
                  borrowed_root: str | None = None) -> None:
        kind = node.kind
        if kind in ("let", "var"):
            ty = self.infer(node.children[0])
            if node.annotation:
                if "MockResource" in node.annotation and node.annotation.startswith("Array<"):
                    raise scope_issue("resource_array", SourceNode("type", start=node.start + 6,
                                                                     end=node.start + 6 + len(node.annotation)))
                if ty != "Array<?>" and ty != node.annotation:
                    raise mismatch(node.annotation, ty, node.children[0])
                ty = node.annotation
            self.bindings[node.value] = Binding(ty, kind == "var", node.start + 4,
                                                node.start + 4 + len(node.value),
                                                (node.children[0].children[0].start,
                                                 node.children[0].children[0].end)
                                                if node.children[0].kind == "array" and
                                                node.children[0].children else None)
            return
        if kind == "assign":
            target, rhs = node.children
            target_type = self.infer(target)
            if node.value != "=" and target_type != "I64":
                raise mismatch("I64", target_type, target)
            root = target
            while root.kind == "index":
                root = root.children[0]
            if root.kind == "name" and borrowed_root == root.value:
                origin = self.active_array_loan[borrowed_root]
                raise borrow_conflict(root, origin, self.find(borrowed_root),
                                      "read", "write")
            found = self.infer(rhs)
            if found != target_type:
                raise mismatch(target_type, found, rhs,
                               origin=self.find(root.value).element_origin if root.kind == "name" else None)
            owner = root_name(target)
            read = first_read(rhs, owner) if owner and node.value != "=" else None
            if read is not None:
                raise borrow_conflict(read, target, self.find(owner), "inout")
            if root.kind == "name" and not self.find(root.value).mutable:
                binding = self.find(root.value)
                raise RuleViolation(Issue("NX-E0105", None, root.start, root.end,
                                          "ownership", "place.write", {},
                                          (("declaration", binding.start, binding.end),)))
            return
        if kind == "for_edit":
            raise RuleViolation(Issue("NX-E0006", None, node.start + 4, node.start + 8,
                                      "parse", "syntax.parse",
                                      {"pending_scope": "later_adr"}, feature_id="for_iteration"))
        if kind == "for":
            iterable = node.children[0]
            while iterable.kind == "group":
                iterable = iterable.children[0]
            if iterable.kind == "binary" and iterable.value in ("..<", "..="):
                for part in iterable.children:
                    found = self.infer(part)
                    if found != "I64":
                        raise mismatch("I64", found, part)
                item_type, root = "I64", None
            else:
                found = self.infer(iterable)
                if not found.startswith("Array<"):
                    raise mismatch("domain:iterable", found, iterable,
                                   primary=(iterable.start, iterable.end))
                item_type = found[6:-1]
                root = iterable.value if iterable.kind == "name" else None
            old = self.bindings.get(node.value)
            if root is not None:
                self.active_array_loan[root] = iterable
            self.bindings[node.value] = Binding(item_type, False, node.start + 4,
                                                node.start + 4 + len(node.value))
            for child in node.children[1].children:
                self.statement(child, loop_depth + 1, root)
            if old is None:
                del self.bindings[node.value]
            else:
                self.bindings[node.value] = old
            if root is not None:
                del self.active_array_loan[root]
            return
        if kind in ("if", "while"):
            condition = self.infer(node.children[0])
            if condition != "Bool":
                raise mismatch("Bool", condition, node.children[0])
            for block in node.children[1:]:
                for child in block.children:
                    self.statement(child, loop_depth + (1 if kind == "while" else 0), borrowed_root)
            return
        if kind in ("break", "continue"):
            if not loop_depth:
                subcause = kind + "_outside_loop"
                raise RuleViolation(Issue("NX-E1326", subcause, node.start, node.end - 1,
                                          "types", "types.check", {}))
            return
        if kind == "return":
            if node.children:
                self.infer(node.children[0])
            return
        if kind in ("expr", "block"):
            for child in node.children:
                if kind == "expr":
                    self.infer(child)
                else:
                    self.statement(child, loop_depth, borrowed_root)
            return
        raise ValueError(kind)

    def check(self) -> None:
        self.check_constants()
        for name, function in self.program.functions.items():
            self.bindings = {param: Binding(ty, mode == "edit", 0, 0)
                             for param, mode, ty in function.params}
            for statement in function.body:
                self.statement(statement)
            if function.return_type != "()" and not self.must_return(function.body):
                start, end = function.return_span
                raise RuleViolation(Issue("NX-E1317", None, function.end - 1,
                                          function.end, "ownership", "flow.return", {},
                                          (("expected_type", start, end),)))
            if function.contract == "no_alloc":
                for node in body_nodes(function.body):
                    if node.kind == "call" and node.children[0].kind == "name" and \
                            node.children[0].value in ("Push", "JoinArrays"):
                        callee = node.children[0]
                        a, b = function.contract_span
                        raise RuleViolation(Issue("NX-E0412", None, callee.start, callee.end,
                                                  "effects", "intrinsic.call",
                                                  {"effect_model": "f0-0005-sim-v1",
                                                   "effect_evidence": "known_effect",
                                                   "required_effect": "alloc",
                                                   "contract_origin": span(a, b)},
                                                  (("contract", a, b),),
                                                  cause=({"kind": "call",
                                                          "operation": "intrinsic.call",
                                                          "span": span(callee.start, callee.end),
                                                          "entity": {"kind": "intrinsic",
                                                                     "id": "prelude:" +
                                                                     ("join_arrays" if callee.value == "JoinArrays"
                                                                      else callee.value.lower())},
                                                          "evidence": "verified"},)))
            for binding in self.bindings.values():
                if binding.ty == "Array<?>":
                    raise RuleViolation(Issue("NX-E0103", None, 16, 18,
                                              "types", "types.infer", {}))

    def must_return(self, body: tuple[SourceNode, ...]) -> bool:
        for stmt in body:
            if stmt.kind == "return":
                return True
            if stmt.kind == "if" and self.must_return(stmt.children[1].children) and \
                    self.must_return(stmt.children[2].children):
                return True
            if stmt.kind == "while":
                condition = stmt.children[0]
                while condition.kind == "group":
                    condition = condition.children[0]
                if condition.kind == "bool" and condition.value and \
                        self.must_return(stmt.children[1].children) and \
                        not any(n.kind == "break" for n in body_nodes(stmt.children[1].children)):
                    return True
        return False

    def references(self, nodes: tuple[SourceNode, ...], visited: set[str]) -> set[str]:
        found: set[str] = set()
        for node in nodes:
            if node.kind == "name" and node.value in self.consts:
                found.add(node.value)
            if node.kind == "call" and node.children[0].kind == "name":
                name = node.children[0].value
                if name in self.program.functions and name not in visited:
                    visited.add(name)
                    found |= self.references(tuple(body_nodes(self.program.functions[name].body)), visited)
            found |= self.references(node.children, visited)
        return found

    def impure_path(self, node: SourceNode, visited: set[str]) -> list[SourceNode]:
        if node.kind == "call" and node.children[0].kind == "name":
            callee = node.children[0]
            if callee.value in ("Print", "Push", "Pop", "JoinArrays", "MockOpen"):
                return [callee]
            if callee.value in self.program.functions and callee.value not in visited:
                visited.add(callee.value)
                function = self.program.functions[callee.value]
                for child in body_nodes(function.body):
                    path = self.impure_path(child, visited)
                    if path:
                        return [callee, *path]
        for child in node.children:
            result = self.impure_path(child, visited)
            if result:
                return result
        return []

    def check_constants(self) -> None:
        for name, annotation, expr in self.program.constants:
            if annotation not in ("I64", "Bool", "()"):
                a, b = self.program.constant_type_spans[name]
                raise RuleViolation(Issue("NX-E0006", None, a, b,
                                          "types", "types.check",
                                          {"pending_scope": "later_adr"},
                                          feature_id="comptime"))
        graph = {name: self.references((expr,), set())
                 for name, _, expr in self.program.constants}
        for origin in graph:
            stack: list[tuple[str, list[str]]] = [(origin, [origin])]
            while stack:
                current, path = stack.pop()
                for following in sorted(graph[current]):
                    if following == origin:
                        a, b = self.program.constant_spans[origin]
                        cycle = path + [origin]
                        related = tuple(("dependency", *self.program.constant_spans[name])
                                        for name in path[1:])
                        raise RuleViolation(Issue("NX-E1323", "constant_cycle", a, b,
                                                  "types", "types.check",
                                                  {"cycle": cycle}, related))
                    if following not in path:
                        stack.append((following, path + [following]))
        vm = SourceVM(self.program)
        for name, _, expr in self.program.constants:
            path = self.impure_path(expr, set())
            if path:
                a, b = self.program.constant_spans[name]
                first = path[0]
                chain = []
                if len(path) > 1:
                    for item in path:
                        callee = item.value
                        if callee in self.program.functions:
                            function = self.program.functions[callee]
                            entity = {"kind": "function",
                                      "id": f"function:{function.name_span[0]}"}
                            operation = "function.call"
                        else:
                            entity = {"kind": "intrinsic",
                                      "id": "prelude:" + callee.lower()}
                            operation = "intrinsic.call"
                        chain.append({"kind": "call", "operation": operation,
                                      "span": span(item.start, item.end),
                                      "entity": entity, "evidence": "verified"})
                raise RuleViolation(Issue("NX-E1316", None, first.start, first.end,
                                          "effects", "const.check",
                                          {"reason": "impure_operation"},
                                          (("constant", a, b),), cause=tuple(chain)))
            try:
                vm.eval(expr)
            except ModelFault as fault:
                if fault.code.startswith("R"):
                    raise RuleViolation(Issue("NX-E1327", "arithmetic_fault",
                                              fault.start, fault.end, "lower",
                                              "const.evaluate", {"simulator_code": fault.code})) from fault
                raise


def check_source(source: str) -> Issue | None:
    try:
        program = parse_source(source)
        SourceChecker(program).check()
        try:
            SourceVM(program, limit=100_000).run()
        except ModelFault as fault:
            if fault.code.startswith("R"):
                return Issue(fault.code, None, fault.start, fault.end,
                             "runtime", fault.operation, {})
            raise
    except RuleViolation as failure:
        return failure.issue
    except SourceScopeFault as failure:
        return Issue("NX-E0006", None, failure.start, failure.end,
                     failure.stage, failure.operation,
                     {"pending_scope": "later_adr"}, feature_id=failure.feature_id)
    except SourceDuplicateConstant as failure:
        return Issue("NX-E0104", None, failure.token.start, failure.token.end,
                     "names", "names.declare", {"declaration_kind": "constant"},
                     (("previous_declaration", *failure.previous),))
    return None
