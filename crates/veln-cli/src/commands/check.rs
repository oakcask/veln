use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_ast::lower_surface_ast;
use veln_diagnostics::DiagnosticEnvelope;
use veln_project::Project;
use veln_sema::lower_checked_surface_module;
use veln_syntax::parse;
use veln_test::{doctest_sources, reconcile_expected_doctest_failures};

use crate::diagnostics::{has_error, parse_diagnostic_to_envelope, print_human, tool_info};

pub(crate) fn check(json: bool, inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let mut project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let doctests = doctest_sources(&project.files);
    let mut diagnostics = doctests.diagnostics;
    project.files.extend(doctests.sources);

    for source in &project.files {
        let parsed = parse(source);
        let surface_ast = lower_surface_ast(&parsed.tree);
        let has_parse_diagnostics = !parsed.diagnostics.is_empty();
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if !has_parse_diagnostics {
            diagnostics.extend(lower_checked_surface_module(&surface_ast).diagnostics);
        }
    }

    let diagnostics = reconcile_expected_doctest_failures(diagnostics, &doctests.expected_failures);
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
