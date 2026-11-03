// (b) T1: primary of a group is always the operand without parentheses. Pair generator: same tree rendered
// plain (mandatory parens only) and with random extra group wrappers; diagnostics are compared by node identity.
use crate::common::*;
use crate::tree_gen::*;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ctx {
    LetAnn,
    LetPlain,
    Assign,
    IdxAssign,
    IdxAssignIdx,
    Call,
    If,
    While,
    AssertEq,
    Print,
    LenStmt,
    Inspect,
    Return,
}
pub const CTXS: [Ctx; 13] = [
    Ctx::LetAnn,
    Ctx::LetPlain,
    Ctx::Assign,
    Ctx::IdxAssign,
    Ctx::IdxAssignIdx,
    Ctx::Call,
    Ctx::If,
    Ctx::While,
    Ctx::AssertEq,
    Ctx::Print,
    Ctx::LenStmt,
    Ctx::Inspect,
    Ctx::Return,
];
pub const TYS: [&str; 5] = ["I64", "Bool", "Array<I64>", "Array<Bool>", "Array<Array<I64>>"];
const INITS: [&str; 5] = ["0", "false", "[0]", "[false]", "[[0]]"];
const FNS: [&str; 5] = ["FI", "FB", "FA", "FAB", "FAA"];
pub const RESOLVE: [&str; 5] = ["", "r=[1];", "r=[true];", "r=[[1]];", "r=1;"];

pub struct Spec {
    pub ctx: Ctx,
    pub ty: usize,
    pub e1: N,
    pub e2: N,
    pub resolve: usize,
    pub use_d: bool,
    pub defer: Option<&'static str>,
    pub live: bool,
}

pub fn gen_spec(r: &mut Rng, depth: u32, allow_extra: bool) -> Spec {
    let ctx = *r.pick(&CTXS);
    let ty = r.below(5) as usize;
    let use_d = r.chance(1, 6);
    let mut names = vec!["x", "p", "ar", "br", "r", "r"];
    if use_d {
        names.push("d");
    }
    let resolve = r.below(5) as usize;
    let mut g = Gen { r, ids: Ids(0), names, allow_extra };
    let e1 = g.expr(depth);
    let e2 = g.expr(depth);
    Spec { ctx, ty, e1, e2, resolve, use_d, defer: None, live: true }
}

