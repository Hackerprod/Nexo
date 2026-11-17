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
    return cases


def ticks_cases() -> list[dict]:
    rows = []
    for line in ADR.read_text(encoding="utf-8").splitlines()[768:782]:
        cells = [part.strip() for part in line.strip().strip("|").split("|")]
        id = cells[0].split()[0]
        rows.append(case(id, cells[1][1:-1], "run", {"exit": 0, "diagnostics": [],
                         "outcome": "unit", "events": [], "stdout": [],
                         "work_ticks": int(cells[-1])}, "F0-0003 §7.2"))
    assert len(rows) == 14
    return rows


def limits() -> list[dict]:
    rows = []
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
        rows.append(case(f"type-depth-{depth}", source, "check", expected, "F0-0003 §§10.3,13.2"))
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
        row = case(f"state-cells-{n}", source, "run", expected, "F0-0003 §7.2")
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
        row = case(f"active-frames-{depth}", source, "run", expected, "F0-0003 §7.2")
        row["profile"] = "f0-0003-b-test-frames-8-v1"
        rows.append(row)
    for n in SIZES:
        source = "fn Main(){match 0{" + "".join(f"{i}=>{{}}," for i in range(n)) + "_=>{}}}"
        rows.append(case(f"i64-residual-{n}", source, "run", {"exit": 0,
                         "diagnostics": [], "outcome": "unit", "events": [], "stdout": []},
                         "F0-0003 §§3.3,13.2"))
    return rows


def evaluate(binary: Path, fixture: dict, directory: Path) -> dict:
    source = fixture["source"]
    path = directory / (fixture["id"] + ".n")
    path.write_bytes(source.encode("utf-8"))
    try:
        run = subprocess.run([str(binary), fixture["action"], str(path), "--profile",
                              fixture["profile"], "--format", "json"], capture_output=True,
                             timeout=20, check=False)
        report = json.loads(run.stdout)
        def normalize(value):
            if isinstance(value, dict):
                return {key: ("<memory>" if key == "file" else normalize(item))
                        for key, item in value.items()}
            if isinstance(value, list):
                return [normalize(item) for item in value]
            return value
        observed = {"exit": run.returncode, "outcome": report["outcome"]["kind"],
                    "events": report["events"], "stdout": report["stdout"],
                    "diagnostics": [{"code": d["code"], "subcause": d["subcause"],
                                     "primary": [d["primary_span"]["start"], d["primary_span"]["end"]]
                                     if d["primary_span"] else None, "data": normalize(d["data"]),
                                     "feature_id": d["feature_id"]} for d in report["diagnostics"]]}
    except (subprocess.TimeoutExpired, ValueError, KeyError) as error:
        return dict(status="FALLA", observed={"harness_error": type(error).__name__})
    expected = fixture["expected"]
    faults = []
    for field in ("exit", "outcome", "events", "stdout"):
        if field in expected and expected[field] != observed[field]:
            faults.append(field)
    if len(expected["diagnostics"]) != len(observed["diagnostics"]):
        faults.append("diagnostic_count")
    else:
        for e, o in zip(expected["diagnostics"], observed["diagnostics"]):
            faults.extend("diagnostic." + field for field, value in e.items() if o.get(field) != value)
    return dict(status="PASA" if not faults else "FALLA", observed=observed, mismatches=faults)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--debug", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "debug/nexo.exe")
    parser.add_argument("--release", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "release/nexo.exe")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    fixtures = section_nine_pairs() + ticks_cases() + modeled_cases() + boundary_cases()
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
