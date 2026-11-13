use nexo::interp::{MockEnvironment, Outcome, run_scalar};

const FLOW1: &str = r#"
fn Sumar(records: view Array<Result<I64,I64>>) -> Result<I64,I64> with no_alloc,no_block,no_suspend,no_io {
 var i=0;
 var total=0;
 while i<Len(records){let value=records[i]?;total=TryAdd(total,value)?;i=i+1;}
 return Ok(total);
}
fn Main() -> Result<(),I64> {
 let file=MockOpen(7);
 let records=MockReadRecords(file)?;
 let total=Sumar(records)?;
 MockWriteI64(total)?;
 return Ok(());
}
"#;

#[test]
fn normative_flows_have_literal_runtime_results() {
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"12"},{"ok":"8"},{"ok":"5"}]}],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#).unwrap();
    let execution = run_scalar(&nexo::ir::compile_scalar("flow1.n", FLOW1).unwrap(), env);
    assert_eq!(execution.outcome, Outcome::Ok);
    assert_eq!(execution.observations.stdout(), ["25"]);
    assert_eq!(
        execution.observations.events(),
        [
            "open:0:7",
            "read_records:0:ok:3",
            "write_i64:25:ok",
            "close:0:7"
        ]
    );
    assert_eq!(execution.observations.bytes(), 102);
    assert_eq!(execution.observations.reserved_bytes(), 0);

    let flow2 = "fn Incrementar(xs: edit Array<I64>) with no_alloc,no_block,no_suspend,no_io {var i=0;while i<Len(xs){xs[i]=xs[i]+1;i=i+1;}} fn Main(){var original=[1,2];var copia=original;Incrementar(edit copia);AssertEq(original,[1,2]);AssertEq(copia,[2,3]);}";
    let execution = run_scalar(
        &nexo::ir::compile_scalar("flow2.n", flow2).unwrap(),
        MockEnvironment::empty(),
    );
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.observations.observations().is_empty());

    let flow3 = "fn Responder(route: view I64)->I64 with no_alloc,no_block,no_suspend,no_io{return if route==0 then {value:200;} else {value:404;};} fn Main()->Result<(),I64>{let route=MockRequest()?;MockReply(Responder(route))?;return Ok(());}";
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"0"}],"mock_reply":[{"ok":null}]}"#).unwrap();
    let execution = run_scalar(&nexo::ir::compile_scalar("flow3.n", flow3).unwrap(), env);
    assert_eq!(execution.outcome, Outcome::Ok);
    assert_eq!(
        execution.observations.events(),
        ["request:ok:0", "reply:200:ok"]
    );
    assert_eq!(execution.observations.bytes(), 42);
}

#[test]
fn write_ok_and_err_match_literal_json_contract_data() {
    let source = "fn Main()->Result<(),I64>{MockWriteI64(25)?;return Ok(());} ";
    let program = nexo::ir::compile_scalar("write.n", source).unwrap();
    let ok = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#).unwrap();
    let execution = run_scalar(&program, ok);
    assert_eq!(execution.outcome, Outcome::Ok);
    assert_eq!(execution.observations.bytes(), 27);
    assert_eq!(execution.observations.events(), ["write_i64:25:ok"]);
    assert_eq!(execution.observations.stdout(), ["25"]);

    let err = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"err":"40"}],"mock_request":[],"mock_reply":[]}"#).unwrap();
    let execution = run_scalar(&program, err);
    assert_eq!(execution.outcome, Outcome::Err(40));
    assert_eq!(execution.observations.bytes(), 28);
    assert_eq!(execution.observations.events(), ["write_i64:25:err:40"]);
    assert!(execution.observations.stdout().is_empty());
}

#[test]
fn mock_json_schema_and_canonical_i64_are_enforced() {
    let valid = br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"0"},{"err":"-9223372036854775808"}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    assert!(MockEnvironment::parse(valid).is_ok());
    for invalid in [
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"extra":[]}"#.as_slice(),
        br#"{"schema_version":1,"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
        br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":1}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null,"err":"1"}],"mock_request":[],"mock_reply":[]}"#,
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"-0"}],"mock_reply":[]}"#,
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"9223372036854775808"}],"mock_reply":[]}"#,
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[] } garbage"#,
    ] {
        assert!(MockEnvironment::parse(invalid).is_err(), "{}", String::from_utf8_lossy(invalid));
    }
    let nested = format!("{}null{}", "[".repeat(17), "]".repeat(17));
    let deep = format!(
        "{{\"schema_version\":1,\"mock_read_records\":{nested},\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}"
    );
    assert!(matches!(
        MockEnvironment::parse(deep.as_bytes()),
        Err(nexo::interp::MockEnvironmentError::InvalidJson)
    ));
}

#[test]
fn mock_json_accepts_tab_as_whitespace() {
    // A2 §12.4, «Esquema del entorno», l. 1501: el entorno es JSON UTF-8.
    let json = b"{\"schema_version\":1,\t\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}";
    assert!(MockEnvironment::parse(json).is_ok());
}

#[test]
fn mock_json_rejects_raw_0x1f_inside_string() {
    // A2 §12.4, «Esquema del entorno», ll. 1501 y 1538: JSON inválido es invalid_mock_json.
    let mut json = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"x":""#.to_vec();
    json.push(0x1f);
    json.extend_from_slice(b"\"}");
    assert_eq!(
        MockEnvironment::parse(&json).unwrap_err(),
        nexo::interp::MockEnvironmentError::InvalidJson
    );
}

