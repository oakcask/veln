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
            push_byte_preview_note(&mut diagnostic, byte_entries);
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
            push_byte_preview_note(&mut diagnostic, byte_entries);
            diagnostic
        }
        "schema.length_out_of_bounds" => {
            let expected_count = json_number(byte_entries, "expected_count")?;
            let available_count = json_number(byte_entries, "available_count")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("payload length out of bounds at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Payload length expected {expected_count} byte(s); {available_count} byte(s) were available."
            )));
            push_byte_preview_note(&mut diagnostic, byte_entries);
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
            push_byte_preview_note(&mut diagnostic, byte_entries);
            diagnostic
        }
        "schema.validation_failed" => {
            let predicate = json_string(byte_entries, "predicate")?;
            let field_value = json_number(byte_entries, "field_value")?;
            let decoded_values = json_string(byte_entries, "decoded_values").or_else(|| {
                let length = json_number(byte_entries, "length")?;
                let padding_length = json_number(byte_entries, "padding_length")?;
                Some(format!("length={length}, padding_length={padding_length}"))
            })?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("schema validation failed at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Predicate `{predicate}` failed for field value {field_value}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Decoded values: {decoded_values}.")));
            push_byte_preview_note(&mut diagnostic, byte_entries);
            diagnostic
        }
        "schema.dispatch_unknown_tag" => {
            let tag_field = json_string(byte_entries, "tag_field")?;
            let decoded_tag_value = json_number(byte_entries, "decoded_tag_value")?;
            let expected_tags = json_string(byte_entries, "expected_tags")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("unknown dispatch tag at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Dispatch tag field `{tag_field}` decoded value {decoded_tag_value}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Expected tag values: {expected_tags}.")));
            push_byte_preview_note(&mut diagnostic, byte_entries);
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
        "http2.protocol.partial_preface" => {
            let pending_count = json_number(protocol_entries, "pending_count")?;
            let expected_count = json_number(protocol_entries, "expected_count")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!(
                    "input ended with partial client connection preface at byte offset {byte_offset}"
                ),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Input end arrived after {pending_count} of {expected_count} preface byte(s)."
            )));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {provenance}.")));
            Some(diagnostic)
        }
        "http2.protocol.invalid_preface" => {
            let expected_byte = json_number(protocol_entries, "expected_byte")?;
            let actual_byte = json_number(protocol_entries, "actual_byte")?;
            let matched_count = json_number(protocol_entries, "matched_prefix_count")?;
            let expected_count = json_number(protocol_entries, "expected_count")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("invalid client connection preface at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Observed byte {actual_byte}; expected byte {expected_byte} after {matched_count} of {expected_count} preface byte(s)."
            )));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {provenance}.")));
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
        "http2.protocol.invalid_frame_kind" => {
            let actual_kind = json_number(protocol_entries, "actual_frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let expected_kind = json_number(protocol_entries, "expected_frame_kind")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("invalid frame kind at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {actual_kind} on {stream_ref} {stream_id} did not match expected frame kind {expected_kind}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {provenance}.")));
            Some(diagnostic)
        }
        "http2.protocol.invalid_stream_id" => {
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let required_domain = json_string(protocol_entries, "required_stream_id_domain")?;
            let endpoint_role = json_string(protocol_entries, "endpoint_role")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("invalid stream id at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} requires {required_domain} for {endpoint_role}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {provenance}.")));
            Some(diagnostic)
        }
        "http2.protocol.invalid_payload_length" => {
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let observed_length = json_number(protocol_entries, "observed_payload_length")?;
            let expected_length = json_number(protocol_entries, "expected_payload_length")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("invalid payload length at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} declared {observed_length} byte(s); expected {expected_length} byte(s)."
            )));
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {provenance}.")));
            Some(diagnostic)
        }
        "http2.peer_limit.frame_size_exceeded" => {
            let observed_length = json_number(protocol_entries, "observed_payload_length")?;
            let allowed_length = json_number(protocol_entries, "allowed_max_frame_size")?;
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let provenance = json_string(protocol_entries, "receive_limit_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!(
                    "frame payload length exceeds receive maximum at byte offset {byte_offset}"
                ),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} declared {observed_length} byte(s); active receive maximum is {allowed_length} byte(s)."
            )));
            diagnostic.related.push(note_json(format!(
                "Receive limit provenance: {provenance}."
            )));
            Some(diagnostic)
        }
        "http2.peer_limit.flow_control_window_exceeded" => {
            let observed_length = json_number(protocol_entries, "observed_payload_length")?;
            let allowed_credit = json_number(protocol_entries, "allowed_window_credit")?;
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("flow-control window exceeded at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} declared {observed_length} byte(s); available receive window credit is {allowed_credit} byte(s)."
            )));
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {provenance}.")));
            Some(diagnostic)
        }
        "http2.peer_limit.concurrent_streams_exceeded" => {
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let attempted_count =
                json_number(protocol_entries, "attempted_concurrent_stream_count")?;
            let allowed_count = json_number(protocol_entries, "allowed_concurrent_stream_count")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let limit_provenance = json_string(protocol_entries, "receive_limit_provenance")?;
            let rule_provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("concurrent stream receive limit exceeded at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Opening {stream_ref} {stream_id} would make {attempted_count} concurrent peer-created stream(s); active receive limit is {allowed_count}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic.related.push(note_json(format!(
                "Receive limit provenance: {limit_provenance}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {rule_provenance}.")));
            Some(diagnostic)
        }
        "http2.peer_limit.settings_value_out_of_range" => {
            let setting_identifier = json_number(protocol_entries, "setting_identifier")?;
            let setting_name = json_string(protocol_entries, "setting_name")?;
            let observed_value = json_number(protocol_entries, "observed_value")?;
            let accepted_min_value = json_number(protocol_entries, "accepted_min_value")?;
            let accepted_max_value = json_number(protocol_entries, "accepted_max_value")?;
            let provenance = json_string(protocol_entries, "peer_limit_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("SETTINGS value outside accepted range at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "{setting_name} ({setting_identifier}) declared {observed_value}; accepted range is {accepted_min_value}..{accepted_max_value}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Peer limit provenance: {provenance}.")));
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

