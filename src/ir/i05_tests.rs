use super::{CompileError, compile_scalar};
use crate::interp::{Outcome, run_scalar};

fn byte_span(source: &str, needle: &str, occurrence: usize) -> (u64, u64) {
    let start = source
        .match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("missing occurrence {occurrence} of {needle:?}"))
        .0;
    (start as u64, (start + needle.len()) as u64)
}

fn assert_consumed_binding_diagnostic(
    source: &str,
    use_span: (u64, u64),
    declaration: (u64, u64),
    consumption: (u64, u64),
    dropped: bool,
) {
    assert_consumed_binding_diagnostics(
        source,
        &[use_span],
        declaration,
        consumption,
        dropped,
        false,
    );
}

fn assert_consumed_binding_diagnostics(
    source: &str,
    use_spans: &[(u64, u64)],
    declaration: (u64, u64),
    consumption: (u64, u64),
    dropped: bool,
    parameter: bool,
) {
    use crate::diagnostic::{
        Cause, CauseKind, CauseOperation, Diagnostic, Entity, EntityKind, Evidence, Namespace,
        Operation, RelatedRole, RelatedSpan, Severity, Span, Stage,
    };
    let CompileError::Rejected(actual) = compile_scalar("d7.n", source).unwrap_err() else {
        panic!("expected NX-E0202 for {source}")
    };
    let span = |(start, end)| Span {
        file: "d7.n".into(),
        start,
        end,
    };
    let expected = Diagnostic {
        schema_version: 1,
        diagnostic_namespace: Namespace::NexoProposedV1,
        profile_id: crate::diagnostic::PROFILE_ID.into(),
        code: "NX-E0202".into(),
        subcause: Some("definitely_consumed".into()),
        rule: "NX-E0202/definitely_consumed".into(),
        explanation_id: Some("NX-E0202/definitely_consumed".into()),
        severity: Severity::Error,
        stage: Stage::Ownership,
        message: "Acceso propietario inválido.".into(),
        operation: Operation::OwnershipUse,
        primary_span: Some(span(use_spans[0])),
        related_spans: vec![
            RelatedSpan {
                role: RelatedRole::Declaration,
                span: span(declaration),
            },
            RelatedSpan {
                role: RelatedRole::Consumption,
                span: span(consumption),
            },
        ],
        cause_chain: vec![Cause {
            kind: if dropped {
                CauseKind::Drop
            } else {
                CauseKind::Consume
            },
            operation: if dropped {
                CauseOperation::ValueDrop
            } else {
                CauseOperation::ValueConsume
            },
            span: Some(span(consumption)),
            entity: Some(Entity {
                kind: if parameter {
                    EntityKind::Parameter
                } else {
                    EntityKind::Binding
                },
                id: format!(
                    "{}:{}",
                    if parameter { "parameter" } else { "binding" },
                    declaration.0
                ),
            }),
            evidence: Evidence::Verified,
        }],
        feature_id: None,
        unexpected_token: None,
        expected_tokens: vec![],
        expected_tokens_kind: None,
        parser_context: None,
        recognized_prefix: None,
        repair: vec![],
        data: Default::default(),
    };
    assert_eq!(actual.len(), use_spans.len(), "{source}");
    for (diagnostic, use_span) in actual.iter().zip(use_spans) {
        let mut expected = expected.clone();
        expected.primary_span = Some(span(*use_span));
        assert_eq!(**diagnostic, expected, "{source}");
        diagnostic.validate("d7.n", source).unwrap();
    }
}

#[test]
fn consumed_index_reads_keep_the_complete_ownership_diagnostic() {
    for (source, dropped) in [
        (
            "fn Main() {\n    let a = [1];\n    Drop(a);\n    Print(a[0]);\n}\n",
            true,
        ),
        (
            "fn Main() {\n    let a = [[1, 2]];\n    let b = move a;\n    Print(a[0][1]);\n}\n",
            false,
        ),
        (
            "fn Show(x: view I64) {}\nfn Main() {\n    let a = [1];\n    Drop(a);\n    Show(a[0]);\n}\n",
            true,
        ),
        (
            "fn Main() {\n    let a = [1];\n    Drop(a);\n    let x = a[0];\n}\n",
            true,
        ),
        (
            "fn F() -> I64 {\n    let a = [1];\n    Drop(a);\n    return a[0];\n}\nfn Main() {}\n",
            true,
        ),
        (
            "fn Main() {\n    let a = [1];\n    Drop(a);\n    Print(a[0] == 1);\n}\n",
            true,
        ),
        (
            "fn Take(x: move Array<I64>) {}\nfn Main() {\n    let a = [1];\n    Take(move a);\n    Print(a[0]);\n}\n",
            false,
        ),
        (
            "fn Main() {\n    let a = [1];\n    Drop(a);\n    let b = a[0];\n    Print(b);\n}\n",
            true,
        ),
    ] {
        let use_at = byte_span(source, "a[", 0).0;
        let declaration = if source.contains("let a =") {
            byte_span(source, "a =", 0).0
        } else {
            byte_span(source, "a: move", 0).0
        };
        let consumption = if dropped {
            byte_span(source, "Drop(a)", 0)
        } else {
            byte_span(source, "move a", 0)
        };
        assert_consumed_binding_diagnostic(
            source,
            (use_at, use_at + 1),
            (declaration, declaration + 1),
            consumption,
            dropped,
        );
    }
}

