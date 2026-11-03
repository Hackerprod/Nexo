#!/usr/bin/env python3
"""Enumera cada cita normativa de I07c junto con el párrafo citado del ADR."""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ADR = ROOT / "docs/adr/F0-0002-second-slice.md"
SOURCES = (
    ROOT / "src/check/types.rs",
    ROOT / "src/ir/i07c_tests.rs",
    ROOT / "tests/i07c_mutation_depth.rs",
    ROOT / "docs/validation/F0-0002-I07c-implementation-notes.md",
    ROOT / "docs/validation/F0-0002-I07c-contrato-de-cierre.md",
    ROOT / "docs/validation/i07c-mutations/equivalence.json",
)
HEADING = re.compile(r"^#{2,5} (\d+(?:\.\d+)*)\.?(?:\s|$)", re.M)
REFERENCE_GROUP = re.compile(r"§+\s*(\d+(?:\.\d+)*(?:\s*[/,]\s*\d+(?:\.\d+)*)*)")


def sections(source: str) -> dict[str, str]:
    headings = list(HEADING.finditer(source))
    result = {
        match.group(1): source[match.end():headings[index + 1].start() if index + 1 < len(headings) else len(source)]
        for index, match in enumerate(headings)
    }
    deferred = result.get("4.1.2", "")
    paragraphs = list(re.finditer(r"(?m)^(\d)\. \*\*", deferred))
    for index, match in enumerate(paragraphs):
        result[f"4.1.2.{match.group(1)}"] = deferred[
            match.start():paragraphs[index + 1].start() if index + 1 < len(paragraphs) else len(deferred)
        ]
    return result


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    adr = sections(ADR.read_text(encoding="utf-8"))
    errors = []
    count = 0
    for path in SOURCES:
        lines = path.read_text(encoding="utf-8").splitlines()
        for index, line in enumerate(lines, 1):
            refs = list(
                number
                for group in REFERENCE_GROUP.findall(line)
                for number in re.findall(r"\d+(?:\.\d+)*", group)
            )
            for ref in refs:
                count += 1
                paragraph = adr.get(ref)
                if paragraph is None:
                    errors.append(f"{path.relative_to(ROOT)}:{index}: §{ref} no existe")
                    continue
                first = next((part.strip() for part in paragraph.split("\n\n") if part.strip()), "")
                print(f"{path.relative_to(ROOT)}:{index}\t§{ref}\t{line.strip()}\t{first[:180]}")
            for quote in re.findall(r"«([^»]+)»", line):
                if refs and not any(quote in adr.get(ref, "") for ref in refs):
                    errors.append(f"{path.relative_to(ROOT)}:{index}: cita literal ausente de {refs}: {quote}")
    print(f"Citas individuales: {count}; errores mecánicos: {len(errors)}")
    if errors:
        raise SystemExit("\n".join(errors))


if __name__ == "__main__":
    main()
