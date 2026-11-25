#!/usr/bin/env python3
"""Audit frozen corpus rows against an independently built main and the ADR oracle.

This writes measurements, not normative expected values. The three examples per
family in the companion table must be derived from the ADR by a reviewer.
"""

import collections
import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "scripts"))
import i07c_rd_oracle as oracle  # noqa: E402
from verify_corpora import FILE  # noqa: E402


def source(row):
    if isinstance(row, str):
        return row
    for key in ("wrapped", "w", "flat", "source"):
        if key in row:
            return row[key]
    raise ValueError("row has no source")


def project_main(data):
    output = []
    for d in data.get("diagnostics", []):
        roles = {x["role"]: [x["span"]["start"], x["span"]["end"]]
                 for x in d.get("related_spans", [])}
        fields = d.get("data", {})
        span = d["primary_span"]
        output.append([d["code"], d.get("subcause"), [span["start"], span["end"]],
                       fields.get("expected_type", ""), fields.get("found_type", ""),
                       roles.get("expected_type"), roles.get("found_type")])
    return output


def project_oracle(data):
    return [[d["code"], d.get("subcause"), d["primary"],
             d.get("expected_type", ""), d.get("found_type", ""),
             d.get("rel_expected"), d.get("rel_found")]
            for d in data["diagnostics"]]


def family(expected, main):
    if expected == main:
        return "equal"
    if len(main) > len(expected):
        return "main_extra_diagnostic"
    if len(expected) > len(main):
        return "main_missing_diagnostic"
    if all(x[:5] == y[:5] and x[6] == y[6] for x, y in zip(expected, main)):
        return "provenance_relation"
    if all(x[:3] == y[:3] for x, y in zip(expected, main)):
        return "different_type_or_relation"
    return "different_code_or_primary"


def main():
    executable, case_path, output_path = map(Path, sys.argv[1:4])
    limit = int(sys.argv[4]) if len(sys.argv) > 4 else None
    entries = json.loads((HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["corpora"]
    result = {"schema": "i07c-r12-family-audit-v1",
              "main_commit": "31d91dbebd31c35b3cd09095c2d8e65b96c80548",
              "main_binary_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
              "oracle_script_sha256": hashlib.sha256((HERE.parents[2] / "scripts/i07c_rd_oracle.py").read_bytes()).hexdigest(),
              "projection": "ordered code, subcause, primary bytes, expected/found types and relation bytes",
              "corpora": {}, "families": {}}
    cache = {}
    started = time.monotonic()
    for entry in entries:
        corpus_id = entry["id"]
        data = (HERE / FILE[corpus_id.split("@")[0]]).read_bytes()
        assert hashlib.sha256(data).hexdigest() == entry["sha256"]
        rows = json.loads(data)
        if isinstance(rows, dict):
            rows = rows["progs"]
        assert len(rows) == entry["rows"]
        if limit is not None:
            rows = rows[:limit]
        counts = collections.Counter()
        for index, row in enumerate(rows):
            src = source(row)
            key = (entry["profile"], src)
            if key not in cache:
                derived = oracle.derive(src)
                if derived.get("error") or derived.get("unmodeled") or derived.get("no_fijado") or derived.get("d_optional"):
                    cache[key] = None
                else:
                    case_path.write_bytes(src.encode("utf-8"))
                    run = subprocess.run([str(executable), "check", str(case_path), "--profile",
                                          entry["profile"], "--format", "json"],
                                         capture_output=True, timeout=20, check=False)
                    assert run.returncode in (0, 1), (corpus_id, index, run.returncode, run.stderr)
                    observed = json.loads(run.stdout)
                    cache[key] = (project_oracle(derived), project_main(observed))
            pair = cache[key]
            if pair is None:
                counts["unmodeled"] += 1
                continue
            expected, observed = pair
            label = family(expected, observed)
            counts[label] += 1
            if label == "equal":
                continue
            group = result["families"].setdefault(label, {"by_corpus": {}, "examples": [], "row_ids": {}})
            group["by_corpus"][corpus_id] = group["by_corpus"].get(corpus_id, 0) + 1
            group["row_ids"].setdefault(corpus_id, []).append(index)
            example = {"id": f"{corpus_id}:{index}", "bytes": len(src.encode("utf-8")),
                       "source": src, "oracle": expected, "main": observed}
            old = group["examples"]
            old.append(example)
            old.sort(key=lambda item: (item["bytes"], item["id"]))
            del old[3:]
        result["corpora"][corpus_id] = {"rows": len(rows), "sha256": entry["sha256"],
                                         "profile": entry["profile"], "counts": dict(counts)}
        print(corpus_id, dict(counts), "seconds", round(time.monotonic() - started, 1), flush=True)
        output_path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
