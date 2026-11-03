// Harness plumbing for check, run, and diagnostics.
use nexo::diagnostic::{DataValue, Diagnostic};
use nexo::ir::{CompileError, check_scalar, compile_scalar};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub struct Rng(pub u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut r = Rng(seed.wrapping_mul(0xA24BAED4963EE407) ^ 0x9FB21C651E98DF25);
        for _ in 0..6 {
            r.next();
        }
        r
    }
    pub fn next(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    pub fn chance(&mut self, p: u64, of: u64) -> bool {
        self.below(of) < p
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len() as u64) as usize]
    }
}

#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub struct D {
    pub code: String,
    pub sub: String,
    pub stage: String,
    pub op: String,
    pub s: u64,
    pub e: u64,
    pub exp: String,
    pub found: String,
    pub rel: Vec<(String, u64, u64)>,
    pub data: String,
    pub extra: String,
}
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum R {
    Ok,
    Rej(Vec<D>),
    Bug(String),
}
pub fn conv(d: &Diagnostic) -> D {
    let g = |k: &str| match d.data.get(k) {
        Some(DataValue::String(s)) => s.clone(),
        _ => String::new(),
    };
    let (s, e) = d.primary_span.as_ref().map(|p| (p.start, p.end)).unwrap_or((0, 0));
    let mut rel: Vec<_> = d
        .related_spans
        .iter()
        .map(|r| (r.role.as_str().to_string(), r.span.start, r.span.end))
        .collect();
    rel.sort_by_key(|x| (x.1, x.2));
    D {
        code: d.code.clone(),
        sub: d.subcause.clone().unwrap_or_default(),
        stage: d.stage.as_str().to_string(),
        op: d.operation.as_str().to_string(),
        s,
        e,
        exp: g("expected_type"),
        found: g("found_type"),
        rel,
        data: format!("{:?}", d.data),
        extra: format!(
            "chain={} repair={} feat={:?} rule={} expl={:?}",
            d.cause_chain.len(),
            d.repair.len(),
            d.feature_id,
            d.rule,
            d.explanation_id
        ),
    }
}
fn rej(v: Vec<Box<Diagnostic>>) -> R {
    R::Rej(v.iter().map(|d| conv(d)).collect())
}
pub fn check(src: &str) -> R {
    match catch_unwind(AssertUnwindSafe(|| check_scalar("t.n", src))) {
        Err(_) => R::Bug("panic(check)".into()),
        Ok(Ok(_)) => R::Ok,
        Ok(Err(CompileError::Rejected(v))) => rej(v),
        Ok(Err(CompileError::InvalidIr(m))) => R::Bug(format!("InvalidIr:{m}")),
        Ok(Err(_)) => R::Bug("Dependent/other".into()),
    }
}
/// compile + execute. Ok carries the outcome kind.
pub fn run(src: &str) -> (R, String) {
    match catch_unwind(AssertUnwindSafe(|| match compile_scalar("t.n", src) {
        Ok(p) => {
            let ex = nexo::interp::run_scalar(&p);
            match ex.outcome {
                nexo::interp::Outcome::InternalError(m) => (R::Bug(format!("run internal:{m}")), "internal".into()),
                o => (R::Ok, format!("{:?}", o)),
            }
        }
        Err(CompileError::Rejected(v)) => (rej(v), "rejected".into()),
        Err(CompileError::InvalidIr(m)) => (R::Bug(format!("InvalidIr:{m}")), "invalidir".into()),
        Err(_) => (R::Bug("Dependent/other".into()), "other".into()),
    })) {
        Err(_) => (R::Bug("panic(run)".into()), "panic".into()),
        Ok(r) => r,
    }
}
/// Duplicate = same code, primary and related, or same everything.
pub fn has_dup(r: &R) -> bool {
    if let R::Rej(v) = r {
        let mut seen = std::collections::HashSet::new();
        for d in v {
            if !seen.insert((d.code.clone(), d.sub.clone(), d.s, d.e, d.exp.clone(), d.found.clone(), d.rel.clone())) {
                return true;
            }
        }
    }
    false
}
pub fn has_dup_loose(r: &R) -> bool {
    // same code+primary only (any related)
    if let R::Rej(v) = r {
        let mut seen = std::collections::HashSet::new();
        for d in v {
            if !seen.insert((d.code.clone(), d.sub.clone(), d.s, d.e)) {
                return true;
            }
        }
    }
    false
}
pub fn has_site_dup(r: &R) -> bool {
    if let R::Rej(v) = r {
        let mut seen = std::collections::HashSet::new();
        for d in v.iter().filter(|d| d.stage == "types") {
            if !seen.insert((d.s, d.e)) {
                return true;
            }
        }
    }
    false
}
pub fn par<F: Fn(u64) + Sync>(n: u64, threads: u64, f: F) {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
    let done = AtomicU64::new(0);
    let cur: Vec<AtomicU64> = (0..threads).map(|_| AtomicU64::new(u64::MAX)).collect();
    let since: Vec<AtomicU64> = (0..threads).map(|_| AtomicU64::new(0)).collect();
    let t0 = std::time::Instant::now();
    std::thread::scope(|sc| {
        let (done, cur, since) = (&done, &cur, &since);
        sc.spawn(move || {
            let mut last = 0;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(20));
                let d = done.load(Relaxed);
                if d >= n {
                    break;
                }
                eprintln!("PROGRESS {d}/{n} t={}s", t0.elapsed().as_secs());
                let now = t0.elapsed().as_secs();
                for (t, c) in cur.iter().enumerate() {
                    let i = c.load(Relaxed);
                    if i != u64::MAX && now.saturating_sub(since[t].load(Relaxed)) > 25 {
                        eprintln!("SLOW worker {t} index {i} running {}s", now - since[t].load(Relaxed));
                    }
                }
                if d == last && d > 0 { eprintln!("NOPROGRESS"); }
                last = d;
            }
        });
        for t in 0..threads {
            let f = &f;
            std::thread::Builder::new()
                .stack_size(32 << 20)
                .spawn_scoped(sc, move || {
                    let mut i = t;
                    while i < n {
                        cur[t as usize].store(i, Relaxed);
                        since[t as usize].store(t0.elapsed().as_secs(), Relaxed);
                        f(i);
                        done.fetch_add(1, Relaxed);
                        i += threads;
                    }
                    cur[t as usize].store(u64::MAX, Relaxed);
                })
                .unwrap();
        }
    });
}
