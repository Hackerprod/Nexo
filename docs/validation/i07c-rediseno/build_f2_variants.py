#!/usr/bin/env python3
"""Freeze source and derived F2 expectations for pending Result/Option payloads."""

import hashlib
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CASES = [
    ("result_both_pending_valid", "fn Main(){var q=[];var r=[];let x=(if true then Ok(q) else Err(r));q=[1];r=[true];}"),
    ("result_ok_before_err", "fn Main(){var q=[];var r=[];let x=(if true then Ok(q) else Err(r));let y:Result<Array<Bool>,Array<I64>>=x;q=[1];r=[true];}"),
    ("result_two_mismatches", "fn Main(){var q=[];var r=[];let x=(if true then Ok(q) else Err(r));q=[1];r=[true];let y:Result<Array<Bool>,Array<I64>>=x;}"),
    ("option_nested_valid", "fn Main(){var q=[];var r=[];let x=(if true then Some(Some(q)) else Some(Some(r)));q=[1];r=[1];}"),
    ("option_nested_conflict", "fn Main(){var q=[];var r=[];let x=(if true then Some(Some(q)) else Some(Some(r)));q=[1];r=[true];}"),
]
EXPECTED = {
    "result_both_pending_valid": {"outcome": "checked", "diagnosticos": []},
    "result_ok_before_err": {"outcome": "rejected", "diagnosticos": [
        ["NX-E0102", "type_mismatch", [109, 110], "Bool", "I64", [73, 103]],
        ["NX-E0102", "type_mismatch", [115, 119], "I64", "Bool", [73, 103]],
    ]},
    "result_two_mismatches": {"outcome": "rejected", "diagnosticos": [
        ["NX-E0102", "type_mismatch", [119, 120], "Result<Array<Bool>,Array<I64>>", "Result<Array<I64>,Array<Bool>>", [88, 118]],
    ], "first_incompatible_component": "Result.Ok", "not_selected": "Result.Err"},
    "option_nested_valid": {"outcome": "checked", "diagnosticos": []},
    "option_nested_conflict": {"outcome": "rejected", "diagnosticos": [
        ["NX-E0102", "type_mismatch", [91, 95], "I64", "Bool", [84, 87]],
    ], "path": "Option.payload/Option.payload/Array.element"},
}

rows = []
for name, source in CASES:
    rows.append({"id": name, "profile": "f0-0003-b-v1", "source": source,
                 "source_sha256": hashlib.sha256(source.encode("ascii")).hexdigest(), "bytes": len(source.encode("ascii")),
                 "expected_adr": EXPECTED[name]})
    if len(sys.argv) > 1:
        file = Path("D:/nexo-work/tmp") / (name + ".n")
        file.write_bytes(source.encode("ascii"))
        run = subprocess.run([sys.argv[1], "check", "--format", "json", "--profile", "f0-0003-b-v1", str(file)], capture_output=True, text=True, timeout=60)
        value = json.loads(run.stdout)
        print(name, value.get("outcome"), [(d["code"], d["primary_span"]["start"], d["primary_span"]["end"], d.get("related_spans")) for d in value.get("diagnostics", [])])
(HERE / "r9-f2-variants.json").write_text(json.dumps({"schema": "i07c-r9-f2-variants-v1", "rows": rows}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
