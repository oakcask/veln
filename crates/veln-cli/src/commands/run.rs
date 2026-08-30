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

use super::run_report::{RunJsonReport, runtime_error_message, transport_failure_from_trace};

mod byte_diagnostics;
mod diagnostic_details;
mod protocol_diagnostics;
mod value_diagnostics;

use byte_diagnostics::byte_result_failure_diagnostic;
use diagnostic_details::*;
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
    let jvm =
        generate_classfiles_with_entry_arg_types(&prepared.ir, &entry, &prepared.entry_arg_types);
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

    let Some(entry_arg_types) = checked_entry_arg_types(&analysis, entry, entry_args)? else {
        write_timings(timings)?;
        return Ok(None);
    };
    let Some(ir) = lower_run_entry(json, &analysis, entry, timings.as_mut())? else {
        write_timings(timings)?;
        return Ok(None);
    };
    Ok(Some(PreparedRun {
        ir,
        entry_arg_types,
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
    let diagnostics = analysis.source_diagnostics();
    if has_error(&diagnostics) {
        report_pre_execution_diagnostics(json, diagnostics)?;
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
    json: bool,
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
        report_pre_execution_diagnostics(json, lowered.diagnostics)?;
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
    Ok(Some(ir))
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
mod tests;
