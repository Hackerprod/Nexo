#!/usr/bin/env python3
"""Transcribe §14.10 RD from the normative ADR and compares check/run JSON.

The fixture is derived only from the ADR table. Its fragment hash makes an ADR
change invalidate the fixture until it is explicitly regenerated and reviewed.
"""

import argparse
from collections import Counter
import hashlib
import json
import re
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ADR = ROOT / "docs/adr/F0-0002-second-slice.md"
FIXTURE = ROOT / "tests/fixtures/i07c_rd.json"
DV = "(if true then {return;} else {return;})"
ENTRY = re.compile(r"^(?:Una )?([MJXSU])(?::| sobre)\s*`?\[(\d+),(\d+)\)`?")
TYPES = re.compile(r"; `([^`]+)` / `([^`]+)`; R=(?:`\[(\d+),(\d+)\)`|—)")
FOUND = re.compile(r"`found_type` relacionado=`\[(\d+),(\d+)\)`")
CODES = {
    "M": ("NX-E0102", "type_mismatch", "types.check"),
    "J": ("NX-E1303", None, "types.check"),
    "X": ("NX-E0102", "not_indexable", "types.check"),
    "S": ("NX-E0102", "resource_required", "types.check"),
    "U": ("NX-E0103", None, "types.infer"),
}


def table() -> tuple[str, list[dict]]:
    adr = ADR.read_text(encoding="utf-8")
    start = adr.index("| ID | Fuente o expansión exacta | Diagnósticos completos:")
    end = adr.index("\n\nLas filas con contexto intrínseco", start)
    fragment = adr[start:end]
    rows = []
    for line in fragment.splitlines():
        match = re.match(r"^\| (RD\d+) \| (.*?) \| (.*?) \| (.*?) \| (.*?) \|$", line)
        if not match:
            continue
        rid, sources, results, _rule, labels = match.groups()
        source_parts = re.findall(r"(?:\(([a-d])\) )?`([^`]+)`", sources)
        result_parts = re.split(r"(?:^|<br>)\(([a-d])\) ", results)
        if result_parts[0] == "":
            result_parts = result_parts[1:]
            by_variant = {result_parts[i]: result_parts[i + 1]
                          for i in range(0, len(result_parts), 2)}
        else:
            by_variant = {"": results}
        label_parts = re.split(r"(?:^|;\s*)\(([a-d])\)\s*", labels)
        if label_parts[0] == "":
            label_parts = label_parts[1:]
            by_label_variant = {label_parts[i]: label_parts[i + 1]
                                for i in range(0, len(label_parts), 2)}
        else:
            by_label_variant = {"": labels}
        for variant, abbreviated in source_parts:
            source = abbreviated
            if source.startswith("F(") and source.endswith(")"):
                source = "fn Main(){" + source[2:-1] + "}"
            source = source.replace("DV", DV)
            result = by_variant.get(variant, "")
            if not result:
                raise ValueError(f"{rid}{variant}: missing result")
            classification = by_label_variant.get(variant)
            if classification is None:
                raise ValueError(f"{rid}{variant}: missing C/D/V annotation")
            expected = []
            for part in result.split("<br>"):
                if part == "`diagnostics=[]`":
                    continue
                head = ENTRY.match(part)
                if not head:
                    raise ValueError(f"{rid}{variant}: {part}")
                kind, a, b = head.groups()
                code, subcause, operation = CODES[kind]
                diag = {"code": code, "subcause": subcause,
                        "stage": "types", "operation": operation,
                        "primary": [int(a), int(b)], "data": {}, "related": []}
                if kind != "U":
                    types = TYPES.search(part)
                    if not types:
                        raise ValueError(f"{rid}{variant}: missing types in {part}")
                    wanted, found, rstart, rend = types.groups()
                    diag["data"] = {"expected_type": wanted, "found_type": found}
                    if rstart is not None:
                        diag["related"].append(["expected_type", int(rstart), int(rend)])
                    found_relation = FOUND.search(part)
                    fa, fb = (map(int, found_relation.groups()) if found_relation
                              else (int(a), int(b)))
                    diag["related"].append(["found_type", fa, fb])
                    diag["related"].sort(key=lambda relation: (relation[1], relation[2]))
                expected.append(diag)
            tagged = {int(index): level for level, index in
                      re.findall(r"\b([CDV])(\d+)\b", classification)}
            if tagged != {index: tagged[index] for index in range(1, len(expected) + 1)}:
                raise ValueError(f"{rid}{variant}: incomplete C/D/V annotation {classification}")
            for index, diag in enumerate(expected, 1):
                diag["level"] = tagged[index]
            for diag in expected:
                a, b = diag["primary"]
                if not 0 <= a < b <= len(source):
                    raise ValueError(f"{rid}{variant}: primary outside source {a}:{b}")
                for _, start, end in diag["related"]:
                    if not 0 <= start < end <= len(source):
                        raise ValueError(f"{rid}{variant}: relation outside source {start}:{end}")
            rows.append({"id": rid + (variant or ""), "source": source,
                         "expected": expected})
    if len(rows) != 152 or len({r["id"] for r in rows}) != 152:
        raise ValueError(f"RD expansion has {len(rows)} rows, expected 152")
    if len({re.match(r"RD\d+", r["id"]).group() for r in rows}) != 127:
        raise ValueError("RD table must contain 127 IDs")
    return hashlib.sha256(fragment.encode("utf-8")).hexdigest(), rows


