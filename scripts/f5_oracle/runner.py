"""Public CLI differential for sources predicted by the independent F5 model."""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import random
import shutil
import subprocess
import sys
import tempfile
import time
from typing import Any, Iterator

from .generate import Case, cases
from .diagnostics import missing_main, semantic
from .model import Expr, ModelFault, SourceVM, Stmt, parse_source

ROOT = Path(__file__).resolve().parents[2]
ADR = ROOT / "docs/adr/F0-0005-operators-control-collections.md"
TMP = Path("C:/nexo-work/tmp")
CAPABILITIES = ("L01b", "L02b", "L03b", "L04b", "L05b", "L07b",
                "L06b", "L08", "L09", "L10", "L11", "L12")
PHASE_LIMIT = {"G1b": 3, "G2b": 5, "G3b": 7, "G4": 8,
               "G5": 10, "G6": 12}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def oracle_sha() -> str:
    digest = hashlib.sha256()
    for name in ("model.py", "diagnostics.py", "charges.py", "generate.py",
                 "selftest.py", "runner.py", "cli.py"):
        digest.update((Path(__file__).parent / name).read_bytes())
    digest.update((ROOT / "tests/fixtures/f5_oracle_literals.json").read_bytes())
    return digest.hexdigest()


def normalize_files(value: Any) -> Any:
    if isinstance(value, dict):
        return {key: ("case.n" if key == "file" else normalize_files(item))
                for key, item in value.items()}
    if isinstance(value, list):
        return [normalize_files(item) for item in value]
    return value


def manifest(phase: str, complete: str) -> tuple[str, ...]:
    if phase not in PHASE_LIMIT:
        raise ValueError(f"unknown phase: {phase}")
    declared = tuple(complete.split(",")) if complete else ()
    if len(set(declared)) != len(declared) or declared != CAPABILITIES[:len(declared)]:
        raise ValueError("capabilities must be a unique ordered prefix of F5-r1 subtasks")
    if len(declared) > PHASE_LIMIT[phase]:
        raise ValueError(f"{phase} cannot include {declared[-1]}")
    return declared


def _expr_requirements(expr: Expr) -> set[str]:
    required = set()
    if expr.kind == "binary":
        if expr.value in ("*", "/", "%", "<", "<=", ">", ">=", "!="):
            required.add("L01b")
        if expr.value in ("and", "or"):
            required.add("L02b")
    elif expr.kind in ("neg", "not"):
        if expr.kind == "neg":
            required.add("L01b")
        else:
            required.add("L02b")
    elif expr.kind == "join":
        required.add("L07b")
    for child in expr.args:
        required.update(_expr_requirements(child))
    return required


def _stmt_requirements(stmt: Stmt) -> set[str]:
    required = set()
    if stmt.kind == "compound":
        required.add("L03b")
    elif stmt.kind == "for_range":
        required.add("L05b")
    elif stmt.kind == "for_array":
        required.add("L06b")
    if stmt.expr is not None:
        required.update(_expr_requirements(stmt.expr))
    if stmt.upper is not None:
        required.update(_expr_requirements(stmt.upper))
    for child in stmt.body + stmt.otherwise:
        required.update(_stmt_requirements(child))
    return required


def requirements(case: Case) -> set[str]:
    result = set()
    for statement in case.program.body:
        result.update(_stmt_requirements(statement))
    return result


def projection(case: Case, verb: str, complete: tuple[str, ...]) -> dict[str, Any]:
    """Expected observable fields; no value is read from the Nexo binary."""
    required = requirements(case)
    missing = next((cap for cap in CAPABILITIES if cap in required and cap not in complete), None)
    static = case.expected_issue
    if static is not None and static["stage"] in ("parse", "names"):
        missing = None
    if missing is not None:
        scope = missing.removesuffix("b")
        stage = "ownership" if missing == "L04b" else "types"
        return {"exit": 1, "outcome": "rejected", "diagnostics": [{
            "code": "NX-E0006", "subcause": None, "stage": stage,
            "operation": {"ownership": "ownership.check", "types": "types.check"}[stage],
            "feature_id": "implementation_limit", "data": {"pending_scope": scope}}]}
    if static is not None:
        return {"exit": 1, "outcome": "rejected", "diagnostics": [static]}
    if required and "L08" not in complete:
        return {"exit": 1, "outcome": "rejected", "diagnostics": [{
            "code": "NX-E0006", "subcause": None, "stage": "lower",
            "operation": "ir.lower", "feature_id": "implementation_limit",
            "data": {"pending_scope": "L08"}}]}
    if case.expected_fault is not None and verb == "run":
        try:
            SourceVM(parse_source(case.source)).run()
        except ModelFault as fault:
            assert fault.code == case.expected_fault
            diagnostic = semantic(fault.code, "runtime", fault.operation,
                                  fault.start, fault.end)
            expected_diag = project_diagnostic(diagnostic)
        else:
            raise AssertionError((case.family, "reference runtime fault absent"))
        return {"exit": 1, "outcome": "simulator_fault",
                "diagnostics": [expected_diag]}
    return {"exit": 0, "outcome": "checked" if verb == "check" else "unit",
            "diagnostics": []}


