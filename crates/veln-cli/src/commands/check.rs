use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_analysis::{DoctestMode, checked_project_diagnostics};
use veln_diagnostics::DiagnosticEnvelope;
use veln_project::Project;

use crate::diagnostics::{has_error, print_human, tool_info};

pub(crate) fn check(json: bool, inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let diagnostics = checked_project_diagnostics(project, DoctestMode::Include);
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