#[test]
fn mock_json_uppercase_exponent_is_a_schema_error() {
    // A2 §12.4, «Esquema del entorno», ll. 1505–1517 y 1538: versión y razones.
    let json = br#"{"schema_version":1E0,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    assert_eq!(
        MockEnvironment::parse(json).unwrap_err(),
        nexo::interp::MockEnvironmentError::InvalidSchema
    );
}

#[test]
fn mock_json_empty_arrays_restore_nesting_depth() {
    // A2 §12.4, «Esquema del entorno», ll. 1523–1538: tipos y profundidad acotada.
    let reads = std::iter::repeat_n(r#"{"ok":[]}"#, 14)
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        r#"{{"schema_version":1,"mock_read_records":[{reads}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#
    );
    assert!(MockEnvironment::parse(json.as_bytes()).is_ok());
}

#[test]
fn mock_json_syntax_error_precedes_duplicate_key_schema_error() {
    // A2 §12.4, «Esquema del entorno», ll. 1517 y 1538: se valida el documento JSON
    // entero antes de clasificar su esquema.
    let json = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"mock_reply":[]} basura"#;
    assert_eq!(
        MockEnvironment::parse(json).unwrap_err(),
        nexo::interp::MockEnvironmentError::InvalidJson
    );
    assert_eq!(
        MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"mock_reply":[]}"#).unwrap_err(),
        nexo::interp::MockEnvironmentError::InvalidSchema
    );
}

#[test]
fn mock_write_queue_consumes_one_reply_per_call() {
    // A2 §12.4, «Esquema del entorno», l. 1536: cada llamada consume su siguiente respuesta.
    let source = "fn Main()->Result<(),I64>{MockWriteI64(25)?;MockWriteI64(3)?;return Ok(());} ";
    let program = nexo::ir::compile_scalar("write-queue.n", source).unwrap();
    let env = MockEnvironment::parse(
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"err":"4"}],"mock_request":[],"mock_reply":[]}"#,
    )
    .unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Err(4));
    assert_eq!(
        execution.observations.events(),
        ["write_i64:25:ok", "write_i64:3:err:4"]
    );
    assert_eq!(execution.observations.stdout(), ["25"]);
}

#[test]
fn mock_read_queue_consumes_one_reply_per_call() {
    // A2 §12.4, «Esquema del entorno», l. 1536: cada llamada consume su siguiente respuesta.
    let source = "fn Main()->Result<(),I64>{let first=MockReadRecords(MockOpen(1))?;let second=MockReadRecords(MockOpen(2))?;return Ok(());} ";
    let program = nexo::ir::compile_scalar("read-queue.n", source).unwrap();
    let env = MockEnvironment::parse(
        br#"{"schema_version":1,"mock_read_records":[{"ok":[]},{"err":"7"}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
    )
    .unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Err(7));
    assert_eq!(
        execution
            .observations
            .events()
            .into_iter()
            .filter(|event| event.starts_with("read_records:"))
            .collect::<Vec<_>>(),
        ["read_records:0:ok:0", "read_records:1:err:7"]
    );
}

#[test]
fn mock_write_event_preserves_negative_decimal_value() {
    // A2 §12.4, «Eventos exactos», ll. 1542–1551: los enteros usan decimal canónico.
    let source = "fn Main()->Result<(),I64>{MockWriteI64(-5)?;return Ok(());} ";
    let program = nexo::ir::compile_scalar("write-negative.n", source).unwrap();
    let env = MockEnvironment::parse(
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#,
    )
    .unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Ok);
    assert_eq!(execution.observations.events(), ["write_i64:-5:ok"]);
    assert_eq!(execution.observations.stdout(), ["-5"]);
}

#[test]
fn mock_read_ok_event_reports_all_records() {
    // A2 §12.4, «Eventos exactos», ll. 1544–1551: lectura incluye la cantidad completa.
    let source =
        "fn Main()->Result<(),I64>{let rows=MockReadRecords(MockOpen(1))?;return Ok(());} ";
    let records = std::iter::repeat_n(r#"{"ok":"1"}"#, 12)
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{records}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#
    );
    let execution = run_scalar(
        &nexo::ir::compile_scalar("read-count.n", source).unwrap(),
        MockEnvironment::parse(json.as_bytes()).unwrap(),
    );
    assert_eq!(execution.outcome, Outcome::Ok);
    assert_eq!(
        execution.observations.events(),
        ["open:0:1", "read_records:0:ok:12", "close:0:1"]
    );
}

#[test]
fn mock_read_err_reserves_two_cells() {
    // A2 §11.6, l. 1062: una respuesta Err de lectura reserva 2 celdas.
    let big = vec!["0"; 60].join(",");
    let source = format!("fn Main(){{let big=[{big}];let result=MockReadRecords(MockOpen(1));}}");
    let program = nexo::ir::compile_scalar_with_profile(
        "read-err-cells.n",
        &source,
        nexo::ir::ReducedProfile::Cells64,
        true,
    )
    .unwrap();
    let env = MockEnvironment::parse(
        br#"{"schema_version":1,"mock_read_records":[{"err":"10"}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
    )
    .unwrap();
    let execution = nexo::interp::run_with_limits(
        &program,
        env,
        nexo::interp::ReducedExecutionProfile::Cells64,
    );
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
}

