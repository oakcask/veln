fn is_function_declaration_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|previous| matches!(previous.kind, TokenKind::Fn | TokenKind::Test))
}

fn is_parameter_name(tokens: &[Token], index: usize) -> bool {
    next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
}

fn is_local_binding_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index).is_some_and(|previous| previous.kind == TokenKind::Let)
        || is_let_pattern_binding_name(tokens, index)
        || is_match_arm_pattern_binding_name(tokens, index)
        || is_satisfy_candidate_binding_name(tokens, index)
}

fn is_let_pattern_binding_name(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    if token.kind != TokenKind::Ident {
        return false;
    }
    let Some(let_index) = tokens[..index]
        .iter()
        .enumerate()
        .rev()
        .take_while(|(_, token)| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .find_map(|(previous_index, token)| {
            (token.kind == TokenKind::Let).then_some(previous_index)
        })
    else {
        return false;
    };
    let_pattern_binding_names(tokens, let_index)
        .iter()
        .any(|(name, _, end)| name == &token.text && *end == token.range.end)
}

fn is_match_arm_pattern_binding_name(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token.kind == TokenKind::Ident
        && tokens[index + 1..]
            .iter()
            .take_while(|next| next.kind != TokenKind::Newline && next.kind != TokenKind::Eof)
            .any(|next| next.kind == TokenKind::FatArrow)
        && is_pattern_binding_token(tokens, index)
}

fn is_satisfy_candidate_binding_name(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Ident && previous.text == "satisfy")
        && next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::FatArrow)
}

fn is_satisfy_arrow(tokens: &[Token], index: usize) -> bool {
    let Some(candidate_index) = previous_non_layout_index(tokens, index) else {
        return false;
    };
    if tokens[candidate_index].kind != TokenKind::Ident {
        return false;
    }
    previous_non_layout_token(tokens, candidate_index)
        .is_some_and(|previous| previous.kind == TokenKind::Ident && previous.text == "satisfy")
}

fn is_field_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index).is_some_and(|previous| previous.kind == TokenKind::Dot)
        || next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
}

fn is_ensure_reference(tokens: &[Token], index: usize) -> bool {
    tokens[..index]
        .iter()
        .rev()
        .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .any(|token| token.kind == TokenKind::Ensure)
}

fn is_function_alias_target_reference(tokens: &[Token], index: usize, name: &str) -> bool {
    tokens[index].text == name
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Equal)
        && tokens[..index]
            .iter()
            .rev()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .any(|token| token.kind == TokenKind::Fn)
}

fn is_codec_implementation_function_reference(tokens: &[Token], index: usize, name: &str) -> bool {
    tokens[index].text == name
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Ident && previous.text == "with")
        && inside_codec_declaration(tokens, index)
}

fn is_call_target_token(tokens: &[Token], index: usize) -> bool {
    next_non_whitespace_token(tokens, index).is_some_and(|next| next.kind == TokenKind::LParen)
}

fn is_bare_function_reference_token(
    tokens: &[Token],
    scopes: &[FunctionScope],
    index: usize,
    name: &str,
) -> bool {
    tokens[index].kind == TokenKind::Ident
        && tokens[index].text == name
        && previous_non_layout_token(tokens, index)
            .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
        && !is_field_name(tokens, index)
        && !is_function_declaration_name(tokens, index)
        && !is_parameter_name(tokens, index)
        && !is_local_binding_name(tokens, index)
        && !is_handler_operation_clause_operation_name(tokens, index)
        && (is_call_target_token(tokens, index)
            || token_scope(scopes, tokens[index].range.start)
                .is_some_and(|scope| !scope.shadows(name, tokens, index))
            || is_handler_operation_clause_call_target(tokens, index)
            || is_function_alias_target_reference(tokens, index, name)
            || is_codec_implementation_function_reference(tokens, index, name))
}

fn is_constructor_reference_token(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && !is_effect_operation_declaration_name(tokens, index)
        && !is_constructor_declaration_name(tokens, index)
        && !is_handler_operation_clause_operation_name(tokens, index)
        && (is_call_target_token(tokens, index)
            || is_bare_nullary_constructor_expression(tokens, index)
            || is_bare_nullary_constructor_pattern(tokens, index))
}

