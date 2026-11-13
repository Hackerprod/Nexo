//! Sealed prelude entries reserved by the F0-0003 profile.

use crate::union::{UnionKind, VariantTag};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingIntrinsic {
    pub identity: &'static str,
    pub spelling: &'static str,
    pub scope: &'static str,
    pub arity: usize,
    pub variant: Option<(UnionKind, VariantTag)>,
}

pub const PENDING_INTRINSICS: [PendingIntrinsic; 3] = [
    PendingIntrinsic {
        identity: "prelude:Some",
        spelling: "Some",
        scope: "J02",
        arity: 1,
        variant: Some((UnionKind::Option, VariantTag(1))),
    },
    PendingIntrinsic {
        identity: "prelude:None",
        spelling: "None",
        scope: "J02",
        arity: 0,
        variant: Some((UnionKind::Option, VariantTag(0))),
    },
    PendingIntrinsic {
        identity: "prelude:Iterator.Next",
        spelling: "Iterator.Next",
        scope: "J11",
        arity: 1,
        variant: None,
    },
];

pub fn pending_intrinsic(spelling: &str) -> Option<&'static PendingIntrinsic> {
    PENDING_INTRINSICS
        .iter()
        .find(|entry| entry.spelling == spelling)
}
