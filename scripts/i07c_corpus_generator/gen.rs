// Independent structured generators for diagnostic and runtime differential programs.
// Families (selected by index hash):
//   U untyped-AST soup, S shuffled scripts, T tainted initialisers + indexed writes, R mutation of the RD rows
//   (statement/char level, different operators than earlier rounds), K functions & calls, D typed generator with injected defects.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64, idx: u64) -> Rng {
        let mut r = Rng(seed.wrapping_mul(0xA24BAED4963EE407)
            ^ idx.wrapping_mul(0x9FB21C651E98DF25)
            ^ 0x18B0_7A11_C0DE_5EED);
        for _ in 0..3 {
            r.next();
        }
        r
    }
    pub fn next(&mut self) -> u64 {
        // xorshift64* variant (differs from earlier rounds' splitmix)
        let mut x = self.0 ^ 0x2545F4914F6CDD1D;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x.wrapping_add(0x9E3779B97F4A7C15);
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        ((self.next() >> 11) % n as u64) as usize
    }
    pub fn p(&mut self, pct: usize) -> bool {
        self.below(100) < pct
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn shuffle<T>(&mut self, xs: &mut Vec<T>) {
        for i in (1..xs.len()).rev() {
            let j = self.below(i + 1);
            xs.swap(i, j);
        }
    }
}

pub const NEVERS: [&str; 5] = [
    "(if true then {return;} else {return;})",
    "(if c then {return;} else {return;})",
    "(if false then {return;} else {return;})",
    "(if c then {return;} else {return;})",
    "(if true then {return;} else {return;})",
];

const LEAVES: [&str; 14] = [
    "0", "1", "5", "true", "false", "c", "ar", "[]", "[1]", "[true]", "[[]]", "[[1]]", "x", "br",
];

fn nev(r: &mut Rng) -> String {
    r.pick(&NEVERS).to_string()
}