def parse_text(stderr: str) -> list[dict]:
    diagnostics = []
    current = None
    for line in stderr.splitlines():
        header = re.match(r"^(NX-[ER]\d+):\d+:\d+: ", line)
        if header:
            if current is not None:
                diagnostics.append(current)
            current = {"code": header.group(1), "subcause": None,
                       "stage": None, "operation": None,
                       "primary": None, "data": {}, "related": []}
            continue
        if current is None:
            continue
        item = re.match(r"^  ([a-z_]+): (.*)$", line)
        if not item:
            continue
        key, value = item.groups()
        if key in ("stage", "operation", "subcause"):
            current[key] = value
        elif key == "at":
            span = json.loads(value)
            current["primary"] = [span["start"], span["end"]]
        elif key == "data":
            current["data"] = json.loads(value)
        elif key == "related":
            relation = json.loads(value)
            span = relation["span"]
            current["related"].append([relation["role"], span["start"], span["end"]])
    if current is not None:
        diagnostics.append(current)
    for diag in diagnostics:
        diag["related"].sort(key=lambda relation: (relation[1], relation[2]))
    return diagnostics


def verify(binary: Path, rows: list[dict]) -> int:
    failed = 0
    snapshots = 0
    with tempfile.TemporaryDirectory(prefix="i07c-rd-", dir=ROOT) as temporary:
        path = Path(temporary) / "case.n"
        for row in rows:
            path.write_bytes(row["source"].encode("ascii"))
            row_failed = False
            for command in ("check", "run"):
                done = subprocess.run([str(binary), command, str(path)],
                                      capture_output=True, text=True, encoding="utf-8")
                try:
                    actual = parse_text(done.stderr)
                except (ValueError, KeyError, TypeError) as error:
                    print(row["id"], command, "unreadable output", error, done.stdout[:300], done.stderr[:300])
                    row_failed = True
                    continue
                expected = row["expected"]
                causal = Counter((diag["code"], diag["subcause"], tuple(diag["primary"]))
                                 for diag in expected if diag["level"] == "C")
                observed = Counter((diag["code"], diag["subcause"], tuple(diag["primary"]))
                                   for diag in actual if diag["primary"] is not None)
                sites = Counter(tuple(diag["primary"]) for diag in actual
                                if diag["primary"] is not None)
                normative_error = (done.returncode != (1 if expected else 0)
                                   or bool(actual) != bool(expected)
                                   or any(observed[key] < count for key, count in causal.items())
                                   or any(count > 1 for count in sites.values())
                                   or any(diag["code"] not in {"NX-E0102", "NX-E0103", "NX-E1303"}
                                          or diag["primary"] is None
                                          or not 0 <= diag["primary"][0] < diag["primary"][1] <= len(row["source"])
                                          for diag in actual))
                snapshot_expected = [{key: value for key, value in diag.items() if key != "level"}
                                     for diag in expected]
                if normative_error:
                    print(row["id"], command, "NORMATIVE expected C", dict(causal),
                          "expected acceptance", not expected, "actual", actual, "exit", done.returncode)
                    row_failed = True
                elif actual != snapshot_expected:
                    snapshots += 1
                    print(row["id"], command, "SNAPSHOT expected", snapshot_expected,
                          "actual", actual)
            failed += row_failed
    print(f"RD snapshot differences: {snapshots} (informative)")
    return failed


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--generate", action="store_true")
    parser.add_argument("--verify", type=Path)
    args = parser.parse_args()
    digest, rows = table()
    fixture = {"adr_fragment_sha256": digest, "rows": rows}
    if args.generate:
        FIXTURE.write_text(json.dumps(fixture, ensure_ascii=False, indent=2) + "\n",
                           encoding="utf-8", newline="\n")
    else:
        saved = json.loads(FIXTURE.read_text(encoding="utf-8"))
        if saved != fixture:
            raise SystemExit("RD fixture differs from the current ADR table")
    if args.verify:
        failures = verify(args.verify, rows)
        print(f"RD normative: {len(rows) - failures}/{len(rows)} check/run pairs")
        raise SystemExit(bool(failures))


if __name__ == "__main__":
    main()
