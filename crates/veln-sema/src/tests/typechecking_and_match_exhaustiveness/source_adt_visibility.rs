use super::*;

#[test]
fn imported_source_adt_constructor_resolves_through_module_and_type_paths() {
    let types = SourceFile::new(
        "types.veln",
        concat!(
            "mod types\n",
            "pub type Maybe<A>\n",
            "  pub Missing\n",
            "  pub Just(A)\n",
            "end\n",
        ),
    );
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use types\n",
            "fn module_qualified() -> Maybe<Int>\n",
            "  types::Just(1)\n",
            "end\n",
            "fn type_qualified() -> Maybe<Int>\n",
            "  types::Maybe::Just(1)\n",
            "end\n",
            "fn label(value: Maybe<Int>) -> String\n",
            "  match value\n",
            "    types::Missing => \"missing\"\n",
            "    types::Maybe::Just(_) => \"value\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let types = lower_surface_ast(&parse(&types).tree);
    let app = lower_surface_ast(&parse(&app).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: types.types,
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn public_type_alias_reexports_imported_constructors() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod spec.app\n",
            "use spec.api\n",
            "pub fn main() -> Int\n",
            "  match api::Circle(3)\n",
            "    api::Circle(radius) => radius\n",
            "  end\n",
            "end\n",
        ),
    );
    let api_source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "use spec.impl\n",
            "pub type Shape = impl::Shape\n",
        ),
    );
    let impl_source = SourceFile::new(
        "impl.veln",
        concat!(
            "mod spec.impl\n",
            "type Shape\n",
            "  pub Circle(Int)\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let api = lower_surface_ast(&parse(&api_source).tree);
    let implementation = lower_surface_ast(&parse(&impl_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses.into_iter().chain(api.uses).collect(),
        aliases: api.aliases,
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: implementation.types,
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn private_source_adt_constructor_is_hidden_from_importing_module() {
    let shapes = SourceFile::new(
        "shapes.veln",
        concat!("mod shapes\n", "pub type Rect\n", "  Rect\n", "end\n",),
    );
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use shapes\n",
            "fn main() -> Rect\n",
            "  Rect\n",
            "end\n",
        ),
    );
    let shapes = lower_surface_ast(&parse(&shapes).tree);
    let app = lower_surface_ast(&parse(&app).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: shapes.types,
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `Rect`"
    }));
}

#[test]
fn private_source_adt_constructor_pattern_does_not_satisfy_imported_exhaustiveness() {
    let shapes = SourceFile::new(
        "shapes.veln",
        concat!(
            "mod shapes\n",
            "pub type Shape\n",
            "  Hidden\n",
            "  pub Shown\n",
            "end\n",
        ),
    );
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use shapes\n",
            "fn label(value: Shape) -> String\n",
            "  match value\n",
            "    shapes::Hidden => \"hidden\"\n",
            "    shapes::Shown => \"shown\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let shapes = lower_surface_ast(&parse(&shapes).tree);
    let app = lower_surface_ast(&parse(&app).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: shapes.types,
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.match_non_exhaustive"
            && diagnostic.message == "match is missing case Hidden"
    }));
}

#[test]
fn private_source_adt_constructor_remains_usable_in_declaring_module() {
    let shapes = SourceFile::new(
        "shapes.veln",
        concat!(
            "mod shapes\n",
            "pub type Rect\n",
            "  Rect\n",
            "end\n",
            "fn make() -> Rect\n",
            "  Rect\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&shapes).tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_source_adt_constructor_type_path_remains_usable_in_declaring_module() {
    let shapes = SourceFile::new(
        "shapes.veln",
        concat!(
            "mod shapes\n",
            "pub type Rect\n",
            "  Rect\n",
            "end\n",
            "fn make() -> Rect\n",
            "  Rect::Rect\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&shapes).tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn matching_companion_resolves_qualified_private_source_adt_type_and_constructor() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test companion_uses_private_target_adt() -> ()\n",
            "  let value: math::Secret = math::Secret::Hidden(3)\n",
            "  match value\n",
            "    math::Secret::Hidden(_) => ()\n",
            "  end\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!("mod math\n", "type Secret\n", "  Hidden(Int)\n", "end\n",),
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
        types: target.types,
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn matching_companion_resolves_private_constructor_of_public_target_adt() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test companion_uses_private_target_constructor() -> ()\n",
            "  let value: math::Token = math::Token::Hidden(3)\n",
            "  match value\n",
            "    math::Token::Hidden(_) => ()\n",
            "    math::Token::Shown => ()\n",
            "  end\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "pub type Token\n",
            "  Hidden(Int)\n",
            "  pub Shown\n",
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
        types: target.types,
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn companion_private_source_adt_access_requires_import_and_qualified_path() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "test companion_needs_explicit_target_path() -> math::Secret\n",
            "  Hidden(3)\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!("mod math\n", "type Secret\n", "  Hidden(Int)\n", "end\n",),
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
        types: target.types,
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `Hidden`"
    }));
}

#[test]
fn non_matching_companion_cannot_resolve_private_source_adt_constructor() {
    let companion_source = SourceFile::new(
        "other.test.veln",
        concat!(
            "mod other__test_companion\n",
            "use math\n",
            "test wrong_companion_cannot_use_private_target_adt() -> math::Secret\n",
            "  math::Secret::Hidden(3)\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!("mod math\n", "type Secret\n", "  Hidden(Int)\n", "end\n",),
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
        types: target.types,
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `math::Secret::Hidden`"
    }));
}

#[test]
fn integration_test_module_cannot_resolve_private_source_adt_constructor() {
    let integration_source = SourceFile::new(
        "math_test.veln",
        concat!(
            "mod math_test\n",
            "use math\n",
            "test integration_cannot_use_private_target_adt() -> math::Secret\n",
            "  math::Secret::Hidden(3)\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!("mod math\n", "type Secret\n", "  Hidden(Int)\n", "end\n",),
    );
    let integration = lower_surface_ast(&parse(&integration_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let module = SurfaceModule {
        module: integration.module,
        uses: integration.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: target.types,
        functions: integration.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `math::Secret::Hidden`"
    }));
}

#[test]
fn matching_companion_private_source_adt_access_is_not_transitive() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "use support\n",
            "test companion_cannot_use_dependency_private_adt() -> support::Secret\n",
            "  support::Secret::Hidden(3)\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "use support\n",
            "pub fn value() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let support_source = SourceFile::new(
        "support.veln",
        concat!("mod support\n", "type Secret\n", "  Hidden(Int)\n", "end\n",),
    );
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let support = lower_surface_ast(&parse(&support_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses.into_iter().chain(target.uses).collect(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: support.types,
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `support::Secret::Hidden`"
    }));
}

#[test]
fn companion_private_source_adt_use_does_not_change_target_diagnostics() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test companion_uses_private_target_adt() -> ()\n",
            "  let value: math::Secret = math::Secret::Hidden(3)\n",
            "  match value\n",
            "    math::Secret::Hidden(_) => ()\n",
            "  end\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "type Secret\n",
            "  Hidden(Int)\n",
            "end\n",
            "pub fn target() -> Int\n",
            "  missing\n",
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
        types: target.types.clone(),
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
        types: target.types,
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let without_diagnostics = analyze_surface_module(&without_companion);
    let with_diagnostics = analyze_surface_module(&with_companion);
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
        diagnostic_summary(without_diagnostics.iter()),
        diagnostic_summary(target_diagnostics.iter())
    );
}
