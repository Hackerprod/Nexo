#!/usr/bin/env python3
"""Check which prototype switches trigger the reduced match/if internal error."""
import hashlib
import json
from pathlib import Path

from i07c_rd_drive_r6 import CANDIDATE, FLAGS
from i07c_rd_internal_min_r6 import CASES
from i07c_rd_pass_errors_r6 import run


def main():
    source = CASES["g"]
    modes = {
        "r6": FLAGS,
        "minus_witness": ",".join(x for x in FLAGS.split(",") if x != "r6witness"),
        "minus_noshape": ",".join(x for x in FLAGS.split(",") if x != "noshape"),
        "noshape_only": "noshape",
        "plus_retired_rmm3": FLAGS + ",rmm3b,rmm3e,rmm3f",
        "unflagged": None,
    }
    rows = {name: {command: run(CANDIDATE, source, command, flags)
                   for command in ("check", "run")}
            for name, flags in modes.items()}
    doc = {"source": source, "bytes": len(source.encode()),
           "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
           "binary_sha256": hashlib.sha256(Path(CANDIDATE).read_bytes()).hexdigest(),
           "modes": modes, "rows": rows}
    output = Path("docs/validation/i07c-rediseno/r6-internal-error-ablation.json")
    output.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for mode, results in rows.items():
        print(mode, results["check"]["outcome"]["kind"], results["run"]["outcome"]["kind"])


if __name__ == "__main__":
    main()
