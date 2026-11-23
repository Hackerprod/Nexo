"""Frozen mutation population for the regions of integrated I07c cuts (S6-COR, S5b.1).

Region = production lines of HEAD attributed (git blame) to the commits of a cut's range
(`<base>..<end>`), in the files listed for the cut. Operators: those of scripts/i07c_mutaciones.py
(candidates(), unchanged: ==/!=, </<=, &&/||, true/false, continue/break, +=1/-=1, dropped
record_error/push/insert/assignments, swap primary_span, swap unify args) plus the EXTRA set below
(>/>=, +/-, .start/.end, is_some/is_none, is_ok/is_err, min/max, first/last, any/all, integer
constants +-1, negation of `if` conditions and match guards, dropped `return`, swapped arguments of
two-argument calls). If the population exceeds the cap it is sampled deterministically with
random.Random(seed) over the ordered list (no result-based choice); an execution order is fixed
beforehand by a seeded shuffle (field `order`), so a time-limited run covers an unbiased subset.
Usage: python -B mut_population.py <repo_root> <cut> <out.json>
"""
import functools, hashlib, importlib.util, json, random, re, subprocess, sys
from pathlib import Path

CUTS = {
    "s6cor": {"base": "10109388bcb2dd937277589e23d14ff56b82228a", "end": "2bc5c8d5bea61be167c922ad247fad9668ca2a52",
              "files": ["src/frontend/parse.rs", "src/ir/effects.rs", "src/check/names.rs", "src/check/types.rs"],
              "cap": 400, "seed": 20261123},
    "s5b1": {"base": "4acee20e5172f18340540a7761c05ea6feebc40b", "end": "10109388bcb2dd937277589e23d14ff56b82228a",
             "files": ["src/check/types.rs"], "cap": 400, "seed": 20261123},
}
root = Path(sys.argv[1]).resolve(); cut = CUTS[sys.argv[2]]; out = Path(sys.argv[3])
git = lambda *a: subprocess.run(["git", *a], cwd=root, check=True, capture_output=True, text=True, encoding="utf-8").stdout
in_range = set(git("rev-list", f"{cut['base']}..{cut['end']}").split())
spec = importlib.util.spec_from_file_location("mut", root / "scripts/i07c_mutaciones.py")
mut = importlib.util.module_from_spec(spec); spec.loader.exec_module(mut); mut.ROOT = root
mut.test_only_regions = functools.lru_cache(maxsize=None)(mut.test_only_regions)
mut.code_mask = functools.lru_cache(maxsize=None)(mut.code_mask)


def enclosing_fns(text, blamed):
    """Whole-function expansion: every `fn` (by indentation) that contains a blamed line is in the region."""
    ls = text.splitlines()
    out_ = set()
    for n in sorted(blamed):
        i = n - 1
        while i >= 0:
            m = re.match(r"^(\s*)(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+\w+", ls[i])
            if m:
                ind = len(m.group(1))
                j = i
                while j < len(ls) and not (j > i and ls[j].startswith(" " * ind + "}") and len(ls[j]) - len(ls[j].lstrip()) == ind) and not (j == i and ls[j].rstrip().endswith("}") and "{" in ls[j]):
                    j += 1
                if j >= n - 1:
                    out_.update(range(i + 1, j + 2))
                    break
            i -= 1
    return out_
lines = {}
for f in cut["files"]:
    s = set()
    for l in git("blame", "--line-porcelain", "HEAD", "--", f).splitlines():
        h = l.split(" ")
        if len(h) >= 3 and len(h[0]) == 40 and h[2].isdigit() and h[0] in in_range:
            s.add(int(h[2]))
    lines[f] = s | enclosing_fns((root / f).read_text(encoding="utf-8"), s)
mut.changed_lines = lambda base: {f: s for f, s in lines.items() if s}
full = mut.candidates("HEAD")

EXTRA = (
    ("gt", r"(?<=\s)>(?=\s)", ">="), ("ge", r"(?<=\s)>=(?=\s)", ">"),
    ("add", r"(?<=\s)\+(?=\s)", "-"), ("sub", r"(?<=\s)-(?=\s)", "+"),
    ("start_end", r"\.start\b(?!\()", ".end"), ("end_start", r"\.end\b(?!\()", ".start"),
    ("is_some", r"\bis_some\(\)", "is_none()"), ("is_none", r"\bis_none\(\)", "is_some()"),
    ("is_ok", r"\bis_ok\(\)", "is_err()"), ("is_err", r"\bis_err\(\)", "is_ok()"),
    ("min_max", r"\.min\(", ".max("), ("max_min", r"\.max\(", ".min("),
    ("first_last", r"\.first\(\)", ".last()"), ("last_first", r"\.last\(\)", ".first()"),
    ("any_all", r"\.any\(", ".all("), ("all_any", r"\.all\(", ".any("),
    ("int_inc", r"(?<![\w.'\"])\d+(?![\w.])", None), ("int_dec", r"(?<![\w.'\"])[1-9]\d*(?![\w.])", None),
)


