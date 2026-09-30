fn local_binding_shadows_call_target(tokens: &[Token], index: usize, name: &str) -> bool {
    local_binding_shadows_call_target_in_scopes(&function_scopes(tokens), tokens, index, name)
}

fn local_binding_shadows_call_target_in_scopes(
    scopes: &[FunctionScope],
    tokens: &[Token],
    index: usize,
    name: &str,
) -> bool {
    local_binding_shadowing_call_target_in_scopes(scopes, tokens, index, name).is_some()
}

fn local_binding_shadowing_call_target_in_scopes<'a>(
    scopes: &'a [FunctionScope],
    tokens: &[Token],
    index: usize,
    name: &str,
) -> Option<ScopeShadow<'a>> {
    let offset = tokens[index].range.start;
    scopes
        .iter()
        .find(|scope| offset >= scope.body_start && offset < scope.end)
        .and_then(|scope| scope.shadowing_binding(name, tokens, index))
}

fn function_scopes(tokens: &[Token]) -> Vec<FunctionScope> {
    let defer_block_openers = defer_block_openers(tokens);
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
        let end = function_scope_end_with_defer_openers(tokens, index + 1, &defer_block_openers)
            .unwrap_or(body_start);
        let params = parameter_names(tokens, index, body_start);
        let result_binding = result_binding_name(tokens, index, body_start);
        let local_bindings =
            local_bindings_with_defer_openers(tokens, body_start, end, &defer_block_openers);
        let local_bindings_by_name = local_binding_index_by_name(&local_bindings);
        scopes.push(FunctionScope {
            body_start,
            end,
            params,
            result_binding,
            local_bindings,
            local_bindings_by_name,
        });
    }
    scopes.extend(handler_operation_clause_scopes(
        tokens,
        &defer_block_openers,
    ));
    scopes
}

fn handler_operation_clause_scopes(
    tokens: &[Token],
    defer_block_openers: &[bool],
) -> Vec<FunctionScope> {
    let file_end = tokens.last().map_or(0, |token| token.range.end);
    let clause_headers = handler_operation_clause_headers(tokens, defer_block_openers);
    clause_headers
        .iter()
        .enumerate()
        .filter_map(|(index, is_header)| is_header.then_some(index))
        .map(|arrow_index| {
            let arrow = &tokens[arrow_index];
            let body_start = arrow.range.end;
            let end = handler_operation_clause_body_end_with_defer_openers(
                tokens,
                arrow_index,
                file_end,
                defer_block_openers,
                &clause_headers,
            );
            let local_bindings = local_bindings_with_defer_openers(
                tokens,
                body_start,
                end,
                defer_block_openers,
            );
            let local_bindings_by_name = local_binding_index_by_name(&local_bindings);
            FunctionScope {
                body_start,
                end,
                params: Vec::new(),
                result_binding: None,
                local_bindings,
                local_bindings_by_name,
            }
        })
        .collect()
}

fn handler_operation_clause_headers(
    tokens: &[Token],
    defer_block_openers: &[bool],
) -> Vec<bool> {
    handler_operation_clause_headers_with(
        tokens,
        defer_block_openers,
        record_handler_clause_scope_token_visit,
    )
}

fn handler_operation_clause_headers_with(
    tokens: &[Token],
    defer_block_openers: &[bool],
    mut record_token_visit: impl FnMut(),
) -> Vec<bool> {
    let mut headers = vec![false; tokens.len()];
    let mut blocks = Vec::new();
    let mut line_has_non_whitespace = false;
    let mut first_line_token_is_identifier = None;
    let mut saw_lparen = false;
    let mut saw_rparen = false;
    let mut saw_fat_arrow = false;
    let mut previous_non_layout = None;

    for (index, token) in tokens.iter().enumerate() {
        record_token_visit();
        if token.kind == TokenKind::FatArrow
            && blocks.last() == Some(&TokenKind::Handler)
            && !saw_fat_arrow
            && first_line_token_is_identifier == Some(true)
            && saw_lparen
            && saw_rparen
        {
            headers[index] = true;
        }

        match token.kind {
            TokenKind::If if previous_non_layout != Some(TokenKind::Else) => {
                blocks.push(token.kind);
            }
            TokenKind::Fn | TokenKind::Test | TokenKind::Handler | TokenKind::Codec
                if !line_has_non_whitespace || previous_non_layout == Some(TokenKind::Pub) =>
            {
                blocks.push(token.kind)
            }
            TokenKind::Match | TokenKind::Begin => blocks.push(token.kind),
            TokenKind::Defer if defer_block_openers[index] => blocks.push(token.kind),
            TokenKind::End => {
                blocks.pop();
            }
            _ => {}
        }

        match token.kind {
            TokenKind::Newline => {
                line_has_non_whitespace = false;
                first_line_token_is_identifier = None;
                saw_lparen = false;
                saw_rparen = false;
                saw_fat_arrow = false;
            }
            TokenKind::Whitespace => {}
            TokenKind::LParen => {
                line_has_non_whitespace = true;
                first_line_token_is_identifier.get_or_insert(false);
                saw_lparen = true;
            }
            TokenKind::RParen => {
                line_has_non_whitespace = true;
                first_line_token_is_identifier.get_or_insert(false);
                saw_rparen = true;
            }
            TokenKind::FatArrow => {
                line_has_non_whitespace = true;
                first_line_token_is_identifier.get_or_insert(false);
                saw_fat_arrow = true;
            }
            _ => {
                line_has_non_whitespace = true;
                first_line_token_is_identifier
                    .get_or_insert(token.kind == TokenKind::Ident && is_identifier(&token.text));
            }
        }
        if !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline) {
            previous_non_layout = Some(token.kind);
        }
    }
    headers
}

