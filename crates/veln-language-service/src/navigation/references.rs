fn call_references(file: &IndexedFile, name: &str) -> Vec<SourceSpan> {
    let tokens = &file.tokens;
    let scopes = function_scopes(tokens);
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.text == name
                && is_identifier(&token.text)
                && previous_non_layout_token(tokens, *index)
                    .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
                && !is_field_name(tokens, *index)
                && !is_function_declaration_name(tokens, *index)
                && !is_parameter_name(tokens, *index)
                && !is_local_binding_name(tokens, *index)
                && !is_handler_operation_clause_operation_name(tokens, *index)
                && (token_scope(&scopes, token.range.start)
                    .is_some_and(|scope| {
                        !file.inside_handler_operation_clause_body(token.range.start)
                            && !scope.shadows(name, tokens, *index)
                    })
                    || handler_function_reference_is_unshadowed(
                        file, tokens, &scopes, *index, name,
                    )
                    || is_function_alias_target_reference(tokens, *index, name)
                    || is_codec_implementation_function_reference(tokens, *index, name))
        })
        .map(|(_, token)| file.source.span(token.range))
        .collect()
}

impl IndexedFile {
    fn type_reference_spans(&self, name: &str) -> Vec<(usize, SourceSpan)> {
        if !name
            .chars()
            .next()
            .is_some_and(|initial| initial.is_ascii_uppercase())
        {
            return Vec::new();
        }
        self.type_reference_spans_named(name)
    }

    fn type_reference_spans_named(&self, name: &str) -> Vec<(usize, SourceSpan)> {
        self.type_reference_locations()
            .iter()
            .filter(|(candidate, _, _)| candidate == name)
            .map(|(_, token_index, span)| (*token_index, span.clone()))
            .collect()
    }

    fn type_reference_locations(&self) -> &TypeReferenceLocations {
        self.type_reference_locations.get_or_init(|| {
            #[cfg(test)]
            record_type_reference_collection();
            collect_type_reference_locations(&self.source, &self.tokens)
        })
    }
}

fn collect_type_reference_locations(
    source: &SourceFile,
    tokens: &[Token],
) -> TypeReferenceLocations {
    let parsed = parse(source);
    let mut spans: TypeReferenceLocations = parsed
        .tree
        .items
        .iter()
        .flat_map(|item| type_reference_locations_in_item(source, tokens, item))
        .collect();
    normalize_type_reference_locations(&mut spans);
    spans
}

fn type_reference_locations_in_item(
    source: &SourceFile,
    tokens: &[Token],
    item: &SyntaxItem,
) -> TypeReferenceLocations {
    match item {
        SyntaxItem::Function(function) => type_references_in_function(source, tokens, function),
        SyntaxItem::Handler(handler) => type_references_in_handler(source, tokens, handler),
        SyntaxItem::Effect(effect) => effect
            .operations
            .iter()
            .flat_map(|operation| {
                type_references_in_params(source, tokens, &operation.params)
                    .into_iter()
                    .chain(type_references_after_token_in_span(
                        source,
                        tokens,
                        &operation.span,
                        TokenKind::Arrow,
                    ))
            })
            .collect(),
        SyntaxItem::Type(type_decl) => type_decl
            .variants
            .iter()
            .flat_map(|variant| {
                variant
                    .fields
                    .iter()
                    .flat_map(|field| type_references_in_variant_field(source, tokens, &field.span))
            })
            .collect(),
        SyntaxItem::PublicAlias(alias) if alias.kind == PublicAliasKind::Type => alias
            .target_spans
            .iter()
            .flat_map(|span| type_reference_tokens_in_span(source, tokens, span))
            .collect(),
        _ => Vec::new(),
    }
}

fn type_references_in_function(
    source: &SourceFile,
    tokens: &[Token],
    function: &FunctionDecl,
) -> TypeReferenceLocations {
    let mut spans = type_references_in_params(source, tokens, &function.params);
    if let Some(span) = &function.return_type_span {
        spans.extend(type_reference_tokens_in_span(source, tokens, span));
    }
    spans.extend(type_references_in_body_lines(
        source,
        tokens,
        &function.body,
    ));
    spans
}

fn type_references_in_handler(
    source: &SourceFile,
    tokens: &[Token],
    handler: &veln_syntax::HandlerDecl,
) -> TypeReferenceLocations {
    let mut spans = type_references_in_params(source, tokens, &handler.params);
    for clause in &handler.operation_clauses {
        spans.extend(type_references_in_params(source, tokens, &clause.params));
        spans.extend(type_references_in_cleanup_expr(
            source,
            tokens,
            &clause.body,
        ));
    }
    spans
}

fn normalize_type_reference_locations(spans: &mut TypeReferenceLocations) {
    spans.sort_by(|left, right| {
        left.2
            .file
            .as_str()
            .cmp(right.2.file.as_str())
            .then(left.2.start.offset.cmp(&right.2.start.offset))
            .then(left.2.end.offset.cmp(&right.2.end.offset))
    });
    spans.dedup_by(|left, right| {
        left.0 == right.0
            && left.2.file == right.2.file
            && left.2.start.offset == right.2.start.offset
            && left.2.end.offset == right.2.end.offset
    });
}

