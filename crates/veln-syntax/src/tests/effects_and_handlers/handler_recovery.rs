use super::*;

#[test]
fn rejects_old_and_missing_handler_separators_without_losing_following_items() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "handler ok() for handles::Ask\n",
            "  value() => 0\n",
            "end\n\n",
            "handler old() handles Ask\n",
            "  value() => 1\n",
            "end\n\n",
            "handler missing() Ask\n",
            "  value() => 2\n",
            "end\n\n",
            "handler missing_path() handles::Ask\n",
            "  value() => 3\n",
            "end\n\n",
            "handler old_qualified() handles handles::Ask\n",
            "  value() => 4\n",
            "end\n\n",
            "handler missing_bare() handles\n",
            "  value() => 5\n",
            "end\n\n",
            "handler missing_bare_effects() handles effects [stdio]\n",
            "  value() => 6\n",
            "end\n\n",
            "handler old_bare() handles handles\n",
            "  value() => 7\n",
            "end\n\n",
            "fn following() -> Int\n",
            "  8\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_separator_diagnostics(&output);
    assert_recovered_targets(&output);
    assert_retained_effects_and_following_item(&output);
}

fn assert_separator_diagnostics(output: &ParseOutput) {
    let separator_diagnostics = output
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "parse.expected_token"
                && diagnostic.parser_context == "handler_declaration"
                && diagnostic.expected == vec!["for"]
        })
        .collect::<Vec<_>>();
    let expected = [
        ("handles", (5, 15, 22), RecoveryStrategy::SkipToken),
        ("Ask", (9, 19, 22), RecoveryStrategy::InsertToken),
        ("handles", (13, 24, 31), RecoveryStrategy::InsertToken),
        ("handles", (17, 25, 32), RecoveryStrategy::SkipToken),
        ("handles", (21, 24, 31), RecoveryStrategy::InsertToken),
        ("handles", (25, 32, 39), RecoveryStrategy::InsertToken),
        ("handles", (29, 20, 27), RecoveryStrategy::SkipToken),
    ];

    assert_eq!(output.diagnostics.len(), 7, "{:#?}", output.diagnostics);
    assert_eq!(separator_diagnostics.len(), expected.len());
    for (diagnostic, &(unexpected, span, strategy)) in
        separator_diagnostics.into_iter().zip(&expected)
    {
        assert_handler_separator_diagnostic(diagnostic, unexpected, span, strategy);
    }
}

fn assert_recovered_targets(output: &ParseOutput) {
    let expected_effects = [
        vec!["handles".to_string(), "Ask".to_string()],
        vec!["Ask".to_string()],
        vec!["Ask".to_string()],
        vec!["handles".to_string(), "Ask".to_string()],
        vec!["handles".to_string(), "Ask".to_string()],
        vec!["handles".to_string()],
        vec!["handles".to_string()],
        vec!["handles".to_string()],
    ];
    let expected_spans = [
        (1, 18, 30),
        (5, 23, 26),
        (9, 19, 22),
        (13, 24, 36),
        (17, 33, 45),
        (21, 24, 31),
        (25, 32, 39),
        (29, 28, 35),
    ];

    for (item, (expected_effect, span)) in output.tree.items[..8]
        .iter()
        .zip(expected_effects.iter().zip(expected_spans))
    {
        assert_handler_target(item, expected_effect, span);
    }
}

fn assert_retained_effects_and_following_item(output: &ParseOutput) {
    let SyntaxItem::Handler(handler_with_effects) = &output.tree.items[6] else {
        panic!("expected recovered handler declaration with retained effects");
    };
    assert_eq!(
        handler_with_effects.effects,
        Some(vec!["stdio".to_string()])
    );
    assert!(!handler_with_effects.effects_recovered);
    assert_following_item(output);
}

#[test]
fn missing_separator_before_bare_handles_retains_target_and_following_item() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect handles\n",
            "  value() -> Int\n",
            "end\n\n",
            "handler h() handles\n",
            "  value() => 1\n",
            "end\n\n",
            "fn following() -> Int\n",
            "  2\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    assert_eq!(output.diagnostics[0].id, "parse.expected_token");
    assert_eq!(output.diagnostics[0].parser_context, "handler_declaration");
    assert_eq!(output.diagnostics[0].expected, vec!["for"]);
    assert_handler_separator_diagnostic(
        &output.diagnostics[0],
        "handles",
        (5, 13, 20),
        RecoveryStrategy::InsertToken,
    );
    assert_handler_target(&output.tree.items[1], &["handles".to_string()], (5, 13, 20));
    assert_following_item(&output);
}

fn assert_following_item(output: &ParseOutput) {
    assert!(matches!(
        output.tree.items.last(),
        Some(SyntaxItem::Function(function)) if function.name.as_deref() == Some("following")
    ));
}

fn assert_handler_separator_diagnostic(
    diagnostic: &ParseDiagnostic,
    unexpected: &str,
    (line, start_column, end_column): (usize, usize, usize),
    strategy: RecoveryStrategy,
) {
    let span = diagnostic.span.as_ref().unwrap();
    assert_eq!(diagnostic.unexpected.text, unexpected);
    assert_eq!(span.start.line, line);
    assert_eq!(span.start.column, start_column);
    assert_eq!(span.end.line, line);
    assert_eq!(span.end.column, end_column);
    assert_eq!(diagnostic.recovery.strategy, strategy);
}

fn assert_handler_target(
    item: &SyntaxItem,
    expected_effect: &[String],
    (line, start_column, end_column): (usize, usize, usize),
) {
    let SyntaxItem::Handler(handler) = item else {
        panic!("expected recovered handler declaration");
    };
    assert_eq!(handler.effect, expected_effect);
    assert_eq!(handler.effect_span.start.line, line);
    assert_eq!(handler.effect_span.start.column, start_column);
    assert_eq!(handler.effect_span.end.line, line);
    assert_eq!(handler.effect_span.end.column, end_column);
    assert!(!handler.effect_recovered);
}
