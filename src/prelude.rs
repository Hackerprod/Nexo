//! Sealed prelude entries reserved by the F0-0003 profile.

use crate::union::{UnionKind, VariantTag};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingIntrinsic {
    pub identity: &'static str,
    pub spelling: &'static str,
    pub arity: usize,
    pub variant: Option<(UnionKind, VariantTag)>,
}

pub const PENDING_INTRINSICS: [PendingIntrinsic; 3] = [
    PendingIntrinsic {
        identity: "prelude:Some",
        spelling: "Some",
        arity: 1,
        variant: Some((UnionKind::Option, VariantTag(1))),
    },
    PendingIntrinsic {
        identity: "prelude:None",
        spelling: "None",
        arity: 0,
        variant: Some((UnionKind::Option, VariantTag(0))),
    },
    PendingIntrinsic {
        identity: "prelude:Iterator.Next",
        spelling: "Iterator.Next",
        arity: 1,
        variant: None,
    },
];

/// Destinations reserved for F0-0005. Name resolution connects these only when
/// the public profile opens in L01b; historical profiles keep their namespace.
pub const F5_ARRAY_INTRINSICS: [PendingIntrinsic; 3] = [
    PendingIntrinsic {
        identity: "prelude:push",
        spelling: "Push",
        arity: 2,
        variant: None,
    },
    PendingIntrinsic {
        identity: "prelude:pop",
        spelling: "Pop",
        arity: 1,
        variant: None,
    },
    PendingIntrinsic {
        identity: "prelude:join_arrays",
        spelling: "JoinArrays",
        arity: 2,
        variant: None,
    },
];

pub fn pending_intrinsic(spelling: &str) -> Option<&'static PendingIntrinsic> {
    PENDING_INTRINSICS
        .iter()
        .find(|entry| entry.spelling == spelling)
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
