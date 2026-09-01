fn handler_operation_clause_symbol(
    file: &IndexedFile,
    tokens: &[Token],
    token_index: usize,
    name: &str,
    selection: &SourceSpan,
) -> Option<LocalSymbol> {
    handler_operation_clause_bindings(file, tokens)
        .into_iter()
        .find(|binding| {
            let token_offset = tokens[token_index].range.start;
            binding.name == name
                && !is_invalid_declaration_name(file, &binding.declaration)
                && ((selection.start.offset >= binding.declaration.start.offset
                    && selection.start.offset < binding.declaration.end.offset)
                    || (token_offset >= binding.start
                        && token_offset < binding.end
                        && (binding.kind != LocalSymbolKind::HandlerContextParameter
                            || inside_handler_operation_clause_body(tokens, token_offset))
                        && !local_binding_shadows_name(
                            tokens,
                            &binding.name,
                            token_offset,
                            binding.start,
                            binding.end,
                        )))
        })
        .map(|binding| LocalSymbol {
            name: binding.name,
            declaration: binding.declaration,
            scope_file: file.source.path().as_str().to_string(),
            scope_start: binding.start,
            scope_end: binding.end,
            kind: binding.kind,
        })
}

fn handler_operation_clause_bindings(file: &IndexedFile, tokens: &[Token]) -> Vec<ClauseBinding> {
    handler_operation_clause_bindings_for_source(&file.source, tokens)
}

fn handler_operation_clause_bindings_for_source(
    source: &SourceFile,
    tokens: &[Token],
) -> Vec<ClauseBinding> {
    let mut clause_bindings = Vec::new();
    for (arrow_index, arrow) in tokens.iter().enumerate() {
        if arrow.kind != TokenKind::FatArrow
            || !inside_top_level_block(tokens, arrow_index, TokenKind::Handler)
        {
            continue;
        }
        let line_start_index = line_start_index(tokens, arrow_index);
        let body_end = handler_operation_clause_body_end(tokens, arrow_index, source.text().len());
        let Some(lparen_index) = tokens[line_start_index..arrow_index]
            .iter()
            .position(|token| token.kind == TokenKind::LParen)
            .map(|index| line_start_index + index)
        else {
            continue;
        };
        let Some(rparen_index) = tokens[lparen_index + 1..arrow_index]
            .iter()
            .position(|token| token.kind == TokenKind::RParen)
            .map(|index| lparen_index + 1 + index)
        else {
            continue;
        };
        for token in &tokens[lparen_index + 1..rparen_index] {
            if token.kind == TokenKind::Ident && is_identifier(&token.text) {
                clause_bindings.push(ClauseBinding {
                    name: token.text.clone(),
                    declaration: source.span(token.range),
                    start: arrow.range.end,
                    end: body_end,
                    kind: LocalSymbolKind::HandlerOperationClauseParameter,
                });
            }
        }
    }
    clause_bindings.extend(handler_context_parameter_bindings_for_source(source, tokens));
    clause_bindings
}

fn handler_context_parameter_bindings_for_source(
    source: &SourceFile,
    tokens: &[Token],
) -> Vec<ClauseBinding> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.kind == TokenKind::Handler)
        .flat_map(|(handler_index, _)| {
            handler_context_parameter_bindings_for_handler(source, tokens, handler_index)
        })
        .collect()
}

fn handler_context_parameter_bindings_for_handler(
    source: &SourceFile,
    tokens: &[Token],
    handler_index: usize,
) -> Vec<ClauseBinding> {
    let Some(body_start) = tokens[handler_index..]
        .iter()
        .find(|token| token.kind == TokenKind::Newline)
        .map(|token| token.range.end)
    else {
        return Vec::new();
    };
    let handler_end = function_scope_end(tokens, handler_index + 1).unwrap_or(body_start);
    let Some(lparen_index) = tokens[handler_index..]
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
        .map(|index| handler_index + index)
    else {
        return Vec::new();
    };
    let Some(rparen_index) = matching_rparen_index(tokens, lparen_index, tokens.len()) else {
        return Vec::new();
    };
    tokens[lparen_index + 1..rparen_index]
        .iter()
        .enumerate()
        .filter(|(_, token)| token.kind == TokenKind::Ident && is_identifier(&token.text))
        .filter(|(relative_index, _)| {
            let index = lparen_index + 1 + relative_index;
            next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
        })
        .map(|(_, token)| ClauseBinding {
            name: token.text.clone(),
            declaration: source.span(token.range),
            start: body_start,
            end: handler_end,
            kind: LocalSymbolKind::HandlerContextParameter,
        })
        .collect()
}

fn handler_operation_clause_body_end(
    tokens: &[Token],
    arrow_index: usize,
    file_end: usize,
) -> usize {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[arrow_index + 1..].iter().enumerate() {
        let index = arrow_index + 1 + relative_index;
        match token.kind {
            TokenKind::Eof => return file_end,
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks += 1,
            TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::End if nested_blocks == 0 => return token.range.start,
            TokenKind::End => nested_blocks = nested_blocks.saturating_sub(1),
            TokenKind::FatArrow if nested_blocks == 0 && !is_satisfy_arrow(tokens, index) => {
                return match_arm_pattern_start_from_arrow(tokens, token.range.start);
            }
            _ => {}
        }
    }
    file_end
}

fn line_start_index(tokens: &[Token], index: usize) -> usize {
    tokens[..index]
        .iter()
        .rposition(|token| token.kind == TokenKind::Newline)
        .map_or(0, |index| index + 1)
}

fn matching_rparen_index(tokens: &[Token], lparen_index: usize, end_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (relative_index, token) in tokens[lparen_index..end_index].iter().enumerate() {
        let index = lparen_index + relative_index;
        match token.kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}
