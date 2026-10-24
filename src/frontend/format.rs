//! Canonical AST emission with CST comment attachments.
use super::cst::{self, Attachment, Comment, Cst};
use super::emit::{Emission, Role, Writer};
use super::parse;
use super::print;
use crate::diagnostic::Diagnostic;

pub fn format(file: &str, source: &str) -> Result<String, Box<Diagnostic>> {
    #[cfg(test)]
    let _allocation_capture = crate::test_alloc::capture_for(file);
    let cst = cst::parse_cst(file, source)?;
    format_cst(file, &cst)
}

pub(crate) fn worker_diagnostic(
    file: &str,
    source: &str,
    _: print::PrintWorkerUnavailable,
) -> Box<Diagnostic> {
    parse::worker_unavailable(file, source)
}

pub fn format_cst(file: &str, cst: &Cst) -> Result<String, Box<Diagnostic>> {
    let canonical =
        print::print_with_cst(cst).map_err(|error| worker_diagnostic(file, &cst.source, error))?;
    if cst.comments.is_empty() {
        return Ok(canonical.output);
    }
    Ok(with_comments(cst, &canonical.emissions))
}

fn trailing_position(events: &[Emission], position: usize) -> usize {
    let mut after = position + 1;
    while events
        .get(after)
        .is_some_and(|event| event.role == Role::Terminator)
    {
        after += 1;
    }
    after
}

fn leading_position(events: &[Emission], position: usize) -> usize {
    if events[position].role == Role::Terminator {
        trailing_position(events, position)
    } else {
        position
    }
}

struct Anchors {
    exact: Vec<Option<usize>>,
    previous: Vec<Option<usize>>,
    following: Vec<Option<usize>>,
}

impl Anchors {
    fn new(cst: &Cst, events: &[Emission]) -> Self {
        let mut exact = vec![None; cst.tokens.len()];
        for (at, event) in events.iter().enumerate() {
            if let Some(id) = event.source_id {
                // A single emitted token may fuse several source tokens (for
                // example the sign and digits of -1). Only its first source
                // token is an anchor; comments on the rest use the same gap
                // relocation as comments on any other omitted token.
                exact[id] = Some(at);
            }
        }
        let mut previous = vec![None; exact.len()];
        let mut nearest = None;
        for (id, at) in exact.iter().enumerate() {
            previous[id] = nearest;
            if let Some(at) = at {
                nearest = Some(*at);
            }
        }
        let mut following = vec![None; exact.len()];
        let mut nearest = None;
        for (id, at) in exact.iter().enumerate().rev() {
            following[id] = nearest;
            if let Some(at) = at {
                nearest = Some(*at);
            }
        }
        Self {
            exact,
            previous,
            following,
        }
    }
}

enum Position {
    Before(usize),
    Eof,
}

fn position(events: &[Emission], anchors: &Anchors, comment: &Comment) -> Position {
    let (id, trailing) = match comment.attachment {
        Attachment::Previous(id) => (id, true),
        Attachment::Next(id) => (id, false),
        Attachment::Eof => return Position::Eof,
    };
    if let Some(at) = anchors.exact[id] {
        return Position::Before(if trailing {
            trailing_position(events, at)
        } else {
            leading_position(events, at)
        });
    }
    // Relocation is a fixed point of canonical emission. A trailing comment
    // goes after the last emitted event in the omitted token's gap; a standalone
    // comment goes before the first event in that gap. The gap includes inserted
    // parentheses and terminators and depends only on emission order, never on
    // source line positions that formatting may change. On reparse the chosen
    // event is an exact neighbor and the same attachment produces the same slot.
    // A trailing comment is advanced past a mandatory semicolon. If earlier
    // comments already occupy that slot, Writer emits it on its own line while
    // preserving comment order; on reparse both comments select that slot.
    let before = anchors.previous[id];
    let after = anchors.following[id];
    if trailing && let Some(before) = before {
        if let Some(after) = after {
            assert!(after > before, "emitted source anchors remain ordered");
            return Position::Before(trailing_position(events, after - 1));
        }
        return Position::Eof;
    }
    match (before, after) {
        (Some(before), Some(after)) if before + 1 < after => {
            Position::Before(leading_position(events, before + 1))
        }
        (_, Some(after)) => Position::Before(leading_position(events, after)),
        _ => Position::Eof,
    }
}

pub(crate) fn comment_slots(cst: &Cst, events: &[Emission]) -> Vec<Vec<(usize, bool)>> {
    let anchors = Anchors::new(cst, events);
    let mut comments_at = vec![Vec::new(); events.len() + 1];
    let mut last = 0;
    for (order, comment) in cst.comments.iter().enumerate() {
        let calculated = match position(events, &anchors, comment) {
            Position::Before(at) => at,
            Position::Eof => events.len(),
        };
        // ADR §5.2: anchors are monotone in source order. An omitted token can
        // put a trailing comment after inserted punctuation while the next
        // standalone comment selects the start of that same emission gap.
        // Taking the prefix maximum here constructs the final emission order;
        // the invariant below checks the result rather than repairing it.
        let at = calculated.max(last);
        assert!(
            at >= last,
            "comment anchors must preserve source order: {:?} at {at} after {last}",
            &cst.source[comment.start..comment.end]
        );
        let shares_anchor = !comments_at[at].is_empty();
        comments_at[at].push((order, shares_anchor));
        last = at;
    }
    comments_at
}

fn with_comments(cst: &Cst, events: &[Emission]) -> String {
    let comments_at = comment_slots(cst, events);
    let mut writer = Writer::default();
    for (at, event) in events.iter().enumerate() {
        for &(order, shares_anchor) in &comments_at[at] {
            let comment = &cst.comments[order];
            let text = cst.source[comment.start..comment.end]
                .trim_end_matches(super::lex::is_horizontal_space);
            let after = !shares_anchor && matches!(comment.attachment, Attachment::Previous(_));
            writer.comment(text, after);
        }
        writer.token(&event.text, event.role, event.structural_end);
    }
    for &(order, shares_anchor) in &comments_at[events.len()] {
        let comment = &cst.comments[order];
        let text = cst.source[comment.start..comment.end]
            .trim_end_matches(super::lex::is_horizontal_space);
        writer.comment(
            text,
            !shares_anchor && matches!(comment.attachment, Attachment::Previous(_)),
        );
    }
    writer.line();
    writer.output
}
