//! Canonical layout of structurally classified syntax tokens.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Role {
    #[default]
    Word,
    Function,
    Else,
    Open,
    AttachedOpen,
    Close,
    Comma,
    Colon,
    Terminator,
    BlockOpen,
    BlockClose,
    ContractOpen,
    ContractClose,
    Binary,
    Arrow,
    Suffix,
    TypeOpen,
    TypeClose,
}

#[derive(Default)]
pub(crate) struct Writer {
    pub output: String,
    depth: usize,
    previous: Option<Role>,
    previous_structural_end: bool,
    pub(crate) emissions: Vec<Emission>,
    source_tokens: Vec<SourceToken>,
    comment_spans: Vec<(usize, usize)>,
    group_ends: Vec<usize>,
    #[cfg(test)]
    track_comment_work: bool,
}

#[cfg(test)]
static COMMENT_SEARCH_STEPS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn reset_comment_search_steps() {
    COMMENT_SEARCH_STEPS.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn comment_search_steps() -> usize {
    COMMENT_SEARCH_STEPS.load(std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone)]
pub(crate) struct Emission {
    pub text: String,
    pub role: Role,
    pub structural_end: bool,
    pub source_id: Option<usize>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub source_end: usize,
}

pub(crate) struct SourceToken {
    pub start: usize,
    pub end: usize,
    pub id: usize,
    pub text: String,
}

impl Writer {
    pub(crate) fn with_source(
        source_tokens: Vec<SourceToken>,
        comment_spans: Vec<(usize, usize)>,
        #[cfg(test)] file: &str,
    ) -> Self {
        Self {
            source_tokens,
            comment_spans,
            #[cfg(test)]
            track_comment_work: file == "comment_search_scale.n",
            ..Self::default()
        }
    }

    pub(crate) fn has_comment(&self, start: usize, end: usize) -> bool {
        let first = self.comment_spans.partition_point(|&(at, _)| {
            #[cfg(test)]
            if self.track_comment_work {
                COMMENT_SEARCH_STEPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            at < start
        });
        self.comment_spans
            .get(first)
            .is_some_and(|&(at, _)| at < end)
    }

    pub(crate) fn has_comment_before(&self, start: usize) -> bool {
        let at = self
            .source_tokens
            .partition_point(|token| token.start < start);
        let from = at.checked_sub(1).map_or(0, |id| self.source_tokens[id].end);
        // Starting at the preceding token's start would find the same comments:
        // lexer trivia cannot begin inside that token. Its end states the gap.
        self.has_comment(from, start)
    }

    pub(crate) fn group_start(&mut self, end: usize) {
        self.group_ends.push(end);
    }

    pub(crate) fn group_end(&mut self) {
        self.group_ends.pop();
    }

    pub(crate) fn has_comment_after_in_group(&self, end: usize) -> bool {
        self.group_ends
            .last()
            .is_some_and(|&group_end| self.has_comment(end, group_end))
    }

    pub(crate) fn token_after(&mut self, literal: &str, role: Role, bound: usize) -> usize {
        if self.source_tokens.is_empty() {
            self.token(literal, role, false);
            return bound;
        }
        let id = self
            .source_tokens
            .partition_point(|token| token.start < bound);
        let token = self
            .source_tokens
            .get(id)
            .expect("source token after AST boundary");
        let (start, end) = (token.start, token.end);
        self.token_at(literal, role, false, start, end);
        end
    }

    pub(crate) fn token_before(&mut self, literal: &str, role: Role, bound: usize) -> usize {
        if self.source_tokens.is_empty() {
            self.token(literal, role, false);
            return bound;
        }
        let id = self
            .source_tokens
            .partition_point(|token| token.end <= bound);
        let token = id
            .checked_sub(1)
            .and_then(|id| self.source_tokens.get(id))
            .expect("source token before AST boundary");
        let (start, end) = (token.start, token.end);
        self.token_at(literal, role, false, start, end);
        end
    }

    pub(crate) fn terminator(&mut self, end: usize) {
        // A statement span ends at its written semicolon, if present. Its
        // identity is independent of syntax discarded earlier in the tree.
        let start = end.saturating_sub(1);
        if self
            .source_tokens
            .binary_search_by_key(&start, |token| token.start)
            .ok()
            .is_some_and(|id| {
                self.source_tokens[id].end == end && self.source_tokens[id].text == ";"
            })
        {
            self.token_at(";", Role::Terminator, false, start, end);
        } else {
            self.token(";", Role::Terminator, false);
        }
    }

    pub fn line(&mut self) {
        while self.output.ends_with(' ') {
            self.output.pop();
        }
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn indent(&mut self) {
        if self.output.is_empty() || self.output.ends_with('\n') {
            self.output.push_str(&"    ".repeat(self.depth));
        }
    }

    pub fn comment(&mut self, literal: &str, after: bool) {
        if after && !self.output.ends_with('\n') && !self.output.is_empty() {
            self.output.push(' ');
        } else {
            self.line();
            self.indent();
        }
        self.output.push_str(literal);
        self.line();
    }

    pub fn token(&mut self, literal: &str, role: Role, structural_end: bool) {
        self.emit(literal, role, structural_end, None, 0);
    }

    pub(crate) fn token_at(
        &mut self,
        literal: &str,
        role: Role,
        structural_end: bool,
        start: usize,
        end: usize,
    ) {
        let origin = self
            .source_tokens
            .binary_search_by_key(&start, |token| token.start)
            .ok()
            .and_then(|id| {
                let token = &self.source_tokens[id];
                // For accepted AST/CST pairs, an exact token span determines
                // this spelling. Keep the check as a local origin invariant.
                (token.end == end && token.text == literal).then_some(token.id)
            });
        assert!(
            self.source_tokens.is_empty() || origin.is_some(),
            "AST token has no exact CST origin: {literal:?} [{start},{end})"
        );
        self.emit(
            literal,
            role,
            structural_end,
            origin,
            origin.map_or(0, |id| id + 1),
        );
    }

    pub(crate) fn token_from_span(
        &mut self,
        literal: &str,
        role: Role,
        structural_end: bool,
        start: usize,
        end: usize,
    ) {
        let first = self
            .source_tokens
            .partition_point(|token| token.start < start);
        let last = self.source_tokens.partition_point(|token| token.end <= end);
        let origin = (first < last).then_some(first);
        assert!(
            self.source_tokens.is_empty() || origin.is_some(),
            "AST token range has no CST origin: [{start},{end})"
        );
        self.emit(literal, role, structural_end, origin, last);
    }

    fn emit(
        &mut self,
        literal: &str,
        role: Role,
        structural_end: bool,
        source_id: Option<usize>,
        source_end: usize,
    ) {
        if !self.source_tokens.is_empty() {
            self.emissions.push(Emission {
                text: literal.to_owned(),
                role,
                structural_end,
                source_id,
                source_end,
            });
        }
        use Role::*;
        if role == BlockClose {
            self.depth = self.depth.saturating_sub(1);
            if self.previous != Some(BlockOpen) || self.output.ends_with('\n') {
                self.line();
            }
        } else if role == Function && self.previous == Some(BlockClose) {
            self.line();
            self.output.push('\n');
        } else if (self.previous == Some(BlockOpen) && role != BlockClose)
            || (self.previous == Some(Terminator) && role != BlockClose)
            || (self.previous_structural_end && role != Else)
        {
            self.line();
        }
        self.indent();
        if !self.output.is_empty()
            && !self.output.ends_with([' ', '\n'])
            && Self::space_before(self.previous, role)
        {
            self.output.push(' ');
        }
        self.output.push_str(literal);
        if role == BlockOpen {
            self.depth += 1;
        }
        self.previous = Some(role);
        self.previous_structural_end = structural_end;
    }

    fn space_before(previous: Option<Role>, next: Role) -> bool {
        use Role::*;
        let Some(previous) = previous else {
            return false;
        };
        if matches!(
            next,
            Close
                | Comma
                | Colon
                | Terminator
                | ContractClose
                | Suffix
                | TypeClose
                | TypeOpen
                | AttachedOpen
                | BlockClose
        ) {
            return false;
        }
        !matches!(previous, Open | AttachedOpen | ContractOpen | TypeOpen)
    }
}
