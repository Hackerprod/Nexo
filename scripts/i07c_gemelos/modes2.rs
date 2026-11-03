// Literal ADR cases, grouping pairs, unification oracle, fuzz, and stack depth.
use crate::ast::*;
use crate::common::*;
use crate::run::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

// ------------------------------------------------------------------ literal tables (ADR 14.10, DG07, DG08)

fn nth(src: &str, pat: &str, n: usize) -> (u64, u64) {
    let mut from = 0;
    let mut pos = None;
    for _ in 0..=n {
        let i = src[from..].find(pat).unwrap_or_else(|| panic!("no {pat} #{n} in {src}")) + from;
        pos = Some(i);
        from = i + 1;
    }
    let i = pos.unwrap();
    (i as u64, (i + pat.len()) as u64)
}
fn last(src: &str, pat: &str) -> (u64, u64) {
    let i = src.rfind(pat).unwrap();
    (i as u64, (i + pat.len()) as u64)
}

const DV: &str = "(if true then {return;}else{return;})";

struct Exp {
    code: &'static str,
    prim: (u64, u64),
    exp: &'static str,
    found: &'static str,
    rel: Vec<(&'static str, (u64, u64))>,
    sub: &'static str,
}
fn verify(name: &str, src: &str, want: &[Exp], fails: &mut Vec<String>) {
    match check(src) {
        R::Ok => {
            if !want.is_empty() {
                fails.push(format!("{name}: accepted, expected {} diagnostics", want.len()));
            }
        }
        R::Bug(m) => fails.push(format!("{name}: BUG {m}")),
        R::Rej(v) => {
            if v.len() != want.len() {
                fails.push(format!("{name}: {} diagnostics, expected {}: {:?}", v.len(), want.len(), v.iter().map(|d| (d.code.clone(), d.s, d.e)).collect::<Vec<_>>()));
                return;
            }
            for (d, w) in v.iter().zip(want) {
                let mut rel: Vec<(String, u64, u64)> = w.rel.iter().map(|(r, (s, e))| (r.to_string(), *s, *e)).collect();
                rel.sort_by_key(|x| (x.1, x.2));
                let ok = d.code == w.code
                    && d.sub == w.sub
                    && d.stage == "types"
                    && d.op == "types.check"
                    && (d.s, d.e) == w.prim
                    && d.exp == w.exp
                    && d.found == w.found
                    && d.rel == rel
                    && d.extra.starts_with("chain=0 repair=0 feat=None");
                if !ok {
                    fails.push(format!("{name}: got {:?} stage={} op={} prim=({},{}) exp={} found={} rel={:?} extra={} / want prim={:?} exp={} found={} rel={:?}", d.code, d.stage, d.op, d.s, d.e, d.exp, d.found, d.rel, d.extra, w.prim, w.exp, w.found, rel));
                }
                let dj = d.data.clone();
                if !dj.contains(w.exp) || !dj.contains(w.found) {
                    fails.push(format!("{name}: data {dj}"));
                }
            }
        }
    }
}
fn accepted_and_unit(name: &str, src: &str, fails: &mut Vec<String>) {
    if check(src) != R::Ok {
        fails.push(format!("{name}: not accepted: {:?}", check(src)));
        return;
    }
    let (r, o) = crate::common::run(src);
    if r != R::Ok || o != "Unit" {
        fails.push(format!("{name}: run {r:?} {o}"));
    }
}
fn tm(prim: (u64, u64), exp: &'static str, found: &'static str, rel: Vec<(&'static str, (u64, u64))>) -> Exp {
    Exp { code: "NX-E0102", prim, exp, found, rel, sub: "type_mismatch" }
}

pub fn lits() -> (usize, Vec<String>) {
    let mut f = Vec::new();
    let n = std::cell::Cell::new(0usize);
    let t = |f: &mut Vec<String>, name: &str, src: String, want: Vec<Exp>| {
        n.set(n.get() + 1);
        verify(name, &src, &want, f);
    };
    // TD01
    let s = format!("fn Main(){{var v={DV};var w=v;v=3;w=true;}}");
    let p = last(&s, "true");
    let e3 = nth(&s, "3", 0);
    t(&mut f, "TD01", s.clone(), vec![tm(p, "I64", "Bool", vec![("expected_type", e3), ("found_type", p)])]);
    // TD01 positive
    let sp = s.replace("w=true;", "w=4;");
    t(&mut f, "TD01+", sp, vec![]);
    // TD02
    let s = "fn Main(){var v=3;var w=v;v=3;w=true;}".to_string();
    let p = last(&s, "true");
    let e3 = nth(&s, "3", 0);
    t(&mut f, "TD02", s.clone(), vec![tm(p, "I64", "Bool", vec![("expected_type", e3), ("found_type", p)])]);
    t(&mut f, "TD02+", s.replace("w=true;", "w=4;"), vec![]);
    // TD03
    let s = format!("fn Main(){{var v={DV};var w=true;w=v;v=3;}}");
    let p = last(&s, "3");
    let et = nth(&s, "true", 1); // first "true" is inside DV
    let et = if s.matches("true").count() == 3 { et } else { et };
    // occurrences of "true": DV has one ("if true"), then "var w=true"
    t(&mut f, "TD03", s.clone(), vec![tm(p, "Bool", "I64", vec![("expected_type", et), ("found_type", p)])]);
    let mut sp = s.clone();
    let i = sp.rfind("v=3;").unwrap() + 2;
    sp.replace_range(i..i + 1, "false");
    t(&mut f, "TD03+", sp, vec![]);
    // TD04 / TD05 / TD06 / live twin
    for (name, src) in [
        ("TD04a", format!("fn Main(){{let x={DV}==true;}}")),
        ("TD04b", format!("fn Main(){{let x=true=={DV};}}")),
        ("TD05a", format!("fn Main(){{let x=[{DV},true];}}")),
        ("TD05b", format!("fn Main(){{let x=[true,{DV}];}}")),
        ("TD06", format!("fn Main(){{var v={DV};var w=v;v=[1];w=[2];w[0]=9;}}")),
        ("TD06live", "fn Main(){var v=[0];var w=v;v=[1];w=[2];w[0]=9;AssertEq(v,[1]);AssertEq(w,[9]);}".to_string()),
    ] {
        n.set(n.get() + 1);
        accepted_and_unit(name, &src, &mut f);
    }
    // TD07
    {
        n.set(n.get() + 1);
        let s = "fn Main(){let x=missing;let y=x;let z=true+1;}";
        match check(s) {
            R::Rej(v) if v.len() == 1 && v[0].code == "NX-E0101" && v[0].stage == "names" && (v[0].s, v[0].e) == nth(s, "missing", 0) => {}
            o => f.push(format!("TD07 names: {o:?}")),
        }
        let s = "fn Main(){let x=0;let y=x;let z=true+1;}".to_string();
        let p = last(&s, "true");
        t(&mut f, "TD07+", s, vec![tm(p, "I64", "Bool", vec![("found_type", p)])]);
    }
    // isolated negatives with Never
    for (name, src, cut) in [
        ("DV+true", format!("fn Main(){{let x={DV}+true;}}"), "true+"),
        ("true+DV", format!("fn Main(){{let x=true+{DV};}}"), "true"),
    ] {
        let _ = cut;
        let p = if name == "DV+true" { last(&src, "true") } else { nth(&src, "true", 0) };
        t(&mut f, name, src.clone(), vec![tm(p, "I64", "Bool", vec![("found_type", p)])]);
        let mut sp = src.clone();
        sp.replace_range(p.0 as usize..p.1 as usize, "1");
        t(&mut f, &format!("{name}+"), sp, vec![]);
    }
    for (name, src) in [
        ("[DV,1,true]", format!("fn Main(){{let x=[{DV},1,true];}}")),
        ("[1,DV,true]", format!("fn Main(){{let x=[1,{DV},true];}}")),
    ] {
        let p = last(&src, "true");
        let one = nth(&src, "1", 0);
        t(&mut f, name, src.clone(), vec![tm(p, "I64", "Bool", vec![("expected_type", one), ("found_type", p)])]);
        let mut sp = src.clone();
        let i = sp.rfind("true").unwrap();
        sp.replace_range(i..i + 4, "1");
        t(&mut f, &format!("{name}+"), sp, vec![]);
    }
    // contamination in one stage
    {
        let s = "fn Main(){let x=true+1;let y=x;let z=false+1;}".to_string();
        let a = nth(&s, "true", 0);
        let b = nth(&s, "false", 0);
        t(&mut f, "contam", s.clone(), vec![tm(a, "I64", "Bool", vec![("found_type", a)]), tm(b, "I64", "Bool", vec![("found_type", b)])]);
        t(&mut f, "contam++", s.replace("true", "0").replace("false", "0"), vec![]);
        t(&mut f, "contam+1", s.replace("true", "0"), vec![tm((b.0 - 3, b.1 - 3 + 0), "I64", "Bool", vec![("found_type", (b.0 - 3, b.1 - 3))])]);
        t(&mut f, "contam+2", s.replace("false", "0"), vec![tm(a, "I64", "Bool", vec![("found_type", a)])]);
    }
    // DG07 / DG08
    {
        let s = "fn Main(){var a:Array<I64>=[0];a[0]=true;}".to_string();
        let p = last(&s, "true");
        let ann = nth(&s, "Array<I64>", 0);
        t(&mut f, "DG07", s.clone(), vec![tm(p, "I64", "Bool", vec![("expected_type", ann), ("found_type", p)])]);
        t(&mut f, "DG07+", s.replace("true", "1"), vec![]);
        let s = "fn Main(){var a=[0];a[0]=true;}".to_string();
        let p = last(&s, "true");
        let init = nth(&s, "[0]", 0);
        t(&mut f, "DG08", s.clone(), vec![tm(p, "I64", "Bool", vec![("expected_type", init), ("found_type", p)])]);
        t(&mut f, "DG08+", s.replace("true", "1"), vec![]);
    }
    (n.get(), f)
}

// ------------------------------------------------------------------ parenthesis pairs

fn rel_key(pr: &Printed, d: &D) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = d
        .rel
        .iter()
        .map(|(role, s, e)| {
            let k = pr.spans.get(&(*s, *e)).map(|i| format!("n{i}")).unwrap_or_else(|| pr.src[*s as usize..*e as usize].to_string());
            (role.clone(), k)
        })
        .collect();
    v.sort();
    v
}
type Key = Vec<(String, String, String, String, String, Vec<(String, String)>)>;
fn full_key(pr: &Printed, v: &[D]) -> Key {
    let mut k: Key = v
        .iter()
        .map(|d| {
            let site = if let Some(i) = pr.ops.get(&(d.s, d.e)) {
                format!("op{i}")
            } else if let Some(i) = pr.spans.get(&(d.s, d.e)) {
                format!("n{i}")
            } else {
                format!("t:{}", &pr.src[d.s as usize..d.e as usize])
            };
            (d.code.clone(), d.sub.clone(), site, d.exp.clone(), d.found.clone(), rel_key(pr, d))
        })
        .collect();
    k.sort();
    k
}
fn wrap_at(p: &mut Prog, target: u32) -> bool {
    fn go(x: &mut N, target: u32, done: &mut bool) {
        if *done {
            return;
        }
        if x.id == target && !matches!(x.k, K::Blk(_)) {
            let old = std::mem::replace(x, N { id: 0, k: K::Int(0) });
            *x = N { id: 1_000_000 + target, k: K::Grp(Box::new(old)) };
            *done = true;
            return;
        }
        match &mut x.k {
            K::Bin(_, a, b) | K::Idx(a, b) => {
                go(a, target, done);
                go(b, target, done);
            }
            K::If(c, a, b) => {
                go(c, target, done);
                go(a, target, done);
                go(b, target, done);
            }
            K::Blk(a) | K::Grp(a) => go(a, target, done),
            K::Arr(v) => v.iter_mut().for_each(|e| go(e, target, done)),
            _ => {}
        }
    }
    let mut done = false;
    for s in &mut p.stmts {
        match s {
            S::Let(_, e) | S::Assign(_, e) | S::IdxAssign(_, e) | S::Ret(_, e) | S::Call(_, e) | S::Cond(e) | S::While(e) => go(e, target, &mut done),
            S::Assert(a, b) => {
                go(a, target, &mut done);
                go(b, target, &mut done);
            }
        }
    }
    done
}

pub fn grp(n: u64, seed: u64, threads: u64) {
    let pairs = AtomicU64::new(0);
    let withdiag = AtomicU64::new(0);
    let diffs = AtomicU64::new(0);
    let ctxcount: Mutex<BTreeMap<String, u64>> = Mutex::new(BTreeMap::new());
    let ex: Mutex<Vec<String>> = Mutex::new(vec![]);
    par(n, threads, |i| {
        let mut r = Rng::new(seed.wrapping_mul(0x517C_C1B7_2722_0A95).wrapping_add(i).wrapping_mul(0x2545_F491_4F6C_DD1D));
        let pf = if r.chance(1, 3) { profile("defer") } else { profile("plain") };
        let mut g = G { r: &mut r, id: 0, pf, allow_rq: true };
        let p = g.prog();
        // pick a node to wrap
        let mut ids: Vec<(u32, String)> = vec![];
        for s in &p.stmts {
            let kind = match s {
                S::Let(Some(_), _) => "let-annot",
                S::Let(None, _) => "let",
                S::Assign(..) => "assign",
                S::IdxAssign(..) => "idx-assign",
                S::Ret(..) => "return",
                S::Call(..) => "call-arg",
                S::Cond(_) => "if-stmt-cond",
                S::While(_) => "while-cond",
                S::Assert(..) => "assert-arg",
            };
            for nn in s.nodes() {
                let top = nn.id;
                nn.walk(&mut |x| {
                    let ctx = if x.id == top {
                        kind.to_string()
                    } else {
                        match &x.k {
                            K::Bin(..) => "operand:bin",
                            K::Idx(..) => "operand:index",
                            _ => "operand",
                        }
                        .to_string()
                    };
                    if !matches!(x.k, K::Blk(_)) {
                        ids.push((x.id, ctx));
                    }
                });
            }
        }
        if ids.is_empty() {
            return;
        }
        let (tid, tctx) = ids[r.below(ids.len() as u64) as usize].clone();
        let mut p2 = p.clone();
        if !wrap_at(&mut p2, tid) {
            return;
        }
        // finer context label for the wrapped node: how it is used by its parent
        let mut label = tctx.clone();
        for s in &p.stmts {
            for nn in s.nodes() {
                nn.walk(&mut |x| match &x.k {
                    K::If(c, a, b) => {
                        if c.id == tid {
                            label = "if-expr-cond".into()
                        } else if a.id == tid || b.id == tid {
                            label = "if-branch".into()
                        }
                    }
                    K::Blk(a) if a.id == tid => label = "value-block".into(),
                    K::Arr(es) if es.iter().any(|e| e.id == tid) => label = "array-element".into(),
                    K::Bin(op, a, b) if a.id == tid || b.id == tid => {
                        label = format!("operand:{}:{}", op.s(), if a.id == tid { "left" } else { "right" })
                    }
                    K::Idx(a, b) if a.id == tid || b.id == tid => label = if a.id == tid { "index-base".into() } else { "index-selector".into() },
                    _ => {}
                });
            }
        }
        let a = print(&p, true);
        let b = print(&p2, true);
        let ra = check(&a.src);
        let rb = check(&b.src);
        pairs.fetch_add(1, Relaxed);
        *ctxcount.lock().unwrap().entry(label.clone()).or_default() += 1;
        let (ka, kb) = match (&ra, &rb) {
            (R::Rej(x), R::Rej(y)) => (full_key(&a, x), full_key(&b, y)),
            (R::Ok, R::Ok) => (vec![], vec![]),
            _ => (vec![("shape".into(), "".into(), "A".into(), "".into(), format!("{:?}", matches!(ra, R::Ok)), vec![])], vec![("shape".into(), "".into(), "B".into(), "".into(), format!("{:?}", matches!(rb, R::Ok)), vec![])]),
        };
        if !ka.is_empty() {
            withdiag.fetch_add(1, Relaxed);
        }
        if ka != kb {
            diffs.fetch_add(1, Relaxed);
            let mut e = ex.lock().unwrap();
            if e.len() < 8 {
                e.push(format!("[{label}] A={} B={} :: {:?} vs {:?}", a.src, b.src, ka, kb));
            }
        }
    });
    println!("GRP pairs={} with_diagnostics={} differences={} (span/found/expected/cardinality/related)", pairs.load(Relaxed), withdiag.load(Relaxed), diffs.load(Relaxed));
    println!("contexts={}", ctxcount.lock().unwrap().iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join(" "));
    for e in ex.lock().unwrap().iter() {
        println!("EX {}", &e[..e.len().min(900)]);
    }
}

// ------------------------------------------------------------------ unification oracle with permutations

#[derive(Clone, Debug)]
enum Init {
    Dv,
    Lit(u8),   // 0 I64, 1 Bool, 2 Arr(I64), 3 Arr(Bool)
    Empty,     // []
    Alias(usize),
}
#[derive(Clone, Debug)]
enum Con {
    AssignLit(usize, u8),
    AssignVar(usize, usize),
    Eq(usize, usize),
    Arr2(usize, usize),
    IfJoin(usize, usize),
}
const LITS: [&str; 4] = ["3", "true", "[1]", "[true]"];
// concrete kind: 0 I64, 1 Bool, 2 Arr(I64), 3 Arr(Bool), 4 Arr(?) (from `[]`)
fn compat(a: u8, b: u8) -> Option<u8> {
    if a == b {
        return Some(a);
    }
    match (a, b) {
        (4, 2) | (2, 4) => Some(2),
        (4, 3) | (3, 4) => Some(3),
        _ => None,
    }
}
fn oracle_accepts(inits: &[Init], cons: &[Con]) -> bool {
    let n = inits.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut Vec<usize>, x: usize) -> usize {
        if p[x] != x {
            let r = find(p, p[x]);
            p[x] = r;
        }
        p[x]
    }
    let mut un = |p: &mut Vec<usize>, a: usize, b: usize| {
        let (ra, rb) = (find(p, a), find(p, b));
        if ra != rb {
            p[ra] = rb;
        }
    };
    let mut bind: Vec<(usize, u8)> = vec![];
    for (i, it) in inits.iter().enumerate() {
        match it {
            Init::Dv => {}
            Init::Lit(k) => bind.push((i, *k)),
            Init::Empty => bind.push((i, 4)),
            Init::Alias(j) => un(&mut parent, i, *j),
        }
    }
    for c in cons {
        match c {
            Con::AssignLit(i, k) => bind.push((*i, *k)),
            Con::AssignVar(i, j) | Con::Eq(i, j) | Con::Arr2(i, j) | Con::IfJoin(i, j) => un(&mut parent, *i, *j),
        }
    }
    let mut cls: HashMap<usize, u8> = HashMap::new();
    for (v, k) in bind {
        let r = find(&mut parent, v);
        match cls.get(&r) {
            None => {
                cls.insert(r, k);
            }
            Some(&o) => match compat(o, k) {
                Some(m) => {
                    cls.insert(r, m);
                }
                None => return false,
            },
        }
    }
    // unresolved `[]` element
    if cls.values().any(|&k| k == 4) {
        return false;
    }
    // an array constructor whose element class never receives a concrete type stays unresolved (E0103)
    for c in cons {
        if let Con::Arr2(i, j) = c {
            for v in [i, j] {
                let r = find(&mut parent, *v);
                if !cls.contains_key(&r) {
                    return false;
                }
            }
        }
    }
    true
}
fn oracle_src(inits: &[Init], cons: &[Con]) -> String {
    let mut s = String::from("fn Main(){");
    for (i, it) in inits.iter().enumerate() {
        match it {
            Init::Dv => s += &format!("var v{i}={DV};"),
            Init::Lit(k) => s += &format!("var v{i}={};", LITS[*k as usize]),
            Init::Empty => s += &format!("var v{i}=[];"),
            Init::Alias(j) => s += &format!("var v{i}=v{j};"),
        }
    }
    for (k, c) in cons.iter().enumerate() {
        match c {
            Con::AssignLit(i, l) => s += &format!("v{i}={};", LITS[*l as usize]),
            Con::AssignVar(i, j) => s += &format!("v{i}=v{j};"),
            Con::Eq(i, j) => s += &format!("let e{k}=v{i}==v{j};"),
            Con::Arr2(i, j) => s += &format!("let a{k}=[v{i},v{j}];"),
            Con::IfJoin(i, j) => s += &format!("let j{k}=(if true then {{value:v{i};}}else{{value:v{j};}});"),
        }
    }
    s + "}"
}
pub fn oracle(n: u64, seed: u64, threads: u64) {
    let progs = AtomicU64::new(0);
    let bad = AtomicU64::new(0);
    let accepted = AtomicU64::new(0);
    let rejected = AtomicU64::new(0);
    let perm_dep = AtomicU64::new(0);
    let seen: Mutex<HashSet<u64>> = Mutex::new(HashSet::new());
    let ex: Mutex<Vec<String>> = Mutex::new(vec![]);
    use std::hash::{Hash, Hasher};
    par(n, threads, |i| {
        let mut r = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(i).wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
        let nv = 2 + r.below(4) as usize;
        let mut inits = vec![];
        for k in 0..nv {
            let c = r.below(10);
            inits.push(match c {
                0..=3 => Init::Dv,
                4 | 5 => Init::Lit(r.below(4) as u8),
                6 => Init::Empty,
                _ => {
                    if k > 0 {
                        Init::Alias(r.below(k as u64) as usize)
                    } else {
                        Init::Dv
                    }
                }
            });
        }
        let nc = 2 + r.below(5) as usize;
        let mut cons = vec![];
        for _ in 0..nc {
            let a = r.below(nv as u64) as usize;
            let b = r.below(nv as u64) as usize;
            cons.push(match r.below(10) {
                0..=3 => Con::AssignLit(a, r.below(4) as u8),
                4 | 5 => Con::AssignVar(a, b),
                6 | 7 => Con::Eq(a, b),
                8 => Con::Arr2(a, b),
                _ => Con::IfJoin(a, b),
            });
        }
        let want = oracle_accepts(&inits, &cons);
        let mut prev: Option<bool> = None;
        // identity + 3 random permutations of the constraint statements
        for k in 0..4 {
            let mut c2 = cons.clone();
            if k > 0 {
                for x in (1..c2.len()).rev() {
                    let y = r.below(x as u64 + 1) as usize;
                    c2.swap(x, y);
                }
            }
            let src = oracle_src(&inits, &c2);
            let mut h = std::collections::hash_map::DefaultHasher::new();
            src.hash(&mut h);
            let fresh = seen.lock().unwrap().insert(h.finish());
            let got = match check(&src) {
                R::Ok => true,
                R::Rej(_) => false,
                R::Bug(m) => {
                    ex.lock().unwrap().push(format!("BUG {m}: {src}"));
                    bad.fetch_add(1, Relaxed);
                    continue;
                }
            };
            if fresh {
                progs.fetch_add(1, Relaxed);
                if got {
                    accepted.fetch_add(1, Relaxed);
                } else {
                    rejected.fetch_add(1, Relaxed);
                }
            }
            if let Some(p) = prev {
                if p != got {
                    perm_dep.fetch_add(1, Relaxed);
                    ex.lock().unwrap().push(format!("ORDER-DEPENDENT: {src}"));
                }
            }
            prev = Some(got);
            if got != want {
                bad.fetch_add(1, Relaxed);
                let mut e = ex.lock().unwrap();
                if e.len() < 12 {
                    e.push(format!("DISAGREE oracle={want} impl={got}: {src}"));
                }
            }
        }
    });
    println!("ORACLE distinct_sources={} accepted={} rejected={} disagreements={} order_dependent={}", progs.load(Relaxed), accepted.load(Relaxed), rejected.load(Relaxed), bad.load(Relaxed), perm_dep.load(Relaxed));
    for e in ex.lock().unwrap().iter().take(10) {
        println!("EX {}", &e[..e.len().min(500)]);
    }
}