#[test]
fn mock_read_err_charges_two_work_units() {
    // A2 §11.6, ll. 1026, 1062: llamada/materialización; la fila fija 2 celdas
    // para read Err, no su coste de trabajo separado. Esta regresión fija el 2 observado.
    let mut source = String::from("fn Main(){var i=0;while i<71425{i=i+1;}");
    for slot in 0..8 {
        source.push_str(&format!("let z{slot}=1;"));
    }
    source.push_str("let result=MockReadRecords(MockOpen(1));}");
    let program = nexo::ir::compile_scalar("read-err-work.n", &source).unwrap();
    let env = MockEnvironment::parse(
        br#"{"schema_version":1,"mock_read_records":[{"err":"10"}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
    )
    .unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert_eq!(execution.work_consumed, 1_000_000);
}

#[test]
fn indexed_reads_accept_owned_borrowed_and_read_scalar_temporaries() {
    for source in [
        "fn Main(){let xs=[5,6];let x=xs[1];AssertEq(x,6);}",
        "fn Read(xs:view Array<I64>,i:view I64){let x=xs[i];AssertEq(x,6);}fn Main(){Read([5,6],1);}",
        "fn Main(){let xs=[1,6];let i=xs[0];let value=[5,6][i];AssertEq(value,6);}",
        "fn Main(){var xs=[1,2];let i=0;let y=xs[i];xs[i]=y+2;AssertEq(xs,[3,2]);}",
        "fn Set(x:edit I64){x=9;}fn Main(){var xs=[1,2];var i=1;Set(edit xs[i]);AssertEq(xs,[1,9]);}",
        "fn Read(xs:view Array<I64>,i:view I64){AssertEq(xs[i],6);}fn Main(){Read([5,6],1);}",
        "fn Read(xs:view Array<I64>,idxs:view Array<I64>){AssertEq(xs[idxs[0]],6);}fn Main(){Read([5,6],[1]);}",
        "fn Read(xs:view Array<I64>,ys:view Array<I64>,zs:view Array<I64>){AssertEq(xs[ys[zs[0]]],6);}fn Main(){Read([5,6],[1],[0]);}",
        "fn Set(xs:edit Array<I64>,ys:view Array<I64>,zs:view Array<I64>){xs[ys[zs[0]]]=9;}fn Main(){var xs=[5,6];Set(edit xs,[1],[0]);AssertEq(xs,[5,9]);}",
    ] {
        let program = nexo::ir::compile_scalar("index.n", source)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(
            run_scalar(&program, MockEnvironment::empty()).outcome,
            Outcome::Unit,
            "{source}"
        );
    }
}

#[test]
fn shadowed_name_is_a_user_function_and_earlier_name_errors_win() {
    let shadow = "fn MockRequest()->Result<I64,I64>{return Ok(7);}fn Main()->Result<(),I64>{let x=MockRequest()?;return Ok(());}";
    let execution = run_scalar(
        &nexo::ir::compile_scalar("shadow.n", shadow).unwrap(),
        MockEnvironment::empty(),
    );
    assert_eq!(execution.outcome, Outcome::Ok);
    assert!(execution.observations.observations().is_empty());

    let error = nexo::ir::compile_scalar("priority.n", "fn Main(){let x=missing();MockRequest();}")
        .unwrap_err();
    let nexo::ir::CompileError::Rejected(errors) = error else {
        panic!("expected source diagnostic")
    };
    assert_eq!(errors[0].code, "NX-E0101");
}

#[test]
fn read_records_is_allocating_and_the_removed_gate_has_no_runtime_branch() {
    let source = "fn Main() with no_alloc {let r=MockOpen(1);let rows=MockReadRecords(r);}";
    let error = nexo::ir::compile_scalar("read-effect.n", source).unwrap_err();
    let nexo::ir::CompileError::Rejected(errors) = error else {
        panic!("expected effect diagnostic")
    };
    assert!(
        errors.iter().any(|d| {
            d.code == "NX-E0412"
                && d.data.get("required_effect")
                    == Some(&nexo::diagnostic::DataValue::String("alloc".into()))
        }),
        "{errors:?}"
    );

    assert!(
        !std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/check/limits.rs")).exists()
    );
    let checker = include_str!("../src/check.rs");
    assert!(!checker.contains("mod limits") && !checker.contains("enforce_capabilities"));
    assert!(!include_str!("../src/ir/lower.rs").contains("capability limit"));
}

