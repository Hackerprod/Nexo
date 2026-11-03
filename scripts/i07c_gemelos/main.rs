#[path = "../i07c_common.rs"]
mod common;
mod ast;
mod model;
mod run;
mod pairs;
mod modes2;
use common::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str).unwrap_or("") {
        "sum" => {
            let src = std::fs::read_to_string(&a[2]).unwrap();
            print_sum(&src);
        }
        "model" => {
            let n: u64 = a[3].parse().unwrap();
            let seed: u64 = a[4].parse().unwrap();
            run::run_model_mode(&a[2], n, seed, a.get(5).and_then(|x| x.parse().ok()).unwrap_or(8), a.get(6));
        }
        "pairs" => {
            let n: u64 = a[2].parse().unwrap();
            let stride: u64 = a[3].parse().unwrap();
            let off: u64 = a[4].parse().unwrap();
            run::run_pairs_mode(n, stride, off, a.get(5).and_then(|x| x.parse().ok()).unwrap_or(8), a.get(6));
        }
        "gen" => {
            let i: u64 = a[4].parse().unwrap();
            if a[2] == "pairs" {
                println!("{}", run::pair_src(i));
            } else {
                println!("{}", run::gen_src(&a[2], a[3].parse().unwrap(), i));
            }
        }
        "lits" => {
            let (n, f) = modes2::lits();
            println!("LITS cases={} failures={}", n, f.len());
            for x in f {
                println!("  FAIL {x}");
            }
        }
        "grp" => {
            let n: u64 = a[2].parse().unwrap();
            let seed: u64 = a[3].parse().unwrap();
            modes2::grp(n, seed, a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8));
        }
        "oracle" => {
            let n: u64 = a[2].parse().unwrap();
            let seed: u64 = a[3].parse().unwrap();
            modes2::oracle(n, seed, a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8));
        }
        "fuzz" => {
            let n: u64 = a[2].parse().unwrap();
            let seed: u64 = a[3].parse().unwrap();
            modes2::fuzz(n, seed, a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8));
        }
        "deep" => {
            let kib: usize = a[2].parse().unwrap();
            let depths: Vec<usize> = a[3].split(',').map(|x| x.parse().unwrap()).collect();
            modes2::deep(kib, &depths);
        }
        _ => eprintln!("modes: sum model pairs lits grp oracle fuzz deep"),
    }
}
pub fn print_sum(src: &str) {
    match check(src) {
        R::Ok => println!("OK"),
        R::Bug(m) => println!("BUG {m}"),
        R::Rej(v) => {
            for d in v {
                let t = |s: u64, e: u64| src.get(s as usize..e as usize).unwrap_or("?").to_string();
                let rel: Vec<String> = d.rel.iter().map(|(r, s, e)| format!("{r}={}[{s},{e})", t(*s, *e))).collect();
                println!("{}{} {} `{}`[{},{}) exp={} found={} rel={:?}", d.code, if d.sub.is_empty() { String::new() } else { format!("/{}", d.sub) }, d.stage, t(d.s, d.e), d.s, d.e, d.exp, d.found, rel);
            }
        }
    }
}
