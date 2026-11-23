"""Single reading surface of the oracle (class LECTOR).

Only this module (plus ``audit`` for provenance) opens files.  It reads fixtures named
``f4_oracle_<name>.(json|jsonl|py)`` and the pinned ADR files from a closed table.
"""

from __future__ import annotations

import hashlib
import json
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
FIXTURE_DIR = os.path.join(ROOT, "tests", "fixtures")
NAME_RE = re.compile(r"^f4_oracle_[a-z0-9_]+\.(json|jsonl|py)$")

ADR_TABLE = {
    "K": "docs/adr/F0-0004-text-and-user-types.md",
    "A2": "docs/adr/F0-0002-second-slice.md",
    "F0003": "docs/adr/F0-0003-third-slice.md",
    "F0005": "docs/adr/F0-0005-operators-control-collections.md",
    "D15": "docs/adr/F0-D15-sintaxis-oficial.md",
}


class FixtureError(Exception):
    pass


def _lf(data: bytes) -> bytes:
    return data.replace(b"\r\n", b"\n")


def fixture_path(name: str) -> str:
    if not NAME_RE.match(name):
        raise FixtureError(f"fixture name not allowed: {name!r}")
    return os.path.join(FIXTURE_DIR, name)


def allowed_read_paths() -> list[str]:
    """Absolute paths (or directory prefixes) the oracle may open while generating."""
    paths = [os.path.join(ROOT, rel.replace("/", os.sep)) for rel in ADR_TABLE.values()]
    paths.append(FIXTURE_DIR + os.sep + "f4_oracle_")
    return paths


def read_fixture_bytes(name: str) -> bytes:
    with open(fixture_path(name), "rb") as handle:
        return _lf(handle.read())


def load_json(name: str):
    return json.loads(read_fixture_bytes(name).decode("utf-8"))


def load_jsonl(name: str) -> list:
    out = []
    for line in read_fixture_bytes(name).decode("utf-8").split("\n"):
        if line.strip():
            out.append(json.loads(line))
    return out


def adr_bytes(key: str) -> bytes:
    if key not in ADR_TABLE:
        raise FixtureError(f"ADR key not pinned: {key!r}")
    with open(os.path.join(ROOT, ADR_TABLE[key].replace("/", os.sep)), "rb") as handle:
        return _lf(handle.read())


def adr_text(key: str) -> str:
    return adr_bytes(key).decode("utf-8")


def adr_sha256(key: str) -> str:
    return hashlib.sha256(adr_bytes(key)).hexdigest()


def adr_blob(key: str) -> str:
    data = adr_bytes(key)
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def dumps(obj) -> str:
    """Canonical JSON used by every record the oracle writes."""
    return json.dumps(obj, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


# ---------------------------------------------------------------- fragment extraction

def section_lines(text: str, number: str) -> list[str]:
    """Lines of the section whose heading starts with ``number`` (e.g. ``10.1``)."""
    lines = text.split("\n")
    start = None
    for i, line in enumerate(lines):
        if re.match(r"^#{2,4} " + re.escape(number) + r"\.? ", line) or line.startswith("## " + number + " "):
            start = i
            break
    if start is None:
        raise FixtureError(f"section not found: {number}")
    level = len(lines[start]) - len(lines[start].lstrip("#"))
    end = len(lines)
    for j in range(start + 1, len(lines)):
        m = re.match(r"^(#+) ", lines[j])
        if m and len(m.group(1)) <= level:
            end = j
            break
    return lines[start:end]


def extract(text: str, section: str, locator: dict) -> str:
    """Return the literal fragment designated by ``locator`` inside ``section``."""
    lines = section_lines(text, section)
    kind = locator["kind"]
    if kind == "rows":
        ident = locator["id"]
        pat = re.compile(r"^\| " + re.escape(ident) + r"(?=[ ;|])")
        found = [ln for ln in lines if pat.match(ln)]
        if locator.get("table") is not None:
            found = [found[locator["table"]]]
        if not found:
            raise FixtureError(f"no rows for {ident} in {section}")
        return "\n".join(found)
    if kind == "ordinal":
        rows = [ln for ln in lines if ln.startswith("| ")]
        data = [ln for ln in rows if not ln.startswith("| ---") and not ln.startswith("|---")]
        data = data[1:]  # header of the first table
        return data[locator["n"] - 1]
    if kind == "line_contains":
        hits = [ln for ln in lines if locator["text"] in ln]
        if len(hits) != 1:
            raise FixtureError(f"line_contains needs exactly one hit: {locator['text']!r} ({len(hits)})")
        return hits[0]
    if kind == "line_prefix":
        hits = [ln for ln in lines if ln.startswith(locator["text"])]
        if len(hits) != 1:
            raise FixtureError(f"line_prefix needs exactly one hit: {locator['text']!r} ({len(hits)})")
        return hits[0]
    if kind == "block_after":
        marker = locator["text"]
        for i, ln in enumerate(lines):
            if marker in ln:
                j = i + 1
                while j < len(lines) and not lines[j].startswith("```"):
                    j += 1
                k = j + 1
                body = []
                while k < len(lines) and not lines[k].startswith("```"):
                    body.append(lines[k])
                    k += 1
                return "\n".join(body)
        raise FixtureError(f"block_after marker not found: {marker!r}")
    raise FixtureError(f"unknown locator kind: {kind}")


def extract_doc(key: str, section: str, locator: dict) -> str:
    return extract(adr_text(key), section, locator)
