#!/usr/bin/env python3
"""Regresiones de archivos de integración omitidos en una copia descartable."""

import io
import os
from pathlib import Path
import re
import runpy
import shutil
import subprocess
import tarfile
import tempfile

REPO = Path(__file__).resolve().parent.parent
TMP = Path("C:/nexo-work/tmp")


def invoke(tree, env):
    return subprocess.run(["python", "-B", "scripts/check-nextest-selection.py"],
                          cwd=tree, env=env, capture_output=True, text=True,
                          encoding="utf-8", errors="replace", timeout=600)


def expect(tree, env, label, expected, name=None):
    result = invoke(tree, env)
    output = result.stdout + result.stderr
    if (result.returncode == 0) != expected or (name and name not in output):
        raise AssertionError(f"{label}: exit {result.returncode}; salida:\n{output[-4000:]}")
    print(f"{label}: {'aceptado' if expected else 'rechazado'}")


def main():
    scrub = runpy.run_path(str(REPO / "scripts/check-nextest-selection.py"))["rust_without_inert_text"]
    inert = '/* #[test] /* #[test] */ */\n// #[test]\nlet s = r#"#[test]"#;\n'
    assert "#[test]" not in scrub(inert), "comentarios/cadenas contados como pruebas"
    TMP.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="nx-selection-", dir=TMP) as folder:
        tree = Path(folder) / "repo"
        tree.mkdir()
        archive = subprocess.check_output(["git", "archive", "HEAD"], cwd=REPO)
        with tarfile.open(fileobj=io.BytesIO(archive)) as bundle:
            bundle.extractall(tree, filter="data")
        shutil.copyfile(REPO / "scripts/check-nextest-selection.py",
                        tree / "scripts/check-nextest-selection.py")
        shutil.copyfile(REPO / "scripts/validate.sh", tree / "scripts/validate.sh")
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(Path(folder) / "target")
        env["CARGO_BUILD_JOBS"] = "6"
        suite = tree / "tests/suite.rs"
        original = suite.read_text(encoding="utf-8")
        declaration = '#[path = "cli.rs"]\nmod cli;'
        assert declaration in original
        expect(tree, env, "árbol limpio", True)

        new = tree / "tests/zz.rs"
        new.write_text('#[test]\nfn zz() { panic!("no ejecutado"); }\n', encoding="utf-8")
        expect(tree, env, "archivo nuevo", False, "tests")
        door = subprocess.run(["sh", "scripts/validate.sh"], cwd=tree, env=env,
                              capture_output=True, text=True, encoding="utf-8",
                              errors="replace", timeout=600)
        if door.returncode == 0 or "suite.rs omite" not in door.stdout + door.stderr:
            raise AssertionError(f"puerta con zz.rs: exit {door.returncode}; salida:\n"
                                 f"{(door.stdout + door.stderr)[-4000:]}")
        print("puerta completa con zz.rs: rechazada antes de los tests")
        new.unlink()

        nested = tree / "tests/zz/main.rs"
        nested.parent.mkdir()
        nested.write_text('#[test]\nfn zz() { panic!("no ejecutado"); }\n', encoding="utf-8")
        expect(tree, env, "subdirectorio main.rs", False, "tests/zz/main.rs")
        nested.unlink()
        nested.parent.rmdir()

        suite.write_text(original.replace(declaration, '#[cfg(any())]\n' + declaration), encoding="utf-8")
        expect(tree, env, "cfg anulador", False, "tests/cli.rs")
        suite.write_text(original.replace(declaration, '#[path = "cli.rs"]\n#[cfg(any())]\nmod cli;'), encoding="utf-8")
        expect(tree, env, "cfg antes de mod", False, "tests/cli.rs")
        suite.write_text(original.replace(declaration, '/*\n' + declaration + '\n*/'), encoding="utf-8")
        expect(tree, env, "declaración comentada", False, "suite.rs")
        suite.write_text(original.replace(declaration, '// #[path = "cli.rs"]\n// mod cli;'), encoding="utf-8")
        expect(tree, env, "declaración en comentario de línea", False, "suite.rs")
        suite.write_text(original + '\n' + declaration + '\n', encoding="utf-8")
        expect(tree, env, "módulo duplicado", False, "suite.rs")
        suite.write_text(original.replace(declaration, '#[path = "corpus.rs"]\nmod cli;'), encoding="utf-8")
        expect(tree, env, "path cruzado", False, "#[path]")
        suite.write_text(original, encoding="utf-8")

        table = tree / "docs/validation/NX-build-mediciones.md"
        config = tree / ".config/nextest.toml"
        table_original = table.read_text(encoding="utf-8")
        config_original = config.read_text(encoding="utf-8")
        exclusions = table_original.split("<!-- NX-BUILD-EXCLUSIONS-START -->", 1)[1]
        exclusions = exclusions.split("<!-- NX-BUILD-EXCLUSIONS-END -->", 1)[0]
        pair = re.search(r'\| `([^`]+)` \| `([^`]+)` \|', exclusions)
        assert pair and pair[2] in config_original
        table.write_text(table_original.replace(pair[2], "__nx_absent__"), encoding="utf-8")
        config.write_text(config_original.replace(pair[2], "__nx_absent__"), encoding="utf-8")
        expect(tree, env, "exclusión fast ausente", False, "pares pesados ausentes")
        table.write_text(table_original, encoding="utf-8")
        config.write_text(config_original, encoding="utf-8")
        expect(tree, env, "árbol restaurado", True)


if __name__ == "__main__":
    main()
