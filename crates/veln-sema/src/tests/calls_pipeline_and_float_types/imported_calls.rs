use super::*;

fn merged_import_module(primary: SourceFile, dependencies: Vec<SourceFile>) -> SurfaceModule {
    let mut merged = lower_surface_ast(&parse(&primary).tree);
    for dependency in dependencies {
        let module = lower_surface_ast(&parse(&dependency).tree);
        merged.uses.extend(module.uses);
        merged.aliases.extend(module.aliases);
        merged.effects.extend(module.effects);
        merged.handlers.extend(module.handlers);
        merged.schemas.extend(module.schemas);
        merged.types.extend(module.types);
        merged.functions.extend(module.functions);
        merged.invalid_names.extend(module.invalid_names);
    }
    merged
}

fn checked_main_return(module: &SurfaceModule) -> CoreExpr {
    let lowered = lower_checked_surface_module(module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .into_iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    expr.clone()
}

fn assert_main_returns_function_call(module: &SurfaceModule, expected: &str) {
    let expr = checked_main_return(module);
    let CoreExprKind::Call { target, .. } = expr.kind else {
        panic!("tail expression should lower as a call");
    };
    assert_eq!(target, CoreCallTarget::Function(expected.to_string()));
}

fn assert_main_passes_function_value(module: &SurfaceModule, index: usize, expected: &str) {
    let expr = checked_main_return(module);
    let CoreExprKind::Call { args, .. } = expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(
        &args[index].kind,
        CoreExprKind::FunctionValue { name, callsite: false } if name == expected
    ));
}

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

    let expr = checked_main_return(&module);

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
    let module = merged_import_module(main_source, vec![math_source]);

    assert_main_returns_function_call(&module, "double");
}

#[test]
fn explicit_import_alias_and_written_path_select_the_same_function_identity() {
    for callee in ["wire::double", "app::math::double"] {
        let main_source = SourceFile::new(
            "main.veln",
            format!(
                "mod app::main\nuse app::math as wire\npub fn main() -> Int\n  {callee}(2)\nend\n"
            ),
        );
        let math_source = SourceFile::new(
            "math.veln",
            "mod app::math\npub fn double(value: Int) -> Int\n  value + value\nend\n",
        );
        let module = merged_import_module(main_source, vec![math_source]);
        assert_main_returns_function_call(&module, "double");
    }
}

#[test]
fn explicit_import_alias_contributes_no_bare_function_candidate() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app::main\n",
            "use app::math as math\n",
            "use app::other\n",
            "pub fn main() -> Int\n",
            "  double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        "mod app::math\npub fn double(value: Int) -> Int\n  value + value\nend\n",
    );
    let other_source = SourceFile::new(
        "other.veln",
        "mod app::other\npub fn double(value: Int) -> Int\n  value\nend\n",
    );
    let module = merged_import_module(main_source, vec![math_source, other_source]);

    assert_main_returns_function_call(&module, "double");
}

#[test]
fn invalid_import_collision_blocks_only_the_shared_qualifier() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app::main\n",
            "use Bad as shared\n",
            "use app::math as shared\n",
            "pub fn alias_call() -> Int\n",
            "  shared::double(2)\n",
            "end\n",
            "pub fn exact_call() -> Int\n",
            "  app::math::double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        "mod app::math\npub fn double(value: Int) -> Int\n  value + value\nend\n",
    );
    let module = merged_import_module(main_source, vec![math_source]);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.duplicate"),
        "{diagnostics:#?}"
    );
    assert!(
        crate::name_recovery::normal_imported_use_for_path(
            &module,
            &["shared".to_string()],
            Some("app::main"),
        )
        .is_none()
    );
    assert_eq!(
        crate::name_recovery::normal_imported_use_for_path(
            &module,
            &["app".to_string(), "math".to_string()],
            Some("app::main"),
        )
        .map(|use_decl| use_decl.name.as_str()),
        Some("app::math")
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.message != "unresolved call_target `app::math::double`"
        })
    );
}

#[test]
fn blocked_exact_import_path_does_not_fall_through_to_an_alias() {
    for imports in [
        [
            "use foo as first\n",
            "use foo as second\n",
            "use bar as foo\n",
        ],
        [
            "use bar as foo\n",
            "use foo as second\n",
            "use foo as first\n",
        ],
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("mod app::main\n{}{}{}", imports[0], imports[1], imports[2]),
        );
        let module = lower_surface_ast(&parse(&source).tree);

        assert!(
            crate::name_recovery::normal_imported_use_for_path(
                &module,
                &["foo".to_string()],
                Some("app::main"),
            )
            .is_none(),
            "imports={imports:?}"
        );
    }
}

#[test]
fn recovered_import_does_not_quarantine_a_following_valid_import() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app::main\n",
            "use app::math as\n",
            "use app::math as wire\n",
            "pub fn main() -> Int\n",
            "  wire::double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        "mod app::math\npub fn double(value: Int) -> Int\n  value + value\nend\n",
    );
    let module = merged_import_module(main_source, vec![math_source]);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| { diagnostic.message != "unresolved call_target `wire::double`" })
    );
    assert_eq!(
        crate::name_recovery::normal_imported_use_for_path(
            &module,
            &["wire".to_string()],
            Some("app::main"),
        )
        .map(|use_decl| use_decl.name.as_str()),
        Some("app::math")
    );
}

#[test]
fn reserved_explicit_prelude_alias_contributes_no_lookup_candidate() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app::main\n",
            "use app::math as prelude\n",
            "pub fn main() -> Int\n",
            "  prelude::double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        "mod app::math\npub fn double(value: Int) -> Int\n  value + value\nend\n",
    );
    let module = merged_import_module(main_source, vec![math_source]);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.reserved"),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `prelude::double`"
    }));
    assert!(
        crate::name_recovery::normal_imported_use_for_path(
            &module,
            &["prelude".to_string()],
            Some("app::main"),
        )
        .is_none()
    );
}

#[test]
fn nested_module_only_function_paths_keep_function_leaf_role() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use foo::bar\n",
            "pub fn main() -> Int\n",
            "  foo::bar::double(3)\n",
            "end\n",
        ),
    );
    let helper_source = SourceFile::new(
        "bar.veln",
        concat!(
            "mod foo::bar\n",
            "pub fn double(value: Int) -> Int\n",
            "  value + value\n",
            "end\n",
        ),
    );
    let module = merged_import_module(main_source, vec![helper_source]);

    assert_main_returns_function_call(&module, "double");
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
    let module = merged_import_module(main_source, vec![text_source]);

    assert_main_passes_function_value(&module, 1, "stringify");
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
    let module = merged_import_module(main_source, vec![math_source]);

    assert_main_returns_function_call(&module, "double");
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
    let module = merged_import_module(main_source, vec![text_source]);

    assert_main_passes_function_value(&module, 1, "stringify");
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
    let module = merged_import_module(main_source, vec![math_source]);

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
    let module = merged_import_module(main_source, vec![left_source, right_source]);

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
    let module = merged_import_module(main_source, vec![math_source]);

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
    let module = merged_import_module(companion_source, vec![target_source]);

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
    let module = merged_import_module(companion_source, vec![target_source]);

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
    let module = merged_import_module(companion_source, vec![target_source]);

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
    let module = merged_import_module(integration_source, vec![target_source]);

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
    let module = merged_import_module(companion_source, vec![target_source, support_source]);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `support::private_helper`"
    }));
}