fn is_type_reference_token(file: &IndexedFile, name: &str, selection: &SourceSpan) -> bool {
    file.type_reference_spans(name)
        .into_iter()
        .any(|(_, span)| {
            span.file == selection.file
                && span.start.offset == selection.start.offset
                && span.end.offset == selection.end.offset
        })
}

fn is_type_reference_token_named(file: &IndexedFile, name: &str, selection: &SourceSpan) -> bool {
    file.type_reference_spans_named(name)
        .into_iter()
        .any(|(_, span)| {
            span.file == selection.file
                && span.start.offset == selection.start.offset
                && span.end.offset == selection.end.offset
        })
}

fn type_references_in_params(
    source: &SourceFile,
    tokens: &[Token],
    params: &[veln_syntax::Param],
) -> TypeReferenceLocations {
    params
        .iter()
        .filter_map(|param| param.ty_span.as_ref())
        .flat_map(|span| type_reference_tokens_in_span(source, tokens, span))
        .collect()
}

fn type_references_in_body_lines(
    source: &SourceFile,
    tokens: &[Token],
    body: &[BodyLine],
) -> TypeReferenceLocations {
    collect_type_references_in_cleanup(
        source,
        tokens,
        body.iter().rev().map(CleanupReferenceWork::BodyLine),
    )
}

fn type_references_in_cleanup_expr(
    source: &SourceFile,
    tokens: &[Token],
    expr: &Expr,
) -> TypeReferenceLocations {
    collect_type_references_in_cleanup(
        source,
        tokens,
        std::iter::once(CleanupReferenceWork::Expr(expr)),
    )
}

enum CleanupReferenceWork<'a> {
    BodyLine(&'a BodyLine),
    Expr(&'a Expr),
}

fn collect_type_references_in_cleanup<'a>(
    source: &SourceFile,
    tokens: &[Token],
    initial: impl IntoIterator<Item = CleanupReferenceWork<'a>>,
) -> TypeReferenceLocations {
    let mut spans = Vec::new();
    let mut pending = initial.into_iter().collect::<Vec<_>>();
    while let Some(work) = pending.pop() {
        match work {
            CleanupReferenceWork::BodyLine(line) => {
                collect_type_references_in_cleanup_line(
                    source,
                    tokens,
                    line,
                    &mut spans,
                    &mut pending,
                );
            }
            CleanupReferenceWork::Expr(expr) => match &expr.kind {
                ExprKind::Begin { body, .. } => {
                    pending.extend(body.iter().rev().map(CleanupReferenceWork::BodyLine));
                }
                _ => queue_cleanup_expr_children(expr, &mut pending),
            },
        }
    }
    spans
}

fn queue_cleanup_expr_children<'a>(
    expr: &'a Expr,
    pending: &mut Vec<CleanupReferenceWork<'a>>,
) {
    match &expr.kind {
        ExprKind::TypeApply { callee, .. }
        | ExprKind::SchemaEncode { value: callee, .. }
        | ExprKind::FieldAccess { base: callee, .. }
        | ExprKind::Try { expr: callee, .. }
        | ExprKind::Prefix { expr: callee, .. } => {
            pending.push(CleanupReferenceWork::Expr(callee));
        }
        ExprKind::Call { callee, args }
        | ExprKind::Handle {
            body: callee, args, ..
        } => {
            pending.push(CleanupReferenceWork::Expr(callee));
            pending.extend(args.iter().map(CleanupReferenceWork::Expr));
        }
        ExprKind::Perform { args, .. } | ExprKind::List(args) => {
            pending.extend(args.iter().map(CleanupReferenceWork::Expr));
        }
        ExprKind::SchemaDecode { input, base, .. }
        | ExprKind::Binary {
            left: input,
            right: base,
            ..
        } => {
            pending.push(CleanupReferenceWork::Expr(input));
            pending.push(CleanupReferenceWork::Expr(base));
        }
        _ => queue_structured_cleanup_expr_children(expr, pending),
    }
}

