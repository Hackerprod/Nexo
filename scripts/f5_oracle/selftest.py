"""Adversarial checks on the independent judge and ADR transcriptions."""

from __future__ import annotations

import hashlib
import io
import json
from contextlib import redirect_stdout
from pathlib import Path
import subprocess
import sys
import tempfile

from .charges import Charges, Limits, BudgetFault, scalar_loop_total, short_circuit_total
from .diagnostics import RuleViolation, SourceChecker, missing_main, pending, syntax
from .diagnostics import check_source
from .generate import Case, audit_seed, cases, corpus_sha, make_invalid, make_valid, minimize_case, render_expr
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
    # F5 §9.4 storage-96: the same first growth fails at the adjacent 95.
    lower = Charges(Limits(storage_bytes=95, allocation_bytes=96))
    lower.allocate_array(1)
    try:
        lower.grow_array(1, 1)
    except BudgetFault as fault:
        assert fault.kind == "array_storage_bytes" and fault.observed == 96
    else:
        raise AssertionError("storage limit minus one")
    assert lower.storage_bytes == 32
    return 10


def check_generator() -> int:
    checksum = corpus_sha(0, 100, 100)
    assert checksum == "c40ab3c7cfb0d0c13252d52ff21faa370e8d290804b79d1a54947b5facad668c"
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
        if invalid.expected_issue is not None and invalid.expected_issue["stage"] == "parse":
            try:
                parse_source(invalid.source)
            except SourceSyntaxFault as fault:
                assert [fault.token.start, fault.token.end] == invalid.expected_issue["primary"]
                assert fault.expected == invalid.expected_issue["expected_tokens"]
                assert fault.context == invalid.expected_issue["parser_context"]
                assert [fault.prefix_start, fault.prefix_end] == \
                    invalid.expected_issue["recognized_prefix"]
            else:
                raise AssertionError((index, "truncation accepted"))
            SourceChecker(parse_source(invalid.positive_source)).check()
            continue
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
    duplicate = make_invalid(0, 108)
    assert duplicate.family == "const.duplicate_name"
    assert duplicate.expected_issue == {
        "code": "NX-E0104", "subcause": None, "stage": "names",
        "operation": "names.declare", "primary": [29, 30],
        "data": {"declaration_kind": "constant"},
        "feature_id": None, "related": [["previous_declaration", 6, 7]]}
    issue = check_source(duplicate.source)
    assert issue is not None and issue.code == "NX-E0104"
    SourceChecker(parse_source(duplicate.positive_source)).check()
    return 203


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
    assert requirements(make_valid(0, 11)) == {"L06b"}
    assert requirements(make_valid(0, 15)) == set()
    from dataclasses import replace
    assert requirements(replace(make_valid(0, 15), source="fn Main(){let x=1<=2;}")) == {"L01b"}
    assert requirements(replace(make_valid(0, 15), source="fn Main(){let x=1<2;}")) == set()
    assert requirements(replace(make_valid(0, 15), source="fn F()->I64{while true{return 1;}}fn Main(){}")) == {"L04b"}
    # F5 §2.2, §8.2 and F5-C02: only literal true removes the false CFG edge.
    # The code, stage, operation, closing-brace span and return-type link
    # follow the ADR's NX-E1317 contract, independently of nexo.exe.
    false_source = "fn F()->I64{while false{}}fn Main(){}"
    false_expected = {"code": "NX-E1317", "subcause": None,
                      "stage": "ownership", "operation": "flow.return",
                      "primary": [25, 26], "data": {}, "feature_id": None,
                      "related": [["expected_type", 8, 11]]}
    assert false_source[25:26] == "}" and false_source[8:11] == "I64"
    issue = check_source(false_source)
    assert issue is not None
    assert {"code": issue.code, "subcause": issue.subcause,
            "stage": issue.stage, "operation": issue.operation,
            "primary": [issue.start, issue.end], "data": issue.data,
            "feature_id": issue.feature_id,
            "related": [list(item) for item in issue.related]} == false_expected
    false_case = replace(make_valid(0, 15), source=false_source,
                         expected_issue=false_expected)
    assert requirements(false_case) == set()
    assert projection(false_case, "check", complete) == {
        "exit": 1, "outcome": "rejected", "diagnostics": [false_expected]}
    for source, expected_caps in (
            ("fn F()->I64{while true{break;}}fn Main(){}", set()),
            ("fn F()->I64{while ((true)){break;}}fn Main(){}", set()),
            ("fn F()->I64{while ((true)){return 1;}}fn Main(){}", {"L04b"}),
            ("fn F()->I64{let b=false;while b{}}fn Main(){}", set()),
            ("fn F()->I64{while true{return 1;}return 2;}fn Main(){}", set())):
        assert requirements(replace(false_case, source=source)) == expected_caps, source
    for index in (27, 41, 55, 69):
        case = make_invalid(0, index)
        assert case.family == "control.while_return"
        assert case.expected_issue["code"] == "NX-E1317"
        assert requirements(case) == set()
    assert projection(make_valid(0, 10), "check", complete)["diagnostics_cardinality"] == "unspecified"
    assert projection(make_invalid(0, 10), "check", complete)["diagnostics_cardinality"] == "unspecified"
    assert requirements(make_invalid(0, 5)) == set()
    assert requirements(make_invalid(0, 6)) == set()
    assert projection(make_invalid(0, 6), "check", complete)["diagnostics"][0]["code"] == "NX-E1326"
    # F5 §9.1 and F5-O02: R errors have a valid check and a faulting run.
    fault_case = Case(0, 0, "invalid", "F5-O02", Program(()),
                      "fn Main(){let a=1/0;}", None, "R0502")
    fault_expected = {
        "exit": 1, "outcome": "simulator_fault",
        "diagnostics": [{"code": "R0502", "subcause": None,
                         "stage": "runtime", "operation": "value.div",
                         "primary": [16, 19], "data": {}, "feature_id": None}],
        "stdout": [], "events": [], "observations": [],
        "observation_bytes": 0, "reserved_observation_bytes": 0}
    fault_projection = projection(fault_case, "run", CAPABILITIES[:8])
    assert fault_projection["exit"] == fault_expected["exit"]
    assert fault_projection["outcome"] == fault_expected["outcome"]
    assert all(fault_projection[key] == fault_expected[key] for key in
               ("stdout", "events", "observations", "observation_bytes",
                "reserved_observation_bytes"))
    assert all(fault_projection["diagnostics"][0][key] == value for key, value in
               fault_expected["diagnostics"][0].items())
    # F5-N02 and §8.2: a name collision precedes pending const capabilities.
    duplicate = Case(0, 0, "invalid", "F5-N02", Program(()),
                     "const k:I64=comptime 1;const k:I64=comptime 2;fn Main(){}",
                     None, "NX-E0104", expected_issue={
                         "code": "NX-E0104", "subcause": None,
                         "stage": "names", "operation": "names.declare",
                         "primary": [29, 30],
                         "data": {"declaration_kind": "constant"},
                         "feature_id": None,
                         "related": [["previous_declaration", 6, 7]]})
    assert projection(duplicate, "check", complete) == {
        "exit": 1, "outcome": "rejected",
        "diagnostics": [duplicate.expected_issue]}
    assert make_invalid(0, 108).family == "const.duplicate_name"
    assert projection(make_invalid(0, 108), "check", complete)["diagnostics"][0][
        "code"] == "NX-E0104"
    assert normalize_files({"x": [{"file": "C:/tmp/a.n", "text": "C:/tmp/a.n"}]}) == \
        {"x": [{"file": "case.n", "text": "C:/tmp/a.n"}]}
    assert list(seed_range("0:2")) == [0, 1, 2]
    literals = list(literal_cases("f0-0005-b-v1"))
    assert len(literals) == 302
    assert not any(identifier == "F5-S08-positive" for identifier, *_ in literals)
    assert {identifier for identifier, *_ in phase_h_cases(complete)} == {
        "F5-H01", "F5-H05", "F5-H06", "F5-H07", "F5-H09"}
    return 18


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
        vm = SourceVM(parsed)
        try:
            observed = vm.run()
        except ModelFault as fault:
            assert row.get("fault") == fault.code, row["id"]
            observed = vm
        else:
            assert "fault" not in row, row["id"]
        assert observed.stdout == row["stdout"], row["id"]
        assert observed.events == row.get("events", []), row["id"]
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


