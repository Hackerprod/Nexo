// (e) Free fuzz: random statement shells over random (mostly ill-typed) expressions with deferred names, Never
// initializers and group wrappers, plus byte-level mutations. Checks: no panic / InvalidIr / internal error,
// check and run coherent, no strict duplicate diagnostics.
use crate::common::*;
use crate::tree_gen::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

fn stmt(r: &mut Rng, b: &mut B, ids: &mut usize, names: &Vec<&'static str>, depth: u32, nest: u32) {
    let mut ex = |r: &mut Rng, b: &mut B, d: u32| {
        let mut g = Gen { r, ids: Ids(*ids), names: names.clone(), allow_extra: true };
        let e = g.expr(d);
        *ids = g.ids.0 + 1;
        b.expr(&e);
    };
    match r.below(14) {
        0 | 1 => {
            let n = *r.pick(&["a0", "a1", "a2"]);
            b.t(&format!("{n}="));
            ex(r, b, depth);
            b.t(";");
        }
        2 => {
            b.t("let z=");
            ex(r, b, depth);
            b.t(";");
        }
        3 => {
            let t = *r.pick(&["I64", "Bool", "Array<I64>", "Array<Bool>", "Array<Array<I64>>"]);
            b.t(&format!("let y:{t}="));
            ex(r, b, depth);
            b.t(";");
        }
        4 => {
            let n = *r.pick(&["ar", "br", "aar", "r"]);
            b.t(&format!("{n}["));
            ex(r, b, depth.saturating_sub(1));
            b.t("]=");
            ex(r, b, depth);
            b.t(";");
        }
        5 => {
            let n = *r.pick(&["ar", "br", "aar", "r"]);
            b.t(&format!("{n}[0]="));
            ex(r, b, depth);
            b.t(";");
        }
        6 | 7 if nest < 2 => {
            b.t("if ");
            ex(r, b, depth.saturating_sub(1));
            b.t(" {");
            for _ in 0..1 + r.below(2) {
                stmt(r, b, ids, names, depth, nest + 1);
            }
            b.t("}");
            if r.chance(1, 2) {
                b.t(" else {");
                stmt(r, b, ids, names, depth, nest + 1);
                b.t("}");
            }
        }
        8 if nest < 2 => {
            b.t("while 1<0 {");
            stmt(r, b, ids, names, depth, nest + 1);
            b.t("}");
        }
        9 => {
            b.t("AssertEq(");
            ex(r, b, depth);
            b.t(",");
            ex(r, b, depth);
            b.t(");");
        }
        10 => {
            b.t("Print(");
            ex(r, b, depth);
            b.t(");");
        }
        11 => {
            let n = *r.pick(&["r", "r", "ar", "br", "aar"]);
            b.t(&format!("{n}="));
            ex(r, b, depth);
            b.t(";");
        }
        12 => {
            b.t("var w=");
            ex(r, b, depth);
            b.t(";");
        }
        _ => {
            b.t("let k=");
            ex(r, b, depth);
            b.t(";");
        }
    }
}

