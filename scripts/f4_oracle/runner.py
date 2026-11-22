"""Comparison harness of the F4 oracle (class ARNES).

It only READS the JSONL corpus written by ``cli generate`` / ``cli fuzz-corpus``: every
expected value is in the record (with ``expected_sha256``) before the first invocation and is
never recomputed here.  It imports no model module; the model lives on the other side of the
JSONL boundary.  A divergence, a harness failure, ``internal_error``, a panic, ``InvalidIr`` or a
confirmed timeout is never a PASS (exit 1); a harness/configuration error is exit 2.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import threading
from concurrent.futures import ThreadPoolExecutor

from . import fixtures_io as fio
from .generic import Invocation, _first_difference, invoke, sha

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
TMP = Path("C:/nexo-work/tmp")
PHASE_ORDER = {f"G{i}": i for i in range(11)}
TOP_KEYS = ("schema_version", "profile_id", "outcome", "stdout", "events", "observations",
            "observation_bytes", "reserved_observation_bytes")
KNOWN_KEYS = ("family", "code", "path", "max_phase", "cite", "finding", "reviewed_by")
NULL_SHA = hashlib.sha256(b"null").hexdigest()


class HarnessError(Exception):
    """Configuration or corpus error: the run is invalid (exit 2), not a product verdict."""


# ------------------------------------------------------------------ corpus (JSONL only)

def read_jsonl(path) -> list:
    data = Path(path).read_bytes().decode("utf-8")
    out = []
    for n, line in enumerate(data.split("\n"), 1):
        if not line.strip():
            continue
        try:
            out.append(json.loads(line))
        except ValueError as exc:
            raise HarnessError(f"{path}:{n}: invalid JSON ({exc})")
    return out


def validate_record(rec: dict) -> None:
    for key in ("source_hex", "file", "verbs", "profile_id", "seed", "index", "family"):
        if key not in rec:
            raise HarnessError(f"record without {key}")
    if "expected_sha256" not in rec or not isinstance(rec["expected_sha256"], str):
        raise HarnessError(f"record {rec.get('seed')}:{rec.get('index')} without expected_sha256")
    expected = rec.get("expected")
    want = NULL_SHA if expected is None else fio.sha256_text(fio.dumps(expected))
    if rec["expected_sha256"] != want:
        raise HarnessError(f"record {rec.get('seed')}:{rec.get('index')} expected_sha256 does not match")
    for verb in rec["verbs"]:
        if verb not in ("check", "run"):
            raise HarnessError(f"unknown verb {verb}")
        if expected is not None and verb not in expected:
            raise HarnessError("record lacks the expected document of its verb")


def group_by_seed(records: list) -> dict:
    out: dict = {}
    for rec in records:
        out.setdefault(rec["seed"], []).append(rec)
    return dict(sorted(out.items()))


# ------------------------------------------------------------------ known issues

def load_known(path) -> list:
    if path is None:
        return []
    doc = json.loads(Path(path).read_bytes().decode("utf-8"))
    entries = doc["entries"] if isinstance(doc, dict) else doc
    for e in entries:
        for key in KNOWN_KEYS:
            if key not in e or e[key] in (None, ""):
                raise HarnessError(f"known_issue without {key}")
        if e["max_phase"] not in PHASE_ORDER:
            raise HarnessError("known_issue with unknown max_phase")
        if e["reviewed_by"] == "C":
            raise HarnessError("known_issue reviewed by its implementer")
    return entries


def known_matches(entry: dict, finding: dict, phase: str) -> bool:
    return (finding.get("category") == "divergence" and entry["family"] == finding["family"]
            and entry["code"] == finding.get("code") and entry["path"] == finding["path"]
            and PHASE_ORDER[phase] <= PHASE_ORDER[entry["max_phase"]])


# ------------------------------------------------------------------ binaries and invocation

class Session:
    """Private copies of the two binaries and the per-case scratch directories."""

    def __init__(self, binary_debug, binary_release, *, timeout=10.0, tmp_root=TMP):
        self.timeout = timeout
        self.tmp_root = Path(tmp_root)
        self.originals = {"debug": Path(binary_debug), "release": Path(binary_release)}
        self.root = self.tmp_root / f"f4-oracle-{os.getpid()}"
        self.bins: dict = {}
        self._n = 0
        self._lock = threading.Lock()

    def __enter__(self):
        if self.root.exists():
            shutil.rmtree(self.root, ignore_errors=True)
        (self.root / "bin").mkdir(parents=True)
        for name, src in self.originals.items():
            if not src.is_file():
                raise HarnessError(f"binary not found: {src}")
            dst = self.root / "bin" / f"{name}-{src.name}"
            shutil.copyfile(src, dst)
            if sha(dst) != sha(src):
                raise HarnessError(f"copy of {src} differs from the original")
            self.bins[name] = dst
        return self

    def __exit__(self, *exc):
        shutil.rmtree(self.root, ignore_errors=True)
        return False

    def sha(self, name: str) -> str:
        return sha(self.originals[name])

    def _case_dir(self) -> Path:
        with self._lock:
            self._n += 1
            n = self._n
        d = self.root / "cases" / f"c{n}"
        d.mkdir(parents=True)
        return d

    def invoke(self, rec: dict, verb: str, which: str) -> Invocation:
        d = self._case_dir()
        try:
            src = d / rec["file"]
            src.write_bytes(bytes.fromhex(rec["source_hex"]))
            env = None
            if rec.get("env_hex") and verb == "run":
                env = d / "env.json"
                env.write_bytes(bytes.fromhex(rec["env_hex"]))
            return invoke(self.bins[which], verb, src, rec["profile_id"], timeout=self.timeout,
                          file_name=rec["file"], mock_env=env)
        finally:
            shutil.rmtree(d, ignore_errors=True)

    # ---- one record
    def check(self, rec: dict) -> list:
        """Findings of one record: divergences, harness failures and debug/release differences."""
        findings = []
        expected = rec.get("expected")
        for verb in rec["verbs"]:
            invs = {}
            for which in ("debug", "release"):
                inv = self.invoke(rec, verb, which)
                invs[which] = inv
                if inv.error is not None:
                    findings.append(_finding(rec, verb, which, inv.error, "$", failure=inv))
                    continue
                if expected is not None:
                    exp_doc = expected[verb]
                    obs = observed(inv, expected["fields"])
                    path = _first_difference(exp_doc, obs)
                    if path is not None:
                        findings.append(_finding(rec, verb, which, "divergence", path,
                                                 exp_doc=exp_doc, obs=obs))
            d, r = invs["debug"], invs["release"]
            if d.error is None and r.error is None:
                path = _first_difference({"exit": d.exit, "doc": d.document},
                                         {"exit": r.exit, "doc": r.document})
                if path is not None:
                    findings.append(_finding(rec, verb, "debug_release", "debug_release", path))
        return findings


def observed(inv: Invocation, fields) -> dict:
    """Comparable view of an invocation: exit plus the contract fields, diagnostics projected."""
    doc = inv.document or {}
    obs = {"exit": inv.exit}
    for key in TOP_KEYS:
        if key in doc:
            obs[key] = doc[key]
    if "diagnostics" in doc:
        diags = doc["diagnostics"]
        if isinstance(diags, list):
            obs["diagnostics"] = [{f: d[f] for f in fields if f in d} if isinstance(d, dict) else d
                                  for d in diags]
        else:
            obs["diagnostics"] = diags
    return obs


def _first_diag(exp_doc, key):
    if not exp_doc:
        return None
    diags = exp_doc.get("diagnostics") or []
    if diags and isinstance(diags[0], dict):
        return diags[0].get(key)
    return None


def _finding(rec, verb, which, category, path, *, failure=None, exp_doc=None, obs=None) -> dict:
    out = {"type": "finding", "category": category, "seed": rec["seed"], "index": rec["index"],
           "role": rec.get("role"), "cls": rec.get("cls"), "family": rec["family"], "verb": verb,
           "binary": which, "path": path, "code": _first_diag(exp_doc, "code"),
           "subcause": _first_diag(exp_doc, "subcause"),
           "expected_sha256": rec["expected_sha256"], "source_hex": rec["source_hex"],
           "file": rec["file"], "profile_id": rec["profile_id"], "repr": rec.get("repr")}
    if failure is not None:
        out["stderr"] = failure.stderr[:400]
        out["exit"] = failure.exit
    if exp_doc is not None:
        out["expected"] = exp_doc
        out["observed"] = obs
    return out


# ------------------------------------------------------------------ judge hash and resume

def judge_sha() -> str:
    """Hash of the oracle sources, its fixtures and the pinned ADRs (the \"judge\")."""
    h = hashlib.sha256()
    for name in sorted(os.listdir(HERE)):
        if name.endswith(".py"):
            h.update(name.encode() + (HERE / name).read_bytes())
    fx = ROOT / "tests" / "fixtures"
    for name in sorted(os.listdir(fx)):
        if name.startswith("f4_oracle_"):
            h.update(name.encode() + (fx / name).read_bytes())
    for key in sorted(fio.ADR_TABLE):
        h.update(key.encode() + fio.adr_sha256(key).encode())
    return h.hexdigest()


def _header(mode, session, profile, phase, complete, extra=None) -> dict:
    head = {"type": "header", "mode": mode, "judge": judge_sha(),
            "adr": {k: fio.adr_sha256(k) for k in sorted(fio.ADR_TABLE)},
            "debug_sha256": session.sha("debug"), "release_sha256": session.sha("release"),
            "profile": profile, "phase": phase, "complete": list(complete or [])}
    if extra:
        head.update(extra)
    return head


def _read_output(path):
    if not Path(path).exists():
        return None
    return read_jsonl(path)


# ------------------------------------------------------------------ drivers

def _drive(*, mode, corpus, binary_debug, binary_release, profile, phase, complete, output,
           resume, known_path, workers, timeout, tmp_root, gate_info, progress=None) -> int:
    records = read_jsonl(corpus)
    for rec in records:
        validate_record(rec)
        if rec["profile_id"] != profile:
            raise HarnessError(f"record profile {rec['profile_id']} differs from --profile {profile}")
    seeds = group_by_seed(records)
    known = load_known(known_path) if mode == "compare" else []
    if known and phase not in PHASE_ORDER:
        raise HarnessError("known issues need a phase")
    totals = {"records": 0, "divergences": 0, "known": 0, "failures": 0, "debug_release": 0,
              "skipped_seeds": 0}
    families_run: set = set()
    matched: set = set()
    with Session(binary_debug, binary_release, timeout=timeout, tmp_root=tmp_root) as session:
        head = _header(mode, session, profile, phase, complete)
        done: set = set()
        prior = _read_output(output) if resume else None
        lines: list = []
        if prior:
            old = [x for x in prior if x.get("type") == "header"]
            if not old or {k: v for k, v in old[0].items()} != head:
                raise HarnessError("resume refused: judge, ADR, binaries, profile or manifest changed")
            for x in prior:
                if x.get("type") == "seed":
                    done.add(x["seed"])
                    totals["records"] += x["records"]
                    totals["divergences"] += x["divergences"]
                    totals["known"] += x.get("known", 0)
                    totals["failures"] += x["failures"]
                    totals["debug_release"] += x.get("debug_release", 0)
                    families_run |= set(x.get("families", []))
                    matched |= {tuple(m) for m in x.get("keys", [])}
            lines = [x for x in prior if x.get("type") in ("gate", "header", "finding", "seed")]
        else:
            lines = ([gate_info] if gate_info else []) + [head]
        out = open(output, "wb")
        try:
            for x in lines:
                out.write((fio.dumps(x) + "\n").encode("utf-8"))
            out.flush()
            for seed, recs in seeds.items():
                if seed in done:
                    totals["skipped_seeds"] += 1
                    continue
                if workers > 1:
                    with ThreadPoolExecutor(max_workers=workers) as pool:
                        results = list(pool.map(session.check, recs))
                else:
                    results = [session.check(r) for r in recs]
                findings = [f for group in results for f in group]
                n_div = n_known = n_fail = n_dr = 0
                seed_matched: set = set()
                for f in findings:
                    if f["category"] == "divergence":
                        seed_matched.add((f["family"], f["code"], f["path"]))
                        hit = next((e for e in known if known_matches(e, f, phase)), None)
                        if hit is not None:
                            f["category"] = "known"
                            f["known_issue"] = hit["finding"]
                            n_known += 1
                        else:
                            n_div += 1
                    elif f["category"] == "debug_release":
                        n_dr += 1
                    else:
                        n_fail += 1
                    out.write((fio.dumps(f) + "\n").encode("utf-8"))
                fams = sorted({r["family"] for r in recs})
                seed_line = {"type": "seed", "seed": seed, "records": len(recs), "divergences": n_div + n_dr,
                             "known": n_known, "failures": n_fail, "debug_release": n_dr, "families": fams,
                             "keys": sorted(seed_matched)}
                out.write((fio.dumps(seed_line) + "\n").encode("utf-8"))
                out.flush()
                totals["records"] += len(recs)
                totals["divergences"] += n_div + n_dr
                totals["known"] += n_known
                totals["failures"] += n_fail
                totals["debug_release"] += n_dr
                families_run |= set(fams)
                matched |= seed_matched
                if progress:
                    progress(seed, seed_line)
            obsolete = [e for e in known if e["family"] in families_run
                        and (e["family"], e["code"], e["path"]) not in matched]
            code = 0
            if totals["divergences"] or totals["failures"]:
                code = 1
            elif obsolete:
                code = 2
            summary = dict(totals, type="summary", exit=code,
                           obsolete=[e["finding"] for e in obsolete])
            out.write((fio.dumps(summary) + "\n").encode("utf-8"))
        finally:
            out.close()
    if code == 2:
        raise HarnessError("obsolete known_issue entries must be removed: "
                           + ", ".join(summary["obsolete"]))
    return code


def run_compare(*, corpus, binary_debug, binary_release, profile, phase, complete, output,
                resume=False, known_path=None, workers=2, timeout=10.0, tmp_root=TMP,
                gate_info=None, progress=None) -> int:
    return _drive(mode="compare", corpus=corpus, binary_debug=binary_debug,
                  binary_release=binary_release, profile=profile, phase=phase, complete=complete,
                  output=output, resume=resume, known_path=known_path, workers=workers,
                  timeout=timeout, tmp_root=tmp_root, gate_info=gate_info, progress=progress)


def run_fuzz(*, corpus, binary_debug, binary_release, profile, output, resume=False, workers=2,
             timeout=10.0, tmp_root=TMP, progress=None) -> int:
    """Fuzz sources carry no expected semantics: any pánico, ``internal_error``, ``InvalidIr``,
    confirmed timeout, or debug/release difference is a finding."""
    return _drive(mode="fuzz", corpus=corpus, binary_debug=binary_debug,
                  binary_release=binary_release, profile=profile, phase="", complete=(),
                  output=output, resume=resume, known_path=None, workers=workers, timeout=timeout,
                  tmp_root=tmp_root, gate_info=None, progress=progress)

