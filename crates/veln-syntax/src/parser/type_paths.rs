use super::*;

pub(super) fn type_paths_from_tokens(
    source: &SourceFile,
    tokens: &[Token],
) -> Vec<TypePathSegments> {
    let mut paths = Vec::new();
    let mut cursor = 0usize;
    while cursor < tokens.len() {
        if tokens[cursor].kind == TokenKind::Effects {
            cursor = skip_effect_clause(tokens, cursor);
            continue;
        }
        if !is_type_path_segment(&tokens[cursor]) {
            cursor += 1;
            continue;
        }

        let scanned = scan_type_path(source, tokens, cursor);
        cursor = scanned.next_cursor;
        if let Some(path) = scanned.path {
            paths.push(path);
        }
    }
    paths
}

struct ScannedTypePath {
    path: Option<TypePathSegments>,
    next_cursor: usize,
}

fn scan_type_path(source: &SourceFile, tokens: &[Token], cursor: usize) -> ScannedTypePath {
    if tokens.get(cursor + 1).map(|token| token.kind) == Some(TokenKind::DoubleColon) {
        return scan_qualified_type_path(source, tokens, cursor);
    }
    ScannedTypePath {
        path: scan_unqualified_type_path(source, tokens, cursor),
        next_cursor: cursor + 1,
    }
}

fn scan_unqualified_type_path(
    source: &SourceFile,
    tokens: &[Token],
    cursor: usize,
) -> Option<TypePathSegments> {
    if tokens.get(cursor + 1).map(|token| token.kind) == Some(TokenKind::Less) {
        let (type_argument_count, application_end) = generic_application(tokens, cursor + 1)?;
        if tokens.get(application_end).map(|token| token.kind) == Some(TokenKind::DoubleColon) {
            return None;
        }
        return Some(build_type_path(
            source,
            tokens,
            cursor,
            application_end,
            vec![tokens[cursor].text.clone()],
            vec![source.span(tokens[cursor].range)],
            type_argument_count,
        ));
    }
    if tokens.get(cursor.wrapping_sub(1)).map(|token| token.kind) == Some(TokenKind::DoubleColon)
        || tokens.get(cursor + 1).map(|token| token.kind) == Some(TokenKind::Colon)
    {
        return None;
    }
    Some(build_type_path(
        source,
        tokens,
        cursor,
        cursor + 1,
        vec![tokens[cursor].text.clone()],
        vec![source.span(tokens[cursor].range)],
        0,
    ))
}

fn scan_qualified_type_path(
    source: &SourceFile,
    tokens: &[Token],
    path_start: usize,
) -> ScannedTypePath {
    let mut segments = vec![tokens[path_start].text.clone()];
    let mut segment_spans = vec![source.span(tokens[path_start].range)];
    let mut cursor = path_start + 2;
    while let Some(token) = tokens
        .get(cursor)
        .filter(|token| is_type_path_segment(token))
    {
        segments.push(token.text.clone());
        segment_spans.push(source.span(token.range));
        cursor += 1;
        if tokens.get(cursor).map(|token| token.kind) != Some(TokenKind::DoubleColon) {
            break;
        }
        cursor += 1;
    }

    let path = (segments.len() > 1).then(|| {
        let (type_argument_count, application_end) = generic_application_after(tokens, cursor);
        build_type_path(
            source,
            tokens,
            path_start,
            application_end,
            segments,
            segment_spans,
            type_argument_count,
        )
    });
    ScannedTypePath {
        path,
        next_cursor: cursor,
    }
}

fn generic_application_after(tokens: &[Token], cursor: usize) -> (usize, usize) {
    if tokens.get(cursor).map(|token| token.kind) == Some(TokenKind::Less) {
        generic_application(tokens, cursor).unwrap_or((0, cursor))
    } else {
        (0, cursor)
    }
}

fn build_type_path(
    source: &SourceFile,
    tokens: &[Token],
    path_start: usize,
    application_end: usize,
    segments: Vec<String>,
    segment_spans: Vec<SourceSpan>,
    type_argument_count: usize,
) -> TypePathSegments {
    let fallback = segments.join("::");
    TypePathSegments {
        segments,
        segment_spans,
        type_argument_count,
        application_span: source.span(TextRange::new(
            tokens[path_start].range.start,
            tokens[application_end.saturating_sub(1)].range.end,
        )),
        written: bounded_type_application_text(tokens, path_start, application_end, &fallback),
    }
}

pub(super) fn bounded_type_application_text(
    tokens: &[Token],
    start: usize,
    end: usize,
    fallback: &str,
) -> String {
    const MAX_WRITTEN_TYPE_BYTES: usize = 256;
    let mut parts = Vec::new();
    let mut bytes = 0usize;
    for token in &tokens[start..end] {
        bytes += token.text.len();
        if bytes > MAX_WRITTEN_TYPE_BYTES {
            return fallback.to_string();
        }
        parts.push(token.text.clone());
    }
    normalize_type_text(parts)
}

fn generic_application(tokens: &[Token], opener: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut argument_count = 1usize;
    for (index, token) in tokens.iter().enumerate().skip(opener) {
        match token.kind {
            TokenKind::Less => depth += 1,
            TokenKind::Greater => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some((argument_count, index + 1));
                }
            }
            TokenKind::Comma if depth == 1 => argument_count += 1,
            _ => {}
        }
    }
    None
}

pub(super) fn skip_effect_clause(tokens: &[Token], cursor: usize) -> usize {
    let mut cursor = cursor + 1;
    if tokens.get(cursor).map(|token| token.kind) != Some(TokenKind::LBracket) {
        return cursor;
    }
    cursor += 1;
    let mut depth = 1usize;
    while let Some(token) = tokens.get(cursor) {
        match token.kind {
            TokenKind::LBracket => depth += 1,
            TokenKind::RBracket => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return cursor + 1;
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    cursor
}

pub(super) fn is_type_path_segment(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Ident | TokenKind::Callsite | TokenKind::Hole
    )
}