def _first_difference(expected: Any, actual: Any, path: str = "$") -> str | None:
    if isinstance(expected, dict):
        if not isinstance(actual, dict):
            return path
        for key, value in expected.items():
            if key not in actual:
                return f"{path}.{key}"
            found = _first_difference(value, actual[key], f"{path}.{key}")
            if found is not None:
                return found
        return None
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(expected) != len(actual):
            return path
        for index, value in enumerate(expected):
            found = _first_difference(value, actual[index], f"{path}[{index}]")
            if found is not None:
                return found
        return None
    return None if expected == actual else path


@dataclass(frozen=True)
class Invocation:
    command: list[str]
    exit: int
    document: dict[str, Any] | None
    stderr: str
    error: str | None


def invoke(binary: Path, verb: str, source: Path, profile: str,
           *, timeout: float = 10.0) -> Invocation:
    command = [str(binary), verb, str(source), "--profile", profile,
               "--format", "json"]
    try:
        result = subprocess.run(command, capture_output=True, text=True,
                                encoding="utf-8", timeout=timeout)
    except subprocess.TimeoutExpired:
        return Invocation(command, -1, None, "", "timeout")
    try:
        document = json.loads(result.stdout)
        if not isinstance(document, dict):
            raise ValueError("JSON root is not an object")
    except (ValueError, json.JSONDecodeError):
        return Invocation(command, result.returncode, None, result.stderr,
                          "non_json_output")
    outcome = document.get("outcome", {})
    if outcome.get("kind") == "driver_error" and outcome.get("reason") == "internal_error":
        return Invocation(command, result.returncode, normalize_files(document), result.stderr,
                          "internal_error")
    if "InvalidIr" in result.stderr or "panicked at" in result.stderr:
        return Invocation(command, result.returncode, normalize_files(document), result.stderr,
                          "panic_or_invalid_ir")
    return Invocation(command, result.returncode, normalize_files(document),
                      result.stderr, None)


def observed(invocation: Invocation, profile: str) -> dict[str, Any]:
    document = invocation.document or {}
    return {
        "exit": invocation.exit,
        "schema_version": document.get("schema_version"),
        "profile_id": document.get("profile_id"),
        "outcome": document.get("outcome", {}).get("kind"),
        "stdout": document.get("stdout"), "events": document.get("events"),
        "observations": document.get("observations"),
        "observation_bytes": document.get("observation_bytes"),
        "reserved_observation_bytes": document.get("reserved_observation_bytes"),
        "diagnostics": [project_diagnostic(diag) for diag in document.get("diagnostics", [])],
    }


def project_diagnostic(diag: dict[str, Any]) -> dict[str, Any]:
    primary = diag.get("primary_span", {})
    related = diag.get("related_spans", [])
    token = diag.get("unexpected_token")
    prefix = diag.get("recognized_prefix")
    return {
        "code": diag.get("code"), "subcause": diag.get("subcause"),
        "stage": diag.get("stage"), "operation": diag.get("operation"),
        "feature_id": diag.get("feature_id"), "data": diag.get("data"),
        "profile_id": diag.get("profile_id"),
        "rule": diag.get("rule"), "explanation_id": diag.get("explanation_id"),
        "diagnostic_namespace": diag.get("diagnostic_namespace"),
        "severity": diag.get("severity"),
        "primary": [primary.get("start"), primary.get("end")],
        "related": [[item.get("role"), item.get("span", {}).get("start"),
                     item.get("span", {}).get("end")] for item in related],
        "cause": diag.get("cause_chain"),
        "unexpected_token": None if token is None else {
            "kind": token.get("kind"), "text": token.get("text"),
            "span": [token.get("span", {}).get("start"),
                     token.get("span", {}).get("end")]},
        "expected_tokens": diag.get("expected_tokens"),
        "expected_tokens_kind": diag.get("expected_tokens_kind"),
        "parser_context": diag.get("parser_context"),
        "recognized_prefix": None if prefix is None else [prefix.get("start"),
                                                            prefix.get("end")],
    }


