#[path = "../i07c_common.rs"]
mod common;
mod lits;
mod tree_gen;
mod grp;
mod model;
mod defer;
mod oracle;
mod fuzz;
mod deep;
mod projection;
mod types;
use common::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mode = a.get(1).map(String::as_str).unwrap_or("");
    match mode {
        "lits" => {
            for entry in ["Main"] {
                let f = lits::run_all(entry);
                println!("lits entry={entry}: {} failures", f.len());
                for x in f {
                    println!("  {x}");
                }
            }
        }
        "grp" => {
            let n: u64 = a[2].parse().unwrap();
            let seed: u64 = a[3].parse().unwrap();
            grp::run(n, seed, a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8));
        }
        "model" => {
            let n: u64 = a[2].parse().unwrap();
            let seed: u64 = a[3].parse().unwrap();
            model::run(n, seed, a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8), &a[3]);
        }
        "defer" => {
            let n: u64 = a[2].parse().unwrap();
            let seed: u64 = a[3].parse().unwrap();
            defer::run(n, seed, a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8));
        }
        "oracle" => {
            let n: u64 = a[2].parse().unwrap();
            let seed: u64 = a[3].parse().unwrap();
            oracle::run(n, seed, a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8));
        }
        "fuzz" => {
            let n: u64 = a[2].parse().unwrap();
            let seed: u64 = a[3].parse().unwrap();
            fuzz::run(n, seed, a.get(4).and_then(|x| x.parse().ok()).unwrap_or(8));
        }
        "types" => {
            let n: u64 = a[2].parse().unwrap();
            let seed: u64 = a[3].parse().unwrap();
            types::run_cases(n, seed, a.get(4).and_then(|x| x.parse().ok()).unwrap_or(1));
        }
        "deep" => {
            let kib: usize = a[2].parse().unwrap();
            let depths: Vec<usize> = a[3].split(',').map(|x| x.parse().unwrap()).collect();
            deep::run(kib, &depths);
        }
        "sum" => {
            let src = std::fs::read_to_string(&a[2]).unwrap();
            match check(&src) {
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
        "diag" => {
            let src = std::fs::read_to_string(&a[2]).unwrap();
            println!("check: {:?}", check(&src));
            println!("run: {:?}", run(&src));
        }
        "projection" => projection::run_corpus(&a[2], &a[3]),
        "projection_full" => projection::run_corpus_full(&a[2], &a[3]),
        "projection_runtime" => projection::run_accepted_corpus(&a[2], &a[3]),
        _ => eprintln!("modes: lits diag"),
    }
}
