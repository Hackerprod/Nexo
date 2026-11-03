// Independent typed grammar for deferred and concrete program twins.
// Extras vs earlier rounds: typed random RHS expressions, aliases (var w=r), 2-arg labeled calls, return contexts,
// nested assignment placement (if/while wrappers), adjacent-assignment permutation, run-equivalence of accepted twins.
use nexo::diagnostic::{DataValue, Diagnostic};
use nexo::ir::{CompileError, check_scalar, compile_scalar};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        let mut r = Rng(seed.wrapping_mul(0x9E6C63D0676A9A99) ^ 0xA0761D6478BD642F);
        for _ in 0..6 {
            r.next();
        }
        r
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D) >> 3
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn chance(&mut self, p: u64, of: u64) -> bool {
        self.below(of) < p
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Ty {
    I,
    B,
    A(Box<Ty>),
}
fn ai() -> Ty {
    Ty::A(Box::new(Ty::I))
}
fn ab() -> Ty {
    Ty::A(Box::new(Ty::B))
}
fn aai() -> Ty {
    Ty::A(Box::new(ai()))
}
fn aab() -> Ty {
    Ty::A(Box::new(ab()))
}
fn tname(t: &Ty) -> String {
    match t {
        Ty::I => "I64".into(),
        Ty::B => "Bool".into(),
        Ty::A(u) => format!("Array<{}>", tname(u)),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Op {
    Add,
    Sub,
    Lt,
    Eq,
}
impl Op {
    fn s(&self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "-",
            Op::Lt => "<",
            Op::Eq => "==",
        }
    }
}

#[derive(Clone, Debug)]
enum E {
    Lit(&'static str),
    Pv(usize),
    Bin(Op, Box<E>, Box<E>),
    If(Box<E>, Box<E>, Box<E>, bool),
    Arr(Vec<E>),
    Idx(Box<E>, Box<E>),
    Len(Box<E>),
    Call(usize, Box<E>),
    Call2(Box<E>, Box<E>),
    Grp(Box<E>),
}
const DV: &str = "(if true then {return;} else {return;})";
const PN: [&str; 6] = ["r", "q", "s", "w0", "w1", "w2"];

fn lit_ty(s: &str) -> Ty {
    match s {
        "x" | "1" | "0" | "7" => Ty::I,
        "p" | "true" | "false" => Ty::B,
        "ar" => ai(),
        "br" => ab(),
        "aar" => aai(),
        "bbr" => aab(),
        _ => Ty::I,
    }
}
const LITS: [&str; 10] = ["x", "1", "0", "7", "p", "true", "false", "ar", "br", "aar"];

#[derive(Clone, Debug)]
enum S {
    Let(Option<Ty>, E),
    Assert(E, E),
    Cond(E),
    While(E),
    Print(E),
    G(usize, E),
    Ret(E),
}
const GT: [&str; 5] = [
    "I64",
    "Bool",
    "Array<I64>",
    "Array<Bool>",
    "Array<Array<I64>>",
];

#[derive(Clone, Debug)]
struct Prog {
    ret: Option<Ty>,
    ptys: Vec<Ty>,             // all pending vars incl aliases
    alias: Vec<Option<usize>>, // alias[j]=Some(i): var wj=var i
    dvinit: Vec<bool>,
    rhs: Vec<E>, // for non-alias vars (indexed same as ptys; aliases hold Lit("0") unused)
    pos: Vec<usize>,
    place: Vec<u8>,
    stmts: Vec<S>,
    bd: u8,
}

struct Gn<'a> {
    rg: &'a mut Rng,
    pts: Vec<Ty>,
    pe: u64,
    nf: u32,
    maxf: u32,
    dvp: u64,
    allow_pend: bool,
    dvroot: Vec<bool>,
    bd: u8,
}
fn rand_ty(rg: &mut Rng) -> Ty {
    match rg.below(10) {
        0 | 1 => Ty::I,
        2 | 3 => Ty::B,
        4 | 5 | 6 => ai(),
        7 => ab(),
        8 => aai(),
        _ => aab(),
    }
}
fn dep(e: &E) -> bool {
    match e {
        E::Pv(_) => true,
        E::Idx(b, _) => dep(b),
        E::If(_, a, b, _) => dep(a) || dep(b),
        E::Arr(v) => v.iter().any(dep),
        E::Grp(x) => dep(x),
        _ => false,
    }
}
impl Gn<'_> {
    fn leaf(&mut self, t: &Ty) -> E {
        let mut c: Vec<E> = vec![];
        if self.allow_pend && self.bd != 1 {
            for (i, pt) in self.pts.iter().enumerate() {
                if pt == t {
                    c.push(E::Pv(i));
                    c.push(E::Pv(i));
                    c.push(E::Pv(i));
                }
            }
        }
        for l in LITS {
            if lit_ty(l) == *t {
                c.push(E::Lit(l));
            }
        }
        if *t == aab() {
            c.push(E::Lit("bbr"));
        }
        if let Ty::A(u) = t {
            c.push(E::Arr(vec![self.leaf_simple(u)]));
        }
        if self.rg.chance(self.dvp, 40) {
            c.push(E::Lit(DV));
        }
        let n = c.len() as u64;
        c.swap_remove(self.rg.below(n) as usize)
    }
    fn pend_form(&mut self, t: &Ty, d: u32, restr: bool) -> Option<E> {
        if self.bd != 0 {
            let n = self.pts.len();
            let i = self.rg.below(n as u64) as usize;
            let j = self.rg.below(n as u64) as usize;
            return match t {
                Ty::I => {
                    if self.bd == 2 && self.rg.chance(1, 2) {
                        Some(E::Pv(i))
                    } else if self.bd == 1 || self.rg.chance(1, 2) {
                        Some(E::Len(Box::new(E::Pv(i))))
                    } else {
                        None
                    }
                }
                Ty::B => Some(E::Bin(Op::Eq, Box::new(E::Pv(i)), Box::new(E::Pv(j)))),
                _ => None,
            };
        }
        let n = self.pts.len();
        let i = self.rg.below(n as u64) as usize;
        let pt = self.pts[i].clone();
        match self.rg.below(5) {
            0 => {
                if *t == Ty::I {
                    return Some(E::Len(Box::new(E::Pv(i))));
                }
            }
            1 => {
                if *t == Ty::B {
                    let other = self.mk(&pt, d - 1, true);
                    return Some(E::Bin(Op::Eq, Box::new(E::Pv(i)), Box::new(other)));
                }
            }
            2 | 3 => {
                let mut cur = pt.clone();
                let mut j = 0;
                while let Ty::A(u) = cur.clone() {
                    cur = *u;
                    j += 1;
                    if cur == *t {
                        let mut e = E::Pv(i);
                        for _ in 0..j {
                            let s = self.mk(&Ty::I, d - 1, restr);
                            e = E::Idx(Box::new(e), Box::new(s));
                        }
                        return Some(e);
                    }
                }
            }
            _ => {
                if pt == ai() {
                    let k = match t {
                        Ty::I => Some(0),
                        Ty::B => Some(1),
                        t if *t == ai() => Some(2),
                        t if *t == ab() => Some(3),
                        t if *t == aai() => Some(4),
                        _ => None,
                    };
                    if let Some(k) = k {
                        return Some(E::Call(k, Box::new(E::Pv(i))));
                    }
                }
                if let Ty::A(u) = t {
                    if **u == pt {
                        return Some(E::Arr(vec![E::Pv(i)]));
                    }
                }
            }
        }
        None
    }
    fn leaf_simple(&mut self, t: &Ty) -> E {
        match t {
            Ty::I => E::Lit(["x", "1", "7"][self.rg.below(3) as usize]),
            Ty::B => E::Lit(["p", "true", "false"][self.rg.below(3) as usize]),
            Ty::A(u) => E::Arr(vec![self.leaf_simple(u)]),
        }
    }
    fn mk(&mut self, t: &Ty, d: u32, restr: bool) -> E {
        if !restr && self.nf < self.maxf && self.rg.chance(self.pe, 1000) {
            self.nf += 1;
            if self.bd == 0 && matches!(t, Ty::I | Ty::B) && self.allow_pend && self.rg.chance(1, 3)
            {
                let c: Vec<usize> = (0..self.pts.len())
                    .filter(|i| matches!(self.pts[*i], Ty::A(_)) && !self.dvroot[*i])
                    .collect();
                if !c.is_empty() {
                    let i = c[self.rg.below(c.len() as u64) as usize];
                    return E::Pv(i);
                }
            }
            let mut w = rand_ty(self.rg);
            while w == *t {
                w = rand_ty(self.rg);
            }
            let save = self.allow_pend;
            self.allow_pend = false;
            let e = self.gen_ok(&w, d.min(2), true);
            self.allow_pend = save;
            return e;
        }
        self.gen_ok(t, d, restr)
    }
    fn gen_ok(&mut self, t: &Ty, d: u32, restr: bool) -> E {
        if d > 0 && self.allow_pend && self.rg.chance(40, 100) {
            if let Some(e) = self.pend_form(t, d, restr) {
                return e;
            }
        }
        if d == 0 || self.rg.chance(2, 9) {
            return self.leaf(t);
        }
        let k = self.rg.below(14);
        match (t, k) {
            (Ty::I, 0 | 1 | 2) => {
                let op = if k == 1 { Op::Sub } else { Op::Add };
                E::Bin(
                    op,
                    Box::new(self.mk(&Ty::I, d - 1, restr)),
                    Box::new(self.mk(&Ty::I, d - 1, restr)),
                )
            }
            (Ty::I, 3 | 4) => {
                let u = rand_ty(self.rg);
                let u = if u == Ty::I && self.rg.chance(1, 2) {
                    ai()
                } else {
                    u
                };
                E::Len(Box::new(self.mk(&Ty::A(Box::new(u)), d - 1, restr)))
            }
            (Ty::I, 5) => E::Call(0, Box::new(self.mk(&ai(), d - 1, restr))),
            (Ty::B, 0 | 1 | 2) => E::Bin(
                Op::Lt,
                Box::new(self.mk(&Ty::I, d - 1, restr)),
                Box::new(self.mk(&Ty::I, d - 1, restr)),
            ),
            (Ty::B, 3 | 4) => {
                let u = rand_ty(self.rg);
                let a = self.mk(&u, d - 1, restr);
                let r2 = restr || dep(&a);
                E::Bin(Op::Eq, Box::new(a), Box::new(self.mk(&u, d - 1, r2)))
            }
            (Ty::B, 5) => E::Call(1, Box::new(self.mk(&ai(), d - 1, restr))),
            (Ty::B, 12) => E::Call2(
                Box::new(self.mk(&ai(), d - 1, restr)),
                Box::new(self.mk(&Ty::I, d - 1, restr)),
            ),
            (Ty::I, 12) => E::Len(Box::new(self.mk(&ai(), d - 1, restr))),
            (_, 6 | 7) => {
                let c = self.mk(&Ty::B, d - 1, restr);
                let a = self.mk(t, d - 1, restr);
                let r2 = restr;
                let b = self.mk(t, d - 1, r2);
                E::If(Box::new(c), Box::new(a), Box::new(b), self.rg.chance(1, 3))
            }
            (_, 8 | 9) => E::Idx(
                Box::new(self.mk(&Ty::A(Box::new(t.clone())), d - 1, restr)),
                Box::new(self.mk(&Ty::I, d - 1, restr)),
            ),
            (Ty::A(u), 10 | 11) => {
                let n = 1 + self.rg.below(3);
                let mut v: Vec<E> = vec![];
                let mut r2 = restr;
                for _ in 0..n {
                    let e = self.mk(u, d - 1, r2);
                    if dep(&e) {
                        r2 = true;
                    }
                    v.push(e);
                }
                E::Arr(v)
            }
            (Ty::A(_), 5) => match t {
                t if *t == ai() => E::Call(2, Box::new(self.mk(&ai(), d - 1, restr))),
                t if *t == ab() => E::Call(3, Box::new(self.mk(&ai(), d - 1, restr))),
                t if *t == aai() => E::Call(4, Box::new(self.mk(&ai(), d - 1, restr))),
                _ => self.leaf(t),
            },
            (_, 13) => E::Grp(Box::new(self.mk(t, d - 1, restr))),
            _ => self.leaf(t),
        }
    }
    fn rt(&mut self) -> Ty {
        if self.allow_pend && self.rg.chance(1, 2) {
            let i = self.rg.below(self.pts.len() as u64) as usize;
            let mut t = self.pts[i].clone();
            for _ in 0..self.rg.below(3) {
                if let Ty::A(u) = t.clone() {
                    t = *u;
                }
            }
            return t;
        }
        rand_ty(self.rg)
    }
    fn stmt(&mut self, d: u32, in_ret: &Option<Ty>) -> S {
        let _ = in_ret;
        match self.rg.below(20) {
            0..=5 => {
                let t = self.rt();
                S::Let(None, self.mk(&t, d, false))
            }
            6..=8 => {
                let t = self.rt();
                S::Let(Some(t.clone()), self.mk(&t, d, false))
            }
            9..=10 => {
                let t = self.rt();
                let a = self.mk(&t, d, false);
                let r2 = dep(&a);
                S::Assert(a, self.mk(&t, d, r2))
            }
            11 => S::Cond(self.mk(&Ty::B, d, false)),
            12 => S::While(self.mk(&Ty::B, d, false)),
            13 | 14 => {
                let t = self.rt();
                S::Print(self.mk(&t, d, false))
            }
            _ => {
                let k = self.rg.below(5) as usize;
                let t = match k {
                    0 => Ty::I,
                    1 => Ty::B,
                    2 => ai(),
                    3 => ab(),
                    _ => aai(),
                };
                S::G(k, self.mk(&t, d, false))
            }
        }
    }
}

fn clean_rhs(rg: &mut Rng, t: &Ty, d: u32) -> E {
    let mut g = Gn {
        rg,
        pts: vec![],
        pe: 0,
        nf: 0,
        maxf: 0,
        dvp: 0,
        allow_pend: false,
        dvroot: vec![],
        bd: 0,
    };
    g.mk(t, d, true)
}

fn gen_prog(seed: u64, idx: u64, depth: u32) -> Prog {
    let mut rg = Rng::new(
        seed.wrapping_mul(2_654_435_761)
            .wrapping_add(idx)
            .wrapping_mul(7919),
    );
    let nbase = 1 + [0, 0, 1, 1, 1, 2][rg.below(6) as usize];
    let mut ptys: Vec<Ty> = vec![];
    for _ in 0..nbase {
        ptys.push(match rg.below(8) {
            0..=2 => ai(),
            3 | 4 => aai(),
            5 => ab(),
            6 => aab(),
            _ => ai(),
        });
    }
    // aliases
    let mut alias: Vec<Option<usize>> = vec![None; nbase];
    let mut dvinit = vec![false; nbase];
    for i in 0..nbase {
        dvinit[i] = rg.chance(1, 8);
    }
    if rg.chance(1, 4) {
        let src = rg.below(nbase as u64) as usize;
        ptys.push(ptys[src].clone());
        alias.push(Some(src));
        dvinit.push(false);
    }
    let mut rhs: Vec<E> = vec![];
    for (i, t) in ptys.iter().enumerate() {
        if alias[i].is_some() {
            rhs.push(E::Lit("0"));
        } else {
            let dd = rg.below(3) as u32;
            rhs.push(clean_rhs(&mut rg, t, dd));
        }
    }
    let ret = if rg.chance(1, 6) {
        Some(rand_ty(&mut rg))
    } else {
        None
    };
    if ret.is_some() {
        for x in dvinit.iter_mut() {
            *x = false;
        }
    }
    let pe = [0, 10, 25, 60, 120][rg.below(5) as usize];
    let maxf = [1, 1, 1, 2, 3, 5][rg.below(6) as usize];
    let dvp = if rg.chance(1, 5) && ret.is_none() {
        3
    } else {
        0
    };
    let n = 1 + rg.below(3) as usize;
    let mut stmts: Vec<S> = vec![];
    {
        let dvr: Vec<bool> = (0..ptys.len())
            .map(|i| dvinit[i] || alias[i].map(|s| dvinit[s]).unwrap_or(false))
            .collect();
        let mut g = Gn {
            rg: &mut rg,
            pts: ptys.clone(),
            pe,
            nf: 0,
            maxf,
            dvp,
            allow_pend: true,
            dvroot: dvr.clone(),
            bd: 0,
        };
        for _ in 0..n {
            let s = g.stmt(depth, &None);
            stmts.push(s);
        }
        if let Some(t) = &ret {
            let e = g.mk(t, depth, false);
            stmts.push(S::Ret(e));
        }
    }
    let end = stmts.len();
    let together = rg.chance(3, 5);
    let mut pos = vec![];
    for _ in 0..ptys.len() {
        pos.push(if together || rg.chance(1, 2) {
            end - if ret.is_some() { 1 } else { 0 }
        } else {
            rg.below(end as u64 + 1) as usize
        });
    }
    let place: Vec<u8> = (0..ptys.len())
        .map(|_| {
            if rg.chance(1, 4) {
                1 + rg.below(2) as u8
            } else {
                0
            }
        })
        .collect();
    Prog {
        ret,
        ptys,
        alias,
        dvinit,
        rhs,
        pos,
        place,
        stmts,
        bd: 0,
    }
}

// ---------- rendering with node ids
#[derive(Default, Clone)]
struct Sp {
    nodes: HashMap<(u64, u64), String>,
    ops: HashMap<(u64, u64), String>,
    kinds: HashMap<String, &'static str>,
    pre_end: u64,
}
struct W {
    bdrep: bool,
    out: String,
    sp: Sp,
    path: Vec<u16>,
}
impl W {
    fn pos(&self) -> u64 {
        self.out.len() as u64
    }
    fn id(&self) -> String {
        self.path.iter().map(|b| format!("{b}.")).collect()
    }
    fn sub(&mut self, k: u16, e: &E) {
        self.path.push(k);
        self.e(e);
        self.path.pop();
    }
    fn rec(&mut self, st: u64, en: u64, kind: &'static str) {
        let id = self.id();
        self.sp.kinds.insert(id.clone(), kind);
        self.sp.nodes.insert((st, en), id);
    }
    fn e(&mut self, e: &E) {
        let st = self.pos();
        match e {
            E::Lit(s) => {
                self.out += s;
                let en = self.pos();
                self.rec(st, en, "lit");
            }
            E::Pv(i) => {
                if self.bdrep {
                    self.out += "ar";
                } else {
                    self.out += PN[*i];
                }
                let en = self.pos();
                self.rec(st, en, "pv");
            }
            E::Grp(x) => {
                self.out += "(";
                self.sub(0, x);
                self.out += ")";
            }
            E::Bin(op, a, b) => {
                self.out += "(";
                self.sub(0, a);
                self.out += " ";
                let os = self.pos();
                self.out += op.s();
                let oe = self.pos();
                self.sp.ops.insert((os, oe), self.id());
                self.out += " ";
                self.sub(1, b);
                self.out += ")";
                let en = self.pos() - 1;
                self.rec(st + 1, en, "bin");
            }
            E::If(c, a, b, brace) => {
                self.out += "(if ";
                self.sub(0, c);
                if *brace {
                    self.out += " then {value: ";
                    self.sub(1, a);
                    self.out += ";} else {value: ";
                    self.sub(2, b);
                    self.out += ";})";
                } else {
                    self.out += " then ";
                    self.sub(1, a);
                    self.out += " else ";
                    self.sub(2, b);
                    self.out += ")";
                }
                let en = self.pos() - 1;
                self.rec(st + 1, en, "if");
            }
            E::Arr(v) => {
                self.out += "[";
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        self.out += ",";
                    }
                    self.sub(i as u16, x);
                }
                self.out += "]";
                let en = self.pos();
                self.rec(st, en, "arr");
            }
            E::Idx(b, i) => {
                self.sub(0, b);
                self.out += "[";
                self.sub(1, i);
                self.out += "]";
                let en = self.pos();
                self.rec(st, en, "idx");
            }
            E::Len(x) => {
                self.out += "Len(";
                self.sub(0, x);
                self.out += ")";
                let en = self.pos();
                self.rec(st, en, "len");
            }
            E::Call(k, x) => {
                self.out += &format!("H{k}(");
                self.sub(0, x);
                self.out += ")";
                let en = self.pos();
                self.rec(st, en, "call");
            }
            E::Call2(a, b) => {
                self.out += "K(a: ";
                self.sub(0, a);
                self.out += ", b: ";
                self.sub(1, b);
                self.out += ")";
                let en = self.pos();
                self.rec(st, en, "call");
            }
        }
    }
}
struct Rendered {
    src: String,
    sp: Sp,
}
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Deferred,
    Twin,
}
// order: order of emission for assignments sharing the same position; plain: ignore wrappers
fn render(p: &Prog, mode: Mode, order: &[usize], plain: bool) -> Rendered {
    let mut w = W {
        bdrep: false,
        out: String::new(),
        sp: Sp::default(),
        path: vec![],
    };
    for k in 0..5 {
        let gl = ["0", "false", "[0]", "[false]", "[[0]]"][k];
        w.out += &format!(
            "fn G{k}(a:view {}){{}} fn H{k}(a:view Array<I64>)->{}{{return {};}} ",
            GT[k], GT[k], gl
        );
    }
    w.out += "fn K(a:view Array<I64>, b:view I64)->Bool{return false;} ";
    let pre = "let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];let bbr=[[true]];";
    if let Some(t) = &p.ret {
        w.out += &format!("fn Main(){{let z=F();}} fn F()->{}{{{}", tname(t), pre);
    } else {
        w.out += &format!("fn Main(){{{}", pre);
    }
    w.sp.pre_end = w.pos();
    let np = p.ptys.len();
    for i in 0..np {
        match (mode, p.alias[i]) {
            (Mode::Deferred, None) => {
                if p.dvinit[i] {
                    w.out += &format!("var {}={};", PN[i], DV);
                } else {
                    w.out += &format!("var {}=[];", PN[i]);
                }
            }
            (Mode::Deferred, Some(s)) => w.out += &format!("var {}={};", PN[i], PN[s]),
            (Mode::Twin, None) if p.bd == 1 => {}
            (Mode::Twin, None) => {
                w.out += &format!("let {}=", PN[i]);
                w.path = vec![200 + i as u16];
                let e = p.rhs[i].clone();
                w.e(&e);
                w.out += ";";
            }
            (Mode::Twin, Some(s)) => w.out += &format!("let {}={};", PN[i], PN[s]),
        }
    }
    let place = |w: &mut W, at: usize| {
        if mode == Mode::Deferred && p.bd != 1 {
            for &i in order {
                if p.alias[i].is_none() && p.pos[i] == at {
                    let wrap = if plain { 0 } else { p.place[i] };
                    match wrap {
                        1 => w.out += "if p {",
                        2 => w.out += "while false {",
                        _ => {}
                    }
                    w.out += &format!("{}=", PN[i]);
                    w.path = vec![200 + i as u16];
                    let e = p.rhs[i].clone();
                    w.e(&e);
                    w.out += ";";
                    if wrap != 0 {
                        w.out += "}";
                    }
                }
            }
        }
    };
    if p.bd == 1 && mode == Mode::Twin {
        w.bdrep = true;
    }
    for (k, s) in p.stmts.iter().enumerate() {
        place(&mut w, k);
        w.path = vec![100 + k as u16];
        match s {
            S::Let(a, e) => {
                match a {
                    Some(t) => w.out += &format!("let a{k}:{}=", tname(t)),
                    None => w.out += &format!("let a{k}="),
                }
                w.sub(0, e);
                w.out += ";";
            }
            S::Assert(a, b) => {
                w.out += "AssertEq(";
                w.sub(0, a);
                w.out += ",";
                w.sub(1, b);
                w.out += ");";
            }
            S::Cond(e) => {
                w.out += "if ";
                w.sub(0, e);
                w.out += " {}";
            }
            S::While(e) => {
                w.out += "while ";
                w.sub(0, e);
                w.out += " {}";
            }
            S::Print(e) => {
                w.out += "Print(";
                w.sub(0, e);
                w.out += ");";
            }
            S::G(g, e) => {
                w.out += &format!("G{g}(");
                w.sub(0, e);
                w.out += ");";
            }
            S::Ret(e) => {
                w.out += "return ";
                w.sub(0, e);
                w.out += ";";
            }
        }
    }
    place(&mut w, p.stmts.len());
    w.out += "}";
    Rendered {
        src: w.out,
        sp: w.sp,
    }
}

