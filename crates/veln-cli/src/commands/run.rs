use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_ast::FunctionKind;
use veln_backend_jvm::{EntryArgType, generate_classfiles_with_entry_arg_types};
use veln_diagnostics::{DiagnosticEnvelope, JsonValue};
use veln_project::Project;
use veln_test::{TestFailure, contract_failure_from_trace};

use crate::analysis::{DoctestMode, analyze_project};
use crate::diagnostics::{has_error, print_human_stderr, tool_info};
use crate::java::{
    JvmRunResult, create_build_dir, prepare_and_run_jvm, prepare_and_run_jvm_capture_with_env,
};

pub(crate) fn run_entry(
    json: bool,
    entry: String,
    inputs: Vec<PathBuf>,
    entry_args: Vec<String>,
) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let analysis = analyze_project(project, DoctestMode::Exclude);
    let diagnostics = analysis.source_diagnostics();

    if has_error(&diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), diagnostics))?;
        return Ok(ExitCode::from(1));
    }

    let Some(entry_function) = analysis.module.functions.iter().find(|function| {
        function.kind == FunctionKind::Function && function.name.as_deref() == Some(entry.as_str())
    }) else {
        eprintln!("veln: run entry `{entry}` was not found");
        return Ok(ExitCode::from(1));
    };
    if entry_function.params.len() != entry_args.len() {
        eprintln!(
            "veln: run entry `{entry}` expects {} argument(s), got {}",
            entry_function.params.len(),
            entry_args.len()
        );
        eprintln!("veln: note: pass entry arguments after `--`");
        return Ok(ExitCode::from(1));
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
            return Ok(ExitCode::from(1));
        };
        if let Err(message) = validate_entry_arg(arg_type, &param.name, raw_arg) {
            eprintln!("{message}");
            return Ok(ExitCode::from(1));
        }
        entry_arg_types.push(arg_type);
    }

    let reachable = analysis.lower_reachable_entry(&entry, FunctionKind::Function);
    let lowered = reachable.lowered;
    if has_error(&lowered.diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        return Ok(ExitCode::from(1));
    }
    let Some(ir) = lowered.ir else {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        eprintln!("veln: run blocked: checked program is not executable");
        return Ok(ExitCode::from(1));
    };

    let jvm = generate_classfiles_with_entry_arg_types(&ir, &entry, &entry_arg_types);
    let build_dir = create_build_dir("veln-run").map_err(|error| error.to_string())?;
    let result = if json {
        run_json(&build_dir, &jvm, &entry_args)
    } else {
        prepare_and_run_jvm(&build_dir, &jvm, &entry_args)
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

fn run_json(
    build_dir: &std::path::Path,
    program: &veln_backend_jvm::JvmProgram,
    entry_args: &[String],
) -> Result<ExitCode, String> {
    let contract_error_file = build_dir.join("contract-errors.tsv");
    let event_env = [("VELN_CONTRACT_ERRORS", contract_error_file.as_os_str())];
    let result = prepare_and_run_jvm_capture_with_env(
        build_dir, program, "veln run", &event_env, entry_args,
    )?;
    let contract_error_trace = fs::read_to_string(&contract_error_file).unwrap_or_default();

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
