// Typed and free grammars with independent seeds. Compare deferred
// assignments with concrete initializer twins.
use nexo::diagnostic::{DataValue, Diagnostic};
use nexo::ir::{CompileError, check_scalar, compile_scalar};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        let mut r = Rng(seed.wrapping_mul(0xD1B54A32D192ED03) ^ 0x2545F4914F6CDD1D);
        for _ in 0..4 {
            r.next();
        }
        r
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn chance(&mut self, p: u64, of: u64) -> bool {
        self.below(of) < p
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
    Lf(&'static str),
    Pv(usize),
    Bin(Op, Box<E>, Box<E>),
    If(Box<E>, Box<E>, Box<E>),
    IfB(Box<E>, Box<E>, Box<E>), // branches as {value: e;}
    Arr(Vec<E>),
    Idx(Box<E>, Box<E>),
    Len(Box<E>),
    Call(usize, Box<E>), // H<k>(e) -> returns type k
    Grp(Box<E>),
}
const PN: [&str; 3] = ["r", "q", "s"];
const DV: &str = "(if true then {return;} else {return;})";

#[derive(Clone, Debug)]
enum S {
    Let(Option<&'static str>, E),
    Assert(E, E),
    Cond(E),
    While(E),
    Print(E),
    CallG(usize, E),
    Assign(&'static str, &'static str, E),
    IdxAssign(&'static str, &'static str, E),
    Poison(E),
}
const GT: [&str; 5] = [
    "I64",
    "Bool",
    "Array<I64>",
    "Array<Bool>",
    "Array<Array<I64>>",
];
const GL: [&str; 5] = ["0", "false", "[0]", "[false]", "[[0]]"];

#[derive(Clone, Debug)]
struct Prog {
    npend: usize,
    rhs: Vec<&'static str>,
    pos: Vec<usize>,
    stmts: Vec<S>,
}
const RHS: [&str; 10] = [
    "[3]", "[3]", "[[3]]", "[true]", "[[true]]", "[x]", "[ar]", "ar", "br", "aar",
];

struct Gn<'a> {
    rg: &'a mut Rng,
    np: usize,
    dvp: u64,
}
impl Gn<'_> {
    fn leaf(&mut self) -> E {
        let k = self.rg.below(20);
        match k {
            0..=7 => E::Pv(self.rg.below(self.np as u64) as usize),
            8 => E::Lf("x"),
            9 => E::Lf("p"),
            10 => E::Lf("ar"),
            11 => E::Lf("br"),
            12 => E::Lf("aar"),
            13 => E::Lf("1"),
            14 => E::Lf("true"),
            15 => E::Lf("0"),
            16 => E::Lf("false"),
            17 => E::Lf("ar"),
            18 => E::Lf("x"),
            _ => {
                if self.rg.chance(self.dvp, 10) {
                    E::Lf(DV)
                } else {
                    E::Lf("p")
                }
            }
        }
    }
    fn ex(&mut self, d: u32) -> E {
        if d == 0 || self.rg.chance(2, 9) {
            return self.leaf();
        }
        match self.rg.below(20) {
            0..=6 => {
                let op = [Op::Add, Op::Sub, Op::Lt, Op::Eq, Op::Lt, Op::Eq, Op::Add]
                    [self.rg.below(7) as usize];
                E::Bin(op, Box::new(self.ex(d - 1)), Box::new(self.ex(d - 1)))
            }
            7..=9 => E::If(
                Box::new(self.ex(d - 1)),
                Box::new(self.ex(d - 1)),
                Box::new(self.ex(d - 1)),
            ),
            10 => E::IfB(
                Box::new(self.ex(d - 1)),
                Box::new(self.ex(d - 1)),
                Box::new(self.ex(d - 1)),
            ),
            11..=13 => {
                let n = 1 + self.rg.below(3);
                E::Arr((0..n).map(|_| self.ex(d - 1)).collect())
            }
            14 | 15 => E::Idx(Box::new(self.ex(d - 1)), Box::new(self.ex(d - 1))),
            16 => E::Len(Box::new(self.ex(d - 1))),
            17 => E::Call(self.rg.below(5) as usize, Box::new(self.ex(d - 1))),
            18 => E::Grp(Box::new(self.ex(d - 1))),
            _ => {
                let op = [Op::Lt, Op::Eq, Op::Add][self.rg.below(3) as usize];
                E::Bin(op, Box::new(self.leaf()), Box::new(self.ex(d - 1)))
            }
        }
    }
    fn stmt(&mut self, d: u32) -> S {
        let e = self.ex(d);
        match self.rg.below(20) {
            0..=6 => S::Let(None, e),
            7..=8 => S::Let(Some(GT[self.rg.below(5) as usize]), e),
            9..=10 => S::Assert(e, self.ex(d)),
            11 => S::Cond(e),
            12 => S::While(e),
            13 => S::Print(e),
            14 => S::CallG(self.rg.below(5) as usize, e),
            15 | 16 => {
                let k = self.rg.below(5) as usize;
                S::Assign(GT[k], GL[k], e)
            }
            17 => {
                let k = self.rg.below(5) as usize;
                S::IdxAssign(GT[k], GL[k], e)
            }
            18 => S::Poison(e),
            _ => S::Let(None, e),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Ty {
    I,
    B,
    A(Box<Ty>),
}
fn ty_of_gt(k: usize) -> Ty {
    match k {
        0 => Ty::I,
        1 => Ty::B,
        2 => Ty::A(Box::new(Ty::I)),
        3 => Ty::A(Box::new(Ty::B)),
        _ => Ty::A(Box::new(Ty::A(Box::new(Ty::I)))),
    }
}
fn rhs_ty(r: &str) -> Ty {
    let ai = Ty::A(Box::new(Ty::I));
    match r {
        "[3]" | "[x]" | "ar" => ai,
        "[[3]]" | "[ar]" | "aar" => Ty::A(Box::new(Ty::A(Box::new(Ty::I)))),
        "[true]" | "br" => Ty::A(Box::new(Ty::B)),
        _ => Ty::A(Box::new(Ty::A(Box::new(Ty::B)))),
    }
}
struct Tg<'a> {
    rg: &'a mut Rng,
    pt: Vec<Ty>,
    pe: u64,
    dvp: u64,
    noidx: bool,
    noadd: bool,
    nounion: bool,
}
impl Tg<'_> {
    fn rand_ty(&mut self) -> Ty {
        match self.rg.below(8) {
            0 | 1 => Ty::I,
            2 => Ty::B,
            3 | 4 => ty_of_gt(2),
            5 => ty_of_gt(3),
            _ => ty_of_gt(4),
        }
    }
    fn leaf_of(&mut self, t: &Ty) -> E {
        let mut c: Vec<E> = vec![];
        for (i, pt) in self.pt.iter().enumerate() {
            if pt == t {
                c.push(E::Pv(i));
                c.push(E::Pv(i));
            }
        }
        match t {
            Ty::I => {
                c.push(E::Lf("x"));
                c.push(E::Lf("1"));
                c.push(E::Lf("0"));
            }
            Ty::B => {
                c.push(E::Lf("p"));
                c.push(E::Lf("true"));
                c.push(E::Lf("false"));
            }
            Ty::A(u) => match (**u).clone() {
                Ty::I => {
                    c.push(E::Lf("ar"));
                    c.push(E::Arr(vec![E::Lf("2")]));
                }
                Ty::B => {
                    c.push(E::Lf("br"));
                    c.push(E::Arr(vec![E::Lf("true")]));
                }
                Ty::A(w) => {
                    if *w == Ty::I {
                        c.push(E::Lf("aar"));
                    }
                    c.push(E::Arr(vec![self.leaf_of(&Ty::A(w))]));
                }
            },
        }
        if self.rg.chance(self.dvp, 40) {
            c.push(E::Lf(DV));
        }
        let n = c.len() as u64;
        c.swap_remove(self.rg.below(n) as usize)
    }
    fn gt(&mut self, t: &Ty, d: u32) -> E {
        if self.rg.chance(self.pe, 100) {
            let w = self.rand_ty();
            if w != *t {
                return self.gt_ok(&w, d);
            }
        }
        self.gt_ok(t, d)
    }
    fn gt_ok(&mut self, t: &Ty, d: u32) -> E {
        if d == 0 || self.rg.chance(2, 9) {
            return self.leaf_of(t);
        }
        let mut k = self.rg.below(12);
        if self.noidx && matches!(k, 7 | 8) {
            k = 99;
        }
        if self.noadd && matches!((t, k), (Ty::I, 0..=2)) {
            k = 99;
        }
        if self.nounion && matches!(k, 4..=6) {
            k = 99;
        }
        match (t, k) {
            (Ty::I, 0..=2) => {
                let op = if k == 1 { Op::Sub } else { Op::Add };
                E::Bin(
                    op,
                    Box::new(self.gt(&Ty::I, d - 1)),
                    Box::new(self.gt(&Ty::I, d - 1)),
                )
            }
            (Ty::I, 3) => {
                let u = self.rand_ty();
                E::Len(Box::new(self.gt(&Ty::A(Box::new(u)), d - 1)))
            }
            (Ty::B, 0..=2) => E::Bin(
                Op::Lt,
                Box::new(self.gt(&Ty::I, d - 1)),
                Box::new(self.gt(&Ty::I, d - 1)),
            ),
            (Ty::B, 3) => {
                let u = self.rand_ty();
                E::Bin(
                    Op::Eq,
                    Box::new(self.gt(&u, d - 1)),
                    Box::new(self.gt(&u, d - 1)),
                )
            }
            (_, 4 | 5) => E::If(
                Box::new(self.gt(&Ty::B, d - 1)),
                Box::new(self.gt(t, d - 1)),
                Box::new(self.gt(t, d - 1)),
            ),
            (_, 6) => E::IfB(
                Box::new(self.gt(&Ty::B, d - 1)),
                Box::new(self.gt(t, d - 1)),
                Box::new(self.gt(t, d - 1)),
            ),
            (_, 7 | 8) => E::Idx(
                Box::new(self.gt(&Ty::A(Box::new(t.clone())), d - 1)),
                Box::new(self.gt(&Ty::I, d - 1)),
            ),
            (Ty::A(u), 9 | 10) => {
                let n = 1 + self.rg.below(3);
                E::Arr((0..n).map(|_| self.gt(u, d - 1)).collect())
            }
            (_, 11) => E::Grp(Box::new(self.gt(t, d - 1))),
            _ => self.leaf_of(t),
        }
    }
}
fn gen_typed(rg: &mut Rng, rhs: &[&'static str], depth: u32, pe: u64, dvp: u64) -> Vec<S> {
    let pt: Vec<Ty> = rhs.iter().map(|r| rhs_ty(r)).collect();
    let noidx = std::env::var("NOIDX").is_ok();
    let noadd = std::env::var("NOADD").is_ok();
    let nounion = std::env::var("NOUNION").is_ok();
    let mut g = Tg {
        rg,
        pt,
        pe,
        dvp,
        noidx,
        noadd,
        nounion,
    };
    let n = 1 + g.rg.below(3) as usize;
    (0..n)
        .map(|_| match g.rg.below(20) {
            0..=5 => {
                let t = g.rand_ty();
                S::Let(None, g.gt(&t, depth))
            }
            6..=8 => {
                let k = g.rg.below(5) as usize;
                let t = ty_of_gt(k);
                S::Let(Some(GT[k]), g.gt(&t, depth))
            }
            9..=10 => {
                let t = g.rand_ty();
                let a = g.gt(&t, depth);
                let b = g.gt(&t, depth);
                S::Assert(a, b)
            }
            11 => S::Cond(g.gt(&Ty::B, depth)),
            12 => S::While(g.gt(&Ty::B, depth)),
            13 => {
                let t = g.rand_ty();
                S::Print(g.gt(&t, depth))
            }
            14 => {
                let k = g.rg.below(5) as usize;
                S::CallG(k, g.gt(&ty_of_gt(k), depth))
            }
            15 | 16 => {
                let k = g.rg.below(5) as usize;
                S::Assign(GT[k], GL[k], g.gt(&ty_of_gt(k), depth))
            }
            17 => {
                let k = g.rg.below(5) as usize;
                S::IdxAssign(GT[k], GL[k], g.gt(&ty_of_gt(k), depth))
            }
            18 => {
                let t = g.rand_ty();
                S::Poison(g.gt(&t, depth))
            }
            _ => {
                let t = g.rand_ty();
                S::Let(None, g.gt(&t, depth))
            }
        })
        .collect()
}
fn gen_prog(seed: u64, idx: u64, depth: u32) -> Prog {
    let mut rg = Rng::new(seed.wrapping_mul(1_000_003).wrapping_add(idx));
    let npend = 1 + [0, 0, 0, 1, 1, 2][rg.below(6) as usize];
    let dvp = if rg.chance(1, 4) { 3 } else { 0 };
    let rhs: Vec<&'static str> = (0..npend)
        .map(|_| RHS[rg.below(RHS.len() as u64) as usize])
        .collect();
    let typed = rg.chance(7, 10);
    let pe = [3, 6, 10, 15][rg.below(4) as usize];
    let stmts: Vec<S> = if typed {
        gen_typed(&mut rg, &rhs, depth, pe, dvp)
    } else {
        let n = 1 + rg.below(3) as usize;
        let mut g = Gn {
            rg: &mut rg,
            np: npend,
            dvp,
        };
        (0..n).map(|_| g.stmt(depth)).collect()
    };
    let end = stmts.len();
    let pos: Vec<usize> = (0..npend)
        .map(|_| {
            if rg.chance(3, 4) {
                end
            } else {
                rg.below(end as u64 + 1) as usize
            }
        })
        .collect();
    Prog {
        npend,
        rhs,
        pos,
        stmts,
    }
}

// ---------- rendering with spans
#[derive(Default, Clone)]
struct Sp {
    nodes: HashMap<(u64, u64), String>,
    ops: HashMap<(u64, u64), String>,
    stm: Vec<(u64, u64, bool)>,
    pre_end: u64,
}
struct W {
    out: String,
    sp: Sp,
    path: Vec<u8>,
}
impl W {
    fn pos(&self) -> u64 {
        self.out.len() as u64
    }
    fn id(&self) -> String {
        self.path.iter().map(|b| format!("{b}.")).collect()
    }
    fn sub(&mut self, k: u8, e: &E) {
        self.path.push(k);
        self.e(e);
        self.path.pop();
    }
    fn e(&mut self, e: &E) {
        let st = self.pos();
        match e {
            E::Lf(s) => {
                self.out += s;
                let en = self.pos();
                self.sp.nodes.insert((st, en), self.id());
            }
            E::Pv(i) => {
                self.out += PN[*i];
                let en = self.pos();
                self.sp.nodes.insert((st, en), self.id());
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
                self.sp.nodes.insert((st + 1, self.pos() - 1), self.id());
            }
            E::If(c, a, b) => {
                self.out += "(if ";
                self.sub(0, c);
                self.out += " then ";
                self.sub(1, a);
                self.out += " else ";
                self.sub(2, b);
                self.out += ")";
                self.sp.nodes.insert((st + 1, self.pos() - 1), self.id());
            }
            E::IfB(c, a, b) => {
                self.out += "(if ";
                self.sub(0, c);
                self.out += " then {value: ";
                self.sub(1, a);
                self.out += ";} else {value: ";
                self.sub(2, b);
                self.out += ";})";
                self.sp.nodes.insert((st + 1, self.pos() - 1), self.id());
            }
            E::Arr(v) => {
                self.out += "[";
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        self.out += ",";
                    }
                    self.sub(i as u8, x);
                }
                self.out += "]";
                let en = self.pos();
                self.sp.nodes.insert((st, en), self.id());
            }
            E::Idx(b, i) => {
                self.sub(0, b);
                self.out += "[";
                self.sub(1, i);
                self.out += "]";
                let en = self.pos();
                self.sp.nodes.insert((st, en), self.id());
            }
            E::Len(x) => {
                self.out += "Len(";
                self.sub(0, x);
                self.out += ")";
                let en = self.pos();
                self.sp.nodes.insert((st, en), self.id());
            }
            E::Call(k, x) => {
                self.out += &format!("H{k}(");
                self.sub(0, x);
                self.out += ")";
                let en = self.pos();
                self.sp.nodes.insert((st, en), self.id());
            }
        }
    }
}
struct Rendered {
    src: String,
    sp: Sp,
}
fn render(p: &Prog, deferred: bool, order: &[usize]) -> Rendered {
    let mut w = W {
        out: String::new(),
        sp: Sp::default(),
        path: vec![],
    };
    for k in 0..5 {
        w.out += &format!(
            "fn G{k}(a:view {}){{}} fn H{k}(a:view Array<I64>)->{}{{return {};}} ",
            GT[k], GT[k], GL[k]
        );
    }
    w.out += "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];";
    w.sp.pre_end = w.pos();
    for (i, name) in PN.iter().enumerate().take(p.npend) {
        if deferred {
            w.out += &format!("var {name}=[];");
        } else {
            w.out += &format!("let {name}={};", p.rhs[i]);
        }
    }
    let mut stm: Vec<(u64, u64, bool)> = vec![];
    let place = |w: &mut W, at: usize, stm: &mut Vec<(u64, u64, bool)>| {
        if deferred {
            for &i in order {
                if p.pos[i] == at {
                    let s = w.pos();
                    w.out += &format!("{}={};", PN[i], p.rhs[i]);
                    stm.push((s, w.pos(), true));
                }
            }
        }
    };
    for (k, s) in p.stmts.iter().enumerate() {
        place(&mut w, k, &mut stm);
        let s0 = w.pos();
        w.path = vec![100 + k as u8];
        match s {
            S::Let(a, e) => {
                match a {
                    Some(t) => w.out += &format!("let a{k}:{t}="),
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
            S::CallG(g, e) => {
                w.out += &format!("G{g}(");
                w.sub(0, e);
                w.out += ");";
            }
            S::Assign(t, l, e) => {
                w.out += &format!("var s{k}:{t}={l};s{k}=");
                w.sub(0, e);
                w.out += ";";
            }
            S::IdxAssign(t, l, e) => {
                w.out += &format!("var i{k}:Array<{t}>=[{l}];i{k}[0]=");
                w.sub(0, e);
                w.out += ";";
            }
            S::Poison(e) => {
                w.out += &format!("var t{k}=true+1;t{k}=");
                w.sub(0, e);
                w.out += ";";
            }
        }
        stm.push((s0, w.pos(), false));
    }
    place(&mut w, p.stmts.len(), &mut stm);
    w.out += "}";
    w.sp.stm = stm;
    Rendered {
        src: w.out,
        sp: w.sp,
    }
}

// ---------- twin typing (own classifier of "clean" sites)
#[derive(Clone, PartialEq, Eq, Debug)]
enum Tr {
    T(Ty),
    Bad,
    Nv,
}
#[derive(Clone, Default, Debug)]
struct Inf {
    self_ok: bool,
    kids_ok: bool,
    union_chain: bool,
    f1: bool,
    vidx: bool,
    g1: bool,
    sib_bad: bool,
}
fn pid(path: &[u8]) -> String {
    path.iter().map(|b| format!("{b}.")).collect()
}
fn lf_ty(s: &str) -> Tr {
    match s {
        "x" | "1" | "0" | "2" => Tr::T(Ty::I),
        "p" | "true" | "false" => Tr::T(Ty::B),
        "ar" => Tr::T(ty_of_gt(2)),
        "br" => Tr::T(ty_of_gt(3)),
        "aar" => Tr::T(ty_of_gt(4)),
        _ => Tr::Nv,
    }
}
fn leaf_id(e: &E, path: &[u8], k: u8) -> String {
    let mut p = path.to_vec();
    p.push(k);
    let mut cur = e;
    while let E::Grp(x) = cur {
        p.push(0);
        cur = x;
    }
    pid(&p)
}
fn mark_sibs(inf: &mut HashMap<String, Inf>, path: &[u8], kids: &[(u8, &E, bool)]) {
    for (i, (k, ke, _)) in kids.iter().enumerate() {
        let others_bad = kids
            .iter()
            .enumerate()
            .any(|(j, (_, _, ok))| j != i && !*ok);
        if others_bad {
            let id = leaf_id(ke, path, *k);
            if let Some(x) = inf.get_mut(&id) {
                x.sib_bad = true;
            }
        }
    }
}
#[derive(Clone, Copy)]
struct TypeContext {
    uc: bool,
    f1: bool,
    vidx: bool,
    g1: bool,
}

fn tyck(
    e: &E,
    path: &mut Vec<u8>,
    p: &Prog,
    inf: &mut HashMap<String, Inf>,
    context: TypeContext,
) -> Tr {
    let TypeContext { uc, f1, vidx, g1 } = context;
    let me = pid(path);
    let sub = |k: u8,
               x: &E,
               path: &mut Vec<u8>,
               inf: &mut HashMap<String, Inf>,
               u: bool,
               f: bool,
               v: bool,
               g: bool|
     -> Tr {
        path.push(k);
        let r = tyck(
            x,
            path,
            p,
            inf,
            TypeContext {
                uc: u,
                f1: f1 || f,
                vidx: v,
                g1: g1 || g,
            },
        );
        path.pop();
        r
    };
    let (r, kids_ok): (Tr, bool) = match e {
        E::Lf(s) => (lf_ty(s), true),
        E::Pv(i) => (Tr::T(rhs_ty(p.rhs[*i])), true),
        E::Grp(x) => {
            let r = sub(0, x, path, inf, uc, false, vidx, false);
            return r;
        }
        E::Bin(op, a, b) => {
            let ta = sub(0, a, path, inf, false, false, false, false);
            let tb = sub(
                1,
                b,
                path,
                inf,
                false,
                matches!(op, Op::Add | Op::Sub) && !matches!(ta, Tr::T(_)),
                false,
                false,
            );
            let ok = matches!(ta, Tr::T(_)) && matches!(tb, Tr::T(_));
            mark_sibs(
                inf,
                path,
                &[
                    (0, &**a, matches!(ta, Tr::T(_))),
                    (1, &**b, matches!(tb, Tr::T(_))),
                ],
            );
            let r = match (op, &ta, &tb) {
                (Op::Add | Op::Sub, Tr::T(Ty::I), Tr::T(Ty::I)) => Tr::T(Ty::I),
                (Op::Lt, Tr::T(Ty::I), Tr::T(Ty::I)) => Tr::T(Ty::B),
                (Op::Eq, Tr::T(x), Tr::T(y)) if x == y => Tr::T(Ty::B),
                _ => {
                    if ta == Tr::Nv || tb == Tr::Nv {
                        Tr::Nv
                    } else {
                        Tr::Bad
                    }
                }
            };
            (r, ok)
        }
        E::If(c, a, b) | E::IfB(c, a, b) => {
            let tc = sub(0, c, path, inf, false, false, false, false);
            let ta = sub(1, a, path, inf, uc, false, false, false);
            let tb = sub(2, b, path, inf, uc, false, false, false);
            let ok = matches!(tc, Tr::T(Ty::B))
                && matches!(ta, Tr::T(_))
                && matches!(tb, Tr::T(_))
                && ta == tb;
            mark_sibs(
                inf,
                path,
                &[
                    (1, &**a, matches!(ta, Tr::T(_))),
                    (2, &**b, matches!(tb, Tr::T(_))),
                ],
            );
            let r = match (&ta, &tb) {
                (Tr::T(x), Tr::T(y)) if x == y && matches!(tc, Tr::T(Ty::B)) => Tr::T(x.clone()),
                (Tr::Nv, other) | (other, Tr::Nv) if matches!(tc, Tr::T(Ty::B)) => other.clone(),
                _ => Tr::Bad,
            };
            (r, ok)
        }
        E::Arr(v) => {
            let ts: Vec<Tr> = v
                .iter()
                .enumerate()
                .map(|(i, x)| sub(i as u8, x, path, inf, uc, false, false, false))
                .collect();
            let ok =
                ts.iter().all(|t| matches!(t, Tr::T(_))) && ts.windows(2).all(|w| w[0] == w[1]);
            {
                let kids: Vec<(u8, &E, bool)> = v
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (i as u8, x, matches!(ts[i], Tr::T(_))))
                    .collect();
                mark_sibs(inf, path, &kids);
            }
            let r = if ok {
                if let Tr::T(t) = &ts[0] {
                    Tr::T(Ty::A(Box::new(t.clone())))
                } else {
                    Tr::Bad
                }
            } else if ts.contains(&Tr::Nv) {
                Tr::Nv
            } else {
                Tr::Bad
            };
            (r, ok)
        }
        E::Idx(b, i) => {
            let tb = sub(0, b, path, inf, false, false, false, false);
            let ti = sub(
                1,
                i,
                path,
                inf,
                false,
                false,
                !matches!(tb, Tr::T(Ty::A(_))),
                tb == Tr::Bad,
            );
            let ok = matches!(tb, Tr::T(_)) && matches!(ti, Tr::T(_));
            let r = match (&tb, &ti) {
                (Tr::T(Ty::A(u)), Tr::T(Ty::I)) => Tr::T((**u).clone()),
                _ => {
                    if tb == Tr::Nv || ti == Tr::Nv {
                        Tr::Nv
                    } else {
                        Tr::Bad
                    }
                }
            };
            (r, ok)
        }
        E::Len(x) => {
            let t = sub(0, x, path, inf, false, false, false, false);
            let r = match &t {
                Tr::T(Ty::A(_)) => Tr::T(Ty::I),
                Tr::Nv => Tr::Nv,
                _ => Tr::Bad,
            };
            (r, matches!(t, Tr::T(_)))
        }
        E::Call(k, x) => {
            let t = sub(0, x, path, inf, false, false, false, false);
            let r = match &t {
                Tr::T(tt) if *tt == ty_of_gt(2) => Tr::T(ty_of_gt(*k)),
                Tr::Nv => Tr::Nv,
                _ => Tr::Bad,
            };
            (r, matches!(t, Tr::T(_)))
        }
    };
    inf.insert(
        me,
        Inf {
            self_ok: matches!(r, Tr::T(_)),
            kids_ok,
            union_chain: uc,
            f1,
            vidx,
            g1,
            sib_bad: false,
        },
    );
    r
}
fn build_inf(p: &Prog) -> HashMap<String, Inf> {
    let mut inf = HashMap::new();
    for (k, s) in p.stmts.iter().enumerate() {
        let ext = !matches!(s, S::Let(None, _) | S::Assert(..) | S::Print(_));
        for j in 0..2 {
            if let Some(e) = s_expr(s, j) {
                let mut path = vec![100 + k as u8, j as u8];
                let _ = tyck(
                    e,
                    &mut path,
                    p,
                    &mut inf,
                    TypeContext {
                        uc: !ext,
                        f1: false,
                        vidx: false,
                        g1: false,
                    },
                );
            }
        }
        if let S::Assert(a, b) = s {
            let mut pa = vec![100 + k as u8, 0];
            let mut pb = vec![100 + k as u8, 1];
            let context = TypeContext {
                uc: false,
                f1: false,
                vidx: false,
                g1: false,
            };
            let ta = tyck(a, &mut pa, p, &mut inf, context);
            let tb = tyck(b, &mut pb, p, &mut inf, context);
            let (ia, ib) = (
                leaf_id(a, &[100 + k as u8], 0),
                leaf_id(b, &[100 + k as u8], 1),
            );
            if !matches!(ta, Tr::T(_))
                && let Some(x) = inf.get_mut(&ib)
            {
                x.sib_bad = true;
            }
            if !matches!(tb, Tr::T(_))
                && let Some(x) = inf.get_mut(&ia)
            {
                x.sib_bad = true;
            }
        }
    }
    inf
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
fn run_bug(src: &str) -> Option<String> {
    if src.contains("while ") {
        return None;
    }
    match catch_unwind(AssertUnwindSafe(|| match compile_scalar("t.n", src) {
        Ok(p) => {
            let ex = nexo::interp::run_scalar(&p);
            if let nexo::interp::Outcome::InternalError(m) = ex.outcome {
                Some(format!("run internal:{m}"))
            } else {
                None
            }
        }
        Err(CompileError::Rejected(_)) => None,
        Err(CompileError::InvalidIr(m)) => Some(format!("InvalidIr(run):{m}")),
        Err(_) => Some("other(run)".into()),
    })) {
        Err(_) => Some("panic(run)".into()),
        Ok(x) => x,
    }
}

#[derive(Debug, Clone)]
struct Diff {
    side: &'static str,
    k: K,
    cls: &'static str,
}
fn proj(r: &R, assign_spans: &[(u64, u64)], drop_assign: bool) -> BTreeMap<K, usize> {
    let mut m = BTreeMap::new();
    if let R::Rej(v) = r {
        for (k, s, _) in v {
            if drop_assign && assign_spans.iter().any(|(a, b)| s >= a && s < b) {
                continue;
            }
            *m.entry(k.clone()).or_insert(0) += 1;
        }
    }
    m
}
struct Res {
    diffs: Vec<Diff>,
    assign_blamed: bool,
    bug: Option<String>,
    dup: bool,
    nd: usize,
    nt: usize,
    accepted: bool,
}
fn orders(n: usize) -> Vec<Vec<usize>> {
    let mut v = vec![(0..n).collect::<Vec<_>>()];
    if n == 2 {
        v.push(vec![1, 0]);
    }
    if n == 3 {
        v.push(vec![2, 1, 0]);
        v.push(vec![1, 2, 0]);
    }
    v
}
fn aspans(r: &Rendered) -> Vec<(u64, u64)> {
    r.sp.stm
        .iter()
        .filter(|x| x.2)
        .map(|x| (x.0, x.1))
        .collect()
}
fn norm(m: &BTreeMap<K, usize>, level2: bool) -> BTreeMap<K, usize> {
    let mut o = BTreeMap::new();
    for (k, v) in m {
        if k.loc.starts_with("pre:") {
            continue;
        }
        let mut k2 = k.clone();
        if !level2 {
            k2.exp.clear();
            k2.found.clear();
        }
        *o.entry(k2).or_insert(0) += v;
    }
    o
}
fn classify(inf: &HashMap<String, Inf>, k: &K) -> &'static str {
    // A: clean site (conserved C expected); B: operand/child poisoned (V-possible); D: union leaf w/o external context (D-possible); U: unknown
    if let Some(id) = k.loc.strip_prefix("op:") {
        return match inf.get(id) {
            Some(i) if i.f1 => "F",
            Some(i) if i.g1 => "G",
            Some(i) if i.kids_ok => "A",
            Some(_) => "B",
            None => "U",
        };
    }
    if let Some(id) = k.loc.strip_prefix("n:") {
        return match inf.get(id) {
            Some(i) if i.f1 => "F",
            Some(i) if i.g1 => "G",
            Some(i) if i.sib_bad => "B",
            Some(i) if i.vidx => "B",
            Some(i) if i.union_chain && i.self_ok && id.matches('.').count() > 2 => "D",
            Some(i) if i.self_ok => "A",
            Some(_) => "B",
            None => "U",
        };
    }
    "U"
}
fn compare(p: &Prog, level2: bool) -> Res {
    let ord = orders(p.npend);
    let d = render(p, true, &ord[0]);
    let c = render(p, false, &ord[0]);
    let rd = check(&d);
    let rc = check(&c);
    let mut res = Res {
        diffs: vec![],
        assign_blamed: false,
        bug: None,
        dup: false,
        nd: 0,
        nt: 0,
        accepted: false,
    };
    for (nm, r) in [("deferred", &rd), ("twin", &rc)] {
        if let R::Bug(m) = r {
            res.bug = Some(format!("{nm}:{m}"));
            return res;
        }
    }
    res.accepted = matches!(rd, R::Ok) && matches!(rc, R::Ok);
    if matches!(rd, R::Ok)
        && let Some(b) = run_bug(&d.src)
    {
        res.bug = Some(format!("deferred:{b}"));
        return res;
    }
    if matches!(rc, R::Ok)
        && let Some(b) = run_bug(&c.src)
    {
        res.bug = Some(format!("twin:{b}"));
        return res;
    }
    let asp = aspans(&d);
    if let R::Rej(v) = &rd {
        for (_, s, _) in v {
            if asp.iter().any(|(a, b)| s >= a && s < b) {
                res.assign_blamed = true;
            }
        }
        let mut seen = BTreeSet::new();
        for (k, s, e) in v {
            if !seen.insert((k.code.clone(), *s, *e)) {
                res.dup = true;
            }
        }
    }
    if let R::Rej(v) = &rc {
        let mut seen = BTreeSet::new();
        for (k, s, e) in v {
            if !seen.insert((k.code.clone(), *s, *e)) {
                res.dup = true;
            }
        }
    }
    let md = proj(&rd, &asp, true);
    let mc = proj(&rc, &[], false);
    res.nd = md.values().sum();
    res.nt = mc.values().sum();
    let (nd, nc) = (norm(&md, level2), norm(&mc, level2));
    let inf = build_inf(p);
    for (k, v) in &nd {
        if nc.get(k).copied().unwrap_or(0) < *v {
            res.diffs.push(Diff {
                side: "extra_in_deferred",
                k: k.clone(),
                cls: classify(&inf, k),
            });
        }
    }
    for (k, v) in &nc {
        if nd.get(k).copied().unwrap_or(0) < *v {
            res.diffs.push(Diff {
                side: "missing_in_deferred",
                k: k.clone(),
                cls: classify(&inf, k),
            });
        }
    }
    if p.npend > 1 {
        for o in ord.iter().skip(1) {
            let d2 = render(p, true, o);
            let r2 = check(&d2);
            if let R::Bug(m) = &r2 {
                res.bug = Some(format!("perm:{m}"));
                return res;
            }
            let a1 = matches!(rd, R::Ok);
            let a2 = matches!(r2, R::Ok);
            let kk = K {
                code: format!("{o:?}"),
                sub: String::new(),
                loc: String::new(),
                exp: String::new(),
                found: String::new(),
            };
            if a1 != a2 {
                res.diffs.push(Diff {
                    side: "perm_accept",
                    k: kk,
                    cls: "A",
                });
            } else {
                let m2 = norm(&proj(&r2, &aspans(&d2), true), level2);
                if m2 != nd {
                    res.diffs.push(Diff {
                        side: "perm_diff",
                        k: kk,
                        cls: "A",
                    });
                }
            }
        }
    }
    // poison oracle: `var t=true+1; t=E;` vs `let z=E;` inside the same deferred program
    for (k, st) in p.stmts.iter().enumerate() {
        if let S::Poison(e) = st {
            let mut p2 = p.clone();
            p2.stmts[k] = S::Let(None, e.clone());
            let d2 = render(&p2, true, &ord[0]);
            let r2 = check(&d2);
            if let R::Bug(m) = &r2 {
                res.bug = Some(format!("poison-variant:{m}"));
                return res;
            }
            let pref = format!("{}.", 100 + k);
            let f = |m: BTreeMap<K, usize>| -> BTreeMap<K, usize> {
                m.into_iter()
                    .filter(|(kk, _)| {
                        kk.loc
                            .split(':')
                            .nth(1)
                            .is_some_and(|l| l.starts_with(&pref))
                    })
                    .collect()
            };
            let a1 = f(norm(&proj(&rd, &[], false), level2));
            let a2 = f(norm(&proj(&r2, &[], false), level2));
            if a1 != a2 {
                let kk = a1
                    .keys()
                    .chain(a2.keys())
                    .find(|x| a1.get(*x) != a2.get(*x))
                    .unwrap()
                    .clone();
                res.diffs.push(Diff {
                    side: "poison_diff",
                    k: kk,
                    cls: "A",
                });
            }
        }
    }
    res
}

// ---------- shrink
fn shrink_e(e: &E, out: &mut Vec<E>) {
    match e {
        E::Lf(_) | E::Pv(_) => {}
        E::Bin(o, a, b) => {
            out.push((**a).clone());
            out.push((**b).clone());
            let mut v = vec![];
            shrink_e(a, &mut v);
            for x in v {
                out.push(E::Bin(*o, Box::new(x), b.clone()));
            }
            let mut v = vec![];
            shrink_e(b, &mut v);
            for x in v {
                out.push(E::Bin(*o, a.clone(), Box::new(x)));
            }
        }
        E::If(c, a, b) | E::IfB(c, a, b) => {
            let blk = matches!(e, E::IfB(..));
            let mk = move |c: E, a: E, b: E| {
                if blk {
                    E::IfB(Box::new(c), Box::new(a), Box::new(b))
                } else {
                    E::If(Box::new(c), Box::new(a), Box::new(b))
                }
            };
            out.push((**a).clone());
            out.push((**b).clone());
            let mut v = vec![];
            shrink_e(c, &mut v);
            for x in v {
                out.push(mk(x, (**a).clone(), (**b).clone()));
            }
            let mut v = vec![];
            shrink_e(a, &mut v);
            for x in v {
                out.push(mk((**c).clone(), x, (**b).clone()));
            }
            let mut v = vec![];
            shrink_e(b, &mut v);
            for x in v {
                out.push(mk((**c).clone(), (**a).clone(), x));
            }
        }
        E::Arr(v) => {
            for i in 0..v.len() {
                out.push(v[i].clone());
                if v.len() > 1 {
                    let mut w = v.clone();
                    w.remove(i);
                    out.push(E::Arr(w));
                }
                let mut s = vec![];
                shrink_e(&v[i], &mut s);
                for x in s {
                    let mut w = v.clone();
                    w[i] = x;
                    out.push(E::Arr(w));
                }
            }
        }
        E::Idx(b, i) => {
            out.push((**b).clone());
            out.push((**i).clone());
            let mut v = vec![];
            shrink_e(b, &mut v);
            for x in v {
                out.push(E::Idx(Box::new(x), i.clone()));
            }
            let mut v = vec![];
            shrink_e(i, &mut v);
            for x in v {
                out.push(E::Idx(b.clone(), Box::new(x)));
            }
        }
        E::Len(x) => {
            out.push((**x).clone());
            let mut v = vec![];
            shrink_e(x, &mut v);
            for y in v {
                out.push(E::Len(Box::new(y)));
            }
        }
        E::Call(k, x) => {
            out.push((**x).clone());
            let mut v = vec![];
            shrink_e(x, &mut v);
            for y in v {
                out.push(E::Call(*k, Box::new(y)));
            }
        }
        E::Grp(x) => {
            out.push((**x).clone());
            let mut v = vec![];
            shrink_e(x, &mut v);
            for y in v {
                out.push(E::Grp(Box::new(y)));
            }
        }
    }
}
fn s_expr(s: &S, i: usize) -> Option<&E> {
    match (s, i) {
        (
            S::Let(_, e)
            | S::Cond(e)
            | S::While(e)
            | S::Print(e)
            | S::CallG(_, e)
            | S::Assign(_, _, e)
            | S::IdxAssign(_, _, e)
            | S::Poison(e),
            0,
        ) => Some(e),
        (S::Assert(a, _), 0) => Some(a),
        (S::Assert(_, b), 1) => Some(b),
        _ => None,
    }
}
fn s_with(s: &S, i: usize, ne: E) -> S {
    let mut s = s.clone();
    match (&mut s, i) {
        (
            S::Let(_, e)
            | S::Cond(e)
            | S::While(e)
            | S::Print(e)
            | S::CallG(_, e)
            | S::Assign(_, _, e)
            | S::IdxAssign(_, _, e)
            | S::Poison(e),
            0,
        ) => *e = ne,
        (S::Assert(a, _), 0) => *a = ne,
        (S::Assert(_, b), 1) => *b = ne,
        _ => {}
    }
    s
}
fn shrink_p(p: &Prog) -> Vec<Prog> {
    let mut o = vec![];
    for i in 0..p.stmts.len() {
        if p.stmts.len() > 1 {
            let mut q = p.clone();
            q.stmts.remove(i);
            for x in q.pos.iter_mut() {
                if *x > i {
                    *x -= 1;
                }
            }
            o.push(q);
        }
        if let S::Assert(a, _) = &p.stmts[i] {
            let mut q = p.clone();
            q.stmts[i] = S::Cond(a.clone());
            o.push(q);
        }
        if let S::Let(Some(_), e) = &p.stmts[i] {
            let mut q = p.clone();
            q.stmts[i] = S::Let(None, e.clone());
            o.push(q);
        }
        for j in 0..2 {
            if let Some(e) = s_expr(&p.stmts[i], j) {
                let mut v = vec![];
                shrink_e(e, &mut v);
                for x in v {
                    let mut q = p.clone();
                    q.stmts[i] = s_with(&p.stmts[i], j, x);
                    o.push(q);
                }
                for lf in [E::Lf("1"), E::Lf("true"), E::Lf("ar"), E::Lf("x")] {
                    let mut q = p.clone();
                    q.stmts[i] = s_with(&p.stmts[i], j, lf);
                    o.push(q);
                }
            }
        }
    }
    for k in 0..p.npend {
        for r in ["[3]", "[[3]]", "[true]"] {
            if p.rhs[k] != r {
                let mut q = p.clone();
                q.rhs[k] = r;
                o.push(q);
            }
        }
    }
    o
}
fn uses_pend(p: &Prog) -> bool {
    fn u(e: &E, k: usize) -> bool {
        match e {
            E::Pv(i) => *i == k,
            E::Lf(_) => false,
            E::Bin(_, a, b) => u(a, k) || u(b, k),
            E::If(a, b, c) | E::IfB(a, b, c) => u(a, k) || u(b, k) || u(c, k),
            E::Arr(v) => v.iter().any(|x| u(x, k)),
            E::Idx(a, b) => u(a, k) || u(b, k),
            E::Len(x) | E::Call(_, x) | E::Grp(x) => u(x, k),
        }
    }
    (0..p.npend).all(|k| {
        p.stmts
            .iter()
            .any(|s| (0..2).any(|j| s_expr(s, j).is_some_and(|e| u(e, k))))
    })
}
fn esz(e: &E) -> usize {
    match e {
        E::Lf(_) | E::Pv(_) => 1,
        E::Bin(_, a, b) | E::Idx(a, b) => 1 + esz(a) + esz(b),
        E::If(a, b, c) | E::IfB(a, b, c) => 1 + esz(a) + esz(b) + esz(c),
        E::Arr(v) => 1 + v.iter().map(esz).sum::<usize>(),
        E::Len(x) | E::Call(_, x) | E::Grp(x) => 1 + esz(x),
    }
}
fn psz(p: &Prog) -> (usize, usize) {
    let mut n = p.stmts.len() * 10;
    for s in &p.stmts {
        for j in 0..2 {
            if let Some(e) = s_expr(s, j) {
                n += esz(e);
            }
        }
        for x in std::iter::once(0) {
            let _ = x;
        }
        if let S::Let(Some(_), _) = s {
            n += 1;
        }
        if let S::Assert(..) = s {
            n += 1;
        }
    }
    let rk = p
        .rhs
        .iter()
        .map(|r| match *r {
            "[3]" => 0,
            "[true]" => 1,
            "[[3]]" => 2,
            _ => 3,
        })
        .sum();
    (n, rk)
}
fn reduce(p: &Prog, target: (&str, &str, &str), level2: bool) -> Prog {
    let mut cur = p.clone();
    loop {
        let mut progress = false;
        for q in shrink_p(&cur) {
            if !uses_pend(&q) || psz(&q) >= psz(&cur) {
                continue;
            }
            let r = compare(&q, level2);
            if r.bug.is_some() || r.assign_blamed {
                continue;
            }
            if r.diffs.iter().any(|d| {
                d.cls == "A" && d.side == target.0 && d.k.code == target.1 && d.k.sub == target.2
            }) {
                cur = q;
                progress = true;
                break;
            }
        }
        if !progress {
            return cur;
        }
    }
}

fn bug_of(p: &Prog) -> Option<String> {
    let ord = orders(p.npend);
    for (nm, def) in [("deferred", true), ("twin", false)] {
        let r = render(p, def, &ord[0]);
        match check(&r) {
            R::Bug(m) => return Some(format!("{nm}:{m}")),
            R::Ok => {
                if let Some(b) = run_bug(&r.src) {
                    return Some(format!("{nm}:{b}"));
                }
            }
            _ => {}
        }
    }
    for o in ord.iter().skip(1) {
        let r = render(p, true, o);
        if let R::Bug(m) = check(&r) {
            return Some(format!("perm:{m}"));
        }
    }
    None
}
fn reduce_bug(p: &Prog, msg: &str) -> Prog {
    let mut cur = p.clone();
    loop {
        let mut progress = false;
        for q in shrink_p(&cur) {
            if psz(&q) >= psz(&cur) {
                continue;
            }
            if bug_of(&q).is_some_and(|m| m == msg) {
                cur = q;
                progress = true;
                break;
            }
        }
        if !progress {
            return cur;
        }
    }
}
fn pstr(p: &Prog) -> (String, String) {
    let o: Vec<usize> = (0..p.npend).collect();
    let f = |s: String| s[s.find("fn Main(){").unwrap()..].to_string();
    (f(render(p, true, &o).src), f(render(p, false, &o).src))
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mode = a.get(1).map(String::as_str).unwrap_or("");
    let n: u64 = a.get(2).and_then(|x| x.parse().ok()).unwrap_or(1000);
    let seed: u64 = a.get(3).and_then(|x| x.parse().ok()).unwrap_or(1);
    let threads: u64 = a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8);
    let depth: u32 = a.get(5).and_then(|x| x.parse().ok()).unwrap_or(3);
    let level2 = std::env::var("L2").is_ok();
    let maxshow: usize = std::env::var("MAXSHOW")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(150);
    let redmax: usize = std::env::var("REDMAX")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(400);
    match mode {
        "reduce-show" => {
            let p = gen_prog(seed, n, depth);
            let original = compare(&p, level2);
            let Some(first) = original.diffs.iter().find(|d| d.cls == "A") else {
                println!("no A difference");
                return;
            };
            let rp = reduce(&p, (first.side, &first.k.code, &first.k.sub), level2);
            let ord = orders(rp.npend);
            for (label, source) in [
                ("deferred", render(&rp, true, &ord[0])),
                ("concrete", render(&rp, false, &ord[0])),
            ] {
                println!("{label}: {}", source.src);
                match check(&source) {
                    R::Ok => println!("OK"),
                    R::Bug(message) => println!("BUG {message}"),
                    R::Rej(rows) => {
                        for (key, start, end) in rows {
                            println!("  {key:?} [{start},{end}) {}", &source.src[start as usize..end as usize]);
                        }
                    }
                }
            }
            for order in ord.iter().skip(1) {
                let source = render(&rp, true, order);
                println!("permutation {order:?}: {}", source.src);
                match check(&source) {
                    R::Ok => println!("OK"),
                    R::Bug(message) => println!("BUG {message}"),
                    R::Rej(rows) => {
                        for (key, start, end) in rows {
                            println!("  {key:?} [{start},{end}) {}", &source.src[start as usize..end as usize]);
                        }
                    }
                }
            }
        }
        "show" => {
            let p = gen_prog(seed, n, depth);
            let (d, c) = pstr(&p);
            println!("{d}\n{c}");
        }
        "sum" => {
            let src = std::fs::read_to_string(&a[2]).unwrap();
            match catch_unwind(AssertUnwindSafe(|| check_scalar("t.n", &src))) {
                Err(_) => println!("PANIC"),
                Ok(Ok(_)) => println!("OK"),
                Ok(Err(CompileError::Rejected(v))) => {
                    for d in v.iter() {
                        let (s, e) = d
                            .primary_span
                            .as_ref()
                            .map(|p| (p.start as usize, p.end as usize))
                            .unwrap_or((0, 0));
                        let rel: Vec<String> = d
                            .related_spans
                            .iter()
                            .map(|r| {
                                format!(
                                    "{}={}",
                                    r.role.as_str(),
                                    &src[r.span.start as usize..r.span.end as usize]
                                )
                            })
                            .collect();
                        println!(
                            "{}{} `{}`[{},{}) exp={} found={} rel={:?}",
                            d.code,
                            d.subcause
                                .as_ref()
                                .map(|x| format!("/{x}"))
                                .unwrap_or_default(),
                            &src[s..e],
                            s,
                            e,
                            gs(d, "expected_type"),
                            gs(d, "found_type"),
                            rel
                        );
                    }
                }
                Ok(Err(CompileError::InvalidIr(m))) => println!("InvalidIr: {m}"),
                Ok(Err(_)) => println!("OTHER(Dependent)"),
            }
        }
        "bugs" => {
            let found: Vec<(u64, Prog, String)> = std::thread::scope(|sc| {
                let hs: Vec<_> = (0..threads)
                    .map(|t| {
                        sc.spawn(move || {
                            let mut o = vec![];
                            let mut i = t;
                            while i < n {
                                let p = gen_prog(seed, i, depth);
                                if let Some(m) = bug_of(&p)
                                    && o.len() < 400
                                {
                                    o.push((i, p, m));
                                }
                                i += threads;
                            }
                            o
                        })
                    })
                    .collect();
                hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
            });
            println!("programs={n} seed={seed} bugs={}", found.len());
            let mut seen: BTreeMap<String, (usize, String)> = BTreeMap::new();
            for (i, p, m) in found.iter().step_by((found.len() / 20).max(1)).take(20) {
                let rp = reduce_bug(p, m);
                let (d, _) = pstr(&rp);
                let e = seen.entry(d.clone()).or_insert((0, m.clone()));
                e.0 += 1;
                let _ = i;
            }
            for (d, (c, m)) in seen {
                println!(
                    "#{c} {m} :: {}",
                    d.replace(
                        "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];",
                        ""
                    )
                );
            }
        }
        "twin" => {
            let results: Vec<(u64, Prog, Res)> = std::thread::scope(|sc| {
                let hs: Vec<_> = (0..threads)
                    .map(|t| {
                        sc.spawn(move || {
                            let mut out = vec![];
                            let (mut bugs, mut ablame, mut dups, mut tot, mut rej, mut acc, mut other) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
                            let mut i = t;
                            while i < n {
                                let p = gen_prog(seed, i, depth);
                                let r = compare(&p, level2);
                                tot += 1;
                                if r.nd + r.nt > 0 {
                                    rej += 1;
                                }
                                if r.accepted {
                                    acc += 1;
                                }
                                if r.bug.is_some() {
                                    bugs += 1;
                                    out.push((i, p, r));
                                } else if r.dup {
                                    dups += 1;
                                    if out.len() < 500 {
                                        out.push((i, p, r));
                                    }
                                } else if r.assign_blamed {
                                    ablame += 1;
                                } else if r.diffs.iter().any(|d| d.cls == "A") && out.len() < 20000 {
                                    out.push((i, p, r));
                                } else if !r.diffs.is_empty() { other += 1; }
                                i += threads;
                            }
                            eprintln!("thread {t}: tot={tot} rej={rej} acc_both={acc} bugs={bugs} assign_blamed={ablame} dups={dups} nonA_diff={other}");
                            out
                        })
                    })
                    .collect();
                hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
            });
            println!(
                "programs={n} seed={seed} depth={depth} flagged={}",
                results.len()
            );
            let mut groups: BTreeMap<String, (usize, u64, Prog)> = BTreeMap::new();
            let mut sorted = results;
            sorted.sort_by_key(|x| x.0);
            let mut bug_list = vec![];
            let mut cnt_raw: BTreeMap<String, usize> = BTreeMap::new();
            let mut todo: Vec<(u64, Prog, String, String, String)> = vec![];
            for (i, p, r) in &sorted {
                if let Some(b) = &r.bug {
                    bug_list.push((i, b.clone()));
                    continue;
                }
                if r.dup {
                    let (d, _) = pstr(p);
                    println!("DUP idx={i} {d}");
                    continue;
                }
                let d0 = r.diffs.iter().find(|d| d.cls == "A").unwrap();
                *cnt_raw
                    .entry(format!("{} {} {}", d0.side, d0.k.code, d0.k.sub))
                    .or_insert(0) += 1;
                if todo.len() < redmax {
                    todo.push((
                        *i,
                        p.clone(),
                        d0.side.to_string(),
                        d0.k.code.clone(),
                        d0.k.sub.clone(),
                    ));
                }
            }
            println!("raw first-diff classes: {cnt_raw:?}");
            let redres: Vec<(u64, Prog, String)> = std::thread::scope(|sc| {
                let todo = &todo;
                let hs: Vec<_> = (0..threads)
                    .map(|t| {
                        sc.spawn(move || {
                            let mut o = vec![];
                            let mut j = t as usize;
                            while j < todo.len() {
                                let (i, p, a, b, c) = &todo[j];
                                let side: &'static str = match a.as_str() {
                                    "extra_in_deferred" => "extra_in_deferred",
                                    "missing_in_deferred" => "missing_in_deferred",
                                    "perm_accept" => "perm_accept",
                                    "poison_diff" => "poison_diff",
                                    _ => "perm_diff",
                                };
                                let rp = reduce(p, (side, b, c), level2);
                                o.push((*i, rp, format!("{a} {b} {c}")));
                                j += threads as usize;
                            }
                            o
                        })
                    })
                    .collect();
                hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
            });
            for (i, rp, tg) in redres {
                let (d, _) = pstr(&rp);
                let sig = format!("{tg} :: {d}");
                let e = groups.entry(sig).or_insert((0, i, rp.clone()));
                e.0 += 1;
            }
            for (i, b) in bug_list {
                println!("BUG idx={i} {b}");
            }
            let mut gl: Vec<_> = groups.into_iter().collect();
            gl.sort_by_key(|(k, v)| (std::cmp::Reverse(v.0), k.len()));
            println!("distinct reduced signatures={}", gl.len());
            let strip = |x: String| {
                x.replace(
                    "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];",
                    "",
                )
            };
            for (shown, (_sig, (cnt, i, p))) in gl.iter().enumerate() {
                if shown >= maxshow {
                    break;
                }
                let (d, _) = pstr(p);
                let r = compare(p, level2);
                let ds: Vec<String> = r
                    .diffs
                    .iter()
                    .map(|x| {
                        format!(
                            "{}[{}]{}{}@{}",
                            &x.side[..3],
                            x.cls,
                            x.k.code.trim_start_matches("NX-"),
                            if x.k.sub.is_empty() {
                                String::new()
                            } else {
                                format!("/{}", x.k.sub)
                            },
                            x.k.loc
                        )
                    })
                    .collect();
                println!("#{cnt} idx={i} {}  || {}", strip(d), ds.join(" "));
            }
        }
        _ => eprintln!("usage: v17 twin N SEED THREADS DEPTH | show IDX SEED _ DEPTH"),
    }
}
