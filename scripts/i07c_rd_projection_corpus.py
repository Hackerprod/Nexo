#!/usr/bin/env python3
"""Freeze independent pre-union projections for the I07c redesign.

The ``b`` recipe is transcribed from B's b-repro/fuzz.py (2026-11-24).
The ``owner`` recipe adds separate prior statements and array siblings.
This generator writes sources only; expected diagnostics come from the
independent I07c oracle and are never produced by the candidate binary.
"""

import argparse
import hashlib
import json
import random
from pathlib import Path

import i07c_rd_oracle as oracle

LEAVES = ["q", "r", "s", "[q]", "[r]", "[q,r]", "1", "true", "ar", "aar", "q[0]", "r[0]", "s[0]"]
ASSIGNMENTS = ["q=[[ar]]", "q=[1]", "q=[true]", "q=r", "r=q", "r=[[ar]]", "r=[true]", "s=q", "q=ar", "r=[3]", "s=[[true]]", "s=r", "r=s", "q=[[1<2]]"]


def union(rng, depth, failed_condition):
    if depth == 0 or rng.random() < 0.3:
        return rng.choice(LEAVES)
    condition = rng.choice(["p"] * 6 + ["1", "ar", "q"]) if failed_condition else "p"
    return f"(if {condition} then {union(rng, depth - 1, failed_condition)} else {union(rng, depth - 1, failed_condition)})"


def generate_b(rng, failed_condition):
    pre = ["let p=true;", "let ar=[1];", "let aar=[[1]];", "var q=[];", "var r=[];", "var s=[];"]
    body = []
    for k in range(rng.choice([0, 0, 1, 2])):
        value = rng.choice("qrs")
        body.append(rng.choice([f"let b{k}={value}[0];", f"let b{k}=(if p then {value}[0] else {value}[0]);", f"let b{k}=({value}[0]);"]))
    value = union(rng, rng.choice([2, 3]), failed_condition)
    form = rng.choice(["let a0={};", "{};", "let a0=[{}];", "let a0=[q,{}];", "let a0=[{},r];"])
    body.append(form.format(value))
    if rng.random() < 0.4:
        body.append("let a1=" + union(rng, 2, failed_condition) + ";")
    body.extend(assignment + ";" for assignment in rng.sample(ASSIGNMENTS, rng.choice([0, 1, 2, 2, 3])))
    return "fn Main(){" + "".join(pre + body) + "}"


def generate_owner(rng, failed_condition):
    pre = "let p=true;let ar=[1];let aar=[[1]];var q=[];var r=[];var s=[];"
    projected = rng.choice("qrs")
    prior = rng.choice([
        f"let prior={projected}[0];",
        f"let prior=(if p then {projected}[0] else {projected}[0]);",
        f"let prior=[{projected}[0],1];",
    ])
    if rng.random() < 0.5:
        expression = "(if p then [q] else (if p then [1] else q))"
    else:
        expression = "(if p then r[0] else (if p then q else r))"
    if failed_condition and rng.random() < 0.35:
        expression = expression.replace("if p then", "if 0 then", 1)
    assignments = rng.choice(["r=q;q=[1];", "r=q;r=[true];", "q=[1];r=q;", "r=[true];r=q;"])
    return "fn Main(){" + pre + prior + expression + ";" + assignments + "}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("recipe", choices=["b", "owner"])
    parser.add_argument("seed", type=int)
    parser.add_argument("count", type=int)
    parser.add_argument("output", type=Path)
    parser.add_argument("--failed-condition", action="store_true")
    args = parser.parse_args()
    rng = random.Random(args.seed)
    rows = []
    seen = set()
    generator = generate_b if args.recipe == "b" else generate_owner
    attempts = 0
    while len(rows) < args.count:
        attempts += 1
        if attempts > args.count * 200:
            raise RuntimeError("too many unmodelable or duplicate sources")
        source = generator(rng, args.failed_condition)
        if source in seen:
            continue
        seen.add(source)
        try:
            derived = oracle.derive(source)
        except Exception:
            continue
        if derived is None or derived.get("unmodeled") or derived.get("error"):
            continue
        rows.append(source)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    content = json.dumps(rows, ensure_ascii=False, separators=(",", ":"), indent=None).encode("utf-8")
    args.output.write_bytes(content)
    print(json.dumps({"recipe": args.recipe, "seed": args.seed, "n": len(rows), "sha256": hashlib.sha256(content).hexdigest(), "path": str(args.output)}))


if __name__ == "__main__":
    main()