// ---------- checking
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct K {
    code: String,
    sub: String,
    loc: String,
    exp: String,
    found: String,
}
enum R {
    Ok,
    Rej(Vec<(K, u64, u64)>),
    Bug(String),
}
fn gs(d: &Diagnostic, k: &str) -> String {
    match d.data.get(k) {
        Some(DataValue::String(s)) => s.clone(),
        _ => String::new(),
    }
}
fn key(d: &Diagnostic, sp: &Sp, src: &str) -> (K, u64, u64) {
    let (s, e) = d
        .primary_span
        .as_ref()
        .map(|p| (p.start, p.end))
        .unwrap_or((0, 0));
    let loc = if let Some(id) = sp.ops.get(&(s, e)) {
        format!("op:{id}")
    } else if let Some(id) = sp.nodes.get(&(s, e)) {
        format!("n:{id}")
    } else if s < sp.pre_end {
        format!("pre:{}", src.get(s as usize..e as usize).unwrap_or("?"))
    } else {
        format!("o:{}", src.get(s as usize..e as usize).unwrap_or("?"))
    };
    (
        K {
            code: d.code.clone(),
            sub: d.subcause.clone().unwrap_or_default(),
            loc,
            exp: gs(d, "expected_type"),
            found: gs(d, "found_type"),
        },
        s,
        e,
    )
}
fn check(r: &Rendered) -> R {
    match catch_unwind(AssertUnwindSafe(|| check_scalar("t.n", &r.src))) {
        Err(_) => R::Bug("panic".into()),
        Ok(Ok(_)) => R::Ok,
        Ok(Err(CompileError::Rejected(v))) => {
            R::Rej(v.iter().map(|d| key(d, &r.sp, &r.src)).collect())
        }
        Ok(Err(CompileError::InvalidIr(m))) => R::Bug(format!("InvalidIr:{m}")),
        Ok(Err(_)) => R::Bug("other".into()),
    }
}
fn run_obs(src: &str) -> Result<String, String> {
    if src.contains("while ") {
        return Ok(String::new());
    }
    match catch_unwind(AssertUnwindSafe(|| match compile_scalar("t.n", src) {
        Ok(p) => {
            let ex = nexo::interp::run_scalar(&p);
            if let nexo::interp::Outcome::InternalError(m) = ex.outcome {
                Err(format!("run internal:{m}"))
            } else {
                Ok(format!("{:?}|{:?}", ex.outcome, ex.observations))
            }
        }
        Err(CompileError::Rejected(_)) => Ok(String::new()),
        Err(CompileError::InvalidIr(m)) => Err(format!("InvalidIr(run):{m}")),
        Err(_) => Err("other(run)".into()),
    })) {
        Err(_) => Err("panic(run)".into()),
        Ok(x) => x,
    }
}

