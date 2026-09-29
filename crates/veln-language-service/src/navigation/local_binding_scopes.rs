fn local_bindings(tokens: &[Token], body_start: usize, end: usize) -> Vec<LocalBinding> {
    let mut bindings: Vec<LocalBinding> = Vec::new();
    let mut scope_bindings = vec![Vec::new()];
    let mut pending_lets = Vec::new();
    let first_index = tokens.partition_point(|token| token.range.start < body_start);

    for (relative_index, token) in tokens[first_index..].iter().enumerate() {
        if token.range.start >= end {
            break;
        }
        record_local_binding_scope_token_visit();
        let index = first_index + relative_index;
        match token.kind {
            TokenKind::Let => record_pending_let(
                tokens,
                index,
                token,
                &mut bindings,
                &scope_bindings,
                &mut pending_lets,
            ),
            TokenKind::Newline => activate_pending_lets(
                token.range.end,
                &mut bindings,
                &mut scope_bindings,
                &mut pending_lets,
            ),
            _ => update_local_binding_scope(tokens, index, token, &mut bindings, &mut scope_bindings),
        }
    }

    for scope in &mut scope_bindings {
        close_local_binding_scope(&mut bindings, scope, end);
    }
    bindings.extend(match_arm_pattern_binding_names(tokens, body_start, end));
    bindings.extend(satisfy_candidate_binding_names(tokens, body_start, end));
    bindings
}

fn record_pending_let(
    tokens: &[Token],
    index: usize,
    token: &Token,
    bindings: &mut Vec<LocalBinding>,
    scope_bindings: &[Vec<usize>],
    pending_lets: &mut Vec<(usize, Vec<usize>)>,
) {
    let binding_indices = let_binding_names(tokens, index)
        .into_iter()
        .map(|(name, declaration_start, declaration_end)| {
            let binding_index = bindings.len();
            bindings.push(LocalBinding {
                name,
                declaration_start,
                declaration_end,
                start: token.range.end,
                end: usize::MAX,
                navigation_supported: true,
            });
            binding_index
        })
        .collect();
    pending_lets.push((scope_bindings.len(), binding_indices));
}

fn activate_pending_lets(
    start: usize,
    bindings: &mut [LocalBinding],
    scope_bindings: &mut [Vec<usize>],
    pending_lets: &mut Vec<(usize, Vec<usize>)>,
) {
    while pending_lets
        .last()
        .is_some_and(|(depth, _)| *depth == scope_bindings.len())
    {
        let (_, binding_indices) = pending_lets.pop().expect("pending let");
        for binding_index in binding_indices {
            bindings[binding_index].start = start;
            scope_bindings
                .last_mut()
                .expect("local binding scope")
                .push(binding_index);
        }
    }
}

fn update_local_binding_scope(
    tokens: &[Token],
    index: usize,
    token: &Token,
    bindings: &mut [LocalBinding],
    scope_bindings: &mut Vec<Vec<usize>>,
) {
    match token.kind {
        TokenKind::If if !is_else_if(tokens, index) => scope_bindings.push(Vec::new()),
        TokenKind::Match | TokenKind::Handler | TokenKind::Begin | TokenKind::Defer => {
            scope_bindings.push(Vec::new());
        }
        TokenKind::Else => close_local_binding_scope(
            bindings,
            scope_bindings.last_mut().expect("local binding scope"),
            token.range.start,
        ),
        TokenKind::End => {
            close_local_binding_scope(
                bindings,
                scope_bindings.last_mut().expect("local binding scope"),
                token.range.start,
            );
            if scope_bindings.len() > 1 {
                scope_bindings.pop();
            }
        }
        _ => {}
    }
}

fn close_local_binding_scope(
    bindings: &mut [LocalBinding],
    scope: &mut Vec<usize>,
    end: usize,
) {
    for binding_index in scope.drain(..) {
        bindings[binding_index].end = end;
    }
}

fn let_binding_names(tokens: &[Token], let_index: usize) -> Vec<(String, usize, usize)> {
    let mut names = let_pattern_binding_names(tokens, let_index)
        .into_iter()
        .collect::<Vec<_>>();
    if let Some((name, start, end)) = simple_let_binding_name(tokens, let_index)
        && !names
            .iter()
            .any(|(existing, _, _)| existing == &name)
    {
        names.push((name, start, end));
    }
    names
}

fn simple_let_binding_name(tokens: &[Token], let_index: usize) -> Option<(String, usize, usize)> {
    let token_index = next_non_layout_index(tokens, let_index)?;
    let token = &tokens[token_index];
    (token.kind == TokenKind::Ident
        && is_identifier(&token.text)
        && next_non_layout_token(tokens, token_index)
            .is_some_and(|next| matches!(next.kind, TokenKind::Colon | TokenKind::Equal)))
    .then(|| (token.text.clone(), token.range.start, token.range.end))
}
