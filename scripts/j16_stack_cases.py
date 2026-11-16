#!/usr/bin/env python3
"""Reproduce the J16 parser phase on a real 204 KiB thread."""

from __future__ import annotations

import hashlib
import os
from pathlib import Path
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
TEST = "frontend::parse::j16_parser_stack_probe::direct_204_kib_file_child"


def source(family: str, depth: int) -> str:
    if family.startswith("objective_"):
        return (ROOT / "corpus" / "objetivo" / family.removeprefix("objective_")).read_text(encoding="utf-8")
    if family == "block":
        return f'fn Main(){{{"{" * depth}let x=1;{"}" * depth}}}'
    if family == "group":
        return f'fn Main(){{let x={"(" * depth}1{")" * depth};}}'
    if family == "array":
        return f'fn Main(){{let x={"[" * depth}1{"]" * depth};}}'
    if family == "index":
        return f'fn Main(){{let a=[1];let x=a{"[0]" * depth};}}'
    if family == "binary_left":
        return f'fn Main(){{let x=1{"+1" * depth};}}'
    if family == "binary_right":
        return f'fn Main(){{let x={"1+(" * depth}1{")" * depth};}}'
    if family == "call_arg":
        return f'fn F(x:view I64)->I64{{return x;}}fn Main(){{let x={"F(" * depth}1{")" * depth};}}'
    if family == "mixed":
        return f'fn Main(){{{"{" * depth}let x={"(" * depth}1{")" * depth};{"}" * depth}}}'
    if family == "yield_group":
        return f'gen fn G()->I64{{yield {"(" * depth}1{")" * depth};}}fn Main(){{}}'
    if family == "else_if":
        return f'fn Main(){{{"if true{}else " * depth}{{}}}}'
    if family == "if_expr":
        expr = "1"
        for _ in range(depth):
            expr = f"if true then ({expr}) else 0"
        return f"fn Main(){{let x={expr};}}"
    if family == "if_stmt":
        return f'fn Main(){{{"if true{" * depth}{"}" * depth}}}'
    if family == "while":
        return f'fn Main(){{{"while true{" * depth}{"}" * depth}}}'
    if family == "closure":
        return f'fn Main(){{let f={"fn(){let g=" * depth}1{";}" * depth};}}'
    if family == "closure_called":
        expr = "1"
        for _ in range(depth):
            expr = f"(fn ()->I64{{return {expr};}})()"
        return f"fn Main(){{let x={expr};}}"
    if family == "match_scrutinee":
        expr = "1"
        for _ in range(depth):
            expr = f"match ({expr}){{_=>{{value:1;}}}}"
        return f"fn Main(){{let x={expr};}}"
    if family == "match_expr":
        expr = "1"
        for _ in range(depth):
            expr = f"match 1{{_=>{{value:{expr};}}}}"
        return f"fn Main(){{let x={expr};}}"
    if family == "match_stmt":
        body = ""
        for _ in range(depth):
            body = f"match 1{{_=>{{{body}}}}}"
        return f"fn Main(){{{body}}}"
    if family == "type":
        return f'fn F(x:view {"Array<" * depth}I64{">" * depth}){{}}fn Main(){{}}'
    if family == "pattern":
        return 'fn Main(){match 1{' + '(' * depth + '_' + ')' * depth + '=>{},}}'
    if family == "edit_nested":
        return ('fn F(x:edit I64)->I64{return 1;}fn Main(){var a=[1];let y='
                + 'F(edit a[' * depth + '0' + '])' * depth + ';}')
    if family in ("move_call", "edit_call", "mode_alternating"):
        modes = (["move"] * depth if family == "move_call" else
                 ["edit"] * depth if family == "edit_call" else
                 ["move" if i % 2 == 0 else "edit" for i in range(depth)])
        return ('fn Main(){let f=fn(p:move I64){};let y='
                + ''.join(f'{mode} f(' for mode in modes) + '1' + ')' * depth + ';}')
    if family == "move_arg":
        return ('fn F(x:move I64){}fn Main(){let f=fn(p:move I64){};F('
                + 'move f(' * depth + '1' + ')' * depth + ');}')
    if family == "edit_simple":
        return 'fn F(x:edit I64){}fn Main(){var a=1;F(edit a);}'
    raise ValueError(family)


CASES = [
    ("block", 126, "ok"),
    ("group", 126, "ok"),
    ("array", 126, "ok"),
    ("index", 126, "ok"),
    ("binary_left", 126, "ok"),
    ("binary_right", 126, "ok"),
    ("call_arg", 126, "ok"),
    ("mixed", 63, "ok"),
    ("yield_group", 126, "ok"),
    ("else_if", 126, "ok"),
    ("if_expr", 126, "ok"),
    ("if_stmt", 126, "ok"),
    ("while", 126, "ok"),
    ("closure", 63, "ok"),
    ("closure_called", 40, "ok"),
    ("match_scrutinee", 60, "ok"),
    ("match_expr", 63, "ok"),
    ("match_stmt", 60, "ok"),
    ("type", 126, "ok"),
    ("pattern", 126, "ok"),
    ("edit_simple", 1, "ok"),
    ("edit_nested", 40, "ok"),
    ("edit_nested", 42, "ok"),
    ("move_call", 63, "ok"),
    ("edit_call", 63, "ok"),
    ("mode_alternating", 63, "ok"),
    ("move_arg", 42, "ok"),
    ("block", 128, "NX-E0003:syntax_nesting:137:138"),
    ("type", 128, "NX-E0003:syntax_nesting:780:783"),
    ("pattern", 128, "NX-E0003:syntax_nesting:145:146"),
]
CASES.extend((f"objective_{path.name}", 0, "ok") for path in sorted(
    (ROOT / "corpus" / "objetivo").glob("*.n")) if path.name[:2].isdigit()
    and 1 <= int(path.name[:2]) <= 15)


def main() -> int:
    scratch = ROOT / ".scratch"
    scratch.mkdir(exist_ok=True)
    if not scratch.resolve().is_relative_to(ROOT.resolve()):
        raise RuntimeError("probe scratch is outside the project")
    for profile, kib in ((profile, kib) for profile in ("debug", "release")
                         for kib in (204, 256)):
        with tempfile.TemporaryDirectory(prefix="j16-p-", dir=scratch) as tmp:
            directory = Path(tmp)
            if not directory.resolve().is_relative_to(scratch.resolve()):
                raise RuntimeError("probe directory escaped scratch")
            for index, (family, depth, expected) in enumerate(CASES):
                program = source(family, depth)
                path = directory / f"{index:02d}-{family}.n"
                path.write_text(program, encoding="utf-8")
                env = os.environ.copy()
                env["J16_PARSER_CASE_FILE"] = str(path)
                env["J16_PARSER_EXPECT"] = expected
                env["J16_PARSER_STACK_KIB"] = str(kib)
                command = ["cargo", "test", "--locked", "--offline"]
                if profile == "release":
                    command.append("--release")
                command += ["--lib", TEST, "--", "--exact", TEST]
                result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True,
                                        text=True, encoding="utf-8", errors="replace", timeout=120)
                sha = hashlib.sha256(program.encode()).hexdigest()
                print(f"{profile} {kib} {family} {depth} {sha} exit={result.returncode}", flush=True)
                if result.returncode:
                    print(result.stdout)
                    print(result.stderr)
                    return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