#[test]
fn reduced_cell_profile_keeps_read_batch_atomic_and_cleans_open_resource() {
    let records = (0..31)
        .map(|_| r#"{"ok":"1"}"#)
        .collect::<Vec<_>>()
        .join(",");
    let env_json = format!(
        r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{records}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#
    );
    let env = MockEnvironment::parse(env_json.as_bytes()).unwrap();
    let source =
        "fn Main()->Result<(),I64>{let r=MockOpen(1);let rows=MockReadRecords(r)?;return Ok(());} ";
    let program = nexo::ir::compile_scalar_with_profile(
        "cells.n",
        source,
        nexo::ir::ReducedProfile::Cells64,
        true,
    )
    .unwrap();
    let execution = nexo::interp::run_with_limits(
        &program,
        env,
        nexo::interp::ReducedExecutionProfile::Cells64,
    );
    assert_eq!(execution.diagnostics[0].code, "R0002");
    assert_eq!(
        execution.diagnostics[0].data.get("runtime_budget_kind"),
        Some(&nexo::diagnostic::DataValue::String(
            "live_value_cells".into()
        ))
    );
    assert_eq!(
        execution.diagnostics[0].data.get("limit"),
        Some(&nexo::diagnostic::DataValue::UInt(64))
    );
    assert_eq!(
        execution.diagnostics[0].data.get("observed"),
        Some(&nexo::diagnostic::DataValue::UInt(65))
    );
    assert_eq!(execution.observations.events(), ["open:0:1", "close:0:1"]);
    assert_eq!(execution.observations.reserved_bytes(), 0);
}

#[test]
fn write_event_and_stdout_batch_is_rejected_without_partial_commit() {
    let source = "fn Main()->Result<(),I64>{Print(-9223372036854775808);MockWriteI64(-9223372036854775808)?;return Ok(());} ";
    let program = nexo::ir::compile_scalar_with_profile(
        "observation-batch.n",
        source,
        nexo::ir::ReducedProfile::Observations64,
        true,
    )
    .unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#).unwrap();
    let execution = nexo::interp::run_with_limits(
        &program,
        env,
        nexo::interp::ReducedExecutionProfile::Observations64,
    );
    assert_eq!(execution.diagnostics[0].code, "R0002");
    assert_eq!(
        execution.diagnostics[0].data.get("runtime_budget_kind"),
        Some(&nexo::diagnostic::DataValue::String(
            "observation_bytes".into()
        ))
    );
    assert!(execution.observations.events().is_empty());
    assert_eq!(execution.observations.stdout(), ["-9223372036854775808"]);
    assert_eq!(execution.observations.bytes(), 21);
    assert_eq!(execution.observations.reserved_bytes(), 0);
}

#[test]
fn mock_read_borrows_temporary_resources_and_closes_once() {
    for (env_json, expected, bytes, outcome) in [
        (
            br#"{"schema_version":1,"mock_read_records":[{"ok":[]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#.as_slice(),
            ["open:0:1", "read_records:0:ok:0", "close:0:1"],
            75,
            Outcome::Ok,
        ),
        (
            br#"{"schema_version":1,"mock_read_records":[{"err":"10"}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#.as_slice(),
            ["open:0:1", "read_records:0:err:10", "close:0:1"],
            77,
            Outcome::Err(10),
        ),
    ] {
        let source = "fn Main()->Result<(),I64>{let a=MockReadRecords(MockOpen(1))?;return Ok(());}";
        let program = nexo::ir::compile_scalar("temporary-read.n", source).unwrap();
        let execution = run_scalar(&program, MockEnvironment::parse(env_json).unwrap());
        assert_eq!(execution.outcome, outcome);
        assert_eq!(execution.observations.events(), expected);
        assert_eq!(execution.observations.bytes(), bytes);
        assert_eq!(execution.observations.reserved_bytes(), 0);
    }

    for source in [
        "fn Main(){MockReadRecords(MockOpen(3));}",
        "fn Main(){let r=MockOpen(3);let a=MockReadRecords(r);Inspect(r);}",
        "fn F(r:view MockResource)->Result<Array<Result<I64,I64>>,I64>{return MockReadRecords(r);}fn Main()->Result<(),I64>{let r=MockOpen(1);let a=F(r)?;return Ok(());}",
        "fn Main(){let x=MockWriteI64(1+2);let y=MockReply(200);}",
    ] {
        let program = nexo::ir::compile_scalar("temporary-view.n", source)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let execution = run_scalar(&program, MockEnvironment::empty());
        assert!(
            !matches!(execution.outcome, Outcome::InternalError(_)),
            "{source}"
        );
        assert_eq!(
            execution
                .observations
                .events()
                .iter()
                .filter(|event| event.starts_with("close:"))
                .count(),
            1.min(
                execution
                    .observations
                    .events()
                    .iter()
                    .filter(|event| event.starts_with("open:"))
                    .count()
            )
        );
    }
}

#[test]
fn read_batch_quota_failure_closes_temporary_resource() {
    let records = (0..31)
        .map(|_| r#"{"ok":"1"}"#)
        .collect::<Vec<_>>()
        .join(",");
    let env = MockEnvironment::parse(format!(r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{records}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#).as_bytes()).unwrap();
    let source = "fn Main()->Result<(),I64>{let a=MockReadRecords(MockOpen(1))?;return Ok(());}";
    let program = nexo::ir::compile_scalar_with_profile(
        "temporary-read.n",
        source,
        nexo::ir::ReducedProfile::Cells64,
        true,
    )
    .unwrap();
    let execution = nexo::interp::run_with_limits(
        &program,
        env,
        nexo::interp::ReducedExecutionProfile::Cells64,
    );
    assert_eq!(execution.diagnostics[0].code, "R0002");
    assert_eq!(
        execution.diagnostics[0].data["observed"],
        nexo::diagnostic::DataValue::UInt(65)
    );
    assert_eq!(execution.observations.events(), ["open:0:1", "close:0:1"]);
    assert_eq!(execution.observations.bytes(), 47);
    assert_eq!(execution.observations.reserved_bytes(), 0);

    let env = MockEnvironment::parse(
        br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"1"},{"ok":"1"},{"ok":"1"}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
    )
    .unwrap();
    let program = nexo::ir::compile_scalar_with_profile(
        "temporary-read.n",
        source,
        nexo::ir::ReducedProfile::Observations64,
        true,
    )
    .unwrap();
    let execution = nexo::interp::run_with_limits(
        &program,
        env,
        nexo::interp::ReducedExecutionProfile::Observations64,
    );
    assert_eq!(execution.diagnostics[0].code, "R0002");
    assert_eq!(execution.observations.events(), ["open:0:1", "close:0:1"]);
    assert_eq!(execution.observations.reserved_bytes(), 0);
}

#[test]
fn request_reply_queues_preserve_fifo_errors_and_ordered_observations() {
    let source = "fn Main()->Result<(),I64>{let a=MockRequest()?;let b=MockRequest()?;MockReply(a)?;MockReply(b)?;return Ok(());} ";
    let program = nexo::ir::compile_scalar("request.n", source).unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"0"},{"ok":"1"}],"mock_reply":[{"ok":null},{"ok":null}]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Ok);
    assert_eq!(
        execution.observations.events(),
        ["request:ok:0", "request:ok:1", "reply:0:ok", "reply:1:ok"]
    );
    assert_eq!(execution.observations.bytes(), 80);

    let source =
        "fn Main()->Result<(),I64>{let route=MockRequest()?;MockReply(404)?;return Ok(());} ";
    let program = nexo::ir::compile_scalar("request.n", source).unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"1"}],"mock_reply":[{"err":"5"}]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Err(5));
    assert_eq!(
        execution.observations.events(),
        ["request:ok:1", "reply:404:err:5"]
    );
    assert_eq!(
        execution
            .observations
            .observations()
            .iter()
            .map(|observation| (observation.channel, observation.text.as_str()))
            .collect::<Vec<_>>(),
        [
            (nexo::interp::observation::Channel::Event, "request:ok:1"),
            (nexo::interp::observation::Channel::Event, "reply:404:err:5"),
        ]
    );
    assert_eq!(execution.observations.bytes(), 45);

    let source =
        "fn Main()->Result<(),I64>{let route=MockRequest()?;MockReply(404)?;return Ok(());} ";
    let program = nexo::ir::compile_scalar("request.n", source).unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"err":"9"}],"mock_reply":[{"ok":null}]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Err(9));
    assert_eq!(execution.observations.events(), ["request:err:9"]);
    assert_eq!(execution.observations.bytes(), 22);

    let source = "fn Main()->Result<(),I64>{MockReply(200)?;MockReply(201)?;return Ok(());} ";
    let program = nexo::ir::compile_scalar("reply.n", source).unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[{"ok":null},{"err":"7"}]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Err(7));
    assert_eq!(
        execution.observations.events(),
        ["reply:200:ok", "reply:201:err:7"]
    );
    assert_eq!(execution.observations.bytes(), 45);
}

