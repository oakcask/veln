use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use veln_analysis::{DoctestMode, ProjectAnalysis, analyze_project};
use veln_ast::Function;
use veln_ast::FunctionKind;
use veln_backend_jvm::{EntryArgType, generate_classfiles_with_entry_arg_types};
use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity};
use veln_project::Project;
use veln_test::{TestFailure, contract_failure_from_trace, result_failure_from_trace};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};
use crate::java::{
    JvmRunResult, create_build_dir, exit_code_from_status, forward_process_output,
    prepare_and_run_jvm_capture_with_env,
};

pub(crate) fn run_entry(
    json: bool,
    entry: String,
    inputs: Vec<PathBuf>,
    entry_args: Vec<String>,
) -> Result<ExitCode, String> {
    let analysis = analyze_run_project(&inputs)?;
    if report_source_errors(&analysis)? {
        return Ok(ExitCode::from(1));
    }

    let Some(entry_arg_types) = checked_entry_arg_types(&analysis, &entry, &entry_args)? else {
        return Ok(ExitCode::from(1));
    };
    let Some(ir) = lower_run_entry(&analysis, &entry)? else {
        return Ok(ExitCode::from(1));
    };

    let jvm = generate_classfiles_with_entry_arg_types(&ir, &entry, &entry_arg_types);
    let build_dir = create_build_dir("veln-run").map_err(|error| error.to_string())?;
    let result = if json {
        run_json(&build_dir, &jvm, &entry_args)
    } else {
        run_human(&build_dir, &jvm, &entry_args)
    };
    let cleanup_result = fs::remove_dir_all(&build_dir);
    if let Err(error) = cleanup_result {
        eprintln!(
            "veln: warning: failed to remove build directory `{}`: {error}",
            build_dir.display()
        );
    }
    result
}

fn analyze_run_project(inputs: &[PathBuf]) -> Result<ProjectAnalysis, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, inputs).map_err(|error| error.to_string())?;
    Ok(analyze_project(project, DoctestMode::Exclude))
}

fn report_source_errors(analysis: &ProjectAnalysis) -> Result<bool, String> {
    let diagnostics = analysis.source_diagnostics();
    if has_error(&diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), diagnostics))?;
        return Ok(true);
    }
    Ok(false)
}

fn find_entry_function<'a>(analysis: &'a ProjectAnalysis, entry: &str) -> Option<&'a Function> {
    analysis.module.functions.iter().find(|function| {
        function.kind == FunctionKind::Function && function.name.as_deref() == Some(entry)
    })
}

fn checked_entry_arg_types(
    analysis: &ProjectAnalysis,
    entry: &str,
    entry_args: &[String],
) -> Result<Option<Vec<EntryArgType>>, String> {
    let Some(entry_function) = find_entry_function(analysis, entry) else {
        eprintln!("veln: run entry `{entry}` was not found");
        return Ok(None);
    };
    validate_entry_args(entry_function, entry, entry_args)
}

fn validate_entry_args(
    entry_function: &Function,
    entry: &str,
    entry_args: &[String],
) -> Result<Option<Vec<EntryArgType>>, String> {
    if entry_function.params.len() != entry_args.len() {
        eprintln!(
            "veln: run entry `{entry}` expects {} argument(s), got {}",
            entry_function.params.len(),
            entry_args.len()
        );
        eprintln!("veln: note: pass entry arguments after `--`");
        return Ok(None);
    }
    let mut entry_arg_types = Vec::new();
    for (param, raw_arg) in entry_function.params.iter().zip(entry_args.iter()) {
        let Some(arg_type) = param.ty.as_deref().and_then(entry_arg_type) else {
            eprintln!(
                "veln: run entry parameter `{}` cannot be supplied from a command-line argument",
                param.name
            );
            eprintln!(
                "veln: note: supported entry argument types are String, Int, Float, and Bool"
            );
            return Ok(None);
        };
        if let Err(message) = validate_entry_arg(arg_type, &param.name, raw_arg) {
            eprintln!("{message}");
            return Ok(None);
        }
        entry_arg_types.push(arg_type);
    }
    Ok(Some(entry_arg_types))
}

