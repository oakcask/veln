fn local_binding_shadows_call_target(tokens: &[Token], index: usize, name: &str) -> bool {
    let offset = tokens[index].range.start;
    function_scopes(tokens).iter().any(|scope| {
        offset >= scope.body_start && offset < scope.end && scope.shadows(name, tokens, index)
    })
}

fn function_scopes(tokens: &[Token]) -> Vec<FunctionScope> {
    let mut scopes = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token.kind, TokenKind::Fn | TokenKind::Test) {
            continue;
        }
        let Some(body_start) = tokens[index..]
            .iter()
            .find(|token| token.kind == TokenKind::Newline)
            .map(|token| token.range.end)
        else {
            continue;
        };
        let end = function_scope_end(tokens, index + 1).unwrap_or(body_start);
        let params = parameter_names(tokens, index, body_start);
        let result_binding = result_binding_name(tokens, index, body_start);
        let local_bindings = local_bindings(tokens, body_start, end);
        scopes.push(FunctionScope {
            body_start,
            end,
            params,
            result_binding,
            local_bindings,
        });
    }
    scopes
}

impl FunctionScope {
    fn shadows(&self, name: &str, tokens: &[Token], index: usize) -> bool {
        let offset = tokens[index].range.start;
        self.params.contains(name)
            || self
                .result_binding
                .as_deref()
                .is_some_and(|binding| binding == name && is_ensure_reference(tokens, index))
            || self.local_bindings.iter().any(|binding| {
                binding.name == name && binding.start <= offset && offset < binding.end
            })
    }
}

fn function_scope_end(tokens: &[Token], start: usize) -> Option<usize> {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[start..].iter().enumerate() {
        let index = start + relative_index;
        match token.kind {
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks += 1,
            TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::End if nested_blocks == 0 => return Some(token.range.start),
            TokenKind::End => nested_blocks -= 1,
            TokenKind::Eof => return None,
            _ => {}
        }
    }
    None
}

fn parameter_names(tokens: &[Token], start: usize, body_start: usize) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut depth = 0usize;
    let mut expect_parameter_name = false;
    for token in tokens[start..]
        .iter()
        .take_while(|token| token.range.start < body_start)
    {
        match token.kind {
            TokenKind::LParen => {
                depth += 1;
                if depth == 1 {
                    expect_parameter_name = true;
                }
            }
            TokenKind::RParen => {
                depth = depth.saturating_sub(1);
                expect_parameter_name = false;
            }
            TokenKind::Comma if depth == 1 => expect_parameter_name = true,
            TokenKind::Ident if depth == 1 && expect_parameter_name => {
                names.insert(token.text.clone());
                expect_parameter_name = false;
            }
            token_kind if !is_layout_token_kind(token_kind) && depth == 1 => {
                expect_parameter_name = false;
            }
            _ => {}
        }
    }
    names
}

fn result_binding_name(tokens: &[Token], start: usize, body_start: usize) -> Option<String> {
    let arrow_index = tokens[start..]
        .iter()
        .position(|token| token.kind == TokenKind::Arrow)
        .map(|index| start + index)?;
    if tokens[arrow_index].range.start >= body_start {
        return None;
    }
    let candidate_index = next_non_layout_index(tokens, arrow_index)?;
    let candidate = &tokens[candidate_index];
    if candidate.kind != TokenKind::Ident || !is_identifier(&candidate.text) {
        return None;
    }
    next_non_layout_token(tokens, candidate_index)
        .is_some_and(|next| next.kind == TokenKind::Colon)
        .then(|| candidate.text.clone())
}

fn local_bindings(tokens: &[Token], body_start: usize, end: usize) -> Vec<LocalBinding> {
    let mut bindings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.range.start < body_start
            || token.range.start >= end
            || token.kind != TokenKind::Let
        {
            continue;
        }
        let binding_end = local_binding_scope_end(tokens, index, end);
        let binding_start = let_binding_scope_start(tokens, index);
        bindings.extend(
            let_binding_names(tokens, index)
                .into_iter()
                .map(|name| LocalBinding {
                    name,
                    start: binding_start,
                    end: binding_end,
                }),
        );
    }
    bindings.extend(match_arm_pattern_binding_names(tokens, body_start, end));
    bindings.extend(satisfy_candidate_binding_names(tokens, body_start, end));
    bindings
}

fn let_binding_scope_start(tokens: &[Token], let_index: usize) -> usize {
    tokens[let_index + 1..]
        .iter()
        .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .last()
        .map(|token| token.range.end)
        .unwrap_or_else(|| tokens[let_index].range.end)
}

