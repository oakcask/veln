use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use veln_analysis::{DoctestMode, ProjectAnalysis, analyze_project};
use veln_ast::Function;
use veln_ast::FunctionKind;
use veln_backend_jvm::{EntryArgScalar, EntryArgType, generate_classfiles_with_entry_arg_types};
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
    let fixed_param_count = entry_function
        .params
        .iter()
        .filter(|param| !param.is_variadic)
        .count();
    let variadic_param = entry_function.params.iter().find(|param| param.is_variadic);
    let wrong_count = if variadic_param.is_some() {
        entry_args.len() < fixed_param_count
    } else {
        entry_args.len() != fixed_param_count
    };
    if wrong_count {
        let expects = if variadic_param.is_some() {
            format!("at least {fixed_param_count}")
        } else {
            fixed_param_count.to_string()
        };
        eprintln!(
            "veln: run entry `{entry}` expects {expects} argument(s), got {}",
            entry_args.len()
        );
        eprintln!("veln: note: pass entry arguments after `--`");
        return Ok(None);
    }
    let mut entry_arg_types = Vec::new();
    for (param, raw_arg) in entry_function
        .params
        .iter()
        .filter(|param| !param.is_variadic)
        .zip(entry_args.iter())
    {
        let Some(arg_type) = param.ty.as_deref().and_then(entry_arg_scalar) else {
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
        entry_arg_types.push(entry_arg_type_from_scalar(arg_type));
    }
    if let Some(param) = variadic_param {
        let Some(element_type) = param.ty.as_deref().and_then(entry_arg_scalar) else {
            eprintln!(
                "veln: run entry parameter `{}` cannot be supplied from command-line arguments",
                param.name
            );
            eprintln!(
                "veln: note: supported entry argument types are String, Int, Float, and Bool"
            );
            return Ok(None);
        };
        let tail = &entry_args[fixed_param_count..];
        for raw_arg in tail {
            if let Err(message) = validate_entry_arg(element_type, &param.name, raw_arg) {
                eprintln!("{message}");
                return Ok(None);
            }
        }
        entry_arg_types.push(EntryArgType::VariadicList {
            element: element_type,
            count: tail.len(),
        });
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
    byte_result_failure_diagnostic(failure)
        .or_else(|| value_result_failure_diagnostic(failure))
        .or_else(|| protocol_result_failure_diagnostic(failure))
}

fn byte_result_failure_diagnostic(failure: &TestFailure) -> Option<Diagnostic> {
    let details = json_object(&failure.details)?;
    let byte_diagnostic = json_field(details, "byte_diagnostic")?;
    let byte_entries = json_object(byte_diagnostic)?;
    let id = json_string(byte_entries, "id")?;
    let byte_offset = byte_offset_value(byte_entries)?;

    if is_decode_error_result_failure(failure) {
        return Some(decode_error_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        ));
    }
    if id == "codec.incomplete_input" && is_decode_need_more_result_failure(failure) {
        return Some(decode_need_more_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        ));
    }

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
        "codec.byte_range_out_of_bounds" => {
            let requested_count = json_number(byte_entries, "requested_count")?;
            let available_count = json_number(byte_entries, "available_count")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("byte range out of bounds at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Byte range requested {requested_count} byte(s); {available_count} byte(s) were available from the offset."
            )));
            push_byte_preview_note(&mut diagnostic, byte_entries);
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
        "schema.integer_out_of_range" => {
            let byte_width = json_number(byte_entries, "byte_width")?;
            let min_value = json_number(byte_entries, "min_value")?;
            let max_value = json_number(byte_entries, "max_value")?;
            let actual_value = json_number(byte_entries, "actual_value")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("schema integer out of range at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "{byte_width}-byte schema integer expected value between {min_value} and {max_value}; actual value was {actual_value}."
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
            if let Some(field_value) = json_number(byte_entries, "field_value") {
                diagnostic.related.push(note_json(format!(
                    "Predicate `{predicate}` failed for field value {field_value}."
                )));
            } else {
                diagnostic
                    .related
                    .push(note_json(format!("Schema predicate `{predicate}` failed.")));
            }
            diagnostic
                .related
                .push(note_json(format!("Decoded values: {decoded_values}.")));
            push_byte_preview_note(&mut diagnostic, byte_entries);
            diagnostic
        }
        "schema.mapping_division_by_zero" => {
            let target_field = json_string(byte_entries, "target_field")?;
            let operator = json_string(byte_entries, "operator")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("schema mapping division by zero at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Mapping target field `{target_field}` evaluated `{operator}` with divisor 0."
            )));
            push_byte_preview_note(&mut diagnostic, byte_entries);
            diagnostic
        }
        "schema.length_division_by_zero" => {
            let length_expression = json_string(byte_entries, "length_expression")?;
            let divisor_operand = json_string(byte_entries, "divisor_operand")?;
            let operator = json_string(byte_entries, "operator")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("schema length division by zero at byte offset {byte_offset}"),
                None,
                byte_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Length expression `{length_expression}` evaluated `{operator}` with divisor operand `{divisor_operand}` equal to 0."
            )));
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
    } else if let Some(field_path) = json_string(byte_entries, "field_path_display")
        && !field_path.is_empty()
    {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    }
    Some(diagnostic)
}

fn is_decode_error_result_failure(failure: &TestFailure) -> bool {
    result_failure_value(failure)
        .as_deref()
        .is_some_and(|value| value.starts_with("DecodeError("))
}

fn is_decode_need_more_result_failure(failure: &TestFailure) -> bool {
    result_failure_value(failure)
        .as_deref()
        .is_some_and(|value| value.starts_with("NeedMore("))
}