def compare_invocation(invocation: Invocation, case: Case, verb: str,
                       complete: tuple[str, ...], profile: str) -> tuple[str | None, dict[str, Any]]:
    expected = projection(case, verb, complete)
    expected.update({"schema_version": 1, "profile_id": profile,
                     "stdout": [], "events": [], "observations": [],
                     "observation_bytes": 0, "reserved_observation_bytes": 0})
    if invocation.error:
        return invocation.error, expected
    actual = observed(invocation, profile)
    return _first_difference(expected, actual), expected


def literal_cases(profile: str) -> Iterator[tuple[str, str, str, dict[str, Any]]]:
    """G6 sources and expectations copied from the reviewed ADR fixture."""
    fixture = json.loads((ROOT / "tests/fixtures/f5_oracle_literals.json")
                         .read_text(encoding="utf-8"))

    def envelope(verb: str, *, kind: str = "unit",
                 diagnostics: list[dict[str, Any]] | None = None,
                 stdout: list[str] | None = None, observation_bytes: int = 0) -> dict[str, Any]:
        lines = stdout or []
        return {"exit": 0 if kind in ("checked", "unit") else 1,
                "schema_version": 1, "profile_id": profile,
                "outcome": kind, "stdout": lines, "events": [],
                "observations": [{"channel": "stdout", "text": line} for line in lines],
                "observation_bytes": observation_bytes,
                "reserved_observation_bytes": 0,
                "diagnostics": diagnostics or []}

    for identifier, row in fixture["rows"].items():
        contract = row["contract"]
        diagnostic = {"code": row["code"], "subcause": row["subcause"],
                      "stage": contract["stage"], "operation": contract["operation"],
                      "primary": row["primary"], "data": contract["data"],
                      "related": contract["related"], "feature_id": contract["feature_id"],
                      "cause": contract["cause"]}
        runtime = contract["stage"] == "runtime"
        for verb in ("check", "run"):
            expected = envelope(verb, kind=("checked" if verb == "check" else
                                           "simulator_fault") if runtime else "rejected",
                                diagnostics=[] if runtime and verb == "check" else [diagnostic])
            yield identifier + "-negative", row["source"], verb, expected
        for verb in ("check", "run"):
            yield identifier + "-positive", row["positive_source"], verb, \
                envelope(verb, kind="checked" if verb == "check" else "unit")

    for identifier, row in fixture["syntax"].items():
        contract = row["contract"]
        diagnostic = {"code": "NX-E0002", "subcause": None,
                      "stage": "parse", "operation": "syntax.parse",
                      "primary": contract["primary"], "data": {},
                      "unexpected_token": {"kind": contract["token_kind"],
                                           "text": contract["token_text"],
                                           "span": contract["primary"]},
                      "expected_tokens": contract["expected_tokens"],
                      "expected_tokens_kind": "admissible_set",
                      "parser_context": contract["context"],
                      "recognized_prefix": contract["prefix"]}
        for verb in ("check", "run"):
            yield identifier + "-negative", row["source"], verb, \
                envelope(verb, kind="rejected", diagnostics=[diagnostic])
            if identifier != "F5-S08":
                yield identifier + "-positive", row["positive_source"], verb, \
                    envelope(verb, kind="checked" if verb == "check" else "unit")

    for identifier, source in fixture["acceptance_programs"].items():
        output = fixture["acceptance_stdout"][identifier]
        lines = output.rstrip("\n").split("\n") if output else []
        for verb in ("check", "run"):
            yield identifier, source, verb, envelope(
                verb, kind="checked" if verb == "check" else "unit",
                stdout=[] if verb == "check" else lines,
                observation_bytes=0 if verb == "check" else
                fixture["acceptance_observation_bytes"][identifier])

    for index, source in enumerate(fixture["behavior_programs"]):
        output = fixture["behavior_stdout"][index]
        lines = output.rstrip("\n").split("\n") if output else []
        for verb in ("check", "run"):
            yield f"F5-behavior-{index}", source, verb, envelope(
                verb, kind="checked" if verb == "check" else "unit",
                stdout=[] if verb == "check" else lines,
                observation_bytes=0 if verb == "check" else len(output.encode("utf-8")))


