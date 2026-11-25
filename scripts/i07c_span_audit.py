"""Audit raw expression spans in the deferred type solver.

`primary_span` chooses a diagnostic operand and only delegates to the single
group-removal helper in `src/frontend/parse.rs` (F0-0002 lagunas, "Primario de
una expresion bajo agrupaciones"); the sites that report an expression use that
helper. Other raw spans serve solver identity, lexical boundaries, a deferred
result, or a different diagnostic family; group wrappers retain distinct keys.
A new raw use always needs explicit review.
"""

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tempfile


SOURCE = Path(__file__).resolve().parents[1] / "src/check/types.rs"
HELPER = SOURCE.parents[1] / "frontend/parse.rs"
# names.rs and effects.rs have no raw-span class to classify by pattern: every raw `.span()` use
# outside their test modules is reviewed one by one (expression + context), and a new one fails.
NAMES = SOURCE.parent / "names.rs"
EFFECTS = SOURCE.parents[1] / "ir/effects.rs"
REVIEWED_NAMES = {
    ("binding_span", "value.span().start"): ("boundary", "stmt_span.start", "lower bound of the token search, not a primary"),
    ("walk", "self.enter(expr.span())"): ("identity", "self.enter(", "entry/quota key of the node, keeps the group"),
    ("bind_call_target", "target.span().clone()"): ("relation", "partial_transfer", "Context relation names the whole call target"),
    ("bind", "a.value.span().clone()"): ("unreachable_fallback", "duplicate_argument", "unlabelled duplicate primary, unreachable (plan section 2)"),
    ("bind", "occupied[slot] = Some(label_span(a).unwrap_or_else(|| a.value.span().clone()));"): ("relation", "label_span(a)", "occupied slot span, Context relation of a later duplicate"),
}
REVIEWED_EFFECTS = {
    ("scope_sites", "guard.span().clone()"): ("identity", "Frame::Guard(guard)", "match guard region used to contain witnesses, not a primary"),
}
# This inventory is indexed by (enclosing function, expression), with counts for repeated sites.
# It derives from the I07c-S6-COR inventory (I07c-S6 plan section 3.3).
# Classes: the sites that report an expression call the group-removal
# helper (`delegated_primary`, `expression_primary`, `argument_target_primary`); the rest keep a raw
# span for identity, an operator token, a deferred result, a Pattern or a budget charge. The next
# cut that edits types.rs must re-reconcile it. The function, expression, count and surrounding
# call/block must remain recognisable; moving a line does not change the review.
REVIEWED = {
    ("primary_span", "crate::frontend::parse::expression_primary_span(e)"): ("delegated_primary", "fn primary_span(e: &Expr)", "Group removal is delegated to the parse.rs helper"),
    ("prepare_for_type", "Self::primary_span(iterable)"): ("expression_primary", "for_iteration", "for_iteration Array barrier reports the iterable without outer groups", 3),
    ("binary_range_error", "left.span(),"): ("operator", "checker.operator_span(", "range-outside-for primary is the operator token located between operands; operands only bound the search"),
    ("binary_range_error", "right.span(),"): ("operator", "checker.operator_span(", "same operator_span call, right operand bound"),
    ("drive", "inner.span().clone()"): ("identity", "active_if_span.replace", "scope of the current If, not a primary"),
    ("expr_inner", "e.span(),"): ("deferred_result", "self.defer_nominal_domain(", "arithmetic nominal result; possible deferred primary"),
    ("expr_inner", "Self::primary_span(target),"): ("expression_primary", "non_callable_target", "call target reported without outer groups; the declaration relation is unchanged"),
    ("finish_deferred_binary_right", "e.span(),"): ("deferred_result", "self.defer_nominal_domain(", "nominal result; possible deferred primary", 2),
    ("finish_selection", "arm.pattern.span().clone()"): ("pattern_primary", "unreachable_arm", "NX-E1304 Pattern primary, no Expr::Group"),
    ("finish_selection", "pattern.span().clone()"): ("pattern_relation", "RelatedRole::Context", "preceding Pattern relation, no Expr::Group"),
    ("pattern_type", "pattern.span()"): ("quota_charge", "charge_solver(", "Pattern budget charge, possible NX-E0003 primary"),
    ("match_advance", "let span = expr.span();"): ("identity", "OccurrenceFlag::PartialNormal", "R6 match witness identity and containment, not a diagnostic primary"),
    ("prepare_closure_call", "Self::primary_span(target),"): ("expression_primary", '"NX-E1111"', "closure target reported without outer groups; branch NO EVALUADO, see below"),
    ("prepare_closure_call", "&Self::primary_span(target),"): ("argument_target_primary", "names::bind_arguments", "signature target for argument binding is the normalized target"),
}