#[test]
fn consumed_index_reads_use_the_actual_local_after_scalars_shadowing_and_parameters() {
    for source in [
        "fn Main() {\n    let x = 1;\n    let a = [1];\n    Drop(a);\n    Print(a[0]);\n}\n",
        "fn Main() {\n    let x = 1;\n    let y = 2;\n    let a = [1];\n    Drop(a);\n    Print(a[0]);\n}\n",
        "fn Main() {\n    let x = 1;\n    let a = [1];\n    {\n        let a = [2];\n        Drop(a);\n        Print(a[0]);\n    }\n}\n",
        "fn F(x: view I64, a: move Array<I64>) {\n    Drop(a);\n    Print(a[0]);\n}\nfn Main() {\n    F(1, [1]);\n}\n",
    ] {
        let use_at = byte_span(source, "a[", 0).0;
        let declaration = if source.contains("a: move") {
            byte_span(source, "a: move", 0).0
        } else {
            source.rfind("a = [").expect("array binding") as u64
        };
        assert_consumed_binding_diagnostics(
            source,
            &[(use_at, use_at + 1)],
            (declaration, declaration + 1),
            byte_span(source, "Drop(a)", 0),
            true,
            source.starts_with("fn F("),
        );
    }
}

#[test]
fn dead_consumed_index_reads_keep_sequential_diagnostics() {
    for source in [
        "fn Main() {\n    return;\n    let a = [1];\n    Drop(a);\n    Print(a[0]);\n}\n",
        "fn Main() {\n    let x = 1;\n    return;\n    {\n        let a = [1];\n        Drop(a);\n        Print(a[0]);\n    }\n}\n",
        "fn Main() {\n    let a = [1];\n    return;\n    Drop(a);\n    Print(a[0]);\n}\n",
    ] {
        let use_at = byte_span(source, "a[", 0).0;
        let declaration = byte_span(source, "a = [", 0).0;
        assert_consumed_binding_diagnostic(
            source,
            (use_at, use_at + 1),
            (declaration, declaration + 1),
            byte_span(source, "Drop(a)", 0),
            true,
        );
    }
}

#[test]
fn dead_block_consumption_reports_each_later_use() {
    let source = "fn Main() {\n    let r = MockOpen(1);\n    return;\n    {\n        Drop(r);\n    }\n    Inspect(r);\n    Inspect(r);\n}\n";
    assert_consumed_binding_diagnostics(
        source,
        &[
            (
                byte_span(source, "r);", 1).0,
                byte_span(source, "r);", 1).0 + 1,
            ),
            (
                byte_span(source, "r);", 2).0,
                byte_span(source, "r);", 2).0 + 1,
            ),
        ],
        (
            byte_span(source, "r = MockOpen", 0).0,
            byte_span(source, "r = MockOpen", 0).0 + 1,
        ),
        byte_span(source, "Drop(r)", 0),
        true,
        false,
    );
}

#[test]
fn adr_gap_index_read_checks_root_before_evaluating_index() {
    let source =
        "fn Main() {\n    let a = [1];\n    Drop(a);\n    Print(a[9223372036854775807 + 1]);\n}\n";
    let use_at = byte_span(source, "a[", 0).0;
    assert_consumed_binding_diagnostic(
        source,
        (use_at, use_at + 1),
        (
            byte_span(source, "a = [", 0).0,
            byte_span(source, "a = [", 0).0 + 1,
        ),
        byte_span(source, "Drop(a)", 0),
        true,
    );
}

#[test]
fn dead_reinitialization_keeps_prior_consumption_diagnostic() {
    let source = "fn Main() {\n    var r = MockOpen(1);\n    Drop(r);\n    {\n        return;\n        let s = r;\n    }\n}\n";
    let use_at = byte_span(source, "r;", 0).0;
    let declaration = byte_span(source, "r = MockOpen", 0).0;
    assert_consumed_binding_diagnostic(
        source,
        (use_at, use_at + 1),
        (declaration, declaration + 1),
        byte_span(source, "Drop(r)", 0),
        true,
    );
    let source = "fn Main() {\n    var r = MockOpen(1);\n    Drop(r);\n    {\n        return;\n        r = MockOpen(2);\n    }\n    Inspect(r);\n}\n";
    let use_at = byte_span(source, "Inspect(r)", 0).0 + "Inspect(".len() as u64;
    let declaration = byte_span(source, "r = MockOpen", 0).0;
    assert_consumed_binding_diagnostic(
        source,
        (use_at, use_at + 1),
        (declaration, declaration + 1),
        byte_span(source, "Drop(r)", 0),
        true,
    );
}

