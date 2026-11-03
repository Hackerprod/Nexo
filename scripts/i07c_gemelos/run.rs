// Evaluate generated programs against the independent model and concrete twins.
use crate::ast::*;
use crate::common::*;
use crate::model::*;
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

pub type Tup = (String, String, Site, String, String);

pub fn map_impl(pr: &Printed, v: &[D]) -> (Vec<Tup>, bool) {
    let mut out = vec![];
    let mut unm = false;
    for d in v {
        let k = (d.s, d.e);
        let site = if let Some(i) = pr.ops.get(&k) {
            Site::Op(*i)
        } else if let Some(i) = pr.spans.get(&k) {
            Site::Node(*i)
        } else {
            unm = true;
            Site::Node(u32::MAX)
        };
        out.push((d.code.clone(), d.sub.clone(), site, d.exp.clone(), d.found.clone()));
    }
    out.sort();
    (out, unm)
}
pub fn map_model(m: &Model) -> Vec<Tup> {
    let mut v: Vec<Tup> = m.diags.iter().map(|d| (d.code.to_string(), d.sub.to_string(), d.site.clone(), d.exp.clone(), d.found.clone())).collect();
    v.sort();
    v
}
pub fn sig(v: &[Tup]) -> String {
    v.iter().map(|(c, s, st, e, f)| format!("{}/{}@{:?}:{}>{}", c, s, st, e, f)).collect::<Vec<_>>().join(";")
}

#[derive(Default)]
pub struct Stats {
    pub total: AtomicU64,
    pub modeled: AtomicU64,
    pub uncertain: AtomicU64,
    pub exact: AtomicU64,
    pub exact_p2: AtomicU64,
    pub accepted: AtomicU64,
    pub rejected: AtomicU64,
    pub dup_strict: AtomicU64,
    pub dup_site: AtomicU64,
    pub extra_valid_child: AtomicU64,
    pub extra_same_site: AtomicU64,
    pub lost: AtomicU64,
    pub fielddiff: AtomicU64,
    pub origin_diff: AtomicU64,
    pub bug: AtomicU64,
    pub unmapped: AtomicU64,
    pub other_stage: AtomicU64,
    pub secondary_valid_unc: AtomicU64,
    pub twin_checked: AtomicU64,
    pub twin_diff: AtomicU64,
    pub twin_exempt: AtomicU64,
    pub twin_exempt_diff: AtomicU64,
    pub examples: Mutex<BTreeMap<String, Vec<String>>>,
    pub reasons: Mutex<BTreeMap<String, u64>>,
    pub exemption_reasons: Mutex<BTreeMap<String, (u64, u64)>>,
    pub annot_checked: AtomicU64,
    pub annot_bad: AtomicU64,
}
impl Stats {
    pub fn ex(&self, k: &str, s: String) {
        let mut g = self.examples.lock().unwrap();
        let v = g.entry(k.to_string()).or_default();
        if v.len() < 512 {
            v.push(s);
        }
    }
    pub fn report(&self, title: &str) {
        println!("== {title}");
        let g = |a: &AtomicU64| a.load(Relaxed);
        println!(
            "programs={} modeled={} uncertain={} exact_without_partial_witness={} exact_with_partial_witness={} accepted={} rejected={} twin_checked={} twin_diff={} twin_exempt={} twin_exempt_diff={}",
            g(&self.total), g(&self.modeled), g(&self.uncertain), g(&self.exact), g(&self.exact_p2), g(&self.accepted), g(&self.rejected), g(&self.twin_checked), g(&self.twin_diff), g(&self.twin_exempt), g(&self.twin_exempt_diff)
        );
        println!(
            "dup_strict={} dup_site={} extra_on_valid_child={} extra_same_site={} lost_leaf={} fielddiff={} origin_diff={} bug={} unmapped_span={} other_stage={} (uncertain-set: dup/ secondary counted separately)",
            g(&self.dup_strict), g(&self.dup_site), g(&self.extra_valid_child), g(&self.extra_same_site), g(&self.lost), g(&self.fielddiff), g(&self.origin_diff), g(&self.bug), g(&self.unmapped), g(&self.other_stage)
        );
        println!("annotation-origin checks={} violations={}", g(&self.annot_checked), g(&self.annot_bad));
        println!("uncertainty reasons (programs excluded from the exact comparison, adr_gap candidates): {:?}", self.reasons.lock().unwrap());
        for (reason, (count, differences)) in self.exemption_reasons.lock().unwrap().iter() {
            println!("EXEMPT\t{reason}\t{count}\t{differences}");
        }
        for (k, v) in self.examples.lock().unwrap().iter() {
            for s in v {
                println!("EX[{k}] {}", &s[..s.len().min(700)]);
            }
        }
    }
}

