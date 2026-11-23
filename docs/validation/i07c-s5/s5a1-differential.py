"""Compare S5a.1 candidate binaries against the base on a frozen corpus.

Sources: the frozen directed corpus (`s5a1-corpus.py`), the 27 twin cases and their
trailing-assignment permutations, the S5a repros, every `corpus/` and `tests/fixtures/**/*.n`
source and the 152 RD sources. Each source runs `check` and `run` under the three normative
profiles, debug and release, on the base and on the candidate.

Usage: python -B .../s5a1-differential.py <base-debug> <base-release> <cand-debug> <cand-release>
           <output.json> [start count]
"""

from concurrent.futures import ThreadPoolExecutor
import hashlib
import importlib.util
import itertools
import json
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
PROFILES = ("f0-0002-b-v2", "f0-0003-b-v1", "f0-0005-b-v1")
REPROS = {
    "A": "fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[3];}",
    "A_control": "fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));r=[[3]];}",
    "A_no_assignment": "fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then aar else [r]));}",
    "A_reordered": "fn Main(){let aar=[[1]];var r=[];let a=(if true then r else (if true then [r] else aar));r=[3];}",
    "A_twin": "fn Main(){let aar=[[1]];var r=[3];let a=(if true then r else (if true then aar else [r]));}",
    "B_qr": "fn Main(){let p=true;var r=[];var q=[];let a=(if p then p else q);let c=[q,[r]];q=[3];r=[3];}",
    "B_rq": "fn Main(){let p=true;var r=[];var q=[];let a=(if p then p else q);let c=[q,[r]];r=[3];q=[3];}",
    "F06": "fn Main(){let ar=[1];var r=[];let a0=(if true then r[1] else (if false then r else ar));r=[3];}",
    "M2": "fn Main(){let aar=[[1]];var r=[];let a=(if true then r[0] else (if true then r else aar));r=aar;}",
    "D_REF_valid": "fn Main(){let aar=[[1]];var r=[];let a=(if false then r[0] else aar);r=[[[3]]];}",
    "constructor_ok": "fn Main(){var r=[];r=[3];}",
    "constructor_recursive": "fn Main(){var r=[];r=[r];}",
    "protect_nested_union": "fn Main(){let p=true;var r=[];let a0=(if p then r else (if p then 1 else 2));r=[true];}",
    "protect_equality": "fn Main(){var r=[];var s=[];let a0=(r==[s]);let a1=(if r then {value:[true];} else {value:r;});r=[[3]];s=[3];}",
    "protect_len": "fn Main(){let x=1;var r=[];Print((if true then {value: Len(r[x]);} else {value: r[0];}));r=[[3]];}",
    "known_limit": "fn Main(){var r=[];let a=(if true then r else (if true then [true] else [1]));r=[1];}",
}


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def trailing_permutations(source):
    body = source[len("fn Main(){"):-1]
    depth, cuts = 0, []
    for index, char in enumerate(body):
        depth += char in "([{"
        depth -= char in ")]}"
        if char == ";" and depth == 0:
            cuts.append(index + 1)
    statements, end = [], len(body)
    for cut in reversed(cuts):
        if cut != end:
            continue
        start = max([c for c in cuts if c < cut], default=0)
        name, _, rest = body[start:cut].partition("=")
        if not (name.isalnum() and name.islower() and rest and rest[0] != "="):
            break
        statements.append(body[start:cut])
        end = start
        if len(statements) == 3:
            break
    if len(statements) < 2:
        return []
    head = "fn Main(){" + body[:end]
    return [head + "".join(order) + "}" for order in itertools.permutations(reversed(statements))]


def sources():
    corpus = load("s5a1_corpus", HERE / "s5a1-corpus.py")
    generated = corpus.generated()
    frozen = json.loads((HERE / "s5a1-corpus.json").read_text(encoding="utf-8"))
    assert frozen["seed"] == corpus.SEED and frozen["count"] == len(generated) == corpus.COUNT
    assert frozen["sha256"] == corpus.digest(generated)
    yield from ((f"generated:{i:04d}", source) for i, source in enumerate(generated))
    cases = json.loads((ROOT / "docs/validation/i07c-twin/casos.json").read_text(encoding="utf-8"))
    for case in cases:
        yield "twin:" + case["id"], case["reduced"]
        for n, variant in enumerate(trailing_permutations(case["reduced"])):
            yield f"twin:{case['id']}:perm{n}", variant
    yield from (("repro:" + name, source) for name, source in REPROS.items())
    for directory in (ROOT / "corpus", ROOT / "tests/fixtures"):
        for file in sorted(directory.rglob("*.n")):
            yield str(file.relative_to(ROOT)).replace("\\", "/"), file.read_text(encoding="utf-8")
    rd = load("i07c_rd", ROOT / "scripts/i07c_rd_conformidad.py")
    _, rows = rd.table()
    assert len(rows) == 152
    yield from (("rd:" + row["id"], row["source"]) for row in rows)


def call(binary, verb, profile, file):
    process = subprocess.run((str(binary), verb, str(file), "--format", "json", "--profile", profile),
                             capture_output=True, timeout=60)
    text = lambda data: data.decode("utf-8", errors="replace").replace(str(file), "case.n")
    return process.returncode, text(process.stdout), text(process.stderr)


def compare(task):
    identifier, source, binaries = task
    base_debug, base_release, current_debug, current_release = binaries
    with tempfile.TemporaryDirectory(prefix="s5a1-diff-") as directory:
        file = Path(directory) / "case.n"
        file.write_text(source, encoding="utf-8")
        found = []
        for profile in PROFILES:
            for verb in ("check", "run"):
                for mode, base, current in (("debug", base_debug, current_debug),
                                            ("release", base_release, current_release)):
                    old, new = call(base, verb, profile, file), call(current, verb, profile, file)
                    if old != new:
                        found.append({"source_id": identifier, "source": source, "profile": profile,
                                      "verb": verb, "mode": mode, "base": old, "candidate": new})
        return found


def main():
    if len(sys.argv) not in (6, 8):
        raise SystemExit(__doc__)
    binaries = tuple(Path(arg) for arg in sys.argv[1:5])
    output = Path(sys.argv[5])
    start, count = (int(sys.argv[6]), int(sys.argv[7])) if len(sys.argv) == 8 else (0, None)
    assert all(binary.is_file() for binary in binaries)
    identity = {name: hashlib.sha256(binary.read_bytes()).hexdigest()
                for name, binary in zip(("base_debug", "base_release", "cand_debug", "cand_release"), binaries)}
    tasks = [(identifier, source, binaries) for offset, (identifier, source) in enumerate(sources())
             if offset >= start and (count is None or offset < start + count)]
    differences = []
    with ThreadPoolExecutor(max_workers=4) as pool:
        for done, found in enumerate(pool.map(compare, tasks), 1):
            differences.extend(found)
            if done % 200 == 0:
                print(f"sources={done}/{len(tasks)} changes={len(differences)}", flush=True)
    totals = {"sources": len(tasks), "invocations": len(tasks) * 24, "changes": len(differences)}
    report = {"manifest": json.loads((HERE / "s5a1-corpus.json").read_text()),
              "range": [start, start + len(tasks)], "binary_sha256": identity, "totals": totals,
              "differences": differences}
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"saved={output} totals={totals}")


if __name__ == "__main__":
    main()
