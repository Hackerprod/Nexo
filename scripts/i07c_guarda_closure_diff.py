#!/usr/bin/env python3
"""Differential harness of the S6COR-GUARDA-CLOSURE cut (andamiaje).

Corpus and fuzz generators are frozen (plan docs/correcciones/I07c-S6COR-GUARDA-CLOSURE-plan.md,
Apendices A and C): their SHA-256 is checked before anything runs. The harness compares a base
binary with a candidate one over `check` and `run`, profiles f0-0003-b-v1 and f0-0005-b-v1.

Modes (one per call):
  --corpus          840 frozen sources; classes that must change vs classes that must not.
  --gen2 SEED       1500 sources of the S5c generator (fn + match, no closures): 0 changes.
  --fuzz N          frozen fuzz (N = 3000): no panics, check/run and debug/release agree,
                    no base-rejected source becomes accepted.
  --files PATH...   files or directories of .n/.nx sources: 0 changes.
  --self-test       generator hashes, comparison logic on synthetic data and, when
                    --base-debug is given, base == base on a slice of the corpus.

Binaries: --base-debug/--cand-debug (required except for --self-test), optional
--base-release/--cand-release (debug == release is then required for base and candidate).
"""

import argparse
import concurrent.futures
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
PROFILES = ("f0-0003-b-v1", "f0-0005-b-v1")
VERBS = ("check", "run")
CORPUS_SHA = "b82fa323f5d4c8ab0f0101017e02206b08857066aeeff1d06310e3a5ea97b860"
FUZZ_SHA = "a797ef463114cde39105012fd6f85e210ae7ab9b027ac5211ee87be092c90e01"
GEN2_SHA256 = "11f10ff1cc3dab3bcbc7f8964951de23634b39549bdff1f8b9f23f477b357b90"
GEN2_BLOB = "11ac8537bda405ae60b97a444dea112d2e41a5dc"
GEN2_SEEDS = {
    "6112023": "48f6e809e10f701ae6b746ff21a523bf06c9cd8e9e1177da75b4b43b688213c4",
    "1357911": "9e4c4d9f96dbbe469d30f6aeb163bb69576b0cba3a8bb07b41674bb373ffe513",
    "424242": "f9952957f7093440140c2e831551fdb1e3ce11996bceb0fed57bc32eb96bab3e",
}
GEN2_REF = "origin/i07c-estructural-r2"
GEN2_PATH = "docs/validation/i07c-s5c/b-d2/gen2.py"
TIME_LIMIT = 60.0

# Classes whose source must change (closure called in a guard, or guard inside a closure, with effect).
CHANGING = {
    "closure_guard_eff", "literal_guard_eff", "closure_param_eff", "closure_transitive_eff",
    "closure_chain_eff", "closure_capture_read_eff", "closure_capture_move_eff",
    "closure_pattern_bound_eff", "closure_two_calls_eff", "guard_in_closure_fn_eff",
    "guard_in_closure_cl_eff", "guard_in_closure_direct_eff",
}

# ---------------------------------------------------------------------------------------------
# Frozen generator, plan Appendix A (no randomness: the order is that of the lists).
BODIES = [
    ("pure", "return true;", False),
    ("cmp", "return 1==1;", False),
    ("print", "Print(1);return true;", True),
    ("unknown", "MockUnknown();return true;", True),
    ("open", "let r=MockOpen(1);return true;", True),
    ("array", "let a=[1];return true;", True),
    ("req", "let q=MockRequest();return true;", True),
    ("block", "let r=MockBlockingOpen(1);return true;", True),
]
CALLS = ["{n}()", "({n})()", "(({n}))()", "{n}()==true", "(if {n}() then true else false)", "(if true then {n}() else false)"]
CTX = [
    "match true{true if %s=>{},_=>{}}",
    "match false{true if %s=>{},_=>{}}",
    "let x=match true{true if %s=>{value:1;},_=>{value:2;}};",
    "match Some(1){Some(v) if %s=>{},_=>{}}",
    "if true{match true{true if %s=>{},_=>{}}}",
]