fn local_binding_scope_end(tokens: &[Token], let_index: usize, function_end: usize) -> usize {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[let_index + 1..].iter().enumerate() {
        let index = let_index + 1 + relative_index;
        if token.range.start >= function_end {
            break;
        }
        match token.kind {
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks += 1,
            TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::Else if nested_blocks == 0 => return token.range.start,
            TokenKind::End if nested_blocks == 0 => return token.range.start,
            TokenKind::End => nested_blocks -= 1,
            _ => {}
        }
    }
    function_end
}

fn let_binding_names(tokens: &[Token], let_index: usize) -> Vec<String> {
    let mut names = let_pattern_binding_names(tokens, let_index)
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    if let Some(name) = simple_let_binding_name(tokens, let_index)
        && !names.iter().any(|existing| existing == &name)
    {
        names.push(name);
    }
    names
}

fn simple_let_binding_name(tokens: &[Token], let_index: usize) -> Option<String> {
    let token_index = next_non_layout_index(tokens, let_index)?;
    let token = &tokens[token_index];
    (token.kind == TokenKind::Ident
        && is_identifier(&token.text)
        && next_non_layout_token(tokens, token_index)
            .is_some_and(|next| matches!(next.kind, TokenKind::Colon | TokenKind::Equal)))
    .then(|| token.text.clone())
}

fn local_binding_shadows_name(
    tokens: &[Token],
    name: &str,
    offset: usize,
    scope_start: usize,
    scope_end: usize,
) -> bool {
    local_bindings(tokens, scope_start, scope_end)
        .iter()
        .any(|binding| binding.name == name && offset >= binding.start && offset < binding.end)
}

fn handler_operation_clause_parameter_shadows_name(
    tokens: &[Token],
    name: &str,
    offset: usize,
    scope_start: usize,
    scope_end: usize,
) -> bool {
    if offset < scope_start || offset >= scope_end {
        return false;
    }
    let file_end = tokens.last().map_or(scope_end, |token| token.range.end);
    tokens.iter().enumerate().any(|(arrow_index, arrow)| {
        if arrow.kind != TokenKind::FatArrow
            || !is_handler_operation_clause_arrow(tokens, arrow_index)
        {
            return false;
        }
        let Some((lparen_index, rparen_index)) =
            handler_operation_clause_parameter_range(tokens, arrow_index)
        else {
            return false;
        };
        let body_end = handler_operation_clause_body_end(tokens, arrow_index, file_end);
        offset >= tokens[lparen_index].range.start
            && offset < body_end
            && handler_operation_clause_parameter_names_in_range(tokens, lparen_index, rparen_index)
                .contains(name)
    })
}

fn handler_operation_clause_parameter_range(
    tokens: &[Token],
    arrow_index: usize,
) -> Option<(usize, usize)> {
    let lparen_index = tokens[..arrow_index]
        .iter()
        .rposition(|token| token.kind == TokenKind::LParen)?;
    let rparen_index = tokens[lparen_index + 1..arrow_index]
        .iter()
        .position(|token| token.kind == TokenKind::RParen)
        .map(|index| lparen_index + 1 + index)?;
    Some((lparen_index, rparen_index))
}

fn handler_operation_clause_parameter_names_in_range(
    tokens: &[Token],
    lparen_index: usize,
    rparen_index: usize,
) -> BTreeSet<String> {
    tokens[lparen_index + 1..rparen_index]
        .iter()
        .filter(|token| token.kind == TokenKind::Ident && is_identifier(&token.text))
        .map(|token| token.text.clone())
        .collect()
}

fn let_pattern_binding_names(tokens: &[Token], let_index: usize) -> Vec<(String, usize)> {
    let mut names = Vec::new();
    let mut depth = 0usize;
    let mut index = let_index + 1;
    while index < tokens.len() {
        let token = &tokens[index];
        if token.kind == TokenKind::Eof || token.kind == TokenKind::Newline {
            break;
        }
        if depth == 0 && matches!(token.kind, TokenKind::Colon | TokenKind::Equal) {
            break;
        }
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Ident if is_pattern_binding_token(tokens, index) => {
                names.push((token.text.clone(), token.range.end));
            }
            _ => {}
        }
        index += 1;
    }
    names
}

