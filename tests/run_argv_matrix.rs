use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const DEFAULT_PROFILE: &str = "f0-0002-b-v2";
const OTHER_PROFILE: &str = "f0-0002-b-test-types-32-v2";
const EMPTY_ENV: &str = r#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
const OPTIONS: [&str; 3] = ["--mock-env", "--format", "--profile"];
const PROFILE_IDS: [&str; 9] = [
    DEFAULT_PROFILE,
    "f0-0002-b-test-types-32-v2",
    "f0-0002-b-test-flow-32-v2",
    "f0-0002-b-test-flow-global-32-v2",
    "f0-0002-b-test-effects-32-v2",
    "f0-0002-b-test-ir-32-v2",
    "f0-0002-b-test-frames-8-v2",
    "f0-0002-b-test-cells-64-v2",
    "f0-0002-b-test-observations-64-v2",
];

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-fixtures")
            .join(format!(
                "i11a-val62-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("p.n"), "fn Main(){}").unwrap();
        fs::write(path.join("-p.n"), "fn Main(){}").unwrap();
        for name in ["env.json", "-x", "-", "other-env.json", "env--route.json"] {
            fs::write(path.join(name), EMPTY_ENV).unwrap();
        }
        Self(path)
    }

    fn invoke(&self, args: &[String]) -> Actual {
        let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap();
        Actual::from_output(output)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Actual {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Actual {
    fn from_output(output: Output) -> Self {
        Self {
            code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }

    fn literal(code: i32, stdout: impl AsRef<[u8]>, stderr: impl AsRef<[u8]>) -> Self {
        Self {
            code: Some(code),
            stdout: stdout.as_ref().to_vec(),
            stderr: stderr.as_ref().to_vec(),
        }
    }
}

#[derive(Clone, Copy)]
enum ValueClass {
    AbsentOption,
    MissingValue,
    Literal(&'static str),
}

#[derive(Clone, Copy)]
enum Repetition {
    Once,
    Same,
    Different,
}

fn groups(option: &str, value: ValueClass, repetition: Repetition) -> Vec<Vec<String>> {
    match value {
        ValueClass::AbsentOption => Vec::new(),
        ValueClass::MissingValue => vec![vec![option.to_owned()]],
        ValueClass::Literal(first) => {
            let mut output = vec![vec![option.to_owned(), first.to_owned()]];
            match repetition {
                Repetition::Once => {}
                Repetition::Same => output.push(vec![option.to_owned(), first.to_owned()]),
                Repetition::Different => {
                    let second = match option {
                        "--format" if first == "text" => "json",
                        "--format" => "text",
                        "--profile" if first == DEFAULT_PROFILE => OTHER_PROFILE,
                        "--profile" => DEFAULT_PROFILE,
                        "--mock-env" if first == "env.json" => "other-env.json",
                        "--mock-env" => "env.json",
                        _ => unreachable!(),
                    };
                    output.push(vec![option.to_owned(), second.to_owned()]);
                }
            }
            output
        }
    }
}

fn argv_at_position(
    command: &str,
    option_groups: &[Vec<String>],
    position: usize,
    program: &str,
) -> Vec<String> {
    let mut argv = vec![command.to_owned()];
    for index in 0..=option_groups.len() {
        if index == position {
            argv.push(program.to_owned());
        }
        if let Some(group) = option_groups.get(index) {
            argv.extend(group.iter().cloned());
        }
    }
    argv
}

fn usage() -> Actual {
    Actual::literal(2, "", "nexo-driver:invalid_arguments\n")
}

fn driver_error(profile: &str, reason: &str, format: &str) -> Actual {
    if format == "json" {
        Actual::literal(
            2,
            format!(
                "{{\"schema_version\":1,\"profile_id\":\"{profile}\",\"outcome\":{{\"kind\":\"driver_error\",\"reason\":\"{reason}\"}},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}}\n"
            ),
            "",
        )
    } else {
        Actual::literal(2, "", format!("nexo-driver:{reason}\n"))
    }
}

/// Oracle transcribed from A2 §12.4 and D15 §7.1. D4 fixes the split-token
/// rule: a value beginning with `--` is an option, while a single leading
/// dash is ordinary value/path text. D5 records the coordinator's resolution
/// for command spellings, bare `--`, and empty values not fully specified by
/// those clauses. This model does not inspect the implementation parser.
fn expected(argv: &[String], directory: &Path) -> Actual {
    let Some(command) = argv.first().map(String::as_str) else {
        return usage();
    };
    if !matches!(command, "run" | "check") {
        return usage();
    }

    let mut values = BTreeMap::new();
    let mut files = Vec::new();
    let mut index = 1;
    while index < argv.len() {
        let token = &argv[index];
        if token.starts_with("--") {
            if token.contains('=') || !OPTIONS.contains(&token.as_str()) {
                return usage();
            }
            if token == "--mock-env" && command != "run" {
                return usage();
            }
            if values.contains_key(token.as_str()) || index + 1 >= argv.len() {
                return usage();
            }
            let value = &argv[index + 1];
            if value.starts_with("--") {
                return usage();
            }
            values.insert(token.as_str(), value.as_str());
            index += 2;
        } else {
            files.push(token.as_str());
            index += 1;
        }
    }
    if files.len() != 1 {
        return usage();
    }

    let format = values.get("--format").copied().unwrap_or("text");
    if !matches!(format, "text" | "json") {
        return usage();
    }
    let profile = values.get("--profile").copied().unwrap_or(DEFAULT_PROFILE);
    if !PROFILE_IDS.contains(&profile) {
        return driver_error(profile, "unknown_profile", format);
    }

    let source = directory.join(files[0]);
    if !source.is_file() {
        return driver_error(profile, "source_unreadable", format);
    }
    if let Some(path) = values.get("--mock-env")
        && !directory.join(path).is_file()
    {
        return driver_error(profile, "mock_env_unreadable", format);
    }

    match (command, format) {
        ("check", "text") => Actual::literal(0, "nexo-check:ok\n", ""),
        ("check", "json") => Actual::literal(
            0,
            format!(
                "{{\"schema_version\":1,\"profile_id\":\"{profile}\",\"outcome\":{{\"kind\":\"checked\"}},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}}\n"
            ),
            "",
        ),
        ("run", "text") => Actual::literal(0, "", "nexo-result:()\n"),
        ("run", "json") => Actual::literal(
            0,
            format!(
                "{{\"schema_version\":1,\"profile_id\":\"{profile}\",\"outcome\":{{\"kind\":\"unit\"}},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}}\n"
            ),
            "",
        ),
        _ => unreachable!(),
    }
}

fn value_cases(option: &str) -> Vec<(&'static str, ValueClass, Repetition)> {
    let mut cases = vec![
        ("option-absent", ValueClass::AbsentOption, Repetition::Once),
        ("missing-value", ValueClass::MissingValue, Repetition::Once),
    ];
    let values: &[&str] = match option {
        "--mock-env" => &[
            "",
            "-",
            "-x",
            "--",
            "--x",
            DEFAULT_PROFILE,
            "F0-0002-B-V2",
            "env.json",
            "missing-env.json",
        ],
        "--format" => &[
            "",
            "-",
            "-x",
            "--",
            "--x",
            DEFAULT_PROFILE,
            "F0-0002-B-V2",
            "env.json",
            "missing-env.json",
        ],
        "--profile" => &[
            "",
            "-",
            "-x",
            "--",
            "--x",
            DEFAULT_PROFILE,
            "F0-0002-B-V2",
            "env.json",
            "missing-env.json",
        ],
        _ => unreachable!(),
    };
    for value in values {
        for repetition in [Repetition::Once, Repetition::Same, Repetition::Different] {
            cases.push(("value", ValueClass::Literal(value), repetition));
        }
    }
    cases
}

fn permutations<T: Copy>(items: [T; 3]) -> Vec<[T; 3]> {
    vec![
        [items[0], items[1], items[2]],
        [items[0], items[2], items[1]],
        [items[1], items[0], items[2]],
        [items[1], items[2], items[0]],
        [items[2], items[0], items[1]],
        [items[2], items[1], items[0]],
    ]
}

#[test]
fn generated_run_cli_argv_matrix_matches_d5_d15_a2_oracle() {
    let fixture = Fixture::new();
    let commands = ["run", "check", "runx", "mystery"];
    let programs = ["p.n", "-p.n"];
    let mut cases = 0usize;

    for command in commands {
        for option in OPTIONS {
            for (label, value, repetition) in value_cases(option) {
                let option_groups = groups(option, value, repetition);
                for program in programs {
                    for position in 0..=option_groups.len() {
                        let argv = argv_at_position(command, &option_groups, position, program);
                        assert_eq!(
                            fixture.invoke(&argv),
                            expected(&argv, &fixture.0),
                            "case={label} command={command} option={option} position={position} program={program} argv={argv:?}"
                        );
                        cases += 1;
                    }
                }
            }
        }
    }

    // Every order of the three published options is exercised with the source
    // before, between, and after option groups. D15 §7.1 permits any position.
    for command in ["run", "check"] {
        for order in permutations(OPTIONS) {
            let option_groups = order
                .iter()
                .map(|option| match *option {
                    "--mock-env" => vec![option.to_string(), "env.json".to_owned()],
                    "--format" => vec![option.to_string(), "json".to_owned()],
                    "--profile" => vec![option.to_string(), DEFAULT_PROFILE.to_owned()],
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>();
            for program in programs {
                for position in 0..=option_groups.len() {
                    let argv = argv_at_position(command, &option_groups, position, program);
                    assert_eq!(
                        fixture.invoke(&argv),
                        expected(&argv, &fixture.0),
                        "ordered argv={argv:?}"
                    );
                    cases += 1;
                }
            }
        }
    }

    let fuzz_cases = std::env::var("NEXO_CLI_ARGV_FUZZ_CASES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut state = std::env::var("NEXO_CLI_ARGV_FUZZ_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0x11a0_0062_2026_1112);
    for case in 0..fuzz_cases {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let command = commands[(state as usize) % commands.len()];
        let option = OPTIONS[((state >> 8) as usize) % OPTIONS.len()];
        let choices = value_cases(option);
        let (_, value, repetition) = choices[((state >> 16) as usize) % choices.len()];
        let option_groups = groups(option, value, repetition);
        let position = ((state >> 24) as usize) % (option_groups.len() + 1);
        let program = programs[((state >> 32) as usize) % programs.len()];
        let argv = argv_at_position(command, &option_groups, position, program);
        assert_eq!(
            fixture.invoke(&argv),
            expected(&argv, &fixture.0),
            "seeded argv case={case} seed_state={state} argv={argv:?}"
        );
        cases += 1;
    }

    let branch_cases: &[&[&str]] = &[
        &["run", "p.n", "--unknown"],
        &["run", "--unknown", "p.n"],
        &["run", "--format=json", "p.n"],
        &["run", "p.n", "--format=json"],
        &["run", "p.n", "p.n"],
        &["check", "p.n", "--mock-env", "env.json"],
        &["check", "--mock-env", "env.json", "p.n"],
        &["run", "p.n", "--"],
    ];
    for raw in branch_cases {
        let argv = raw
            .iter()
            .map(|part| (*part).to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            fixture.invoke(&argv),
            expected(&argv, &fixture.0),
            "argv={argv:?}"
        );
        cases += 1;
    }

    eprintln!(
        "val62_cli_argv_matrix cases={cases} dimensions=command-option-value-position-repetition fuzz_seeded={fuzz_cases}"
    );
}

#[test]
fn format_option_rejects_values_outside_the_closed_text_json_pair() {
    let fixture = Fixture::new();
    for value in ["", "-", "-x", "--", "--x", "---", "text--json", "JSON"] {
        let args = ["run", "p.n", "--format", value]
            .map(str::to_owned)
            .to_vec();
        assert_eq!(fixture.invoke(&args), usage(), "format={value:?}");
    }
    assert_eq!(
        fixture.invoke(&["run", "p.n", "--format", "text"].map(str::to_owned)),
        Actual::literal(0, "", "nexo-result:()\n")
    );
    assert_eq!(
        fixture.invoke(&["run", "p.n", "--format", "json"].map(str::to_owned)),
        Actual::literal(
            0,
            format!(
                "{{\"schema_version\":1,\"profile_id\":\"{DEFAULT_PROFILE}\",\"outcome\":{{\"kind\":\"unit\"}},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}}\n"
            ),
            ""
        )
    );
}