fn is_bare_nullary_constructor_expression(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token
        .text
        .chars()
        .next()
        .is_some_and(|initial| initial.is_ascii_uppercase())
        && !is_type_position_token(tokens, index)
        && !is_function_declaration_name(tokens, index)
        && !is_type_declaration_name(tokens, index)
        && !is_constructor_declaration_name(tokens, index)
        && !is_parameter_name(tokens, index)
        && !is_local_binding_name(tokens, index)
        && !is_field_name(tokens, index)
        && !is_handler_operation_clause_operation_name(tokens, index)
}

fn is_bare_nullary_constructor_pattern(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token
        .text
        .chars()
        .next()
        .is_some_and(|initial| initial.is_ascii_uppercase())
        && is_match_arm_pattern_token(tokens, index)
        && previous_non_layout_token(tokens, index)
            .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
        && next_non_layout_token(tokens, index)
            .is_none_or(|next| !matches!(next.kind, TokenKind::DoubleColon | TokenKind::Colon))
}

fn is_match_arm_pattern_token(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token.kind == TokenKind::Ident
        && tokens[index + 1..]
            .iter()
            .take_while(|next| next.kind != TokenKind::Newline && next.kind != TokenKind::Eof)
            .position(|next| next.kind == TokenKind::FatArrow)
            .is_some_and(|relative_arrow| {
                let arrow_index = index + 1 + relative_arrow;
                token.range.start >= match_arm_pattern_start(tokens, arrow_index, 0)
            })
}

fn is_type_declaration_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|previous| previous.kind == TokenKind::Type)
}

fn is_constructor_declaration_name(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && inside_top_level_block(tokens, index, TokenKind::Type)
        && constructor_declaration_prefix_is_visible(tokens, index)
}

fn constructor_declaration_prefix_is_visible(tokens: &[Token], index: usize) -> bool {
    line_tokens_before(tokens, index).iter().all(|token| {
        matches!(
            token.kind,
            TokenKind::Whitespace | TokenKind::Newline | TokenKind::Pub
        )
    })
}

fn is_type_position_token(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|previous| matches!(previous.kind, TokenKind::Colon | TokenKind::Arrow))
}

fn is_effect_operation_declaration_name(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && inside_top_level_block(tokens, index, TokenKind::Effect)
        && line_tokens_before(tokens, index)
            .iter()
            .all(|token| matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        && next_non_whitespace_token(tokens, index)
            .is_some_and(|next| next.kind == TokenKind::LParen)
}

fn is_schema_operation_path_leaf_token(file: &IndexedFile, index: usize) -> bool {
    is_schema_operation_path_leaf_candidate_token(&file.tokens, index)
        && next_non_layout_token(&file.tokens, index)
            .is_some_and(|next| next.kind == TokenKind::From)
        && file.schema_operation_leaf_spans.iter().any(|span| {
            span.start.offset == file.tokens[index].range.start
                && span.end.offset == file.tokens[index].range.end
        })
}

fn is_schema_operation_path_leaf_candidate_token(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && line_tokens_before(tokens, index)
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Decode | TokenKind::Encode))
        && tokens[index + 1..]
            .iter()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .any(|token| token.kind == TokenKind::From)
        && next_non_layout_token(tokens, index)
            .is_none_or(|next| next.kind != TokenKind::DoubleColon)
}

fn is_schema_composition_path_leaf_token(tokens: &[Token], index: usize) -> bool {
    if tokens[index].kind != TokenKind::Ident || !inside_schema_declaration(tokens, index) {
        return false;
    }
    let Some(field_type) = schema_composition_field_type(tokens, index) else {
        return false;
    };
    schema_path_leaf_in(&field_type, tokens) == Some(index)
        || is_repeat_schema_path_leaf(&field_type, tokens, index)
        || is_array_schema_path_leaf(&field_type, tokens, index)
}