#[test]
fn consumed_bindings_in_every_read_form_never_become_invalid_ir() {
    for source in [
        "fn Main() {\n    let x = 1;\n    Drop(x);\n    Print(x);\n}\n",
        "fn Main() {\n    let a = [1];\n    Drop(a);\n    Print(a[0]);\n}\n",
        "fn Main() {\n    let a = [[1]];\n    Drop(a);\n    Print(a[0][0]);\n}\n",
        "fn Main() {\n    let x = 1;\n    Drop(x);\n    AssertEq(x, 1);\n}\n",
        "fn F(x: view I64) {}\nfn Main() {\n    let x = 1;\n    Drop(x);\n    F(x);\n}\n",
        "fn Main() {\n    let x = 1;\n    Drop(x);\n    Print(x + 1);\n}\n",
        "fn Main() {\n    let x = 1;\n    Drop(x);\n    let y = x;\n}\n",
        "fn F() -> I64 {\n    let x = 1;\n    Drop(x);\n    return x;\n}\nfn Main() {}\n",
        "fn Main() {\n    let x = 1;\n    Drop(x);\n    if x == 1 {\n        Print(1);\n    }\n}\n",
        "fn Main() {\n    let x = 1;\n    Drop(x);\n    while x == 1 {\n        Print(1);\n    }\n}\n",
    ] {
        let result = compile_scalar("reads.n", source);
        match result {
            Err(CompileError::Rejected(ref errors)) => {
                assert_eq!(errors.len(), 1, "{source}");
                assert_eq!(errors[0].code, "NX-E0202", "{source}");
                assert_eq!(errors[0].subcause.as_deref(), Some("definitely_consumed"));
            }
            other => panic!("unexpected result for {source}: {other:?}"),
        }
    }
}

#[test]
fn shadowed_resource_declaration_uses_the_later_token() {
    let source = "fn Main() {\n    let r = MockOpen(1);\n    {\n        let r = MockOpen(2);\n        let s = r;\n    }\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("shadow.n", source).unwrap_err() else {
        panic!("expected resource copy diagnostic")
    };
    assert_eq!(errors.len(), 1);
    let declaration = &errors[0].related_spans[0].span;
    let inner = source.rfind("r = MockOpen").unwrap();
    assert_eq!(
        (declaration.start, declaration.end),
        (inner as u64, inner as u64 + 1)
    );
}

#[test]
fn dead_transfer_does_not_change_later_diagnostic_or_execution() {
    let source = "fn Main() {\n    let r = MockOpen(1);\n    {\n        return;\n        Drop(r);\n    }\n    Inspect(r);\n}\n";
    let program = compile_scalar("dead.n", source).unwrap();
    let execution = run_scalar(&program);
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(
        execution.observations.events(),
        vec!["open:0:1", "close:0:1"]
    );
}

#[test]
fn dead_declaration_keeps_live_name_span_for_later_error() {
    let source = "fn Main() {\n    let r = MockOpen(1);\n    {\n        return;\n        let x = 1;\n    }\n    let s = r;\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("dead_span.n", source).unwrap_err() else {
        panic!("expected resource copy diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0201");
    let span = &errors[0].related_spans[0].span;
    let start = source.find("r = MockOpen").unwrap() as u64;
    assert_eq!((span.start, span.end), (start, start + 1));
}

