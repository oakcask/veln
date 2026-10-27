use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use veln_analysis::{
    AnalysisTiming, DoctestMode, ProjectAnalysis, analyze_project, analyze_project_with_timings,
};
use veln_ast::{FunctionKind, InvalidName, NameClass, NameOccurrence};
use veln_backend_jvm::{EntryArgType, generate_classfiles_with_entry_arg_types};
use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity};
use veln_project::{Project, explicit_companion_inputs, production_analysis_inputs};
use veln_test::TestFailure;

use crate::diagnostics::{
    harness_source_diagnostic_artifact_requested, has_error, print_human_stderr, tool_info,
    write_harness_source_diagnostic_artifact,
};
use crate::java::{
    JvmExecution, JvmExecutionPreparation, create_build_dir,
    prepare_and_run_jvm_capture_with_execution, prepare_jvm_execution,
};

use super::run_report::{RunHumanTraceFiles, RunJsonReport, RunJsonTraceFiles};

mod byte_diagnostics;
mod diagnostic_details;
mod entry_args;
mod protocol_diagnostics;
mod value_diagnostics;

use byte_diagnostics::byte_result_failure_diagnostic;
use diagnostic_details::*;
use entry_args::checked_entry;
use protocol_diagnostics::protocol_result_failure_diagnostic;
use value_diagnostics::value_result_failure_diagnostic;

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
    let Some(prepared) = prepare_run_program(
        start.package_root,
        &inputs,
        &entry,
        &entry_args,
        json,
        &mut timings,
    )?
    else {
        return Ok(ExitCode::from(1));
    };

    let backend_start = timings.is_some().then(Instant::now);
    let jvm = generate_classfiles_with_entry_arg_types(
        &prepared.ir,
        &prepared.entry_target,
        &prepared.entry_arg_types,
    );
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

struct PreparedRun {
    ir: veln_ir::TypedProgram,
    entry_target: String,
    entry_arg_types: Vec<EntryArgType>,
}

fn prepare_run_program(
    root: PathBuf,
    inputs: &[PathBuf],
    entry: &str,
    entry_args: &[String],
    json: bool,
    timings: &mut Option<RunAnalysisTimings>,
) -> Result<Option<PreparedRun>, String> {
    let analysis = analyze_run_project(root, inputs, timings.as_mut())?;
    write_harness_source_diagnostic_artifact(&analysis.checked_diagnostics())?;
    if report_source_errors(json, &analysis)? {
        write_timings(timings)?;
        return Ok(None);
    }

    let Some(checked_entry) = checked_entry(&analysis, entry, entry_args) else {
        write_timings(timings)?;
        return Ok(None);
    };
    let Some(lowered_entry) = lower_run_entry(
        json,
        &analysis,
        entry,
        &checked_entry.function,
        timings.as_mut(),
    )?
    else {
        write_timings(timings)?;
        return Ok(None);
    };
    Ok(Some(PreparedRun {
        ir: lowered_entry.ir,
        entry_target: lowered_entry.target,
        entry_arg_types: checked_entry.arg_types,
    }))
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

fn report_source_errors(json: bool, analysis: &ProjectAnalysis) -> Result<bool, String> {
    let mut diagnostics = analysis.source_diagnostics();
    diagnostics.extend(source_module_header_invalid_case_diagnostics(analysis));
    if has_error(&diagnostics) {
        report_pre_execution_diagnostics(json, diagnostics)?;
        return Ok(true);
    }
    Ok(false)
}

fn source_module_header_invalid_case_diagnostics(analysis: &ProjectAnalysis) -> Vec<Diagnostic> {
    analysis
        .module
        .invalid_names
        .iter()
        .filter(|invalid| {
            invalid.class == NameClass::Module && invalid.occurrence == NameOccurrence::Declaration
        })
        .map(source_module_header_invalid_case_diagnostic)
        .collect()
}

fn source_module_header_invalid_case_diagnostic(invalid: &InvalidName) -> Diagnostic {
    Diagnostic::new(
        "name.invalid_case",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "module name `{}` must start with an ASCII lowercase letter",
            invalid.name
        ),
        Some(invalid.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("origin", JsonValue::string("source")),
            ("occurrence", JsonValue::string("declaration")),
            ("name", JsonValue::string(invalid.name.clone())),
            ("name_class", JsonValue::string("module")),
            ("required_initial", JsonValue::string("ascii_lowercase")),
            (
                "observed_initial",
                JsonValue::string(observed_module_initial(&invalid.name)),
            ),
        ]),
    )
}

fn observed_module_initial(name: &str) -> &'static str {
    name.as_bytes().first().map_or("other", |initial| {
        if initial.is_ascii_uppercase() {
            "ascii_uppercase"
        } else if initial.is_ascii_lowercase() {
            "ascii_lowercase"
        } else if *initial == b'_' {
            "underscore"
        } else {
            "other"
        }
    })
}

struct LoweredRunEntry {
    ir: veln_ir::TypedProgram,
    target: String,
}

