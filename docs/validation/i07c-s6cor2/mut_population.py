"""Frozen mutation population for the regions touched by I07c-S6-COR-2.

Uses the operator set and the literal/replacement machinery of scripts/i07c_mutaciones.py (same
tokens: ==/!=, </<=, &&/||, true/false, continue/break, +=1/-=1, dropped push/insert and
assignments) over whole regions instead of changed lines: `prepare_expr`, `guard_violations`
(whole functions) and a window of 15 lines on each side of the S1-S4 tick sites of lower.rs.
Usage: python -B mut_population.py <repo_root> <out.json>
"""
import hashlib, importlib.util, json, re, sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
spec = importlib.util.spec_from_file_location("mut", root / "scripts/i07c_mutaciones.py")
mut = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mut)

def fn_range(text, name):
    lines = text.splitlines()
    start = next(i for i, l in enumerate(lines, 1) if re.match(rf"\s*fn {name}\(", l))
    indent = len(lines[start - 1]) - len(lines[start - 1].lstrip())
    end = next(i for i in range(start + 1, len(lines) + 1)
               if lines[i - 1].startswith(" " * indent + "}") and len(lines[i - 1]) - len(lines[i - 1].lstrip()) == indent)
    return start, end

lower = (root / "src/ir/lower.rs").read_text(encoding="utf-8")
match = (root / "src/ir/lower_match.rs").read_text(encoding="utf-8")
ranges = {"src/ir/lower.rs": [fn_range(lower, "prepare_expr")], "src/ir/lower_match.rs": [fn_range(match, "guard_violations")]}
lines = lower.splitlines()
for needle in ("WorkEvent::ExpressionStatement,", "WorkEvent::Assignment,", "WorkEvent::WhileCondition,", "WorkEvent::ValueStatement,"):
    for i, l in enumerate(lines, 1):
        if needle in l:
            ranges["src/ir/lower.rs"].append((i - 15, i + 15))
mut.changed_lines = lambda base: {
    path: {n for a, b in spans for n in range(a, b + 1)} for path, spans in ranges.items()}
mut.ROOT = root
population = mut.candidates("HEAD")
blobs = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in ranges}
doc = {"regions": ranges, "blob_sha256": blobs, "count": len(population), "mutations": population}
raw = json.dumps(doc, ensure_ascii=False, indent=1)
Path(sys.argv[2]).write_text(raw, encoding="utf-8", newline="\n")
print(len(population), hashlib.sha256(raw.encode()).hexdigest())
