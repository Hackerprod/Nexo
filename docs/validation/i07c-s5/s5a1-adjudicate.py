"""Enumerate and classify every S5a.1 differential, one row per changed source.

Classes (plan section 3 and 7):
  A  the candidate drops NX-E0103 diagnostics and the dropped ones are the `[]` of a `var`
     that a later assignment to the same name could fix (spurious E0103, sections 4.1 and 4.1.2.2);
     every other diagnostic of the base is kept, possibly displaced by V6.
  B  the NX-E0103 set is unchanged but other diagnostics change: only E1303/E0102 that depended on
     the retyping of earlier reads by a later assignment (4.1.2.2, 4.1.2.7), checked by the
     control `assignments removed`: base and candidate must agree once the late assignments
     are removed.
Any other change (outcome, exit, stderr, new E0103, new diagnostic code) is `UNEXPLAINED`.

Usage: python -B .../s5a1-adjudicate.py <diff.json> <base-debug.exe> <cand-debug.exe> <out.json>
"""

import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile


# Rows the structural rule cannot classify, adjudicated by hand against the ADR (reduced reproducers
# found with a delta reducer; see I07c-C-S5a1-informe.md). Each entry: class, clause, derivation.
MANUAL = {
    "repro:A_no_assignment": ("A", "plan S5 section 7 (A sin asignacion tardia); 4.1.2.4", "the compatible `aar` fixes the element of `r`; only the first later incompatible leaf `[r]` is reported"),
    "twin:F16": ("A", "plan S5 section 7 (corpus 26+1: ausencia del E0103 espurio); 4.1", "no constructor stays unresolved at close; the new conflicts are not asserted in tests"),
    "twin:F16:perm0": ("A", "idem F16", "assignment permutation of F16"),
    "twin:F16:perm1": ("A", "idem F16", "assignment permutation of F16"),
    "generated:1256": ("A", "4.1.2.4 (D-REF), 4.1.2.2", "reduced to `(if c then q[1] else (if x then (if x then [r] else r) else [1]))`: `[r]` fixes `q[1]`, the recursive leaf `r` is the first incompatible (its E1303 is omitted because `r`'s element keeps its own NX-E0103), `[1]` is a later incompatible leaf of the same union (one NX-E1303 per union) and `[r]` has a resolved element"),
    "generated:1322": ("A", "4.1.2.4, 4.1 (E0103 only if the element stays unresolved at close)", "reduced to `(if [r][0] then [r] else (if false[x] then q[1] else ar))`: the union equality `q[1] ~ [r]` fixes the element of `q`; only `r` keeps an unresolved element (NX-E0103 kept)"),
    "generated:1760": ("A", "4.1.2.2, 4.1.2.7 (no retroactive retyping), serialization omission rule", "reduced to `while q {} ... (if c then [r] else q[1]) ... q=[true]`: the union fixes the element of `q` to `Array<Array<a>>` with `a` pending (r keeps its NX-E0103), `q=[true]` is incompatible and cannot retype `q`, so the `while q` mismatch has an unserializable type and is omitted because `a` emits NX-E0103"),
    "generated:2447": ("A", "4.1.2.2, 4.1.2.4 (D-REF)", "reduced to `[(if p then r[1] else (if [r][0] then r else [1])),r];r=[r]`: the recursive leaf `r` is rejected and the compatible leaf `[1]` fixes the element to `Array<I64>`, so `r` has no NX-E0103"),
}


def compact(diagnostic):
    span = diagnostic["primary_span"]
    return [diagnostic["code"] + ("/" + diagnostic["subcause"] if diagnostic["subcause"] else ""),
            span["start"], span["end"], diagnostic["data"]]


def parse(output):
    document = json.loads(output[1])
    return output[0], document["outcome"]["kind"], [compact(d) for d in document["diagnostics"]]


