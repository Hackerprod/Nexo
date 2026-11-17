#!/usr/bin/env python3
"""F0-0003 §9/§13.2 source oracle; never reads compiler IR or imports nexo.

All expectations are built from the normative literals and the small resource /
generator model before any invocation of the Nexo executable.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
ADR = ROOT / "docs/adr/F0-0003-third-slice.md"
OUTPUT = ROOT / "docs/validation/F0-0003-J16-matrix.json"
PROFILE = "f0-0003-b-v1"
SEED = 0x4E45584F
SIZES = (0, 1, 2, 8, 32, 128, 512)
MAX_SOURCE_BYTES = 65536
DIAGNOSTIC_FIELDS = frozenset((
    "schema_version", "diagnostic_namespace", "profile_id", "code", "subcause",
    "rule", "explanation_id", "severity", "stage", "message", "operation",
    "primary_span", "related_spans", "cause_chain", "feature_id",
    "unexpected_token", "expected_tokens", "expected_tokens_kind",
    "parser_context", "recognized_prefix", "repair", "data",
))


def sha(source: str) -> str:
    return hashlib.sha256(source.encode("utf-8")).hexdigest()


def case(id: str, source: str, action: str, expected: dict, section: str,
         kind: str = "núcleo", limit: str | None = None) -> dict:
    assert len(source.encode("utf-8")) <= MAX_SOURCE_BYTES, id
    return dict(id=id, source=source, source_sha256=sha(source), profile=PROFILE,
                action=action, expected=expected, section=section, class_=kind,
                later_limit=limit)


def section_nine_pairs() -> list[dict]:
    # Markdown is authoritative data, not a compiler output or an IR dump.
    lines = ADR.read_text(encoding="utf-8").splitlines()
    negative: dict[str, tuple[str, str, tuple[int, int]]] = {}
    positive: dict[str, str] = {}
    exact_data: dict[str, dict] = {}
    pairs: list[dict] = []
    for line in lines[982:1105]:
        if not line.startswith("| "):
            continue
        cells = [part.strip() for part in line.strip().strip("|").split("|")]
        if not re.fullmatch(r"[A-Z]\d\d", cells[0]):
            continue
        id = cells[0]
        if len(cells) == 4 and cells[1].startswith("`fn ") or (
            len(cells) == 4 and cells[1].startswith("`gen fn ")
        ):
            if cells[2].startswith("NX-") and re.fullmatch(r"\[\d+,\d+\)", cells[3]):
                negative[id] = (cells[1][1:-1], cells[2], tuple(map(int, re.findall(r"\d+", cells[3]))))
        elif len(cells) == 2 and cells[1].startswith("`"):
            positive[id] = cells[1][1:-1]
        elif len(cells) >= 4 and cells[1].startswith("`{"):
            exact_data[id] = json.loads(cells[1][1:-1])
    for line in lines[1182:1197]:
        if not line.startswith("| "):
            continue
        cells = [part.strip() for part in line.strip().strip("|").split("|")]
        if len(cells) != 5 or not re.fullmatch(r"[A-Z]\d\d", cells[0]):
            continue
        id = cells[0]
        negative[id] = (cells[1][1:-1], cells[2], tuple(map(int, re.findall(r"\d+", cells[3]))))
        positive[id] = cells[4][1:-1]
    assert len(negative) == len(positive) == 46
    for id in sorted(negative):
        bad, rule, span = negative[id]
        code, _, subcause = rule.partition("/")
        if id == "G08":
            subcause = ""
        expected = {"exit": 1, "diagnostics": [{"code": code,
            "subcause": subcause or None, "primary": list(span)}]}
        if id in exact_data:
            expected["diagnostics"][0]["data"] = exact_data[id]
        if id == "G08":
            expected["diagnostics"][0]["feature_id"] = "generator"
            expected["diagnostics"][0]["data"] = {"pending_scope": "later_adr"}
        pairs.append(case("F3-" + id + "-negative", bad, "check", expected, "F0-0003 §9"))
        pairs.append(case("F3-" + id + "-positive", positive[id], "check",
                          {"exit": 0, "diagnostics": []}, "F0-0003 §9"))
    return pairs


def modeled_cases() -> list[dict]:
    cases: list[dict] = []
    def accepted(id: str, source: str, events: list[str], section: str):
        cases.append(case(id, source, "run", {"exit": 0, "diagnostics": [],
                          "outcome": "unit", "events": events, "stdout": []}, section))

    # The model allocates resource IDs in Open order, and closes on scope exit.
    for n in SIZES:
        source = "fn Main(){" + "".join(f"let r{i}=MockOpen({i});" for i in range(n)) + "}"
        events = [f"open:{i}:{i}" for i in range(n)]
        events += [f"close:{i}:{i}" for i in reversed(range(n))]
        accepted(f"resource-reverse-{n}", source, events, "F0-0003 §§4,9.2,13.2")
    for n in SIZES:
        body = "".join(f"yield {i};" for i in range(n))
        nexts = "".join(f"let x{i}=Iterator.Next(edit g);AssertEq(x{i},Some({i}));" for i in range(n))
        source = f"gen fn G()->I64{{{body}}}fn Main(){{var g=G();{nexts}let a=Iterator.Next(edit g);AssertEq(a,None);let b=Iterator.Next(edit g);AssertEq(b,None);}}"
        accepted(f"generator-yields-{n}", source, [], "F0-0003 §§5,9.2,13.2")
    accepted("resource-break", "gen fn G()->MockResource{yield MockOpen(1);yield MockOpen(2);}fn Main(){for r in G(){Inspect(r);break;}}",
             ["open:0:1", "inspect:0:1", "close:0:1"], "F0-0003 §9.2")
    accepted("resource-continue", "gen fn G()->MockResource{yield MockOpen(1);yield MockOpen(2);}fn Main(){for r in G(){Inspect(r);continue;}}",
             ["open:0:1", "inspect:0:1", "close:0:1", "open:1:2", "inspect:1:2", "close:1:2"], "F0-0003 §9.2")
    accepted("generator-cold", "gen fn G(r:move MockResource)->I64{Inspect(r);yield 1;}fn Main(){let g=G(MockOpen(7));Drop(g);}",
             ["open:0:7", "close:0:7"], "F0-0003 §9.2")
    accepted("generator-resume", "gen fn G(r:move MockResource)->I64{Inspect(r);yield 1;Inspect(r);yield 2;}fn Main(){var g=G(MockOpen(7));AssertEq(Iterator.Next(edit g),Some(1));AssertEq(Iterator.Next(edit g),Some(2));AssertEq(Iterator.Next(edit g),None);AssertEq(Iterator.Next(edit g),None);}",
             ["open:0:7", "inspect:0:7", "inspect:0:7", "close:0:7"], "F0-0003 §9.2")
    accepted("closure-retain", "fn Main(){let r=MockOpen(7);let f=fn [move r](){Inspect(r);};f();f();Drop(f);}",
             ["open:0:7", "inspect:0:7", "inspect:0:7", "close:0:7"], "F0-0003 §9.2")
    accepted("closure-cold", "fn Main(){let r=MockOpen(7);let f=fn [move r](){Inspect(r);};Drop(f);}",
             ["open:0:7", "close:0:7"], "F0-0003 §9.2")
    accepted("generator-next-once", "gen fn G(r:move MockResource)->I64{Inspect(r);yield 1;Inspect(r);yield 2;}fn Main(){var g=G(MockOpen(7));AssertEq(Iterator.Next(edit g),Some(1));Drop(g);}",
             ["open:0:7", "inspect:0:7", "close:0:7"], "F0-0003 §9.2")
    accepted("generator-result-items", "gen fn G()->Result<I64,I64>{yield Err(9);yield Ok(2);}fn Main(){var g=G();let a=Iterator.Next(edit g);let b=Iterator.Next(edit g);AssertEq(a,Some(Err(9)));AssertEq(b,Some(Ok(2)));}",
             [], "F0-0003 §9.2")
    accepted("resource-break-no-inspect", "gen fn G()->MockResource{yield MockOpen(1);yield MockOpen(2);}fn Main(){for r in G(){break;}}",
             ["open:0:1", "close:0:1"], "F0-0003 §9.2")
    return cases


def extra_cases() -> list[dict]:
    rows: list[dict] = []
    def negative(id: str, source: str, code: str, subcause: str | None,
                 marker: str, section: str, *, last=False, data: dict | None = None,
                 primary_width: int | None = None):
        start = source.rindex(marker) if last else source.index(marker)
        width = primary_width if primary_width is not None else len(marker.encode("utf-8"))
        diagnostic = {"code": code, "subcause": subcause,
                      "primary": [start, start + width]}
        if data is not None:
            diagnostic["data"] = data
        rows.append(case(id, source, "check", {"exit": 1,
                         "diagnostics": [diagnostic]}, section))
    def positive(id: str, source: str, section: str):
        rows.append(case(id, source, "check", {"exit": 0,
                         "diagnostics": []}, section))
    def run(id: str, source: str, events: list[str], section: str,
            *, outcome: str = "unit", error: int | None = None,
            stdout: list[str] | None = None, mock_env: dict | None = None,
            observation_bytes: int | None = None):
        expected = {"exit": 0 if error is None else 1, "diagnostics": [],
                    "outcome": outcome, "events": events, "stdout": stdout or [],
                    "reserved_observation_bytes": 0}
        if error is not None:
            expected["outcome_detail"] = {"kind": outcome, "error": str(error)}
        if observation_bytes is not None:
            expected["observation_bytes"] = observation_bytes
        item = case(id, source, "run", expected, section,
                    kind="andamiaje" if mock_env is not None or observation_bytes is not None
                    else "núcleo")
        if mock_env is not None:
            item["mock_env"] = mock_env
        rows.append(item)

    # §9.2 distinguishes an Err item from propagation by its consumer.
    err_item = "gen fn G()->Result<I64,I64>{yield Err(9);yield Ok(2);}fn Main()->Result<(),I64>{var g=G();let item=Iterator.Next(edit g);match move item{Some(move r)=>{let n=r?;},None=>{}}return Ok(());}" 
    run("generator-result-propagation", err_item, [], "F0-0003 §9.2",
        outcome="err", error=9)

    # Two independent closure loans must both end before an exclusive write.
    for order in ("fg", "gf"):
        source = "fn Main(){var a=[1];let f=fn [view a](){};let g=fn [view a](){};" + \
            "".join(f"Drop({name});" for name in order) + "a[0]=2;}"
        run("retained-readers-" + order, source, [], "F0-0003 §9.2")
    held = "fn Main(){var a=[1];let f=fn [view a](){};let g=fn [view a](){};Drop(f);a[0]=2;Drop(g);}"
    negative("retained-reader-still-live", held, "NX-E1105", "active_borrow_conflict",
             "a[0]", "F0-0003 §9.2", primary_width=1)

    both = "fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Drop(r);},false=>{Drop(r);}}Inspect(r);}fn Main(){}"
    negative("branch-both-consume", both, "NX-E0202", "definitely_consumed",
             "r", "F0-0003 §9.2", last=True)
    reinit = "fn F(b:view Bool){var r=MockOpen(1);match b{true=>{Drop(r);r=MockOpen(2);},false=>{Drop(r);r=MockOpen(3);}}Inspect(r);}fn Main(){F(true);}"
    run("branch-reinitialized", reinit,
        ["open:0:1", "close:0:1", "open:1:2", "inspect:1:2", "close:1:2"],
        "F0-0003 §9.2")
    returned = "fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Drop(r);return;},false=>{}}Inspect(r);}fn Main(){F(false);}"
    run("branch-return-no-contamination", returned,
        ["open:0:1", "inspect:0:1", "close:0:1"], "F0-0003 §9.2")

    for guard, chosen in (("false", 2), ("true", 1)):
        source = "fn Main(){let o=Some(MockOpen(1));match move o{" + \
            f"Some(move r) if {guard}=>{{Inspect(r);}}," + \
            "Some(move r)=>{Inspect(r);},None=>{}}}"
        run("guard-before-extraction-" + guard, source,
            ["open:0:1", "inspect:0:1", "close:0:1"], "F0-0003 §9.2")

    contextual = "fn Main(){let x:I64=match true{true=>{value:1;},false=>{value:false;}};}"
    negative("selection-context-type", contextual, "NX-E0102", "type_mismatch",
             "false", "F0-0003 §9.2", last=True)
    owned = "fn Main(){let o:Option<MockResource>=Some(MockOpen(1));match move o{Some(move r)=>{Inspect(r);},None=>{}}}"
    run("selection-resource-owned", owned,
        ["open:0:1", "inspect:0:1", "close:0:1"], "F0-0003 §9.2")
    nested = "fn Main(){let o:Option<Result<Bool,I64>>=Some(Ok(true));match o{None=>{},Some(Ok(true))=>{},Some(Ok(false))=>{},Some(Err(_))=>{}}}"
    positive("nested-coverage-complete", nested, "F0-0003 §9.2")
    missing = nested.replace("Some(Ok(false))=>{},", "")
    negative("nested-coverage-missing-leaf", missing, "NX-E1304", "non_exhaustive",
             "match", "F0-0003 §9.2")
    read_resource = "fn Main(){let o:Option<MockResource>=Some(MockOpen(1));match o{Some(r)=>{Drop(r);},None=>{}}}"
    negative("selection-resource-read-cannot-drop", read_resource, "NX-E1105", "permission_escalation",
             "r", "F0-0003 §9.2", last=True)
    escaped = "fn Main(){let o=if true then {let x=1;value:Some(fn [view x]()->I64{return x;});}else{return;};Drop(o);}"
    negative("escape-wrapped-closure", escaped, "NX-E1104", None,
             "Some(fn [view x]()->I64{return x;})", "F0-0003 §9.2")
    copied = "fn Main(){var a=[1];let f=fn [copy a](){AssertEq(a,[1]);};a[0]=2;f();}"
    run("captured-array-independent", copied, [], "F0-0003 §9.2")
    edited = "fn Main(){var x=1;let f=fn [edit x](){x=2;};let g=f;}"
    negative("captured-edit-not-copyable", edited, "NX-E0201", None,
             "f", "F0-0003 §9.2", last=True)
    moved_edit = "fn Main(){var x=1;var f=fn [edit x](){x=2;};var g=move f;edit g();Drop(g);AssertEq(x,2);}"
    run("captured-edit-move-preserves-loan", moved_edit, [], "F0-0003 §9.2")
    generator_copy = "gen fn G()->I64{yield 1;}fn Main(){var g=G();let h=g;}"
    negative("generator-state-not-copyable", generator_copy, "NX-E0201", None,
             "g", "F0-0003 §9.2", last=True)
    generator_move = "gen fn G()->I64{yield 1;}fn Main(){var g=G();var h=move g;AssertEq(Iterator.Next(edit h),Some(1));}"
    run("generator-state-move-resumes", generator_move, [], "F0-0003 §9.2")
    hypothetical = "fn Main(){return;let r=MockOpen(1);let f=fn [copy r](){};}"
    negative("hypothetical-return-keeps-copy-check", hypothetical, "NX-E0201", None,
             "r", "F0-0003 §9.2", last=True)

    for name, source in (
        ("effect-chain", "fn Hoja(){Print(1);}fn Medio(){Hoja();}fn Main() with no_io{Medio();}"),
        ("effect-cycle", "fn A(){B();}fn B(){A();Print(1);}fn Main() with no_io{A();}"),
    ):
        negative(name, source, "NX-E0415", None,
                 "Medio" if name == "effect-chain" else "A();", "F0-0003 §9.2",
                 last=True, primary_width=1 if name == "effect-cycle" else None)
    positive("effect-latent-closure", "fn Main() with no_io{let f=fn [](){Print(1);};}",
             "F0-0003 §9.2")
    positive("generator-without-yield-no-suspend", "gen fn G()->I64 with no_suspend{}fn Main(){}",
             "F0-0003 §9.2")
    nested_yield = "gen fn G()->I64{let f=fn [](){yield 1;};yield 2;}fn Main(){}"
    negative("lexical-yield-in-closure", nested_yield, "NX-E1213", None,
             "yield", "F0-0003 §9.2")
    nested_break = "fn Main(){while true{let f=fn [](){break;};break;}}"
    negative("lexical-break-in-closure", nested_break, "NX-E1326",
             "break_outside_loop", "break", "F0-0003 §9.2")

    # A2 §12.2/§12.3: the official B surface of the reduced EX02/EX03 flows.
    flow2 = "fn Incrementar(xs: edit Array<I64>) with no_alloc,no_block,no_suspend,no_io {var i=0;while i<Len(xs){xs[i]=xs[i]+1;i=i+1;}} fn Main(){var original=[1,2];var copia=original;Incrementar(edit copia);AssertEq(original,[1,2]);AssertEq(copia,[2,3]);}"
    run("A2-EX03-reduced", flow2, [], "F0-0002 §12.2")
    flow1 = ("fn Sumar(records: view Array<Result<I64,I64>>) -> Result<I64,I64> with no_alloc,no_block,no_suspend,no_io {\n"
             " var i=0;\n var total=0;\n while i<Len(records){let value=records[i]?;total=TryAdd(total,value)?;i=i+1;}\n"
             " return Ok(total);\n}\nfn Main() -> Result<(),I64> {\n let file=MockOpen(7);\n"
             " let records=MockReadRecords(file)?;\n let total=Sumar(records)?;\n"
             " MockWriteI64(total)?;\n return Ok(());\n}")
    for id, read, write in (
        ("sum25", {"ok": [{"ok": "12"}, {"ok": "8"}, {"ok": "5"}]}, {"ok": None}),
        ("empty", {"ok": []}, {"ok": None}),
        ("read-error", {"err": "10"}, {"ok": None}),
        ("record-error", {"ok": [{"ok": "12"}, {"err": "20"}, {"ok": "5"}]}, {"ok": None}),
        ("overflow", {"ok": [{"ok": "9223372036854775807"}, {"ok": "1"}]}, {"ok": None}),
        ("write-error", {"ok": [{"ok": "12"}]}, {"err": "40"}),
    ):
        events = ["open:0:7"]
        error = int(read["err"]) if "err" in read else None
        stdout: list[str] = []
        if error is not None:
            events.append(f"read_records:0:err:{error}")
        else:
            records = read["ok"]
            events.append(f"read_records:0:ok:{len(records)}")
            total = 0
            for record in records:
                if "err" in record:
                    error = int(record["err"])
                    break
                value = int(record["ok"])
                if total + value > 9223372036854775807 or total + value < -9223372036854775808:
                    error = 30
                    break
                total += value
            if error is None:
                if "err" in write:
                    error = int(write["err"])
                    events.append(f"write_i64:{total}:err:{error}")
                else:
                    events.append(f"write_i64:{total}:ok")
                    stdout.append(str(total))
        events.append("close:0:7")
        env = {"schema_version": 1, "mock_read_records": [read], "mock_write_i64": [write],
               "mock_request": [], "mock_reply": []}
        run("A2-EX01-" + id, flow1, events, "F0-0002 §12.1",
            outcome="err" if error is not None else "ok", error=error,
            stdout=stdout, mock_env=env)
    flow3 = "fn Responder(route: view I64)->I64 with no_alloc,no_block,no_suspend,no_io{return if route==0 then {value:200;} else {value:404;};} fn Main()->Result<(),I64>{let route=MockRequest()?;MockReply(Responder(route))?;return Ok(());}" 
    env3 = {"schema_version": 1, "mock_read_records": [], "mock_write_i64": [],
            "mock_request": [{"ok": "0"}], "mock_reply": [{"ok": None}]}
    run("A2-EX02-reduced", flow3, ["request:ok:0", "reply:200:ok"],
        "F0-0002 §12.3", outcome="ok", mock_env=env3)

    # Source is the literal §11 code fence; the recurrence and traces are
    # computed here before running either binary.
    section11 = ADR.read_text(encoding="utf-8").split("## 11. EX04 reducido ejecutable", 1)[1]
    ex04 = re.search(r"```nexo\n(.*?)\n```", section11, re.S).group(1)
    def ex04_case(id: str, request: dict, writes: list[dict], *, early_break=False):
        source = ex04.replace("MockWriteI64(valor)?;", "MockWriteI64(valor)?;break;") if early_break else ex04
        value = request.get("ok")
        error = request.get("err")
        events = [f"request:{'err' if error is not None else 'ok'}:{error if error is not None else value}"]
        stdout: list[str] = []
        result_error = int(error) if error is not None else None
        if result_error is None and not 0 <= int(value) <= 1000:
            result_error = 41
        if result_error is None:
            events.append("open:0:4")
            square, odd = 0, 1
            for i in range(int(value)):
                events.append("inspect:0:4")
                response = writes[i] if i < len(writes) else {"err": "-9000"}
                if "err" in response:
                    result_error = int(response["err"])
                    events.append(f"write_i64:{square}:err:{result_error}")
                    break
                events.append(f"write_i64:{square}:ok")
                stdout.append(str(square))
                if early_break:
                    break
                square += odd
                odd += 2
            events.append("close:0:4")
        prefix = lambda event: "mock-resource:" if event.startswith(("open:", "inspect:", "close:")) else "mock-io:"
        byte_count = sum(len((prefix(e) + e + "\n").encode()) for e in events)
        byte_count += sum(len((line + "\n").encode()) for line in stdout)
        env = {"schema_version": 1, "mock_read_records": [], "mock_write_i64": writes,
               "mock_request": [request], "mock_reply": []}
        run(id, source, events, "F0-0003 §11", outcome="err" if result_error is not None else "ok",
            error=result_error, stdout=stdout, mock_env=env,
            observation_bytes=byte_count)
    ex04_case("EX04-complete", {"ok": "3"}, [{"ok": None}] * 3)
    ex04_case("EX04-empty", {"ok": "0"}, [])
    ex04_case("EX04-request-error", {"err": "10"}, [])
    ex04_case("EX04-domain-low", {"ok": "-1"}, [])
    ex04_case("EX04-domain-high", {"ok": "1001"}, [])
    ex04_case("EX04-write-error", {"ok": "3"}, [{"ok": None}, {"err": "40"}, {"ok": None}])
    ex04_case("EX04-exhausted", {"ok": "1"}, [])
    ex04_case("EX04-break", {"ok": "3"}, [{"ok": None}] * 3, early_break=True)
    return rows


def ticks_cases() -> list[dict]:
    rows = []
    for line in ADR.read_text(encoding="utf-8").splitlines()[768:782]:
        cells = [part.strip() for part in line.strip().strip("|").split("|")]
        id = cells[0].split()[0]
        rows.append(case(id, cells[1][1:-1], "run", {"exit": 0, "diagnostics": [],
                         "outcome": "unit", "events": [], "stdout": [],
                         "work_ticks": int(cells[-1])}, "F0-0003 §7.2",
                         kind="andamiaje"))
    assert len(rows) == 14
    return rows


def callonce_cases() -> list[dict]:
    prefix = "fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};"
    def diag(code: str, subcause: str | None, start: int, end: int) -> dict:
        return {"code": code, "subcause": subcause, "primary": [start, end]}
    definitely = lambda start, end: diag("NX-E0202", "definitely_consumed", start, end)
    maybe = lambda start, end: diag("NX-E0202", "maybe_consumed", start, end)
    mode = lambda start, end: diag("NX-E1107", None, start, end)
    cases = [
        ("callonce-consumed-twice", prefix + "move f();f();}", [definitely(67, 68)]),
        ("callonce-while-after", prefix + "move f();f();var i=0;while i<1{i=i+1;}}", [definitely(67, 68)]),
        ("callonce-while-before", "fn Main(){var i=0;while i<1{i=i+1;}let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();f();}", [definitely(92, 93)]),
        ("callonce-while-inside", prefix + "var i=0;while i<2{move f();f();i=i+1;}}", [maybe(76, 84), definitely(85, 86)]),
        ("callonce-after-while", prefix + "var i=0;while i<1{move f();i=i+1;}f();}", [maybe(76, 84), maybe(92, 93)]),
        ("callonce-branch", prefix + "if true{move f();}f();}", [maybe(76, 77)]),
        ("callonce-branch-while", prefix + "if true{move f();}f();var i=0;while i<1{i=i+1;}}", [maybe(76, 77)]),
        ("callonce-transfer", prefix + "let g=move f;f();}", [definitely(71, 72)]),
        ("callonce-hypothetical-return", prefix + "move f();return;f();}", [definitely(74, 75)]),
        ("callonce-two-after-consume", prefix + "move f();f();f();}", [definitely(67, 68), definitely(71, 72)]),
        ("callonce-mode-then-consume", prefix + "f();move f();f();}", [mode(58, 59), definitely(71, 72)]),
        ("callonce-mode-then-consume-while", prefix + "f();move f();f();var i=0;while i<1{i=i+1;}}", [mode(58, 59), definitely(71, 72)]),
        ("callonce-mode-twice", prefix + "f();f();}", [mode(58, 59), mode(62, 63)]),
        ("callonce-mode-single", prefix + "f();}", [mode(58, 59)]),
        ("callonce-edit-after-consume", "fn Main(){let r=MockOpen(1);var f=fn [move r](){Drop(r);};move f();edit f();}", [definitely(72, 73)]),
        ("callonce-edit-mode", "fn Main(){let r=MockOpen(1);var f=fn [move r](){Drop(r);};edit f();}", [mode(63, 64)]),
        ("callonce-invalid-copy", prefix + "let g=f;g();f();}", [diag("NX-E0201", None, 64, 65), mode(66, 67), mode(70, 71)]),
    ]
    rows = []
    for id, source, diagnostics in cases:
        for action in ("check", "run"):
            expected = {"exit": 1, "outcome": "rejected", "diagnostics": diagnostics,
                        "events": [], "stdout": [], "reserved_observation_bytes": 0}
            if id == "callonce-consumed-twice":
                expected["diagnostic_full"] = [{
                    "schema_version": 1, "diagnostic_namespace": "Nexo-proposed-v1",
                    "profile_id": PROFILE, "code": "NX-E0202",
                    "subcause": "definitely_consumed", "rule": "NX-E0202/definitely_consumed",
                    "explanation_id": "NX-E0202/definitely_consumed", "severity": "error",
                    "stage": "ownership", "message": "Uso que exige valor disponible tras consumo: definitely_consumed si se consumió en todos los caminos pertinentes; maybe_consumed si al menos una continuación lo consumió. Transferencia, drop y extracción consumidora son orígenes, no códigos diferentes.",
                    "operation": "ownership.use",
                    "primary_span": {"file": "<memory>", "start": 67, "end": 68},
                    "related_spans": [
                        {"role": "declaration", "span": {"file": "<memory>", "start": 32, "end": 33}},
                        {"role": "consumption", "span": {"file": "<memory>", "start": 58, "end": 66}}],
                    "cause_chain": [{"kind": "consume", "operation": "value.consume",
                                     "span": {"file": "<memory>", "start": 58, "end": 66},
                                     "entity": {"kind": "binding", "id": "binding:32"},
                                     "evidence": "verified"}],
                    "feature_id": None, "unexpected_token": None,
                    "expected_tokens": [], "expected_tokens_kind": None,
                    "parser_context": None, "recognized_prefix": None,
                    "repair": [], "data": {}}]
            rows.append(case(id + "-" + action, source, action, expected,
                             "F0-0003 §§4.2,8.1–8.2,9.1–9.2"))
    source = prefix + "move f();}"
    rows.append(case("callonce-consumer-positive", source, "run", {
        "exit": 0, "diagnostics": [], "outcome": "unit", "events": ["open:0:1", "close:0:1"],
        "stdout": [], "reserved_observation_bytes": 0}, "F0-0003 §§4.2,9.2"))
    return rows


def limits() -> list[dict]:
    rows = [
        {"id": "J16-STACK-204-KIB", "source": None, "feature_id": "parser_stack_margin",
         "pending_scope": None, "owner": "J16 B1–B2", "status": "PASA",
         "later_limit": None, "stack_kib": 204, "profiles": ["debug", "release"],
         "evidence": "scripts/j16_stack_cases.py --closefix", "families": 24,
         "repetitions_per_family": 10},
        {"id": "J16-STACK-256-KIB", "source": None, "feature_id": "parser_stack_contract",
         "pending_scope": None, "owner": "J16 B1–B2", "status": "PASA",
         "later_limit": None, "stack_kib": 256, "profiles": ["debug", "release"],
         "evidence": "scripts/j16_stack_cases.py --closefix", "families": 24,
         "repetitions_per_family": 10},
    ]
    for id, scope, owner in [
        ("J16-S1C-SOME-TYPES", "Option/Result inferidos, Some×84/×126 y selection_type", "I07c"),
        ("J16-S1C-MATCH-TYPE-DEPTH", "patrón profundo de Option/Result en TYPES", "I07c"),
        ("J16-S1C-ARRAY-HELPER-MARGEN", "helper Array×126 y margen medido", "I07c"),
        ("EX04-full", "módulos, u32, consola y rango", "fase posterior"),
        ("L01", "for edit", "later_adr"),
        ("L02", "Array de closure por valor", "later_adr"),
        ("G08", "ciclo de estados", "later_adr"),
        ("Array-iterable", "Array por valor", "later_adr"),
        ("user-types", "tipos de usuario", "later_adr"),
        ("text", "texto y parsing", "later_adr"),
        ("callbacks", "callbacks escritos", "later_adr"),
        ("regions", "regiones, retains y from", "later_adr"),
        ("publication", "publicación y recarga", "later_adr"),
        ("F0-0005", "capa posterior", "F0-0005"),
        ("F0-0004", "capa posterior", "F0-0004"),
        ("T01-T28", "puertas completas", "propietario"),
        ("F0-approval", "dos revisores y aprobación de fase", "propietario"),
    ]:
        later_limit = ("aplazada a I07c: " + scope) if id.startswith("J16-S1C-") else scope
        rows.append(dict(id=id, source=None, feature_id=id, pending_scope=scope,
                         owner=owner, status="NO EVALUADO", later_limit=later_limit))
    return rows


def boundary_cases() -> list[dict]:
    rows = []
    for depth in (127, 128, 129):
        count = depth - 1
        source = "fn F(x:view " + "Array<" * count + "I64" + ">" * count + "){}fn Main(){}"
        expected = {"exit": 0, "diagnostics": []} if depth < 129 else {
            "exit": 1, "diagnostics": [{"code": "NX-E0003", "subcause": None,
            "primary": [780, 783], "data": {"budget_kind": "syntax_nesting",
            "limit": 128, "observed": 129, "budget_scope": "compilation",
            "incomplete_analysis": "parse"}}]}
        rows.append(case(f"type-depth-{depth}", source, "check", expected,
                         "F0-0003 §§10.3,13.2", kind="andamiaje"))
    for n in (63, 64, 65):
        source = "gen fn G()->I64{}fn Main(){" + "".join(f"let g{i}=G();" for i in range(n))
        source += "".join(f"Drop(g{i});" for i in range(n)) + "}"
        expected = {"exit": 0, "diagnostics": [], "outcome": "unit", "events": [], "stdout": []}
        if n == 65:
            anchor = source.index("let g64=") + len("let g64=")
            expected = {"exit": 1, "diagnostics": [{"code": "R0002", "subcause": None,
                        "primary": [anchor, anchor + 3], "data": {
                        "runtime_budget_kind": "live_value_cells", "limit": 64,
                        "observed": 65}}], "outcome": "simulator_fault", "events": [], "stdout": []}
        row = case(f"state-cells-{n}", source, "run", expected,
                   "F0-0003 §7.2", kind="andamiaje")
        row["profile"] = "f0-0003-b-test-cells-64-v1"
        rows.append(row)
    for n in (63, 64):
        source = "gen fn G()->I64{}fn Main(){" + "".join(f"var g{i}=G();" for i in range(n))
        source += "let item=Iterator.Next(edit g0);" + "".join(f"Drop(g{i});" for i in range(n)) + "}"
        expected = {"exit": 0, "diagnostics": [], "outcome": "unit",
                    "events": [], "stdout": [], "reserved_observation_bytes": 0}
        if n == 64:
            expected = {"exit": 1, "diagnostics": [{"code": "R0002", "subcause": None,
                        "data": {"runtime_budget_kind": "live_value_cells", "limit": 64,
                                 "observed": 65}}], "events": [], "stdout": [],
                        "reserved_observation_bytes": 0}
        row = case(f"shared-next-cells-{n}", source, "run", expected,
                   "F0-0003 §§7.2,13.2", kind="andamiaje")
        row["profile"] = "f0-0003-b-test-cells-64-v1"
        rows.append(row)
    frame_lines = ADR.read_text(encoding="utf-8").splitlines()[797:800]
    for depth, line in zip((7, 8, 9), frame_lines):
        source = re.search(r"`([^`]+)`", line).group(1)
        expected = {"exit": 0, "diagnostics": [], "outcome": "unit", "events": [], "stdout": []}
        if depth == 9:
            expected = {"exit": 1, "diagnostics": [{"code": "R0002", "subcause": None,
                        "primary": [64, 68], "data": {"runtime_budget_kind": "call_frames",
                        "limit": 8, "observed": 9}}], "events": [], "stdout": []}
        row = case(f"active-frames-{depth}", source, "run", expected,
                   "F0-0003 §7.2", kind="andamiaje")
        row["profile"] = "f0-0003-b-test-frames-8-v1"
        rows.append(row)
    for n in SIZES:
        source = "fn Main(){match 0{" + "".join(f"{i}=>{{}}," for i in range(n)) + "_=>{}}}"
        rows.append(case(f"i64-residual-{n}", source, "run", {"exit": 0,
                         "diagnostics": [], "outcome": "unit", "events": [], "stdout": []},
                         "F0-0003 §§3.3,13.2"))
    retained = "gen fn G(r:move MockResource)->I64{}fn Main(){let g=G(MockOpen(4));Drop(g);}"
    row = case("reservation-retained-close", retained, "run", {"exit": 0,
               "diagnostics": [], "outcome": "unit", "events": ["open:0:4", "close:0:4"],
               "stdout": [], "observation_bytes": 47, "reserved_observation_bytes": 0},
                "F0-0003 §7.2", kind="andamiaje")
    row["profile"] = "f0-0003-b-test-observations-64-v1"
    rows.append(row)
    overflow = "gen fn G(r:move MockResource)->I64{}fn Main(){let a=G(MockOpen(4));let b=G(MockOpen(5));Drop(a);Drop(b);}"
    row = case("reservation-shared-limit", overflow, "run", {"exit": 1,
               "diagnostics": [{"code": "R0002", "subcause": None,
                   "data": {"runtime_budget_kind": "observation_bytes", "limit": 64,
                            "observed": 94}}],
               "events": ["open:0:4", "close:0:4"], "stdout": [],
               "observation_bytes": 47, "reserved_observation_bytes": 0},
                "F0-0003 §7.2", kind="andamiaje")
    row["profile"] = "f0-0003-b-test-observations-64-v1"
    rows.append(row)
    return rows


def fuzz_cases() -> list[dict]:
    base = "gen fn G()->MockResource{yield MockOpen(1);yield MockOpen(2);}fn Main(){for r in G(){Inspect(r);break;}}"
    rows = []
    state = SEED
    for index in range(8):
        state = (1664525 * state + 1013904223) & 0xffffffff
        length = state % len(base)
        source = base[:length]
        rows.append(case(f"fuzz-prefix-{index}", source, "check",
                         {"robustness": True}, "F0-0003 §13.2"))
    for id, source in (
        ("fuzz-token", base.replace("yield", "yie@d", 1)),
        ("fuzz-unicode-comment", "// árbol Δ\r\n" + base),
        ("fuzz-crlf", base.replace(";", ";\r\n")),
    ):
        rows.append(case(id, source, "check", {"robustness": True},
                         "F0-0003 §13.2"))
    invalid = base.encode("utf-8")[:40] + b"\xff" + base.encode("utf-8")[40:]
    row = case("fuzz-invalid-utf8", base, "check", {"robustness": True},
               "F0-0003 §13.2", kind="andamiaje")
    row["source_bytes_hex"] = invalid.hex()
    row["source_sha256"] = hashlib.sha256(invalid).hexdigest()
    rows.append(row)
    callonce_prefix = "fn Main(){let r=MockOpen(1);var f=fn [move r](){Drop(r);};"
    operations = ("f();", "edit f();", "move f();", "return;")
    positions = ("", "var i=0;while i<1{i=i+1;}",
                 "if true{move f();}", "var i=0;while i<1{move f();i=i+1;}")
    for index in range(32):
        state = (1664525 * state + 1013904223) & 0xffffffff
        first = operations[(state >> 12) % len(operations)]
        state = (1664525 * state + 1013904223) & 0xffffffff
        second = operations[(state >> 12) % len(operations)]
        source = callonce_prefix + positions[(state >> 20) % len(positions)] + first + second + "}"
        rows.append(case(f"fuzz-callonce-{index}", source, "check",
                         {"robustness": True}, "F0-0003 §§4.2,9.2,13.2"))
    return rows


def evaluate(binary: Path, fixture: dict, directory: Path) -> dict:
    source = fixture["source"]
    source_bytes = (bytes.fromhex(fixture["source_bytes_hex"])
                    if "source_bytes_hex" in fixture else source.encode("utf-8"))
    path = directory / (fixture["id"] + ".n")
    path.write_bytes(source_bytes)
    try:
        command = [str(binary), fixture["action"], str(path), "--profile",
                   fixture["profile"], "--format", "json"]
        if "mock_env" in fixture:
            env_path = directory / (fixture["id"] + ".json")
            env_path.write_text(json.dumps(fixture["mock_env"], separators=(",", ":")),
                                encoding="utf-8")
            command.extend(["--mock-env", str(env_path)])
        run = subprocess.run(command, capture_output=True,
                             timeout=20, check=False)
        report = json.loads(run.stdout)
        def normalize(value):
            if isinstance(value, dict):
                return {key: ("<memory>" if key == "file" else normalize(item))
                        for key, item in value.items()}
            if isinstance(value, list):
                return [normalize(item) for item in value]
            return value
        diagnostics = [normalize(d) for d in report["diagnostics"]]
        observed = {"exit": run.returncode, "outcome": report["outcome"]["kind"],
                    "outcome_detail": report["outcome"],
                    "events": report["events"], "stdout": report["stdout"],
                    "observations": report["observations"],
                    "observation_bytes": report["observation_bytes"],
                    "reserved_observation_bytes": report["reserved_observation_bytes"],
                    "diagnostic_full": diagnostics,
                    "diagnostics": [{"code": d["code"], "subcause": d["subcause"],
                                     "primary": [d["primary_span"]["start"], d["primary_span"]["end"]]
                                     if d["primary_span"] else None, "data": normalize(d["data"]),
                                     "feature_id": d["feature_id"]} for d in diagnostics]}
    except (subprocess.TimeoutExpired, ValueError, KeyError) as error:
        return dict(status="FALLA", observed={"harness_error": type(error).__name__})
    expected = fixture["expected"]
    faults = []
    if expected.get("robustness"):
        if run.returncode not in (0, 1, 2) or observed["outcome"] == "internal_error":
            faults.append("robustness.exit")
        for diagnostic in diagnostics:
            if diagnostic["code"] in ("InvalidIr", "internal_error"):
                faults.append("robustness.internal_diagnostic")
            span = diagnostic.get("primary_span")
            if span and not (0 <= span["start"] <= span["end"] <= len(source_bytes)):
                faults.append("robustness.primary_span")
        return dict(status="PASA" if not faults else "FALLA", observed=observed, mismatches=faults)
    for field in ("exit", "outcome", "outcome_detail", "events", "stdout",
                  "observation_bytes", "reserved_observation_bytes", "diagnostic_full"):
        if field in expected and expected[field] != observed[field]:
            faults.append(field)
    if len(expected["diagnostics"]) != len(observed["diagnostics"]):
        faults.append("diagnostic_count")
    else:
        for e, o in zip(expected["diagnostics"], observed["diagnostics"]):
            faults.extend("diagnostic." + field for field, value in e.items() if o.get(field) != value)
    for diagnostic in diagnostics:
        if set(diagnostic) != DIAGNOSTIC_FIELDS:
            faults.append("diagnostic.schema_fields")
        rule = diagnostic["code"] + ("/" + diagnostic["subcause"] if diagnostic["subcause"] else "")
        nx = diagnostic["code"].startswith("NX-")
        common = {"schema_version": 1, "profile_id": fixture["profile"],
                  "diagnostic_namespace": "Nexo-proposed-v1" if nx else "F0-0002-sim",
                  "rule": rule, "explanation_id": rule if nx else None,
                  "severity": "error", "unexpected_token": None,
                  "expected_tokens": [], "expected_tokens_kind": None,
                  "parser_context": None, "recognized_prefix": None, "repair": []}
        for field, value in common.items():
            if diagnostic.get(field) != value:
                faults.append("diagnostic.schema." + field)
        for field in ("related_spans", "cause_chain"):
            if not isinstance(diagnostic.get(field), list):
                faults.append("diagnostic.schema." + field)
    return dict(status="PASA" if not faults else "FALLA", observed=observed, mismatches=faults)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--debug", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "debug/nexo.exe")
    parser.add_argument("--release", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "release/nexo.exe")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    fixtures = (section_nine_pairs() + ticks_cases() + modeled_cases()
                + boundary_cases() + extra_cases() + callonce_cases() + fuzz_cases())
    for item in fixtures:
        item["class"] = item.pop("class_")
    with tempfile.TemporaryDirectory(prefix="j16-oracle-", dir=os.environ.get("NEXO_J16_TMP", None)) as tmp:
        directory = Path(tmp)
        for fixture in fixtures:
            for mode, binary in (("debug", args.debug), ("release", args.release)):
                fixture[mode] = evaluate(binary, fixture, directory)
            fixture["status"] = "PASA" if all(fixture[mode]["status"] == "PASA" for mode in ("debug", "release")) else "FALLA"
    document = {"schema_version": 1, "seed": SEED, "sizes": list(SIZES),
                "cases": fixtures, "limits": limits()}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n")
    failed = [fixture["id"] for fixture in fixtures if fixture["status"] != "PASA"]
    print(f"J16 oracle: {len(fixtures)} cases, {len(failed)} mismatches: {', '.join(failed)}")
    return bool(failed)


if __name__ == "__main__":
    raise SystemExit(main())
