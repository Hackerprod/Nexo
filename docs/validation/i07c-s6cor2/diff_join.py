"""Joins the release and debug result files: lists every base/candidate difference and every
candidate debug/release difference. Usage: diff_join.py release.jsonl debug.jsonl out.json"""
import json, sys
rel = {r["name"]: r for r in map(json.loads, open(sys.argv[1], encoding="utf-8"))}
dbg = {r["name"]: r for r in map(json.loads, open(sys.argv[2], encoding="utf-8"))}
out = {"release_sources": len(rel), "debug_sources": len(dbg), "base_vs_candidate": [], "debug_vs_release": []}
for mode, rows in (("release", rel), ("debug", dbg)):
    for name, r in sorted(rows.items()):
        for key, (b, c) in r["cells"].items():
            if b != c:
                out["base_vs_candidate"].append({"name": name, "cell": key, "mode": mode, "base": b, "cand": c,
                                                 "texts": r["texts"].get(key)})
for name, d in sorted(dbg.items()):
    for key, (b, c) in d["cells"].items():
        for side, i in (("base", 0), ("cand", 1)):
            if d["cells"][key][i] != rel[name]["cells"][key][i]:
                out["debug_vs_release"].append({"name": name, "cell": key, "side": side})
json.dump(out, open(sys.argv[3], "w", encoding="utf-8"), indent=1)
print(out["release_sources"], out["debug_sources"], len(out["base_vs_candidate"]), len(out["debug_vs_release"]))
