#!/usr/bin/env python3
"""Freeze main's match/if boundary with an exterior expected type.

Usage: python build_r13_match_exterior.py MAIN_EXE INSTRUMENTED_EXE CASE_PATH
The instrumented executable is the 31d91db blob with only a stderr probe at
every Checker::branch_accept; its JSON must equal unmodified main's JSON.
"""

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCES = [
    ("annotation_no_normal", "fn Main(){let c=true;let x:I64=match c{true=>{value:(if c then {return;} else {return;});},false=>{value:1;}};}"),
    ("annotation_one_normal", "fn Main(){let c=true;let x:I64=match c{true=>{value:(if c then 1 else {return;});},false=>{value:2;}};}"),
    ("annotation_mismatch", "fn Main(){let c=true;let x:Bool=match c{true=>{value:(if c then 1 else 2);},false=>{value:true;}};}"),
    ("argument_no_normal", "fn F(x:view I64){}fn Main(){let c=true;F(match c{true=>{value:(if c then {return;} else {return;});},false=>{value:1;}});}"),
    ("argument_two_normal", "fn F(x:view I64){}fn Main(){let c=true;F(match c{true=>{value:(if c then 1 else 2);},false=>{value:3;}});}"),
    ("return_no_normal", "fn F()->I64{let c=true;return match c{true=>{value:(if c then {return 1;} else {return 2;});},false=>{value:3;}};}fn Main(){F();}"),
    ("return_two_normal", "fn F()->I64{let c=true;return match c{true=>{value:(if c then 1 else 2);},false=>{value:3;}};}fn Main(){F();}"),
]
FLAG = re.compile(r"R6FLAGS (\d+)\.\.(\d+) D=(\d) V=(\d) Q=(\d) P=(\d) J=(\d) T=(\d) expected=(\d)")


def run(exe, source, path):
    path.write_bytes(source.encode("ascii"))
    result = subprocess.run([str(exe), "check", str(path), "--profile", "f0-0003-b-v1", "--format", "json"],
                            capture_output=True, timeout=20, check=False)
    assert result.returncode in (0, 1), (result.returncode, result.stderr)
    return canonical(json.loads(result.stdout)), result.stderr.decode("utf-8", errors="replace")


def canonical(value):
    if isinstance(value, dict):
        return {key: "<SOURCE>" if key == "file" and isinstance(item, str) else canonical(item)
                for key, item in value.items()}
    if isinstance(value, list):
        return [canonical(item) for item in value]
    return value


def main():
    main_exe, probe_exe, case = map(Path, sys.argv[1:4])
    rows = []
    for ident, source in SOURCES:
        observed, _ = run(main_exe, source, case)
        probed, stderr = run(probe_exe, source, case)
        assert observed == probed, ident
        flags = [dict(zip(("start", "end", "Deferred", "Divergent", "PendingDomain", "PartialNormal", "FailedJoin", "fact_tainted", "expected_in_branch_accept"),
                          map(int, match.groups()))) for match in FLAG.finditer(stderr)]
        assert flags, (ident, stderr)
        if_value_events = [event for event in flags if "if " in source[event["start"]:event["end"]]]
        assert if_value_events and all(event["expected_in_branch_accept"] == 1 for event in if_value_events), ident
        rows.append({"id": ident, "profile": "f0-0003-b-v1", "source": source,
                     "source_sha256": hashlib.sha256(source.encode("ascii")).hexdigest(),
                     "source_bytes": len(source.encode("ascii")),
                     "expected_main": observed, "branch_accept_events": flags,
                     "if_value_events_with_expected": if_value_events})
    document = {"schema": "i07c-r13-match-exterior-v1", "base_commit": "31d91dbebd31c35b3cd09095c2d8e65b96c80548",
                "main_binary_sha256": hashlib.sha256(main_exe.read_bytes()).hexdigest(),
                "instrumented_binary_sha256": hashlib.sha256(probe_exe.read_bytes()).hexdigest(),
                "instrumentation": "Only eprintln of five OccurrenceFlag bits, fact.tainted, and expected.is_some() immediately before merge_child in every Checker::branch_accept. JSON equality with unmodified main asserted for each row.",
                "rows": rows}
    HERE.joinpath("r13-match-exterior.json").write_text(json.dumps(document, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print(len(rows), "main/instrumented JSON equal", [len(r["if_value_events_with_expected"]) for r in rows])


if __name__ == "__main__":
    main()
