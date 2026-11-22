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
    191: ("value.span().start", "boundary", "stmt_span.start", "lower bound of the token search, not a primary"),
    795: ("self.enter(expr.span())", "identity", "self.enter(", "entry/quota key of the node, keeps the group"),
    1085: ("target.span().clone()", "relation", "partial_transfer", "Context relation names the whole call target"),
    1186: ("a.value.span().clone()", "unreachable_fallback", "duplicate_argument", "unlabelled duplicate primary, unreachable (plan section 2)"),
    1193: ("occupied[slot]", "relation", "label_span(a)", "occupied slot span, Context relation of a later duplicate"),
}
REVIEWED_EFFECTS = {
    1223: ("guard.span().clone()", "identity", "Frame::Guard(guard)", "match guard region used to contain witnesses, not a primary"),
}
# This inventory is tied to the types.rs of the I07c-S6-COR cut (derived from the S5b.1 inventory,
# I07c-S6 plan section 3.3). Classes: the sites that report an expression call the group-removal
# helper (`delegated_primary`, `expression_primary`, `argument_target_primary`); the rest keep a raw
# span for identity, an operator token, a deferred result, a Pattern or a budget charge. The next
# cut that edits types.rs must re-reconcile it. Both the expression and its surrounding call/block
# must remain recognisable; a reviewed line that loses its expression is rejected.
REVIEWED = {
    7078: ("crate::frontend::parse::expression_primary_span(e)", "delegated_primary", "fn primary_span(e: &Expr)", "Group removal is delegated to the parse.rs helper"),
    8279: ("Self::primary_span(iterable)", "expression_primary", "prepare_for_type", "for_iteration Array barrier reports the iterable without outer groups"),
    9061: ("left.span(),", "operator", "checker.operator_span(", "range-outside-for primary is the operator token located between operands; operands only bound the search"),
    9062: ("right.span(),", "operator", "checker.operator_span(", "same operator_span call, right operand bound"),
    11300: ("inner.span().clone()", "identity", "active_if_span.replace", "scope of the current If, not a primary"),
    12248: ("e.span()", "deferred_result", "self.defer_nominal_domain(", "arithmetic nominal result; possible deferred primary"),
    12484: ("Self::primary_span(target)", "expression_primary", "non_callable_target", "call target reported without outer groups; the declaration relation is unchanged"),
    13058: ("e.span()", "deferred_result", "self.defer_nominal_domain(", "right pending arithmetic nominal result; possible deferred primary"),
    13168: ("e.span()", "deferred_result", "self.defer_nominal_domain(", "Bool nominal result; possible deferred primary"),
    13379: ("arm.pattern.span().clone()", "pattern_primary", "unreachable_arm", "NX-E1304 Pattern primary, no Expr::Group"),
    13384: ("pattern.span().clone()", "pattern_relation", "RelatedRole::Context", "preceding Pattern relation, no Expr::Group"),
    13559: ("pattern.span()", "quota_charge", "charge_solver(", "Pattern budget charge, possible NX-E0003 primary"),
    14797: ("Self::primary_span(target)", "expression_primary", '"NX-E1111"', "closure target reported without outer groups; branch NO EVALUADO, see below"),
    14811: ("&Self::primary_span(target)", "argument_target_primary", "names::bind_arguments", "signature target for argument binding is the normalized target"),
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


def audit_plain(path: Path, reviewed: dict) -> int:
    """Every raw `.span()` outside the test module must be a reviewed site."""
    lines = path.read_text(encoding="utf-8").splitlines()
    marker = [i for i, line in enumerate(lines) if line.strip() == "mod tests {"]
    tests_begin = marker[0] + 1 if marker else len(lines) + 1
    seen = set()
    tests = 0
    for number, line in enumerate(lines, 1):
        if ".span()" not in line and number not in reviewed:
            continue
        if number in reviewed:
            expression, kind, context, reason = reviewed[number]
            neighborhood = "\n".join(lines[max(0, number - 45):number + 6])
            if expression not in line or context not in neighborhood:
                raise SystemExit(f"reviewed span changed at {path}:{number}: {line.strip()}")
            seen.add(number)
            print(f"{path.name} {kind}: {number} {expression} - {reason}")
        elif number >= tests_begin:
            tests += 1
        else:
            raise SystemExit(f"unreviewed raw span at {path}:{number}: {line.strip()}")
    if seen != reviewed.keys():
        raise SystemExit(f"reviewed sites missing in {path}: {reviewed.keys() - seen}")
    print(f"{path.name} reviewed: {len(seen)}/{len(reviewed)} (test lines: {tests})")
    return len(seen)


def audit() -> None:
    source_lines = SOURCE.read_text(encoding="utf-8").splitlines()
    check_helper(source_lines)
    tests_begin = source_lines.index("mod quota_tests {") + 1
    classes = {kind: [] for kind in ("identity", "boundary", "operator", "internal_result",
                                    "primary_fallback", "test", "delegated_primary",
                                    "expression_primary", "deferred_result", "pattern_primary",
                                    "pattern_relation", "quota_charge",
                                    "argument_target_primary")}
    seen = set()
    for number, line in enumerate(source_lines, 1):
        if ".span()" not in line and number not in REVIEWED:
            continue
        if number in REVIEWED:
            expression, kind, context, reason = REVIEWED[number]
            neighborhood = "\n".join(source_lines[max(0, number - 45):number + 6])
            if expression not in line or context not in neighborhood:
                raise SystemExit(f"reviewed span changed at {SOURCE}:{number}: {line.strip()}")
            seen.add(number)
            print(f"{kind}: {number} {expression} — {reason}")
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
    if seen != REVIEWED.keys():
        raise SystemExit(f"reviewed sites missing: {REVIEWED.keys() - seen}")
    for kind, lines in classes.items():
        print(f"{kind}: {len(lines)} lines={','.join(map(str, lines))}")
    print(f"types.rs reviewed: {len(seen)}/{len(REVIEWED)}")
    others = audit_plain(NAMES, REVIEWED_NAMES) + audit_plain(EFFECTS, REVIEWED_EFFECTS)
    total = len(seen) + others
    print(f"reviewed: {total}/{len(REVIEWED) + len(REVIEWED_NAMES) + len(REVIEWED_EFFECTS)}")
    print("NO EVALUADO solo la rama NX-E1111 de prepare_closure_call (sin borrador de closure); "
          "las demas rutas de primario agrupado las ejercita tests/i07c_group_primary.rs")
    print(f"total: {sum(map(len, classes.values()))}")


def self_test() -> None:
    def run(script: Path) -> subprocess.CompletedProcess:
        return subprocess.run([sys.executable, "-B", str(script)], capture_output=True, text=True)

    with tempfile.TemporaryDirectory(prefix="i07c-span-") as folder:
        root = Path(folder)
        script = root / "scripts/i07c_span_audit.py"
        source = root / "src/check/types.rs"
        helper = root / "src/frontend/parse.rs"
        names = root / "src/check/names.rs"
        effects = root / "src/ir/effects.rs"
        original_names = NAMES.read_text(encoding="utf-8")
        original_effects = EFFECTS.read_text(encoding="utf-8")
        for path in (script, source, helper, names, effects):
            path.parent.mkdir(parents=True, exist_ok=True)
        script.write_bytes(Path(__file__).read_bytes())
        original = SOURCE.read_text(encoding="utf-8")
        original_helper = HELPER.read_text(encoding="utf-8")

        def write(types_text: str, parse_text: str, names_text: str = original_names,
                  effects_text: str = original_effects) -> None:
            source.write_text(types_text, encoding="utf-8", newline="\n")
            helper.write_text(parse_text, encoding="utf-8", newline="\n")
            names.write_text(names_text, encoding="utf-8", newline="\n")
            effects.write_text(effects_text, encoding="utf-8", newline="\n")

        def revert(number: int, old: str, new: str) -> str:
            lines = original.splitlines(keepends=True)
            assert old in lines[number - 1], (number, old)
            lines[number - 1] = lines[number - 1].replace(old, new, 1)
            return "".join(lines)

        write(original, original_helper)
        good = run(script)
        assert good.returncode == 0 and all(f"{n} " in good.stdout for n in REVIEWED), good.stderr
        assert f"reviewed: {len(REVIEWED)}/{len(REVIEWED)}" in good.stdout, good.stdout
        write(original.replace("mod quota_tests {", "fn zz(e:&Expr)->Span{ e.span().clone() }\nmod quota_tests {", 1), original_helper)
        bad = run(script)
        assert bad.returncode == 1 and "unreviewed raw span" in bad.stderr, (bad.stdout, bad.stderr)
        # Each edited site, reverted on its own, must be rejected.
        loop = "    while let Expr::Group(inner, _) = current {\n        current = inner;\n    }\n"
        assert loop in original_helper
        reverts = [
            ("primary_span without delegation",
             revert(7078, "crate::frontend::parse::expression_primary_span(e)", "e.span().clone()"),
             original_helper, "must only delegate"),
            ("helper without its removal loop", original, original_helper.replace(loop, "", 1),
             "helper changed or missing"),
        ]
        for number, old, new in ((8279, "Self::primary_span(iterable)", "iterable.span().clone()"),
                                 (12484, "Self::primary_span(target)", "target.span().clone()"),
                                 (14797, "Self::primary_span(target)", "target.span().clone()"),
                                 (14811, "&Self::primary_span(target)", "target.span()")):
            reverts.append((f"site {number} back to a raw span", revert(number, old, new),
                            original_helper, "reviewed span changed"))
        for label, types_text, parse_text, message in reverts:
            write(types_text, parse_text)
            result = run(script)
            assert result.returncode == 1 and message in result.stderr, (label, result.stdout, result.stderr)
        # names.rs and effects.rs: a new raw use and the reversion of a reviewed site fail.
        fresh = "fn zz(e: &Expr) -> Span { e.span().clone() }\n"
        extra = [
            ("new raw span in names.rs", dict(names_text=original_names.replace("mod tests {", fresh + "mod tests {", 1)), "unreviewed raw span"),
            ("new raw span in effects.rs", dict(effects_text=original_effects.replace("mod tests {", fresh + "mod tests {", 1)), "unreviewed raw span"),
            ("names.rs site 1085 normalized", dict(names_text=original_names.replace("related(&mut d, RelatedRole::Context, target.span().clone());", "related(&mut d, RelatedRole::Context, expression_primary_span(target));", 1)), "reviewed span changed"),
            ("effects.rs guard site reverted", dict(effects_text=original_effects.replace("out.push(guard.span().clone());", "out.push(guard_span(guard));", 1)), "reviewed span changed"),
        ]
        for label, texts, message in extra:
            write(original, original_helper, **texts)
            result = run(script)
            assert result.returncode == 1 and message in result.stderr, (label, result.stdout, result.stderr)
        reverts = reverts + extra
    print(f"span auditor self-test: PASS ({len(REVIEWED) + len(REVIEWED_NAMES) + len(REVIEWED_EFFECTS)} "
          f"reviewed in 3 files; novel raw use and {len(reverts)} site reversions rejected)")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    self_test() if args.self_test else audit()
