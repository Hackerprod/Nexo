#!/usr/bin/env python3
"""Measure selected I07c source forms against the archived b506d7d checker."""

import io
import contextlib
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
REFERENCE = "b506d7d"
CASES = {
    "array_branch_group": "fn Main(){let nonce=0;let x:Array<I64>=(if true then (([false])) else [false]);}",
    "array_independent_elements": "fn Main(){let nonce=2;let x:Array<Bool>=(if true then [(451),(769),true] else [true,(if true then true else true),705]);}",
    "block_value_nested_branch": "fn Main(){let nonce=36;let x:Array<I64>=(if true then {value:[(428),false];} else {value:[false,false,(if true then 74 else false)];});}",
    "group_nested_continuation": "fn Main(){let nonce=40;let x:I64=((if true then {value:(if true then false else false);} else {value:937;}));}",
    "boolean_comparison_alias_less": "fn Main(){var q=(if true then {return;} else {return;});q=q<q;}",
    "boolean_comparison_alias_equal": "fn Main(){var q=(if true then {return;} else {return;});q=1==q;}",
    "selector_internal_union": "fn Main(){let q=3;let a=q[(if true then 1 else true)];}",
    "array_annotated_elements": "fn Main(){let x:Array<I64>=[true,false];}",
    "later_array_assignment": "fn Main(){var r=[];r=[3];}",
    "immutable_indexed_write_without_entry": (ROOT / "tests/fixtures/i07c/immutable_array_element_write_without_entry.n").read_text(encoding="utf-8"),
    "missing_entry_preempts_immutable_element_write": (ROOT / "tests/fixtures/i07c/missing_entry_preempts_immutable_element_write.n").read_text(encoding="utf-8"),
    "empty_array_comparison_under_print": "fn Main(){Print([]==[48]);}",
    "empty_array_alias_annotated_bool": "fn Main(){var q=[];let r:Array<Bool>=q;}",
    "empty_array_if_arm": "fn Main(){var q=(if true then [] else [false]);}",
    "empty_array_value_block_arm": "fn Main(){let q=(if true then {value:[67];} else {value:[];});}",
    "empty_array_nested_element": "fn Main(){var q=[[],[true,true,false]];}",
    "empty_array_comparison_used_by_len": "fn Main(){var q=[65]==[];var r=Len([q,q]);}",
    "empty_array_index_context": "fn Main(){var q=[];let r=12;let a:Array<I64>=q[r];}",
    "divergent_len_operand": "fn Main(){let n=Len((if true then {return;}else{return;}));}",
    "empty_array_assert_argument": "fn Main(){let y=[1];AssertEq([],y);}",
    "unresolved_empty_array_index": "fn Main(){let x=[][0];}",
}
CAUSAL_RELATIONS = {"array_independent_elements", "block_value_nested_branch", "array_annotated_elements"}
DIAG = re.compile(r"^(NX-E\d+)(?:/([^ ]+))? [^ ]+ .*?\[(\d+),(\d+)\) exp=(.*?) found=(.*?) rel=(.*)$")


def run(command, cwd, env=None):
    result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True,
                            encoding="utf-8", errors="replace", check=False)
    if result.returncode:
        raise RuntimeError(f"{command}: {result.returncode}\n{result.stderr}")
    return result.stdout


def signature(exe, source, folder):
    path = folder / "case.n"
    path.write_text(source, encoding="utf-8", newline="\n")
    output = run([str(exe), "sum", str(path)], folder)
    execution = run([str(exe), "diag", str(path)], folder)
    run_line = next(line for line in execution.splitlines() if line.startswith("run: "))
    run_codes = re.findall(r'code: "(NX-E\d+)"', run_line)
    if output.strip() == "OK":
        return {"accepts": True, "diagnostics": [], "snapshot": [], "run_codes": run_codes}
    parsed = [DIAG.match(line) for line in output.splitlines()]
    if not parsed or any(match is None for match in parsed):
        raise RuntimeError(f"cannot parse harness output:\n{output}")
    return {
        "accepts": False,
        "diagnostics": [[m.group(1), m.group(2), int(m.group(3)), int(m.group(4))] for m in parsed],
        "snapshot": [[m.group(5), m.group(6), m.group(7)] for m in parsed],
        "run_codes": run_codes,
    }


def main():
    (ROOT / ".scratch").mkdir(exist_ok=True)
    with contextlib.nullcontext(Path(tempfile.mkdtemp(prefix="i07c-reference-", dir=ROOT / ".scratch"))) as folder:
        base_override = os.environ.get("I07C_BASE_EXE")
        if base_override:
            base_exe = Path(base_override).resolve(strict=True)
        else:
            tree = folder / "reference"
            tree.mkdir()
            archive = subprocess.run(["git", "archive", REFERENCE], cwd=ROOT,
                                     capture_output=True, check=True).stdout
            with tarfile.open(fileobj=io.BytesIO(archive)) as bundle:
                for member in bundle.getmembers():
                    destination = (tree / member.name).resolve()
                    if destination != tree.resolve() and tree.resolve() not in destination.parents:
                        raise RuntimeError(member.name)
                bundle.extractall(tree, filter="data")
            import shutil
            shutil.copytree(ROOT / "scripts/i07c_harness", tree / "scripts/i07c_harness",
                            ignore=shutil.ignore_patterns("target", "__pycache__"))
            shutil.copy2(ROOT / "scripts/i07c_common.rs", tree / "scripts/i07c_common.rs")
            env = os.environ.copy()
            env["CARGO_TARGET_DIR"] = str(folder / "target")
            run(["cargo", "build", "--locked", "--offline", "--release", "--manifest-path",
                 str(tree / "scripts/i07c_harness/Cargo.toml")], tree, env)
            suffix = ".exe" if os.name == "nt" else ""
            base_exe = folder / "target/release" / f"i07c-campaign-harness{suffix}"
        override = os.environ.get("I07C_CURRENT_EXE")
        if override:
            current_exe = Path(override).resolve()
            assert current_exe.is_file(), current_exe
        else:
            current_env = os.environ.copy()
            current_env["CARGO_TARGET_DIR"] = str(ROOT / "scripts/i07c_harness/target")
            run(["cargo", "build", "--locked", "--offline", "--release", "--manifest-path",
                 str(ROOT / "scripts/i07c_harness/Cargo.toml")], ROOT, current_env)
            current_exe = Path(current_env["CARGO_TARGET_DIR"]) / "release" / f"i07c-campaign-harness{suffix}"
        rows = {}
        for name, source in CASES.items():
            base = signature(base_exe, source, folder)
            current = signature(current_exe, source, folder)
            category = ("acceptance" if base["accepts"] != current["accepts"] else
                        "diagnostics" if (base["diagnostics"] != current["diagnostics"] or
                                          base["run_codes"] != current["run_codes"]) else
                        ("normative_relations" if name in CAUSAL_RELATIONS else "snapshot")
                        if base["snapshot"] != current["snapshot"] else "identical")
            rows[name] = {"category": category, "base": base, "current": current}
        output = json.dumps({"reference": REFERENCE, "cases": rows}, ensure_ascii=False, indent=2) + "\n"
        if len(sys.argv) > 1:
            Path(sys.argv[1]).write_text(output, encoding="utf-8", newline="\n")
        else:
            print(output, end="")


if __name__ == "__main__":
    main()
