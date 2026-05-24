use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_ast::FunctionKind;
use veln_backend_jvm::generate_java_with_entry_args;
use veln_diagnostics::DiagnosticEnvelope;
use veln_project::Project;
use veln_sema::lower_checked_surface_module;

use crate::diagnostics::{has_error, print_human_stderr, tool_info};
use crate::java::{compile_and_run_java, create_build_dir};
use crate::surface::{load_surface_module, reachable_entry_module};

pub(crate) fn run_entry(
    entry: String,
    inputs: Vec<PathBuf>,
    entry_args: Vec<String>,
) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let (module, diagnostics) = load_surface_module(&project);

    if has_error(&diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), diagnostics))?;
        return Ok(ExitCode::from(1));
    }

    let Some(entry_function) = module.functions.iter().find(|function| {
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
    for param in &entry_function.params {
        if param.ty.as_deref() != Some("String") {
            eprintln!(
                "veln: run entry parameter `{}` is not a `String` parameter",
                param.name
            );
            eprintln!("veln: note: entry arguments are passed as strings in this slice");
            return Ok(ExitCode::from(1));
        }
    }

    let reachable_module = reachable_entry_module(&module, &entry, FunctionKind::Function);
    let lowered = lower_checked_surface_module(&reachable_module);
    if has_error(&lowered.diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        return Ok(ExitCode::from(1));
    }
    let Some(ir) = lowered.ir else {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        eprintln!("veln: run blocked: checked program is not executable");
        return Ok(ExitCode::from(1));
    };

    let java = generate_java_with_entry_args(&ir, &entry, entry_args.len());
    let build_dir = create_build_dir("veln-run").map_err(|error| error.to_string())?;
    let result = compile_and_run_java(&build_dir, &java, &entry_args);
    let cleanup_result = fs::remove_dir_all(&build_dir);
    if let Err(error) = cleanup_result {
        eprintln!(
            "veln: warning: failed to remove build directory `{}`: {error}",
            build_dir.display()
        );
    }
    result
}