pub struct OutFile(pub Mutex<Option<std::io::BufWriter<std::fs::File>>>);

/// Evaluates one program. `twin` printing is used for the model only (spans taken from the deferred print).
pub fn eval(idx: u64, p: &Prog, st: &Stats, of: &OutFile) {
    st.total.fetch_add(1, Relaxed);
    let pr = print(p, true);
    let res = check(&pr.src);
    let m1 = run_model(p, false);
    let m2 = run_model(p, true);
    let unc = !m1.uncertain.is_empty();
    let mt1 = map_model(&m1);
    let mt2 = map_model(&m2);
    let (it, impl_sig, rejected) = match &res {
        R::Ok => (vec![], String::new(), false),
        R::Bug(m) => {
            st.bug.fetch_add(1, Relaxed);
            st.ex("bug", format!("{m}: {}", pr.src));
            return;
        }
        R::Rej(v) => {
            if v.iter().any(|d| d.stage != "types") {
                st.other_stage.fetch_add(1, Relaxed);
                st.ex("other_stage", format!("{:?}: {}", v.iter().map(|d| (d.code.clone(), d.sub.clone(), d.stage.clone())).collect::<Vec<_>>(), pr.src));
                return;
            }
            let (it, unm) = map_impl(&pr, v);
            if unm {
                st.unmapped.fetch_add(1, Relaxed);
                st.ex("unmapped", pr.src.clone());
            }
            let s = sig(&it);
            (it, s, true)
        }
    };
    if rejected {
        st.rejected.fetch_add(1, Relaxed);
    } else {
        st.accepted.fetch_add(1, Relaxed);
    }
    if has_dup(&res) {
        st.dup_strict.fetch_add(1, Relaxed);
        st.ex("dup_strict", pr.src.clone());
    }
    if has_site_dup(&res) {
        st.dup_site.fetch_add(1, Relaxed);
        st.ex("dup_site", pr.src.clone());
    }
    if mt2 == it {
        st.exact_p2.fetch_add(1, Relaxed);
    }
    // metamorphic twin: deferred variables resolved by a later assignment vs the same program with the concrete type
    let mut twin_sig: Option<String> = None;
    {
        let uses_def = p.stmts.iter().any(|s| s.nodes().iter().any(|nn| {
            let mut f = false;
            nn.walk(&mut |x| if matches!(x.k, K::Var(V::R) | K::Var(V::Q)) { f = true });
            f
        }));
        let later_conflict = match &res {
            R::Rej(diagnostics) => {
                let tails = [(uses(p, V::R), "r=", p.ft_r.as_ref()),
                             (uses(p, V::Q), "q=", p.ft_q.as_ref())]
                    .into_iter()
                    .filter_map(|(used, prefix, ty)| {
                        let ty = ty?;
                        let at = used.then(|| pr.src.rfind(prefix)).flatten()?;
                        let start = at + prefix.len();
                        Some((start as u64, (start + ty.lit_nonzero().len()) as u64))
                    })
                    .collect::<Vec<_>>();
                diagnostics.iter().any(|d| d.code == "NX-E0102"
                    && d.sub == "type_mismatch"
                    && tails.iter().any(|(start, end)| *start <= d.s && d.e <= *end))
            }
            _ => false,
        };
        // ADR §4.1.2.7, RD07/RD08: a rejection at the final source
        // assignment proves that its concrete type is incompatible with
        // an equality already accepted in the deferred prefix. The twin
        // starts with that type and has a different compatible prefix.
        let exemption = if later_conflict {
            Some("later assignment conflicts with accepted prefix equality")
        } else {
            m1.uncertain.iter().find(|r| matches!(**r, "deferred binding mismatch" | "r elem-level mismatch" | "array elements checked against a deferred element type")).copied()
        };
        let incons = exemption.is_some();
        if uses_def {
            let tw = crate::ast::print_modes(p, false, true);
            let tr = check(&tw.src);
            if let R::Bug(m) = &tr {
                st.bug.fetch_add(1, Relaxed);
                st.ex("twin_bug", format!("{m}: {}", tw.src));
                return;
            }
            if has_dup(&tr) || has_site_dup(&tr) {
                st.dup_site.fetch_add(1, Relaxed);
                st.ex("twin_duplicate", tw.src.clone());
            }
            let twt: Vec<Tup> = match &tr {
                R::Rej(v) => map_impl(&tw, v).0,
                _ => vec![],
            };
            twin_sig = Some(sig(&twt));
            if incons {
                // §4.1.2.7: a use can constrain an unresolved binding before
                // its later assignment. The concrete twin starts with that
                // type and can reject a different source operand.
                st.twin_exempt.fetch_add(1, Relaxed);
                let mut reasons = st.exemption_reasons.lock().unwrap();
                let entry = reasons.entry(exemption.unwrap().to_string()).or_default();
                entry.0 += 1;
                if twt != it {
                    st.twin_exempt_diff.fetch_add(1, Relaxed);
                    entry.1 += 1;
                    st.ex(&format!("exempt:{}", exemption.unwrap()),
                        format!("{} :: deferred={} twin={}", pr.src, sig(&it), sig(&twt)));
                }
            } else {
                st.twin_checked.fetch_add(1, Relaxed);
                if twt != it {
                    st.twin_diff.fetch_add(1, Relaxed);
                    st.ex("twin_diff", format!("{} :: deferred={} twin={}", pr.src, sig(&it), sig(&twt)));
                }
            }
        }
    }
    if let Some(w) = of.0.lock().unwrap().as_mut() {
        let _ = writeln!(w, "{idx}	{}	{}	{}	{}	{}", if unc { "U" } else { "M" }, sig(&mt1), impl_sig, m1.uncertain.join("|"), twin_sig.clone().unwrap_or_else(|| "-".into()));
    }
    if unc {
        st.uncertain.fetch_add(1, Relaxed);
        let mut g = st.reasons.lock().unwrap();
        for r in &m1.uncertain {
            *g.entry(r.to_string()).or_default() += 1;
        }
        return;
    }
    // annotation origin: E0102 whose expected type flows unchanged from a `let x:T=` annotation must relate that annotation
    if let R::Rej(v) = &res {
        let mut ann_i = 0usize;
        for s in &p.stmts {
            if let S::Let(Some(t), e) = s {
                let span = pr.ann.get(ann_i).cloned();
                ann_i += 1;
                fn direct(x: &N, out: &mut Vec<u32>) {
                    match &x.k {
                        K::Grp(i) | K::Blk(i) => direct(i, out),
                        K::If(_, a, b) => {
                            direct(a, out);
                            direct(b, out);
                        }
                        _ => out.push(x.id),
                    }
                }
                let mut ids = vec![];
                direct(e, &mut ids);
                for d in v.iter().filter(|d| d.code == "NX-E0102" && d.sub == "type_mismatch" && d.exp == t.s()) {
                    if let Some(i) = pr.spans.get(&(d.s, d.e)) {
                        if ids.contains(i) {
                            st.annot_checked.fetch_add(1, Relaxed);
                            let ok = d.rel.iter().any(|r| r.0 == "expected_type" && Some((r.1, r.2)) == span);
                            if !ok {
                                st.annot_bad.fetch_add(1, Relaxed);
                                st.ex("annotation_origin", format!("{} :: {:?}", pr.src, d.rel));
                            }
                        }
                    }
                }
            }
        }
    }
    st.modeled.fetch_add(1, Relaxed);
    if mt1 == it {
        st.exact.fetch_add(1, Relaxed);
        // origin check for E1303 and found_type relation for all
        if let R::Rej(v) = &res {
            for d in v {
                if d.code == "NX-E1303" {
                    let exp_rel = d.rel.iter().find(|r| r.0 == "expected_type");
                    let md = m1.diags.iter().find(|x| matches!(&x.site, Site::Node(i) if pr.by_id.get(i) == Some(&(d.s, d.e))) && x.code == "NX-E1303");
                    if let (Some(er), Some(md)) = (exp_rel, md) {
                        let want = md.org.and_then(|o| pr.by_id.get(&o)).cloned();
                        if want != Some((er.1, er.2)) {
                            st.origin_diff.fetch_add(1, Relaxed);
                            st.ex("origin_diff", format!("want {:?} got [{},{}) in {}", want, er.1, er.2, pr.src));
                        }
                    }
                }
            }
        }
        return;
    }
    // mismatch classification
    let msites: Vec<&Site> = mt1.iter().map(|t| &t.2).collect();
    let mut left = mt1.clone();
    let mut extras = vec![];
    for t in &it {
        if let Some(i) = left.iter().position(|x| x == t) {
            left.remove(i);
        } else {
            extras.push(t.clone());
        }
    }
    let mut cat = vec![];
    for e in &extras {
        // same site in `left`?
        if let Some(i) = left.iter().position(|x| x.2 == e.2) {
            st.fielddiff.fetch_add(1, Relaxed);
            cat.push(format!("fielddiff impl={:?} model={:?}", e, left[i]));
            left.remove(i);
        } else if msites.contains(&&e.2) {
            st.extra_same_site.fetch_add(1, Relaxed);
            cat.push(format!("extra_same_site {:?}", e));
        } else {
            st.extra_valid_child.fetch_add(1, Relaxed);
            cat.push(format!("extra_on_valid_child {:?}", e));
        }
    }
    for l in &left {
        st.lost.fetch_add(1, Relaxed);
        cat.push(format!("lost {:?}", l));
    }
    let key = cat.first().map(|c| c.split(' ').next().unwrap().to_string()).unwrap_or("none".into());
    st.ex(&format!("mismatch:{key}"), format!("{} :: {} :: model={} impl={}", pr.src, cat.join(" | "), sig(&mt1), impl_sig));
}

