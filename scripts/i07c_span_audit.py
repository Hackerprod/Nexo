"""Audit raw expression spans in the deferred type solver.

`primary_span` chooses a diagnostic operand; raw spans below serve only
identity, lexical order/containment, or the range of an operator. Group wrappers
must retain their distinct keys so propagated solver state can be copied to
the wrapper without losing the child. A new raw use needs an explicit review.
"""

from pathlib import Path
import re


SOURCE = Path(__file__).resolve().parents[1] / "src/check/types.rs"
classes = {"identity": [], "boundary": [], "operator": [], "internal_result": [], "primary_fallback": [], "test": []}
source_lines = SOURCE.read_text(encoding="utf-8").splitlines()
tests_begin = source_lines.index("mod quota_tests {") + 1
for number, line in enumerate(source_lines, 1):
    if ".span()" not in line:
        continue
    if number >= tests_begin:
        kind = "test"
    elif "span_key(" in line:
        kind = "identity"
    elif "operator_span(" in line:
        kind = "operator"
    elif "active_if_span.replace(e.span().clone())" in line:
        # Scope identity for the current if; never a diagnostic primary.
        kind = "identity"
    elif re.search(r"\.span\(\)\.(start|end)", line):
        kind = "boundary"
    elif re.search(r"(?:result: e|_ => e)\.span\(\)\.clone\(\)", line):
        kind = "internal_result" if "result:" in line else "primary_fallback"
    elif "e.span()," in line and "self.defer_domain_result(" in "\n".join(source_lines[number - 10:number]):
        # Deferred domain result key, not a diagnostic primary.
        kind = "identity"
    else:
        raise SystemExit(f"unreviewed raw span at {SOURCE}:{number}: {line.strip()}")
    classes[kind].append(number)

reasons = {
    "identity": "solver identity and deferred result keys; wrapper and child must remain distinct",
    "boundary": "token search, source order, or containment of an inner join in its enclosing operand",
    "operator": "lexical gap between operand ranges, used to locate the operator token",
    "internal_result": "type table key for the complete result expression",
    "primary_fallback": "non-Group fallback; Group is recursively unwrapped first",
    "test": "test code, outside production diagnostics",
}
for kind, lines in classes.items():
    print(f"{kind}: {len(lines)} lines={','.join(map(str, lines))} — {reasons[kind]}")
print(f"total: {sum(map(len, classes.values()))}")
