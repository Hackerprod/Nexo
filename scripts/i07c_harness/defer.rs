// (c) T3: deferred-element programs. Each program is compared with its live twin (same text, the deferred array name
// declared with its final literal). Independent leaves (every node that is not a use of the deferred name) must give the
// same diagnostics; no diagnostic may repeat.
use crate::common::*;
use crate::grp::{assemble_with, ident, Ctx, Spec};
use crate::model::*;
use crate::tree_gen::*;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

fn qnodes(n: &N, q: &str, out: &mut Vec<usize>) {
    match &n.k {
        K::Name(s) if *s == q => out.push(n.id),
        K::Arr(v) => v.iter().for_each(|c| qnodes(c, q, out)),
        K::Bin(_, a, b) | K::Idx(a, b) => {
            qnodes(a, q, out);
            qnodes(b, q, out);
        }
        K::If(a, b, c) => {
            qnodes(a, q, out);
            qnodes(b, q, out);
            qnodes(c, q, out);
        }
        K::Blk(a) | K::Len(a) => qnodes(a, q, out),
        _ => {}
    }
}

pub fn run(n: u64, seed: u64, threads: u64) {
    let programs = AtomicU64::new(0);
    let strict_dups = AtomicU64::new(0);
    let site_dups = AtomicU64::new(0);
    let missing = AtomicU64::new(0);
    let extra = AtomicU64::new(0);
    let same = AtomicU64::new(0);
    let accept_diff = AtomicU64::new(0);
    let q_lost = AtomicU64::new(0);
    let bugs = AtomicU64::new(0);
    let skipped = AtomicU64::new(0);
    let uses_q = AtomicU64::new(0);
    let ex: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());
    par(n, threads, |i| {
        let mut r = Rng::new(seed.wrapping_mul(104_729).wrapping_add(i));
        let ctxs = [Ctx::LetAnn, Ctx::Assign, Ctx::Call, Ctx::Return, Ctx::IdxAssign, Ctx::LetPlain, Ctx::AssertEq];
        let ctx = *r.pick(&ctxs);
        let tyi = r.below(5) as usize;
        let q: &'static str = *r.pick(&["ar", "br", "aar"]);
        let ty = match tyi {
            0 => Ty::I,
            1 => Ty::B,
            2 => Ty::A(Box::new(Ty::I)),
            3 => Ty::A(Box::new(Ty::B)),
            _ => Ty::A(Box::new(Ty::A(Box::new(Ty::I)))),
        };
        let depth = 1 + r.below(4) as u32;
        let want1 = match ctx {
            Ctx::IdxAssign => Ty::I,
            _ => ty.clone(),
        };
        let (e1, e2) = {
            let mut g = TG { r: &mut r, ids: Ids(0), extra: true, avoid: q };
            let e1 = g.mkt(&want1, depth);
            let w2 = g.mkt_ty();
            let e2 = g.mkt(&w2, depth);
            (e1, e2)
        };
        if ctx == Ctx::IdxAssign && q == "ar" {
            return;
        }
        let mut qs = Vec::new();
        qnodes(&e1, q, &mut qs);
        qnodes(&e2, q, &mut qs);
        let mk = |live: bool| Spec { ctx, ty: tyi, e1: e1.clone(), e2: e2.clone(), resolve: 0, use_d: false, defer: Some(q), live };
        let (bl, bd) = (assemble_with(&mk(true), false, true), assemble_with(&mk(false), false, true));
        programs.fetch_add(2, Relaxed);
        if !qs.is_empty() {
            uses_q.fetch_add(1, Relaxed);
        }
        let (rl, rd) = (crate::common::check(&bl.out), crate::common::check(&bd.out));
        for (b, x) in [(&bl, &rl), (&bd, &rd)] {
            if let R::Bug(m) = x {
                bugs.fetch_add(1, Relaxed);
                ex.lock().unwrap().entry("BUG".into()).or_default().push(format!("{m}: {}", b.out));
            }
        }
        if has_dup(&rd) {
            strict_dups.fetch_add(1, Relaxed);
            ex.lock().unwrap().entry("STRICTDUP".into()).or_default().push(bd.out.clone());
        } else if has_site_dup(&rd) {
            site_dups.fetch_add(1, Relaxed);
            ex.lock().unwrap().entry("SAMESITE".into()).or_default().push(bd.out.clone());
        }
        if has_dup(&rl) || has_site_dup(&rl) {
            ex.lock().unwrap().entry("LIVEDUP".into()).or_default().push(bl.out.clone());
        }
        let qset: Vec<String> = qs.iter().map(|id| format!("n{id}")).collect();
        let sig = |b: &B, x: &R| -> Vec<String> {
            let mut v = Vec::new();
            let q_spans: Vec<_> = b.sites.iter()
                .filter(|site| qset.contains(&format!("n{}", site.id)))
                .map(|site| site.core)
                .collect();
            if let R::Rej(ds) = x {
                for d in ds {
                    if d.code == "NX-E0103" {
                        continue;
                    }
                    let (p, _) = ident(b, d.s, d.e);
                    if (!p.starts_with('n') && !p.starts_with("op")) || qset.contains(&p) {
                        continue;
                    }
                    // A diagnostic on an ancestor of q is dependent on q's
                    // deferred constraint. A sibling leaf's own span does
                    // not contain q and remains in the independent oracle.
                    if q_spans.iter().any(|(a, z)| d.s <= *a && *z <= d.e) {
                        continue;
                    }
                    v.push(format!("{}/{}|{}|{}|{}", d.code, d.sub, p, d.exp, d.found));
                }
            }
            v.sort();
            v
        };
        let q_involved = match &rl {
            R::Rej(ds) => ds.iter().any(|d| {
                let hit = |s: u64, e: u64| {
                    let id = ident(&bl, s, e).0;
                    qset.contains(&id) || id == format!("p:{q}0")
                };
                hit(d.s, d.e) || d.rel.iter().any(|r| hit(r.1, r.2))
            }),
            _ => false,
        };
        let (sl, sd) = (sig(&bl, &rl), sig(&bd, &rd));
        let accl = rl == R::Ok;
        let accd = rd == R::Ok;
        if accl != accd {
            accept_diff.fetch_add(1, Relaxed);
            ex.lock().unwrap().entry("ACCEPT".into()).or_default().push(format!("{}\n   live={:?}", bd.out, rl));
        }
        if q_involved {
            skipped.fetch_add(1, Relaxed);
        } else if sl == sd {
            same.fetch_add(1, Relaxed);
        } else {
            let miss: Vec<_> = sl.iter().filter(|x| !sd.contains(x)).collect();
            let exs: Vec<_> = sd.iter().filter(|x| !sl.contains(x)).collect();
            if !miss.is_empty() {
                missing.fetch_add(1, Relaxed);
                ex.lock().unwrap().entry("MISSING".into()).or_default().push(format!("{}\n   missing(in deferred)={:?}\n   deferred={:?}", bd.out, miss, sd));
            }
            if !exs.is_empty() {
                extra.fetch_add(1, Relaxed);
                ex.lock().unwrap().entry("EXTRA".into()).or_default().push(format!("{}\n   extra(in deferred)={:?}\n   live={:?}", bd.out, exs, sl));
            }
        }
        if !qs.is_empty() {
            let live_q = matches!(&rl, R::Rej(ds) if ds.iter().any(|d| qset.contains(&ident(&bl, d.s, d.e).0)));
            if live_q && rd == R::Ok {
                q_lost.fetch_add(1, Relaxed);
                ex.lock().unwrap().entry("QLOST".into()).or_default().push(bd.out.clone());
            }
        }
    });
    println!(
        "DEFER programs={} q_involved_skipped={} uses_deferred_name={} identical_independent_sets={} missing_leaf_programs={} extra_diag_programs={} acceptance_diff={} q_lost={} strict_dup_programs={} same_site_programs={} bugs={}",
        programs.load(Relaxed),
        skipped.load(Relaxed),
        uses_q.load(Relaxed),
        same.load(Relaxed),
        missing.load(Relaxed),
        extra.load(Relaxed),
        accept_diff.load(Relaxed),
        q_lost.load(Relaxed),
        strict_dups.load(Relaxed),
        site_dups.load(Relaxed),
        bugs.load(Relaxed)
    );
    for (k, v) in ex.lock().unwrap().iter() {
        println!("-- {k}: {}", v.len());
        for e in v.iter().take(6) {
            println!("{}", e.replace("fn FI(a:view I64){} fn FB(a:view Bool){} fn FA(a:view Array<I64>){} fn FAB(a:view Array<Bool>){} fn FAA(a:view Array<Array<I64>>){} ", ""));
        }
    }
    assert_eq!(site_dups.load(Relaxed), 0, "one diagnosis per type site");
}
