"""Regression for the 65 KiB TYPES stack boundary (Windows MSVC, rustc 1.99.1)."""

import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parent.parent
CHILD = "check::types::j16_probe_types::s3_types_or_checker_child"
CASES = ("option_match126", "result_match126")
OVERFLOW = 0xC00000FD


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=180, **kwargs)


def require(ok, message, process=None):
    if not ok:
        if process is not None:
            print(process.stdout, process.stderr, sep="\n", file=sys.stderr)
        raise RuntimeError(message)


def main():
    rustc = run(["rustc", "--version"])
    require(rustc.returncode == 0, "rustc --version failed", rustc)
    if not (sys.platform == "win32" and platform.machine().lower() in ("amd64", "x86_64")
            and "1.99.1" in rustc.stdout and "msvc" in run(["rustc", "-vV"]).stdout):
        print("NO EVALUADO 65 KiB: requires Windows x86_64 MSVC rustc 1.99.1")
        return

    names = ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS")
    names += tuple(name for name in os.environ
                   if name.startswith(("CARGO_PROFILE_TEST_", "CARGO_PROFILE_DEV_")))
    require(not any(os.environ.get(name) for name in names),
            "65 KiB requires empty Rust flags and unmodified test/dev profile: "
            + ", ".join(name for name in names if os.environ.get(name)))
    build = run(["cargo", "test", "--lib", "--no-run", "--profile", "test",
                 "--message-format", "json", "--locked", "--offline"])
    require(build.returncode == 0, "test-profile lib build failed", build)
    artifacts = [json.loads(line) for line in build.stdout.splitlines()
                 if line.startswith("{") and json.loads(line).get("reason") == "compiler-artifact"]
    libs = [artifact for artifact in artifacts
            if artifact["target"]["name"] == "nexo"
            and "lib" in artifact["target"]["kind"]
            and artifact["profile"]["test"] and artifact.get("executable")]
    require(len(libs) == 1, f"expected one lib test artifact, got {len(libs)}")
    artifact = libs[0]
    require(artifact["profile"]["debuginfo"] == 2
            and artifact["profile"]["opt_level"] == "0",
            f"unexpected cargo test profile: {artifact['profile']}")
    exe = Path(artifact["executable"])
    listed = run([str(exe), "--exact", CHILD, "--list"])
    require(listed.returncode == 0 and
            [line for line in listed.stdout.splitlines() if line.startswith(CHILD + ":")]
            == [CHILD + ": test"], "expected exactly one stack probe child", listed)

    from i07c_stack_probe.probe import sources

    with tempfile.TemporaryDirectory() as directory:
        source = Path(directory) / "option_match126.n"
        source.write_text(sources()[CASES[0]], encoding="ascii")
        env = dict(os.environ, I07C_S3_SOURCE=str(source), I07C_S3_MODE="TYPES",
                   I07C_S3_STACK_KIB="16")
        control = run([str(exe), "--exact", CHILD, "--nocapture"], env=env)
        require(control.returncode & 0xFFFFFFFF == OVERFLOW
                and "has overflowed its stack" in control.stderr,
                "16 KiB control did not overflow as expected", control)

    probe = ROOT / "scripts" / "i07c_stack_probe" / "probe.py"
    print(f"f5-stack65 script_sha256={hashlib.sha256(Path(__file__).read_bytes()).hexdigest()} "
          f"exe_sha256={hashlib.sha256(exe.read_bytes()).hexdigest()} control=0x{OVERFLOW:08x}")
    for case in CASES:
        for attempt in range(1, 6):
            result = run([sys.executable, "-B", str(probe), str(exe), "--mode", "TYPES",
                          "--kib", "65", "--case", case])
            require(result.returncode == 0, f"{case} 65 KiB attempt {attempt} failed", result)
            print(f"{case} 65 KiB {attempt}/5 exit={result.returncode}")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.TimeoutExpired, OSError, ValueError, KeyError) as error:
        print(f"f5-stack65: {error}", file=sys.stderr)
        sys.exit(1)
