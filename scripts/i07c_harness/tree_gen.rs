// Expression trees with explicit group wrappers and per-node span tables.
use crate::common::*;

#[derive(Clone, Debug)]
pub enum K {
    Int(u32),
    Bool(bool),
    Name(&'static str),
    Arr(Vec<N>),
    Bin(&'static str, Box<N>, Box<N>),
    If(Box<N>, Box<N>, Box<N>),
    Blk(Box<N>),
    Idx(Box<N>, Box<N>),
    Len(Box<N>),
    Dv,
}
#[derive(Clone, Debug)]
pub struct N {
    pub id: usize,
    pub k: K,
    pub mand: u8,
    pub extra: u8,
}
#[derive(Clone, Debug)]
pub struct Site {
    pub id: usize,
    pub core: (u64, u64),
    pub wraps: Vec<(u64, u64)>,
    pub op: Option<(u64, u64)>,
}

pub struct B {
    pub out: String,
    pub sites: Vec<Site>,
    pub pseudo: Vec<(String, (u64, u64))>,
    pub grouped: bool,
}
impl B {
    pub fn new(grouped: bool) -> B {
        B { out: String::new(), sites: Vec::new(), pseudo: Vec::new(), grouped }
    }
    pub fn t(&mut self, s: &str) {
        self.out.push_str(s);
    }
    pub fn pseudo_text(&mut self, name: &str, s: &str) {
        let a = self.out.len() as u64;
        self.out.push_str(s);
        self.pseudo.push((name.to_string(), (a, a + s.len() as u64)));
    }
    pub fn expr(&mut self, n: &N) {
        let wraps = n.mand as usize + if self.grouped && !matches!(n.k, K::Blk(_)) { n.extra as usize } else { 0 };
        for _ in 0..wraps {
            self.out.push('(');
        }
        let cs = self.out.len() as u64;
        let mut op = None;
        match &n.k {
            K::Int(v) => self.out.push_str(&v.to_string()),
            K::Bool(b) => self.out.push_str(if *b { "true" } else { "false" }),
            K::Name(s) => self.out.push_str(s),
            K::Arr(v) => {
                self.out.push('[');
                for (i, e) in v.iter().enumerate() {
                    if i > 0 {
                        self.out.push(',');
                    }
                    self.expr(e);
                }
                self.out.push(']');
            }
            K::Bin(o, l, r) => {
                self.expr(l);
                let a = self.out.len() as u64;
                self.out.push_str(o);
                op = Some((a, a + o.len() as u64));
                self.expr(r);
            }
            K::If(c, t, e) => {
                self.out.push_str("if ");
                self.expr(c);
                self.out.push_str(" then ");
                self.expr(t);
                self.out.push_str(" else ");
                self.expr(e);
            }
            K::Blk(e) => {
                self.out.push_str("{value:");
                self.expr(e);
                self.out.push_str(";}");
            }
            K::Idx(b, i) => {
                self.expr(b);
                self.out.push('[');
                self.expr(i);
                self.out.push(']');
            }
            K::Len(e) => {
                self.out.push_str("Len(");
                self.expr(e);
                self.out.push(')');
            }
            K::Dv => self.out.push_str("if true then {return;} else {return;}"),
        }
        let ce = self.out.len() as u64;
        for _ in 0..wraps {
            self.out.push(')');
        }
        let ws = (1..=wraps as u64).map(|k| (cs - k, ce + k)).collect();
        self.sites.push(Site { id: n.id, core: (cs, ce), wraps: ws, op });
    }
}

pub struct Ids(pub usize);
impl Ids {
    pub fn n(&mut self, k: K) -> N {
        self.0 += 1;
        N { id: self.0, k, mand: 0, extra: 0 }
    }
}

pub fn needs_paren_child(k: &K) -> bool {
    matches!(k, K::Bin(..) | K::If(..) | K::Dv)
}

/// Random raw tree (types not enforced).
pub struct Gen<'a> {
    pub r: &'a mut Rng,
    pub ids: Ids,
    pub names: Vec<&'static str>,
    pub allow_extra: bool,
}
impl<'a> Gen<'a> {
    fn finish(&mut self, mut n: N) -> N {
        if matches!(n.k, K::If(..) | K::Dv) {
            n.mand = 1;
        }
        n.extra = if !self.allow_extra {
            0
        } else {
            match self.r.below(10) {
                0..=6 => 0,
                7..=8 => 1,
                _ => 2,
            }
        };
        n
    }
    fn child_of_bin(&mut self, depth: u32) -> N {
        let mut c = self.expr(depth);
        if matches!(c.k, K::Bin(..)) {
            c.mand = 1;
        }
        c
    }
    pub fn leaf(&mut self) -> N {
        let k = match self.r.below(5) {
            0 => K::Int(self.r.below(300) as u32),
            1 => K::Bool(self.r.chance(1, 2)),
            _ => K::Name(self.r.pick(&self.names.clone())),
        };
        let n = self.ids.n(k);
        self.finish(n)
    }
    pub fn expr(&mut self, depth: u32) -> N {
        if depth == 0 || self.r.chance(2, 7) {
            return self.leaf();
        }
        let d = depth - 1;
        let k = match self.r.below(9) {
            0 | 1 => {
                let n = 1 + self.r.below(3) as usize;
                K::Arr((0..n).map(|_| self.expr(d)).collect())
            }
            2 | 3 => {
                let op = *self.r.pick(&["+", "-", "<", "=="]);
                let l = self.child_of_bin(d);
                let r = self.child_of_bin(d);
                K::Bin(op, Box::new(l), Box::new(r))
            }
            4 | 5 => {
                let c = if self.r.chance(3, 4) {
                    let n = self.ids.n(K::Bool(true));
                    self.finish(n)
                } else {
                    self.expr(d)
                };
                let mut br = |g: &mut Self| {
                    if g.r.chance(1, 3) {
                        let v = g.expr(d);
                        let b = g.ids.n(K::Blk(Box::new(v)));
                        g.finish(b)
                    } else {
                        g.expr(d)
                    }
                };
                let t = br(self);
                let e = br(self);
                K::If(Box::new(c), Box::new(t), Box::new(e))
            }
            6 => {
                let mut b = self.expr(d);
                if needs_paren_child(&b.k) {
                    b.mand = b.mand.max(1);
                }
                let i = self.expr(d);
                K::Idx(Box::new(b), Box::new(i))
            }
            7 => K::Len(Box::new(self.expr(d))),
            _ => K::Dv,
        };
        let n = self.ids.n(k);
        self.finish(n)
    }
}

pub fn count_nodes(n: &N) -> usize {
    1 + match &n.k {
        K::Arr(v) => v.iter().map(count_nodes).sum(),
        K::Bin(_, a, b) | K::Idx(a, b) => count_nodes(a) + count_nodes(b),
        K::If(a, b, c) => count_nodes(a) + count_nodes(b) + count_nodes(c),
        K::Blk(a) | K::Len(a) => count_nodes(a),
        _ => 0,
    }
}
