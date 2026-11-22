"""Audit raw expression spans in the deferred type solver.

`primary_span` chooses a diagnostic operand; other raw spans can serve solver
identity, lexical boundaries, a deferred result, or a different diagnostic
family. Group wrappers retain distinct keys. The three grouped primaries marked
`pendiente_decision` require a normative decision, not an auditor exemption.
A new raw use always needs explicit review.
"""

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tempfile


SOURCE = Path(__file__).resolve().parents[1] / "src/check/types.rs"
# This inventory is tied to the types.rs blob 6e358e9a3288506c59c623766bd36e1fdac0317b of the
# S5b.1 commit (previously reviewed blob b70f9c56f9175fe01007298b03458d5eb81c55f1,
# I07c-S6 plan §3.3, reconciled with F5-pila-r1). Only line numbers moved (+63/+74); classes and
# reasons are unchanged. The next cut that edits types.rs must re-reconcile it.
# Both the expression and its surrounding call/block must remain recognisable.
REVIEWED = {
    7082: ("e.span().clone()", "primary_unwrapped", "while let Expr::Group(inner, _) = e", "Group removed in primary_span"),
    8283: ("iterable.span().clone()", "pendiente_decision", "prepare_for_type", "for_iteration Array barrier; grouped primary undecided, observed repro"),
    9065: ("left.span(),", "operator", "checker.operator_span(", "range-outside-for primary is the operator token located between operands; operands only bound the search"),
    9066: ("right.span(),", "operator", "checker.operator_span(", "same operator_span call, right operand bound"),
    11304: ("inner.span().clone()", "identity", "active_if_span.replace", "scope of the current If, not a primary"),
    12252: ("e.span()", "deferred_result", "self.defer_nominal_domain(", "arithmetic nominal result; possible deferred primary"),
    12492: ("target.span().clone()", "pendiente_decision", "non_callable_target", "call target primary; grouped primary undecided, observed repro"),
    13065: ("e.span()", "deferred_result", "self.defer_nominal_domain(", "right pending arithmetic nominal result; possible deferred primary"),
    13175: ("e.span()", "deferred_result", "self.defer_nominal_domain(", "Bool nominal result; possible deferred primary"),
    13386: ("arm.pattern.span().clone()", "pattern_primary", "unreachable_arm", "NX-E1304 Pattern primary, no Expr::Group"),
    13391: ("pattern.span().clone()", "pattern_relation", "RelatedRole::Context", "preceding Pattern relation, no Expr::Group"),
    13566: ("pattern.span()", "quota_charge", "charge_solver(", "Pattern budget charge, possible NX-E0003 primary"),
    14804: ("target.span().clone()", "pendiente_decision", '"NX-E1111"', "closure target primary; grouped primary undecided, branch not reproduced"),
    14818: ("target.span()", "argument_target", "names::bind_arguments", "signature target for argument binding"),
}


def audit() -> None:
    source_lines = SOURCE.read_text(encoding="utf-8").splitlines()
    tests_begin = source_lines.index("mod quota_tests {") + 1
    classes = {kind: [] for kind in ("identity", "boundary", "operator", "internal_result",
                                    "primary_fallback", "test", "primary_unwrapped",
                                    "pendiente_decision", "deferred_result", "pattern_primary",
                                    "pattern_relation", "quota_charge", "argument_target")}
    seen = set()
    for number, line in enumerate(source_lines, 1):
        if ".span()" not in line:
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
    print("PENDIENTE DECISIÓN: 8283/12492 (repro), 14804 (sin repro); Flujo A, K1-d2 NO EVALUADO")
    print(f"total: {sum(map(len, classes.values()))}")


def self_test() -> None:
    with tempfile.TemporaryDirectory(prefix="i07c-span-") as folder:
        root = Path(folder)
        script = root / "scripts/i07c_span_audit.py"
        source = root / "src/check/types.rs"
        script.parent.mkdir(parents=True)
        source.parent.mkdir(parents=True)
        script.write_bytes(Path(__file__).read_bytes())
        original = SOURCE.read_text(encoding="utf-8")
        source.write_text(original, encoding="utf-8")
        good = subprocess.run([sys.executable, "-B", str(script)], capture_output=True, text=True)
        assert good.returncode == 0 and all(f"{n} " in good.stdout for n in REVIEWED), good.stderr
        source.write_text(original.replace("mod quota_tests {", "fn zz(e:&Expr)->Span{ e.span().clone() }\nmod quota_tests {", 1), encoding="utf-8")
        bad = subprocess.run([sys.executable, "-B", str(script)], capture_output=True, text=True)
        assert bad.returncode == 1 and "unreviewed raw span" in bad.stderr, (bad.stdout, bad.stderr)
    print("span auditor self-test: PASS (14 reviewed; novel raw use rejected)")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    self_test() if args.self_test else audit()
