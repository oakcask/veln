use std::thread;

use super::*;
use veln_ast::{UseDecl, lower_surface_ast_with_module_identity};
use veln_source::TextRange;

#[test]
fn reusable_standard_environment_matches_uncached_analysis_for_table_cases() {
    let standard = standard_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard);

    for case in [
        app_case(
            "successful project",
            "src/main.veln",
            concat!(
                "fn compute() -> Int effects [prelude::Ask]\n",
                "  perform prelude::Ask::value()\n",
                "end\n",
                "\n",
                "pub fn main(value: {value: Int}) -> Result<{value: Int}, String>\n",
                "  let observed = handle compute() with prelude::ask(1)\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(observed))\n",
                "  prelude::byte_decode_public_packet(value)\n",
                "end\n",
            ),
        ),
        app_case(
            "type and effect diagnostics",
            "src/main.veln",
            concat!(
                "fn compute() -> Int\n",
                "  perform prelude::Ask::value()\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  prelude::answer(prelude::PayloadShape(1))\n",
                "end\n",
            ),
        ),
        app_case(
            "overlapping app path and declaration names",
            "std/prelude.veln",
            concat!(
                "fn provide(offset: Int) -> Int\n",
                "  offset + 2\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  provide(1)\n",
                "end\n",
            ),
        ),
    ] {
        let module = merge_modules(vec![standard.clone(), case.module]);
        let uncached = check_project_surface_module(&module);
        let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

        assert_same_analysis(case.name, uncached, cached);
    }
}

#[test]
fn reusable_standard_environment_is_prepared_once_for_repeated_and_concurrent_projects() {
    crate::standard_reuse_counters::reset();
    let standard = standard_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard);
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);

    let alpha = merge_modules(vec![
        standard.clone(),
        app_case(
            "alpha",
            "src/shared.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);
    let beta = merge_modules(vec![
        standard.clone(),
        app_case(
            "beta",
            "src/shared.veln",
            concat!(
                "pub fn main() -> Bool\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let alpha_expected = checked_json(&alpha, &reusable);
    let beta_expected = checked_json(&beta, &reusable);
    assert_ne!(alpha_expected, beta_expected);

    for module in [alpha.clone(), beta.clone(), alpha.clone(), beta.clone()] {
        let _ = check_project_surface_module_with_standard_environment(&module, &reusable);
    }
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);

    thread::scope(|scope| {
        let handles = (0..12)
            .map(|index| {
                let module = if index % 2 == 0 {
                    alpha.clone()
                } else {
                    beta.clone()
                };
                let reusable = &reusable;
                scope.spawn(move || (index, checked_json(&module, reusable)))
            })
            .collect::<Vec<_>>();

        for handle in handles {
            let (index, diagnostics) = handle.join().expect("analysis should not panic");
            if index % 2 == 0 {
                assert_eq!(diagnostics, alpha_expected);
            } else {
                assert_eq!(diagnostics, beta_expected);
            }
        }
    });
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
}

struct AppCase {
    name: &'static str,
    module: SurfaceModule,
}

fn app_case(name: &'static str, path: &str, text: &str) -> AppCase {
    let mut module = module_with_identity(path, text, "app");
    let span = SourceFile::new(path, text).span(TextRange::new(0, 0));
    module
        .uses
        .push(UseDecl::implicit_standard_prelude("app".to_string(), span));
    AppCase { name, module }
}

fn standard_module() -> SurfaceModule {
    module_with_identity(
        "prelude.veln",
        concat!(
            "pub effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn provide(offset: Int) -> Int\n",
            "  offset + 1\n",
            "end\n",
            "\n",
            "pub handler ask(offset: Int) handles Ask\n",
            "  value = provide\n",
            "end\n",
            "\n",
            "type PayloadShape\n",
            "  PayloadShape(Int)\n",
            "end\n",
            "\n",
            "pub type SharedPayload = PayloadShape\n",
            "\n",
            "schema Packet\n",
            "  value: Int\n",
            "end\n",
            "\n",
            "pub schema PublicPacket = Packet\n",
            "\n",
            "fn identity(value: SharedPayload) -> SharedPayload\n",
            "  value\n",
            "end\n",
            "\n",
            "pub fn answer = identity\n",
        ),
        "std::prelude",
    )
}

fn module_with_identity(path: &str, text: &str, module_name: &str) -> SurfaceModule {
    let source = SourceFile::new(path, text);
    lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        module_name.to_string(),
        source.span(TextRange::new(0, 0)),
    )
}

fn merge_modules(modules: Vec<SurfaceModule>) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: Vec::new(),
    };
    for module in modules {
        merged.uses.extend(module.uses);
        merged.aliases.extend(module.aliases);
        merged.effects.extend(module.effects);
        merged.handlers.extend(module.handlers);
        merged.schemas.extend(module.schemas);
        merged.codecs.extend(module.codecs);
        merged.types.extend(module.types);
        merged.functions.extend(module.functions);
    }
    merged
}

fn assert_same_analysis(
    name: &str,
    uncached: (Vec<Diagnostic>, LoweredSurfaceModule),
    cached: (Vec<Diagnostic>, LoweredSurfaceModule),
) {
    assert_eq!(
        diagnostic_messages(&cached.0),
        diagnostic_messages(&uncached.0),
        "{name}: semantic diagnostics differ"
    );
    assert_eq!(
        diagnostic_messages(&cached.1.diagnostics),
        diagnostic_messages(&uncached.1.diagnostics),
        "{name}: checked diagnostics differ"
    );
    assert_eq!(
        format!("{:?}", cached.1.core),
        format!("{:?}", uncached.1.core),
        "{name}: lowered core differs"
    );
    assert_eq!(
        format!("{:?}", cached.1.ir),
        format!("{:?}", uncached.1.ir),
        "{name}: lowered IR differs"
    );
}

fn checked_json(module: &SurfaceModule, reusable: &ReusableStandardEnvironment) -> Vec<String> {
    let (_, checked) = check_project_surface_module_with_standard_environment(module, reusable);
    checked
        .diagnostics
        .iter()
        .map(|diagnostic| format!("{}:{}", diagnostic.id, diagnostic.message))
        .collect()
}

fn diagnostic_messages(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| format!("{}:{}", diagnostic.id, diagnostic.message))
        .collect()
}
