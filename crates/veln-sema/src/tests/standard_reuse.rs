use std::sync::{Mutex, MutexGuard};
use std::thread;

use super::*;
use veln_ast::{
    CodecDecl, CodecDirection, CodecImplementationClause, CodecImplementationKind, UseDecl,
    Visibility, lower_surface_ast_with_module_identity,
};
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
                "pub fn main(input: ByteView, base: ByteOffset, value: {value: Int}) -> Result<{value: Int}, String>\n",
                "  let observed = handle compute() with prelude::ask(1)\n",
                "  let boxed = prelude::answer(prelude::PayloadShape(observed))\n",
                "  let decoded = prelude::PayloadCodec(input, base)\n",
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
fn reusable_standard_environment_does_not_expose_unloaded_standard_functions() {
    let _guard = standard_reuse_test_lock();
    let reusable =
        prepare_reusable_standard_surface_module_environment(&standard_modules_with_extra_module())
            .with_current_identity_for_test();
    let module = merge_modules(vec![
        standard_module(),
        app_case(
            "unloaded extra module",
            "src/main.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  extra::extra_answer(1)\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached = check_project_surface_module(&module);
    let cached = check_project_surface_module_with_standard_environment(&module, &reusable);

    assert!(
        diagnostic_json(&cached.0)
            .iter()
            .any(|diagnostic| diagnostic.contains("extra_answer")),
        "the unloaded standard function should remain unresolved"
    );
    assert_same_analysis("unloaded extra module", uncached, cached);
}

#[test]
fn reusable_standard_environment_selects_loaded_standard_module_sets() {
    let _guard = standard_reuse_test_lock();
    let standard = standard_modules_with_extra_module();
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    assert_eq!(reusable.prepared_environment_count_for_test(), 1);

    let prelude_only = merge_modules(vec![
        standard_module(),
        app_case(
            "prelude only",
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
    let prelude_and_extra = merge_modules(vec![
        standard_module(),
        extra_standard_module(),
        app_case(
            "prelude and extra",
            "src/main.veln",
            concat!(
                "use std::extra\n",
                "\n",
                "pub fn main() -> Int\n",
                "  extra::extra_answer(1)\n",
                "end\n",
            ),
        )
        .module,
    ]);

    let uncached_prelude_only = check_project_surface_module(&prelude_only);
    let cached_prelude_only =
        check_project_surface_module_with_standard_environment(&prelude_only, &reusable);

    assert_same_analysis("prelude only", uncached_prelude_only, cached_prelude_only);

    let uncached_prelude_and_extra = check_project_surface_module(&prelude_and_extra);
    let cached_prelude_and_extra =
        check_project_surface_module_with_standard_environment(&prelude_and_extra, &reusable);

    assert_same_analysis(
        "prelude and extra",
        uncached_prelude_and_extra,
        cached_prelude_and_extra,
    );

    let _ = check_project_surface_module_with_standard_environment(&prelude_and_extra, &reusable);
    assert_eq!(reusable.prepared_environment_count_for_test(), 1);
}

#[test]
fn reusable_standard_environment_keeps_selected_facts_constant_for_unrelated_standard_modules() {
    let _guard = standard_reuse_test_lock();
    let selected_modules = std::iter::once("std::prelude".to_string()).collect();

    let base_standard = standard_modules_with_extra_module();
    let base_reusable = prepare_reusable_standard_surface_module_environment(&base_standard)
        .with_current_identity_for_test();
    let base_environment = base_reusable.environment_for_modules_for_test(&selected_modules);

    let expanded_standard = merge_modules(vec![
        standard_modules_with_extra_module(),
        unrelated_annotated_standard_module(128),
    ]);
    let expanded_reusable =
        prepare_reusable_standard_surface_module_environment(&expanded_standard)
            .with_current_identity_for_test();
    let expanded_environment =
        expanded_reusable.environment_for_modules_for_test(&selected_modules);

    assert_eq!(
        expanded_environment.standard_function_modules_for_test(),
        base_environment.standard_function_modules_for_test()
    );
    assert_eq!(
        expanded_reusable.selected_declaration_count_for_test(&selected_modules),
        base_reusable.selected_declaration_count_for_test(&selected_modules)
    );

    let module = merge_modules(vec![
        standard_module(),
        app_case(
            "prelude selected",
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

    let base_checked =
        check_project_surface_module_with_standard_environment(&module, &base_reusable);
    let expanded_checked =
        check_project_surface_module_with_standard_environment(&module, &expanded_reusable);

    assert_same_analysis("prelude selected", base_checked, expanded_checked);
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
    assert_eq!(
        crate::standard_reuse_counters::standard_environment_builds(),
        1
    );

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
    assert_eq!(
        crate::standard_reuse_counters::standard_environment_builds(),
        1
    );

    for module in [alpha.clone(), beta.clone(), alpha.clone(), beta.clone()] {
        let _ = check_project_surface_module_with_standard_environment(&module, &reusable);
    }
    assert_eq!(crate::standard_reuse_counters::standard_prepares(), 1);
    assert_eq!(crate::standard_reuse_counters::application_prepares(), 6);
    assert_eq!(
        crate::standard_reuse_counters::standard_environment_builds(),
        1
    );

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
    assert_eq!(
        crate::standard_reuse_counters::standard_environment_builds(),
        1
    );
}

#[test]
fn reachable_project_lowering_with_reusable_standard_environment_keeps_standard_bodies() {
    let _guard = standard_reuse_test_lock();
    let mut standard = standard_module();
    for decl in &mut standard.types {
        if decl.name.as_deref() == Some("PayloadShape") {
            decl.visibility = Visibility::Public;
        }
    }
    for decl in &mut standard.schemas {
        if decl.name.as_deref() == Some("Packet") {
            decl.visibility = Visibility::Public;
        }
    }
    for decl in &mut standard.functions {
        if decl.name.as_deref() == Some("identity") {
            decl.visibility = Visibility::Public;
        }
    }
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    let app = app_case(
        "reachable standard bodies",
        "src/main.veln",
        concat!(
            "mod app.main\n",
            "\n",
            "fn compute() -> Int effects [prelude::Ask]\n",
            "  perform prelude::Ask::value()\n",
            "end\n",
            "\n",
            "pub fn main(input: ByteView, base: ByteOffset, payload: prelude::SharedPayload) -> DecodeStep<{value: Int}>\n",
            "  let observed = handle compute() with prelude::ask(1)\n",
            "  let boxed = payload\n",
            "  prelude::PayloadCodec(input, base)\n",
            "end\n",
        ),
    )
    .module;
    let module_header = app.module.clone();
    let mut module = merge_modules(vec![standard, app]);
    module.module = module_header;

    let lowered =
        lower_project_reachable_surface_module_with_standard_environment(&module, &reusable);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("reachable core should lower");
    assert!(
        core.effects.iter().any(|effect| effect.name == "Ask"),
        "standard effect should be present: {:#?}",
        core.effects
    );
    assert_core_function(core, "__veln_std$prelude$provide");
    assert_core_function(core, "__veln_std$prelude$decode_payload_packet");
    assert!(
        core.functions.iter().any(|function| {
            function.name == "main"
                && function.body.iter().any(|stmt| {
                    matches!(
                        &stmt.kind,
                        CoreStmtKind::Let { expr, .. }
                            if matches!(
                                &expr.kind,
                                CoreExprKind::Handle { providers, .. }
                                    if providers.iter().any(|provider| {
                                        provider.function
                                            == "__veln_std$prelude$__handler_3$ask_5$value"
                                    })
                            )
                    )
                })
        }),
        "application entry should retain the standard handler provider: {:#?}",
        core.functions
    );
    assert!(
        core.functions.iter().any(|function| {
            function.name == "main"
                && function.body.iter().any(|stmt| {
                    matches!(
                        &stmt.kind,
                        CoreStmtKind::Return { expr }
                            if matches!(
                                &expr.kind,
                                CoreExprKind::Call {
                                    target: CoreCallTarget::CodecDecode { function, codec },
                                    ..
                                } if function == "__veln_std$prelude$decode_payload_packet"
                                    && codec == "PayloadCodec"
                            )
                    )
                })
        }),
        "application entry should call the standard codec: {:#?}",
        core.functions
    );
    assert!(
        core.functions.iter().any(|function| {
            function.name == "main"
                && function
                    .params
                    .iter()
                    .any(|param| format!("{:?}", param.ty).contains("SharedPayload"))
        }),
        "application entry should retain the standard type: {:#?}",
        core.functions
    );

    let ir = lowered.ir.as_ref().expect("reachable IR should lower");
    assert!(
        ir.schema_decoders.iter().any(|schema| {
            schema.schema_name == "Packet" && schema.function_name == "byte_decode_packet"
        }),
        "standard schema decoder should be present: {:#?}",
        ir.schema_decoders
    );
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
    let mut module = module_with_identity(
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
            "  value() => provide(offset)\n",
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
            "\n",
            "fn decode_payload_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{value: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
        ),
        "std::prelude",
    );
    add_payload_codec(&mut module);
    module
}

fn add_payload_codec(module: &mut SurfaceModule) {
    let schema = module
        .schemas
        .iter()
        .find(|schema| schema.name.as_deref() == Some("Packet"))
        .expect("test standard module should define Packet schema");
    module.codecs.push(CodecDecl {
        node_id: schema.node_id,
        module_name: Some("std::prelude".to_string()),
        visibility: Visibility::Public,
        name: Some("PayloadCodec".to_string()),
        schema: Some("Packet".to_string()),
        directions: vec![CodecDirection::Decode],
        implementations: vec![CodecImplementationClause {
            node_id: schema.node_id,
            direction: CodecDirection::Decode,
            kind: CodecImplementationKind::With {
                function: Some("decode_payload_packet".to_string()),
            },
            span: schema.span.clone(),
        }],
        span: schema.span.clone(),
    });
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

fn standard_modules_with_extra_module() -> SurfaceModule {
    merge_modules(vec![standard_module(), extra_standard_module()])
}

fn extra_standard_module() -> SurfaceModule {
    module_with_identity(
        "extra.veln",
        concat!(
            "pub fn extra_answer(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
        ),
        "std::extra",
    )
}

fn unrelated_annotated_standard_module(function_count: usize) -> SurfaceModule {
    let mut text = String::new();
    for index in 0..function_count {
        text.push_str(&format!(
            "pub fn unrelated_{index}(value: Int) -> Int\n  value + {index}\nend\n\n"
        ));
    }
    module_with_identity("unrelated.veln", &text, "std::unrelated")
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

fn assert_core_function(core: &veln_core::CheckedProgram, name: &str) {
    assert!(
        core.functions.iter().any(|function| function.name == name),
        "missing core function {name}: {:#?}",
        core.functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>()
    );
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