fn lower_run_entry(
    analysis: &ProjectAnalysis,
    entry: &str,
) -> Result<Option<veln_ir::TypedProgram>, String> {
    let reachable = analysis.lower_reachable_entry(entry, FunctionKind::Function);
    let lowered = reachable.lowered;
    if has_error(&lowered.diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        return Ok(None);
    }
    let Some(ir) = lowered.ir else {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        eprintln!("veln: run blocked: checked program is not executable");
        return Ok(None);
    };
    Ok(Some(ir))
}

fn run_human(
    build_dir: &std::path::Path,
    program: &veln_backend_jvm::JvmProgram,
    entry_args: &[String],
) -> Result<ExitCode, String> {
    let result_error_file = build_dir.join("result-errors.tsv");
    let event_env = [("VELN_RESULT_ERRORS", result_error_file.as_os_str())];
    let result = prepare_and_run_jvm_capture_with_env(
        build_dir, program, "veln run", &event_env, entry_args,
    )?;
    let output = match result {
        JvmRunResult::Ran(output) => output,
        JvmRunResult::ToolError(message) => {
            eprintln!("{message}");
            return Ok(ExitCode::from(1));
        }
    };
    if output.status.success() {
        forward_process_output(&output)?;
        return Ok(exit_code_from_status(output.status));
    }

    let result_error_trace = fs::read_to_string(&result_error_file).unwrap_or_default();
    let result_failure = result_failure_from_trace(&result_error_trace);
    let diagnostic = result_failure
        .as_ref()
        .and_then(runtime_result_failure_diagnostic);
    if let (Some(failure), Some(diagnostic)) = (result_failure.as_ref(), diagnostic) {
        io::stdout()
            .write_all(&output.stdout)
            .map_err(|error| error.to_string())?;
        io::stderr()
            .write_all(stderr_without_result_failure_line(&output.stderr, failure))
            .map_err(|error| error.to_string())?;
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), vec![diagnostic]))?;
    } else {
        forward_process_output(&output)?;
    }
    Ok(exit_code_from_status(output.status))
}

fn runtime_result_failure_diagnostic(failure: &TestFailure) -> Option<Diagnostic> {
    byte_result_failure_diagnostic(failure).or_else(|| protocol_result_failure_diagnostic(failure))
}

fn byte_result_failure_diagnostic(failure: &TestFailure) -> Option<Diagnostic> {
    let details = json_object(&failure.details)?;
    let byte_diagnostic = json_field(details, "byte_diagnostic")?;
    let byte_entries = json_object(byte_diagnostic)?;
    let id = json_string(byte_entries, "id")?;
    let byte_offset = byte_offset_value(byte_entries)?;

    let mut diagnostic = match id.as_str() {
        "codec.incomplete_input" => {
            let expected_count = json_number(byte_entries, "expected_count")?;
            let available_count = json_number(byte_entries, "available_count")?;
            let readiness = json_string(byte_entries, "readiness")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("missing byte at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "pending readiness is `{readiness}` because input is closed."
            )));
            diagnostic.related.push(note_json(format!(
                "Fixed-width read expected {expected_count} byte(s); {available_count} byte(s) were available."
            )));
            diagnostic
        }
        "schema.fixed_field_mismatch" => {
            let expected_value = json_number(byte_entries, "expected_value")?;
            let actual_value = json_number(byte_entries, "actual_value")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("fixed field mismatch at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Fixed field expected value {expected_value}; actual value was {actual_value}."
            )));
            if let Some(context) = json_string(byte_entries, "nearby_context")
                && !context.is_empty()
            {
                diagnostic
                    .related
                    .push(note_json(format!("Nearby bytes: {context}.")));
            }
            diagnostic
        }
        "schema.truncated_field" => {
            let expected_count = json_number(byte_entries, "expected_count")?;
            let available_count = json_number(byte_entries, "available_count")?;
            let readiness = json_string(byte_entries, "readiness")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("truncated schema field at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "pending readiness is `{readiness}` because input is closed."
            )));
            diagnostic.related.push(note_json(format!(
                "Schema field expected {expected_count} byte(s); {available_count} byte(s) were available."
            )));
            if let Some(context) = json_string(byte_entries, "nearby_context")
                && !context.is_empty()
            {
                diagnostic
                    .related
                    .push(note_json(format!("Nearby bytes: {context}.")));
            }
            diagnostic
        }
        "schema.reserved_bits_mismatch" => {
            let bit_width = json_number(byte_entries, "bit_width")?;
            let expected_value = json_number(byte_entries, "expected_value")?;
            let actual_value = json_number(byte_entries, "actual_value")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("reserved bits mismatch at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "ReservedBits({bit_width}, {expected_value}) expected value {expected_value}; actual value was {actual_value}."
            )));
            if let Some(context) = json_string(byte_entries, "nearby_context")
                && !context.is_empty()
            {
                diagnostic
                    .related
                    .push(note_json(format!("Nearby bytes: {context}.")));
            }
            diagnostic
        }
        _ => return None,
    };
    if let Some(field_path) = field_path_text(byte_entries) {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    }
    Some(diagnostic)
}