def gen_corpus():
    out = []

    def emit(kind, tag, src):
        out.append({"kind": kind, "tag": tag, "src": src})

    def eff(kind, e):
        return kind + ("_eff" if e else "_pure")

    for bid, body, e in BODIES:
        for ci, call in enumerate(CALLS):
            for xi, ctx in enumerate(CTX):
                tag = "%s-c%d-x%d" % (bid, ci, xi)
                emit(eff("closure_guard", e), tag, "fn Main(){let g=fn ()->Bool{%s};%s}" % (body, ctx % call.format(n="g")))
                emit(eff("fn_guard", e), tag, "fn G()->Bool{%s}fn Main(){%s}" % (body, ctx % call.format(n="G")))
                emit("closure_outside_guard", tag, "fn Main(){let g=fn ()->Bool{%s};let y=g();%s}" % (body, ctx % "true"))
    for bid, body, e in BODIES:
        for gi, form in enumerate(["(fn ()->Bool{%s})()", "((fn ()->Bool{%s}))()"]):
            emit(eff("literal_guard", e), "%s-l%d" % (bid, gi), "fn Main(){match true{true if %s=>{},_=>{}}}" % (form % body))
        emit(eff("guard_in_closure_fn", e), bid, "fn G()->Bool{%s}fn Main(){let f=fn (){match true{true if G()=>{},_=>{}}};}" % body)
        emit(eff("guard_in_closure_cl", e), bid, "fn Main(){let g=fn ()->Bool{%s};let f=fn (){match true{true if g()=>{},_=>{}}};}" % body)
        emit(eff("guard_in_closure_direct", e), bid, "fn Main(){let f=fn (){match true{true if (if true then {%s value:true;} else {value:false;})=>{},_=>{}}};}" % body.replace("return true;", ""))
        emit(eff("contract_cl", e), bid, "fn Main() with no_io{let g=fn ()->Bool{%s};let y=g();}" % body)
        emit(eff("contract_fn", e), bid, "fn G()->Bool{%s}fn Main() with no_io{let y=G();}" % body)
        emit(eff("closure_param", e), bid, "fn Main(){let g=fn (x:view I64)->Bool{%s};match true{true if g(1)=>{},_=>{}}}" % body)
        emit(eff("closure_transitive", e), bid, "fn G()->Bool{%s}fn Main(){let g=fn ()->Bool{return G();};match true{true if g()=>{},_=>{}}}" % body)
        emit(eff("closure_chain", e), bid, "fn Main(){let h=fn ()->Bool{%s};let g=fn ()->Bool{return h();};match true{true if g()=>{},_=>{}}}" % body)
        emit(eff("closure_capture_read", e), bid, "fn Main(){let x=1;let g=fn ()->Bool{let y=x;%s};match true{true if g()=>{},_=>{}}}" % body)
        emit(eff("closure_capture_move", e), bid, "fn Main(){let z=[1];let g=fn [move z]()->Bool{%s};match true{true if g()=>{},_=>{}}}" % body)
        emit(eff("closure_pattern_bound", e), bid, "fn Main(){let g=fn ()->Bool{%s};match Some(g){Some(h) if h()=>{},_=>{}}}" % body)
        emit(eff("closure_two_calls", e), bid, "fn Main(){let g=fn ()->Bool{%s};match true{true if g()==g()=>{},_=>{}}}" % body)
    for tag, src in [
        ("edit", "fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true if edit g()=>{},_=>{}}}"),
        ("move", "fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true if move g()=>{},_=>{}}}"),
        ("capedit", "fn Main(){var x=1;let g=fn [edit x]()->Bool{x=2;return true;};match true{true if g()=>{},_=>{}}}"),
        ("capedit2", "fn Main(){var x=1;let g=fn [edit x]()->Bool{x=2;Print(x);return true;};match true{true if edit g()=>{},_=>{}}}"),
        ("capmove", "fn Main(){let a=[1];let g=fn [move a]()->Bool{return true;};match true{true if g()=>{},_=>{}}}"),
        ("drop", "fn Main(){let a=Some(1);match true{true if (if true then {Drop(a);value:true;} else {value:false;})=>{},_=>{}}}"),
        ("scrutinee", "fn Main(){let g=fn ()->Bool{Print(1);return true;};match g(){true if true=>{},_=>{}}}"),
        ("arm_body", "fn Main(){let g=fn ()->Bool{Print(1);return true;};match true{true=>{g();},_=>{}}}"),
    ]:
        emit("ownership_or_other", tag, src)
    return out


