use super::imported_calls::{assert_main_returns_function_call, merged_import_module};
use super::*;

fn collect_core_function_call_targets(function: &veln_core::CoreFunction) -> Vec<&str> {
    fn collect<'a>(expr: &'a CoreExpr, targets: &mut Vec<&'a str>) {
        match &expr.kind {
            CoreExprKind::Call { target, args } => {
                if let CoreCallTarget::Function(name) = target {
                    targets.push(name);
                }
                for arg in args {
                    collect(arg, targets);
                }
            }
            CoreExprKind::Binary { left, right, .. } => {
                collect(left, targets);
                collect(right, targets);
            }
            _ => {}
        }
    }

    let mut targets = Vec::new();
    for statement in &function.body {
        if let CoreStmtKind::Expr { expr } | CoreStmtKind::Return { expr } = &statement.kind {
            collect(expr, &mut targets);
        }
    }
    targets
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
fn contextual_import_aliases_select_qualified_calls() {
    for alias in ["callsite", "handler", "handle"] {
        let main_source = SourceFile::new(
            "main.veln",
            format!(
                "mod app::main\nuse app::helper as {alias}\npub fn main() -> Int\n\t{alias}::answer()\nend\n"
            ),
        );
        let helper_source = SourceFile::new(
            "helper.veln",
            "mod app::helper\npub fn answer() -> Int\n\t42\nend\n",
        );
        let module = merged_import_module(main_source, vec![helper_source]);

        assert_main_returns_function_call(&module, "answer");
    }
}

#[test]
fn explicit_import_alias_contributes_no_old_leaf_qualifier() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app::main\n",
            "use app::math as wire\n",
            "pub fn main() -> Int\n",
            "  math::double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        "mod app::math\npub fn double(value: Int) -> Int\n  value + value\nend\n",
    );
    let module = merged_import_module(main_source, vec![math_source]);
    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `math::double`"
    }));
    assert!(lowered.core.is_none());
}

#[test]
fn same_named_imports_keep_distinct_module_link_identities() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app::main\n",
            "use app::math as calc\n",
            "use app::other\n",
            "pub fn main() -> Int\n",
            "  calc::double(2) + app::math::double(3) + double(4)\n",
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
    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert!(
        core.functions
            .iter()
            .any(|function| function.name == "app::math::double")
    );
    assert!(
        core.functions
            .iter()
            .any(|function| function.name == "app::other::double")
    );
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let targets = collect_core_function_call_targets(main);
    assert_eq!(
        targets,
        vec![
            "app::math::double",
            "app::math::double",
            "app::other::double",
        ]
    );
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

    assert_main_returns_function_call(&module, "app::other::double");
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
fn malformed_import_aliases_bind_nothing_and_keep_a_following_valid_import() {
    for malformed in [
        "use app::math as\n",
        "use app::math as from \"pkg\"\n",
        "use app::math as \"wire\"\n",
        "use app::math as wire::nested\n",
        "use app::math from \"pkg\" as wire\n",
        "use app::math as wire as nested\n",
        "use app::math as wire from\n",
    ] {
        let main_source = SourceFile::new(
            "main.veln",
            format!(
                "mod app::main\n{malformed}use app::math as wire\npub fn main() -> Int\n  wire::double(2)\nend\n"
            ),
        );
        let parsed = parse(&main_source);
        assert!(!parsed.diagnostics.is_empty(), "{malformed}");
        let lowered = lower_surface_ast(&parsed.tree);
        assert!(lowered.uses[0].recovered, "{malformed}");
        for qualifier in [vec!["wire"], vec!["app", "math"]] {
            let qualifier = qualifier
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
            assert_eq!(
                crate::name_recovery::normal_imported_use_for_path(
                    &lowered,
                    &qualifier,
                    Some("app::main"),
                )
                .map(|use_decl| use_decl.name.as_str()),
                Some("app::math"),
                "malformed={malformed:?}, qualifier={qualifier:?}"
            );
        }

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
            "malformed={malformed:?}, diagnostics={diagnostics:#?}"
        );
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.message != "unresolved call_target `wire::double`"
            })
        );
    }
}

#[test]
fn reserved_explicit_prelude_alias_contributes_no_lookup_candidate() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app::main\n",
            "use app::math as prelude\n",
            "pub fn qualifier() -> Int\n",
            "  prelude::double(2)\n",
            "end\n",
            "pub fn exact() -> Int\n",
            "  app::math::double(2)\n",
            "end\n",
            "pub fn bare() -> Int\n",
            "  double(2)\n",
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
    for target in ["prelude::double", "app::math::double", "double"] {
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == format!("unresolved call_target `{target}`")
        }));
    }
    for path in [vec!["prelude"], vec!["app", "math"]] {
        assert!(
            crate::name_recovery::normal_imported_use_for_path(
                &module,
                &path.into_iter().map(str::to_string).collect::<Vec<_>>(),
                Some("app::main"),
            )
            .is_none()
        );
    }
}

#[test]
fn explicit_import_alias_preserves_exact_companion_and_transitive_privacy_boundaries() {
    for (path, owner, imports, callee, allowed) in [
        (
            "math.test.veln",
            "math__test_companion",
            "use math as target\n",
            "target::increment",
            true,
        ),
        (
            "app.veln",
            "app",
            "use math as target\n",
            "target::increment",
            false,
        ),
        (
            "other.test.veln",
            "other__test_companion",
            "use math as target\n",
            "target::increment",
            false,
        ),
        (
            "math.test.veln",
            "math__test_companion",
            "use math as target\nuse support as helper\n",
            "helper::private_helper",
            false,
        ),
        (
            "math.test.veln",
            "math__test_companion",
            "use math as target\n",
            "helper::public_helper",
            false,
        ),
        (
            "math.test.veln",
            "math__test_companion",
            "use math as target\n",
            "target::helper::public_helper",
            false,
        ),
    ] {
        let module = merged_import_module(
            SourceFile::new(
                path,
                format!(
                    "mod {owner}\n{imports}test boundary() -> ()\n  let observed: Int = {callee}(1)\n  ()\nend\n"
                ),
            ),
            vec![
                SourceFile::new(
                    "math.veln",
                    concat!(
                        "mod math\nuse support as helper\n",
                        "fn increment(value: Int) -> Int\n  value + 1\nend\n",
                        "pub fn visible(value: Int) -> Int\n  helper::public_helper(value)\nend\n",
                    ),
                ),
                SourceFile::new(
                    "support.veln",
                    concat!(
                        "mod support\n",
                        "fn private_helper(value: Int) -> Int\n  value\nend\n",
                        "pub fn public_helper(value: Int) -> Int\n  private_helper(value)\nend\n",
                    ),
                ),
            ],
        );
        let diagnostics = analyze_surface_module(&module);
        if allowed {
            assert!(diagnostics.is_empty(), "{diagnostics:#?}");
            assert!(lower_checked_surface_module(&module).core.is_some());
        } else {
            assert!(
                diagnostics.iter().any(|diagnostic| {
                    diagnostic.id == "name.unresolved"
                        && diagnostic.message == format!("unresolved call_target `{callee}`")
                }),
                "path={path}, callee={callee}, diagnostics={diagnostics:#?}"
            );
            assert!(lower_checked_surface_module(&module).core.is_none());
        }
    }
}
