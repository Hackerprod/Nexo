#!/usr/bin/env python3
"""Freeze and stream the exact C0 pass-meter universe."""

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from audit_families import source  # noqa: E402
from verify_corpora import FILE  # noqa: E402

MATCH = "r8-match-main.json"
DIRECTED = "r8-directed74.json"
MATRIX = "r8-matrix45.json"
INVENTORY = HERE / "c0-pass-meter-inventory.json"


def rows_for(path):
    data = json.loads(path.read_text(encoding="utf-8"))
    return data.get("progs", data.get("rows", data)) if isinstance(data, dict) else data


def collect():
    entries = json.loads((HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["corpora"]
    pairs = []
    seen = set()
    source_inventories = {}

    def add(label, profile, rows):
        for item in rows:
            text = source(item)
            row_profile = item.get("profile", profile) if isinstance(item, dict) else profile
            if row_profile is None:
                raise ValueError(f"missing profile for {label}")
            key = (row_profile, text)
            if key in seen:
                continue
            seen.add(key)
            pairs.append((hashlib.sha256(text.encode("utf-8")).hexdigest(), row_profile, text))

    for entry in entries:
        name = entry["id"].split("@")[0]
        path = HERE / FILE[name]
        data = path.read_bytes()
        actual_sha = hashlib.sha256(data).hexdigest()
        if actual_sha != entry["sha256"]:
            raise ValueError(f"frozen corpus SHA mismatch for {entry['id']}: {actual_sha}")
        source_inventories[entry["id"]] = actual_sha
        add(entry["id"], entry["profile"], rows_for(path))

    for label in (MATCH, DIRECTED, MATRIX):
        path = HERE / label
        data = path.read_bytes()
        source_inventories[label] = hashlib.sha256(data).hexdigest()
        payload = json.loads(data.decode("utf-8"))
        add(label, None, payload["rows"])

    return pairs, source_inventories


def inventory_payload(pairs, source_inventories):
    return {
        "schema": "i07c-c0-pass-meter-inventory-v1",
        "base_commit": "31d91dbebd31c35b3cd09095c2d8e65b96c80548",
        "source_inventory_sha256": source_inventories,
        "frozen_batteries_rows": 151188,
        "frozen_batteries_unique_pairs": 145049,
        "match_rows": 103,
        "match_added_pairs": 47,
        "directed74_rows": 74,
        "directed74_added_pairs": 48,
        "matrix45_rows": 45,
        "matrix45_added_pairs": 45,
        "total_unique_source_profile_pairs": len(pairs),
        "pairs": [{"source_sha256": digest, "profile": profile} for digest, profile, _ in pairs],
    }


def main():
    pairs, source_inventories = collect()
    payload = inventory_payload(pairs, source_inventories)
    encoded = json.dumps(payload, ensure_ascii=False, separators=(",", ":")) + "\n"
    if "--write-inventory" in sys.argv:
        INVENTORY.write_text(encoded, encoding="utf-8", newline="\n")
        return
    elif INVENTORY.exists():
        frozen = json.loads(INVENTORY.read_text(encoding="utf-8"))
        if frozen != payload:
            raise ValueError("generated source/profile universe differs from c0-pass-meter-inventory.json")
    else:
        raise FileNotFoundError(f"required frozen inventory missing: {INVENTORY}")

    sys.stdout.write(f"#inventory-count\t{len(pairs)}\n")
    for digest, profile, text in pairs:
        sys.stdout.write(f"{digest}\t{profile}\t{text.encode('utf-8').hex()}\n")


if __name__ == "__main__":
    main()