// ---------- U: untyped AST ----------
#[derive(Clone)]
pub enum E {
    Leaf(String),
    Bin(&'static str, Box<E>, Box<E>),
    Idx(Box<E>, Box<E>),
    Len(Box<E>),
    Arr(Vec<E>),
    If(Box<E>, Box<E>, Box<E>),
    IfV(Box<E>, Box<E>, Box<E>), // (if c then {value:a;} else b)
    Grp(Box<E>),
}

pub fn gen_e(r: &mut Rng, vars: &[String], depth: usize) -> E {
    if depth == 0 || r.p(18) {
        if !vars.is_empty() && r.p(55) {
            return E::Leaf(r.pick(vars).clone());
        }
        if r.p(10) {
            return E::Leaf(nev(r));
        }
        return E::Leaf(r.pick(&LEAVES).to_string());
    }
    let d = depth - 1;
    match r.below(12) {
        0 | 1 => E::Bin(
            r.pick(&["+", "-", "<", "==", "==", "<"]),
            Box::new(gen_e(r, vars, d)),
            Box::new(gen_e(r, vars, d)),
        ),
        2 | 3 => E::Idx(Box::new(gen_e(r, vars, d)), Box::new(gen_e(r, vars, d))),
        4 => E::Len(Box::new(gen_e(r, vars, d))),
        5 | 6 => {
            let n = r.below(4);
            E::Arr((0..n).map(|_| gen_e(r, vars, d)).collect())
        }
        7 => E::If(
            Box::new(gen_e(r, vars, d)),
            Box::new(gen_e(r, vars, d)),
            Box::new(gen_e(r, vars, d)),
        ),
        8 => E::IfV(
            Box::new(gen_e(r, vars, d)),
            Box::new(gen_e(r, vars, d)),
            Box::new(gen_e(r, vars, d)),
        ),
        9 => E::Grp(Box::new(gen_e(r, vars, d))),
        _ => E::Bin(
            r.pick(&["+", "==", "<", "-"]),
            Box::new(gen_e(r, vars, d)),
            Box::new(E::Leaf(r.pick(&LEAVES).to_string())),
        ),
    }
}

impl E {
    pub fn text(&self) -> String {
        match self {
            E::Leaf(s) => s.clone(),
            E::Bin(op, a, b) => format!("({}{}{})", a.text(), op, b.text()),
            E::Idx(a, i) => {
                let at = a.text();
                let atom = matches!(**a, E::Leaf(_) | E::Idx(..) | E::Arr(_) | E::Grp(_));
                if atom {
                    format!("{at}[{}]", i.text())
                } else {
                    format!("({at})[{}]", i.text())
                }
            }
            E::Len(a) => format!("Len({})", a.text()),
            E::Arr(xs) => format!(
                "[{}]",
                xs.iter().map(|x| x.text()).collect::<Vec<_>>().join(",")
            ),
            E::If(c, a, b) => format!("(if {} then {} else {})", c.text(), a.text(), b.text()),
            E::IfV(c, a, b) => format!(
                "(if {} then {{value:{};}} else {})",
                c.text(),
                a.text(),
                b.text()
            ),
            E::Grp(a) => format!("({})", a.text()),
        }
    }
}

const BASE_PRE: &str = "let c=true;let x=1;let ar=[1];let br=[true];";

pub fn fam_u(r: &mut Rng) -> String {
    let mut vars: Vec<String> = vec!["c", "x", "ar", "br"]
        .into_iter()
        .map(String::from)
        .collect();
    let mut muts: Vec<String> = vec![];
    let mut s = format!("fn Main(){{{BASE_PRE}");
    let nstmt = 1 + r.below(7);
    for k in 0..nstmt {
        let sz = 1 + r.below(4);
        let e = gen_e(r, &vars, sz).text();
        match r.below(14) {
            0 | 1 | 2 => {
                s += &format!("let l{k}={e};");
                vars.push(format!("l{k}"));
            }
            3 | 4 => {
                s += &format!("var m{k}={e};");
                vars.push(format!("m{k}"));
                muts.push(format!("m{k}"));
            }
            5 => {
                s += &format!("var m{k}=[];");
                vars.push(format!("m{k}"));
                muts.push(format!("m{k}"));
            }
            6 => {
                s += &format!("var m{k}={};", nev(r));
                vars.push(format!("m{k}"));
                muts.push(format!("m{k}"));
            }
            7 | 8 if !muts.is_empty() => {
                let v = r.pick(&muts).clone();
                s += &format!("{v}={e};");
            }
            9 if !muts.is_empty() => {
                let v = r.pick(&muts).clone();
                let i = gen_e(r, &vars, 2).text();
                s += &format!("{v}[{i}]={e};");
            }
            10 => s += &format!("Print({e});"),
            11 => {
                let f = gen_e(r, &vars, sz).text();
                s += &format!("AssertEq({e},{f});");
            }
            12 => {
                let t = r.pick(&[
                    "I64",
                    "Bool",
                    "Array<I64>",
                    "Array<Bool>",
                    "Array<Array<I64>>",
                ]);
                s += &format!("let a{k}:{t}={e};");
            }
            _ => s += &format!("if {e} {{let w{k}={};}}", gen_e(r, &vars, 2).text()),
        }
    }
    s + "}"
}

// ---------- S: shuffled scripts ----------
pub fn fam_s(r: &mut Rng) -> String {
    let nv = 2 + r.below(3);
    let names: Vec<String> = (0..nv).map(|i| format!("v{i}")).collect();
    let mut s = format!("fn Main(){{{BASE_PRE}");
    for n in &names {
        let init = match r.below(4) {
            0 => nev(r),
            1 => "[]".to_string(),
            _ => nev(r),
        };
        s += &format!("var {n}={init};");
    }
    let mut stmts: Vec<String> = vec![];
    let ns = 3 + r.below(6);
    let pool: Vec<String> = names
        .iter()
        .cloned()
        .chain(["c", "x", "ar", "br"].iter().map(|s| s.to_string()))
        .collect();
    for _ in 0..ns {
        let a = r.pick(&names).clone();
        let b = r.pick(&names).clone();
        let st = match r.below(14) {
            0 => format!("{a}=3;"),
            1 => format!("{a}=true;"),
            2 => format!("{a}=[1];"),
            3 => format!("{a}=[true];"),
            4 => format!("{a}={b};"),
            5 => format!("{a}[0]={};", r.pick(&["1", "true", "[1]", "ar"])),
            6 => format!("let q=({a}=={b});"),
            7 => format!("let q=({a}<{b});"),
            8 => format!("AssertEq({a},{});", r.pick(&pool)),
            9 => format!("let q=({a}+{});", r.pick(&["1", "true", "x", "c"])),
            10 => format!("let q=Len({a});"),
            11 => format!("let q=[{a},{}];", r.pick(&pool)),
            12 => format!("let q={a}[{}];", r.pick(&["0", "true", "x"])),
            _ => format!("Print({a});"),
        };
        stmts.push(st);
    }
    if r.p(85) {
        r.shuffle(&mut stmts);
    }
    // unique `let q` names to avoid duplicate-binding noise
    let mut k = 0;
    for st in stmts {
        if st.starts_with("let q=") {
            k += 1;
            s += &st.replacen("let q=", &format!("let q{k}="), 1);
        } else {
            s += &st;
        }
    }
    s + "}"
}

// ---------- T: tainted initialisers + indexed writes ----------
pub fn fam_t(r: &mut Rng) -> String {
    let bad = [
        "(true+1)",
        "(1+[1])",
        "(1<true)",
        "(1==[1])",
        "ar[c]",
        "c[0]",
        "Len(1)",
        "[1,true]",
        "(if c then 1 else [1])",
        "[(true+1)]",
        "([1]<[1])",
        "(c==1)",
        "[[1],[true]]",
        "ar[ar]",
        "(if 1 then 1 else 2)",
        "Len(c)",
        "br[br]",
        "[1][true]",
        "((1==true)==1)",
    ];
    let mut s = format!("fn Main(){{{BASE_PRE}");
    let nt = 1 + r.below(3);
    let mut ts = vec![];
    for i in 0..nt {
        let init = match r.below(6) {
            0 => nev(r),
            1 => "[]".into(),
            _ => r.pick(&bad).to_string(),
        };
        s += &format!("var t{i}={init};");
        ts.push(format!("t{i}"));
    }
    let vars: Vec<String> = ts
        .iter()
        .cloned()
        .chain(["c", "x", "ar", "br"].iter().map(|s| s.to_string()))
        .collect();
    let n = 1 + r.below(7);
    for k in 0..n {
        let t = r.pick(&ts).clone();
        let dd = 1 + r.below(3);
        let e = gen_e(r, &vars, dd).text();
        let idx = gen_e(r, &vars, 1).text();
        let st = match r.below(12) {
            0 | 1 => format!("{t}={e};"),
            2 | 3 | 4 => format!("{t}[{idx}]={e};"),
            5 => format!("{t}[{idx}][{}]={e};", r.pick(&["0", "1", "true", "x"])),
            6 => format!("let u{k}={t};"),
            7 => format!("let u{k}=({t}=={e});"),
            8 => format!("if {} {{{t}[0]={e};}}", r.pick(&["c", "true", "1", "ar"])),
            9 => format!("AssertEq({t},{e});"),
            10 => format!("{t}[{idx}]=({t}[{}]);", r.pick(&["0", "x", "1"])),
            _ => format!("let u{k}={t}[{idx}];"),
        };
        s += &st;
    }
    s + "}"
}

// ---------- R: mutation of RD rows (char/statement level) ----------
fn split_stmts(body: &str) -> Vec<String> {
    let mut out = vec![];
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in body.chars() {
        cur.push(ch);
        match ch {
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => depth -= 1,
            ';' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
            }
            _ => {}
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

pub fn fam_r(r: &mut Rng, rd: &[String]) -> String {
    let src = r.pick(rd).clone();
    let (Some(a), Some(b)) = (src.find("fn Main(){"), src.rfind('}')) else {
        return src;
    };
    let head = &src[..a + 10];
    let mut stmts = split_stmts(&src[a + 10..b]);
    let tail = &src[b..];
    let lits = [
        "0", "1", "7", "true", "false", "[]", "[1]", "[true]", "[[1]]", "c", "ar",
    ];
    for _ in 0..1 + r.below(3) {
        if stmts.is_empty() {
            break;
        }
        let i = r.below(stmts.len());
        match r.below(10) {
            0 => {
                stmts.remove(i);
            }
            1 => {
                let d = stmts[i].clone();
                stmts.insert(i, d);
            }
            2 if stmts.len() > 1 => {
                let j = r.below(stmts.len());
                stmts.swap(i, j);
            }
            3 => {
                // replace a literal occurrence by another literal
                let st = stmts[i].clone();
                let cands = ["true", "false", "1", "3", "[3]", "[]"];
                let from = *r.pick(&cands);
                if let Some(pos) = st.find(from) {
                    let to = r.pick(&lits);
                    stmts[i] = format!("{}{}{}", &st[..pos], to, &st[pos + from.len()..]);
                }
            }
            4 => {
                let st = &stmts[i];
                stmts[i] = if st.starts_with("var ") {
                    st.replacen("var ", "let ", 1)
                } else {
                    st.replacen("let ", "var ", 1)
                };
            }
            5 => {
                let st = stmts[i].clone();
                stmts[i] =
                    st.replace("I64", "Bool")
                        .replacen("Bool", "I64", if r.p(50) { 0 } else { 1 });
            }
            6 => {
                // wrap RHS of first '=' in Len( ) or [ ]
                let st = stmts[i].clone();
                if let Some(p) = st.find('=') {
                    if st.ends_with(';') && !st[p..].starts_with("==") {
                        let rhs = &st[p + 1..st.len() - 1];
                        let w = *r.pick(&[
                            ("Len(", ")"),
                            ("[", "]"),
                            ("(", "==c)"),
                            ("(1+", ")"),
                            ("(", "<2)"),
                        ]);
                        stmts[i] = format!("{}={}{}{};", &st[..p], w.0, rhs, w.1);
                    }
                }
            }
            7 => {
                // swap two identifier-like names present in stmt
                let st = stmts[i].clone();
                let names = ["q", "r", "v", "w", "ar", "p", "x", "c"];
                let (n1, n2) = (*r.pick(&names), *r.pick(&names));
                let tmp = st.replace(n1, "\u{1}").replace(n2, n1).replace('\u{1}', n2);
                stmts[i] = tmp;
            }
            8 => {
                stmts.insert(
                    i,
                    format!(
                        "{};",
                        r.pick(&[
                            "Print(q)",
                            "Print(r)",
                            "AssertEq(r,r)",
                            "let zz=1;",
                            "q=[3]",
                            "r=[true]",
                            "v=1",
                            "w=true"
                        ])
                        .trim_end_matches(';')
                    ),
                );
            }
            _ => {
                let st = stmts[i].clone();
                let ops = [
                    ("==", "<"),
                    ("<", "=="),
                    ("+", "=="),
                    ("[0]", "[true]"),
                    ("1", "true"),
                    ("true", "1"),
                    ("then", "then"),
                ];
                let (f, t) = *r.pick(&ops);
                stmts[i] = st.replacen(f, t, 1);
            }
        }
    }
    format!("{head}{}{tail}", stmts.concat())
}

// ---------- K: functions and calls ----------
pub fn fam_k(r: &mut Rng) -> String {
    let nf = 1 + r.below(3);
    let defects = *r.pick(&[0usize, 0, 4, 10]);
    let mut s = String::new();
    let mut sigs: Vec<(String, Vec<Ty>, Option<Ty>)> = vec![];
    for i in 0..nf {
        let np = r.below(3);
        let ps: Vec<Ty> = (0..np).map(|_| *r.pick(&TYS)).collect();
        let rt = if r.p(25) { None } else { Some(*r.pick(&TYS)) };
        let names = ["p", "q"];
        let params: Vec<String> = ps
            .iter()
            .enumerate()
            .map(|(k, t)| format!("{}:view {}", names[k], t.name()))
            .collect();
        let vars: Vec<(String, Ty)> = ps
            .iter()
            .enumerate()
            .map(|(k, t)| (names[k].to_string(), *t))
            .collect();
        let mut g = Gd {
            r,
            vars,
            defects,
            injected: 0,
            emptyp: 0,
        };
        let body = match rt {
            None => "return;".to_string(),
            Some(t) => {
                let v = g.term(t, 2);
                match g.r.below(4) {
                    0 => format!("return {v};"),
                    1 => format!("var z={v};return z;"),
                    2 => format!("if true {{return {v};}} return {v};"),
                    _ => format!("var z={};z={v};return z;", nev(g.r)),
                }
            }
        };
        let rts = rt.map(|t| format!("->{}", t.name())).unwrap_or_default();
        s += &format!("fn G{i}({}){rts}{{{body}}}", params.join(","));
        sigs.push((format!("G{i}"), ps, rt));
    }
    s += &format!("fn Main(){{{BASE_PRE}");
    let mut g = Gd {
        r,
        vars: vec![
            ("c".into(), Ty::B),
            ("x".into(), Ty::I),
            ("ar".into(), Ty::A(1, false)),
            ("br".into(), Ty::A(1, true)),
        ],
        defects,
        injected: 0,
        emptyp: 0,
    };
    for k in 0..1 + g.r.below(6) {
        let (nm, ps, rt) = g.r.pick(&sigs).clone();
        let named = g.r.p(30);
        let args: Vec<String> = ps
            .iter()
            .enumerate()
            .map(|(j, t)| {
                let a = g.term(*t, 2);
                if named {
                    format!("{}: {a}", ["p", "q"][j])
                } else {
                    a
                }
            })
            .collect();
        let call = format!("{nm}({})", args.join(","));
        s += &match (g.r.below(9), rt) {
            (0, _) => format!("{call};"),
            (1, Some(_)) => format!("let r{k}={call};"),
            (2, Some(t)) => {
                let q = g.term(t, 2);
                format!("AssertEq({call},{q});")
            }
            (3, Some(t)) => {
                let q = g.term(t, 2);
                format!("let r{k}=({call}=={q});")
            }
            (4, Some(Ty::I)) => {
                let q = g.term(Ty::I, 2);
                format!("let r{k}=({call}+{q});")
            }
            (5, Some(t)) => format!("let r{k}:{}={call};", t.name()),
            (6, Some(_)) => format!("Print({call});"),
            (7, Some(t)) => {
                let q = g.term(t, 1);
                format!("let r{k}=(if c then {call} else {q});")
            }
            (_, Some(_)) => format!("let r{k}={call};"),
            (_, None) => format!("let r{k}={call};"),
        };
    }
    s + "}"
}

// ---------- D: typed generator with defects ----------
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ty {
    I,
    B,
    A(u8, bool), // depth 1..=2, elem bool?
}
impl Ty {
    fn name(self) -> String {
        match self {
            Ty::I => "I64".into(),
            Ty::B => "Bool".into(),
            Ty::A(1, b) => format!("Array<{}>", if b { "Bool" } else { "I64" }),
            Ty::A(_, b) => format!("Array<Array<{}>>", if b { "Bool" } else { "I64" }),
        }
    }
}
const TYS: [Ty; 6] = [
    Ty::I,
    Ty::B,
    Ty::A(1, false),
    Ty::A(1, true),
    Ty::A(2, false),
    Ty::A(2, true),
];

struct Gd<'a> {
    r: &'a mut Rng,
    vars: Vec<(String, Ty)>,
    defects: usize,
    injected: usize,
    emptyp: usize,
}
impl Gd<'_> {
    fn term(&mut self, t: Ty, d: usize) -> String {
        let mut t = t;
        if self.defects > 0 && self.r.below(100) < self.defects {
            let o = *self.r.pick(&TYS);
            if o != t {
                t = o;
                self.injected += 1;
            }
        }
        self.ok(t, d)
    }
    fn ok(&mut self, t: Ty, d: usize) -> String {
        let vs: Vec<String> = self
            .vars
            .iter()
            .filter(|(_, ty)| *ty == t)
            .map(|(n, _)| n.clone())
            .collect();
        if d == 0 || self.r.p(20) {
            if !vs.is_empty() && self.r.p(50) {
                return self.r.pick(&vs).clone();
            }
            return match t {
                Ty::I => self.r.pick(&["0", "1", "9"]).to_string(),
                Ty::B => self.r.pick(&["true", "false"]).to_string(),
                Ty::A(1, false) => {
                    if self.r.below(100) < self.emptyp {
                        "[]".into()
                    } else {
                        self.r.pick(&["[1]", "[2,3]"]).to_string()
                    }
                }
                Ty::A(1, true) => {
                    if self.r.below(100) < self.emptyp {
                        "[]".into()
                    } else {
                        self.r.pick(&["[true]", "[false,true]"]).to_string()
                    }
                }
                Ty::A(_, false) => {
                    if self.r.below(100) < self.emptyp {
                        "[[]]".into()
                    } else {
                        self.r.pick(&["[[1]]", "[[2],[3]]"]).to_string()
                    }
                }
                Ty::A(_, true) => {
                    if self.r.below(100) < self.emptyp {
                        "[[]]".into()
                    } else {
                        self.r.pick(&["[[true]]", "[[false],[true]]"]).to_string()
                    }
                }
            };
        }
        let a = d - 1;
        match t {
            Ty::I => match self.r.below(6) {
                0 => format!("({}+{})", self.term(Ty::I, a), self.term(Ty::I, a)),
                1 => {
                    let q = *self.r.pick(&TYS[2..]);
                    format!("Len({})", self.term(q, a))
                }
                2 => format!("{}[{}]", self.term(Ty::A(1, false), a), self.term(Ty::I, a)),
                3 => format!(
                    "(if {} then {} else {})",
                    self.term(Ty::B, a),
                    self.term(Ty::I, a),
                    self.term(Ty::I, a)
                ),
                4 => format!(
                    "{}[{}][{}]",
                    self.term(Ty::A(2, false), a),
                    self.term(Ty::I, a),
                    self.term(Ty::I, a)
                ),
                _ => format!(
                    "(if c then {{value:{};}} else {})",
                    self.term(Ty::I, a),
                    self.term(Ty::I, a)
                ),
            },
            Ty::B => match self.r.below(6) {
                0 => format!("({}<{})", self.term(Ty::I, a), self.term(Ty::I, a)),
                1 | 2 => {
                    let q = *self.r.pick(&TYS);
                    format!("({}=={})", self.term(q, a), self.term(q, a))
                }
                3 => format!("{}[{}]", self.term(Ty::A(1, true), a), self.term(Ty::I, a)),
                4 => format!(
                    "(if {} then {} else {})",
                    self.term(Ty::B, a),
                    self.term(Ty::B, a),
                    self.term(Ty::B, a)
                ),
                _ => format!(
                    "{}[{}][{}]",
                    self.term(Ty::A(2, true), a),
                    self.term(Ty::I, a),
                    self.term(Ty::I, a)
                ),
            },
            Ty::A(dp, b) => {
                let elem = if dp == 1 {
                    if b { Ty::B } else { Ty::I }
                } else {
                    Ty::A(1, b)
                };
                match self.r.below(6) {
                    0 | 1 => {
                        let n = 1 + self.r.below(3);
                        let xs: Vec<String> = (0..n).map(|_| self.term(elem, a)).collect();
                        format!("[{}]", xs.join(","))
                    }
                    2 => format!(
                        "(if {} then {} else {})",
                        self.term(Ty::B, a),
                        self.term(t, a),
                        self.term(t, a)
                    ),
                    3 if dp == 1 => {
                        format!("{}[{}]", self.term(Ty::A(2, b), a), self.term(Ty::I, a))
                    }
                    4 => format!(
                        "(if c then {} else {{value:{};}})",
                        self.term(t, a),
                        self.term(t, a)
                    ),
                    _ => self.ok(t, 0),
                }
            }
        }
    }
}

pub fn fam_d(r: &mut Rng) -> (String, usize) {
    let defects = *r.pick(&[0usize, 0, 3, 6, 12]);
    let emptyp = if defects == 0 { 0 } else { 6 };
    let mut g = Gd {
        r,
        vars: vec![
            ("c".into(), Ty::B),
            ("x".into(), Ty::I),
            ("ar".into(), Ty::A(1, false)),
            ("br".into(), Ty::A(1, true)),
        ],
        defects,
        injected: 0,
        emptyp,
    };
    let mut s = format!("fn Main(){{{BASE_PRE}");
    let mut open: Vec<(String, Option<Ty>)> = vec![]; // `var q=[]` bindings whose type is not yet fixed
    for k in 0..4 + g.r.below(8) {
        let t = *g.r.pick(&TYS);
        match g.r.below(10) {
            0 | 1 => {
                let e = g.term(t, 3);
                s += &format!("let b{k}={e};");
                g.vars.push((format!("b{k}"), t));
            }
            2 => {
                let e = g.term(t, 2);
                s += &format!("let b{k}:{}={e};", t.name());
                g.vars.push((format!("b{k}"), t));
            }
            3 | 4 => {
                s += &format!("var o{k}=[];");
                open.push((format!("o{k}"), None));
            }
            5 | 6 if !open.is_empty() => {
                let i = g.r.below(open.len());
                let ty = open[i].1.unwrap_or(match t {
                    Ty::I | Ty::B => Ty::A(1, t == Ty::B),
                    x => x,
                });
                let e = g.term(ty, 3);
                s += &format!("{}={e};", open[i].0);
                if open[i].1.is_none() {
                    open[i].1 = Some(ty);
                    g.vars.push((open[i].0.clone(), ty));
                }
            }
            7 => {
                let q = *g.r.pick(&TYS);
                s += &format!("AssertEq({},{});", g.term(q, 2), g.term(q, 2));
            }
            8 => {
                s += &format!("if {} {{let z{k}={};}}", g.term(Ty::B, 2), g.term(t, 2));
            }
            _ => {
                let e = g.term(t, 3);
                s += &format!("var b{k}={e};");
                g.vars.push((format!("b{k}"), t));
            }
        }
    }
    for (n, ty) in open.iter() {
        if ty.is_none() && g.r.p(85) {
            let tt = *g.r.pick(&TYS[2..]);
            s += &format!("{n}={};", g.ok(tt, 2));
        }
    }
    let inj = g.injected;
    (s + "}", inj)
}

pub fn gen_program(seed: u64, idx: u64, rd: &[String]) -> (char, String) {
    let mut r = Rng::new(seed, idx);
    let (f, s) = match r.below(100) {
        0..=24 => ('U', fam_u(&mut r)),
        25..=38 => ('S', fam_s(&mut r)),
        39..=52 => ('T', fam_t(&mut r)),
        53..=70 => ('R', fam_r(&mut r, rd)),
        71..=82 => ('K', fam_k(&mut r)),
        _ => ('D', fam_d(&mut r).0),
    };
    let s = if r.p(8) {
        s.replace(';', "; ")
    } else if r.p(4) {
        s.replace("fn Main(){", "fn Main() {\n")
    } else {
        s
    };
    (f, s)
}

pub fn rd_sources() -> Vec<String> {
    let t = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/i07c_rd.json"
    ));
    let v: Vec<String> = {
        let mut out = vec![];
        let mut rest = t;
        while let Some(i) = rest.find("\"source\": \"") {
            let b = &rest[i + 11..];
            let mut s = String::new();
            let mut it = b.chars();
            while let Some(ch) = it.next() {
                match ch {
                    '\\' => match it.next() {
                        Some('n') => s.push('\n'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some(o) => s.push(o),
                        None => break,
                    },
                    '"' => break,
                    _ => s.push(ch),
                }
            }
            out.push(s);
            rest = &b[1..];
        }
        out
    };
    v
}

