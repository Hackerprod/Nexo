#!/usr/bin/env python3
"""Measure disposable planning-mark variants against frozen directed oracles."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

from i07c_rd_campaign import BASE_FLAGS

VARIANTS = {
    "projection_statement": "txmarks,txmarksproj,txmarksprojstatement",
    "projection_before": "txmarks,txmarksproj,txmarksprojbefore",
    "projection_allowconcrete": "txmarks,txmarksproj,txmarksallowconcrete",
    "statement_allowconcrete": "txmarks,txmarksstatement,txmarksallowconcrete",
    "occurrence_allowconcrete": "txmarks,txmarksoccurrence,txmarksallowconcrete",
    "projection_statement_allowconcrete": "txmarks,txmarksproj,txmarksprojstatement,txmarksallowconcrete",
    "projection_before_allowconcrete": "txmarks,txmarksproj,txmarksprojbefore,txmarksallowconcrete",
}


def main():
    exe = Path(sys.argv[1]).resolve()
    with exe.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    output = {"binary": str(exe), "sha256": digest, "variants": {}}
    for name, extra in VARIANTS.items():
        flags = BASE_FLAGS + "," + extra
        proc = subprocess.run([sys.executable, "-B", "scripts/i07c_rd_cheap.py", str(exe), flags],
                              capture_output=True, text=True, check=True)
        result = json.loads(proc.stdout)
        output["variants"][name] = result
        print(name, result["summary"], flush=True)
    path = Path("docs/validation/i07c-rediseno/sondas-variantes-marca.json")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(output, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"completed={path} sha256={digest}", flush=True)


if __name__ == "__main__":
    main()
