"""Frozen differential corpus for I07c-S6-COR-2 (expression primaries and tick origins under groups).

Deterministic: `python -B gen_corpus.py <out_dir> [--seed N]` writes the sources and prints the
count, the seed and the SHA-256 of the manifest (name + SHA-256 of every source, in name order).
Families: (a) the tick-origin and guard sites of the cut with pad 0..11 and wrappers none / ( ) /
(( )) / ( e ) over loops and over bare blocks; (b) quota-boundary programs with grouped call
targets; (c) wrappers around other expression positions (if/else-if/match/return/index/operators/
arguments/let/for) so that sites the cut does not touch are in the population too; (d) seeded random
compositions of statements with random wrappers; (e) the repository files (`corpus/`,
`tests/fixtures/`) and the 152 RD rows, with `--repo <root>`.
"""

import argparse
import hashlib
import json
import random
from pathlib import Path

PRE = "fn Id(x:view I64)->I64{return x;}fn F(x:view I64){}fn G(x:view I64)->I64{return @R;}"
WRAPS = [("", ""), ("(", ")"), ("((", "))"), ("( ", " )")]
INNERS_EXPR = ["n+1", "n", "1", "true", "Id(n)", "Id(1)", "AssertEq(1,1)", "fn (){}", "()", "[1]", "n<2", "[n,1]"]

# (family, loop-body template with @E; inners).  `@L` is the loop head, `@T` its tail.
STMT = [
    ("expr_statement", "@E;", INNERS_EXPR),
    ("assignment", "@E=1;", ["n"]),
    ("compound_assignment", "@E+=1;", ["n"]),
    ("assign_value", "n=@E;", ["n+1", "n", "1", "Id(n)"]),
    ("let_initializer", "let y=@E;", ["n", "n+1", "Id(n)", "1", "true", "[n]"]),
    ("branch_then", "let y=if true then @E else 2;", ["n", "n+1", "1", "Id(n)"]),
    ("branch_else", "let y=if false then 2 else @E;", ["n", "n+1", "1", "Id(n)"]),
    ("if_condition", "if @E then {n=n+1;}", ["n<5", "true", "n==0"]),
    ("else_if_condition", "if false then {} else if @E then {n=n+1;}", ["n<5", "true", "n==0"]),
    ("return_value", "n=G(n);", ["n"]),
    ("operator_left", "let y=@E+1;", ["n", "n*2", "Id(n)"]),
    ("operator_right", "let y=1+@E;", ["n", "n*2", "Id(n)"]),
    ("argument", "Id(@E);", ["n", "n+1", "1"]),
    ("argument_view", "F(@E);", ["n", "n+1", "1"]),
    ("index_target", "let y=@E[0];", ["a"]),
    ("index_value", "let y=a[@E];", ["0", "n"]),
    ("assign_index_target", "@E[0]=3;", ["a"]),
    ("assign_index_place", "@E=3;", ["a[0]", "a[n]"]),
    ("direct_call_target", "@E(1);", ["Id"]),
    ("match_scrutinee", "match @E{true=>{},_=>{}}", ["true", "n==0"]),
    ("match_guard_mutation", "match true{true if (if true then {@E=2;value:true;} else {value:false;}) =>{},_=>{}}", ["x"]),
    ("match_guard_transfer", "match true{true if (if true then {@E(a);value:true;} else {value:false;}) =>{},_=>{}}", ["Drop", "F"]),
    ("match_guard_statement", "match true{true if (if true then {@E;value:true;} else {value:false;}) =>{},_=>{}}", ["x", "x+1"]),
    ("match_guard_let", "match true{true if (if true then {let z=@E;value:true;} else {value:false;}) =>{},_=>{}}", ["x"]),
    ("match_guard_value", "match true{true if (if true then {value:@E;} else {value:false;}) =>{},_=>{}}", ["true"]),
    ("for_iterable", "for q in @E {}", ["[1]", "[1,2]", "a"]),
]


def program(pad: int, body: str, loop: bool, r: str = "x", drop: bool = False) -> str:
    decls = "var n=0;var x=1;var a=[1,2];"
    if drop:
        decls = "var n=0;var x=1;let a=Some(1);"
    inner = body
    if loop:
        inner = "while true{" + body + "}"
    else:
        inner = "{" + body + "}"
    return PRE.replace("@R", r) + "fn Main(){" + "{}" * pad + decls + inner + "}"


def wrapped(template: str, inner: str, wrap) -> str:
    return template.replace("@E", wrap[0] + inner + wrap[1])


