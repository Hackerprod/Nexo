#!/usr/bin/env python3
"""Transcribe §14.10 RD from the normative ADR and compares check/run JSON.

The fixture is derived only from the ADR table. Its fragment hash makes an ADR
change invalidate the fixture until it is explicitly regenerated and reviewed.
"""

import argparse
from collections import Counter
import copy
import hashlib
import json
import os
import re
import subprocess
import tempfile
import time
from pathlib import Path

import i07c_rd_sweep as sweep


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


TYPE = re.compile(r"(?:I64|Bool|\(\)|MockResource|MockBlockingResource|Array<|Result<|Option<)")
DOMAINS = {"domain:array", "domain:copyable_data", "domain:mock_resource", "domain:result"}
TOKEN = re.compile(r"[A-Za-z_][A-Za-z_0-9]*|[0-9]+|==|->|\S")


def type_is_concrete(value: str, domain: bool = False) -> bool:
    if not isinstance(value, str):
        return False
    if domain and value in DOMAINS:
        return True

    def parse(pos: int) -> int:
        match = TYPE.match(value, pos)
        if not match:
            return -1
        atom = match.group()
        pos = match.end()
        if atom.endswith("<"):
            pos = parse(pos)
            if pos < 0:
                return -1
            if atom == "Result<":
                if value[pos:pos + 1] != ",":
                    return -1
                pos = parse(pos + 1)
                if pos < 0:
                    return -1
            if value[pos:pos + 1] != ">":
                return -1
            pos += 1
        return pos

    return parse(0) == len(value)


def lexical_site(source: str, span: list[int]) -> bool:
    if not isinstance(span, list) or len(span) != 2 or not all(type(i) is int for i in span):
        return False
    start, end = span
    if not 0 <= start < end <= len(source.encode("ascii")):
        return False
    tokens = list(TOKEN.finditer(source))
    return (start in {m.start() for m in tokens} and end in {m.end() for m in tokens}
            and not source[start:end].isspace())


def normalized(diag: dict) -> dict:
    primary = diag["primary_span"]
    return {"code": diag["code"], "subcause": diag["subcause"],
            "stage": diag["stage"], "operation": diag["operation"],
            "primary": [primary["start"], primary["end"]], "data": diag["data"],
            "related": sorted([[r["role"], r["span"]["start"], r["span"]["end"]]
                               for r in diag["related_spans"]], key=lambda r: (r[1], r[2]))}


def examine(row: dict, diagnostics: list[dict], command: str, file: str) -> tuple[list[str], list[dict]]:
    """Public JSON has no C/D/V labels: subtract exactly the fixture's C sites."""
    source = row["source"]
    errors = []
    expected_c = Counter((d["code"], d["subcause"], tuple(d["primary"]))
                         for d in row["expected"] if d["level"] == "C")
    observed = Counter((d["code"], d["subcause"], tuple(d["primary"]))
                       for d in diagnostics)
    for key, count in expected_c.items():
        if observed[key] != count:
            errors.append(f"C {key}: expected {count}, observed {observed[key]}")
    remaining = expected_c.copy()
    extras = []
    sites = Counter(tuple(d["primary"]) for d in diagnostics)
    for diag in diagnostics:
        key = (diag["code"], diag["subcause"], tuple(diag["primary"]))
        if remaining[key]:
            remaining[key] -= 1
        else:
            extras.append(diag)
        if (diag["code"], diag["subcause"], diag["operation"]) not in CODES.values() or diag["stage"] != "types":
            errors.append(f"invalid RD operation/code/subcause: {key} {diag['operation']}")
        if not lexical_site(source, diag["primary"]) or sites[tuple(diag["primary"])] != 1:
            errors.append(f"invalid/duplicate primary: {diag['primary']}")
        if diag.get("file") != file:
            errors.append(f"wrong primary file: {diag.get('file')}")
        data = diag["data"]
        if diag["code"] == "NX-E0103":
            if data != {} or diag["related"]:
                errors.append("U must have empty data/relations")
        elif (set(data) != {"expected_type", "found_type"}
              or not type_is_concrete(data.get("expected_type"), domain=True)
              or not type_is_concrete(data.get("found_type"))):
            errors.append(f"unserializable/invalid type data: {data}")
        for role, start, end, related_file in diag["related"]:
            if related_file != file or role not in ("expected_type", "found_type") or not lexical_site(source, [start, end]):
                errors.append(f"invalid relation: {role} {start}:{end} {related_file}")
            if role == "expected_type" and [start, end] == diag["primary"]:
                errors.append("expected_type self-relation")
        # Apply the same recursive type check even if a future schema nests data.
        def check_nested(value: object, name: str = "") -> None:
            if isinstance(value, dict):
                for k, v in value.items():
                    check_nested(v, k)
            elif isinstance(value, list):
                for v in value:
                    check_nested(v, name)
            elif name in ("expected_type", "found_type") and not type_is_concrete(value, domain=name == "expected_type"):
                errors.append(f"unserializable nested {name}: {value}")
        check_nested(data)
    return errors, extras


def public_diagnostics(document: dict) -> list[dict]:
    result = []
    for item in document["diagnostics"]:
        diag = normalized(item)
        diag["file"] = item["primary_span"]["file"]
        diag["related"] = sorted([[r["role"], r["span"]["start"], r["span"]["end"], r["span"]["file"]]
                                  for r in item["related_spans"]], key=lambda r: (r[1], r[2]))
        result.append(diag)
    return result