impl FunctionScope {
    fn shadows(&self, name: &str, tokens: &[Token], index: usize) -> bool {
        self.shadowing_binding(name, tokens, index).is_some()
    }

    fn shadowing_binding(
        &self,
        name: &str,
        tokens: &[Token],
        index: usize,
    ) -> Option<ScopeShadow<'_>> {
        let offset = tokens[index].range.start;
        self.local_bindings_by_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|index| &self.local_bindings[*index])
            .filter(|binding| binding.start <= offset && offset < binding.end)
            .max_by_key(|binding| binding.declaration_start)
            .map(ScopeShadow::LocalBinding)
            .or_else(|| {
                self.params
                    .iter()
                    .find(|binding| binding.name == name)
                    .map(ScopeShadow::FunctionBinding)
            })
            .or_else(|| {
                self.result_binding
                    .as_ref()
                    .filter(|binding| binding.name == name && is_ensure_reference(tokens, index))
                    .map(ScopeShadow::FunctionBinding)
            })
    }
}

fn local_binding_index_by_name(bindings: &[LocalBinding]) -> BTreeMap<String, Vec<usize>> {
    let mut by_name = BTreeMap::new();
    for (index, binding) in bindings.iter().enumerate() {
        by_name
            .entry(binding.name.clone())
            .or_insert_with(Vec::new)
            .push(index);
    }
    by_name
}

enum ScopeShadow<'a> {
    FunctionBinding(&'a ScopedBinding),
    LocalBinding(&'a LocalBinding),
}

impl ScopeShadow<'_> {
    fn declaration_range(&self) -> (usize, usize) {
        match self {
            ScopeShadow::FunctionBinding(binding) => {
                (binding.declaration_start, binding.declaration_end)
            }
            ScopeShadow::LocalBinding(binding) => {
                (binding.declaration_start, binding.declaration_end)
            }
        }
    }
}

fn function_scope_end_with_defer_openers(
    tokens: &[Token],
    start: usize,
    defer_block_openers: &[bool],
) -> Option<usize> {
    let mut nested_blocks = Vec::new();
    for (relative_index, token) in tokens[start..].iter().enumerate() {
        let index = start + relative_index;
        match token.kind {
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks.push(token.kind),
            TokenKind::Match | TokenKind::Handler | TokenKind::Begin => {
                nested_blocks.push(token.kind)
            }
            TokenKind::Defer if defer_block_openers[index] => nested_blocks.push(token.kind),
            TokenKind::End
                if nested_blocks.is_empty()
                    || (nested_blocks
                        .last()
                        .is_some_and(|kind| matches!(kind, TokenKind::Begin | TokenKind::Defer))
                        && end_is_followed_by_top_level_item(tokens, index)) =>
            {
                return Some(token.range.start);
            }
            TokenKind::End => {
                nested_blocks.pop();
            }
            TokenKind::Eof => return None,
            _ => {}
        }
    }
    None
}

fn end_is_followed_by_top_level_item(tokens: &[Token], index: usize) -> bool {
    let Some(item_index) = next_non_trivia_index(tokens, index) else {
        return false;
    };
    match tokens[item_index].kind {
        TokenKind::Fn
        | TokenKind::Test
        | TokenKind::Type
        | TokenKind::Schema
        | TokenKind::Effect
        | TokenKind::Handler
        | TokenKind::Codec => true,
        TokenKind::Pub => next_non_trivia_index(tokens, item_index).is_some_and(|index| {
            matches!(
                tokens[index].kind,
                TokenKind::Fn
                    | TokenKind::Type
                    | TokenKind::Schema
                    | TokenKind::Effect
                    | TokenKind::Handler
                    | TokenKind::Codec
            )
        }),
        _ => false,
    }
}