fn protocol_result_failure_diagnostic(failure: &TestFailure) -> Option<Diagnostic> {
    let details = json_object(&failure.details)?;
    let protocol_diagnostic = json_field(details, "protocol_diagnostic")?;
    let protocol_entries = json_object(protocol_diagnostic)?;
    let id = json_string(protocol_entries, "id")?;
    let byte_offset = byte_offset_value(protocol_entries)?;

    match id.as_str() {
        "http2.protocol.closed_with_pending" => {
            let pending_count = json_number(protocol_entries, "pending_count")?;
            let active_continuation = json_string(protocol_entries, "active_continuation")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("input ended with pending bytes at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Input end arrived while {pending_count} byte(s) remained undecoded."
            )));
            diagnostic.related.push(note_json(format!(
                "Active continuation state: {active_continuation}."
            )));
            Some(diagnostic)
        }
        "http2.protocol.continuation_expected" => {
            let actual_kind = json_number(protocol_entries, "actual_frame_kind")?;
            let actual_stream = json_number(protocol_entries, "actual_stream_id")?;
            let expected_stream = json_number(protocol_entries, "expected_stream_id")?;
            let started_kind = json_number(protocol_entries, "started_frame_kind")?;
            let started_offset = json_number(protocol_entries, "started_byte_offset")?;
            let active_continuation = json_string(protocol_entries, "active_continuation")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("expected CONTINUATION frame at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Incoming frame kind {actual_kind} on stream {actual_stream} violated active continuation state `{active_continuation}`."
            )));
            diagnostic.related.push(note_json(format!(
                "Pending header block started with frame kind {started_kind} at byte offset {started_offset} for stream {expected_stream}."
            )));
            Some(diagnostic)
        }
        _ => None,
    }
}

fn byte_offset_value(entries: &[(String, JsonValue)]) -> Option<i64> {
    let offset = json_field(entries, "byte_offset")?;
    let offset_entries = json_object(offset)?;
    json_number(offset_entries, "value")
}

fn stderr_without_result_failure_line<'a>(stderr: &'a [u8], failure: &TestFailure) -> &'a [u8] {
    let Some(value) = result_failure_value(failure) else {
        return stderr;
    };
    let line = format!("Err({value})\n");
    stderr.strip_suffix(line.as_bytes()).unwrap_or(stderr)
}

fn result_failure_value(failure: &TestFailure) -> Option<String> {
    let details = json_object(&failure.details)?;
    json_string(details, "value")
}

