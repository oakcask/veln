//! Shared project analysis for Veln tools.

mod analysis;
mod diagnostics;
mod surface;

pub use analysis::{
    AnalysisTiming, DoctestMode, ProjectAnalysis, ReachableEntryAnalysis, analyze_project,
    analyze_project_with_timings, checked_project_diagnostics,
    checked_project_diagnostics_with_captured_dependencies,
};
pub use diagnostics::parse_diagnostic_to_envelope;
pub use surface::{
    CapturedDependencyProject, derive_source_module_path,
    invalid_case_rejected_visible_module_path, load_embedded_standard_surface_module,
    load_surface_module, validate_manifest_dependencies, validate_manifest_exports,
};

#[cfg(test)]
mod tests;
