"""Minimizer of failing cases (class ORQUESTA; plan 4).

Input: a finding written by ``compare`` (it carries the oracle's own representation ``repr`` and
the first divergent field ``path``).  ``generate.reductions`` proposes smaller candidates; the
model recomputes each candidate's expected result; the candidate is invoked with
``runner.Session``.  A reduction is accepted only if the same finding survives: same category,
verb, binary, first divergent path and diagnostic code/subcause.  60 seconds per failure; when
the search does not reach a fixpoint within the budget the ORIGINAL is kept.  A confirmed
reproduction moves to the regressions with a K citation (done by hand, never by this tool).
"""

from __future__ import annotations

import time

from . import fixtures_io as fio, generate
from .runner import HarnessError, Session, TMP

BUDGET = 60.0


def _record(res: dict, rep: dict, failure: dict, profile: str):
    exp = res["expected"]
    verb = failure["verb"]
    verbs = [verb] if verb in exp else []
    if not verbs:
        return None
    return {"seed": failure["seed"], "index": failure["index"], "role": "minimize", "cls": rep["cls"],
            "family": rep["family"], "file": failure["file"], "profile_id": profile,
            "source_hex": res["source"].encode("utf-8").hex(), "verbs": verbs, "expected": exp,
            "expected_sha256": fio.sha256_text(fio.dumps(exp)), "repr": rep}


def _same(finding: dict, failure: dict) -> bool:
    return (finding["category"] == failure["category"] and finding["verb"] == failure["verb"]
            and finding["binary"] == failure["binary"] and finding["path"] == failure["path"]
            and finding.get("code") == failure.get("code")
            and finding.get("subcause") == failure.get("subcause"))


def minimize(failure: dict, *, binary_debug, binary_release, profile: str, budget: float = BUDGET,
             timeout: float = 10.0, tmp_root=TMP) -> dict:
    rep = failure.get("repr")
    if not rep:
        return {"type": "minimized", "status": "no_repr", "source_hex": failure["source_hex"], "steps": 0}
    started = time.monotonic()

    def out(status, rep_, source_hex, steps):
        return {"type": "minimized", "status": status, "repr": rep_, "source_hex": source_hex,
                "original_source_hex": failure["source_hex"], "steps": steps, "path": failure["path"],
                "code": failure.get("code"), "elapsed": round(time.monotonic() - started, 3)}

    with Session(binary_debug, binary_release, timeout=timeout, tmp_root=tmp_root) as session:
        def reproduces(candidate):
            res = generate.expect(candidate, profile)
            if res["expected"] is None:
                return None
            rec = _record(res, candidate, failure, profile)
            if rec is None:
                return None
            ok = any(_same(f, failure) for f in session.check(rec))
            return (res["source"].encode("utf-8").hex()) if ok else None

        base = reproduces(rep)
        if base is None:
            return out("not_reproducible", rep, failure["source_hex"], 0)
        current, current_hex, steps = rep, base, 0
        while True:
            advanced = False
            for cand in generate.reductions(current):
                if time.monotonic() - started > budget:
                    return out("not_converged", rep, failure["source_hex"], 0)
                got = reproduces(cand)
                if got is not None and len(got) < len(current_hex):
                    current, current_hex, steps = cand, got, steps + 1
                    advanced = True
                    break
            if not advanced:
                break
    return out("minimized" if steps else "unchanged", current, current_hex, steps)
