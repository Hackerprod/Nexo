"""Regression for the 65 KiB TYPES stack boundary (Windows MSVC, rustc 1.99.1)."""

import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import tomllib


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


def config_files(root, cargo_home):
    """Archivos de configuración de cargo que aplican: del repo hacia arriba y CARGO_HOME."""
    found = []
    for directory in (Path(root).resolve(), *Path(root).resolve().parents):
        for name in ("config.toml", "config"):
            found.append(directory / ".cargo" / name)
    if cargo_home:
        for name in ("config.toml", "config"):
            found.append(Path(cargo_home) / name)
    seen, unique = set(), []
    for path in found:
        key = os.path.normcase(str(path))
        if key not in seen and path.is_file():
            seen.add(key)
            unique.append(path)
    return unique


def find_debug(table, trail):
    for key, value in table.items():
        if key == "debug":
            yield ".".join(trail + [key])
        elif isinstance(value, dict):
            yield from find_debug(value, trail + [key])


def config_violations(files):
    # Limite conocido (no bloqueante): `include = [...]` de la config de cargo no se sigue.
    """Claves de configuración que cambiarían la pila medida (rustflags o debuginfo)."""
    problems = []
    for path in files:
        try:
            data = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
            problems.append(f"{path}: configuración ilegible ({error})")
            continue
        build = data.get("build")
        if isinstance(build, dict):
            for key in ("rustflags", "encoded-rustflags"):
                if key in build:
                    problems.append(f"{path}: build.{key}")
        target = data.get("target")
        if isinstance(target, dict):
            for triple, value in target.items():
                if isinstance(value, dict) and "rustflags" in value:
                    problems.append(f"{path}: target.{triple}.rustflags")
        profile = data.get("profile")
        if isinstance(profile, dict):
            for found in find_debug(profile, ["profile"]):
                problems.append(f"{path}: {found}")
    return problems


def selftest():
    with tempfile.TemporaryDirectory() as directory:
        base = Path(directory)

        def write(relative, text):
            path = base / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            return path

        assert config_violations([]) == [], "sin configuración no debe fallar"
        assert config_violations([write("a/config.toml", "")]) == [], "config vacío falló"
        clean = write("b/config.toml", '[build]\njobs = 4\n[profile.release]\nopt-level = 3\n'
                      '[target.x86_64-pc-windows-msvc]\nlinker = "link.exe"\n')
        assert config_violations([clean]) == [], "config sin claves prohibidas falló"
        cases = {
            "build.rustflags": '[build]\nrustflags = ["-C", "debuginfo=0"]\n',
            "build.encoded-rustflags": '[build]\nencoded-rustflags = "-Cdebuginfo=0"\n',
            "target.x86_64-pc-windows-msvc.rustflags":
                '[target.x86_64-pc-windows-msvc]\nrustflags = ["-C", "debuginfo=0"]\n',
            "profile.test.debug": "[profile.test]\ndebug = 0\n",
            "profile.dev.debug": "[profile.dev]\ndebug = false\n",
            "profile.custom.debug": "[profile.custom]\ndebug = 1\n",
            "profile.dev.package.nexo.debug": "[profile.dev.package.nexo]\ndebug = 0\n",
        }
        for index, (expected, text) in enumerate(cases.items()):
            found = config_violations([write(f"c{index}/config.toml", text)])
            assert len(found) == 1 and found[0].endswith(expected), (expected, found)
        assert config_violations([write("d/config.toml", "[build\n")]), "TOML roto aceptado"
        # Descubrimiento: .cargo/config.toml y .cargo/config del árbol y CARGO_HOME.
        repo = base / "arbol" / "repo"
        repo.mkdir(parents=True)
        write("arbol/.cargo/config", '[build]\nrustflags = ["-Cdebuginfo=0"]\n')
        home = base / "home"
        write("home/config.toml", "[profile.test]\ndebug = 0\n")
        found = config_violations(config_files(repo, home))
        assert len(found) == 2, found
        assert config_violations(config_files(repo, base / "sin-home")) != []
    print("f5-stack65 selftest: rustflags/debug en configuracion fallan; vacio o sin configuracion no")


def check_cargo_config():
    home = os.environ.get("CARGO_HOME") or str(Path.home() / ".cargo")
    problems = config_violations(config_files(ROOT, home))
    require(not problems, "65 KiB requires cargo config without rustflags or debug overrides: "
            + "; ".join(problems))


def main():
    if "--selftest" in sys.argv[1:]:
        selftest()
        return
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
    check_cargo_config()
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
