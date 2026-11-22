"""Fictitious ``nexo`` executable for the F4 oracle self-test.

It implements no Nexo semantics: it looks the source up (by SHA-256 of its bytes) in a table
written by the self-test (``F4_MOCK_TABLE``) and prints the response recorded there, optionally
damaged by ONE named corruption (``F4_MOCK_CORRUPT``).  The faithful run (no corruption) replays
the literal response of a K row, or the model's expected document for a generated case (this only
proves the harness is coherent); every corruption must make the harness report a divergence or
a failure.  ``--list-corruptions`` prints the names this file implements.

    python -B f4_oracle_mock_nexo.py check|run SOURCE --profile P --format json [--mock-env PATH]
"""

import hashlib
import json
import os
import sys
import time

DIAG_FIELDS = (
    "schema_version", "diagnostic_namespace", "profile_id", "code", "subcause", "rule",
    "explanation_id", "severity", "stage", "operation", "primary_span", "related_spans",
    "cause_chain", "feature_id", "unexpected_token", "expected_tokens",
    "expected_tokens_kind", "parser_context", "recognized_prefix", "repair", "data",
)
RESULT = ("exit", "outcome_kind", "outcome_value", "stdout", "events", "observations",
          "observation_bytes", "reserved_observation_bytes")
HARNESS = ("cardinality_extra", "cardinality_missing", "debug_release", "check_run", "json_double",
           "json_absent", "timeout", "internal_error", "type_true", "type_string", "type_float",
           "null_vs_absent", "arrays_reordered", "file_other", "panic_stderr", "invalid_ir")


def names():
    return ["diag:" + f for f in DIAG_FIELDS] + list(RESULT) + list(HARNESS)


def bump(value):
    if value is None:
        return "corrupt"
    if isinstance(value, bool):
        return not value
    if isinstance(value, int):
        return value + 1
    if isinstance(value, str):
        return value + "x"
    if isinstance(value, list):
        return value + ["corrupt"]
    if isinstance(value, dict):
        return dict(value, corrupt=1)
    return "corrupt"


def set_file(value, path):
    if isinstance(value, dict):
        for key in list(value):
            if key == "file" and isinstance(value[key], str):
                value[key] = path
            else:
                set_file(value[key], path)
    elif isinstance(value, list):
        for item in value:
            set_file(item, path)


def benign(profile):
    return {"exit": 1, "schema_version": 1, "profile_id": profile, "outcome": {"kind": "rejected"},
            "stdout": [], "events": [], "observations": [], "observation_bytes": 0,
            "reserved_observation_bytes": 0, "diagnostics": []}


def corrupt(name, doc, code, verb):
    """Apply one named corruption; returns (doc, exit code, mode) with mode in
    {'normal', 'double', 'absent', 'sleep', 'panic', 'invalid_ir'}."""
    mode = "normal"
    diags = doc.get("diagnostics") if isinstance(doc.get("diagnostics"), list) else []
    if name.startswith("diag:"):
        if diags:
            field = name[5:]
            diags[0][field] = bump(diags[0].get(field))
    elif name == "exit":
        code = code + 1
    elif name == "outcome_kind":
        doc["outcome"]["kind"] = bump(doc["outcome"].get("kind"))
    elif name == "outcome_value":
        doc["outcome"]["error"] = "corrupt"
    elif name in ("stdout", "events", "observations"):
        doc[name] = bump(doc.get(name, []))
    elif name in ("observation_bytes", "reserved_observation_bytes"):
        doc[name] = doc.get(name, 0) + 1
    elif name == "cardinality_extra":
        diags.append(dict(diags[0]) if diags else {"code": "NX-E0000"})
        doc["diagnostics"] = diags
    elif name == "cardinality_missing":
        doc["diagnostics"] = []
    elif name == "debug_release":
        if "release" in os.path.basename(sys.argv[0]):
            doc["outcome"]["kind"] = bump(doc["outcome"].get("kind"))
    elif name == "check_run":
        if verb == "run":
            doc["outcome"] = {"kind": "checked"}
    elif name == "json_double":
        mode = "double"
    elif name == "json_absent":
        mode = "absent"
    elif name == "timeout":
        mode = "sleep"
    elif name == "internal_error":
        doc["outcome"] = {"kind": "driver_error", "reason": "internal_error"}
    elif name == "type_true":
        doc["schema_version"] = True
    elif name == "type_string":
        doc["schema_version"] = "1"
    elif name == "type_float":
        doc["schema_version"] = 1.0
    elif name == "null_vs_absent":
        if diags and "feature_id" in diags[0]:
            del diags[0]["feature_id"]
    elif name == "arrays_reordered":
        if isinstance(doc.get("stdout"), list) and len(doc["stdout"]) > 1:
            doc["stdout"] = list(reversed(doc["stdout"]))
        elif len(diags) > 1:
            doc["diagnostics"] = list(reversed(diags))
    elif name == "file_other":
        set_file(doc, "other.n")
    elif name == "panic_stderr":
        mode = "panic"
    elif name == "invalid_ir":
        mode = "invalid_ir"
    elif name:
        sys.stderr.write("unknown corruption " + name + "\n")
        sys.exit(64)
    return doc, code, mode


def main():
    argv = sys.argv[1:]
    if argv and argv[0] == "--list-corruptions":
        sys.stdout.write("\n".join(names()) + "\n")
        return 0
    verb, source = argv[0], argv[1]
    if os.environ.get("F4_MOCK_LOG"):
        with open(os.environ["F4_MOCK_LOG"], "a", encoding="utf-8") as log:
            log.write(verb + (" env" if "--mock-env" in argv else " -") + chr(10))
    profile = argv[argv.index("--profile") + 1] if "--profile" in argv else ""
    with open(os.environ["F4_MOCK_TABLE"], "rb") as handle:
        table = json.loads(handle.read().decode("utf-8"))
    with open(source, "rb") as handle:
        key = hashlib.sha256(handle.read()).hexdigest()
    entry = table["sources"].get(key)
    if entry is not None and verb in entry:
        doc = json.loads(json.dumps(entry[verb]))
    elif table.get("fuzz"):
        doc = benign(profile)
    else:
        doc = {"exit": 2, "schema_version": 1, "profile_id": profile,
               "outcome": {"kind": "driver_error", "reason": "mock_unknown_source"}}
    code = doc.pop("exit")
    set_file(doc, source)
    doc, code, mode = corrupt(os.environ.get("F4_MOCK_CORRUPT", ""), doc, code, verb)
    if mode == "sleep":
        time.sleep(float(os.environ.get("F4_MOCK_SLEEP", "30")))
    text = (json.dumps(doc, ensure_ascii=False) + "\n").encode("utf-8")
    if mode == "absent":
        return code
    if mode == "double":
        text = text + text
    if mode == "panic":
        sys.stderr.write("thread 'main' panicked at mock\n")
    if mode == "invalid_ir":
        sys.stderr.write("InvalidIr: mock\n")
    sys.stdout.buffer.write(text)
    sys.stdout.buffer.flush()
    return code


if __name__ == "__main__":
    sys.exit(main())
