use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

use super::{check_with_partial_limit, names, parse};

#[test]
#[ignore = "full C0 pass meter over all frozen source/profile pairs"]
fn c0_complete_pass_meter_corpus() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_path = std::env::var_os("NEXO_PASS_METER_OUT")
        .map(std::path::PathBuf::from)
        .expect("NEXO_PASS_METER_OUT must point outside the repository");
    let expected_path = std::env::var_os("NEXO_PASS_METER_EXPECTED_OUT")
        .map(std::path::PathBuf::from)
        .expect("NEXO_PASS_METER_EXPECTED_OUT must point outside the repository");
    let source_script = root.join("docs/validation/i07c-rediseno/pass_meter_sources.py");
    let mut source_rows = Command::new("python")
        .args(["-B", source_script.to_str().expect("script path is UTF-8")])
        .current_dir(&root)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Python must stream the frozen pass-meter corpus");
    let stdout = source_rows.stdout.take().expect("source stream is piped");
    let mut rows = BufReader::new(stdout).lines();
    let mut output = BufWriter::new(
        std::fs::File::create(&output_path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", output_path.display())),
    );
    let mut expected = BufWriter::new(
        std::fs::File::create(&expected_path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", expected_path.display())),
    );
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let mut pair_count = 0usize;
    let inventory_header = rows
        .next()
        .expect("source inventory count is missing")
        .unwrap();
    let inventory_count = inventory_header
        .strip_prefix("#inventory-count\t")
        .expect("source inventory header has the wrong format")
        .parse::<usize>()
        .expect("source inventory count is not an integer");
    let inventory_path = root.join("docs/validation/i07c-rediseno/c0-pass-meter-inventory.json");
    let inventory =
        std::fs::read_to_string(&inventory_path).expect("frozen pass-meter inventory must exist");
    let total = inventory
        .split_once("\"total_unique_source_profile_pairs\":")
        .expect("frozen pass-meter inventory must include its source/profile total")
        .1
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse::<usize>()
        .expect("frozen source/profile total must be an integer");
    assert_eq!(
        total, inventory_count,
        "streamed universe differs from frozen pass-meter inventory"
    );

    for line in rows {
        let line = line.expect("source stream read failed");
        let mut fields = line.splitn(3, '\t');
        let source_sha256 = fields.next().expect("source hash field");
        let profile = fields.next().expect("profile field");
        let source_hex = fields.next().expect("source bytes field");
        let bytes = decode_hex(source_hex);
        let source = std::str::from_utf8(&bytes).expect("corpus source is UTF-8");
        let file = "c0-pass-meter.n";
        let ast = parse::parse_new_profile(file, source)
            .unwrap_or_else(|error| panic!("parse failed for {source_sha256}: {error:?}"));
        let allow_option = profile.starts_with("f0-0003-") || profile.starts_with("f0-0005-");
        let resolved = names::resolve_with_option(file, source, &ast, allow_option);
        for mode in ["check", "run"] {
            for function_ordinal in 0..ast.functions.len() {
                writeln!(
                    expected,
                    "{{\"source_sha256\":\"{source_sha256}\",\"profile\":\"{profile}\",\"mode\":\"{mode}\",\"build\":\"{build}\",\"function_ordinal\":{function_ordinal}}}"
                )
                .expect("pass-meter expected JSONL write failed");
            }
            expected
                .flush()
                .expect("pass-meter expected JSONL flush failed");

            // Both CLI modes type-check the same source before their mode-specific stage.
            // Run the checker once for each mode label so the meter has complete keys.
            let records = match &resolved {
                Ok(resolved) => {
                    let report = check_with_partial_limit(file, source, &ast, resolved, 1_000_000);
                    let stats = report.function_stats();
                    assert_eq!(
                        stats.len(),
                        ast.functions.len(),
                        "checker omitted function stats for {source_sha256} in {mode}"
                    );
                    stats.to_vec()
                }
                Err(_) => ast
                    .functions
                    .iter()
                    .enumerate()
                    .map(|(function_ordinal, _)| super::FunctionCheckStats {
                        function_ordinal: u32::try_from(function_ordinal).unwrap_or(u32::MAX),
                        passes: 0,
                        planning_calls: 0,
                    })
                    .collect(),
            };
            for entry in &records {
                assert!(
                    entry.passes <= 3,
                    "more than three check passes for {source_sha256}: {entry:?}"
                );
                writeln!(
                    output,
                    "{{\"source_sha256\":\"{source_sha256}\",\"profile\":\"{profile}\",\"mode\":\"{mode}\",\"build\":\"{build}\",\"function_ordinal\":{},\"passes\":{},\"planning_calls\":{}}}",
                    entry.function_ordinal, entry.passes, entry.planning_calls
                )
                .expect("pass meter JSONL write failed");
            }
            output.flush().expect("pass-meter flush failed");
        }
        pair_count += 1;
        if pair_count.is_multiple_of(1000) {
            output.flush().expect("pass meter flush failed");
            expected.flush().expect("expected pass meter flush failed");
            println!("pass meter {build}: {pair_count}/{inventory_count} source/profile pairs");
        }
    }
    output.flush().expect("pass meter flush failed");
    expected.flush().expect("expected pass meter flush failed");
    let source_status = source_rows.wait().expect("source generator wait failed");
    assert!(
        source_status.success(),
        "pass-meter source generator failed"
    );
    assert_eq!(pair_count, inventory_count, "incomplete pass-meter corpus");

    let meter_script = root.join("scripts/i07c_rd_pass_meter.py");
    let status = Command::new("python")
        .args([
            "-B",
            meter_script.to_str().expect("script path is UTF-8"),
            expected_path.to_str().expect("expected path is UTF-8"),
            output_path.to_str().expect("actual path is UTF-8"),
        ])
        .current_dir(&root)
        .status()
        .expect("Python must verify the complete pass-meter JSONL");
    assert!(status.success(), "pass-meter JSONL verification failed");
}

#[test]
fn c0_pass_meter_counts_only_full_checker_entries() {
    let source = "fn Helper(){let x=1;} fn Main(){let y=2;}";
    let ast = parse::parse_new_profile("c0-stats.n", source).unwrap();
    let resolved = names::resolve("c0-stats.n", source, &ast).unwrap();
    let report = check_with_partial_limit("c0-stats.n", source, &ast, &resolved, 1_000_000);
    let stats = report.function_stats();
    assert_eq!(stats.len(), ast.functions.len());
    assert_eq!(
        stats
            .iter()
            .map(|entry| entry.function_ordinal)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(stats.iter().all(|entry| entry.passes == 1));
    assert!(stats.iter().all(|entry| entry.planning_calls >= 1));
}

fn decode_hex(encoded: &str) -> Vec<u8> {
    assert_eq!(encoded.len() % 2, 0, "odd-length source hex");
    encoded
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let high = hex_value(pair[0]);
            let low = hex_value(pair[1]);
            high * 16 + low
        })
        .collect()
}

fn hex_value(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        _ => panic!("invalid source hex digit"),
    }
}
