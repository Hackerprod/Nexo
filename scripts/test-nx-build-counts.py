#!/usr/bin/env python3
"""Regresiones de recuentos nextest y confirmación de mutantes en copia sintética."""

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


REPO = Path(__file__).resolve().parent.parent
TMP = Path("C:/nexo-work/tmp")


def run(argv, cwd, *, env=None):
    done = subprocess.run(argv, cwd=cwd, env=env, text=True, capture_output=True,
                          encoding="utf-8", errors="replace", timeout=900)
    if done.returncode:
        raise AssertionError(f"{argv}: exit {done.returncode}\n{done.stdout}\n{done.stderr}")
    return done.stdout


def write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(data, encoding="utf-8")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def count_forms():
    spec = importlib.util.spec_from_file_location("mutant_verdict", REPO / "scripts/mutant-verdict.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    for count, noun in ((1, "test"), (2, "tests")):
        result = {"timeout": False, "exit": 0,
                  "stdout": json.dumps({"type": "suite", "event": "ok"}) + "\n",
                  "stderr": f"Summary [ 0.1s] {count} {noun} run: {count} passed, 0 skipped"}
        assert module.classify(result)[0] == "VERDE", (count, noun)


def mutant_case(root, name, *, flaky):
    tree = root / name
    source = "pub fn value() -> i32 { let x = 1; x }\n"
    write(tree / "Cargo.toml", '[package]\nname = "nx_counts"\nversion = "0.1.0"\nedition = "2021"\n')
    write(tree / "src/lib.rs", source)
    if flaky:
        test = '''#[test]
fn real() {
    let marker = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("first-run.marker");
    if !marker.exists() { std::fs::write(marker, "seen").unwrap(); panic!("primera vez"); }
    assert!(nx_counts::value() > 0);
}
'''
    else:
        test = '#[test]\nfn real() { assert_eq!(nx_counts::value(), 1); }\n'
    write(tree / "tests/real.rs", test)
    run(["cargo", "generate-lockfile", "--offline"], tree)
    original = (tree / "src/lib.rs").read_bytes()
    mutated = original.replace(b"let x = 1;", b"let x = 2;")
    patch = {"file": "src/lib.rs", "before": "let x = 1;", "after": "let x = 2;",
             "occurrence": 0, "sha_initial": sha(original), "sha_mutated": sha(mutated)}
    patch_file = root / (name + "-patch.json")
    patch_file.write_text(json.dumps(patch), encoding="utf-8")
    output = root / (name + "-result.json")
    run(["python", "-B", str(REPO / "scripts/mutant-verdict.py"), "--tree", str(tree),
         "--patch", str(patch_file), "--id", name, "--output", str(output)], tree)
    row = json.loads(output.read_text(encoding="utf-8"))
    expected = "INCONCLUSO" if flaky else "MUERTO"
    assert row["verdict"] == expected, row
    confirmation = row["profiles"][0]["confirmation"]
    assert confirmation["baseline_verdict"] == "VERDE", confirmation
    assert confirmation["mutated_verdict"] == ("VERDE" if flaky else "MUERTO"), confirmation
    assert (tree / "src/lib.rs").read_bytes() == original


def gate_one_test(root):
    tree = root / "gate"
    write(tree / "Cargo.toml", '[package]\nname = "nexo"\nversion = "0.1.0"\nedition = "2021"\n')
    write(tree / "src/main.rs", "fn main() {}\n")
    write(tree / "tests/real.rs", "#[test]\nfn real() { assert_eq!(2 + 2, 4); }\n")
    for name in ("gate.sh", "heavy-lock.py"):
        dest = tree / "scripts" / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(REPO / "scripts" / name, dest)
    write(tree / "scripts/validate.sh", "#!/usr/bin/env sh\nset -eu\ncargo nextest run --locked --offline --profile default --retries 0\ncargo nextest run --locked --offline --profile default --retries 0 --release\n")
    write(tree / "scripts/validate-fast.sh", "#!/usr/bin/env sh\nexit 0\n")
    write(tree / "scripts/check-nextest-selection.py", "# Fixture del inventario\n")
    write(tree / "scripts/objetivo.py", "print('Programas que ejecutan: 1/1')\n")
    write(tree / "scripts/i07c_rd_conformidad.py", "print('RD normative: 1/1 check/run pairs')\n")
    write(tree / ".config/nextest.toml", "[profile.default]\n")
    run(["cargo", "generate-lockfile", "--offline"], tree)
    run(["git", "init", "-q"], tree)
    run(["git", "add", "."], tree)
    run(["git", "-c", "user.name=Nexo", "-c", "user.email=nexo@example.invalid",
         "commit", "-qm", "fixture de un test"], tree)
    env = os.environ.copy()
    env["NEXO_HEAVY_LOCK_HELD"] = "1"
    output = run(["sh", "scripts/gate.sh", "HEAD"], tree, env=env)
    assert output.startswith("GREEN:"), output
    verified = run(["sh", "scripts/gate.sh", "HEAD", "--verify"], tree, env=env)
    assert "registro GREEN verificado" in verified, verified


def main():
    TMP.mkdir(parents=True, exist_ok=True)
    count_forms()
    with tempfile.TemporaryDirectory(prefix="nx-b2-fix2-", dir=TMP) as folder:
        root = Path(folder)
        prior_target = os.environ.get("CARGO_TARGET_DIR")
        os.environ["CARGO_TARGET_DIR"] = str(root / "target")
        try:
            mutant_case(root, "kill", flaky=False)
            mutant_case(root, "flaky", flaky=True)
            gate_one_test(root)
        finally:
            if prior_target is None:
                os.environ.pop("CARGO_TARGET_DIR", None)
            else:
                os.environ["CARGO_TARGET_DIR"] = prior_target
    print("Recuentos singular/plural, mutante MUERTO, flaky INCONCLUSO y gate de un test: correctos")


if __name__ == "__main__":
    main()
