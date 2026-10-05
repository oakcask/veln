fn valid_schema_operation_leaf_spans(syntax: &SyntaxTree) -> Vec<SourceSpan> {
    let mut spans = Vec::new();
    for item in &syntax.items {
        match item {
            SyntaxItem::Function(function) => {
                collect_body_schema_operation_leaf_spans(&function.body, &mut spans);
            }
            SyntaxItem::Handler(handler) => {
                for clause in &handler.operation_clauses {
                    collect_valid_schema_operation_leaf_spans(&clause.body, &mut spans);
                }
            }
            _ => {}
        }
    }
    spans
}

fn collect_body_schema_operation_leaf_spans(body: &[BodyLine], spans: &mut Vec<SourceSpan>) {
    for line in body {
        match line {
            BodyLine::Let { expr, .. } | BodyLine::Expr { expr, .. } => {
                collect_valid_schema_operation_leaf_spans(expr, spans);
            }
            BodyLine::Defer { body, .. } => {
                collect_body_schema_operation_leaf_spans(body, spans);
            }
        }
    }
}

fn valid_schema_composition_leaf_spans(
    source: &SourceFile,
    tokens: &[Token],
    parsed: &ParseOutput,
) -> Vec<SourceSpan> {
    if parsed
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.parser_context == "schema_field" && diagnostic.span.is_none())
    {
        return Vec::new();
    }
    let mut recovery_ranges = parsed
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.parser_context == "schema_field")
        .filter_map(|diagnostic| diagnostic.span.as_ref())
        .filter(|span| span.file == *source.path())
        .map(|span| (span.start.offset, span.end.offset))
        .collect::<Vec<_>>();
    recovery_ranges.sort_unstable();

    let mut spans = Vec::new();
    let mut token_cursor = 0usize;
    let mut recovery_cursor = 0usize;
    let mut recovery_end = 0usize;
    for field in parsed
        .tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Schema(schema) => Some(schema.fields.as_slice()),
            _ => None,
        })
        .flatten()
    {
        while token_cursor < tokens.len()
            && tokens[token_cursor].range.end <= field.span.start.offset
        {
            token_cursor += 1;
        }
        let field_start = token_cursor;
        while token_cursor < tokens.len()
            && tokens[token_cursor].range.start < field.span.end.offset
        {
            token_cursor += 1;
        }
        #[cfg(test)]
        record_schema_composition_field_token_visits(token_cursor - field_start);
        let Some(leaf_index) =
            schema_composition_path_leaf_in_field(tokens, field_start, token_cursor)
        else {
            continue;
        };
        let token = &tokens[leaf_index];
        while recovery_cursor < recovery_ranges.len()
            && recovery_ranges[recovery_cursor].0 <= token.range.start
        {
            recovery_end = recovery_end.max(recovery_ranges[recovery_cursor].1);
            recovery_cursor += 1;
        }
        if recovery_end >= token.range.end {
            continue;
        }
        spans.push(source.span(token.range));
    }
    spans
}

fn schema_composition_path_leaf_in_field(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Option<usize> {
    let mut significant = (start..end)
        .filter(|index| {
            !matches!(
                tokens[*index].kind,
                TokenKind::Whitespace | TokenKind::Comment | TokenKind::Newline
            )
        })
        .collect::<Vec<_>>();
    if let Some(where_position) = significant
        .iter()
        .position(|index| tokens[*index].kind == TokenKind::Where)
    {
        significant.truncate(where_position);
    }
    let colon_position = significant
        .iter()
        .position(|index| tokens[*index].kind == TokenKind::Colon)?;
    let field_type = &significant[colon_position + 1..];
    schema_path_leaf_in(field_type, tokens)
        .or_else(|| repeat_schema_path_leaf(field_type, tokens))
        .or_else(|| array_schema_path_leaf(field_type, tokens))
}

fn collect_valid_schema_operation_leaf_spans(expr: &Expr, spans: &mut Vec<SourceSpan>) {
    match &expr.kind {
        ExprKind::SchemaDecode {
            schema_spans,
            recovered,
            input,
            base,
            ..
        } => {
            if !recovered && let Some(leaf) = schema_spans.last() {
                spans.push(leaf.clone());
            }
            collect_valid_schema_operation_leaf_spans(input, spans);
            collect_valid_schema_operation_leaf_spans(base, spans);
        }
        ExprKind::SchemaEncode {
            schema_spans,
            recovered,
            value,
            ..
        } => {
            if !recovered && let Some(leaf) = schema_spans.last() {
                spans.push(leaf.clone());
            }
            collect_valid_schema_operation_leaf_spans(value, spans);
        }
        ExprKind::TypeApply { callee, .. }
        | ExprKind::FieldAccess { base: callee, .. }
        | ExprKind::Try { expr: callee, .. }
        | ExprKind::Prefix { expr: callee, .. } => {
            collect_valid_schema_operation_leaf_spans(callee, spans);
        }
        ExprKind::Call { callee, args } => {
            collect_valid_schema_operation_leaf_spans(callee, spans);
            for arg in args {
                collect_valid_schema_operation_leaf_spans(arg, spans);
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_valid_schema_operation_leaf_spans(arg, spans);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_valid_schema_operation_leaf_spans(body, spans);
            for arg in args {
                collect_valid_schema_operation_leaf_spans(arg, spans);
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_valid_schema_operation_leaf_spans(&field.expr, spans);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_valid_schema_operation_leaf_spans(&entry.key, spans);
                collect_valid_schema_operation_leaf_spans(&entry.value, spans);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_valid_schema_operation_leaf_spans(item, spans);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_valid_schema_operation_leaf_spans(scrutinee, spans);
            for arm in arms {
                collect_valid_schema_operation_leaf_spans(&arm.expr, spans);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_valid_schema_operation_leaf_spans(condition, spans);
            collect_valid_schema_operation_leaf_spans(then_branch, spans);
            for branch in else_if_branches {
                collect_valid_schema_operation_leaf_spans(&branch.condition, spans);
                collect_valid_schema_operation_leaf_spans(&branch.expr, spans);
            }
            collect_valid_schema_operation_leaf_spans(else_branch, spans);
        }
        ExprKind::Begin { body, .. } => collect_body_schema_operation_leaf_spans(body, spans),
        ExprKind::Binary { left, right, .. } => {
            collect_valid_schema_operation_leaf_spans(left, spans);
            collect_valid_schema_operation_leaf_spans(right, spans);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}