pub fn gen_program(seed: u64) -> String {
    let mut r = Rng::new(seed);
    if r.chance(1, 12) {
        let value = 1 + r.below(10_000);
        let dv = "(if true then {return;}else{return;})";
        return match r.below(5) {
            0 => format!("fn Main(){{let ar=[{value}];let aar=[[{value}]];var r=[];let a=[r,ar,aar];r=[{value}];}}"),
            1 => format!("fn Main(){{var r=[];let a=[1,r]+[{value}];r=[{value}];}}"),
            2 => format!("fn Main(){{var q={dv};let y=(true+{value})<q;q=true;}}"),
            3 => format!("fn Main(){{var q={dv};let a=q[q<q];q=[{value}];}}"),
            _ => format!("fn Main(){{let ar=[{value}];var r=[];let a=ar==r[r[1]];r=[{value}];}}"),
        };
    }
    let mut b = B::new(r.chance(1, 2));
    b.t("fn FI(a:view I64){} fn FA(a:view Array<I64>){} ");
    if r.chance(1, 4) {
        b.t("fn G(a:view Bool)->Bool{return a;} ");
    }
    b.t("fn Main(){var x=1;var p=true;var ar=[1];var br=[true];var aar=[[1]];var r=[];");
    let use_d = r.chance(1, 4);
    if use_d {
        b.t("var d=(if true then {return;} else {return;});");
    }
    b.t("var a0=");
    b.expr(&{
        let mut g = Gen { r: &mut r, ids: Ids(0), names: vec!["x", "p", "ar"], allow_extra: false };
        g.expr(1)
    });
    b.t(";var a1=[];var a2=x;");
    let mut names = vec!["x", "p", "ar", "br", "aar", "r", "a0", "a1", "a2"];
    if use_d {
        names.push("d");
    }
    let n = 1 + r.below(6);
    let mut ids = 1000;
    for _ in 0..n {
        let depth = 1 + r.below(4) as u32;
        stmt(&mut r, &mut b, &mut ids, &names, depth, 0);
    }
    b.t("}");
    let mut s = b.out;
    if r.chance(1, 12) {
        // byte-level mutation
        let mut bytes: Vec<u8> = s.into_bytes();
        let i = r.below(bytes.len() as u64) as usize;
        match r.below(4) {
            0 => {
                bytes.remove(i);
            }
            1 => {
                bytes.insert(i, *r.pick(&[b'(', b')', b'[', b']', b'{', b'}', b';', b',', b'=', b'<', b'+']));
            }
            2 => {
                let j = r.below(bytes.len() as u64) as usize;
                bytes.swap(i, j);
            }
            _ => {
                let c = bytes[i];
                bytes.insert(i, c);
            }
        }
        s = String::from_utf8_lossy(&bytes).into_owned();
    }
    s
}

pub fn run(n: u64, seed: u64, threads: u64) {
    let progs = AtomicU64::new(0);
    let accepted = AtomicU64::new(0);
    let bugs = AtomicU64::new(0);
    let incoherent = AtomicU64::new(0);
    let dups = AtomicU64::new(0);
    let same_site = AtomicU64::new(0);
    let runs_ok = AtomicU64::new(0);
    let ex: Mutex<Vec<String>> = Mutex::new(Vec::new());
    par(n, threads, |i| {
        let src = gen_program(seed.wrapping_mul(2_654_435_761).wrapping_add(i));
        progs.fetch_add(1, Relaxed);
            if std::env::var("V10_TRACE").is_ok() { eprintln!("TRACE {src}"); }
        let c = check(&src);
        let (rr, outcome) = crate::common::run(&src);
        if let R::Bug(m) = &c {
            bugs.fetch_add(1, Relaxed);
            ex.lock().unwrap().push(format!("BUG(check) {m}: {src}"));
        }
        if let R::Bug(m) = &rr {
            bugs.fetch_add(1, Relaxed);
            ex.lock().unwrap().push(format!("BUG(run) {m}: {src}"));
        }
        if c != rr {
            incoherent.fetch_add(1, Relaxed);
            ex.lock().unwrap().push(format!("INCOHERENT: {src}"));
        }
        if has_dup(&c) {
            dups.fetch_add(1, Relaxed);
            ex.lock().unwrap().push(format!("STRICTDUP: {src}"));
        }
        if has_site_dup(&c) {
            same_site.fetch_add(1, Relaxed);
            ex.lock().unwrap().push(format!("SITE DUPLICATE: {src}"));
        }
        if c == R::Ok {
            accepted.fetch_add(1, Relaxed);
            if outcome == "Unit" {
                runs_ok.fetch_add(1, Relaxed);
            }
        }
    });
    println!(
        "FUZZ programs={} accepted={} run_unit={} bugs_panic_invalidir_internal={} check_run_incoherent={} strict_dup_programs={} same_site_programs={}",
        progs.load(Relaxed),
        accepted.load(Relaxed),
        runs_ok.load(Relaxed),
        bugs.load(Relaxed),
        incoherent.load(Relaxed),
        dups.load(Relaxed),
        same_site.load(Relaxed)
    );
    for e in ex.lock().unwrap().iter().take(10) {
        println!("{e}");
    }
    assert_eq!(same_site.load(Relaxed), 0, "one diagnosis per type site");
}