def run(binary, source, profile="f0-0002-b-v2", verb="check"):
    with tempfile.TemporaryDirectory(prefix="s5a1-adj-") as directory:
        file = Path(directory) / "case.n"
        file.write_text(source, encoding="utf-8")
        process = subprocess.run((binary, verb, str(file), "--format", "json", "--profile", profile),
                                 capture_output=True, timeout=60)
        text = process.stdout.decode("utf-8").replace(str(file), "case.n")
        return parse((process.returncode, text))


def without_assignments(source):
    body = source[len("fn Main(){"):-1]
    while True:
        match = re.search(r";([a-z][a-z0-9]*=[^;{}]*;)$", body)
        if not match:
            break
        body = body[:match.start() + 1]
    return "fn Main(){" + body + "}"


def fixable_constructor(source, span):
    if source[span[0]:span[1]] != "[]":
        return False
    name = re.search(r"var ([a-z][a-z0-9]*)=$", source[:span[0]])
    return bool(name) and re.search(rf";{name.group(1)}=[^=]", source[span[1]:]) is not None


def main():
    diff_path, base, candidate, output = sys.argv[1:]
    report = json.loads(Path(diff_path).read_text(encoding="utf-8"))
    by_source = {}
    for diff in report["differences"]:
        by_source.setdefault(diff["source_id"], []).append(diff)
    rows, unexplained = {}, []
    for source_id, diffs in by_source.items():
        source = diffs[0]["source"]
        old, new = parse(diffs[0]["base"]), parse(diffs[0]["candidate"])
        uniform = all(parse(d["base"]) == old and parse(d["candidate"]) == new for d in diffs)
        old_e0103 = {(d[1], d[2]) for d in old[2] if d[0] == "NX-E0103"}
        new_e0103 = {(d[1], d[2]) for d in new[2] if d[0] == "NX-E0103"}
        dropped = sorted(old_e0103 - new_e0103)
        new_codes = {d[0] for d in new[2]} <= {d[0] for d in old[2]} | {"NX-E1303", "NX-E0102/type_mismatch"}
        same_outcome = old[:2] == new[:2]
        if dropped and not (new_e0103 - old_e0103) and all(fixable_constructor(source, s) for s in dropped):
            cls, clause = "A", "4.1 (E0103 solo si sigue irresuelto al cierre), 4.1.2.2"
        elif not dropped and old_e0103 == new_e0103:
            control = without_assignments(source)
            agree = control != source and run(base, control) == run(candidate, control)
            cls = "B" if agree else "UNEXPLAINED"
            clause = "4.1.2.2 y 4.1.2.7 (una asignacion rechazada no retipa lecturas anteriores)"
        else:
            cls, clause = "UNEXPLAINED", ""
        if not (uniform and same_outcome and new_codes):
            cls = "UNEXPLAINED"
        derivation = None
        if cls == "UNEXPLAINED" and source_id in MANUAL and uniform and same_outcome:
            cls, clause, derivation = MANUAL[source_id]
            cls += "-manual"
        row = {"source": source, "class": cls, "clause": clause, "derivation": derivation, "dropped_e0103": dropped,
               "base": old, "candidate": new, "invocations": len(diffs),
               "profiles": sorted({d["profile"] for d in diffs}), "uniform": uniform}
        rows[source_id] = row
        if cls == "UNEXPLAINED":
            unexplained.append(source_id)
    summary = {"changed_sources": len(rows), "by_class": {c: sum(r["class"] == c for r in rows.values())
                                                          for c in ("A", "B", "A-manual", "UNEXPLAINED")},
               "unexplained": unexplained, "totals": report["totals"],
               "binary_sha256": report["binary_sha256"], "manifest": report["manifest"]}
    Path(output).write_text(json.dumps({"summary": summary, "rows": rows}, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(summary), "sha256=" + hashlib.sha256(Path(output).read_bytes()).hexdigest())


if __name__ == "__main__":
    main()
