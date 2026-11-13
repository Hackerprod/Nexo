//! Symbolic coverage for the unary nominal variants admitted by F0-0003.

use super::types::{Type, concrete_union};
use crate::frontend::parse::Pattern;
use crate::union::UnionKind;

fn total(ty: &Type, pattern: &Pattern) -> bool {
    matches!(pattern, Pattern::Wildcard(_))
        || matches!(pattern, Pattern::Name { name, .. } if !(name == "None" && matches!(ty, Type::Union(UnionKind::Option, ..))))
}

fn variant_arg<'a>(ty: &Type, pattern: &'a Pattern, name: &str) -> Option<Option<&'a Pattern>> {
    if total(ty, pattern) {
        return None;
    }
    match pattern {
        Pattern::Name { name: written, .. } if written == name && name == "None" => Some(None),
        Pattern::Constructor {
            name: written,
            args,
            ..
        } if written == name => Some(args.first()),
        _ => None,
    }
}

pub(super) fn missing(ty: &Type, patterns: &[&Pattern]) -> Option<String> {
    if patterns.iter().any(|pattern| total(ty, pattern)) {
        return None;
    }
    match ty {
        Type::Bool => {
            for (value, text) in [(false, "false"), (true, "true")] {
                if !patterns
                    .iter()
                    .any(|pattern| matches!(pattern, Pattern::Bool(found, _) if *found == value))
                {
                    return Some(text.into());
                }
            }
            None
        }
        Type::I64 => {
            for index in 0..=patterns.len() {
                let value = if index == 0 {
                    0
                } else if index % 2 == 1 {
                    i64::try_from(index.div_ceil(2)).ok()?
                } else {
                    -i64::try_from(index / 2).ok()?
                };
                if !patterns.iter().any(|pattern| matches!(pattern, Pattern::Integer(written, _) if written.parse::<i64>().ok() == Some(value))) {
                    return Some(value.to_string());
                }
            }
            None
        }
        _ => {
            let Some(descriptor) = concrete_union(ty) else {
                return Some("_".into());
            };
            for variant in descriptor.variants {
                let selected: Vec<&Pattern> = patterns
                    .iter()
                    .filter_map(|pattern| variant_arg(ty, pattern, variant.name).flatten())
                    .collect();
                if let Some(payload) = variant.payload {
                    if let Some(witness) = missing(payload, &selected) {
                        return Some(format!("{}({witness})", variant.name));
                    }
                } else if !patterns
                    .iter()
                    .any(|pattern| variant_arg(ty, pattern, variant.name).is_some())
                {
                    return Some(variant.name.into());
                }
            }
            None
        }
    }
}

pub(super) fn subsumed(ty: &Type, target: &Pattern, previous: &[&Pattern]) -> bool {
    if previous.iter().any(|pattern| total(ty, pattern)) {
        return true;
    }
    if total(ty, target) {
        return missing(ty, previous).is_none();
    }
    match target {
        Pattern::Bool(value, _) => previous
            .iter()
            .any(|pattern| matches!(pattern, Pattern::Bool(found, _) if found == value)),
        Pattern::Integer(value, _) => previous
            .iter()
            .any(|pattern| matches!(pattern, Pattern::Integer(found, _) if found == value)),
        _ => {
            let Some(descriptor) = concrete_union(ty) else {
                return false;
            };
            for variant in descriptor.variants {
                let Some(target_arg) = variant_arg(ty, target, variant.name) else {
                    continue;
                };
                if let Some(payload) = variant.payload {
                    let Some(target_arg) = target_arg else {
                        return false;
                    };
                    let selected: Vec<&Pattern> = previous
                        .iter()
                        .filter_map(|pattern| variant_arg(ty, pattern, variant.name).flatten())
                        .collect();
                    return subsumed(payload, target_arg, &selected);
                }
                return previous
                    .iter()
                    .any(|pattern| variant_arg(ty, pattern, variant.name).is_some());
            }
            false
        }
    }
}