#[test]
fn flow_three_covers_route_404_and_both_propagation_errors() {
    let source = "fn Responder(route:view I64)->I64 with no_alloc,no_block,no_suspend,no_io{return if route==0 then {value:200;} else {value:404;};} fn Main()->Result<(),I64>{let route=MockRequest()?;MockReply(Responder(route))?;return Ok(());} ";
    let program = nexo::ir::compile_scalar("flow3.n", source).unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"1"}],"mock_reply":[{"ok":null}]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Ok);
    assert_eq!(
        execution.observations.events(),
        ["request:ok:1", "reply:404:ok"]
    );
    assert_eq!(execution.observations.bytes(), 42);

    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"err":"9"}],"mock_reply":[{"ok":null}]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Err(9));
    assert_eq!(execution.observations.events(), ["request:err:9"]);
    assert_eq!(execution.observations.bytes(), 22);

    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"1"}],"mock_reply":[{"err":"5"}]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Err(5));
    assert_eq!(
        execution.observations.events(),
        ["request:ok:1", "reply:404:err:5"]
    );
    assert_eq!(execution.observations.bytes(), 45);
}

#[test]
fn exhausted_mock_queues_return_the_contract_error() {
    let cases = [
        (
            "fn Main()->Result<(),I64>{let r=MockOpen(1);let x=MockReadRecords(r)?;return Ok(());}",
            "read_records:0:err:-9000",
            80,
        ),
        (
            "fn Main()->Result<(),I64>{MockWriteI64(25)?;return Ok(());}",
            "write_i64:25:err:-9000",
            31,
        ),
    ];
    for (source, event, bytes) in cases {
        let program = nexo::ir::compile_scalar("empty.n", source).unwrap();
        let execution = run_scalar(&program, MockEnvironment::empty());
        assert_eq!(execution.outcome, Outcome::Err(-9000));
        assert!(execution.observations.events().contains(&event));
        assert_eq!(execution.observations.bytes(), bytes);
    }
}

#[test]
fn observation_limits_are_atomic_and_include_close_reservations() {
    let program = nexo::ir::compile_scalar_with_profile(
        "bytes.n",
        "fn Main()->Result<(),I64>{MockWriteI64(1234)?;MockWriteI64(56789)?;return Ok(());}",
        nexo::ir::ReducedProfile::Observations64,
        true,
    )
    .unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"ok":null}],"mock_request":[],"mock_reply":[]}"#).unwrap();
    let execution = nexo::interp::run_with_limits(
        &program,
        env,
        nexo::interp::ReducedExecutionProfile::Observations64,
    );
    assert_eq!(execution.outcome, Outcome::Ok);
    assert_eq!(execution.observations.bytes(), 64);

    let program = nexo::ir::compile_scalar_with_profile(
        "bytes.n",
        "fn Main()->Result<(),I64>{let r=MockOpen(1);MockWriteI64(5)?;Inspect(r);return Ok(());}",
        nexo::ir::ReducedProfile::Observations64,
        true,
    )
    .unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#).unwrap();
    let execution = nexo::interp::run_with_limits(
        &program,
        env,
        nexo::interp::ReducedExecutionProfile::Observations64,
    );
    assert_eq!(execution.diagnostics[0].code, "R0002");
    assert_eq!(
        execution.diagnostics[0].data["observed"],
        nexo::diagnostic::DataValue::UInt(72)
    );
    assert_eq!(execution.observations.events(), ["open:0:1", "close:0:1"]);
    assert_eq!(execution.observations.reserved_bytes(), 0);
}

