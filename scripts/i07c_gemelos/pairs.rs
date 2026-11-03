// Deterministic enumeration of leaf, shape, context, and deferred resolutions.
use crate::ast::*;

pub const NLEAF: u64 = 22;
pub const NSHAPE: u64 = 12;
pub const NCTX: u64 = 18;

fn pleaf(id: &mut u32, k: u64) -> N {
    let m = |id: &mut u32, k: K| n(id, k);
    match k {
        0 => m(id, K::Int(1)),
        1 => m(id, K::Bool(true)),
        2 => m(id, K::Var(V::X)),
        3 => m(id, K::Var(V::P)),
        4 => m(id, K::Var(V::Ar)),
        5 => m(id, K::Var(V::Br)),
        6 => m(id, K::Var(V::Aar)),
        7 => m(id, K::Var(V::R)),
        8 => m(id, K::Var(V::Q)),
        9 => m(id, K::Dv),
        10 => {
            let a = m(id, K::Int(1));
            m(id, K::Grp(Box::new(a)))
        }
        11 => {
            let a = m(id, K::Bool(true));
            m(id, K::Grp(Box::new(a)))
        }
        12 => {
            let a = m(id, K::Var(V::R));
            m(id, K::Grp(Box::new(a)))
        }
        13 => {
            let a = m(id, K::Var(V::Q));
            m(id, K::Grp(Box::new(a)))
        }
        14 => {
            let a = m(id, K::Int(1));
            m(id, K::Arr(vec![a]))
        }
        15 => {
            let a = m(id, K::Bool(true));
            m(id, K::Arr(vec![a]))
        }
        16 => {
            let a = m(id, K::Bool(true));
            let b = m(id, K::Int(1));
            let s = m(id, K::Bin(Op::Add, Box::new(a), Box::new(b)));
            m(id, K::Grp(Box::new(s)))
        }
        17 => {
            let a = m(id, K::Var(V::Ar));
            let z = m(id, K::Int(0));
            m(id, K::Idx(Box::new(a), Box::new(z)))
        }
        18 => {
            let a = m(id, K::Var(V::R));
            m(id, K::Arr(vec![a]))
        }
        19 => {
            let c = m(id, K::Var(V::P));
            let a = m(id, K::Var(V::R));
            let b = m(id, K::Var(V::X));
            let i = m(id, K::If(Box::new(c), Box::new(a), Box::new(b)));
            m(id, K::Grp(Box::new(i)))
        }
        20 => {
            let c = m(id, K::Var(V::P));
            let a = m(id, K::Var(V::Q));
            let b = m(id, K::Bool(true));
            let i = m(id, K::If(Box::new(c), Box::new(a), Box::new(b)));
            m(id, K::Grp(Box::new(i)))
        }
        _ => {
            let a = m(id, K::Var(V::Aar));
            let z = m(id, K::Int(0));
            m(id, K::Idx(Box::new(a), Box::new(z)))
        }
    }
}
fn grp(id: &mut u32, x: N) -> N {
    n(id, K::Grp(Box::new(x)))
}
fn mkif(id: &mut u32, a: N, b: N) -> N {
    let c = n(id, K::Var(V::P));
    let i = n(id, K::If(Box::new(c), Box::new(a), Box::new(b)));
    grp(id, i)
}
pub fn pair_prog(mut i: u64) -> Prog {
    let nl = NLEAF;
    let (fx, fy) = (i % nl, (i / nl) % nl);
    i /= nl * nl;
    let shape = i % NSHAPE;
    i /= NSHAPE;
    let ctx = i % NCTX;
    i /= NCTX;
    let ft_r = [arr(T::B), arr(T::I), arr(arr(T::I)), arr(arr(T::B))][(i % 4) as usize].clone();
    i /= 4;
    let ft_q = [T::I, T::B, arr(T::I), arr(T::B), arr(arr(T::I))][(i % 5) as usize].clone();
    let mut idc = 0u32;
    let id = &mut idc;
    let x = pleaf(id, fx);
    let y = pleaf(id, fy);
    let x2 = pleaf(id, fx);
    let y2 = pleaf(id, fy);
    let e = match shape {
        0 => mkif(id, x, y),
        1 => n(id, K::Arr(vec![x, y])),
        2 => n(id, K::Bin(Op::Eq, Box::new(x), Box::new(y))),
        3 => n(id, K::Bin(Op::Add, Box::new(x), Box::new(y))),
        4 => n(id, K::Bin(Op::Lt, Box::new(x), Box::new(y))),
        5 => {
            let inner = mkif(id, x, y);
            mkif(id, inner, y2)
        }
        6 => {
            let inner = mkif(id, y, x2);
            mkif(id, x, inner)
        }
        7 => {
            let a = n(id, K::Blk(Box::new(x)));
            let b = n(id, K::Blk(Box::new(y)));
            mkif(id, a, b)
        }
        8 => {
            let inner = mkif(id, x, x2);
            mkif(id, inner, y)
        }
        9 => {
            let i2 = mkif(id, x, y);
            n(id, K::Arr(vec![i2]))
        }
        10 => {
            let i2 = mkif(id, x, y);
            n(id, K::Bin(Op::Eq, Box::new(i2), Box::new(x2)))
        }
        _ => {
            let i2 = mkif(id, x, y);
            n(id, K::Arr(vec![x2, i2]))
        }
    };
    let _ = y2;
    let ts = [T::I, T::B, arr(T::I), arr(T::B), arr(arr(T::I))];
    let stmt = match ctx {
        0 => S::Let(None, e),
        1..=5 => S::Let(Some(ts[(ctx - 1) as usize].clone()), e),
        6 => S::Assign(T::I, e),
        7 => S::Assign(T::B, e),
        8 => S::Assign(arr(T::I), e),
        9 => S::Call(T::B, e),
        10 => S::Call(arr(T::I), e),
        11 => S::Cond(e),
        12 => {
            let o = n(id, K::Int(1));
            S::Assert(e, o)
        }
        13 => {
            let o = n(id, K::Int(1));
            S::Assert(o, e)
        }
        14 => S::IdxAssign(T::I, e),
        15 => S::IdxAssign(T::B, e),
        16 => {
            let o = n(id, K::Var(V::Ar));
            let g = grp(id, e);
            let ix = n(id, K::Idx(Box::new(o), Box::new(g)));
            S::Let(None, ix)
        }
        _ => {
            let o = n(id, K::Int(1));
            let g = grp(id, e);
            let b = n(id, K::Bin(Op::Add, Box::new(o), Box::new(g)));
            S::Let(None, b)
        }
    };
    let mut p = Prog { stmts: vec![stmt], ft_r: Some(ft_r), ft_q: Some(ft_q) };
    fix_ids(&mut p);
    p
}
pub fn pair_total() -> u64 {
    NLEAF * NLEAF * NSHAPE * NCTX * 4 * 5
}
