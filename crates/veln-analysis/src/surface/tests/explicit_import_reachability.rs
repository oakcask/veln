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