fn field_path_text(entries: &[(String, JsonValue)]) -> Option<String> {
    let JsonValue::Array(segments) = json_field(entries, "field_path")? else {
        return None;
    };
    let mut parts = Vec::new();
    for segment in segments {
        let segment_entries = json_object(segment)?;
        let kind = json_string(segment_entries, "kind")?;
        let name = json_string(segment_entries, "name")?;
        parts.push(format!("{kind} `{name}`"));
    }
    (!parts.is_empty()).then(|| parts.join(" / "))
}

fn note_json(message: String) -> JsonValue {
    JsonValue::object([("message", JsonValue::string(message))])
}

fn json_field<'a>(entries: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    entries
        .iter()
        .find_map(|(entry_key, value)| (entry_key == key).then_some(value))
}

fn json_object(value: &JsonValue) -> Option<&[(String, JsonValue)]> {
    match value {
        JsonValue::Object(entries) => Some(entries),
        _ => None,
    }
}

fn json_string(entries: &[(String, JsonValue)], key: &str) -> Option<String> {
    match json_field(entries, key)? {
        JsonValue::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn json_number(entries: &[(String, JsonValue)], key: &str) -> Option<i64> {
    match json_field(entries, key)? {
        JsonValue::Number(value) => Some(*value),
        _ => None,
    }
}

fn run_json(
    build_dir: &std::path::Path,
    program: &veln_backend_jvm::JvmProgram,
    entry_args: &[String],
) -> Result<ExitCode, String> {
    let contract_error_file = build_dir.join("contract-errors.tsv");
    let result_error_file = build_dir.join("result-errors.tsv");
    let event_env = [
        ("VELN_CONTRACT_ERRORS", contract_error_file.as_os_str()),
        ("VELN_RESULT_ERRORS", result_error_file.as_os_str()),
    ];
    let result = prepare_and_run_jvm_capture_with_env(
        build_dir, program, "veln run", &event_env, entry_args,
    )?;
    let contract_error_trace = fs::read_to_string(&contract_error_file).unwrap_or_default();
    let result_error_trace = fs::read_to_string(&result_error_file).unwrap_or_default();

    let report = match result {
        JvmRunResult::ToolError(message) => RunJsonReport::tool_error(message),
        JvmRunResult::Ran(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let exit_code = output.status.code().unwrap_or(1);
            if output.status.success() {
                RunJsonReport::passed(exit_code, stdout, stderr)
            } else if let Some(failure) = contract_failure_from_trace(&contract_error_trace) {
                RunJsonReport::failed(exit_code, stdout, stderr, failure)
            } else if let Some(failure) = result_failure_from_trace(&result_error_trace) {
                RunJsonReport::failed(exit_code, stdout, stderr, failure)
            } else {
                RunJsonReport::runtime_error(
                    exit_code,
                    stdout,
                    stderr,
                    format!("run process exited with status {}", output.status),
                )
            }
        }
    };
    let exit_code = report.exit_code();
    println!("{}", report.to_json());
    Ok(exit_code)
}

struct RunJsonReport {
    status: &'static str,
    exit_code: i32,
    stdout: String,
    stderr: String,
    error: Option<RunJsonError>,
}

impl RunJsonReport {
    fn passed(exit_code: i32, stdout: String, stderr: String) -> Self {
        Self {
            status: "passed",
            exit_code,
            stdout,
            stderr,
            error: None,
        }
    }

    fn failed(exit_code: i32, stdout: String, stderr: String, failure: TestFailure) -> Self {
        Self {
            status: "failed",
            exit_code,
            stdout,
            stderr,
            error: Some(RunJsonError::from_test_failure(failure)),
        }
    }

    fn runtime_error(exit_code: i32, stdout: String, stderr: String, message: String) -> Self {
        Self {
            status: "failed",
            exit_code,
            stdout,
            stderr,
            error: Some(RunJsonError::runtime(message)),
        }
    }

    fn tool_error(message: String) -> Self {
        Self {
            status: "error",
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(RunJsonError::runner(message)),
        }
    }

    fn exit_code(&self) -> ExitCode {
        if self.status == "passed" {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        }
    }

    fn to_json(&self) -> String {
        JsonValue::object([
            ("schema_version", JsonValue::string("veln-run-json/v0")),
            ("command", JsonValue::string("run")),
            ("status", JsonValue::string(self.status)),
            ("exit_code", JsonValue::Number(self.exit_code.into())),
            ("stdout", JsonValue::string(self.stdout.clone())),
            ("stderr", JsonValue::string(self.stderr.clone())),
            (
                "error",
                self.error
                    .as_ref()
                    .map_or(JsonValue::Null, RunJsonError::to_json),
            ),
        ])
        .to_json()
    }
}

struct RunJsonError {
    kind: String,
    message: String,
    details: JsonValue,
}

impl RunJsonError {
    fn from_test_failure(failure: TestFailure) -> Self {
        Self {
            kind: failure.kind,
            message: failure.message,
            details: failure.details,
        }
    }

    fn runtime(message: String) -> Self {
        Self {
            kind: "runtime".to_string(),
            message,
            details: JsonValue::object([("phase", JsonValue::string("runtime"))]),
        }
    }

    fn runner(message: String) -> Self {
        Self {
            kind: "runner".to_string(),
            message,
            details: JsonValue::object([("phase", JsonValue::string("tool"))]),
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string(self.kind.clone())),
            ("message", JsonValue::string(self.message.clone())),
            ("details", self.details.clone()),
        ])
    }
}