def self_test(rows: list[dict]) -> None:
    # Synthetic decisions exercise the filter, never predict product emissions.
    row = next(r for r in rows if r["id"] == "RD83")
    file = "case.n"
    causal = [copy.deepcopy(d) for d in row["expected"] if d["level"] == "C"]
    for d in causal:
        d.pop("level")
        d["file"] = file
        d["related"] = [[*r, file] for r in d["related"]]
    assert not examine(row, causal, "check", file)[0]
    extra = copy.deepcopy(causal[0])
    extra["primary"] = [90, 95]  # RD83's other false leaf; presence is I.
    extra["related"] = [["found_type", 90, 95, file]]
    assert not examine(row, causal + [extra], "check", file)[0]
    cases = []
    cases.append(causal[1:])  # missing C
    for change in (
        {"code": "NX-E9999"}, {"primary": [0, 0]}, {"primary": [9999, 10000]},
        {"primary": [0, 1]}, {"primary": [80, 83]}, {"data": {"expected_type": "Array<?>", "found_type": "Bool"}},
        {"data": {"expected_type": "?17", "found_type": "Bool"}},
        {"related": [["expected_type", 79, 84, file]]},
        {"related": [["found_type", 10000, 10001, file]]},
        {"related": [["found_type", 79, 79, file]]},
    ):
        altered = copy.deepcopy(causal[0]); altered.update(change)
        cases.append([altered])
    cases.append(causal + [copy.deepcopy(causal[0])])
    for case in cases:
        assert examine(row, case, "check", file)[0], case
    for rid in ("RD101", "RD119a", "RD119b"):
        current = next(r for r in rows if r["id"] == rid)
        base = [dict(d, file=file, related=[[*r, file] for r in d["related"]])
                for d in current["expected"] if d["level"] == "C"]
        assert not examine(current, base, "run", file)[0]
        corrupt = copy.deepcopy(base)
        corrupt[0]["data"] = {"expected_type": "?17", "found_type": "Bool"}
        assert examine(current, corrupt, "run", file)[0]
    print("RD filter self-test: PASS (missing C, vocabulary, lexical sites, duplicates, types, relations)")


def verify(binary: Path, rows: list[dict]) -> int:
    failed = 0
    snapshots = 0
    with tempfile.TemporaryDirectory(prefix="i07c-rd-") as temporary:
        path = Path(temporary) / "case.n"
        for row in rows:
            path.write_bytes(row["source"].encode("ascii"))
            row_failed = False
            for command in ("check", "run"):
                with tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stdout, \
                     tempfile.TemporaryFile(mode="w+t", encoding="utf-8") as stderr:
                    process = subprocess.Popen(
                        [str(binary), command, "case.n", "--format", "json",
                         "--profile", "f0-0002-b-v2"], cwd=temporary,
                        stdout=stdout, stderr=stderr,
                    )
                    last_output = time.monotonic()
                    last_size = 0
                    while process.poll() is None:
                        size = os.fstat(stdout.fileno()).st_size + os.fstat(stderr.fileno()).st_size
                        if size != last_size:
                            last_output, last_size = time.monotonic(), size
                        if sweep.working_set(process.pid) > sweep.MAX_WORKING_SET:
                            process.kill()  # This invocation created this PID.
                            process.wait()
                            raise RuntimeError(f"INCONCLUSO {row['id']} {command}: >6 GB")
                        if time.monotonic() - last_output > sweep.SILENCE_LIMIT:
                            process.kill()  # This invocation created this PID.
                            process.wait()
                            raise RuntimeError(f"INCONCLUSO {row['id']} {command}: >20 min silent")
                        time.sleep(0.02)
                    stdout.seek(0)
                    stderr.seek(0)
                    done = subprocess.CompletedProcess(
                        process.args, process.returncode, stdout.read(), stderr.read())
                try:
                    document = json.loads(done.stdout)
                    actual = public_diagnostics(document)
                except (ValueError, KeyError, TypeError) as error:
                    print(row["id"], command, "unreadable output", error, done.stdout[:300], done.stderr[:300])
                    row_failed = True
                    continue
                expected = row["expected"]
                errors, extras = examine(row, actual, command, "case.n")
                positive = not expected
                desired = "checked" if command == "check" else "unit"
                outcome = "rejected" if not positive else desired
                if (done.returncode != (0 if positive else 1)
                    or document["outcome"]["kind"] != outcome
                    or bool(actual) != bool(expected)
                    or document["stdout"] != (["[3]"] if positive and command == "run" and row["id"] == "RD87c" else [])
                    or document["events"] != []
                    or (not positive and document["observations"] != [])
                    or (positive and row["id"] != "RD87c" and document["observations"] != [])
                    or done.stderr):
                    errors.append(f"exit/outcome/channels: {done.returncode}, {document['outcome']}, {done.stderr[:200]}")
                snapshot_expected = [dict((k, v) for k, v in diag.items() if k != "level") for diag in expected]
                snapshot_actual = [{k: v for k, v in diag.items() if k != "file"}
                                   for diag in actual]
                snapshot_actual = [{**d, "related": [r[:3] for r in d["related"]]} for d in snapshot_actual]
                if errors:
                    print(row["id"], command, "NORMATIVE", errors, "actual", actual)
                    row_failed = True
                elif snapshot_actual != snapshot_expected:
                    snapshots += 1
                    print(row["id"], command, "SNAPSHOT I expected", snapshot_expected,
                          "actual", snapshot_actual, "delta I by structural filter; independent causal review required",
                          "extras", extras)
            failed += row_failed
    print(f"RD snapshot differences: {snapshots} (informative)")
    return failed


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--generate", action="store_true")
    parser.add_argument("--verify", type=Path)
    parser.add_argument("--self-test", action="store_true")
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
        self_test(rows)
        failures = verify(args.verify, rows)
        print(f"RD normative: {len(rows) - failures}/{len(rows)} check/run pairs")
        raise SystemExit(bool(failures))
    if args.self_test:
        self_test(rows)


if __name__ == "__main__":
    main()
