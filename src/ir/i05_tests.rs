use super::{CompileError, compile_scalar};
use crate::interp::{Outcome, run_scalar};

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
    let CompileError::Rejected(actual) = compile_scalar("d7.nx", source).unwrap_err() else {
        panic!("expected NX-E0202 for {source}")
    };
    let span = |(start, end)| Span {
        file: "d7.nx".into(),
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
        diagnostic.validate("d7.nx", source).unwrap();
    }
}

#[test]
fn consumed_index_reads_keep_the_complete_ownership_diagnostic() {
    for (source, use_span, declaration, consumption, dropped) in [
        (
            "fn main(){let a=[1];drop(a);print(a[0]);}",
            (34, 35),
            (14, 15),
            (20, 27),
            true,
        ),
        (
            "fn main(){let a=[[1,2]];let b=consume a;print(a[0][1]);}",
            (46, 47),
            (14, 15),
            (30, 39),
            false,
        ),
        (
            "fn show(x:i64){} fn main(){let a=[1];drop(a);show(a[0]);}",
            (50, 51),
            (31, 32),
            (37, 44),
            true,
        ),
        (
            "fn main(){let a=[1];drop(a);let x=a[0];}",
            (34, 35),
            (14, 15),
            (20, 27),
            true,
        ),
        (
            "fn f()->i64{let a=[1];drop(a);return a[0];} fn main(){}",
            (37, 38),
            (16, 17),
            (22, 29),
            true,
        ),
        (
            "fn main(){let a=[1];drop(a);print(a[0]==1);}",
            (34, 35),
            (14, 15),
            (20, 27),
            true,
        ),
        (
            "fn take(consume x:Array<i64>){} fn main(){let a=[1];take(consume a);print(a[0]);}",
            (74, 75),
            (46, 47),
            (57, 66),
            false,
        ),
        (
            "fn main(){let a=[1];drop(a);let b=a[0];print(b);}",
            (34, 35),
            (14, 15),
            (20, 27),
            true,
        ),
    ] {
        assert_consumed_binding_diagnostic(source, use_span, declaration, consumption, dropped);
    }
}

#[test]
fn consumed_index_reads_use_the_actual_local_after_scalars_shadowing_and_parameters() {
    for (source, use_span, declaration, consumption) in [
        (
            "fn main(){let x=1;let a=[1];drop(a);print(a[0]);}",
            (42, 43),
            (22, 23),
            (28, 35),
        ),
        (
            "fn main(){let x=1;let y=2;let a=[1];drop(a);print(a[0]);}",
            (50, 51),
            (30, 31),
            (36, 43),
        ),
        (
            "fn main(){let x=1;let a=[1];{let a=[2];drop(a);print(a[0]);}}",
            (53, 54),
            (33, 34),
            (39, 46),
        ),
        (
            "fn f(x:i64,consume a:Array<i64>){drop(a);print(a[0]);} fn main(){f(1,[1]);}",
            (47, 48),
            (19, 20),
            (33, 40),
        ),
    ] {
        assert_consumed_binding_diagnostics(
            source,
            &[use_span],
            declaration,
            consumption,
            true,
            source.starts_with("fn f("),
        );
    }
}

#[test]
fn dead_consumed_index_reads_keep_sequential_diagnostics() {
    for (source, use_span, declaration, consumption) in [
        (
            "fn main(){return;let a=[1];drop(a);print(a[0]);}",
            (41, 42),
            (21, 22),
            (27, 34),
        ),
        (
            "fn main(){let x=1;return;{let a=[1];drop(a);print(a[0]);}}",
            (50, 51),
            (30, 31),
            (36, 43),
        ),
        (
            "fn main(){let a=[1];return;drop(a);print(a[0]);}",
            (41, 42),
            (14, 15),
            (27, 34),
        ),
    ] {
        assert_consumed_binding_diagnostic(source, use_span, declaration, consumption, true);
    }
}

#[test]
fn dead_block_consumption_reports_each_later_use() {
    let source = "fn main(){let r=mock_open(1);return;{drop(r);}inspect(r);inspect(r);}";
    assert_consumed_binding_diagnostics(
        source,
        &[(54, 55), (65, 66)],
        (14, 15),
        (37, 44),
        true,
        false,
    );
}

#[test]
fn adr_gap_index_read_checks_root_before_evaluating_index() {
    let source = "fn main(){let a=[1];drop(a);print(a[9223372036854775807+1]);}";
    assert_consumed_binding_diagnostic(source, (34, 35), (14, 15), (20, 27), true);
}