fn schema_composition_field_type(tokens: &[Token], index: usize) -> Option<Vec<usize>> {
    let line_start = line_start_index(tokens, index);
    let line_end = tokens[index..]
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Newline | TokenKind::Eof))
        .map_or(tokens.len(), |offset| index + offset);
    let mut significant = (line_start..line_end)
        .filter(|candidate| {
            !matches!(
                tokens[*candidate].kind,
                TokenKind::Whitespace | TokenKind::Comment
            )
        })
        .collect::<Vec<_>>();
    if let Some(where_position) = significant
        .iter()
        .position(|candidate| tokens[*candidate].kind == TokenKind::Where)
    {
        significant.truncate(where_position);
    }
    let colon_position = significant
        .iter()
        .position(|candidate| tokens[*candidate].kind == TokenKind::Colon)
        ?;
    Some(significant[colon_position + 1..].to_vec())
}

fn is_repeat_schema_path_leaf(field_type: &[usize], tokens: &[Token], index: usize) -> bool {
    if field_type.len() >= 5
        && tokens[field_type[0]].kind == TokenKind::Ident
        && tokens[field_type[0]].text == "Repeat"
        && tokens[field_type[1]].kind == TokenKind::LParen
        && tokens[*field_type.last().unwrap()].kind == TokenKind::RParen
    {
        let inner = &field_type[2..field_type.len() - 1];
        if let Some(comma) = top_level_separator(inner, tokens, TokenKind::Comma) {
            return schema_path_leaf_in(&inner[comma + 1..], tokens) == Some(index);
        }
    }
    false
}

fn is_array_schema_path_leaf(field_type: &[usize], tokens: &[Token], index: usize) -> bool {
    if field_type.len() >= 4
        && tokens[field_type[0]].kind == TokenKind::LBracket
        && tokens[*field_type.last().unwrap()].kind == TokenKind::RBracket
    {
        let inner = &field_type[1..field_type.len() - 1];
        if let Some(semicolon) = top_level_separator(inner, tokens, TokenKind::Semicolon) {
            return schema_path_leaf_in(&inner[..semicolon], tokens) == Some(index);
        }
    }
    false
}

fn schema_path_leaf_in(indices: &[usize], tokens: &[Token]) -> Option<usize> {
    if indices.is_empty() || indices.len().is_multiple_of(2) {
        return None;
    }
    for (position, index) in indices.iter().enumerate() {
        let expected = if position % 2 == 0 {
            TokenKind::Ident
        } else {
            TokenKind::DoubleColon
        };
        if tokens[*index].kind != expected {
            return None;
        }
    }
    indices.last().copied()
}

fn top_level_separator(indices: &[usize], tokens: &[Token], separator: TokenKind) -> Option<usize> {
    let mut depth = 0usize;
    for (position, index) in indices.iter().enumerate() {
        match tokens[*index].kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1)
            }
            kind if kind == separator && depth == 0 => return Some(position),
            _ => {}
        }
    }
    None
}

fn inside_schema_declaration(tokens: &[Token], index: usize) -> bool {
    let mut nested_blocks = 0usize;
    for token in tokens[..index].iter().rev() {
        if token.kind == TokenKind::End {
            nested_blocks += 1;
            continue;
        }
        if !matches!(
            token.kind,
            TokenKind::Fn
                | TokenKind::Test
                | TokenKind::Type
                | TokenKind::Schema
                | TokenKind::Codec
                | TokenKind::Effect
                | TokenKind::Handler
                | TokenKind::If
                | TokenKind::Match
        ) {
            continue;
        }
        if nested_blocks == 0 {
            return token.kind == TokenKind::Schema;
        }
        nested_blocks -= 1;
    }
    false
}

fn is_effect_reference_token(tokens: &[Token], index: usize) -> bool {
    is_effect_list_token(tokens, index) || is_handler_handled_effect_token(tokens, index)
}

fn is_effect_list_token(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && line_tokens_before(tokens, index)
            .iter()
            .any(|token| token.kind == TokenKind::Effects)
        && line_tokens_before(tokens, index)
            .iter()
            .any(|token| token.kind == TokenKind::LBracket)
        && tokens[index + 1..]
            .iter()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .any(|token| token.kind == TokenKind::RBracket)
}

fn is_handler_handled_effect_token(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Handles)
}

