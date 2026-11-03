// (d) Own executable model of: "each child with a concrete expected type emits its own diagnostic on its own operand;
// the join only emits with an indeterminate expected type" (ADR 4.7, 13.9, 13.15, 13.17). Concrete types only.
use crate::common::*;
use crate::grp::{assemble_with, ident};
use crate::tree_gen::*;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

#[derive(Clone, PartialEq, Debug)]
pub enum Ty {
    I,
    B,
    A(Box<Ty>),
}
impl Ty {
    pub fn s(&self) -> String {
        match self {
            Ty::I => "I64".into(),
            Ty::B => "Bool".into(),
            Ty::A(t) => format!("Array<{}>", t.s()),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Org {
    None,
    Written(&'static str),
    Node(usize),
    Param(usize),
}
#[derive(Clone, Debug)]
pub struct Dg {
    pub code: &'static str,
    pub sub: &'static str,
    pub prim: String, // "n12" or "op12"
    pub exp: String,
    pub found: String,
    pub org: Org,
}
pub struct Res {
    pub ty: Option<Ty>,
    pub d: Vec<Dg>,
    pub abstain: bool,
    pub context_rejected: bool,
}
impl Res {
    fn ok(t: Ty) -> Res {
        Res { ty: Some(t), d: vec![], abstain: false, context_rejected: false }
    }
    fn tainted(d: Vec<Dg>) -> Res {
        Res { ty: None, d, abstain: false, context_rejected: true }
    }
}

pub fn name_ty(n: &str) -> Ty {
    match n {
        "x" => Ty::I,
        "p" => Ty::B,
        "ar" => Ty::A(Box::new(Ty::I)),
        "br" => Ty::A(Box::new(Ty::B)),
        "aar" => Ty::A(Box::new(Ty::A(Box::new(Ty::I)))),
        _ => panic!("name {n}"),
    }
}

fn leaf_ty(n: &N) -> Option<Ty> {
    match &n.k {
        K::Int(_) => Some(Ty::I),
        K::Bool(_) => Some(Ty::B),
        K::Name(s) => Some(name_ty(s)),
        _ => None,
    }
}
fn merge(a: &mut Res, b: Res) -> Option<Ty> {
    a.d.extend(b.d);
    a.abstain |= b.abstain;
    b.ty
}
fn val(n: &N) -> &N {
    match &n.k {
        K::Blk(e) => val(e),
        _ => n,
    }
}

pub fn check(e: &N, exp: &Ty, org: &Org) -> Res {
    let e = val(e);
    if let Some(t) = leaf_ty(e) {
        if &t == exp {
            return Res::ok(t);
        }
        return Res::tainted(vec![Dg { code: "NX-E0102", sub: "type_mismatch", prim: format!("n{}", e.id), exp: exp.s(), found: t.s(), org: org.clone() }]);
    }
    match &e.k {
        K::Arr(items) => {
            let Ty::A(x) = exp else {
                return Res { ty: None, d: vec![], abstain: true, context_rejected: false };
            };
            let mut r = Res::ok(exp.clone());
            for it in items {
                let c = check(it, x, org);
                r.d.extend(c.d);
                r.abstain |= c.abstain;
                r.context_rejected |= c.context_rejected;
            }
            if !r.d.is_empty() {
                r.ty = None; // §4.1.2.5: a failed constructor has no result type.
            }
            r
        }
        K::If(c, t, f) => {
            let mut r = Res::ok(exp.clone());
            let cc = check(c, &Ty::B, &Org::None);
            if !cc.d.is_empty() {
                r.abstain = true;
            }
            r.d.extend(cc.d);
            r.abstain |= cc.abstain;
            for br in [t, f] {
                let b = check(br, exp, org);
                r.d.extend(b.d);
                r.abstain |= b.abstain;
                r.context_rejected |= b.context_rejected;
            }
            if !r.d.is_empty() {
                r.ty = None; // §4.1.2.5: a failed union cannot type its consumer.
            }
            r
        }
        _ => {
            let mut s = synth(e);
            if s.abstain {
                return s;
            }
            match &s.ty {
                Some(t) if t != exp => {
                    s.d.push(Dg { code: "NX-E0102", sub: "type_mismatch", prim: format!("n{}", e.id), exp: exp.s(), found: t.s(), org: org.clone() });
                    s.ty = None;
                    s.context_rejected = true;
                    s
                }
                _ => s,
            }
        }
    }
}

pub fn synth(e: &N) -> Res {
    let e = val(e);
    if let Some(t) = leaf_ty(e) {
        return Res::ok(t);
    }
    match &e.k {
        K::Arr(items) => {
            let mut r = Res::ok(Ty::I);
            let first = synth(&items[0]);
            let ft = merge(&mut r, first);
            let Some(ft) = ft else {
                r.abstain = true;
                return r;
            };
            if !r.d.is_empty() {
                r.abstain = true; // erroneous first element: later comparisons are tainted-vs-independent (gap)
            }
            for it in &items[1..] {
                let c = check(it, &ft, &Org::Node(val(&items[0]).id));
                r.d.extend(c.d);
                r.abstain |= c.abstain;
            }
            r.ty = if r.d.is_empty() { Some(Ty::A(Box::new(ft))) } else { None };
            r
        }
        K::If(..) => {
            let mut r = Res::ok(Ty::I);
            let mut leaves: Vec<&N> = Vec::new();
            fn walk<'a>(n: &'a N, r: &mut Res, leaves: &mut Vec<&'a N>) {
                if let K::If(c, t, f) = &n.k {
                    let cc = check(c, &Ty::B, &Org::None);
                    if !cc.d.is_empty() {
                        r.abstain = true;
                    }
                    r.d.extend(cc.d);
                    r.abstain |= cc.abstain;
                    for br in [t, f] {
                        if matches!(br.k, K::If(..)) {
                            walk(br, r, leaves);
                        } else {
                            if matches!(val(br).k, K::If(..)) {
                                r.abstain = true;
                            }
                            leaves.push(br);
                        }
                    }
                }
            }
            walk(e, &mut r, &mut leaves);
            let mut tys: Vec<Option<Ty>> = Vec::new();
            for lf in &leaves {
                let s = synth(lf);
                r.abstain |= s.abstain;
                r.d.extend(s.d);
                tys.push(s.ty);
            }
            let mut refi = None;
            for (i, t) in tys.iter().enumerate() {
                if t.is_some() {
                    refi = Some(i);
                    break;
                }
            }
            let mut bad = 0;
            if let Some(ri) = refi {
                let rt = tys[ri].clone().unwrap();
                for i in ri + 1..leaves.len() {
                    if let Some(t) = &tys[i] {
                        if *t != rt {
                            bad += 1;
                            if bad == 1 {
                                r.d.push(Dg { code: "NX-E1303", sub: "", prim: format!("n{}", val(leaves[i]).id), exp: rt.s(), found: t.s(), org: Org::Node(val(leaves[ri]).id) });
                            }
                        }
                    }
                }
                if bad > 1 {
                    r.abstain = true;
                }
                r.ty = if bad == 0 && tys.iter().all(|t| t.is_some()) { Some(rt) } else { None };
            } else {
                r.ty = None;
            }
            r
        }
        K::Bin(op, l, rr) => {
            let mut r = Res::ok(Ty::I);
            match *op {
                "+" | "-" => {
                    let a = check(l, &Ty::I, &Org::None);
                    // §4.1.2.4, RD94/RD126: a rejected context on the left
                    // suppresses a second domain report from this operation.
                    // Internal left errors leave the right I64 obligation
                    // independent, including its distributed child checks.
                    let b = if a.context_rejected {
                        synth(rr)
                    } else {
                        check(rr, &Ty::I, &Org::None)
                    };
                    let bad = !a.d.is_empty() || !b.d.is_empty() || a.abstain || b.abstain;
                    r.d.extend(a.d);
                    r.d.extend(b.d);
                    r.abstain |= a.abstain | b.abstain;
                    r.ty = if bad { None } else { Some(Ty::I) };
                }
                _ => {
                    let a = synth(l);
                    let lt = a.ty.clone();
                    r.d.extend(a.d);
                    r.abstain |= a.abstain;
                    let b = synth(rr);
                    let rt = b.ty.clone();
                    r.d.extend(b.d);
                    r.abstain |= b.abstain;
                    match (lt, rt) {
                        (Some(lt), Some(rt)) => {
                            if lt != rt {
                                r.d.push(Dg { code: "NX-E0102", sub: "type_mismatch", prim: format!("n{}", val(rr).id), exp: lt.s(), found: rt.s(), org: Org::Node(val(l).id) });
                                r.ty = None;
                            } else if *op == "<" && lt != Ty::I {
                                r.d.push(Dg { code: "NX-E0102", sub: "type_mismatch", prim: format!("op{}", e.id), exp: "I64".into(), found: lt.s(), org: Org::None });
                                r.ty = None;
                            } else {
                                r.ty = Some(Ty::B);
                            }
                        }
                        (Some(t), None) | (None, Some(t)) => {
                            // §4.1.2.4: the valid operand keeps its own
                            // intrinsic domain check after the cross-type
                            // equality is suppressed by the failed sibling.
                            if *op == "<" && t != Ty::I {
                                r.d.push(Dg { code: "NX-E0102", sub: "type_mismatch", prim: format!("op{}", e.id), exp: "I64".into(), found: t.s(), org: Org::None });
                            }
                            r.ty = None;
                        }
                        _ => r.ty = None,
                    }
                }
            }
            r
        }
        K::Idx(b, i) => {
            let mut r = Res::ok(Ty::I);
            let base = synth(b);
            let bt = base.ty.clone();
            r.d.extend(base.d);
            r.abstain |= base.abstain;
            // §4.1.2.3, RD84/RD121: I64 is conditional on an Array base.
            let ic = if matches!(bt, Some(Ty::A(_))) {
                check(i, &Ty::I, &Org::None)
            } else {
                synth(i)
            };
            let ibad = !ic.d.is_empty();
            r.d.extend(ic.d);
            r.abstain |= ic.abstain;
            match bt {
                Some(Ty::A(x)) => r.ty = if ibad { None } else { Some(*x) },
                Some(t) => {
                    r.d.push(Dg { code: "NX-E0102", sub: "not_indexable", prim: format!("n{}", val(b).id), exp: "domain:array".into(), found: t.s(), org: Org::None });
                    r.ty = None;
                }
                None => r.ty = None,
            }
            r
        }
        K::Len(a) => {
            let mut r = Res::ok(Ty::I);
            let c = synth(a);
            r.abstain |= c.abstain;
            let t = c.ty.clone();
            r.d.extend(c.d);
            match t {
                Some(Ty::A(_)) => {}
                Some(found) => {
                    // §4.1.2.3, RD86: non-Array arguments fail on the
                    // argument itself; an invalid child has no domain result.
                    r.d.push(Dg { code: "NX-E0102", sub: "not_indexable", prim: format!("n{}", val(a).id), exp: "domain:array".into(), found: found.s(), org: Org::None });
                    r.ty = None;
                }
                None => r.ty = None,
            }
            r
        }
        _ => Res { ty: None, d: vec![], abstain: true, context_rejected: false },
    }
}

// ---------- typed-biased generator ----------
pub struct TG<'a> {
    pub r: &'a mut Rng,
    pub ids: Ids,
    pub extra: bool,
    pub avoid: &'static str,
}
impl<'a> TG<'a> {
    fn fin(&mut self, mut n: N) -> N {
        if matches!(n.k, K::If(..)) {
            n.mand = 1;
        }
        n.extra = if self.extra {
            match self.r.below(10) {
                0..=6 => 0,
                7..=8 => 1,
                _ => 2,
            }
        } else {
            0
        };
        n
    }
    fn mk(&mut self, k: K) -> N {
        let n = self.ids.n(k);
        self.fin(n)
    }
    pub fn mkt_ty(&mut self) -> Ty {
        self.rand_ty()
    }
    fn rand_ty(&mut self) -> Ty {
        match self.r.below(6) {
            0 | 1 => Ty::I,
            2 | 3 => Ty::B,
            4 => Ty::A(Box::new(Ty::I)),
            _ => {
                if self.r.chance(1, 2) {
                    Ty::A(Box::new(Ty::B))
                } else {
                    Ty::A(Box::new(Ty::A(Box::new(Ty::I))))
                }
            }
        }
    }
    fn names_of(t: &Ty) -> Vec<&'static str> {
        match t {
            Ty::I => vec!["x"],
            Ty::B => vec!["p"],
            Ty::A(i) => match **i {
                Ty::I => vec!["ar"],
                Ty::B => vec!["br"],
                Ty::A(_) => vec!["aar"],
            },
        }
    }
    pub fn mkt(&mut self, want: &Ty, depth: u32) -> N {
        // mutation: wrong type
        let want = if self.r.chance(1, 8) { self.rand_ty() } else { want.clone() };
        let want = &want;
        if depth == 0 || self.r.chance(1, 4) {
            return self.leaf(want);
        }
        let d = depth - 1;
        match self.r.below(8) {
            0 | 1 => self.leaf(want),
            2 | 3 => match want {
                Ty::I => {
                    let op = if self.r.chance(1, 2) { "+" } else { "-" };
                    let mut l = self.mkt(&Ty::I, d);
                    let mut r = self.mkt(&Ty::I, d);
                    if matches!(l.k, K::Bin(..)) {
                        l.mand = 1;
                    }
                    if matches!(r.k, K::Bin(..)) {
                        r.mand = 1;
                    }
                    self.mk(K::Bin(op, Box::new(l), Box::new(r)))
                }
                Ty::B => {
                    if self.r.chance(1, 2) {
                        let mut l = self.mkt(&Ty::I, d);
                        let mut r = self.mkt(&Ty::I, d);
                        if matches!(l.k, K::Bin(..)) {
                            l.mand = 1;
                        }
                        if matches!(r.k, K::Bin(..)) {
                            r.mand = 1;
                        }
                        self.mk(K::Bin("<", Box::new(l), Box::new(r)))
                    } else {
                        let u = self.rand_ty();
                        let mut l = self.mkt(&u, d);
                        let mut r = self.mkt(&u, d);
                        if matches!(l.k, K::Bin(..)) {
                            l.mand = 1;
                        }
                        if matches!(r.k, K::Bin(..)) {
                            r.mand = 1;
                        }
                        self.mk(K::Bin("==", Box::new(l), Box::new(r)))
                    }
                }
                Ty::A(x) => {
                    let n = 1 + self.r.below(3) as usize;
                    let items = (0..n).map(|_| self.mkt(x, d)).collect();
                    self.mk(K::Arr(items))
                }
            },
            4 | 5 => {
                let c = self.mkt(&Ty::B, d);
                let mut br = |g: &mut Self| {
                    let v = g.mkt(want, d);
                    if g.r.chance(1, 3) { g.mk(K::Blk(Box::new(v))) } else { v }
                };
                let t = br(self);
                let f = br(self);
                self.mk(K::If(Box::new(c), Box::new(t), Box::new(f)))
            }
            6 => match want {
                Ty::I if self.avoid == "ar" => self.leaf(want),
                Ty::I => {
                    let base = self.mk(K::Name("ar"));
                    let i = self.mkt(&Ty::I, d);
                    self.mk(K::Idx(Box::new(base), Box::new(i)))
                }
                Ty::A(x) if **x == Ty::I && self.avoid == "aar" => self.leaf(want),
                Ty::A(x) if **x == Ty::I => {
                    let base = self.mk(K::Name("aar"));
                    let i = self.mkt(&Ty::I, d);
                    self.mk(K::Idx(Box::new(base), Box::new(i)))
                }
                _ => self.leaf(want),
            },
            _ => match want {
                Ty::I => {
                    let nm = *self.r.pick(&["ar", "br", "aar"]);
                    let n = self.mk(K::Name(nm));
                    self.mk(K::Len(Box::new(n)))
                }
                _ => self.leaf(want),
            },
        }
    }
    fn leaf(&mut self, want: &Ty) -> N {
        match want {
            Ty::I => {
                if self.r.chance(1, 2) {
                    let v = self.r.below(300) as u32;
                    self.mk(K::Int(v))
                } else {
                    self.mk(K::Name("x"))
                }
            }
            Ty::B => {
                if self.r.chance(1, 2) {
                    let v = self.r.chance(1, 2);
                    self.mk(K::Bool(v))
                } else {
                    self.mk(K::Name("p"))
                }
            }
            Ty::A(x) => {
                if self.r.chance(1, 2) {
                    let n = 1 + self.r.below(3) as usize;
                    let items = (0..n).map(|_| self.leaf(x)).collect();
                    self.mk(K::Arr(items))
                } else {
                    let nm = Self::names_of(want)[0];
                    self.mk(K::Name(nm))
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MCtx {
    LetAnn,
    LetPlain,
    Assign,
    IdxAssign,
    Call,
    If,
    While,
    AssertEq,
    Return,
}
const MCTXS: [MCtx; 9] = [MCtx::LetAnn, MCtx::LetPlain, MCtx::Assign, MCtx::IdxAssign, MCtx::Call, MCtx::If, MCtx::While, MCtx::AssertEq, MCtx::Return];

#[derive(Default)]
pub struct MStats {
    pub programs: AtomicU64,
    pub abstained: AtomicU64,
    pub compared: AtomicU64,
    pub agree: AtomicU64,
    pub code_set_diff: AtomicU64,
    pub prim_diff: AtomicU64,
    pub field_diff: AtomicU64,
    pub origin_diff: AtomicU64,
    pub accept_diff: AtomicU64,
    pub model_diags: AtomicU64,
    pub impl_diags: AtomicU64,
    pub dups: AtomicU64,
    pub bugs: AtomicU64,
    pub ex: Mutex<BTreeMap<String, Vec<String>>>,
    pub accepted_prog: AtomicU64,
    pub run_unit: AtomicU64,
    pub run_other: AtomicU64,
}

pub fn run(n: u64, seed: u64, threads: u64, tag: &str) {
    let st = MStats::default();
    let run_too = std::env::var("V10_RUN").is_ok();
    par(n, threads, |i| {
        let mut r = Rng::new(seed.wrapping_mul(7_919).wrapping_add(i));
        let ctx = *r.pick(&MCTXS);
        let tyi = r.below(5) as usize;
        let ty = match tyi {
            0 => Ty::I,
            1 => Ty::B,
            2 => Ty::A(Box::new(Ty::I)),
            3 => Ty::A(Box::new(Ty::B)),
            _ => Ty::A(Box::new(Ty::A(Box::new(Ty::I)))),
        };
        let depth = 1 + r.below(4) as u32;
        let (e1, e2, want1);
        {
            let mut g = TG { r: &mut r, ids: Ids(0), extra: true, avoid: "" };
            want1 = match ctx {
                MCtx::If | MCtx::While => Ty::B,
                MCtx::IdxAssign => Ty::I,
                _ => ty.clone(),
            };
            let w2 = g.rand_ty();
            e1 = g.mkt(&want1, depth);
            e2 = g.mkt(&w2, depth);
        }
        // model
        let res: Res;
        let mut org_for = |o: Org| o;
        let _ = &mut org_for;
        let (gctx, gname): (crate::grp::Ctx, ()) = match ctx {
            MCtx::LetAnn => (crate::grp::Ctx::LetAnn, ()),
            MCtx::LetPlain => (crate::grp::Ctx::LetPlain, ()),
            MCtx::Assign => (crate::grp::Ctx::Assign, ()),
            MCtx::IdxAssign => (crate::grp::Ctx::IdxAssign, ()),
            MCtx::Call => (crate::grp::Ctx::Call, ()),
            MCtx::If => (crate::grp::Ctx::If, ()),
            MCtx::While => (crate::grp::Ctx::While, ()),
            MCtx::AssertEq => (crate::grp::Ctx::AssertEq, ()),
            MCtx::Return => (crate::grp::Ctx::Return, ()),
        };
        let _ = gname;
        res = match ctx {
            MCtx::LetAnn => check(&e1, &ty, &Org::Written("ann")),
            MCtx::LetPlain => synth(&e1),
            MCtx::Assign => check(&e1, &ty, &Org::Written("ann")),
            MCtx::IdxAssign => check(&e1, &Ty::I, &Org::Written("ar0")),
            MCtx::Call => check(&e1, &ty, &Org::Param(tyi)),
            MCtx::If | MCtx::While => check(&e1, &Ty::B, &Org::None),
            MCtx::AssertEq => {
                let mut a = synth(&e1);
                let b = synth(&e2);
                let bt = b.ty.clone();
                let at = a.ty.clone();
                a.d.extend(b.d);
                a.abstain |= b.abstain;
                // §4.1.2.5/.6: both arguments are independent, but their
                // cross-type equality needs two valid argument types.
                if let (Some(t), Some(bt)) = (at, bt) {
                    if bt != t {
                        a.d.push(Dg { code: "NX-E0102", sub: "type_mismatch", prim: format!("n{}", val(&e2).id), exp: t.s(), found: bt.s(), org: Org::Node(val(&e1).id) });
                    }
                }
                a
            }
            MCtx::Return => check(&e1, &ty, &Org::Written("ret")),
        };
        let mut names: std::collections::HashMap<usize, &'static str> = std::collections::HashMap::new();
        collect_names(&e1, &mut names);
        collect_names(&e2, &mut names);
        let spec = crate::grp::Spec { ctx: gctx, ty: tyi, e1, e2, resolve: 0, use_d: false, defer: None, live: true };
        let b = assemble_with(&spec, i % 2 == 1, true);
        st.programs.fetch_add(1, Relaxed);
        let imp = check_prog(&b.out);
        if run_too {
            let (rr, outcome) = crate::common::run(&b.out);
            if rr != imp {
                st.bugs.fetch_add(1, Relaxed);
                st.ex.lock().unwrap().entry("INCOHERENT".into()).or_default().push(b.out.clone());
            }
            if imp == R::Ok {
                match outcome.as_str() {
                    "Unit" => st.run_unit.fetch_add(1, Relaxed),
                    _ => st.run_other.fetch_add(1, Relaxed),
                };
            }
        }
        match &imp {
            R::Bug(m) => {
                st.bugs.fetch_add(1, Relaxed);
                st.ex.lock().unwrap().entry("BUG".into()).or_default().push(format!("{m}: {}", b.out));
                return;
            }
            _ => {}
        }
        if has_dup(&imp) || has_dup_loose(&imp) {
            st.dups.fetch_add(1, Relaxed);
            st.ex.lock().unwrap().entry("DUP".into()).or_default().push(b.out.clone());
        }
        if res.abstain {
            st.abstained.fetch_add(1, Relaxed);
            return;
        }
        st.compared.fetch_add(1, Relaxed);
        if imp == R::Ok {
            st.accepted_prog.fetch_add(1, Relaxed);
        }
        // implementation signature
        let mut isig: Vec<String> = Vec::new();
        let mut orig_ok = true;
        let mut other_diag = false;
        if let R::Rej(ds) = &imp {
            for d in ds {
                if d.code != "NX-E0102" && d.code != "NX-E1303" {
                    other_diag = true;
                    continue;
                }
                let (p, _) = ident(&b, d.s, d.e);
                isig.push(format!("{}/{}|{}|{}|{}", d.code, d.sub, p, d.exp, d.found));
            }
        }
        if other_diag {
            st.ex.lock().unwrap().entry("OTHERCODE".into()).or_default().push(b.out.clone());
        }
        let mut msig: Vec<String> = res.d.iter().map(|d| format!("{}/{}|{}|{}|{}", d.code, d.sub, d.prim, d.exp, d.found)).collect();
        isig.sort();
        msig.sort();
        st.model_diags.fetch_add(msig.len() as u64, Relaxed);
        st.impl_diags.fetch_add(isig.len() as u64, Relaxed);
        // origin check
        if let R::Rej(ds) = &imp {
            for d in ds {
                let (p, _) = ident(&b, d.s, d.e);
                if let Some(md) = res.d.iter().find(|m| m.prim == p && m.exp == d.exp && m.found == d.found && m.code == d.code) {
                    let exp_rel: Vec<_> = d.rel.iter().filter(|r| r.0 == "expected_type").collect();
                    let ok = match &md.org {
                        Org::None => exp_rel.is_empty(),
                        Org::Written(nm) => exp_rel.len() == 1 && ident(&b, exp_rel[0].1, exp_rel[0].2).0 == format!("p:{}", nm),
                        Org::Node(id) => {
                            exp_rel.len() == 1 && {
                                let got = ident(&b, exp_rel[0].1, exp_rel[0].2).0;
                                got == format!("n{}", id) || names.get(id).is_some_and(|nm| got == format!("p:{nm}0"))
                            }
                        }
                        Org::Param(k) => {
                            exp_rel.len() == 1 && {
                                let id = ident(&b, exp_rel[0].1, exp_rel[0].2).0;
                                id == format!("p:param{k}") || id == format!("p:paramview{k}")
                            }
                        }
                    };
                    let found_rel: Vec<_> = d.rel.iter().filter(|r| r.0 == "found_type").collect();
                    let fok = found_rel.len() == 1 && (p.starts_with("op") || (found_rel[0].1, found_rel[0].2) == (d.s, d.e));
                    if !ok || !fok {
                        orig_ok = false;
                        st.ex.lock().unwrap().entry("ORIGIN".into()).or_default().push(format!("{} :: model {:?} impl rel {:?}", b.out, md, d.rel));
                    }
                }
            }
        }
        if !orig_ok {
            st.origin_diff.fetch_add(1, Relaxed);
        }
        if isig == msig {
            st.agree.fetch_add(1, Relaxed);
        } else {
            let imp_acc = isig.is_empty() && !other_diag;
            let mod_acc = msig.is_empty();
            let cat = if imp_acc != mod_acc { st.accept_diff.fetch_add(1, Relaxed); "ACCEPT" } else if isig.len() != msig.len() { st.code_set_diff.fetch_add(1, Relaxed); "COUNT" } else { st.field_diff.fetch_add(1, Relaxed); "FIELD" };
            let mut ex = st.ex.lock().unwrap();
            let v = ex.entry(cat.into()).or_default();
            if v.len() < 60 {
                v.push(format!("{}\n    model={:?}\n    impl ={:?}\n    sites={:?}", b.out, msig, isig,
                    b.sites.iter().map(|site| (site.id, &b.out[site.core.0 as usize..site.core.1 as usize])).collect::<Vec<_>>()));
            }
        }
    });
    println!(
        "MODEL[{tag}] programs={} abstained={} compared={} agree={} accept_diff={} count_diff={} field_diff={} origin_diff={} accepted={} model_diags={} impl_diags={} dups={} bugs={} run_unit={} run_other={}",
        st.programs.load(Relaxed),
        st.abstained.load(Relaxed),
        st.compared.load(Relaxed),
        st.agree.load(Relaxed),
        st.accept_diff.load(Relaxed),
        st.code_set_diff.load(Relaxed),
        st.field_diff.load(Relaxed),
        st.origin_diff.load(Relaxed),
        st.accepted_prog.load(Relaxed),
        st.model_diags.load(Relaxed),
        st.impl_diags.load(Relaxed),
        st.dups.load(Relaxed),
        st.bugs.load(Relaxed),
        st.run_unit.load(Relaxed),
        st.run_other.load(Relaxed)
    );
    let ex = st.ex.lock().unwrap();
    for (k, v) in ex.iter() {
        println!("-- {k}: {} examples", v.len());
        for e in v.iter().take(8) {
            println!("{}", e.replace("fn FI(a:view I64){} fn FB(a:view Bool){} fn FA(a:view Array<I64>){} fn FAB(a:view Array<Bool>){} fn FAA(a:view Array<Array<I64>>){} ", ""));
        }
    }
}

pub fn check_prog(src: &str) -> R {
    crate::common::check(src)
}

fn collect_names(n: &N, m: &mut std::collections::HashMap<usize, &'static str>) {
    match &n.k {
        K::Name(s) => {
            m.insert(n.id, s);
        }
        K::Idx(b, i) if matches!(val(b).k, K::Name(_)) => {
            if let K::Name(s) = &val(b).k {
                m.insert(n.id, s);
            }
            collect_names(i, m);
        }
        K::Arr(v) => v.iter().for_each(|c| collect_names(c, m)),
        K::Bin(_, a, b) | K::Idx(a, b) => {
            collect_names(a, m);
            collect_names(b, m);
        }
        K::If(a, b, c) => {
            collect_names(a, m);
            collect_names(b, m);
            collect_names(c, m);
        }
        K::Blk(a) | K::Len(a) => collect_names(a, m),
        _ => {}
    }
}
