fn module_rename_conflict(
    declaration: NavigationLocation,
    module: &str,
) -> (NavigationLocation, RenameAffectedScope) {
    (
        declaration,
        RenameAffectedScope::Module {
            name: module.to_string(),
        },
    )
}

fn declaration_matches(
    expected_name: &str,
    selection: &SourceSpan,
    actual_name: &str,
    package: Option<&str>,
    declaration: &SourceSpan,
) -> bool {
    actual_name == expected_name
        && package.is_none()
        && declaration.file == selection.file
        && declaration.start.offset == selection.start.offset
        && declaration.end.offset == selection.end.offset
}

fn is_qualified_path_token(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|token| token.kind == TokenKind::DoubleColon)
        || next_non_layout_token(tokens, index)
            .is_some_and(|token| token.kind == TokenKind::DoubleColon)
}
