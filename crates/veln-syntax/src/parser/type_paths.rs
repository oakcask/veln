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

        if tokens.get(cursor + 1).map(|token| token.kind) == Some(TokenKind::Less) {
            let Some((type_argument_count, end_cursor)) = generic_application(tokens, cursor + 1)
            else {
                cursor += 1;
                continue;
            };
            if tokens.get(end_cursor).map(|token| token.kind) == Some(TokenKind::DoubleColon) {
                cursor += 1;
                continue;
            }
            paths.push(TypePathSegments {
                segments: vec![tokens[cursor].text.clone()],
                segment_spans: vec![source.span(tokens[cursor].range)],
                type_argument_count,
                application_span: source.span(TextRange::new(
                    tokens[cursor].range.start,
                    tokens[end_cursor - 1].range.end,
                )),
                written: bounded_type_application_text(
                    tokens,
                    cursor,
                    end_cursor,
                    &tokens[cursor].text,
                ),
            });
            cursor += 1;
            continue;
        }
        if tokens.get(cursor + 1).map(|token| token.kind) != Some(TokenKind::DoubleColon) {
            if tokens.get(cursor.wrapping_sub(1)).map(|token| token.kind)
                != Some(TokenKind::DoubleColon)
                && tokens.get(cursor + 1).map(|token| token.kind) != Some(TokenKind::Colon)
            {
                let span = source.span(tokens[cursor].range);
                paths.push(TypePathSegments {
                    segments: vec![tokens[cursor].text.clone()],
                    segment_spans: vec![span.clone()],
                    type_argument_count: 0,
                    application_span: span,
                    written: tokens[cursor].text.clone(),
                });
            }
            cursor += 1;
            continue;
        }

        let path_start = cursor;
        let mut segments = vec![tokens[cursor].text.clone()];
        let mut segment_spans = vec![source.span(tokens[cursor].range)];
        cursor += 2;
        while let Some(token) = tokens.get(cursor) {
            if !is_type_path_segment(token) {
                break;
            }
            segments.push(token.text.clone());
            segment_spans.push(source.span(token.range));
            cursor += 1;
            if tokens.get(cursor).map(|token| token.kind) != Some(TokenKind::DoubleColon) {
                break;
            }
            cursor += 1;
        }

        if segments.len() > 1 {
            let (type_argument_count, application_end) =
                if tokens.get(cursor).map(|token| token.kind) == Some(TokenKind::Less) {
                    generic_application(tokens, cursor).unwrap_or((0, cursor))
                } else {
                    (0, cursor)
                };
            let fallback = segments.join("::");
            let written =
                bounded_type_application_text(tokens, path_start, application_end, &fallback);
            paths.push(TypePathSegments {
                segments,
                segment_spans,
                type_argument_count,
                application_span: source.span(TextRange::new(
                    tokens[path_start].range.start,
                    tokens[application_end.saturating_sub(1)].range.end,
                )),
                written,
            });
        }
    }
    paths
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
