#[cfg(test)]
mod i07c_quota_property {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn pick(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    fn atom(r: &mut Rng) -> String {
        match r.pick(6) {
            0 => format!("{}", r.pick(90)),
            1 => "true".into(),
            2 => "false".into(),
            3 => format!("[{}]", r.pick(9)),
            4 => "[true]".into(),
            _ => format!("{}", r.pick(900)),
        }
    }
    fn expr(r: &mut Rng, d: u32, vars: &[String]) -> String {
        if d == 0 || r.pick(4) == 0 {
            if !vars.is_empty() && r.pick(3) == 0 {
                return vars[r.pick(vars.len() as u64) as usize].clone();
            }
            return atom(r);
        }
        match r.pick(7) {
            0 => format!(
                "(if {} then {} else {})",
                expr(r, d - 1, vars),
                expr(r, d - 1, vars),
                expr(r, d - 1, vars)
            ),
            1 => format!("({}+{})", expr(r, d - 1, vars), expr(r, d - 1, vars)),
            2 => format!("({}=={})", expr(r, d - 1, vars), expr(r, d - 1, vars)),
            3 => format!("[{},{}]", expr(r, d - 1, vars), expr(r, d - 1, vars)),
            4 => format!("[{}]", expr(r, d - 1, vars)),
            5 => format!(
                "(if {} then {{value:{};}} else {})",
                atom(r),
                expr(r, d - 1, vars),
                expr(r, d - 1, vars)
            ),
            _ => format!("({}<{})", expr(r, d - 1, vars), expr(r, d - 1, vars)),
        }
    }
    fn ty(r: &mut Rng) -> &'static str {
        match r.pick(5) {
            0 => "I64",
            1 => "Bool",
            2 => "Array<I64>",
            3 => "Array<Bool>",
            _ => "Array<Array<I64>>",
        }
    }
    fn body(r: &mut Rng, fid: usize, nfun: usize, ctr: &mut usize) -> String {
        let mut s = String::new();
        let mut vars: Vec<String> = Vec::new();
        let n = 2 + r.pick(5);
        for _ in 0..n {
            *ctr += 1;
            let v = format!("v{}n{}", fid, *ctr);
            let d = 1 + r.pick(3) as u32;
            match r.pick(8) {
                0 => s += &format!("let {v}:{}={};", ty(r), expr(r, d, &vars)),
                1 | 2 => s += &format!("let {v}={};", expr(r, d, &vars)),
                3 => s += &format!("var {v}=[];{v}={};", expr(r, d, &vars)),
                4 => {
                    s += &format!("AssertEq({},{});", expr(r, d, &vars), expr(r, d, &vars));
                    continue;
                }
                5 => {
                    let callee = 1 + r.pick(nfun as u64) as usize;
                    if callee != fid {
                        s += &format!("F{callee}({});", expr(r, d, &vars));
                        continue;
                    }
                    s += &format!("let {v}={};", expr(r, d, &vars));
                }
                6 => s += &format!("var {v}={};{v}={};", expr(r, d, &vars), expr(r, d, &vars)),
                _ => {
                    s += &format!(
                        "let {v}=(if {} then {} else {});",
                        expr(r, d, &vars),
                        expr(r, d, &vars),
                        expr(r, d, &vars)
                    )
                }
            }
            vars.push(v);
        }
        s
    }
    fn program(r: &mut Rng) -> String {
        let nfun = 3 + r.pick(3) as usize;
        let mut ctr = 0;
        let mut s = String::new();
        for f in 1..=nfun {
            let pt = if r.pick(2) == 0 { "I64" } else { "Array<I64>" };
            s += &format!("fn F{f}(p{f}:view {pt}){{{}}}", body(r, f, nfun, &mut ctr));
        }
        s += &format!("fn Main(){{{}}}", body(r, 99, nfun, &mut ctr));
        s
    }
    type Key = (String, Option<String>, Option<(u64, u64)>, String, String);
    fn key(d: &crate::diagnostic::Diagnostic) -> Key {
        (
            d.code.clone(),
            d.subcause.clone(),
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            format!("{:?}", d.related_spans),
            format!("{:?}", d.data),
        )
    }

    #[test]
    #[ignore]
    fn reduced_quota_property() {
        let seeds: Vec<u64> = std::env::var("I07C_QUOTA_SEEDS")
            .unwrap_or("1".into())
            .split(',')
            .map(|s| s.parse().unwrap())
            .collect();
        let per: usize = std::env::var("I07C_QUOTA_PER")
            .unwrap_or("400".into())
            .parse()
            .unwrap();
        let (mut progs, mut withq, mut multi, mut checks, mut viol, mut skipped) =
            (0, 0, 0, 0, 0, 0);
        for seed in seeds {
            let mut r = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
            for _ in 0..per {
                let source = program(&mut r);
                let ast = match crate::frontend::parse::parse("q.n", &source) {
                    Ok(a) => a,
                    Err(e) => {
                        skipped += 1;
                        if skipped < 4 {
                            eprintln!("SKIPP {e:?} {source}");
                        }
                        continue;
                    }
                };
                let resolved = match names::resolve("q.n", &source, &ast) {
                    Ok(a) => a,
                    Err(e) => {
                        skipped += 1;
                        if skipped < 8 {
                            eprintln!("SKIPR {e:?} {source}");
                        }
                        continue;
                    }
                };
                progs += 1;
                let full = check_with_partial_limit("q.n", &source, &ast, &resolved, 1_000_000);
                if full.errors.len() > 1 {
                    multi += 1;
                }
                let fullkeys: Vec<Key> = full.errors.iter().map(|d| key(d)).collect();
                let mut limits: Vec<u64> = (1..=48).collect();
                for _ in 0..8 {
                    limits.push(49 + r.pick(400));
                }
                let mut seen_no_quota = false;
                limits.sort();
                for limit in limits {
                    checks += 1;
                    let red = check_with_partial_limit("q.n", &source, &ast, &resolved, limit);
                    let q: Vec<_> = red.errors.iter().filter(|d| d.code == "NX-E0003").collect();
                    let mut bad = String::new();
                    if q.len() > 1 {
                        bad = "multiple quota".into();
                    }
                    if q.len() == 1 {
                        withq += 1;
                        if seen_no_quota {
                            bad = "quota reappears at larger limit".into();
                        }
                        if q[0].data.get("observed") != Some(&DataValue::UInt(limit + 1)) {
                            bad = format!("observed {:?}", q[0].data.get("observed"));
                        }
                        for d in red.errors.iter().filter(|d| d.code != "NX-E0003") {
                            if !fullkeys.contains(&key(d)) {
                                bad = format!("not subset: {d:?}");
                            }
                        }
                    } else {
                        seen_no_quota = true;
                        let rk: Vec<Key> = red.errors.iter().map(|d| key(d)).collect();
                        if rk != fullkeys {
                            bad = "no quota but differs from full".into();
                        }
                    }
                    if !bad.is_empty() {
                        viol += 1;
                        if viol < 6 {
                            eprintln!("VIOL {bad} limit {limit} src {source}");
                        }
                    }
                }
            }
        }
        eprintln!(
            "TOTAL programs {progs} skipped {skipped} with_quota_checks {withq} multi_diag_full {multi} checks {checks} violations {viol}"
        );
        assert_eq!(viol, 0);
    }
}
