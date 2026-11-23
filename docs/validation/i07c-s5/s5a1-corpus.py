"""Deterministic corpus for S5a.1 (seeded shapes, deferred unions, late assignments).

Directed at Array/Option/Result leaves, nested `if` unions with pending references,
projections, conditions over pending arrays, late assignments in every order and
their concrete twins. The manifest (seed, count, SHA-256) is frozen before any CLI use.

Templates use `@0`, `@1`, `@2` for the declared pending names and `#` for a union slot.

Usage: python -B docs/validation/i07c-s5/s5a1-corpus.py --freeze
       python -B docs/validation/i07c-s5/s5a1-corpus.py --verify
"""

import argparse
import hashlib
import json
from pathlib import Path
import random


HERE = Path(__file__).resolve().parent
SEED = 20261122
COUNT = 3200
PREFIX = "fn Main(){let x=1;let p=true;let ar=[1];let aar=[[1]];"
PENDING = ("r", "q", "s")
CONDS = ("true", "p", "(x<1)", "x", "[@0][0]", "false[x]")
LEAVES = ("@0", "@1", "@2", "ar", "aar", "[@0]", "[@1]", "@0[0]", "@0[x]", "@1[1]", "1", "true",
          "[1]", "[true]", "[[2]]", "[@0,@1]", "@0", "@1", "[@0]", "[[@0]]")
# Option and Result leaves exist only in the f0-0003 and f0-0005 profiles; they are a minority.
OPTION_RESULT = ("Some(1)", "None", "Ok(1)", "Err(true)", "[Some(1)]", "Some(@0)", "[Ok(1)]")
ASSIGN = ("[3]", "[[3]]", "[x]", "aar", "ar", "[true]", "[[true]]", "[@0]", "[@1]", "[]", "[3]", "[[3]]",
          "[Some(2)]", "None")
WRAPS = ("let a=#;", "let a=[#,#];", "let a=(#==ar);", "let a=(1+#);", "Print(#);",
         "let a={value:#;};", "let a=@0[#];", "let a=Len(#);", "let a=(#==[@2]);",
         "while # {}", "let a=(if p then # else #);")


def fill(text, names):
    for index in range(3):
        text = text.replace(f"@{index}", names[index % len(names)])
    return text


def union(rng, depth):
    if depth == 0 or rng.random() < 0.3:
        return rng.choice(OPTION_RESULT if rng.random() < 0.1 else LEAVES)
    return f"(if {rng.choice(CONDS)} then {union(rng, depth - 1)} else {union(rng, depth - 1)})"


def use(rng):
    template = rng.choice(WRAPS)
    while "#" in template:
        template = template.replace("#", union(rng, rng.choice((1, 2, 3))), 1)
    return template


def program(rng):
    names = PENDING[: rng.choice((1, 2, 3))]
    decls = "".join(f"var {name}=[];" for name in names)
    uses = "".join(use(rng) for _ in range(rng.choice((1, 1, 2))))
    assignments = [f"{rng.choice(names)}={rng.choice(ASSIGN)};"
                   for _ in range(rng.choice((1, 1, 2, 3)))]
    if rng.random() < 0.25:
        # concrete twin: the first late assignment becomes the initializer
        name, rhs = assignments[0][:-1].split("=", 1)
        others = "".join(f"var {other}=[];" for other in names if other != name)
        return fill(f"{PREFIX}let {name}={fill(rhs, names)};{others}{uses}" + "}", names)
    return fill(PREFIX + decls + uses + "".join(assignments) + "}", names)


def generated():
    rng = random.Random(SEED)
    seen, cases = set(), []
    while len(cases) < COUNT:
        source = program(rng)
        if source not in seen:
            seen.add(source)
            cases.append(source)
    return cases


def digest(cases):
    return hashlib.sha256(b"".join(
        len(source.encode("ascii")).to_bytes(4, "big") + source.encode("ascii")
        for source in cases
    )).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--freeze", action="store_true")
    mode.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    cases = generated()
    assert len(cases) == COUNT and len(set(cases)) == COUNT
    metadata = {"seed": SEED, "count": len(cases), "sha256": digest(cases),
                "encoding": "length-prefixed ASCII, four-byte big-endian length, generation order"}
    manifest = HERE / "s5a1-corpus.json"
    if args.freeze:
        if manifest.exists():
            raise SystemExit("manifest already frozen; refusing to overwrite")
        manifest.write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    else:
        assert json.loads(manifest.read_text(encoding="utf-8")) == metadata
    print(json.dumps(metadata, sort_keys=True))


if __name__ == "__main__":
    main()