#[test]
fn entity_ids_follow_declared_kind_and_name_offset() {
    let source =
        "fn F(x: view MockResource) -> MockResource {\n    return move x;\n}\nfn Main() {}\n";
    let CompileError::Rejected(errors) = compile_scalar("entity.n", source).unwrap_err() else {
        panic!("expected permission diagnostic")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(d.code, "NX-E1105");
    let expected = crate::diagnostic::DataValue::Object(
        [
            (
                "kind".into(),
                crate::diagnostic::DataValue::String("parameter".into()),
            ),
            (
                "id".into(),
                crate::diagnostic::DataValue::String("parameter:5".into()),
            ),
        ]
        .into_iter()
        .collect(),
    );
    assert_eq!(d.data.get("origin"), Some(&expected));
    assert_eq!(d.data.get("parameter"), Some(&expected));
    assert_eq!(d.cause_chain[0].entity.as_ref().unwrap().id, "parameter:5");
}

#[test]
fn consumed_and_dropped_binding_causes_use_binding_entity() {
    for action in ["let s=move r;", "Drop(r);"] {
        let source = format!("fn Main(){{let r=MockOpen(1);{action}Inspect(r);}}");
        let CompileError::Rejected(errors) = compile_scalar("entity.n", &source).unwrap_err()
        else {
            panic!("expected use-after-consumption diagnostic")
        };
        assert_eq!(errors.len(), 1);
        let d = &errors[0];
        assert_eq!(d.code, "NX-E0202");
        let entity = d.cause_chain[0].entity.as_ref().unwrap();
        assert_eq!(entity.kind, crate::diagnostic::EntityKind::Binding);
        assert_eq!(
            entity.id,
            format!("binding:{}", source.find("r=MockOpen").unwrap())
        );
    }
}

#[test]
fn failed_argument_cleans_prepared_temporaries_before_locals_in_reverse_order() {
    for (source, events) in [
        (
            "fn G(a: move MockResource, b: move MockResource, n: view I64) {}\nfn Main() {\n    G(MockOpen(1), MockOpen(2), 9223372036854775807 + 1);\n}\n",
            vec!["open:0:1", "open:1:2", "close:1:2", "close:0:1"],
        ),
        (
            "fn G(a: move MockResource, b: move MockResource, n: view I64) {}\nfn Main() {\n    let r = MockOpen(0);\n    G(MockOpen(1), MockOpen(2), 9223372036854775807 + 1);\n}\n",
            vec![
                "open:0:0",
                "open:1:1",
                "open:2:2",
                "close:2:2",
                "close:1:1",
                "close:0:0",
            ],
        ),
    ] {
        let e = run_scalar(&compile_scalar("cleanup.n", source).unwrap());
        assert_eq!(e.outcome, Outcome::SimulatorFault);
        assert_eq!(e.diagnostics[0].code, "R0101");
        assert_eq!(e.observations.events(), events);
    }
}

#[test]
fn failed_let_poison_suppresses_dependent_uses_and_allows_independent_errors() {
    for (source, code) in [
        (
            "fn Main() {\n    let r = MockOpen(1);\n    let s = r;\n    Inspect(s);\n    Inspect(s);\n}\n",
            "NX-E0201",
        ),
        (
            "fn F(a: view MockResource) {\n    let x = move a;\n    Inspect(x);\n    Inspect(x);\n}\nfn Main() {}\n",
            "NX-E1105",
        ),
        (
            "fn Take(r: view MockResource) {}\nfn Main() {\n    let r = MockOpen(1);\n    let s = r;\n    Take(s);\n}\n",
            "NX-E0201",
        ),
        (
            "fn Main() {\n    let r = MockOpen(1);\n    let s = r;\n    {\n        let s = MockOpen(2);\n        Inspect(s);\n    }\n    Inspect(s);\n}\n",
            "NX-E0201",
        ),
        (
            "fn F() -> MockResource {\n    let r = MockOpen(1);\n    let s = r;\n    return s;\n}\nfn Main() {}\n",
            "NX-E0201",
        ),
        (
            "fn Main() {\n    let r = MockOpen(1);\n    let s = r;\n    let t = s;\n    Inspect(t);\n}\n",
            "NX-E0201",
        ),
        (
            "fn Main() {\n    let r = MockOpen(1);\n    {\n        let s = r;\n        Inspect(s);\n    }\n    Print(1);\n}\n",
            "NX-E0201",
        ),
    ] {
        let CompileError::Rejected(ds) = compile_scalar("poison.n", source).unwrap_err() else {
            panic!("expected source diagnostics: {source}")
        };
        assert_eq!(ds.len(), 1, "{source}");
        assert_eq!(ds[0].code, code, "{source}");
    }
    let source = "fn Main() {\n    let r = MockOpen(1);\n    let s = r;\n    Inspect(s);\n    let t = MockOpen(2);\n    let u = t;\n    Inspect(u);\n}\n";
    let CompileError::Rejected(ds) = compile_scalar("poison.n", source).unwrap_err() else {
        panic!("expected two independent diagnostics")
    };
    assert_eq!(
        ds.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0201", "NX-E0201"]
    );
    let source = "fn Main() {\n    let x = MockOpen(1);\n    {\n        let r = MockOpen(2);\n        let x = r;\n    }\n    let z = x;\n}\n";
    let CompileError::Rejected(ds) = compile_scalar("poison.n", source).unwrap_err() else {
        panic!("expected errors inside and after shadowing block")
    };
    assert_eq!(
        ds.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0201", "NX-E0201"]
    );
}

#[test]
fn named_owner_arguments_keep_parameter_slots_and_reverse_callee_cleanup() {
    let e = run_scalar(&compile_scalar("named.n", "fn G(a: move MockResource, b: move MockResource) {}\nfn Main() {\n    G(b: MockOpen(1), a: MockOpen(2));\n}\n").unwrap());
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "open:1:2", "close:0:1", "close:1:2"]
    );
}

