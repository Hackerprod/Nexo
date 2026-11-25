#!/usr/bin/env python3
"""Compare the 103 frozen match rows against main, R6 and the declared table."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

import i07c_rd_sweep as sweep

TABLE = Path("C:/projects/Nexo-i07c/docs/validation/i07c-s5c-explor/match-conservacion-ext.json")
OUTPUT = Path("docs/validation/i07c-rediseno/r6-match103.json")


def inspect(exe, src, profile, flags):
    with tempfile.TemporaryDirectory(prefix="i07c-r5-match-") as directory:
        source = Path(directory) / "p.n"
        source.write_bytes(src.encode("ascii"))
        env = os.environ.copy()
        if flags is None:
            env.pop("NEXO_XV", None)
        else:
            env["NEXO_XV"] = flags
        with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
            process = subprocess.Popen(
                [exe, "check", "--format", "json", "--profile", profile, str(source)],
                stdout=stdout, stderr=stderr, env=env,
            )
            last_output = time.monotonic()
            last_size = 0
            while process.poll() is None:
                size = os.fstat(stdout.fileno()).st_size + os.fstat(stderr.fileno()).st_size
                if size != last_size:
                    last_output, last_size = time.monotonic(), size
                if sweep.working_set(process.pid) > sweep.MAX_WORKING_SET:
                    process.kill()  # This process belongs to this invocation.
                    process.wait()
                    raise RuntimeError("INCONCLUSO: child exceeded 6 GB")
                if time.monotonic() - last_output > sweep.SILENCE_LIMIT:
                    process.kill()  # This process belongs to this invocation.
                    process.wait()
                    raise RuntimeError("INCONCLUSO: child exceeded 20 min of silence")
                time.sleep(0.02)
            stdout.seek(0)
            stderr.seek(0)
            output, error = stdout.read(), stderr.read()
    try:
        data = json.loads(output)
    except (TypeError, ValueError) as exc:
        raise RuntimeError(f"INCONCLUSO: child exit {process.returncode}: {error[-300:]!r}") from exc
    diagnostics = []
    for item in data["diagnostics"]:
        details = item.get("data") or {}
        diagnostics.append({
            "code": item["code"], "subcause": item.get("subcause"),
            "primary": [item["primary_span"]["start"], item["primary_span"]["end"]],
            "expected_type": details.get("expected_type"),
            "found_type": details.get("found_type"),
            "relations": [{"role": relation["role"],
                           "span": [relation["span"]["start"], relation["span"]["end"]]}
                          for relation in item.get("related_spans", [])],
        })
    return {"outcome": (data.get("outcome") or {}).get("kind"), "diagnosticos": diagnostics}


def main():
    base, candidate = sys.argv[1:3]
    table_bytes = TABLE.read_bytes()
    recorded_sha = TABLE.with_suffix(".sha256").read_text(encoding="ascii").split()[0]
    assert hashlib.sha256(table_bytes).hexdigest() == recorded_sha
    table = json.loads(table_bytes)
    assert len(table["filas"]) == 103
    flags = "pendfa,retfa,nojoin,atomic0,e0103rule,retfail,nojoinr,skipv3,r5prefix,r1t5,r5t5,noshape,r6witness"
    rows = []
    for item in table["filas"]:
        expected = {"outcome": item["outcome"], "diagnosticos": item["diagnosticos"]}
        original = inspect(base, item["fuente"], item["perfil"], None)
        actual = inspect(candidate, item["fuente"], item["perfil"], flags)
        rows.append({
            "id": item["id"], "grupo": item["grupo"], "perfil": item["perfil"],
            "main_equals_declared": original == expected,
            "r6_equals_declared": actual == expected,
            "r6_equals_main": actual == original,
            "main": original, "r6": actual,
        })
    result = {
        "table_sha256": hashlib.sha256(table_bytes).hexdigest(),
        "base_sha256": hashlib.sha256(Path(base).read_bytes()).hexdigest(),
        "r6_sha256": hashlib.sha256(Path(candidate).read_bytes()).hexdigest(),
        "flags": flags, "rows": rows,
        "conservation_failures": [r["id"] for r in rows if r["grupo"] == "conservacion"
                                  and (not r["main_equals_declared"] or not r["r6_equals_declared"])],
        "changed_ids": [r["id"] for r in rows if not r["r6_equals_main"]],
        "declared_mismatches": [r["id"] for r in rows if not r["r6_equals_declared"]],
    }
    OUTPUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
    print(json.dumps({key: result[key] for key in ("conservation_failures", "changed_ids", "declared_mismatches")},
                     ensure_ascii=False))


if __name__ == "__main__":
    main()
