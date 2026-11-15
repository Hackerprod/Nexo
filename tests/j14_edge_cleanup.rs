use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn report(command: &str, source: &str) -> (i32, String) {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let file = std::env::temp_dir().join(format!("nexo-j14-edge-{}-{id}.n", std::process::id()));
    fs::write(&file, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([
            command,
            file.to_str().unwrap(),
            "--profile",
            "f0-0003-b-v1",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    fs::remove_file(file).unwrap();
    (
        output.status.code().expect("nexo must exit"),
        String::from_utf8(output.stdout).unwrap(),
    )
}

fn accepted(source: &str, events: &[&str], stdout: &[&str]) {
    let (status, check) = report("check", source);
    assert_eq!(status, 0, "check: {source}: {check}");
    assert!(check.contains("\"kind\":\"checked\""), "{check}");
    assert!(check.contains("\"diagnostics\":[]"), "{check}");
    let (status, run) = report("run", source);
    assert_eq!(status, 0, "run: {source}: {run}");
    assert!(run.contains("\"kind\":\"unit\""), "{run}");
    assert!(run.contains("\"diagnostics\":[]"), "{run}");
    assert!(run.contains("\"reserved_observation_bytes\":0"), "{run}");
    let list = |items: &[&str]| {
        format!(
            "[{}]",
            items
                .iter()
                .map(|item| format!("\"{item}\""))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    assert!(
        run.contains(&format!("\"events\":{}", list(events))),
        "{source}: {run}"
    );
    assert!(
        run.contains(&format!("\"stdout\":{}", list(stdout))),
        "{source}: {run}"
    );
}

#[test]
fn break_closes_pending_owner_before_body_local() {
    accepted(
        "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}fn Main(){var i=0;while i<1{let r=MockOpen(1);let x=Use(MockOpen(7),(if i==0 then{break;}else{value:1;}));i=i+1;}}",
        &["open:0:1", "open:1:7", "close:1:7", "close:0:1"],
        &[],
    );
    accepted(
        "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}fn Main(){var i=0;while i<1{let r=MockOpen(1);let x=Use(MockOpen(7),(if i==0 then{value:1;}else{break;}));i=i+1;}}",
        &[
            "open:0:1",
            "open:1:7",
            "inspect:1:7",
            "close:1:7",
            "close:0:1",
        ],
        &[],
    );
}

#[test]
fn for_return_uses_none_edge_state_without_body_temporaries() {
    accepted(
        "gen fn V()->I64{yield 1;}fn Main(){for i in V(){let x=1-(if i==0 then{return;}else{return;});}}",
        &[],
        &[],
    );
    accepted(
        "gen fn V()->I64{yield 1;}fn Main(){for i in V(){let x=1-(if i==0 then{continue;}else{return;});}}",
        &[],
        &[],
    );
}

#[test]
fn for_break_releases_prior_view_before_binding() {
    accepted(
        "fn Add(a:view I64,b:view I64)->I64{return a+b;}gen fn V()->I64{yield 0;}fn Main(){for i in V(){let x=Add(i,(if i==0 then{value:i;}else{break;}));}}",
        &[],
        &[],
    );
    accepted(
        "fn Add(a:view I64,b:view I64)->I64{return a+b;}gen fn V()->I64{yield 0;}fn Main(){for i in V(){let x=Add(i,(if i==0 then{value:i;}else{continue;}));}}",
        &[],
        &[],
    );
}

#[test]
fn unreachable_match_place_does_not_publish_layout() {
    accepted(
        "fn Main(){var i=0;while i<2{let x=(if true then{break;}else{break;})+(match i{0=>{value:0;},_=>{value:1;}});i=i+1;}}",
        &[],
        &[],
    );
    accepted(
        "fn Main(){var i=0;while i<2{let x=(if true then{break;}else{break;})+(match true{true=>{value:0;},false=>{value:1;}});i=i+1;}}",
        &[],
        &[],
    );
    accepted(
        "fn Main(){var a=[0];while true{let x=(if true then{break;}else{break;})+(match a[0]{0=>{value:0;},_=>{value:1;}});}}",
        &[],
        &[],
    );
    accepted(
        "fn Main(){var o=Some(1);while true{let x=(if true then{break;}else{break;})+(match edit o{Some(edit n)=>{value:n;},None=>{value:0;}});}}",
        &[],
        &[],
    );
}

#[test]
fn inner_loop_preserves_outer_operand_until_use() {
    accepted(
        "fn Main(){let x=1+(if true then{var j=0;while j<2{break;}value:j;}else{value:2;});}",
        &[],
        &[],
    );
    accepted(
        "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}fn Main(){let x=Use(MockOpen(7),(if true then{var j=0;while j<2{break;}value:j;}else{value:2;}));}",
        &["open:0:7", "inspect:0:7", "close:0:7"],
        &[],
    );
}

#[test]
fn for_advance_enters_from_reachable_continue() {
    accepted(
        "gen fn V()->I64{yield 1;yield 2;}fn Main(){for i in V(){let y=1+(if i==0 then{continue;}else{continue;});}}",
        &[],
        &[],
    );
    accepted(
        "gen fn V()->I64{yield 1;yield 2;}fn Main(){for i in V(){let y=1+(if i==0 then{value:i;}else{continue;});}}",
        &[],
        &[],
    );
}

#[test]
fn match_scrutinee_loan_ends_once_before_break() {
    accepted(
        "fn Main(){var i=0;while i<3{match i{0=>{break;},_=>{}}i=i+1;}}",
        &[],
        &[],
    );
    accepted(
        "fn Main(){var i=0;while i<3{match i{0=>{},_=>{}}i=i+1;}}",
        &[],
        &[],
    );
}

#[test]
fn return_closes_pending_owner_before_nested_arm_locals() {
    let events = [
        "open:0:1",
        "open:1:7",
        "open:2:2",
        "close:1:7",
        "close:2:2",
        "close:0:1",
    ];
    accepted(
        "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}fn Main(){var i=0;while i<1{let r=MockOpen(1);let x=Use(MockOpen(7),(match true{true=>{let q=MockOpen(2);return;},false=>{value:2;}}));i=i+1;}}",
        &events,
        &[],
    );
    accepted(
        "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}fn F()->Result<I64,I64>{let r=MockOpen(1);let x=Use(MockOpen(7),(if true then{if true{let q=MockOpen(2);return Ok(1);}value:1;}else{value:2;}));return Ok(x);}fn Main(){let u=F();}",
        &events,
        &[],
    );
}

#[test]
fn for_break_and_continue_close_body_owner_once() {
    let head = "gen fn V()->I64{yield 1;yield 2;}fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}";
    accepted(
        &format!(
            "{head}fn Main(){{for i in V(){{let r=MockOpen(1);let x=Use(MockOpen(7),(if i==1 then{{break;}}else{{value:2;}}));}}}}"
        ),
        &["open:0:1", "open:1:7", "close:1:7", "close:0:1"],
        &[],
    );
    accepted(
        &format!(
            "{head}fn Main(){{for i in V(){{let r=MockOpen(1);let x=Use(MockOpen(7),(if i==1 then{{continue;}}else{{value:2;}}));}}}}"
        ),
        &[
            "open:0:1",
            "open:1:7",
            "close:1:7",
            "close:0:1",
            "open:2:1",
            "open:3:7",
            "inspect:3:7",
            "close:3:7",
            "close:2:1",
        ],
        &[],
    );
}

#[test]
fn generator_yield_preserves_resource_until_resume_and_completion() {
    accepted(
        "gen fn V()->I64{let r=MockOpen(7);yield 2;yield 4;}fn Main(){for i in V(){Print(i);}}",
        &["open:0:7", "close:0:7"],
        &["2", "4"],
    );
}

#[test]
fn generator_yield_keeps_parent_expression_temp_until_resume() {
    accepted(
        "gen fn G()->I64{let x=1+(if true then{yield 2;value:3;}else{value:4;});yield x;}fn Main(){for v in G(){Print(v);}}",
        &[],
        &["2", "4"],
    );
    accepted(
        "gen fn G()->I64{let x=1+(if true then{value:3;}else{value:4;});yield x;}fn Main(){for v in G(){Print(v);}}",
        &[],
        &["4"],
    );
}

#[test]
fn break_closes_multiple_pending_operands_in_reverse_initialization_order() {
    accepted(
        "fn Use(a:move MockResource,b:move MockResource,c:view I64)->I64{Inspect(a);Inspect(b);return c;}fn Main(){var i=0;while i<1{let r=MockOpen(1);let x=Use(MockOpen(7),MockOpen(8),(if i==0 then{break;}else{value:1;}));i=i+1;}}",
        &[
            "open:0:1",
            "open:1:7",
            "open:2:8",
            "close:2:8",
            "close:1:7",
            "close:0:1",
        ],
        &[],
    );
}

#[test]
fn nested_for_and_match_preserve_parent_operand() {
    let head = "gen fn V()->I64{yield 1;}fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}";
    for inner in ["for j in V(){break;}", "for j in V(){continue;}"] {
        accepted(
            &format!(
                "{head}fn Main(){{let x=Use(MockOpen(7),(if true then{{{inner}value:2;}}else{{value:3;}}));}}"
            ),
            &["open:0:7", "inspect:0:7", "close:0:7"],
            &[],
        );
    }
    accepted(
        "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}fn Main(){let x=Use(MockOpen(7),(match true{true=>{var j=0;while j<2{break;}value:j;},false=>{value:2;}}));}",
        &["open:0:7", "inspect:0:7", "close:0:7"],
        &[],
    );
    accepted(
        "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}fn Main(){let x=Use(MockOpen(7),(if true then{var j=0;while j<2{match j{0=>{break;},_=>{}}j=j+1;}value:j;}else{value:2;}));}",
        &["open:0:7", "inspect:0:7", "close:0:7"],
        &[],
    );
}

#[test]
fn borrowed_match_permissions_keep_their_source_diagnostics() {
    accepted(
        "fn Main(){var o:Option<I64>=Some(1);match edit o{Some(edit n)=>{},None=>{}}}",
        &[],
        &[],
    );
    let (status, edit_report) = report(
        "check",
        "fn Main(){var o:Option<I64>=Some(1);match o{Some(edit n)=>{},None=>{}}}",
    );
    assert_eq!(status, 1, "{edit_report}");
    assert_eq!(
        edit_report.matches("\"code\":\"NX-E1105\"").count(),
        1,
        "{edit_report}"
    );
    for expected in [
        "\"subcause\":\"permission_escalation\"",
        "\"operation\":\"ownership.access\"",
        "\"start\":49,\"end\":55",
        "\"data\":{\"active_permission\":\"read\",\"origin\":{\"id\":\"binding:14\",\"kind\":\"binding\"},\"permission_source\":{\"id\":\"loan:42:0\",\"kind\":\"loan\"},\"requested_access\":\"borrow_inout\"}",
        "\"role\":\"declaration\"",
        "\"role\":\"origin\"",
        "\"operation\":\"borrow.read\"",
    ] {
        assert!(
            edit_report.contains(expected),
            "missing {expected}: {edit_report}"
        );
    }
    accepted(
        "fn Main(){let o=Some(1);match move o{Some(move n)=>{},None=>{}}}",
        &[],
        &[],
    );
    let (status, move_report) = report(
        "check",
        "fn F(o:view Option<I64>){match move o{Some(move n)=>{},None=>{}}}fn Main(){let o=Some(1);F(o);}",
    );
    assert_eq!(status, 1, "{move_report}");
    assert_eq!(
        move_report.matches("\"code\":\"NX-E1105\"").count(),
        1,
        "{move_report}"
    );
    for expected in [
        "\"subcause\":\"permission_escalation\"",
        "\"operation\":\"ownership.access\"",
        "\"start\":36,\"end\":37",
        "\"data\":{\"active_permission\":\"read\",\"origin\":{\"id\":\"parameter:5\",\"kind\":\"parameter\"},\"parameter\":{\"id\":\"parameter:5\",\"kind\":\"parameter\"},\"requested_access\":\"consume\"}",
        "\"operation\":\"borrow.read\"",
    ] {
        assert!(
            move_report.contains(expected),
            "missing {expected}: {move_report}"
        );
    }
    accepted(
        "fn Main(){var o:Option<I64>=Some(1);match o{Some(n)=>{},None=>{}}}",
        &[],
        &[],
    );
    let (status, view_report) = report(
        "check",
        "fn Main(){var o:Option<I64>=Some(1);match view o{Some(n)=>{},None=>{}}}",
    );
    assert_eq!(status, 1, "{view_report}");
    assert_eq!(
        view_report.matches("\"code\":\"NX-E0002\"").count(),
        1,
        "{view_report}"
    );
    for expected in [
        "\"start\":47,\"end\":48",
        "\"unexpected_token\":{\"kind\":\"name\",\"text\":\"o\"",
        "\"expected_tokens\":[\"{\"]",
        "\"parser_context\":\"grammar.Match\"",
    ] {
        assert!(
            view_report.contains(expected),
            "missing {expected}: {view_report}"
        );
    }
}

#[test]
fn return_from_nested_block_and_loop_closes_temporaries_before_locals() {
    let events = [
        "open:0:1",
        "open:1:7",
        "open:2:2",
        "close:1:7",
        "close:2:2",
        "close:0:1",
    ];
    let head = "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}";
    let tail = "fn Main(){let u=F();}";
    for body in [
        "{let q=MockOpen(2);return Ok(1);}value:1;",
        "var j=0;while j<1{let q=MockOpen(2);return Ok(1);}value:1;",
        "let q=MockOpen(2);if true{return Ok(1);}value:1;",
    ] {
        accepted(
            &format!(
                "{head}fn F()->Result<I64,I64>{{let r=MockOpen(1);let x=Use(MockOpen(7),(if true then{{{body}}}else{{value:2;}}));return Ok(x);}}{tail}"
            ),
            &events,
            &[],
        );
    }
}

#[test]
fn propagated_error_keeps_the_same_temporary_before_local_order() {
    accepted(
        "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}fn F()->Result<I64,I64>{let r=MockOpen(1);let x=Use(MockOpen(7),(match true{true=>{let q=MockOpen(2);let e:Result<I64,I64>=Err(9);let z=e?;value:3;},false=>{value:2;}}));return Ok(0);}fn Main(){let x=F();}",
        &[
            "open:0:1",
            "open:1:7",
            "open:2:2",
            "close:1:7",
            "close:2:2",
            "close:0:1",
        ],
        &[],
    );
}
