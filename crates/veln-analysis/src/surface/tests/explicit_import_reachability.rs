use super::*;

#[test]
fn local_std_path_does_not_create_an_unwritten_relative_qualifier() {
    fn reachable_for(import: &str, qualifier: &str) -> Vec<(String, String)> {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "main.veln",
                    format!("{import}\nfn main() -> Int\n  {qualifier}::answer()\nend\n"),
                ),
                SourceFile::new("std/helper.veln", "pub fn answer() -> Int\n  42\nend\n"),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        reachable_function_names(&reachable)
            .into_iter()
            .map(|(module, name)| (module.to_string(), name.to_string()))
            .collect()
    }

    assert_eq!(
        reachable_for("use std::helper", "helper"),
        [("main".to_string(), "main".to_string())]
    );
    assert_eq!(
        reachable_for("use std::helper", "std::helper"),
        [
            ("main".to_string(), "main".to_string()),
            ("std::helper".to_string(), "answer".to_string())
        ]
    );
    assert_eq!(
        reachable_for("use std::helper as helper", "helper"),
        [
            ("main".to_string(), "main".to_string()),
            ("std::helper".to_string(), "answer".to_string())
        ]
    );
}

#[test]
fn invalid_import_collision_blocks_alias_reachability_but_keeps_exact_path() {
    let mut module = lower(concat!(
        "mod app::main\n",
        "use Bad as shared\n",
        "use app::math as shared\n",
        "pub fn alias_entry() -> Int\n",
        "  shared::double(2)\n",
        "end\n",
        "pub fn exact_entry() -> Int\n",
        "  app::math::double(2)\n",
        "end\n",
    ));
    let dependency = lower(concat!(
        "mod app::math\n",
        "pub fn double(value: Int) -> Int\n",
        "  value + value\n",
        "end\n",
    ));
    module.functions.extend(dependency.functions);

    let alias_reachable = reachable_entry_module(&module, "alias_entry", FunctionKind::Function);
    assert_eq!(
        reachable_function_names(&alias_reachable),
        vec![("app::main", "alias_entry")]
    );

    let exact_reachable = reachable_entry_module(&module, "exact_entry", FunctionKind::Function);
    assert_eq!(
        reachable_function_names(&exact_reachable),
        vec![("app::main", "exact_entry"), ("app::math", "double")]
    );
}

#[test]
fn recovered_import_does_not_block_valid_alias_reachability() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "mod app::main\n",
            "use app::math as\n",
            "use app::math as wire\n",
            "pub fn alias_entry() -> Int\n",
            "  wire::double(2)\n",
            "end\n",
            "pub fn exact_entry() -> Int\n",
            "  app::math::double(2)\n",
            "end\n",
        ),
    ));
    assert!(!parsed.diagnostics.is_empty());
    let mut module = lower_surface_ast(&parsed.tree);
    let dependency = lower(concat!(
        "mod app::math\n",
        "pub fn double(value: Int) -> Int\n",
        "  value + value\n",
        "end\n",
    ));
    module.functions.extend(dependency.functions);

    let reachable = reachable_entry_module(&module, "alias_entry", FunctionKind::Function);
    assert_eq!(
        reachable_function_names(&reachable),
        vec![("app::main", "alias_entry"), ("app::math", "double")]
    );

    let reachable = reachable_entry_module(&module, "exact_entry", FunctionKind::Function);
    assert_eq!(
        reachable_function_names(&reachable),
        vec![("app::main", "exact_entry"), ("app::math", "double")]
    );
}

#[test]
fn reserved_explicit_prelude_alias_adds_no_reachable_target() {
    let mut module = lower(concat!(
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
    ));
    let dependency = lower(concat!(
        "mod app::math\n",
        "pub fn double(value: Int) -> Int\n",
        "  value + value\n",
        "end\n",
    ));
    module.functions.extend(dependency.functions);

    for entry in ["qualifier", "exact", "bare"] {
        let reachable = reachable_entry_module(&module, entry, FunctionKind::Function);
        assert_eq!(
            reachable_function_names(&reachable),
            vec![("app::main", entry)]
        );
    }
}

#[test]
fn canonical_standard_path_keeps_aliased_import_target_reachable() {
    let standard = lower(concat!(
        "mod std::net\n",
        "pub fn endpoint() -> Int\n",
        "  1\n",
        "end\n",
    ));
    let mut application = lower(concat!(
        "mod app\n",
        "use net as wire from \"std\"\n",
        "pub fn main() -> Int\n",
        "  std::net::endpoint()\n",
        "end\n",
    ));
    application.uses[0].name = "std::net".to_string();

    let reachable = reachable_entry_module_with_standard_cache(
        &standard,
        &application,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );

    assert_eq!(
        reachable_function_names(&reachable),
        vec![("app", "main"), ("std::net", "endpoint")]
    );
}