#[test]
fn dead_reinitialization_keeps_prior_consumption_diagnostic() {
    assert_consumed_binding_diagnostic(
        "fn main(){var r=mock_open(1);drop(r);{return;let s=r;}}",
        (51, 52),
        (14, 15),
        (29, 36),
        true,
    );
    assert_consumed_binding_diagnostic(
        "fn main(){var r=mock_open(1);drop(r);{return;r=mock_open(2);}inspect(r);}",
        (69, 70),
        (14, 15),
        (29, 36),
        true,
    );
}

#[test]
fn consumed_bindings_in_every_read_form_never_become_invalid_ir() {
    for (source, supported) in [
        ("fn main(){let x=1;drop(x);print(x);}", true),
        ("fn main(){let a=[1];drop(a);print(a[0]);}", true),
        ("fn main(){let a=[[1]];drop(a);print(a[0][0]);}", true),
        ("fn main(){let x=1;drop(x);assert_eq(x,1);}", true),
        ("fn f(x:i64){} fn main(){let x=1;drop(x);f(x);}", true),
        ("fn main(){let x=1;drop(x);print(x+1);}", true),
        ("fn main(){let x=1;drop(x);let y=x;}", true),
        ("fn f()->i64{let x=1;drop(x);return x;} fn main(){}", true),
        ("fn main(){let x=1;drop(x);if x==1{print(1);}}", false),
        ("fn main(){let x=1;drop(x);while x==1{print(1);}}", false),
    ] {
        let result = compile_scalar("reads.nx", source);
        match result {
            Err(CompileError::Rejected(ref errors)) if supported => {
                assert_eq!(errors.len(), 1, "{source}");
                assert_eq!(errors[0].code, "NX-E0202", "{source}");
                assert_eq!(errors[0].subcause.as_deref(), Some("definitely_consumed"));
            }
            Err(CompileError::OutsideScalarSlice(_)) if !supported => {}
            other => panic!("unexpected result for {source}: {other:?}"),
        }
    }
}

#[test]
fn shadowed_resource_declaration_uses_the_later_token() {
    let source = "fn main(){let r=mock_open(1);{let r=mock_open(2);let s=r;}}";
    let CompileError::Rejected(errors) = compile_scalar("shadow.nx", source).unwrap_err() else {
        panic!("expected resource copy diagnostic")
    };
    assert_eq!(errors.len(), 1);
    let declaration = &errors[0].related_spans[0].span;
    let inner = source.rfind("r=mock_open").unwrap();
    assert_eq!(
        (declaration.start, declaration.end),
        (inner as u64, inner as u64 + 1)
    );
}

#[test]
fn dead_transfer_does_not_change_later_diagnostic_or_execution() {
    let source = "fn main(){let r=mock_open(1);{return;drop(r);}inspect(r);}";
    let program = compile_scalar("dead.nx", source).unwrap();
    let execution = run_scalar(&program);
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(
        execution.observations.events(),
        vec!["open:0:1", "close:0:1"]
    );
}

