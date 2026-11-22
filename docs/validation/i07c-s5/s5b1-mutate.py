"""Invert only the failed-base selector visit in a disposable archived tree.

Usage: python -B .../s5b1-mutate.py --prepare <archived-tree>
       python -B .../s5b1-mutate.py --restore <archived-tree>
"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil


ROOT = Path(__file__).resolve().parents[3]
GUARD = "Err(error) if error.operation != Operation::AnalysisLimit => {"
MUTANT = "Err(error) if false && error.operation != Operation::AnalysisLimit => {"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--prepare", type=Path)
    mode.add_argument("--restore", type=Path)
    args = parser.parse_args()
    tree = args.prepare or args.restore
    assert (tree / "Cargo.toml").exists() and tree.resolve() != ROOT
    dest = tree / "src/check/types.rs"
    saved = tree / "source-original-s5b1.bin"
    manifest = tree / "mutation-s5b1.json"
    if args.prepare:
        assert not saved.exists() and not manifest.exists()
        original = (ROOT / "src/check/types.rs").read_bytes()
        text = original.decode("utf-8")
        assert text.count(GUARD) == 1
        mutated = text.replace(GUARD, MUTANT).encode("utf-8")
        saved.write_bytes(original)
        dest.write_bytes(mutated)
        for name in ("suite.rs", "i07c_s5b_selector.rs", "i07c_s4_context_regressions.rs"):
            shutil.copyfile(ROOT / "tests" / name, tree / "tests" / name)
        data = {"initial_sha256": sha(original), "mutated_sha256": sha(mutated),
                "original_blob_sha256": sha((tree / "source-original-s5b1.bin").read_bytes()),
                "inversion": [GUARD, MUTANT]}
        manifest.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
        assert sha(dest.read_bytes()) == data["mutated_sha256"]
    else:
        data = json.loads(manifest.read_text(encoding="utf-8"))
        assert sha(dest.read_bytes()) == data["mutated_sha256"]
        original = saved.read_bytes()
        assert sha(original) == data["initial_sha256"]
        dest.write_bytes(original)
        assert sha(dest.read_bytes()) == data["initial_sha256"]
    print(json.dumps(data, sort_keys=True))


if __name__ == "__main__":
    main()