def corpus_text():
    return json.dumps(gen_corpus(), indent=0)


# ---------------------------------------------------------------------------------------------
# Frozen fuzz generator, plan Appendix C (random.Random(20261123)).
FZ_BODIES = ["return true;", "return 1==1;", "Print(1);return true;", "MockUnknown();return true;",
             "let r=MockOpen(1);return true;", "let a=[1];return true;", "let q=MockRequest();return true;",
             "let r=MockBlockingOpen(1);return true;"]
FZ_CTX = ["match true{true if %s=>{},_=>{}}", "match Some(1){Some(v) if %s=>{},_=>{}}",
          "let x=match true{true if %s=>{value:1;},_=>{value:2;}};", "if true{match true{true if %s=>{},_=>{}}}",
          "match true{true=>{match true{true if %s=>{},_=>{}}},_=>{}}"]


def _group(rng, text):
    for _ in range(rng.randint(0, 3)):
        text = "(" + text + ")"
    return text


def _call(rng, names):
    n = rng.choice(names)
    if n.startswith("fn "):
        return "(%s)()" % n[3:]
    return _group(rng, n) + "()" if rng.random() < 0.3 else n + "()"


def _guard(rng, names):
    parts = [_call(rng, names) for _ in range(rng.randint(1, 3))]
    g = "==".join(parts)
    if len(parts) > 1 or rng.random() < 0.4:
        g = "(if true then %s else false)" % parts[0] if len(parts) == 1 else g
    return g


def _make(rng):
    defs, names = "", []
    for i in range(rng.randint(0, 3)):
        body = rng.choice(FZ_BODIES)
        if rng.random() < 0.35 and names:
            body = "return %s();" % rng.choice(names)
        if rng.random() < 0.8:
            defs += "let g%d=fn ()->Bool{%s};" % (i, body)
            names.append("g%d" % i)
    lit = "fn ()->Bool{%s}" % rng.choice(FZ_BODIES)
    names.append("fn (%s)" % lit)
    ctx = rng.choice(FZ_CTX) % _guard(rng, names)
    if rng.random() < 0.3:
        ctx = "let f=fn (){%s};" % ctx
    pre = ""
    if rng.random() < 0.3:
        pre = "fn G()->Bool{%s}" % rng.choice(FZ_BODIES)
        ctx = ctx.replace("g0()", "G()") if rng.random() < 0.5 else ctx
    return "%sfn Main(){%s%s}" % (pre, defs, ctx)


def gen_fuzz(n):
    import random
    rng = random.Random(20261123)
    return [_make(rng) for _ in range(n)]


def fuzz_text(n):
    return json.dumps(gen_fuzz(n), indent=0)


def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()


# ---------------------------------------------------------------------------------------------
# Execution
def run_one(binary, verb, profile, path):
    start = time.time()
    proc = subprocess.run([str(binary), verb, str(path), "--profile", profile, "--format", "json"],
                          capture_output=True, timeout=300, check=False)
    return {"exit": proc.returncode, "stdout": proc.stdout.decode("utf-8", "replace"),
            "stderr": proc.stderr.decode("utf-8", "replace"), "secs": time.time() - start}


def run_sources(binary, sources, workdir, jobs, profiles=PROFILES, verbs=VERBS):
    """Returns {(index, profile, verb): result}."""
    paths = []
    for i, src in enumerate(sources):
        path = Path(workdir) / ("s%05d.n" % i)
        path.write_bytes(src.encode("utf-8"))
        paths.append(path)
    keys = [(i, p, v) for i in range(len(sources)) for p in profiles for v in verbs]
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        results = list(pool.map(lambda k: run_one(binary, k[2], k[1], paths[k[0]]), keys))
    return dict(zip(keys, results))


def parse(result):
    try:
        return json.loads(result["stdout"])
    except ValueError:
        return None


def accepted(result):
    return result["exit"] == 0