// ------------------------------------------------------------------ free fuzz

pub fn fuzz_prog(seed: u64) -> String {
    let mut r = Rng::new(seed.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ 0xA5A5_5A5A_1234_5678);
    let pf = match r.below(4) {
        0 => profile("plain"),
        1 | 2 => profile("defer"),
        _ => profile("heavy"),
    };
    let mut g = G { r: &mut r, id: 0, pf, allow_rq: true };
    let mut p = g.prog();
    // extra statements
    let extra = g.r.below(3);
    for _ in 0..extra {
        let s = g.stmt();
        p.stmts.push(s);
    }
    fix_ids(&mut p);
    let mut s = print(&p, !g.r.chance(1, 5)).src;
    if r.chance(1, 8) {
        let mut b = s.into_bytes();
        let i = r.below(b.len() as u64) as usize;
        match r.below(5) {
            0 => {
                b.remove(i);
            }
            1 => b.insert(i, *r.pick(&[b'(', b')', b'[', b']', b'{', b'}', b';', b',', b'=', b'<', b'+', b'-', b' '])),
            2 => {
                let j = r.below(b.len() as u64) as usize;
                b.swap(i, j);
            }
            3 => {
                let c = b[i];
                b.insert(i, c);
            }
            _ => {
                let j = (i + r.below(30) as usize).min(b.len());
                b.drain(i..j);
            }
        }
        s = String::from_utf8_lossy(&b).into_owned();
    }
    s
}
pub fn fuzz(n: u64, seed: u64, threads: u64) {
    let progs = AtomicU64::new(0);
    let accepted = AtomicU64::new(0);
    let bugs = AtomicU64::new(0);
    let incoh = AtomicU64::new(0);
    let dups = AtomicU64::new(0);
    let sitedup = AtomicU64::new(0);
    let entry = AtomicU64::new(0);
    let unit = AtomicU64::new(0);
    let ex: Mutex<Vec<String>> = Mutex::new(vec![]);
    par(n, threads, |i| {
        let src = fuzz_prog(seed.wrapping_mul(2_654_435_761).wrapping_add(i));
        progs.fetch_add(1, Relaxed);
        let c = check(&src);
        let (rr, outcome) = crate::common::run(&src);
        let mut note = |k: &str| {
            let mut e = ex.lock().unwrap();
            if e.len() < 12 {
                e.push(format!("{k}: {}", &src[..src.len().min(600)]));
            }
        };
        if matches!(c, R::Bug(_)) || matches!(rr, R::Bug(_)) {
            bugs.fetch_add(1, Relaxed);
            note(&format!("BUG {c:?} {rr:?}"));
        }
        // the entry stage exists only for run (F0-0002 13.9): check accepts, run may reject with stage `entry`
        let entry_only = c == R::Ok && matches!(&rr, R::Rej(v) if v.iter().all(|d| d.stage == "entry"));
        if entry_only {
            entry.fetch_add(1, Relaxed);
        }
        if c != rr && !entry_only {
            incoh.fetch_add(1, Relaxed);
            note("INCOHERENT");
        }
        if has_dup(&c) {
            dups.fetch_add(1, Relaxed);
            note("STRICTDUP");
        }
        if has_site_dup(&c) {
            sitedup.fetch_add(1, Relaxed);
            note("SITEDUP");
        }
        if c == R::Ok {
            accepted.fetch_add(1, Relaxed);
            if outcome == "Unit" {
                unit.fetch_add(1, Relaxed);
            }
        }
    });
    println!(
        "FUZZ seed={seed} programs={} accepted={} run_unit={} panic_invalidir_internal={} check_run_incoherent={} strict_dup={} same_site_dup={} entry_stage_only(run)={}",
        progs.load(Relaxed), accepted.load(Relaxed), unit.load(Relaxed), bugs.load(Relaxed), incoh.load(Relaxed), dups.load(Relaxed), sitedup.load(Relaxed), entry.load(Relaxed)
    );
    for e in ex.lock().unwrap().iter() {
        println!("EX {e}");
    }
}

