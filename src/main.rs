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
    let Some((command, file, mock_path, profile_id, format)) = parse_invocation(&args_os) else {
        return driver_error(None, "invalid_arguments", false);
    };
    if nexo::ir::ReducedProfile::from_id(&profile_id).is_none() {
        return driver_error(Some(&profile_id), "unknown_profile", format == "json");
    }
    let bytes = match fs::File::open(file) {
        Ok(handle) => match diagnostic::read_bounded(handle) {
            Ok(bytes) => bytes,
            Err(error) => return input_failure(file, error, 0, &profile_id, format),
        },
        Err(error) => {
            return input_failure(
                file,
                InputError::Unreadable(error.to_string()),
                0,
                &profile_id,
                format,
            );
        }
    };
    let bytes_len = bytes.len();
    let source = match diagnostic::decode_utf8(bytes) {
        Ok(source) => source,
        Err(error) => return input_failure(file, error, bytes_len, &profile_id, format),
    };
    let Some(profile) = nexo::ir::ReducedProfile::from_id(&profile_id) else {
        unreachable!()
    };
    let compiled = nexo::ir::compile_scalar_with_profile(file, &source, profile, command == "run");
    let program = match compiled {
        Ok(program) => program,
        Err(nexo::ir::CompileError::Rejected(mut errors)) => {
            for error in &mut errors {
                error.profile_id = profile_id.clone();
            }
            if format == "json" {
                return diagnostic_result(
                    file,
                    &source,
                    bytes_len,
                    &profile_id,
                    "rejected",
                    &errors,
                    1,
                );
            }
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
        Err(error) => {
            let (status, message) = internal_error_exit(&error);
            if format == "json" {
                return driver_error(Some(&profile_id), "internal_error", true);
            }
            eprintln!("{message}");
            return ExitCode::from(status);
        }
    };
    if command == "check" {
        if format == "json" {
            println!(
                "{}",
                execution_json(&profile_id, r#"{"kind":"checked"}"#, &[], 0, 0, &[])
            );
            return ExitCode::SUCCESS;
        }
        println!("nexo-check:ok");
        return ExitCode::SUCCESS;
    }
    let mock_env = if let Some(path) = mock_path {
        let env_bytes = match fs::File::open(path) {
            Ok(handle) => match diagnostic::read_bounded(handle) {
                Ok(bytes) => bytes,
                Err(InputError::TooLarge { .. }) => {
                    return driver_error(Some(&profile_id), "mock_env_too_large", format == "json");
                }
                Err(_) => {
                    return driver_error(
                        Some(&profile_id),
                        "mock_env_unreadable",
                        format == "json",
                    );
                }
            },
            Err(_) => {
                return driver_error(Some(&profile_id), "mock_env_unreadable", format == "json");
            }
        };
        match nexo::interp::MockEnvironment::parse(&env_bytes) {
            Ok(mock_env) => mock_env,
            Err(nexo::interp::MockEnvironmentError::InvalidJson) => {
                return driver_error(Some(&profile_id), "invalid_mock_json", format == "json");
            }
            Err(nexo::interp::MockEnvironmentError::InvalidSchema) => {
                return driver_error(Some(&profile_id), "invalid_mock_schema", format == "json");
            }
            Err(nexo::interp::MockEnvironmentError::TooLarge) => {
                return driver_error(Some(&profile_id), "mock_env_too_large", format == "json");
            }
        }
    } else {
        nexo::interp::MockEnvironment::empty()
    };
    // run_scalar verifies the program immediately before execution.
    let runtime_profile = nexo::interp::ReducedExecutionProfile::from_id(&profile_id).unwrap();
    let execution = nexo::interp::run_with_limits(&program, mock_env, runtime_profile);
    if format == "json" {
        let outcome = match execution.outcome {
            nexo::interp::Outcome::Unit => r#"{"kind":"unit"}"#.to_owned(),
            nexo::interp::Outcome::Ok => r#"{"kind":"ok"}"#.to_owned(),
            nexo::interp::Outcome::Err(error) => format!(r#"{{"kind":"err","error":"{error}"}}"#),
            nexo::interp::Outcome::SimulatorFault => r#"{"kind":"simulator_fault"}"#.to_owned(),
            nexo::interp::Outcome::InternalError(_) => {
                r#"{"kind":"driver_error","reason":"internal_error"}"#.to_owned()
            }
        };
        let diagnostics_json: Vec<String> = execution
            .diagnostics
            .iter()
            .map(|d| {
                d.to_json_view(
                    file,
                    SourceView {
                        bytes_len,
                        text: Some(&source),
                    },
                )
                .unwrap_or_else(|_| "null".to_owned())
            })
            .collect();
        println!(
            "{}",
            execution_json(
                &profile_id,
                &outcome,
                &execution
                    .observations
                    .observations()
                    .iter()
                    .map(|o| (o.channel, o.text.as_str(), o.prefix.as_str()))
                    .collect::<Vec<_>>(),
                execution.observations.bytes(),
                execution.observations.reserved_bytes(),
                &diagnostics_json
            )
        );
        return match execution.outcome {
            nexo::interp::Outcome::Err(_) | nexo::interp::Outcome::SimulatorFault => {
                ExitCode::FAILURE
            }
            nexo::interp::Outcome::InternalError(_) => ExitCode::from(2),
            _ => ExitCode::SUCCESS,
        };
    }
    for observation in execution.observations.observations() {
        match observation.channel {
            nexo::interp::observation::Channel::Stdout => println!("{}", observation.text),
            nexo::interp::observation::Channel::Event => {
                eprintln!("{}{}", observation.prefix, observation.text)
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
        nexo::interp::Outcome::Ok => {
            eprintln!("nexo-result:Ok(())");
            ExitCode::SUCCESS
        }
        nexo::interp::Outcome::Err(error) => {
            eprintln!("nexo-result:Err({error})");
            ExitCode::FAILURE
        }
        nexo::interp::Outcome::SimulatorFault => ExitCode::FAILURE,
        nexo::interp::Outcome::InternalError(_) => {
            eprintln!("nexo-driver:internal_error");
            ExitCode::from(2)
        }
    }
}

fn parse_invocation(
    args: &[std::ffi::OsString],
) -> Option<(&str, &str, Option<&str>, String, &str)> {
    let command = args.first()?.to_str()?;
    if command != "check" && command != "run" {
        return None;
    }
    let mut file = None;
    let mut mock = None;
    let mut profile = diagnostic::PROFILE_ID.to_owned();
    let mut format = "text";
    let mut seen_profile = false;
    let mut seen_format = false;
    let mut seen_mock = false;
    let mut i = 1;
    while i < args.len() {
        let arg = args[i].to_str()?;
        if arg.starts_with("--") {
            if arg.contains('=') {
                return None;
            }
            match arg {
                "--profile"
                    if !seen_profile
                        && i + 1 < args.len()
                        && !args[i + 1].to_string_lossy().starts_with("--") =>
                {
                    seen_profile = true;
                    i += 1;
                    profile = args[i].to_str()?.to_owned();
                }
                "--format"
                    if !seen_format
                        && i + 1 < args.len()
                        && !args[i + 1].to_string_lossy().starts_with("--") =>
                {
                    seen_format = true;
                    i += 1;
                    format = args[i].to_str()?;
                    if format != "text" && format != "json" {
                        return None;
                    }
                }
                "--mock-env"
                    if command == "run"
                        && !seen_mock
                        && i + 1 < args.len()
                        && !args[i + 1].to_string_lossy().starts_with("--") =>
                {
                    seen_mock = true;
                    i += 1;
                    mock = Some(args[i].to_str()?);
                }
                _ => return None,
            }
        } else if file.replace(arg).is_some() {
            return None;
        }
        i += 1;
    }
    Some((command, file?, mock, profile, format))
}

fn driver_error(profile: Option<&str>, reason: &str, json: bool) -> ExitCode {
    if json {
        println!(
            "{}",
            execution_json(
                profile.unwrap_or(diagnostic::PROFILE_ID),
                &format!(
                    r#"{{"kind":"driver_error","reason":{}}}"#,
                    json_quote(reason)
                ),
                &[],
                0,
                0,
                &[]
            )
        );
    } else {
        eprintln!("nexo-driver:{reason}");
    }
    ExitCode::from(2)
}

fn diagnostic_result(
    file: &str,
    source: &str,
    bytes_len: usize,
    profile: &str,
    kind: &str,
    errors: &[Box<Diagnostic>],
    status: u8,
) -> ExitCode {
    diagnostic_result_view(
        file,
        SourceView {
            bytes_len,
            text: Some(source),
        },
        profile,
        kind,
        errors,
        status,
    )
}

fn diagnostic_result_view(
    file: &str,
    source: SourceView<'_>,
    profile: &str,
    kind: &str,
    errors: &[Box<Diagnostic>],
    status: u8,
) -> ExitCode {
    let rendered: Vec<String> = errors
        .iter()
        .map(|d| {
            d.to_json_view(file, source)
                .unwrap_or_else(|_| "null".into())
        })
        .collect();
    let outcome = format!(r#"{{"kind":"{kind}"}}"#);
    println!(
        "{}",
        execution_json(profile, &outcome, &[], 0, 0, &rendered)
    );
    ExitCode::from(status)
}

fn execution_json(
    profile: &str,
    outcome: &str,
    observations: &[(nexo::interp::observation::Channel, &str, &str)],
    bytes: u64,
    reserved: u64,
    diagnostics: &[String],
) -> String {
    let stdout = observations
        .iter()
        .filter(|(c, _, _)| *c == nexo::interp::observation::Channel::Stdout)
        .map(|(_, t, _)| json_quote(t))
        .collect::<Vec<_>>()
        .join(",");
    let events = observations
        .iter()
        .filter(|(c, _, _)| *c == nexo::interp::observation::Channel::Event)
        .map(|(_, t, _)| json_quote(t))
        .collect::<Vec<_>>()
        .join(",");
    let obs = observations
        .iter()
        .map(|(c, t, _)| {
            format!(
                r#"{{"channel":"{}","text":{}}}"#,
                if *c == nexo::interp::observation::Channel::Stdout {
                    "stdout"
                } else {
                    "event"
                },
                json_quote(t)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"schema_version":1,"profile_id":{},"outcome":{},"stdout":[{}],"events":[{}],"observations":[{}],"observation_bytes":{},"reserved_observation_bytes":{},"diagnostics":[{}]}}"#,
        json_quote(profile),
        outcome,
        stdout,
        events,
        obs,
        bytes,
        reserved,
        diagnostics.join(",")
    )
}
fn json_quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn internal_error_exit(_error: &nexo::ir::CompileError) -> (u8, &'static str) {
    (2, "nexo-driver:internal_error")
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    #[test]
    fn checker_worker_failure_maps_to_controlled_exit_two_message() {
        let error = nexo::ir::CompileError::InvalidIr("internal_error");
        let (status, message) = super::internal_error_exit(&error);
        assert_eq!(status, 2);
        assert_eq!(message, "nexo-driver:internal_error");
        assert!(!message.contains("panicked"));
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

fn input_failure(
    file: &str,
    error: InputError,
    bytes_len: usize,
    profile: &str,
    format: &str,
) -> ExitCode {
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
    let mut diagnostic = diagnostic;
    diagnostic.profile_id = profile.to_owned();
    if format == "json" {
        return diagnostic_result_view(
            file,
            view,
            profile,
            "input_error",
            &[Box::new(diagnostic)],
            status,
        );
    }
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
    let mut format_seen = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--subcause" if subcause.is_none() && i + 1 < args.len() => {
                subcause = Some(args[i + 1].as_str());
                i += 2;
            }
            "--format"
                if !format_seen
                    && i + 1 < args.len()
                    && matches!(args[i + 1].as_str(), "text" | "json") =>
            {
                format_seen = true;
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