pub fn open_out(path: Option<&String>) -> OutFile {
    OutFile(Mutex::new(path.map(|p| std::io::BufWriter::new(std::fs::File::create(p).unwrap()))))
}
pub fn flush(of: &OutFile) {
    if let Some(w) = of.0.lock().unwrap().as_mut() {
        let _ = w.flush();
    }
}

pub fn profile(name: &str) -> Profile {
    match name {
        "plain" => Profile { defer: false, w_q: 0, w_r: 0, depth: 4, all_ctx: true, exp_only: false },
        "expd" => Profile { defer: false, w_q: 0, w_r: 0, depth: 4, all_ctx: true, exp_only: true },
        "expd_defer" => Profile { defer: true, w_q: 5, w_r: 8, depth: 4, all_ctx: true, exp_only: true },
        "defer" => Profile { defer: true, w_q: 6, w_r: 8, depth: 4, all_ctx: true, exp_only: false },
        "heavy" => Profile { defer: true, w_q: 10, w_r: 14, depth: 5, all_ctx: true, exp_only: false },
        _ => panic!("profile"),
    }
}

pub fn run_model_mode(prof: &str, n: u64, seed: u64, threads: u64, out: Option<&String>) {
    let st = Stats::default();
    let of = open_out(out);
    let pf = profile(prof);
    par(n, threads, |i| {
        let mut r = Rng::new(seed.wrapping_mul(0x9E37_79B9).wrapping_add(i).wrapping_mul(0xD6E8_FEB8_6659_FD93));
        let mut g = G { r: &mut r, id: 0, pf, allow_rq: true };
        let p = g.prog();
        eval(i, &p, &st, &of);
    });
    flush(&of);
    st.report(&format!("model profile={prof} n={n} seed={seed}"));
}

pub fn run_pairs_mode(n: u64, stride: u64, offset: u64, threads: u64, out: Option<&String>) {
    let st = Stats::default();
    let of = open_out(out);
    let total = crate::pairs::pair_total();
    par(n, threads, |i| {
        let k = (offset + i).wrapping_mul(stride) % total;
        let p = crate::pairs::pair_prog(k);
        eval(k, &p, &st, &of);
    });
    flush(&of);
    st.report(&format!("pairs n={n} stride={stride} offset={offset} total_space={total}"));
}

pub fn gen_src(prof: &str, seed: u64, i: u64) -> String {
    let mut r = Rng::new(seed.wrapping_mul(0x9E37_79B9).wrapping_add(i).wrapping_mul(0xD6E8_FEB8_6659_FD93));
    let mut g = G { r: &mut r, id: 0, pf: profile(prof), allow_rq: true };
    let p = g.prog();
    print(&p, true).src
}
pub fn pair_src(k: u64) -> String {
    print(&crate::pairs::pair_prog(k), true).src
}
