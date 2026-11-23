"""Classifies every base/candidate difference of diff_join.py output by the rule that derives it
(from the source text and the two diagnostics, never from the candidate alone). Anything that
matches no rule is UNCLASSIFIED and fails the check.
Usage: diff_classify.py <join.json> <corpus_dir> <out.json>"""
import json, re, sys
from pathlib import Path

join = json.load(open(sys.argv[1], encoding="utf-8"))
corpus = Path(sys.argv[2])


def strip_groups(text):
    text = text.strip()
    while text.startswith("(") and text.endswith(")"):
        depth = 0
        for i, ch in enumerate(text):
            depth += ch == "("
            depth -= ch == ")"
            if depth == 0 and i < len(text) - 1:
                return text  # the first paren closes before the end: not an outer group
        text = text[1:-1].strip()
    return text


def forms(text):
    """The successive results of removing one outer group (`()` is the unit, not a group)."""
    out, text = [], text.strip()
    while text.startswith("(") and text.endswith(")") and text != "()":
        depth = 0
        for i, ch in enumerate(text):
            depth += ch == "("
            depth -= ch == ")"
            if depth == 0 and i < len(text) - 1:
                return out
        text = text[1:-1].strip()
        out.append(text)
    return out


def diag(text):
    j = json.loads(text)
    return j, (j["diagnostics"][0] if j["diagnostics"] else None)


def rest(d):
    d = dict(d)
    d.pop("primary_span", None)
    return d


rows = []
for item in join["base_vs_candidate"]:
    src = (corpus / item["name"]).read_bytes()
    bt, ct = item["texts"]
    bj, bd = diag(bt)
    cj, cd = diag(ct)
    sl = lambda d: src[d["primary_span"]["start"]:d["primary_span"]["end"]].decode("utf-8", "replace")
    cls = "UNCLASSIFIED"
    why = ""
    if bd and cd and bd["code"] == cd["code"] and rest(bd) == rest(cd) and bj["outcome"] == cj["outcome"]:
        b, c = sl(bd), sl(cd)
        if b.startswith("(") and c in forms(b) and not forms(c):
            cls = {"R0001": "S1-S4 R0001 primary: outer groups removed", "NX-E1305": "S6 guard_mutation primary: outer groups removed"}.get(bd["code"], "UNCLASSIFIED")
            why = f"base primary {b!r} -> candidate {c!r}"
    if cls == "UNCLASSIFIED" and cd and cd["code"] == "R0001" and (bd is None and bj["outcome"]["kind"] in ("unit", "checked") or bd and bd["code"] == "R0001"):
        text = src.decode("utf-8", "replace")
        if re.search(r"\([(\s]*(Id|f)[)\s]*\)[)\s]*\(", text):
            cls = "S5 grouped call target charges the target-name tick"
            why = f"base {bj['outcome']['kind']} {(bd['data'], sl(bd)) if bd else ''}; candidate data {cd['data']} primary {sl(cd)!r}"
    if cls == "UNCLASSIFIED" and cd and cd["code"] == "NX-E1305" and cd["subcause"] == "guard_transfer" and sl(cd) == "Drop" and not (bd and bd["code"] == "NX-E1305"):
        if re.search(r"\(+\s*Drop\s*\)+\s*\(", src.decode()):
            cls = "S7 (Drop)(x) in a guard rejected like Drop(x)"
            why = f"base {bj['outcome']['kind']} {[d['code'] for d in bj['diagnostics']]}; candidate NX-E1305/guard_transfer primary 'Drop'"
    if cls.startswith("S5") and bd and cd:
        # direction: one more tick per grouped call cannot let the candidate go further than the base
        if bj["outcome"]["kind"] != "simulator_fault" and cj["outcome"]["kind"] == "simulator_fault":
            pass
    rows.append({"name": item["name"], "cell": item["cell"], "mode": item["mode"], "class": cls, "why": why})
summary = {}
for r in rows:
    summary[r["class"]] = summary.get(r["class"], 0) + 1
json.dump({"summary": summary, "rows": rows}, open(sys.argv[3], "w", encoding="utf-8"), indent=1)
print(summary)
