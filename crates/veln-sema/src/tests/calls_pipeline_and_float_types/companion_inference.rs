use super::*;

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
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

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
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let without_companion = SurfaceModule {
        module: target.module.clone(),
        uses: target.uses.clone(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: target.functions.clone(),
        invalid_names: Vec::new(),
    };
    let with_companion = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let without_diagnostics = analyze_surface_module(&without_companion);
    let with_diagnostics = analyze_surface_module(&with_companion);
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
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

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
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let without_companion = SurfaceModule {
        module: target.module.clone(),
        uses: target.uses.clone(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: target.functions.clone(),
        invalid_names: Vec::new(),
    };
    let with_companion = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let lowered_without = lower_checked_surface_module(&without_companion);
    let lowered_with = lower_checked_surface_module(&with_companion);
    assert!(
        lowered_without.diagnostics.is_empty(),
        "{lowered_without:#?}"
    );
    assert!(lowered_with.diagnostics.is_empty(), "{lowered_with:#?}");

    let production_without = lowered_without
        .core
        .as_ref()
        .and_then(|core| {
            core.functions
                .iter()
                .find(|function| function.name == "production_emit")
        })
        .expect("target production function should lower");
    let production_with = lowered_with
        .core
        .as_ref()
        .and_then(|core| {
            core.functions
                .iter()
                .find(|function| function.name == "production_emit")
        })
        .expect("target production function should lower");
    let private_without = lowered_without
        .core
        .as_ref()
        .and_then(|core| {
            core.functions
                .iter()
                .find(|function| function.name == "emit")
        })
        .expect("target private function should lower");
    let private_with = lowered_with
        .core
        .as_ref()
        .and_then(|core| {
            core.functions
                .iter()
                .find(|function| function.name == "emit")
        })
        .expect("target private function should lower");

    assert_eq!(production_with.params, production_without.params);
    assert_eq!(production_with.return_type, production_without.return_type);
    assert_eq!(production_with.effects, production_without.effects);
    assert_eq!(private_with.params, private_without.params);
    assert_eq!(private_with.return_type, private_without.return_type);
    assert_eq!(private_with.effects, private_without.effects);
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
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "effect.missing_test"),
        "{diagnostics:#?}"
    );
}