#[test]
fn same_spelled_owner_modules_do_not_cross_quarantine_reachable_imports() {
    let mut standard = lower(concat!(
        "mod shared\n",
        "use net as standard_wire from \"std\"\n",
        "pub fn standard_entry() -> Int\n",
        "  standard_wire::endpoint()\n",
        "end\n",
    ));
    standard.uses[0].name = "std::net".to_string();
    standard.uses[0].importing_package = Some("std".to_string());
    standard.functions[0].package_name = Some("std".to_string());
    let mut target = lower(concat!(
        "mod std::net\n",
        "pub fn endpoint() -> Int\n",
        "  1\n",
        "end\n",
    ));
    target.functions[0].package_name = Some("std".to_string());
    standard.functions.extend(target.functions);

    let mut application = lower(concat!(
        "mod shared\n",
        "use std::net as local_wire from \"std\"\n",
        "pub fn qualifier() -> Int\n",
        "  local_wire::endpoint()\n",
        "end\n",
        "pub fn exact() -> Int\n",
        "  std::net::endpoint()\n",
        "end\n",
    ));
    application.uses[0].name = "std::net".to_string();

    for entry in ["qualifier", "exact"] {
        let reachable = reachable_entry_module_with_standard_cache(
            &standard,
            &application,
            entry,
            FunctionKind::Function,
            &ReachabilityCache::default(),
        );
        assert_eq!(
            reachable_function_names(&reachable),
            vec![("shared", entry), ("std::net", "endpoint")],
            "entry={entry}"
        );
    }
}

#[test]
fn qualified_calls_keep_same_spelled_target_packages_distinct() {
    let mut standard = lower(concat!(
        "mod std::net\n",
        "pub fn endpoint() -> Int\n",
        "  1\n",
        "end\n",
    ));
    standard.functions[0].package_name = Some("std".to_string());

    let mut application = lower(concat!(
        "mod shared\n",
        "use net as standard_wire from \"std\"\n",
        "use std::net as local_wire\n",
        "pub fn main() -> Int\n",
        "  standard_wire::endpoint() + local_wire::endpoint()\n",
        "end\n",
    ));
    application.uses[0].name = "std::net".to_string();
    let local_target = lower(concat!(
        "mod std::net\n",
        "pub fn endpoint() -> Int\n",
        "  41\n",
        "end\n",
    ));
    application.functions.extend(local_target.functions);

    let reachable = reachable_entry_module_with_standard_cache(
        &standard,
        &application,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );
    let endpoint_packages = reachable
        .functions
        .iter()
        .filter(|function| function.name.as_deref() == Some("endpoint"))
        .map(|function| function.package_name.clone())
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(
        endpoint_packages,
        [None, Some("std".to_string())].into_iter().collect()
    );
}

#[test]
fn equivalent_standard_target_spellings_add_no_reachable_code_in_both_orders() {
    let standard = lower(concat!(
        "mod std::net\n",
        "pub fn endpoint() -> Int\n",
        "  1\n",
        "end\n",
    ));
    for imports in [
        "use net as first from \"std\"\nuse std::net as second from \"std\"\n",
        "use std::net as second from \"std\"\nuse net as first from \"std\"\n",
    ] {
        let mut application = lower(&format!(
            "mod app\n{imports}pub fn main() -> Int\n  first::endpoint() + second::endpoint() + net::endpoint() + std::net::endpoint()\nend\n"
        ));
        for use_decl in &mut application.uses {
            use_decl.name = "std::net".to_string();
        }

        let reachable = reachable_entry_module_with_standard_cache(
            &standard,
            &application,
            "main",
            FunctionKind::Function,
            &ReachabilityCache::default(),
        );

        assert_eq!(
            reachable_function_names(&reachable),
            vec![("app", "main")],
            "imports={imports:?}"
        );
    }
}

