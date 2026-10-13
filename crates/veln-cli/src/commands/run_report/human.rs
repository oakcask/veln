use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Output};

use veln_diagnostics::{Diagnostic, DiagnosticEnvelope};
use veln_test::{TestFailure, result_failure_from_trace};

use crate::diagnostics::{print_human_stderr, tool_info};
use crate::java::{JvmRunResult, exit_code_from_status, forward_process_output};

use super::{cleanup_related_failures, read_trace};

pub(in crate::commands) struct RunHumanTraceFiles {
    result_error: PathBuf,
    cleanup_error: PathBuf,
}

impl RunHumanTraceFiles {
    pub(in crate::commands) fn new(build_dir: &Path) -> Self {
        Self {
            result_error: build_dir.join("result-errors.tsv"),
            cleanup_error: build_dir.join("cleanup-errors.tsv"),
        }
    }

    pub(in crate::commands) fn event_env(&self) -> [(&'static str, &OsStr); 2] {
        [
            ("VELN_RESULT_ERRORS", self.result_error.as_os_str()),
            ("VELN_CLEANUP_ERRORS", self.cleanup_error.as_os_str()),
        ]
    }

    pub(in crate::commands) fn report(&self, result: JvmRunResult) -> RunHumanReport {
        let output = match result {
            JvmRunResult::Ran(output) => output,
            JvmRunResult::ToolError(message) => return RunHumanReport::ToolError(message),
        };
        if output.status.success() {
            return RunHumanReport::Ran {
                output,
                result_failure: None,
                cleanup_related: Vec::new(),
            };
        }
        let result_failure = result_failure_from_trace(&read_trace(&self.result_error));
        let cleanup_related = result_failure.as_ref().map_or_else(Vec::new, |_| {
            cleanup_related_failures(&read_trace(&self.cleanup_error))
        });
        RunHumanReport::Ran {
            output,
            result_failure,
            cleanup_related,
        }
    }
}

pub(in crate::commands) enum RunHumanReport {
    ToolError(String),
    Ran {
        output: Output,
        result_failure: Option<TestFailure>,
        cleanup_related: Vec<String>,
    },
}

impl RunHumanReport {
    pub(in crate::commands) fn write(
        self,
        diagnostic_for: fn(&TestFailure) -> Option<Diagnostic>,
        stderr_without_failure: for<'a> fn(&'a [u8], &TestFailure) -> &'a [u8],
    ) -> Result<ExitCode, String> {
        let (output, result_failure, cleanup_related) = match self {
            Self::ToolError(message) => {
                eprintln!("{message}");
                return Ok(ExitCode::from(1));
            }
            Self::Ran {
                output,
                result_failure,
                cleanup_related,
            } => (output, result_failure, cleanup_related),
        };
        let exit_code = exit_code_from_status(output.status);
        if let Some(failure) = result_failure {
            if let Some(diagnostic) = diagnostic_for(&failure) {
                io::stdout()
                    .write_all(&output.stdout)
                    .map_err(|error| error.to_string())?;
                io::stderr()
                    .write_all(stderr_without_failure(&output.stderr, &failure))
                    .map_err(|error| error.to_string())?;
                print_human_stderr(&DiagnosticEnvelope::new(tool_info(), vec![diagnostic]))?;
            } else {
                forward_process_output(&output)?;
            }
            for related in cleanup_related {
                eprintln!("related cleanup failure: {related}");
            }
        } else {
            forward_process_output(&output)?;
        }
        Ok(exit_code)
    }
}
