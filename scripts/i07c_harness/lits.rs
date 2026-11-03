// (a) TD01-TD07, DG07/DG08 and the literal cases of 14.10; expectations transcribed from the ADR text.
use crate::common::*;

pub const DV: &str = "(if true then {return;} else {return;})";

pub fn find_nth(src: &str, needle: &str, n: usize) -> (u64, u64) {
    let mut from = 0;
    let mut i = 0;
    loop {
        let p = src[from..].find(needle).unwrap_or_else(|| panic!("{needle} not in {src}")) + from;
        if i == n {
            return (p as u64, (p + needle.len()) as u64);
        }
        i += 1;
        from = p + 1;
    }
}

struct Exp {
    code: &'static str,
    sub: &'static str,
    stage: &'static str,
    prim: (u64, u64),
    exp: &'static str,
    found: &'static str,
    rel: Vec<(&'static str, (u64, u64))>,
}

fn verify(name: &str, src: &str, exps: &[Exp], fails: &mut Vec<String>) {
    let c = check(src);
    let (r, outcome) = run(src);
    if c != r {
        fails.push(format!("{name}: check != run: {c:?} / {r:?}"));
    }
    if exps.is_empty() {
        if c != R::Ok {
            fails.push(format!("{name}: expected accepted, got {c:?}"));
            return;
        }
        if outcome != "Unit" {
            fails.push(format!("{name}: run outcome {outcome}"));
        }
        return;
    }
    let R::Rej(v) = &c else {
        fails.push(format!("{name}: expected rejection, got {c:?}"));
        return;
    };
    if v.len() != exps.len() {
        fails.push(format!("{name}: {} diagnostics, expected {}: {v:?}", v.len(), exps.len()));
        return;
    }
    for (d, e) in v.iter().zip(exps) {
        let mut want_rel: Vec<(String, u64, u64)> = e.rel.iter().map(|(r, s)| (r.to_string(), s.0, s.1)).collect();
        want_rel.sort_by_key(|x| (x.1, x.2));
        let ok = d.code == e.code
            && d.sub == e.sub
            && d.stage == e.stage
            && (d.s, d.e) == e.prim
            && d.exp == e.exp
            && d.found == e.found
            && d.rel == want_rel
            && (e.code != "NX-E0102" || d.op == "types.check")
            && d.extra.starts_with("chain=0 repair=0 feat=None");
        if !ok {
            fails.push(format!(
                "{name}: got {d:?}\n   want code={} sub={} stage={} prim={:?} exp={} found={} rel={:?}",
                e.code, e.sub, e.stage, e.prim, e.exp, e.found, want_rel
            ));
        }
        if e.code == "NX-E0102" {
            // data holds exactly the two keys
            let keys = d.data.matches("\": ").count() + d.data.matches("\":").count();
            let _ = keys;
            if !(d.data.contains("expected_type") && d.data.contains("found_type")) || d.data.matches("_type").count() != 2 {
                fails.push(format!("{name}: data {}", d.data));
            }
        }
    }
    if outcome != "rejected" {
        fails.push(format!("{name}: run outcome {outcome} for rejected"));
    }
}

fn mismatch(prim: (u64, u64), exp: &'static str, found: &'static str, exp_rel: Option<(u64, u64)>) -> Exp {
    let mut rel = vec![("found_type", prim)];
    if let Some(e) = exp_rel {
        rel.push(("expected_type", e));
    }
    Exp { code: "NX-E0102", sub: "type_mismatch", stage: "types", prim, exp, found, rel }
}

pub fn run_all(entry: &str) -> Vec<String> {
    let mut f = Vec::new();
    let dv = DV;
    let m = |body: &str| format!("fn {entry}(){{{body}}}");
    // TD01
    let s = m(&format!("var v={dv};var w=v;v=3;w=true;"));
    let tru = find_nth(&s, "true", 1);
    let three = find_nth(&s, "3", 0);
    verify("TD01", &s, &[mismatch(tru, "I64", "Bool", Some(three))], &mut f);
    verify("TD01+", &m(&format!("var v={dv};var w=v;v=3;w=4;")), &[], &mut f);
    // TD02
    let s = m("var v=3;var w=v;v=3;w=true;");
    let tru = find_nth(&s, "true", 0);
    let three = find_nth(&s, "3", 0);
    verify("TD02", &s, &[mismatch(tru, "I64", "Bool", Some(three))], &mut f);
    verify("TD02+", &m("var v=3;var w=v;v=3;w=4;"), &[], &mut f);
    // TD03
    let s = m(&format!("var v={dv};var w=true;w=v;v=3;"));
    let tr = find_nth(&s, "true", 1);
    let three = find_nth(&s, "3", 0);
    verify("TD03", &s, &[mismatch(three, "Bool", "I64", Some(tr))], &mut f);
    verify("TD03+", &m(&format!("var v={dv};var w=true;w=v;v=false;")), &[], &mut f);
    // TD04
    verify("TD04a", &m(&format!("let x=({dv})==true;")), &[], &mut f);
    verify("TD04b", &m(&format!("let x=true=={dv};")), &[], &mut f);
    // TD05
    verify("TD05a", &m(&format!("let x=[{dv},true];")), &[], &mut f);
    verify("TD05b", &m(&format!("let x=[true,{dv}];")), &[], &mut f);
    // TD06
    verify("TD06", &m(&format!("var v={dv};var w=v;v=[1];w=[2];w[0]=9;")), &[], &mut f);
    verify("TD06live", &m("var v=[0];var w=v;v=[1];w=[2];w[0]=9;AssertEq(v,[1]);AssertEq(w,[9]);"), &[], &mut f);
    // TD07
    let s = m("let x=missing;let y=x;let z=true+1;");
    let p = find_nth(&s, "missing", 0);
    let c = check(&s);
    match &c {
        R::Rej(v) if v.len() == 1 && v[0].code == "NX-E0101" && (v[0].s, v[0].e) == p && v[0].stage == "names" && v[0].data.contains("value") => {}
        o => f.push(format!("TD07a: {o:?}")),
    }
    let s = m("let x=0;let y=x;let z=true+1;");
    let tru = find_nth(&s, "true", 0);
    verify("TD07b", &s, &[mismatch(tru, "I64", "Bool", None)], &mut f);
    // isolated negatives
    for (name, body, ti) in [("DV+true", format!("let x={dv}+true;"), 1usize), ("true+DV", format!("let x=true+{dv};"), 0usize)] {
        let s = m(&body);
        let tru = find_nth(&s, "true", ti);
        verify(name, &s, &[mismatch(tru, "I64", "Bool", None)], &mut f);
        let pos = if ti == 1 { body.replace("+true", "+1") } else { body.replace("true+", "1+") };
        verify(&format!("{name}+"), &m(&pos), &[], &mut f);
    }
    for (name, body) in [("[DV,1,true]", format!("let x=[{dv},1,true];")), ("[1,DV,true]", format!("let x=[1,{dv},true];"))] {
        let s = m(&body);
        // the DV text itself contains `true` (condition); the final `true` is the last occurrence
        let last = s.rfind("true").unwrap() as u64;
        let tru = (last, last + 4);
        let one = find_nth(&s, "1", 0);
        verify(name, &s, &[mismatch(tru, "I64", "Bool", Some(one))], &mut f);
        let pos = format!("{}2];", &body[..body.len() - 6]);
        verify(&format!("{name}+"), &m(&pos), &[], &mut f);
    }
    // contamination
    let s = m("let x=true+1;let y=x;let z=false+1;");
    let t = find_nth(&s, "true", 0);
    let fl = find_nth(&s, "false", 0);
    verify("contam", &s, &[mismatch(t, "I64", "Bool", None), mismatch(fl, "I64", "Bool", None)], &mut f);
    verify("contam+", &m("let x=0+1;let y=x;let z=0+1;"), &[], &mut f);
    let s1 = m("let x=0+1;let y=x;let z=false+1;");
    let s2 = m("let x=true+1;let y=x;let z=0+1;");
    verify("contam-only-false", &s1, &[mismatch(find_nth(&s1, "false", 0), "I64", "Bool", None)], &mut f);
    verify("contam-only-true", &s2, &[mismatch(find_nth(&s2, "true", 0), "I64", "Bool", None)], &mut f);
    // DG07 / DG08
    let s = m("var a:Array<I64>=[0];a[0]=true;");
    let ann = find_nth(&s, "Array<I64>", 0);
    let tru = find_nth(&s, "true", 0);
    verify("DG07", &s, &[mismatch(tru, "I64", "Bool", Some(ann))], &mut f);
    verify("DG07+", &m("var a:Array<I64>=[0];a[0]=1;"), &[], &mut f);
    let s = m("var a=[0];a[0]=true;");
    let init = find_nth(&s, "[0]", 0);
    let tru = find_nth(&s, "true", 0);
    verify("DG08", &s, &[mismatch(tru, "I64", "Bool", Some(init))], &mut f);
    verify("DG08+", &m("var a=[0];a[0]=1;"), &[], &mut f);
    if entry == "Main" {
        // The ADR 14.10 offsets for the grammar-free spelling of DG07/DG08 (no `then` involved).
        if let R::Rej(v) = check("fn main(){var a:Array<I64>=[0];a[0]=true;}") {
            if !((v[0].s, v[0].e) == (36, 40) && v[0].rel == vec![("expected_type".to_string(), 16, 26), ("found_type".to_string(), 36, 40)]) {
                f.push(format!("DG07 adr offsets {:?}", v[0]));
            }
        }
        if let R::Rej(v) = check("fn main(){var a=[0];a[0]=true;}") {
            if !((v[0].s, v[0].e) == (25, 29) && v[0].rel == vec![("expected_type".to_string(), 16, 19), ("found_type".to_string(), 25, 29)]) {
                f.push(format!("DG08 adr offsets {:?}", v[0]));
            }
        }
        // TD02 literal offsets [32,36) / [16,17)
        if let R::Rej(v) = check("fn main(){var v=3;var w=v;v=3;w=true;}") {
            if !((v[0].s, v[0].e) == (32, 36) && v[0].rel == vec![("expected_type".to_string(), 16, 17), ("found_type".to_string(), 32, 36)]) {
                f.push(format!("TD02 adr offsets {:?}", v[0]));
            }
        }
    }
    f
}