def generate(out: Path, seed: int, repo: Path | None = None) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    sources: dict[str, bytes] = {}

    def add(name: str, text) -> None:
        sources[name] = text if isinstance(text, bytes) else text.encode("utf-8")

    # (a) + (c): statement families, loop and once, every wrapper, pads 0..11 (loop) / 0,1 (once)
    for family, template, inners in STMT:
        for inner in inners:
            for wi, wrap in enumerate(WRAPS):
                body = wrapped(template, inner, wrap)
                drop = family == "match_guard_transfer"
                for pad in range(12):
                    ret = "x" if family != "return_value" else wrapped("@E", "x", wrap)
                    add(f"a_{family}_{inner_id(inner)}_w{wi}_p{pad}_loop",
                        program(pad, body, True, ret, drop))
                for pad in (0, 1):
                    ret = "x" if family != "return_value" else wrapped("@E", "x", wrap)
                    add(f"a_{family}_{inner_id(inner)}_w{wi}_p{pad}_once",
                        program(pad, body, False, ret, drop))
    # while-condition family
    for inner in ["n<9999999", "true", "n<3", "n==0", "n+1<2"]:
        for wi, wrap in enumerate(WRAPS):
            for pad in range(12):
                add(f"a_while_condition_{inner_id(inner)}_w{wi}_p{pad}",
                    PRE.replace("@R", "x") + "fn Main(){" + "{}" * pad + "var n=0;while "
                    + wrapped("@E", inner, wrap) + "{}}")
                add(f"a_while_condition_body_{inner_id(inner)}_w{wi}_p{pad}",
                    PRE.replace("@R", "x") + "fn Main(){" + "{}" * pad + "var n=0;while "
                    + wrapped("@E", inner, wrap) + "{n=n+1;}}")
    # (b) quota boundaries with grouped call targets
    bounds = [1, 2, 5, 100, 1000, 20000, 43476, 43477, 43478, 43479, 45452, 45453, 45454, 47617, 47618, 47619, 52630, 52631]
    for bound in bounds:
        for wi, wrap in enumerate(WRAPS):
            add(f"b_call_id_n{bound}_w{wi}",
                PRE.replace("@R", "x") + "fn Main(){var n=0;while n<" + str(bound) + "{n=n+1;" + wrapped("@E", "Id", wrap) + "(1);}}")
            add(f"b_call_closure_n{bound}_w{wi}",
                "fn Main(){var n=0;let f=fn (){};while n<" + str(bound) + "{n=n+1;" + wrapped("@E", "f", wrap) + "();}}")
            add(f"b_call_two_n{bound}_w{wi}",
                PRE.replace("@R", "x") + "fn Main(){var n=0;while n<" + str(bound) + "{n=n+1;" + wrapped("@E", "Id", wrap) + "(" + wrapped("@E", "Id", wrap) + "(1));}}")
    # (d) seeded random compositions
    rng = random.Random(seed)
    pool = [(t, i) for (_f, t, inn) in STMT for i in inn if "match_guard" not in _f and _f != "return_value"]
    for k in range(700):
        picks = [rng.choice(pool) for _ in range(rng.randint(1, 4))]
        parts = []
        for template, inner in picks:
            depth = rng.choice([0, 0, 1, 1, 2, 3])
            text = inner
            for _ in range(depth):
                opener, closer = rng.choice([("(", ")"), ("( ", " )")])
                text = opener + text + closer
            parts.append(template.replace("@E", text))
        loop = rng.random() < 0.7
        add(f"d_random_{k:04d}", program(rng.randint(0, 11), "".join(parts), loop, "x"))
    if repo is not None:
        for path in sorted(list((repo / "corpus").rglob("*.n")) + list((repo / "tests/fixtures").rglob("*.n"))):
            flat = path.relative_to(repo).as_posix().replace("/", "__")[:-2]
            add("e_repo_" + flat, path.read_bytes())
        rows = json.loads((repo / "tests/fixtures/i07c_rd.json").read_text(encoding="utf-8"))["rows"]
        for row in rows:
            add("f_rd_" + row["id"], row["source"])
    for name, text in sources.items():
        (out / (name + ".n")).write_bytes(text)
    manifest = "\n".join(
        f"{hashlib.sha256(text).hexdigest()}  {name}.n" for name, text in sorted(sources.items())
    ) + "\n"
    (out / "MANIFEST.txt").write_text(manifest, encoding="ascii", newline="\n")
    return {
        "count": len(sources),
        "seed": seed,
        "manifest_sha256": hashlib.sha256(manifest.encode("ascii")).hexdigest(),
    }


def inner_id(inner: str) -> str:
    return "".join(c if c.isalnum() else "_" for c in inner) + "_" + hashlib.sha256(inner.encode()).hexdigest()[:6]


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("out")
    parser.add_argument("--seed", type=int, default=20261122)
    parser.add_argument("--repo", default=None, help="repository root for the (e) family")
    args = parser.parse_args()
    print(json.dumps(generate(Path(args.out), args.seed, Path(args.repo) if args.repo else None)))
