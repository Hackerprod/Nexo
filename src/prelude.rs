//! Sealed prelude entries reserved by the F0-0003 profile.

use crate::union::{UnionKind, VariantTag};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingIntrinsic {
    pub identity: &'static str,
    pub spelling: &'static str,
    pub arity: usize,
    pub variant: Option<(UnionKind, VariantTag)>,
    pub inout_first: bool,
}

pub const PENDING_INTRINSICS: [PendingIntrinsic; 3] = [
    PendingIntrinsic {
        identity: "prelude:Some",
        spelling: "Some",
        arity: 1,
        variant: Some((UnionKind::Option, VariantTag(1))),
        inout_first: false,
    },
    PendingIntrinsic {
        identity: "prelude:None",
        spelling: "None",
        arity: 0,
        variant: Some((UnionKind::Option, VariantTag(0))),
        inout_first: false,
    },
    PendingIntrinsic {
        identity: "prelude:Iterator.Next",
        spelling: "Iterator.Next",
        arity: 1,
        variant: None,
        inout_first: true,
    },
];

/// Destinations reserved for F0-0005. Name resolution connects these only when
/// the public profile opens in L01b; historical profiles keep their namespace.
const F5_ARRAY_INTRINSICS: [PendingIntrinsic; 3] = [
    PendingIntrinsic {
        identity: "prelude:push",
        spelling: "Push",
        arity: 2,
        variant: None,
        inout_first: true,
    },
    PendingIntrinsic {
        identity: "prelude:pop",
        spelling: "Pop",
        arity: 1,
        variant: None,
        inout_first: true,
    },
    PendingIntrinsic {
        identity: "prelude:join_arrays",
        spelling: "JoinArrays",
        arity: 2,
        variant: None,
        inout_first: false,
    },
];

pub fn pending_intrinsic(spelling: &str) -> Option<&'static PendingIntrinsic> {
    PENDING_INTRINSICS
        .iter()
        .find(|entry| entry.spelling == spelling)
}

pub fn f5_array_intrinsic(spelling: &str) -> Option<&'static PendingIntrinsic> {
    F5_ARRAY_INTRINSICS
        .iter()
        .find(|entry| entry.spelling == spelling || entry.identity == spelling)
}

#[cfg(test)]
#[test]
fn f5_array_destinations_are_reserved_without_opening_the_j_prelude() {
    assert_eq!(
        F5_ARRAY_INTRINSICS.map(|entry| entry.identity),
        ["prelude:push", "prelude:pop", "prelude:join_arrays",]
    );
    for entry in F5_ARRAY_INTRINSICS {
        assert!(pending_intrinsic(entry.spelling).is_none());
    }
}