type M = BTreeMap<K, usize>;
fn proj(r: &R, l2: bool) -> M {
    let mut m = BTreeMap::new();
    if let R::Rej(v) = r {
        for (k, _, _) in v {
            if k.loc.starts_with("pre:") {
                continue;
            }
            let mut k2 = k.clone();
            if !l2 {
                k2.exp.clear();
                k2.found.clear();
            }
            *m.entry(k2).or_insert(0) += 1;
        }
    }
    m
}
fn locid(k: &K) -> Option<&str> {
    k.loc.split_once(':').map(|x| x.1)
}
// classification of a diff at k, given all loc ids diagnosed on either side and node kinds
fn classify(k: &K, all: &BTreeSet<String>, kinds: &HashMap<String, &'static str>) -> &'static str {
    let Some(id) = locid(k) else { return "U" };
    if k.loc.starts_with("o:") {
        return "U";
    }
    // V candidate: some other diagnosed node is a strict descendant of this one
    for o in all {
        if o.len() > id.len() && o.starts_with(id) {
            return "V";
        }
    }
    // V candidate: selector of an idx whose base subtree already has a diagnostic
    {
        let comps: Vec<&str> = id.split('.').filter(|c| !c.is_empty()).collect();
        if comps.len() > 1 && comps[comps.len() - 1] == "1" {
            let parent: String = comps[..comps.len() - 1]
                .iter()
                .map(|c| format!("{c}."))
                .collect();
            if kinds.get(&parent).copied() == Some("idx") {
                let base = format!("{parent}0.");
                if all.iter().any(|o| o.starts_with(&base)) {
                    return "V";
                }
            }
        }
    }
    // D candidate: parent is an if/arr and a sibling subtree also has a diagnostic
    let comps: Vec<&str> = id.split('.').filter(|c| !c.is_empty()).collect();
    if comps.len() > 1 {
        let parent: String = comps[..comps.len() - 1]
            .iter()
            .map(|c| format!("{c}."))
            .collect();
        let pk = kinds.get(&parent).copied().unwrap_or("");
        if pk == "if" || pk == "arr" {
            for o in all {
                if o != id
                    && o.starts_with(&parent)
                    && !o.starts_with(id)
                    && !id.starts_with(o.as_str())
                {
                    return "D";
                }
            }
        }
    }
    "A"
}
#[derive(Debug, Clone)]
struct Diff {
    kind: &'static str,
    k: K,
    cls: &'static str,
}
struct Res {
    diffs: Vec<Diff>,
    bug: Option<String>,
    dup: bool,
    accepted_d: bool,
    accepted_t: bool,
}
fn locs(m: &M) -> BTreeSet<String> {
    m.keys()
        .filter_map(|k| locid(k).map(|s| s.to_string()))
        .collect()
}
fn adjacent_orders(p: &Prog) -> Vec<Vec<usize>> {
    let n = p.ptys.len();
    let id: Vec<usize> = (0..n).collect();
    let mut rev = id.clone();
    rev.reverse();
    let mut v = vec![id];
    if n >= 2 {
        v.push(rev);
    }
    v
}
fn compare(p: &Prog, l2: bool) -> Res {
    let ord = adjacent_orders(p);
    let d = render(p, Mode::Deferred, &ord[0], false);
    let t = render(p, Mode::Twin, &ord[0], false);
    let rd = check(&d);
    let rt = check(&t);
    let mut res = Res {
        diffs: vec![],
        bug: None,
        dup: false,
        accepted_d: matches!(rd, R::Ok),
        accepted_t: matches!(rt, R::Ok),
    };
    for (nm, r) in [("deferred", &rd), ("twin", &rt)] {
        if let R::Bug(m) = r {
            res.bug = Some(format!("{nm}:{m}"));
            return res;
        }
    }
    // duplicates per site
    for r in [&rd, &rt] {
        if let R::Rej(v) = r {
            let mut seen = BTreeSet::new();
            for (k, s, e) in v {
                if !seen.insert((s, e)) && !k.loc.starts_with("pre:") {
                    res.dup = true;
                }
            }
        }
    }
    // runs
    let skip_run = d.src.contains("while ") || d.src.contains("{return;}") || d.src.len() % 4 != 0;
    let od = if res.accepted_d && !skip_run {
        run_obs(&d.src)
    } else {
        Ok(String::new())
    };
    let ot = if res.accepted_t && !skip_run {
        run_obs(&t.src)
    } else {
        Ok(String::new())
    };
    match (&od, &ot) {
        (Err(m), _) => {
            res.bug = Some(format!("deferred:{m}"));
            return res;
        }
        (_, Err(m)) => {
            res.bug = Some(format!("twin:{m}"));
            return res;
        }
        (Ok(a), Ok(b)) => {
            if false && res.accepted_d && res.accepted_t && !skip_run && a != b {
                res.diffs.push(Diff {
                    kind: "run_diff",
                    k: K {
                        code: "run".into(),
                        sub: String::new(),
                        loc: String::new(),
                        exp: String::new(),
                        found: String::new(),
                    },
                    cls: "A",
                });
            }
        }
    }
    let md = proj(&rd, l2);
    let mt = proj(&rt, l2);
    if res.accepted_d != res.accepted_t {
        res.diffs.push(Diff {
            kind: if res.accepted_d {
                "accept_d_reject_t"
            } else {
                "reject_d_accept_t"
            },
            k: md.keys().chain(mt.keys()).next().cloned().unwrap_or(K {
                code: "acc".into(),
                sub: String::new(),
                loc: String::new(),
                exp: String::new(),
                found: String::new(),
            }),
            cls: "A",
        });
    }
    let mut all = locs(&md);
    all.extend(locs(&mt));
    let mut kinds = d.sp.kinds.clone();
    kinds.extend(t.sp.kinds.clone());
    for (k, v) in &md {
        if mt.get(k).copied().unwrap_or(0) < *v {
            res.diffs.push(Diff {
                kind: "extra_in_deferred",
                k: k.clone(),
                cls: classify(k, &all, &kinds),
            });
        }
    }
    for (k, v) in &mt {
        if md.get(k).copied().unwrap_or(0) < *v {
            res.diffs.push(Diff {
                kind: "missing_in_deferred",
                k: k.clone(),
                cls: classify(k, &all, &kinds),
            });
        }
    }
    // metamorphic: plain placement
    if p.place.iter().any(|&x| x != 0) {
        let d2 = render(p, Mode::Deferred, &ord[0], true);
        let r2 = check(&d2);
        if let R::Bug(m) = &r2 {
            res.bug = Some(format!("plain:{m}"));
            return res;
        }
        let m2 = proj(&r2, l2);
        if matches!(r2, R::Ok) != res.accepted_d {
            res.diffs.push(Diff {
                kind: "place_accept",
                k: K {
                    code: "place".into(),
                    sub: String::new(),
                    loc: String::new(),
                    exp: String::new(),
                    found: String::new(),
                },
                cls: "A",
            });
        } else if m2 != md {
            diff_maps(&mut res, "place_diff", &md, &m2, &kinds);
        }
    }
    // metamorphic: adjacent assignment permutation (only meaningful for assignments sharing a position)
    if ord.len() > 1 {
        let same_pos = (0..p.ptys.len()).any(|i| {
            (0..p.ptys.len()).any(|j| {
                i != j && p.alias[i].is_none() && p.alias[j].is_none() && p.pos[i] == p.pos[j]
            })
        });
        if same_pos {
            let d2 = render(p, Mode::Deferred, &ord[1], false);
            let r2 = check(&d2);
            if let R::Bug(m) = &r2 {
                res.bug = Some(format!("perm:{m}"));
                return res;
            }
            if matches!(r2, R::Ok) != res.accepted_d {
                res.diffs.push(Diff {
                    kind: "perm_accept",
                    k: K {
                        code: "perm".into(),
                        sub: String::new(),
                        loc: String::new(),
                        exp: String::new(),
                        found: String::new(),
                    },
                    cls: "A",
                });
            } else {
                let m2 = proj(&r2, l2);
                if m2 != md {
                    diff_maps(&mut res, "perm_diff", &md, &m2, &kinds);
                }
            }
        }
    }
    res
}
fn diff_maps(
    res: &mut Res,
    kind: &'static str,
    a: &M,
    b: &M,
    kinds: &HashMap<String, &'static str>,
) {
    let mut all = locs(a);
    all.extend(locs(b));
    for (k, v) in a {
        if b.get(k).copied().unwrap_or(0) != *v {
            res.diffs.push(Diff {
                kind,
                k: k.clone(),
                cls: classify(k, &all, kinds),
            });
        }
    }
    for (k, v) in b {
        if a.get(k).copied().unwrap_or(0) != *v {
            res.diffs.push(Diff {
                kind,
                k: k.clone(),
                cls: classify(k, &all, kinds),
            });
        }
    }
}

// ---------- shrinking
fn children(e: &E) -> Vec<E> {
    match e {
        E::Lit(_) | E::Pv(_) => vec![],
        E::Grp(x) | E::Len(x) | E::Call(_, x) => vec![(**x).clone()],
        E::Bin(_, a, b) | E::Idx(a, b) | E::Call2(a, b) => vec![(**a).clone(), (**b).clone()],
        E::If(c, a, b, _) => vec![(**c).clone(), (**a).clone(), (**b).clone()],
        E::Arr(v) => v.clone(),
    }
}
fn with_child(e: &E, i: usize, n: E) -> E {
    let mut e = e.clone();
    match &mut e {
        E::Grp(x) | E::Len(x) | E::Call(_, x) => **x = n,
        E::Bin(_, a, b) | E::Idx(a, b) | E::Call2(a, b) => {
            if i == 0 {
                **a = n
            } else {
                **b = n
            }
        }
        E::If(c, a, b, _) => match i {
            0 => **c = n,
            1 => **a = n,
            _ => **b = n,
        },
        E::Arr(v) => v[i] = n,
        _ => {}
    }
    e
}
fn shrink_e(e: &E, out: &mut Vec<E>) {
    for c in children(e) {
        out.push(c);
    }
    if let E::Arr(v) = e {
        if v.len() > 1 {
            for i in 0..v.len() {
                let mut w = v.clone();
                w.remove(i);
                out.push(E::Arr(w));
            }
        }
    }
    if !matches!(e, E::Lit(_)) {
        out.push(E::Lit("1"));
        out.push(E::Lit("p"));
        out.push(E::Lit("ar"));
    }
    for (i, c) in children(e).iter().enumerate() {
        let mut sub = vec![];
        shrink_e(c, &mut sub);
        for s in sub {
            out.push(with_child(e, i, s));
        }
    }
}
fn s_exprs(s: &S) -> Vec<&E> {
    match s {
        S::Let(_, e) | S::Cond(e) | S::While(e) | S::Print(e) | S::G(_, e) | S::Ret(e) => vec![e],
        S::Assert(a, b) => vec![a, b],
    }
}
fn s_with(s: &S, i: usize, n: E) -> S {
    match s.clone() {
        S::Let(t, _) => S::Let(t, n),
        S::Cond(_) => S::Cond(n),
        S::While(_) => S::While(n),
        S::Print(_) => S::Print(n),
        S::G(g, _) => S::G(g, n),
        S::Ret(_) => S::Ret(n),
        S::Assert(a, b) => {
            if i == 0 {
                S::Assert(n, b)
            } else {
                S::Assert(a, n)
            }
        }
    }
}
fn shrink_p(p: &Prog) -> Vec<Prog> {
    let mut out = vec![];
    if p.stmts.len() > 1 {
        for i in 0..p.stmts.len() {
            if matches!(p.stmts[i], S::Ret(_)) {
                continue;
            }
            let mut q = p.clone();
            q.stmts.remove(i);
            for ps in q.pos.iter_mut() {
                if *ps > i {
                    *ps -= 1;
                }
            }
            out.push(q);
        }
    }
    for (si, s) in p.stmts.iter().enumerate() {
        for (ei, e) in s_exprs(s).into_iter().enumerate() {
            let mut sh = vec![];
            shrink_e(e, &mut sh);
            for n in sh {
                let mut q = p.clone();
                q.stmts[si] = s_with(s, ei, n);
                out.push(q);
            }
        }
        if let S::Let(Some(_), e) = s {
            let mut q = p.clone();
            q.stmts[si] = S::Let(None, e.clone());
            out.push(q);
        }
    }
    for i in 0..p.ptys.len() {
        if p.alias[i].is_none() {
            let mut sh = vec![];
            shrink_e(&p.rhs[i], &mut sh);
            for n in sh {
                let mut q = p.clone();
                q.rhs[i] = n;
                out.push(q);
            }
        }
        if p.place[i] != 0 {
            let mut q = p.clone();
            q.place[i] = 0;
            out.push(q);
        }
        if p.dvinit[i] {
            let mut q = p.clone();
            q.dvinit[i] = false;
            out.push(q);
        }
    }
    out
}
fn esz(e: &E) -> usize {
    1 + children(e).iter().map(esz).sum::<usize>()
}
fn psz(p: &Prog) -> usize {
    p.stmts.len() * 10
        + p.stmts.iter().flat_map(s_exprs).map(esz).sum::<usize>()
        + p.rhs.iter().map(esz).sum::<usize>()
        + p.ptys.len()
}
fn matches_sig(p: &Prog, sig: &(String, String, String, String), l2: bool) -> bool {
    if !consistent(p) {
        return false;
    }
    let r = compare(p, l2);
    if let Some(b) = &r.bug {
        return sig.0 == "bug" && b.split(':').take(2).collect::<Vec<_>>().join(":") == sig.1;
    }
    if sig.0 == "dup" {
        return r.dup;
    }
    r.diffs.iter().any(|d| {
        d.kind == sig.0
            && d.k.code == sig.1
            && d.k.sub == sig.2
            && (sig.3.is_empty() || d.cls == sig.3)
    })
}
fn reduce(p: &Prog, sig: &(String, String, String, String), l2: bool) -> Prog {
    let mut cur = p.clone();
    let mut budget = 3000;
    loop {
        let mut improved = false;
        let mut cands = shrink_p(&cur);
        cands.sort_by_key(psz);
        for c in cands {
            if psz(&c) >= psz(&cur)
                && !(c.place.iter().sum::<u8>() < cur.place.iter().sum::<u8>())
                && c.dvinit.iter().filter(|x| **x).count()
                    >= cur.dvinit.iter().filter(|x| **x).count()
            {
                continue;
            }
            budget -= 1;
            if budget <= 0 {
                return cur;
            }
            if matches_sig(&c, sig, l2) {
                cur = c;
                improved = true;
                break;
            }
        }
        if !improved {
            return cur;
        }
    }
}
fn fmt_r(r: &R) -> String {
    match r {
        R::Ok => "OK".into(),
        R::Bug(m) => format!("BUG {m}"),
        R::Rej(v) => v
            .iter()
            .map(|(k, _, _)| {
                format!(
                    "{} {} @{} exp={} found={}",
                    k.code, k.sub, k.loc, k.exp, k.found
                )
            })
            .collect::<Vec<_>>()
            .join(" || "),
    }
}
fn show(p: &Prog) -> String {
    let ord = adjacent_orders(p);
    let d = render(p, Mode::Deferred, &ord[0], false);
    let t = render(p, Mode::Twin, &ord[0], false);
    let body = |s: &str| {
        s.split("fn K(")
            .nth(1)
            .map(|x| {
                x.split_once("{return false;} ")
                    .map(|y| y.1)
                    .unwrap_or(x)
                    .to_string()
            })
            .unwrap_or_default()
    };
    let mut s = format!(
        "  DEF: {}\n  TWN: {}\n  d: {}\n  t: {}\n",
        body(&d.src),
        body(&t.src),
        fmt_r(&check(&d)),
        fmt_r(&check(&t))
    );
    if ord.len() > 1 {
        let d2 = render(p, Mode::Deferred, &ord[1], false);
        s += &format!("  PERM: {}\n  p: {}\n", body(&d2.src), fmt_r(&check(&d2)));
    }
    s
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(|s| s.as_str()) {
        Some("camp") => {
            let seed: u64 = a[2].parse().unwrap();
            let n: u64 = a[3].parse().unwrap();
            let depth: u32 = a[4].parse().unwrap();
            let l2 = a.get(5).map(|s| s == "l2").unwrap_or(false);
            let bdm: u8 = a.get(6).map(|s| s.parse().unwrap_or(0)).unwrap_or(0);
            let mut nbug = 0;
            let mut ndup = 0;
            let mut acc = 0;
            let mut rej_both = 0;
            let mut cls_count: BTreeMap<String, usize> = BTreeMap::new();
            let mut examples: BTreeMap<(String, String, String, String), (Prog, usize)> =
                BTreeMap::new();
            let mut syn = 0;
            let mut skipped = 0;
            for i in 0..n {
                let p = if bdm != 0 {
                    gen_prog_bd(seed, i, depth, bdm)
                } else {
                    gen_prog(seed, i, depth)
                };
                if !consistent(&p) {
                    skipped += 1;
                    continue;
                }
                let r = compare(&p, l2);
                if r.accepted_d && r.accepted_t {
                    acc += 1;
                }
                if !r.accepted_d && !r.accepted_t {
                    rej_both += 1;
                }
                if r.dup {
                    ndup += 1;
                    examples
                        .entry(("dup".into(), String::new(), String::new(), String::new()))
                        .or_insert((p.clone(), 0))
                        .1 += 1;
                }
                if let Some(b) = &r.bug {
                    nbug += 1;
                    let sg = (
                        "bug".to_string(),
                        b.split(':').take(2).collect::<Vec<_>>().join(":"),
                        String::new(),
                        String::new(),
                    );
                    let e = examples.entry(sg).or_insert((p.clone(), 0));
                    e.1 += 1;
                    continue;
                }
                let mut seen = BTreeSet::new();
                for d in &r.diffs {
                    *cls_count
                        .entry(format!("{}:{}", d.kind, d.cls))
                        .or_insert(0) += 1;
                    if d.k.loc.starts_with("o:") && d.k.code.starts_with("NX-E00") {
                        syn += 1;
                    }
                    if d.cls == "A" || d.cls == "U" {
                        let sg = (
                            d.kind.to_string(),
                            d.k.code.clone(),
                            d.k.sub.clone(),
                            d.cls.to_string(),
                        );
                        if seen.insert(sg.clone()) {
                            let e = examples.entry(sg).or_insert((p.clone(), 0));
                            e.1 += 1;
                            if psz(&p) < psz(&e.0) {
                                e.0 = p.clone();
                            }
                        }
                    }
                }
            }
            println!(
                "seed={seed} n={n} depth={depth} acc_both={acc} rej_both={rej_both} bugs={nbug} dup={ndup} syn={syn} skipped={skipped}"
            );
            for (k, v) in &cls_count {
                println!("  {k}: {v}");
            }
            for (sg, (p, c)) in &examples {
                let rp = reduce(p, sg, l2);
                println!("SIG {:?} count={c}\n{}", sg, show(&rp));
            }
        }
        Some("sum") => {
            let src = std::fs::read_to_string(&a[2]).unwrap();
            let r = Rendered {
                src: src.clone(),
                sp: Sp::default(),
            };
            match check(&r) {
                R::Ok => println!("OK"),
                R::Bug(m) => println!("BUG {m}"),
                R::Rej(v) => {
                    for (k, s, e) in v {
                        println!(
                            "{} {} [{}..{}] `{}` exp={} found={}",
                            k.code,
                            k.sub,
                            s,
                            e,
                            &src[s as usize..e as usize],
                            k.exp,
                            k.found
                        );
                    }
                }
            }
        }
        Some("one") => {
            let seed: u64 = a[2].parse().unwrap();
            let i: u64 = a[3].parse().unwrap();
            let depth: u32 = a[4].parse().unwrap();
            let p = gen_prog(seed, i, depth);
            println!("{}", show(&p));
        }
        _ => eprintln!("usage"),
    }
}

// ---------- consistency of pending uses with their final type (twin-equivalence precondition, ADR 4.1.2.7)
#[derive(Clone, PartialEq, Debug)]
enum Tt {
    T(Ty),
    Nv,
    Bad,
}
fn ok_t(t: &Tt, w: &Ty) -> bool {
    matches!(t, Tt::Nv) || *t == Tt::T(w.clone())
}
fn same_or_nv(a: &Tt, b: &Tt) -> bool {
    match (a, b) {
        (Tt::T(x), Tt::T(y)) => x == y,
        (Tt::Nv, _) | (_, Tt::Nv) => true,
        _ => false,
    }
}
fn pair_ok_ab(a: &E, b: &E, p: &Prog) -> bool {
    let (ta, tb) = (tyof(a, p), tyof(b, p));
    same_or_nv(&ta, &tb)
        || (shape_fault(a, Some(&tb), None, p) && !dep(b))
        || (shape_fault(b, Some(&ta), None, p) && !dep(a))
}
fn tyof(e: &E, p: &Prog) -> Tt {
    match e {
        E::Lit(s) => {
            if *s == DV {
                Tt::Nv
            } else {
                Tt::T(lit_ty(s))
            }
        }
        E::Pv(i) => Tt::T(p.ptys[*i].clone()),
        E::Grp(x) => tyof(x, p),
        E::Bin(op, a, b) => {
            let (ta, tb) = (tyof(a, p), tyof(b, p));
            match op {
                Op::Add | Op::Sub => {
                    if ok_t(&ta, &Ty::I) && ok_t(&tb, &Ty::I) {
                        Tt::T(Ty::I)
                    } else {
                        Tt::Bad
                    }
                }
                Op::Lt => {
                    if ok_t(&ta, &Ty::I) && ok_t(&tb, &Ty::I) {
                        Tt::T(Ty::B)
                    } else {
                        Tt::Bad
                    }
                }
                Op::Eq => {
                    if same_or_nv(&ta, &tb) {
                        Tt::T(Ty::B)
                    } else {
                        Tt::Bad
                    }
                }
            }
        }
        E::If(c, a, b, _) => {
            let (tc, ta, tb) = (tyof(c, p), tyof(a, p), tyof(b, p));
            if !ok_t(&tc, &Ty::B) {
                return Tt::Bad;
            }
            match (ta, tb) {
                (Tt::Nv, x) | (x, Tt::Nv) => x,
                (Tt::T(x), Tt::T(y)) if x == y => Tt::T(x),
                _ => Tt::Bad,
            }
        }
        E::Arr(v) => {
            let mut r: Option<Ty> = None;
            for x in v {
                match tyof(x, p) {
                    Tt::T(t) => match &r {
                        None => r = Some(t),
                        Some(u) if *u == t => {}
                        _ => return Tt::Bad,
                    },
                    Tt::Nv => {}
                    Tt::Bad => return Tt::Bad,
                }
            }
            match r {
                Some(t) => Tt::T(Ty::A(Box::new(t))),
                None => Tt::Bad,
            }
        }
        E::Idx(b, i) => {
            let (tb, ti) = (tyof(b, p), tyof(i, p));
            match (tb, ok_t(&ti, &Ty::I)) {
                (Tt::T(Ty::A(u)), true) => Tt::T(*u),
                _ => Tt::Bad,
            }
        }
        E::Len(x) => match tyof(x, p) {
            Tt::T(Ty::A(_)) => Tt::T(Ty::I),
            _ => Tt::Bad,
        },
        E::Call(k, x) => {
            if ok_t(&tyof(x, p), &ai()) {
                Tt::T([Ty::I, Ty::B, ai(), ab(), aai()][*k].clone())
            } else {
                Tt::Bad
            }
        }
        E::Call2(a, b) => {
            if ok_t(&tyof(a, p), &ai()) && ok_t(&tyof(b, p), &Ty::I) {
                Tt::T(Ty::B)
            } else {
                Tt::Bad
            }
        }
    }
}
fn bare_pv(e: &E) -> Option<usize> {
    match e {
        E::Pv(i) => Some(*i),
        E::Grp(x) => bare_pv(x),
        _ => None,
    }
}
fn dvroot(p: &Prog, i: usize) -> bool {
    p.dvinit[i] || p.alias[i].map(|s| p.dvinit[s]).unwrap_or(false)
}
// a non-DV pending array used where a scalar is required: shape conflict decided by the constructor of [] in both modes
fn shape_fault(e: &E, other: Option<&Tt>, exp: Option<&Ty>, p: &Prog) -> bool {
    let Some(i) = bare_pv(e) else { return false };
    if dvroot(p, i) {
        return false;
    }
    if let Some(x) = exp {
        return matches!(x, Ty::I | Ty::B);
    }
    matches!(other, Some(Tt::T(Ty::I)) | Some(Tt::T(Ty::B)))
}
// every dependent (pending-typed) subtree must sit where its final type is acceptable
fn cons(e: &E, exp: Option<&Ty>, p: &Prog) -> bool {
    if shape_fault(e, None, exp, p) {
        return true;
    }
    if dep(e) {
        match (tyof(e, p), exp) {
            (Tt::T(t), Some(x)) if t != *x => return false,
            (Tt::Bad, _) => return false,
            _ => {}
        }
    }
    match e {
        E::Lit(_) | E::Pv(_) => true,
        E::Grp(x) => cons(x, exp, p),
        E::Bin(op, a, b) => match op {
            Op::Add | Op::Sub => cons(a, Some(&Ty::I), p) && cons(b, Some(&Ty::I), p),
            Op::Lt => {
                let pair_ok = if dep(a) || dep(b) {
                    pair_ok_ab(a, b, p)
                } else {
                    true
                };
                pair_ok && cons(a, Some(&Ty::I), p) && cons(b, Some(&Ty::I), p)
            }
            Op::Eq => {
                let pair_ok = if dep(a) || dep(b) {
                    pair_ok_ab(a, b, p)
                } else {
                    true
                };
                pair_ok && cons(a, None, p) && cons(b, None, p)
            }
        },
        E::If(c, a, b, _) => {
            let pair = if dep(a) || dep(b) {
                pair_ok_ab(a, b, p)
            } else {
                true
            };
            pair && cons(c, Some(&Ty::B), p) && cons(a, exp, p) && cons(b, exp, p)
        }
        E::Arr(v) => {
            let anydep = v.iter().any(dep);
            let mut first: Option<Ty> = None;
            let mut ok = true;
            for x in v {
                if let Tt::T(t) = tyof(x, p) {
                    match &first {
                        None => first = Some(t),
                        Some(f) => {
                            if *f != t && anydep {
                                ok = false;
                            }
                        }
                    }
                }
            }
            let elem = match exp {
                Some(Ty::A(u)) => Some((**u).clone()),
                _ => None,
            };
            ok && v.iter().all(|x| cons(x, elem.as_ref(), p))
        }
        E::Idx(b, i) => {
            let eb = exp.map(|t| Ty::A(Box::new(t.clone())));
            cons(b, eb.as_ref(), p) && cons(i, Some(&Ty::I), p)
        }
        E::Len(x) => {
            let okk = !dep(x) || matches!(tyof(x, p), Tt::T(Ty::A(_)) | Tt::Nv);
            okk && cons(x, None, p)
        }
        E::Call(_, x) => cons(x, Some(&ai()), p),
        E::Call2(a, b) => cons(a, Some(&ai()), p) && cons(b, Some(&Ty::I), p),
    }
}
fn consistent(p: &Prog) -> bool {
    if p.bd != 0 {
        return bd_forms_ok(p);
    }
    let g = [Ty::I, Ty::B, ai(), ab(), aai()];
    for s in &p.stmts {
        let ok = match s {
            S::Let(Some(t), e) => cons(e, Some(t), p),
            S::Let(None, e) | S::Print(e) => cons(e, None, p),
            S::Cond(e) | S::While(e) => cons(e, Some(&Ty::B), p),
            S::G(k, e) => cons(e, Some(&g[*k]), p),
            S::Ret(e) => cons(e, p.ret.as_ref(), p),
            S::Assert(a, b) => {
                let pair = if dep(a) || dep(b) {
                    pair_ok_ab(a, b, p)
                } else {
                    true
                };
                pair && cons(a, None, p) && cons(b, None, p)
            }
        };
        if !ok {
            return false;
        }
    }
    for i in 0..p.ptys.len() {
        if p.alias[i].is_none() {
            let t = tyof(&p.rhs[i], p);
            if matches!(t, Tt::Nv) || !ok_t(&t, &p.ptys[i]) {
                return false;
            }
        }
    }
    true
}

// ---------- BD: pending domain that is never resolved (bd=1) or resolved to a scalar (bd=2)
fn bd_e(e: &E, bd: u8) -> bool {
    match e {
        E::Pv(_) => false, // bare Pv is only allowed in the explicit contexts below
        E::Lit(_) => true,
        E::Grp(x) => bd_e(x, bd),
        E::Len(x) => matches!(**x, E::Pv(_)) || bd_e(x, bd),
        E::Bin(op, a, b) => match op {
            Op::Eq if matches!(**a, E::Pv(_)) && matches!(**b, E::Pv(_)) => true,
            Op::Add | Op::Sub | Op::Lt if bd == 2 => bd_i(a, bd) && bd_i(b, bd),
            _ => bd_e(a, bd) && bd_e(b, bd),
        },
        E::If(c, a, b, _) => bd_e(c, bd) && bd_e(a, bd) && bd_e(b, bd),
        E::Arr(v) => v.iter().all(|x| bd_e(x, bd)),
        E::Idx(b, i) => bd_e(b, bd) && if bd == 2 { bd_i(i, bd) } else { bd_e(i, bd) },
        E::Call(_, x) => bd_e(x, bd),
        E::Call2(a, b) => bd_e(a, bd) && bd_e(b, bd),
    }
}
fn bd_i(e: &E, bd: u8) -> bool {
    matches!(e, E::Pv(_)) || bd_e(e, bd)
}
fn bd_forms_ok(p: &Prog) -> bool {
    p.stmts.iter().all(|s| match s {
        S::Print(E::Pv(_)) => true,
        S::Assert(E::Pv(_), E::Pv(_)) => true,
        S::Let(_, e) | S::Print(e) | S::Cond(e) | S::While(e) | S::G(_, e) | S::Ret(e) => {
            bd_e(e, p.bd)
        }
        S::Assert(a, b) => bd_e(a, p.bd) && bd_e(b, p.bd),
    })
}
fn gen_prog_bd(seed: u64, idx: u64, depth: u32, bd: u8) -> Prog {
    let mut rg = Rng::new(
        seed.wrapping_mul(40_503)
            .wrapping_add(idx)
            .wrapping_mul(104_729),
    );
    let n = 1 + rg.below(2) as usize;
    let ty = if bd == 1 { ai() } else { Ty::I };
    let ptys = vec![ty.clone(); n];
    let pe = [0, 20, 60, 150][rg.below(4) as usize];
    let maxf = [1, 1, 2, 3][rg.below(4) as usize];
    let ns = 1 + rg.below(3) as usize;
    let mut stmts = vec![];
    {
        let mut g = Gn {
            rg: &mut rg,
            pts: ptys.clone(),
            pe,
            nf: 0,
            maxf,
            dvp: 0,
            allow_pend: true,
            dvroot: vec![true; n],
            bd,
        };
        for _ in 0..ns {
            stmts.push(g.stmt(depth, &None));
        }
    }
    // explicit domain-only statements
    if rg.chance(1, 3) {
        let i = rg.below(n as u64) as usize;
        let at = rg.below(stmts.len() as u64 + 1) as usize;
        stmts.insert(at, S::Print(E::Pv(i)));
    }
    if rg.chance(1, 4) {
        let i = rg.below(n as u64) as usize;
        let j = rg.below(n as u64) as usize;
        let at = rg.below(stmts.len() as u64 + 1) as usize;
        stmts.insert(at, S::Assert(E::Pv(i), E::Pv(j)));
    }
    let end = stmts.len();
    let pos = (0..n)
        .map(|_| {
            if rg.chance(1, 2) {
                end
            } else {
                rg.below(end as u64 + 1) as usize
            }
        })
        .collect();
    let rhs = (0..n).map(|_| E::Lit("3")).collect();
    Prog {
        ret: None,
        ptys,
        alias: vec![None; n],
        dvinit: vec![true; n],
        rhs,
        pos,
        place: vec![0; n],
        stmts,
        bd,
    }
}
