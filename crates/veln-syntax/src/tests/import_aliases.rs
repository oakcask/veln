use super::*;

#[test]
fn parses_and_formats_explicit_import_alias_before_package_source() {
    let source = SourceFile::new(
        "main.veln",
        "use transport::net as wire from \"std\"\nfn main() -> ()\n  ()\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let use_decl = &output.tree.uses[0];
    assert_eq!(use_decl.name, "transport::net");
    let alias = use_decl.alias.as_ref().expect("explicit alias");
    assert_eq!(alias.name, "wire");
    assert_eq!((alias.span.start.line, alias.span.start.column), (1, 23));
    assert_eq!(use_decl.package.as_ref().unwrap().name, "std");
    assert!(!use_decl.recovered);
    let formatted = format_tree(&output.tree);
    assert!(formatted.starts_with("use transport::net as wire from \"std\"\n"));
    let reparsed = parse(&SourceFile::new("main.veln", formatted));
    assert_eq!(format_tree(&reparsed.tree), format_tree(&output.tree));
}

#[test]
fn retains_invalid_case_alias_tokens_for_semantic_diagnostics() {
    for alias in ["Wire", "_wire", "_"] {
        let source = SourceFile::new("main.veln", format!("use transport::net as {alias}\n"));
        let output = parse(&source);
        assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
        assert_eq!(output.tree.uses[0].alias.as_ref().unwrap().name, alias);
        assert!(!output.tree.uses[0].recovered);
    }
}

#[test]
fn contextual_import_aliases_are_usable_as_qualified_call_paths() {
    for alias in ["callsite", "handler", "handle"] {
        let source = SourceFile::new(
            "main.veln",
            format!("use helper as {alias}\n\nfn main() -> Int\n\t{alias}::answer()\nend\n"),
        );
        let output = parse(&source);

        assert!(
            output.diagnostics.is_empty(),
            "{alias}: {:#?}",
            output.diagnostics
        );
        assert_eq!(output.tree.uses[0].alias.as_ref().unwrap().name, alias);
        assert!(!output.tree.uses[0].recovered);
        let function = first_function(&output);
        let BodyLine::Expr { expr, .. } = &function.body[0] else {
            panic!("expected qualified call expression");
        };
        let ExprKind::Call { callee, args } = &expr.kind else {
            panic!("expected qualified call for {alias}");
        };
        assert!(args.is_empty());
        assert!(matches!(
            &callee.kind,
            ExprKind::NamePath { segments, .. }
                if segments == &vec![alias.to_string(), "answer".to_string()]
        ));
    }

    let ordinary = parse(&SourceFile::new(
        "main.veln",
        "fn main() -> Int\n\thandle answer() with handler()\nend\n",
    ));
    assert!(
        ordinary.diagnostics.is_empty(),
        "{:#?}",
        ordinary.diagnostics
    );
    let BodyLine::Expr { expr, .. } = &first_function(&ordinary).body[0] else {
        panic!("expected handle expression");
    };
    assert!(matches!(&expr.kind, ExprKind::Handle { .. }));
}

#[test]
fn malformed_import_aliases_recover_at_the_declaration_newline() {
    let cases = [
        ("use a as\n", "parse.expected_identifier", 9, None, None),
        (
            "use a as from \"pkg\"\n",
            "parse.expected_identifier",
            10,
            None,
            Some("pkg"),
        ),
        (
            "use a as \"local\"\n",
            "parse.expected_identifier",
            10,
            None,
            None,
        ),
        (
            "use a as b::c\n",
            "parse.expected_newline",
            11,
            Some("b"),
            None,
        ),
        (
            "use a from \"pkg\" as local\n",
            "parse.expected_newline",
            18,
            None,
            Some("pkg"),
        ),
        (
            "use a as b as c\n",
            "parse.expected_newline",
            12,
            Some("b"),
            None,
        ),
        (
            "use a as b from\n",
            "parse.expected_token",
            16,
            Some("b"),
            None,
        ),
    ];
    for (malformed, diagnostic_id, column, expected_alias, expected_package) in cases {
        let source_text = format!("{malformed}use valid\nfn main() -> ()\n  ()\nend\n");
        let source = SourceFile::new("main.veln", source_text.clone());
        let output = parse(&source);
        let diagnostic = output
            .diagnostics
            .first()
            .expect("malformed import diagnostic");
        assert_eq!(diagnostic.id, diagnostic_id, "{malformed}");
        assert!(diagnostic.allows_semantic_analysis(), "{malformed}");
        let span = diagnostic.span.as_ref().expect("malformed import span");
        assert_eq!(
            (span.start.line, span.start.column),
            (1, column),
            "{malformed}"
        );
        assert_recovered_import(&output.tree, malformed, expected_alias, expected_package);
        assert_recovered_import_formatting(
            &output.tree,
            &source_text,
            malformed,
            diagnostic_id,
            expected_alias,
            expected_package,
        );
    }
}

fn assert_recovered_import(
    tree: &SyntaxTree,
    malformed: &str,
    expected_alias: Option<&str>,
    expected_package: Option<&str>,
) {
    let recovered = &tree.uses[0];
    assert!(recovered.recovered, "{malformed}");
    assert_eq!(recovered.name, "a", "{malformed}");
    assert_eq!(recovered.name_spans.len(), 1, "{malformed}");
    assert_eq!(
        recovered.alias.as_ref().map(|alias| alias.name.as_str()),
        expected_alias,
        "{malformed}"
    );
    if let Some(alias) = &recovered.alias {
        assert_eq!(alias.span.start.column, 10, "{malformed}");
        assert_eq!(alias.span.end.column, 11, "{malformed}");
    }
    assert_eq!(
        recovered
            .package
            .as_ref()
            .map(|package| package.name.as_str()),
        expected_package,
        "{malformed}"
    );
    assert_eq!(
        (recovered.span.start.line, recovered.span.start.column),
        (1, 1)
    );
    assert_eq!((recovered.span.end.line, recovered.span.end.column), (2, 1));
    assert_eq!(tree.uses[1].name, "valid", "{malformed}");
    assert!(!tree.uses[1].recovered, "{malformed}");
    assert!(matches!(tree.items[0], SyntaxItem::Function(_)));
}

fn assert_recovered_import_formatting(
    tree: &SyntaxTree,
    source_text: &str,
    malformed: &str,
    diagnostic_id: &str,
    expected_alias: Option<&str>,
    expected_package: Option<&str>,
) {
    let formatted = format_tree(tree);
    assert_eq!(formatted, source_text, "{malformed}");

    let reparsed = parse(&SourceFile::new("main.veln", formatted.clone()));
    assert_eq!(
        reparsed.diagnostics.first().map(|diagnostic| diagnostic.id),
        Some(diagnostic_id),
        "{malformed}"
    );
    let reparsed_use = &reparsed.tree.uses[0];
    assert!(
        reparsed_use.recovered,
        "formatting must not turn {malformed:?} into a binding"
    );
    assert_eq!(reparsed_use.name, tree.uses[0].name, "{malformed}");
    assert_eq!(
        reparsed_use.alias.as_ref().map(|alias| alias.name.as_str()),
        expected_alias,
        "{malformed}"
    );
    assert_eq!(
        reparsed_use
            .package
            .as_ref()
            .map(|package| package.name.as_str()),
        expected_package,
        "{malformed}"
    );
    assert_eq!(reparsed.tree.uses[1].name, "valid", "{malformed}");
    assert!(!reparsed.tree.uses[1].recovered, "{malformed}");
    assert_eq!(format_tree(&reparsed.tree), formatted, "{malformed}");
}
