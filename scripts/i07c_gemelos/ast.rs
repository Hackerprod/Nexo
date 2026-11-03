// AST and printer with span tables for the differential campaign.
use crate::common::Rng;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum T {
    I,
    B,
    A(Box<T>),
}
impl T {
    pub fn s(&self) -> String {
        match self {
            T::I => "I64".into(),
            T::B => "Bool".into(),
            T::A(x) => format!("Array<{}>", x.s()),
        }
    }
    pub fn lit(&self) -> String {
        match self {
            T::I => "0".into(),
            T::B => "false".into(),
            T::A(x) => format!("[{}]", x.lit()),
        }
    }
    pub fn is_arr(&self) -> bool {
        matches!(self, T::A(_))
    }
}
pub fn arr(t: T) -> T {
    T::A(Box::new(t))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Lt,
    Eq,
}
impl Op {
    pub fn s(self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "-",
            Op::Lt => "<",
            Op::Eq => "==",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum V {
    X,
    P,
    Ar,
    Br,
    Aar,
    R,
    Q,
}
impl V {
    pub fn name(self) -> &'static str {
        match self {
            V::X => "x",
            V::P => "p",
            V::Ar => "ar",
            V::Br => "br",
            V::Aar => "aar",
            V::R => "r",
            V::Q => "q",
        }
    }
}
#[derive(Clone, Debug)]
pub enum K {
    Int(i64),
    Bool(bool),
    Var(V),
    Dv,
    Bin(Op, Box<N>, Box<N>),
    If(Box<N>, Box<N>, Box<N>),
    Blk(Box<N>),
    Arr(Vec<N>),
    Idx(Box<N>, Box<N>),
    Grp(Box<N>),
}
#[derive(Clone, Debug)]
pub struct N {
    pub id: u32,
    pub k: K,
}
#[derive(Clone, Debug)]
pub enum S {
    Let(Option<T>, N),
    Assign(T, N),
    IdxAssign(T, N),
    Ret(T, N),
    Call(T, N),
    Cond(N),
    While(N),
    Assert(N, N),
}
#[derive(Clone, Debug)]
pub struct Prog {
    pub stmts: Vec<S>,
    pub ft_r: Option<T>,
    pub ft_q: Option<T>,
}

pub fn n(id: &mut u32, k: K) -> N {
    *id += 1;
    N { id: *id, k }
}

impl N {
    pub fn walk<'a>(&'a self, f: &mut dyn FnMut(&'a N)) {
        f(self);
        match &self.k {
            K::Bin(_, a, b) | K::Idx(a, b) => {
                a.walk(f);
                b.walk(f);
            }
            K::If(c, a, b) => {
                c.walk(f);
                a.walk(f);
                b.walk(f);
            }
            K::Blk(a) | K::Grp(a) => a.walk(f),
            K::Arr(v) => v.iter().for_each(|x| x.walk(f)),
            _ => {}
        }
    }
}
impl S {
    pub fn nodes(&self) -> Vec<&N> {
        match self {
            S::Let(_, e) | S::Assign(_, e) | S::IdxAssign(_, e) | S::Ret(_, e) | S::Call(_, e) | S::Cond(e) | S::While(e) => vec![e],
            S::Assert(a, b) => vec![a, b],
        }
    }
}

pub struct Printed {
    pub src: String,
    pub spans: HashMap<(u64, u64), u32>, // span -> node id
    pub by_id: HashMap<u32, (u64, u64)>,
    pub ops: HashMap<(u64, u64), u32>, // operator token span -> bin node id
    pub ops_by_id: HashMap<u32, (u64, u64)>,
    pub ann: Vec<(u64, u64)>,
}

struct Pr {
    out: String,
    by_id: HashMap<u32, (u64, u64)>,
    ops_by_id: HashMap<u32, (u64, u64)>,
}
impl Pr {
    fn node(&mut self, x: &N) {
        let s = self.out.len() as u64;
        match &x.k {
            K::Int(i) => self.out += &i.to_string(),
            K::Bool(b) => self.out += if *b { "true" } else { "false" },
            K::Var(v) => self.out += v.name(),
            K::Dv => self.out += "(if true then {return;} else {return;})",
            K::Bin(op, a, b) => {
                self.node(a);
                let os = self.out.len() as u64;
                self.out += op.s();
                self.ops_by_id.insert(x.id, (os, self.out.len() as u64));
                self.node(b);
            }
            K::If(c, a, b) => {
                self.out += "if ";
                self.node(c);
                self.out += " then ";
                self.node(a);
                self.out += " else ";
                self.node(b);
            }
            K::Blk(a) => {
                self.out += "{value:";
                self.node(a);
                self.out += ";}";
            }
            K::Arr(v) => {
                self.out += "[";
                for (i, e) in v.iter().enumerate() {
                    if i > 0 {
                        self.out += ",";
                    }
                    self.node(e);
                }
                self.out += "]";
            }
            K::Idx(a, i) => {
                self.node(a);
                self.out += "[";
                self.node(i);
                self.out += "]";
            }
            K::Grp(a) => {
                self.out += "(";
                self.node(a);
                self.out += ")";
            }
        }
        self.by_id.insert(x.id, (s, self.out.len() as u64));
    }
}

pub fn uses(p: &Prog, v: V) -> bool {
    let mut found = false;
    for s in &p.stmts {
        for nn in s.nodes() {
            nn.walk(&mut |x| {
                if let K::Var(w) = &x.k {
                    if *w == v {
                        found = true;
                    }
                }
            });
        }
    }
    found
}

/// deferred=true: `var r=[]` / `var q=DV` with the assignment at the end. false: concrete twin.
pub fn print(p: &Prog, deferred: bool) -> Printed {
    print_modes(p, deferred, deferred)
}

pub fn print_modes(p: &Prog, r_deferred: bool, q_deferred: bool) -> Printed {
    let mut pr = Pr { out: String::new(), by_id: HashMap::new(), ops_by_id: HashMap::new() };
    let mut ann = Vec::new();
    let mut pre = String::new();
    let mut body_idx = 0;
    for s in &p.stmts {
        if let S::Ret(t, _) = s {
            body_idx += 1;
            let _ = t;
        }
    }
    let _ = body_idx;
    // functions first (Ret / Call)
    let mut fn_text: Vec<String> = Vec::new();
    let prelude = "let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];";
    // We print in two passes: the Main body is built with the real printer; function bodies need spans too.
    // Since function text precedes Main, build Main body first into a separate buffer, then shift offsets.
    let mut main = Pr { out: String::new(), by_id: HashMap::new(), ops_by_id: HashMap::new() };
    let mut main_ann = Vec::new();
    let use_r = uses(p, V::R);
    let use_q = uses(p, V::Q);
    let mut head = String::from("fn Main(){");
    head += prelude;
    if use_r {
        if r_deferred {
            head += "var r=[];";
        } else {
            head += &format!("var r={};", p.ft_r.as_ref().unwrap().lit_nonzero());
        }
    }
    if use_q {
        if q_deferred {
            head += "var q=(if true then {return;} else {return;});";
        } else {
            head += &format!("var q={};", p.ft_q.as_ref().unwrap().lit_nonzero());
        }
    }
    main.out += &head;
    // ret functions are emitted before Main; track them as separate pieces with their own offsets
    struct Piece {
        pr: Pr,
        ann: Vec<(u64, u64)>,
    }
    let mut pieces: Vec<Piece> = Vec::new();
    let mut calls_decl = String::new();
    for (k, s) in p.stmts.iter().enumerate() {
        match s {
            S::Let(a, e) => {
                match a {
                    Some(t) => {
                        main.out += &format!("let a{k}:");
                        let st = main.out.len() as u64;
                        main.out += &t.s();
                        main_ann.push((st, main.out.len() as u64));
                        main.out += "=";
                    }
                    None => main.out += &format!("let a{k}="),
                }
                main.node(e);
                main.out += ";";
            }
            S::Assign(t, e) => {
                main.out += &format!("var s{k}:{}={};s{k}=", t.s(), t.lit());
                main.node(e);
                main.out += ";";
            }
            S::IdxAssign(t, e) => {
                main.out += &format!("var i{k}:Array<{}>=[{}];i{k}[0]=", t.s(), t.lit());
                main.node(e);
                main.out += ";";
            }
            S::Ret(t, e) => {
                let mut f = Pr { out: String::new(), by_id: HashMap::new(), ops_by_id: HashMap::new() };
                f.out += &format!("fn F{k}()->{}{{{prelude}return ", t.s());
                f.node(e);
                f.out += ";} ";
                pieces.push(Piece { pr: f, ann: vec![] });
            }
            S::Call(t, e) => {
                calls_decl += &format!("fn G{k}(a:view {}){{}} ", t.s());
                main.out += &format!("G{k}(");
                main.node(e);
                main.out += ");";
            }
            S::Cond(e) => {
                main.out += "if ";
                main.node(e);
                main.out += " {}";
            }
            S::While(e) => {
                main.out += "while ";
                main.node(e);
                main.out += " {}";
            }
            S::Assert(a, b) => {
                main.out += "AssertEq(";
                main.node(a);
                main.out += ",";
                main.node(b);
                main.out += ");";
            }
        }
    }
    if r_deferred && use_r {
        main.out += &format!("r={};", p.ft_r.as_ref().unwrap().lit_nonzero());
    }
    if q_deferred && use_q {
        main.out += &format!("q={};", p.ft_q.as_ref().unwrap().lit_nonzero());
    }
    main.out += "}";
    // assemble: pieces, calls_decl, main
    let mut out = String::new();
    let mut by_id = HashMap::new();
    let mut ops_by_id = HashMap::new();
    for pc in pieces {
        let off = out.len() as u64;
        for (k, (s, e)) in pc.pr.by_id {
            by_id.insert(k, (s + off, e + off));
        }
        for (k, (s, e)) in pc.pr.ops_by_id {
            ops_by_id.insert(k, (s + off, e + off));
        }
        out += &pc.pr.out;
        let _ = pc.ann;
    }
    out += &calls_decl;
    let off = out.len() as u64;
    for (k, (s, e)) in main.by_id {
        by_id.insert(k, (s + off, e + off));
    }
    for (k, (s, e)) in main.ops_by_id {
        ops_by_id.insert(k, (s + off, e + off));
    }
    for (s, e) in main_ann {
        ann.push((s + off, e + off));
    }
    out += &main.out;
    let _ = (&mut pre, &mut pr);
    let spans = by_id.iter().map(|(k, v)| (*v, *k)).collect();
    let ops = ops_by_id.iter().map(|(k, v)| (*v, *k)).collect();
    Printed { src: out, spans, by_id, ops, ops_by_id, ann }
}

impl T {
    /// non-zero literal of the type (used for deferred-var resolution so that `3`/`true`/`[1]` are visible)
    pub fn lit_nonzero(&self) -> String {
        match self {
            T::I => "3".into(),
            T::B => "true".into(),
            T::A(x) => format!("[{}]", x.lit_nonzero()),
        }
    }
}

// ---------------------------------------------------------------- generator

#[derive(Clone, Copy)]
pub struct Profile {
    pub defer: bool,     // allow r/q
    pub w_q: u64,        // weight of q among leaves (0..)
    pub w_r: u64,
    pub depth: u32,
    pub all_ctx: bool,
    pub exp_only: bool,
}

pub struct G<'a> {
    pub r: &'a mut Rng,
    pub id: u32,
    pub pf: Profile,
    pub allow_rq: bool,
}