def check_full_compare_mock() -> dict[str, int]:
    """Exercise run_compare in G4 with independent stand-in responses."""
    from unittest.mock import patch
    from . import runner

    profile = "f0-0005-b-v1"
    complete = runner.manifest("G4", ",".join(runner.CAPABILITIES[:8]))
    generated = list(cases(0, 18, 14))
    by_source = {case.source: case for case in generated}
    results = {}
    temp_root = Path("C:/nexo-work/tmp")
    temp_root.mkdir(parents=True, exist_ok=True)

    def document(expected: dict) -> dict:
        diagnostics = []
        for row in expected.get("diagnostics", []):
            primary = row.get("primary", [0, 0])
            token = row.get("unexpected_token")
            prefix = row.get("recognized_prefix")
            diagnostics.append({
                "code": row["code"], "subcause": row.get("subcause"),
                "stage": row["stage"], "operation": row["operation"],
                "feature_id": row.get("feature_id"), "data": row.get("data", {}),
                "profile_id": row.get("profile_id"), "rule": row.get("rule"),
                "explanation_id": row.get("explanation_id"),
                "diagnostic_namespace": row.get("diagnostic_namespace"),
                "severity": row.get("severity"),
                "expected_tokens": row.get("expected_tokens"),
                "expected_tokens_kind": row.get("expected_tokens_kind"),
                "parser_context": row.get("parser_context"),
                "unexpected_token": None if token is None else {
                    "kind": token["kind"], "text": token["text"],
                    "span": {"file": "case.n", "start": token["span"][0],
                             "end": token["span"][1]}},
                "recognized_prefix": None if prefix is None else {
                    "file": "case.n", "start": prefix[0], "end": prefix[1]},
                "primary_span": {"file": "case.n", "start": primary[0], "end": primary[1]},
                "related_spans": [{"role": item[0], "span": {"file": "case.n",
                    "start": item[1], "end": item[2]}} for item in row.get("related", [])],
                "cause_chain": row.get("cause", []),
            })
        return {"schema_version": 1, "profile_id": profile,
                "outcome": {"kind": expected["outcome"]},
                "stdout": list(expected.get("stdout", [])),
                "events": list(expected.get("events", [])),
                "observations": list(expected.get("observations", [])),
                "observation_bytes": expected.get("observation_bytes", 0),
                "reserved_observation_bytes": expected.get("reserved_observation_bytes", 0),
                "diagnostics": diagnostics}

    with tempfile.TemporaryDirectory(prefix="f5-oracle-contract-", dir=temp_root) as temp:
        tree = Path(temp)
        debug, release = tree / "debug.exe", tree / "release.exe"
        debug.write_bytes(b"mock-debug")
        release.write_bytes(b"mock-release")
        for mode in ("faithful", "value", "line", "code", "exit", "check"):
            def mock_invoke(binary: Path, verb: str, source: Path,
                            requested_profile: str, *, timeout: float = 10.0):
                assert requested_profile == profile
                program = source.read_text(encoding="ascii")
                if program == "fn F(x:view I64){}":
                    expected = {"exit": 0 if verb == "check" else 1,
                                "outcome": "checked" if verb == "check" else "rejected",
                                "diagnostics": [] if verb == "check" else [{
                                    "code": "NX-E0112", "subcause": "missing_main",
                                    "stage": "entry", "operation": "entry.check",
                                    "primary": [18, 18], "data": {"entry": "Main"}}]}
                else:
                    expected = runner.projection(by_source[program], verb, complete)
                response = document(expected)
                exit_code = expected["exit"]
                if program in by_source:
                    if mode == "value" and verb == "run" and response["stdout"]:
                        response["stdout"][0] += "!"
                    elif mode == "line" and verb == "run" and response["stdout"]:
                        response["stdout"].pop()
                    elif mode == "code" and response["diagnostics"]:
                        response["diagnostics"][0]["code"] = "NX-E9999"
                    elif mode == "exit" and exit_code == 1:
                        exit_code = 0
                    elif mode == "check" and verb == "check" and exit_code == 0:
                        response["outcome"]["kind"] = "unit"
                return runner.Invocation([str(binary), verb], exit_code, response,
                                         "", None, 0.0)

            output = tree / f"{mode}.jsonl"
            with patch.object(runner, "invoke", side_effect=mock_invoke), \
                    patch.object(runner, "minimize_case", side_effect=lambda c, *_args, **_kw: (c, 0)):
                summary = runner.run_compare(binary_debug=debug, binary_release=release,
                    profile=profile, phase="G4", complete=",".join(complete),
                    seeds="0", valid_per_seed=18, invalid_per_seed=14,
                    output=output, include_literals=False)
            records = [json.loads(line) for line in output.read_text(encoding="utf-8").splitlines()]
            rows = [row for row in records if row["kind"] == "case"]
            assert len(rows) == 32
            fields = {probe["difference"] for row in rows
                      for probe in row["observations"].values() if probe["difference"]}
            wanted = {"faithful": set(), "value": {"$.stdout[0]"},
                      "line": {"$.stdout"}, "code": {"$.diagnostics[0].code"},
                      "exit": {"$.exit"}, "check": {"$.outcome"}}[mode]
            assert fields == wanted, (mode, fields)
            assert (summary["differences"] == 0) == (mode == "faithful")
            results[mode] = summary["differences"]
    # F5 §9.1 and F5-O02: this mock is literal, not built from projection().
    fault = Case(0, 0, "invalid", "F5-O02", Program(()),
                 "fn Main(){let a=1/0;}", None, "R0502")
    literal = {"exit": 1, "outcome": "simulator_fault", "stdout": [],
               "events": [], "observations": [], "observation_bytes": 0,
               "reserved_observation_bytes": 0,
               "diagnostics": [{"code": "R0502", "subcause": None,
                                "stage": "runtime", "operation": "value.div",
                                "primary": [16, 19], "data": {},
                                "profile_id": profile, "rule": "R0502",
                                "diagnostic_namespace": "F0-0002-sim",
                                "severity": "error", "expected_tokens": []}]}
    response = document(literal)
    invocation = runner.Invocation(["literal-mock"], 1, response, "", None, 0.0)
    difference, expected = runner.compare_invocation(invocation, fault, "run", complete, profile)
    assert difference is None, (difference, expected, response)
    response["outcome"]["kind"] = "rejected"
    assert runner.compare_invocation(invocation, fault, "run", complete, profile)[0] == \
        "$.outcome"
    results["runtime_literal"] = 1
    return results


