"""Deterministic generator of programs aimed at the primary of an expression under groups.

I07c-S6-COR differential input. Every template has holes written @{expression}@; each hole is
wrapped in 0..3 outer groups, drawn from a seeded generator, so the same expression is observed
bare, grouped and nested in every family of the cut (types, names and arguments, ownership,
lowering barriers, effects, guards, generators, selectors and ranges). Some programs are
deliberately invalid or runtime-faulting: the differential only compares binaries.

Usage: python -B generate.py --seed 20261122 --count 3600 --out corpus.jsonl
The output is one JSON object per line: {"id": n, "source": "..."}; the SHA-256 of the
concatenated sources (separated by NUL) is printed so the list can be frozen.
"""
import argparse
import hashlib
import json
import random
import re

TEMPLATES = [
    # for / ranges / iteration
    "fn Main(){for x in @{[1]}@{}}",
    "fn Main(){for x in @{[1,2]}@{Print(x);}}",
    "fn Main(){let a=[1];for x in @{a}@{}}",
    "fn Main(){for x in @{1}@{}}",
    "fn Main(){for i in @{0}@..<@{3}@{}}",
    "fn Main(){for i in @{1}@..<@{true}@{}}",
    "fn Main(){var a=[1];for x in @{a}@{a=[2];}}",
    "fn Main(){var a=[1];match edit a{_=>{for x in @{a}@{}}}}",
    "fn Main(){let a=[MockOpen(1)];for x in @{a}@{}}",
    "fn Main(){let r=@{0}@..<@{3}@;}",
    # non-callable / call binding
    "fn Main(){let x=1;@{x}@(@{2}@);}",
    "fn Main(){@{None}@(@{2}@);}",
    "fn F(x:view I64){}fn Main(){@{F}@();}",
    "fn F(x:view I64){}fn Main(){@{F}@(@{1}@,@{2}@);}",
    "fn F(x:view I64){}fn Main(){@{F}@(@{1}@);}",
    "fn F(x:view I64,y:view I64){}fn Main(){F(x:1,@{2}@);}",
    "fn F(x:view I64,y:view I64){}fn Main(){F(x:@{1}@,x:@{2}@);}",
    "fn F(x:view I64,y:view I64){}fn Main(){F(y:@{1}@,x:@{2}@);}",
    "fn F(x:edit I64){}fn Main(){F(@{1}@);}",
    "fn F(x:edit I64){}fn Main(){var a=1;F(edit @{a}@);}",
    "fn F(x:edit I64){}fn Main(){var a=1;F(@{a}@);}",
    "fn F(x:view I64){}fn Main(){let a=1;F(edit @{a}@);}",
    "fn F(x:move I64){}fn Main(){let a=1;F(move @{a}@);}",
    "fn Main(){var a=[1];Drop(edit @{a}@);}",
    "fn Main(){var a=[1];Print(edit @{a[0]}@);}",
    "fn Main(){let f=fn (x:view I64){};@{f}@();}",
    "fn Main(){let f=fn (x:view I64){};@{f}@(@{1}@);}",
    "fn Main(){let f=fn (x:view I64){};@{f}@(@{1}@,@{2}@);}",
    "fn Main(){let f=fn (x:view I64){};@{f}@(x:@{1}@,@{2}@);}",
    "fn Main(){let f=fn (x:edit I64){};var a=1;@{f}@(@{a}@);}",
    # partial transfers / consumption
    "fn Main(){let x=[1];Drop(@{x[0]}@);}",
    "fn Main(){let x=[1];Drop(@{x}@);Drop(@{x}@);}",
    "fn Main(){let a=[1];move @{a[0]}@;}",
    "fn Main(){let a=Some(1);let b=move @{a}@;Drop(@{a}@);}",
    "fn Main(){let a=Some(1);Drop(a);match @{a}@{_=>{}}}",
    "fn Main(){let a=Some(1);Drop(a);match move @{a}@{_=>{}}}",
    "fn Main(){let a=Some(1);Drop(a);let b=move @{a}@;}",
    "fn Main(){let a=[1];match move @{a[0]}@{_=>{}}}",
    "fn Main(){let a=[1];Drop(@{a}@);Print(Len(@{a}@));}",
    "fn Main(){let a=[1];let b=a;Drop(@{a}@);}",
    "fn Main(){let r=MockOpen(1);Drop(@{r}@);Drop(@{r}@);}",
    "fn Main(){let r=MockOpen(1);let s=@{r}@;Drop(s);}",
    # match selection modes and borrow conflicts
    "fn Main(){let a=Some(1);match edit @{a}@{_=>{}}}",
    "fn Main(){var a=Some(1);match edit @{a}@{_=>{}}}",
    "fn Main(){var a=Some(1);match edit @{a}@{Some(x)=>{Drop(@{a}@);},None=>{}}}",
    "fn Main(){var a=Some(1);match edit a{Some(x)=>{match @{a}@{_=>{}}},None=>{}}}",
    "fn Main(){var a=[1];match edit a{_=>{Print(Len(@{a}@));}}}",
    "fn F(x:edit I64,y:view I64){}fn Main(){var a=1;F(edit a,@{a}@);}",
    "fn F(x:edit I64,y:view I64){}fn Main(){var a=[1];F(edit a[0],@{a[0]}@);}",
    "fn F(x:edit I64,y:edit I64){}fn Main(){var a=1;F(edit a,edit @{a}@);}",
    "fn F(a:view I64){@{a}@=2;}fn Main(){F(1);}",
    "fn F(a:view I64){Drop(@{a}@);}fn Main(){F(1);}",
    "fn F(a:view I64){match edit @{a}@{_=>{}}}fn Main(){F(1);}",
    "fn F(a:view I64){let f=fn [edit a](){};}fn Main(){F(1);}",
    # closures and escaping values
    "fn Main(){let f=if true then {let x=1;value:@{fn [view x]()->I64{return x;}}@;}else{return;};Drop(f);}",
    "fn Main(){let f=if true then {let x=1;value:@{x}@;}else{return;};}",
    "fn Main(){var x=1;var f=fn [edit x](){x=2;};@{f}@();}",
    "fn Main(){var x=1;var f=fn [edit x](){x=2;};edit @{f}@();}",
    "fn Main(){var x=1;var f=fn [edit x](){x=2;};move @{f}@();}",
    "fn Main(){var a=1;let f=fn [view a](){};@{f}@();Drop(a);}",
    "fn Main(){var a=[1];let f=fn [edit a](){};@{f}@();Print(Len(@{a}@));}",
    "fn Main(){let a=[1];let f=fn [move a](){Drop(a);};@{f}@();@{f}@();}",
    "fn Main(){let a=Some(1);let f=fn [move a](){};@{f}@();Drop(@{a}@);}",
    "fn Main(){let f=fn (){};@{f}@();@{f}@();}",
    # assignment places and lowering barriers
    "fn Main(){let a=1;@{a}@=2;}",
    "fn Main(){let a=[1];@{a}@[0]=2;}",
    "fn Main(){let a=[1];@{a[0]}@=2;}",
    "fn Main(){var a=1;@{a}@=@{a}@+1;}",
    "fn Main(){var a=1;@{a}@+=2;}",
    "fn Main(){var a=[1];@{a[0]}@-=2;}",
    "fn Main(){var a=1;@{a}@*=@{2}@;}",
    "fn Main(){var a=[1];@{a}@[0]+=1;}",
    "fn Main(){var a=1;let b=-@{a}@;}",
    "fn Main(){let b=not @{true}@;}",
    "fn Main(){var a=1;@{a}@/=@{2}@;Print(@{a}@);}",
    # effects
    "fn Main() with no_io{@{Print}@(1);}",
    "fn F(){Print(1);}fn Main() with no_io{@{F}@();}",
    "fn G(){Print(1);}fn Main() with no_io{let f=fn (){G();};@{f}@();}",
    "fn G(){Print(1);}fn H(){@{G}@();}fn Main() with no_io{@{H}@();}",
    "fn Main() with no_alloc{let a=@{[1]}@;}",
    "fn Main() with no_alloc{@{Len}@(@{[1]}@);}",
    "fn Main() with no_alloc{let r=@{MockOpen}@(1);}",
    "fn Main() with no_io{let r=@{MockOpen}@(1);Drop(@{r}@);}",
    "fn Main() with no_io{Drop(@{MockOpen}@(1));}",
    "fn Main() with no_io{let f=fn (){Print(1);};@{f}@();@{f}@();}",
    "fn F(){Print(1);}fn Main() with no_io{@{F}@();@{F}@();}",
    "fn Main() with no_suspend{@{Print}@(1);}",
    "gen fn G()->I64{Print(1);yield 1;}fn Main() with no_io{var g=G();@{Iterator.Next}@(edit g);}",
    "gen fn G()->I64{Print(1);yield 1;}fn Main() with no_io{for x in @{G()}@{}}",
    "gen fn G()->I64{Print(1);yield 1;}fn Main() with no_io{for x in @{G}@(){}}",
    "fn Main(){match true{true if @{MockUnknown}@()==()=>{},_=>{}}}",
    "fn G()->Bool{let u=MockUnknown();return true;}fn Main(){match true{true if @{G}@()=>{},_=>{}}}",
    "fn Main(){match @{1}@{1 if @{Print}@(1)==()=>{},_=>{}}}",
    # types: operands, annotations, selectors
    "fn Main(){let x:Bool=@{1}@;}",
    "fn Main(){var v=1;v=2;let x:Bool=@{v}@;}",
    "fn Main(){let x=@{true}@+@{1}@;}",
    "fn Main(){let x=[1,@{true}@];}",
    "fn Main(){let a=[1];let b=a[@{true}@];}",
    "fn Main(){let a=[1];let b=a[@{0}@];}",
    "fn Main(){let a=[1];let b=@{a}@@{[true]}@;}",
    "fn Main(){let a=[1];let b=@{a}@[0][0];}",
    "fn Main(){let x=@{1}@==@{true}@;}",
    "fn Main(){if @{1}@{}}",
    "fn Main(){while @{1}@{}}",
    "fn Main(){let x=@{Len}@(@{1}@);}",
    "fn Main(){let r=[];let x=@{r}@;}",
    "fn Main(){let x:I64=@{Some(1)}@;}",
    "fn Main(){let x=Some(@{true}@);let y:Option<I64>=x;}",
    "fn Main(){let x=@{Ok(1)}@;}",
    "fn Main(){match @{1}@{true=>{},_=>{}}}",
    "fn Main(){match @{Some(1)}@{Some(x)=>{},Some(y)=>{},_=>{}}}",
    "fn Main(){let a=[1];Push(edit @{a}@,@{true}@);}",
    "fn Main(){var a=[1];Push(edit @{a}@,@{true}@);}",
    "fn Main(){var a=[1];Push(edit a,@{2}@);let z=Pop(edit @{a}@);}",
    "fn Main(){AssertEq(@{1}@,@{true}@);}",
    "fn Main(){AssertEq(@{1}@,@{1}@);}",
    "fn Main(){let x=TryAdd(@{1}@,@{true}@);}",
    "fn Main(){let x=@{1}@/@{0}@;}",
    "fn Main(){let a=[1];let i=@{5}@;Print(a[@{i}@]);}",
    "fn Main(){let a=[1,2];Print(a[@{1}@]+@{2}@);}",
    # accepted programs: grouped operands must not change the run
    "fn F()->I64{return @{1}@;}fn Main(){Print(F());}",
    "fn F(x:view I64)->I64{return @{x}@;}fn Main(){Print(@{F}@(@{1}@));}",
    "fn Main(){var a=1;a=@{2}@;Print(@{a}@);}",
    "fn Main(){let x=if @{true}@ then {value:1;}else{value:2;};Print(x);}",
    "fn Main(){let a=@{[1,2,3]}@;Print(Len(@{a}@));}",
    "fn Main(){var a=[1];Push(edit @{a}@,@{2}@);Print(Len(a));}",
    "fn Main(){let f=fn (x:view I64){Print(x);};@{f}@(@{1}@);}",
    "fn Main(){let o=@{Some(1)}@;match @{o}@{Some(x)=>{Print(x);},None=>{}}}",
    "fn Main(){let r=@{Ok(1)}@;Print(1);}",
]