#[test]
fn borrowed_owner_temporaries_close_in_reverse_written_order() {
    let e = run_scalar(&compile_scalar("borrow.n", "fn G(a: view MockResource, b: view MockResource) {\n    Inspect(a);\n    Inspect(b);\n}\nfn Main() {\n    G(MockOpen(1), MockOpen(2));\n}\n").unwrap());
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        [
            "open:0:1",
            "open:1:2",
            "inspect:0:1",
            "inspect:1:2",
            "close:1:2",
            "close:0:1"
        ]
    );
}

#[test]
fn mixed_argument_modes_use_each_declared_slot() {
    let source = "fn G(a: view MockResource, b: move MockResource) {\n    Inspect(a);\n    Inspect(b);\n}\nfn Main() {\n    let r = MockOpen(1);\n    G(r, MockOpen(2));\n    Inspect(r);\n}\n";
    let e = run_scalar(&compile_scalar("modes.n", source).unwrap());
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        [
            "open:0:1",
            "open:1:2",
            "inspect:0:1",
            "inspect:1:2",
            "close:1:2",
            "inspect:0:1",
            "close:0:1"
        ]
    );
}

#[test]
fn rejected_frame_entry_closes_two_transfers_in_reverse_argument_order() {
    let mut source = String::new();
    for n in 0..256 {
        source.push_str(&format!(
            "fn F{n}(a: move MockResource,b: move MockResource){{F{}(move a,move b);}}",
            n + 1
        ));
    }
    source.push_str("fn F256(a: move MockResource, b: move MockResource) {}\nfn Main() {\n    F0(MockOpen(1), MockOpen(2));\n}\n");
    let e = run_scalar(&compile_scalar("frames.n", &source).unwrap());
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0002");
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "open:1:2", "close:1:2", "close:0:1"]
    );
}

#[test]
fn rejected_frame_entry_does_not_close_borrowed_argument() {
    let mut source = String::new();
    for n in 0..256 {
        source.push_str(&format!(
            "fn F{n}(a:view MockResource,b:move MockResource){{F{}(a,move b);}}",
            n + 1
        ));
    }
    source.push_str("fn F256(a: view MockResource, b: move MockResource) {}\nfn Main() {\n    let a = MockOpen(1);\n    F0(a, MockOpen(2));\n}\n");
    let e = run_scalar(&compile_scalar("frames.n", &source).unwrap());
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0002");
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "open:1:2", "close:1:2", "close:0:1"]
    );
}

#[derive(Clone, Copy)]
enum Action {
    Open(i64),
    Enter,
    Exit,
    Read,
    Take,
    Drop,
    Reopen(i64),
    ReadTemporary(i64),
    PrintSum(i64, i64),
    Return,
    LatePrint,
}

#[derive(Default)]
struct Reference {
    next_id: u64,
    scopes: Vec<Option<(u64, i64)>>,
    events: Vec<String>,
    stdout: Vec<String>,
    stopped: bool,
}
impl Reference {
    fn close(&mut self, resource: (u64, i64)) {
        self.events
            .push(format!("close:{}:{}", resource.0, resource.1));
    }
    fn acquire(&mut self, label: i64) -> (u64, i64) {
        let id = self.next_id;
        self.next_id += 1;
        self.events.push(format!("open:{id}:{label}"));
        (id, label)
    }
    fn current(&self) -> (u64, i64) {
        self.scopes
            .iter()
            .rev()
            .find_map(|x| *x)
            .expect("live model resource")
    }
    fn current_slot(&self) -> usize {
        self.scopes
            .iter()
            .rposition(Option::is_some)
            .expect("live model slot")
    }
    fn apply(&mut self, action: Action) {
        if self.stopped {
            return;
        }
        match action {
            Action::Open(label) => {
                let r = self.acquire(label);
                self.scopes.push(Some(r));
            }
            Action::Enter => self.scopes.push(None),
            Action::Exit => {
                let entry = self.scopes.pop().expect("scope marker");
                if let Some(r) = entry {
                    self.close(r);
                }
                assert!(self.scopes.pop().expect("scope marker").is_none());
            }
            Action::Read => {
                let (id, label) = self.current();
                self.events.push(format!("inspect:{id}:{label}"));
            }
            Action::Take => {
                let slot = self.current_slot();
                let (id, label) = self.scopes[slot].take().unwrap();
                self.events.push(format!("inspect:{id}:{label}"));
                self.close((id, label));
            }
            Action::Drop => {
                let slot = self.current_slot();
                let resource = self.scopes[slot].take().unwrap();
                self.close(resource);
            }
            Action::Reopen(label) => {
                let slot = self.scopes.len() - 1;
                let r = self.acquire(label);
                if let Some(old) = self.scopes[slot].replace(r) {
                    self.close(old);
                }
            }
            Action::ReadTemporary(label) => {
                let (id, label) = self.acquire(label);
                self.events.push(format!("inspect:{id}:{label}"));
                self.close((id, label));
            }
            Action::PrintSum(a, b) => self
                .stdout
                .push((i128::from(a) + i128::from(b)).to_string()),
            Action::Return => self.stopped = true,
            Action::LatePrint => self.stdout.push("999".into()),
        }
    }
    fn finish(mut self) -> Self {
        for r in std::mem::take(&mut self.scopes).into_iter().rev().flatten() {
            self.close(r);
        }
        self
    }
}