def check_harness_contract() -> int:
    from unittest.mock import patch
    from . import cli, runner
    assert runner._first_difference({"diagnostics": [1]},
                                    {"diagnostics": [1, 2]}) == "$.diagnostics"
    assert runner._first_difference({"stdout": ["1", "2"]},
                                    {"stdout": ["1"]}) == "$.stdout"
    assert runner._first_difference({"data": {"pending_scope": "L07"}},
                                    {"data": "x"}) == "$.data"
    assert runner._first_difference({"a": 1}, "x") == "$"
    repeated = make_valid(0, 10)
    barrier = runner.projection(repeated, "check", runner.CAPABILITIES[:3])
    assert barrier["diagnostics_cardinality"] == "unspecified"
    required = barrier["diagnostics"][0]
    diagnostic = {key: required[key] for key in
                  ("code", "subcause", "stage", "operation", "feature_id", "data")}
    response = {"schema_version": 1, "profile_id": "f0-0005-b-v1",
                "outcome": {"kind": "rejected"}, "stdout": [], "events": [],
                "observations": [], "observation_bytes": 0,
                "reserved_observation_bytes": 0,
                "diagnostics": [diagnostic, diagnostic]}
    invocation = runner.Invocation(["mock"], 1, response, "", None, 0.0)
    assert runner.compare_invocation(invocation, repeated, "check",
        runner.CAPABILITIES[:3], "f0-0005-b-v1")[0] is None
    response["diagnostics"] = []
    assert runner.compare_invocation(invocation, repeated, "check",
        runner.CAPABILITIES[:3], "f0-0005-b-v1")[0] == "$.diagnostics"
    arguments = ["f5-oracle", "compare", "--binary-debug", "debug.exe",
                 "--binary-release", "release.exe", "--profile", "f0-0005-b-v1",
                 "--phase", "G4", "--complete", ",".join(runner.CAPABILITIES[:8]),
                 "--seeds", "0", "--output", "C:/nexo-work/tmp/mock.jsonl"]
    for differences in (0, 1):
        with patch.object(sys, "argv", arguments), \
                patch.object(cli, "run_compare", return_value={"differences": differences}):
            with redirect_stdout(io.StringIO()):
                assert cli.main() == (1 if differences else 0)
    fuzz_args = ["f5-oracle", "fuzz", "--binary-debug", "debug.exe",
                 "--binary-release", "release.exe", "--profile", "f0-0005-b-v1",
                 "--phase", "G4", "--complete", ",".join(runner.CAPABILITIES[:8]),
                 "--seeds", "0", "--output", "C:/nexo-work/tmp/mock-fuzz.jsonl"]
    for failures in (0, 1):
        with patch.object(sys, "argv", fuzz_args), \
                patch.object(cli, "run_fuzz", return_value={"failures": failures}):
            with redirect_stdout(io.StringIO()):
                assert cli.main() == (1 if failures else 0)
    # Execute the fuzz path with panic and confirmed timeout stand-ins.
    for error in ("panic", "timeout_confirmed"):
        bad_probe = runner.Invocation(["mock"], 2, None, "", error, 0.0)
        with patch.object(sys, "argv", fuzz_args + ["--cases-per-seed", "1"]), \
                patch.object(runner, "invoke", return_value=bad_probe) as invoke_mock:
            with redirect_stdout(io.StringIO()):
                assert cli.main() == 1
            assert invoke_mock.call_count == 4
    for differences in (0, 1):
        gate_args = arguments.copy()
        gate_args[1] = "gate"
        with patch.object(sys, "argv", gate_args), \
                patch.object(cli.selftest, "run", return_value={}), \
                patch.object(cli, "run_compare", return_value={"differences": differences}):
            with redirect_stdout(io.StringIO()):
                assert cli.main() == (1 if differences else 0)
    response = subprocess.CompletedProcess(["mock"], 0,
        stdout=b'{"outcome":{"kind":"unit"}}', stderr=b"")
    timeout = subprocess.TimeoutExpired(["mock"], 0.01)
    with patch.object(runner.subprocess, "run", side_effect=[timeout, response]) as execute_mock:
        recovered = runner.invoke(Path("mock.exe"), "run", Path("case.n"), profile="default")
        assert recovered.error is None and recovered.timeout_retried
        assert execute_mock.call_count == 2
    with patch.object(runner.subprocess, "run", side_effect=[timeout, timeout]) as execute_mock:
        confirmed = runner.invoke(Path("mock.exe"), "run", Path("case.n"), profile="default")
        assert confirmed.error == "timeout_confirmed" and confirmed.timeout_retried
        assert execute_mock.call_count == 2
    return 16


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
        assert audit["valid_structural"] >= 400, seed
        assert audit["invalid_structural"] >= 400, seed
        report[str(seed)] = {"valid": audit["valid_distinct"],
                             "invalid": audit["invalid_distinct"],
                             "valid_structural": audit["valid_structural"],
                             "invalid_structural": audit["invalid_structural"],
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
    result["full_compare_mock"] = check_full_compare_mock()
    result["harness_contract"] = check_harness_contract()
    result["distinct_by_seed"] = check_scale_hashes()
    return result
