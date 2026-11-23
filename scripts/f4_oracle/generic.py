"""Generic pieces taken from the F5 oracle (class ARNES).

Procedencia (plan 1.4): COPIADAS sin cambios ``sha``, ``Invocation``, ``seed_range`` y
``make_batches``; ADAPTADAS ``invoke``, ``normalize_files`` y ``_first_difference`` (cambios
declarados en ``tests/fixtures/f4_oracle_pins.json``).  ``audit`` exige que este archivo
contenga exactamente esos siete símbolos con los hashes AST pinados y ninguno más.
"""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time
from typing import Any


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def normalize_files(value: Any, file_name: str, source: str | None = None) -> Any:
    accepted = {file_name} if source is None else {file_name, source}
    if isinstance(value, dict):
        return {key: (file_name if key == "file" and isinstance(item, str)
                      and (item in accepted or Path(item).name == file_name)
                      else normalize_files(item, file_name, source))
                for key, item in value.items()}
    if isinstance(value, list):
        return [normalize_files(item, file_name, source) for item in value]
    return value


def _first_difference(expected: Any, actual: Any, path: str = "$") -> str | None:
    if isinstance(expected, dict):
        if not isinstance(actual, dict):
            return path
        for key in sorted(set(expected) | set(actual)):
            if key not in actual or key not in expected:
                return f"{path}.{key}"
            found = _first_difference(expected[key], actual[key], f"{path}.{key}")
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
    if type(expected) is not type(actual):
        return path
    return None if expected == actual else path


@dataclass(frozen=True)
class Invocation:
    command: list[str]
    exit: int
    document: dict[str, Any] | None
    stderr: str
    error: str | None
    elapsed_ms: float
    timeout_retried: bool = False


def invoke(binary: Path, verb: str, source: Path, profile: str,
           *, timeout: float = 10.0, file_name: str | None = None,
           mock_env: Path | None = None) -> Invocation:
    name = source.name if file_name is None else file_name
    command = ([sys.executable, "-B", str(binary)] if binary.suffix == ".py"
               else [str(binary)]) + [verb, str(source), "--profile", profile,
               "--format", "json"]
    if mock_env is not None and verb == "run":
        command += ["--mock-env", str(mock_env)]
    started = time.perf_counter()
    retried = False
    for attempt in range(2):
        try:
            result = subprocess.run(command, capture_output=True, timeout=timeout)
            break
        except subprocess.TimeoutExpired:
            if attempt == 0:
                retried = True
                continue
            return Invocation(command, -1, None, "", "timeout_confirmed",
                              (time.perf_counter() - started) * 1000, True)
    stderr = result.stderr.decode("utf-8", "replace")
    try:
        document = json.loads(result.stdout.decode("utf-8"))
        if not isinstance(document, dict):
            raise ValueError("JSON root is not an object")
    except (ValueError, UnicodeDecodeError):
        return Invocation(command, result.returncode, None, stderr,
                          "non_json_output", (time.perf_counter() - started) * 1000,
                          retried)
    outcome = document.get("outcome", {})
    if not isinstance(outcome, dict):
        outcome = {}
    if outcome.get("kind") == "driver_error" and outcome.get("reason") == "internal_error":
        return Invocation(command, result.returncode,
                          normalize_files(document, name, str(source)), stderr,
                          "internal_error", (time.perf_counter() - started) * 1000,
                          retried)
    if "InvalidIr" in stderr or "panicked at" in stderr:
        return Invocation(command, result.returncode,
                          normalize_files(document, name, str(source)), stderr,
                          "panic_or_invalid_ir", (time.perf_counter() - started) * 1000,
                          retried)
    return Invocation(command, result.returncode, normalize_files(document, name, str(source)),
                      stderr, None, (time.perf_counter() - started) * 1000, retried)


def seed_range(value: str) -> range:
    left, colon, right = value.partition(":")
    if not colon:
        return range(int(left), int(left) + 1)
    start, end = int(left), int(right)
    if start < 0 or end < start:
        raise ValueError(value)
    return range(start, end + 1)


def make_batches(spec: str, width: int) -> list[list[int]]:
    if width < 1:
        raise ValueError("batch width must be positive")
    seeds = list(seed_range(spec))
    return [seeds[offset:offset + width] for offset in range(0, len(seeds), width)]
