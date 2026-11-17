use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const ERR_SOURCE: &str = "fn Use(a:move MockResource,b:view I64)->I64{Inspect(a);return b;}fn F()->Result<I64,I64>{let r=MockOpen(1);let x=Use(MockOpen(7),(match true{true=>{let q=MockOpen(2);let e:Result<I64,I64>=Err(9);let z=e?;value:3;},false=>{value:2;}}));return Ok(0);}fn Main(){let x=F();AssertEq(x,Err(9));}";

fn report(command: &str, source: &str) -> (i32, String) {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let file =
        std::env::temp_dir().join(format!("nexo-j16-question-{}-{id}.n", std::process::id()));
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

fn accepted(source: &str, events: &[&str]) {
    let (status, check) = report("check", source);
    assert_eq!(status, 0, "{check}");
    assert!(check.contains("\"kind\":\"checked\""), "{check}");
    assert!(check.contains("\"diagnostics\":[]"), "{check}");

    let (status, run) = report("run", source);
    assert_eq!(status, 0, "{run}");
    assert!(run.contains("\"kind\":\"unit\""), "{run}");
    assert!(run.contains("\"diagnostics\":[]"), "{run}");
    assert!(run.contains("\"reserved_observation_bytes\":0"), "{run}");
    let events = format!(
        "[{}]",
        events
            .iter()
            .map(|event| format!("\"{event}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    assert!(run.contains(&format!("\"events\":{events}")), "{run}");
}

#[test]
fn err_propagation_closes_pending_temporary_then_nested_and_outer_locals() {
    // AssertEq makes successful execution witness the propagated Err(9).
    accepted(
        ERR_SOURCE,
        &[
            "open:0:1",
            "open:1:7",
            "open:2:2",
            "close:1:7",
            "close:2:2",
            "close:0:1",
        ],
    );
}

#[test]
fn ok_propagation_keeps_the_pending_call_and_value() {
    let source = ERR_SOURCE
        .replace("Err(9);let z=e?", "Ok(9);let z=e?")
        .replace("AssertEq(x,Err(9))", "AssertEq(x,Ok(0))");
    accepted(
        &source,
        &[
            "open:0:1",
            "open:1:7",
            "open:2:2",
            "close:2:2",
            "inspect:1:7",
            "close:1:7",
            "close:0:1",
        ],
    );
}