fn next_non_trivia_index(tokens: &[Token], index: usize) -> Option<usize> {
    tokens[index + 1..]
        .iter()
        .position(|token| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Comment | TokenKind::Newline
            )
        })
        .map(|relative_index| index + 1 + relative_index)
}

fn parameter_names(tokens: &[Token], start: usize, body_start: usize) -> Vec<ScopedBinding> {
    let mut names = Vec::new();
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
                names.push(ScopedBinding {
                    name: token.text.clone(),
                    declaration_start: token.range.start,
                    declaration_end: token.range.end,
                });
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

fn result_binding_name(tokens: &[Token], start: usize, body_start: usize) -> Option<ScopedBinding> {
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
        .then(|| ScopedBinding {
            name: candidate.text.clone(),
            declaration_start: candidate.range.start,
            declaration_end: candidate.range.end,
        })
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

fn let_pattern_binding_names(tokens: &[Token], let_index: usize) -> Vec<(String, usize, usize)> {
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
                names.push((token.text.clone(), token.range.start, token.range.end));
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
    defer_block_openers: &[bool],
) -> Vec<LocalBinding> {
    struct MatchArmRange {
        pattern_start_index: usize,
        arrow_index: usize,
        scope_end: usize,
    }

    enum Block {
        Match { last_arm: Option<usize> },
        Other,
    }

    let first_index = tokens.partition_point(|token| token.range.start < body_start);
    let end_index = tokens.partition_point(|token| token.range.start < function_end);
    let mut previous_non_layout = vec![None; end_index - first_index];
    let mut last_non_layout = None;
    for (relative_index, token) in tokens[first_index..end_index].iter().enumerate() {
        record_local_binding_scope_token_visit();
        previous_non_layout[relative_index] = last_non_layout;
        if !is_layout_token_kind(token.kind) {
            last_non_layout = Some(first_index + relative_index);
        }
    }
    let mut next_non_layout = vec![None; end_index - first_index];
    let mut following_non_layout = None;
    for (relative_index, token) in tokens[first_index..end_index].iter().enumerate().rev() {
        record_local_binding_scope_token_visit();
        next_non_layout[relative_index] = following_non_layout;
        if !is_layout_token_kind(token.kind) {
            following_non_layout = Some(first_index + relative_index);
        }
    }
    let mut blocks = Vec::new();
    let mut arms: Vec<MatchArmRange> = Vec::new();
    let mut line_start_index = first_index;

    for (relative_index, token) in tokens[first_index..end_index].iter().enumerate() {
        record_local_binding_scope_token_visit();
        let index = first_index + relative_index;

        let previous_index = previous_non_layout[relative_index];
        let satisfy_arrow = token.kind == TokenKind::FatArrow
            && previous_index.is_some_and(|candidate_index| {
                tokens[candidate_index].kind == TokenKind::Ident
                    && previous_non_layout[candidate_index - first_index].is_some_and(
                        |satisfy_index| {
                            tokens[satisfy_index].kind == TokenKind::Ident
                                && tokens[satisfy_index].text == "satisfy"
                        },
                    )
            });
        if token.kind == TokenKind::FatArrow
            && !satisfy_arrow
            && let Some(Block::Match { last_arm }) = blocks.last_mut()
        {
            if let Some(previous_arm) = *last_arm {
                arms[previous_arm].scope_end = tokens[line_start_index].range.start;
            }
            let arm_index = arms.len();
            arms.push(MatchArmRange {
                pattern_start_index: line_start_index,
                arrow_index: index,
                scope_end: function_end,
            });
            *last_arm = Some(arm_index);
        }

        match token.kind {
            TokenKind::If
                if previous_index.is_none_or(|index| tokens[index].kind != TokenKind::Else) =>
            {
                blocks.push(Block::Other)
            }
            TokenKind::Match => blocks.push(Block::Match { last_arm: None }),
            TokenKind::Handler | TokenKind::Begin => blocks.push(Block::Other),
            TokenKind::Defer if defer_block_openers[index] => blocks.push(Block::Other),
            TokenKind::End => {
                if let Some(Block::Match {
                    last_arm: Some(last_arm),
                }) = blocks.pop()
                {
                    arms[last_arm].scope_end = token.range.start;
                }
            }
            _ => {}
        }

        if token.kind == TokenKind::Newline {
            line_start_index = index + 1;
        }
    }

    let mut bindings = Vec::new();
    for arm in arms {
        let first_pattern_token_start = tokens[arm.pattern_start_index..arm.arrow_index]
            .iter()
            .inspect(|_| record_local_binding_scope_token_visit())
            .find(|token| !is_layout_token_kind(token.kind))
            .map(|token| token.range.start);
        for index in arm.pattern_start_index..arm.arrow_index {
            record_local_binding_scope_token_visit();
            let token = &tokens[index];
            let relative_index = index - first_index;
            let is_binding = token.kind == TokenKind::Ident
                && is_identifier(&token.text)
                && token.text != "true"
                && token.text != "false"
                && previous_non_layout[relative_index]
                    .is_none_or(|previous| tokens[previous].kind != TokenKind::DoubleColon)
                && next_non_layout[relative_index].is_none_or(|next| {
                    !matches!(tokens[next].kind, TokenKind::DoubleColon | TokenKind::Colon)
                });
            if !is_binding {
                continue;
            }
            bindings.push(LocalBinding {
                name: token.text.clone(),
                declaration_start: token.range.start,
                declaration_end: token.range.end,
                start: tokens[arm.arrow_index].range.end,
                end: arm.scope_end,
                // Whole-pattern bindings retain their established unsupported
                // navigation boundary. Bindings nested in structured patterns
                // have an unambiguous declaration token and remain navigable.
                navigation_supported: first_pattern_token_start != Some(token.range.start),
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
    for (index, token) in function_body_tokens(tokens, body_start, function_end) {
        if token.kind != TokenKind::Ident || token.text != "satisfy" {
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
            declaration_start: candidate.range.start,
            declaration_end: candidate.range.end,
            start: tokens[arrow_index].range.end,
            end,
            navigation_supported: true,
        });
    }
    bindings
}

fn function_body_tokens(
    tokens: &[Token],
    body_start: usize,
    function_end: usize,
) -> impl Iterator<Item = (usize, &Token)> {
    let first_index = tokens.partition_point(|token| token.range.start < body_start);
    let end_index = tokens.partition_point(|token| token.range.start < function_end);
    tokens[first_index..end_index]
        .iter()
        .enumerate()
        .map(move |(relative_index, token)| {
            record_local_binding_scope_token_visit();
            (first_index + relative_index, token)
        })
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

#[derive(Clone, Copy)]
struct BlockContext {
    kind: TokenKind,
    delimiter_depths: (usize, usize, usize),
}

fn defer_block_openers(tokens: &[Token]) -> Vec<bool> {
    let mut openers = vec![false; tokens.len()];
    let mut blocks = Vec::new();
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    let mut braces = 0usize;
    let mut line_has_non_whitespace = false;
    let mut previous_non_layout = None;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Defer
            && !line_has_non_whitespace
            && blocks.last().is_some_and(|context: &BlockContext| {
                matches!(
                    context.kind,
                    TokenKind::Fn | TokenKind::Test | TokenKind::Begin | TokenKind::Defer
                )
                    && context.delimiter_depths == (parentheses, brackets, braces)
            })
        {
            openers[index] = true;
        }
        match token.kind {
            TokenKind::LParen => parentheses += 1,
            TokenKind::RParen => parentheses = parentheses.saturating_sub(1),
            TokenKind::LBracket => brackets += 1,
            TokenKind::RBracket => brackets = brackets.saturating_sub(1),
            TokenKind::LBrace => braces += 1,
            TokenKind::RBrace => braces = braces.saturating_sub(1),
            _ => {}
        }
        match token.kind {
            TokenKind::If if previous_non_layout != Some(TokenKind::Else) => {
                blocks.push(BlockContext {
                    kind: token.kind,
                    delimiter_depths: (parentheses, brackets, braces),
                })
            }
            TokenKind::Fn
            | TokenKind::Test
            | TokenKind::Type
            | TokenKind::Schema
            | TokenKind::Effect
            | TokenKind::Handler
            | TokenKind::Codec
                if parentheses == 0
                    && brackets == 0
                    && braces == 0
                    && (!line_has_non_whitespace
                        || previous_non_layout == Some(TokenKind::Pub)) =>
            {
                blocks.push(BlockContext {
                    kind: token.kind,
                    delimiter_depths: (parentheses, brackets, braces),
                })
            }
            TokenKind::Match | TokenKind::Begin => blocks.push(BlockContext {
                kind: token.kind,
                delimiter_depths: (parentheses, brackets, braces),
            }),
            TokenKind::Defer if openers[index] => blocks.push(BlockContext {
                kind: token.kind,
                delimiter_depths: (parentheses, brackets, braces),
            }),
            TokenKind::End => {
                blocks.pop();
            }
            _ => {}
        }
        match token.kind {
            TokenKind::Newline => line_has_non_whitespace = false,
            TokenKind::Whitespace => {}
            _ => line_has_non_whitespace = true,
        }
        if !is_layout_token_kind(token.kind) {
            previous_non_layout = Some(token.kind);
        }
    }
    openers
}

fn token_scope(scopes: &[FunctionScope], offset: usize) -> Option<&FunctionScope> {
    scopes
        .iter()
        .find(|scope| offset >= scope.body_start && offset < scope.end)
}