def seed_range(value: str) -> range:
    left, colon, right = value.partition(":")
    if not colon:
        return range(int(left), int(left) + 1)
    start, end = int(left), int(right)
    if start < 0 or end < start:
        raise ValueError(value)
    return range(start, end + 1)


def run_compare(*, binary_debug: Path, binary_release: Path, profile: str,
                phase: str, complete: str, seeds: str, valid_per_seed: int,
                invalid_per_seed: int, output: Path, timeout: float = 10.0) -> dict[str, Any]:
    active = manifest(phase, complete)
    if valid_per_seed < 0 or invalid_per_seed < 0:
        raise ValueError("negative case count")
    for binary in (binary_debug, binary_release):
        if not binary.is_file():
            raise FileNotFoundError(binary)
    TMP.mkdir(parents=True, exist_ok=True)
    output.parent.mkdir(parents=True, exist_ok=True)
    metadata = {"python": sys.version.split()[0], "phase": phase, "complete": list(active),
                "profile": profile, "adr_sha256": sha(ADR),
                "oracle_sha256": oracle_sha(),
                "binary_debug_sha256": sha(binary_debug),
                "binary_release_sha256": sha(binary_release)}
    fresh = not output.exists() or output.stat().st_size == 0
    completed: set[tuple[int, str, int]] = set()
    s08_completed = False
    literal_done: set[tuple[str, str]] = set()
    prior_differences = 0
    if output.exists():
        for line in output.read_text(encoding="utf-8").splitlines():
            record = json.loads(line)
            if record.get("kind") == "case":
                completed.add((record["seed"], record["lane"], record["index"]))
                prior_differences += sum(bool(probe["difference"]) for probe in
                                         record["observations"].values())
            elif record.get("kind") == "literal" and record.get("id") == "F5-S08-positive":
                s08_completed = True
                prior_differences += sum(bool(probe["difference"]) for probe in
                                         record["checks"].values())
            elif record.get("kind") == "adr_literal":
                literal_done.add((record["id"], record["verb"]))
                prior_differences += sum(bool(probe["difference"]) for probe in
                                         record["probes"].values())
            elif record.get("kind") == "metadata" and record["metadata"] != metadata:
                raise ValueError("resume metadata differs from current binaries or oracle")
    total, differences = 0, prior_differences
    with tempfile.TemporaryDirectory(prefix="f5-oracle-", dir=TMP) as temporary:
        tree = Path(temporary)
        binaries = {}
        for label, binary in (("debug", binary_debug), ("release", binary_release)):
            copied = tree / f"{label}-nexo.exe"
            shutil.copy2(binary, copied)
            assert sha(copied) == metadata[f"binary_{label}_sha256"]
            binaries[label] = copied
        source_path = tree / "case.n"
        with output.open("a", encoding="utf-8", newline="\n") as log:
            if fresh:
                log.write(json.dumps({"kind": "metadata", "metadata": metadata},
                                     ensure_ascii=False, sort_keys=True) + "\n")
            if not s08_completed:
                fixture = json.loads((ROOT / "tests/fixtures/f5_oracle_literals.json")
                                     .read_text(encoding="utf-8"))
                source = fixture["syntax"]["F5-S08"]["positive_source"]
                assert source == "fn F(x:view I64){}"
                source_path.write_bytes(source.encode("utf-8"))
                expected_entry = missing_main(source)
                checks = {}
                for label, binary in binaries.items():
                    for verb in ("check", "run"):
                        invocation = invoke(binary, verb, source_path, profile,
                                            timeout=timeout)
                        expected = {
                            "exit": 0 if verb == "check" else 1,
                            "schema_version": 1, "profile_id": profile,
                            "outcome": "checked" if verb == "check" else "rejected",
                            "stdout": [], "events": [], "observations": [],
                            "observation_bytes": 0, "reserved_observation_bytes": 0,
                            "diagnostics": [] if verb == "check" else [{
                                "code": expected_entry["code"],
                                "subcause": expected_entry["subcause"],
                                "stage": expected_entry["stage"],
                                "operation": expected_entry["operation"],
                                "primary": [18, 18], "data": {"entry": "Main"}}]}
                        difference = invocation.error or _first_difference(
                            expected, observed(invocation, profile))
                        differences += difference is not None
                        checks[f"{label}.{verb}"] = {
                            "difference": difference, "expected": expected,
                            "obtained": invocation.document, "command": invocation.command,
                            "exit": invocation.exit, "error": invocation.error}
                log.write(json.dumps({"kind": "literal", "id": "F5-S08-positive",
                                      "source": source, "checks": checks},
                                     ensure_ascii=False, sort_keys=True) + "\n")
                log.flush()
            if phase == "G6" and len(active) == len(CAPABILITIES):
                for identifier, source, verb, expected in literal_cases(profile):
                    if (identifier, verb) in literal_done:
                        continue
                    source_path.write_bytes(source.encode("utf-8"))
                    probes = {}
                    for label, binary in binaries.items():
                        invocation = invoke(binary, verb, source_path, profile,
                                            timeout=timeout)
                        difference = invocation.error or _first_difference(
                            expected, observed(invocation, profile))
                        differences += difference is not None
                        probes[label] = {"difference": difference,
                                         "command": invocation.command,
                                         "error": invocation.error,
                                         "exit": invocation.exit,
                                         "obtained": invocation.document}
                    log.write(json.dumps({"kind": "adr_literal", "id": identifier,
                                          "verb": verb, "source": source,
                                          "expected": expected, "probes": probes},
                                         ensure_ascii=False, sort_keys=True) + "\n")
                    log.flush()
            for seed in seed_range(seeds):
                for case in cases(seed, valid_per_seed, invalid_per_seed):
                    key = (seed, case.lane, case.index)
                    if key in completed:
                        continue
                    source_path.write_bytes(case.source.encode("utf-8"))
                    observations = {}
                    for label, binary in binaries.items():
                        for verb in ("check", "run"):
                            invocation = invoke(binary, verb, source_path, profile,
                                                timeout=timeout)
                            difference, expected = compare_invocation(
                                invocation, case, verb, active, profile)
                            observations[f"{label}.{verb}"] = {
                                "difference": difference, "command": invocation.command,
                                "error": invocation.error, "exit": invocation.exit,
                                "expected": expected, "obtained": invocation.document,
                                "stderr": invocation.stderr,
                                "known_h03": case.family == "range.stored" and
                                difference == "internal_error"}
                            differences += difference is not None
                    record = {**case.record(), "kind": "case",
                              "observations": observations}
                    log.write(json.dumps(record, ensure_ascii=False,
                                         sort_keys=True, separators=(",", ":")) + "\n")
                    log.flush()
                    total += 1
    return {"cases_executed": total, "cases_resumed": len(completed),
            "differences": differences, "metadata": metadata, "output": str(output)}


