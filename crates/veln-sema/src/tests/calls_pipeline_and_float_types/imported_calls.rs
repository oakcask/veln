use super::*;

#[test]
fn infers_non_constructor_calls_from_local_function_signatures() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main() -> Result<Int, AppError>\n",
            "  parse(\"1\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn local_binding_value_shadows_same_named_function() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn callback(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "pub fn main(callback: String) -> String\n",
            "  callback\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(&expr.kind, CoreExprKind::Local(name) if name == "callback"));
}

#[test]
fn resolves_qualified_calls_through_import_aliases() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.math\n",
            "pub fn main() -> Int\n",
            "  math::double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod app.math\n",
            "pub fn double(value: Int) -> Int\n",
            "  value + value\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let math = lower_surface_ast(&parse(&math_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: main.types.into_iter().chain(math.types).collect(),
        functions: main.functions.into_iter().chain(math.functions).collect(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { target, .. } = &expr.kind else {
        panic!("qualified call should lower as a call");
    };
    assert_eq!(target, &CoreCallTarget::Function("double".to_string()));
}

#[test]
fn resolves_qualified_function_values_through_import_aliases() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.text\n",
            "pub fn main() -> Vec<String>\n",
            "  vec_map([1], text::stringify)\n",
            "end\n",
        ),
    );
    let text_source = SourceFile::new(
        "text.veln",
        concat!(
            "mod app.text\n",
            "pub fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let text = lower_surface_ast(&parse(&text_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: main.types.into_iter().chain(text.types).collect(),
        functions: main.functions.into_iter().chain(text.functions).collect(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(
        &args[1].kind,
        CoreExprKind::FunctionValue(name) if name == "stringify"
    ));
}

#[test]
fn resolves_unqualified_public_function_imports() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.math\n",
            "pub fn main() -> Int\n",
            "  double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod app.math\n",
            "pub fn double(value: Int) -> Int\n",
            "  value + value\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let math = lower_surface_ast(&parse(&math_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: main.types.into_iter().chain(math.types).collect(),
        functions: main.functions.into_iter().chain(math.functions).collect(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { target, .. } = &expr.kind else {
        panic!("unqualified import should lower as a call");
    };
    assert_eq!(target, &CoreCallTarget::Function("double".to_string()));
}

#[test]
fn resolves_unqualified_imported_function_values() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.text\n",
            "pub fn main() -> Vec<String>\n",
            "  vec_map([1], stringify)\n",
            "end\n",
        ),
    );
    let text_source = SourceFile::new(
        "text.veln",
        concat!(
            "mod app.text\n",
            "pub fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let text = lower_surface_ast(&parse(&text_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: main.types.into_iter().chain(text.types).collect(),
        functions: main.functions.into_iter().chain(text.functions).collect(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(
        &args[1].kind,
        CoreExprKind::FunctionValue(name) if name == "stringify"
    ));
}

#[test]
fn local_functions_shadow_unqualified_function_imports() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.math\n",
            "fn double(value: String) -> String\n",
            "  value\n",
            "end\n",
            "pub fn main() -> String\n",
            "  double(\"ok\")\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod app.math\n",
            "pub fn double(value: Int) -> Int\n",
            "  value + value\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let math = lower_surface_ast(&parse(&math_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: main.types.into_iter().chain(math.types).collect(),
        functions: main.functions.into_iter().chain(math.functions).collect(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn ambiguous_unqualified_public_function_imports_are_rejected() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.left\n",
            "use app.right\n",
            "pub fn main() -> Int\n",
            "  size()\n",
            "end\n",
        ),
    );
    let left_source = SourceFile::new(
        "left.veln",
        concat!("mod app.left\n", "pub fn size() -> Int\n", "  1\n", "end\n",),
    );
    let right_source = SourceFile::new(
        "right.veln",
        concat!(
            "mod app.right\n",
            "pub fn size() -> Int\n",
            "  2\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let left = lower_surface_ast(&parse(&left_source).tree);
    let right = lower_surface_ast(&parse(&right_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: main
            .functions
            .into_iter()
            .chain(left.functions)
            .chain(right.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "name.ambiguous"
                && diagnostic.message == "ambiguous call_target `size`"
        })
        .expect("ambiguous imported call should be diagnosed");
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>();
    assert!(
        related
            .iter()
            .any(|note| note.contains("use `left::size` to select it"))
    );
    assert!(
        related
            .iter()
            .any(|note| note.contains("use `right::size` to select it"))
    );
}

#[test]
fn private_functions_are_hidden_from_unqualified_imports() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.math\n",
            "pub fn main() -> Int\n",
            "  hidden(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod app.math\n",
            "fn hidden(value: Int) -> Int\n",
            "  value\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let math = lower_surface_ast(&parse(&math_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: main.functions.into_iter().chain(math.functions).collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `hidden`"
    }));
}

#[test]
fn matching_companion_resolves_qualified_private_function_imports() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test increment_test() -> ()\n",
            "  let observed: Int = math::increment(1)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
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

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn matching_companion_cannot_bind_qualified_private_function_values() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "test increment_value_test() -> ()\n",
            "  let mapper: fn(Int) -> Int = math::increment\n",
            "  let observed: Int = mapper(1)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
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

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved value `math::increment`"
        }),
        "{:#?}",
        lowered.diagnostics
    );
}

#[test]
fn non_matching_companion_cannot_resolve_qualified_private_function_imports() {
    let companion_source = SourceFile::new(
        "other.test.veln",
        concat!(
            "mod other__test_companion\n",
            "use math\n",
            "test increment_test() -> Int\n",
            "  math::increment(1)\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
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
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `math::increment`"
    }));
}

#[test]
fn integration_test_module_cannot_resolve_qualified_private_function_imports() {
    let integration_source = SourceFile::new(
        "math_test.veln",
        concat!(
            "mod math_test\n",
            "use math\n",
            "test increment_test() -> Int\n",
            "  math::increment(1)\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
        ),
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
        codecs: Vec::new(),
        types: Vec::new(),
        functions: integration
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `math::increment`"
    }));
}

#[test]
fn matching_companion_private_function_access_is_not_transitive() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use support\n",
            "test private_dependency_test() -> Int\n",
            "  support::private_helper(1)\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "use support\n",
            "pub fn visible() -> Int\n",
            "  support::public_helper(1)\n",
            "end\n",
        ),
    );
    let support_source = SourceFile::new(
        "support.veln",
        concat!(
            "mod support\n",
            "fn private_helper(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "pub fn public_helper(value: Int) -> Int\n",
            "  private_helper(value)\n",
            "end\n",
        ),
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
        codecs: Vec::new(),
        types: Vec::new(),
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .chain(support.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `support::private_helper`"
    }));
}
