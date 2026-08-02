use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_diagnostics::DiagnosticEnvelope;
use veln_metrics::{
    analyze_project_metrics, check_project_metrics, render_check_human, render_human,
    report_check_to_json, report_to_json,
};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};

pub(crate) fn metrics(json: bool, check: bool, inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    if check {
        return match check_project_metrics(root, &inputs) {
            Ok(report) => {
                if json {
                    println!("{}", report_check_to_json(&report, tool_info()).to_json());
                } else {
                    print!("{}", render_check_human(&report));
                }
                Ok(if report.has_violations() {
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                })
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
        };
    }

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