def anomaly(result):
    """Exit codes other than 0/1, non-empty stderr, internal errors or a hang (plan 7.3 (c))."""
    text = result["stdout"]
    return (result["exit"] not in (0, 1) or result["stderr"] != "" or "internal_error" in text
            or "InvalidIr" in text or result["secs"] > TIME_LIMIT)


def diagnostics(result):
    doc = parse(result)
    return [] if doc is None else doc.get("diagnostics", [])


def parity_view(result, source, closure):
    """(code, subcause, data, chain operations with closure.call as function.call, primary text)."""
    out = []
    for d in diagnostics(result):
        span = d["primary_span"]
        text = source.encode("utf-8")[span["start"]:span["end"]].decode("utf-8", "replace").lower()
        ops = tuple("function.call" if c["operation"] == "closure.call" and closure else c["operation"]
                    for c in d.get("cause_chain", []))
        out.append((d["code"], d["subcause"], json.dumps(d.get("data"), sort_keys=True), ops, text))
    return out


def has_guard_effect(result):
    return any(d["code"] == "NX-E1305" and d["subcause"] == "guard_effect" for d in diagnostics(result))


# ---------------------------------------------------------------------------------------------
# Criteria
def compare_corpus(corpus, base, cand):
    """Plan 7.3 criteria (a), (b), (c) for the frozen corpus. Returns (failures, changed_per_class)."""
    failures, changed = [], {}
    twins = {}
    for i, item in enumerate(corpus):
        twins[(item["kind"], item["tag"])] = i
    for i, item in enumerate(corpus):
        for profile in PROFILES:
            for verb in VERBS:
                key = (i, profile, verb)
                b, c = base[key], cand[key]
                for label, r in (("base", b), ("cand", c)):
                    if anomaly(r):
                        failures.append(("c", item["kind"], item["tag"], profile, verb, label + " anomaly"))
                same = (b["exit"], b["stdout"]) == (c["exit"], c["stdout"])
                if item["kind"] in CHANGING:
                    if same and verb == "check":
                        failures.append(("b", item["kind"], item["tag"], profile, verb, "not changed"))
                    if not same:
                        changed[item["kind"]] = changed.get(item["kind"], 0) + (1 if verb == "check" else 0)
                    if not has_guard_effect(c):
                        failures.append(("b", item["kind"], item["tag"], profile, verb, "no guard_effect in candidate"))
                    if c["exit"] != 1:
                        failures.append(("b", item["kind"], item["tag"], profile, verb, "candidate not rejected"))
                else:
                    if not same:
                        failures.append(("a", item["kind"], item["tag"], profile, verb, "changed"))
            # Parity with the fn twin (180 closure_guard_eff pairs).
            if item["kind"] == "closure_guard_eff":
                j = twins[("fn_guard_eff", item["tag"])]
                left = parity_view(cand[(i, profile, "check")], item["src"], True)
                right = parity_view(cand[(j, profile, "check")], corpus[j]["src"], False)
                # Entities and spans of chain steps differ by construction; operations and text do not.
                if left != right:
                    failures.append(("b", item["kind"], item["tag"], profile, "check", "parity with fn twin"))
    return failures, changed


def compare_unchanged(sources, base, cand):
    failures = []
    for (i, profile, verb), b in base.items():
        c = cand[(i, profile, verb)]
        for label, r in (("base", b), ("cand", c)):
            if anomaly(r):
                failures.append((i, profile, verb, label + " anomaly"))
        if (b["exit"], b["stdout"]) != (c["exit"], c["stdout"]):
            failures.append((i, profile, verb, "changed"))
    return failures


def compare_fuzz(base, cand):
    failures, changed, total = [], 0, 0
    for (i, profile, verb), b in base.items():
        c = cand[(i, profile, verb)]
        total += 1
        for label, r in (("base", b), ("cand", c)):
            if anomaly(r):
                failures.append((i, profile, verb, label + " anomaly"))
        if (b["exit"], b["stdout"]) != (c["exit"], c["stdout"]):
            changed += 1
        if b["exit"] == 1 and c["exit"] == 0:
            failures.append((i, profile, verb, "rejected by base, accepted by candidate"))
        if verb == "run":
            for label, table in (("base", base), ("cand", cand)):
                if accepted(table[(i, profile, "check")]) != accepted(table[(i, profile, "run")]):
                    failures.append((i, profile, verb, label + " check/run acceptance differs"))
    return failures, changed, total


