#!/usr/bin/env python3
"""Complementary D7 cross gate; never changes the frozen candidate_gate.py.

The causal witness is a recursive source equality and a same-length counterfactual
in which only that equality is blanked.  A constructor C must survive in the
counterfactual.  Its obligation belongs to an earlier, compatible equality;
the later recursive equality cannot consume it (F0-0002 4.1.2.2, 4.1.2.5, 13.9).
The rule covers added, displaced, and absorbed E0103 diagnostics alike.
"""

import argparse
import collections
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import candidate_gate as gate  # frozen SHA is checked below
from audit_families import source
from verify_corpora import FILE

ASSIGN = re.compile(r"\b([A-Za-z_]\w*)(\[0\])?\s*=\s*(\[)?([A-Za-z_]\w*)(\])?\s*;")
ARRAY_EDGE = re.compile(r"\b(?:var\s+|let\s+)?([A-Za-z_]\w*)\s*=\s*\[([A-Za-z_]\w*)\]\s*;")
FROZEN = "48df7fb62c839a165fd99b40af2e626385e70476b2def09d7422cda600f1281c"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def primary(d):
    s = d["primary_span"]
    return s["start"], s["end"]


def is_constructor(src, d):
    a, b = primary(d)
    return d["code"] == "NX-E0103" and src[a:a + 1] == "[" and src[b - 1:b] == "]" and not (
        a and (src[a - 1].isalnum() or src[a - 1] in "_])"))


def edges_before(src, end):
    graph = collections.defaultdict(set)
    for m in ARRAY_EDGE.finditer(src[:end]):
        graph[m[1]].add(m[2])
    return graph


def reaches(graph, start, goal):
    todo, seen = [start], set()
    while todo:
        node = todo.pop()
        if node == goal:
            return True
        if node not in seen:
            seen.add(node)
            todo.extend(graph[node] - seen)
    return False


def recursive_sites(src):
    """Syntactic proof for simple equalities in the frozen corpus grammar.

    Unknown shapes are never silently declared D7.  A C4 candidate changing an
    unknown unmodeled row remains a failure of the frozen cross gate.
    """
    for m in ASSIGN.finditer(src):
        left, index, opening, right, closing = m.groups()
        if bool(opening) != bool(closing):
            continue
        if src[max(0, m.start() - 4):m.start()].endswith(("var ", "let ")):
            continue
        graph = edges_before(src, m.start())
        if index and left == right:
            pass
        elif opening and not reaches(graph, right, left):
            continue
        elif not opening and not index and not (
            reaches(graph, left, right) and left != right or
            reaches(graph, right, left) and left != right
        ):
            continue
        elif index:
            continue
        yield m


def counterfactual(src, site):
    return src[:site.start()] + " " * (site.end() - site.start()) + src[site.end():]


def normalized(diags):
    return [gate.canonical(x) for x in diags]


