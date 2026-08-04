use std::collections::BTreeSet;

use veln_ast::{FunctionKind, SurfaceModule, Visibility};
use veln_core::CheckedProgram;
use veln_diagnostics::{Diagnostic, Severity};
use veln_ir::{TypedProgram, lower_checked_core};

use crate::analysis::{
    check_declared_effect_labels, check_duplicate_constructor_names, check_duplicate_effect_names,
    check_duplicate_function_names, check_duplicate_schema_names, check_duplicate_type_names,
    check_duplicate_use_aliases, check_function_body, check_handler_declarations,
    check_module_boundary, check_public_aliases, check_public_function_boundary,
    check_reserved_prelude_aliases, check_schema_field_primitives, check_schema_type_references,
    check_test_declaration_boundary,
};
use crate::lowering::{lower_project_surface_module_to_core, lower_surface_module_to_core};
use crate::schema;
use crate::types::{
    ReusableStandardEnvironment, TypeEnvironment, prepare_current_reusable_standard_environment,
    prepare_reusable_standard_environment,
};

#[derive(Clone, Debug)]
pub struct LoweredSurfaceModule {
    pub diagnostics: Vec<Diagnostic>,
    pub core: Option<CheckedProgram>,
    pub ir: Option<TypedProgram>,
}

pub fn analyze_surface_module(module: &SurfaceModule) -> Vec<Diagnostic> {
    let environment = TypeEnvironment::from_module(module);
    analyze_surface_module_with_environment(module, &environment, true)
}

pub fn check_project_surface_module(
    module: &SurfaceModule,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let environment = TypeEnvironment::from_module(module);
    check_project_surface_module_with_environment(module, environment)
}

pub fn check_project_surface_module_with_standard_environment(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let environment = TypeEnvironment::from_module_with_standard(module, standard);
    check_project_surface_module_with_environment(module, environment)
}

pub fn check_project_surface_modules_with_standard_environment(
    application_module: &SurfaceModule,
    selected_standard_module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let environment = TypeEnvironment::from_application_module_with_standard(
        application_module,
        selected_standard_module,
        standard,
    );
    check_project_surface_module_with_environment(application_module, environment)
}

pub fn check_project_surface_module_with_standard_modules_environment(
    application_module: &SurfaceModule,
    selected_standard_module_names: &BTreeSet<String>,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let environment = TypeEnvironment::from_application_module_with_standard_module_names(
        application_module,
        selected_standard_module_names,
        standard,
    );
    check_project_surface_module_with_environment(application_module, environment)
}

pub fn prepare_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    prepare_reusable_standard_environment(module)
}

pub fn prepare_current_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    prepare_current_reusable_standard_environment(module)
}

fn check_project_surface_module_with_environment(
    module: &SurfaceModule,
    environment: TypeEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let validate_standard_bodies = should_validate_standard_bodies(module);
    let semantic_diagnostics =
        analyze_surface_module_with_environment(module, &environment, validate_standard_bodies);
    let checked = lower_analyzed_surface_module_with_environment(
        module,
        semantic_diagnostics.clone(),
        &environment,
        true,
    );
    (semantic_diagnostics, checked)
}

fn analyze_surface_module_with_environment(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    validate_standard_bodies: bool,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_duplicate_function_names(module));
    diagnostics.extend(check_duplicate_type_names(module));
    diagnostics.extend(check_duplicate_effect_names(module));
    diagnostics.extend(check_duplicate_schema_names(module));
    diagnostics.extend(check_duplicate_constructor_names(module));
    diagnostics.extend(check_module_boundary(module));
    diagnostics.extend(check_duplicate_use_aliases(module));
    diagnostics.extend(check_reserved_prelude_aliases(module));
    diagnostics.extend(check_public_aliases(module));
    diagnostics.extend(check_schema_field_primitives(module));
    diagnostics.extend(check_schema_type_references(module));
    diagnostics.extend(check_handler_declarations(module, environment));

    for function in &module.functions {
        if !validate_standard_bodies
            && function
                .module_name
                .as_deref()
                .is_some_and(|module| module.starts_with("std::"))
        {
            continue;
        }
        diagnostics.extend(check_declared_effect_labels(function, environment));
        if function.visibility == Visibility::Public {
            diagnostics.extend(check_public_function_boundary(function));
        }
        if function.kind == FunctionKind::Test {
            diagnostics.extend(check_test_declaration_boundary(function));
        }
        diagnostics.extend(check_function_body(function, environment));
    }

    diagnostics
}

pub fn lower_checked_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    lower_analyzed_surface_module(module, analyze_surface_module(module))
}

pub fn lower_project_reachable_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    let environment = TypeEnvironment::from_module(module);
    lower_project_reachable_surface_module_with_environment(module, environment)
}

pub fn lower_project_reachable_surface_module_with_standard_environment(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> LoweredSurfaceModule {
    let environment = TypeEnvironment::from_module_with_standard(module, standard);
    lower_project_reachable_surface_module_with_environment(module, environment)
}

pub fn lower_project_reachable_surface_modules_with_standard_environment(
    reachable_module: &SurfaceModule,
    selected_standard_module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> LoweredSurfaceModule {
    let environment = TypeEnvironment::from_application_module_with_standard(
        reachable_module,
        selected_standard_module,
        standard,
    );
    lower_project_reachable_surface_module_with_environment(reachable_module, environment)
}

fn lower_project_reachable_surface_module_with_environment(
    module: &SurfaceModule,
    environment: TypeEnvironment,
) -> LoweredSurfaceModule {
    let diagnostics = analyze_surface_module_with_environment(
        module,
        &environment,
        should_validate_standard_bodies(module),
    );
    lower_analyzed_surface_module_with_environment(module, diagnostics, &environment, false)
}

fn should_validate_standard_bodies(module: &SurfaceModule) -> bool {
    !module.functions.iter().any(|function| {
        !function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    })
}

pub fn lower_analyzed_surface_module(
    module: &SurfaceModule,
    diagnostics: Vec<Diagnostic>,
) -> LoweredSurfaceModule {
    let environment = TypeEnvironment::from_module(module);
    lower_analyzed_surface_module_with_environment(module, diagnostics, &environment, false)
}

fn lower_analyzed_surface_module_with_environment(
    module: &SurfaceModule,
    mut diagnostics: Vec<Diagnostic>,
    environment: &TypeEnvironment,
    project_check: bool,
) -> LoweredSurfaceModule {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return LoweredSurfaceModule {
            diagnostics,
            core: None,
            ir: None,
        };
    }

    let lowered_core = if project_check {
        lower_project_surface_module_to_core(module, environment)
    } else {
        lower_surface_module_to_core(module, environment)
    };
    diagnostics.extend(lowered_core.diagnostics);
    let ir = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        None
    } else {
        lower_checked_core(&lowered_core.program)
            .ok()
            .map(|mut ir| {
                ir.schema_decoders = schema::ir::schema_decode_specs(module);
                ir
            })
    };

    LoweredSurfaceModule {
        diagnostics,
        core: Some(lowered_core.program),
        ir,
    }
}