fn push_byte_preview_note(diagnostic: &mut Diagnostic, entries: &[(String, JsonValue)]) {
    let context = byte_preview_note(entries).or_else(|| json_string(entries, "nearby_context"));
    if let Some(context) = context
        && !context.is_empty()
    {
        diagnostic
            .related
            .push(note_json(format!("Nearby bytes: {context}.")));
    }
}

fn byte_preview_note(entries: &[(String, JsonValue)]) -> Option<String> {
    let preview = json_field(entries, "byte_preview")?;
    let preview_entries = json_object(preview)?;
    let encoding = json_string(preview_entries, "encoding")?;
    if encoding != "hex" {
        return None;
    }
    let data = json_string(preview_entries, "data")?;
    let preview_byte_count = json_number(preview_entries, "preview_byte_count")?;
    let total_byte_count = json_number(preview_entries, "total_byte_count")?;
    let truncated = json_bool(preview_entries, "truncated")?;
    let state = if truncated { "truncated" } else { "complete" };
    let pairs = spaced_hex_pairs(&data)?;
    let preview_text = if pairs.is_empty() {
        "<empty>"
    } else {
        pairs.as_str()
    };
    Some(format!(
        "{preview_text} (showing {preview_byte_count} of {total_byte_count} byte(s), {state})"
    ))
}

