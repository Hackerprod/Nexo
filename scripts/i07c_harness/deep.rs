// (f) Deep cases under 128/256/512 KiB stacks. One process per stack size; every case is announced before it runs
// so a stack overflow (process abort) identifies the case.
use crate::common::*;
use std::io::Write;

fn rep(s: &str, n: usize) -> String {
    s.repeat(n)
}

pub fn families(d: usize) -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = Vec::new();
    let w = |b: String| format!("fn Main(){{{b}}}");
    v.push(("arr_nest", w(format!("let a={}1{};", rep("[", d), rep("]", d)))));
    v.push(("arr_nest_annot_bad", w(format!("let a:Bool={}1{};", rep("[", d), rep("]", d)))));
    v.push(("arr_nest_true_in_I64", w(format!("let a:Array<I64>={}true{};", rep("[", d), rep("]", d)))));
    v.push(("paren_nest", w(format!("let a={}1{};", rep("(", d), rep(")", d)))));
    v.push(("paren_nest_bad", w(format!("let a:Bool={}1{};", rep("(", d), rep(")", d)))));
    v.push(("binary_left_chain", w(format!("let a=1{};", rep("+1", d)))));
    v.push(("binary_left_chain_bad", w(format!("let a=1{}+true;", rep("+1", d)))));
    v.push(("binary_right_nest", w(format!("let a={}1{};", rep("1+(", d), rep(")", d)))));
    v.push(("if_nest_else", w(format!("let a={}1{};", rep("(if true then 2 else ", d), rep(")", d)))));
    v.push(("if_nest_then", w(format!("let a={}1{};", rep("(if true then ", d), rep(" else 2)", d)))));
    v.push(("if_nest_bad_join", w(format!("let a={}true{};", rep("(if true then 2 else ", d), rep(")", d)))));
    v.push(("blk_nest", w(format!("let a={}1{};", rep("(if true then {value:", d), rep(";} else 2)", d)))));
    v.push(("blk_nest_bad_annot", w(format!("let a:Bool={}1{};", rep("(if true then {value:", d), rep(";} else true)", d)))));
    v.push(("index_chain", w(format!("let a={}1{};let b=a{};", rep("[", d), rep("]", d), rep("[0]", d)))));
    v.push(("stmt_nest_if", w(format!("{}let a=1;{}", rep("if true {", d), rep("}", d)))));
    v.push(("stmt_nest_while", w(format!("{}let a=1;{}", rep("while false {", d), rep("}", d)))));
    v.push(("eq_nest", w(format!("let a={}[1]{};", rep("[", d), rep("]", d)) + &format!("AssertEq(a,{}[1]{});", rep("[", d), rep("]", d)))));
    // deferred chains
    let dv = "(if true then {return;} else {return;})";
    let mut s = format!("var v0={dv};");
    for i in 1..d.min(3000) {
        s += &format!("var v{i}=v{};", i - 1);
    }
    s += "v0=3;";
    v.push(("alias_chain", w(s)));
    let mut s = format!("var v0={dv};");
    for i in 1..d.min(3000) {
        s += &format!("var v{i}=v{};", i - 1);
    }
    s += &format!("v{}=true;v0=3;", d.min(3000) - 1);
    v.push(("alias_chain_conflict", w(s)));
    let mut s = String::new();
    for i in 0..d.min(3000) {
        s += &format!("var v{i}=[];");
    }
    for i in 0..d.min(3000) - 1 {
        s += &format!("v{i}=[v{}];", i + 1);
    }
    s += &format!("v{}=[1];", d.min(3000) - 1);
    v.push(("deferred_array_ladder", w(s)));
    let mut s = String::new();
    for i in 0..d.min(3000) {
        s += &format!("var v{i}=[];");
    }
    for i in 0..d.min(3000) - 1 {
        s += &format!("v{i}=[v{}];", i + 1);
    }
    s += &format!("v{}=[v0];", d.min(3000) - 1);
    v.push(("deferred_array_cycle", w(s)));
    v.push(("deferred_nest_literal", w(format!("var r=[];let a={}r{};r=[1];", rep("[", d), rep("]", d)))));
    v.push(("deferred_nest_bad", w(format!("var r=[];let a={}r{};r=1;", rep("[", d), rep("]", d)))));
    v.push(("many_statements", w(rep("let a=1;", d * 4))));
    v
}

pub fn run(stack_kib: usize, depths: &[usize]) {
    for &d in depths {
        for (name, src) in families(d) {
            println!("CASE stack={stack_kib}KiB depth={d} family={name} bytes={}", src.len());
            std::io::stdout().flush().unwrap();
            let s2 = src.clone();
            let h = std::thread::Builder::new()
                .stack_size(stack_kib * 1024)
                .spawn(move || {
                    let c = check(&s2);
                    let (r, _) = crate::common::run(&s2);
                    (c, r)
                })
                .unwrap();
            let (c, r) = h.join().unwrap();
            let kind = match &c {
                R::Ok => "ok".to_string(),
                R::Rej(v) => format!("rej:{}", v.iter().map(|d| format!("{}/{}", d.code, d.sub)).collect::<std::collections::BTreeSet<_>>().into_iter().collect::<Vec<_>>().join("+")),
                R::Bug(m) => format!("BUG:{m}"),
            };
            let coherent = c == r;
            let dup = has_dup(&c);
            println!("RESULT {kind} coherent={coherent} dup={dup}");
        }
    }
}