#[test]
fn reduced_cell_profile_covers_array_request_and_index_costs() {
    let records = (0..12)
        .map(|_| r#"{"ok":"1"}"#)
        .collect::<Vec<_>>()
        .join(",");
    let env = MockEnvironment::parse(format!(r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{records}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#).as_bytes()).unwrap();
    let big = vec!["0"; 50].join(",");
    let source = format!(
        "{}{}{}",
        "fn Main()->Result<(),I64>{{let a=MockReadRecords(MockOpen(1))?;} let big=[",
        big,
        "];return Ok(());}"
    );
    let program = nexo::ir::compile_scalar_with_profile(
        "cells.n",
        &source,
        nexo::ir::ReducedProfile::Cells64,
        true,
    )
    .unwrap();
    let execution = nexo::interp::run_with_limits(
        &program,
        env,
        nexo::interp::ReducedExecutionProfile::Cells64,
    );
    assert_eq!(execution.outcome, Outcome::Ok);

    for (prefix, suffix, size) in [
        (
            "fn Main()->Result<(),I64>{let r=MockRequest()?;",
            "return Ok(());}",
            61,
        ),
        ("fn Main(){let i=1;let y=[5,6,7][i];", "}", 61),
        (
            "fn F(v:view I64)->I64{return v;} fn Main(){let xs=[5,6,7];let i=1;let y=F(xs[i]);",
            "}",
            57,
        ),
        ("fn Main(){let xs=[5,6,7];let i=1;let y=xs[i];", "}", 57),
    ] {
        let source = format!("{prefix}let big=[{}];{suffix}", vec!["0"; size].join(","));
        let (program, env) = if source.contains("MockRequest") {
            (nexo::ir::compile_scalar_with_profile("cells.n", &source, nexo::ir::ReducedProfile::Cells64, true).unwrap(), MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"1"}],"mock_reply":[]}"#).unwrap())
        } else {
            (
                nexo::ir::compile_scalar_with_profile(
                    "cells.n",
                    &source,
                    nexo::ir::ReducedProfile::Cells64,
                    true,
                )
                .unwrap(),
                MockEnvironment::empty(),
            )
        };
        let execution = nexo::interp::run_with_limits(
            &program,
            env,
            nexo::interp::ReducedExecutionProfile::Cells64,
        );
        assert_eq!(execution.diagnostics[0].code, "R0002", "{source}");
        assert_eq!(
            execution.diagnostics[0].data["observed"],
            nexo::diagnostic::DataValue::UInt(65),
            "{source}"
        );
    }
}

#[test]
fn mock_environment_parser_enforces_exact_size_and_profile_boundaries() {
    let valid = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    let mut at_limit = valid.to_vec();
    at_limit.resize(1_048_576, b' ');
    assert!(MockEnvironment::parse(&at_limit).is_ok());
    at_limit.push(b' ');
    assert_eq!(
        MockEnvironment::parse(&at_limit).unwrap_err(),
        nexo::interp::MockEnvironmentError::TooLarge
    );

    // With exactly five members, a duplicate key necessarily replaces a
    // required schema field; with six members, the cardinality check rejects it.
    for input in [
        br#"{"schema_version":1,"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[]}"#.as_slice(),
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"mock_reply":[]}"#,
    ] {
        assert_eq!(
            MockEnvironment::parse(input).unwrap_err(),
            nexo::interp::MockEnvironmentError::InvalidSchema
        );
    }
    let depth_16 = format!(
        "{{\"schema_version\":1,\"mock_read_records\":{}null{},\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}",
        "[".repeat(15),
        "]".repeat(15)
    );
    assert_eq!(
        MockEnvironment::parse(depth_16.as_bytes()).unwrap_err(),
        nexo::interp::MockEnvironmentError::InvalidSchema
    );
    let depth_17 = format!(
        "{{\"schema_version\":1,\"mock_read_records\":{}null{},\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}",
        "[".repeat(16),
        "]".repeat(16)
    );
    assert_eq!(
        MockEnvironment::parse(depth_17.as_bytes()).unwrap_err(),
        nexo::interp::MockEnvironmentError::InvalidJson
    );

    for count in [10, 11] {
        let body = "a=a+1;".repeat(count);
        let source = format!("fn Main(){{var a=1;{body}}}");
        let result = nexo::ir::compile_scalar_with_profile(
            "types.n",
            &source,
            nexo::ir::ReducedProfile::Types32,
            false,
        );
        assert_eq!(result.is_ok(), count == 10, "count={count}");
        if let Err(nexo::ir::CompileError::Rejected(errors)) = result {
            assert_eq!(errors[0].code, "NX-E0003");
        }
    }

    let source = "fn Main(){let a=[1];let i=5;let y=a[i];}";
    let program = nexo::ir::compile_scalar_with_profile(
        "flow-global.n",
        source,
        nexo::ir::ReducedProfile::FlowCompilation32,
        true,
    )
    .unwrap();
    let execution = nexo::interp::run_with_limits(
        &program,
        MockEnvironment::empty(),
        nexo::interp::ReducedExecutionProfile::from_id("f0-0002-b-test-flow-global-32-v2")
            .expect("global flow test profile"),
    );
    assert_eq!(execution.diagnostics[0].code, "R0102");
    assert_eq!(
        execution.diagnostics[0].profile_id,
        "f0-0002-b-test-flow-global-32-v2"
    );
}

