use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use veln_diagnostics::DiagnosticEnvelope;
use veln_metrics::{
    analyze_project_metrics, baseline_from_json, baseline_to_json, check_project_metrics,
    check_project_metrics_with_baseline, render_check_human, render_human, report_check_to_json,
    report_to_json,
};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};

pub(crate) fn metrics(
    start: super::CommandAnalysisStart,
    json: bool,
    check: bool,
    baseline: Option<PathBuf>,
    write_baseline: Option<PathBuf>,
    inputs: Vec<PathBuf>,
) -> Result<ExitCode, String> {
    let inputs = start.resolve_inputs(inputs);
    let root = start.package_root;
    if let Some(path) = write_baseline {
        return write_metrics_baseline(root, &inputs, &path);
    }
    if check {
        let checked = if let Some(path) = baseline {
            let source = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read metrics baseline: {error}"))?;
            match baseline_from_json(&source) {
                Ok(baseline) => check_project_metrics_with_baseline(
                    root,
                    &inputs,
                    baseline,
                    path.to_string_lossy().replace('\\', "/"),
                ),
                Err(diagnostics) => Err(diagnostics),
            }
        } else {
            check_project_metrics(root, &inputs)
        };
        return match checked {
            Ok(report) => {
                if json {
                    println!("{}", report_check_to_json(&report, tool_info()).to_json());
                } else {
                    print_metrics_report_diagnostics(&report.report)?;
                    print!("{}", render_check_human(&report));
                }
                Ok(
                    if report.has_violations() || report.report.completeness.is_partial() {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    },
                )
            }
            Err(diagnostics) => report_metrics_failure(json, diagnostics),
        };
    }

    match analyze_project_metrics(root, &inputs) {
        Ok(report) => {
            if json {
                println!("{}", report_to_json(&report, tool_info()).to_json());
            } else {
                print_metrics_report_diagnostics(&report)?;
                print!("{}", render_human(&report));
            }
            Ok(if report.completeness.is_partial() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Err(diagnostics) => report_metrics_failure(json, diagnostics),
    }
}

fn report_metrics_failure(
    json: bool,
    diagnostics: Vec<veln_diagnostics::Diagnostic>,
) -> Result<ExitCode, String> {
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

fn write_metrics_baseline(
    root: PathBuf,
    inputs: &[PathBuf],
    path: &Path,
) -> Result<ExitCode, String> {
    match analyze_project_metrics(root, inputs) {
        Ok(report) => {
            if report.completeness.is_partial() {
                print_metrics_report_diagnostics(&report)?;
                eprintln!("metrics baseline requires complete analysis");
                return Ok(ExitCode::from(1));
            }
            write_new_file_atomically(path, &baseline_to_json(&report, tool_info()).to_json())?;
            println!("wrote metrics baseline: {}", path.to_string_lossy());
            Ok(ExitCode::SUCCESS)
        }
        Err(diagnostics) => {
            let has_errors = has_error(&diagnostics);
            let envelope = DiagnosticEnvelope::new(tool_info(), diagnostics);
            print_human_stderr(&envelope)?;
            Ok(if has_errors {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
    }
}

fn print_metrics_report_diagnostics(report: &veln_metrics::MetricsReport) -> Result<(), String> {
    if report.diagnostics.is_empty() {
        return Ok(());
    }
    let envelope = DiagnosticEnvelope::new(tool_info(), report.diagnostics.clone());
    print_human_stderr(&envelope)
}

fn write_new_file_atomically(path: &Path, contents: &str) -> Result<(), String> {
    if path.exists() {
        return Err(format!(
            "metrics baseline already exists: {}",
            path.to_string_lossy()
        ));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create metrics baseline directory `{}`: {error}",
            parent.to_string_lossy()
        )
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "metrics baseline path must name a file".to_string())?
        .to_string_lossy();
    let temp_path = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| format!("failed to create temporary metrics baseline: {error}"))?;
    let write_result = (|| {
        file.write_all(contents.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("failed to write metrics baseline: {error}"));
    }
    if let Err(error) = fs::hard_link(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("failed to create metrics baseline: {error}"));
    }
    fs::remove_file(&temp_path)
        .map_err(|error| format!("failed to remove temporary metrics baseline: {error}"))?;
    Ok(())
}