fn decode_error_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: String,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Runtime,
        format!("decode error at byte offset {byte_offset}"),
        None,
        byte_diagnostic.clone(),
    );
    if let Some(field_path) = field_path_text(byte_entries) {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    } else if let Some(field_path) = json_string(byte_entries, "field_path_display")
        && !field_path.is_empty()
    {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    }
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn decode_need_more_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: String,
    byte_offset: i64,
) -> Diagnostic {
    let readiness = json_string(byte_entries, "readiness").unwrap_or_else(|| "unknown".to_string());
    let mut diagnostic = Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Runtime,
        format!("incomplete input at byte offset {byte_offset}"),
        None,
        byte_diagnostic.clone(),
    );
    diagnostic.related.push(note_json(format!(
        "Decode readiness is `{readiness}` because input is closed."
    )));
    if let Some(needed_count) = json_number(byte_entries, "needed_count") {
        diagnostic.related.push(note_json(format!(
            "Decoder needs at least {needed_count} buffered byte(s) before retrying."
        )));
    }
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeStep value: {value}.")));
    }
    diagnostic
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
            push_byte_preview_note(&mut diagnostic, protocol_entries);
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
            push_byte_preview_note(&mut diagnostic, protocol_entries);
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
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {provenance}.")));
            Some(diagnostic)
        }
        "http2.protocol.invalid_window_update_increment" => {
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let observed_increment = json_number(protocol_entries, "observed_window_increment")?;
            let accepted_min = json_number(protocol_entries, "accepted_min_window_increment")?;
            let accepted_max = json_number(protocol_entries, "accepted_max_window_increment")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("invalid WINDOW_UPDATE increment at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} declared WINDOW_UPDATE increment {observed_increment}; accepted range is {accepted_min}..{accepted_max}."
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
        "http2.protocol.invalid_data_padding" => {
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let pad_length = json_number(protocol_entries, "pad_length")?;
            let remaining_payload_length =
                json_number(protocol_entries, "remaining_payload_length")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("invalid DATA padding at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} declared pad length {pad_length} byte(s); remaining payload length is {remaining_payload_length} byte(s)."
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
        "http2.protocol.unexpected_settings_ack" => {
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("unexpected SETTINGS ACK at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} acknowledged local SETTINGS, but no local SETTINGS batch is outstanding."
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
        "http2.protocol.invalid_request_header_list" => {
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let failed_fact = json_string(protocol_entries, "failed_header_fact")?;
            let header_name = json_string(protocol_entries, "header_name")?;
            let decoded_header_names = json_string(protocol_entries, "decoded_header_names")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let message = match failed_fact.as_str() {
                "missing_required_pseudo_header" => {
                    format!(
                        "request header list is missing {header_name} at byte offset {byte_offset}"
                    )
                }
                "response_only_pseudo_header" => {
                    format!(
                        "request header list contains response-only {header_name} at byte offset {byte_offset}"
                    )
                }
                "duplicate_pseudo_header" => {
                    format!(
                        "request header list contains duplicate {header_name} at byte offset {byte_offset}"
                    )
                }
                "pseudo_header_after_regular_header" => {
                    format!(
                        "request header list places {header_name} after a regular header at byte offset {byte_offset}"
                    )
                }
                "ordinary_header_name_not_lowercase" => {
                    format!(
                        "request header list contains uppercase ordinary header {header_name} at byte offset {byte_offset}"
                    )
                }
                "ordinary_header_name_invalid_token" => {
                    format!(
                        "request header list contains invalid ordinary header name {header_name} at byte offset {byte_offset}"
                    )
                }
                "connection_specific_header" => {
                    format!(
                        "request header list contains connection-specific header {header_name} at byte offset {byte_offset}"
                    )
                }
                "te_header_value_not_trailers" => {
                    format!(
                        "request header list contains te value other than trailers at byte offset {byte_offset}"
                    )
                }
                "scheme_value_not_http_or_https" => {
                    format!(
                        "request header list contains :scheme value other than http or https at byte offset {byte_offset}"
                    )
                }
                "content_length_invalid" => {
                    format!(
                        "request header list contains invalid content-length at byte offset {byte_offset}"
                    )
                }
                "content_length_mismatch" => {
                    format!(
                        "request header list contains mismatched content-length values at byte offset {byte_offset}"
                    )
                }
                _ => format!("invalid request header list at byte offset {byte_offset}"),
            };
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                message,
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} decoded request header names: {decoded_header_names}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {provenance}.")));
            Some(diagnostic)
        }
        "http2.protocol.invalid_response_header_list" => {
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let failed_fact = json_string(protocol_entries, "failed_header_fact")?;
            let header_name = json_string(protocol_entries, "header_name")?;
            let decoded_header_names = json_string(protocol_entries, "decoded_header_names")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let message = match failed_fact.as_str() {
                "missing_required_pseudo_header" => {
                    format!(
                        "response header list is missing {header_name} at byte offset {byte_offset}"
                    )
                }
                "request_only_pseudo_header" => {
                    format!(
                        "response header list contains request-only {header_name} at byte offset {byte_offset}"
                    )
                }
                "duplicate_pseudo_header" => {
                    format!(
                        "response header list contains duplicate {header_name} at byte offset {byte_offset}"
                    )
                }
                "pseudo_header_after_regular_header" => {
                    format!(
                        "response header list places {header_name} after a regular header at byte offset {byte_offset}"
                    )
                }
                "ordinary_header_name_not_lowercase" => {
                    format!(
                        "response header list contains uppercase ordinary header {header_name} at byte offset {byte_offset}"
                    )
                }
                "ordinary_header_name_invalid_token" => {
                    format!(
                        "response header list contains invalid ordinary header name {header_name} at byte offset {byte_offset}"
                    )
                }
                "te_header_value_not_trailers" => {
                    format!(
                        "response header list contains te value other than trailers at byte offset {byte_offset}"
                    )
                }
                "content_length_invalid" => {
                    format!(
                        "response header list contains invalid content-length at byte offset {byte_offset}"
                    )
                }
                "content_length_mismatch" => {
                    format!(
                        "response header list contains mismatched content-length values at byte offset {byte_offset}"
                    )
                }
                _ => format!("invalid response header list at byte offset {byte_offset}"),
            };
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                message,
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} decoded response header names: {decoded_header_names}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Active protocol state: {active_state}.")));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {provenance}.")));
            Some(diagnostic)
        }
        "http2.protocol.invalid_priority_dependency" => {
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let dependency_stream_id = json_number(protocol_entries, "dependency_stream_id")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("invalid PRIORITY dependency at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} declared itself as dependency stream {dependency_stream_id}."
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
        "http2.protocol.stream_after_goaway" => {
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let last_stream_id = json_number(protocol_entries, "last_stream_id")?;
            let shutdown_state = json_string(protocol_entries, "shutdown_state")?;
            let endpoint_role = json_string(protocol_entries, "endpoint_role")?;
            let active_state = json_string(protocol_entries, "active_state")?;
            let provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("stream opened after graceful shutdown at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Peer opened {stream_ref} {stream_id}; graceful shutdown recorded last stream id {last_stream_id}."
            )));
            diagnostic.related.push(note_json(format!(
                "Active shutdown state: {shutdown_state}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Endpoint role: {endpoint_role}.")));
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
        "http2.peer_limit.header_list_size_exceeded" => {
            let observed_size = json_number(protocol_entries, "observed_header_list_size")?;
            let allowed_size = json_number(protocol_entries, "allowed_header_list_size")?;
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let limit_provenance = json_string(protocol_entries, "receive_limit_provenance")?;
            let rule_provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("header list size exceeds receive maximum at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} decoded header list size {observed_size}; active receive maximum is {allowed_size}."
            )));
            diagnostic.related.push(note_json(format!(
                "Receive limit provenance: {limit_provenance}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {rule_provenance}.")));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            Some(diagnostic)
        }
        "http2.peer_limit.header_table_size_exceeded" => {
            let observed_size = json_number(protocol_entries, "observed_header_table_size")?;
            let allowed_size = json_number(protocol_entries, "allowed_header_table_size")?;
            let frame_kind = json_number(protocol_entries, "frame_kind")?;
            let stream_id = json_number(protocol_entries, "stream_id")?;
            let stream_ref = json_string(protocol_entries, "stream_ref")?;
            let limit_provenance = json_string(protocol_entries, "receive_limit_provenance")?;
            let rule_provenance = json_string(protocol_entries, "rule_provenance")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("header table size exceeds receive maximum at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "Frame kind {frame_kind} on {stream_ref} {stream_id} requested HPACK header table size {observed_size}; active receive maximum is {allowed_size}."
            )));
            diagnostic.related.push(note_json(format!(
                "Receive limit provenance: {limit_provenance}."
            )));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {rule_provenance}.")));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
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
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Peer limit provenance: {provenance}.")));
            Some(diagnostic)
        }
        "hpack.fixture.unsupported_header_block" => {
            let observed_size = json_number(protocol_entries, "observed_header_block_size")?;
            let observed_first_byte = json_number(protocol_entries, "observed_first_byte")?;
            let expected_fixture = json_string(protocol_entries, "expected_fixture")?;
            let codec_module = json_string(protocol_entries, "codec_module")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("unsupported HPACK fixture header block at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "HPACK fixture codec `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
            )));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Expected {expected_fixture}.")));
            Some(diagnostic)
        }
        "hpack.fixture.malformed_string_length" => {
            let observed_size = json_number(protocol_entries, "observed_header_block_size")?;
            let observed_first_byte = json_number(protocol_entries, "observed_first_byte")?;
            let expected_fixture = json_string(protocol_entries, "expected_fixture")?;
            let codec_module = json_string(protocol_entries, "codec_module")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("malformed HPACK string length at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "HPACK fixture codec `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
            )));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Expected {expected_fixture}.")));
            Some(diagnostic)
        }
        "hpack.fixture.malformed_raw_string_value" => {
            let observed_size = json_number(protocol_entries, "observed_header_block_size")?;
            let observed_first_byte = json_number(protocol_entries, "observed_first_byte")?;
            let expected_fixture = json_string(protocol_entries, "expected_fixture")?;
            let codec_module = json_string(protocol_entries, "codec_module")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("malformed HPACK raw string value at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "HPACK fixture codec `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
            )));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Expected {expected_fixture}.")));
            Some(diagnostic)
        }
        "hpack.fixture.malformed_huffman_padding" => {
            let observed_size = json_number(protocol_entries, "observed_header_block_size")?;
            let observed_first_byte = json_number(protocol_entries, "observed_first_byte")?;
            let expected_fixture = json_string(protocol_entries, "expected_fixture")?;
            let codec_module = json_string(protocol_entries, "codec_module")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("malformed HPACK Huffman padding at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "HPACK fixture codec `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
            )));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Expected {expected_fixture}.")));
            Some(diagnostic)
        }
        "hpack.fixture.huffman_eos_symbol" => {
            let observed_size = json_number(protocol_entries, "observed_header_block_size")?;
            let observed_first_byte = json_number(protocol_entries, "observed_first_byte")?;
            let expected_fixture = json_string(protocol_entries, "expected_fixture")?;
            let codec_module = json_string(protocol_entries, "codec_module")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!("HPACK Huffman EOS used as decoded symbol at byte offset {byte_offset}"),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "HPACK fixture codec `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
            )));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Expected {expected_fixture}.")));
            Some(diagnostic)
        }
        "hpack.fixture.huffman_non_visible_value" => {
            let observed_size = json_number(protocol_entries, "observed_header_block_size")?;
            let observed_first_byte = json_number(protocol_entries, "observed_first_byte")?;
            let expected_fixture = json_string(protocol_entries, "expected_fixture")?;
            let codec_module = json_string(protocol_entries, "codec_module")?;
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                format!(
                    "HPACK Huffman decoded non-visible header value at byte offset {byte_offset}"
                ),
                None,
                protocol_diagnostic.clone(),
            );
            diagnostic.related.push(note_json(format!(
                "HPACK fixture codec `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
            )));
            push_byte_preview_note(&mut diagnostic, protocol_entries);
            diagnostic
                .related
                .push(note_json(format!("Expected {expected_fixture}.")));
            Some(diagnostic)
        }
        _ => None,
    }
}