def extras():
    out_ = []
    for path, ls in sorted(lines.items()):
        if not ls:
            continue
        source = (root / path).read_text(encoding="utf-8")
        mask = mut.code_mask(source)
        starts = [0] + [i + 1 for i, c in enumerate(source) if c == "\n"]
        for line in sorted(ls):
            if line > len(starts):
                continue
            b = starts[line - 1]
            e = starts[line] - 1 if line < len(starts) else len(source)
            vis = mask[b:e]
            found = []
            for name, pat, rep in EXTRA:
                for m in re.finditer(pat, vis):
                    r = rep
                    if name == "int_inc":
                        r = str(int(m.group(0)) + 1)
                    if name == "int_dec":
                        r = str(int(m.group(0)) - 1)
                    found.append((b + m.start(), b + m.end(), name, r))
            m = re.match(r"^(\s*(?:\}\s*else\s+)?if\s+)(?!let\b)(.+?)(\s*\{\s*)$", vis)
            if m and "=>" not in m.group(2):
                found.append((b + m.start(2), b + m.end(2), "negate_if", "!(" + source[b + m.start(2):b + m.end(2)] + ")"))
            m = re.search(r"\bif\s+(?!let\b)([^=]+?)\s*=>", vis)
            if m:
                found.append((b + m.start(1), b + m.end(1), "negate_guard", "!(" + source[b + m.start(1):b + m.end(1)] + ")"))
            for m in re.finditer(r"\breturn\b[^;]*;", vis):
                found.append((b + m.start(), b + m.end(), "drop_return", ""))
            for call in re.finditer(r"[\w:.]+\(", vis):
                if vis[call.start():call.start() + 3] in ("if ", "whi"):
                    continue
                st = call.end(); depth = 1; commas = []; q = st
                while q < len(vis) and depth:
                    c = vis[q]
                    if c in "([{":
                        depth += 1
                    elif c in ")]}":
                        depth -= 1
                    elif c == "," and depth == 1:
                        commas.append(q)
                    q += 1
                if depth or len(commas) != 1:
                    continue
                a = source[b + st:b + commas[0]].strip()
                c2 = source[b + commas[0] + 1:b + q - 1].strip()
                if not a or not c2 or a == c2 or "|" in a + c2:
                    continue
                pre = re.match(r"\s*", source[b + st:b + q - 1]).group(0)
                found.append((b + st, b + q - 1, "swap_args", pre + c2 + ", " + a))
            for st, en, op, rep in sorted(set(found)):
                try:
                    lit, mutd = mut.unique_literal(source, st, en, rep)
                except AssertionError:
                    continue
                out_.append({"file": path, "line": line, "column": st - b + 1, "operator": op, "literal": lit,
                             "replacement": mutd, "site_count": 1})
    return out_


seen = {(m["file"], m["literal"], m["replacement"]) for m in full}
for m in extras():
    k = (m["file"], m["literal"], m["replacement"])
    if k not in seen and not mut.is_test_only(m) and m["literal"] != m["replacement"]:
        seen.add(k)
        full.append(m)
full.sort(key=lambda m: (m["file"], m["line"], m["column"], m["operator"]))
for i, m in enumerate(full, 1):
    m["id"] = f"M{i:05}"
    m["full_id"] = m["id"]
discarded = 0
if len(full) > cut["cap"]:
    idx = sorted(random.Random(cut["seed"]).sample(range(len(full)), cut["cap"]))
    discarded = len(full) - cut["cap"]
    pop = [full[i] for i in idx]
    for k, m in enumerate(pop, 1):
        m["id"] = f"M{k:05}"
else:
    pop = full
order = list(range(len(pop)))
random.Random(cut["seed"] + 1).shuffle(order)
for rank, i in enumerate(order, 1):
    pop[i]["order"] = rank
blobs = {f: hashlib.sha256(git("show", f"HEAD:{f}").encode("utf-8")).hexdigest() for f in cut["files"]}
doc = {"cut": sys.argv[2], "base": cut["base"], "end": cut["end"], "head": git("rev-parse", "HEAD").strip(),
       "seed": cut["seed"], "cap": cut["cap"], "full_count": len(full), "discarded_by_sampling": discarded,
       "region_lines": {f: len(s) for f, s in lines.items()}, "region_rule": "lines blamed to the cut range, expanded to each whole enclosing fn", "blob_sha256": blobs, "count": len(pop), "mutations": pop}
out.write_text(json.dumps(doc, ensure_ascii=False, indent=1), encoding="utf-8", newline="\n")
from collections import Counter
print(sys.argv[2], len(full), len(pop), doc["region_lines"], dict(Counter(m["operator"] for m in pop)))
print(hashlib.sha256(out.read_bytes()).hexdigest())
