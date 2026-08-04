use std::sync::{Mutex, MutexGuard};
use std::thread;

use super::*;
use veln_ast::{UseDecl, lower_surface_ast_with_module_identity};
use veln_diagnostics::diagnostic_to_json;
use veln_source::TextRange;

static STANDARD_REUSE_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn reusable_standard_environment_matches_uncached_analysis_for_table_cases() {
    let _guard = standard_reuse_test_lock();
    let standard = standard_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();

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
        app_case(
            "application std path outside reusable bundle",
            "std/helper.veln",
            concat!(
                "fn answer(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  answer(1)\n",
                "end\n",
            ),
        )
        .with_module_name("std::helper"),
        app_case(
            "application module collides with embedded standard module name",
            "std/prelude.veln",
            concat!(
                "fn local_only(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  local_only(1)\n",
                "end\n",
            ),
        )
        .with_module_name("std::prelude"),
    ] {
        let module = merge_modules(vec![standard.clone(), case.module]);
        let uncached = check_project_surface_module(&module);
        let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

        assert_same_analysis(case.name, uncached, cached);
    }
}

#[test]
fn reusable_standard_environment_matches_uncached_analysis_for_standard_imports() {
    let _guard = standard_reuse_test_lock();
    let standard = standard_modules_with_imports();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    let module = merge_modules(vec![
        standard,
        app_case(
            "standard import",
            "src/main.veln",
            concat!(
                "pub fn main(value: prelude::ImportedPayload) -> prelude::ImportedPayload\n",
                "  value\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached = check_project_surface_module(&module);
    let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

    assert_same_analysis("standard import", uncached, cached);
}

#[test]
fn reusable_standard_environment_uses_only_loaded_standard_modules() {
    let _guard = standard_reuse_test_lock();
    let standard = standard_modules_with_unused_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    let loaded_standard = standard_module();
    let selected_modules = std::iter::once("std::prelude".to_string()).collect();
    let selected_environment = reusable.environment_for_modules_for_test(&selected_modules);
    assert_eq!(
        selected_environment.standard_function_modules_for_test(),
        selected_modules
    );

    let module = merge_modules(vec![
        loaded_standard,
        app_case(
            "loaded prelude only",
            "src/main.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached = check_project_surface_module(&module);
    let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

    assert_same_analysis("loaded prelude only", uncached, cached);
}

#[test]
fn reusable_standard_environment_identity_mismatch_uses_uncached_analysis() {
    let _guard = standard_reuse_test_lock();
    crate::standard_reuse_counters::reset();
    let standard = standard_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard);
    let module = merge_modules(vec![
        standard,
        app_case(
            "identity mismatch",
            "src/main.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(1))\n",
                "  1\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached = check_project_surface_module(&module);
    let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

    assert_same_analysis("identity mismatch", uncached, cached);
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 0);
}

#[test]
fn embedded_reusable_standard_environment_has_current_identity_and_reuses_application_facts() {
    let _guard = standard_reuse_test_lock();
    crate::standard_reuse_counters::reset();
    let standard = crate::types::embedded_standard_surface_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard);
    assert!(reusable.has_current_identity_for_test());

    let module = merge_modules(vec![
        standard,
        app_case(
            "embedded identity",
            "src/main.veln",
            concat!("pub fn main() -> Int\n", "  1\n", "end\n",),
        )
        .module,
    ]);

    let uncached = check_project_surface_module(&module);
    let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

    assert_same_analysis("embedded identity", uncached, cached);
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 1);
}

#[test]
fn reusable_standard_environment_is_prepared_once_for_repeated_and_concurrent_projects() {
    let _guard = standard_reuse_test_lock();
    crate::standard_reuse_counters::reset();
    let standard = standard_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
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
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 2);

    for module in [alpha.clone(), beta.clone(), alpha.clone(), beta.clone()] {
        let _ = check_project_surface_module_with_standard_environment(&module, &reusable);
    }
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 6);

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
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 18);
}

struct AppCase {
    name: &'static str,
    module: SurfaceModule,
}

impl AppCase {
    fn with_module_name(mut self, module_name: &str) -> Self {
        set_module_name(&mut self.module, module_name);
        self
    }
}

fn app_case(name: &'static str, path: &str, text: &str) -> AppCase {
    let mut module = module_with_identity(path, text, "app");
    let span = SourceFile::new(path, text).span(TextRange::new(0, 0));
    module
        .uses
        .push(UseDecl::implicit_standard_prelude("app".to_string(), span));
    AppCase { name, module }
}

fn set_module_name(module: &mut SurfaceModule, module_name: &str) {
    let module_name = Some(module_name.to_string());
    for decl in &mut module.uses {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.aliases {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.effects {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.handlers {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.schemas {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.codecs {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.types {
        decl.module_name = module_name.clone();
    }
    for decl in &mut module.functions {
        decl.module_name = module_name.clone();
    }
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

fn standard_modules_with_imports() -> SurfaceModule {
    merge_modules(vec![
        module_with_identity(
            "support.veln",
            concat!("type PayloadShape\n", "  PayloadShape(Int)\n", "end\n",),
            "std::support",
        ),
        module_with_identity(
            "prelude.veln",
            concat!(
                "use std::support\n",
                "\n",
                "pub type ImportedPayload = support::PayloadShape\n",
            ),
            "std::prelude",
        ),
    ])
}

fn standard_modules_with_unused_module() -> SurfaceModule {
    merge_modules(vec![
        standard_module(),
        module_with_identity(
            "unused.veln",
            concat!(
                "pub fn unused(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
            ),
            "std::unused",
        ),
    ])
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
        diagnostic_json(&cached.0),
        diagnostic_json(&uncached.0),
        "{name}: semantic diagnostics differ"
    );
    assert_eq!(
        diagnostic_json(&cached.1.diagnostics),
        diagnostic_json(&uncached.1.diagnostics),
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
        .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
        .collect()
}

fn diagnostic_json(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
        .collect()
}

fn standard_reuse_test_lock() -> MutexGuard<'static, ()> {
    STANDARD_REUSE_TEST_LOCK
        .lock()
        .expect("standard reuse tests should not poison their counter lock")
}
