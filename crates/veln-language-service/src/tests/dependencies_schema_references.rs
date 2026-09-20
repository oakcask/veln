mod dependencies_schema_references_tests {
    use super::*;

    include!("dependencies_schema_references/alias_chain_eligibility.rs");
    include!("dependencies_schema_references/indexing_performance.rs");
    include!("dependencies_schema_references/schema_operations.rs");
    include!("dependencies_schema_references/composition_boundaries.rs");
    include!("dependencies_schema_references/prelude_alias_visibility.rs");
    include!("dependencies_schema_references/standard_alias_resolution.rs");
    include!("dependencies_schema_references/standard_alias_eligibility.rs");
    include!("dependencies_schema_references/dependency_module_imports.rs");
    include!("dependencies_schema_references/dependency_import_collisions.rs");
    include!("dependencies_schema_references/dependency_alias_eligibility.rs");
    include!("dependencies_schema_references/dependency_alias_targets.rs");
}