impl G<'_> {
    fn mk(&mut self, k: K) -> N {
        n(&mut self.id, k)
    }
    fn leaf(&mut self) -> N {
        let w_q = if self.pf.defer && self.allow_rq { self.pf.w_q } else { 0 };
        let w_r = if self.pf.defer && self.allow_rq { self.pf.w_r } else { 0 };
        let base = 20u64;
        let tot = base + w_q + w_r;
        let c = self.r.below(tot);
        if c < base {
            let k = match self.r.below(12) {
                0 => K::Int(1),
                1 => K::Int(2 + self.r.below(90) as i64),
                2 => K::Bool(true),
                3 => K::Bool(false),
                4 => K::Var(V::X),
                5 => K::Var(V::P),
                6 | 7 => K::Var(V::Ar),
                8 => K::Var(V::Br),
                9 => K::Var(V::Aar),
                10 => K::Int(5),
                _ => K::Bool(true),
            };
            self.mk(k)
        } else if c < base + w_q {
            self.mk(K::Var(V::Q))
        } else {
            self.mk(K::Var(V::R))
        }
    }
    pub fn cond(&mut self) -> N {
        match self.r.below(20) {
            0..=9 => self.mk(K::Bool(true)),
            10..=13 => self.mk(K::Var(V::P)),
            14..=16 => {
                let a = self.mk(K::Var(V::X));
                let b = self.mk(K::Int(1));
                let l = self.mk(K::Bin(Op::Lt, Box::new(a), Box::new(b)));
                l
            }
            17 => self.mk(K::Bool(false)),
            _ => self.expr(1),
        }
    }
    pub fn expr(&mut self, d: u32) -> N {
        if d == 0 {
            return self.leaf();
        }
        match self.r.below(24) {
            0..=4 => self.leaf(),
            5..=11 => {
                // parenthesised if
                let c = self.cond();
                let a = self.branch(d - 1);
                let b = self.branch(d - 1);
                let i = self.mk(K::If(Box::new(c), Box::new(a), Box::new(b)));
                self.mk(K::Grp(Box::new(i)))
            }
            12 | 13 => {
                let op = *self.r.pick(&[Op::Add, Op::Sub, Op::Add, Op::Lt, Op::Eq, Op::Eq]);
                let a = self.expr(d - 1);
                let a = self.opnd(a);
                let b = self.expr(d - 1);
                let b = self.opnd(b);
                self.mk(K::Bin(op, Box::new(a), Box::new(b)))
            }
            14..=17 => {
                let k = 1 + self.r.below(3);
                let es: Vec<N> = (0..k).map(|_| self.expr(d - 1)).collect();
                self.mk(K::Arr(es))
            }
            18 | 19 => {
                let e = self.expr(d - 1);
                self.mk(K::Grp(Box::new(e)))
            }
            20 => {
                let b = match self.r.below(4) {
                    0 => self.mk(K::Var(V::Aar)),
                    1 => self.mk(K::Var(V::Br)),
                    2 => self.mk(K::Var(V::Ar)),
                    _ => self.expr(d - 1),
                };
                let i = if self.r.chance(4, 5) { self.mk(K::Int(0)) } else { self.expr(d - 1) };
                let b = self.opnd(b);
                self.mk(K::Idx(Box::new(b), Box::new(i)))
            }
            21 => if self.allow_rq { self.mk(K::Dv) } else { self.leaf() },
            _ => {
                let a = self.expr(d - 1);
                self.mk(K::Grp(Box::new(a)))
            }
        }
    }
    fn opnd(&mut self, a: N) -> N {
        if matches!(a.k, K::Bin(..)) { self.mk(K::Grp(Box::new(a))) } else { a }
    }
    fn branch(&mut self, d: u32) -> N {
        let e = self.expr(d);
        if self.r.chance(1, 5) {
            self.mk(K::Blk(Box::new(e)))
        } else {
            e
        }
    }
    pub fn ty(&mut self) -> T {
        match self.r.below(10) {
            0..=2 => T::I,
            3..=5 => T::B,
            6 | 7 => arr(T::I),
            8 => arr(T::B),
            _ => arr(arr(T::I)),
        }
    }
    pub fn stmt(&mut self) -> S {
        let d = 1 + self.r.below(self.pf.depth as u64) as u32;
        let c = if self.pf.exp_only { 2 + self.r.below(7) } else { self.r.below(if self.pf.all_ctx { 20 } else { 10 }) };
        match c {
            0 | 1 => S::Let(None, self.expr(d)),
            2..=4 => {
                let t = self.ty();
                S::Let(Some(t), self.expr(d))
            }
            5 => {
                let t = self.ty();
                S::Assign(t, self.expr(d))
            }
            6 => {
                let t = self.ty();
                S::IdxAssign(t, self.expr(d))
            }
            7 => {
                let t = self.ty();
                S::Call(t, self.expr(d))
            }
            8 => S::Cond(self.expr(d)),
            9 => {
                let a = self.expr(d);
                let b = self.expr(d);
                S::Assert(a, b)
            }
            10 | 11 => {
                self.allow_rq = false;
                let t = self.ty();
                let e = self.expr(d);
                self.allow_rq = true;
                S::Ret(t, e)
            }
            12 => S::While(self.expr(d)),
            13 | 14 => {
                // operator contexts
                let a = self.expr(d);
                let b = self.expr(d.saturating_sub(1));
                let op = *self.r.pick(&[Op::Add, Op::Sub, Op::Lt, Op::Eq]);
                let (l, r) = if self.r.chance(1, 2) { (a, b) } else { (b, a) };
                let l = self.opnd(l);
                let r = self.opnd(r);
                let e = self.mk(K::Bin(op, Box::new(l), Box::new(r)));
                S::Let(None, e)
            }
            15 | 16 => {
                let a = self.expr(d);
                let a = self.opnd(a);
                let e = self.mk(K::Idx(Box::new(N { id: 0, k: K::Var(V::Ar) }), Box::new(a)));
                let e = N { id: e.id, k: e.k };
                S::Let(None, e)
            }
            _ => {
                let a = self.expr(d);
                let c = self.cond();
                let b = self.expr(d.saturating_sub(1));
                let i = self.mk(K::If(Box::new(c), Box::new(a), Box::new(b)));
                let g = self.mk(K::Grp(Box::new(i)));
                S::Let(None, g)
            }
        }
    }
    pub fn prog(&mut self) -> Prog {
        let k = 1 + self.r.below(3);
        let mut stmts = Vec::new();
        for _ in 0..k {
            stmts.push(self.stmt());
        }
        let ft_r = Some(match self.r.below(4) {
            0 => arr(T::B),
            1 => arr(T::I),
            2 => arr(arr(T::I)),
            _ => arr(arr(T::B)),
        });
        let ft_q = Some(match self.r.below(5) {
            0 => T::I,
            1 => T::B,
            2 => arr(T::I),
            3 => arr(T::B),
            _ => arr(arr(T::I)),
        });
        let mut p = Prog { stmts, ft_r, ft_q };
        fix_ids(&mut p);
        p
    }
}

/// ids must be unique: the Idx(ar,..) shortcut above uses id 0 for the base; renumber everything.
pub fn fix_ids(p: &mut Prog) {
    let mut next = 0u32;
    fn ren(x: &mut N, next: &mut u32) {
        *next += 1;
        x.id = *next;
        match &mut x.k {
            K::Bin(_, a, b) | K::Idx(a, b) => {
                ren(a, next);
                ren(b, next);
            }
            K::If(c, a, b) => {
                ren(c, next);
                ren(a, next);
                ren(b, next);
            }
            K::Blk(a) | K::Grp(a) => ren(a, next),
            K::Arr(v) => v.iter_mut().for_each(|e| ren(e, next)),
            _ => {}
        }
    }
    for s in &mut p.stmts {
        match s {
            S::Let(_, e) | S::Assign(_, e) | S::IdxAssign(_, e) | S::Ret(_, e) | S::Call(_, e) | S::Cond(e) | S::While(e) => ren(e, &mut next),
            S::Assert(a, b) => {
                ren(a, &mut next);
                ren(b, &mut next);
            }
        }
    }
}