fn source(actions: &[Action]) -> String {
    let mut s = String::from(
        "fn Sum(a:view I64,b:view I64)->I64{return a+b;} fn Read(r:view MockResource){Inspect(r);} fn Take(r:move MockResource){Inspect(r);} fn Main(){",
    );
    let mut depth = 0;
    let mut has_r = false;
    for action in actions {
        match action {
            Action::Open(label) => {
                if !has_r && depth == 0 {
                    s.push_str("var r=MockOpen(");
                } else {
                    s.push_str("let r=MockOpen(");
                }
                s.push_str(&format!("{label});"));
                has_r = true;
            }
            Action::Enter => {
                s.push('{');
                depth += 1;
                has_r = false;
            }
            Action::Exit => {
                s.push('}');
                depth -= 1;
                has_r = true;
            }
            Action::Read => s.push_str("Read(r);"),
            Action::Take => s.push_str("Take(move r);"),
            Action::Drop => s.push_str("Drop(r);"),
            Action::Reopen(label) => s.push_str(&format!("r=MockOpen({label});")),
            Action::ReadTemporary(label) => s.push_str(&format!("Read(MockOpen({label}));")),
            Action::PrintSum(a, b) => s.push_str(&format!("Print(Sum({a},{b}));")),
            Action::Return => s.push_str("return;"),
            Action::LatePrint => s.push_str("Print(999);"),
        }
    }
    s.push('}');
    s
}

#[test]
fn generated_calls_resources_and_early_returns_match_independent_reference() {
    for n in 0..8_000usize {
        let label = (n % 107) as i64 - 31;
        let second = (n % 83) as i64 + 5;
        let a = (n % 113) as i64 - 50;
        let b = (n % 97) as i64 - 20;
        let actions: Vec<Action> = match n % 6 {
            0 => vec![Action::Open(label), Action::Read, Action::PrintSum(a, b)],
            1 => vec![
                Action::Open(label),
                Action::Enter,
                Action::Open(second),
                Action::Read,
                Action::Exit,
                Action::Read,
            ],
            2 => vec![Action::Open(label), Action::Take, Action::PrintSum(a, b)],
            3 => vec![
                Action::Open(label),
                Action::Drop,
                Action::Reopen(second),
                Action::Read,
            ],
            4 => vec![Action::ReadTemporary(label), Action::PrintSum(a, b)],
            _ => vec![
                Action::Open(label),
                Action::Read,
                Action::Return,
                Action::LatePrint,
            ],
        };
        let source = source(&actions);
        let mut expected = Reference::default();
        for action in &actions {
            expected.apply(*action);
        }
        let expected = expected.finish();
        let p = compile_scalar("generated.n", &source)
            .unwrap_or_else(|e| panic!("case {n}: {e:?}\n{source}"));
        let result = run_scalar(&p);
        assert_eq!(result.outcome, Outcome::Unit, "case {n}");
        assert!(result.diagnostics.is_empty(), "case {n}");
        assert_eq!(
            result.observations.events(),
            expected
                .events
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            "case {n}"
        );
        assert_eq!(
            result.observations.stdout(),
            expected
                .stdout
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            "case {n}"
        );
        let expected_bytes = expected
            .events
            .iter()
            .map(|e| "mock-resource:".len() + e.len() + 1)
            .sum::<usize>()
            + expected.stdout.iter().map(|s| s.len() + 1).sum::<usize>();
        assert_eq!(
            result.observations.bytes(),
            expected_bytes as u64,
            "case {n}"
        );
        assert_eq!(result.observations.reserved_bytes(), 0, "case {n}");
    }
}