// ------------------------------------------------------------------ stack depth

fn rep(s: &str, n: usize) -> String {
    s.repeat(n)
}
pub fn deep_families(d: usize) -> Vec<(&'static str, String)> {
    let w = |b: String| format!("fn Main(){{{b}}}");
    let mut v: Vec<(&'static str, String)> = vec![];
    v.push(("grp_nest_annot", w(format!("let a:Bool={}true{};", rep("(", d), rep(")", d)))));
    v.push(("grp_nest_bad", w(format!("let a:Bool={}7{};", rep("(", d), rep(")", d)))));
    v.push(("if_else_spine_bad", w(format!("let a:Bool={}7{};", rep("(if true then true else ", d), rep(")", d)))));
    v.push(("if_then_spine_bad", w(format!("let a:Bool={}7{};", rep("(if true then ", d), rep(" else true)", d)))));
    v.push(("blk_spine_bad", w(format!("let a:I64={}true{};", rep("(if true then {value:", d), rep(";} else 1)", d)))));
    v.push(("join_spine", w(format!("let a={}true{};", rep("(if true then 1 else ", d), rep(")", d)))));
    v.push(("arr_nest_bad", w(format!("let a:Array<Bool>={}1{};", rep("[", d), rep("]", d)))));
    v.push(("arr_deferred_nest", w(format!("var r=[];let a={}r{};r=[true];", rep("[", d), rep("]", d)))));
    v.push(("add_right_nest", w(format!("let a={}true{};", rep("1+(", d), rep(")", d)))));
    v.push(("add_left_chain_bad", w(format!("let a=1{}+true;", rep("+1", d)))));
    v.push(("idx_nest", w(format!("let b={}1{};let a=b{};", rep("[", d), rep("]", d), rep("[0]", d)))));
    v.push(("eq_pair_nest", w(format!("var q={DV};let a={}q{};AssertEq(a,{}1{});q=3;", rep("[", d), rep("]", d), rep("[", d), rep("]", d)))));
    let m = d.min(2500);
    let mut s = format!("var v0={DV};");
    for i in 1..m {
        s += &format!("var v{i}=v{};", i - 1);
    }
    s += &format!("v0=3;v{}=true;", m - 1);
    v.push(("alias_chain_conflict", w(s)));
    let mut s = String::new();
    for i in 0..m {
        s += &format!("var v{i}=[];");
    }
    for i in 0..m - 1 {
        s += &format!("v{i}=[v{}];", i + 1);
    }
    s += &format!("v{}=[1];", m - 1);
    v.push(("deferred_ladder", w(s)));
    v.push(("stmts", w(rep("let a=(1);", d * 3))));
    v
}
pub fn deep(kib: usize, depths: &[usize]) {
    use std::io::Write;
    for &d in depths {
        for (name, src) in deep_families(d) {
            println!("CASE stack={kib}KiB depth={d} family={name} bytes={}", src.len());
            std::io::stdout().flush().unwrap();
            let s2 = src.clone();
            let h = std::thread::Builder::new()
                .stack_size(kib * 1024)
                .spawn(move || {
                    let c = check(&s2);
                    let (r, _) = crate::common::run(&s2);
                    (c, r)
                })
                .unwrap();
            let (c, r) = h.join().unwrap();
            let kind = match &c {
                R::Ok => "ok".to_string(),
                R::Rej(v) => format!("rej:{}", v.iter().map(|d| format!("{}/{}", d.code, d.sub)).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>().join("+")),
                R::Bug(m) => format!("BUG:{m}"),
            };
            println!("RESULT {kind} coherent={} dup={}", c == r, has_dup(&c));
        }
    }
}