def run_fuzz(*, binary_debug: Path, binary_release: Path, profile: str,
             phase: str, complete: str, seeds: str, cases_per_seed: int,
             output: Path, timeout: float = 10.0) -> dict[str, Any]:
    manifest(phase, complete)
    if cases_per_seed < 0:
        raise ValueError("negative case count")
    TMP.mkdir(parents=True, exist_ok=True)
    output.parent.mkdir(parents=True, exist_ok=True)
    total = failures = 0
    with tempfile.TemporaryDirectory(prefix="f5-oracle-fuzz-", dir=TMP) as temporary:
        path = Path(temporary) / "case.n"
        with output.open("w", encoding="utf-8", newline="\n") as log:
            for seed in seed_range(seeds):
                rng = random.Random(seed)
                for index in range(cases_per_seed):
                    if index % 2 == 0:
                        source = cases(seed, 1, 0).__next__().source.encode("utf-8")
                        payload = source[:rng.randrange(len(source) + 1)]
                    else:
                        payload = rng.randbytes(rng.randrange(65))
                    path.write_bytes(payload)
                    record = {"seed": seed, "index": index,
                              "source_hex": payload.hex(), "probes": {}}
                    for label, binary in (("debug", binary_debug),
                                          ("release", binary_release)):
                        for verb in ("check", "run"):
                            invocation = invoke(binary, verb, path, profile,
                                                timeout=timeout)
                            error = invocation.error
                            if error:
                                failures += 1
                            record["probes"][f"{label}.{verb}"] = {
                                "exit": invocation.exit, "error": error,
                                "outcome": (invocation.document or {}).get("outcome")}
                    log.write(json.dumps(record, sort_keys=True) + "\n")
                    total += 1
    return {"cases": total, "failures": failures, "output": str(output)}
