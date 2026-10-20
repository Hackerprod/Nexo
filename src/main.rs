use std::{env, ffi::OsStr, fs, process::ExitCode};
fn main() -> ExitCode {
    let args_os: Vec<_> = env::args_os().skip(1).collect();
    if args_os
        .first()
        .is_some_and(|arg| arg == OsStr::new("explain"))
    {
        let mut args = Vec::new();
        for arg in &args_os[1..] {
            let Some(arg) = arg.to_str() else {
                return explain_error(
                    None,
                    None,
                    nexo::diagnostic::ExplainError::InvalidArguments,
                    pre_scan_format_os(&args_os[1..]),
                );
            };
            args.push(arg.to_owned());
        }
        return explain_command(&args);
    }
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 2 || !["check", "run"].contains(&args[0].as_str()) {
        eprintln!(
            "usage: nexo <check|run> <file.nx>\nF0 slice 1; unsupported Nexo features are rejected."
        );
        return ExitCode::from(2);
    }
    // Bound the read itself, rather than allocating an arbitrarily large file
    // before the parser can enforce its source-size limit.
    let bytes = match fs::File::open(&args[1]) {
        Ok(file) => match nexo::diagnostic::read_bounded(file) {
            Ok(bytes) => bytes,
            Err(nexo::diagnostic::InputError::TooLarge { .. }) => {
                eprintln!("E0003:1:1: F0 source limit is 1 MiB");
                return ExitCode::FAILURE;
            }
            Err(nexo::diagnostic::InputError::Unreadable(error)) => {
                eprintln!("E0004: cannot read UTF-8 source: {error}");
                return ExitCode::from(2);
            }
            Err(error) => unreachable!("unexpected bounded-read error: {error:?}"),
        },
        Err(e) => {
            eprintln!("E0004: cannot read UTF-8 source: {e}");
            return ExitCode::from(2);
        }
    };
    let source = match nexo::diagnostic::decode_utf8(bytes) {
        Ok(source) => source,
        Err(nexo::diagnostic::InputError::InvalidUtf8 { message, .. }) => {
            eprintln!("E0004: cannot read UTF-8 source: {message}");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("E0004: cannot read UTF-8 source: {e:?}");
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

fn explain_command(args: &[String]) -> ExitCode {
    let mut code = None;
    let mut subcause = None;
    let mut format = pre_scan_format(args);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--subcause" if subcause.is_none() && i + 1 < args.len() => {
                subcause = Some(args[i + 1].as_str());
                i += 2;
            }
            "--format" if i + 1 < args.len() && matches!(args[i + 1].as_str(), "text" | "json") => {
                format = args[i + 1].as_str();
                i += 2;
            }
            value if !value.starts_with('-') && code.is_none() => {
                code = Some(value);
                i += 1;
            }
            _ => {
                return explain_error(
                    code,
                    subcause,
                    nexo::diagnostic::ExplainError::InvalidArguments,
                    format,
                );
            }
        }
    }
    let Some(code) = code else {
        return explain_error(
            None,
            subcause,
            nexo::diagnostic::ExplainError::InvalidArguments,
            format,
        );
    };
    match nexo::diagnostic::explain(code, subcause) {
        Ok(entries) => {
            if format == "json" {
                println!("{}", nexo::diagnostic::explanation_json(code, &entries));
            } else {
                print!("{}", nexo::diagnostic::explanation_text(&entries));
            }
            ExitCode::SUCCESS
        }
        Err(error) => explain_error(Some(code), subcause, error, format),
    }
}

fn pre_scan_format(args: &[String]) -> &'static str {
    match args
        .windows(2)
        .find(|pair| pair[0] == "--format")
        .map(|pair| pair[1].as_str())
    {
        Some("json") => "json",
        _ => "text",
    }
}

fn pre_scan_format_os(args: &[std::ffi::OsString]) -> &'static str {
    match args
        .windows(2)
        .find(|pair| pair[0] == OsStr::new("--format"))
        .and_then(|pair| pair[1].to_str())
    {
        Some("json") => "json",
        _ => "text",
    }
}

fn explain_error(
    code: Option<&str>,
    subcause: Option<&str>,
    error: nexo::diagnostic::ExplainError,
    format: &str,
) -> ExitCode {
    if format == "json" {
        println!(
            "{}",
            nexo::diagnostic::explain_error_json(error, code, subcause)
        );
    } else {
        let reason = match error {
            nexo::diagnostic::ExplainError::UnknownCode => "unknown_code",
            nexo::diagnostic::ExplainError::UnknownSubcause => "unknown_subcause",
            nexo::diagnostic::ExplainError::InvalidArguments => "invalid_arguments",
        };
        eprintln!("nexo-explain:{reason}");
    }
    ExitCode::from(2)
}