#[test]
fn malformed_call_and_resource_sources_never_panic_and_diagnostics_validate() {
    let mut seed = 0x2a56_19b3_u64;
    let fragments = [
        "move ",
        "Drop(",
        "Inspect(",
        "MockOpen(",
        "F(",
        "let r=",
        "return ",
        "a[",
        "{",
        "}",
        "\n",
        ":",
        ",",
        ";",
        "9223372036854775807+1",
        "r",
        "0",
    ];
    for n in 0..20_000 {
        seed ^= seed << 7;
        seed ^= seed >> 9;
        seed ^= seed << 8;
        let one = fragments[(seed as usize) % fragments.len()];
        let two = fragments[((seed >> 11) as usize) % fragments.len()];
        let source = format!(
            "fn F(r:move MockResource,n:view I64){{Inspect(r);}} fn Main(){{let r=MockOpen(1);{one}{two};}}",
        );
        match compile_scalar("fuzz.n", &source) {
            Err(CompileError::Rejected(errors)) => {
                for d in errors {
                    d.validate("fuzz.n", &source)
                        .unwrap_or_else(|e| panic!("case {n}: {e:?}"));
                }
            }
            Ok(p) => {
                let run = run_scalar(&p);
                for d in run.diagnostics {
                    d.validate("fuzz.n", &source).unwrap();
                }
            }
            Err(CompileError::InvalidIr(reason)) => {
                panic!("case {n}: invalid IR {reason}: {source}")
            }
            Err(CompileError::Dependent) => panic!("case {n}: escaped dependent error"),
        }
    }
}
#[test]
fn call_lowering_and_execution_survive_small_caller_stacks() {
    let sources = [
        "fn F(x: view I64) -> I64 {\n    return x + 1;\n}\nfn Main() {\n    AssertEq(F(F(F(1))), 4);\n}\n",
        "fn F(x: move MockResource) {\n    Inspect(x);\n}\nfn Main() {\n    let r = MockOpen(1);\n    F(move r);\n}\n",
        "fn F() {\n    F();\n}\nfn Main() {\n    F();\n}\n",
        "fn Main() {\n    let r = MockOpen(1);\n    {\n        let r = MockOpen(2);\n        Inspect(r);\n    }\n    Inspect(r);\n}\n",
    ];
    for bytes in [128 * 1024, 256 * 1024, 512 * 1024] {
        for source in sources {
            std::thread::Builder::new()
                .stack_size(bytes)
                .spawn(move || {
                    let p = compile_scalar("small.n", source).unwrap();
                    let result = run_scalar(&p);
                    assert!(!matches!(result.outcome, Outcome::InternalError(_)));
                    for d in &result.diagnostics {
                        d.validate("small.n", source).unwrap();
                    }
                })
                .unwrap()
                .join()
                .unwrap();
        }
    }
}
#[test]
#[ignore = "explicit 256 KiB to 1 MiB measurement"]
fn i05_call_and_resource_source_families_scale_linearly() {
    let _campaign = crate::test_alloc::campaign_guard();
    let header = "fn F(x:view I64)->I64{return x+1;} fn Two(a:view I64,b:view I64){} fn Read(r:view MockResource){Inspect(r);} fn Take(r:move MockResource){Inspect(r);} fn Main(){";
    let snippets = [
        "{F(1);}",
        "{F(x:1234567);}",
        "{F(F(1));}",
        "{let x=1;F(x);}",
        "{let r=MockOpen(1);Read(r);}",
        "{let r=MockBlockingOpen(1);Inspect(r);}",
        "{let r=MockOpen(1);Take(move r);}",
        "{Read(MockOpen(1));}",
        "{var r=MockOpen(1);Drop(r);r=MockOpen(2);}",
        "{Two(b:F(2),a:F(1));}",
    ];
    let sizes = [256 * 1024, 1024 * 1024];
    for (family, snippet) in snippets.iter().enumerate() {
        let mut traffic = [(0_u64, 0_u64); 2];
        for (index, size) in sizes.into_iter().enumerate() {
            let mut source = String::from(header);
            let pattern = format!("{snippet}{}", " ".repeat(80));
            while source.len() + pattern.len() + 3 <= size {
                source.push_str(&pattern);
            }
            source.push('}');
            source.push_str("//");
            source.push_str(&"x".repeat(size - source.len()));
            crate::test_alloc::reset("dead_scale_counter.n");
            let p = compile_scalar("dead_scale_counter.n", &source).unwrap();
            assert!(p.functions().function("Main").is_some());
            traffic[index] = crate::test_alloc::snapshot("dead_scale_counter.n");
        }
        println!("i05-family-{family} traffic={traffic:?}");
        for (small, large) in [(traffic[0].0, traffic[1].0), (traffic[0].1, traffic[1].1)] {
            assert!(
                large <= small.saturating_mul(8),
                "family {family}: {traffic:?}"
            );
        }
    }
}

