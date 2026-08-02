use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_diagnostics::DiagnosticEnvelope;
use veln_metrics::{analyze_project_metrics, render_human, report_to_json};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};

pub(crate) fn metrics(json: bool, inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    match analyze_project_metrics(root, &inputs) {
        Ok(report) => {
            if json {
                println!("{}", report_to_json(&report, tool_info()).to_json());
            } else {
                print!("{}", render_human(&report));
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(diagnostics) => {
            let has_errors = has_error(&diagnostics);
            let envelope = DiagnosticEnvelope::new(tool_info(), diagnostics);
            if json {
                println!("{}", envelope.to_json());
            } else {
                print_human_stderr(&envelope)?;
            }
            Ok(if has_errors {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
    }
}