fn value_result_failure_diagnostic(failure: &TestFailure) -> Option<Diagnostic> {
    let details = json_object(&failure.details)?;
    let value_diagnostic = json_field(details, "value_diagnostic")?;
    let value_entries = json_object(value_diagnostic)?;
    let id = json_string(value_entries, "id")?;

    match id.as_str() {
        "schema.validation_failed" => {
            let predicate = json_string(value_entries, "predicate")?;
            let supplied_values = json_string(value_entries, "supplied_values")?;
            let result_value = result_failure_value(failure)?;
            let encode_result = result_value.starts_with("EncodeError(schema.validation_failed,");
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                if encode_result {
                    "schema encode validation failed".to_string()
                } else {
                    result_value.clone()
                },
                None,
                value_diagnostic.clone(),
            );
            if let Some(field_value) = json_number(value_entries, "field_value") {
                diagnostic.related.push(note_json(format!(
                    "Predicate `{predicate}` failed for supplied field value {field_value}."
                )));
            } else {
                diagnostic
                    .related
                    .push(note_json(format!("Schema predicate `{predicate}` failed.")));
            }
            diagnostic
                .related
                .push(note_json(format!("Supplied values: {supplied_values}.")));
            if let Some(field_path) = field_path_text(value_entries) {
                diagnostic
                    .related
                    .push(note_json(format!("Field path: {field_path}.")));
            }
            if encode_result {
                diagnostic
                    .related
                    .push(note_json(format!("Result value: {result_value}.")));
            }
            Some(diagnostic)
        }
        "codec.encode_value_unrepresentable" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "codec.encode_mapping_mismatch" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "codec.dispatch_unknown_tag" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "codec.dispatch_length_mismatch" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "codec.dispatch_mismatch" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "codec.byte_write_value_unrepresentable" => {
            byte_write_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        _ => None,
    }
}

