"""Run the S3 type checker in an isolated Windows thread with a fixed stack."""

import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile


def sources():
    divergent = "(if true then {return;} else {return;})"
    generator = "gen fn G()->I64{yield 1;}fn Main(){let a0=G();"
    for index in range(1, 128):
        generator += f"let a{index}=Some(move a{index - 1});"
    shape = "fn Main(){let l0=" + divergent + ";match l0{Some(x)=>{},_=>{}}let b:"
    return {
        "some84": "fn Main(){let x=" + "Some(" * 84 + "1" + ")" * 84 + ";}",
        "some126": "fn Main(){let x=" + "Some(" * 126 + "1" + ")" * 126 + ";}",
        "array126": "fn Main(){let x=" + "[" * 126 + "1" + "]" * 126 + ";}",
        "array_bool126": "fn Main(){let x:Bool=" + "[" * 126 + "1" + "]" * 126 + ";}",
        "none_array126": "fn Main(){let x=None[" + "[" * 125 + "1" + "]" * 126 + ";}",
        "option_match126": "fn F(x:view " + "Option<" * 126 + "I64" + ">" * 126 + "){match x{_=>{}}}",
        "result_match126": "fn F(x:view " + "Result<" * 126 + "I64" + ",I64>" * 126 + "){match x{_=>{}}}",
        "some_pattern126": "fn F(x:view " + "Option<" * 126 + "I64" + ">" * 126 + "){match x{" + "Some(" * 126 + "_" + ")" * 126 + "=>{},_=>{}}}",
        "array_annotated126": "fn Main(){let x:" + "Array<" * 126 + "I64" + ">" * 126 + "=" + "[" * 126 + "1" + "]" * 126 + ";}",
        "deep_equal126": "fn Main(){var r=" + "[" * 126 + "true" + "]" * 126 + ";r=" + "[" * 126 + "true" + "]" * 126 + ";}",
        "deep_unequal126": "fn Main(){var r=" + "[" * 126 + "true" + "]" * 126 + ";r=" + "[" * 126 + "1" + "]" * 126 + ";}",
        "none_component": "fn Main(){let o=None;}",
        "none_typed": "fn Main(){let o:Option<I64>=None;}",
        "recursive": "fn Main(){var r=[];r=[r];}",
        "recursive_finite": "fn Main(){var r=[];r=[1];}",
        "shape_conflict": shape + "Result<I64,I64>=l0;}",
        "shape_compatible": shape + "Option<I64>=l0;}",
        "generator128": generator + "}",
        "generator129": generator + "let a128=Some(move a127);}",
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("test_exe", type=Path)
    parser.add_argument("--mode", choices=("TYPES", "CHECKER"), required=True)
    parser.add_argument("--kib", type=int)
    parser.add_argument("--minimum", action="store_true")
    parser.add_argument("--case", choices=tuple(sources()) + ("j16", "all"), default="all")
    args = parser.parse_args()
    if not args.minimum and args.kib is None:
        parser.error("--kib is required without --minimum")
    cases = sources()
    if args.case == "j16":
        cases = dict(list(cases.items())[:8])
    elif args.case != "all":
        cases = {args.case: cases[args.case]}
    with tempfile.TemporaryDirectory() as directory:
        for name, source in cases.items():
            path = Path(directory) / (name + ".n")
            path.write_bytes(source.encode("ascii"))
            env = os.environ.copy()
            env.update(I07C_S3_SOURCE=str(path), I07C_S3_MODE=args.mode)
            errors = {"array_bool126": "NX-E0102", "none_array126": "NX-E0113",
                      "deep_unequal126": "NX-E0102", "none_component": "NX-E0113",
                      "recursive": "NX-E0103", "shape_conflict": "NX-E0102",
                      "generator129": "NX-E0003"}
            if name in errors:
                env["I07C_S3_EXPECT_ERROR"] = errors[name]

            def execute(kib):
                env["I07C_S3_STACK_KIB"] = str(kib)
                return subprocess.run(
                    [str(args.test_exe), "--exact", "check::types::j16_probe_types::s3_types_or_checker_child", "--nocapture"],
                    env=env, capture_output=True, text=True, timeout=120,
                )

            if args.minimum:
                for kib in (65, 129, 193, 257):
                    results = [execute(kib) for _ in range(5)]
                    if all(result.returncode == 0 for result in results):
                        previous = [execute(kib - 1) for _ in range(5)]
                        codes = [result.returncode for result in previous]
                        print(f"{name} {args.mode} stable_minimum={kib} KiB passes=5/5 previous={kib - 1} KiB exit_codes={codes} sha256={hashlib.sha256(source.encode('ascii')).hexdigest()}", flush=True)
                        if all(code == 0 for code in codes):
                            raise SystemExit("preceding size also passes consistently")
                        break
                else:
                    raise SystemExit(f"no stable stack size through 257 KiB: {name}")
            else:
                result = execute(args.kib)
                print(f"{name} {args.mode} {args.kib} KiB sha256={hashlib.sha256(source.encode('ascii')).hexdigest()} exit={result.returncode}", flush=True)
                if result.returncode:
                    print(result.stdout, result.stderr, sep="\n", flush=True)
                    raise SystemExit(1)


if __name__ == "__main__":
    main()
