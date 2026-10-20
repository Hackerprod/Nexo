//! Structured diagnostics for the F0-0002 profile.
//!
//! This module is deliberately independent of the existing slice-1 frontend
//! and runtime. It provides the schema and presentation layer that later
//! compiler steps can use without changing their current output.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    io::Read,
};

pub const SCHEMA_VERSION: u8 = 1;
pub const PROFILE_ID: &str = "f0-0002-v1";
pub const SOURCE_LIMIT: usize = 1_048_576;

macro_rules! vocabulary {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
        }
    };
}

vocabulary!(Namespace { NexoProposedV1 => "Nexo-proposed-v1", F0002Sim => "F0-0002-sim" });
vocabulary!(Severity { Error => "error" });
vocabulary!(Stage {
    Input => "input", Lex => "lex", Parse => "parse", Names => "names",
    Arguments => "arguments", Types => "types", Ownership => "ownership",
    Effects => "effects", Lower => "lower", Entry => "entry", Runtime => "runtime"
});
vocabulary!(ExpectedTokensKind {
    AdmissibleSet => "admissible_set", CanonicalInsertion => "canonical_insertion",
    LexicalCategory => "lexical_category"
});
vocabulary!(EntityKind {
    Function => "function", Parameter => "parameter", Binding => "binding",
    Temporary => "temporary", Loan => "loan", Intrinsic => "intrinsic", Type => "type"
});
vocabulary!(Evidence {
    Verified => "verified", ExternalAssumption => "external_assumption", Unknown => "unknown"
});
vocabulary!(TokenKind {
    Name => "name", Keyword => "keyword", Symbol => "symbol", Integer => "integer",
    Decimal => "decimal", String => "string", Eof => "eof", Invalid => "invalid"
});
vocabulary!(RepairStatus { Proposed => "proposed", Applicable => "applicable" });
vocabulary!(RepairMode { Check => "check", RunEntry => "run_entry" });
vocabulary!(RepairResult { Accepted => "accepted", Rejected => "rejected" });
vocabulary!(RelatedRole {
    Declaration => "declaration", PreviousDeclaration => "previous_declaration",
    Contract => "contract", Origin => "origin", Consumption => "consumption",
    Backedge => "backedge", Exit => "exit", ExpectedType => "expected_type",
    FoundType => "found_type", Context => "context", Delimiter => "delimiter",
    PrecedingExpression => "preceding_expression"
});
vocabulary!(CauseKind {
    Call => "call", Copy => "copy", Consume => "consume", Borrow => "borrow",
    Reborrow => "reborrow", Drop => "drop", Return => "return", Backedge => "backedge",
    Effect => "effect", Limit => "limit"
});
vocabulary!(CauseOperation {
    FunctionCall => "function.call", IntrinsicCall => "intrinsic.call",
    BuildArray => "value.build_array", ValueCopy => "value.copy", ValueDrop => "value.drop",
    ResourceDrop => "resource.drop", ValueConsume => "value.consume",
    BorrowRead => "borrow.read", BorrowInout => "borrow.inout",
    BorrowReborrow => "borrow.reborrow", FlowBackedge => "flow.backedge"
});
vocabulary!(Operation {
    SyntaxLex => "syntax.lex", SyntaxParse => "syntax.parse", AnalysisLimit => "analysis.limit",
    InputRead => "input.read", InputDecode => "input.decode", NamesResolve => "names.resolve",
    NamesDeclare => "names.declare", TypesCheck => "types.check", TypesInfer => "types.infer",
    PlaceWrite => "place.write", ValueLiteral => "value.literal", CallBind => "call.bind",
    EntryCheck => "entry.check", ValueCopy => "value.copy", OwnershipUse => "ownership.use",
    OwnershipAccess => "ownership.access", Branch => "flow.branch", Return => "flow.return",
    RuntimeLimit => "runtime.limit", ValueAdd => "value.add", ValueSub => "value.sub",
    ValueIndex => "value.index", AssertEq => "intrinsic.assert_eq", Fail => "intrinsic.fail",
    FunctionCall => "function.call", IntrinsicCall => "intrinsic.call",
    BuildArray => "value.build_array", ValueDrop => "value.drop"
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub file: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelatedSpan {
    pub role: RelatedRole,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub kind: EntityKind,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cause {
    pub kind: CauseKind,
    pub operation: CauseOperation,
    pub span: Option<Span>,
    pub entity: Option<Entity>,
    pub evidence: Evidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub span: Span,
    pub expected_text: String,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairValidation {
    pub profile_id: String,
    pub mode: RepairMode,
    pub result: RepairResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repair {
    pub id: String,
    pub status: RepairStatus,
    pub description: String,
    pub edits: Vec<Edit>,
    pub validation: Option<RepairValidation>,
    pub consequences: Vec<String>,
}

/// JSON values allowed in the extensible, but key-restricted, `data` object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataValue {
    Null,
    Bool(bool),
    UInt(u64),
    String(String),
    Array(Vec<DataValue>),
    Object(BTreeMap<String, DataValue>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub schema_version: u8,
    pub diagnostic_namespace: Namespace,
    pub profile_id: String,
    pub code: String,
    pub subcause: Option<String>,
    pub rule: String,
    pub explanation_id: Option<String>,
    pub severity: Severity,
    pub stage: Stage,
    pub message: String,
    pub operation: Operation,
    pub primary_span: Option<Span>,
    pub related_spans: Vec<RelatedSpan>,
    pub cause_chain: Vec<Cause>,
    pub feature_id: Option<String>,
    pub unexpected_token: Option<Token>,
    pub expected_tokens: Vec<String>,
    pub expected_tokens_kind: Option<ExpectedTokensKind>,
    pub parser_context: Option<String>,
    pub recognized_prefix: Option<Span>,
    pub repair: Vec<Repair>,
    pub data: BTreeMap<String, DataValue>,
}

/// The byte length of the original input and decoded text, when decoding succeeded.
#[derive(Clone, Copy, Debug)]
pub struct SourceView<'a> {
    pub bytes_len: usize,
    pub text: Option<&'a str>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JsonOrder {
    Schema,
    Lexicographic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaError(pub String);
impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SchemaError {}

impl Diagnostic {
    /// Checks schema invariants and all spans against the original source bytes.
    pub fn validate(&self, file: &str, source: &str) -> Result<(), SchemaError> {
        self.validate_view(
            file,
            SourceView {
                bytes_len: source.len(),
                text: Some(source),
            },
        )
    }

    pub fn validate_view(&self, file: &str, source: SourceView<'_>) -> Result<(), SchemaError> {
        let fail = |s: &str| Err(SchemaError(s.to_owned()));
        if self.schema_version != SCHEMA_VERSION {
            return fail("unsupported schema_version");
        }
        const PUBLISHED_PROFILES: &[&str] = &[
            PROFILE_ID,
            "f0-0002-test-types-32-v1",
            "f0-0002-test-flow-32-v1",
            "f0-0002-test-effects-32-v1",
            "f0-0002-test-ir-32-v1",
            "f0-0002-test-frames-8-v1",
            "f0-0002-test-cells-64-v1",
            "f0-0002-test-observations-64-v1",
        ];
        if !PUBLISHED_PROFILES.contains(&self.profile_id.as_str()) {
            return fail("unknown profile_id");
        }
        if self.code.starts_with("NX-E") {
            if catalog_entry(&self.code).is_none() {
                return fail("unknown canonical NX-E code");
            }
            if self.diagnostic_namespace != Namespace::NexoProposedV1 {
                return fail("NX-E namespace mismatch");
            }
            if self.rule != rule(&self.code, self.subcause.as_deref()) {
                return fail("rule does not match code/subcause");
            }
            if self.explanation_id.as_deref() != Some(self.rule.as_str()) {
                return fail("NX-E explanation_id mismatch");
            }
            if self.code == "NX-E0006" {
                if self.subcause.is_some()
                    || !feature_known(self.feature_id.as_deref().unwrap_or(""))
                {
                    return fail("invalid NX-E0006 feature_id");
                }
            } else {
                if self.feature_id.is_some() {
                    return fail("feature_id is reserved for NX-E0006");
                }
                if let Some(subcause) = &self.subcause {
                    if !catalog_subcauses(&self.code)
                        .iter()
                        .any(|known| known == subcause)
                    {
                        return fail("unknown canonical subcause");
                    }
                } else if !catalog_subcauses(&self.code).is_empty() {
                    return fail("subcause required for this canonical code");
                }
            }
            if availability(&self.code, self.subcause.as_deref()) != "in_profile" {
                return fail("§13.12 selection is not emitible in this profile");
            }
        } else if self.code.starts_with('R') {
            if self.diagnostic_namespace != Namespace::F0002Sim
                || self.subcause.is_some()
                || self.explanation_id.is_some()
            {
                return fail("invalid runtime diagnostic identity");
            }
            if self.rule != self.code {
                return fail("runtime rule must equal code");
            }
            if ![
                "R0001", "R0002", "R0101", "R0102", "R0103", "R0201", "R0301",
            ]
            .contains(&self.code.as_str())
            {
                return fail("unknown runtime code");
            }
            if self.feature_id.is_some() {
                return fail("runtime diagnostics cannot have feature_id");
            }
        } else {
            return fail("unknown diagnostic code");
        }
        if self.message.chars().any(char::is_control) {
            return fail("message contains terminal controls");
        }
        if self.expected_tokens_kind.is_none() && !self.expected_tokens.is_empty() {
            return fail("§13.14 expected_tokens_kind required");
        }
        if self.expected_tokens_kind.is_some() && self.expected_tokens.is_empty() {
            return fail("§13.14 expected_tokens required");
        }
        if matches!(
            self.expected_tokens_kind,
            Some(ExpectedTokensKind::AdmissibleSet)
        ) {
            let mut sorted = self.expected_tokens.clone();
            sorted.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
            sorted.dedup();
            if sorted != self.expected_tokens {
                return fail("admissible expected_tokens must be unique and UTF-8 sorted");
            }
        }
        let mut spans = Vec::new();
        if let Some(s) = &self.primary_span {
            spans.push(s);
        }
        if self.code == "NX-E0003"
            && self.data.get("budget_kind") == Some(&DataValue::String("source_bytes".into()))
            && (self.data.get("limit") != Some(&DataValue::UInt(1_048_576))
                || self.data.get("observed") != Some(&DataValue::UInt(1_048_577))
                || self
                    .primary_span
                    .as_ref()
                    .is_none_or(|s| (s.start, s.end) != (1_048_576, 1_048_577)))
        {
            return fail(
                "§13.6 NX-E0003/source_bytes requires limit 1048576, observed 1048577 and [limit,limit+1)",
            );
        }
        if self.code == "NX-E0003" {
            let kind = self.data.get("budget_kind");
            let reduced = match self.profile_id.as_str() {
                "f0-0002-test-types-32-v1" => Some(("type_solver_steps", 32)),
                "f0-0002-test-flow-32-v1" => Some(("ownership_flow_steps", 32)),
                "f0-0002-test-effects-32-v1" => Some(("effect_fixpoint_steps", 32)),
                "f0-0002-test-ir-32-v1" => Some(("ir_items", 32)),
                _ => None,
            };
            let kind_name = match kind {
                Some(DataValue::String(k)) => k.as_str(),
                _ => "",
            };
            let normal_limit = match kind_name {
                "source_bytes" => Some(1_048_576),
                "syntax_nesting"
                | "expression_tree_depth"
                | "array_type_depth"
                | "value_type_depth" => Some(128),
                "function_count" => Some(1_024),
                "type_solver_steps" | "ownership_flow_steps" | "effect_fixpoint_steps" => {
                    Some(1_000_000)
                }
                "ir_items" => Some(262_144),
                _ => None,
            };
            let expected_limit = reduced
                .filter(|(expected_kind, _)| *expected_kind == kind_name)
                .map(|(_, limit)| limit as u64)
                .or(normal_limit);
            if expected_limit
                .is_some_and(|limit| self.data.get("limit") != Some(&DataValue::UInt(limit)))
            {
                return fail("§11.1/§11.4 budget limit does not match profile");
            }
            let costly = matches!(
                kind_name,
                "type_solver_steps" | "effect_fixpoint_steps" | "ir_items" | "ownership_flow_steps"
            );
            let structural = matches!(
                kind_name,
                "syntax_nesting"
                    | "expression_tree_depth"
                    | "function_count"
                    | "array_type_depth"
                    | "value_type_depth"
            );
            let invalid_structural_span = self.primary_span.as_ref().is_some_and(|s| {
                if kind_name == "syntax_nesting" {
                    s.start == s.end && s.end != source.bytes_len as u64
                } else {
                    s.start >= s.end
                }
            });
            if self.primary_span.is_none()
                || (costly && self.primary_span.as_ref().is_some_and(|s| s.start >= s.end))
                || (structural && invalid_structural_span)
            {
                return fail(
                    "§13.13 NX-E0003 requires primary_span; costly analysis spans are nonempty",
                );
            }
        }
        for x in &self.related_spans {
            spans.push(&x.span);
        }
        if let Some(t) = &self.unexpected_token {
            spans.push(&t.span);
        }
        if let Some(s) = &self.recognized_prefix {
            spans.push(s);
        }
        for c in &self.cause_chain {
            if let Some(s) = &c.span {
                spans.push(s);
            }
        }
        for r in &self.repair {
            for e in &r.edits {
                spans.push(&e.span);
            }
        }
        for s in spans {
            if s.file != file {
                return fail("span file differs from source file");
            }
            let (start, end) = (
                usize::try_from(s.start).map_err(|_| SchemaError("span offset overflow".into()))?,
                usize::try_from(s.end).map_err(|_| SchemaError("span offset overflow".into()))?,
            );
            if start > end
                || end > source.bytes_len
                || source.text.is_some_and(|text| {
                    !text.is_char_boundary(start) || !text.is_char_boundary(end)
                })
            {
                return fail("span is outside source or not on UTF-8 boundaries");
            }
        }
        if let Some(token) = &self.unexpected_token
            && token.kind == TokenKind::Invalid
            && let Some(text) = &token.text
            && let Some(source_text) = source.text
        {
            let start = token.span.start as usize;
            let end = token.span.end as usize;
            if source_text.get(start..end) != Some(text.as_str()) {
                return fail("§13.6 unexpected_token invalid text differs from source lexeme");
            }
        }
        self.validate_catalog_coordinates()?;
        self.validate_code_contract()?;
        if self
            .related_spans
            .windows(2)
            .any(|w| related_key(&w[0]) > related_key(&w[1]))
        {
            return fail("related_spans are not canonically ordered");
        }
        if self
            .related_spans
            .windows(2)
            .any(|w| related_key(&w[0]) == related_key(&w[1]))
        {
            return fail("related_spans contain an exact duplicate");
        }
        if self.repair.windows(2).any(|w| w[0].id >= w[1].id) {
            return fail("repair list is not ordered by unique id");
        }
        if self.repair.iter().any(|r| r.id != "insert_terminator") {
            return fail("unknown Repair.id");
        }
        for repair in &self.repair {
            if repair.status == RepairStatus::Applicable
                && !repair.validation.as_ref().is_some_and(|v| {
                    v.result == RepairResult::Accepted && v.profile_id == self.profile_id
                })
            {
                return fail("applicable repair lacks accepted validation for this profile");
            }
            let mut prior: Option<(&str, usize)> = None;
            for edit in &repair.edits {
                let start = edit.span.start as usize;
                let end = edit.span.end as usize;
                if source.text.and_then(|text| text.get(start..end))
                    != Some(edit.expected_text.as_str())
                {
                    return fail("repair expected_text differs from source bytes");
                }
                if let Some((prior_file, prior_end)) = prior
                    && (prior_file > edit.span.file.as_str()
                        || (prior_file == edit.span.file && prior_end > start))
                {
                    return fail("repair edits are not ordered and non-overlapping");
                }
                prior = Some((&edit.span.file, end));
            }
        }
        let allowed = [
            "input_path",
            "invalid_bytes_hex",
            "budget_kind",
            "runtime_budget_kind",
            "limit",
            "observed",
            "work_consumed",
            "declaration_kind",
            "use_kind",
            "expected_type",
            "found_type",
            "unresolved_components",
            "type_spelling",
            "pending_scope",
            "entry",
            "effect_model",
            "effect_evidence",
            "contract_origin",
            "required_effect",
            "origin",
            "loan",
            "binding",
            "parameter",
            "active_permission",
            "requested_access",
        ];
        if self.data.keys().any(|k| !allowed.contains(&k.as_str())) {
            return fail("unknown data key");
        }
        for (key, value) in &self.data {
            let valid = match key.as_str() {
                "limit" | "observed" => matches!(value, DataValue::UInt(_)),
                "work_consumed" => true,
                "unresolved_components" => {
                    matches!(value, DataValue::Array(items) if items.iter().all(|item| matches!(item, DataValue::String(_))))
                }
                "origin" | "loan" | "binding" | "parameter" => valid_entity_value(value),
                "contract_origin" => valid_span_value(value),
                _ => matches!(value, DataValue::String(_)),
            };
            if !valid {
                return fail("data value has the wrong schema type");
            }
            if key == "contract_origin" && !span_value_matches_source(value, file, source) {
                return fail("§13.6 data.contract_origin span is outside source or has wrong file");
            }
        }
        self.validate_data()?;
        Ok(())
    }

    fn validate_code_contract(&self) -> Result<(), SchemaError> {
        let err = |rule: &str| Err(SchemaError(format!("rule {rule}")));
        if self
            .unexpected_token
            .as_ref()
            .is_some_and(|token| token.kind == TokenKind::Eof && token.span.start != token.span.end)
        {
            return err("§13.6 EOF token span must be empty");
        }
        if let Some(token) = &self.unexpected_token
            && token.kind == TokenKind::Eof
            && token.text.as_deref() != Some("")
        {
            return err("§13.6 EOF token text must be empty");
        }
        if self.code == "NX-E0005" {
            if self.stage != Stage::Parse || self.operation != Operation::SyntaxParse {
                return err("§13.12 NX-E0005 is parse/syntax.parse");
            }
            if self.unexpected_token.is_none()
                || self.parser_context.is_none()
                || self.recognized_prefix.is_none()
                || self.primary_span.is_none()
            {
                return err(
                    "§13.6 NX-E0005 requires primary_span, unexpected_token, parser_context and recognized_prefix",
                );
            }
            let has_preceding = self
                .related_spans
                .iter()
                .any(|s| s.role == RelatedRole::PrecedingExpression);
            let has_context = self
                .related_spans
                .iter()
                .any(|s| s.role == RelatedRole::Context);
            if has_preceding == has_context {
                return err(
                    "§13.15 NX-E0005 requires exactly one of preceding_expression or context",
                );
            }
            if self.unexpected_token.as_ref().unwrap().span
                != *self
                    .primary_span
                    .as_ref()
                    .unwrap_or(&self.unexpected_token.as_ref().unwrap().span)
            {
                return err("§13.14 NX-E0005 unexpected_token span equals primary_span");
            }
            if self.expected_tokens != [";"]
                || self.expected_tokens_kind != Some(ExpectedTokensKind::CanonicalInsertion)
                || self.parser_context.as_deref() != Some("statement_terminator")
            {
                return err(
                    "§13.14 NX-E0005 requires canonical semicolon insertion in statement_terminator",
                );
            }
        }
        if self.code == "NX-E0001" {
            let Some(primary) = &self.primary_span else {
                return err("§13.14 NX-E0001 requires primary_span");
            };
            let prefix = Span {
                file: primary.file.clone(),
                start: 0,
                end: primary.start,
            };
            if self.stage != Stage::Lex
                || self.operation != Operation::SyntaxLex
                || self.parser_context.as_deref() != Some("lexical_token")
                || self.expected_tokens != ["valid_token"]
                || self.expected_tokens_kind != Some(ExpectedTokensKind::LexicalCategory)
                || self.recognized_prefix.as_ref() != Some(&prefix)
                || self.unexpected_token.is_none()
            {
                return err("§13.14 NX-E0001 lexical token contract");
            }
            let token = self.unexpected_token.as_ref().unwrap();
            if token.span != *primary {
                return err("§13.14 NX-E0001 unexpected_token span equals primary_span");
            }
            if token.kind == TokenKind::Invalid {
                let expected = primary.start as usize;
                let end = primary.end as usize;
                let lexeme = self.data.get("invalid_bytes_hex");
                let text_matches = match (&token.text, lexeme) {
                    (Some(text), _) => text.len() == end.saturating_sub(expected),
                    _ => false,
                };
                if !text_matches {
                    return err("§13.14 NX-E0001 invalid token text must equal its lexeme");
                }
            }
        }
        if self.code == "NX-E0006" && self.primary_span.is_none() {
            return err("§13.12 NX-E0006 requires primary_span");
        }
        if self.code == "NX-E0004" && self.subcause.as_deref() == Some("invalid_utf8") {
            if self.primary_span.is_none() {
                return err("§13.6 NX-E0004/invalid_utf8 requires primary_span");
            }
            if !matches!(self.data.get("invalid_bytes_hex"), Some(DataValue::String(hex)) if !hex.is_empty() && hex.len() % 2 == 0 && hex.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
            {
                return err(
                    "§13.6 invalid_bytes_hex must contain nonempty lowercase hexadecimal byte pairs",
                );
            }
            let Some(DataValue::String(hex)) = self.data.get("invalid_bytes_hex") else {
                return err(
                    "§13.6 invalid_bytes_hex must contain nonempty lowercase hexadecimal byte pairs",
                );
            };
            let span = self.primary_span.as_ref().unwrap();
            if u64::try_from(hex.len() / 2).ok() != Some(span.end - span.start) {
                return err("§13.6 invalid_bytes_hex length must equal invalid_utf8 span length");
            }
        }
        if !self.repair.is_empty() && self.code != "NX-E0005" {
            return err("§13.6 repair is empty outside NX-E0005");
        }
        if self.code.starts_with('R') {
            let operation = match self.code.as_str() {
                "R0001" | "R0002" => Operation::RuntimeLimit,
                "R0101" => Operation::ValueAdd,
                "R0102" => Operation::ValueIndex,
                "R0103" => Operation::ValueSub,
                "R0201" => Operation::AssertEq,
                "R0301" => Operation::Fail,
                _ => return err("§13.12 unknown runtime operation code"),
            };
            if self.stage != Stage::Runtime || self.operation != operation {
                return err(&format!(
                    "§13.12 {} requires runtime/{}",
                    self.code,
                    operation.as_str()
                ));
            }
        }
        for (key, prohibited) in [
            (
                "expected_type",
                !matches!(self.code.as_str(), "NX-E0102" | "NX-E1303"),
            ),
            (
                "found_type",
                !matches!(self.code.as_str(), "NX-E0102" | "NX-E1303"),
            ),
            ("use_kind", self.code != "NX-E0101"),
            ("budget_kind", self.code != "NX-E0003"),
            ("pending_scope", self.code != "NX-E0006"),
        ] {
            if prohibited && self.data.contains_key(key) {
                return err("§13.6 field is absent in other codes");
            }
        }
        if matches!(
            self.code.as_str(),
            "NX-E0412" | "NX-E0413" | "NX-E0414" | "NX-E0415"
        ) {
            for key in [
                "effect_model",
                "contract_origin",
                "effect_evidence",
                "required_effect",
            ] {
                if !self.data.contains_key(key) {
                    return Err(SchemaError(format!(
                        "§13.6 data.{key} is required for effect diagnostics"
                    )));
                }
            }
        }
        if self.code == "NX-E0002" {
            if self.expected_tokens_kind != Some(ExpectedTokensKind::AdmissibleSet) {
                return err("§13.14 NX-E0002 expected_tokens_kind must be admissible_set");
            }
            let Some(token) = &self.unexpected_token else {
                return err("§13.6 NX-E0002 requires unexpected_token");
            };
            if self.primary_span.as_ref() != Some(&token.span) {
                return err("§13.14 NX-E0002 unexpected_token span equals primary_span");
            }
        }
        if matches!(self.code.as_str(), "NX-E0002" | "NX-E0005") {
            let token = self.unexpected_token.as_ref().unwrap();
            if self
                .recognized_prefix
                .as_ref()
                .is_none_or(|p| p.file != token.span.file || p.end != token.span.start)
            {
                return err("§13.14 recognized_prefix ends at unexpected token start");
            }
        }
        if let Some(token) = &self.unexpected_token {
            match token.kind {
                TokenKind::Invalid if token.text.is_none() && self.code != "NX-E0004" => {
                    return err("§13.6 decoded invalid token requires text");
                }
                TokenKind::Invalid => {}
                _ => {}
            }
        }
        for cause in &self.cause_chain {
            let coherent = match cause.kind {
                CauseKind::Call => match cause.operation {
                    CauseOperation::FunctionCall | CauseOperation::IntrinsicCall => true,
                    CauseOperation::BuildArray
                    | CauseOperation::ValueCopy
                    | CauseOperation::ValueDrop
                    | CauseOperation::ResourceDrop
                    | CauseOperation::ValueConsume
                    | CauseOperation::BorrowRead
                    | CauseOperation::BorrowInout
                    | CauseOperation::BorrowReborrow
                    | CauseOperation::FlowBackedge => false,
                },
                CauseKind::Copy => match cause.operation {
                    CauseOperation::ValueCopy => true,
                    CauseOperation::FunctionCall
                    | CauseOperation::IntrinsicCall
                    | CauseOperation::BuildArray
                    | CauseOperation::ValueDrop
                    | CauseOperation::ResourceDrop
                    | CauseOperation::ValueConsume
                    | CauseOperation::BorrowRead
                    | CauseOperation::BorrowInout
                    | CauseOperation::BorrowReborrow
                    | CauseOperation::FlowBackedge => false,
                },
                CauseKind::Consume => match cause.operation {
                    CauseOperation::ValueConsume => true,
                    CauseOperation::FunctionCall
                    | CauseOperation::IntrinsicCall
                    | CauseOperation::BuildArray
                    | CauseOperation::ValueCopy
                    | CauseOperation::ValueDrop
                    | CauseOperation::ResourceDrop
                    | CauseOperation::BorrowRead
                    | CauseOperation::BorrowInout
                    | CauseOperation::BorrowReborrow
                    | CauseOperation::FlowBackedge => false,
                },
                CauseKind::Borrow => match cause.operation {
                    CauseOperation::BorrowRead | CauseOperation::BorrowInout => true,
                    CauseOperation::FunctionCall
                    | CauseOperation::IntrinsicCall
                    | CauseOperation::BuildArray
                    | CauseOperation::ValueCopy
                    | CauseOperation::ValueDrop
                    | CauseOperation::ResourceDrop
                    | CauseOperation::ValueConsume
                    | CauseOperation::BorrowReborrow
                    | CauseOperation::FlowBackedge => false,
                },
                CauseKind::Reborrow => match cause.operation {
                    CauseOperation::BorrowReborrow => true,
                    CauseOperation::FunctionCall
                    | CauseOperation::IntrinsicCall
                    | CauseOperation::BuildArray
                    | CauseOperation::ValueCopy
                    | CauseOperation::ValueDrop
                    | CauseOperation::ResourceDrop
                    | CauseOperation::ValueConsume
                    | CauseOperation::BorrowRead
                    | CauseOperation::BorrowInout
                    | CauseOperation::FlowBackedge => false,
                },
                CauseKind::Drop => match cause.operation {
                    CauseOperation::ValueDrop | CauseOperation::ResourceDrop => true,
                    CauseOperation::FunctionCall
                    | CauseOperation::IntrinsicCall
                    | CauseOperation::BuildArray
                    | CauseOperation::ValueCopy
                    | CauseOperation::ValueConsume
                    | CauseOperation::BorrowRead
                    | CauseOperation::BorrowInout
                    | CauseOperation::BorrowReborrow
                    | CauseOperation::FlowBackedge => false,
                },
                CauseKind::Return | CauseKind::Limit => match cause.operation {
                    CauseOperation::FunctionCall
                    | CauseOperation::IntrinsicCall
                    | CauseOperation::BuildArray
                    | CauseOperation::ValueCopy
                    | CauseOperation::ValueDrop
                    | CauseOperation::ResourceDrop
                    | CauseOperation::ValueConsume
                    | CauseOperation::BorrowRead
                    | CauseOperation::BorrowInout
                    | CauseOperation::BorrowReborrow
                    | CauseOperation::FlowBackedge => false,
                },
                CauseKind::Backedge => match cause.operation {
                    CauseOperation::FlowBackedge => true,
                    CauseOperation::FunctionCall
                    | CauseOperation::IntrinsicCall
                    | CauseOperation::BuildArray
                    | CauseOperation::ValueCopy
                    | CauseOperation::ValueDrop
                    | CauseOperation::ResourceDrop
                    | CauseOperation::ValueConsume
                    | CauseOperation::BorrowRead
                    | CauseOperation::BorrowInout
                    | CauseOperation::BorrowReborrow => false,
                },
                CauseKind::Effect => match cause.operation {
                    CauseOperation::BuildArray => true,
                    CauseOperation::FunctionCall
                    | CauseOperation::IntrinsicCall
                    | CauseOperation::ValueCopy
                    | CauseOperation::ValueDrop
                    | CauseOperation::ResourceDrop
                    | CauseOperation::ValueConsume
                    | CauseOperation::BorrowRead
                    | CauseOperation::BorrowInout
                    | CauseOperation::BorrowReborrow
                    | CauseOperation::FlowBackedge => false,
                },
            };
            if !coherent {
                return err("§13.15 cause kind/operation correspondence");
            }
        }
        Ok(())
    }

    fn validate_catalog_coordinates(&self) -> Result<(), SchemaError> {
        if !self.code.starts_with("NX-E") {
            return Ok(());
        }
        let (stage, operation) = if self.code == "NX-E0003" {
            let kind = self.data.get("budget_kind").and_then(|v| match v {
                DataValue::String(s) => Some(s.as_str()),
                _ => None,
            });
            let stage = match kind {
                Some("source_bytes") => "input",
                Some("syntax_nesting" | "expression_tree_depth" | "function_count") => "parse",
                Some("array_type_depth" | "value_type_depth" | "type_solver_steps") => "types",
                Some("ownership_flow_steps") => "ownership",
                Some("effect_fixpoint_steps") => "effects",
                Some("ir_items") => "lower",
                _ => {
                    return Err(SchemaError(
                        "§13.13 NX-E0003 budget_kind has no stage".into(),
                    ));
                }
            };
            if self.stage.as_str() != stage {
                return Err(SchemaError(
                    "§13.13 NX-E0003 budget_kind/stage mismatch".into(),
                ));
            }
            (stage, "analysis.limit")
        } else if self.code == "NX-E0006" {
            let f = self.feature_id.as_deref().unwrap_or("");
            match f {
                "function_value" => ("names", "names.resolve"),
                "inferred_non_unit_result" | "resource_array" | "option" | "unsupported_type" => {
                    ("types", "types.check")
                }
                "async" if self.stage == Stage::Types => ("types", "types.check"),
                "partial_transfer" if self.stage == Stage::Arguments => ("arguments", "call.bind"),
                "legacy_move"
                | "float_literal"
                | "range"
                | "tuple"
                | "attributes"
                | "default_argument"
                | "closure"
                | "receiver_mode"
                | "no_heap_alloc"
                | "match"
                | "user_types"
                | "traits_generics"
                | "modules_visibility"
                | "extensions"
                | "provenance_clauses"
                | "comptime"
                | "text"
                | "methods"
                | "compound_assignment"
                | "for_iteration"
                | "loop_control"
                | "generator"
                | "region"
                | "partial_transfer"
                | "async" => ("parse", "syntax.parse"),
                _ => {
                    return Err(SchemaError(
                        "§13.12/§14.6 NX-E0006 unknown feature coordinates".into(),
                    ));
                }
            }
        } else if let Some(entry) = catalog_entry(&self.code) {
            if self.code == "NX-E0412"
                || self.code == "NX-E0413"
                || self.code == "NX-E0414"
                || self.code == "NX-E0415"
            {
                if self.stage.as_str() != "effects"
                    || ![
                        "function.call",
                        "intrinsic.call",
                        "value.build_array",
                        "value.copy",
                        "value.drop",
                    ]
                    .contains(&self.operation.as_str())
                {
                    return Err(SchemaError(
                        "§13.12/§13.15 effect diagnostic coordinates".into(),
                    ));
                }
                return Ok(());
            }
            let coords = self
                .subcause
                .as_deref()
                .and_then(|sub| {
                    entry
                        .subcauses
                        .iter()
                        .find(|c| c.name == sub)
                        .map(|c| (c.stage, c.operation))
                })
                .unwrap_or((entry.stage, entry.operation));
            if coords.0.is_empty() {
                // §13.12 supplies the grouped coordinates for rows whose catalog entry has no literal coordinates.
                coordinates_by_code(&self.code, self.subcause.as_deref())
                    .ok_or_else(|| SchemaError("§13.12 missing catalog coordinates".into()))?
            } else {
                coords
            }
        } else {
            return Ok(());
        };
        if self.stage.as_str() != stage || self.operation.as_str() != operation {
            return Err(SchemaError(format!(
                "§13.12 {}/{} requires {stage}/{operation}",
                self.code,
                self.subcause.as_deref().unwrap_or("")
            )));
        }
        Ok(())
    }

    fn validate_data(&self) -> Result<(), SchemaError> {
        let has = |k: &str| self.data.contains_key(k);
        let code = self.code.as_str();
        let cause = self.subcause.as_deref();
        let allowed_for_code: &[&str] = match code {
            "NX-E0003" => &["budget_kind", "limit", "observed"],
            "NX-E0004" if cause == Some("source_unreadable") => &["input_path"],
            "NX-E0004" => &["invalid_bytes_hex"],
            "NX-E0006" => &["pending_scope", "type_spelling"],
            "NX-E0101" => &["use_kind"],
            "NX-E0102" | "NX-E1303" => &["expected_type", "found_type"],
            "NX-E0104" => &["declaration_kind"],
            "NX-E0112" => &["entry"],
            "NX-E0113" => &["unresolved_components"],
            "NX-E0412" | "NX-E0413" | "NX-E0414" | "NX-E0415" => &[
                "effect_model",
                "effect_evidence",
                "contract_origin",
                "required_effect",
            ],
            "NX-E1105" => &[
                "origin",
                "loan",
                "binding",
                "parameter",
                "active_permission",
                "requested_access",
            ],
            "R0001" => &["limit", "observed", "work_consumed"],
            "R0002" => &["runtime_budget_kind", "limit", "observed"],
            _ => &[],
        };
        if self
            .data
            .keys()
            .any(|k| !allowed_for_code.contains(&k.as_str()))
        {
            return Err(SchemaError(format!(
                "§13.6 data key is absent for {}{}",
                code,
                cause.map(|s| format!("/{s}")).unwrap_or_default()
            )));
        }
        if code == "NX-E0004" && cause == Some("source_unreadable") && self.primary_span.is_some() {
            return Err(SchemaError(
                "§13.6 source_unreadable primary_span must be null".into(),
            ));
        }
        if code == "NX-E0002"
            && (self.parser_context.is_none()
                || self.unexpected_token.is_none()
                || self.recognized_prefix.is_none())
        {
            return Err(SchemaError(
                "§13.6 NX-E0002 requires parser_context, unexpected_token and recognized_prefix"
                    .into(),
            ));
        }
        if matches!(code, "NX-E0102" | "NX-E1303") && (!has("expected_type") || !has("found_type"))
        {
            return Err(SchemaError(format!(
                "§13.6/§13.17 {code} requires expected_type and found_type"
            )));
        }
        if code == "NX-E0006"
            && self.feature_id.as_deref() == Some("unsupported_type")
            && !has("type_spelling")
        {
            return Err(SchemaError(
                "§13.6 NX-E0006/unsupported_type requires type_spelling".into(),
            ));
        }
        if code == "NX-E0006"
            && self.feature_id.as_deref() != Some("unsupported_type")
            && has("type_spelling")
        {
            return Err(SchemaError(
                "§13.6 NX-E0006 type_spelling is absent outside unsupported_type".into(),
            ));
        }
        if code == "NX-E0112" && self.data.get("entry") != Some(&DataValue::String("main".into())) {
            return Err(SchemaError("§13.6 NX-E0112 entry must be main".into()));
        }
        if code == "NX-E0113" {
            let valid_path = |s: &str| {
                s == "$"
                    || s.strip_prefix("$").is_some_and(|tail| {
                        let mut rest = tail;
                        while !rest.is_empty() {
                            if let Some(next) = rest.strip_prefix(".element") {
                                rest = next;
                            } else if let Some(next) = rest.strip_prefix(".ok") {
                                rest = next;
                            } else if let Some(next) = rest.strip_prefix(".err") {
                                rest = next;
                            } else {
                                return false;
                            }
                        }
                        true
                    })
            };
            if !matches!(self.data.get("unresolved_components"), Some(DataValue::Array(xs)) if !xs.is_empty() && xs.iter().all(|x| matches!(x, DataValue::String(s) if valid_path(s))))
            {
                return Err(SchemaError(
                    "§13.17 unresolved_components violates ComponentPath".into(),
                ));
            }
            if let Some(DataValue::Array(paths)) = self.data.get("unresolved_components") {
                let mut unique = BTreeSet::new();
                if paths.iter().any(|path| match path {
                    DataValue::String(path) => !unique.insert(path.as_str()),
                    _ => false,
                }) {
                    return Err(SchemaError(
                        "§13.17 unresolved_components paths must be unique".into(),
                    ));
                }
            }
        }
        if code == "NX-E1105" {
            let sub = cause.unwrap_or("");
            let (required, pairs): (&[&str], &[(&str, &str)]) = match sub {
                "permission_escalation" => (
                    &[
                        "origin",
                        "parameter",
                        "active_permission",
                        "requested_access",
                    ],
                    &[
                        ("read", "write"),
                        ("read", "consume"),
                        ("read", "drop"),
                        ("read", "borrow_inout"),
                        ("inout", "consume"),
                        ("inout", "drop"),
                    ],
                ),
                "active_borrow_conflict" => (
                    &[
                        "origin",
                        "binding",
                        "loan",
                        "active_permission",
                        "requested_access",
                    ],
                    &[
                        ("read", "write"),
                        ("read", "consume"),
                        ("read", "drop"),
                        ("read", "borrow_inout"),
                        ("inout", "read"),
                        ("inout", "write"),
                        ("inout", "consume"),
                        ("inout", "drop"),
                    ],
                ),
                "overlapping_mutable_borrows" => (
                    &[
                        "origin",
                        "binding",
                        "loan",
                        "active_permission",
                        "requested_access",
                    ],
                    &[("inout", "borrow_inout")],
                ),
                _ => return Err(SchemaError("§13.6 unknown NX-E1105 data subcause".into())),
            };
            if required.iter().any(|k| !has(k))
                || self.data.keys().any(|k| !required.contains(&k.as_str()))
            {
                return Err(SchemaError(format!(
                    "§13.6 NX-E1105/{sub} data keys do not match the fixture table"
                )));
            }
            let values = match (
                self.data.get("active_permission"),
                self.data.get("requested_access"),
            ) {
                (Some(DataValue::String(a)), Some(DataValue::String(r))) => {
                    Some((a.as_str(), r.as_str()))
                }
                _ => None,
            };
            if !values.is_some_and(|pair| pairs.contains(&pair)) {
                return Err(SchemaError(format!(
                    "§13.6 NX-E1105/{sub} active_permission/requested_access combination is prohibited"
                )));
            }
        }
        let required: &[&str] = match self.code.as_str() {
            "NX-E0004" if self.subcause.as_deref() == Some("source_unreadable") => &["input_path"],
            "NX-E0004" if self.subcause.as_deref() == Some("invalid_utf8") => {
                &["invalid_bytes_hex"]
            }
            "NX-E0003" => &["budget_kind", "limit", "observed"],
            "R0001" => &["limit", "observed", "work_consumed"],
            "R0002" => &["runtime_budget_kind", "limit", "observed"],
            "NX-E0104" => &["declaration_kind"],
            "NX-E0101" => &["use_kind"],
            "NX-E0113" => &["unresolved_components"],
            "NX-E0006" => &["pending_scope"],
            "NX-E0112" => &["entry"],
            _ => &[],
        };
        for k in required {
            if !has(k) {
                return Err(SchemaError(if *k == "pending_scope" {
                    "rule §13.6 NX-E0006 requires data.pending_scope".into()
                } else {
                    format!("§13.6 data.{k} is required")
                }));
            }
        }
        if self.code == "NX-E0003" && has("runtime_budget_kind") {
            return Err(SchemaError(
                "NX-E0003 cannot use runtime_budget_kind".into(),
            ));
        }
        if self.code == "R0002" && has("budget_kind") {
            return Err(SchemaError("R0002 cannot use budget_kind".into()));
        }
        let enum_value = |key: &str, allowed: &[&str]| -> Result<(), SchemaError> {
            if let Some(DataValue::String(value)) = self.data.get(key) {
                if !allowed.contains(&value.as_str()) {
                    return Err(SchemaError(format!(
                        "data.{key} is outside its closed vocabulary"
                    )));
                }
            } else if has(key) {
                return Err(SchemaError(format!("data.{key} must be a string")));
            }
            Ok(())
        };
        enum_value(
            "budget_kind",
            &[
                "source_bytes",
                "syntax_nesting",
                "addition_chain_length",
                "index_chain_length",
                "expression_tree_depth",
                "array_type_depth",
                "comptime_steps",
                "comptime_depth",
                "comptime_memory_bytes",
                "type_solver_steps",
                "effect_fixpoint_steps",
                "trait_solver_steps",
                "generic_instantiations",
                "value_type_depth",
                "function_count",
                "ownership_flow_steps",
                "ir_items",
            ],
        )?;
        enum_value(
            "runtime_budget_kind",
            &["call_frames", "live_value_cells", "observation_bytes"],
        )?;
        enum_value("declaration_kind", &["binding", "function", "parameter"])?;
        enum_value("use_kind", &["value", "call_target"])?;
        enum_value("pending_scope", &["migration", "F0-0003", "later_adr"])?;
        enum_value("effect_evidence", &["known_effect", "unknown_effect"])?;
        enum_value("required_effect", &["alloc", "block", "suspend", "io"])?;
        if matches!(code, "NX-E0412" | "NX-E0413" | "NX-E0414" | "NX-E0415")
            && self.data.get("effect_model") != Some(&DataValue::String("f0-0002-sim-v1".into()))
        {
            return Err(SchemaError(
                "§13.6 effect_model must be f0-0002-sim-v1".into(),
            ));
        }
        if matches!(code, "NX-E0102" | "NX-E1303") {
            for key in ["expected_type", "found_type"] {
                let expected_position = key == "expected_type";
                if !matches!(self.data.get(key), Some(DataValue::String(value)) if valid_diagnostic_type(value, expected_position))
                {
                    return Err(SchemaError(format!(
                        "§13.17 data.{key} is not a canonical type"
                    )));
                }
            }
            if let Some(DataValue::String(expected)) = self.data.get("expected_type") {
                let domain_allowed = match (self.subcause.as_deref(), expected.as_str()) {
                    (Some("type_mismatch"), "domain:copyable_data" | "domain:result") => true,
                    (Some("not_indexable"), "domain:array") => true,
                    (Some("resource_required"), "domain:mock_resource") => true,
                    _ => !expected.starts_with("domain:"),
                };
                if !domain_allowed {
                    return Err(SchemaError(
                        "§13.17 expected_type domain is not permitted for this check".into(),
                    ));
                }
            }
        }
        if code == "R0001"
            && matches!(self.data.get("work_consumed"), Some(DataValue::UInt(_)))
            && self.data.get("work_consumed") != self.data.get("limit")
        {
            return Err(SchemaError(
                "§13.6 R0001 work_consumed must equal granted work limit".into(),
            ));
        }
        if code == "R0002" {
            let reduced = match self.profile_id.as_str() {
                "f0-0002-test-frames-8-v1" => Some(("call_frames", 8)),
                "f0-0002-test-cells-64-v1" => Some(("live_value_cells", 64)),
                "f0-0002-test-observations-64-v1" => Some(("observation_bytes", 64)),
                _ => None,
            };
            let kind = match self.data.get("runtime_budget_kind") {
                Some(DataValue::String(k)) => k.as_str(),
                _ => "",
            };
            let normal_limit = match kind {
                "call_frames" => Some(256),
                "live_value_cells" | "observation_bytes" => Some(65_536),
                _ => None,
            };
            let expected_limit = reduced
                .filter(|(expected_kind, _)| *expected_kind == kind)
                .map(|(_, limit)| limit as u64)
                .or(normal_limit);
            if expected_limit
                .is_some_and(|limit| self.data.get("limit") != Some(&DataValue::UInt(limit)))
            {
                return Err(SchemaError(
                    "§11.2/§11.4 runtime budget limit does not match profile".into(),
                ));
            }
        }
        enum_value("active_permission", &["read", "inout"])?;
        enum_value(
            "requested_access",
            &["read", "write", "consume", "drop", "borrow_inout"],
        )?;
        for key in ["limit", "observed", "work_consumed"] {
            if self.data.get(key).is_some_and(
                |value| !matches!(value, DataValue::UInt(n) if *n <= 9_007_199_254_740_991),
            ) {
                return Err(SchemaError(format!(
                    "§13.6 data.{key} must be a JSON-safe UInt"
                )));
            }
        }
        if code == "R0001"
            && matches!(
                (self.data.get("limit"), self.data.get("observed")),
                (Some(DataValue::UInt(limit)), Some(DataValue::UInt(observed)))
                    if limit.checked_add(1) != Some(*observed)
            )
        {
            return Err(SchemaError(
                "§11.2/§11.6 R0001 observed must equal limit + 1".into(),
            ));
        }
        if self.code == "NX-E0006" {
            let expected = if self.feature_id.as_deref() == Some("legacy_move") {
                "migration"
            } else if matches!(
                self.feature_id.as_deref(),
                Some("closure" | "generator" | "option" | "match")
            ) {
                "F0-0003"
            } else {
                "later_adr"
            };
            if self.data.get("pending_scope") != Some(&DataValue::String(expected.into())) {
                return Err(SchemaError(
                    "pending_scope does not match feature_id".into(),
                ));
            }
        }
        if self.code.starts_with("R") && !self.cause_chain.is_empty() {
            return Err(SchemaError(
                "runtime diagnostics have empty cause_chain".into(),
            ));
        }
        Ok(())
    }

    pub fn to_json(&self, file: &str, source: &str) -> Result<String, SchemaError> {
        self.validate(file, source)?;
        Ok(self.to_json_unchecked())
    }

    pub fn to_json_view(&self, file: &str, source: SourceView<'_>) -> Result<String, SchemaError> {
        self.validate_view(file, source)?;
        Ok(self.to_json_unchecked())
    }

    fn to_json_unchecked(&self) -> String {
        let mut out = String::new();
        write!(&mut out, "{{\"schema_version\":{},\"diagnostic_namespace\":{},\"profile_id\":{},\"code\":{},\"subcause\":{},\"rule\":{},\"explanation_id\":{},\"severity\":{},\"stage\":{},\"message\":{},\"operation\":{},\"primary_span\":{},\"related_spans\":[", self.schema_version, q(self.diagnostic_namespace.as_str()), q(&self.profile_id), q(&self.code), opt_str(&self.subcause), q(&self.rule), opt_str(&self.explanation_id), q(self.severity.as_str()), q(self.stage.as_str()), q(&self.message), q(self.operation.as_str()), opt_span(&self.primary_span)).unwrap();
        comma_join(&mut out, &self.related_spans, |x| {
            related_json(x, JsonOrder::Schema)
        });
        out.push_str("],\"cause_chain\":[");
        comma_join(&mut out, &self.cause_chain, |x| {
            cause_json(x, JsonOrder::Schema)
        });
        write!(
            &mut out,
            "],\"feature_id\":{},\"unexpected_token\":{},\"expected_tokens\":[",
            opt_str(&self.feature_id),
            self.unexpected_token
                .as_ref()
                .map(|x| token_json(x, JsonOrder::Schema))
                .unwrap_or_else(|| "null".into())
        )
        .unwrap();
        comma_join(&mut out, &self.expected_tokens, |s| q(s));
        write!(&mut out, "],\"expected_tokens_kind\":{},\"parser_context\":{},\"recognized_prefix\":{},\"repair\":[", self.expected_tokens_kind.map(|x| q(x.as_str())).unwrap_or_else(|| "null".into()), opt_str(&self.parser_context), opt_span(&self.recognized_prefix)).unwrap();
        comma_join(&mut out, &self.repair, |x| {
            repair_json(x, JsonOrder::Schema)
        });
        out.push_str("],\"data\":{");
        let mut first = true;
        for (k, v) in &self.data {
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&q(k));
            out.push(':');
            out.push_str(&data_json(v));
        }
        out.push_str("}}");
        out
    }

    pub fn render_text(&self, file: &str, source: &str) -> Result<String, SchemaError> {
        self.render_text_view(
            file,
            SourceView {
                bytes_len: source.len(),
                text: Some(source),
            },
        )
    }

    pub fn render_text_view(
        &self,
        file: &str,
        source: SourceView<'_>,
    ) -> Result<String, SchemaError> {
        self.validate_view(file, source)?;
        let (line, col) = match (self.primary_span.as_ref(), source.text) {
            (Some(span), Some(text)) => line_column(text, span.start as usize),
            _ => (0, 0),
        };
        let mut out = format!("{}:{line}:{col}: {}\n", self.code, self.message);
        writeln!(out, "  namespace: {}", self.diagnostic_namespace.as_str()).unwrap();
        writeln!(out, "  profile: {}", self.profile_id).unwrap();
        writeln!(out, "  stage: {}", self.stage.as_str()).unwrap();
        writeln!(out, "  operation: {}", self.operation.as_str()).unwrap();
        writeln!(
            out,
            "  at: {}",
            self.primary_span
                .as_ref()
                .map(|s| span_json(s, JsonOrder::Lexicographic))
                .unwrap_or_else(|| "null".into())
        )
        .unwrap();
        if let Some(s) = &self.subcause {
            writeln!(out, "  subcause: {s}").unwrap();
        }
        if let Some(s) = &self.feature_id {
            writeln!(out, "  feature_id: {s}").unwrap();
        }
        for x in &self.related_spans {
            writeln!(
                out,
                "  related: {}",
                related_json(x, JsonOrder::Lexicographic)
            )
            .unwrap();
        }
        for x in &self.cause_chain {
            writeln!(out, "  cause: {}", cause_json(x, JsonOrder::Lexicographic)).unwrap();
        }
        if let Some(t) = &self.unexpected_token {
            let v = format!(
                "{{\"expected_tokens\":[{}],\"expected_tokens_kind\":{},\"parser_context\":{},\"recognized_prefix\":{},\"unexpected_token\":{}}}",
                self.expected_tokens
                    .iter()
                    .map(|x| q(x))
                    .collect::<Vec<_>>()
                    .join(","),
                self.expected_tokens_kind
                    .map(|x| q(x.as_str()))
                    .unwrap_or_else(|| "null".into()),
                opt_str(&self.parser_context),
                self.recognized_prefix
                    .as_ref()
                    .map(|s| span_json(s, JsonOrder::Lexicographic))
                    .unwrap_or_else(|| "null".into()),
                token_json(t, JsonOrder::Lexicographic)
            );
            writeln!(out, "  syntax: {v}").unwrap();
        }
        if !self.data.is_empty() {
            writeln!(out, "  data: {}", data_object_json(&self.data)).unwrap();
        }
        for x in &self.repair {
            writeln!(
                out,
                "  repair: {}",
                repair_json(x, JsonOrder::Lexicographic)
            )
            .unwrap();
        }
        Ok(out)
    }
}

fn related_key(x: &RelatedSpan) -> (&str, u64, u64, &str) {
    (&x.span.file, x.span.start, x.span.end, x.role.as_str())
}
fn valid_entity_value(value: &DataValue) -> bool {
    let DataValue::Object(fields) = value else {
        return false;
    };
    if fields.len() != 2 {
        return false;
    }
    matches!(fields.get("kind"), Some(DataValue::String(kind)) if ["function", "parameter", "binding", "temporary", "loan", "intrinsic", "type"].contains(&kind.as_str()))
        && matches!(fields.get("id"), Some(DataValue::String(id)) if !id.is_empty())
}
fn valid_span_value(value: &DataValue) -> bool {
    let DataValue::Object(fields) = value else {
        return false;
    };
    fields.len() == 3
        && matches!(fields.get("file"), Some(DataValue::String(_)))
        && matches!(fields.get("start"), Some(DataValue::UInt(_)))
        && matches!(fields.get("end"), Some(DataValue::UInt(_)))
}
fn span_value_matches_source(value: &DataValue, file: &str, source: SourceView<'_>) -> bool {
    let DataValue::Object(fields) = value else {
        return false;
    };
    let (
        Some(DataValue::String(span_file)),
        Some(DataValue::UInt(start)),
        Some(DataValue::UInt(end)),
    ) = (fields.get("file"), fields.get("start"), fields.get("end"))
    else {
        return false;
    };
    if span_file != file || start > end || *end > source.bytes_len as u64 {
        return false;
    }
    source.text.is_none_or(|text| {
        usize::try_from(*start)
            .ok()
            .zip(usize::try_from(*end).ok())
            .is_some_and(|(s, e)| text.is_char_boundary(s) && text.is_char_boundary(e))
    })
}

fn valid_diagnostic_type(value: &str, expected: bool) -> bool {
    if expected
        && [
            "domain:copyable_data",
            "domain:array",
            "domain:mock_resource",
            "domain:result",
        ]
        .contains(&value)
    {
        return true;
    }
    const MAX_DIAGNOSTIC_TYPE_DEPTH: usize = 128;
    fn parse(bytes: &[u8], index: &mut usize, depth: usize) -> bool {
        if depth > MAX_DIAGNOSTIC_TYPE_DEPTH {
            return false;
        }
        let rest = &bytes[*index..];
        for (name, compound) in [
            (b"i64".as_slice(), false),
            (b"bool", false),
            (b"()", false),
            (b"MockResource", false),
            (b"MockBlockingResource", false),
        ] {
            if rest.starts_with(name) {
                *index += name.len();
                return !compound;
            }
        }
        if rest.starts_with(b"Array<") {
            *index += 6;
            return parse(bytes, index, depth + 1) && take(bytes, index, b">");
        }
        if rest.starts_with(b"Result<") {
            *index += 7;
            return parse(bytes, index, depth + 1)
                && take(bytes, index, b",")
                && parse(bytes, index, depth + 1)
                && take(bytes, index, b">");
        }
        false
    }
    fn take(bytes: &[u8], index: &mut usize, expected: &[u8]) -> bool {
        if bytes.get(*index..*index + expected.len()) == Some(expected) {
            *index += expected.len();
            true
        } else {
            false
        }
    }
    let mut index = 0;
    parse(value.as_bytes(), &mut index, 0) && index == value.len()
}
fn q(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) <= 0x1f => write!(o, "\\u{:04x}", c as u32).unwrap(),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}
fn opt_str(x: &Option<String>) -> String {
    x.as_ref().map(|s| q(s)).unwrap_or_else(|| "null".into())
}
fn span_json(x: &Span, order: JsonOrder) -> String {
    match order {
        JsonOrder::Schema => format!(
            "{{\"file\":{},\"start\":{},\"end\":{}}}",
            q(&x.file),
            x.start,
            x.end
        ),
        JsonOrder::Lexicographic => format!(
            "{{\"end\":{},\"file\":{},\"start\":{}}}",
            x.end,
            q(&x.file),
            x.start
        ),
    }
}
fn opt_span(x: &Option<Span>) -> String {
    x.as_ref()
        .map(|s| span_json(s, JsonOrder::Schema))
        .unwrap_or_else(|| "null".into())
}
fn related_json(x: &RelatedSpan, order: JsonOrder) -> String {
    format!(
        "{{\"role\":{},\"span\":{}}}",
        q(x.role.as_str()),
        span_json(&x.span, order)
    )
}
fn entity_json(x: &Entity, order: JsonOrder) -> String {
    match order {
        JsonOrder::Schema => format!("{{\"kind\":{},\"id\":{}}}", q(x.kind.as_str()), q(&x.id)),
        JsonOrder::Lexicographic => {
            format!("{{\"id\":{},\"kind\":{}}}", q(&x.id), q(x.kind.as_str()))
        }
    }
}
fn cause_json(x: &Cause, order: JsonOrder) -> String {
    if order == JsonOrder::Lexicographic {
        return format!(
            "{{\"entity\":{},\"evidence\":{},\"kind\":{},\"operation\":{},\"span\":{}}}",
            x.entity
                .as_ref()
                .map(|e| entity_json(e, order))
                .unwrap_or_else(|| "null".into()),
            q(x.evidence.as_str()),
            q(x.kind.as_str()),
            q(x.operation.as_str()),
            x.span
                .as_ref()
                .map(|s| span_json(s, order))
                .unwrap_or_else(|| "null".into())
        );
    }
    format!(
        "{{\"kind\":{},\"operation\":{},\"span\":{},\"entity\":{},\"evidence\":{}}}",
        q(x.kind.as_str()),
        q(x.operation.as_str()),
        opt_span(&x.span),
        x.entity
            .as_ref()
            .map(|e| entity_json(e, order))
            .unwrap_or_else(|| "null".into()),
        q(x.evidence.as_str())
    )
}
fn token_json(x: &Token, order: JsonOrder) -> String {
    if order == JsonOrder::Lexicographic {
        return format!(
            "{{\"kind\":{},\"span\":{},\"text\":{}}}",
            q(x.kind.as_str()),
            span_json(&x.span, order),
            opt_str(&x.text)
        );
    }
    format!(
        "{{\"kind\":{},\"text\":{},\"span\":{}}}",
        q(x.kind.as_str()),
        opt_str(&x.text),
        span_json(&x.span, order)
    )
}
fn edit_json(x: &Edit) -> String {
    format!(
        "{{\"span\":{},\"expected_text\":{},\"replacement\":{}}}",
        span_json(&x.span, JsonOrder::Schema),
        q(&x.expected_text),
        q(&x.replacement)
    )
}
fn repair_json(x: &Repair, order: JsonOrder) -> String {
    if order == JsonOrder::Lexicographic {
        let edits = x
            .edits
            .iter()
            .map(|edit| {
                format!(
                    "{{\"expected_text\":{},\"replacement\":{},\"span\":{}}}",
                    q(&edit.expected_text),
                    q(&edit.replacement),
                    span_json(&edit.span, order)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let validation = x
            .validation
            .as_ref()
            .map(|v| {
                format!(
                    "{{\"mode\":{},\"profile_id\":{},\"result\":{}}}",
                    q(v.mode.as_str()),
                    q(&v.profile_id),
                    q(v.result.as_str())
                )
            })
            .unwrap_or_else(|| "null".into());
        return format!(
            "{{\"consequences\":[{}],\"description\":{},\"edits\":[{}],\"id\":{},\"status\":{},\"validation\":{}}}",
            x.consequences
                .iter()
                .map(|s| q(s))
                .collect::<Vec<_>>()
                .join(","),
            q(&x.description),
            edits,
            q(&x.id),
            q(x.status.as_str()),
            validation
        );
    }
    format!(
        "{{\"id\":{},\"status\":{},\"description\":{},\"edits\":[{}],\"validation\":{},\"consequences\":[{}]}}",
        q(&x.id),
        q(x.status.as_str()),
        q(&x.description),
        x.edits.iter().map(edit_json).collect::<Vec<_>>().join(","),
        x.validation
            .as_ref()
            .map(|v| format!(
                "{{\"profile_id\":{},\"mode\":{},\"result\":{}}}",
                q(&v.profile_id),
                q(v.mode.as_str()),
                q(v.result.as_str())
            ))
            .unwrap_or_else(|| "null".into()),
        x.consequences
            .iter()
            .map(|v| q(v))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn data_json(x: &DataValue) -> String {
    match x {
        DataValue::Null => "null".into(),
        DataValue::Bool(b) => b.to_string(),
        DataValue::UInt(n) => n.to_string(),
        DataValue::String(s) => q(s),
        DataValue::Array(a) => format!(
            "[{}]",
            a.iter().map(data_json).collect::<Vec<_>>().join(",")
        ),
        DataValue::Object(m) => data_object_json(m),
    }
}
fn data_object_json(x: &BTreeMap<String, DataValue>) -> String {
    format!(
        "{{{}}}",
        x.iter()
            .map(|(k, v)| format!("{}:{}", q(k), data_json(v)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn comma_join<T>(out: &mut String, xs: &[T], f: impl Fn(&T) -> String) {
    for (i, x) in xs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&f(x));
    }
}
fn line_column(s: &str, offset: usize) -> (usize, usize) {
    let p = &s[..offset];
    let line = p.bytes().filter(|b| *b == b'\n').count() + 1;
    let col = p.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, col)
}
fn rule(code: &str, subcause: Option<&str>) -> String {
    subcause
        .map(|s| format!("{code}/{s}"))
        .unwrap_or_else(|| code.to_owned())
}

/// Reads at most limit + 1 bytes so an oversized file is detected before UTF-8 decoding.
pub fn read_bounded(reader: impl Read) -> Result<Vec<u8>, InputError> {
    let limit = SOURCE_LIMIT;
    let mut bytes = Vec::with_capacity(limit.saturating_add(1).min(8192));
    reader
        .take(limit.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| InputError::Unreadable(e.to_string()))?;
    if bytes.len() > limit {
        return Err(InputError::TooLarge {
            limit,
            observed: limit.saturating_add(1),
        });
    }
    Ok(bytes)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputError {
    Unreadable(String),
    TooLarge {
        limit: usize,
        observed: usize,
    },
    InvalidUtf8 {
        start: usize,
        end: usize,
        bytes_hex: String,
        message: String,
    },
}
pub fn decode_utf8(bytes: Vec<u8>) -> Result<String, InputError> {
    String::from_utf8(bytes).map_err(|e| {
        let start = e.utf8_error().valid_up_to();
        let end = e
            .utf8_error()
            .error_len()
            .map(|n| start + n)
            .unwrap_or(e.as_bytes().len());
        let hex = e.as_bytes()[start..end]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        InputError::InvalidUtf8 {
            start,
            end,
            bytes_hex: hex,
            message: e.to_string(),
        }
    })
}

/// Input feature IDs explicitly published for NX-E0006/f0-0002-v1.
pub const FEATURE_IDS: &[&str] = &[
    "async",
    "attributes",
    "closure",
    "compound_assignment",
    "comptime",
    "default_argument",
    "extensions",
    "float_literal",
    "for_iteration",
    "function_value",
    "generator",
    "inferred_non_unit_result",
    "legacy_move",
    "loop_control",
    "match",
    "methods",
    "modules_visibility",
    "no_heap_alloc",
    "option",
    "partial_transfer",
    "provenance_clauses",
    "range",
    "receiver_mode",
    "region",
    "resource_array",
    "text",
    "traits_generics",
    "tuple",
    "unsupported_type",
    "user_types",
];
fn feature_known(s: &str) -> bool {
    FEATURE_IDS.contains(&s)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explanation {
    pub subcause: Option<String>,
    pub explanation_id: String,
    pub availability: &'static str,
    pub description: String,
    pub negative_source: String,
    pub positive_source: String,
    pub example_profile_id: Option<String>,
    pub feature_ids: Vec<String>,
    pub repair_consequences: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplainError {
    UnknownCode,
    UnknownSubcause,
    InvalidArguments,
}

/// Looks up only identities present in the canonical ADR tables. Catalog text is
/// embedded at build time so explain never reads the working directory or network.
pub fn explain(code: &str, subcause: Option<&str>) -> Result<Vec<Explanation>, ExplainError> {
    if code == "NX-E0006" {
        if subcause.is_some() {
            return Err(ExplainError::UnknownSubcause);
        }
        let feature_availability = availability(code, None);
        return Ok(FEATURE_IDS
            .iter()
            .map(|id| Explanation {
                subcause: None,
                explanation_id: code.into(),
                availability: feature_availability,
                description: feature_description(id),
                negative_source: feature_negative_source(id),
                positive_source: feature_positive_source(id),
                example_profile_id: Some(PROFILE_ID.into()),
                feature_ids: vec![(*id).into()],
                // §13.6/§13.10 publish no repair consequences per NX-E0006 feature.
                repair_consequences: vec![],
            })
            .collect());
    }
    let desc = catalog_entry(code)
        .ok_or(ExplainError::UnknownCode)?
        .description
        .to_owned();
    let causes = catalog_subcauses(code);
    if causes.is_empty() {
        if subcause.is_some() {
            return Err(ExplainError::UnknownSubcause);
        }
        return Ok(vec![explanation_for(code, None, desc)]);
    }
    if let Some(wanted) = subcause {
        if !causes.iter().any(|s| s == wanted) {
            return Err(ExplainError::UnknownSubcause);
        }
        Ok(vec![explanation_for(code, Some(wanted), desc)])
    } else {
        let mut causes = causes;
        causes.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        Ok(causes
            .iter()
            .map(|s| explanation_for(code, Some(s), desc.clone()))
            .collect())
    }
}

#[derive(Clone, Copy)]
struct FeatureFixture {
    id: &'static str,
    description: &'static str,
    negative: &'static str,
    #[allow(dead_code)] // §14.6 edit examples are not published repair consequences.
    repair: &'static str,
}

// Literal F01-F33 rows from section 14.6, keyed by the catalog feature identity.
static FEATURE_FIXTURES: &[FeatureFixture] = &[
    FeatureFixture {
        id: "legacy_move",
        description: "Operador provisional move de F0-0001; consume es la migración candidata, sujeta a recomprobación.",
        negative: "fn main(){let r=mock_open(1);let s=move r;}",
        repair: "Cambiar `move` por `consume`",
    },
    FeatureFixture {
        id: "inferred_non_unit_result",
        description: "Función sin anotación de resultado que requiere inferir un resultado distinto de unidad.",
        negative: "fn f(){return 1;} fn main(){}",
        repair: "Añadir `->i64` después de `f()`",
    },
    FeatureFixture {
        id: "float_literal",
        description: "Literal decimal reconocido; semántica flotante no implementada.",
        negative: "fn main(){let x=0.5;}",
        repair: "Sustituir el literal por `0`",
    },
    FeatureFixture {
        id: "range",
        description: "Rango reconocido mediante .., sin ejecución de rangos en el perfil.",
        negative: "fn main(){let n=1;let x=0..n;}",
        repair: "Sustituir `..` por `+`",
    },
    FeatureFixture {
        id: "tuple",
        description: "Tipo, expresión o patrón de producto distinto de unidad/agrupación.",
        negative: "fn main(){let x=(1,2);}",
        repair: "Quitar `,2`; queda agrupación `(1)`",
    },
    FeatureFixture {
        id: "attributes",
        description: "Atributos, incluido @derive.",
        negative: "@derive(JsonEncode) fn main(){}",
        repair: "Quitar `@derive(JsonEncode)`",
    },
    FeatureFixture {
        id: "default_argument",
        description: "Valor por defecto de parámetro.",
        negative: "fn f(x:i64=1){} fn main(){f(1);}",
        repair: "Quitar `=1`",
    },
    FeatureFixture {
        id: "closure",
        description: "Literal de closure y sus capturas.",
        negative: "fn main(){let f=fn[](){return ();};}",
        repair: "Sustituir el inicializador completo por `()`",
    },
    FeatureFixture {
        id: "function_value",
        description: "Uso almacenado, retornado o pasado de una función como valor.",
        negative: "fn f(){} fn main(){let g=f;}",
        repair: "Sustituir `let g=f;` por `f();`",
    },
    FeatureFixture {
        id: "receiver_mode",
        description: "Prefijo inout/consume aplicado al receptor de una llamada completa.",
        negative: "fn f(){} fn main(){consume f();}",
        repair: "Quitar el prefijo `consume`",
    },
    FeatureFixture {
        id: "partial_transfer",
        description: "Transferencia consume de una proyección, en vez de un binding completo.",
        negative: "fn main(){let a=[1];let x=consume a[0];}",
        repair: "Quitar `consume`; se copia el elemento",
    },
    FeatureFixture {
        id: "resource_array",
        description: "Array cuyo elemento contiene transitivamente un recurso.",
        negative: "fn main(){let a=[mock_open(1)];}",
        repair: "Quitar los corchetes del inicializador",
    },
    FeatureFixture {
        id: "no_heap_alloc",
        description: "Contrato reconocido, fuera del conjunto implementado por este perfil.",
        negative: "fn main() with no_heap_alloc{}",
        repair: "Sustituir por `no_alloc`",
    },
    FeatureFixture {
        id: "match",
        description: "Selección por patrones; pending_scope = F0-0003 identifica el diseño de la selección necesaria para next/Option, sin prometer toda la selección por patrones de la base en ese corte.",
        negative: "fn main(){match true{true=>{},false=>{}}}",
        repair: "Sustituir el `match` completo por `if true{}else{}`",
    },
    FeatureFixture {
        id: "option",
        description: "Tipo o constructores intrínsecos de Option.",
        negative: "fn f(x:Option<i64>){} fn main(){}",
        repair: "Sustituir `Option<i64>` por `Result<i64,i64>`",
    },
    FeatureFixture {
        id: "user_types",
        description: "Struct, enum o alias declarado por el usuario.",
        negative: "struct S{} fn main(){}",
        repair: "Quitar `struct S{}`",
    },
    FeatureFixture {
        id: "traits_generics",
        description: "Traits, impl, genéricos, where o despacho dinámico.",
        negative: "trait T{} fn main(){}",
        repair: "Quitar `trait T{}`",
    },
    FeatureFixture {
        id: "modules_visibility",
        description: "Mod, use, pub o rutas de módulos adicionales fuera del contexto de tipo descrito abajo.",
        negative: "use std.app;fn main(){}",
        repair: "Quitar `use std.app;`",
    },
    FeatureFixture {
        id: "extensions",
        description: "Extend, use extension o llamada cualificada de extensión.",
        negative: "extend i64{} fn main(){}",
        repair: "Quitar `extend i64{}`",
    },
    FeatureFixture {
        id: "provenance_clauses",
        description: "From, result_from o retains escritos.",
        negative: "fn f(x:i64)->i64 from(x){return x;} fn main(){}",
        repair: "Quitar `from(x)`",
    },
    FeatureFixture {
        id: "comptime",
        description: "Evaluación o constantes de módulo mediante comptime.",
        negative: "const X:i64=comptime 1;fn main(){}",
        repair: "Quitar la declaración de constante completa",
    },
    FeatureFixture {
        id: "text",
        description: "Cadenas e interpolación.",
        negative: "fn main(){let x=\"hola\";}",
        repair: "Sustituir la cadena por `1`",
    },
    FeatureFixture {
        id: "methods",
        description: "Selección o invocación de métodos/campos fuera de las proyecciones indexadas del perfil; self como expresión de receptor reconocida, con span de ese token y etapa parse. En posición que exige Name, self conserva NX-E0002.",
        negative: "fn main(){let a=[1];a.foo();}",
        repair: "Sustituir `a.foo()` por `len(a)`",
    },
    FeatureFixture {
        id: "compound_assignment",
        description: "Asignación compuesta, incluido +=.",
        negative: "fn main(){var x=1;x+=1;}",
        repair: "Sustituir `x+=1;` por `x=x+1;`",
    },
    FeatureFixture {
        id: "for_iteration",
        description: "For y su protocolo de iteración.",
        negative: "fn main(){for n in [1]{}}",
        repair: "Sustituir el bucle completo por `while false{}`",
    },
    FeatureFixture {
        id: "loop_control",
        description: "Break o continue.",
        negative: "fn main(){while false{break;}}",
        repair: "Quitar `break;`",
    },
    FeatureFixture {
        id: "generator",
        description: "Gen fn, yield y operaciones de generador.",
        negative: "gen fn g(){} fn main(){}",
        repair: "Quitar `gen`",
    },
    FeatureFixture {
        id: "async",
        description: "Async, await y tipos u operaciones reconocidos de tareas/futuros.",
        negative: "async fn f(){} fn main(){}",
        repair: "Quitar `async`",
    },
    FeatureFixture {
        id: "unsupported_type",
        description: "Ruta de tipo reconocida que no pertenece a la lista aceptada ni tiene una exclusión más específica; incluye u32, u64, f32, String y app.Error.",
        negative: "fn f(x:u32){} fn main(){}",
        repair: "Sustituir `u32` por `i64`",
    },
    FeatureFixture {
        id: "unsupported_type",
        description: "Ruta de tipo reconocida que no pertenece a la lista aceptada ni tiene una exclusión más específica; incluye u32, u64, f32, String y app.Error.",
        negative: "fn f(x:String){} fn main(){}",
        repair: "Sustituir `String` por `Array<i64>`",
    },
    FeatureFixture {
        id: "unsupported_type",
        description: "Ruta de tipo reconocida que no pertenece a la lista aceptada ni tiene una exclusión más específica; incluye u32, u64, f32, String y app.Error.",
        negative: "fn f(x:app.Error){} fn main(){}",
        repair: "Sustituir la ruta completa por `i64`",
    },
    FeatureFixture {
        id: "region",
        description: "Regiones y su grupo de tareas.",
        negative: "fn main(){region r{}}",
        repair: "Quitar `region r`; queda un bloque ordinario",
    },
    FeatureFixture {
        id: "methods",
        description: "Selección o invocación de métodos/campos fuera de las proyecciones indexadas del perfil; self como expresión de receptor reconocida, con span de ese token y etapa parse. En posición que exige Name, self conserva NX-E0002.",
        negative: "fn main(){let x=self;}",
        repair: "Sustituir `self` por `0`",
    },
];
fn feature_description(id: &str) -> String {
    FEATURE_FIXTURES
        .iter()
        .find(|f| f.id == id)
        .map(|f| f.description.to_owned())
        .unwrap_or_default()
}
fn feature_negative_source(id: &str) -> String {
    FEATURE_FIXTURES
        .iter()
        .find(|f| f.id == id)
        .map(|f| f.negative.to_owned())
        .unwrap_or_default()
}

fn feature_positive_source(id: &str) -> String {
    match id {
        "legacy_move" => "fn main(){let r=mock_open(1);let s=consume r;}".into(),
        "inferred_non_unit_result" => "fn f()->i64{return 1;} fn main(){}".into(),
        "float_literal" => "fn main(){let x=0;}".into(),
        "range" => "fn main(){let n=1;let x=0+n;}".into(),
        "tuple" => "fn main(){let x=(1);}".into(),
        "attributes" => " fn main(){}".into(),
        "default_argument" => "fn f(x:i64){} fn main(){f(1);}".into(),
        "closure" => "fn main(){let f=();}".into(),
        "function_value" => "fn f(){} fn main(){f();}".into(),
        "receiver_mode" => "fn f(){} fn main(){ f();}".into(),
        "partial_transfer" => "fn main(){let a=[1];let x= a[0];}".into(),
        "resource_array" => "fn main(){let a=mock_open(1);}".into(),
        "no_heap_alloc" => "fn main() with no_alloc{}".into(),
        "match" => "fn main(){if true{}else{}}".into(),
        "option" => "fn f(x:Result<i64,i64>){} fn main(){}".into(),
        "user_types" => " fn main(){}".into(),
        "traits_generics" => " fn main(){}".into(),
        "modules_visibility" => "fn main(){}".into(),
        "extensions" => " fn main(){}".into(),
        "provenance_clauses" => "fn f(x:i64)->i64 {return x;} fn main(){}".into(),
        "comptime" => "fn main(){}".into(),
        "text" => "fn main(){let x=1;}".into(),
        "methods" => "fn main(){let a=[1];len(a);}".into(),
        "compound_assignment" => "fn main(){var x=1;x=x+1;}".into(),
        "for_iteration" => "fn main(){while false{}}".into(),
        "loop_control" => "fn main(){while false{}}".into(),
        "generator" => " fn g(){} fn main(){}".into(),
        "async" => " fn f(){} fn main(){}".into(),
        "unsupported_type" => "fn f(x:i64){} fn main(){}".into(),
        "region" => "fn main(){{}}".into(),
        _ => String::new(),
    }
}

fn explanation_for(code: &str, subcause: Option<&str>, description: String) -> Explanation {
    let (negative_source, positive_source) = if code == "NX-E0003" && subcause.is_none() {
        let mut negative = String::from("fn main(){var x=0;x=");
        negative.push_str(&"[".repeat(200));
        negative.push('1');
        negative.push_str(&"]".repeat(200));
        negative.push_str(";}");
        (negative, "fn main(){var x=0;x=0;}".to_owned())
    } else {
        let (negative, positive) = published_source(code, subcause);
        (negative.to_owned(), positive.to_owned())
    };
    let available = availability(code, subcause);
    let example_profile_id =
        (available == "in_profile" && !negative_source.is_empty()).then(|| PROFILE_ID.into());
    Explanation {
        subcause: subcause.map(str::to_owned),
        explanation_id: rule(code, subcause),
        availability: available,
        description,
        negative_source,
        positive_source,
        example_profile_id,
        feature_ids: vec![],
        repair_consequences: if code == "NX-E0005" {
            vec!["Termina el binding antes del cierre del bloque.".into()]
        } else {
            vec![]
        },
    }
}

fn published_source(code: &str, subcause: Option<&str>) -> (&'static str, &'static str) {
    match (code, subcause) {
        ("NX-E0111", Some("missing_argument")) => (
            "fn f(x:i64){} fn main(){f();}",
            "fn f(x:i64){} fn main(){f(1);}",
        ),
        ("NX-E0111", Some("excess_positional_argument")) => (
            "fn f(x:i64){} fn main(){f(1,2);}",
            "fn f(x:i64){} fn main(){f(1);}",
        ),
        ("NX-E0111", Some("unknown_argument_label")) => (
            "fn f(x:i64){} fn main(){f(y:1);}",
            "fn f(x:i64){} fn main(){f(x:1);}",
        ),
        ("NX-E0111", Some("duplicate_argument")) => (
            "fn f(x:i64){} fn main(){f(x:1,x:2);}",
            "fn f(x:i64){} fn main(){f(x:1);}",
        ),
        ("NX-E0111", Some("positional_after_named")) => (
            "fn f(x:i64,y:i64){} fn main(){f(x:1,2);}",
            "fn f(x:i64,y:i64){} fn main(){f(x:1,y:2);}",
        ),
        ("NX-E0111", Some("argument_mode_mismatch")) => (
            "fn f(inout x:i64){} fn main(){var x=1;f(x);}",
            "fn f(inout x:i64){} fn main(){var x=1;f(inout x);}",
        ),
        ("NX-E0111", Some("non_callable_target")) => {
            ("fn main(){let f=1;f();}", "fn f(){} fn main(){f();}")
        }
        ("NX-E0112", Some("missing_main")) => ("fn f(){}", "fn f(){} fn main(){}"),
        ("NX-E0112", Some("invalid_main_signature")) => ("fn main(x:i64){}", "fn main(){}"),
        ("NX-E0105", None) => (
            "fn f(inout x:i64){} fn main(){let x=1;f(inout x);}",
            "fn f(inout x:i64){} fn main(){var x=1;f(inout x);}",
        ),
        ("NX-E0113", None) => (
            "fn main(){let r=Ok(1);}",
            "fn main(){let r:Result<i64,i64>=Ok(1);}",
        ),
        ("NX-E0103", None) => ("fn main(){let xs=[];}", "fn main(){let xs:Array<i64>=[];}"),
        ("NX-E1317", None) => (
            "fn f()->i64 { } fn main(){}",
            "fn f()->i64 { return 1;} fn main(){}",
        ),
        ("NX-E0102", Some("type_mismatch")) => (
            "fn f()->i64{return true;} fn main(){}",
            "fn f()->i64{return 1;} fn main(){}",
        ),
        ("NX-E0102", Some("not_indexable")) => {
            ("fn main(){let x=1[0];}", "fn main(){let x=[1][0];}")
        }
        ("NX-E0102", Some("resource_required")) => (
            "fn main(){let x=1;inspect(x);}",
            "fn main(){let x=mock_open(1);inspect(x);}",
        ),
        ("NX-E0107", None) => (
            "fn main() { let x = 9223372036854775808; }",
            "fn main() { let x = 9223372036854775807; }",
        ),
        ("NX-E0104", None) => (
            "fn f(){} fn f(){} fn main(){}",
            "fn f(){} fn g(){} fn main(){}",
        ),
        ("NX-E0101", None) => (
            "fn main(){ausente();}",
            "fn ausente(){} fn main(){ausente();}",
        ),
        ("NX-E0002", None) => ("fn main(){let match=1;}", "fn main(){let match_value=1;}"),
        ("NX-E0201", None) => (
            "fn main(){let r=mock_open(1);let s=r;}",
            "fn main(){let r=mock_open(1);let s=consume r;}",
        ),
        ("NX-E0202", Some("definitely_consumed")) => (
            "fn main(){let r=mock_open(1);let s=consume r;inspect(r);}",
            "fn main(){let r=mock_open(1);let s=consume r;inspect(s);}",
        ),
        ("NX-E0202", Some("maybe_consumed")) => (
            "fn f(b:bool){let r=mock_open(1);if b{drop(r);}inspect(r);} fn main(){}",
            "fn f(b:bool){let r=mock_open(1);if b{drop(r);return;}inspect(r);} fn main(){}",
        ),
        ("NX-E1105", Some("permission_escalation")) => (
            "fn f(x:MockResource)->MockResource{return consume x;} fn main(){}",
            "fn f(consume x:MockResource)->MockResource{return consume x;} fn main(){}",
        ),
        ("NX-E1105", Some("overlapping_mutable_borrows")) => (
            "fn f(inout x:i64,inout y:i64){} fn main(){var x=1;f(inout x,inout x);}",
            "fn f(inout x:i64,inout y:i64){} fn main(){var x=1;var y=2;f(inout x,inout y);}",
        ),
        ("NX-E1105", Some("active_borrow_conflict")) => (
            "fn f(x:i64,inout y:i64){} fn main(){var x=1;f(x,inout x);}",
            "fn f(x:i64,inout y:i64){} fn main(){var x=1;f(x+0,inout x);}",
        ),
        ("NX-E1302", Some("missing_else")) => (
            "fn f(b:bool)->i64{return if b{value:1;};} fn main(){}",
            "fn f(b:bool)->i64{return if b{value:1;}else{value:2;};} fn main(){}",
        ),
        ("NX-E1302", Some("missing_branch_value")) => (
            "fn f()->i64{return if true{1;}else{value:2;};} fn main(){}",
            "fn f()->i64{return if true{value:1;}else{value:2;};} fn main(){}",
        ),
        ("NX-E1302", Some("invalid_value_target")) => {
            ("fn main(){value:1;}", "fn main(){let value=1;}")
        }
        ("NX-E1302", Some("missing_value_operand")) => (
            "fn f()->i64{return if true{value:}else{value:2;};} fn main(){}",
            "fn f()->i64{return if true{value:1;}else{value:2;};} fn main(){}",
        ),
        ("NX-E1302", Some("non_final_branch_value")) => (
            "fn f()->i64{return if true{value:1;2;}else{value:2;};} fn main(){}",
            "fn f()->i64{return if true{value:1;}else{value:2;};} fn main(){}",
        ),
        ("NX-E1303", None) => (
            "fn f(){let n=if true{value:1;}else{value:true;};} fn main(){}",
            "fn f(){let n=if true{value:1;}else{value:2;};} fn main(){}",
        ),
        ("NX-E0417", Some("duplicate_operation")) => (
            "fn main() with {call(no_io),call(no_alloc)}{}",
            "fn main() with {call(no_io)}{}",
        ),
        ("NX-E0417", Some("invalid_operation")) => (
            "fn main() with {resume(no_io)}{}",
            "fn main() with {call(no_io)}{}",
        ),
        ("NX-E0417", Some("duplicate_restriction")) => {
            ("fn main() with no_io,no_io{}", "fn main() with no_io{}")
        }
        ("NX-E0417", Some("empty_operation_list")) => (
            "fn main() with {}{}",
            "fn main() with {call(unrestricted)}{}",
        ),
        ("NX-E0417", Some("unrestricted_mixed")) => (
            "fn main() with unrestricted,no_io{}",
            "fn main() with no_io{}",
        ),
        ("NX-E0412", None) => (
            "fn main() with no_alloc{let a=[1];}",
            "fn main() with no_alloc{let a=1;}",
        ),
        ("NX-E0413", None) => (
            "fn main() with no_block{let r=mock_blocking_open(1);}",
            "fn main() with no_block{let r=mock_open(1);}",
        ),
        ("NX-E0414", None) => (
            "fn main() with no_suspend{mock_unknown();}",
            "fn main() with no_suspend{assert_eq(1,1);}",
        ),
        ("NX-E0415", None) => (
            "fn hoja(){print(1);} fn medio(){hoja();} fn main() with no_io{medio();}",
            "fn hoja(){} fn medio(){hoja();} fn main() with no_io{medio();}",
        ),
        ("NX-E0001", None) => ("fn main(){let x=1/2;}", "fn main(){let x=1+2;}"),
        ("NX-E0005", None) => (
            "fn f()->i64{return 1 } fn main(){}",
            "fn f()->i64{return 1; } fn main(){}",
        ),
        ("NX-E0006", None) => ("fn main(){let x=0.5;}", "fn main(){let x=0; }"),
        _ => ("", ""),
    }
}

// §13.12 grouped coordinates not repeated in static catalog literals.
fn coordinates_by_code(code: &str, subcause: Option<&str>) -> Option<(&'static str, &'static str)> {
    Some(match code {
        "NX-E0001" => ("lex", "syntax.lex"),
        "NX-E0002" | "NX-E0005" => ("parse", "syntax.parse"),
        "NX-E0101" => ("names", "names.resolve"),
        "NX-E0102" | "NX-E1303" => ("types", "types.check"),
        "NX-E0103" | "NX-E0113" => ("types", "types.infer"),
        "NX-E0104" => ("names", "names.declare"),
        "NX-E0105" => ("ownership", "place.write"),
        "NX-E0107" => ("types", "value.literal"),
        "NX-E0111" => ("arguments", "call.bind"),
        "NX-E0112" => ("entry", "entry.check"),
        "NX-E0201" => ("ownership", "value.copy"),
        "NX-E0202" => ("ownership", "ownership.use"),
        "NX-E1105" => ("ownership", "ownership.access"),
        "NX-E1302" if subcause == Some("missing_branch_value") => ("ownership", "flow.branch"),
        "NX-E1302" => ("parse", "syntax.parse"),
        "NX-E1317" => ("ownership", "flow.return"),
        "NX-E0417" => ("parse", "syntax.parse"),
        _ => return None,
    })
}

fn availability(code: &str, subcause: Option<&str>) -> &'static str {
    let Some(entry) = catalog_entry(code) else {
        return "outside_profile";
    };
    if let Some(name) = subcause {
        let Some(cause) = entry.subcauses.iter().find(|cause| cause.name == name) else {
            return "outside_profile";
        };
        return cause.availability;
    }
    entry.availability
}

#[derive(Clone, Copy)]
struct CatalogSubcause {
    name: &'static str,
    availability: &'static str,
    stage: &'static str,
    operation: &'static str,
}
#[derive(Clone, Copy)]
struct CatalogEntry {
    code: &'static str,
    description: &'static str,
    subcauses: &'static [CatalogSubcause],
    availability: &'static str,
    stage: &'static str,
    operation: &'static str,
}

// Static transcription of the 70 canonical NX-E identities from F0-CODIGOS.md
// and its addendum. section 13.12 coordinates are attached to each identity/subcause.
// Empty coordinates mean no stage/operation is assigned here: for example section 13.12
// delegates effects to section 13.15 and NX-E0006 to its feature/context table.
static CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        code: "NX-E0001",
        description: "Token no reconocido por la gramática del corte.",
        subcauses: &[],
        availability: "in_profile",
        stage: "lex",
        operation: "syntax.lex",
    },
    CatalogEntry {
        code: "NX-E0002",
        description: "Secuencia que no satisface la producción sintáctica esperada.",
        subcauses: &[],
        availability: "in_profile",
        stage: "parse",
        operation: "syntax.parse",
    },
    CatalogEntry {
        code: "NX-E0003",
        description: "Presupuesto explícito de análisis excedido; budget_kind usa exclusivamente la enumeración cerrada de este catálogo. Subcause = null.",
        subcauses: &[],
        availability: "in_profile",
        stage: "by budget_kind",
        operation: "analysis.limit",
    },
    CatalogEntry {
        code: "NX-E0004",
        description: "source_unreadable: fallo de apertura/lectura de fuente; invalid_utf8: bytes leídos y de tamaño permitido que no decodifican como UTF-8.",
        subcauses: &[
            CatalogSubcause {
                name: "source_unreadable",
                availability: "in_profile",
                stage: "input",
                operation: "input.read",
            },
            CatalogSubcause {
                name: "invalid_utf8",
                availability: "in_profile",
                stage: "input",
                operation: "input.decode",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E0005",
        description: "Falta terminador de sentencia, declaración simple o requisito sin cuerpo; MissingTerminator reconoce la recuperación rechazada (D13 G.2). Token/span exigidos, reparación explícita y prioridad sobre errores de flujo derivados. Subcause = null.",
        subcauses: &[],
        availability: "in_profile",
        stage: "parse",
        operation: "syntax.parse",
    },
    CatalogEntry {
        code: "NX-E0006",
        description: "Construcción reconocida pero fuera del perfil experimental solicitado. Subcause = null. Exige profile_id y feature_id registrado para ese perfil; no acredita una prohibición del lenguaje completo ni una regla semántica todavía no implementada.",
        subcauses: &[],
        availability: "in_profile",
        stage: "by feature/context",
        operation: "by feature/context",
    },
    CatalogEntry {
        code: "NX-E0101",
        description: "Nombre de valor no declarado, incluido un binding usado como expresión o un destino de llamada no declarado. Subcause = null. En F0-0002, use_kind distingue value y call_target; un nombre resuelto a valor no invocable usa NX-E0111/non_callable_target.",
        subcauses: &[],
        availability: "in_profile",
        stage: "names",
        operation: "names.resolve",
    },
    CatalogEntry {
        code: "NX-E0102",
        description: "type_mismatch: tipo incompatible; not_indexable: indexación no aplicable; resource_required: operación que requiere recurso; field_requires_grouped_call: miembro campo usado en forma de llamada de método x.m(); la invocación del campo se escribe (x.m)() (D13 G.1, C52d/C52c).",
        subcauses: &[
            CatalogSubcause {
                name: "type_mismatch",
                availability: "in_profile",
                stage: "types",
                operation: "types.check",
            },
            CatalogSubcause {
                name: "not_indexable",
                availability: "in_profile",
                stage: "types",
                operation: "types.check",
            },
            CatalogSubcause {
                name: "resource_required",
                availability: "in_profile",
                stage: "types",
                operation: "types.check",
            },
            CatalogSubcause {
                name: "field_requires_grouped_call",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E0103",
        description: "Elemento de array vacío no inferible sin anotación.",
        subcauses: &[],
        availability: "in_profile",
        stage: "types",
        operation: "types.infer",
    },
    CatalogEntry {
        code: "NX-E0104",
        description: "Declaración duplicada en el mismo ámbito: binding, función o parámetro. Subcause = null. En F0-0002 exige declaration_kind = binding, function o parameter, span de la segunda declaración y primera declaración relacionada.",
        subcauses: &[],
        availability: "in_profile",
        stage: "names",
        operation: "names.declare",
    },
    CatalogEntry {
        code: "NX-E0105",
        description: "Acceso de modificación solicitado sobre binding inmutable, incluida asignación, receptor inout y match inout.",
        subcauses: &[],
        availability: "in_profile",
        stage: "ownership",
        operation: "place.write",
    },
    CatalogEntry {
        code: "NX-E0107",
        description: "Literal fuera del rango i64 del corte.",
        subcauses: &[],
        availability: "in_profile",
        stage: "types",
        operation: "value.literal",
    },
    CatalogEntry {
        code: "NX-E0110",
        description: "Una impl o derivación de usuario intenta afirmar una propiedad/protocolo reservado al compilador (incluidos Call* y SuspendSafe).",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E0111",
        description: "Asociación o destino de llamada inválidos: missing_argument, excess_positional_argument, unknown_argument_label, duplicate_argument, positional_after_named, argument_mode_mismatch, non_callable_target. Una incompatibilidad de tipos con asociación válida conserva NX-E0102/type_mismatch; copiar un argumento no copiable conserva NX-E0201.",
        subcauses: &[
            CatalogSubcause {
                name: "missing_argument",
                availability: "in_profile",
                stage: "arguments",
                operation: "call.bind",
            },
            CatalogSubcause {
                name: "excess_positional_argument",
                availability: "in_profile",
                stage: "arguments",
                operation: "call.bind",
            },
            CatalogSubcause {
                name: "unknown_argument_label",
                availability: "in_profile",
                stage: "arguments",
                operation: "call.bind",
            },
            CatalogSubcause {
                name: "duplicate_argument",
                availability: "in_profile",
                stage: "arguments",
                operation: "call.bind",
            },
            CatalogSubcause {
                name: "positional_after_named",
                availability: "in_profile",
                stage: "arguments",
                operation: "call.bind",
            },
            CatalogSubcause {
                name: "argument_mode_mismatch",
                availability: "in_profile",
                stage: "arguments",
                operation: "call.bind",
            },
            CatalogSubcause {
                name: "non_callable_target",
                availability: "in_profile",
                stage: "arguments",
                operation: "call.bind",
            },
        ],
        availability: "outside_profile",
        stage: "arguments",
        operation: "call.bind",
    },
    CatalogEntry {
        code: "NX-E0112",
        description: "Entrada solicitada no disponible: missing_main, invalid_main_signature. No se emite por ausencia de main al comprobar un módulo de biblioteca.",
        subcauses: &[
            CatalogSubcause {
                name: "missing_main",
                availability: "in_profile",
                stage: "entry",
                operation: "entry.check",
            },
            CatalogSubcause {
                name: "invalid_main_signature",
                availability: "in_profile",
                stage: "entry",
                operation: "entry.check",
            },
        ],
        availability: "outside_profile",
        stage: "entry",
        operation: "entry.check",
    },
    CatalogEntry {
        code: "NX-E0113",
        description: "No existe una determinación única de los tipos requeridos por un constructor de valor del subconjunto, después de aplicar el contexto local disponible. Subcause = null. No se inventa un tipo comodín. El array vacío conserva NX-E0103 y la firma de closure conserva NX-E1111.",
        subcauses: &[],
        availability: "in_profile",
        stage: "types",
        operation: "types.infer",
    },
    CatalogEntry {
        code: "NX-E0201",
        description: "Intento de copia lógica de valor no copiable, sea recurso, closure, futuro o agregado.",
        subcauses: &[],
        availability: "in_profile",
        stage: "ownership",
        operation: "value.copy",
    },
    CatalogEntry {
        code: "NX-E0202",
        description: "Uso que exige valor disponible tras consumo: definitely_consumed si se consumió en todos los caminos pertinentes; maybe_consumed si al menos una continuación lo consumió. Transferencia, drop y extracción consumidora son orígenes, no códigos diferentes.",
        subcauses: &[
            CatalogSubcause {
                name: "definitely_consumed",
                availability: "in_profile",
                stage: "ownership",
                operation: "ownership.use",
            },
            CatalogSubcause {
                name: "maybe_consumed",
                availability: "in_profile",
                stage: "ownership",
                operation: "ownership.use",
            },
        ],
        availability: "outside_profile",
        stage: "ownership",
        operation: "ownership.use",
    },
    CatalogEntry {
        code: "NX-E0203",
        description: "Limitación experimental: unsupported_move_position, unsupported_resource_position, unsupported_resource_rebinding, unsupported_transfer_kind. No acredita una prohibición del lenguaje completo.",
        subcauses: &[
            CatalogSubcause {
                name: "unsupported_move_position",
                availability: "historical",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "unsupported_resource_position",
                availability: "historical",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "unsupported_resource_rebinding",
                availability: "historical",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "unsupported_transfer_kind",
                availability: "historical",
                stage: "",
                operation: "",
            },
        ],
        availability: "historical",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E0412",
        description: "No se verifica el contrato declarado no_alloc. Procedencia: ejemplo de roadmap §6.9.",
        subcauses: &[],
        availability: "in_profile",
        stage: "effects",
        operation: "by section 13.15",
    },
    CatalogEntry {
        code: "NX-E0413",
        description: "Recurso con destructor bloqueante y contrato de efectos, con positivo pareado.",
        subcauses: &[],
        availability: "in_profile",
        stage: "effects",
        operation: "by section 13.15",
    },
    CatalogEntry {
        code: "NX-E0414",
        description: "Propuesto: no se verifica el contrato declarado no_suspend.",
        subcauses: &[],
        availability: "in_profile",
        stage: "effects",
        operation: "by section 13.15",
    },
    CatalogEntry {
        code: "NX-E0415",
        description: "Propuesto: no se verifica el contrato declarado no_io.",
        subcauses: &[],
        availability: "in_profile",
        stage: "effects",
        operation: "by section 13.15",
    },
    CatalogEntry {
        code: "NX-E0416",
        description: "Propuesto: no se verifica el contrato declarado no_heap_alloc.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E0417",
        description: "duplicate_operation, invalid_operation, duplicate_restriction, missing_body_operation, operation_list_required: estructura contextual de WithClause inválida; missing_with_clause: cota Call* sin with; empty_operation_list: mapa with {} sin operaciones; unrestricted_mixed: unrestricted acompañado por una restricción distinta en su lista. Las tres últimas formas se reconocen antes de rechazarlas. Missing_with_clause y empty_operation_list preceden a ausencias de fases; invalid_operation precede a missing_body_operation; las omisiones con código específico prevalecen sobre missing_body_operation. Una repetición precede a unrestricted_mixed.",
        subcauses: &[
            CatalogSubcause {
                name: "duplicate_operation",
                availability: "in_profile",
                stage: "parse",
                operation: "syntax.parse",
            },
            CatalogSubcause {
                name: "invalid_operation",
                availability: "in_profile",
                stage: "parse",
                operation: "syntax.parse",
            },
            CatalogSubcause {
                name: "duplicate_restriction",
                availability: "in_profile",
                stage: "parse",
                operation: "syntax.parse",
            },
            CatalogSubcause {
                name: "missing_body_operation",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "operation_list_required",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "missing_with_clause",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "empty_operation_list",
                availability: "in_profile",
                stage: "parse",
                operation: "syntax.parse",
            },
            CatalogSubcause {
                name: "unrestricted_mixed",
                availability: "in_profile",
                stage: "parse",
                operation: "syntax.parse",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1101",
        description: "Binding libre ausente de lista exhaustiva de capturas.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1102",
        description: "Binding capturado más de una vez en la misma lista.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1104",
        description: "Escape de préstamo retenido por una closure.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1105",
        description: "active_borrow_conflict: acceso incompatible con préstamo vivo; permission_escalation: derivar permiso más fuerte del recibido; overlapping_mutable_borrows: crear segundo préstamo mutable solapado.",
        subcauses: &[
            CatalogSubcause {
                name: "active_borrow_conflict",
                availability: "in_profile",
                stage: "ownership",
                operation: "ownership.access",
            },
            CatalogSubcause {
                name: "permission_escalation",
                availability: "in_profile",
                stage: "ownership",
                operation: "ownership.access",
            },
            CatalogSubcause {
                name: "overlapping_mutable_borrows",
                availability: "in_profile",
                stage: "ownership",
                operation: "ownership.access",
            },
        ],
        availability: "outside_profile",
        stage: "ownership",
        operation: "ownership.access",
    },
    CatalogEntry {
        code: "NX-E1107",
        description: "Modo explícito de receptor insuficiente para la invocación.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1108",
        description: "Actor/estado compartido anidado y requisito concreto de independencia.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1109",
        description: "LocalRc/recurso afín confiado, dominios local/migrable y transferencia.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1110",
        description: "Generador F2 que retenga una closure prestada válida al construir, sin escape previo.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1111",
        description: "Inferencia local de firma de closure sin solución única.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1112",
        description: "Firma pública invocable omite una cota de operación requerida por su gramática.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1113",
        description: "Argumento no implementa el protocolo invocable nominal exigido por la cota (por ejemplo, solo CallOnce ante Call).",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1114",
        description: "Cota invocable privada escrita omite call obligatorio.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1201",
        description: "Await fuera de cuerpo async.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1202",
        description: "Firma pública async o tipo Future<T> escrito omite una de create/resume/cancel/drop. D13 reutiliza esta entrada; async local exige solo resume con NX-E0417/missing_body_operation.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1204",
        description: "No se deriva SuspendSafe para un campo/permiso retenido en suspensión, por incompatibilidad o contrato desconocido; property_evidence y cause_chain explican el fallo. suspension_kind = await o yield es dato, no otra regla.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1205",
        description: "Futuro/handle retiene una procedencia o grupo que no vive hasta su uso.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1206",
        description: "Mezcla excluida de async y generadores: async_generator_declaration, yield_in_async.",
        subcauses: &[
            CatalogSubcause {
                name: "async_generator_declaration",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "yield_in_async",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1208",
        description: "Transporte con retención externa sin protocolo de retiro.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1210",
        description: "Propagación ? aplicada a TaskOutcome sin tratar Completed/Cancelled.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1211",
        description: "Destructor que suspenda, sin contrato with que confunda la violación intrínseca.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1212",
        description: "request_cancel se dirige a un grupo distinto del propietario del handle.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1213",
        description: "Yield dentro de un cuerpo invocable ordinario: su invocable léxico más próximo no es gen fn ni async. Incluye closures ordinarias anidadas en gen. En async se conserva NX-E1206/yield_in_async.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1301",
        description: "Forma corta de closure reconocida por ImplicitClosureBody y rechazada por faltar return: implicit_closure_body; span del operando completo y => relacionado, nunca NX-E0002 por esa forma. Una expresión final analizada como sentencia válida no es esta forma: su caída sin retorno usa NX-E1317.",
        subcauses: &[CatalogSubcause {
            name: "implicit_closure_body",
            availability: "outside_profile",
            stage: "",
            operation: "",
        }],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1302",
        description: "missing_else: if expresión sin else; missing_branch_value: continuación sin value:; invalid_value_target: value: sin bloque destinatario directo; missing_value_operand: value: sin expresión, también por salto tras :; non_final_branch_value: value: no es la última sentencia directa de su rama. Value sin : es Name ordinario (D13 §1).",
        subcauses: &[
            CatalogSubcause {
                name: "missing_else",
                availability: "in_profile",
                stage: "parse",
                operation: "syntax.parse",
            },
            CatalogSubcause {
                name: "missing_branch_value",
                availability: "in_profile",
                stage: "ownership",
                operation: "flow.branch",
            },
            CatalogSubcause {
                name: "invalid_value_target",
                availability: "in_profile",
                stage: "parse",
                operation: "syntax.parse",
            },
            CatalogSubcause {
                name: "missing_value_operand",
                availability: "in_profile",
                stage: "parse",
                operation: "syntax.parse",
            },
            CatalogSubcause {
                name: "non_final_branch_value",
                availability: "in_profile",
                stage: "parse",
                operation: "syntax.parse",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1303",
        description: "Tipos de caminos normales no unificables conforme a igualdad nominal.",
        subcauses: &[],
        availability: "in_profile",
        stage: "types",
        operation: "types.check",
    },
    CatalogEntry {
        code: "NX-E1304",
        description: "non_exhaustive: faltan casos; unreachable_arm: todos los valores del patrón están cubiertos por la unión de patrones de brazos anteriores sin guarda; no exige un único brazo anterior que lo cubra (D13 §4).",
        subcauses: &[
            CatalogSubcause {
                name: "non_exhaustive",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "unreachable_arm",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1305",
        description: "Guarda no permitida: guard_effect, guard_mutation, guard_transfer.",
        subcauses: &[
            CatalogSubcause {
                name: "guard_effect",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "guard_mutation",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "guard_transfer",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1307",
        description: "Vista concreta que escape por una rama/error/wrapper.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1308",
        description: "private_access: acceso fuera del dominio privado; private_reexport: intento de elevar visibilidad mediante pub use.",
        subcauses: &[
            CatalogSubcause {
                name: "private_access",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "private_reexport",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1309",
        description: "alias_collision: alias local de declaraciones distintas; duplicate_import: mismo import repetido.",
        subcauses: &[
            CatalogSubcause {
                name: "alias_collision",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "duplicate_import",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1310",
        description: "unexported_type: tipo público sin ruta; missing_provenance: omisión de procedencia requerida en resultado público; missing_signature_type: tipo público omitido; missing_effect_bound: límite público ordinario omitido.",
        subcauses: &[
            CatalogSubcause {
                name: "unexported_type",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "missing_provenance",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "missing_signature_type",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "missing_effect_bound",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1311",
        description: "undeclared_dependency: uso transitivo no declarado; incompatible_versions: revisiones incompatibles; package_cycle: ciclo de paquetes.",
        subcauses: &[
            CatalogSubcause {
                name: "undeclared_dependency",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "incompatible_versions",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "package_cycle",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1312",
        description: "Paquete no es propietario autorizado del trait ni del tipo nominal exterior de impl.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1313",
        description: "Implementaciones canónicas potencialmente solapadas sin prueba de disjunción.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1314",
        description: "Múltiples candidatos de trait/extensión igualmente aplicables.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1315",
        description: "Método de extensión existente pero no activado explícitamente.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1316",
        description: "Inicializador ConstDecl de módulo no admisible como valor independiente del subconjunto comptime puro (incluidos recurso vivo o efectos).",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1317",
        description: "Función/closure ordinaria o async con resultado final distinto de unidad alcanza salida normal sin return. Excluye gen fn: -> T es Item y su final/bare return termina la secuencia (D13 §1.1).",
        subcauses: &[],
        availability: "in_profile",
        stage: "ownership",
        operation: "flow.return",
    },
    CatalogEntry {
        code: "NX-E1318",
        description: "Portador genérico y constructor de vista ordinaria con duración comprobable.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1319",
        description: "Retener procedencia prestada en un argumento/receptor propietario sin declarar retains: por almacenamiento directo o transitivamente mediante una llamada directa/indirecta cuyo contrato retiene (D13 §6, C47b).",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1320",
        description: "Cláusula from/result_from/retains escrita no cubre los orígenes o destino reales del cuerpo; aplica también en privado.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1321",
        description: "duplicate_module_file: mismo archivo canónico declarado dos veces; path_collision: rutas normalizadas distintas con igual clave portable, aunque el disco las identifique; module_file_missing: mod sin archivo requerido.",
        subcauses: &[
            CatalogSubcause {
                name: "duplicate_module_file",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "path_collision",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "module_file_missing",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1322",
        description: "Método duplicado en bloques extend del mismo módulo y tipo nominal.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1323",
        description: "alias_cycle: expansión circular de aliases; constant_cycle: dependencia circular de constantes; recursive_value_layout: tamaño de tipo recursivo sin indirección.",
        subcauses: &[
            CatalogSubcause {
                name: "alias_cycle",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "constant_cycle",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
            CatalogSubcause {
                name: "recursive_value_layout",
                availability: "outside_profile",
                stage: "",
                operation: "",
            },
        ],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
    CatalogEntry {
        code: "NX-E1324",
        description: "Return con operando dentro del cuerpo de generador; su tipo declarado es Item, no un resultado final de return. Incluye operando unidad.",
        subcauses: &[],
        availability: "outside_profile",
        stage: "",
        operation: "",
    },
];

fn catalog_entry(code: &str) -> Option<&'static CatalogEntry> {
    CATALOG.iter().find(|entry| entry.code == code)
}
fn catalog_subcauses(code: &str) -> Vec<String> {
    catalog_entry(code)
        .map(|e| e.subcauses.iter().map(|s| s.name.to_owned()).collect())
        .unwrap_or_default()
}

pub fn explanation_json(code: &str, entries: &[Explanation]) -> String {
    let mut out = format!(
        "{{\"schema_version\":1,\"diagnostic_namespace\":\"Nexo-proposed-v1\",\"code\":{},\"entries\":[",
        q(code)
    );
    for (i, e) in entries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write!(out,"{{\"subcause\":{},\"explanation_id\":{},\"availability\":{},\"description\":{},\"negative_source\":{},\"positive_source\":{},\"example_profile_id\":{},\"feature_ids\":[{}],\"repair_consequences\":[{}]}}",opt_str(&e.subcause),q(&e.explanation_id),q(e.availability),q(&e.description),q(&e.negative_source),q(&e.positive_source),opt_str(&e.example_profile_id),e.feature_ids.iter().map(|x|q(x)).collect::<Vec<_>>().join(","),e.repair_consequences.iter().map(|x|q(x)).collect::<Vec<_>>().join(",")).unwrap();
    }
    out.push_str("]}");
    out
}

pub fn explanation_text(entries: &[Explanation]) -> String {
    let mut out = String::new();
    for (i, e) in entries.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        writeln!(out,"{}\navailability: {}\nexample_profile_id: {}\nrule: {}\nnegative_source: {}\npositive_source: {}\nfeature_ids: {}\nrepair_consequences: {}",e.explanation_id,e.availability,opt_str(&e.example_profile_id),e.description,q(&e.negative_source),q(&e.positive_source),data_json(&DataValue::Array(e.feature_ids.iter().cloned().map(DataValue::String).collect())),data_json(&DataValue::Array(e.repair_consequences.iter().cloned().map(DataValue::String).collect()))).unwrap();
    }
    out
}

pub fn explain_error_json(
    error: ExplainError,
    code: Option<&str>,
    subcause: Option<&str>,
) -> String {
    let reason = match error {
        ExplainError::UnknownCode => "unknown_code",
        ExplainError::UnknownSubcause => "unknown_subcause",
        ExplainError::InvalidArguments => "invalid_arguments",
    };
    format!(
        "{{\"schema_version\":1,\"error\":{{\"reason\":{},\"code\":{},\"subcause\":{}}}}}",
        q(reason),
        code.map(q).unwrap_or_else(|| "null".into()),
        subcause.map(q).unwrap_or_else(|| "null".into())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Diagnostic {
        Diagnostic {
            schema_version: 1,
            diagnostic_namespace: Namespace::NexoProposedV1,
            profile_id: PROFILE_ID.into(),
            code: "NX-E0006".into(),
            subcause: None,
            rule: "NX-E0006".into(),
            explanation_id: Some("NX-E0006".into()),
            severity: Severity::Error,
            stage: Stage::Parse,
            message: "Literal flotante fuera del perfil.".into(),
            operation: Operation::SyntaxParse,
            primary_span: Some(Span {
                file: "decimal.nx".into(),
                start: 16,
                end: 19,
            }),
            related_spans: vec![],
            cause_chain: vec![],
            feature_id: Some("float_literal".into()),
            unexpected_token: None,
            expected_tokens: vec![],
            expected_tokens_kind: None,
            parser_context: None,
            recognized_prefix: None,
            repair: vec![],
            data: BTreeMap::from([(
                "pending_scope".into(),
                DataValue::String("later_adr".into()),
            )]),
        }
    }
    #[test]
    fn nx_json_matches_published_fixture_and_has_all_keys() {
        let s = "fn main(){let x=0.5;}";
        let d = sample();
        let json = d.to_json("decimal.nx", s).unwrap();
        assert_eq!(
            json,
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0002-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"parse","message":"Literal flotante fuera del perfil.","operation":"syntax.parse","primary_span":{"file":"decimal.nx","start":16,"end":19},"related_spans":[],"cause_chain":[],"feature_id":"float_literal","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"later_adr"}}"#
        );
        let keys = [
            "schema_version",
            "diagnostic_namespace",
            "profile_id",
            "code",
            "subcause",
            "rule",
            "explanation_id",
            "severity",
            "stage",
            "message",
            "operation",
            "primary_span",
            "related_spans",
            "cause_chain",
            "feature_id",
            "unexpected_token",
            "expected_tokens",
            "expected_tokens_kind",
            "parser_context",
            "recognized_prefix",
            "repair",
            "data",
        ];
        assert!(keys.iter().all(|key| json.contains(&format!("\"{key}\":"))));
    }
    #[test]
    fn runtime_schema_uses_its_closed_namespace_and_vocabulary() {
        let mut d = sample();
        d.diagnostic_namespace = Namespace::F0002Sim;
        d.code = "R0101".into();
        d.rule = "R0101".into();
        d.explanation_id = None;
        d.feature_id = None;
        d.stage = Stage::Runtime;
        d.operation = Operation::ValueAdd;
        d.primary_span = Some(Span {
            file: "decimal.nx".into(),
            start: 16,
            end: 19,
        });
        d.data.clear();
        let json = d.to_json("decimal.nx", "fn main(){let x=0.5;}").unwrap();
        assert!(json.contains("F0-0002-sim"));
        d.message = "Desbordamiento de suma i64.".into();
        d.operation = Operation::ValueAdd;
        d.primary_span = Some(Span {
            file: "suma.nx".into(),
            start: 16,
            end: 37,
        });
        let source = "fn main(){print(9223372036854775807+1);}";
        assert_eq!(
            d.to_json("suma.nx", source).unwrap(),
            r#"{"schema_version":1,"diagnostic_namespace":"F0-0002-sim","profile_id":"f0-0002-v1","code":"R0101","subcause":null,"rule":"R0101","explanation_id":null,"severity":"error","stage":"runtime","message":"Desbordamiento de suma i64.","operation":"value.add","primary_span":{"file":"suma.nx","start":16,"end":37},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}"#
        );
    }
    #[test]
    fn rejects_invalid_unicode_boundary_and_unregistered_vocab() {
        let mut d = sample();
        d.primary_span = Some(Span {
            file: "x".into(),
            start: 1,
            end: 2,
        });
        assert!(d.validate("x", "🙂").is_err());
        d = sample();
        d.feature_id = Some("invented".into());
        assert!(d.validate("decimal.nx", "fn main(){let x=0.5;}").is_err());
    }

    #[test]
    fn invalid_byte_spans_use_original_input_length() {
        let mut d = sample();
        d.code = "NX-E0003".into();
        d.rule = "NX-E0003".into();
        d.explanation_id = Some("NX-E0003".into());
        d.stage = Stage::Input;
        d.operation = Operation::AnalysisLimit;
        d.feature_id = None;
        d.subcause = None;
        d.primary_span = Some(Span {
            file: "x".into(),
            start: 1_048_576,
            end: 1_048_577,
        });
        d.data = BTreeMap::from([
            (
                "budget_kind".into(),
                DataValue::String("source_bytes".into()),
            ),
            ("limit".into(), DataValue::UInt(1_048_576)),
            ("observed".into(), DataValue::UInt(1_048_577)),
        ]);
        assert!(
            d.to_json_view(
                "x",
                SourceView {
                    bytes_len: 1_048_577,
                    text: None
                }
            )
            .is_ok()
        );
        assert!(
            d.validate_view(
                "x",
                SourceView {
                    bytes_len: 1_048_576,
                    text: None
                }
            )
            .is_err()
        );

        d.code = "NX-E0004".into();
        d.rule = "NX-E0004/invalid_utf8".into();
        d.explanation_id = Some(d.rule.clone());
        d.subcause = Some("invalid_utf8".into());
        d.stage = Stage::Input;
        d.operation = Operation::InputDecode;
        d.primary_span = Some(Span {
            file: "x".into(),
            start: 1,
            end: 2,
        });
        d.data = BTreeMap::from([("invalid_bytes_hex".into(), DataValue::String("ff".into()))]);
        assert!(
            d.to_json_view(
                "x",
                SourceView {
                    bytes_len: 2,
                    text: None
                }
            )
            .is_ok()
        );
        assert!(
            d.validate_view(
                "x",
                SourceView {
                    bytes_len: 1,
                    text: None
                }
            )
            .is_err()
        );
    }

    #[test]
    fn json_key_order_is_fixed_by_the_serialized_schema() {
        let json = sample()
            .to_json("decimal.nx", "fn main(){let x=0.5;}")
            .unwrap();
        let names = [
            "schema_version",
            "diagnostic_namespace",
            "profile_id",
            "code",
            "subcause",
            "rule",
            "explanation_id",
            "severity",
            "stage",
            "message",
            "operation",
            "primary_span",
            "related_spans",
            "cause_chain",
            "feature_id",
            "unexpected_token",
            "expected_tokens",
            "expected_tokens_kind",
            "parser_context",
            "recognized_prefix",
            "repair",
            "data",
        ];
        let positions = names.map(|name| json.find(&format!("\"{name}\":")).unwrap());
        assert!(positions.windows(2).all(|w| w[0] < w[1]));
    }
    #[test]
    fn bounded_read_checks_size_before_utf8_and_reports_invalid_byte_range() {
        let mut bytes = vec![b'a'; SOURCE_LIMIT];
        bytes.extend_from_slice("🙂".as_bytes());
        assert_eq!(
            read_bounded(bytes.as_slice()),
            Err(InputError::TooLarge {
                limit: SOURCE_LIMIT,
                observed: SOURCE_LIMIT + 1
            })
        );
        assert_eq!(
            decode_utf8(vec![b'a', 0xff]),
            Err(InputError::InvalidUtf8 {
                start: 1,
                end: 2,
                bytes_hex: "ff".into(),
                message: "invalid utf-8 sequence of 1 bytes from index 1".into(),
            })
        );
    }
    #[test]
    fn text_columns_count_unicode_scalars_and_crlf_as_one_line_break() {
        assert_eq!(line_column("🙂\r\nç", 8), (2, 2));
    }

    #[test]
    fn text_uses_lexicographic_json_keys_for_related_cause_and_data() {
        let mut d = sample();
        d.related_spans.push(RelatedSpan {
            role: RelatedRole::Origin,
            span: Span {
                file: "decimal.nx".into(),
                start: 10,
                end: 16,
            },
        });
        d.cause_chain.push(Cause {
            kind: CauseKind::Call,
            operation: CauseOperation::FunctionCall,
            span: None,
            entity: Some(Entity {
                kind: EntityKind::Intrinsic,
                id: "prelude:print".into(),
            }),
            evidence: Evidence::Verified,
        });
        let rendered = d
            .render_text("decimal.nx", "fn main(){let x=0.5;}")
            .unwrap();
        assert_eq!(
            rendered,
            concat!(
                "NX-E0006:1:17: Literal flotante fuera del perfil.\n",
                "  namespace: Nexo-proposed-v1\n  profile: f0-0002-v1\n  stage: parse\n  operation: syntax.parse\n",
                "  at: {\"end\":19,\"file\":\"decimal.nx\",\"start\":16}\n",
                "  feature_id: float_literal\n",
                "  related: {\"role\":\"origin\",\"span\":{\"end\":16,\"file\":\"decimal.nx\",\"start\":10}}\n",
                "  cause: {\"entity\":{\"id\":\"prelude:print\",\"kind\":\"intrinsic\"},\"evidence\":\"verified\",\"kind\":\"call\",\"operation\":\"function.call\",\"span\":null}\n",
                "  data: {\"pending_scope\":\"later_adr\"}\n"
            )
        );
    }

    #[test]
    fn json_strings_escape_required_controls_but_keep_del_and_c1_unicode() {
        assert_eq!(q("\u{7f}\u{85}\u{1b}"), "\"\u{7f}\u{85}\\u001b\"");
        assert_eq!(q("\n"), "\"\\n\"");
        assert_eq!(q("a\\b\"c"), "\"a\\\\b\\\"c\"");
    }

    #[test]
    fn missing_terminator_text_uses_published_lines_and_canonical_json() {
        let source = "fn main(){let x=1}";
        let file = "terminador.nx";
        let mut diagnostic = Diagnostic {
            schema_version: 1,
            diagnostic_namespace: Namespace::NexoProposedV1,
            profile_id: PROFILE_ID.into(),
            code: "NX-E0005".into(),
            subcause: None,
            rule: "NX-E0005".into(),
            explanation_id: Some("NX-E0005".into()),
            severity: Severity::Error,
            stage: Stage::Parse,
            message: "Falta un terminador de sentencia.".into(),
            operation: Operation::SyntaxParse,
            primary_span: Some(Span {
                file: file.into(),
                start: 17,
                end: 18,
            }),
            related_spans: vec![RelatedSpan {
                role: RelatedRole::PrecedingExpression,
                span: Span {
                    file: file.into(),
                    start: 16,
                    end: 17,
                },
            }],
            cause_chain: vec![],
            feature_id: None,
            unexpected_token: Some(Token {
                kind: TokenKind::Symbol,
                text: Some("}".into()),
                span: Span {
                    file: file.into(),
                    start: 17,
                    end: 18,
                },
            }),
            expected_tokens: vec![";".into()],
            expected_tokens_kind: Some(ExpectedTokensKind::CanonicalInsertion),
            parser_context: Some("statement_terminator".into()),
            recognized_prefix: Some(Span {
                file: file.into(),
                start: 10,
                end: 17,
            }),
            repair: vec![Repair {
                id: "insert_terminator".into(),
                status: RepairStatus::Proposed,
                description: "Insertar un terminador explícito.".into(),
                edits: vec![Edit {
                    span: Span {
                        file: file.into(),
                        start: 17,
                        end: 17,
                    },
                    expected_text: String::new(),
                    replacement: ";".into(),
                }],
                validation: None,
                consequences: vec!["Termina el binding antes del cierre del bloque.".into()],
            }],
            data: BTreeMap::new(),
        };
        let text = diagnostic.render_text(file, source).unwrap();
        assert_eq!(
            diagnostic.to_json(file, source).unwrap(),
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0002-v1","code":"NX-E0005","subcause":null,"rule":"NX-E0005","explanation_id":"NX-E0005","severity":"error","stage":"parse","message":"Falta un terminador de sentencia.","operation":"syntax.parse","primary_span":{"file":"terminador.nx","start":17,"end":18},"related_spans":[{"role":"preceding_expression","span":{"file":"terminador.nx","start":16,"end":17}}],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"symbol","text":"}","span":{"file":"terminador.nx","start":17,"end":18}},"expected_tokens":[";"],"expected_tokens_kind":"canonical_insertion","parser_context":"statement_terminator","recognized_prefix":{"file":"terminador.nx","start":10,"end":17},"repair":[{"id":"insert_terminator","status":"proposed","description":"Insertar un terminador explícito.","edits":[{"span":{"file":"terminador.nx","start":17,"end":17},"expected_text":"","replacement":";"}],"validation":null,"consequences":["Termina el binding antes del cierre del bloque."]}],"data":{}}"#
        );
        assert_eq!(
            text,
            concat!(
                "NX-E0005:1:18: Falta un terminador de sentencia.\n",
                "  namespace: Nexo-proposed-v1\n  profile: f0-0002-v1\n  stage: parse\n  operation: syntax.parse\n",
                "  at: {\"end\":18,\"file\":\"terminador.nx\",\"start\":17}\n",
                "  related: {\"role\":\"preceding_expression\",\"span\":{\"end\":17,\"file\":\"terminador.nx\",\"start\":16}}\n",
                "  syntax: {\"expected_tokens\":[\";\"],\"expected_tokens_kind\":\"canonical_insertion\",\"parser_context\":\"statement_terminator\",\"recognized_prefix\":{\"end\":17,\"file\":\"terminador.nx\",\"start\":10},\"unexpected_token\":{\"kind\":\"symbol\",\"span\":{\"end\":18,\"file\":\"terminador.nx\",\"start\":17},\"text\":\"}\"}}\n",
                "  repair: {\"consequences\":[\"Termina el binding antes del cierre del bloque.\"],\"description\":\"Insertar un terminador explícito.\",\"edits\":[{\"expected_text\":\"\",\"replacement\":\";\",\"span\":{\"end\":17,\"file\":\"terminador.nx\",\"start\":17}}],\"id\":\"insert_terminator\",\"status\":\"proposed\",\"validation\":null}\n"
            )
        );
        diagnostic.repair[0].edits[0].expected_text = "}".into();
        assert!(diagnostic.render_text(file, source).is_err());
    }
}
