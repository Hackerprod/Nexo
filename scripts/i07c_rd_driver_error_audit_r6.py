#!/usr/bin/env python3
"""Reproduce every pass-counter driver_error against main and R6."""
import hashlib
import json
from pathlib import Path

import i07c_rd_sweep as sweep
from i07c_rd_drive_r6 import BASE, CANDIDATE, CORPORA, FLAGS
from i07c_rd_profile_acceptance_r6 import inspect


DEST = Path("docs/validation/i07c-rediseno")
DEBUG = "C:/nexo-work/target-r6/debug/nexo.exe"


def main():
    records = []
    for corpus in CORPORA:
        path = DEST / f"r6-pasadas-{corpus}.json"
        if not path.exists():
            continue
        document = json.loads(path.read_text(encoding="utf-8"))
        sources = sweep.data(corpus)
        for index, original_error in document.get("driver_errors", []):
            source = sweep.source(sources[index])
            profile = sweep.profile(corpus)
            readings = {name: {command: inspect(exe, command, profile, source, flags)
                               for command in ("check", "run")}
                        for name, exe, flags in (("main", BASE, None), ("r6", CANDIDATE, FLAGS),
                                                 ("r6_debug", DEBUG, FLAGS))}
            records.append({"corpus": corpus, "index": index, "profile": profile,
                            "bytes": len(source.encode()),
                            "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
                            "source": source, "pass_counter_error": original_error,
                            "readings": readings})
    result = {"base_sha256": hashlib.sha256(Path(BASE).read_bytes()).hexdigest(),
              "candidate_sha256": hashlib.sha256(Path(CANDIDATE).read_bytes()).hexdigest(),
              "candidate_debug_sha256": hashlib.sha256(Path(DEBUG).read_bytes()).hexdigest(),
              "flags": FLAGS, "rows": records}
    path = DEST / "r6-driver-error-audit.json"
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"rows": len(records), "by_corpus": {name: sum(r["corpus"] == name for r in records)
                                                        for name in CORPORA if any(r["corpus"] == name for r in records)},
                      "main_outcomes": sorted(set(r["readings"]["main"]["check"]["outcome"]
                                                  for r in records))}))


if __name__ == "__main__":
    main()
