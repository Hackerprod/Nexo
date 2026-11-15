"""J14 B1-R1: literal 62-cell production-state differential.

Usage: python -B tests/j14_r1_manifest.py OLD_DEBUG NEW_DEBUG [OLD_RELEASE NEW_RELEASE]
The expected values below are the J14-b1-r1.md v4 table, not compiler output.
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

W = ("for", "yield", "break", "continue", "gen")
E = ["(", "-", "IntLiteral", "Name", "[", "edit", "false", "fn", "if", "match", "move", "true"]
T = ["(", "-", "IntLiteral", "Name", "[", "edit", "false", "fn", "move", "true"]
G = ["(", ")", "-", "IntLiteral", "Name", "[", "edit", "false", "fn", "if", "match", "move", "true"]
A = ["(", "-", "IntLiteral", "Name", "[", "]", "edit", "false", "fn", "if", "match", "move", "true"]
B = ["(", "-", "IntLiteral", "Name", "[", "edit", "false", "fn", "move", "true", "{"]
K = ["(", "-", "IntLiteral", "Name", "[", "break", "continue", "edit", "false", "fn", "for", "if", "let", "match", "move", "return", "true", "var", "while", "yield", "{", "}"]

# template, token byte start, new context, FIRST, new prefix start
ROWS = (
    ("fn Main(){let x={w};}", 16, "binding_initializer", E, 10),
    ("gen fn G(){yield {w};}", 17, "grammar.YieldStmt", E, 11),
    ("fn Main(){return {w};}", 17, "statement_terminator", [";"], 10),
    ("fn Main(){let x=if true then {value: {w};} else 0;}", 37, "missing_value_operand", [], None),
    ("fn Main(){let x=1+{w};}", 18, "grammar.TryExpr", T, 18),
    ("fn Main(){Print({w});}", 16, "grammar.Arguments", G, 15),
    ("fn Main(){Print(1,{w});}", 18, "grammar.Argument", E, 18),
    ("fn Main(){let x=[{w}];}", 17, "grammar.Primary", A, 16),
    ("fn Main(){let x=[1,{w}];}", 19, "array_element", E, 16),
    ("fn Main(){let x=({w});}", 17, "grammar.Primary", G, 16),
    ("fn Main(){let x=if true then {w} else 0;}", 29, "grammar.BranchOperand", B, 29),
    ("fn Main(){let x=if true then 0 else {w};}", 36, "grammar.BranchOperand", B, 16),
)
BLOCK = (
    ("fn Main(){gen}", 10, "block_item", 9),
    ("fn Main(){let x=1;gen}", 18, "grammar.Block", 9),
)
NO_DELTA = (
    "fn Main(){let x=1;}",
    "fn Main(){if gen{}}",
    "fn Main(){while gen{}}",
    "fn Main(){match gen{_=>{}}}",
    "fn Main(){var x=0;x=gen;}",
    "fn Main(){let x=[1];let y=x[gen];}",
    "fn Main(){match 1{_ if gen=>{}}}",
    "fn Main(){Print(label:gen);}",
    "fn Main(){break(1);}",
    "fn Main(){for edit x in G(){}}",
    "fn Main(){let x=self;}",
    "fn Main(){let x=struct S;}",
    "gen fn G(){yield 1;}fn Main(){}",
    "fn Main(){let x=1;for x in G(){}}",
    "fn Main(){let x=1;break;}",
    "fn Main(){let x=1;continue;}",
)


def span(start, end):
    return {"file": "case.n", "start": start, "end": end}


def diagnostic(code, token, start, context, first, prefix):
    q = start + len(token)
    if code == "NX-E1302":
        return dict(code=code, subcause="missing_value_operand", rule="NX-E1302/missing_value_operand",
                    explanation_id="NX-E1302/missing_value_operand", message="Invalid branch value syntax.",
                    primary_span=span(30, 36), related_spans=[{"role": "context", "span": span(29, 30)},
                                                             {"role": "delimiter", "span": span(start, q)}],
                    unexpected_token=None, expected_tokens=[], expected_tokens_kind=None,
                    parser_context=None, recognized_prefix=None, repair=[])
    repair = []
    related = []
    message = "Syntax is not admitted by the grammar."
    kind = "admissible_set"
    if code == "NX-E0005":
        message = "Falta un terminador de sentencia."
        kind = "canonical_insertion"
        related = [{"role": "context", "span": span(10, 16)}]
        repair = [{"id": "insert_terminator", "status": "proposed",
                   "description": "Insertar un terminador explícito.",
                   "edits": [{"span": span(16, 16), "expected_text": "", "replacement": ";"}],
                   "validation": None, "consequences": ["Termina la sentencia antes del siguiente token."]}]
    return dict(code=code, subcause=None, rule=code, explanation_id=code, message=message,
                primary_span=span(start, q), related_spans=related,
                unexpected_token={"kind": "keyword", "text": token, "span": span(start, q)},
                expected_tokens=first, expected_tokens_kind=kind, parser_context=context,
                recognized_prefix=span(prefix, start), repair=repair)


def expected(code, token, start, context, first, prefix):
    d = diagnostic(code, token, start, context, first, prefix)
    d.update(schema_version=1, diagnostic_namespace="Nexo-proposed-v1", profile_id="f0-0003-b-v1",
             severity="error", stage="parse", operation="syntax.parse", cause_chain=[],
             feature_id=None, data={})
    return d


def run(binary, command, source, expected_status=1):
    with tempfile.TemporaryDirectory(prefix="j14-r1-") as directory:
        path = Path(directory) / "case.n"
        path.write_bytes(source.encode("utf-8"))
        proc = subprocess.run([binary, command, str(path), "--profile", "f0-0003-b-v1", "--format", "json"],
                              capture_output=True, text=True, encoding="utf-8", timeout=30)
        assert proc.returncode == expected_status, (binary, command, source, proc.returncode, proc.stdout, proc.stderr)
        result = json.loads(proc.stdout)
        def normalize(value):
            if isinstance(value, dict):
                return {k: ("case.n" if k == "file" else normalize(v)) for k, v in value.items()}
            if isinstance(value, list):
                return [normalize(v) for v in value]
            return value
        return normalize(result)


def check_pair(old, new, source, token, start, context, first, prefix):
    old_expected = expected("NX-E0002", token, start, "grammar.Primary", E, start)
    new_code = "NX-E1302" if context == "missing_value_operand" else ("NX-E0005" if context == "statement_terminator" else "NX-E0002")
    new_expected = expected(new_code, token, start, context, first, prefix)
    for command in ("check", "run"):
        before = run(old, command, source)
        after = run(new, command, source)
        assert before != after, (source, command, "missing delta")
        for label, report, wanted in (("old", before, old_expected), ("new", after, new_expected)):
            assert report["diagnostics"] == [wanted], (label, source, command, report["diagnostics"], wanted)
            assert report["outcome"] == {"kind": "rejected"}, (label, source, command, report)
            assert report["stdout"] == report["events"] == report["observations"] == [], (label, source, command, report)
            assert report["observation_bytes"] == report["reserved_observation_bytes"] == 0, (label, source, command, report)


def main():
    pairs = list(zip(sys.argv[1::2], sys.argv[2::2]))
    assert len(pairs) in (1, 2), "provide debug pair and optional release pair"
    seen = set()
    for row_index, (template, start, context, first, prefix) in enumerate(ROWS):
        for word in W:
            source = template.replace("{w}", word)
            assert source[start:start + len(word)] == word, (row_index, word)
            key = (row_index, word)
            assert key not in seen
            seen.add(key)
            for old, new in pairs:
                check_pair(old, new, source, word, start, context, first, prefix)
    for row_index, (source, start, context, prefix) in enumerate(BLOCK, len(ROWS)):
        key = (row_index, "gen")
        assert key not in seen
        seen.add(key)
        for old, new in pairs:
            for command in ("check", "run"):
                before = run(old, command, source)
                after = run(new, command, source)
                assert before != after, (source, command, "missing delta")
                assert before["diagnostics"] == [expected("NX-E0002", "gen", start, "block_item", K, start)]
                assert after["diagnostics"] == [expected("NX-E0002", "gen", start, context, K, prefix)]
    assert len(seen) == 62
    for source in NO_DELTA:
        for old, new in pairs:
            for command in ("check", "run"):
                with tempfile.TemporaryDirectory(prefix="j14-r1-control-") as directory:
                    path = Path(directory) / "case.n"
                    path.write_bytes(source.encode("utf-8"))
                    args = [command, str(path), "--profile", "f0-0003-b-v1", "--format", "json"]
                    a = subprocess.run([old, *args], capture_output=True, timeout=30)
                    b = subprocess.run([new, *args], capture_output=True, timeout=30)
                    assert (a.returncode, a.stdout) == (b.returncode, b.stdout), (source, command, a.stdout, b.stdout)
    print(f"R1: {len(seen)} exact delta cells, {len(NO_DELTA)} no-delta controls, {len(pairs)} binary pair(s), check/run")


if __name__ == "__main__":
    main()
