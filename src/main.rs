use std::{env, fs, io::Read, process::ExitCode};
fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 2 || !["check", "run"].contains(&args[0].as_str()) {
        eprintln!(
            "usage: nexo <check|run> <file.nx>\nF0 slice 1; unsupported Nexo features are rejected."
        );
        return ExitCode::from(2);
    }
    // Bound the read itself, rather than allocating an arbitrarily large file
    // before the parser can enforce its source-size limit.
    let bytes = match fs::File::open(&args[1]).and_then(|f| {
        let mut bytes = Vec::new();
        f.take(1_048_577).read_to_end(&mut bytes)?;
        Ok(bytes)
    }) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("E0004: cannot read UTF-8 source: {e}");
            return ExitCode::from(2);
        }
    };
    if bytes.len() > 1_048_576 {
        eprintln!("E0003:1:1: F0 source limit is 1 MiB");
        return ExitCode::FAILURE;
    }
    let source = match String::from_utf8(bytes) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("E0004: cannot read UTF-8 source: {e}");
            return ExitCode::from(2);
        }
    };
    let program = match nexo::compile(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e.render(&source));
            return ExitCode::FAILURE;
        }
    };
    if args[0] == "check" {
        println!("checked: F0 slice 1 (not full-language validation)");
        return ExitCode::SUCCESS;
    }
    let result = nexo::runtime::execute(&program);
    for line in result.output {
        println!("{line}")
    }
    for event in result.events {
        eprintln!("mock-resource:{event}")
    }
    if let Some(error) = result.error {
        eprintln!("{}", error.render(&source));
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
