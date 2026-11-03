// (d2) Unification oracle (own union-find with occurs check + groundness of every array-literal occurrence), with
// statement-order permutations. ADR 4.1.1: acceptance does not depend on the solver / statement order.
use crate::common::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

#[derive(Clone, Copy, PartialEq, Debug)]
enum T {
    Free,
    I,
    B,
    Arr(usize),
}
struct U {
    p: Vec<usize>,
    t: Vec<T>,
}
impl U {
    fn new() -> U {
        U { p: vec![], t: vec![] }
    }
    fn node(&mut self, t: T) -> usize {
        self.p.push(self.p.len());
        self.t.push(t);
        self.p.len() - 1
    }
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.p[r] != r {
            r = self.p[r];
        }
        r
    }
    /// Detects cyclic (infinite) types among the current classes.
    fn cyclic(&mut self) -> bool {
        let n = self.p.len();
        let mut state = vec![0u8; n]; // 0 new, 1 on stack, 2 done
        for start in 0..n {
            let mut x = self.find(start);
            if state[x] != 0 {
                continue;
            }
            let mut path = vec![];
            loop {
                if state[x] == 1 {
                    return true;
                }
                if state[x] == 2 {
                    break;
                }
                state[x] = 1;
                path.push(x);
                match self.t[x] {
                    T::Arr(c) => x = self.find(c),
                    _ => break,
                }
            }
            for y in path {
                state[y] = 2;
            }
        }
        false
    }
    fn unify(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return true;
        }
        match (self.t[ra], self.t[rb]) {
            (T::Free, _) => {
                self.p[ra] = rb;
                true
            }
            (_, T::Free) => {
                self.p[rb] = ra;
                true
            }
            (T::I, T::I) | (T::B, T::B) => {
                self.p[ra] = rb;
                true
            }
            (T::Arr(x), T::Arr(y)) => {
                self.p[ra] = rb;
                self.unify(x, y)
            }
            _ => false,
        }
    }
    fn ground(&mut self, x: usize) -> bool {
        let r = self.find(x);
        match self.t[r] {
            T::Free => false,
            T::I | T::B => true,
            T::Arr(c) => self.ground(c),
        }
    }
}

#[derive(Clone, Debug)]
enum St {
    Assign(usize, usize),
    ArrAssign(usize, usize),
    LitAssign(usize, usize),
    Eq(usize, usize),
    IdxAssign(usize, usize),
    AssertEq(usize, usize),
    IdxRead(usize),
    Pair(usize, usize),
}
const LITS: [&str; 5] = ["1", "true", "[1]", "[true]", "[[1]]"];

struct Prog {
    kinds: Vec<bool>,           // per declared var: true = DV, false = []
    aliases: Vec<(usize, usize)>, // (new var index, source var index)
    nvars: usize,
    stmts: Vec<St>,
}

fn name(i: usize) -> String {
    format!("v{i}")
}
fn render(p: &Prog, order: &[usize]) -> String {
    let mut s = String::from("fn Main(){");
    for (i, d) in p.kinds.iter().enumerate() {
        if *d {
            s += &format!("var {}=(if true then {{return;}} else {{return;}});", name(i));
        } else {
            s += &format!("var {}=[];", name(i));
        }
    }
    for (n, src) in &p.aliases {
        s += &format!("var {}={};", name(*n), name(*src));
    }
    for (k, &i) in order.iter().enumerate() {
        match &p.stmts[i] {
            St::Assign(a, b) => s += &format!("{}={};", name(*a), name(*b)),
            St::ArrAssign(a, b) => s += &format!("{}=[{}];", name(*a), name(*b)),
            St::LitAssign(a, l) => s += &format!("{}={};", name(*a), LITS[*l]),
            St::Eq(a, b) => s += &format!("let e{k}={}=={};", name(*a), name(*b)),
            St::IdxAssign(a, l) => s += &format!("{}[0]={};", name(*a), LITS[*l]),
            St::AssertEq(a, b) => s += &format!("AssertEq({},{});", name(*a), name(*b)),
            St::IdxRead(a) => s += &format!("let r{k}={}[0];", name(*a)),
            St::Pair(a, b) => s += &format!("let c{k}=[{},{}];", name(*a), name(*b)),
        }
    }
    s += "}";
    s
}

