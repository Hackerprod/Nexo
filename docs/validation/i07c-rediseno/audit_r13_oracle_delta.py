#!/usr/bin/env python3
"""Reclassify only rows changed by the clause-(6) oracle correction."""

import hashlib
import json
import subprocess
import sys
import types
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "scripts"))
import i07c_rd_oracle as new_oracle  # noqa: E402
from audit_families import family, project_main, project_oracle, source  # noqa: E402
from verify_corpora import FILE  # noqa: E402


def modeled(derived):
    return not any(derived.get(key) for key in ("error", "unmodeled", "no_fijado", "d_optional"))


def main():
    executable, case_path = map(Path, sys.argv[1:3])
    old_source = subprocess.check_output(["git", "show", "32aadf3:scripts/i07c_rd_oracle.py"]).decode("utf-8")
    old_oracle = types.ModuleType("old_oracle")
    old_oracle.__file__ = "scripts/i07c_rd_oracle.py"
    exec(compile(old_source, old_oracle.__file__, "exec"), old_oracle.__dict__)
    prior = json.loads((HERE / "r12-family-audit.json").read_text(encoding="utf-8"))
    current = json.loads(json.dumps(prior))
    current["schema"] = "i07c-r13-family-audit-v1"
    current["oracle_script_sha256"] = hashlib.sha256((HERE.parents[2] / "scripts/i07c_rd_oracle.py").read_bytes()).hexdigest()
    current["main_binary_sha256"] = hashlib.sha256(executable.read_bytes()).hexdigest()
    current["delta_from_r12"] = []
    cache = {}
    definitions = json.loads((HERE / "r7-frozen-batteries.json").read_text(encoding="utf-8"))["corpora"]
    for entry in definitions:
        ident = entry["id"]
        path = HERE / FILE[ident.split("@")[0]]
        data = path.read_bytes()
        assert hashlib.sha256(data).hexdigest() == entry["sha256"]
        rows = json.loads(data)
        if isinstance(rows, dict):
            rows = rows["progs"]
        assert len(rows) == entry["rows"]
        for index, item in enumerate(rows):
            src = source(item)
            key = (entry["profile"], src)
            if key not in cache:
                old = old_oracle.derive(src)
                new = new_oracle.derive(src)
                changed = modeled(old) != modeled(new) or (modeled(old) and project_oracle(old) != project_oracle(new))
                if changed:
                    case_path.write_bytes(src.encode("utf-8"))
                    run = subprocess.run([str(executable), "check", str(case_path), "--profile", entry["profile"], "--format", "json"],
                                         capture_output=True, timeout=20, check=False)
                    assert run.returncode in (0, 1), (ident, index, run.returncode, run.stderr)
                    observed = project_main(json.loads(run.stdout))
                    old_label = family(project_oracle(old), observed) if modeled(old) else "unmodeled"
                    new_label = family(project_oracle(new), observed) if modeled(new) else "unmodeled"
                    cache[key] = (old_label, new_label, project_oracle(old) if modeled(old) else None,
                                  project_oracle(new) if modeled(new) else None, observed)
                else:
                    cache[key] = None
            change = cache[key]
            if change is None:
                continue
            before, after, old_expected, new_expected, observed = change
            assert current["corpora"][ident]["counts"][before] > 0
            current["corpora"][ident]["counts"][before] -= 1
            current["corpora"][ident]["counts"][after] = current["corpora"][ident]["counts"].get(after, 0) + 1
            if before != "unmodeled" and before != "equal":
                old_group = current["families"][before]
                old_group["by_corpus"][ident] -= 1
                old_group["row_ids"][ident].remove(index)
            if after != "unmodeled" and after != "equal":
                new_group = current["families"][after]
                new_group["by_corpus"][ident] = new_group["by_corpus"].get(ident, 0) + 1
                new_group["row_ids"].setdefault(ident, []).append(index)
                new_group["row_ids"][ident].sort()
            current["delta_from_r12"].append({"id": f"{ident}:{index}", "source_sha256": hashlib.sha256(src.encode("utf-8")).hexdigest(),
                                                "before": before, "after": after, "old_oracle": old_expected,
                                                "new_oracle": new_expected, "main": observed})
        print(ident, "changed", sum(item["id"].startswith(ident + ":") for item in current["delta_from_r12"]), flush=True)
    assert all(sum(v["counts"].values()) == v["rows"] for v in current["corpora"].values())
    (HERE / "r13-family-audit.json").write_text(json.dumps(current, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")
    print("total changed rows", len(current["delta_from_r12"]))


if __name__ == "__main__":
    main()