#[test]
fn dead_declaration_keeps_live_name_span_for_later_error() {
    let source = "fn main(){let r=mock_open(1);{return;let x=1;}let s=r;}";
    let CompileError::Rejected(errors) = compile_scalar("dead_span.nx", source).unwrap_err() else {
        panic!("expected resource copy diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0201");
    let span = &errors[0].related_spans[0].span;
    let start = source.find("r=mock_open").unwrap() as u64;
    assert_eq!((span.start, span.end), (start, start + 1));
}

#[test]
fn entity_ids_follow_declared_kind_and_name_offset() {
    let source = "fn f(x:MockResource)->MockResource{return consume x;} fn main(){}";
    let CompileError::Rejected(errors) = compile_scalar("entity.nx", source).unwrap_err() else {
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
    for action in ["let s=consume r;", "drop(r);"] {
        let source = format!("fn main(){{let r=mock_open(1);{action}inspect(r);}}");
        let CompileError::Rejected(errors) = compile_scalar("entity.nx", &source).unwrap_err()
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
            format!("binding:{}", source.find("r=mock_open").unwrap())
        );
    }
}

#[test]
fn failed_argument_cleans_prepared_temporaries_before_locals_in_reverse_order() {
    for (source, events) in [
        (
            "fn g(consume a:MockResource,consume b:MockResource,n:i64){} fn main(){g(mock_open(1),mock_open(2),9223372036854775807+1);}",
            vec!["open:0:1", "open:1:2", "close:1:2", "close:0:1"],
        ),
        (
            "fn g(consume a:MockResource,consume b:MockResource,n:i64){} fn main(){let r=mock_open(0);g(mock_open(1),mock_open(2),9223372036854775807+1);}",
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
        let e = run_scalar(&compile_scalar("cleanup.nx", source).unwrap());
        assert_eq!(e.outcome, Outcome::SimulatorFault);
        assert_eq!(e.diagnostics[0].code, "R0101");
        assert_eq!(e.observations.events(), events);
    }
}

#[test]
fn failed_let_poison_suppresses_dependent_uses_and_allows_independent_errors() {
    for (source, code) in [
        (
            "fn main(){let r=mock_open(1);let s=r;inspect(s);inspect(s);}",
            "NX-E0201",
        ),
        (
            "fn f(a:MockResource){let x=consume a;inspect(x);inspect(x);} fn main(){}",
            "NX-E1105",
        ),
        (
            "fn take(r:MockResource){} fn main(){let r=mock_open(1);let s=r;take(s);}",
            "NX-E0201",
        ),
        (
            "fn main(){let r=mock_open(1);let s=r;{let s=mock_open(2);inspect(s);}inspect(s);}",
            "NX-E0201",
        ),
        (
            "fn f()->MockResource{let r=mock_open(1);let s=r;return s;} fn main(){}",
            "NX-E0201",
        ),
        (
            "fn main(){let r=mock_open(1);let s=r;let t=s;inspect(t);}",
            "NX-E0201",
        ),
        (
            "fn main(){let r=mock_open(1);{let s=r;inspect(s);}print(1);}",
            "NX-E0201",
        ),
    ] {
        let CompileError::Rejected(ds) = compile_scalar("poison.nx", source).unwrap_err() else {
            panic!("expected source diagnostics: {source}")
        };
        assert_eq!(ds.len(), 1, "{source}");
        assert_eq!(ds[0].code, code, "{source}");
    }
    let source =
        "fn main(){let r=mock_open(1);let s=r;inspect(s);let t=mock_open(2);let u=t;inspect(u);}";
    let CompileError::Rejected(ds) = compile_scalar("poison.nx", source).unwrap_err() else {
        panic!("expected two independent diagnostics")
    };
    assert_eq!(
        ds.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0201", "NX-E0201"]
    );
    let source = "fn main(){let x=mock_open(1);{let r=mock_open(2);let x=r;}let z=x;}";
    let CompileError::Rejected(ds) = compile_scalar("poison.nx", source).unwrap_err() else {
        panic!("expected errors inside and after shadowing block")
    };
    assert_eq!(
        ds.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0201", "NX-E0201"]
    );
}

#[test]
fn named_owner_arguments_keep_parameter_slots_and_reverse_callee_cleanup() {
    let e = run_scalar(&compile_scalar("named.nx", "fn g(consume a:MockResource,consume b:MockResource){} fn main(){g(b:mock_open(1),a:mock_open(2));}").unwrap());
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "open:1:2", "close:0:1", "close:1:2"]
    );
}

