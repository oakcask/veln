//! Name, type, effect, contract, and hole analysis.

mod analysis;
mod contracts;
mod diagnostics;
mod effects;
mod lowering;
mod prelude;
#[cfg(test)]
mod tests;
mod types;

use veln_ast::{FunctionKind, SurfaceModule, Visibility};
use veln_core::CheckedProgram;
use veln_diagnostics::{Diagnostic, Severity};
use veln_ir::{TypedProgram, lower_checked_core};

use crate::analysis::{
    check_duplicate_function_names, check_duplicate_use_aliases, check_function_body,
    check_public_function_boundary, check_test_declaration_boundary,
};
use crate::lowering::lower_surface_module_to_core;
use crate::types::TypeEnvironment;

#[derive(Clone, Debug)]
pub struct LoweredSurfaceModule {
    pub diagnostics: Vec<Diagnostic>,
    pub core: Option<CheckedProgram>,
    pub ir: Option<TypedProgram>,
}

pub fn analyze_surface_module(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let environment = TypeEnvironment::from_module(module);

    diagnostics.extend(check_duplicate_function_names(module));
    diagnostics.extend(check_duplicate_use_aliases(module));

    for function in &module.functions {
        if function.visibility == Visibility::Public {
            diagnostics.extend(check_public_function_boundary(function));
        }
        if function.kind == FunctionKind::Test {
            diagnostics.extend(check_test_declaration_boundary(function));
        }
        diagnostics.extend(check_function_body(function, &environment));
    }

    diagnostics
}

pub fn lower_checked_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    let diagnostics = analyze_surface_module(module);
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

    let environment = TypeEnvironment::from_module(module);
    let core = lower_surface_module_to_core(module, &environment);
    let ir = lower_checked_core(&core).ok();

    LoweredSurfaceModule {
        diagnostics,
        core: Some(core),
        ir,
    }
}
