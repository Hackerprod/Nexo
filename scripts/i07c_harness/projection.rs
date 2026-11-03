use crate::common::{R, check, run};
use std::io::{BufRead, BufReader, BufWriter, Write};

// This raw projection records acceptance, code, subcause, and primary site.
// The expanded corpus includes dependent consumers, so each changed row is
// adjudicated under F0-0002 §4.1.2.5 before it is counted as normative.
fn normative(result: &R) -> String {
    match result {
        R::Ok => "A".into(),
        R::Bug(reason) => format!("BUG:{reason}"),
        R::Rej(diagnostics) => {
            let mut out = String::from("R");
            for d in diagnostics {
                out.push('|');
                out.push_str(&d.code);
                out.push(':');
                out.push_str(&d.sub);
                out.push(':');
                out.push_str(&d.s.to_string());
                out.push('-');
                out.push_str(&d.e.to_string());
            }
            out
        }
    }
}

fn run_corpus_with(input: &str, output: &str, render: fn(&R) -> String) {
    let source = std::fs::File::open(input).expect("projection corpus");
    let target = std::fs::File::create(output).expect("projection output");
    let mut target = BufWriter::new(target);
    let programs: Vec<String> = BufReader::new(source)
        .lines()
        .map(|line| line.expect("program line"))
        .collect();
    let available = std::thread::available_parallelism()
        .map(|count| count.get().min(12))
        .unwrap_or(1);
    let workers = std::env::var("I07C_PROJECTION_WORKERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|count| (1..=available).contains(count))
        .unwrap_or(available);
    let chunk_size = programs.len().div_ceil(workers).max(1);
    let parts: Vec<Vec<String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = programs
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|program| render(&check(program)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("projection worker"))
            .collect()
    });
    for part in parts {
        for line in part {
            writeln!(target, "{line}").expect("projection output line");
        }
    }
    target.flush().expect("projection flush");
    eprintln!("projection programs={}", programs.len());
}

pub fn run_corpus(input: &str, output: &str) {
    run_corpus_with(input, output, normative);
}

pub fn run_corpus_full(input: &str, output: &str) {
    run_corpus_with(input, output, |result| format!("{result:?}"));
}

// This second pass contains exactly the sources accepted by the baseline
// check projection. It detects rejection or internal failure while compiling
// and running a program that the checker accepted.
pub fn run_accepted_corpus(input: &str, output: &str) {
    let source = std::fs::File::open(input).expect("accepted projection corpus");
    let target = std::fs::File::create(output).expect("runtime projection output");
    let mut target = BufWriter::new(target);
    let mut count = 0_u64;
    for line in BufReader::new(source).lines() {
        let program = line.expect("accepted program line");
        let (executed, _) = run(&program);
        writeln!(target, "{}", normative(&executed)).expect("runtime projection output line");
        count += 1;
    }
    target.flush().expect("runtime projection flush");
    eprintln!("runtime projection programs={count}");
}