fn entry_arg_type(ty: &str) -> Option<EntryArgType> {
    match ty {
        "String" => Some(EntryArgType::String),
        "Int" => Some(EntryArgType::Int),
        "Float" => Some(EntryArgType::Float),
        "Bool" => Some(EntryArgType::Bool),
        _ => None,
    }
}

fn validate_entry_arg(ty: EntryArgType, param_name: &str, raw_arg: &str) -> Result<(), String> {
    match ty {
        EntryArgType::String => Ok(()),
        EntryArgType::Int => raw_arg.parse::<i64>().map(|_| ()).map_err(|_| {
            format!("veln: invalid Int argument for parameter `{param_name}`: `{raw_arg}`")
        }),
        EntryArgType::Float => raw_arg.parse::<f64>().map(|_| ()).map_err(|_| {
            format!("veln: invalid Float argument for parameter `{param_name}`: `{raw_arg}`")
        }),
        EntryArgType::Bool if raw_arg == "true" || raw_arg == "false" => Ok(()),
        EntryArgType::Bool => Err(format!(
            "veln: invalid Bool argument for parameter `{param_name}`: `{raw_arg}`"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_result_failure_diagnostic_projects_field_path_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.incomplete_input")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(7)),
                ]),
            ),
            (
                "field_path",
                JsonValue::array([
                    JsonValue::object([
                        ("kind", JsonValue::string("schema")),
                        ("name", JsonValue::string("DemoPacket")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            ("expected_count", JsonValue::Number(4)),
            ("available_count", JsonValue::Number(1)),
            ("readiness", JsonValue::string("need_bytes")),
        ]);
        let failure = TestFailure::result_with_details(
            "short input".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.incomplete_input");
        assert_eq!(diagnostic.kind, DiagnosticKind::Runtime);
        assert_eq!(diagnostic.message, "missing byte at byte offset 7");
        assert_eq!(diagnostic.related.len(), 3);
        assert!(diagnostic.related[0].to_json().contains("need_bytes"));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("expected 4 byte(s); 1 byte(s) were available")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("schema `DemoPacket` / field `payload`")
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_fixed_field_mismatch_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.fixed_field_mismatch")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            (
                "field_path",
                JsonValue::array([
                    JsonValue::object([
                        ("kind", JsonValue::string("schema")),
                        ("name", JsonValue::string("DemoPacket")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("kind")),
                    ]),
                ]),
            ),
            ("expected_value", JsonValue::Number(1)),
            ("actual_value", JsonValue::Number(255)),
            ("nearby_context", JsonValue::string("ff0001")),
        ]);
        let failure = TestFailure::result_with_details(
            "fixed field mismatch at byte offset 0".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "schema.fixed_field_mismatch");
        assert_eq!(diagnostic.message, "fixed field mismatch at byte offset 0");
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("expected value 1; actual value was 255")
        );
        assert!(diagnostic.related[1].to_json().contains("ff0001"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("schema `DemoPacket` / field `kind`")
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_truncated_schema_field_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.truncated_field")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(6)),
                ]),
            ),
            (
                "field_path",
                JsonValue::array([
                    JsonValue::object([
                        ("kind", JsonValue::string("schema")),
                        ("name", JsonValue::string("Http2FrameHeader")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("stream_id")),
                    ]),
                ]),
            ),
            ("expected_count", JsonValue::Number(4)),
            ("available_count", JsonValue::Number(1)),
            ("readiness", JsonValue::string("need_bytes")),
            ("nearby_context", JsonValue::string("000005010400")),
        ]);
        let failure = TestFailure::result_with_details(
            "truncated schema field `stream_id` at byte offset 6".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "schema.truncated_field");
        assert_eq!(
            diagnostic.message,
            "truncated schema field at byte offset 6"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(diagnostic.related[0].to_json().contains("need_bytes"));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("expected 4 byte(s); 1 byte(s) were available")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("schema `Http2FrameHeader` / field `stream_id`")
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_reserved_bits_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.reserved_bits_mismatch")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(5)),
                ]),
            ),
            (
                "field_path",
                JsonValue::array([
                    JsonValue::object([
                        ("kind", JsonValue::string("schema")),
                        ("name", JsonValue::string("Http2FrameHeader")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("stream_reserved")),
                    ]),
                ]),
            ),
            ("bit_width", JsonValue::Number(1)),
            ("expected_value", JsonValue::Number(0)),
            ("actual_value", JsonValue::Number(1)),
            ("nearby_context", JsonValue::string("000005010480000001")),
        ]);
        let failure = TestFailure::result_with_details(
            "reserved bits mismatch at byte offset 5".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "schema.reserved_bits_mismatch");
        assert_eq!(
            diagnostic.message,
            "reserved bits mismatch at byte offset 5"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("ReservedBits(1, 0) expected value 0; actual value was 1")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("schema `Http2FrameHeader` / field `stream_reserved`")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_closed_input_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.closed_with_pending"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("pending_count", JsonValue::Number(4)),
            ("input_event", JsonValue::string("end")),
            ("active_continuation", JsonValue::string("none")),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 input ended with 4 pending byte(s) at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.closed_with_pending");
        assert_eq!(
            diagnostic.message,
            "input ended with pending bytes at byte offset 0"
        );
        assert_eq!(diagnostic.related.len(), 2);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("4 byte(s) remained undecoded")
        );
        assert!(diagnostic.related[1].to_json().contains("none"));
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_continuation_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.continuation_expected"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("actual_frame_kind", JsonValue::Number(0)),
            ("actual_stream_id", JsonValue::Number(1)),
            ("expected_stream_id", JsonValue::Number(1)),
            ("started_frame_kind", JsonValue::Number(1)),
            ("started_byte_offset", JsonValue::Number(0)),
            ("active_continuation", JsonValue::string("headers")),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 expected CONTINUATION frame at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.continuation_expected");
        assert_eq!(
            diagnostic.message,
            "expected CONTINUATION frame at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 2);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("frame kind 0 on stream 1")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("frame kind 1 at byte offset 0")
        );
    }

    #[test]
    fn stderr_without_result_failure_line_keeps_user_stderr() {
        let failure = TestFailure::result_with_details("short input".to_string(), None, None, None);
        let stderr = b"user warning\nErr(short input)\n";

        assert_eq!(
            stderr_without_result_failure_line(stderr, &failure),
            b"user warning\n"
        );
    }
}
