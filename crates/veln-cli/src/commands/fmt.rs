use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_project::Project;
use veln_syntax::{format_tree, parse};

use crate::diagnostics::print_parse_diagnostic_human;

pub(crate) fn fmt(
    start: super::CommandAnalysisStart,
    inputs: Vec<PathBuf>,
) -> Result<ExitCode, String> {
    let inputs = start.resolve_inputs(inputs);
    let root = start.package_root;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let mut formatted = Vec::new();
    let mut diagnostics = Vec::new();

    for source in &project.files {
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().cloned());
        formatted.push((
            source.path().as_str().to_string(),
            format_tree(&parsed.tree),
        ));
    }

    if !diagnostics.is_empty() {
        for diagnostic in &diagnostics {
            print_parse_diagnostic_human(diagnostic);
        }
        return Ok(ExitCode::from(1));
    }

    for (path, text) in formatted {
        let path = project.root.join(path);
        fs::write(path, text).map_err(|error| error.to_string())?;
    }

    Ok(ExitCode::SUCCESS)
}
