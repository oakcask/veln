use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_diagnostics::DiagnosticEnvelope;
use veln_project::Project;

use crate::analysis::{DoctestMode, analyze_project};
use crate::diagnostics::{has_error, print_human, tool_info};

pub(crate) fn check(json: bool, inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let diagnostics = check_diagnostics(project);
    let has_errors = has_error(&diagnostics);
    let envelope = DiagnosticEnvelope::new(tool_info(), diagnostics);

    if json {
        println!("{}", envelope.to_json());
    } else {
        print_human(&envelope);
    }

    Ok(if has_errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

pub(crate) fn check_diagnostics(project: Project) -> Vec<veln_diagnostics::Diagnostic> {
    analyze_project(project, DoctestMode::Include).checked_diagnostics()
}
