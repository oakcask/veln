//! Name, type, effect, contract, and hole analysis.

mod adt;
mod analysis;
mod call_resolution;
mod contracts;
mod diagnostics;
mod effects;
mod lowering;
mod name_recovery;
mod pipeline;
mod prelude;
mod repair_candidates;
mod schema;
mod semantic_model;
mod source_less_lookup;
mod standard_names;
mod standard_symbols;
#[cfg(test)]
mod tests;
mod type_lowering;
mod type_relations;
mod type_syntax;
mod types;

#[cfg(test)]
pub(crate) use pipeline::analyze_surface_module_with_base_for_test;
pub use pipeline::{
    LoweredSurfaceModule, analyze_surface_module, check_project_surface_module,
    check_project_surface_module_with_standard_environment,
    check_project_surface_module_with_standard_modules_environment,
    check_project_surface_modules_with_standard_environment, lower_analyzed_surface_module,
    lower_checked_surface_module, lower_project_reachable_surface_module,
    lower_project_reachable_surface_module_with_standard_environment,
    lower_project_reachable_surface_modules_with_standard_environment,
    prepare_current_reusable_standard_surface_module_environment,
    prepare_reusable_standard_surface_module_environment,
    try_prepare_current_reusable_standard_surface_module_environment,
    try_prepare_reusable_standard_surface_module_environment,
    validate_standard_symbol_registry_diagnostic,
};
pub use type_syntax::{type_annotation_reference_names, type_annotation_reference_paths};
pub use types::ReusableStandardEnvironment;
#[cfg(test)]
pub use types::standard_reuse_counters;