#[test]
fn borrowed_owner_temporaries_close_in_reverse_written_order() {
    let e = run_scalar(&compile_scalar("borrow.nx", "fn g(a:MockResource,b:MockResource){inspect(a);inspect(b);} fn main(){g(mock_open(1),mock_open(2));}").unwrap());
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
    let source = "fn g(a:MockResource,consume b:MockResource){inspect(a);inspect(b);} fn main(){let r=mock_open(1);g(r,mock_open(2));inspect(r);}";
    let e = run_scalar(&compile_scalar("modes.nx", source).unwrap());
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
            "fn f{n}(consume a:MockResource,consume b:MockResource){{f{}(consume a,consume b);}}",
            n + 1
        ));
    }
    source.push_str("fn f256(consume a:MockResource,consume b:MockResource){} fn main(){f0(mock_open(1),mock_open(2));}");
    let e = run_scalar(&compile_scalar("frames.nx", &source).unwrap());
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
            "fn f{n}(a:MockResource,consume b:MockResource){{f{}(a,consume b);}}",
            n + 1
        ));
    }
    source.push_str("fn f256(a:MockResource,consume b:MockResource){} fn main(){let a=mock_open(1);f0(a,mock_open(2));}");
    let e = run_scalar(&compile_scalar("frames.nx", &source).unwrap());
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
        "fn sum(a:i64,b:i64)->i64{return a+b;} fn read(r:MockResource){inspect(r);} fn take(consume r:MockResource){inspect(r);} fn main(){",
    );
    let mut depth = 0;
    let mut has_r = false;
    for action in actions {
        match action {
            Action::Open(label) => {
                if !has_r && depth == 0 {
                    s.push_str("var r=mock_open(");
                } else {
                    s.push_str("let r=mock_open(");
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
            Action::Read => s.push_str("read(r);"),
            Action::Take => s.push_str("take(consume r);"),
            Action::Drop => s.push_str("drop(r);"),
            Action::Reopen(label) => s.push_str(&format!("r=mock_open({label});")),
            Action::ReadTemporary(label) => s.push_str(&format!("read(mock_open({label}));")),
            Action::PrintSum(a, b) => s.push_str(&format!("print(sum({a},{b}));")),
            Action::Return => s.push_str("return;"),
            Action::LatePrint => s.push_str("print(999);"),
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
        let p = compile_scalar("generated.nx", &source)
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
        "consume ",
        "drop(",
        "inspect(",
        "mock_open(",
        "f(",
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
            "fn f(consume r:MockResource,n:i64){{inspect(r);}} fn main(){{let r=mock_open(1);{one}{two};}}",
        );
        match compile_scalar("fuzz.nx", &source) {
            Err(CompileError::Rejected(errors)) => {
                for d in errors {
                    d.validate("fuzz.nx", &source)
                        .unwrap_or_else(|e| panic!("case {n}: {e:?}"));
                }
            }
            Ok(p) => {
                let run = run_scalar(&p);
                for d in run.diagnostics {
                    d.validate("fuzz.nx", &source).unwrap();
                }
            }
            Err(CompileError::OutsideScalarSlice(_)) => {}
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
        "fn f(x:i64)->i64{return x+1;} fn main(){assert_eq(f(f(f(1))),4);}",
        "fn f(consume x:MockResource){inspect(x);} fn main(){let r=mock_open(1);f(consume r);}",
        "fn f(){f();} fn main(){f();}",
        "fn main(){let r=mock_open(1);{let r=mock_open(2);inspect(r);}inspect(r);}",
    ];
    for bytes in [128 * 1024, 256 * 1024, 512 * 1024] {
        for source in sources {
            std::thread::Builder::new()
                .stack_size(bytes)
                .spawn(move || {
                    let p = compile_scalar("small.nx", source).unwrap();
                    let result = run_scalar(&p);
                    assert!(!matches!(result.outcome, Outcome::InternalError(_)));
                    for d in &result.diagnostics {
                        d.validate("small.nx", source).unwrap();
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
    use std::time::Instant;
    let header = "fn f(x:i64)->i64{return x+1;} fn two(a:i64,b:i64){} fn read(r:MockResource){inspect(r);} fn take(consume r:MockResource){inspect(r);} fn main(){";
    let snippets = [
        "{f(1);}",
        "{f(x:1234567);}",
        "{f(f(1));}",
        "{let x=1;f(x);}",
        "{let r=mock_open(1);read(r);}",
        "{let r=mock_blocking_open(1);inspect(r);}",
        "{let r=mock_open(1);take(consume r);}",
        "{read(mock_open(1));}",
        "{var r=mock_open(1);drop(r);r=mock_open(2);}",
        "{two(b:f(2),a:f(1));}",
    ];
    let sizes = [256 * 1024, 1024 * 1024];
    for (family, snippet) in snippets.iter().enumerate() {
        let mut elapsed = [0.0; 2];
        for (index, size) in sizes.into_iter().enumerate() {
            let mut source = String::from(header);
            let pattern = format!("{snippet}{}", " ".repeat(80));
            while source.len() + pattern.len() + 3 <= size {
                source.push_str(&pattern);
            }
            source.push('}');
            source.push_str("//");
            source.push_str(&"x".repeat(size - source.len()));
            let start = Instant::now();
            let p = compile_scalar("scale.nx", &source).unwrap();
            assert!(p.functions().function("main").is_some());
            elapsed[index] = start.elapsed().as_secs_f64();
        }
        let ratio = elapsed[1] / elapsed[0];
        println!(
            "i05-family-{family} 256KiB={:.3}s 1MiB={:.3}s ratio={ratio:.3}",
            elapsed[0], elapsed[1]
        );
        assert!(ratio <= 8.0, "family {family}: {ratio}");
    }
}

#[test]
#[ignore = "explicit 256 KiB to 1 MiB lowering measurement"]
fn i05_large_signature_and_many_functions_scale_linearly() {
    use std::time::Instant;
    for family in 0..5 {
        let mut elapsed = [0.0; 2];
        for (scale, size) in [(1usize, 256 * 1024), (4, 1024 * 1024)] {
            let mut source = String::new();
            match family {
                0..=2 => {
                    let count = 250 * scale;
                    for n in 0..count {
                        let body = if family == 1 && n + 1 < count {
                            format!("return f{}(x);", n + 1)
                        } else if family == 2 {
                            "let r=mock_open(1);inspect(r);return x;".into()
                        } else {
                            "return x+1;".into()
                        };
                        source.push_str(&format!("fn f{n}(x:i64)->i64{{{body}}}\n"));
                        source.push_str(&" ".repeat(850));
                    }
                    source.push_str("fn main(){print(f0(1));}\n");
                }
                3 => {
                    source.push_str("fn f(x:i64)->i64{return x+1;} fn main(){");
                    for _ in 0..(2500 * scale) {
                        source.push_str("f(f(1));");
                    }
                    source.push('}');
                }
                4 => {
                    source.push_str("fn f(");
                    for n in 0..(12_000 * scale) {
                        if n > 0 {
                            source.push(',');
                        }
                        source.push_str(&format!("p{n}:i64"));
                    }
                    source.push_str("){} fn main(){}");
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
            let start = Instant::now();
            let program = compile_scalar("scale.nx", &source).unwrap();
            assert!(program.functions().function("main").is_some());
            elapsed[usize::from(scale == 4)] = start.elapsed().as_secs_f64();
        }
        let ratio = elapsed[1] / elapsed[0];
        println!(
            "i05-large-family-{family} 256KiB={:.3}s 1MiB={:.3}s ratio={ratio:.3}",
            elapsed[0], elapsed[1]
        );
        assert!(ratio < 8.0, "family {family}: {ratio}");
    }
}

fn unreachable_scale_source(family: usize, size: usize) -> String {
    let mut source = String::from("fn g(x:i64)->i64{return x;} fn main(){");
    if family == 5 {
        for n in 0..2000 {
            source.push_str(&format!("let v{n}=1;"));
        }
    }
    let pattern = match family {
        0 | 5 | 8 | 9 => "{return;g(g(1));}",
        1 => "{return;{g(g(1));}}",
        2 => "{{return;g(g(1));}}",
        3 => "{let r=mock_open(1);return;inspect(r);}",
        4 => "{return;let x=1;g(x);}",
        6 => "{{{{{return;g(g(1));}}}}}",
        7 => "{return;let r=mock_open(1);drop(r);}",
        10 => "{let r=mock_open(1);}",
        11 => "{let x=1;}",
        _ => unreachable!(),
    };
    while source.len() + pattern.len() + 70 < size {
        source.push_str(pattern);
    }
    if family == 8 {
        source.push_str("if true{return;}else{return;}g(g(1));");
    } else if family == 9 {
        source.push_str("while true{return;}g(g(1));");
    }
    source.push('}');
    source.push_str("//");
    source.push_str(&"x".repeat(size - source.len()));
    source
}

#[test]
fn unreachable_lowering_short_scale_regression() {
    use std::time::Instant;
    let mut ratios = Vec::new();
    for family in 0..7 {
        let mut elapsed = [0.0; 2];
        for (index, size) in [64 * 1024, 256 * 1024].into_iter().enumerate() {
            let source = unreachable_scale_source(family, size);
            let start = Instant::now();
            compile_scalar("dead_scale.nx", &source).unwrap();
            elapsed[index] = start.elapsed().as_secs_f64();
        }
        ratios.push(elapsed[1] / elapsed[0]);
    }
    println!("dead-code scale ratios: {ratios:?}");
    assert!(
        ratios.iter().all(|r| *r < 10.0),
        "dead lowering grew quadratically: {ratios:?}"
    );
}

#[test]
#[ignore = "explicit dead-code and dense-declaration compilation measurement"]
fn i05_unreachable_and_dense_declarations_scale_across_twelve_families() {
    use std::time::Instant;
    for family in 0..12 {
        let mut elapsed = [0.0; 2];
        for (index, size) in [256 * 1024, 1024 * 1024].into_iter().enumerate() {
            let source = unreachable_scale_source(family, size);
            let start = Instant::now();
            let result = compile_scalar("dead_scale.nx", &source);
            if family == 8 || family == 9 {
                assert!(matches!(result, Err(CompileError::OutsideScalarSlice(_))));
            } else {
                result.unwrap();
            }
            elapsed[index] = start.elapsed().as_secs_f64();
        }
        let ratio = elapsed[1] / elapsed[0];
        println!(
            "i05-dead-dense-{family} 256KiB={:.3}s 1MiB={:.3}s ratio={ratio:.3}",
            elapsed[0], elapsed[1]
        );
        assert!(ratio < 8.0, "family {family}: {ratio}");
    }
}