fn spaced_hex_pairs(data: &str) -> Option<String> {
    if !data.len().is_multiple_of(2) {
        return None;
    }
    let mut parts = Vec::with_capacity(data.len() / 2);
    for index in (0..data.len()).step_by(2) {
        let pair = data.get(index..index + 2)?;
        if !pair
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
        {
            return None;
        }
        parts.push(pair);
    }
    Some(parts.join(" "))
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

fn json_bool(entries: &[(String, JsonValue)], key: &str) -> Option<bool> {
    match json_field(entries, key)? {
        JsonValue::Bool(value) => Some(*value),
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

    fn byte_preview(data: &str) -> JsonValue {
        byte_preview_with_counts(data, (data.len() / 2) as i64, false)
    }

    fn byte_preview_with_counts(data: &str, total_byte_count: i64, truncated: bool) -> JsonValue {
        let preview_byte_count = (data.len() / 2) as i64;
        JsonValue::object([
            ("encoding", JsonValue::string("hex")),
            ("data", JsonValue::string(data)),
            ("preview_byte_count", JsonValue::Number(preview_byte_count)),
            ("total_byte_count", JsonValue::Number(total_byte_count)),
            ("truncated", JsonValue::Bool(truncated)),
        ])
    }

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
            ("byte_preview", byte_preview("ff0001")),
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
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("ff 00 01 (showing 3 of 3 byte(s), complete)")
        );
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
            ("byte_preview", byte_preview("000005010400")),
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
            ("byte_preview", byte_preview("000005010480000001")),
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
    fn byte_result_failure_diagnostic_projects_payload_length_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.length_out_of_bounds")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(11)),
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
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            ("expected_count", JsonValue::Number(5)),
            ("available_count", JsonValue::Number(2)),
            ("byte_preview", byte_preview("000005010400000001aabb")),
        ]);
        let failure = TestFailure::result_with_details(
            "payload length out of bounds at byte offset 11".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "schema.length_out_of_bounds");
        assert_eq!(
            diagnostic.message,
            "payload length out of bounds at byte offset 11"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("expected 5 byte(s); 2 byte(s) were available")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("schema `Http2FrameHeader` / field `payload`")
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_validation_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.validation_failed")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(3)),
                ]),
            ),
            (
                "field_path",
                JsonValue::array([
                    JsonValue::object([
                        ("kind", JsonValue::string("schema")),
                        ("name", JsonValue::string("SchemaValidationSample")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("padding_length")),
                    ]),
                ]),
            ),
            ("predicate", JsonValue::string("padding_length <= length")),
            ("field_value", JsonValue::Number(6)),
            (
                "decoded_values",
                JsonValue::string("length=5, padding_length=6"),
            ),
            ("length", JsonValue::Number(5)),
            ("padding_length", JsonValue::Number(6)),
            ("byte_preview", byte_preview("00000506")),
        ]);
        let failure = TestFailure::result_with_details(
            "schema validation failed at byte offset 3".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "schema.validation_failed");
        assert_eq!(
            diagnostic.message,
            "schema validation failed at byte offset 3"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("padding_length <= length")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("length=5, padding_length=6")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("00 00 05 06 (showing 4 of 4 byte(s), complete)")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("schema `SchemaValidationSample` / field `padding_length`")
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_truncated_preview_counts() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.length_out_of_bounds")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(11)),
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
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            ("expected_count", JsonValue::Number(5)),
            ("available_count", JsonValue::Number(2)),
            (
                "byte_preview",
                byte_preview_with_counts("0000050104000000", 11, true),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "payload length out of bounds at byte offset 11".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 00 05 01 04 00 00 00 (showing 8 of 11 byte(s), truncated)")
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_keeps_empty_preview_counts() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.truncated_field")),
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
                        ("name", JsonValue::string("EmptyPacket")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("kind")),
                    ]),
                ]),
            ),
            ("expected_count", JsonValue::Number(1)),
            ("available_count", JsonValue::Number(0)),
            ("readiness", JsonValue::string("need_bytes")),
            ("byte_preview", byte_preview_with_counts("", 0, false)),
        ]);
        let failure = TestFailure::result_with_details(
            "truncated schema field at byte offset 0".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("<empty> (showing 0 of 0 byte(s), complete)")
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
    fn protocol_result_failure_diagnostic_projects_partial_preface_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            ("id", JsonValue::string("http2.protocol.partial_preface")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("pending_count", JsonValue::Number(6)),
            ("expected_count", JsonValue::Number(24)),
            ("byte_preview", byte_preview("505249202a20")),
            ("active_state", JsonValue::string("connection-preface")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_client_connection_preface"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 input ended with partial client connection preface at byte offset 0"
                .to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.partial_preface");
        assert_eq!(
            diagnostic.message,
            "input ended with partial client connection preface at byte offset 0"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("6 of 24 preface byte(s)")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("50 52 49 20 2a 20")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("showing 6 of 6 byte(s), complete")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("connection-preface")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_client_connection_preface")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_invalid_preface_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            ("id", JsonValue::string("http2.protocol.invalid_preface")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(4)),
                ]),
            ),
            ("expected_byte", JsonValue::Number(42)),
            ("actual_byte", JsonValue::Number(43)),
            ("matched_prefix_count", JsonValue::Number(4)),
            ("expected_count", JsonValue::Number(24)),
            ("byte_preview", byte_preview("505249202b")),
            ("active_state", JsonValue::string("connection-preface")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_client_connection_preface"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 invalid client connection preface at byte offset 4".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_preface");
        assert_eq!(
            diagnostic.message,
            "invalid client connection preface at byte offset 4"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Observed byte 43; expected byte 42")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("4 of 24 preface byte(s)")
        );
        assert!(diagnostic.related[1].to_json().contains("50 52 49 20 2b"));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("showing 5 of 5 byte(s), complete")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("connection-preface")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_client_connection_preface")
        );
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
    fn protocol_result_failure_diagnostic_projects_frame_size_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.peer_limit.frame_size_exceeded"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("observed_payload_length", JsonValue::Number(16385)),
            ("allowed_max_frame_size", JsonValue::Number(16384)),
            ("frame_kind", JsonValue::Number(0)),
            ("stream_id", JsonValue::Number(3)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "receive_limit_provenance",
                JsonValue::string("protocol_default"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 frame payload length exceeds receive maximum at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.peer_limit.frame_size_exceeded");
        assert_eq!(
            diagnostic.message,
            "frame payload length exceeds receive maximum at byte offset 0"
        );
        assert_eq!(diagnostic.related.len(), 2);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("declared 16385 byte(s)")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("active receive maximum is 16384 byte(s)")
        );
        assert!(diagnostic.related[1].to_json().contains("protocol_default"));
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_flow_control_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.peer_limit.flow_control_window_exceeded"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("observed_payload_length", JsonValue::Number(4)),
            ("allowed_window_credit", JsonValue::Number(3)),
            ("frame_kind", JsonValue::Number(0)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            ("active_state", JsonValue::string("open-stream")),
            (
                "rule_provenance",
                JsonValue::string("stream_receive_window"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 flow-control window exceeded at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "http2.peer_limit.flow_control_window_exceeded"
        );
        assert_eq!(
            diagnostic.message,
            "flow-control window exceeded at byte offset 0"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("declared 4 byte(s)")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("available receive window credit is 3 byte(s)")
        );
        assert!(diagnostic.related[1].to_json().contains("open-stream"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("stream_receive_window")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_concurrent_stream_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.peer_limit.concurrent_streams_exceeded"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("stream_id", JsonValue::Number(3)),
            ("stream_ref", JsonValue::string("stream")),
            ("attempted_concurrent_stream_count", JsonValue::Number(2)),
            ("allowed_concurrent_stream_count", JsonValue::Number(1)),
            ("active_state", JsonValue::string("open-stream")),
            (
                "receive_limit_provenance",
                JsonValue::string("local_configuration"),
            ),
            (
                "rule_provenance",
                JsonValue::string("peer_created_stream_receive_limit"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 concurrent stream receive limit exceeded at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "http2.peer_limit.concurrent_streams_exceeded"
        );
        assert_eq!(
            diagnostic.message,
            "concurrent stream receive limit exceeded at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("make 2 concurrent peer-created stream(s)")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("active receive limit is 1")
        );
        assert!(diagnostic.related[1].to_json().contains("open-stream"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("local_configuration")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("peer_created_stream_receive_limit")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_settings_value_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.peer_limit.settings_value_out_of_range"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("setting_identifier", JsonValue::Number(5)),
            ("setting_name", JsonValue::string("SETTINGS_MAX_FRAME_SIZE")),
            ("observed_value", JsonValue::Number(16383)),
            ("accepted_min_value", JsonValue::Number(16384)),
            ("accepted_max_value", JsonValue::Number(16777215)),
            ("peer_limit_provenance", JsonValue::string("peer_settings")),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 SETTINGS value outside accepted range at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "http2.peer_limit.settings_value_out_of_range"
        );
        assert_eq!(
            diagnostic.message,
            "SETTINGS value outside accepted range at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 2);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("SETTINGS_MAX_FRAME_SIZE (5)")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("accepted range is 16384..16777215")
        );
        assert!(diagnostic.related[1].to_json().contains("peer_settings"));
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_invalid_frame_kind_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            ("id", JsonValue::string("http2.protocol.invalid_frame_kind")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("actual_frame_kind", JsonValue::Number(0)),
            ("stream_id", JsonValue::Number(0)),
            ("stream_ref", JsonValue::string("connection")),
            ("expected_frame_kind", JsonValue::Number(4)),
            ("active_state", JsonValue::string("connection-control")),
            (
                "rule_provenance",
                JsonValue::string("connection_frames_require_settings"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 invalid frame kind at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_frame_kind");
        assert_eq!(diagnostic.message, "invalid frame kind at byte offset 0");
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Frame kind 0 on connection 0")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("expected frame kind 4")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("connection-control")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("connection_frames_require_settings")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_invalid_stream_id_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            ("id", JsonValue::string("http2.protocol.invalid_stream_id")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(2)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "required_stream_id_domain",
                JsonValue::string("nonzero client-initiated stream id"),
            ),
            ("endpoint_role", JsonValue::string("server")),
            ("active_state", JsonValue::string("stream-id-domain")),
            (
                "rule_provenance",
                JsonValue::string("server_receives_client_initiated_streams"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 invalid stream id at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_stream_id");
        assert_eq!(diagnostic.message, "invalid stream id at byte offset 0");
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Frame kind 1 on stream 2")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("nonzero client-initiated stream id")
        );
        assert!(diagnostic.related[1].to_json().contains("stream-id-domain"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("server_receives_client_initiated_streams")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_invalid_payload_length_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_payload_length"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(6)),
            ("stream_id", JsonValue::Number(0)),
            ("stream_ref", JsonValue::string("connection")),
            ("observed_payload_length", JsonValue::Number(7)),
            ("expected_payload_length", JsonValue::Number(8)),
            ("active_state", JsonValue::string("connection-control")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_ping_payload_length"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 invalid payload length at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_payload_length");
        assert_eq!(
            diagnostic.message,
            "invalid payload length at byte offset 0"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Frame kind 6 on connection 0")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("expected 8 byte(s)")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("connection-control")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_ping_payload_length")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_stream_invalid_frame_kind_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            ("id", JsonValue::string("http2.protocol.invalid_frame_kind")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("actual_frame_kind", JsonValue::Number(0)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            ("expected_frame_kind", JsonValue::Number(1)),
            ("active_state", JsonValue::string("idle-stream")),
            (
                "rule_provenance",
                JsonValue::string("idle_streams_require_headers"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 invalid frame kind at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_frame_kind");
        assert_eq!(diagnostic.message, "invalid frame kind at byte offset 0");
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Frame kind 0 on stream 1")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("expected frame kind 1")
        );
        assert!(diagnostic.related[1].to_json().contains("idle-stream"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("idle_streams_require_headers")
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