#[test]
fn mock_primitive_work_costs_use_exact_normal_limit() {
    // A2 §11.6, ll. 1026, 1062–1064: llamada y resultado de cada primitiva mock.
    let make = |iterations: usize, write: bool| {
        let mut source =
            format!("fn Main()->Result<(),I64>{{var i=0;while i<{iterations}{{i=i+1;}}");
        if write {
            for slot in 0..7 {
                source.push_str(&format!("let z{slot}=1;"));
            }
            source.push_str("MockWriteI64(25)?;return Ok(());}");
        } else {
            source.push_str("let z0=1;let a=MockReadRecords(MockOpen(1))?;return Ok(());}");
        }
        source
    };

    let source = make(71_425, true);
    let program = nexo::ir::compile_scalar("work.n", &source).unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.diagnostics[0].code, "R0001");
    assert_eq!(
        execution.diagnostics[0].data["observed"],
        nexo::diagnostic::DataValue::UInt(1_000_001)
    );
    assert_eq!(
        execution.diagnostics[0].data["work_consumed"],
        nexo::diagnostic::DataValue::UInt(1_000_000)
    );

    let source = make(71_425, true);
    let program = nexo::ir::compile_scalar("work.n", &source).unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"err":"40"}],"mock_request":[],"mock_reply":[]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Err(40));

    let source = make(71_426, false);
    let program = nexo::ir::compile_scalar("work.n", &source).unwrap();
    let env = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[{"ok":[]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#).unwrap();
    let execution = run_scalar(&program, env);
    assert_eq!(execution.outcome, Outcome::Ok);
}

#[test]
#[ignore = "deterministic I11a campaign; execute with scripts/i11a_campanas.py"]
fn i11a_seeded_runtime_campaign() {
    const BATCH: usize = 1_000;
    let (cases, mut state) = if cfg!(debug_assertions) {
        (100_000usize, 117_212u64)
    } else {
        (300_000usize, 117_211u64)
    };
    let seed = state;
    let source = "fn Main()->Result<(),I64>{var i=0;while i<250{let rows=MockReadRecords(MockOpen(i))?;let value=rows[0]?;i=i+1;}i=0;while i<250{MockWriteI64(i)?;i=i+1;}i=0;while i<250{let route=MockRequest()?;MockReply(route)?;i=i+1;}return Ok(());} ";
    let program = nexo::ir::compile_scalar("campaign.n", source).unwrap();
    let mut checked = 0usize;
    while checked < cases {
        let count = BATCH.min(cases - checked);
        assert_eq!(count % 4, 0);
        let per_primitive = count / 4;
        let mut numbers = Vec::with_capacity(per_primitive * 2);
        for _ in 0..count {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            if numbers.len() < per_primitive * 2 {
                numbers.push(((state >> 16) % 2_000_001) as i64 - 1_000_000);
            }
        }
        let reads = numbers[..per_primitive]
            .iter()
            .map(|n| format!(r#"{{"ok":[{{"ok":"{n}"}}]}}"#))
            .collect::<Vec<_>>()
            .join(",");
        let requests = numbers[per_primitive..]
            .iter()
            .map(|n| format!(r#"{{"ok":"{n}"}}"#))
            .collect::<Vec<_>>()
            .join(",");
        let writes = std::iter::repeat_n(r#"{"ok":null}"#, per_primitive)
            .collect::<Vec<_>>()
            .join(",");
        let replies = std::iter::repeat_n(r#"{"ok":null}"#, per_primitive)
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(
            r#"{{"schema_version":1,"mock_read_records":[{reads}],"mock_write_i64":[{writes}],"mock_request":[{requests}],"mock_reply":[{replies}]}}"#
        );
        let env = MockEnvironment::parse(json.as_bytes()).unwrap();
        let execution = run_scalar(&program, env);
        assert_eq!(execution.outcome, Outcome::Ok, "seed={seed} case={checked}");
        let mut expected = Vec::with_capacity(per_primitive * 8);
        for index in 0..per_primitive {
            expected.push(format!("open:{index}:{index}"));
            expected.push(format!("read_records:{index}:ok:1"));
            expected.push(format!("close:{index}:{index}"));
        }
        for index in 0..per_primitive {
            expected.push(format!("write_i64:{index}:ok"));
        }
        for number in &numbers[per_primitive..] {
            expected.push(format!("request:ok:{number}"));
            expected.push(format!("reply:{number}:ok"));
        }
        assert_eq!(
            execution.observations.events(),
            expected,
            "seed={seed} case={checked}"
        );
        let bytes = expected
            .iter()
            .map(|event| {
                let prefix = if event.starts_with("open:") || event.starts_with("close:") {
                    "mock-resource:"
                } else {
                    "mock-io:"
                };
                (prefix.len() + event.len() + 1) as u64
            })
            .sum::<u64>()
            + (0..per_primitive)
                .map(|n| (n.to_string().len() + 1) as u64)
                .sum::<u64>();
        assert_eq!(
            execution.observations.stdout(),
            (0..per_primitive)
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            execution.observations.bytes(),
            bytes,
            "seed={seed} case={checked}"
        );
        assert_eq!(execution.observations.reserved_bytes(), 0);
        checked += count;
    }
    eprintln!(
        "i11a_campaign_seed={seed} cases={checked} batch={BATCH} mode={} campaign=uniform_ok",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}

#[test]
#[ignore = "deterministic I11a mixed queue campaign; execute with scripts/i11a_campanas.py"]
fn i11a_seeded_mixed_queue_campaign() {
    const BATCH: usize = 1_000;
    let (cases, mut state) = if cfg!(debug_assertions) {
        (100_000usize, 117_213u64)
    } else {
        (300_000usize, 117_214u64)
    };
    let seed = state;
    let per_queue = BATCH / 4;
    let source = format!(
        "fn Main(){{var i=0;while i<{per_queue}{{MockWriteI64(i);MockRequest();MockReply(i+1000);i=i+1;}}}}"
    );
    let program = nexo::ir::compile_scalar("mixed-campaign.n", &source).unwrap();
    let mut checked = 0usize;
    while checked < cases {
        let count = BATCH.min(cases - checked);
        assert_eq!(count % 4, 0);
        let per_queue = count / 4;
        let mut writes = Vec::with_capacity(per_queue);
        let mut requests = Vec::with_capacity(per_queue);
        let mut replies = Vec::with_capacity(per_queue);
        for _ in 0..per_queue {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let write_ok = state & 1 == 0;
            let write_error = ((state >> 16) % 1_000 + 1) as i64;
            writes.push((write_ok, write_error));

            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let request_value = ((state >> 16) % 2_000_001) as i64 - 1_000_000;
            let request_ok = state & 1 == 0;
            requests.push(if request_ok {
                Ok(request_value)
            } else {
                Err(((state >> 32) % 1_000 + 1) as i64)
            });

            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            replies.push((state & 1 == 0, ((state >> 32) % 1_000 + 1) as i64));
        }
        assert!(writes.iter().any(|(ok, _)| *ok));
        assert!(writes.iter().any(|(ok, _)| !*ok));
        assert!(requests.iter().any(Result::is_ok));
        assert!(requests.iter().any(Result::is_err));
        assert!(replies.iter().any(|(ok, _)| *ok));
        assert!(replies.iter().any(|(ok, _)| !*ok));

        let writes_json = writes
            .iter()
            .map(|(ok, error)| {
                if *ok {
                    r#"{"ok":null}"#.to_owned()
                } else {
                    format!(r#"{{"err":"{error}"}}"#)
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        let requests_json = requests
            .iter()
            .map(|reply| match reply {
                Ok(value) => format!(r#"{{"ok":"{value}"}}"#),
                Err(error) => format!(r#"{{"err":"{error}"}}"#),
            })
            .collect::<Vec<_>>()
            .join(",");
        let replies_json = replies
            .iter()
            .map(|(ok, error)| {
                if *ok {
                    r#"{"ok":null}"#.to_owned()
                } else {
                    format!(r#"{{"err":"{error}"}}"#)
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(
            r#"{{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{writes_json}],"mock_request":[{requests_json}],"mock_reply":[{replies_json}]}}"#
        );
        let env = MockEnvironment::parse(json.as_bytes()).unwrap();
        let execution = run_scalar(&program, env);
        assert_eq!(
            execution.outcome,
            Outcome::Unit,
            "seed={seed} case={checked}"
        );
        assert!(
            execution.diagnostics.is_empty(),
            "seed={seed} case={checked}"
        );

        let mut expected_events = Vec::with_capacity(per_queue * 3);
        let mut expected_stdout = Vec::with_capacity(per_queue / 2);
        for index in 0..per_queue {
            let (write_ok, write_error) = writes[index];
            if write_ok {
                expected_events.push(format!("write_i64:{index}:ok"));
                expected_stdout.push(index.to_string());
            } else {
                expected_events.push(format!("write_i64:{index}:err:{write_error}"));
            }
            expected_events.push(match requests[index] {
                Ok(value) => format!("request:ok:{value}"),
                Err(error) => format!("request:err:{error}"),
            });
            let (reply_ok, reply_error) = replies[index];
            if reply_ok {
                expected_events.push(format!("reply:{}:ok", index + 1_000));
            } else {
                expected_events.push(format!("reply:{}:err:{reply_error}", index + 1_000));
            }
        }
        assert_eq!(
            execution.observations.events(),
            expected_events,
            "seed={seed} case={checked}"
        );
        assert_eq!(
            execution.observations.stdout(),
            expected_stdout,
            "seed={seed} case={checked}"
        );
        let bytes = expected_events
            .iter()
            .map(|event| ("mock-io:".len() + event.len() + 1) as u64)
            .sum::<u64>()
            + expected_stdout
                .iter()
                .map(|line| (line.len() + 1) as u64)
                .sum::<u64>();
        assert_eq!(
            execution.observations.bytes(),
            bytes,
            "seed={seed} case={checked}"
        );
        assert_eq!(execution.observations.reserved_bytes(), 0);
        checked += count;
    }
    eprintln!(
        "i11a_campaign_seed={seed} cases={checked} batch={BATCH} mode={} campaign=mixed_queue",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
}