def witness(src, main, without, site):
    if main["outcome"]["kind"] != "rejected" or without["outcome"]["kind"] != "rejected":
        return None
    prior_c = [d for d in without["diagnostics"] if is_constructor(src, d) and primary(d)[0] < site.start()]
    if not prior_c:
        return None
    # The counterfactual may retain unrelated later constructor duties.  Every
    # E0103 must still name a real Array constructor, and non-C errors must agree.
    all_c = [d for d in without["diagnostics"] if d["code"] == "NX-E0103"]
    if any(not is_constructor(src, d) for d in all_c):
        return None
    other_main = normalized([d for d in main["diagnostics"] if d["code"] != "NX-E0103"])
    other_without = normalized([d for d in without["diagnostics"] if d["code"] != "NX-E0103"])
    if other_main != other_without:
        return None
    actual_c = normalized([d for d in main["diagnostics"] if d["code"] == "NX-E0103"])
    expected_c = normalized(all_c)
    if actual_c == expected_c:
        return None
    expected = normalized(without["diagnostics"])
    # The equality's rejection must not change unrelated diagnostics.  The
    # surviving constructor diagnostics are ordered by the ADR's 13.9 rule.
    expected.sort(key=lambda d: (primary(d)[0], primary(d)[1], d["code"], d.get("subcause") or ""))
    return expected


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--main-exe", type=Path, required=True)
    p.add_argument("--candidate-exe", type=Path, required=True)
    p.add_argument("--case", type=Path, required=True)
    p.add_argument("--commit", choices=[f"C{i}" for i in range(1, 7)], required=True)
    p.add_argument("--cross-report", type=Path, required=True, help="complete report produced by frozen candidate_gate.py")
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args()
    assert digest((HERE / "candidate_gate.py").read_bytes()) == FROZEN
    contract = json.loads((HERE / "r9-commit-gates.json").read_text(encoding="utf-8"))
    assert digest(Path(__file__).read_bytes()) == contract["d7_classifier_sha256"]
    cross = json.loads(a.cross_report.read_text(encoding="utf-8"))
    assert cross["complete"] and cross["commit"] == a.commit
    assert cross["classifier_sha256"] == FROZEN
    assert cross["main_sha256"] == digest(a.main_exe.read_bytes())
    assert cross["candidate_sha256"] == digest(a.candidate_exe.read_bytes())
    definitions = json.loads((HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["corpora"]
    assert len(definitions) == 20 and sum(e["rows"] for e in definitions) == 151188
    failures = []
    members = {}
    main_cache, candidate_cache, without_cache = {}, {}, {}
    for entry in definitions:
        ident, profile = entry["id"], entry["profile"]
        corpus = HERE / FILE[ident.split("@")[0]]
        assert digest(corpus.read_bytes()) == entry["sha256"]
        rows = json.loads(corpus.read_text(encoding="utf-8"))
        if isinstance(rows, dict):
            rows = rows["progs"]
        assert len(rows) == entry["rows"]
        for i, item in enumerate(rows):
            src = source(item)
            for site in recursive_sites(src):
                key = profile, src
                if key not in main_cache:
                    main_cache[key] = gate.execute(a.main_exe, src, profile, a.case)
                altered = counterfactual(src, site)
                alt_key = profile, altered
                if alt_key not in without_cache:
                    without_cache[alt_key] = gate.execute(a.main_exe, altered, profile, a.case)
                expected = witness(src, main_cache[key], without_cache[alt_key], site)
                if expected is None:
                    continue
                row_id = f"{ident}:{i}"
                if row_id in members and members[row_id]["expected_adr_diagnostics"] != expected:
                    failures.append(row_id + "=ambiguous-causal-witness")
                    continue
                if key not in candidate_cache:
                    candidate_cache[key] = gate.execute(a.candidate_exe, src, profile, a.case)
                candidate = candidate_cache[key]
                actual = normalized(candidate["diagnostics"])
                if a.commit in ("C1", "C2", "C3"):
                    if candidate != main_cache[key]:
                        failures.append(row_id + "=must-conserve-main")
                elif candidate["outcome"]["kind"] != "rejected" or actual != expected:
                    failures.append(row_id + "=candidate-not-ADR")
                members[row_id] = {
                    "source": src, "source_sha256": digest(src.encode("utf-8")),
                    "row_index": i, "profile": profile,
                    "corpus_sha256": entry["sha256"], "equality": [site.start(), site.end()],
                    "prior_constructor_primaries": [list(primary(d)) for d in without_cache[alt_key]["diagnostics"]
                                                    if is_constructor(src, d) and primary(d)[0] < site.start()],
                    "main_code_primary": [[d["code"], list(primary(d))] for d in main_cache[key]["diagnostics"]],
                    "expected_adr_diagnostics": expected,
                    "clause": "F0-0002 §§4.1.2.2, 4.1.2.5, 13.9",
                }
    manual = contract["d7_declared_main_differences"]["manual_witness"]
    src = manual["source"]
    assert digest(src.encode("ascii")) == manual["source_sha256"] and len(src) == 78
    sites = list(recursive_sites(src))
    assert len(sites) == 1
    profile = "f0-0002-b-v2"
    m = gate.execute(a.main_exe, src, profile, a.case)
    without = gate.execute(a.main_exe, counterfactual(src, sites[0]), profile, a.case)
    expected = witness(src, m, without, sites[0])
    assert expected is not None
    assert [[d["code"], list(primary(d))] for d in expected] == manual["adr_code_primary"]
    candidate = gate.execute(a.candidate_exe, src, profile, a.case)
    if a.commit in ("C1", "C2", "C3"):
        if candidate != m:
            failures.append("D7-manual-78=must-conserve-main")
    elif candidate["outcome"]["kind"] != "rejected" or normalized(candidate["diagnostics"]) != expected:
        failures.append("D7-manual-78=candidate-not-ADR")
    # candidate_gate is immutable. Its A-unmodeled entries may be discharged
    # only by a causal witness, never by a row-ID allowlist.
    cross_failures = [f for f in cross["failures"] if not (
        a.commit in ("C4", "C5", "C6") and f.endswith("=A-unmodeled") and
        f.rsplit("=", 1)[0] in members)]
    failures.extend(cross_failures)
    report = {"schema": "i07c-d7-v1", "commit": a.commit, "complete": True,
              "classifier_sha256": digest(Path(__file__).read_bytes()),
              "cross_classifier_sha256": FROZEN, "corpora": 20, "rows": 151188,
              "members": members, "manual_78_expected_adr_diagnostics": expected,
              "failures": failures}
    a.output.write_text(json.dumps(report, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print("D7", len(members), "members;", len(failures), "failures")
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
