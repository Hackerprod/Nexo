use std::process::Command;

#[test]
fn rd_table_matches_check_and_run() {
    // ADR §14.10, RD01–RD128 (RD79 withdrawn): the transcriber protects the
    // table fragment and checks the normative fields of all 152 expansions.
    let status = Command::new("python")
        .arg("scripts/i07c_rd_conformidad.py")
        .arg("--verify")
        .arg(env!("CARGO_BIN_EXE_nexo"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("Python must be available to transcribe ADR §14.10");
    assert!(status.success(), "RD conformity differs from ADR §14.10");
}

#[test]
fn rd_snapshots_are_reviewable_without_reclassifying_c() {
    // ADR §4.1.2 N(a), N(i), N(j), I(a–c), §14.10: synthetic extras are I
    // only after the structural filter preserves every normative C.
    let status = Command::new("python")
        .args(["-B", "scripts/i07c_rd_conformidad.py", "--self-test"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap();
    assert!(
        status.success(),
        "RD filter must reject fake spans, data and missing C"
    );
}
