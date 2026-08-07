use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use veln_analysis::{
    AnalysisTiming, DoctestMode, ProjectAnalysis, analyze_project, analyze_project_with_timings,
};
use veln_ast::Function;
use veln_ast::FunctionKind;
use veln_backend_jvm::{EntryArgScalar, EntryArgType, generate_classfiles_with_entry_arg_types};
use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity};
use veln_project::{Project, explicit_companion_inputs, production_analysis_inputs};
use veln_test::{TestFailure, contract_failure_from_trace, result_failure_from_trace};

use crate::diagnostics::{
    harness_source_diagnostic_artifact_requested, has_error, print_human_stderr, tool_info,
    write_harness_source_diagnostic_artifact,
};
use crate::java::{
    JvmExecution, JvmExecutionPreparation, JvmRunResult, create_build_dir, exit_code_from_status,
    forward_process_output, prepare_and_run_jvm_capture_with_execution, prepare_jvm_execution,
};

pub(crate) fn run_entry(
    start: super::CommandAnalysisStart,
    json: bool,
    entry: String,
    inputs: Vec<PathBuf>,
    entry_args: Vec<String>,
) -> Result<ExitCode, String> {
    let inputs = start.resolve_inputs(inputs);
    if let Some(exit_code) =
        reject_explicit_companion_run_input(&start.package_root, json, &inputs)?
    {
        return Ok(exit_code);
    }
    let mut timings = RunAnalysisTimings::from_env();
    let analysis = analyze_run_project(start.package_root, &inputs, timings.as_mut())?;
    write_harness_source_diagnostic_artifact(&analysis.checked_diagnostics())?;
    if report_source_errors(&analysis)? {
        write_timings(&timings)?;
        return Ok(ExitCode::from(1));
    }

    let Some(entry_arg_types) = checked_entry_arg_types(&analysis, &entry, &entry_args)? else {
        write_timings(&timings)?;
        return Ok(ExitCode::from(1));
    };
    let Some(ir) = lower_run_entry(&analysis, &entry, timings.as_mut())? else {
        write_timings(&timings)?;
        return Ok(ExitCode::from(1));
    };

    let backend_start = timings.is_some().then(Instant::now);
    let jvm = generate_classfiles_with_entry_arg_types(&ir, &entry, &entry_arg_types);
    let execution = match prepare_run_jvm_execution(json, &mut timings, backend_start)? {
        Some(execution) => execution,
        None => return Ok(ExitCode::from(1)),
    };
    let build_dir = create_build_dir("veln-run").map_err(|error| error.to_string())?;
    let result = if json {
        run_json(&build_dir, &jvm, &entry_args, &execution)
    } else {
        run_human(&build_dir, &jvm, &entry_args, &execution)
    };
    let cleanup_result = fs::remove_dir_all(&build_dir);
    if let Err(error) = cleanup_result {
        eprintln!(
            "veln: warning: failed to remove build directory `{}`: {error}",
            build_dir.display()
        );
    }
    if let (Some(timings), Some(start)) = (timings.as_mut(), backend_start) {
        timings.push("backend_runtime_remainder", start.elapsed());
    }
    write_timings(&timings)?;
    result
}

fn prepare_run_jvm_execution(
    json: bool,
    timings: &mut Option<RunAnalysisTimings>,
    backend_start: Option<Instant>,
) -> Result<Option<JvmExecution>, String> {
    match prepare_jvm_execution("veln run") {
        Ok(JvmExecutionPreparation::Ready(execution)) => Ok(Some(execution)),
        Ok(JvmExecutionPreparation::ToolError(message)) => {
            push_backend_runtime_timing(timings, backend_start);
            write_timings(timings)?;
            report_run_tool_error(json, message)?;
            Ok(None)
        }
        Err(message) => {
            push_backend_runtime_timing(timings, backend_start);
            write_timings(timings)?;
            Err(message)
        }
    }
}

fn push_backend_runtime_timing(
    timings: &mut Option<RunAnalysisTimings>,
    backend_start: Option<Instant>,
) {
    if let (Some(timings), Some(start)) = (timings.as_mut(), backend_start) {
        timings.push("backend_runtime_remainder", start.elapsed());
    }
}

fn report_run_tool_error(json: bool, message: String) -> Result<ExitCode, String> {
    if json {
        let report = RunJsonReport::tool_error(message);
        println!("{}", report.to_json());
        return Ok(report.exit_code());
    }
    eprintln!("{message}");
    Ok(ExitCode::from(1))
}

fn analyze_run_project(
    root: PathBuf,
    inputs: &[PathBuf],
    timings: Option<&mut RunAnalysisTimings>,
) -> Result<ProjectAnalysis, String> {
    let source_start = timings.as_ref().map(|_| Instant::now());
    let discovered_inputs;
    let analysis_inputs = if harness_source_diagnostic_artifact_requested() {
        Vec::new()
    } else {
        discovered_inputs =
            production_analysis_inputs(&root, inputs).map_err(|error| error.to_string())?;
        discovered_inputs
    };
    let project = Project::discover(root, &analysis_inputs).map_err(|error| error.to_string())?;
    if let (Some(timings), Some(start)) = (timings, source_start) {
        timings.push("source_loading", start.elapsed());
        let doctest_mode = if harness_source_diagnostic_artifact_requested() {
            DoctestMode::Include
        } else {
            DoctestMode::Exclude
        };
        let (analysis, analysis_timings) = analyze_project_with_timings(project, doctest_mode);
        timings.extend(analysis_timings);
        return Ok(analysis);
    }
    let doctest_mode = if harness_source_diagnostic_artifact_requested() {
        DoctestMode::Include
    } else {
        DoctestMode::Exclude
    };
    Ok(analyze_project(project, doctest_mode))
}

fn reject_explicit_companion_run_input(
    root: &std::path::Path,
    json: bool,
    inputs: &[PathBuf],
) -> Result<Option<ExitCode>, String> {
    let companions = explicit_companion_inputs(root, inputs);
    let Some(companion) = companions.first() else {
        return Ok(None);
    };
    let diagnostic = test_only_run_input_diagnostic(companion);
    write_harness_source_diagnostic_artifact(&[])?;
    let envelope = DiagnosticEnvelope::new(tool_info(), vec![diagnostic]);
    if json {
        println!("{}", envelope.to_json());
    } else {
        print_human_stderr(&envelope)?;
    }
    Ok(Some(ExitCode::from(1)))
}

fn test_only_run_input_diagnostic(path: &str) -> Diagnostic {
    Diagnostic::new(
        "module.test_only_run_input",
        Severity::Error,
        DiagnosticKind::Module,
        format!("test companion `{path}` cannot be used as a run input"),
        None,
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("run_input")),
            ("source_path", JsonValue::string(path)),
            ("boundary", JsonValue::string("run")),
        ]),
    )
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
    timings: Option<&mut RunAnalysisTimings>,
) -> Result<Option<veln_ir::TypedProgram>, String> {
    let reachable = if let Some(timings) = timings {
        let (reachable, timing) =
            analysis.lower_reachable_entry_with_timing(entry, FunctionKind::Function);
        timings.push_analysis(timing);
        reachable
    } else {
        analysis.lower_reachable_entry(entry, FunctionKind::Function)
    };
    let lowered = reachable.lowered;
    if has_error(&lowered.diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        return Ok(None);
    }
    if let Some(diagnostic) = retained_user_effect_diagnostic(
        &reachable.module,
        lowered.core.as_ref(),
        entry,
        FunctionKind::Function,
    ) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), vec![diagnostic]))?;
        return Ok(None);
    }
    let Some(ir) = lowered.ir else {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        eprintln!("veln: run blocked: checked program is not executable");
        return Ok(None);
    };
    Ok(Some(ir))
}

struct RunAnalysisTimings {
    file: PathBuf,
    workload: String,
    run: String,
    records: Vec<AnalysisTiming>,
}

impl RunAnalysisTimings {
    fn from_env() -> Option<Self> {
        let file = env::var_os("VELN_ANALYSIS_TIMING_FILE").map(PathBuf::from)?;
        Some(Self {
            file,
            workload: env::var("VELN_ANALYSIS_TIMING_WORKLOAD")
                .unwrap_or_else(|_| "unknown".to_string()),
            run: env::var("VELN_ANALYSIS_TIMING_RUN").unwrap_or_else(|_| "unknown".to_string()),
            records: Vec::new(),
        })
    }

    fn push(&mut self, stage: &'static str, duration: Duration) {
        self.records.push(AnalysisTiming { stage, duration });
    }

    fn push_analysis(&mut self, timing: AnalysisTiming) {
        self.records.push(timing);
    }

    fn extend(&mut self, timings: Vec<AnalysisTiming>) {
        self.records.extend(timings);
    }

    fn write(&self) -> Result<(), String> {
        if self.records.is_empty() {
            return Ok(());
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)
            .map_err(|error| error.to_string())?;
        for record in &self.records {
            writeln!(
                file,
                "{{\"workload\":{},\"run\":{},\"stage\":{},\"boundary\":{},\"duration_seconds\":{}}}",
                json_literal_string(&self.workload),
                json_literal_string(&self.run),
                json_literal_string(record.stage),
                json_literal_string(record.stage),
                duration_seconds(record.duration),
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn write_timings(timings: &Option<RunAnalysisTimings>) -> Result<(), String> {
    if let Some(timings) = timings {
        timings.write()?;
    }
    Ok(())
}

fn json_literal_string(value: &str) -> String {
    JsonValue::string(value).to_json()
}

fn duration_seconds(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    format!("{}.{:09}", nanos / 1_000_000_000, nanos % 1_000_000_000)
}

const HOST_EFFECT_LABELS: &[&str] = &[
    "stdio",
    "fs",
    "net",
    "db",
    "time",
    "random",
    "process",
    "concurrency",
];

fn retained_user_effect_diagnostic(
    module: &veln_ast::SurfaceModule,
    core: Option<&veln_core::CheckedProgram>,
    entry: &str,
    kind: FunctionKind,
) -> Option<Diagnostic> {
    let function = module
        .functions
        .iter()
        .find(|function| function.kind == kind && function.name.as_deref() == Some(entry))?;
    let effects = core
        .and_then(|core| {
            core.functions
                .iter()
                .find(|core_function| core_function.node_id == function.node_id)
        })
        .map(|core_function| &core_function.effects)?;
    let effect = effects
        .iter()
        .find(|effect| !HOST_EFFECT_LABELS.contains(&effect.as_str()))?;
    Some(Diagnostic::new(
        "effect.unhandled_user",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("runnable entry retains user-defined effect `{effect}`"),
        Some(function.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("effect")),
            ("node_id", JsonValue::string(function.node_id.display("fn"))),
            ("effect", JsonValue::string(effect.clone())),
            ("boundary", JsonValue::string("run_entry")),
        ]),
    ))
}

fn run_human(
    build_dir: &std::path::Path,
    program: &veln_backend_jvm::JvmProgram,
    entry_args: &[String],
    execution: &JvmExecution,
) -> Result<ExitCode, String> {
    let result_error_file = build_dir.join("result-errors.tsv");
    let event_env = [("VELN_RESULT_ERRORS", result_error_file.as_os_str())];
    let result = prepare_and_run_jvm_capture_with_execution(
        execution, program, "veln run", &event_env, entry_args,
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

    let mut diagnostic = codec_byte_diagnostic(byte_diagnostic, byte_entries, &id, byte_offset)
        .or_else(|| schema_field_diagnostic(byte_diagnostic, byte_entries, &id, byte_offset))
        .or_else(|| {
            schema_constraint_diagnostic(byte_diagnostic, byte_entries, &id, byte_offset)
        })?;
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

fn codec_byte_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    match id {
        "codec.incomplete_input" => incomplete_input_diagnostic(details, entries, id, byte_offset),
        "codec.byte_range_out_of_bounds" => {
            byte_range_out_of_bounds_diagnostic(details, entries, id, byte_offset)
        }
        _ => None,
    }
}

fn schema_field_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    match id {
        "schema.fixed_field_mismatch" => {
            fixed_field_mismatch_diagnostic(details, entries, id, byte_offset)
        }
        "schema.truncated_field" => truncated_field_diagnostic(details, entries, id, byte_offset),
        "schema.length_out_of_bounds" => {
            length_out_of_bounds_diagnostic(details, entries, id, byte_offset)
        }
        "schema.integer_out_of_range" => {
            integer_out_of_range_diagnostic(details, entries, id, byte_offset)
        }
        "schema.reserved_bits_mismatch" => {
            reserved_bits_mismatch_diagnostic(details, entries, id, byte_offset)
        }
        _ => None,
    }
}

fn schema_constraint_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    match id {
        "schema.validation_failed" => {
            validation_failed_diagnostic(details, entries, id, byte_offset)
        }
        "schema.length_division_by_zero" => {
            length_division_by_zero_diagnostic(details, entries, id, byte_offset)
        }
        "schema.length_multiple_mismatch" => {
            length_multiple_mismatch_diagnostic(details, entries, id, byte_offset)
        }
        "schema.dispatch_unknown_tag" => {
            dispatch_unknown_tag_diagnostic(details, entries, id, byte_offset)
        }
        _ => None,
    }
}

fn incomplete_input_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let expected_count = json_number(entries, "expected_count")?;
    let available_count = json_number(entries, "available_count")?;
    let readiness = json_string(entries, "readiness")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("missing byte at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "pending readiness is `{readiness}` because input is closed."
    )));
    diagnostic.related.push(note_json(format!(
        "Fixed-width read expected {expected_count} byte(s); {available_count} byte(s) were available."
    )));
    Some(diagnostic)
}