fn byte_write_result_failure_diagnostic(
    failure: &TestFailure,
    value_diagnostic: &JsonValue,
    value_entries: &[(String, JsonValue)],
) -> Option<Diagnostic> {
    let id = json_string(value_entries, "id")?;
    let helper_name = json_string(value_entries, "helper_name")?;
    let supplied_value = json_number(value_entries, "supplied_value")?;
    let min_value = json_number(value_entries, "min_value")?;
    let max_value = json_number(value_entries, "max_value")?;
    let width = json_number(value_entries, "width")?;
    let byte_order = json_string(value_entries, "byte_order")?;
    let mut diagnostic = Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Runtime,
        "byte write value is unrepresentable",
        None,
        value_diagnostic.clone(),
    );
    diagnostic.related.push(note_json(format!(
        "Byte write helper `{helper_name}` received value {supplied_value}."
    )));
    diagnostic.related.push(note_json(format!(
        "Accepted range is {min_value}..{max_value}."
    )));
    diagnostic.related.push(note_json(format!(
        "Write width is {width} byte(s) with `{byte_order}` byte order."
    )));
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("Result value: {value}.")));
    }
    Some(diagnostic)
}

fn encode_result_failure_diagnostic(
    failure: &TestFailure,
    value_diagnostic: &JsonValue,
    value_entries: &[(String, JsonValue)],
) -> Option<Diagnostic> {
    let id = json_string(value_entries, "id")?;
    let reason = json_string(value_entries, "reason")?;
    let mut diagnostic = Diagnostic::new(
        id.clone(),
        Severity::Error,
        DiagnosticKind::Runtime,
        encode_diagnostic_message(&id),
        None,
        value_diagnostic.clone(),
    );
    if let Some(field_path) = field_path_text(value_entries) {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    } else if let Some(field_path) = json_string(value_entries, "field_path_display") {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    }
    diagnostic
        .related
        .push(note_json(format!("Encode failure reason: {reason}.")));
    if let (Some(expected_count), Some(actual_count)) = (
        json_number(value_entries, "expected_count"),
        json_number(value_entries, "actual_count"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected {expected_count} byte(s); supplied ByteView has {actual_count} byte(s)."
        )));
    }
    if let Some(byte_offset) = json_number(value_entries, "byte_offset") {
        diagnostic.related.push(note_json(format!(
            "Supplied ByteView starts at byte offset {byte_offset}."
        )));
    }
    push_byte_preview_note(&mut diagnostic, value_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("Result value: {value}.")));
    }
    Some(diagnostic)
}

fn encode_diagnostic_message(id: &str) -> String {
    match id {
        "codec.encode_value_unrepresentable" => "encode value is unrepresentable",
        "codec.encode_mapping_mismatch" => "encode mapping does not match value",
        "codec.dispatch_unknown_tag" => "unknown dispatch tag in encode value",
        "codec.dispatch_length_mismatch" => "dispatch payload length mismatch",
        "codec.dispatch_mismatch" => "dispatch tag and payload mismatch",
        _ => "encode failed",
    }
    .to_string()
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
                let message = runtime_error_message(&stderr, output.status);
                RunJsonReport::runtime_error(exit_code, stdout, stderr, message)
            }
        }
    };
    let exit_code = report.exit_code();
    println!("{}", report.to_json());
    Ok(exit_code)
}

fn runtime_error_message(stderr: &str, status: std::process::ExitStatus) -> String {
    stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("run process exited with status {status}"))
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

fn entry_arg_scalar(ty: &str) -> Option<EntryArgScalar> {
    match ty {
        "String" => Some(EntryArgScalar::String),
        "Int" => Some(EntryArgScalar::Int),
        "Float" => Some(EntryArgScalar::Float),
        "Bool" => Some(EntryArgScalar::Bool),
        _ => None,
    }
}

fn entry_arg_type_from_scalar(ty: EntryArgScalar) -> EntryArgType {
    match ty {
        EntryArgScalar::String => EntryArgType::String,
        EntryArgScalar::Int => EntryArgType::Int,
        EntryArgScalar::Float => EntryArgType::Float,
        EntryArgScalar::Bool => EntryArgType::Bool,
    }
}

