"""Builds the patch JSON files: the inverse mutation of each solution (from the auditor registry
GROUP_SITES: the group-removal call goes back to the raw access) and the frozen population.
Usage: mut_patches.py <repo_root> <population.json> <out_dir>"""
import hashlib, importlib.util, json, re, sys
from pathlib import Path
root, pop, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]), Path(sys.argv[3])
out.mkdir(parents=True, exist_ok=True)
spec = importlib.util.spec_from_file_location("a", root / "scripts/i07c_span_audit.py")
a = importlib.util.module_from_spec(spec); spec.loader.exec_module(a)
sha = lambda b: hashlib.sha256(b).hexdigest()
def patch(file, before, after, occurrence=0):
    text = (root / file).read_bytes().decode("utf-8")
    at = [m.start() for m in re.finditer(re.escape(before), text)][occurrence]
    mutated = text[:at] + after + text[at + len(before):]
    return {"file": file, "before": before, "after": after, "occurrence": occurrence,
            "sha_initial": sha(text.encode()), "sha_mutated": sha(mutated.encode())}
ids = []
for ident, (name, pattern, raw) in a.GROUP_SITES.items():
    file = "src/ir/" + name
    text = (root / file).read_bytes().decode("utf-8")
    m = re.search(pattern, text)
    before = text[m.start("site"):m.end("site")]
    occurrence = len(re.findall(re.escape(before), text[:m.start("site")]))
    raw = {"S5": "target.as_ref()", "S7": "target.as_ref()"}.get(ident, raw)  # the auditor raw text is not compilable there
    p = patch(file, before, raw, occurrence)
    key = "inv-" + ident.replace(" ", "-")
    (out / f"{key}.json").write_text(json.dumps(p, indent=1), encoding="utf-8"); ids.append(key)
for m in json.loads(pop.read_text(encoding="utf-8"))["mutations"]:
    p = patch(m["file"], m["literal"], m["replacement"])
    (out / f"{m['id']}.json").write_text(json.dumps(p, indent=1), encoding="utf-8"); ids.append(m["id"])
print(len(ids))