pub fn assemble(s: &Spec, grouped: bool) -> B {
    assemble_with(s, grouped, false)
}
pub fn assemble_with(s: &Spec, grouped: bool, concrete_only: bool) -> B {
    let mut b = B::new(grouped);
    for (i, f) in FNS.iter().enumerate() {
        b.t(&format!("fn {f}(a:"));
        let a = b.out.len() as u64;
        b.t("view ");
        b.pseudo_text(&format!("param{i}"), TYS[i]);
        let e = b.out.len() as u64;
        b.pseudo.push((format!("paramview{i}"), (a, e)));
        b.t("){} ");
    }
    let decls = |b: &mut B| {
        b.t("let x=");
        b.pseudo_text("x0", "1");
        b.t(";let p=");
        b.pseudo_text("p0", "true");
        b.t(";");
        if concrete_only {
            for (nm, init) in [("ar", "[1]"), ("br", "[true]"), ("aar", "[[1]]")] {
                if nm != "ar" {
                    b.t(&format!("var {nm}="));
                } else {
                    b.t("var ar=");
                }
                if s.defer == Some(nm) && !s.live {
                    b.pseudo_text(&format!("{nm}0"), "[]");
                } else {
                    b.pseudo_text(&format!("{nm}0"), init);
                }
                b.t(";");
            }
            return;
        }
        b.t("var ar=");
        b.pseudo_text("ar0", "[1]");
        b.t(";let br=");
        b.pseudo_text("br0", "[true]");
        b.t(";var r=");
        b.pseudo_text("r0", "[]");
        b.t(";");
        if s.use_d {
            b.t("var d=");
            b.pseudo_text("d0", "(if true then {return;} else {return;})");
            b.t(";");
        }
    };
    let ty = TYS[s.ty];
    match s.ctx {
        Ctx::Return => {
            b.t("fn H()->");
            b.pseudo_text("ret", ty);
            b.t("{");
            decls(&mut b);
            b.pseudo_text("retkw", "return");
            b.t(" ");
            b.expr(&s.e1);
            b.t(";");
            defer_res(&mut b, s);
            resolve(&mut b, s.resolve);
            b.t("} fn Main(){}");
            return b;
        }
        _ => {}
    }
    b.t("fn Main(){");
    decls(&mut b);
    match s.ctx {
        Ctx::LetAnn => {
            b.t("let a:");
            b.pseudo_text("ann", ty);
            b.t("=");
            b.expr(&s.e1);
            b.t(";");
        }
        Ctx::LetPlain => {
            b.t("let a=");
            b.expr(&s.e1);
            b.t(";");
        }
        Ctx::Assign => {
            b.t("var v:");
            b.pseudo_text("ann", ty);
            b.t("=");
            b.pseudo_text("vinit", INITS[s.ty]);
            b.t(";v=");
            b.expr(&s.e1);
            b.t(";");
        }
        Ctx::IdxAssign => {
            b.t("ar[0]=");
            b.expr(&s.e1);
            b.t(";");
        }
        Ctx::IdxAssignIdx => {
            b.t("ar[");
            b.expr(&s.e1);
            b.t("]=1;");
        }
        Ctx::Call => {
            b.t(FNS[s.ty]);
            b.t("(");
            b.expr(&s.e1);
            b.t(");");
        }
        Ctx::If => {
            b.t("if ");
            b.expr(&s.e1);
            b.t(" {}");
        }
        Ctx::While => {
            b.t("while ");
            b.expr(&s.e1);
            b.t(" {}");
        }
        Ctx::AssertEq => {
            b.t("AssertEq(");
            b.expr(&s.e1);
            b.t(",");
            b.expr(&s.e2);
            b.t(");");
        }
        Ctx::Print => {
            b.t("Print(");
            b.expr(&s.e1);
            b.t(");");
        }
        Ctx::LenStmt => {
            b.t("Len(");
            b.expr(&s.e1);
            b.t(");");
        }
        Ctx::Inspect => {
            b.t("Inspect(");
            b.expr(&s.e1);
            b.t(");");
        }
        Ctx::Return => unreachable!(),
    }
    defer_res(&mut b, s);
    resolve(&mut b, s.resolve);
    b.t("}");
    b
}
fn defer_res(b: &mut B, s: &Spec) {
    if let (Some(q), false) = (s.defer, s.live) {
        let init = match q {
            "ar" => "[1]",
            "br" => "[true]",
            _ => "[[1]]",
        };
        b.t(q);
        b.t("=");
        b.pseudo_text("res", init);
        b.t(";");
    }
}
fn resolve(b: &mut B, k: usize) {
    let t = RESOLVE[k];
    if let Some(rhs) = t.strip_prefix("r=") {
        b.t("r=");
        b.pseudo_text("res", rhs.trim_end_matches(';'));
        b.t(";");
    }
}

/// Maps a span to a stable identity; flags group/raw spans.
pub fn ident(b: &B, s: u64, e: u64) -> (String, u8) {
    for st in &b.sites {
        if st.core == (s, e) {
            return (format!("n{}", st.id), 0);
        }
    }
    for (n, sp) in &b.pseudo {
        if *sp == (s, e) {
            return (format!("p:{n}"), 0);
        }
    }
    for st in &b.sites {
        if st.op == Some((s, e)) {
            return (format!("op{}", st.id), 0);
        }
    }
    for st in &b.sites {
        if st.wraps.contains(&(s, e)) {
            return (format!("GROUP{}", st.id), 1);
        }
    }
    let t = &b.out[s as usize..e as usize];
    let paren = t.starts_with('(') && t.ends_with(')');
    (format!("RAW[{}]", t), if paren { 2 } else { 0 })
}

pub fn signature(b: &B, r: &R) -> (Vec<String>, u64, u64) {
    let mut v = Vec::new();
    let (mut groups, mut raws) = (0, 0);
    if let R::Rej(ds) = r {
        for d in ds {
            let (p, f) = ident(b, d.s, d.e);
            groups += (f == 1) as u64;
            raws += (f == 2) as u64;
            let mut rel: Vec<String> = d
                .rel
                .iter()
                .map(|(role, s, e)| {
                    let (i, f) = ident(b, *s, *e);
                    groups += (f == 1) as u64;
                    raws += (f == 2) as u64;
                    format!("{role}={i}")
                })
                .collect();
            rel.sort();
            v.push(format!("{}/{}/{}|{}|{}->{}|{}|{}", d.code, d.sub, d.stage, p, d.exp, d.found, rel.join(","), d.data));
        }
    }
    v.sort();
    (v, groups, raws)
}

#[derive(Default)]
pub struct Stats {
    pub pairs: AtomicU64,
    pub parse_skipped: AtomicU64,
    pub rejected: AtomicU64,
    pub accepted: AtomicU64,
    pub diag_total: AtomicU64,
    pub diffs: AtomicU64,
    pub card_diffs: AtomicU64,
    pub group_spans: AtomicU64,
    pub raw_spans: AtomicU64,
    pub dups: AtomicU64,
    pub sitedups: AtomicU64,
    pub bugs: AtomicU64,
    pub ex: Mutex<Vec<String>>,
    pub codes: Mutex<HashMap<String, u64>>,
}