fn is_perform_effect_qualifier_token(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Perform)
        && next_non_layout_token(tokens, index)
            .is_some_and(|next| next.kind == TokenKind::DoubleColon)
}

fn is_perform_operation_token(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::DoubleColon)
        && next_non_whitespace_token(tokens, index)
            .is_some_and(|next| next.kind == TokenKind::LParen)
        && tokens[..index]
            .iter()
            .rev()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .any(|token| token.kind == TokenKind::Perform)
}

fn is_handler_reference_token(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::With)
        && next_non_whitespace_token(tokens, index)
            .is_some_and(|next| next.kind == TokenKind::LParen)
        && tokens[..index]
            .iter()
            .rev()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .any(|token| token.kind == TokenKind::Handle)
}

fn is_handler_operation_clause_call_target(tokens: &[Token], index: usize) -> bool {
    is_call_target_token(tokens, index)
        && inside_handler_operation_clause_body(tokens, tokens[index].range.start)
}

fn is_handler_operation_clause_operation_name(tokens: &[Token], index: usize) -> bool {
    tokens
        .get(index)
        .is_some_and(|token| token.kind == TokenKind::Ident && is_identifier(&token.text))
        && tokens[index + 1..]
            .iter()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .position(|token| token.kind == TokenKind::FatArrow)
            .map(|relative_index| index + 1 + relative_index)
            .is_some_and(|arrow_index| {
                is_handler_operation_clause_arrow(tokens, arrow_index)
                    && line_tokens_before(tokens, arrow_index)
                        .iter()
                        .position(|token| {
                            !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline)
                        })
                        .is_some_and(|first_index| {
                            let line_start = line_start_index(tokens, arrow_index);
                            line_start + first_index == index
                        })
            })
}

fn inside_handler_operation_clause_body(tokens: &[Token], offset: usize) -> bool {
    let file_end = tokens.last().map_or(offset, |token| token.range.end);
    tokens.iter().enumerate().any(|(arrow_index, arrow)| {
        arrow.kind == TokenKind::FatArrow
            && is_handler_operation_clause_arrow(tokens, arrow_index)
            && offset >= arrow.range.end
            && offset < handler_operation_clause_body_end(tokens, arrow_index, file_end)
    })
}

fn is_handler_operation_clause_arrow(tokens: &[Token], arrow_index: usize) -> bool {
    if !inside_top_level_block(tokens, arrow_index, TokenKind::Handler) {
        return false;
    }
    let line_tokens = line_tokens_before(tokens, arrow_index);
    line_tokens
        .iter()
        .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .is_some_and(|token| token.kind == TokenKind::Ident && is_identifier(&token.text))
        && line_tokens
            .iter()
            .any(|token| token.kind == TokenKind::LParen)
        && line_tokens
            .iter()
            .any(|token| token.kind == TokenKind::RParen)
}

fn line_tokens_before(tokens: &[Token], index: usize) -> &[Token] {
    &tokens[line_start_index(tokens, index)..index]
}

fn next_non_whitespace_token(tokens: &[Token], index: usize) -> Option<&Token> {
    tokens[index + 1..]
        .iter()
        .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .find(|token| token.kind != TokenKind::Whitespace)
}

fn inside_codec_declaration(tokens: &[Token], index: usize) -> bool {
    inside_top_level_block(tokens, index, TokenKind::Codec)
}

fn inside_top_level_block(tokens: &[Token], index: usize, start_kind: TokenKind) -> bool {
    enclosing_top_level_block_index(tokens, index, start_kind).is_some()
}

fn enclosing_top_level_block_index(
    tokens: &[Token],
    index: usize,
    start_kind: TokenKind,
) -> Option<usize> {
    let mut nested_blocks = 0usize;
    for (candidate_index, token) in tokens[..index].iter().enumerate().rev() {
        match token.kind {
            TokenKind::End => nested_blocks += 1,
            kind if kind == start_kind && nested_blocks == 0 => return Some(candidate_index),
            TokenKind::Fn
            | TokenKind::Test
            | TokenKind::If
            | TokenKind::Match
            | TokenKind::Handler
            | TokenKind::Codec => nested_blocks = nested_blocks.saturating_sub(1),
            _ => {}
        }
    }
    None
}
