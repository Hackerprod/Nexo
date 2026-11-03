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
