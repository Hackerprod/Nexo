// Executable model: cada hijo con tipo esperado concreto emite su propio
// diagn-stico en su propio operando; el join solo emite con tipo esperado indeterminado- (F0-0002 -4.1.1, -4.6,
// -4.7, -13.9, -13.15, -13.17). Written from the ADR text; it does not read the implementation.
use crate::ast::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Site {
    Node(u32),
    Op(u32),
}
#[derive(Clone, Debug)]
pub struct MD {
    pub code: &'static str,
    pub sub: &'static str,
    pub site: Site,
    pub exp: String,
    pub found: String,
    pub org: Option<u32>, // E1303: node of the first normal continuation operand
}

#[derive(Clone, Debug)]
enum Tv {
    K(T),
    Never,
    Taint,
}
#[derive(Clone, Debug)]
struct Res {
    ty: Tv,
    ev: Option<(T, u32)>, // evidence type (also kept for partially tainted joins)
    def: u8,              // 1 = derived from q (unknown until resolved), 2 = derived from r
}
impl Res {
    fn k(t: T, id: u32, def: u8) -> Res {
        Res { ev: Some((t.clone(), id)), ty: Tv::K(t), def }
    }
    fn taint(def: u8) -> Res {
        Res { ty: Tv::Taint, ev: None, def }
    }
}

pub struct Model {
    pub diags: Vec<MD>,
    pub uncertain: Vec<&'static str>,
    pub nrep: u32,
    pub partial_eq: bool, // compare a valid continuation's partial witness after an invalid `if` arm
    ft_r: T,
    ft_q: T,
}

fn is_never(x: &N) -> bool {
    match &x.k {
        K::Dv => true,
        K::Grp(i) | K::Blk(i) => is_never(i),
        K::If(_, a, b) => is_never(a) && is_never(b),
        _ => false,
    }
}
/// site of a later continuation: a nested `if` contributes its first normal leaf (-4.1.1 -primera restricci-n incompatible-)
fn leaf_site(x: &N) -> u32 {
    match &x.k {
        K::Grp(i) | K::Blk(i) => leaf_site(i),
        K::If(_, a, b) => {
            if !is_never(a) {
                leaf_site(a)
            } else {
                leaf_site(b)
            }
        }
        _ => x.id,
    }
}
fn prim(x: &N) -> u32 {
    match &x.k {
        K::Grp(i) | K::Blk(i) => prim(i),
        _ => x.id,
    }
}