fn byte_range_out_of_bounds_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let requested_count = json_number(entries, "requested_count")?;
    let available_count = json_number(entries, "available_count")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("byte range out of bounds at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Byte range requested {requested_count} byte(s); {available_count} byte(s) were available from the offset."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn fixed_field_mismatch_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let expected_value = json_number(entries, "expected_value")?;
    let actual_value = json_number(entries, "actual_value")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("fixed field mismatch at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Fixed field expected value {expected_value}; actual value was {actual_value}."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn truncated_field_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let expected_count = json_number(entries, "expected_count")?;
    let available_count = json_number(entries, "available_count")?;
    let readiness = json_string(entries, "readiness")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("truncated schema field at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "pending readiness is `{readiness}` because input is closed."
    )));
    diagnostic.related.push(note_json(format!(
        "Schema field expected {expected_count} byte(s); {available_count} byte(s) were available."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn length_out_of_bounds_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let expected_count = json_number(entries, "expected_count")?;
    let available_count = json_number(entries, "available_count")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("payload length out of bounds at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Payload length expected {expected_count} byte(s); {available_count} byte(s) were available."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn integer_out_of_range_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let byte_width = json_number(entries, "byte_width")?;
    let min_value = json_number(entries, "min_value")?;
    let max_value = json_number(entries, "max_value")?;
    let actual_value = json_number_display(entries, "actual_value")
        .or_else(|| json_string(entries, "actual_value_text"))?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("schema integer out of range at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "{byte_width}-byte schema integer expected value between {min_value} and {max_value}; actual value was {actual_value}."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn reserved_bits_mismatch_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let bit_width = json_number(entries, "bit_width")?;
    let expected_value = json_number(entries, "expected_value")?;
    let actual_value = json_number(entries, "actual_value")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("reserved bits mismatch at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "ReservedBits({bit_width}, {expected_value}) expected value {expected_value}; actual value was {actual_value}."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn validation_failed_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let predicate = json_string(entries, "predicate")?;
    let decoded_values = json_string(entries, "decoded_values").or_else(|| {
        let length = json_number(entries, "length")?;
        let padding_length = json_number(entries, "padding_length")?;
        Some(format!("length={length}, padding_length={padding_length}"))
    })?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("schema validation failed at byte offset {byte_offset}"),
        details,
    );
    if let Some(field_value) = json_number(entries, "field_value") {
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
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn length_division_by_zero_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let length_expression = json_string(entries, "length_expression")?;
    let divisor_operand = json_string(entries, "divisor_operand")?;
    let operator = json_string(entries, "operator")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("schema length division by zero at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Length expression `{length_expression}` evaluated `{operator}` with divisor operand `{divisor_operand}` equal to 0."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn length_multiple_mismatch_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let observed_count = json_number(entries, "observed_count")?;
    let required_multiple = json_number(entries, "required_multiple")?;
    let multiple_operand = json_string(entries, "multiple_operand")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("payload length multiple mismatch at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Payload count {observed_count} must be a multiple of `{multiple_operand}` value {required_multiple}."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn dispatch_unknown_tag_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let tag_field = json_string(entries, "tag_field")?;
    let decoded_tag_value = json_number(entries, "decoded_tag_value")?;
    let expected_tags = json_string(entries, "expected_tags")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("unknown dispatch tag at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Dispatch tag field `{tag_field}` decoded value {decoded_tag_value}."
    )));
    diagnostic
        .related
        .push(note_json(format!("Expected tag values: {expected_tags}.")));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn runtime_byte_diagnostic(id: &str, message: String, details: &JsonValue) -> Diagnostic {
    Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Runtime,
        message,
        None,
        details.clone(),
    )
}

fn is_decode_error_result_failure(failure: &TestFailure) -> bool {
    result_failure_value(failure)
        .as_deref()
        .is_some_and(|value| {
            value.starts_with("DecodeError(") || value.starts_with("DecodeErrorWithReason(")
        })
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
    if id == "codec.checksum_mismatch" {
        return checksum_mismatch_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.length_mismatch" {
        return length_mismatch_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.payload_length_mismatch" {
        return payload_length_mismatch_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.padding_mismatch" {
        return padding_mismatch_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.integer_out_of_range" {
        return integer_out_of_range_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.sequence_mismatch" {
        return sequence_mismatch_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.version_mismatch" {
        return version_mismatch_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.tag_mismatch" {
        return tag_mismatch_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.magic_mismatch" {
        return magic_mismatch_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.unsupported_feature" {
        return unsupported_feature_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.trailing_input" {
        return trailing_input_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
    if id == "codec.consumed_count_invalid" {
        return consumed_count_invalid_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        );
    }
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
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Decode failure reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn checksum_mismatch_result_failure_diagnostic(
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
        format!("checksum mismatch at byte offset {byte_offset}"),
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
    if let (Some(expected_checksum), Some(actual_checksum)) = (
        json_string(byte_entries, "expected_checksum"),
        json_string(byte_entries, "actual_checksum"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected checksum `{expected_checksum}`; actual checksum was `{actual_checksum}`."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Checksum failure reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn length_mismatch_result_failure_diagnostic(
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
        format!("length mismatch at byte offset {byte_offset}"),
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
    if let (Some(expected_length), Some(actual_length)) = (
        json_number(byte_entries, "expected_length"),
        json_number(byte_entries, "actual_length"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected length {expected_length}; actual length was {actual_length}."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Length mismatch reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn payload_length_mismatch_result_failure_diagnostic(
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
        format!("payload length mismatch at byte offset {byte_offset}"),
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
    if let (Some(expected_payload_length), Some(actual_payload_length)) = (
        json_number(byte_entries, "expected_payload_length"),
        json_number(byte_entries, "actual_payload_length"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected payload length {expected_payload_length}; actual payload length was {actual_payload_length}."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic.related.push(note_json(format!(
            "Payload length mismatch reason: {reason}."
        )));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn padding_mismatch_result_failure_diagnostic(
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
        format!("padding mismatch at byte offset {byte_offset}"),
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
    if let (Some(expected_padding_length), Some(actual_padding_length)) = (
        json_number(byte_entries, "expected_padding_length"),
        json_number(byte_entries, "actual_padding_length"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected padding length {expected_padding_length}; actual padding length was {actual_padding_length}."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Padding mismatch reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn integer_out_of_range_result_failure_diagnostic(
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
        format!("integer out of range at byte offset {byte_offset}"),
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
    if let (Some(byte_width), Some(min_value), Some(max_value), Some(actual_value)) = (
        json_number(byte_entries, "byte_width"),
        json_number(byte_entries, "min_value"),
        json_number(byte_entries, "max_value"),
        json_number(byte_entries, "actual_value"),
    ) {
        diagnostic.related.push(note_json(format!(
            "{byte_width}-byte integer expected value between {min_value} and {max_value}; actual value was {actual_value}."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Integer conversion reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn sequence_mismatch_result_failure_diagnostic(
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
        format!("sequence mismatch at byte offset {byte_offset}"),
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
    if let (Some(expected_sequence), Some(actual_sequence)) = (
        json_string(byte_entries, "expected_sequence"),
        json_string(byte_entries, "actual_sequence"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected sequence `{expected_sequence}`; actual sequence was `{actual_sequence}`."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Sequence mismatch reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn tag_mismatch_result_failure_diagnostic(
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
        format!("tag mismatch at byte offset {byte_offset}"),
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
    if let (Some(expected_tag), Some(actual_tag)) = (
        json_string(byte_entries, "expected_tag"),
        json_string(byte_entries, "actual_tag"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected tag `{expected_tag}`; actual tag was `{actual_tag}`."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Tag mismatch reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn magic_mismatch_result_failure_diagnostic(
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
        format!("magic mismatch at byte offset {byte_offset}"),
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
    if let (Some(expected_magic), Some(actual_magic)) = (
        json_string(byte_entries, "expected_magic"),
        json_string(byte_entries, "actual_magic"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected magic `{expected_magic}`; actual magic was `{actual_magic}`."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Magic mismatch reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn version_mismatch_result_failure_diagnostic(
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
        format!("version mismatch at byte offset {byte_offset}"),
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
    if let (Some(expected_version), Some(actual_version)) = (
        json_string(byte_entries, "expected_version"),
        json_string(byte_entries, "actual_version"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected version `{expected_version}`; actual version was `{actual_version}`."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Version mismatch reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn unsupported_feature_result_failure_diagnostic(
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
        format!("unsupported feature failed at byte offset {byte_offset}"),
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
    if let Some(unsupported_feature) = json_string(byte_entries, "unsupported_feature") {
        diagnostic.related.push(note_json(format!(
            "Unsupported feature: `{unsupported_feature}`."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Unsupported feature reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn trailing_input_result_failure_diagnostic(
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
        format!("trailing input at byte offset {byte_offset}"),
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
    if let (Some(consumed_count), Some(available_count), Some(remaining_count)) = (
        json_number(byte_entries, "consumed_count"),
        json_number(byte_entries, "available_count"),
        json_number(byte_entries, "remaining_count"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Consumed {consumed_count} of {available_count} available bytes; {remaining_count} bytes remain."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Trailing input reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn consumed_count_invalid_result_failure_diagnostic(
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
        format!("invalid decoded consumed count at byte offset {byte_offset}"),
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
    if let (Some(available_count), Some(actual_consumed_count)) = (
        json_number(byte_entries, "available_count"),
        json_number(byte_entries, "actual_consumed_count"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Decoder consumed {actual_consumed_count} byte(s); supplied view length was {available_count} byte(s)."
        )));
    }
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Consumed count reason: {reason}.")));
    }
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn push_decode_byte_context_notes(diagnostic: &mut Diagnostic, entries: &[(String, JsonValue)]) {
    if let Some(local_byte_offset) = json_number(entries, "local_byte_offset") {
        diagnostic.related.push(note_json(format!(
            "Local byte offset: {local_byte_offset}."
        )));
    }
    if let (Some(expected_count), Some(available_count)) = (
        json_number(entries, "expected_count"),
        json_number(entries, "available_count"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Decoder expected {expected_count} byte(s); {available_count} byte(s) were available."
        )));
    }
    push_byte_preview_note(diagnostic, entries);
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
    ProtocolDiagnosticContext::from_failure(failure)?.project()
}

struct ProtocolDiagnosticContext<'a> {
    source: &'a JsonValue,
    entries: &'a [(String, JsonValue)],
    id: String,
    byte_offset: i64,
}

impl<'a> ProtocolDiagnosticContext<'a> {
    fn from_failure(failure: &'a TestFailure) -> Option<Self> {
        let details = json_object(&failure.details)?;
        let source = json_field(details, "protocol_diagnostic")?;
        let entries = json_object(source)?;
        Some(Self {
            source,
            entries,
            id: json_string(entries, "id")?,
            byte_offset: byte_offset_value(entries)?,
        })
    }

    fn project(&self) -> Option<Diagnostic> {
        self.project_connection_rule()
            .or_else(|| self.project_header_list_rule())
            .or_else(|| self.project_stream_rule())
            .or_else(|| self.project_peer_limit_rule())
            .or_else(|| self.project_hpack_fixture_rule())
    }

    fn project_connection_rule(&self) -> Option<Diagnostic> {
        self.project_connection_lifecycle_rule()
            .or_else(|| self.project_frame_shape_rule())
    }

    fn project_connection_lifecycle_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.protocol.closed_with_pending" => self.project_closed_with_pending(),
            "http2.protocol.partial_preface" => self.project_partial_preface(),
            "http2.protocol.invalid_preface" => self.project_invalid_preface(),
            "http2.protocol.continuation_expected" => self.project_continuation_expected(),
            _ => None,
        }
    }

    fn project_closed_with_pending(&self) -> Option<Diagnostic> {
        let pending_count = self.number("pending_count")?;
        let active_continuation = self.string("active_continuation")?;
        let expected_stream = self.number("expected_stream_id")?;
        let started_kind = self.number("started_frame_kind")?;
        let started_offset = self.number("started_byte_offset")?;
        let accumulated = self.number("accumulated_header_block_bytes")?;
        let rule_provenance = self.string("rule_provenance")?;
        let mut diagnostic = self.diagnostic(format!(
            "input ended with pending bytes at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Input end arrived while {pending_count} byte(s) remained undecoded."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic.related.push(note_json(format!(
            "Active continuation state: {active_continuation}."
        )));
        if active_continuation != "none" {
            diagnostic.related.push(note_json(format!(
                "Pending header block started with frame kind {started_kind} at byte offset {started_offset} for stream {expected_stream}; accumulated {accumulated} header-block byte(s)."
            )));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {rule_provenance}.")));
        }
        Some(diagnostic)
    }

    fn project_partial_preface(&self) -> Option<Diagnostic> {
        let pending_count = self.number("pending_count")?;
        let expected_count = self.number("expected_count")?;
        let mut diagnostic = self.diagnostic(format!(
            "input ended with partial client connection preface at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Input end arrived after {pending_count} of {expected_count} preface byte(s)."
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_preface(&self) -> Option<Diagnostic> {
        let expected_byte = self.number("expected_byte")?;
        let actual_byte = self.number("actual_byte")?;
        let matched_count = self.number("matched_prefix_count")?;
        let expected_count = self.number("expected_count")?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid client connection preface at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Observed byte {actual_byte}; expected byte {expected_byte} after {matched_count} of {expected_count} preface byte(s)."
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_continuation_expected(&self) -> Option<Diagnostic> {
        let actual_kind = self.number("actual_frame_kind")?;
        let actual_stream = self.number("actual_stream_id")?;
        let expected_stream = self.number("expected_stream_id")?;
        let started_kind = self.number("started_frame_kind")?;
        let started_offset = self.number("started_byte_offset")?;
        let active_continuation = self.string("active_continuation")?;
        let accumulated = self.number("accumulated_header_block_bytes")?;
        let rule_provenance = self.string("rule_provenance")?;
        let mut diagnostic = self.diagnostic(format!(
            "expected CONTINUATION frame at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Incoming frame kind {actual_kind} on stream {actual_stream} violated active continuation state `{active_continuation}`."
        )));
        diagnostic.related.push(note_json(format!(
            "Pending header block started with frame kind {started_kind} at byte offset {started_offset} for stream {expected_stream}; accumulated {accumulated} header-block byte(s)."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Rule provenance: {rule_provenance}.")));
        Some(diagnostic)
    }

    fn project_frame_shape_rule(&self) -> Option<Diagnostic> {
        self.project_frame_identity_rule()
            .or_else(|| self.project_frame_payload_rule())
            .or_else(|| self.project_settings_ack_rule())
    }

    fn project_frame_identity_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.protocol.initial_peer_settings_required" => {
                self.project_initial_peer_settings_required()
            }
            "http2.protocol.invalid_frame_kind" => self.project_invalid_frame_kind(),
            "http2.protocol.invalid_stream_id" => self.project_invalid_stream_id(),
            "http2.protocol.settings_not_allowed_for_endpoint" => {
                self.project_settings_not_allowed_for_endpoint()
            }
            "http2.protocol.peer_stream_id_not_increasing" => {
                self.project_peer_stream_id_not_increasing()
            }
            _ => None,
        }
    }

    fn project_initial_peer_settings_required(&self) -> Option<Diagnostic> {
        let actual_kind = self.number("actual_frame_kind")?;
        let actual_flags = self.number("actual_flags")?;
        let endpoint_role = self.string("endpoint_role")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "initial peer frame must be non-ACK SETTINGS at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {actual_kind} with flags {actual_flags} on {} {} cannot start a {endpoint_role} endpoint connection.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_frame_kind(&self) -> Option<Diagnostic> {
        let actual_kind = self.number("actual_frame_kind")?;
        let expected_kind = self.number("expected_frame_kind")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid frame kind at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {actual_kind} on {} {} did not match expected frame kind {expected_kind}.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_stream_id(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let required_domain = self.string("required_stream_id_domain")?;
        let endpoint_role = self.string("endpoint_role")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid stream id at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} requires {required_domain} for {endpoint_role}.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_settings_not_allowed_for_endpoint(&self) -> Option<Diagnostic> {
        let setting_identifier = self.number("setting_identifier")?;
        let setting_name = self.string("setting_name")?;
        let endpoint_role = self.string("endpoint_role")?;
        let frame_kind = self.number("frame_kind")?;
        let mut diagnostic = self.diagnostic(format!(
            "{setting_name} is not allowed for {endpoint_role} endpoints at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "{setting_name} ({setting_identifier}) appeared in frame kind {frame_kind}."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Endpoint role: {endpoint_role}.")));
        self.push_active_state(&mut diagnostic)?;
        let rule_provenance = self.string("rule_provenance")?;
        diagnostic
            .related
            .push(note_json(format!("Rule provenance: {rule_provenance}.")));
        Some(diagnostic)
    }

    fn project_peer_stream_id_not_increasing(&self) -> Option<Diagnostic> {
        let frame = self.frame_ref()?;
        let previous_stream_id = self.number("previous_peer_stream_id")?;
        let endpoint_role = self.string("endpoint_role")?;
        let mut diagnostic = self.diagnostic(format!(
            "peer-created stream id {} is not greater than {previous_stream_id} at byte offset {}",
            frame.stream_id, self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "The {endpoint_role} endpoint attempted to create idle stream {} after peer-created stream {previous_stream_id}.",
            frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        diagnostic.related.push(note_json(format!(
            "Use a new peer-created stream id greater than {previous_stream_id}."
        )));
        Some(diagnostic)
    }

    fn project_frame_payload_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.protocol.invalid_payload_length" => self.project_invalid_payload_length(),
            "http2.protocol.invalid_window_update_increment" => {
                self.project_invalid_window_update_increment()
            }
            "http2.protocol.invalid_data_padding" => self.project_invalid_data_padding(),
            "http2.protocol.content_length_mismatch" => self.project_content_length_mismatch(),
            _ => None,
        }
    }

    fn project_invalid_payload_length(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let observed_length = self.number("observed_payload_length")?;
        let expected_length = self.number("expected_payload_length")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid payload length at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} declared {observed_length} byte(s); expected {expected_length} byte(s).",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_window_update_increment(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let observed_increment = self.number("observed_window_increment")?;
        let accepted_min = self.number("accepted_min_window_increment")?;
        let accepted_max = self.number("accepted_max_window_increment")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid WINDOW_UPDATE increment at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} declared WINDOW_UPDATE increment {observed_increment}; accepted range is {accepted_min}..{accepted_max}.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_data_padding(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let pad_length = self.number("pad_length")?;
        let remaining_payload_length = self.number("remaining_payload_length")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid DATA padding at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} declared pad length {pad_length} byte(s); remaining payload length is {remaining_payload_length} byte(s).",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_content_length_mismatch(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let expected_length = self.number("expected_content_length")?;
        let observed_length = self.number("observed_body_length")?;
        let active_state = self.string("active_state")?;
        let rule_provenance = self.string("rule_provenance")?;
        let frame = self.frame_ref()?;
        let no_content_status = match active_state.as_str() {
            "no-content-response-204" => Some("204"),
            "no-content-response-304" => Some("304"),
            _ => None,
        };
        let mut diagnostic = match no_content_status {
            Some(status) if rule_provenance == "rfc9110_no_content_response_body" => {
                let mut diagnostic = self.diagnostic(format!(
                    "response status {status} prohibits nonempty DATA at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} contributed {observed_length} DATA application byte(s); response status {status} permits no application content.",
                    frame.stream_ref, frame.stream_id
                )));
                diagnostic
            }
            _ => {
                let mut diagnostic = self.diagnostic(format!(
                    "content-length body length mismatch at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} observed {observed_length} DATA application byte(s); accepted content-length is {expected_length} byte(s).",
                    frame.stream_ref, frame.stream_id
                )));
                diagnostic
            }
        };
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_settings_ack_rule(&self) -> Option<Diagnostic> {
        if self.id != "http2.protocol.unexpected_settings_ack" {
            return None;
        }
        let frame_kind = self.number("frame_kind")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "unexpected SETTINGS ACK at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} acknowledged local SETTINGS, but no local SETTINGS batch is outstanding.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_header_list_rule(&self) -> Option<Diagnostic> {
        let (decoded_label, message) = match self.id.as_str() {
            "http2.protocol.invalid_request_header_list" => (
                "request header",
                protocol_header_list_message(
                    "request header list",
                    &self.string("failed_header_fact")?,
                    &self.string("header_name")?,
                    self.byte_offset,
                ),
            ),
            "http2.protocol.invalid_response_header_list" => {
                let active_state = self.string("active_state")?;
                let subject = if active_state == "response-trailers" {
                    "response trailer list"
                } else {
                    "response header list"
                };
                let decoded_label = if active_state == "response-trailers" {
                    "response trailer"
                } else {
                    "response header"
                };
                (
                    decoded_label,
                    protocol_header_list_message(
                        subject,
                        &self.string("failed_header_fact")?,
                        &self.string("header_name")?,
                        self.byte_offset,
                    ),
                )
            }
            _ => return None,
        };
        let frame_kind = self.number("frame_kind")?;
        let decoded_header_names = self.string("decoded_header_names")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(message);
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} decoded {decoded_label} names: {decoded_header_names}.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_stream_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.protocol.invalid_priority_dependency" => {
                let frame_kind = self.number("frame_kind")?;
                let dependency_stream_id = self.number("dependency_stream_id")?;
                let frame = self.frame_ref()?;
                let mut diagnostic = self.diagnostic(format!(
                    "invalid PRIORITY dependency at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} declared itself as dependency stream {dependency_stream_id}.",
                    frame.stream_ref, frame.stream_id
                )));
                self.push_preview_state_and_provenance(&mut diagnostic)?;
                Some(diagnostic)
            }
            "http2.protocol.stream_after_goaway" => {
                let frame = self.frame_ref()?;
                let last_stream_id = self.number("last_stream_id")?;
                let shutdown_state = self.string("shutdown_state")?;
                let endpoint_role = self.string("endpoint_role")?;
                let mut diagnostic = self.diagnostic(format!(
                    "stream opened after graceful shutdown at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Peer opened {} {}; graceful shutdown recorded last stream id {last_stream_id}.",
                    frame.stream_ref, frame.stream_id
                )));
                diagnostic.related.push(note_json(format!(
                    "Active shutdown state: {shutdown_state}."
                )));
                diagnostic
                    .related
                    .push(note_json(format!("Endpoint role: {endpoint_role}.")));
                self.push_preview_state_and_provenance(&mut diagnostic)?;
                Some(diagnostic)
            }
            _ => None,
        }
    }

    fn project_peer_limit_rule(&self) -> Option<Diagnostic> {
        self.project_peer_size_limit_rule()
            .or_else(|| self.project_peer_flow_limit_rule())
            .or_else(|| self.project_peer_settings_limit_rule())
    }

    fn project_peer_size_limit_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.peer_limit.frame_size_exceeded" => {
                let observed_length = self.number("observed_payload_length")?;
                let allowed_length = self.number("allowed_max_frame_size")?;
                let frame_kind = self.number("frame_kind")?;
                let frame = self.frame_ref()?;
                let provenance = self.string("receive_limit_provenance")?;
                let mut diagnostic = self.diagnostic(format!(
                    "frame payload length exceeds receive maximum at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} declared {observed_length} byte(s); active receive maximum is {allowed_length} byte(s).",
                    frame.stream_ref, frame.stream_id
                )));
                push_byte_preview_note(&mut diagnostic, self.entries);
                diagnostic.related.push(note_json(format!(
                    "Receive limit provenance: {provenance}."
                )));
                Some(diagnostic)
            }
            "http2.peer_limit.header_list_size_exceeded" => self.project_size_limit_rule(
                "observed_header_list_size",
                "allowed_header_list_size",
                "header list size exceeds receive maximum",
                "decoded header list size",
            ),
            "http2.peer_limit.header_table_size_exceeded" => self.project_size_limit_rule(
                "observed_header_table_size",
                "allowed_header_table_size",
                "header table size exceeds receive maximum",
                "requested HPACK header table size",
            ),
            _ => None,
        }
    }

    fn project_peer_flow_limit_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.peer_limit.flow_control_window_exceeded" => {
                let observed_length = self.number("observed_payload_length")?;
                let allowed_credit = self.number("allowed_window_credit")?;
                let frame_kind = self.number("frame_kind")?;
                let frame = self.frame_ref()?;
                let mut diagnostic = self.diagnostic(format!(
                    "flow-control window exceeded at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} declared {observed_length} byte(s); available receive window credit is {allowed_credit} byte(s).",
                    frame.stream_ref, frame.stream_id
                )));
                self.push_preview_state_and_provenance(&mut diagnostic)?;
                Some(diagnostic)
            }
            "http2.peer_limit.concurrent_streams_exceeded" => {
                let frame = self.frame_ref()?;
                let current_count = self.number("current_open_peer_created_stream_count")?;
                let attempted_count = self.number("attempted_concurrent_stream_count")?;
                let allowed_count = self.number("allowed_concurrent_stream_count")?;
                let endpoint_role = self.string("endpoint_role")?;
                let limit_provenance = self.string("receive_limit_provenance")?;
                let rule_provenance = self.string("rule_provenance")?;
                let mut diagnostic = self.diagnostic(format!(
                    "concurrent stream receive limit exceeded at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Opening {} {} would make {attempted_count} concurrent peer-created stream(s); {current_count} peer-created stream(s) are currently open and the active receive limit is {allowed_count}.",
                    frame.stream_ref, frame.stream_id
                )));
                push_byte_preview_note(&mut diagnostic, self.entries);
                self.push_active_state(&mut diagnostic)?;
                diagnostic
                    .related
                    .push(note_json(format!("Endpoint role: {endpoint_role}.")));
                diagnostic.related.push(note_json(format!(
                    "Receive limit provenance: {limit_provenance}."
                )));
                diagnostic
                    .related
                    .push(note_json(format!("Rule provenance: {rule_provenance}.")));
                Some(diagnostic)
            }
            _ => None,
        }
    }

    fn project_peer_settings_limit_rule(&self) -> Option<Diagnostic> {
        if self.id != "http2.peer_limit.settings_value_out_of_range" {
            return None;
        }
        let setting_identifier = self.number("setting_identifier")?;
        let setting_name = self.string("setting_name")?;
        let observed_value = self.number("observed_value")?;
        let accepted_min_value = self.number("accepted_min_value")?;
        let accepted_max_value = self.number("accepted_max_value")?;
        let provenance = self.string("peer_limit_provenance")?;
        let mut diagnostic = self.diagnostic(format!(
            "SETTINGS value outside accepted range at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "{setting_name} ({setting_identifier}) declared {observed_value}; accepted range is {accepted_min_value}..{accepted_max_value}."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Peer limit provenance: {provenance}.")));
        Some(diagnostic)
    }

    fn project_size_limit_rule(
        &self,
        observed_key: &str,
        allowed_key: &str,
        message: &str,
        observed_label: &str,
    ) -> Option<Diagnostic> {
        let observed_size = self.number(observed_key)?;
        let allowed_size = self.number(allowed_key)?;
        let frame_kind = self.number("frame_kind")?;
        let frame = self.frame_ref()?;
        let limit_provenance = self.string("receive_limit_provenance")?;
        let rule_provenance = self.string("rule_provenance")?;
        let peer_outbound = limit_provenance == "peer_settings_item";
        let message = if peer_outbound {
            "header list size exceeds peer-advertised outbound maximum"
        } else {
            message
        };
        let maximum_label = if peer_outbound {
            "active peer-advertised outbound maximum"
        } else {
            "active receive maximum"
        };
        let provenance_label = if peer_outbound {
            "Peer setting provenance"
        } else {
            "Receive limit provenance"
        };
        let mut diagnostic =
            self.diagnostic(format!("{message} at byte offset {}", self.byte_offset));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} {observed_label} {observed_size}; {maximum_label} is {allowed_size}.",
            frame.stream_ref, frame.stream_id
        )));
        diagnostic.related.push(note_json(format!(
            "{provenance_label}: {limit_provenance}."
        )));
        diagnostic
            .related
            .push(note_json(format!("Rule provenance: {rule_provenance}.")));
        push_byte_preview_note(&mut diagnostic, self.entries);
        Some(diagnostic)
    }

    fn project_hpack_fixture_rule(&self) -> Option<Diagnostic> {
        let message = match self.id.as_str() {
            "hpack.fixture.unsupported_header_block" => "unsupported HPACK fixture header block",
            "hpack.fixture.unsupported_static_index" => "unsupported HPACK static index",
            "hpack.static.unsupported_index" => "unsupported HPACK static index",
            "hpack.fixture.malformed_string_length" => "malformed HPACK string length",
            "hpack.fixture.malformed_raw_string_value" => "malformed HPACK raw string value",
            "hpack.fixture.malformed_huffman_padding" => "malformed HPACK Huffman padding",
            "hpack.fixture.huffman_eos_symbol" => "HPACK Huffman EOS used as decoded symbol",
            "hpack.fixture.huffman_non_visible_value" => {
                "HPACK Huffman decoded non-visible header value"
            }
            "hpack.fixture.table_size_update_malformed" => {
                "malformed HPACK table-size update integer"
            }
            "hpack.fixture.dynamic_index_out_of_range" => {
                return self.project_hpack_dynamic_index_rule();
            }
            "hpack.fixture.dynamic_name_continuation_missing"
            | "hpack.fixture.dynamic_name_continuation_malformed"
            | "hpack.fixture.dynamic_name_continuation_out_of_range" => {
                return self.project_hpack_dynamic_name_rule();
            }
            "hpack.fixture.table_size_update_not_at_start"
            | "hpack.fixture.table_size_update_trailing_bytes" => {
                return self.project_hpack_table_size_update_rule();
            }
            _ => return None,
        };
        self.project_hpack_fixture_message(message)
    }

    fn project_hpack_fixture_message(&self, message: &str) -> Option<Diagnostic> {
        let observed_size = self.number("observed_header_block_size")?;
        let observed_first_byte = self.number("observed_first_byte")?;
        let expected_fixture = self.string("expected_fixture")?;
        let codec_module = self.string("codec_module")?;
        let mut diagnostic =
            self.diagnostic(format!("{message} at byte offset {}", self.byte_offset));
        if self.id == "hpack.static.unsupported_index" {
            diagnostic.related.push(note_json(format!(
                "HPACK static decoder `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
            )));
        } else {
            diagnostic.related.push(note_json(format!(
                "HPACK fixture codec `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
            )));
        }
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Expected {expected_fixture}.")));
        Some(diagnostic)
    }

    fn project_hpack_dynamic_index_rule(&self) -> Option<Diagnostic> {
        let observed_size = self.number("observed_header_block_size")?;
        let observed_first_byte = self.number("observed_first_byte")?;
        let requested_index = self.number("requested_dynamic_index")?;
        let entry_count = self.number("dynamic_table_entry_count")?;
        let expected_fixture = self.string("expected_fixture")?;
        let codec_module = self.string("codec_module")?;
        let mut diagnostic = self.diagnostic(format!(
            "HPACK dynamic index out of range at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "HPACK dynamic index {requested_index} was requested, but the fixture dynamic table currently contains {entry_count} entry/entries."
        )));
        diagnostic.related.push(note_json(format!(
            "HPACK fixture codec `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Expected {expected_fixture}.")));
        Some(diagnostic)
    }

    fn project_hpack_dynamic_name_rule(&self) -> Option<Diagnostic> {
        let observed_size = self.number("observed_header_block_size")?;
        let observed_first_byte = self.number("observed_first_byte")?;
        let requested_index = self.number("requested_dynamic_index")?;
        let entry_count = self.number("dynamic_table_entry_count")?;
        let expected_fixture = self.string("expected_fixture")?;
        let codec_module = self.string("codec_module")?;
        let message = match self.id.as_str() {
            "hpack.fixture.dynamic_name_continuation_missing" => {
                "HPACK dynamic-name continuation is missing a fixture table entry"
            }
            "hpack.fixture.dynamic_name_continuation_malformed" => {
                "HPACK dynamic-name continuation is malformed"
            }
            "hpack.fixture.dynamic_name_continuation_out_of_range" => {
                "HPACK dynamic-name continuation is out of range"
            }
            _ => return None,
        };
        let mut diagnostic =
            self.diagnostic(format!("{message} at byte offset {}", self.byte_offset));
        diagnostic.related.push(note_json(format!(
            "HPACK dynamic-name continuation requested dynamic index {requested_index}, and the fixture dynamic table currently contains {entry_count} entry/entries."
        )));
        diagnostic.related.push(note_json(format!(
            "HPACK fixture codec `{codec_module}` observed header block size {observed_size} and first byte {observed_first_byte}."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Expected {expected_fixture}.")));
        Some(diagnostic)
    }

    fn project_hpack_table_size_update_rule(&self) -> Option<Diagnostic> {
        let observed_size = self.number("observed_header_block_size")?;
        let observed_first_byte = self.number("observed_first_byte")?;
        let observed_update_size = self.number("observed_header_table_size")?;
        let frame_kind = self.number("frame_kind")?;
        let frame = self.frame_ref()?;
        let active_state = self.string("active_state")?;
        let expected_fixture = self.string("expected_fixture")?;
        let codec_module = self.string("codec_module")?;
        let message = match self.id.as_str() {
            "hpack.fixture.table_size_update_not_at_start" => {
                "HPACK table-size update appears after a header field"
            }
            "hpack.fixture.table_size_update_trailing_bytes" => {
                "HPACK table-size update leaves trailing bytes"
            }
            _ => return None,
        };
        let fact = match self.id.as_str() {
            "hpack.fixture.table_size_update_not_at_start" => "after a decoded header field",
            "hpack.fixture.table_size_update_trailing_bytes" => {
                "before unexpected trailing header-block bytes"
            }
            _ => return None,
        };
        let mut diagnostic =
            self.diagnostic(format!("{message} at byte offset {}", self.byte_offset));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} requested HPACK header table size {observed_update_size} {fact}.",
            frame.stream_ref, frame.stream_id
        )));
        diagnostic.related.push(note_json(format!(
            "HPACK fixture codec `{codec_module}` observed header block size {observed_size}, first byte {observed_first_byte}, and active state {active_state}."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Expected {expected_fixture}.")));
        Some(diagnostic)
    }

    fn diagnostic(&self, message: String) -> Diagnostic {
        Diagnostic::new(
            self.id.clone(),
            Severity::Error,
            DiagnosticKind::Runtime,
            message,
            None,
            self.source.clone(),
        )
    }

    fn frame_ref(&self) -> Option<ProtocolFrameRef> {
        Some(ProtocolFrameRef {
            stream_id: self.number("stream_id")?,
            stream_ref: self.string("stream_ref")?,
        })
    }

    fn number(&self, key: &str) -> Option<i64> {
        json_number(self.entries, key)
    }

    fn string(&self, key: &str) -> Option<String> {
        json_string(self.entries, key)
    }

    fn push_preview_state_and_provenance(&self, diagnostic: &mut Diagnostic) -> Option<()> {
        push_byte_preview_note(diagnostic, self.entries);
        self.push_state_and_provenance(diagnostic)
    }

    fn push_state_and_provenance(&self, diagnostic: &mut Diagnostic) -> Option<()> {
        self.push_active_state(diagnostic)?;
        self.push_rule_provenance(diagnostic)
    }

    fn push_active_state(&self, diagnostic: &mut Diagnostic) -> Option<()> {
        diagnostic.related.push(note_json(format!(
            "Active protocol state: {}.",
            self.string("active_state")?
        )));
        Some(())
    }

    fn push_rule_provenance(&self, diagnostic: &mut Diagnostic) -> Option<()> {
        diagnostic.related.push(note_json(format!(
            "Rule provenance: {}.",
            self.string("rule_provenance")?
        )));
        Some(())
    }
}

struct ProtocolFrameRef {
    stream_id: i64,
    stream_ref: String,
}

fn protocol_header_list_message(
    subject: &str,
    failed_fact: &str,
    header_name: &str,
    byte_offset: i64,
) -> String {
    match failed_fact {
        "protocol_on_non_connect_request" => format!(
            "{subject} contains :protocol on a non-CONNECT request at byte offset {byte_offset}"
        ),
        "duplicate_protocol_pseudo_header" => {
            format!("{subject} contains duplicate :protocol at byte offset {byte_offset}")
        }
        "protocol_value_empty" => {
            format!("{subject} contains empty :protocol at byte offset {byte_offset}")
        }
        "extended_connect_scheme_missing" => format!(
            "{subject} is missing required extended CONNECT :scheme at byte offset {byte_offset}"
        ),
        "extended_connect_path_missing" => format!(
            "{subject} is missing required extended CONNECT :path at byte offset {byte_offset}"
        ),
        "extended_connect_authority_missing" => format!(
            "{subject} is missing required extended CONNECT :authority at byte offset {byte_offset}"
        ),
        "extended_connect_not_negotiated" => format!(
            "{subject} uses extended CONNECT before negotiation at byte offset {byte_offset}"
        ),
        "connect_authority_missing" => {
            format!("{subject} is missing required CONNECT :authority at byte offset {byte_offset}")
        }
        "connect_authority_empty" => {
            format!("{subject} contains empty CONNECT :authority at byte offset {byte_offset}")
        }
        "connect_scheme_present" => {
            format!("{subject} contains forbidden CONNECT :scheme at byte offset {byte_offset}")
        }
        "connect_path_present" => {
            format!("{subject} contains forbidden CONNECT :path at byte offset {byte_offset}")
        }
        "missing_required_pseudo_header" => {
            format!("{subject} is missing {header_name} at byte offset {byte_offset}")
        }
        "response_only_pseudo_header" => {
            format!("{subject} contains response-only {header_name} at byte offset {byte_offset}")
        }
        "request_only_pseudo_header" => {
            format!("{subject} contains request-only {header_name} at byte offset {byte_offset}")
        }
        "duplicate_pseudo_header" => {
            format!("{subject} contains duplicate {header_name} at byte offset {byte_offset}")
        }
        "trailer_pseudo_header" => {
            format!("{subject} contains pseudo-header {header_name} at byte offset {byte_offset}")
        }
        "pseudo_header_after_regular_header" => format!(
            "{subject} places {header_name} after a regular header at byte offset {byte_offset}"
        ),
        "ordinary_header_name_not_lowercase" => format!(
            "{subject} contains uppercase ordinary header {header_name} at byte offset {byte_offset}"
        ),
        "ordinary_header_name_invalid_token" => format!(
            "{subject} contains invalid ordinary header name {header_name} at byte offset {byte_offset}"
        ),
        "connection_specific_header" => format!(
            "{subject} contains connection-specific header {header_name} at byte offset {byte_offset}"
        ),
        "te_header_value_not_trailers" => {
            format!("{subject} contains te value other than trailers at byte offset {byte_offset}")
        }
        "method_value_empty" => {
            format!("{subject} contains empty :method at byte offset {byte_offset}")
        }
        "scheme_value_not_http_or_https" => format!(
            "{subject} contains :scheme value other than http or https at byte offset {byte_offset}"
        ),
        "path_value_empty" => {
            format!("{subject} contains empty :path at byte offset {byte_offset}")
        }
        "authority_value_invalid" => {
            format!("{subject} contains invalid :authority at byte offset {byte_offset}")
        }
        "content_length_invalid" => {
            format!("{subject} contains invalid content-length at byte offset {byte_offset}")
        }
        "content_length_mismatch" => format!(
            "{subject} contains mismatched content-length values at byte offset {byte_offset}"
        ),
        "switching_protocols_status_forbidden" => {
            format!("{subject} uses switching protocols status at byte offset {byte_offset}")
        }
        "informational_response_end_stream" => {
            format!("informational response ended the stream at byte offset {byte_offset}")
        }
        _ => format!("invalid {subject} at byte offset {byte_offset}"),
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
        "schema.encode_value_unrepresentable" | "codec.encode_value_unrepresentable" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "schema.dispatch_unknown_tag" | "codec.dispatch_unknown_tag" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "schema.dispatch_length_mismatch" | "codec.dispatch_length_mismatch" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "schema.dispatch_mismatch" | "codec.dispatch_mismatch" => {
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
        "schema.encode_value_unrepresentable" | "codec.encode_value_unrepresentable" => {
            "encode value is unrepresentable"
        }
        "schema.dispatch_unknown_tag" | "codec.dispatch_unknown_tag" => {
            "unknown dispatch tag in encode value"
        }
        "schema.dispatch_length_mismatch" | "codec.dispatch_length_mismatch" => {
            "dispatch payload length mismatch"
        }
        "schema.dispatch_mismatch" | "codec.dispatch_mismatch" => {
            "dispatch tag and payload mismatch"
        }
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

fn json_number_display(entries: &[(String, JsonValue)], key: &str) -> Option<String> {
    json_number(entries, key).map(|value| value.to_string())
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
    execution: &JvmExecution,
) -> Result<ExitCode, String> {
    let contract_error_file = build_dir.join("contract-errors.tsv");
    let result_error_file = build_dir.join("result-errors.tsv");
    let transport_error_file = build_dir.join("transport-errors.tsv");
    let event_env = [
        ("VELN_CONTRACT_ERRORS", contract_error_file.as_os_str()),
        ("VELN_RESULT_ERRORS", result_error_file.as_os_str()),
        ("VELN_TRANSPORT_ERRORS", transport_error_file.as_os_str()),
    ];
    let result = prepare_and_run_jvm_capture_with_execution(
        execution, program, "veln run", &event_env, entry_args,
    )?;
    let contract_error_trace = fs::read_to_string(&contract_error_file).unwrap_or_default();
    let result_error_trace = fs::read_to_string(&result_error_file).unwrap_or_default();
    let transport_error_trace = fs::read_to_string(&transport_error_file).unwrap_or_default();

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
            } else if let Some(failure) = transport_failure_from_trace(&transport_error_trace) {
                RunJsonReport::runtime_transport_error(exit_code, stdout, stderr, failure)
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
        let details = invalid_shift_runtime_details(&message)
            .unwrap_or_else(|| JsonValue::object([("phase", JsonValue::string("runtime"))]));
        Self {
            status: "failed",
            exit_code,
            stdout,
            stderr,
            error: Some(RunJsonError::runtime(message, details)),
        }
    }

    fn runtime_transport_error(
        exit_code: i32,
        stdout: String,
        _stderr: String,
        failure: TransportFailureTrace,
    ) -> Self {
        let message = format!(
            "transport {} failed: {}",
            failure.operation.replace('_', " "),
            failure.category
        );
        Self {
            status: "failed",
            exit_code,
            stdout,
            stderr: format!("{message}\n"),
            error: Some(RunJsonError::runtime(message, failure.details())),
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

struct TransportFailureTrace {
    operation: String,
    category: String,
    local_endpoint: Option<String>,
    peer_endpoint: Option<String>,
    listener_id: Option<String>,
    stream_id: Option<String>,
    lifecycle_phase: String,
    input_committed: Option<bool>,
    output_committed: Option<bool>,
    ownership_committed: Option<bool>,
    platform_cause: Option<String>,
}

impl TransportFailureTrace {
    fn details(&self) -> JsonValue {
        let mut entries = vec![
            ("phase".to_string(), JsonValue::string("runtime")),
            (
                "id".to_string(),
                JsonValue::string("runtime.transport_failure"),
            ),
            (
                "operation".to_string(),
                JsonValue::string(self.operation.clone()),
            ),
            (
                "category".to_string(),
                JsonValue::string(self.category.clone()),
            ),
            (
                "lifecycle_phase".to_string(),
                JsonValue::string(self.lifecycle_phase.clone()),
            ),
        ];
        push_optional_string(&mut entries, "local_endpoint", &self.local_endpoint);
        push_optional_string(&mut entries, "peer_endpoint", &self.peer_endpoint);
        push_optional_string(&mut entries, "listener_id", &self.listener_id);
        push_optional_string(&mut entries, "stream_id", &self.stream_id);
        push_optional_bool(&mut entries, "input_committed", self.input_committed);
        push_optional_bool(&mut entries, "output_committed", self.output_committed);
        push_optional_bool(
            &mut entries,
            "ownership_committed",
            self.ownership_committed,
        );
        push_optional_string(&mut entries, "platform_cause", &self.platform_cause);
        JsonValue::Object(entries)
    }
}

fn push_optional_string(entries: &mut Vec<(String, JsonValue)>, key: &str, value: &Option<String>) {
    if let Some(value) = value {
        entries.push((key.to_string(), JsonValue::string(value.clone())));
    }
}

fn push_optional_bool(entries: &mut Vec<(String, JsonValue)>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        entries.push((key.to_string(), JsonValue::Bool(value)));
    }
}

fn transport_failure_from_trace(trace: &str) -> Option<TransportFailureTrace> {
    let fields: Vec<&str> = trace
        .lines()
        .rev()
        .find(|line| line.starts_with("transport\t"))?
        .split('\t')
        .collect();
    if fields.len() != 12 {
        return None;
    }
    Some(TransportFailureTrace {
        operation: trace_string(fields[1])?,
        category: trace_string(fields[2])?,
        local_endpoint: trace_optional_string(fields[3])?,
        peer_endpoint: trace_optional_string(fields[4])?,
        listener_id: trace_optional_string(fields[5])?,
        stream_id: trace_optional_string(fields[6])?,
        lifecycle_phase: trace_string(fields[7])?,
        input_committed: trace_optional_bool(fields[8])?,
        output_committed: trace_optional_bool(fields[9])?,
        ownership_committed: trace_optional_bool(fields[10])?,
        platform_cause: trace_optional_string(fields[11])?,
    })
}

fn trace_optional_string(value: &str) -> Option<Option<String>> {
    if value == "-" {
        Some(None)
    } else {
        trace_string(value).map(Some)
    }
}

fn trace_string(value: &str) -> Option<String> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

fn trace_optional_bool(value: &str) -> Option<Option<bool>> {
    match value {
        "true" => Some(Some(true)),
        "false" => Some(Some(false)),
        "unknown" => Some(None),
        _ => None,
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

    fn runtime(message: String, details: JsonValue) -> Self {
        Self {
            kind: "runtime".to_string(),
            message,
            details,
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

fn invalid_shift_runtime_details(message: &str) -> Option<JsonValue> {
    let count_text = message.strip_prefix("invalid shift count ")?;
    let (count_text, operator_text) = count_text.split_once(" for operator `")?;
    let (operator, suffix) = operator_text.split_once('`')?;
    if suffix != "; expected a value between 0 and 63" {
        return None;
    }
    let count = count_text.parse::<i64>().ok()?;
    Some(JsonValue::object([
        ("phase", JsonValue::string("runtime")),
        ("id", JsonValue::string("runtime.invalid_shift_count")),
        ("operator", JsonValue::string(operator)),
        ("actual_count", JsonValue::Number(count)),
        ("minimum_count", JsonValue::Number(0)),
        ("maximum_count", JsonValue::Number(63)),
    ]))
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
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

    fn trace_hex(value: &str) -> String {
        value
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn transport_failure_trace_projects_known_context_and_commit_facts() {
        let trace = format!(
            "transport\t{}\t{}\t{}\t{}\t-\t{}\t{}\tfalse\tfalse\ttrue\t{}\n",
            trace_hex("shutdown"),
            trace_hex("event_record_failed"),
            trace_hex("127.0.0.1:0"),
            trace_hex("fixture-stream"),
            trace_hex("fixture-stream"),
            trace_hex("write_shutdown"),
            trace_hex("could not record VELN_NET_EVENTS"),
        );

        let failure = transport_failure_from_trace(&trace).expect("transport trace should parse");
        assert_eq!(failure.operation, "shutdown");
        assert_eq!(failure.category, "event_record_failed");
        assert_eq!(failure.local_endpoint.as_deref(), Some("127.0.0.1:0"));
        assert_eq!(failure.peer_endpoint.as_deref(), Some("fixture-stream"));
        assert_eq!(failure.listener_id, None);
        assert_eq!(failure.stream_id.as_deref(), Some("fixture-stream"));
        assert_eq!(failure.lifecycle_phase, "write_shutdown");
        assert_eq!(failure.input_committed, Some(false));
        assert_eq!(failure.output_committed, Some(false));
        assert_eq!(failure.ownership_committed, Some(true));
    }

    #[test]
    fn transport_failure_trace_preserves_unknown_facts_as_absent() {
        let trace = format!(
            "transport\t{}\t{}\t-\t-\t-\t-\t{}\tunknown\tunknown\tunknown\t{}\n",
            trace_hex("connect"),
            trace_hex("io_failure"),
            trace_hex("during_operation"),
            trace_hex("production socket failed"),
        );

        let failure = transport_failure_from_trace(&trace).expect("transport trace should parse");
        assert_eq!(failure.local_endpoint, None);
        assert_eq!(failure.peer_endpoint, None);
        assert_eq!(failure.input_committed, None);
        assert_eq!(failure.output_committed, None);
        assert_eq!(failure.ownership_committed, None);
        let details = failure.details().to_json();
        assert!(!details.contains("local_endpoint"));
        assert!(!details.contains("input_committed"));
    }

    #[test]
    fn run_generation_excludes_companion_sources() {
        let root = temp_dir("run-generation-excludes-companion-sources");
        fs::write(root.join("main.veln"), "pub fn main() -> Int\n\t1\nend\n")
            .expect("production source should be written");
        fs::write(
            root.join("main.test.veln"),
            "pub fn companion_marker() -> Int\n\t2\nend\n",
        )
        .expect("companion source should be written");

        let analysis_inputs =
            production_analysis_inputs(&root, &[]).expect("production inputs should resolve");
        assert_eq!(analysis_inputs.len(), 1);
        assert!(analysis_inputs[0].ends_with("main.veln"));
        let project = Project::discover(root.clone(), &analysis_inputs)
            .expect("production project should discover");
        let analysis = veln_analysis::analyze_project(project, DoctestMode::Exclude);
        assert!(
            analysis.checked_diagnostics().is_empty(),
            "production analysis should exclude companion diagnostics: {:#?}",
            analysis.checked_diagnostics()
        );
        let ir = lower_run_entry(&analysis, "main", None)
            .expect("entry should lower")
            .expect("entry should produce IR");

        let jvm = generate_classfiles_with_entry_arg_types(&ir, "main", &[]);

        assert!(
            jvm.classes
                .iter()
                .any(|class| class.path == "VelnProgram$fn_main.class")
        );
        assert!(
            jvm.classes
                .iter()
                .all(|class| !class.path.contains("companion_marker")),
            "companion function should not be emitted in run classfiles: {:?}",
            jvm.classes
                .iter()
                .map(|class| class.path.as_str())
                .collect::<Vec<_>>()
        );

        fs::remove_dir_all(root).expect("test project should be removed");
    }

    #[test]
    fn run_analysis_timings_write_deterministic_json_lines() {
        let root = temp_dir("run-analysis-timings-json-lines");
        let timing_file = root.join("timings.jsonl");
        let mut timings = RunAnalysisTimings {
            file: timing_file.clone(),
            workload: "http2_core".to_string(),
            run: "new-1".to_string(),
            records: Vec::new(),
        };

        timings.push("source_loading", Duration::from_nanos(250_000_000));
        timings.write().expect("timing records should be written");

        assert_eq!(
            fs::read_to_string(&timing_file).expect("timing file should be readable"),
            "{\"workload\":\"http2_core\",\"run\":\"new-1\",\"stage\":\"source_loading\",\"boundary\":\"source_loading\",\"duration_seconds\":0.250000000}\n"
        );

        fs::remove_dir_all(root).expect("test project should be removed");
    }

    fn byte_preview(data: &str) -> JsonValue {
        byte_preview_with_counts(data, (data.len() / 2) as i64, false)
    }

    fn temp_dir(name: &str) -> PathBuf {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let root = env::temp_dir().join(format!("veln-cli-{name}-{nanos}-{id}"));
        fs::create_dir_all(&root).expect("test directory should be created");
        root
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
    fn byte_result_failure_diagnostic_projects_decode_error_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.invalid_input")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(62)),
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
                "reason",
                JsonValue::string("kind value exceeds declared length"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.kind"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.invalid_input, ByteOffset(62), ManualPacketWire.kind, kind value exceeds declared length)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.invalid_input");
        assert_eq!(diagnostic.message, "decode error at byte offset 62");
        assert_eq!(diagnostic.related.len(), 3);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Decode failure reason: kind value exceeds declared length.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.invalid_input, ByteOffset(62), ManualPacketWire.kind, kind value exceeds declared length).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_checksum_mismatch_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.checksum_mismatch")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(12)),
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
                        ("name", JsonValue::string("checksum")),
                    ]),
                ]),
            ),
            ("expected_checksum", JsonValue::string("0xabcd")),
            ("actual_checksum", JsonValue::string("0x1234")),
            (
                "reason",
                JsonValue::string("payload checksum did not match header checksum"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.checksum"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.checksum_mismatch, ByteOffset(12), ManualPacketWire.checksum, expected_checksum=0xabcd; actual_checksum=0x1234; reason=payload checksum did not match header checksum)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.checksum_mismatch");
        assert_eq!(diagnostic.message, "checksum mismatch at byte offset 12");
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Expected checksum `0xabcd`; actual checksum was `0x1234`.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Checksum failure reason: payload checksum did not match header checksum.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.checksum_mismatch, ByteOffset(12), ManualPacketWire.checksum, expected_checksum=0xabcd; actual_checksum=0x1234; reason=payload checksum did not match header checksum).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_length_mismatch_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.length_mismatch")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(9)),
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
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            ("expected_length", JsonValue::Number(4)),
            ("actual_length", JsonValue::Number(3)),
            (
                "reason",
                JsonValue::string("payload length did not match header length"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.payload"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.length_mismatch, ByteOffset(9), ManualPacketWire.payload, expected_length=4; actual_length=3; reason=payload length did not match header length)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.length_mismatch");
        assert_eq!(diagnostic.message, "length mismatch at byte offset 9");
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Expected length 4; actual length was 3.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Length mismatch reason: payload length did not match header length.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.length_mismatch, ByteOffset(9), ManualPacketWire.payload, expected_length=4; actual_length=3; reason=payload length did not match header length).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_payload_length_mismatch_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.payload_length_mismatch")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(21)),
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
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            ("expected_payload_length", JsonValue::Number(8)),
            ("actual_payload_length", JsonValue::Number(5)),
            (
                "reason",
                JsonValue::string("payload length did not match frame header"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.payload"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.payload_length_mismatch, ByteOffset(21), ManualPacketWire.payload, expected_payload_length=8; actual_payload_length=5; reason=payload length did not match frame header)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.payload_length_mismatch");
        assert_eq!(
            diagnostic.message,
            "payload length mismatch at byte offset 21"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Expected payload length 8; actual payload length was 5.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Payload length mismatch reason: payload length did not match frame header.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.payload_length_mismatch, ByteOffset(21), ManualPacketWire.payload, expected_payload_length=8; actual_payload_length=5; reason=payload length did not match frame header).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_padding_mismatch_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.padding_mismatch")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(24)),
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
                        ("name", JsonValue::string("padding")),
                    ]),
                ]),
            ),
            ("expected_padding_length", JsonValue::Number(2)),
            ("actual_padding_length", JsonValue::Number(5)),
            (
                "reason",
                JsonValue::string("DATA padding did not match payload boundary"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.padding"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.padding_mismatch, ByteOffset(24), ManualPacketWire.padding, expected_padding_length=2; actual_padding_length=5; reason=DATA padding did not match payload boundary)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.padding_mismatch");
        assert_eq!(diagnostic.message, "padding mismatch at byte offset 24");
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Expected padding length 2; actual padding length was 5.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Padding mismatch reason: DATA padding did not match payload boundary.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.padding_mismatch, ByteOffset(24), ManualPacketWire.padding, expected_padding_length=2; actual_padding_length=5; reason=DATA padding did not match payload boundary).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_integer_out_of_range_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.integer_out_of_range")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(17)),
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
                        ("name", JsonValue::string("stream_id")),
                    ]),
                ]),
            ),
            ("byte_width", JsonValue::Number(4)),
            ("min_value", JsonValue::Number(0)),
            ("max_value", JsonValue::Number(2147483647)),
            ("actual_value", JsonValue::Number(2147483648)),
            (
                "reason",
                JsonValue::string("decoded value exceeds signed integer range"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.stream_id"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.integer_out_of_range, ByteOffset(17), ManualPacketWire.stream_id, byte_width=4; min_value=0; max_value=2147483647; actual_value=2147483648; reason=decoded value exceeds signed integer range)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.integer_out_of_range");
        assert_eq!(diagnostic.message, "integer out of range at byte offset 17");
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"4-byte integer expected value between 0 and 2147483647; actual value was 2147483648.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Integer conversion reason: decoded value exceeds signed integer range.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.integer_out_of_range, ByteOffset(17), ManualPacketWire.stream_id, byte_width=4; min_value=0; max_value=2147483647; actual_value=2147483648; reason=decoded value exceeds signed integer range).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_sequence_mismatch_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.sequence_mismatch")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(13)),
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
                        ("name", JsonValue::string("sequence")),
                    ]),
                ]),
            ),
            (
                "expected_sequence",
                JsonValue::string("client_preface,settings"),
            ),
            ("actual_sequence", JsonValue::string("settings")),
            (
                "reason",
                JsonValue::string("frame sequence violated protocol state"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.sequence"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.sequence_mismatch, ByteOffset(13), ManualPacketWire.sequence, expected_sequence=client_preface,settings; actual_sequence=settings; reason=frame sequence violated protocol state)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.sequence_mismatch");
        assert_eq!(diagnostic.message, "sequence mismatch at byte offset 13");
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Expected sequence `client_preface,settings`; actual sequence was `settings`.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Sequence mismatch reason: frame sequence violated protocol state.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.sequence_mismatch, ByteOffset(13), ManualPacketWire.sequence, expected_sequence=client_preface,settings; actual_sequence=settings; reason=frame sequence violated protocol state).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_tag_mismatch_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.tag_mismatch")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(14)),
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
            ("expected_tag", JsonValue::string("DATA")),
            ("actual_tag", JsonValue::string("HEADERS")),
            (
                "reason",
                JsonValue::string("dispatch tag did not match selected payload"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.kind"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.tag_mismatch, ByteOffset(14), ManualPacketWire.kind, expected_tag=DATA; actual_tag=HEADERS; reason=dispatch tag did not match selected payload)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.tag_mismatch");
        assert_eq!(diagnostic.message, "tag mismatch at byte offset 14");
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Expected tag `DATA`; actual tag was `HEADERS`.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Tag mismatch reason: dispatch tag did not match selected payload.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.tag_mismatch, ByteOffset(14), ManualPacketWire.kind, expected_tag=DATA; actual_tag=HEADERS; reason=dispatch tag did not match selected payload).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_magic_mismatch_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.magic_mismatch")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(18)),
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
                        ("name", JsonValue::string("magic")),
                    ]),
                ]),
            ),
            ("expected_magic", JsonValue::string("VELN")),
            ("actual_magic", JsonValue::string("VEIN")),
            (
                "reason",
                JsonValue::string("file magic did not match expected signature"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.magic"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.magic_mismatch, ByteOffset(18), ManualPacketWire.magic, expected_magic=VELN; actual_magic=VEIN; reason=file magic did not match expected signature)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.magic_mismatch");
        assert_eq!(diagnostic.message, "magic mismatch at byte offset 18");
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Expected magic `VELN`; actual magic was `VEIN`.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Magic mismatch reason: file magic did not match expected signature.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.magic_mismatch, ByteOffset(18), ManualPacketWire.magic, expected_magic=VELN; actual_magic=VEIN; reason=file magic did not match expected signature).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_unsupported_feature_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.unsupported_feature")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(27)),
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
                        ("name", JsonValue::string("extension")),
                    ]),
                ]),
            ),
            (
                "unsupported_feature",
                JsonValue::string("dynamic_table_size_update"),
            ),
            (
                "reason",
                JsonValue::string("dynamic table size updates are disabled for this profile"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.extension"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.unsupported_feature, ByteOffset(27), ManualPacketWire.extension, feature=dynamic_table_size_update; reason=dynamic table size updates are disabled for this profile)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.unsupported_feature");
        assert_eq!(
            diagnostic.message,
            "unsupported feature failed at byte offset 27"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Unsupported feature: `dynamic_table_size_update`.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Unsupported feature reason: dynamic table size updates are disabled for this profile.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.unsupported_feature, ByteOffset(27), ManualPacketWire.extension, feature=dynamic_table_size_update; reason=dynamic table size updates are disabled for this profile).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_trailing_input_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.trailing_input")),
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
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            ("consumed_count", JsonValue::Number(5)),
            ("available_count", JsonValue::Number(8)),
            ("remaining_count", JsonValue::Number(3)),
            (
                "reason",
                JsonValue::string("packet decoder completed before the bounded input ended"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.payload"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.trailing_input, ByteOffset(5), ManualPacketWire.payload, consumed_count=5; available_count=8; remaining_count=3; reason=packet decoder completed before the bounded input ended)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.trailing_input");
        assert_eq!(diagnostic.message, "trailing input at byte offset 5");
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Consumed 5 of 8 available bytes; 3 bytes remain.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Trailing input reason: packet decoder completed before the bounded input ended.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.trailing_input, ByteOffset(5), ManualPacketWire.payload, consumed_count=5; available_count=8; remaining_count=3; reason=packet decoder completed before the bounded input ended).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_preserves_plain_trailing_input_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.trailing_input")),
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
                        ("name", JsonValue::string("ManualPacketWire")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            (
                "reason",
                JsonValue::string("bounded input has trailing bytes"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.payload"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.trailing_input, ByteOffset(7), ManualPacketWire.payload, bounded input has trailing bytes)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.trailing_input");
        assert_eq!(diagnostic.message, "trailing input at byte offset 7");
        assert_eq!(diagnostic.related.len(), 3);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Trailing input reason: bounded input has trailing bytes.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.trailing_input, ByteOffset(7), ManualPacketWire.payload, bounded input has trailing bytes).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_version_mismatch_reason() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.version_mismatch")),
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
                        ("name", JsonValue::string("ManualPacketWire")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("field")),
                        ("name", JsonValue::string("version")),
                    ]),
                ]),
            ),
            ("expected_version", JsonValue::string("2")),
            ("actual_version", JsonValue::string("1")),
            (
                "reason",
                JsonValue::string("codec version is not supported"),
            ),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.version"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.version_mismatch, ByteOffset(3), ManualPacketWire.version, expected_version=2; actual_version=1; reason=codec version is not supported)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.version_mismatch");
        assert_eq!(diagnostic.message, "version mismatch at byte offset 3");
        assert_eq!(diagnostic.related.len(), 4);
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"Expected version `2`; actual version was `1`.\"}"
        );
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Version mismatch reason: codec version is not supported.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.version_mismatch, ByteOffset(3), ManualPacketWire.version, expected_version=2; actual_version=1; reason=codec version is not supported).\"}"
        );
    }

    #[test]
    fn byte_result_failure_diagnostic_projects_decode_error_byte_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.invalid_input")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(62)),
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
                        ("name", JsonValue::string("payload")),
                    ]),
                ]),
            ),
            (
                "reason",
                JsonValue::string("byte view range exceeds view length"),
            ),
            ("local_byte_offset", JsonValue::Number(2)),
            ("expected_count", JsonValue::Number(4)),
            ("available_count", JsonValue::Number(1)),
            ("byte_preview", byte_preview("07")),
            (
                "field_path_display",
                JsonValue::string("ManualPacketWire.payload"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "DecodeErrorWithReason(codec.invalid_input, ByteOffset(62), ManualPacketWire.payload, byte view range exceeds view length)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.invalid_input");
        assert_eq!(diagnostic.message, "decode error at byte offset 62");
        assert_eq!(diagnostic.related.len(), 6);
        assert_eq!(
            diagnostic.related[2].to_json(),
            "{\"message\":\"Local byte offset: 2.\"}"
        );
        assert_eq!(
            diagnostic.related[3].to_json(),
            "{\"message\":\"Decoder expected 4 byte(s); 1 byte(s) were available.\"}"
        );
        assert_eq!(
            diagnostic.related[4].to_json(),
            "{\"message\":\"Nearby bytes: 07 (showing 1 of 1 byte(s), complete).\"}"
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
    fn byte_result_failure_diagnostic_projects_decode_need_end_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("codec.incomplete_input")),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(0)),
                ]),
            ),
            ("field_path", JsonValue::array([])),
            ("readiness", JsonValue::string("need_end")),
        ]);
        let failure = TestFailure::result_with_details(
            "NeedMore(NeedEnd)".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "codec.incomplete_input");
        assert_eq!(diagnostic.message, "incomplete input at byte offset 0");
        assert_eq!(diagnostic.related.len(), 2);
        assert_eq!(
            diagnostic.related[0].to_json(),
            "{\"message\":\"Decode readiness is `need_end` because input is closed.\"}"
        );
        assert_eq!(
            diagnostic.related[1].to_json(),
            "{\"message\":\"DecodeStep value: NeedMore(NeedEnd).\"}"
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
    fn byte_result_failure_diagnostic_projects_length_multiple_mismatch_context() {
        let byte_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("byte_diagnostic")),
            ("id", JsonValue::string("schema.length_multiple_mismatch")),
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
            ("observed_count", JsonValue::Number(5)),
            ("required_multiple", JsonValue::Number(2)),
            ("multiple_operand", JsonValue::string("frame_count")),
            ("byte_preview", byte_preview("0502aabbccddee")),
        ]);
        let failure = TestFailure::result_with_details(
            "payload length multiple mismatch at byte offset 2".to_string(),
            None,
            Some(byte_diagnostic),
            None,
        );

        let diagnostic =
            byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

        assert_eq!(diagnostic.id, "schema.length_multiple_mismatch");
        assert_eq!(
            diagnostic.message,
            "payload length multiple mismatch at byte offset 2"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Payload count 5 must be a multiple of `frame_count` value 2")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("05 02 aa bb cc dd ee (showing 7 of 7 byte(s), complete)")
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
                JsonValue::string("schema.encode_value_unrepresentable"),
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
            message: "runtime result failure: Err(EncodeError(schema.encode_value_unrepresentable, PacketWire.payload, byte view count 3 does not match length field `length` value 2))".to_string(),
            details: JsonValue::object([
                ("kind", JsonValue::string("result")),
                ("phase", JsonValue::string("runtime")),
                (
                    "value",
                    JsonValue::string("EncodeError(schema.encode_value_unrepresentable, PacketWire.payload, byte view count 3 does not match length field `length` value 2)"),
                ),
                ("value_diagnostic", value_diagnostic),
            ]),
        };

        let diagnostic =
            value_result_failure_diagnostic(&failure).expect("value diagnostic should project");

        assert_eq!(diagnostic.id, "schema.encode_value_unrepresentable");
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
            ("expected_stream_id", JsonValue::Number(0)),
            ("started_frame_kind", JsonValue::Number(0)),
            ("started_byte_offset", JsonValue::Number(0)),
            ("accumulated_header_block_bytes", JsonValue::Number(0)),
            ("rule_provenance", JsonValue::string("none")),
            ("byte_preview", byte_preview("01020304")),
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
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("4 byte(s) remained undecoded")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("01 02 03 04 (showing 4 of 4 byte(s), complete)")
        );
        assert!(diagnostic.related[2].to_json().contains("none"));
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
            ("accumulated_header_block_bytes", JsonValue::Number(3)),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_continuation_sequence"),
            ),
            (
                "byte_preview",
                byte_preview_with_counts("0000000000000000", 9, true),
            ),
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
        assert_eq!(diagnostic.related.len(), 4);
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
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("00 00 00 00 00 00 00 00 (showing 8 of 9 byte(s), truncated)")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_continuation_sequence")
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
            (
                "byte_preview",
                byte_preview_with_counts("0000000000000000", 9, true),
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
        assert_eq!(diagnostic.related.len(), 3);
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
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("showing 8 of 9 byte(s), truncated")
        );
        assert!(diagnostic.related[2].to_json().contains("protocol_default"));
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
            ("byte_preview", byte_preview("01020304")),
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
        assert_eq!(diagnostic.related.len(), 4);
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
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("01 02 03 04 (showing 4 of 4 byte(s), complete)")
        );
        assert!(diagnostic.related[2].to_json().contains("open-stream"));
        assert!(
            diagnostic.related[3]
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
            (
                "current_open_peer_created_stream_count",
                JsonValue::Number(1),
            ),
            ("attempted_concurrent_stream_count", JsonValue::Number(2)),
            ("allowed_concurrent_stream_count", JsonValue::Number(1)),
            ("endpoint_role", JsonValue::string("server")),
            ("active_state", JsonValue::string("open-stream")),
            (
                "receive_limit_provenance",
                JsonValue::string("local_configuration"),
            ),
            (
                "rule_provenance",
                JsonValue::string("peer_created_stream_receive_limit"),
            ),
            (
                "byte_preview",
                byte_preview_with_counts("0000000104000000", 9, true),
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
        assert_eq!(diagnostic.related.len(), 6);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("make 2 concurrent peer-created stream(s)")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("1 peer-created stream(s) are currently open")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("active receive limit is 1")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 00 00 01 04 00 00 00 (showing 8 of 9 byte(s), truncated)")
        );
        assert!(diagnostic.related[2].to_json().contains("open-stream"));
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("Endpoint role: server")
        );
        assert!(
            diagnostic.related[4]
                .to_json()
                .contains("local_configuration")
        );
        assert!(
            diagnostic.related[5]
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
            ("byte_preview", byte_preview("828486")),
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
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Frame kind 9 on stream 1")
        );
        assert!(diagnostic.related[0].to_json().contains(":scheme,:path"));
        assert!(diagnostic.related[1].to_json().contains("82 84 86"));
        assert!(
            diagnostic.related[3]
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
            ("byte_preview", byte_preview("88")),
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
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("Frame kind 9 on stream 1")
        );
        assert!(diagnostic.related[0].to_json().contains("server"));
        assert!(diagnostic.related[1].to_json().contains("88"));
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_response_pseudo_headers")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_response_trailer_list_context() {
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
                JsonValue::string("trailer_pseudo_header"),
            ),
            ("header_name", JsonValue::string(":status")),
            ("decoded_header_names", JsonValue::string(":status")),
            ("byte_preview", byte_preview("88")),
            ("active_state", JsonValue::string("response-trailers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_trailer_pseudo_headers"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 response trailer list contains pseudo-header :status at byte offset 12"
                .to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.invalid_response_header_list");
        assert_eq!(
            diagnostic.message,
            "response trailer list contains pseudo-header :status at byte offset 12"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("decoded response trailer names: :status")
        );
        assert!(diagnostic.related[1].to_json().contains("88"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("response-trailers")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_trailer_pseudo_headers")
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
    fn protocol_result_failure_diagnostic_projects_empty_request_path_value() {
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
            ("failed_header_fact", JsonValue::string("path_value_empty")),
            ("header_name", JsonValue::string(":path")),
            (
                "decoded_header_names",
                JsonValue::string(":method,:scheme,:path"),
            ),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_request_pseudo_headers"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list contains empty :path at byte offset 12".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "request header list contains empty :path at byte offset 12"
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains(":method,:scheme,:path")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_request_pseudo_headers")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_empty_request_method_value() {
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
                JsonValue::string("method_value_empty"),
            ),
            ("header_name", JsonValue::string(":method")),
            (
                "decoded_header_names",
                JsonValue::string(":method,:scheme,:path"),
            ),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_request_pseudo_headers"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list contains empty :method at byte offset 12".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "request header list contains empty :method at byte offset 12"
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains(":method,:scheme,:path")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_request_pseudo_headers")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_request_authority_value() {
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
                JsonValue::string("authority_value_invalid"),
            ),
            ("header_name", JsonValue::string(":authority")),
            (
                "decoded_header_names",
                JsonValue::string(":method,:scheme,:path,:authority"),
            ),
            ("active_state", JsonValue::string("request-headers")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_request_authority"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 request header list contains invalid :authority at byte offset 12".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.message,
            "request header list contains invalid :authority at byte offset 12"
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains(":method,:scheme,:path,:authority")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("rfc9113_request_authority")
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
    fn protocol_result_failure_diagnostic_projects_hpack_table_size_malformed_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.table_size_update_malformed"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(77)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(2)),
            ("observed_first_byte", JsonValue::Number(63)),
            (
                "expected_fixture",
                JsonValue::string("fixture HPACK malformed table-size update integer"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("3f80")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK fixture malformed table-size update integer at byte offset 77".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "hpack.fixture.table_size_update_malformed");
        assert_eq!(
            diagnostic.message,
            "malformed HPACK table-size update integer at byte offset 77"
        );
        assert_eq!(diagnostic.related.len(), 3);
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("3f 80 (showing 2 of 2 byte(s), complete)")
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
    fn protocol_result_failure_diagnostic_projects_hpack_dynamic_index_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.dynamic_index_out_of_range"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(27)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(1)),
            ("observed_first_byte", JsonValue::Number(190)),
            ("requested_dynamic_index", JsonValue::Number(0)),
            ("dynamic_table_entry_count", JsonValue::Number(0)),
            (
                "expected_fixture",
                JsonValue::string("fixture dynamic indexed header"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("be")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK dynamic index out of range at byte offset 27".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "hpack.fixture.dynamic_index_out_of_range");
        assert_eq!(
            diagnostic.message,
            "HPACK dynamic index out of range at byte offset 27"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(diagnostic.related[0].to_json().contains("dynamic index 0"));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("header block size 1")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("be (showing 1 of 1 byte(s), complete)")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_hpack_dynamic_name_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.dynamic_name_continuation_out_of_range"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(98)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(8)),
            ("observed_first_byte", JsonValue::Number(127)),
            ("requested_dynamic_index", JsonValue::Number(3)),
            ("dynamic_table_entry_count", JsonValue::Number(3)),
            (
                "expected_fixture",
                JsonValue::string("fixture dynamic-name continuation range"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("7f02055041544348")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK dynamic-name continuation out of range at byte offset 98".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "hpack.fixture.dynamic_name_continuation_out_of_range"
        );
        assert_eq!(
            diagnostic.message,
            "HPACK dynamic-name continuation is out of range at byte offset 98"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(diagnostic.related[0].to_json().contains("dynamic index 3"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("7f 02 05 50 41 54 43 48")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_hpack_table_size_update_placement_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.table_size_update_not_at_start"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(10)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(2)),
            ("observed_first_byte", JsonValue::Number(62)),
            ("observed_header_table_size", JsonValue::Number(30)),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            ("active_state", JsonValue::string("hpack-fixture")),
            (
                "expected_fixture",
                JsonValue::string("fixture HPACK table-size update at header block start"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("823e")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK fixture table-size update after header field at byte offset 10".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "hpack.fixture.table_size_update_not_at_start"
        );
        assert_eq!(
            diagnostic.message,
            "HPACK table-size update appears after a header field at byte offset 10"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("requested HPACK header table size 30")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("active state hpack-fixture")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("82 3e (showing 2 of 2 byte(s), complete)")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("fixture HPACK table-size update at header block start")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_hpack_table_size_trailing_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("hpack.fixture.table_size_update_trailing_bytes"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(80)),
                ]),
            ),
            ("observed_header_block_size", JsonValue::Number(3)),
            ("observed_first_byte", JsonValue::Number(63)),
            ("observed_header_table_size", JsonValue::Number(33)),
            ("frame_kind", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(1)),
            ("stream_ref", JsonValue::string("stream")),
            ("active_state", JsonValue::string("hpack-fixture")),
            (
                "expected_fixture",
                JsonValue::string("fixture HPACK table-size update without trailing bytes"),
            ),
            ("codec_module", JsonValue::string("hpack_fixture")),
            ("byte_preview", byte_preview("3f0200")),
        ]);
        let failure = TestFailure::result_with_details(
            "HPACK fixture table-size update has trailing bytes at byte offset 80".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "hpack.fixture.table_size_update_trailing_bytes"
        );
        assert_eq!(
            diagnostic.message,
            "HPACK table-size update leaves trailing bytes at byte offset 80"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("before unexpected trailing header-block bytes")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("3f 02 00 (showing 3 of 3 byte(s), complete)")
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
    fn protocol_result_failure_diagnostic_projects_initial_peer_settings_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.initial_peer_settings_required"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(24)),
                ]),
            ),
            ("actual_frame_kind", JsonValue::Number(6)),
            ("actual_flags", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(0)),
            ("stream_ref", JsonValue::string("connection")),
            ("endpoint_role", JsonValue::string("server")),
            (
                "byte_preview",
                byte_preview_with_counts("0000000601000000", 9, true),
            ),
            (
                "active_state",
                JsonValue::string("expect-initial-peer-settings"),
            ),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_initial_peer_frame_requires_non_ack_settings"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 initial peer frame must be non-ACK SETTINGS at byte offset 24".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("initial peer SETTINGS diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "http2.protocol.initial_peer_settings_required"
        );
        assert_eq!(
            diagnostic.message,
            "initial peer frame must be non-ACK SETTINGS at byte offset 24"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(diagnostic.related[0].to_json().contains("flags 1"));
        assert!(diagnostic.related[0].to_json().contains("server endpoint"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("expect-initial-peer-settings")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_rejects_incomplete_frame_identity_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.initial_peer_settings_required"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(24)),
                ]),
            ),
            ("actual_frame_kind", JsonValue::Number(6)),
            ("actual_flags", JsonValue::Number(1)),
            ("stream_id", JsonValue::Number(0)),
            ("stream_ref", JsonValue::string("connection")),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 initial peer frame context is incomplete".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        assert!(protocol_result_failure_diagnostic(&failure).is_none());
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
    fn protocol_result_failure_diagnostic_projects_peer_stream_ordering_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.peer_stream_id_not_increasing"),
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
            ("previous_peer_stream_id", JsonValue::Number(5)),
            ("endpoint_role", JsonValue::string("server")),
            (
                "byte_preview",
                byte_preview_with_counts("0000000104000000", 9, true),
            ),
            ("active_state", JsonValue::string("idle-stream")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_peer_stream_ids_increase"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 peer-created stream id is not increasing".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "http2.protocol.peer_stream_id_not_increasing"
        );
        assert_eq!(
            diagnostic.message,
            "peer-created stream id 3 is not greater than 5 at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 5);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("server endpoint attempted to create idle stream 3")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 00 00 01 04 00 00 00")
        );
        assert!(diagnostic.related[2].to_json().contains("idle-stream"));
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_peer_stream_ids_increase")
        );
        assert!(diagnostic.related[4].to_json().contains("greater than 5"));
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
    fn protocol_result_failure_diagnostic_projects_content_length_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.content_length_mismatch"),
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
            ("expected_content_length", JsonValue::Number(5)),
            ("observed_body_length", JsonValue::Number(3)),
            ("byte_preview", byte_preview("aabbcc")),
            ("active_state", JsonValue::string("open-stream")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_content_length_body"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 content-length body length mismatch at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.content_length_mismatch");
        assert_eq!(
            diagnostic.message,
            "content-length body length mismatch at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("observed 3 DATA application byte(s)")
        );
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("accepted content-length is 5 byte(s)")
        );
        assert!(diagnostic.related[1].to_json().contains("aa bb cc"));
        assert!(diagnostic.related[2].to_json().contains("open-stream"));
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9113_content_length_body")
        );
    }

    #[test]
    fn protocol_result_failure_diagnostic_projects_no_content_response_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.content_length_mismatch"),
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
            ("expected_content_length", JsonValue::Number(0)),
            ("observed_body_length", JsonValue::Number(3)),
            ("byte_preview", byte_preview("aabbcc")),
            ("active_state", JsonValue::string("no-content-response-204")),
            (
                "rule_provenance",
                JsonValue::string("rfc9110_no_content_response_body"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 response status 204 prohibits nonempty DATA at byte offset 9".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(diagnostic.id, "http2.protocol.content_length_mismatch");
        assert_eq!(
            diagnostic.message,
            "response status 204 prohibits nonempty DATA at byte offset 9"
        );
        assert_eq!(diagnostic.related.len(), 4);
        assert!(
            diagnostic.related[0]
                .to_json()
                .contains("response status 204 permits no application content")
        );
        assert!(diagnostic.related[1].to_json().contains("aa bb cc"));
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("no-content-response-204")
        );
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("rfc9110_no_content_response_body")
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
    fn protocol_result_failure_diagnostic_projects_settings_endpoint_role_context() {
        let protocol_diagnostic = JsonValue::object([
            ("kind", JsonValue::string("protocol_diagnostic")),
            (
                "id",
                JsonValue::string("http2.protocol.settings_not_allowed_for_endpoint"),
            ),
            (
                "byte_offset",
                JsonValue::object([
                    ("kind", JsonValue::string("ByteOffset")),
                    ("value", JsonValue::Number(15)),
                ]),
            ),
            ("setting_identifier", JsonValue::Number(2)),
            ("setting_name", JsonValue::string("SETTINGS_ENABLE_PUSH")),
            ("endpoint_role", JsonValue::string("client")),
            ("frame_kind", JsonValue::Number(4)),
            ("stream_id", JsonValue::Number(0)),
            ("stream_ref", JsonValue::string("connection")),
            ("byte_preview", byte_preview("000200000001")),
            ("active_state", JsonValue::string("peer-settings")),
            (
                "rule_provenance",
                JsonValue::string("rfc9113_client_must_not_receive_settings_enable_push"),
            ),
        ]);
        let failure = TestFailure::result_with_details(
            "HTTP/2 SETTINGS item is not allowed for endpoint role at byte offset 15".to_string(),
            None,
            None,
            Some(protocol_diagnostic),
        );

        let diagnostic = protocol_result_failure_diagnostic(&failure)
            .expect("protocol diagnostic should project");

        assert_eq!(
            diagnostic.id,
            "http2.protocol.settings_not_allowed_for_endpoint"
        );
        assert_eq!(
            diagnostic.message,
            "SETTINGS_ENABLE_PUSH is not allowed for client endpoints at byte offset 15"
        );
        assert_eq!(diagnostic.related.len(), 5);
        assert!(diagnostic.related[0].to_json().contains("(2)"));
        assert!(diagnostic.related[0].to_json().contains("frame kind 4"));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("00 02 00 00 00 01")
        );
        assert!(diagnostic.related[2].to_json().contains("client"));
        assert!(diagnostic.related[3].to_json().contains("peer-settings"));
        assert!(
            diagnostic.related[4]
                .to_json()
                .contains("rfc9113_client_must_not_receive_settings_enable_push")
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
            (
                "byte_preview",
                byte_preview_with_counts("0000000104000000", 9, true),
            ),
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
        assert_eq!(diagnostic.related.len(), 6);
        assert!(diagnostic.related[0].to_json().contains("stream 7"));
        assert!(diagnostic.related[0].to_json().contains("last stream id 5"));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("graceful_shutdown")
        );
        assert!(diagnostic.related[2].to_json().contains("server"));
        assert!(
            diagnostic.related[3]
                .to_json()
                .contains("00 00 00 01 04 00 00 00 (showing 8 of 9 byte(s), truncated)")
        );
        assert!(
            diagnostic.related[5]
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
