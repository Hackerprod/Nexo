#!/usr/bin/env python3
"""Transcribe B round-3 acceptance cases to a fixed byte-exact fixture.

Run once with the coordination report, then review and commit the JSON. The
runtime acceptance runner reads only the committed JSON, never B's report.
"""

import json
from pathlib import Path
import re
import sys


def main() -> None:
    report = Path(sys.argv[1]).read_text(encoding="utf-8-sig")
    section = report.split("## (4) Sondas de aceptacion", 1)[1].split("## No comprobado", 1)[0]
    rows = []
    for line in section.splitlines():
        for match in re.finditer(r"(\d\d) `([^`]+)` => (.*?)(?= ;; \d\d `|$)", line):
            number, source, answer = match.groups()
            if "fn " not in source:
                continue
            expected = [
                {"code": "NX-E" + code, "start": int(start), "end": int(end)}
                for code, start, end in re.findall(r"(\d{4})@(\d+)-(\d+)", answer)
            ]
            if not expected and answer.strip() != "limpio":
                raise ValueError((number, answer))
            if any(not (0 <= d["start"] < d["end"] <= len(source.encode("ascii"))) for d in expected):
                raise ValueError((number, expected))
            rows.append({"id": number, "profile": "f0-0002-b-v2", "source": source,
                         "bytes": len(source.encode("ascii")), "expected": expected})
    if [r["id"] for r in rows] != [f"{i:02}" for i in range(1, 36)]:
        raise ValueError([r["id"] for r in rows])
    output = Path(sys.argv[2])
    output.write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {len(rows)} byte-exact cases to {output}")


if __name__ == "__main__":
    main()
