"""Adversarial checks on the independent judge and ADR transcriptions."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

from .charges import Charges, Limits, BudgetFault, scalar_loop_total, short_circuit_total
from .diagnostics import RuleViolation, SourceChecker, missing_main, pending, syntax
from .diagnostics import check_source
from .generate import audit_seed, corpus_sha, make_invalid, make_valid, minimize_case, render_expr
from .model import (
    Array, Expr, MAX_I64, MIN_I64, ModelFault, Program, Stmt, binary,
    SourceSyntaxFault, SourceVM, copy_value, evaluate, execute, parse_source, quotient,
    remainder, run_source, type_check,
)

ROOT = Path(__file__).resolve().parents[2]
ADR_PATH = "docs/adr/F0-0005-operators-control-collections.md"
FIXTURE_PATH = ROOT / "tests/fixtures/f5_oracle_literals.json"


def check_literal_integrity() -> tuple[int, int]:
    fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
    blob = subprocess.run(["git", "show", "HEAD:" + ADR_PATH], cwd=ROOT,
                          check=True, capture_output=True).stdout
    assert hashlib.sha256(blob).hexdigest() == fixture["adr_blob_sha256"], \
        "ADR blob changed: literal fixture needs independent review"
    adr = blob.decode("utf-8")
    assert len(fixture["rows"]) == 52
    for identifier, row in fixture["rows"].items():
        assert row["normative_row"] in adr, identifier
        assert row["normative_row"].startswith("| " + identifier + " | ") or \
            identifier == "F5-B10", identifier
        assert row["code"] and row["primary"], identifier
        source = row["source"].encode("utf-8")
        start, end = row["primary"]
        assert 0 <= start < end <= len(source), identifier
        # Every primary must point to precisely the token printed in the ADR.
        import re
        tokens = re.findall(r"`([^`]+)`", row["normative_row"])
        if identifier == "F5-B10":
            assert source[start:end] == b"JoinArrays"
        else:
            assert len(tokens) >= 2 and source[start:end].decode("utf-8") == tokens[1], identifier
    assert len(fixture["syntax"]) == 18
    for identifier, row in fixture["syntax"].items():
        assert row["normative_row"] in adr and row["source"], identifier
    assert len(fixture["behavior_programs"]) == 3
    assert len(fixture["acceptance_programs"]) == 9
    for identifier, source in fixture["acceptance_programs"].items():
        assert identifier in adr and source.rstrip("\n") in adr, identifier
    assert len(fixture["json_literals"]) == 17
    for identifier, expected in fixture["json_literals"].items():
        assert identifier in adr and expected["profile_id"] == "f0-0005-b-v1"
    return len(fixture["rows"]), len(fixture["syntax"])


def check_diagnostic_literals() -> int:
    fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
    literal = fixture["json_literals"]
    pending_cases = {
        "F5-H01": ("L08", "lower", 16, 19),
        "F5-H02": ("L01", "types", 17, 18),
        "F5-H04": ("L01", "types", 20, 21),
        "F5-H06": ("L06", "types", 10, 13),
        "F5-H07": ("L07", "types", 20, 24),
        "F5-H08": ("L01", "types", 17, 19),
        "F5-H09": ("L09", "types", 0, 5),
        "F5-H10": ("L08", "lower", 0, 5),
        "F5-H11": ("L10", "lower", 21, 22),
    }
    checked = 0
    for identifier, args in pending_cases.items():
        assert pending(*args) == literal[identifier], identifier
        checked += 1
    for identifier in ("F5-S14", "F5-S15", "F5-S16", "F5-S17", "F5-S18"):
        diag = literal[identifier]
        token = diag["unexpected_token"]
        a, b = diag["recognized_prefix"]["start"], diag["recognized_prefix"]["end"]
        assert syntax(token["span"]["start"], token["span"]["end"], token["text"],
                      token["kind"], diag["expected_tokens"], diag["parser_context"],
                      a, b) == diag, identifier
        source = fixture["syntax"][identifier]["source"].encode("utf-8")
        assert source[token["span"]["start"]:token["span"]["end"]].decode() == token["text"]
        try:
            parse_source(source.decode("utf-8"))
        except SourceSyntaxFault as fault:
            assert (fault.token.text, fault.token.start, fault.token.end,
                    fault.context, fault.expected, fault.prefix_start, fault.prefix_end) == (
                token["text"], token["span"]["start"], token["span"]["end"],
                diag["parser_context"], diag["expected_tokens"], a, b), identifier
        else:
            raise AssertionError(f"syntax literal accepted: {identifier}")
        checked += 1
    return checked


def check_integer_rules() -> int:
    signs = [(-7, 3, -2, -1), (7, -3, -2, 1), (-7, -3, 2, -1)]
    for a, b, q, r in signs:
        assert quotient(a, b) == q
        assert remainder(a, b) == r
        assert a == q * b + r
    assert remainder(MIN_I64, -1) == 0
    for operation, args, code in [
        (quotient, (7, 0), "R0502"),
        (quotient, (MIN_I64, -1), "R0503"),
        (remainder, (7, 0), "R0504"),
        (lambda a, b: binary("*", a, b), (MAX_I64, 2), "R0501"),
        (lambda a, b: binary("+", a, b), (MAX_I64, 1), "R0101"),
        (lambda a, b: binary("-", a, b), (MIN_I64, 1), "R0103"),
    ]:
        try:
            operation(*args)
        except ModelFault as fault:
            assert fault.code == code
        else:
            raise AssertionError(code)
    return len(signs) * 2 + 1 + 6


def check_control_and_arrays() -> int:
    false = Expr("bool", "Bool", value=False)
    true = Expr("bool", "Bool", value=True)
    missing = Expr("name", "Bool", value="missing")
    assert evaluate(Expr("binary", "Bool", (false, missing), "and"), {}, []) is False
    assert evaluate(Expr("binary", "Bool", (true, missing), "or"), {}, []) is True
    try:
        type_check(Expr("binary", "Bool", (false, missing), "and"), {})
    except KeyError:
        pass
    else:
        raise AssertionError("skipped RHS was not checked statically")
    a = Array([Array([1], 1)], 1)
    b = copy_value(a)
    b.items[0].items.append(2)
    assert a.items[0].items == [1] and b.items[0].items == [1, 2]
    for op, lo, hi, expected in [
        ("..<", 2, 2, []), ("..=", 2, 2, [2]),
        ("..=", 3, 2, []), ("..=", MAX_I64, MAX_I64, [MAX_I64]),
    ]:
        program = Program((Stmt("for_range", "i", Expr("int", "I64", value=lo),
                                body=(), op=op, upper=Expr("int", "I64", value=hi)),))
        got = execute(program)
        assert [int(line[5:]) for line in got.journal if line.startswith("iter:")] == expected
    return 8


def check_charge_literals() -> int:
    assert scalar_loop_total(3, False) == 19
    assert scalar_loop_total(3, True) == 20
    for left, op, expected in [(False, "and", 7), (True, "and", 9),
                               (True, "or", 7), (False, "or", 9)]:
        assert short_circuit_total(left, op) == expected
    charges = Charges(Limits(storage_bytes=96, allocation_bytes=96))
    charges.allocate_array(1)
    assert charges.grow_array(1, 1) == 2
    assert charges.storage_bytes == 64 and charges.allocation_bytes == 96
    try:
        charges.grow_array(2, 2)
    except BudgetFault as fault:
        assert fault.kind == "array_storage_bytes" and fault.observed == 192
    else:
        raise AssertionError("storage growth boundary")
    assert charges.storage_bytes == 64 and charges.allocation_bytes == 96
    return 8


def check_generator() -> int:
    checksum = corpus_sha(0, 100, 100)
    assert checksum == "6eea5d276377afc36b3e2f312f2c8f5c3cbe6c3dea4ef4ebc7cbd01d3e7e93c7"
    assert "for i in 0..<1" in make_valid(0, 220).source
    for index in range(100):
        valid = make_valid(0, index)
        if valid.program.body:
            state = execute(valid.program)
            assert state.env["answer"] == valid.expected_value
        parsed = parse_source(valid.source)
        SourceChecker(parsed).check()
        observed = SourceVM(parsed).run()
        assert observed.stdout and observed.events == []
        if valid.program.body:
            assert observed.stdout == state.stdout
        else:
            assert observed.stdout == valid.expected_value
        invalid = make_invalid(0, index)
        parsed = parse_source(invalid.source)
        if invalid.expected_issue is None:
            try:
                execute(invalid.program)
            except ModelFault as fault:
                assert fault.code == invalid.expected_fault
            else:
                raise AssertionError((index, invalid.family))
            SourceChecker(parsed).check()
            try:
                SourceVM(parsed).run()
            except ModelFault as fault:
                assert fault.code == invalid.expected_fault
            else:
                raise AssertionError((index, invalid.family, "source accepted"))
        else:
            try:
                SourceChecker(parsed).check()
            except RuleViolation as failure:
                issue = failure.issue
                assert {"code": issue.code, "subcause": issue.subcause,
                        "stage": issue.stage, "operation": issue.operation,
                        "primary": [issue.start, issue.end], "data": issue.data,
                        "feature_id": issue.feature_id,
                        "related": [list(item) for item in issue.related]} == \
                    invalid.expected_issue
            else:
                raise AssertionError((index, invalid.family, "negative accepted"))
        assert invalid.positive_source is not None
        repair = parse_source(invalid.positive_source)
        SourceChecker(repair).check()
        SourceVM(repair).run()
    assert render_expr(Expr("binary", "I64", (
        Expr("int", "I64", value=2),
        Expr("binary", "I64", (Expr("int", "I64", value=3),
                                 Expr("int", "I64", value=4)), "*")), "+")) == "2+3*4"
    original = make_valid(0, 4)
    smaller, attempts = minimize_case(original, lambda case: "same_fault_path",
                                      seconds=1)
    assert attempts >= 1 and len(smaller.source) < len(original.source)
    assert execute(smaller.program).env["answer"] == smaller.expected_value
    return 202


def check_source_programs() -> int:
    fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
    adr = subprocess.run(["git", "show", "HEAD:" + ADR_PATH], cwd=ROOT,
                         check=True, capture_output=True).stdout.decode("utf-8")
    for identifier, source in fixture["acceptance_programs"].items():
        output = fixture["acceptance_stdout"][identifier]
        assert "Salida: `" + output.replace("\n", "\\n") + "`" in adr, identifier
        state = run_source(source)
        assert state.stdout == output.rstrip("\n").split("\n"), identifier
        assert state.events == [] and state.observation_bytes == \
            fixture["acceptance_observation_bytes"][identifier], identifier
    for index, source in enumerate(fixture["behavior_programs"]):
        state = run_source(source)
        expected = fixture["behavior_stdout"][index]
        assert state.stdout == ([] if not expected else expected.rstrip("\n").split("\n")), index
        assert state.events == [] and state.observation_bytes == len(expected.encode("utf-8")), index
    for source in fixture["intrinsic_effect_positives"]:
        assert source in adr
        assert check_source(source) is None
        state = run_source(source)
        assert state.stdout == [] and state.events == []
    return 14


def check_negative_literals() -> int:
    fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
    for identifier, row in fixture["rows"].items():
        actual = check_source(row["source"])
        assert actual is not None, identifier
        assert (actual.code, actual.subcause, [actual.start, actual.end]) == (
            row["code"], row["subcause"], row["primary"]), identifier
        contract = row["contract"]
        assert (actual.stage, actual.operation, actual.data,
                [list(item) for item in actual.related], actual.feature_id,
                list(actual.cause)) == (
            contract["stage"], contract["operation"], contract["data"],
            contract["related"], contract["feature_id"], contract["cause"]), identifier
    return len(fixture["rows"])


def check_positive_repairs() -> int:
    fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
    for identifier, row in fixture["rows"].items():
        positive = row["positive_source"]
        assert positive != row["source"], identifier
        assert check_source(positive) is None, identifier
        state = run_source(positive)
        assert state.stdout == [] and state.events == [], identifier
    return len(fixture["rows"])


def check_syntax_pairs() -> dict[str, int]:
    """Exercise every F5-S negative and repair, including CLI entry semantics."""
    fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
    checked = run = missing_main_count = 0
    for identifier, row in fixture["syntax"].items():
        source = row["source"]
        contract = row["contract"]
        try:
            parse_source(source)
        except SourceSyntaxFault as fault:
            token = fault.token
            assert (fault.context, fault.expected, fault.prefix_start,
                    fault.prefix_end, token.start, token.end) == (
                contract["context"], contract["expected_tokens"],
                *contract["prefix"], *contract["primary"]), identifier
            assert ("" if token.text == "<EOF>" else token.text) == \
                contract["token_text"], identifier
        else:
            raise AssertionError(f"syntax negative accepted: {identifier}")
        positive = row["positive_source"]
        program = parse_source(positive)
        SourceChecker(program).check()
        checked += 1
        if "Main" not in program.functions:
            # D15 §7.2: check accepts a library; run requires Main. S08 is
            # the only library among the normative F5-S repairs.
            assert identifier == "F5-S08" and positive == "fn F(x:view I64){}"
            assert len(positive.encode("utf-8")) == 18
            diagnostic = missing_main(positive)
            assert (diagnostic["code"], diagnostic["subcause"],
                    diagnostic["stage"], diagnostic["operation"],
                    diagnostic["primary_span"], diagnostic["data"]) == (
                "NX-E0112", "missing_main", "entry", "entry.check",
                {"file": "case.n", "start": 18, "end": 18},
                {"entry": "Main"})
            missing_main_count += 1
            continue
        state = SourceVM(program).run()
        assert state.stdout == [] and state.events == [] and \
            state.observation_bytes == 0, identifier
        run += 1
    assert (checked, run, missing_main_count) == (18, 17, 1)
    return {"syntax_repairs_checked": checked,
            "syntax_repairs_run_unit": run,
            "syntax_repairs_run_missing_main": missing_main_count}


def check_runner() -> int:
    from .runner import (CAPABILITIES, _first_difference, literal_cases, manifest, normalize_files,
                         phase_h_cases, projection, requirements, seed_range)
    complete = manifest("G1b", "L01b,L02b,L03b")
    assert complete == ("L01b", "L02b", "L03b")
    for bad in ("L01b,L03b", "L01b,L02b,L03b,L04b"):
        try:
            manifest("G1b", bad)
        except ValueError:
            pass
        else:
            raise AssertionError(bad)
    loop = make_valid(0, 4)
    assert requirements(make_valid(0, 36)) == {"L01b"}
    assert requirements(loop) == {"L03b", "L05b"}
    assert "L01b" in requirements(make_invalid(0, 1))
    assert projection(loop, "check", complete)["diagnostics"][0]["data"] == \
        {"pending_scope": "L05"}
    assert projection(loop, "check", complete)["diagnostics"][0]["stage"] == "types"
    assert projection(loop, "run", manifest("G3b",
        "L01b,L02b,L03b,L04b,L05b,L07b,L06b"))["diagnostics"][0]["data"] == \
        {"pending_scope": "L08"}
    assert projection(make_valid(0, 2), "check", complete)["outcome"] == "checked"
    assert projection(make_invalid(0, 4), "check", CAPABILITIES)["diagnostics"][0][
        "code"] == "NX-E0102"
    assert projection(make_valid(0, 16), "check", CAPABILITIES[:9])["diagnostics"][0][
        "stage"] == "lower"
    assert _first_difference({"a": [1, 2]}, {"a": [1, 3]}) == "$.a[1]"
    assert normalize_files({"x": [{"file": "C:/tmp/a.n", "text": "C:/tmp/a.n"}]}) == \
        {"x": [{"file": "case.n", "text": "C:/tmp/a.n"}]}
    assert list(seed_range("0:2")) == [0, 1, 2]
    literals = list(literal_cases("f0-0005-b-v1"))
    assert len(literals) == 302
    assert not any(identifier == "F5-S08-positive" for identifier, *_ in literals)
    assert {identifier for identifier, *_ in phase_h_cases(complete)} == {
        "F5-H01", "F5-H05", "F5-H06", "F5-H07", "F5-H09"}
    return 15


def check_batch() -> int:
    from .batch import make_batches, validate_scale
    from .runner import CAPABILITIES
    assert make_batches("0:4", 2) == [[0, 1], [2, 3], [4]]
    assert make_batches("2:2", 4) == [[2]]
    assert validate_scale("G1b", "L01b,L02b,L03b", 75,
                          Path("C:/nexo-work/tmp")) == CAPABILITIES[:3]
    assert validate_scale("G6", ",".join(CAPABILITIES), 75,
                          Path("C:/nexo-work/tmp")) == CAPABILITIES
    for phase, complete, gap, path in [
        ("G6", "L01b,L02b,L03b", 75, Path("C:/nexo-work/tmp")),
        ("G1b", "L01b,L02b,L03b", 59, Path("C:/nexo-work/tmp")),
        ("G1b", "L01b,L02b,L03b", 75, Path("D:/tmp")),
    ]:
        try:
            validate_scale(phase, complete, gap, path)
        except ValueError:
            pass
        else:
            raise AssertionError((phase, complete, gap, path))
    return 7


def check_regressions() -> int:
    path = ROOT / "tests/fixtures/f5_oracle_regressions.jsonl"
    records = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]
    assert len(records) == 1
    row = records[0]
    assert row["id"] == "F5-H03-range-stored"
    assert row["source"] == "fn Main(){let r=0..<3;}"
    assert row["source_encoding"] == "ASCII-no-LF"
    assert row["expected"]["diagnostic"] == {
        "code": "NX-E0006", "subcause": None, "stage": "types",
        "operation": "types.check", "primary": [17, 20],
        "feature_id": "range", "data": {"pending_scope": "later_adr"}}
    assert row["observed"] == {"exit": 2, "outcome": "driver_error",
                               "reason": "internal_error"}
    return 1


def check_model_pairs() -> int:
    rows = json.loads((ROOT / "tests/fixtures/f5_oracle_model_pairs.json")
                      .read_text(encoding="utf-8"))
    for row in rows:
        parsed = parse_source(row["source"])
        SourceChecker(parsed).check()
        observed = SourceVM(parsed).run()
        assert observed.stdout == row["stdout"], row["id"]
        assert observed.events == [], row["id"]
    return len(rows)


def check_differential_sensitivity() -> int:
    """Two executable stand-ins prove the run comparison checks actual values."""
    from .runner import CAPABILITIES, compare_invocation, invoke
    case = make_valid(0, 0)
    assert "47%1" in case.source
    profile = "f0-0005-b-v1"
    good = SourceVM(parse_source(case.source)).run()
    assert good.stdout[0] == "0"
    base = {"schema_version": 1, "profile_id": profile,
            "outcome": {"kind": "unit"}, "stdout": good.stdout,
            "events": good.events,
            "observations": [{"channel": "stdout", "text": line}
                             for line in good.stdout],
            "observation_bytes": good.observation_bytes,
            "reserved_observation_bytes": 0, "diagnostics": []}
    temp_root = Path("C:/nexo-work/tmp")
    temp_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="f5-oracle-mock-", dir=temp_root) as temp:
        root = Path(temp)
        source = root / "case.n"
        source.write_bytes(case.source.encode("ascii"))
        for wrong in (False, True):
            document = json.loads(json.dumps(base))
            if wrong:
                document["stdout"][0] = "1"
                document["observations"][0]["text"] = "1"
            executable = root / ("bad.py" if wrong else "good.py")
            executable.write_text("import sys\nprint(" + repr(json.dumps(document)) +
                                  ")\n", encoding="utf-8")
            invocation = invoke(executable, "run", source, profile)
            difference, _ = compare_invocation(invocation, case, "run",
                                                CAPABILITIES[:8], profile)
            assert difference == ("$.stdout[0]" if wrong else None), difference
    return 2


def check_scale_hashes() -> dict[str, dict[str, int]]:
    fixture = json.loads((ROOT / "tests/fixtures/f5_oracle_corpus_sha.json")
                         .read_text(encoding="utf-8"))
    assert sys.version.split()[0] == fixture["python"] == "3.13.1"
    assert fixture["seeds"] == list(range(32))
    assert fixture["counts"]["valid"] == fixture["counts"]["invalid"] == 32000
    report = {}
    for seed in fixture["seeds"]:
        audit = audit_seed(seed, fixture["valid_per_seed"],
                           fixture["invalid_per_seed"])
        assert audit["sha256"] == fixture["hashes"][str(seed)]
        assert audit["valid_distinct"] >= 1000, seed
        assert audit["invalid_distinct"] >= 1000, seed
        assert audit["max_valid_nodes"] <= 32, seed
        report[str(seed)] = {"valid": audit["valid_distinct"],
                             "invalid": audit["invalid_distinct"],
                             "max_nodes": audit["max_valid_nodes"]}
    return report


def run() -> dict[str, int]:
    rows, syntax_rows = check_literal_integrity()
    result = {
        "literal_rows_integrity": rows,
        "syntax_rows_integrity": syntax_rows,
        "literal_diagnostics_modeled": check_diagnostic_literals(),
        "integer_rules": check_integer_rules(),
        "control_array_rules": check_control_and_arrays(),
        "charge_anchors": check_charge_literals(),
        "generated_cases_modeled": check_generator(),
        "positive_source_programs_modeled": check_source_programs(),
        "negative_literal_heads_modeled": check_negative_literals(),
        "positive_literal_repairs_modeled": check_positive_repairs(),
    }
    result.update(check_syntax_pairs())
    result["runner_contracts"] = check_runner()
    result["batch_contracts"] = check_batch()
    result["regression_records"] = check_regressions()
    result["model_pairs"] = check_model_pairs()
    result["differential_sensitivity"] = check_differential_sensitivity()
    result["distinct_by_seed"] = check_scale_hashes()
    return result