PREFIXES = ["", "", "// ñé utf-8\n", "// 中文\n", "// \U0001f600 emoji\n"]
WEIGHTS = [(0, 15), (1, 35), (2, 30), (3, 20)]


def depth(rng):
    total = sum(w for _, w in WEIGHTS)
    pick = rng.randrange(total)
    for value, weight in WEIGHTS:
        if pick < weight:
            return value
        pick -= weight
    raise AssertionError


def expand(template, rng):
    def hole(match):
        n = depth(rng)
        return "(" * n + match.group(1) + ")" * n

    return re.sub(r"@\{(.+?)\}@", hole, template)


def generate(seed, count):
    rng = random.Random(seed)
    seen = set()
    programs = []
    attempts = 0
    while len(programs) < count:
        attempts += 1
        assert attempts < count * 200, "template space exhausted"
        template = TEMPLATES[rng.randrange(len(TEMPLATES))]
        source = PREFIXES[rng.randrange(len(PREFIXES))] + expand(template, rng)
        if source in seen:
            continue
        seen.add(source)
        programs.append(source)
    return programs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--count", type=int, required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    programs = generate(args.seed, args.count)
    with open(args.out, "w", encoding="utf-8", newline="\n") as out:
        for index, source in enumerate(programs):
            out.write(json.dumps({"id": index, "source": source}, ensure_ascii=True) + "\n")
    digest = hashlib.sha256("\0".join(programs).encode("utf-8")).hexdigest()
    print(f"seed={args.seed} count={len(programs)} templates={len(TEMPLATES)} sha256={digest}")


if __name__ == "__main__":
    main()
