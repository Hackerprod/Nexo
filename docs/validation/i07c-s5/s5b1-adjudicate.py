"""Adjudicate every differential, emitting one enumerated row per changed source.

Usage: python -B .../s5b1-adjudicate.py <diff-*.json>... <out.json>
"""

import hashlib
import importlib.util
import json
from pathlib import Path
import sys


HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("s5b1_corpus", HERE / "s5b1-corpus.py")
corpus = importlib.util.module_from_spec(spec)
spec.loader.exec_module(corpus)


def compact(diagnostic):
    span = diagnostic["primary_span"]
    return {"code": diagnostic["code"], "subcause": diagnostic["subcause"],
            "span": [span["start"], span["end"]], "data": diagnostic["data"],
            "related": [[item["role"], item["span"]["start"], item["span"]["end"]]
                        for item in diagnostic["related_spans"]]}


def adjudicate(diff):
    source = diff["source"]
    expression = source.split("let z=", 1)[1].split(";", 1)[0]
    selectors = [(base, selector, source.index(f"let z={base}[{selector}]") + len(f"let z={base}["))
                 for base in corpus.BASES for selector in corpus.SELECTORS
                 if expression == f"{base}[{selector}]"]
    assert len(selectors) == 1, diff["source_id"]
    base, selector, selector_start = selectors[0]
    before, after = diff["base"], diff["candidate"]
    assert before[0] == after[0] == 1 and before[2] == after[2] == "", diff["source_id"]
    old, new = json.loads(before[1]), json.loads(after[1])
    old_diagnostics = old.pop("diagnostics")
    new_diagnostics = new.pop("diagnostics")
    assert old == new and len(new_diagnostics) == len(old_diagnostics) + 1, diff["source_id"]
    options = [index for index in range(len(new_diagnostics))
               if new_diagnostics[:index] + new_diagnostics[index + 1:] == old_diagnostics]
    assert len(options) == 1, diff["source_id"]
    added = new_diagnostics[options[0]]
    primary = added["primary_span"]
    assert (added["code"], added["subcause"], added["operation"], added["stage"]) == (
        "NX-E0102", "type_mismatch", "types.check", "types")
    assert selector_start <= primary["start"] < primary["end"] <= selector_start + len(selector), diff["source_id"]
    assert any(item["primary_span"]["start"] < selector_start for item in old_diagnostics), diff["source_id"]
    return {"base": base, "selector": selector, "previous": list(map(compact, old_diagnostics)),
            "added": compact(added), "profile": diff["profile"], "verb": diff["verb"],
            "mode": diff["mode"]}


def main():
    paths = [Path(arg) for arg in sys.argv[1:]]
    *inputs, output = paths
    assert len(inputs) == 7
    coverage = set()
    rows = {}
    totals = {"sources": 0, "invocations": 0, "changes": 0}
    binary_hashes = None
    for path in inputs:
        report = json.loads(path.read_text(encoding="utf-8"))
        first, last = report["range"]
        assert not coverage.intersection(range(first, last))
        coverage.update(range(first, last))
        for key in totals:
            totals[key] += report["totals"][key]
        if binary_hashes is None:
            binary_hashes = report["binary_sha256"]
        assert binary_hashes == report["binary_sha256"]
        for diff in report["differences"]:
            assert diff["source_id"].startswith("generated:")
            checked = adjudicate(diff)
            row = rows.setdefault(diff["source_id"], {"source": diff["source"],
                                                       "base": checked["base"], "selector": checked["selector"],
                                                       "previous": checked["previous"], "added": checked["added"],
                                                       "invocations": []})
            assert all(row[key] == checked[key] for key in ("base", "selector", "previous", "added"))
            row["invocations"].append([checked["profile"], checked["verb"], checked["mode"]])
    assert coverage == set(range(3369)), (len(coverage), min(coverage), max(coverage))
    assert totals["sources"] == 3369 and totals["invocations"] == 3369 * 24
    assert totals["changes"] == sum(len(row["invocations"]) for row in rows.values())
    assert all(len(row["invocations"]) == 12 for row in rows.values())
    output.write_text(json.dumps({"manifest": json.loads((HERE / "s5b1-corpus.json").read_text()),
                                  "binary_sha256": binary_hashes, "totals": totals,
                                  "enumerated_differences": rows}, indent=2) + "\n", encoding="utf-8")
    print(f"all differences adjudicated: {totals}; changed_sources={len(rows)} "
          f"sha256={hashlib.sha256(output.read_bytes()).hexdigest()}")


if __name__ == "__main__":
    main()
