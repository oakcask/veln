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
        && is_match_arm_pattern_binding_name(tokens, index)
        && previous_non_layout_token(tokens, index)
            .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
        && next_non_layout_token(tokens, index)
            .is_none_or(|next| !matches!(next.kind, TokenKind::DoubleColon | TokenKind::Colon))
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

fn is_schema_path_leaf_token(tokens: &[Token], index: usize) -> bool {
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
