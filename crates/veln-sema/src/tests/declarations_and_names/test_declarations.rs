use super::*;

#[test]
fn test_declaration_requires_explicit_test_shape() {
    let source = SourceFile::new(
        "main_test.veln",
        "test bad(value: Int) -> Int\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.parameters"
            && diagnostic.message == "test declaration has parameters"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.return_type"
            && diagnostic.message == "test declaration returns `Int`"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn test_declaration_checks_omitted_effect_boundary() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test prints() -> ()\n",
            "  stdio::println(\"hello\")\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_test");
    assert_eq!(
        diagnostics[0].message,
        "test declaration uses undeclared effect `stdio`"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"node_id\":\"test-1\"")
    );
}

#[test]
fn function_declaration_rejects_empty_effects_list() {
    let source = SourceFile::new("main.veln", "fn helper() -> Int effects []\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.empty_declaration");
    assert_eq!(
        diagnostics[0].message,
        "empty effects list is not allowed on a function declaration"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"boundary\":\"private_function\""));
    assert!(details.contains("\"declared_effects\":[]"));
    assert_eq!(diagnostics[0].related.len(), 2);
}

#[test]
fn public_function_declaration_rejects_empty_effects_list() {
    let source = SourceFile::new("main.veln", "pub fn helper() -> Int effects []\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.empty_declaration");
    assert_eq!(
        diagnostics[0].message,
        "empty effects list is not allowed on a function declaration"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"boundary\":\"public_function\"")
    );
}

#[test]
fn test_declaration_rejects_empty_effects_list() {
    let source = SourceFile::new(
        "main_test.veln",
        "test helper() -> () effects []\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.empty_declaration");
    assert_eq!(
        diagnostics[0].message,
        "empty effects list is not allowed on a test declaration"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"boundary\":\"test_declaration\"")
    );
}

#[test]
fn test_declaration_accepts_result_unit_return() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test returns_result() -> Result<(), String>\n",
            "  Ok(())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn test_declaration_accepts_unit_return() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!("test returns_unit() -> ()\n", "  ()\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}