impl Model {
    #[allow(clippy::too_many_arguments)]
    fn later(&mut self, b: &N, t0: &T, o0: u32, d0: u8, reported: &mut bool, tainted: &mut bool, d: &mut u8) {
        match &b.k {
            K::Grp(i) | K::Blk(i) => self.later(i, t0, o0, d0, reported, tainted, d),
            K::If(c, x, y) => {
                let rc = self.chk(c, Some(&T::B));
                if matches!(rc.ty, Tv::Taint) {
                    self.unc("erroneous if condition");
                }
                // each nested `if` owns one join: its first incompatible leaf is denounced against the inherited type
                let mut local = false;
                self.later_in(x, t0, o0, d0, &mut local, tainted, d);
                self.later_in(y, t0, o0, d0, &mut local, tainted, d);
                if local {
                    *tainted = true;
                    self.unc("report inside a nested later continuation");
                }
                let _ = reported;
            }
            _ => {
                let r = self.chk(b, None);
                *d |= r.def;
                match &r.ty {
                    Tv::K(t) => {
                        if t != t0 && !*reported {
                            let t = t.clone();
                            self.defcheck(d0, t0, r.def, &t);
                            self.push("NX-E1303", "", Site::Node(prim(b)), t0, &t, Some(o0));
                            *reported = true;
                            self.nrep += 1;
                        }
                    }
                    Tv::Taint => *tainted = true,
                    Tv::Never => {}
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn later_in(&mut self, b: &N, t0: &T, o0: u32, d0: u8, local: &mut bool, tainted: &mut bool, d: &mut u8) {
        match &b.k {
            K::Grp(i) | K::Blk(i) => self.later_in(i, t0, o0, d0, local, tainted, d),
            K::If(..) => {
                let mut sub = false;
                self.later(b, t0, o0, d0, &mut sub, tainted, d);
            }
            _ => self.later(b, t0, o0, d0, local, tainted, d),
        }
    }
    fn unc(&mut self, why: &'static str) {
        if !self.uncertain.contains(&why) {
            self.uncertain.push(why);
        }
    }
    fn push(&mut self, code: &'static str, sub: &'static str, site: Site, exp: &T, found: &T, org: Option<u32>) {
        self.diags.push(MD { code, sub, site, exp: exp.s(), found: found.s(), org });
    }
    fn defcheck(&mut self, a: u8, ta: &T, b: u8, tb: &T) {
        // a mismatch whose blame may move to the later resolving assignment of a deferred variable
        if (a | b) & 1 != 0 {
            self.unc("deferred binding mismatch");
        } else if (a | b) & 2 != 0 && ta.is_arr() && tb.is_arr() {
            self.unc("r elem-level mismatch");
        }
    }
    fn fin(&mut self, x: &N, r: Res, exp: Option<&T>) -> Res {
        if let (Some(t), Tv::K(u)) = (exp, &r.ty) {
            if t != u {
                let (t2, u2) = (t.clone(), u.clone());
                self.defcheck(0, &t2, r.def, &u2);
                self.push("NX-E0102", "type_mismatch", Site::Node(prim(x)), &t2, &u2, None);
                return Res::taint(r.def);
            }
        }
        r
    }
    fn eff(&self, r: &Res) -> Tv {
        match (&r.ty, self.partial_eq, &r.ev) {
            (Tv::Taint, true, Some((t, _))) => Tv::K(t.clone()),
            _ => r.ty.clone(),
        }
    }
    pub fn chk(&mut self, x: &N, exp: Option<&T>) -> Res {
        match &x.k {
            K::Grp(i) => self.chk(i, exp),
            K::Blk(i) => self.chk(i, exp),
            K::Int(_) => self.fin(x, Res::k(T::I, x.id, 0), exp),
            K::Bool(_) => self.fin(x, Res::k(T::B, x.id, 0), exp),
            K::Var(v) => {
                let (t, d) = match v {
                    V::X => (T::I, 0),
                    V::P => (T::B, 0),
                    V::Ar => (arr(T::I), 0),
                    V::Br => (arr(T::B), 0),
                    V::Aar => (arr(arr(T::I)), 0),
                    V::R => (self.ft_r.clone(), 2),
                    V::Q => (self.ft_q.clone(), 1),
                };
                self.fin(x, Res::k(t, x.id, d), exp)
            }
            K::Dv => Res { ty: Tv::Never, ev: None, def: 0 },
            K::Bin(op, l, r) => match op {
                Op::Add | Op::Sub => {
                    // 4.6/13.17: only the first incompatible operand (left to right) is denounced; once the left
                    // operand is erroneous the right one is analysed without the intrinsic domain (synthesis).
                    let n0 = self.diags.len();
                    let a = self.chk(l, Some(&T::I));
                    if self.diags.len() > n0 && matches!(l.k, K::Var(V::R) | K::Var(V::Q)) {
                        self.unc("deferred left operand of arithmetic");
                    }
                    let b = if matches!(a.ty, Tv::Taint) { self.chk(r, None) } else { self.chk(r, Some(&T::I)) };
                    let d = a.def | b.def;
                    let res = if matches!(a.ty, Tv::Taint) || matches!(b.ty, Tv::Taint) { Res::taint(0) } else { Res::k(T::I, x.id, 0) };
                    let _ = d;
                    self.fin(x, res, exp)
                }
                Op::Lt | Op::Eq => {
                    let a = self.chk(l, None);
                    let b = self.chk(r, None);
                    let d = a.def | b.def;
                    if (matches!(a.ty, Tv::Taint) && a.ev.is_some()) || (matches!(b.ty, Tv::Taint) && b.ev.is_some()) {
                        self.unc("partially tainted operand of comparison");
                    }
                    let (ea, eb) = (self.eff(&a), self.eff(&b));
                    if *op == Op::Lt && (matches!(ea, Tv::Never) || matches!(eb, Tv::Never)) {
                        self.unc("lt with never operand");
                    }
                    let res = match (ea, eb) {
                        (Tv::Taint, _) | (_, Tv::Taint) => Res::taint(d),
                        (Tv::K(p), Tv::K(q)) => {
                            if p != q {
                                self.defcheck(a.def, &p, b.def, &q);
                                self.push("NX-E0102", "type_mismatch", Site::Node(prim(r)), &p, &q, None);
                                Res::taint(d)
                            } else if *op == Op::Lt && p != T::I {
                                self.push("NX-E0102", "type_mismatch", Site::Op(x.id), &T::I, &p, None);
                                if d & 1 != 0 {
                                    self.unc("deferred binding mismatch");
                                }
                                Res::taint(d)
                            } else {
                                Res::k(T::B, x.id, 0)
                            }
                        }
                        (Tv::K(p), Tv::Never) | (Tv::Never, Tv::K(p)) => {
                            if *op == Op::Lt && p != T::I {
                                self.unc("lt with never operand");
                            }
                            Res::k(T::B, x.id, 0)
                        }
                        _ => {
                            self.unc("both never");
                            Res::k(T::B, x.id, 0)
                        }
                    };
                    self.fin(x, res, exp)
                }
            },
            K::Idx(b, i) => {
                let rb = self.chk(b, None);
                if matches!(rb.ty, Tv::Taint) && rb.ev.is_some() {
                    self.unc("partially tainted index base");
                }
                let base_is_arr = matches!(&rb.ty, Tv::K(T::A(_)));
                let n0 = self.diags.len();
                let ri = if base_is_arr { self.chk(i, Some(&T::I)) } else { self.chk(i, None) };
                if !base_is_arr && self.diags.len() > n0 {
                    self.unc("index has errors under a non-indexable base");
                }
                let d = rb.def | ri.def;
                if matches!(ri.ty, Tv::Taint) {
                    self.unc("index result with erroneous selector");
                }
                if matches!(rb.ty, Tv::Taint) {
                    self.unc("selector under tainted base");
                }
                let res = match &rb.ty {
                    Tv::K(T::A(e)) => {
                        if matches!(ri.ty, Tv::Taint) {
                            Res::taint(d)
                        } else {
                            Res::k((**e).clone(), x.id, if rb.def != 0 { 1 } else { 0 })
                        }
                    }
                    Tv::K(t) => {
                        let t = t.clone();
                        self.diags.push(MD { code: "NX-E0102", sub: "not_indexable", site: Site::Node(prim(b)), exp: "domain:array".into(), found: t.s(), org: None });
                        Res::taint(d)
                    }
                    Tv::Never => {
                        self.unc("index of never");
                        Res::taint(d)
                    }
                    Tv::Taint => Res::taint(d),
                };
                self.fin(x, res, exp)
            }
            K::Arr(es) => {
                if let Some(T::A(u)) = exp {
                    let u = (**u).clone();
                    let mut any = false;
                    let mut d = 0;
                    for e in es {
                        let r = self.chk(e, Some(&u));
                        d |= r.def;
                        if matches!(r.ty, Tv::Taint) {
                            any = true;
                        }
                    }
                    let d = if d != 0 { 1 } else { 0 };
                    return if any { Res::taint(d) } else { Res::k(arr(u), x.id, d) };
                }
                let mut t0: Option<T> = None;
                let mut tainted = false;
                let mut d = 0;
                let mut t0_def = 0u8;
                let mut n_at_fix = 0usize;
                for e in es {
                    let r = match &t0 {
                        None => self.chk(e, None),
                        Some(t) => {
                            let t = t.clone();
                            self.chk(e, Some(&t))
                        }
                    };
                    d |= r.def;
                    if t0.is_some() && t0_def != 0 && self.diags.len() > n_at_fix {
                        self.unc("array elements checked against a deferred element type");
                    }
                    match (&r.ty, &t0) {
                        (Tv::K(t), None) => {
                            t0 = Some(t.clone());
                            t0_def = r.def;
                            n_at_fix = self.diags.len();
                        }
                        (Tv::Taint, None) => {
                            tainted = true;
                            self.unc("tainted first element");
                        }
                        (Tv::Taint, Some(_)) => tainted = true,
                        _ => {}
                    }
                }
                let d = if d != 0 { 1 } else { 0 };
                let base = match t0 {
                    None => {
                        self.unc("array without concrete element");
                        Res::taint(d)
                    }
                    Some(t) => {
                        if tainted {
                            { let _ = t; Res { ty: Tv::Taint, ev: None, def: d } }
                        } else {
                            Res::k(arr(t), x.id, d)
                        }
                    }
                };
                if exp.is_some() {
                    self.fin(x, base, exp)
                } else {
                    base
                }
            }
            K::If(c, a, b) => {
                let rc = self.chk(c, Some(&T::B));
                if matches!(rc.ty, Tv::Taint) {
                    self.unc("erroneous if condition");
                }
                match exp {
                    Some(t) => {
                        let t = t.clone();
                        let ra = self.chk(a, Some(&t));
                        let rb = self.chk(b, Some(&t));
                        let d = ra.def | rb.def;
                        if matches!(ra.ty, Tv::Taint) || matches!(rb.ty, Tv::Taint) {
                            Res { ty: Tv::Taint, ev: Some((t, x.id)), def: d }
                        } else if matches!(ra.ty, Tv::Never) && matches!(rb.ty, Tv::Never) {
                            Res { ty: Tv::Never, ev: None, def: d }
                        } else {
                            Res::k(t, x.id, if d != 0 { 1 } else { 0 })
                        }
                    }
                    None => {
                        // First continuation synthesizes (own join); later continuations are walked leaf by leaf
                        // against its type and only the first incompatible leaf is denounced (one E1303 per join).
                        let before_a = self.nrep;
                        let ra = self.chk(a, None);
                        if self.nrep > before_a && matches!(ra.ty, Tv::Taint) {
                            self.unc("first continuation contaminated by a nested join report");
                        }
                        let t0: Option<(T, u32)> = match &ra.ty {
                            Tv::K(t) => Some((t.clone(), prim(a))),
                            Tv::Taint => ra.ev.as_ref().map(|e| (e.0.clone(), prim(a))),
                            Tv::Never => None,
                        };
                        match t0 {
                            None => {
                                // first continuation does not fix a type: the second one is the first normal continuation
                                let rb = self.chk(b, None);
                                if matches!(ra.ty, Tv::Taint) {
                                    Res { ty: Tv::Taint, ev: rb.ev.clone(), def: ra.def | rb.def }
                                } else {
                                    Res { ty: rb.ty.clone(), ev: rb.ev.clone(), def: ra.def | rb.def }
                                }
                            }
                            Some((t0, o0)) => {
                                let mut reported = false;
                                let before = self.nrep;
                                let mut tainted = matches!(ra.ty, Tv::Taint);
                                let mut d = ra.def;
                                self.later(b, &t0, o0, ra.def, &mut reported, &mut tainted, &mut d);
                                if self.nrep - before >= 2 {
                                    self.unc("several reports inside a nested later continuation");
                                }
                                if reported {
                                    // a failed join demonstrates no type
                                    Res { ty: Tv::Taint, ev: None, def: d }
                                } else if tainted {
                                    Res { ty: Tv::Taint, ev: Some((t0, x.id)), def: d }
                                } else {
                                    Res::k(t0, x.id, if d != 0 { 1 } else { 0 })
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn run_model(p: &Prog, partial_eq: bool) -> Model {
    let mut m = Model {
        diags: vec![],
        uncertain: vec![],
        nrep: 0,
        partial_eq,
        ft_r: p.ft_r.clone().unwrap_or(T::I),
        ft_q: p.ft_q.clone().unwrap_or(T::I),
    };
    for s in &p.stmts {
        match s {
            S::Let(a, e) => {
                m.chk(e, a.as_ref());
            }
            S::Assign(t, e) | S::IdxAssign(t, e) | S::Ret(t, e) | S::Call(t, e) => {
                m.chk(e, Some(t));
            }
            S::Cond(e) | S::While(e) => {
                m.chk(e, Some(&T::B));
            }
            S::Assert(a, b) => {
                let ra = m.chk(a, None);
                let n0 = m.diags.len();
                let rb = m.chk(b, None);
                if matches!(ra.ty, Tv::Taint) && m.diags.len() > n0 {
                    m.unc("assert: second argument has errors after erroneous first");
                }
                if (matches!(ra.ty, Tv::Taint) && ra.ev.is_some()) || (matches!(rb.ty, Tv::Taint) && rb.ev.is_some()) {
                    m.unc("partially tainted operand of comparison");
                }
                let (ea, eb) = (m.eff(&ra), m.eff(&rb));
                if let (Tv::K(p), Tv::K(q)) = (ea, eb) {
                    if p != q {
                        m.defcheck(ra.def, &p, rb.def, &q);
                        m.push("NX-E0102", "type_mismatch", Site::Node(prim(b)), &p, &q, None);
                    }
                }
            }
        }
    }
    m
}