fn validate_entry_arg(ty: EntryArgScalar, param_name: &str, raw_arg: &str) -> Result<(), String> {
    match ty {
        EntryArgScalar::String => Ok(()),
        EntryArgScalar::Int => raw_arg.parse::<i64>().map(|_| ()).map_err(|_| {
            format!("veln: invalid Int argument for parameter `{param_name}`: `{raw_arg}`")
        }),
        EntryArgScalar::Float => raw_arg.parse::<f64>().map(|_| ()).map_err(|_| {
            format!("veln: invalid Float argument for parameter `{param_name}`: `{raw_arg}`")
        }),
        EntryArgScalar::Bool if raw_arg == "true" || raw_arg == "false" => Ok(()),
        EntryArgScalar::Bool => Err(format!(
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
    fn byte_result_failure_diagnostic_projects_range_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.byte_range_out_of_bounds")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(2)),
                ]),
            ),
            ("field_path", JsonValue::array([])),
            ("requested_count", JsonValue::Number(2)),
            ("available_count", JsonValue::Number(1)),
            ("byte_preview", byte_preview_with_counts("02", 1, false)),
        ]);
        let failure = TestFailure::result_with_details(
            "byte view range exceeds chunk length".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.byte_range_out_of_bounds");
        assert_eq!(
            diagnostic.message,
            "byte range out of bounds at byte offset 2"
        );
        assert_eq!(diagnostic.related.len(), 2);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("requested 2 byte(s); 1 byte(s) were available")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("02 (showing 1 of 1 byte(s), complete)")
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_decode_error_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.invalid_input")),
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
                        ("name", JsonValue::string("ManualPacketWire")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("kind")),
                    ]),
                ]),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.kind"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeError(codec.invalid_input, ByteOffset(5), ManualPacketWire.kind)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.invalid_input");
        assert_eq!(diagnostic.message, "decode error at byte offset 5");
        assert_eq!(diagnostic.related.len(), 2);
        assert_eq!(
            diagnostic.related[0].to_json(),
            "{\"message\":\"Field path: schema `ManualPacketWire` / field `kind`.\"}"
        );
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"DecodeError value: DecodeError(codec.invalid_input, ByteOffset(5), ManualPacketWire.kind).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_decode_need_more_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.incomplete_input")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(3)),
                ]),
            ),
            ("field_path", JsonValue::array([])),
            ("readiness", JsonValue::string("need_bytes")),
            ("needed_count", JsonValue::Number(3)),
        ]);
        let failure = TestFailure::result_with_details(
            "NeedMore(NeedBytes(ByteCount(3)))".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.incomplete_input");
        assert_eq!(diagnostic.message, "incomplete input at byte offset 3");
        assert_eq!(diagnostic.related.len(), 3);
        assert_eq!(
            diagnostic.related[0].to_json(),
            "{\"message\":\"Decode readiness is `need_bytes` because input is closed.\"}"
        );
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Decoder needs at least 3 buffered byte(s) before retrying.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"DecodeStep value: NeedMore(NeedBytes(ByteCount(3))).\"}"
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
    fn byte_result_failure_diagnostic_projects_integer_range_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.integer_out_of_range")),
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
                        ("name", JsonValue::string("StreamIdentifierSample")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("stream_id")),
                    ]),
                ]),
            ),
            ("byte_width", JsonValue::Number(4)),
            ("min_value", JsonValue::Number(0)),
            ("max_value", JsonValue::Number(2147483647)),
            ("actual_value", JsonValue::Number(2147483648)),
            ("byte_preview", byte_preview("80000000")),
        ]);
        let failure = TestFailure::result_with_details(
            "schema integer out of range at byte offset 0".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "schema.integer_out_of_range");
        assert_eq!(
            diagnostic.message,
            "schema integer out of range at byte offset 0"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("expected value between 0 and 2147483647; actual value was 2147483648")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("schema `StreamIdentifierSample` / field `stream_id`")
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
    fn byte_result_failure_diagnostic_projects_schema_level_validation_context() {
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
                JsonValue::array([JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("SchemaLevelValidationSample")),
                ])]),
            ),
            (
                "predicate",
                JsonValue::string("length == padding_length + checksum"),
            ),
            (
                "decoded_values",
                JsonValue::string("length=5, padding_length=2, checksum=4"),
            ),
            ("length", JsonValue::Number(5)),
            ("padding_length", JsonValue::Number(2)),
            ("checksum", JsonValue::Number(4)),
            ("byte_preview", byte_preview("050204")),
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
                .contains("Schema predicate `length == padding_length + checksum` failed")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("schema `SchemaLevelValidationSample`")
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_mapping_division_by_zero_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.mapping_division_by_zero")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(2)),
                ]),
            ),
            (
                "field_path",
                JsonValue::array([
                    JsonValue::object([
                        ("kind", JsonValue::string("schema")),
                        ("name", JsonValue::string("PacketWire")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("quotient")),
                    ]),
                ]),
            ),
            ("target_field", JsonValue::string("quotient")),
            ("operator", JsonValue::string("/")),
            ("byte_preview", byte_preview("0c00")),
        ]);
        let failure = TestFailure::result_with_details(
            "schema mapping division by zero at byte offset 2".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "schema.mapping_division_by_zero");
        assert_eq!(
            diagnostic.message,
            "schema mapping division by zero at byte offset 2"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("target field `quotient`")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("0c 00 (showing 2 of 2 byte(s), complete)")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("schema `PacketWire` / field `quotient`")
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_length_division_by_zero_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.length_division_by_zero")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(2)),
                ]),
            ),
            (
                "field_path",
                JsonValue::array([
                    JsonValue::object([
                        ("kind", JsonValue::string("schema")),
                        ("name", JsonValue::string("PacketWire")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            ("length_expression", JsonValue::string("length / divisor")),
            ("divisor_operand", JsonValue::string("divisor")),
            ("operator", JsonValue::string("/")),
            ("byte_preview", byte_preview("0800")),
        ]);
        let failure = TestFailure::result_with_details(
            "schema length division by zero at byte offset 2".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "schema.length_division_by_zero");
        assert_eq!(
            diagnostic.message,
            "schema length division by zero at byte offset 2"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Length expression `length / divisor`")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("08 00 (showing 2 of 2 byte(s), complete)")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("schema `PacketWire` / field `payload`")
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
    fn value_result_failure_diagnostic_projects_byte_write_context() {
        let value_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("value_diagnostic")),
            (
                "id",
                JsonValue::string("codec.byte_write_value_unrepresentable"),
            ),
            ("field_path", JsonValue::array([])),
            ("helper_name", JsonValue::string("byte_write_u31_be")),
            ("supplied_value", JsonValue::Number(2147483648)),
            ("min_value", JsonValue::Number(0)),
            ("max_value", JsonValue::Number(2147483647)),
            ("width", JsonValue::Number(4)),
            ("byte_order", JsonValue::string("big_endian")),
        ]);
        let failure = TestFailure {
            kind: "result".to_string(),
            message: "runtime result failure: Err(byte_write_u31_be value must be between 0 and 2147483647)".to_string(),
            details: JsonValue::object([
                ("kind", JsonValue::string("result")),
                ("phase", JsonValue::string("runtime")),
                (
                    "value",
                    JsonValue::string("byte_write_u31_be value must be between 0 and 2147483647"),
                ),
                ("value_diagnostic", value_diagnostic),
            ]),
        };

        let diagnostic =
            value_result_failure_diagnostic(&failure).expect("value diagnostic should project");

        assert_eq!(diagnostic.id, "codec.byte_write_value_unrepresentable");
        assert_eq!(diagnostic.kind, DiagnosticKind::Runtime);
        assert_eq!(diagnostic.message, "byte write value is unrepresentable");
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("`byte_write_u31_be` received value 2147483648")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("Accepted range is 0..2147483647")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("4 byte(s) with `big_endian` byte order")
        );
    }

    #[test]
    fn value_result_failure_diagnostic_projects_byte_view_encode_context() {
        let value_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("value_diagnostic")),
            (
                "id",
                JsonValue::string("codec.encode_value_unrepresentable"),
            ),
            (
                "field_path",
                JsonValue::array([
                    JsonValue::object([
                        ("kind", JsonValue::string("schema")),
                        ("name", JsonValue::string("PacketWire")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            (
                "field_path_display",
                JsonValue::string("PacketWire.payload"),
            ),
            (
                "reason",
                JsonValue::string("byte view count 3 does not match length field `length` value 2"),
            ),
            ("expected_count", JsonValue::Number(2)),
            ("actual_count", JsonValue::Number(3)),
            ("length_expression", JsonValue::string("length")),
            ("byte_offset", JsonValue::Number(0)),
            ("byte_preview", byte_preview_with_counts("aabbcc", 3, false)),
        ]);
        let failure = TestFailure {
            kind: "result".to_string(),
            message: "runtime result failure: Err(EncodeError(codec.encode_value_unrepresentable, PacketWire.payload, byte view count 3 does not match length field `length` value 2))".to_string(),
            details: JsonValue::object([
                ("kind", JsonValue::string("result")),
                ("phase", JsonValue::string("runtime")),
                (
                    "value",
                    JsonValue::string("EncodeError(codec.encode_value_unrepresentable, PacketWire.payload, byte view count 3 does not match length field `length` value 2)"),
                ),
                ("value_diagnostic", value_diagnostic),
            ]),
        };

        let diagnostic =
            value_result_failure_diagnostic(&failure).expect("value diagnostic should project");

        assert_eq!(diagnostic.id, "codec.encode_value_unrepresentable");
        assert_eq!(diagnostic.message, "encode value is unrepresentable");
        assert_eq!(diagnostic.related.len(), 6);
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("Expected 2 byte(s); supplied ByteView has 3 byte(s)")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("Supplied ByteView starts at byte offset 0")
        );
        assert!(
            diagnostic.related[4]
                .to_json()
                .contains("aa bb cc (showing 3 of 3 byte(s), complete)")
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
    fn protocol_result_failure_diagnostic_projects_header_list_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.peer_limit.header_list_size_exceeded"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("observed_header_list_size", JsonValue::Number(10)),
            ("allowed_header_list_size", JsonValue::Number(9)),
            ("frame_kind", JsonValue::Number(9)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "receive_limit_provenance",
                JsonValue::string("local_configuration"),
            ),
            (
                "rule_provenance",
                JsonValue::string("header_list_receive_limit"),
            ),
            (
                "byte_preview",
                byte_preview_with_counts("060708090a0b0c0d", 9, true),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 header list size exceeds receive maximum at byte offset 12".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.peer_limit.header_list_size_exceeded");
        assert_eq!(
            diagnostic.message,
            "header list size exceeds receive maximum at byte offset 12"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("decoded header list size 10")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("active receive maximum is 9")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("local_configuration")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("header_list_receive_limit")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("06 07 08 09 0a 0b 0c 0d (showing 8 of 9 byte(s), truncated)")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_header_table_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.peer_limit.header_table_size_exceeded"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(35)),
                ]),
            ),
            ("observed_header_table_size", JsonValue::Number(289)),
            ("allowed_header_table_size", JsonValue::Number(160)),
            ("frame_kind", JsonValue::Number(9)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "receive_limit_provenance",
                JsonValue::string("local_configuration"),
            ),
            (
                "rule_provenance",
                JsonValue::string("hpack_dynamic_table_size_update"),
            ),
            ("byte_preview", byte_preview("3f8101")),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 header table size exceeds receive maximum at byte offset 35".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.peer_limit.header_table_size_exceeded");
        assert_eq!(
            diagnostic.message,
            "header table size exceeds receive maximum at byte offset 35"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("requested HPACK header table size 289")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("active receive maximum is 160")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("local_configuration")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("hpack_dynamic_table_size_update")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("3f 81 01 (showing 3 of 3 byte(s), complete)")
        );
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
            ("byte_preview", byte_preview("000500003fff")),
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
        assert_eq!(diagnostic.related.len(), 3);
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
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 05 00 00 3f ff")
        );
        assert!(diagnostic.related[2].to_json().contains("peer_settings"));
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_request_header_list_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_request_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(9)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("missing_required_pseudo_header"),
            ),
            ("header_name", JsonValue::string(":method")),
            ("decoded_header_names", JsonValue::string(":scheme,:path")),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_request_pseudo_headers"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list is missing :method at byte offset 12".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_request_header_list");
        assert_eq!(
            diagnostic.message,
            "request header list is missing :method at byte offset 12"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Frame kind 9 on stream 1")
        );
        assert!(diagnostic.related[0].to_json().contains(":scheme,:path"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_request_pseudo_headers")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_response_header_list_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_response_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(9)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("missing_required_pseudo_header"),
            ),
            ("header_name", JsonValue::string(":status")),
            ("decoded_header_names", JsonValue::string("server")),
            ("active_state", JsonValue::string("response-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_response_pseudo_headers"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 response header list is missing :status at byte offset 12".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_response_header_list");
        assert_eq!(
            diagnostic.message,
            "response header list is missing :status at byte offset 12"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Frame kind 9 on stream 1")
        );
        assert!(diagnostic.related[0].to_json().contains("server"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_response_pseudo_headers")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_duplicate_request_pseudo_header() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_request_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("duplicate_pseudo_header"),
            ),
            ("header_name", JsonValue::string(":method")),
            (
                "decoded_header_names",
                JsonValue::string(":method,:method,:scheme,:path"),
            ),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_request_pseudo_headers"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list contains duplicate :method at byte offset 12".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "request header list contains duplicate :method at byte offset 12"
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains(":method,:method,:scheme,:path")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_request_pseudo_header_order() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_request_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("pseudo_header_after_regular_header"),
            ),
            ("header_name", JsonValue::string(":method")),
            (
                "decoded_header_names",
                JsonValue::string("host,:method,:scheme,:path"),
            ),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_request_pseudo_headers"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list places :method after a regular header at byte offset 12"
                .to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "request header list places :method after a regular header at byte offset 12"
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("host,:method,:scheme,:path")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_uppercase_request_header_name() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_request_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("ordinary_header_name_not_lowercase"),
            ),
            ("header_name", JsonValue::string("Host")),
            (
                "decoded_header_names",
                JsonValue::string(":method,:scheme,:path,Host"),
            ),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_field_name_lowercase"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list contains uppercase ordinary header Host at byte offset 12"
                .to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "request header list contains uppercase ordinary header Host at byte offset 12"
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains(":method,:scheme,:path,Host")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_field_name_lowercase")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_request_connection_specific_header() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_request_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("connection_specific_header"),
            ),
            ("header_name", JsonValue::string("connection")),
            (
                "decoded_header_names",
                JsonValue::string(":method,:scheme,:path,connection"),
            ),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_connection_specific_header_fields"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list contains connection-specific header connection at byte offset 12"
                .to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "request header list contains connection-specific header connection at byte offset 12"
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains(":method,:scheme,:path,connection")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_connection_specific_header_fields")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_request_te_header_value() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_request_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("te_header_value_not_trailers"),
            ),
            ("header_name", JsonValue::string("te")),
            (
                "decoded_header_names",
                JsonValue::string(":method,:scheme,:path,te"),
            ),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_te_trailers_only"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list contains te value other than trailers at byte offset 12"
                .to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "request header list contains te value other than trailers at byte offset 12"
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains(":method,:scheme,:path,te")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_te_trailers_only")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_request_scheme_value() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_request_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("scheme_value_not_http_or_https"),
            ),
            ("header_name", JsonValue::string(":scheme")),
            (
                "decoded_header_names",
                JsonValue::string(":method,:scheme,:path"),
            ),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_request_scheme"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list contains :scheme value other than http or https at byte offset 12"
                .to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "request header list contains :scheme value other than http or https at byte offset 12"
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains(":method,:scheme,:path")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_request_scheme")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_response_ordinary_header_name_facts() {
        let uppercase_protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_response_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("ordinary_header_name_not_lowercase"),
            ),
            ("header_name", JsonValue::string("Server")),
            ("decoded_header_names", JsonValue::string(":status,Server")),
            ("active_state", JsonValue::string("response-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_field_name_lowercase"),
            ),
        ]);
        let uppercase_failure = TestFailure::result_with_details(
            "HTTP/2 response header list contains uppercase ordinary header Server at byte offset 12"
                .to_string(),
            None,
            None,
            Some(uppercase_protocol_diagnostic),
        );

        let uppercase_diagnostic = protocol_result_failure_diagnostic(&uppercase_failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            uppercase_diagnostic.message,
            "response header list contains uppercase ordinary header Server at byte offset 12"
        );
        assert!(
            uppercase_diagnostic.related[0]
                .to_json()
                .contains(":status,Server")
        );
        assert!(
            uppercase_diagnostic.related[2]
                .to_json()
                .contains("rfc9113_field_name_lowercase")
        );

        let token_protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_response_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("ordinary_header_name_invalid_token"),
            ),
            ("header_name", JsonValue::string("bad header")),
            (
                "decoded_header_names",
                JsonValue::string(":status,bad header"),
            ),
            ("active_state", JsonValue::string("response-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9110_field_name_token"),
            ),
        ]);
        let token_failure = TestFailure::result_with_details(
            "HTTP/2 response header list contains invalid ordinary header name bad header at byte offset 12"
                .to_string(),
            None,
            None,
            Some(token_protocol_diagnostic),
        );

        let token_diagnostic = protocol_result_failure_diagnostic(&token_failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            token_diagnostic.message,
            "response header list contains invalid ordinary header name bad header at byte offset 12"
        );
        assert!(
            token_diagnostic.related[2]
                .to_json()
                .contains("rfc9110_field_name_token")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_response_te_header_value() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_response_header_list"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            (
                "failed_header_fact",
                JsonValue::string("te_header_value_not_trailers"),
            ),
            ("header_name", JsonValue::string("te")),
            ("decoded_header_names", JsonValue::string(":status,te")),
            ("active_state", JsonValue::string("response-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_te_trailers_only"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 response header list contains te value other than trailers at byte offset 12"
                .to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "response header list contains te value other than trailers at byte offset 12"
        );
        assert!(diagnostic.related[0].to_json().contains(":status,te"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_te_trailers_only")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_hpack_preview_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.unsupported_header_block"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(27)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(1)),
            ("observed_first_byte", JsonValue::Number(255)),
            (
                "expected_fixture",
                JsonValue::string("fixture header block"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("ff")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK fixture unsupported header block at byte offset 27".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "hpack.fixture.unsupported_header_block");
        assert_eq!(
            diagnostic.message,
            "unsupported HPACK fixture header block at byte offset 27"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("ff (showing 1 of 1 byte(s), complete)")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_hpack_string_length_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.malformed_string_length"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(2)),
            ("observed_first_byte", JsonValue::Number(4)),
            (
                "expected_fixture",
                JsonValue::string("fixture HPACK string length"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("04ff")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK fixture malformed string length at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "hpack.fixture.malformed_string_length");
        assert_eq!(
            diagnostic.message,
            "malformed HPACK string length at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("04 ff (showing 2 of 2 byte(s), complete)")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_hpack_raw_string_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.malformed_raw_string_value"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(5)),
            ("observed_first_byte", JsonValue::Number(8)),
            (
                "expected_fixture",
                JsonValue::string("fixture HPACK raw string value"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("0803321f30")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK fixture malformed raw string value at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "hpack.fixture.malformed_raw_string_value");
        assert_eq!(
            diagnostic.message,
            "malformed HPACK raw string value at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("08 03 32 1f 30 (showing 5 of 5 byte(s), complete)")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_hpack_huffman_padding_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.malformed_huffman_padding"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(3)),
            ("observed_first_byte", JsonValue::Number(4)),
            (
                "expected_fixture",
                JsonValue::string("fixture HPACK Huffman padding"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("048100")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK fixture malformed Huffman padding at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "hpack.fixture.malformed_huffman_padding");
        assert_eq!(
            diagnostic.message,
            "malformed HPACK Huffman padding at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("header block size 3")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("04 81 00 (showing 3 of 3 byte(s), complete)")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_hpack_huffman_eos_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            ("id", JsonValue::string("hpack.fixture.huffman_eos_symbol")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(6)),
            ("observed_first_byte", JsonValue::Number(4)),
            (
                "expected_fixture",
                JsonValue::string("fixture HPACK Huffman data symbol instead of EOS"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("0484ffffffff")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK fixture Huffman EOS symbol at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "hpack.fixture.huffman_eos_symbol");
        assert_eq!(
            diagnostic.message,
            "HPACK Huffman EOS used as decoded symbol at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("04 84 ff ff ff ff (showing 6 of 6 byte(s), complete)")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_hpack_huffman_non_visible_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.huffman_non_visible_value"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(4)),
            ("observed_first_byte", JsonValue::Number(4)),
            (
                "expected_fixture",
                JsonValue::string("fixture HPACK Huffman visible ASCII header value"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("0482ffc7")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK fixture Huffman non-visible value at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "hpack.fixture.huffman_non_visible_value");
        assert_eq!(
            diagnostic.message,
            "HPACK Huffman decoded non-visible header value at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("04 82 ff c7 (showing 4 of 4 byte(s), complete)")
        );
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
            (
                "byte_preview",
                byte_preview_with_counts("0000000000000000", 9, true),
            ),
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
        assert_eq!(diagnostic.related.len(), 4);
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
                .contains("00 00 00 00 00 00 00 00")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("connection-control")
        );
        assert!(
            diagnostic.related[3]
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
            (
                "byte_preview",
                byte_preview_with_counts("0000000104000000", 9, true),
            ),
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
        assert_eq!(diagnostic.related.len(), 4);
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
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 00 00 01 04 00 00 00")
        );
        assert!(diagnostic.related[2].to_json().contains("stream-id-domain"));
        assert!(
            diagnostic.related[3]
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
            ("byte_preview", byte_preview("01020304050607")),
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
        assert_eq!(diagnostic.related.len(), 4);
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
                .contains("01 02 03 04 05 06 07 (showing 7 of 7 byte(s), complete)")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("connection-control")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_ping_payload_length")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_invalid_window_update_increment_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_window_update_increment"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(8)),
            ("stream_id", JsonValue::Number(0)),
            ("stream_ref", JsonValue::string("connection")),
            ("observed_window_increment", JsonValue::Number(0)),
            ("accepted_min_window_increment", JsonValue::Number(1)),
            (
                "accepted_max_window_increment",
                JsonValue::Number(2_147_483_647),
            ),
            ("byte_preview", byte_preview("00000000")),
            ("active_state", JsonValue::string("connection-flow-control")),
            (
                "rule_provenance",
                JsonValue::string("window_update_increment_nonzero"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 invalid WINDOW_UPDATE increment at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "http2.protocol.invalid_window_update_increment"
        );
        assert_eq!(
            diagnostic.message,
            "invalid WINDOW_UPDATE increment at byte offset 0"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("WINDOW_UPDATE increment 0")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("accepted range is 1..2147483647")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 00 00 00 (showing 4 of 4 byte(s), complete)")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("connection-flow-control")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("window_update_increment_nonzero")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_invalid_data_padding_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_data_padding"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(0)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            ("pad_length", JsonValue::Number(2)),
            ("remaining_payload_length", JsonValue::Number(0)),
            ("byte_preview", byte_preview("02")),
            ("active_state", JsonValue::string("open-stream")),
            ("rule_provenance", JsonValue::string("rfc9113_data_padding")),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 invalid DATA padding at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_data_padding");
        assert_eq!(diagnostic.message, "invalid DATA padding at byte offset 9");
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("pad length 2 byte(s)")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("remaining payload length is 0 byte(s)")
        );
        assert!(diagnostic.related[1].to_json().contains("02"));
        assert!(diagnostic.related[2].to_json().contains("open-stream"));
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_data_padding")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_unexpected_settings_ack_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.unexpected_settings_ack"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(4)),
            ("stream_id", JsonValue::Number(0)),
            ("stream_ref", JsonValue::string("connection")),
            (
                "byte_preview",
                byte_preview_with_counts("0000000401000000", 9, true),
            ),
            ("active_state", JsonValue::string("connection-control")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_settings_ack_requires_outstanding_local_settings"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 unexpected SETTINGS ACK at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.unexpected_settings_ack");
        assert_eq!(
            diagnostic.message,
            "unexpected SETTINGS ACK at byte offset 0"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("no local SETTINGS batch is outstanding")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 00 00 04 01 00 00 00")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("connection-control")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_settings_ack_requires_outstanding_local_settings")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_invalid_priority_dependency_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.invalid_priority_dependency"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("frame_kind", JsonValue::Number(2)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            ("dependency_stream_id", JsonValue::Number(1)),
            ("byte_preview", byte_preview("000000010f")),
            ("active_state", JsonValue::string("stream-control")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_priority_dependency"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 invalid PRIORITY dependency at byte offset 0".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_priority_dependency");
        assert_eq!(
            diagnostic.message,
            "invalid PRIORITY dependency at byte offset 0"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("declared itself as dependency stream 1")
        );
        assert!(diagnostic.related[2].to_json().contains("stream-control"));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 00 00 01 0f (showing 5 of 5 byte(s), complete)")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_priority_dependency")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_stream_after_goaway_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.stream_after_goaway"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
                ]),
            ),
            ("stream_id", JsonValue::Number(7)),
            ("stream_ref", JsonValue::string("stream")),
            ("last_stream_id", JsonValue::Number(5)),
            ("shutdown_state", JsonValue::string("graceful_shutdown")),
            ("endpoint_role", JsonValue::string("server")),
            ("active_state", JsonValue::string("graceful_shutdown")),
            (
                "rule_provenance",
                JsonValue::string("goaway_last_stream_id"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 stream opened after graceful shutdown at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.stream_after_goaway");
        assert_eq!(
            diagnostic.message,
            "stream opened after graceful shutdown at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 5);
        assert!(diagnostic.related[0].to_json().contains("stream 7"));
        assert!(diagnostic.related[0].to_json().contains("last stream id 5"));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("graceful_shutdown")
        );
        assert!(diagnostic.related[2].to_json().contains("server"));
        assert!(
            diagnostic.related[4]
                .to_json()
                .contains("goaway_last_stream_id")
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
            (
                "byte_preview",
                byte_preview_with_counts("0000000000000000", 9, true),
            ),
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
        assert_eq!(diagnostic.related.len(), 4);
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
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 00 00 00 00 00 00 00")
        );
        assert!(diagnostic.related[2].to_json().contains("idle-stream"));
        assert!(
            diagnostic.related[3]
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
