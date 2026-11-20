#!/usr/bin/env python3
"""Clasifica un parche en copia aislada mediante la suite default íntegra.

El JSON del parche contiene file, before, after, occurrence, sha_initial y
sha_mutated. La fuente se restaura byte a byte, aun si falla Cargo.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time


def digest(data):
    return hashlib.sha256(data).hexdigest()


def tree_digest(tree):
    total = hashlib.sha256()
    for path in sorted(p for p in tree.rglob("*") if p.is_file() and "target" not in p.parts):
        total.update(path.relative_to(tree).as_posix().encode("utf-8") + b"\0")
        total.update(bytes.fromhex(digest(path.read_bytes())))
    return total.hexdigest()


def command(argv, cwd, env, timeout):
    start = time.monotonic()
    process = subprocess.Popen(argv, cwd=cwd, env=env, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, encoding="utf-8",
                               errors="replace")
    try:
        out, err = process.communicate(timeout=timeout)
        return {"command": argv, "exit": process.returncode, "stdout": out,
                "stderr": err, "seconds": round(time.monotonic() - start, 3),
                "timeout": False}
    except subprocess.TimeoutExpired:
        if os.name == "nt":
            subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                           capture_output=True, check=False)
        else:
            process.kill()
        out, err = process.communicate()
        return {"command": argv, "exit": process.returncode, "stdout": out,
                "stderr": err, "seconds": round(time.monotonic() - start, 3),
                "timeout": True}


def events(stdout):
    parsed = []
    for line in stdout.splitlines():
        if not line.startswith("{"):
            continue
        try:
            parsed.append(json.loads(line))
        except json.JSONDecodeError:
            return None
    return parsed


def classify(result):
    if result["timeout"]:
        return "INCONCLUSO", None, "timeout"
    parsed = events(result["stdout"])
    if parsed is None:
        return "INCONCLUSO", None, "JSON truncado"
    failed = [event for event in parsed if event.get("type") == "test"
              and event.get("event") in ("failed", "timeout")]
    if failed:
        event = failed[0]
        if event["event"] == "timeout" or "time limit" in event.get("reason", "").lower():
            return "INCONCLUSO", event.get("name"), "timeout de test"
        return "MUERTO", event.get("name"), "test fallido"
    if result["exit"] != 0:
        if not parsed and re.search(r"error: could not compile|error\[E\d+\]|error: linking with", result["stderr"]):
            return "INVÁLIDO", None, "compilación fallida"
        return "INCONCLUSO", None, "salida sin test terminal"
    if not parsed or not any(e.get("type") == "suite" and e.get("event") == "ok" for e in parsed):
        return "INCONCLUSO", None, "resumen JSON ausente"
    if not re.search(r"Summary\s+\[.*\]\s+\d+ tests? run: \d+ passed", result["stderr"]):
        return "INCONCLUSO", None, "resumen nextest ausente"
    return "VERDE", None, "suite completa"


def replace_nth(source, before, after, occurrence):
    positions = [m.start() for m in re.finditer(re.escape(before), source)]
    if occurrence >= len(positions):
        raise ValueError("ancla ausente")
    at = positions[occurrence]
    return source[:at] + after + source[at + len(before):]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tree", type=Path, required=True)
    parser.add_argument("--patch", type=Path, required=True)
    parser.add_argument("--id", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=2400)
    args = parser.parse_args()
    tree = args.tree.resolve(strict=True)
    allowed = [Path('C:/nexo-work/tmp').resolve(), Path('C:/nexo-work/val').resolve()]
    if not any(tree.is_relative_to(root) and tree != root for root in allowed):
        parser.error('--tree debe ser una copia bajo C:/nexo-work/tmp o C:/nexo-work/val')
    patch = json.loads(args.patch.read_text(encoding="utf-8"))
    relative = Path(patch["file"])
    if relative.is_absolute() or ".." in relative.parts:
        parser.error("file debe ser relativo al árbol")
    source_path = (tree / relative).resolve(strict=True)
    if tree not in source_path.parents:
        parser.error("file sale del árbol")
    original = source_path.read_bytes()
    if digest(original) != patch["sha_initial"]:
        parser.error("sha_initial no coincide")
    mutated = replace_nth(original.decode("utf-8"), patch["before"],
                          patch["after"], patch.get("occurrence", 0)).encode("utf-8")
    if digest(mutated) != patch["sha_mutated"]:
        parser.error("sha_mutated no coincide")
    env = os.environ.copy()
    env["CARGO_BUILD_JOBS"] = "6"
    env["NEXTEST_EXPERIMENTAL_LIBTEST_JSON"] = "1"
    env["NEXTEST_PROFILE"] = "default"
    row = {"id": args.id, "tree": str(tree), "file": str(relative),
           "sha_initial": digest(original), "sha_mutated": digest(mutated),
           "selection": "default completo; sin -E; retries 0; fail-fast",
           "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
           "nextest": subprocess.check_output(["cargo", "nextest", "--version"], text=True).splitlines()[0],
           "profiles": []}
    source_path.write_bytes(mutated)
    try:
        row["mutated_tree_sha256"] = tree_digest(tree)
        for profile in ("debug", "release"):
            argv = ["cargo", "nextest", "run", "--locked", "--offline", "--profile", "default",
                    "--retries", "0", "--message-format", "libtest-json"]
            if profile == "release":
                argv.append("--release")
            result = command(argv, tree, env, args.timeout)
            log_path = args.output.with_suffix(f".{profile}.log")
            log_path.parent.mkdir(parents=True, exist_ok=True)
            log_path.write_text("COMMAND: " + " ".join(argv) + "\nSTDOUT:\n" +
                                result["stdout"] + "\nSTDERR:\n" + result["stderr"],
                                encoding="utf-8")
            verdict, test, reason = classify(result)
            row["profiles"].append({"profile": profile, "exit": result["exit"],
                                    "seconds": result["seconds"], "verdict": verdict,
                                    "test": test, "reason": reason,
                                    "log": str(log_path),
                                    "stdout_sha256": digest(result["stdout"].encode()),
                                    "stderr_sha256": digest(result["stderr"].encode())})
            if verdict == "MUERTO":
                test_name = test.split('$', 1)[-1]
                isolated = argv + ['-E', 'test(=' + test_name + ')']
                again = command(isolated, tree, env, args.timeout)
                again_verdict, again_test, _ = classify(again)
                source_path.write_bytes(original)
                baseline = command(isolated, tree, env, args.timeout)
                base_verdict, _, _ = classify(baseline)
                source_path.write_bytes(mutated)
                row['profiles'][-1]['confirmation'] = {
                    'selection': isolated, 'mutated_verdict': again_verdict,
                    'mutated_test': again_test, 'mutated_exit': again['exit'],
                    'baseline_verdict': base_verdict, 'baseline_exit': baseline['exit'],
                    'mutated_stdout_sha256': digest(again['stdout'].encode()),
                    'baseline_stdout_sha256': digest(baseline['stdout'].encode())}
                if again_verdict != 'MUERTO' or again_test != test or base_verdict != 'VERDE':
                    verdict = 'INCONCLUSO'
                    row['profiles'][-1]['verdict'] = verdict
                    row['profiles'][-1]['reason'] = 'fallo no reproducido aisladamente en mutante y base'
            if verdict != "VERDE":
                row["verdict"] = verdict
                break
        else:
            row["verdict"] = "SUPERVIVIENTE"
    finally:
        source_path.write_bytes(original)
        row["sha_restored"] = digest(source_path.read_bytes())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(row, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(row, ensure_ascii=False))


if __name__ == "__main__":
    main()