pub fn run(n: u64, seed: u64, threads: u64) {
    let st = Stats::default();
    par(n, threads, |i| {
        let mut r = Rng::new(seed.wrapping_mul(1_000_003).wrapping_add(i));
        let depth = 1 + r.below(4) as u32;
        let spec = gen_spec(&mut r, depth, true);
        let p = assemble(&spec, false);
        let g = assemble(&spec, true);
        let (rp, rg) = (check(&p.out), check(&g.out));
        st.pairs.fetch_add(1, Relaxed);
        for x in [&rp, &rg] {
            if let R::Bug(m) = x {
                st.bugs.fetch_add(1, Relaxed);
                st.ex.lock().unwrap().push(format!("BUG {m}: {}", p.out));
            }
        }
        for (b, x) in [(&p, &rp), (&g, &rg)] {
            if has_dup(x) {
                st.dups.fetch_add(1, Relaxed);
                st.ex.lock().unwrap().push(format!("DUP: {}", b.out));
            } else if has_site_dup(x) {
                st.sitedups.fetch_add(1, Relaxed);
                st.ex.lock().unwrap().push(format!("SITEDUP: {}", b.out));
            }
        }
        // parse-level failures differ legitimately (a wrapper may not be admissible somewhere)
        let parse_fail = |x: &R| matches!(x, R::Rej(v) if v.iter().any(|d| d.stage == "parse" || d.stage == "lex" || d.stage == "names"));
        if parse_fail(&rp) || parse_fail(&rg) {
            if parse_fail(&rp) != parse_fail(&rg) {
                st.ex.lock().unwrap().push(format!("PARSEDIFF:\n {}\n {}", p.out, g.out));
            }
            st.parse_skipped.fetch_add(1, Relaxed);
            return;
        }
        match &rp {
            R::Ok => {
                st.accepted.fetch_add(1, Relaxed);
            }
            R::Rej(v) => {
                st.rejected.fetch_add(1, Relaxed);
                st.diag_total.fetch_add(v.len() as u64, Relaxed);
                let mut c = st.codes.lock().unwrap();
                for d in v {
                    *c.entry(format!("{}/{}", d.code, d.sub)).or_insert(0) += 1;
                }
            }
            _ => {}
        }
        let (sp, gp, rwp) = signature(&p, &rp);
        let (sg, gg, rwg) = signature(&g, &rg);
        st.group_spans.fetch_add(gp + gg, Relaxed);
        st.raw_spans.fetch_add(rwp + rwg, Relaxed);
        let acc_diff = (rp == R::Ok) != (rg == R::Ok);
        if sp.len() != sg.len() || acc_diff {
            st.card_diffs.fetch_add(1, Relaxed);
        }
        if sp != sg || acc_diff || gp + gg + rwp + rwg > 0 {
            st.diffs.fetch_add((sp != sg || acc_diff) as u64, Relaxed);
            let mut ex = st.ex.lock().unwrap();
            if ex.len() < 40 {
                ex.push(format!("DIFF plain={}\n      grouped={}\n  P:{:?}\n  G:{:?}\n  groups={} raws={}", p.out, g.out, sp, sg, gp + gg, rwp + rwg));
            }
        }
    });
    println!(
        "GRP pairs={} (programs={}) parse_or_names_skipped={} accepted={} rejected={} diags={} sig_diffs={} cardinality_diffs={} group_spans={} raw_spans={} strict_dup_programs={} same_site_programs={} bugs={}",
        st.pairs.load(Relaxed),
        st.pairs.load(Relaxed) * 2,
        st.parse_skipped.load(Relaxed),
        st.accepted.load(Relaxed),
        st.rejected.load(Relaxed),
        st.diag_total.load(Relaxed),
        st.diffs.load(Relaxed),
        st.card_diffs.load(Relaxed),
        st.group_spans.load(Relaxed),
        st.raw_spans.load(Relaxed),
        st.dups.load(Relaxed),
        st.sitedups.load(Relaxed),
        st.bugs.load(Relaxed)
    );
    let mut cs: Vec<_> = st.codes.lock().unwrap().iter().map(|(k, v)| (k.clone(), *v)).collect();
    cs.sort();
    println!("GRP codes {:?}", cs);
    let ex = st.ex.lock().unwrap();
    for pre in ["BUG", "DUP", "SITEDUP", "PARSEDIFF", "DIFF"] {
        for e in ex.iter().filter(|e| e.starts_with(pre)).take(6) {
            println!("{e}");
        }
    }
    assert_eq!(st.sitedups.load(Relaxed), 0, "one diagnosis per type site");
}
