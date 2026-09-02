use super::*;

struct CompanionModules {
    target: SurfaceModule,
    combined: SurfaceModule,
}

fn companion_modules(
    companion_source: &SourceFile,
    target_source: &SourceFile,
) -> CompanionModules {
    let companion = lower_surface_ast(&parse(companion_source).tree);
    let target = lower_surface_ast(&parse(target_source).tree);
    let combined = SurfaceModule {
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions.iter().cloned())
            .collect(),
        ..companion
    };
    CompanionModules { target, combined }
}

fn lowered_function<'a>(
    lowered: &'a LoweredSurfaceModule,
    name: &str,
) -> &'a veln_core::CoreFunction {
    lowered
        .core
        .as_ref()
        .and_then(|core| core.functions.iter().find(|function| function.name == name))
        .unwrap_or_else(|| panic!("`{name}` should lower"))
}

fn assert_same_signature_and_effects(
    actual: &veln_core::CoreFunction,
    expected: &veln_core::CoreFunction,
) {
    assert_eq!(actual.params, expected.params);
    assert_eq!(actual.return_type, expected.return_type);
    assert_eq!(actual.effects, expected.effects);
}

#[test]
fn companion_call_does_not_complete_target_private_inference() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test identity_test() -> ()\n",
            "  let observed: Int = math::identity(1)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!("mod math\n", "fn identity(value)\n", "  value\n", "end\n",),
    );
    let module = companion_modules(&companion_source, &target_source).combined;

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private parameter `value` has no inferred type"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private function has no inferred return type"
    }));
}

#[test]
fn companion_call_does_not_change_target_private_inference_diagnostics() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test identity_test() -> ()\n",
            "  let observed: Int = math::identity(1)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!("mod math\n", "fn identity(value)\n", "  value\n", "end\n",),
    );
    let modules = companion_modules(&companion_source, &target_source);

    let without_diagnostics = analyze_surface_module(&modules.target);
    let with_diagnostics = analyze_surface_module(&modules.combined);
    let diagnostic_summary = |diagnostics: Vec<Diagnostic>| {
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                (
                    diagnostic.id,
                    diagnostic.message,
                    diagnostic.span.map(|span| span.file.as_str().to_string()),
                )
            })
            .collect::<Vec<_>>()
    };
    let target_diagnostics = with_diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.file.as_str() == "math.veln")
        })
        .cloned()
        .collect::<Vec<_>>();

    assert_eq!(
        diagnostic_summary(target_diagnostics),
        diagnostic_summary(without_diagnostics)
    );
}

#[test]
fn companion_observes_established_private_signature_and_effects() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test emit_test() -> ()\n",
            "  math::emit(\"ready\")\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "fn emit(value) effects [stdio]\n",
            "  stdio::println(value)\n",
            "end\n",
            "pub fn production() -> () effects [stdio]\n",
            "  emit(\"production\")\n",
            "end\n",
        ),
    );
    let module = companion_modules(&companion_source, &target_source).combined;

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "effect.missing_test"
            && diagnostic.message == "test declaration uses undeclared effect `stdio`"
    }));
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.id == "type.private_inference_incomplete" })
    );
}

#[test]
fn companion_call_does_not_change_established_target_signature_or_effects() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test emit_test() -> () effects [stdio]\n",
            "  math::emit(\"ready\")\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "fn emit(value) effects [stdio]\n",
            "  stdio::println(value)\n",
            "end\n",
            "pub fn production_emit() -> () effects [stdio]\n",
            "  emit(\"production\")\n",
            "end\n",
        ),
    );
    let modules = companion_modules(&companion_source, &target_source);

    let lowered_without = lower_checked_surface_module(&modules.target);
    let lowered_with = lower_checked_surface_module(&modules.combined);
    assert!(
        lowered_without.diagnostics.is_empty(),
        "{lowered_without:#?}"
    );
    assert!(lowered_with.diagnostics.is_empty(), "{lowered_with:#?}");

    assert_same_signature_and_effects(
        lowered_function(&lowered_with, "production_emit"),
        lowered_function(&lowered_without, "production_emit"),
    );
    assert_same_signature_and_effects(
        lowered_function(&lowered_with, "emit"),
        lowered_function(&lowered_without, "emit"),
    );
}

#[test]
fn companion_local_function_effects_do_not_share_target_private_name() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "fn emit(value: String) -> ()\n",
            "  ()\n",
            "end\n",
            "test local_emit_test() -> ()\n",
            "  emit(\"quiet\")\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "fn emit(value: String) -> ()\n",
            "  stdio::println(value)\n",
            "end\n",
            "pub fn production() -> () effects [stdio]\n",
            "  emit(\"production\")\n",
            "end\n",
        ),
    );
    let module = companion_modules(&companion_source, &target_source).combined;

    let diagnostics = analyze_surface_module(&module);

    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "effect.missing_test"),
        "{diagnostics:#?}"
    );
}