def compare_releases(debug, release, label):
    failures = []
    for key, d in debug.items():
        r = release[key]
        if (d["exit"], d["stdout"]) != (r["exit"], r["stdout"]):
            failures.append((key, label + " debug != release"))
    return failures


# ---------------------------------------------------------------------------------------------
def expand_files(paths):
    sources = []
    for raw in paths:
        p = Path(raw)
        items = sorted(q for q in p.rglob("*") if q.is_file() and q.suffix in (".n", ".nx")) if p.is_dir() else [p]
        for q in items:
            try:
                sources.append(q.read_bytes().decode("utf-8"))
            except UnicodeDecodeError:
                continue
    return sources


def gen2_sources(seed):
    if seed not in GEN2_SEEDS:
        sys.exit("unknown seed %s (frozen seeds: %s)" % (seed, ", ".join(GEN2_SEEDS)))
    blob = subprocess.run(["git", "-C", str(ROOT), "show", "%s:%s" % (GEN2_REF, GEN2_PATH)],
                          capture_output=True, check=True).stdout
    if hashlib.sha256(blob).hexdigest() != GEN2_SHA256:
        sys.exit("gen2.py SHA-256 differs from the plan")
    with tempfile.TemporaryDirectory() as tmp:
        script, out = Path(tmp) / "gen2.py", Path(tmp) / "out.json"
        script.write_bytes(blob)
        subprocess.run([sys.executable, "-B", str(script), seed, "1500", str(out)], check=True,
                       cwd=tmp, capture_output=True)
        data = out.read_bytes()
    if hashlib.sha256(data).hexdigest() != GEN2_SEEDS[seed]:
        sys.exit("gen2 output for seed %s differs from the plan hash" % seed)
    items = json.loads(data)
    return [x if isinstance(x, str) else x.get("src", x.get("source")) for x in items]


def self_test(args):
    text = corpus_text()
    assert len(gen_corpus()) == 840 and sha(text) == CORPUS_SHA, (len(gen_corpus()), sha(text))
    fz = fuzz_text(3000)
    assert sha(fz) == FUZZ_SHA, sha(fz)
    # Comparison logic on synthetic results: identical -> no failures; one altered output of an
    # unchanged class -> criterion (a); an unchanged guard_effect source -> criterion (b).
    corpus = gen_corpus()

    def fake(exit_code, diags):
        return {"exit": exit_code, "stdout": json.dumps({"diagnostics": diags}), "stderr": "", "secs": 0.0}

    def diag(src, name):
        at = max(src.find(name, max(src.find("match"), 0)), 0)
        return {"code": "NX-E1305", "subcause": "guard_effect", "primary_span": {"start": at, "end": at + 1},
                "data": {}, "cause_chain": [{"operation": "function.call"}]}

    base = {}
    for i, item in enumerate(corpus):
        for p in PROFILES:
            for v in VERBS:
                if item["kind"] == "fn_guard_eff":
                    base[(i, p, v)] = fake(1, [diag(item["src"], "G")])
                else:
                    base[(i, p, v)] = fake(0, [])
    cand = {}
    for i, item in enumerate(corpus):
        for p in PROFILES:
            for v in VERBS:
                if item["kind"] == "closure_guard_eff":
                    d = diag(item["src"], "g")
                    d["cause_chain"] = [{"operation": "closure.call"}]
                    cand[(i, p, v)] = fake(1, [d])
                elif item["kind"] in CHANGING:
                    d = diag(item["src"], "Main")
                    cand[(i, p, v)] = fake(1, [d])
                else:
                    cand[(i, p, v)] = base[(i, p, v)]
    failures, _ = compare_corpus(corpus, base, cand)
    assert not [f for f in failures if f[0] == "a"], failures[:3]
    victim = next(i for i, t in enumerate(corpus) if t["kind"] == "closure_outside_guard")
    cand[(victim, PROFILES[0], "check")] = fake(1, [diag(corpus[victim]["src"], "Main")])
    failures, _ = compare_corpus(corpus, base, cand)
    assert any(f[0] == "a" and f[2] == corpus[victim]["tag"] for f in failures), "criterion (a) did not fire"
    assert compare_unchanged([], base, base) == []
    altered = dict(base)
    altered[(0, PROFILES[0], "check")] = fake(1, [])
    assert compare_unchanged([], base, altered), "arbitrary change not detected"
    pane = dict(base)
    pane[(0, PROFILES[0], "run")] = {"exit": 101, "stdout": "", "stderr": "panic", "secs": 0.0}
    assert any("anomaly" in f[3] for f in compare_unchanged([], pane, pane)), "panic not detected"
    if args.base_debug:
        sample = [c["src"] for c in corpus[::35]]
        with tempfile.TemporaryDirectory() as tmp:
            one = run_sources(args.base_debug, sample, tmp, args.jobs)
            two = run_sources(args.base_debug, sample, tmp, args.jobs)
        assert compare_unchanged(sample, one, two) == [], "base == base shows differences"
    print("guarda-closure diff self-test: PASS (corpus 840 sha ok, fuzz 3000 sha ok, criteria a/b/c fire%s)"
          % ("; base==base on %d sources" % len(sample) if args.base_debug else ""))


