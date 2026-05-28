use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_diagnostics::DiagnosticEnvelope;
use veln_project::Project;
use veln_sema::lower_checked_surface_module;
use veln_test::{doctest_sources, reconcile_expected_doctest_failures};

use crate::diagnostics::{has_error, print_human, tool_info};
use crate::surface::load_surface_module;

pub(crate) fn check(json: bool, inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let mut project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let diagnostics = check_diagnostics(&mut project);
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

pub(crate) fn check_diagnostics(project: &mut Project) -> Vec<veln_diagnostics::Diagnostic> {
    let doctests = doctest_sources(&project.files);
    let mut diagnostics = doctests.diagnostics;
    project.files.extend(doctests.sources);

    let (module, parse_diagnostics) = load_surface_module(&project);
    diagnostics.extend(parse_diagnostics);
    diagnostics.extend(lower_checked_surface_module(&module).diagnostics);

    reconcile_expected_doctest_failures(diagnostics, &doctests.expected_failures)
}