#[test]
#[ignore = "explicit 256 KiB to 1 MiB lowering measurement"]
fn i05_large_signature_and_many_functions_scale_linearly() {
    let _campaign = crate::test_alloc::campaign_guard();
    for family in 0..5 {
        let mut traffic = [(0_u64, 0_u64); 2];
        for (scale, size) in [(1usize, 256 * 1024), (4, 1024 * 1024)] {
            let mut source = String::new();
            match family {
                0..=2 => {
                    let count = 250 * scale;
                    for n in 0..count {
                        let body = if family == 1 && n + 1 < count {
                            format!("return F{}(x);", n + 1)
                        } else if family == 2 {
                            "let r=MockOpen(1);Inspect(r);return x;".into()
                        } else {
                            "return x+1;".into()
                        };
                        source.push_str(&format!("fn F{n}(x:view I64)->I64{{{body}}}\n"));
                        source.push_str(&" ".repeat(850));
                    }
                    source.push_str("fn Main() {\n    Print(F0(1));\n}\n");
                }
                3 => {
                    source.push_str("fn F(x:view I64)->I64{return x+1;} fn Main(){");
                    for _ in 0..(2500 * scale) {
                        source.push_str("F(F(1));");
                    }
                    source.push('}');
                }
                4 => {
                    source.push_str("fn F(");
                    for n in 0..(12_000 * scale) {
                        if n > 0 {
                            source.push(',');
                        }
                        source.push_str(&format!("p{n}:view I64"));
                    }
                    source.push_str("){} fn Main(){}");
                }
                _ => unreachable!(),
            }
            assert!(
                source.len() <= size,
                "family {family}: {} > {size}",
                source.len()
            );
            source.push_str("//");
            source.push_str(&"a".repeat(size - source.len()));
            crate::test_alloc::reset("dead_scale_counter.n");
            let program = compile_scalar("dead_scale_counter.n", &source).unwrap();
            assert!(program.functions().function("Main").is_some());
            traffic[usize::from(scale == 4)] = crate::test_alloc::snapshot("dead_scale_counter.n");
        }
        println!("i05-large-family-{family} traffic={traffic:?}");
        for (small, large) in [(traffic[0].0, traffic[1].0), (traffic[0].1, traffic[1].1)] {
            assert!(
                large <= small.saturating_mul(8),
                "family {family}: {traffic:?}"
            );
        }
    }
}

fn unreachable_scale_source(family: usize, size: usize) -> String {
    let mut source = String::from("fn G(x:view I64)->I64{return x;} fn Main(){");
    if family == 5 {
        for n in 0..2000 {
            source.push_str(&format!("let v{n}=1;"));
        }
    }
    let pattern = match family {
        0 | 5 | 8 | 9 => "{return;G(G(1));}",
        1 => "{return;{G(G(1));}}",
        2 => "{{return;G(G(1));}}",
        3 => "{let r=MockOpen(1);return;Inspect(r);}",
        4 => "{return;let x=1;G(x);}",
        6 => "{{{{{return;G(G(1));}}}}}",
        7 => "{return;let r=MockOpen(1);Drop(r);}",
        10 => "{let r=MockOpen(1);}",
        11 => "{let x=1;}",
        _ => unreachable!(),
    };
    while source.len() + pattern.len() + 70 < size {
        source.push_str(pattern);
    }
    if family == 8 {
        source.push_str("if true{return;}else{return;}G(G(1));");
    } else if family == 9 {
        source.push_str("while true{return;}G(G(1));");
    }
    source.push('}');
    source.push_str("//");
    source.push_str(&"x".repeat(size - source.len()));
    source
}

#[test]
fn unreachable_lowering_short_scale_regression() {
    for family in 0..7 {
        let mut measurements = [(0_u64, 0_u64); 2];
        for (index, size) in [64 * 1024, 256 * 1024].into_iter().enumerate() {
            let source = unreachable_scale_source(family, size);
            crate::test_alloc::reset("dead_scale_counter.n");
            compile_scalar("dead_scale_counter.n", &source).unwrap();
            let (bytes, allocations) = crate::test_alloc::snapshot("dead_scale_counter.n");
            measurements[index] = (bytes, allocations);
        }
        let small = measurements[0];
        let large = measurements[1];
        for (metric, a, b) in [
            ("bytes", small.0, large.0),
            ("allocations", small.1, large.1),
        ] {
            assert!(
                b <= a.saturating_mul(8),
                "family {family} {metric}: {small:?} -> {large:?}"
            );
        }
    }
}

#[test]
#[ignore = "explicit dead-code and dense-declaration compilation measurement"]
fn i05_unreachable_and_dense_declarations_scale_across_twelve_families() {
    let _campaign = crate::test_alloc::campaign_guard();
    for family in 0..12 {
        let mut traffic = [(0_u64, 0_u64); 2];
        for (index, size) in [256 * 1024, 1024 * 1024].into_iter().enumerate() {
            let source = unreachable_scale_source(family, size);
            crate::test_alloc::reset("dead_scale_counter.n");
            compile_scalar("dead_scale_counter.n", &source).unwrap();
            traffic[index] = crate::test_alloc::snapshot("dead_scale_counter.n");
        }
        println!("i05-dead-dense-{family} traffic={traffic:?}");
        for (small, large) in [(traffic[0].0, traffic[1].0), (traffic[0].1, traffic[1].1)] {
            assert!(
                large <= small.saturating_mul(8),
                "family {family}: {traffic:?}"
            );
        }
    }
}