# S6-COR-2: lowering (`lower.rs`, `lower_match.rs`) has ~100 raw `.span()` uses, most of them IR,
# identity or relation spans that keep the whole group, so it is not reviewed line by line. The
# baseline is frozen by (file, site anchor, expression) -> expected count, where the site anchor is
# the enclosing function and the innermost enclosing match arm (no line numbers). A new expression,
# a larger or a smaller count fails: the change reconciles the baseline with a reason. Each solution
# of the cut removes its own accesses from the baseline and adds its positive anchor in the same
# commit (GROUP_SITES): the site must call `expression_primary_span`/`expression_primary`.
LOWER = SOURCE.parents[1] / "ir/lower.rs"
LOWER_MATCH = SOURCE.parents[1] / "ir/lower_match.rs"
RAW_SPAN = re.compile(r"[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*\.span\(\)(?:\.(?:clone\(\)|start|end|file))*")
LOWERING_BASELINE = {
    "lower.rs": {
        (('f5_capabilities', 'Statement::Assign { => {'), 'place.span().start'): (2, 'identity: ordering key of the L08 anchor candidates; the reported span is parse::expression_primary_span(place)'),
        (('resolved_expr', '-'), 'expr.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('resolved_expr', '-'), 'expr.span().end'): (1, 'identity: key or search bound, not a primary'),
        (('finish_if_expression', '-'), 'expr.span()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('begin_for_range', '-'), 'iterable.span()'): (4, 'for: unreachable today (NX-E0006/NX-E0002 first), debt in Pending.md'),
        (('begin_for_range', '-'), 'iterable.span().clone()'): (5, 'for: unreachable today (NX-E0006/NX-E0002 first), debt in Pending.md'),
        (('begin_for_array', '-'), 'iterable.span().clone()'): (15, 'for: unreachable today (NX-E0006/NX-E0002 first), debt in Pending.md'),
        (('begin_for_array', '-'), 'iterable.span().start'): (2, 'identity: key or search bound, not a primary'),
        (('begin_for_array', '-'), 'iterable.span()'): (2, 'for: unreachable today (NX-E0006/NX-E0002 first), debt in Pending.md'),
        (('begin_for_after_iterable', '-'), 'iterable.span().clone()'): (1, 'for: unreachable today (NX-E0006/NX-E0002 first), debt in Pending.md'),
        (('check_place_access', '-'), 'root.span().clone()'): (2, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('named_closure_receiver', '-'), 'target.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('named_closure_receiver', '-'), 'target.span().clone()'): (5, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Some(Task::WhileAfterCondition(_)) => {'), 'state.condition.span().clone()'): (3, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Some(Task::AssignAfterValue(_)) => {'), 'state.place.span().clone()'): (4, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Some(Task::AssignAfterValue(_)) => {'), 'state.root.span().clone()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Some(Task::AssignAfterValue(_)) => {'), 'state.place.span().file.clone()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Some(Task::AssignAfterValue(_)) => {'), 'state.place.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Some(Task::AssignAfterValue(_)) => {'), 'state.value.span().end'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Ok(left_type) => {'), 'state.left.span().clone()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Ok(left_type) => {'), 'state.left.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Ok(()) => {'), 'state.expr.span().clone()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Task::AssignEnter(statement) => {'), 'root.span().clone()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Task::AssignEnter(statement) => {'), 'root.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Task::AssignEnter(statement) => {'), 'root.span().end'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Task::AssignNextSelector(mut state) => {'), 'state.place.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Task::AssignNextSelector(mut state) => {'), 'state.place.span().clone()'): (5, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Task::ArgumentEnter { => {'), 'root.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Task::ArgumentEnter { => {'), 'root.span().end'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Task::ArgumentEnter { => {'), 'root.span().clone()'): (2, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Task::ArgumentPlaceNext(mut state) => {'), 'state.arg.value.span().clone()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Statement::Let { => {'), 'value.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Ok(inferred) => {'), 'target.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Ok(inferred) => {'), 'target.span().end'): (1, 'identity: key or search bound, not a primary'),
        (('run_lower_tasks', 'Ok(local) => {'), 'root.span().clone()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Task::OtherCallEnter { => {'), 'target.span().clone()'): (2, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Err(error) => {'), 'target.span()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('run_lower_tasks', 'Task::IfElse { => {'), 'expr.span().clone()'): (1, 'IR, relation or frontier span: the addendum keeps the whole group'),
        (('prepare_expr', 'Expr::Call { target, span, .. } => {'), 'callee.span().clone()'): (1, 'S5: the callee is already the group-free primary of the call target'),
        (('prepare_expr', 'Expr::Unary { => {'), 'value.span().end'): (1, 'identity: key or search bound, not a primary'),
    },
    "lower_match.rs": {
        (('guard_violations', 'parse::Expr::Unary { mode, value, span } => {'), 'value.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('guard_violations', 'parse::Expr::Unary { mode, value, span } => {'), 'value.span().end'): (1, 'identity: key or search bound, not a primary'),
        (('guard_violations', 'parse::Expr::Call { target, args, .. } => {'), 'callee.span().clone()'): (1, 'S7: the callee is already the group-free primary of the call target'),
        (('begin_match_selection', 'Some(Mode::Consume) => {'), 'selection.scrutinee.span().clone()'): (1, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
        (('begin_match_selection', 'Some(Mode::Inout) => {'), 'selection.scrutinee.span().clone()'): (3, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
        (('begin_match_selection_after_selected', '-'), 'selection.scrutinee.span().clone()'): (5, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
        (('begin_match_arm', '-'), 'arm.pattern.span().clone()'): (1, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
        (('begin_match_arm', '-'), 'guard.span().clone()'): (1, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
        (('finish_match_guard', '-'), 'guard.span().clone()'): (1, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
        (('match_condition', '-'), 'pattern.span().clone()'): (2, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
        (('match_tests', 'Pattern::Constructor { name, args, .. } => {'), 'current_pattern.span().clone()'): (1, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
        (('match_tests', '-'), 'current_pattern.span().clone()'): (1, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
        (('pattern_permission_error', '-'), 'selection.scrutinee.span().start'): (1, 'identity: key or search bound, not a primary'),
        (('pattern_permission_error', '-'), 'selection.scrutinee.span().clone()'): (2, 'match IR/relation/frontier span, already covered or keeps the whole construct'),
    },
}
# id -> (file, regex with a named group `site` holding the call, raw access the site would revert to)
GROUP_SITES = {
    "S1": ("lower.rs", r"WorkEvent::ExpressionStatement,\s*(?P<site>parse::expression_primary_span\(value\)),", "value.span().clone()"),
    "S2": ("lower.rs", r"WorkEvent::WhileCondition,\s*(?P<site>parse::expression_primary_span\(condition\)),", "condition.span().clone()"),
    "S3": ("lower.rs", r"WorkEvent::Assignment,\s*(?P<site>parse::expression_primary_span\(place\)),", "place.span().clone()"),
    "S4": ("lower.rs", r"WorkEvent::ValueStatement,\s*value_tail\.marker\.cloned\(\)\.unwrap_or_else\(\|\| \{\s*(?P<site>parse::expression_primary_span\(value_tail\.value\))\s*\}\)", "value_tail.value.span().clone()"),
    "S4 mirror": ("lower.rs", r"WorkEvent::ValueStatement,\s*marker\s*\.clone\(\)\s*\.unwrap_or_else\(\|\| (?P<site>parse::expression_primary_span\(value\))\)", "value.span().clone()"),
    "S6": ("lower_match.rs", r'"guard_mutation",\s*(?P<site>parse::expression_primary_span\(place\))\)', "place.span().clone()"),
    "S5": ("lower.rs", r"let callee = (?P<site>parse::expression_primary\(target\));\s*if matches!\(callee, Expr::Name\(\.\.\)\)", "target"),
    "S7": ("lower_match.rs", r"let callee = (?P<site>parse::expression_primary\(target\));\s*if matches!\(callee, parse::Expr::Name\(name, _\) if name == \"Drop\"\)", "target"),
}


def check_helper(source_lines: list[str]) -> None:
    """`primary_span` must only delegate, and parse.rs must own the single removal loop."""
    header = "fn primary_span(e: &Expr) -> Span {"
    starts = [i for i, line in enumerate(source_lines) if line.strip() == header]
    if len(starts) != 1:
        raise SystemExit(f"primary_span must exist once in {SOURCE}")
    body = []
    for line in source_lines[starts[0] + 1:]:
        if line.strip() == "}":
            break
        body.append(line.strip())
    if body != ["crate::frontend::parse::expression_primary_span(e)"]:
        raise SystemExit(f"primary_span must only delegate to the parse.rs helper: {body}")
    helper = HELPER.read_text(encoding="utf-8")
    loop = re.search(r"pub\(crate\) fn expression_primary\(expr: &Expr\) -> &Expr \{\n"
                     r"    let mut current = expr;\n"
                     r"    while let Expr::Group\(inner, _\) = current \{\n"
                     r"        current = inner;\n    \}\n    current\n\}\n", helper)
    span = re.search(r"pub\(crate\) fn expression_primary_span\(expr: &Expr\) -> Span \{\n"
                     r"    expression_primary\(expr\)\.span\(\)\.clone\(\)\n\}\n", helper)
    if not loop or not span:
        raise SystemExit(f"group-removal helper changed or missing in {HELPER}")


def test_lines(text: str) -> set[int]:
    """1-based lines of every `#[cfg(test)]` item (module, function, ...), delimited by its braces.

    A small Rust scanner skips comments, string, raw string and character literals, so braces inside
    them do not move the end of an item; anything outside these items is product code.
    """
    n = len(text)
    line_of = [1] * (n + 1)
    for i, ch in enumerate(text):
        line_of[i + 1] = line_of[i] + (ch == "\n")

    def skip(i: int) -> int:
        """Index after the comment/literal starting at `i`, or `i` if none starts there."""
        two = text[i:i + 2]
        if two == "//":
            j = text.find("\n", i)
            return n if j < 0 else j
        if two == "/*":
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            return j
        m = re.compile(r'b?r(#*)"').match(text, i)
        if m:
            end = text.find('"' + m.group(1), m.end())
            return n if end < 0 else end + 1 + len(m.group(1))
        if text[i] == '"' or text.startswith('b"', i):
            j = i + (2 if text[i] == "b" else 1)
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            return j + 1
        if text[i] == "'":
            if text.startswith("'\\", i):
                j = text.find("'", i + 3)
                return n if j < 0 else j + 1
            if i + 2 < n and text[i + 2] == "'":
                return i + 3
        return i

    marks = []  # (start index, end index) of each cfg(test) item
    i = 0
    stack = []  # indices of open braces
    pending = []  # (attribute index) waiting for the item's opening brace
    open_of = {}  # brace index -> attribute index of a cfg(test) item
    group = 0  # depth of parentheses and brackets (a `;` inside `[u8; 4]` does not end an item)
    while i < n:
        j = skip(i)
        if j != i:
            i = j
            continue
        if text.startswith("#[cfg(test)]", i):
            pending.append(i)
            i += len("#[cfg(test)]")
            continue
        ch = text[i]
        if ch == "{":
            if pending:
                open_of[i] = pending.pop()
            stack.append(i)
        elif ch == "}":
            start = stack.pop()
            if start in open_of:
                marks.append((open_of[start], i))
        elif ch in "([":
            group += 1
        elif ch in ")]":
            group -= 1
        elif ch == ";" and pending and group == 0 and not stack_open_after(pending[-1], stack, i):
            marks.append((pending.pop(), i))
        i += 1
    lines = set()
    for a, b in marks:
        lines.update(range(line_of[a], line_of[b] + 1))
    return lines


def stack_open_after(attribute: int, stack: list, index: int) -> bool:
    """True if a brace opened after the attribute is still open (the `;` is inside the item)."""
    return any(opened > attribute for opened in stack)


def audit_plain(path: Path, reviewed: dict) -> int:
    """Every raw `.span()` outside the cfg(test) items must be a reviewed site."""
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    tests_zone = test_lines(text)
    seen = {}
    tests = 0
    for number, line in enumerate(lines, 1):
        function = site_anchor(lines, number - 1)[0]
        neighborhood = "\n".join(lines[max(0, number - 45):number + 6])
        candidates = [key for key, row in reviewed.items()
                      if key[0] == function and key[1] in line and row[1] in neighborhood]
        key = max(candidates, key=lambda item: len(item[1])) if candidates else None
        if ".span()" not in line and key is None:
            continue
        if key is not None:
            kind, context, reason, *_ = reviewed[key]
            if line.count(".span()") != key[1].count(".span()"):
                raise SystemExit(f"unreviewed raw span at {path}:{number}: {line.strip()}")
            seen[key] = seen.get(key, 0) + 1
            print(f"{path.name} {kind}: {function} {key[1]} - {reason}")
        elif number in tests_zone:
            tests += 1
        else:
            raise SystemExit(f"unreviewed raw span at {path}:{number}: {line.strip()}")
    expected = {key: row[3] if len(row) > 3 else 1 for key, row in reviewed.items()}
    if seen != expected:
        raise SystemExit(f"reviewed sites missing or duplicated in {path}: {seen} != {expected}")
    print(f"{path.name} reviewed: {sum(seen.values())}/{sum(expected.values())} (test lines: {tests})")
    return sum(seen.values())


def _indent(line: str) -> int:
    return len(line) - len(line.lstrip())


def site_anchor(lines: list[str], index: int) -> tuple[str, str]:
    """(enclosing function, innermost enclosing match arm) of the line at `index`, by indentation."""
    need = _indent(lines[index])
    function = arm = "-"
    for j in range(index - 1, -1, -1):
        text = lines[j].strip()
        if not text or _indent(lines[j]) >= need:
            continue
        need = _indent(lines[j])
        head = text
        if text.startswith(")") or text.startswith("}"):
            k = j - 1
            while k >= 0 and (_indent(lines[k]) != need or not lines[k].strip()):
                k -= 1
            head = lines[k].strip() if k >= 0 else text
            if text.startswith("}") and text.endswith("{") and arm == "-" and "=>" in text:
                arm = head + " => {"
        found = re.match(r"(?:pub(?:\([a-z]+\))?\s+)?fn (\w+)", head)
        if found:
            function = found.group(1)
            break
        if arm == "-" and "=>" in text and text.endswith("{") and not text.startswith("}"):
            arm = text
        if need == 0:
            break
    return function, arm


def lowering_sites(text: str) -> dict:
    """Raw `.span()` accesses of product code by ((function, arm), expression) -> count."""
    lines = text.splitlines()
    tests_zone = test_lines(text)
    sites: dict = {}
    for number, line in enumerate(lines, 1):
        if ".span()" not in line or number in tests_zone:
            continue
        found = RAW_SPAN.findall(line)
        if len(found) != line.count(".span()"):
            raise SystemExit(f"unparsed raw span at line {number}: {line.strip()}")
        anchor = site_anchor(lines, number - 1)
        for expression in found:
            sites[(anchor, expression)] = sites.get((anchor, expression), 0) + 1
    return sites


def check_baseline(name: str, text: str, baseline: dict) -> int:
    sites = lowering_sites(text)
    for key, count in sites.items():
        if key not in baseline:
            raise SystemExit(f"unreviewed raw span (new expression) in {name}: {key[1]} at {key[0]}")
        if count > baseline[key][0]:
            raise SystemExit(f"unreviewed raw span (larger count) in {name}: {key[1]} at {key[0]}: {count} > {baseline[key][0]}")
    for key, (count, _reason) in baseline.items():
        if sites.get(key, 0) != count:
            raise SystemExit(f"stale span baseline in {name}: {key[1]} at {key[0]}: {sites.get(key, 0)} != {count}")
    return sum(sites.values())


def check_group_sites(texts: dict, sites: dict) -> None:
    for ident, (name, pattern, _raw) in sites.items():
        if not re.search(pattern, texts[name]):
            raise SystemExit(f"group site {ident} in {name} does not use the group-removal helper")


def audit_lowering() -> None:
    texts = {path.name: path.read_text(encoding="utf-8") for path in (LOWER, LOWER_MATCH)}
    total = 0
    for name, text in texts.items():
        count = check_baseline(name, text, LOWERING_BASELINE[name])
        total += count
        print(f"{name} baseline: {count}/{count} raw spans")
    check_group_sites(texts, GROUP_SITES)
    print(f"group sites: {len(GROUP_SITES)}/{len(GROUP_SITES)}")
    print("lowering span baseline: PASS")


def audit() -> None:
    source_lines = SOURCE.read_text(encoding="utf-8").splitlines()
    check_helper(source_lines)
    tests_begin = source_lines.index("mod quota_tests {") + 1
    classes = {kind: [] for kind in ("identity", "boundary", "operator", "internal_result",
                                    "primary_fallback", "test", "delegated_primary",
                                    "expression_primary", "deferred_result", "pattern_primary",
                                    "pattern_relation", "quota_charge",
                                    "argument_target_primary")}
    seen = {}
    for number, line in enumerate(source_lines, 1):
        function = site_anchor(source_lines, number - 1)[0]
        neighborhood = "\n".join(source_lines[max(0, number - 45):number + 6])
        candidates = [key for key, row in REVIEWED.items()
                      if key[0] == function and key[1] in line and row[1] in neighborhood]
        key = max(candidates, key=lambda item: len(item[1])) if candidates else None
        if ".span()" not in line and key is None:
            continue
        if key is not None:
            kind, context, reason, *_ = REVIEWED[key]
            if line.count(".span()") != key[1].count(".span()"):
                raise SystemExit(f"unreviewed raw span at {SOURCE}:{number}: {line.strip()}")
            seen[key] = seen.get(key, 0) + 1
            print(f"{kind}: {function} {key[1]} — {reason}")
        elif number >= tests_begin:
            kind = "test"
        elif "span_key(" in line:
            kind = "identity"
        elif "operator_span(" in line:
            kind = "operator"
        elif "active_if_span.replace(e.span().clone())" in line:
            kind = "identity"
        elif re.search(r"\.span\(\)\.(start|end)", line):
            kind = "boundary"
        elif re.search(r"(?:result: e|_ => e)\.span\(\)\.clone\(\)", line):
            kind = "internal_result" if "result:" in line else "primary_fallback"
        elif "e.span()," in line and "self.defer_domain_result(" in "\n".join(source_lines[number - 10:number]):
            kind = "identity"
        else:
            raise SystemExit(f"unreviewed raw span at {SOURCE}:{number}: {line.strip()}")
        classes[kind].append(number)
    expected = {key: row[3] if len(row) > 3 else 1 for key, row in REVIEWED.items()}
    if seen != expected:
        raise SystemExit(f"reviewed sites missing or duplicated: {seen} != {expected}")
    for kind, lines in classes.items():
        print(f"{kind}: {len(lines)} sites")
    count = sum(seen.values())
    print(f"types.rs reviewed: {count}/{sum(expected.values())}")
    others = audit_plain(NAMES, REVIEWED_NAMES) + audit_plain(EFFECTS, REVIEWED_EFFECTS)
    total = count + others
    print(f"reviewed: {total}/{total}")
    audit_lowering()
    print("NO EVALUADO solo la rama NX-E1111 de prepare_closure_call (sin borrador de closure); "
          "las demas rutas de primario agrupado las ejercita tests/i07c_group_primary.rs")
    print(f"total: {sum(map(len, classes.values()))}")


def self_test() -> None:
    """Exercise the function/expression index, including a planted raw span."""
    def run(script: Path) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, "-B", str(script)], capture_output=True, text=True)

    with tempfile.TemporaryDirectory(prefix="i07c-span-") as folder:
        root = Path(folder)
        copies = {
            "scripts/i07c_span_audit.py": Path(__file__),
            "src/check/types.rs": SOURCE,
            "src/frontend/parse.rs": HELPER,
            "src/check/names.rs": NAMES,
            "src/ir/effects.rs": EFFECTS,
            "src/ir/lower.rs": LOWER,
            "src/ir/lower_match.rs": LOWER_MATCH,
        }
        originals = {}
        for relative, original in copies.items():
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            content = original.read_text(encoding="utf-8")
            originals[relative] = content
            target.write_text(content, encoding="utf-8", newline="\n")
        script = root / "scripts/i07c_span_audit.py"

        def check(relative: str, old: str, new: str, expected: str) -> None:
            original = originals[relative]
            assert old in original, (relative, old)
            target = root / relative
            target.write_text(original.replace(old, new, 1), encoding="utf-8", newline="\n")
            result = run(script)
            assert result.returncode == 1 and expected in result.stderr, (
                relative, old, result.stdout, result.stderr)
            target.write_text(original, encoding="utf-8", newline="\n")

        good = run(script)
        assert good.returncode == 0 and "reviewed: 23/23" in good.stdout, (
            good.stdout, good.stderr)

        # Moving all historical lines preserves the keyed inventory.
        types = root / "src/check/types.rs"
        types.write_text("\n" * 9 + originals["src/check/types.rs"], encoding="utf-8", newline="\n")
        moved = run(script)
        assert moved.returncode == 0, (moved.stdout, moved.stderr)
        types.write_text(originals["src/check/types.rs"], encoding="utf-8", newline="\n")

        # The planted span is not listed by function/expression and must fail.
        check("src/check/types.rs", "mod quota_tests {",
              "fn planted(e: &Expr) -> Span { e.span().clone() }\nmod quota_tests {",
              "unreviewed raw span")
        check("src/check/names.rs", "fn bind_call_target(",
              "fn planted(e: &Expr) -> Span { e.span().clone() }\nfn bind_call_target(",
              "unreviewed raw span")
        check("src/ir/effects.rs", "fn find_witness(",
              "fn planted(e: &Expr) -> Span { e.span().clone() }\nfn find_witness(",
              "unreviewed raw span")

        # A reviewed access cannot silently change or multiply.
        check("src/check/types.rs",
              "crate::frontend::parse::expression_primary_span(e)",
              "e.span().clone()", "must only delegate")
        check("src/check/types.rs",
              "Self::primary_span(iterable)",
              "iterable.span().clone()", "unreviewed raw span")
        check("src/check/names.rs",
              "related(&mut d, RelatedRole::Context, target.span().clone());",
              "related(&mut d, RelatedRole::Context, expression_primary_span(target));",
              "reviewed sites missing or duplicated")
        check("src/ir/effects.rs",
              "out.push(guard.span().clone());",
              "out.push(guard_span(guard));",
              "reviewed sites missing or duplicated")

        # Lowering inventory and its positive group-removal anchors remain guarded.
        check("src/ir/lower.rs", "fn resolved_expr(",
              "fn planted(e: &Expr) -> Span { e.span().clone() }\nfn resolved_expr(",
              "unreviewed raw span")
        for ident, (name, pattern, raw) in GROUP_SITES.items():
            relative = "src/ir/" + name
            original = originals[relative]
            found = re.search(pattern, original)
            assert found, ident
            changed = original[:found.start("site")] + raw + original[found.end("site"):]
            target = root / relative
            target.write_text(changed, encoding="utf-8", newline="\n")
            result = run(script)
            assert result.returncode == 1, (ident, result.stdout, result.stderr)
            target.write_text(original, encoding="utf-8", newline="\n")
    print("span auditor self-test: PASS (function/expression index; planted raw span rejected)")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    self_test() if args.self_test else audit()
