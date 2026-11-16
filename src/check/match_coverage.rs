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
    struct Frame<'a> {
        ty: &'a Type,
        patterns: Vec<&'a Pattern>,
        variants: Option<crate::union::UnionDescriptor<&'a Type>>,
        next: usize,
        prefix: Option<&'static str>,
    }
    let mut frames = vec![Frame {
        ty,
        patterns: patterns.to_vec(),
        variants: None,
        next: 0,
        prefix: None,
    }];
    let mut result = None;
    while let Some(frame) = frames.last_mut() {
        if let Some(descriptor) = &frame.variants {
            if let Some(witness) = result.take() {
                result = Some(format!("{}({witness})", frame.prefix.take().unwrap()));
                frames.pop();
                continue;
            }
            if frame.next == descriptor.variants.len() {
                frames.pop();
                continue;
            }
            let variant = &descriptor.variants[frame.next];
            frame.next += 1;
            if let Some(payload) = variant.payload {
                let selected = frame
                    .patterns
                    .iter()
                    .filter_map(|pattern| variant_arg(frame.ty, pattern, variant.name).flatten())
                    .collect();
                frame.prefix = Some(variant.name);
                frames.push(Frame {
                    ty: payload,
                    patterns: selected,
                    variants: None,
                    next: 0,
                    prefix: None,
                });
                continue;
            }
            if !frame
                .patterns
                .iter()
                .any(|pattern| variant_arg(frame.ty, pattern, variant.name).is_some())
            {
                result = Some(variant.name.into());
                frames.pop();
            }
            continue;
        }
        if frame
            .patterns
            .iter()
            .any(|pattern| total(frame.ty, pattern))
        {
            frames.pop();
            continue;
        }
        result = match frame.ty {
            Type::Bool => {
                [(false, "false"), (true, "true")]
                    .into_iter()
                    .find_map(|(value, text)| {
                        (!frame.patterns.iter().any(
                            |pattern| matches!(pattern, Pattern::Bool(found, _) if *found == value),
                        ))
                        .then(|| text.into())
                    })
            }
            Type::I64 => {
                let mut witness = None;
                for index in 0..=frame.patterns.len() {
                    let value = if index == 0 {
                        0
                    } else if index % 2 == 1 {
                        i64::try_from(index.div_ceil(2)).ok()?
                    } else {
                        -i64::try_from(index / 2).ok()?
                    };
                    if !frame.patterns.iter().any(|pattern| matches!(pattern, Pattern::Integer(written, _) if written.parse::<i64>().ok() == Some(value))) {
                    witness = Some(value.to_string());
                    break;
                }
                }
                witness
            }
            _ => {
                if let Some(descriptor) = concrete_union(frame.ty) {
                    frame.variants = Some(descriptor);
                    continue;
                }
                Some("_".into())
            }
        };
        frames.pop();
    }
    result
}

pub(super) fn subsumed(ty: &Type, target: &Pattern, previous: &[&Pattern]) -> bool {
    let mut ty = ty;
    let mut target = target;
    let mut previous = previous.to_vec();
    'walk: loop {
        if previous.iter().any(|pattern| total(ty, pattern)) {
            return true;
        }
        if total(ty, target) {
            return missing(ty, &previous).is_none();
        }
        return match target {
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
                        ty = payload;
                        target = target_arg;
                        previous = selected;
                        continue 'walk;
                    }
                    return previous
                        .iter()
                        .any(|pattern| variant_arg(ty, pattern, variant.name).is_some());
                }
                false
            }
        };
    }
}

#[cfg(test)]
mod j16_direct_coverage_stack {
    use super::*;
    use crate::diagnostic::Span;
    use std::{process::Command, sync::Arc};

    #[test]
    fn child() {
        let Ok(phase) = std::env::var("J16_DIRECT_COVERAGE") else {
            return;
        };
        let span = Span {
            file: "pattern.n".into(),
            start: 0,
            end: 1,
        };
        let mut ty = Type::I64;
        for _ in 0..126 {
            ty = Type::Union(UnionKind::Option, Arc::new(ty), None);
        }
        let mut patterns = Vec::new();
        for depth in 0..126 {
            let mut pattern = Pattern::Name {
                name: "None".into(),
                mode: None,
                span: span.clone(),
            };
            for _ in 0..depth {
                pattern = Pattern::Constructor {
                    name: "Some".into(),
                    args: vec![pattern],
                    span: span.clone(),
                };
            }
            patterns.push(pattern);
        }
        let patterns = Arc::new(patterns);
        let ty = Arc::new(ty);
        let patterns_worker = Arc::clone(&patterns);
        let ty_worker = Arc::clone(&ty);
        std::thread::Builder::new()
            .stack_size(204 * 1024)
            .spawn(move || {
                // Reserve room for the checker frames that precede coverage.
                let prior = [0u8; 64 * 1024];
                let previous: Vec<&Pattern> = patterns_worker.iter().collect();
                if phase == "missing" {
                    assert_eq!(
                        missing(&ty_worker, &previous),
                        Some(format!("{}0{}", "Some(".repeat(126), ")".repeat(126)))
                    );
                } else {
                    assert!(!subsumed(&ty_worker, &Pattern::Wildcard(span), &previous));
                }
                std::hint::black_box(&prior);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn parent() {
        for phase in ["missing", "subsumed"] {
            let output = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "check::match_coverage::j16_direct_coverage_stack::child",
                ])
                .env("J16_DIRECT_COVERAGE", phase)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{phase}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