// ---------- W: Result-bearing programs (Ok/Err, annotations, ?, try_add, ==) ----------
pub fn fam_w(seed: u64, idx: u64) -> (char, String) {
    let mut r = Rng::new(seed ^ 0x57, idx);
    let tys = ["I64", "Bool", "Array<I64>"];
    let mut s = String::new();
    // helper functions returning Result
    let nf = r.below(3);
    let mut fns: Vec<(String, String, String)> = vec![];
    for i in 0..nf {
        let a = *r.pick(&tys);
        let b = *r.pick(&tys);
        let val = |t: &str, r: &mut Rng| -> String {
            match t {
                "I64" => r.pick(&["1", "7", "x"]).to_string(),
                "Bool" => r.pick(&["true", "false", "c"]).to_string(),
                _ => r.pick(&["[1]", "ar", "[2,3]"]).to_string(),
            }
        };
        let body = match r.below(4) {
            0 => format!("return Ok({});", val(a, &mut r)),
            1 => format!("return Err({});", val(b, &mut r)),
            2 => format!(
                "if c0 {{return Err({});}} return Ok({});",
                val(b, &mut r),
                val(a, &mut r)
            )
            .replace("c0", "true"),
            _ => format!("return Ok({});", val(*r.pick(&tys), &mut r)),
        };
        let params = "x:view I64,c:view Bool,ar:view Array<I64>";
        s += &format!("fn R{i}({params})->Result<{a},{b}>{{{body}}}");
        fns.push((format!("R{i}"), a.to_string(), b.to_string()));
    }
    s += "fn Main(){let c=true;let x=1;let ar=[1];var q=(if c then {return;} else {return;});";
    let mut rv: Vec<(String, String, String)> = vec![];
    for k in 0..2 + r.below(6) {
        let st = match r.below(12) {
            0 | 1 => {
                let a = *r.pick(&tys);
                let b = *r.pick(&tys);
                let v = match (a, r.below(5)) {
                    ("I64", 0) => "true".to_string(),
                    ("I64", _) => "5".to_string(),
                    ("Bool", 0) => "3".to_string(),
                    ("Bool", _) => "false".to_string(),
                    (_, 0) => "7".to_string(),
                    (_, _) => "[4]".to_string(),
                };
                let side = if r.p(70) {
                    format!("Ok({v})")
                } else {
                    format!(
                        "Err({})",
                        match b {
                            "I64" => "2",
                            "Bool" => "true",
                            _ => "[9]",
                        }
                    )
                };
                rv.push((format!("r{k}"), a.to_string(), b.to_string()));
                format!("let r{k}:Result<{a},{b}>={side};")
            }
            2 if !fns.is_empty() => {
                let (n, a, b) = r.pick(&fns).clone();
                rv.push((format!("r{k}"), a, b));
                format!("let r{k}={n}(x,c,ar);")
            }
            3 => {
                rv.push((format!("r{k}"), "I64".into(), "I64".into()));
                format!(
                    "let r{k}=try_add({},{});",
                    r.pick(&["1", "x", "c", "[1]"]),
                    r.pick(&["2", "x", "true"])
                )
            }
            4 if rv.len() >= 2 => {
                let (a, b) = (r.pick(&rv).clone(), r.pick(&rv).clone());
                format!("let e{k}=({}=={});", a.0, b.0)
            }
            5 if !rv.is_empty() => {
                let a = r.pick(&rv).clone();
                format!(
                    "AssertEq({},{});",
                    a.0,
                    r.pick(&["Ok(1)", "Err(2)", "Ok(true)", "Err(true)", "Ok([1])"])
                )
            }
            6 => format!(
                "let u{k}={};",
                r.pick(&[
                    "Ok(1)",
                    "Err(2)",
                    "Ok(q)",
                    "Err(q)",
                    "[Ok(1),Err(2)]",
                    "[Ok(1),Ok(true)]",
                    "[Err(q)]"
                ])
            ),
            7 => format!(
                "let t{k}:Result<I64,I64>={};",
                r.pick(&["Ok(q)", "Err(q)", "Ok(Len(q))", "Err(Len(q))", "Ok(q[0])"])
            ),
            8 if !rv.is_empty() => {
                let a = r.pick(&rv).clone();
                format!(
                    "let e{k}=({}=={});",
                    a.0,
                    r.pick(&["Ok(1)", "Err(1)", "Ok(q)", "Err(true)"])
                )
            }
            9 => format!(
                "let a{k}:Array<Result<I64,Bool>>=[{}];",
                r.pick(&["Ok(1),Err(true)", "Ok(true)", "Err(1),Err(q)", "Ok(q)", ""])
            ),
            10 => format!("var z{k}=Ok(q);z{k}=Ok(1);"),
            _ => format!("Print({});", r.pick(&["q", "Ok(1)", "Err(q)"])),
        };
        s += &st;
    }
    (' ', s + "}").clone().into_w()
}
trait IntoW {
    fn into_w(self) -> (char, String);
}
impl IntoW for (char, String) {
    fn into_w(self) -> (char, String) {
        ('W', self.1)
    }
}