#[test]
fn project_loading_canonicalizes_and_quarantines_equivalent_standard_targets() {
    for (imports, written_names) in [
        (
            "use net as first from \"std\"\nuse std::net as second from \"std\"\n",
            ["net", "std::net"],
        ),
        (
            "use std::net as second from \"std\"\nuse net as first from \"std\"\n",
            ["std::net", "net"],
        ),
    ] {
        let source_text = format!(
            "{imports}pub fn main() -> Int\n  first::connect(\"localhost\", 1) + second::connect(\"localhost\", 1) + net::connect(\"localhost\", 1) + std::net::connect(\"localhost\", 1)\nend\n"
        );
        let analysis = analyze_project(
            Project {
                root: ".".into(),
                files: vec![SourceFile::new("main.veln", source_text.clone())],
                manifest: None,
            },
            DoctestMode::Exclude,
        );

        let imports = analysis
            .module
            .uses
            .iter()
            .filter(|use_decl| use_decl.origin == UseOrigin::Source)
            .collect::<Vec<_>>();
        assert_eq!(imports.len(), 2, "{:#?}", analysis.module.uses);
        assert_eq!(
            imports
                .iter()
                .map(|use_decl| use_decl.name.as_str())
                .collect::<Vec<_>>(),
            ["std::net", "std::net"]
        );
        assert_eq!(
            imports
                .iter()
                .map(|use_decl| use_decl.written_name.as_str())
                .collect::<Vec<_>>(),
            written_names
        );
        for use_decl in &imports {
            assert_eq!(use_decl.package.as_deref(), Some("std"));
            assert_eq!(
                use_decl.name_spans.len(),
                use_decl.written_name.split("::").count()
            );
            for (span, segment) in use_decl
                .name_spans
                .iter()
                .zip(use_decl.written_name.split("::"))
            {
                assert_eq!(
                    &source_text[span.start.offset..span.end.offset],
                    segment,
                    "{use_decl:#?}"
                );
            }
        }

        let diagnostics = analysis.semantic_diagnostics();
        let duplicates = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.duplicate")
            .collect::<Vec<_>>();
        assert_eq!(duplicates.len(), 1, "{diagnostics:#?}");
        assert_eq!(duplicates[0].related.len(), 1, "{diagnostics:#?}");
        assert!(
            duplicates[0].related[0]
                .to_json()
                .contains("\"start\":{\"line\":1,"),
            "{diagnostics:#?}"
        );

        for path in ["first", "second", "net", "std::net"] {
            let segments = path.split("::").map(str::to_string).collect::<Vec<_>>();
            assert_eq!(
                veln_sema::normal_imported_module_for_path(
                    &analysis.module,
                    &segments,
                    Some("main"),
                ),
                None,
                "imports={imports:?}, path={path:?}"
            );
        }

        let reachable = analysis.lower_reachable_entry("main", FunctionKind::Function);
        assert_eq!(
            reachable_function_names(&reachable.module),
            vec![("main", "main")],
            "{:#?}",
            reachable.module.functions
        );
    }
}

#[test]
fn production_loading_keeps_same_spelled_import_owners_distinct_during_lowering() {
    let analysis = analyze_project(
        Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "main.veln",
                    concat!(
                        "use http2::core as wire from \"std\"\n",
                        "use std::http2::core as local_core\n",
                        "fn main() -> Int\n",
                        "  let headers = wire::empty_header_list()\n",
                        "  7\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "std/http2/core.veln",
                    concat!(
                        "use http2::hpack from \"std\"\n",
                        "pub fn owner_probe() -> Int\n",
                        "  7\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        },
        DoctestMode::Exclude,
    );
    assert!(
        analysis.semantic_diagnostics().is_empty(),
        "{:#?}",
        analysis.semantic_diagnostics()
    );

    let reachable = analysis.lower_reachable_entry("main", FunctionKind::Function);
    assert!(
        reachable.lowered.diagnostics.is_empty(),
        "{:#?}",
        reachable.lowered.diagnostics
    );
    assert!(
        reachable.lowered.ir.is_some(),
        "reachable lowering produced no IR"
    );
}

#[test]
fn quarantined_standard_import_blocks_its_canonical_exact_path() {
    let standard = lower(concat!(
        "mod std::net\n",
        "pub fn endpoint() -> Int\n",
        "  1\n",
        "end\n",
    ));
    let mut application = lower(concat!(
        "mod app\n",
        "use net as Wire from \"std\"\n",
        "use std::net as other\n",
        "pub fn main() -> Int\n",
        "  std::net::endpoint()\n",
        "end\n",
    ));
    application.uses[0].name = "std::net".to_string();

    let reachable = reachable_entry_module_with_standard_cache(
        &standard,
        &application,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );

    assert_eq!(reachable_function_names(&reachable), vec![("app", "main")]);
}

#[test]
fn repaired_explicit_alias_target_does_not_add_reachable_code() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "use Http as wire\n",
                    "\n",
                    "fn main() -> Int\n",
                    "  wire::entry()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "http.veln",
                concat!(
                    "pub fn entry() -> Int\n",
                    "  helper()\n",
                    "end\n",
                    "fn helper() -> Int\n",
                    "  1\n",
                    "end\n"
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(module.invalid_names.iter().any(|invalid| {
        invalid.name == "Http"
            && invalid.class == veln_ast::NameClass::Module
            && invalid.occurrence == veln_ast::NameOccurrence::PathSegment
    }));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(reachable_function_names(&reachable), vec![("app", "main")]);
    assert!(
        reachable
            .uses
            .iter()
            .any(|use_decl| use_decl.name == "Http" && use_decl.alias == "wire")
    );
}