fn queue_structured_cleanup_expr_children<'a>(
    expr: &'a Expr,
    pending: &mut Vec<CleanupReferenceWork<'a>>,
) {
    match &expr.kind {
        ExprKind::Record(fields) => pending.extend(
            fields
                .iter()
                .map(|field| CleanupReferenceWork::Expr(&field.expr)),
        ),
        ExprKind::Dict(entries) => {
            for entry in entries {
                pending.push(CleanupReferenceWork::Expr(&entry.key));
                pending.push(CleanupReferenceWork::Expr(&entry.value));
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            pending.push(CleanupReferenceWork::Expr(scrutinee));
            pending.extend(
                arms.iter()
                    .map(|arm| CleanupReferenceWork::Expr(&arm.expr)),
            );
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            pending.push(CleanupReferenceWork::Expr(condition));
            pending.push(CleanupReferenceWork::Expr(then_branch));
            for branch in else_if_branches {
                pending.push(CleanupReferenceWork::Expr(&branch.condition));
                pending.push(CleanupReferenceWork::Expr(&branch.expr));
            }
            pending.push(CleanupReferenceWork::Expr(else_branch));
        }
        ExprKind::Begin { .. }
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
        ExprKind::TypeApply { .. }
        | ExprKind::SchemaEncode { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try { .. }
        | ExprKind::Prefix { .. }
        | ExprKind::Call { .. }
        | ExprKind::Handle { .. }
        | ExprKind::Perform { .. }
        | ExprKind::List(_)
        | ExprKind::SchemaDecode { .. }
        | ExprKind::Binary { .. } => unreachable!("linear expression children handled first"),
    }
}

fn collect_type_references_in_cleanup_line<'a>(
    source: &SourceFile,
    tokens: &[Token],
    line: &'a BodyLine,
    spans: &mut TypeReferenceLocations,
    pending: &mut Vec<CleanupReferenceWork<'a>>,
) {
    match line {
        BodyLine::Let {
            annotation,
            expr,
            span,
            ..
        } => {
            if annotation.is_some() {
                spans.extend(type_references_after_token_until_token_in_span(
                    source,
                    tokens,
                    span,
                    TokenKind::Colon,
                    TokenKind::Equal,
                ));
            }
            pending.push(CleanupReferenceWork::Expr(expr));
        }
        BodyLine::Expr { expr, .. } => pending.push(CleanupReferenceWork::Expr(expr)),
        BodyLine::Defer { body, .. } => {
            pending.extend(body.iter().rev().map(CleanupReferenceWork::BodyLine));
        }
    }
}

fn type_references_in_variant_field(
    source: &SourceFile,
    tokens: &[Token],
    field_span: &SourceSpan,
) -> TypeReferenceLocations {
    let (field_start, field_end) = token_indices_in_range(
        tokens,
        field_span.start.offset,
        field_span.end.offset,
    );
    let colon_offset = tokens[field_start..field_end]
        .iter()
        .find(|token| {
            record_type_reference_token_visit();
            token.kind == TokenKind::Colon
        })
        .map(|token| token.range.end)
        .unwrap_or(field_span.start.offset);
    type_reference_tokens_in_range(source, tokens, colon_offset, field_span.end.offset)
}

fn type_references_after_token_in_span(
    source: &SourceFile,
    tokens: &[Token],
    span: &SourceSpan,
    start_kind: TokenKind,
) -> TypeReferenceLocations {
    let (span_start, span_end) = token_indices_in_range(
        tokens,
        span.start.offset,
        span.end.offset,
    );
    let start_offset = tokens[span_start..span_end]
        .iter()
        .find(|token| {
            record_type_reference_token_visit();
            token.kind == start_kind
        })
        .map(|token| token.range.end)
        .unwrap_or(span.end.offset);
    type_reference_tokens_in_range(source, tokens, start_offset, span.end.offset)
}

fn type_references_after_token_until_token_in_span(
    source: &SourceFile,
    tokens: &[Token],
    span: &SourceSpan,
    start_kind: TokenKind,
    end_kind: TokenKind,
) -> TypeReferenceLocations {
    let (span_start, span_end) = token_indices_in_range(
        tokens,
        span.start.offset,
        span.end.offset,
    );
    let Some(relative_start_index) = tokens[span_start..span_end].iter().position(|token| {
        record_type_reference_token_visit();
        token.kind == start_kind
    }) else {
        return Vec::new();
    };
    let start_index = span_start + relative_start_index;
    let start_offset = tokens[start_index].range.end;
    let end_offset = tokens[start_index + 1..]
        .iter()
        .take(span_end.saturating_sub(start_index + 1))
        .find(|token| {
            record_type_reference_token_visit();
            token.kind == end_kind
        })
        .map(|token| token.range.start)
        .unwrap_or(span.end.offset);
    type_reference_tokens_in_range(source, tokens, start_offset, end_offset)
}

fn type_reference_tokens_in_span(
    source: &SourceFile,
    tokens: &[Token],
    span: &SourceSpan,
) -> TypeReferenceLocations {
    type_reference_tokens_in_range(source, tokens, span.start.offset, span.end.offset)
}

fn type_reference_tokens_in_range(
    source: &SourceFile,
    tokens: &[Token],
    start_offset: usize,
    end_offset: usize,
) -> TypeReferenceLocations {
    let (start_index, end_index) = token_indices_in_range(tokens, start_offset, end_offset);
    tokens[start_index..end_index]
        .iter()
        .enumerate()
        .filter(|(_, token)| {
            record_type_reference_token_visit();
            token.kind == TokenKind::Ident
        })
        .map(|(index, token)| {
            (
                token.text.clone(),
                start_index + index,
                source.span(token.range),
            )
        })
        .collect()
}

fn token_indices_in_range(
    tokens: &[Token],
    start_offset: usize,
    end_offset: usize,
) -> (usize, usize) {
    let start_index = tokens.partition_point(|token| token.range.start < start_offset);
    let end_index = tokens.partition_point(|token| token.range.end <= end_offset);
    (start_index.min(end_index), end_index)
}
