"""Command line of the F4 oracle (class ORQUESTA).

``python -B -m scripts.f4_oracle.cli`` with: ``selftest [--fast]``, ``audit``, ``generate``,
``fuzz-corpus``, ``compare``, ``gate``, ``fuzz``, ``minimize``, ``batch`` and ``corpus-sha``.

The model lives on this side: ``generate``/``fuzz-corpus`` write the JSONL records (expected
result and ``expected_sha256`` first); ``compare``/``gate``/``fuzz`` hand the JSONL to the harness,
which never recomputes anything.  Exit codes: 0 all good, 1 divergence/failure/known-issue limit,
2 harness or configuration error (impossible manifest, bad corpus, stale known issue, ...).
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import io
import json
import platform
import random
import sys

from . import app, audit, barrier, batch, fixtures_io, generate, generic, minimize, runner, selftest

KNOWN_FIXTURE = "f4_oracle_known_issues.json"


# ------------------------------------------------------------------ corpus

def _complete(text: str) -> list:
    return [k for k in text.split(",") if k]


def _check_manifest(args):
    closed = barrier.validate_manifest(args.phase, _complete(args.complete))
    if getattr(args, "final", False) and args.phase != "G10":
        raise barrier.ManifestError("--final is valid only with --phase G10")
    if args.profile != generate.PROFILE:
        raise barrier.ManifestError(f"profile {args.profile} is not supported by the generators")
    return closed


def _write_lines(path, records) -> int:
    n = 0
    with open(path, "wb") as handle:
        for rec in records:
            handle.write((fixtures_io.dumps(rec) + "\n").encode("utf-8"))
            n += 1
    return n


def corpus_records(args, closed) -> list:
    out = []
    if not getattr(args, "no_literals", False):
        out += selftest.literal_records(args.phase, closed)
    for seed in generic.seed_range(args.seeds):
        recs, _, _ = generate.generate_seed(seed, args.phase, closed, args.valid_per_seed,
                                            args.invalid_per_seed, args.profile)
        out += recs
        out += app.special_records(seed, args.phase, closed, args.valid_per_seed, args.invalid_per_seed,
                               getattr(args, "final", False))
    return out


def fuzz_records(seed: int, phase: str, closed, count: int, profile: str, base_valid: int,
                 base_invalid: int) -> list:
    """Fuzz sources: truncated prefixes, byte edits and slice edits of generated sources and of the
    literal rows of K.  They carry no expected semantics (``expected`` is null)."""
    rng = random.Random(f"f4o-fuzz/{seed}")
    base = generate.generate_seed(seed, phase, closed, base_valid, base_invalid, profile)[0]
    sources = [bytes.fromhex(r["source_hex"]) for r in base if r["role"] == "main"]
    rows = [e["source"].encode("utf-8") for e in selftest.load_literals() if e["source"]]
    null_sha = hashlib.sha256(b"null").hexdigest()
    out = []
    kinds = ["prefix", "flip", "insert", "delete", "dup", "row_prefix"]
    with_env = phase in ("G9", "G10") and "K10" in closed
    if with_env:
        sources.append(app.app_source().encode("utf-8"))
        kinds += ["env_flip", "env_prefix"]
    while len(out) < count:
        kind = rng.choice(kinds)
        if kind.startswith("env_"):
            data = bytearray(app.env_bytes(app.random_env(rng)[0], rng))
            if kind == "env_prefix":
                data = data[:rng.randrange(len(data) + 1)]
            else:
                for _ in range(rng.randint(1, 3)):
                    data[rng.randrange(len(data))] = rng.randrange(256)
            out.append({"seed": seed, "index": len(out), "role": "fuzz", "cls": "F", "fclass": "F",
                        "family": "fuzz_" + kind, "phase": phase, "file": "case.n", "profile_id": profile,
                        "source_hex": app.app_source().encode("utf-8").hex(), "env_hex": bytes(data).hex(),
                        "verbs": ["run"], "expected": None, "expected_sha256": null_sha, "repr": None})
            continue
        pool = rows if kind == "row_prefix" else sources
        src = rng.choice(pool) if pool else b"fn Main(){}"
        data = bytearray(src)
        if kind in ("prefix", "row_prefix"):
            data = data[:rng.randrange(len(data) + 1)]
        elif kind == "flip" and data:
            for _ in range(rng.randint(1, 3)):
                data[rng.randrange(len(data))] = rng.randrange(256)
        elif kind == "insert":
            data.insert(rng.randrange(len(data) + 1), rng.randrange(256))
        elif kind == "delete" and data:
            a = rng.randrange(len(data))
            del data[a:a + rng.randint(1, 8)]
        elif kind == "dup" and data:
            a = rng.randrange(len(data))
            b = a + rng.randint(1, 12)
            data[a:a] = data[a:b]
        out.append({"seed": seed, "index": len(out), "role": "fuzz", "cls": "F", "fclass": "F",
                    "family": "fuzz_" + kind, "phase": phase, "file": "case.n", "profile_id": profile,
                    "source_hex": bytes(data).hex(), "verbs": ["check", "run"], "expected": None,
                    "expected_sha256": null_sha, "repr": None})
    return out


def fuzz_corpus_records(args, closed) -> list:
    out = []
    for seed in generic.seed_range(args.seeds):
        out += fuzz_records(seed, args.phase, closed, args.fuzz_per_seed, args.profile,
                            min(args.valid_per_seed, 40), min(args.invalid_per_seed, 40))
    return out


# ------------------------------------------------------------------ commands

def cmd_generate(args) -> int:
    closed = _check_manifest(args)
    recs = corpus_records(args, closed)
    n = _write_lines(args.output, recs)
    digest = hashlib.sha256(b"".join((fixtures_io.dumps(r) + "\n").encode("utf-8") for r in recs)).hexdigest()
    print(f"generate: {n} records sha256={digest}")
    return 0


def cmd_fuzz_corpus(args) -> int:
    closed = _check_manifest(args)
    n = _write_lines(args.output, fuzz_corpus_records(args, closed))
    print(f"fuzz-corpus: {n} records")
    return 0


def _summary_line(path) -> str:
    lines = [ln for ln in open(path, "rb").read().decode("utf-8").split("\n") if ln.strip()]
    return lines[-1] if lines else ""


def _run_compare(args, gate_info=None, known_path=None) -> int:
    closed = _check_manifest(args)
    if getattr(args, "final", False):
        print("matrix " + fixtures_io.dumps(selftest.matrix_report()))
    corpus = args.output + ".corpus.jsonl"
    _write_lines(corpus, corpus_records(args, closed))
    code = 2
    try:
        code = runner.run_compare(
            corpus=corpus, binary_debug=args.binary_debug, binary_release=args.binary_release,
            profile=args.profile, phase=args.phase, complete=sorted(closed), output=args.output,
            resume=args.resume, known_path=known_path, workers=args.workers, timeout=args.timeout,
            gate_info=gate_info)
    finally:
        print(_summary_line(args.output))
    return code


def cmd_compare(args) -> int:
    return _run_compare(args)


def cmd_gate(args, *, audit_fn=None, selftest_fn=None) -> int:
    """audit and selftest first (exit 0 and the SHA-256 of their log recorded), then compare."""
    _check_manifest(args)
    audit_fn = audit_fn or audit.run
    selftest_fn = selftest_fn or (lambda: selftest.run(fast=False))
    log = io.StringIO()
    with contextlib.redirect_stdout(log):
        findings = audit_fn()
        for item in findings:
            print(f"AUDIT {item}")
        audit_exit = 1 if findings else 0
        selftest_exit = selftest_fn()
    text = log.getvalue()
    info = {"type": "gate", "audit_exit": audit_exit, "selftest_exit": selftest_exit,
            "selftest_log_sha256": hashlib.sha256(text.encode("utf-8")).hexdigest()}
    if audit_exit or selftest_exit:
        print(f"gate: audit={audit_exit} selftest={selftest_exit}; no comparison is run")
        print(text[-2000:])
        return 1
    return _run_compare(args, gate_info=info, known_path=fixtures_io.fixture_path(KNOWN_FIXTURE))


def cmd_fuzz(args) -> int:
    closed = _check_manifest(args)
    corpus = args.output + ".corpus.jsonl"
    _write_lines(corpus, fuzz_corpus_records(args, closed))
    code = 2
    try:
        code = runner.run_fuzz(corpus=corpus, binary_debug=args.binary_debug,
                               binary_release=args.binary_release, profile=args.profile,
                               output=args.output, resume=args.resume, workers=args.workers,
                               timeout=args.timeout)
    finally:
        print(_summary_line(args.output))
    return code


def cmd_minimize(args) -> int:
    lines = [json.loads(ln) for ln in open(args.failure, "rb").read().decode("utf-8").split("\n") if ln.strip()]
    findings = [x for x in lines if x.get("type") == "finding"]
    if not findings:
        raise runner.HarnessError("no finding in the failure file")
    result = minimize.minimize(findings[args.index], binary_debug=args.binary_debug,
                               binary_release=args.binary_release, profile=args.profile,
                               budget=args.budget, timeout=args.timeout)
    with open(args.output, "wb") as handle:
        handle.write((fixtures_io.dumps(result) + "\n").encode("utf-8"))
    print(f"minimize: {result['status']} steps={result['steps']}")
    return 0


def cmd_batch(args, *, runner_fn=None) -> int:
    _check_manifest(args)
    batch.validate_scale(args.gap_seconds, args.output_dir)
    plan = batch.plan(command=args.command, binary_debug=args.binary_debug,
                      binary_release=args.binary_release, profile=args.profile, phase=args.phase,
                      complete=args.complete, seeds=args.seeds, valid=args.valid_per_seed,
                      invalid=args.invalid_per_seed, output_dir=args.output_dir, width=args.batch_seeds,
                      session=args.session, final=args.final, timeout=args.timeout)
    kw = {"runner": runner_fn} if runner_fn else {}
    code, report = batch.run(plan, gap_seconds=args.gap_seconds,
                             continue_on_divergence=args.continue_on_divergence, dry_run=args.dry_run, **kw)
    for item in report:
        print(fixtures_io.dumps(item))
    return code


def _corpus_sha(args) -> int:
    """Print or pin the SHA-256 of the generated corpus per label, seed and class."""
    if args.write:
        table = selftest.corpus_table()
        doc = {"version": 1, "python": platform.python_version(),
               "scale": {k: list(v) for k, v in selftest.SCALE.items()}, "g10": [1000, 1000, 50],
               "note": "SHA-256 of the canonical JSON records per label|seed|class (V, N, L, B; from G9 also A, E, Q; G10F = final recipes); "
                       "regenerate only with `corpus-sha --write` after reviewing the generator change.",
               "entries": table}
        path = fixtures_io.fixture_path("f4_oracle_corpus_sha.json")
        with open(path, "wb") as handle:
            handle.write((fixtures_io.dumps(doc) + chr(10)).encode("utf-8"))
        print(f"corpus-sha: wrote {len(table)} entries")
        return 0
    for label, phase, complete, nv, ni, seeds, _ in selftest.gen_labels():
        if args.label and label != args.label:
            continue
        for seed in seeds:
            if args.seeds:
                lo, _, hi = args.seeds.partition(":")
                if not (int(lo) <= seed <= int(hi or lo)):
                    continue
            recs, _, _ = generate.generate_seed(seed, phase, complete, nv, ni)
            for c in "VNLB":
                print(f"{label}|{seed}|{c}={generate.corpus_sha(recs, c)}")
    return 0


# ------------------------------------------------------------------ parser

def _common(p, *, output=True, seeds=True):
    p.add_argument("--binary-debug", required=True)
    p.add_argument("--binary-release", required=True)
    p.add_argument("--profile", required=True)
    p.add_argument("--phase", required=True)
    p.add_argument("--complete", required=True)
    if seeds:
        p.add_argument("--seeds", required=True)
        p.add_argument("--valid-per-seed", type=int, required=True)
        p.add_argument("--invalid-per-seed", type=int, required=True)
    if output:
        p.add_argument("--output", required=True)
    p.add_argument("--final", action="store_true")
    p.add_argument("--timeout", type=float, default=10.0)


def build_parser():
    parser = argparse.ArgumentParser(prog="f4_oracle")
    sub = parser.add_subparsers(dest="command", required=True)
    st = sub.add_parser("selftest")
    st.add_argument("--fast", action="store_true")
    st.add_argument("--groups", default="")
    st.add_argument("--fail-fast", action="store_true")
    sub.add_parser("audit")
    cs = sub.add_parser("corpus-sha")
    cs.add_argument("--label", default="")
    cs.add_argument("--seeds", default="")
    cs.add_argument("--write", action="store_true")
    for name in ("generate", "fuzz-corpus"):
        p = sub.add_parser(name)
        p.add_argument("--profile", required=True)
        p.add_argument("--phase", required=True)
        p.add_argument("--complete", required=True)
        p.add_argument("--seeds", required=True)
        p.add_argument("--valid-per-seed", type=int, required=True)
        p.add_argument("--invalid-per-seed", type=int, required=True)
        p.add_argument("--output", required=True)
        p.add_argument("--final", action="store_true")
        p.add_argument("--no-literals", action="store_true")
        p.add_argument("--fuzz-per-seed", type=int, default=300)
    for name in ("compare", "gate", "fuzz"):
        p = sub.add_parser(name)
        _common(p)
        p.add_argument("--resume", action="store_true")
        p.add_argument("--workers", type=int, default=2)
        p.add_argument("--no-literals", action="store_true")
        p.add_argument("--fuzz-per-seed", type=int, default=300)
    mz = sub.add_parser("minimize")
    mz.add_argument("--binary-debug", required=True)
    mz.add_argument("--binary-release", required=True)
    mz.add_argument("--profile", required=True)
    mz.add_argument("--failure", required=True)
    mz.add_argument("--output", required=True)
    mz.add_argument("--index", type=int, default=0)
    mz.add_argument("--budget", type=float, default=minimize.BUDGET)
    mz.add_argument("--timeout", type=float, default=10.0)
    bt = sub.add_parser("batch")
    _common(bt, output=False)
    bt.add_argument("--output-dir", required=True)
    bt.add_argument("--command", choices=("gate", "compare"), default="gate")
    bt.add_argument("--batch-seeds", type=int, default=1)
    bt.add_argument("--gap-seconds", type=float, default=75.0)
    bt.add_argument("--session", default="f4-oracle")
    bt.add_argument("--continue-on-divergence", action="store_true")
    bt.add_argument("--dry-run", action="store_true")
    return parser


def main(argv=None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "audit":
            findings = audit.run()
            for item in findings:
                print(f"AUDIT {item}")
            print(f"audit: {len(findings)} findings")
            return 1 if findings else 0
        if args.command == "selftest":
            groups = [g for g in args.groups.split(",") if g] or None
            return selftest.run(fast=args.fast, groups=groups, fail_fast=args.fail_fast)
        if args.command == "corpus-sha":
            return _corpus_sha(args)
        handlers = {"generate": cmd_generate, "fuzz-corpus": cmd_fuzz_corpus, "compare": cmd_compare,
                    "gate": cmd_gate, "fuzz": cmd_fuzz, "minimize": cmd_minimize, "batch": cmd_batch}
        return handlers[args.command](args)
    except (barrier.ManifestError, runner.HarnessError, ValueError, OSError) as exc:
        print(f"harness error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
