use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_backend_jvm::generate_java_with_entry;
use veln_diagnostics::DiagnosticEnvelope;
use veln_project::Project;
use veln_sema::{analyze_surface_module, lower_checked_surface_module};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};
use crate::java::{compile_and_run_java, create_build_dir};
use crate::surface::{load_surface_module, reachable_entry_module};

pub(crate) fn run_entry(entry: String, inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let (module, diagnostics) = load_surface_module(&project);

    if has_error(&diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), diagnostics))?;
        return Ok(ExitCode::from(1));
    }

    let diagnostics = analyze_surface_module(&module);
    if has_error(&diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), diagnostics))?;
        return Ok(ExitCode::from(1));
    }

    let Some(entry_function) = module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some(entry.as_str()))
    else {
        eprintln!("veln: run entry `{entry}` was not found");
        return Ok(ExitCode::from(1));
    };
    if !entry_function.params.is_empty() {
        eprintln!("veln: run entry `{entry}` has parameters");
        eprintln!("veln: note: this slice only executes zero-argument entries");
        return Ok(ExitCode::from(1));
    }

    let reachable_module = reachable_entry_module(&module, &entry);
    let lowered = lower_checked_surface_module(&reachable_module);
    let Some(ir) = lowered.ir else {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        eprintln!("veln: run blocked: checked program is not executable");
        return Ok(ExitCode::from(1));
    };

    let java = generate_java_with_entry(&ir, &entry);
    let build_dir = create_build_dir("veln-run").map_err(|error| error.to_string())?;
    let result = compile_and_run_java(&build_dir, &java);
    let cleanup_result = fs::remove_dir_all(&build_dir);
    if let Err(error) = cleanup_result {
        eprintln!(
            "veln: warning: failed to remove build directory `{}`: {error}",
            build_dir.display()
        );
    }
    result
}
