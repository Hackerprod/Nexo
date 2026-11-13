//! Nominal union metadata shared by inference, IR and execution.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnionKind {
    Result,
    Option,
}

impl UnionKind {
    pub fn variant_tag(self, name: &str) -> Option<VariantTag> {
        match (self, name) {
            (Self::Result, "Ok") => Some(VariantTag(0)),
            (Self::Result, "Err") => Some(VariantTag(1)),
            (Self::Option, "None") => Some(VariantTag(0)),
            (Self::Option, "Some") => Some(VariantTag(1)),
            _ => None,
        }
    }

    pub fn variant_name(self, tag: VariantTag) -> Option<&'static str> {
        match (self, tag.0) {
            (Self::Result, 0) => Some("Ok"),
            (Self::Result, 1) => Some("Err"),
            (Self::Option, 0) => Some("None"),
            (Self::Option, 1) => Some("Some"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VariantTag(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variant<T: Copy> {
    pub name: &'static str,
    pub tag: VariantTag,
    pub payload: Option<T>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnionDescriptor<T: Copy> {
    pub kind: UnionKind,
    pub variants: [Variant<T>; 2],
}

impl<T: Copy> UnionDescriptor<T> {
    pub const fn result(ok: T, err: T) -> Self {
        Self {
            kind: UnionKind::Result,
            variants: [
                Variant {
                    name: "Ok",
                    tag: VariantTag(0),
                    payload: Some(ok),
                },
                Variant {
                    name: "Err",
                    tag: VariantTag(1),
                    payload: Some(err),
                },
            ],
        }
    }

    pub const fn option(some: T) -> Self {
        Self {
            kind: UnionKind::Option,
            variants: [
                Variant {
                    name: "None",
                    tag: VariantTag(0),
                    payload: None,
                },
                Variant {
                    name: "Some",
                    tag: VariantTag(1),
                    payload: Some(some),
                },
            ],
        }
    }

    pub fn variant(&self, tag: VariantTag) -> Option<&Variant<T>> {
        self.variants.iter().find(|variant| variant.tag == tag)
    }

    pub fn result_payloads(&self) -> Option<(T, T)> {
        (self.kind == UnionKind::Result).then(|| {
            (
                self.variants[0].payload.unwrap(),
                self.variants[1].payload.unwrap(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{UnionDescriptor, UnionKind, VariantTag};

    #[test]
    fn nominal_variants_keep_the_published_tags_and_payload_paths() {
        let result = UnionDescriptor::result(11, 22);
        assert_eq!(result.kind, UnionKind::Result);
        assert_eq!(result.variants[0].tag, VariantTag(0));
        assert_eq!(result.variants[0].name, "Ok");
        assert_eq!(result.variants[0].payload, Some(11));
        assert_eq!(result.variants[1].tag, VariantTag(1));
        assert_eq!(result.variants[1].name, "Err");
        assert_eq!(result.variants[1].payload, Some(22));

        let option = UnionDescriptor::option(33);
        assert_eq!(option.kind, UnionKind::Option);
        assert_eq!(option.variants[0].tag, VariantTag(0));
        assert_eq!(option.variants[0].name, "None");
        assert_eq!(option.variants[0].payload, None);
        assert_eq!(option.variants[1].tag, VariantTag(1));
        assert_eq!(option.variants[1].name, "Some");
        assert_eq!(option.variants[1].payload, Some(33));
    }
}