def report(name, failures):
    for f in failures[:20]:
        print("FAIL", name, f)
    print("%s: %s (%d failures)" % (name, "PASS" if not failures else "FAIL", len(failures)))
    return not failures


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base-debug")
    parser.add_argument("--cand-debug")
    parser.add_argument("--base-release")
    parser.add_argument("--cand-release")
    parser.add_argument("--jobs", type=int, default=int(os.environ.get("CARGO_BUILD_JOBS", "4")))
    parser.add_argument("--corpus", action="store_true")
    parser.add_argument("--gen2")
    parser.add_argument("--fuzz", type=int)
    parser.add_argument("--files", nargs="+")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test(args)
    if not (args.base_debug and args.cand_debug):
        parser.error("--base-debug and --cand-debug are required")
    if args.corpus:
        corpus = gen_corpus()
        if sha(corpus_text()) != CORPUS_SHA:
            sys.exit("corpus SHA-256 differs from the plan: not reproduced, stop")
        sources, name = [c["src"] for c in corpus], "corpus"
    elif args.gen2:
        sources, name = gen2_sources(args.gen2), "gen2-" + args.gen2
    elif args.fuzz:
        if args.fuzz == 3000 and sha(fuzz_text(3000)) != FUZZ_SHA:
            sys.exit("fuzz SHA-256 differs from the plan: not reproduced, stop")
        sources, name = gen_fuzz(args.fuzz), "fuzz"
    elif args.files:
        sources, name = expand_files(args.files), "files"
    else:
        parser.error("choose a mode")
    print("%s: %d sources x %d profiles x %d verbs" % (name, len(sources), len(PROFILES), len(VERBS)))
    pairs = [("debug", args.base_debug, args.cand_debug)]
    if args.base_release and args.cand_release:
        pairs.append(("release", args.base_release, args.cand_release))
    ok = True
    results = {}
    with tempfile.TemporaryDirectory(dir=os.environ.get("TMP")) as tmp:
        for label, base_bin, cand_bin in pairs:
            base = run_sources(base_bin, sources, tmp, args.jobs)
            cand = run_sources(cand_bin, sources, tmp, args.jobs)
            results[label] = (base, cand)
            if args.corpus:
                failures, changed = compare_corpus(corpus, base, cand)
                print("%s changed per class (source x profile, check):" % label, json.dumps(changed, sort_keys=True))
                ok &= report("%s corpus" % label, failures)
            elif args.fuzz:
                failures, changed, total = compare_fuzz(base, cand)
                print("%s: %d of %d outputs changed" % (label, changed, total))
                ok &= report("%s fuzz" % label, failures)
            else:
                ok &= report("%s %s" % (label, name), compare_unchanged(sources, base, cand))
    if "release" in results:
        ok &= report("debug==release base", compare_releases(results["debug"][0], results["release"][0], "base"))
        ok &= report("debug==release candidate", compare_releases(results["debug"][1], results["release"][1], "candidate"))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
