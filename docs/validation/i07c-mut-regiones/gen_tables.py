"""Renders the per-cut tables of the report from the frozen populations and the results.
Usage: python -B gen_tables.py <cut>   (prints markdown)"""
import collections, json, sys
from pathlib import Path

here = Path(__file__).resolve().parent
cut = sys.argv[1]
pop = json.loads((here / f"population-{cut}.json").read_text(encoding="utf-8"))
res = {}
rp = here / f"results-{cut}.jsonl"
if rp.exists():
    for line in rp.read_text(encoding="utf-8").splitlines():
        r = json.loads(line)
        res[r["id"]] = r
hunt = {}
hp = here / f"hunt-{cut}.json"
if hp.exists():
    hunt = json.loads(hp.read_text(encoding="utf-8"))
c = collections.Counter(r["verdict"] for r in res.values())
print(f"Población {pop['count']} (enumeración completa {pop['full_count']}), ejecutados {len(res)}: " + ", ".join(f"{k} {v}" for k, v in sorted(c.items())))
print()
print("| id | archivo:línea | operador | SHA ini | SHA mut | veredicto | test que lo mata / nota |")
print("|---|---|---|---|---|---|---|")
for m in sorted(pop["mutations"], key=lambda m: m["order"]):
    r = res.get(m["id"])
    if not r:
        continue
    last = r["profiles"][-1]
    note = (last.get("test") or "").replace("nexo::nexo$", "") if r["verdict"] == "MUERTO" else ""
    if r["verdict"] == "MUERTO":
        note = f"{last['profile']}: {note}"
    if m["id"] in hunt:
        note = hunt[m["id"]]
    print(f"| {m['id']} | {m['file']}:{m['line']} | {m['operator']} | {r['sha_initial'][:8]} | {r.get('sha_mutated', '-')[:8]} | {r['verdict']} | {note} |")
pend = [m["id"] for m in sorted(pop["mutations"], key=lambda m: m["order"]) if m["id"] not in res]
print()
print(f"Pendientes (no ejecutados, {len(pend)}): " + ", ".join(pend))
