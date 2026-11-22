#!/usr/bin/env python3
"""Enumera y congela por SHA-256 la población de mutación de un commit dado.

Uso: python scripts/i07c_mut_freeze.py <commit> --base <base> --out <directorio>

Crea un worktree desechable en <commit>, ejecuta allí `scripts/i07c_mutaciones.py --base <base>`
(solo enumeración, sin compilar) con `I07C_MUT_DIR=<out>`, copia `population.json` a <out> en este
repositorio y escribe `FREEZE.md` con base, código evaluado, recuentos, SHA-256 de la población,
SHA-256 de los blobs de producción afectados y SHA-256 del diff. Es andamiaje: no cambia la
población ni su clasificación; solo hace reproducible su congelación antes de ejecutar.
"""
import argparse
import collections
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def git(*args, cwd=ROOT):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True).stdout


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("commit", help="commit cuyo código de producción se evalúa")
    parser.add_argument("--base", required=True, help="commit base: solo se mutan líneas nuevas o modificadas respecto de él")
    parser.add_argument("--out", required=True, type=Path,
                        help="directorio de salida relativo al repositorio (p. ej. docs/validation/i07c-mutations-xyz)")
    parser.add_argument("--note", default="", help="texto libre para FREEZE.md (alcance y motivo de la población)")
    parser.add_argument("--tmp", type=Path, default=Path("C:/nexo-work/tmp") if os.name == "nt" else None,
                        help="directorio temporal donde crear el worktree")
    args = parser.parse_args()
    if args.out.is_absolute():
        print("--out debe ser relativo al repositorio", file=sys.stderr)
        return 2
    commit = git("rev-parse", "--verify", args.commit + "^{commit}").decode().strip()
    base = git("rev-parse", "--verify", args.base + "^{commit}").decode().strip()
    tmp_root = Path(tempfile.mkdtemp(prefix="mut-freeze-", dir=args.tmp))
    tree = tmp_root / "tree"
    git("worktree", "add", "--detach", str(tree), commit)
    try:
        # La base se pasa tal como se escribió: queda tal cual en population.json (SHA-256 reproducible).
        env = dict(os.environ, I07C_MUT_DIR=args.out.as_posix())
        subprocess.run([sys.executable, "-B", "scripts/i07c_mutaciones.py", "--base", args.base],
                       cwd=tree, env=env, check=True)
        population_path = tree / args.out / "population.json"
        raw = population_path.read_bytes()
        population = json.loads(raw)
        mutations = population["mutations"]
        blobs = {}
        for path in sorted({m["file"] for m in mutations}):
            blobs[path] = sha(git("show", f"{commit}:{path}", cwd=ROOT))
        diff_sha = sha(git("diff", base, commit, "--", "src/check", "src/ir"))
        out = ROOT / args.out
        out.mkdir(parents=True, exist_ok=True)
        (out / "population.json").write_bytes(raw)
        by_file = collections.Counter(m["file"] for m in mutations)
        by_operator = collections.Counter(m["operator"] for m in mutations)
        lines = [
            f"# Población de mutación de `{commit[:7]}` respecto de `{base[:7]}` (congelada antes de ejecutar)",
            "",
        ]
        if args.note:
            lines += [args.note, ""]
        lines += [
            f"- Base de la población: `{base}`.",
            f"- Código evaluado: `{commit}`.",
            "- Enumerador: `scripts/i07c_mutaciones.py --base <base>` ejecutado en un worktree desechable del commit "
            f"evaluado con `I07C_MUT_DIR={args.out.as_posix()}` (`scripts/i07c_mut_freeze.py`).",
            f"- Población: {len(mutations)} sustituciones ("
            + ", ".join(f"`{p}` {n}" for p, n in sorted(by_file.items())) + ").",
            "- Operadores: " + ", ".join(f"`{o}` {n}" for o, n in sorted(by_operator.items())) + ".",
            f"- SHA-256 de `population.json`: `{sha(raw)}`.",
        ]
        for path, digest in blobs.items():
            lines.append(f"- SHA-256 del blob `{path}` en el commit evaluado: `{digest}`.")
        lines += [
            f"- SHA-256 de `git diff {base[:7]} {commit[:7]} -- src/check src/ir`: `{diff_sha}`.",
            "- Criterio: un superviviente solo bloquea con un programa discriminante o una regresión que lo distinga "
            "(AGENTS, «Rigor proporcional»); el resto se registra en `Pending.md`. Las filas `inconclusa` por "
            "contención o plazo se repiten hasta clasificarse.",
            "- Para ejecutar: `I07C_MUT_DIR=" + args.out.as_posix() + " python scripts/i07c_mutaciones.py --base "
            + base[:7] + " --run --resume --full-suite --mutation-profile both --only production "
            "--shards N --shard k --target-dir <target>`, en un clon del commit evaluado con la población idéntica "
            "(comprobar el SHA-256 anterior).",
        ]
        (out / "FREEZE.md").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
        print(f"{len(mutations)} sustituciones; population.json SHA-256 {sha(raw)}")
        return 0
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", str(tree)], cwd=ROOT, capture_output=True)
        shutil.rmtree(tmp_root, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