fn match_arm_pattern_binding_names(
    tokens: &[Token],
    body_start: usize,
    function_end: usize,
) -> Vec<LocalBinding> {
    let mut bindings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.range.start < body_start
            || token.range.start >= function_end
            || token.kind != TokenKind::FatArrow
            || !inside_match(tokens, index, body_start)
        {
            continue;
        }
        let scope_start = token.range.end;
        let scope_end = match_arm_scope_end(tokens, index + 1, function_end);
        let pattern_start = match_arm_pattern_start(tokens, index, body_start);
        for name in pattern_binding_names_in_range(tokens, pattern_start, index) {
            bindings.push(LocalBinding {
                name,
                start: scope_start,
                end: scope_end,
            });
        }
    }
    bindings
}

fn satisfy_candidate_binding_names(
    tokens: &[Token],
    body_start: usize,
    function_end: usize,
) -> Vec<LocalBinding> {
    let mut bindings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.range.start < body_start
            || token.range.start >= function_end
            || token.kind != TokenKind::Ident
            || token.text != "satisfy"
        {
            continue;
        }
        let Some(candidate_index) = next_non_layout_index(tokens, index) else {
            continue;
        };
        let candidate = &tokens[candidate_index];
        if candidate.kind != TokenKind::Ident || !is_identifier(&candidate.text) {
            continue;
        }
        let Some(arrow_index) = next_non_layout_index(tokens, candidate_index) else {
            continue;
        };
        if tokens[arrow_index].kind != TokenKind::FatArrow {
            continue;
        }
        let end = tokens[arrow_index + 1..]
            .iter()
            .find(|token| token.kind == TokenKind::Newline || token.range.start >= function_end)
            .map(|token| token.range.start)
            .unwrap_or(function_end);
        bindings.push(LocalBinding {
            name: candidate.text.clone(),
            start: tokens[arrow_index].range.end,
            end,
        });
    }
    bindings
}

fn inside_match(tokens: &[Token], index: usize, body_start: usize) -> bool {
    let mut nested_blocks = 0usize;
    for token in tokens[..index]
        .iter()
        .rev()
        .take_while(|token| token.range.start >= body_start)
    {
        match token.kind {
            TokenKind::End => nested_blocks += 1,
            TokenKind::Match if nested_blocks == 0 => return true,
            TokenKind::If | TokenKind::Handler | TokenKind::Match => {
                nested_blocks = nested_blocks.saturating_sub(1);
            }
            _ => {}
        }
    }
    false
}

fn match_arm_scope_end(tokens: &[Token], start: usize, function_end: usize) -> usize {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[start..].iter().enumerate() {
        let index = start + relative_index;
        if token.range.start >= function_end {
            break;
        }
        match token.kind {
            TokenKind::If | TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::End if nested_blocks == 0 => return token.range.start,
            TokenKind::End => nested_blocks -= 1,
            TokenKind::FatArrow if nested_blocks == 0 && !is_satisfy_arrow(tokens, index) => {
                return match_arm_pattern_start_from_arrow(tokens, token.range.start);
            }
            _ => {}
        }
    }
    function_end
}

fn match_arm_pattern_start(tokens: &[Token], arrow_index: usize, body_start: usize) -> usize {
    tokens[..arrow_index]
        .iter()
        .rev()
        .take_while(|token| token.range.start >= body_start)
        .find(|token| token.kind == TokenKind::Newline || token.kind == TokenKind::Match)
        .map_or(body_start, |token| token.range.end)
}

fn match_arm_pattern_start_from_arrow(tokens: &[Token], arrow_start: usize) -> usize {
    tokens
        .iter()
        .position(|token| token.range.start == arrow_start)
        .map_or(arrow_start, |index| {
            match_arm_pattern_start(tokens, index, 0)
        })
}

fn pattern_binding_names_in_range(tokens: &[Token], start: usize, end_index: usize) -> Vec<String> {
    tokens[..end_index]
        .iter()
        .enumerate()
        .filter(|(_, token)| token.range.start >= start)
        .filter(|(index, token)| {
            token.kind == TokenKind::Ident && is_pattern_binding_token(tokens, *index)
        })
        .map(|(_, token)| token.text.clone())
        .collect()
}

fn is_pattern_binding_token(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token.kind == TokenKind::Ident
        && is_identifier(&token.text)
        && token.text != "true"
        && token.text != "false"
        && previous_non_layout_token(tokens, index)
            .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
        && next_non_layout_token(tokens, index)
            .is_none_or(|next| !matches!(next.kind, TokenKind::DoubleColon | TokenKind::Colon))
}

fn is_else_if(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|previous| previous.kind == TokenKind::Else)
}

fn token_scope(scopes: &[FunctionScope], offset: usize) -> Option<&FunctionScope> {
    scopes
        .iter()
        .find(|scope| offset >= scope.body_start && offset < scope.end)
}

