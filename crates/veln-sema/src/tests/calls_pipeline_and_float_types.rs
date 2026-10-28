use super::*;
use veln_core::CoreExpr;

fn clean_surface_module(path: &str, text: impl Into<String>) -> SurfaceModule {
    let source = SourceFile::new(path, text);
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{path} should parse without diagnostics: {:#?}",
        parsed.diagnostics
    );
    lower_surface_ast(&parsed.tree)
}

fn core_function<'a>(
    core: &'a veln_core::CheckedProgram,
    name: &str,
) -> &'a veln_core::CoreFunction {
    core.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("{name} should be lowered"))
}

#[test]
#[should_panic(expected = "main.veln should parse without diagnostics")]
fn clean_surface_module_rejects_parser_recovery() {
    clean_surface_module("main.veln", "use app::math as\n");
}

mod companion_inference;
mod explicit_import_aliases;
mod imported_calls;
mod numeric_and_variadic_calls;
mod pipelines_and_aliases;
