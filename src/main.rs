use std::{env, ffi::OsStr, fs, process::ExitCode};

use nexo::diagnostic::{
    self, DataValue, Diagnostic, InputError, Namespace, Operation, Severity, SourceView, Span,
    Stage,
};

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
                    diagnostic::ExplainError::InvalidArguments,
                    pre_scan_format_os(&args_os[1..]),
                );
            };
            args.push(arg.to_owned());
        }
        return explain_command(&args);
    }
    // Reject every option and malformed invocation before opening the source.
    if args_os.len() != 2 || !(args_os[0] == OsStr::new("check") || args_os[0] == OsStr::new("run"))
    {
        eprintln!("nexo-driver:invalid_arguments");
        return ExitCode::from(2);
    }
    let Some(file) = args_os[1].to_str() else {
        eprintln!("nexo-driver:invalid_arguments");
        return ExitCode::from(2);
    };
    if file.starts_with("--") {
        eprintln!("nexo-driver:invalid_arguments");
        return ExitCode::from(2);
    }
    let bytes = match fs::File::open(file) {
        Ok(handle) => match diagnostic::read_bounded(handle) {
            Ok(bytes) => bytes,
            Err(error) => return input_failure(file, error, 0),
        },
        Err(error) => return input_failure(file, InputError::Unreadable(error.to_string()), 0),
    };
    let bytes_len = bytes.len();
    let source = match diagnostic::decode_utf8(bytes) {
        Ok(source) => source,
        Err(error) => return input_failure(file, error, bytes_len),
    };
    let compiled = if args_os[0] == OsStr::new("check") {
        nexo::ir::check_scalar(file, &source)
    } else {
        nexo::ir::compile_scalar(file, &source)
    };
    let program = match compiled {
        Ok(program) => program,
        Err(nexo::ir::CompileError::Rejected(errors)) => {
            for error in errors {
                if render_error(
                    &error,
                    file,
                    SourceView {
                        bytes_len,
                        text: Some(&source),
                    },
                )
                .is_err()
                {
                    return ExitCode::from(2);
                }
            }
            return ExitCode::FAILURE;
        }
        Err(_) => {
            eprintln!("nexo-driver:internal_error");
            return ExitCode::from(2);
        }
    };
    if args_os[0] == OsStr::new("check") {
        println!("nexo-check:ok");
        return ExitCode::SUCCESS;
    }
    // run_scalar verifies the program immediately before execution.
    let execution = nexo::interp::run_scalar(&program);
    for observation in execution.observations.observations() {
        match observation.channel {
            nexo::interp::observation::Channel::Stdout => println!("{}", observation.text),
            nexo::interp::observation::Channel::Event => {
                eprintln!("mock-resource:{}", observation.text)
            }
        }
    }
    for error in &execution.diagnostics {
        if render_error(
            error,
            file,
            SourceView {
                bytes_len,
                text: Some(&source),
            },
        )
        .is_err()
        {
            return ExitCode::from(2);
        }
    }
    match execution.outcome {
        nexo::interp::Outcome::Unit => {
            eprintln!("nexo-result:()");
            ExitCode::SUCCESS
        }
        nexo::interp::Outcome::SimulatorFault => ExitCode::FAILURE,
        nexo::interp::Outcome::InternalError(_) => {
            eprintln!("nexo-driver:internal_error");
            ExitCode::from(2)
        }
    }
}

fn render_error(error: &Diagnostic, file: &str, view: SourceView<'_>) -> Result<(), ()> {
    match error.render_text_view(file, view) {
        Ok(text) => {
            eprint!("{text}");
            Ok(())
        }
        Err(_) => {
            eprintln!("nexo-driver:internal_error");
            Err(())
        }
    }
}

fn input_failure(file: &str, error: InputError, bytes_len: usize) -> ExitCode {
    let (diagnostic, status, view) = match error {
        InputError::Unreadable(_) => {
            let mut d = diagnostic_base(
                "NX-E0004",
                Some("source_unreadable"),
                Stage::Input,
                Operation::InputRead,
                None,
                "No se puede leer la fuente.",
            );
            d.data
                .insert("input_path".into(), DataValue::String(file.into()));
            (
                d,
                2,
                SourceView {
                    bytes_len: 0,
                    text: None,
                },
            )
        }
        InputError::TooLarge { limit, observed } => {
            let mut d = diagnostic_base(
                "NX-E0003",
                None,
                Stage::Input,
                Operation::AnalysisLimit,
                Some(Span {
                    file: file.into(),
                    start: limit as u64,
                    end: observed as u64,
                }),
                "Fuente excede el límite de bytes.",
            );
            d.data.insert(
                "budget_kind".into(),
                DataValue::String("source_bytes".into()),
            );
            d.data.insert(
                "budget_scope".into(),
                DataValue::String("compilation".into()),
            );
            d.data.insert(
                "incomplete_analysis".into(),
                DataValue::String("input".into()),
            );
            d.data.insert("limit".into(), DataValue::UInt(limit as u64));
            d.data
                .insert("observed".into(), DataValue::UInt(observed as u64));
            (
                d,
                1,
                SourceView {
                    bytes_len: observed,
                    text: None,
                },
            )
        }
        InputError::InvalidUtf8 {
            start,
            end,
            bytes_hex,
            ..
        } => {
            let mut d = diagnostic_base(
                "NX-E0004",
                Some("invalid_utf8"),
                Stage::Input,
                Operation::InputDecode,
                Some(Span {
                    file: file.into(),
                    start: start as u64,
                    end: end as u64,
                }),
                "La fuente no es UTF-8 válido.",
            );
            d.data
                .insert("invalid_bytes_hex".into(), DataValue::String(bytes_hex));
            (
                d,
                2,
                SourceView {
                    bytes_len,
                    text: None,
                },
            )
        }
    };
    if render_error(&diagnostic, file, view).is_err() {
        return ExitCode::from(2);
    }
    ExitCode::from(status)
}

fn diagnostic_base(
    code: &str,
    subcause: Option<&str>,
    stage: Stage,
    operation: Operation,
    primary_span: Option<Span>,
    message: &str,
) -> Diagnostic {
    let rule = subcause.map_or_else(|| code.to_owned(), |s| format!("{code}/{s}"));
    Diagnostic {
        schema_version: diagnostic::SCHEMA_VERSION,
        diagnostic_namespace: Namespace::NexoProposedV1,
        profile_id: diagnostic::PROFILE_ID.into(),
        code: code.into(),
        subcause: subcause.map(str::to_owned),
        rule: rule.clone(),
        explanation_id: Some(rule),
        severity: Severity::Error,
        stage,
        message: message.into(),
        operation,
        primary_span,
        related_spans: vec![],
        cause_chain: vec![],
        feature_id: None,
        unexpected_token: None,
        expected_tokens: vec![],
        expected_tokens_kind: None,
        parser_context: None,
        recognized_prefix: None,
        repair: vec![],
        data: Default::default(),
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
