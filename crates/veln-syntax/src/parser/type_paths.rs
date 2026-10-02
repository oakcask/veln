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
        if !is_type_path_segment(&tokens[cursor])
            || tokens.get(cursor + 1).map(|token| token.kind) != Some(TokenKind::DoubleColon)
        {
            cursor += 1;
            continue;
        }

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
            paths.push(TypePathSegments {
                segments,
                segment_spans,
            });
        }
    }
    paths
}

fn skip_effect_clause(tokens: &[Token], cursor: usize) -> usize {
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