/// Returns (accepted according to the oracle).
fn oracle(p: &Prog) -> bool {
    let mut u = U::new();
    let mut node_of: Vec<usize> = Vec::new();
    let mut lits: Vec<usize> = Vec::new(); // array-literal occurrences that must be ground
    // §4.1.2.3, RD114/RD121: a projection is conditional on an Array
    // constructor. A selector alone does not install that constructor.
    let mut projections: Vec<(usize, Option<usize>)> = Vec::new();
    for d in &p.kinds {
        if *d {
            let n = u.node(T::Free);
            node_of.push(n);
        } else {
            let f = u.node(T::Free);
            let a = u.node(T::Arr(f));
            lits.push(a);
            node_of.push(a);
        }
    }
    for (n, src) in &p.aliases {
        let id = node_of[*src];
        while node_of.len() <= *n {
            node_of.push(usize::MAX);
        }
        node_of[*n] = id;
    }
    let lit = |u: &mut U, l: usize, lits: &mut Vec<usize>| -> usize {
        match l {
            0 => u.node(T::I),
            1 => u.node(T::B),
            2 => {
                let e = u.node(T::I);
                let a = u.node(T::Arr(e));
                lits.push(a);
                a
            }
            3 => {
                let e = u.node(T::B);
                let a = u.node(T::Arr(e));
                lits.push(a);
                a
            }
            _ => {
                let e = u.node(T::I);
                let a1 = u.node(T::Arr(e));
                let a2 = u.node(T::Arr(a1));
                lits.push(a2);
                lits.push(a1);
                a2
            }
        }
    };
    for s in &p.stmts {
        let ok = match s {
            St::Assign(a, b) | St::Eq(a, b) | St::AssertEq(a, b) => u.unify(node_of[*a], node_of[*b]),
            St::ArrAssign(a, b) => {
                let arr = u.node(T::Arr(node_of[*b]));
                lits.push(arr);
                u.unify(node_of[*a], arr)
            }
            St::LitAssign(a, l) => {
                let n = lit(&mut u, *l, &mut lits);
                u.unify(node_of[*a], n)
            }
            St::IdxAssign(a, l) => {
                let n = lit(&mut u, *l, &mut lits);
                projections.push((node_of[*a], Some(n)));
                true
            }
            St::IdxRead(a) => {
                projections.push((node_of[*a], None));
                true
            }
            St::Pair(a, b) => {
                let ok = u.unify(node_of[*a], node_of[*b]);
                let arr = u.node(T::Arr(node_of[*a]));
                lits.push(arr);
                ok
            }
        };
        if !ok || u.cyclic() {
            return false;
        }
    }
    loop {
        let mut remaining = Vec::new();
        let mut activated = false;
        for (base, value) in projections {
            let root = u.find(base);
            match u.t[root] {
                T::Arr(element) => {
                    activated = true;
                    if value.is_some_and(|rhs| !u.unify(element, rhs)) || u.cyclic() {
                        return false;
                    }
                }
                T::Free => remaining.push((base, value)),
                T::I | T::B => return false,
            }
        }
        if !activated {
            break;
        }
        projections = remaining;
    }
    for l in lits {
        if !u.ground(l) {
            return false;
        }
    }
    true
}

fn shuffle(r: &mut Rng, v: &mut [usize]) {
    for i in (1..v.len()).rev() {
        let j = r.below(i as u64 + 1) as usize;
        v.swap(i, j);
    }
}

