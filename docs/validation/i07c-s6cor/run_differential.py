"""I07c-S6-COR differential: HEAD binary (base) against the working-tree binary (new).

Every source runs under f0-0002-b-v2, f0-0003-b-v1 and f0-0005-b-v1 with `check` and `run`, in
debug and release, on both binaries (relative file name `case.n`, one scratch directory per task).
Allowed difference, and only this one: the primary span of a diagnostic (and the first call step of
an effects cause chain, which names the same call target) moves from an expression with outer
`Expr::Group`s to the inner expression, or from the empty span of a grouped call target to the name
of that target. Everything else (code, subcause, order, data, relations, outputs, exits) must be
identical; any other difference, any debug/release or check/run divergence of the new binary and
any crash is listed as UNEXPLAINED.

Usage: python -B run_differential.py --base-debug B.exe --base-release B.exe --new-debug N.exe
       --new-release N.exe --sources corpus.jsonl [--sources-dir DIR ...] --scratch DIR --out PREFIX
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

PROFILES = ["f0-0002-b-v2", "f0-0003-b-v1", "f0-0005-b-v1"]
CALL_STEPS = {"function.call", "closure.call", "intrinsic.call"}
NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_.]*$")


def strip_groups(text):
    while text.startswith("(") and text.endswith(")"):
        depth = 0
        for index, char in enumerate(text):
            depth += char == "("
            depth -= char == ")"
            if depth == 0 and index < len(text) - 1:
                return text
        text = text[1:-1]
    return text


def run_one(exe, verb, profile, directory):
    try:
        done = subprocess.run([exe, verb, "case.n", "--profile", profile, "--format", "json"],
                              cwd=directory, capture_output=True, timeout=60)
    except subprocess.TimeoutExpired:
        return {"timeout": True}
    try:
        return {"exit": done.returncode, "json": json.loads(done.stdout.decode("utf-8")),
                "stderr": done.stderr.decode("utf-8", "replace")}
    except Exception:
        return {"exit": done.returncode, "raw": done.stdout.decode("utf-8", "replace")[:300],
                "stderr": done.stderr.decode("utf-8", "replace")[:300]}


def span(diagnostic):
    s = diagnostic.get("primary_span")
    return (s["start"], s["end"]) if s else None


def stripped(diagnostic):
    copy = json.loads(json.dumps(diagnostic))
    copy.pop("primary_span", None)
    if copy.get("stage") == "effects":
        for step in copy.get("cause_chain", []):
            if step.get("operation") in CALL_STEPS:
                step.pop("span", None)
    return copy


def classify(source, before, after):
    """Return (kind, detail). kind: same, primary, unexplained."""
    if before == after:
        return "same", []
    if "json" not in before or "json" not in after:
        return "unexplained", ["non-JSON or timeout output differs"]
    if before["exit"] != after["exit"] or before["stderr"] != after["stderr"]:
        return "unexplained", ["exit or stderr differs"]
    jb, ja = dict(before["json"]), dict(after["json"])
    db, da = jb.pop("diagnostics"), ja.pop("diagnostics")
    if jb != ja:
        return "unexplained", ["non-diagnostic output differs"]
    if len(db) != len(da):
        return "unexplained", [f"diagnostic count {len(db)} -> {len(da)}"]
    raw = source.encode("utf-8")
    moves = []
    for index, (b, a) in enumerate(zip(db, da)):
        if stripped(b) != stripped(a):
            return "unexplained", [f"diagnostic {index} differs beyond the primary"]
        sb, sa = span(b), span(a)
        if b.get("stage") == "effects":
            raw_text = lambda x: raw[x["start"]:x["end"]].decode("utf-8", "replace")
            for position, (cb, ca) in enumerate(zip(b["cause_chain"], a["cause_chain"])):
                if cb.get("operation") not in CALL_STEPS or cb.get("span") == ca.get("span"):
                    continue
                x, y = cb["span"], ca["span"]
                gap = raw[x["start"]:y["start"]].decode("utf-8", "replace")
                if not (x["start"] == x["end"] and gap and set(gap) <= {"("}
                        and NAME.match(raw_text(y))):
                    return "unexplained", [f"diagnostic {index} step {position}: {x} -> {y}"]
                moves.append({"index": index, "rule": b["rule"] + f" step{position}",
                              "stage": b["stage"], "operation": cb["operation"],
                              "before": [x["start"], x["end"]], "after": [y["start"], y["end"]],
                              "before_text": "", "after_text": raw_text(y),
                              "criterion": "call step: empty grouped head -> target name"})
        if sb == sa:
            continue
        before_text = raw[sb[0]:sb[1]].decode("utf-8", "replace")
        after_text = raw[sa[0]:sa[1]].decode("utf-8", "replace")
        if sb[0] == sb[1]:
            gap = raw[sb[0]:sa[0]].decode("utf-8", "replace")
            ok = (set(gap) <= {"("} and gap != "" and bool(NAME.match(after_text)))
            rule = "empty grouped call target -> target name"
        else:
            ok = (strip_groups(before_text) == after_text and before_text.startswith("(")
                  and sb[0] < sa[0] and sa[1] < sb[1])
            rule = "outer groups removed"
        if not ok:
            return "unexplained", [f"diagnostic {index}: {sb} -> {sa} {before_text!r} -> {after_text!r}"]
        moves.append({"index": index, "rule": b["rule"], "stage": b["stage"],
                      "operation": b["operation"], "before": list(sb), "after": list(sa),
                      "before_text": before_text, "after_text": after_text, "criterion": rule})
    return "primary", moves


def task(item):
    sid, source, profile, work, exes = item
    directory = Path(work) / f"t{sid}-{profile[3:7]}"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "case.n").write_bytes(source.encode("utf-8"))
    out = {"id": sid, "profile": profile, "verbs": {}}
    for verb in ("check", "run"):
        results = {name: run_one(exe, verb, profile, directory) for name, exe in exes.items()}
        out["verbs"][verb] = results
    for name in list(Path(directory).iterdir()):
        name.unlink()
    directory.rmdir()
    return out


def main():
    parser = argparse.ArgumentParser()
    for name in ("base-debug", "base-release", "new-debug", "new-release"):
        parser.add_argument(f"--{name}", required=True)
    parser.add_argument("--sources", action="append", default=[])
    parser.add_argument("--sources-dir", action="append", default=[])
    parser.add_argument("--rd", default=None)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--workers", type=int, default=8)
    parser.add_argument("--part", default="0/1", help="i/n: this batch takes sources with index %% n == i")
    args = parser.parse_args()
    exes = {"base-debug": args.base_debug, "base-release": args.base_release,
            "new-debug": args.new_debug, "new-release": args.new_release}
    sources = []
    for path in args.sources:
        for line in Path(path).read_text(encoding="utf-8").splitlines():
            sources.append(json.loads(line)["source"])
    for directory in args.sources_dir:
        for path in sorted(Path(directory).rglob("*.n")):
            sources.append(path.read_bytes().decode("utf-8"))
    if args.rd:
        for row in json.loads(Path(args.rd).read_text(encoding="utf-8"))["rows"]:
            sources.append(row["source"])
    unique = list(dict.fromkeys(sources))
    digest = hashlib.sha256("\0".join(unique).encode("utf-8")).hexdigest()
    print(f"sources={len(unique)} (from {len(sources)}) sha256={digest}", flush=True)
    work = Path(args.scratch)
    work.mkdir(parents=True, exist_ok=True)
    part, parts = (int(x) for x in args.part.split("/"))
    items = [(index, source, profile, str(work), exes)
             for index, source in enumerate(unique) if index % parts == part
             for profile in PROFILES]
    counts = {"executions": 0, "same": 0, "primary": 0, "unexplained": 0,
              "debug_release": 0, "check_run": 0, "base_debug_release": 0}
    primary_rows, unexplained = [], []
    divergences = []
    residual = []
    with concurrent.futures.ThreadPoolExecutor(args.workers) as pool:
        for done in pool.map(task, items, chunksize=8):
            source = unique[done["id"]]
            sha = hashlib.sha256(source.encode("utf-8")).hexdigest()[:16]
            for verb, results in done["verbs"].items():
                counts["executions"] += 4
                # debug/release of each binary must agree
                if results["new-debug"] != results["new-release"]:
                    counts["debug_release"] += 1
                    divergences.append({"sha": sha, "profile": done["profile"], "verb": verb,
                                        "what": "new debug != release"})
                if results["base-debug"] != results["base-release"]:
                    counts["base_debug_release"] += 1
                for build in ("debug", "release"):
                    kind, detail = classify(source, results[f"base-{build}"], results[f"new-{build}"])
                    counts[kind] += 1
                    if kind == "primary":
                        for move in detail:
                            primary_rows.append({"sha": sha, "profile": done["profile"], "verb": verb,
                                                 "build": build, **move})
                    elif kind == "unexplained":
                        unexplained.append({"sha": sha, "profile": done["profile"], "verb": verb,
                                            "build": build, "detail": detail, "source": source})
            # no diagnostic of the new binary may keep a bare group as its primary span
            raw = source.encode("utf-8")
            new_check = done["verbs"]["check"]["new-debug"]
            if "json" in new_check:
                for diagnostic in new_check["json"]["diagnostics"]:
                    where = diagnostic.get("primary_span")
                    if where:
                        text = raw[where["start"]:where["end"]].decode("utf-8", "replace")
                        if text.startswith("(") and strip_groups(text) != text:
                            residual.append({"sha": sha, "profile": done["profile"],
                                             "rule": diagnostic["rule"], "text": text})
            # check vs run of the new binary: same diagnostics
            for build in ("debug", "release"):
                c = done["verbs"]["check"][f"new-{build}"]
                r = done["verbs"]["run"][f"new-{build}"]
                cb = done["verbs"]["check"][f"base-{build}"]
                rb = done["verbs"]["run"][f"base-{build}"]
                base_agrees = ("json" in cb and "json" in rb
                               and cb["json"]["diagnostics"] == rb["json"]["diagnostics"])
                if (base_agrees and "json" in c and "json" in r
                        and c["json"]["diagnostics"] != r["json"]["diagnostics"]):
                    counts["check_run"] += 1
                    divergences.append({"sha": sha, "profile": done["profile"], "build": build,
                                        "what": "new check diagnostics != run diagnostics"})
    # check/run of the base binary may differ the same way: report only if the new differs and the base does not
    Path(args.out + "-summary.json").write_text(json.dumps(
        {"sources": len(unique), "sources_sha256": digest, "profiles": PROFILES, "counts": counts,
         "unexplained": unexplained, "divergences": divergences,
         "residual_group_primaries": residual}, indent=1), encoding="utf-8")
    with open(args.out + "-primary-diffs.tsv", "w", encoding="utf-8", newline="\n") as tsv:
        tsv.write("source_sha16\tprofile\tverb\tbuild\tdiag_index\trule\tstage\toperation\tbefore\tafter"
                  "\tbefore_text\tafter_text\tcriterion\n")
        for row in primary_rows:
            tsv.write("\t".join(str(row[key]) if not isinstance(row[key], str) else
                                row[key].replace("\t", " ").replace("\n", " ") for key in (
                "sha", "profile", "verb", "build", "index", "rule", "stage", "operation", "before",
                "after", "before_text", "after_text", "criterion")) + "\n")
    print(json.dumps(counts))
    print(f"primary moves={len(primary_rows)} unexplained={len(unexplained)} "
          f"divergences={len(divergences)} residual_group_primaries={len(residual)}")
    return 1 if unexplained or divergences else 0


if __name__ == "__main__":
    sys.exit(main())