fn lower_run_entry(
    json: bool,
    analysis: &ProjectAnalysis,
    entry: &str,
    entry_function: &veln_ast::Function,
    timings: Option<&mut RunAnalysisTimings>,
) -> Result<Option<LoweredRunEntry>, String> {
    let reachable = if let Some(timings) = timings {
        let (reachable, timing) = analysis.lower_reachable_function_with_timing(entry_function);
        timings.push_analysis(timing);
        reachable
    } else {
        analysis.lower_reachable_function(entry_function)
    };
    let lowered = reachable.lowered;
    if has_error(&lowered.diagnostics) {
        report_pre_execution_diagnostics(json, lowered.diagnostics)?;
        return Ok(None);
    }
    if let Some(diagnostic) = callsite_entry_diagnostic(&reachable.module, entry) {
        report_pre_execution_diagnostics(json, vec![diagnostic])?;
        return Ok(None);
    }
    if let Some(diagnostic) = retained_user_effect_diagnostic(
        &reachable.module,
        lowered.core.as_ref(),
        entry,
        FunctionKind::Function,
    ) {
        report_pre_execution_diagnostics(json, vec![diagnostic])?;
        return Ok(None);
    }
    let Some(ir) = lowered.ir else {
        report_pre_execution_diagnostics(json, lowered.diagnostics)?;
        eprintln!("veln: run blocked: checked program is not executable");
        return Ok(None);
    };
    let Some(target) = reachable.entry_target else {
        return Err(format!(
            "veln: lowered run entry `{entry}` has no runtime target"
        ));
    };
    Ok(Some(LoweredRunEntry { ir, target }))
}

fn callsite_entry_diagnostic(module: &veln_ast::SurfaceModule, entry: &str) -> Option<Diagnostic> {
    let function = module.functions.iter().find(|function| {
        function.kind == FunctionKind::Function && function.name.as_deref() == Some(entry)
    })?;
    let span = function.callsite.clone()?;
    let mut diagnostic = Diagnostic::new(
        "core.callsite_entry_unsupported",
        Severity::Error,
        DiagnosticKind::Type,
        format!("run entry `{entry}` cannot require call-site context"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("core_lowering")),
            ("entry", JsonValue::string(entry)),
            ("boundary", JsonValue::string("run_entry")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("runtime_support")),
        (
            "message",
            JsonValue::string(
                "A run entry has no Veln call expression from which to construct the hidden location.",
            ),
        ),
    ]));
    Some(diagnostic)
}

fn report_pre_execution_diagnostics(
    json: bool,
    diagnostics: Vec<Diagnostic>,
) -> Result<(), String> {
    let envelope = DiagnosticEnvelope::new(tool_info(), diagnostics);
    if json {
        println!("{}", envelope.to_json());
    } else {
        print_human_stderr(&envelope)?;
    }
    Ok(())
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

fn retained_user_effect_diagnostic(
    module: &veln_ast::SurfaceModule,
    core: Option<&veln_core::CheckedProgram>,
    entry: &str,
    kind: FunctionKind,
) -> Option<Diagnostic> {
    super::retained_user_effect_diagnostic(
        module,
        core,
        entry,
        super::RunnableEntryDiagnostic {
            kind,
            subject: "entry",
            node_kind: "fn",
            boundary: "run_entry",
        },
    )
}

fn run_human(
    build_dir: &std::path::Path,
    program: &veln_backend_jvm::JvmProgram,
    entry_args: &[String],
    execution: &JvmExecution,
) -> Result<ExitCode, String> {
    let trace_files = RunHumanTraceFiles::new(build_dir);
    let event_env = trace_files.event_env();
    let result = prepare_and_run_jvm_capture_with_execution(
        execution, program, "veln run", &event_env, entry_args,
    )?;
    trace_files.report(result).write(
        runtime_result_failure_diagnostic,
        stderr_without_result_failure_line,
    )
}

fn runtime_result_failure_diagnostic(failure: &TestFailure) -> Option<Diagnostic> {
    byte_result_failure_diagnostic(failure)
        .or_else(|| value_result_failure_diagnostic(failure))
        .or_else(|| protocol_result_failure_diagnostic(failure))
}

fn stderr_without_result_failure_line<'a>(stderr: &'a [u8], failure: &TestFailure) -> &'a [u8] {
    let Some(value) = result_failure_value(failure) else {
        return stderr;
    };
    let line = format!("Err({value})\n");
    stderr.strip_suffix(line.as_bytes()).unwrap_or(stderr)
}

fn run_json(
    build_dir: &std::path::Path,
    program: &veln_backend_jvm::JvmProgram,
    entry_args: &[String],
    execution: &JvmExecution,
) -> Result<ExitCode, String> {
    let trace_files = RunJsonTraceFiles::new(build_dir);
    let event_env = trace_files.event_env();
    let result = prepare_and_run_jvm_capture_with_execution(
        execution, program, "veln run", &event_env, entry_args,
    )?;
    let report = trace_files.report(result);
    let exit_code = report.exit_code();
    println!("{}", report.to_json());
    Ok(exit_code)
}

#[cfg(test)]
mod tests;