pub fn run(n: u64, seed: u64, threads: u64) {
    let sources = AtomicU64::new(0);
    let programs = AtomicU64::new(0);
    let disagree = AtomicU64::new(0);
    let order_dep = AtomicU64::new(0);
    let coh = AtomicU64::new(0);
    let accepted = AtomicU64::new(0);
    let bugs = AtomicU64::new(0);
    let dups = AtomicU64::new(0);
    let ex: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let codes: Mutex<std::collections::BTreeMap<String, u64>> = Mutex::new(Default::default());
    par(n, threads, |i| {
        let mut r = Rng::new(seed.wrapping_mul(15_485_863).wrapping_add(i));
        let nk = 2 + r.below(4) as usize;
        let kinds: Vec<bool> = (0..nk).map(|_| r.chance(1, 2)).collect();
        let mut hid: Vec<usize> = (0..nk).map(|_| r.below(5) as usize).collect();
        // L-kind vars (`[]`) are arrays: their hidden type must be an array type
        for (i, d) in kinds.iter().enumerate() {
            if !*d && hid[i] < 2 {
                hid[i] += 2;
            }
        }
        let mut nvars = nk;
        let mut aliases = Vec::new();
        for _ in 0..r.below(3) {
            let src = r.below(nvars as u64) as usize;
            aliases.push((nvars, src));
            hid.push(hid[src]);
            nvars += 1;
        }
        let arr_of = |h: usize| -> Option<usize> {
            match h {
                0 => Some(2),
                1 => Some(3),
                2 => Some(4),
                _ => None,
            }
        };
        let elem_lit = |h: usize| -> Option<usize> {
            match h {
                2 => Some(0),
                3 => Some(1),
                4 => Some(2),
                _ => None,
            }
        };
        let ns = 2 + r.below(8) as usize;
        let mut stmts = Vec::new();
        let consistent = !r.chance(1, 5);
        // resolution statements for most variables
        for v in 0..nvars {
            if r.chance(2, 3) {
                stmts.push(St::LitAssign(v, if consistent { hid[v] } else { r.below(5) as usize }));
            }
        }
        for _ in 0..ns {
            let a = r.below(nvars as u64) as usize;
            let same: Vec<usize> = (0..nvars).filter(|&b| hid[b] == hid[a]).collect();
            let b = if consistent { *r.pick(&same) } else { r.below(nvars as u64) as usize };
            let st = match r.below(12) {
                0 | 1 | 2 => St::Assign(a, b),
                3 | 4 => {
                    let cands: Vec<usize> = (0..nvars).filter(|&c| arr_of(hid[c]) == Some(hid[a])).collect();
                    if consistent && !cands.is_empty() {
                        St::ArrAssign(a, *r.pick(&cands))
                    } else if consistent {
                        St::Assign(a, b)
                    } else {
                        St::ArrAssign(a, r.below(nvars as u64) as usize)
                    }
                }
                5 | 6 | 7 => St::LitAssign(a, if consistent { hid[a] } else { r.below(5) as usize }),
                8 => St::Eq(a, b),
                9 => match elem_lit(hid[a]) {
                    Some(l) if consistent => St::IdxAssign(a, l),
                    _ => St::IdxAssign(a, r.below(5) as usize),
                },
                10 => St::AssertEq(a, b),
                _ => {
                    if r.chance(1, 2) {
                        St::IdxRead(a)
                    } else {
                        St::Pair(a, b)
                    }
                }
            };
            stmts.push(st);
        }
        let mut stmts = stmts;
        stmts.truncate(14);
        let p = Prog { kinds, aliases, nvars, stmts };
        let _ = p.nvars;
        if std::env::var("V10_TRACE").is_ok() { eprintln!("ORACLE-INPUT {}", render(&p, &(0..p.stmts.len()).collect::<Vec<_>>())); }
        let want = oracle(&p);
        let mut order: Vec<usize> = (0..p.stmts.len()).collect();
        let mut seen = std::collections::HashSet::new();
        let mut verdicts = Vec::new();
        for perm in 0..5 {
            if perm > 0 {
                shuffle(&mut r, &mut order);
            }
            // permuted programs have their own oracle: the oracle is order independent for the accept decision
            let src = render(&p, &order);
            if !seen.insert(src.clone()) {
                continue;
            }
            sources.fetch_add(1, Relaxed);
            programs.fetch_add(2, Relaxed);
            if std::env::var("V10_TRACE").is_ok() { eprintln!("TRACE {src}"); }
            let c = check(&src);
            let (rr, _) = run_prog(&src);
            if let R::Bug(m) = &c {
                bugs.fetch_add(1, Relaxed);
                ex.lock().unwrap().push(format!("BUG {m}: {src}"));
            }
            if c != rr {
                coh.fetch_add(1, Relaxed);
                ex.lock().unwrap().push(format!("INCOHERENT: {src}"));
            }
            if has_dup(&c) {
                dups.fetch_add(1, Relaxed);
                ex.lock().unwrap().push(format!("DUP: {src}"));
            }
            let acc = c == R::Ok;
            if acc {
                accepted.fetch_add(1, Relaxed);
            } else if let R::Rej(v) = &c {
                let mut cs = codes.lock().unwrap();
                for d in v {
                    *cs.entry(format!("{}/{}", d.code, d.sub)).or_insert(0) += 1;
                }
            }
            verdicts.push(acc);
            if acc != want {
                disagree.fetch_add(1, Relaxed);
                let mut e = ex.lock().unwrap();
                if e.len() < 40 {
                    e.push(format!("DISAGREE oracle_accept={want} impl_accept={acc}: {src}\n    {:?}", c));
                }
            }
        }
        if verdicts.windows(2).any(|w| w[0] != w[1]) {
            order_dep.fetch_add(1, Relaxed);
        }
    });
    println!(
        "ORACLE seeds={} distinct_sources={} compile_runs={} impl_accepted={} oracle_disagreements={} order_dependent_programs={} check_run_incoherent={} strict_dups={} bugs={}",
        n,
        sources.load(Relaxed),
        programs.load(Relaxed),
        accepted.load(Relaxed),
        disagree.load(Relaxed),
        order_dep.load(Relaxed),
        coh.load(Relaxed),
        dups.load(Relaxed),
        bugs.load(Relaxed)
    );
    println!("ORACLE reject codes {:?}", codes.lock().unwrap());
    for e in ex.lock().unwrap().iter().take(14) {
        println!("{e}");
    }
}

fn run_prog(src: &str) -> (R, String) {
    crate::common::run(src)
}
