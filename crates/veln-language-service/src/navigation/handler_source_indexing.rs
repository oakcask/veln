struct HandlerDiagnosticIndex {
    ranges: Vec<(usize, usize)>,
    range_prefix_max_ends: Vec<usize>,
    points: Vec<usize>,
}

impl HandlerDiagnosticIndex {
    fn new(parsed: &ParseOutput) -> Self {
        let mut ranges = Vec::new();
        let mut points = Vec::new();
        for diagnostic in &parsed.diagnostics {
            #[cfg(test)]
            record_handler_diagnostic_index_visit();
            let Some(span) = diagnostic.span.as_ref() else {
                continue;
            };
            if span.start.offset == span.end.offset {
                points.push(span.start.offset);
            } else {
                ranges.push((span.start.offset, span.end.offset));
            }
        }
        ranges.sort_unstable_by_key(|range| range.0);
        points.sort_unstable();
        let mut greatest_end = 0;
        let range_prefix_max_ends = ranges
            .iter()
            .map(|range| {
                greatest_end = greatest_end.max(range.1);
                greatest_end
            })
            .collect();
        Self {
            ranges,
            range_prefix_max_ends,
            points,
        }
    }

    fn overlaps(&self, span: &SourceSpan) -> bool {
        #[cfg(test)]
        record_handler_diagnostic_overlap_query();
        let range_count = self
            .ranges
            .partition_point(|range| range.0 < span.end.offset);
        if range_count > 0
            && self.range_prefix_max_ends[range_count - 1] > span.start.offset
        {
            return true;
        }
        let point_index = self
            .points
            .partition_point(|point| *point < span.start.offset);
        self.points
            .get(point_index)
            .is_some_and(|point| *point <= span.end.offset)
    }
}

fn recovered_handler_declarations(
    parsed: &ParseOutput,
    diagnostics: &HandlerDiagnosticIndex,
) -> Vec<SourceSpan> {
    parsed
        .tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Handler(handler)
                if !handler.end_present || diagnostics.overlaps(&handler.span) =>
            {
                Some(handler.span.clone())
            }
            _ => None,
        })
        .collect()
}

fn valid_handler_operation_clause_references(
    parsed: &ParseOutput,
    recovered_handlers: &[SourceSpan],
) -> Vec<HandlerOperationClauseReference> {
    let mut references = Vec::new();
    let recovered_ranges = recovered_handlers
        .iter()
        .map(|span| (span.start.offset, span.end.offset))
        .collect::<BTreeSet<_>>();
    for item in &parsed.tree.items {
        let SyntaxItem::Handler(handler) = item else {
            continue;
        };
        let (Some(handler_name), [effect_name]) =
            (handler.name.as_ref(), handler.effect.as_slice())
        else {
            continue;
        };
        if handler.effect_recovered
            || !handler.end_present
            || recovered_ranges.contains(&(handler.span.start.offset, handler.span.end.offset))
        {
            continue;
        }
        let mut operation_counts = BTreeMap::<&str, usize>::new();
        for clause in &handler.operation_clauses {
            if let Some(operation) = clause.operation.as_deref() {
                *operation_counts.entry(operation).or_default() += 1;
            }
        }
        references.extend(handler.operation_clauses.iter().filter_map(|clause| {
            let operation_name = clause.operation.as_ref()?;
            (operation_counts.get(operation_name.as_str()) == Some(&1)).then(|| {
                HandlerOperationClauseReference {
                    handler_name: handler_name.clone(),
                    effect_name: effect_name.clone(),
                    operation_name: operation_name.clone(),
                    span: clause.operation_span.clone(),
                }
            })
        }));
    }
    references
}

fn valid_handler_reference_ranges(
    parsed: &ParseOutput,
    tokens: &[Token],
    argument_delimiters: &HandlerArgumentDelimiters,
    diagnostics: &HandlerDiagnosticIndex,
) -> BTreeSet<(usize, usize)> {
    let mut ranges = BTreeSet::new();
    for item in &parsed.tree.items {
        match item {
            SyntaxItem::Function(function) => {
                collect_body_handler_reference_ranges(
                    &function.body,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    &mut ranges,
                );
            }
            SyntaxItem::Handler(handler) => {
                for clause in &handler.operation_clauses {
                    collect_handler_reference_ranges(
                        &clause.body,
                        diagnostics,
                        tokens,
                        argument_delimiters,
                        &mut ranges,
                    );
                }
            }
            _ => {}
        }
    }
    ranges
}

fn collect_body_handler_reference_ranges(
    body: &[BodyLine],
    diagnostics: &HandlerDiagnosticIndex,
    tokens: &[Token],
    argument_delimiters: &HandlerArgumentDelimiters,
    ranges: &mut BTreeSet<(usize, usize)>,
) {
    for line in body {
        match line {
            BodyLine::Let { expr, .. } | BodyLine::Expr { expr, .. } => {
                collect_handler_reference_ranges(
                    expr,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    ranges,
                );
            }
            BodyLine::Defer { body, .. } => collect_body_handler_reference_ranges(
                body,
                diagnostics,
                tokens,
                argument_delimiters,
                ranges,
            ),
        }
    }
}

fn collect_handler_reference_ranges(
    expr: &Expr,
    diagnostics: &HandlerDiagnosticIndex,
    tokens: &[Token],
    argument_delimiters: &HandlerArgumentDelimiters,
    ranges: &mut BTreeSet<(usize, usize)>,
) {
    if let ExprKind::Handle {
        handler,
        handler_span,
        ..
    } = &expr.kind
        && handler.len() == 1
        && !diagnostics.overlaps(&expr.span)
        && argument_delimiters.complete_arguments_follow(handler_span, &expr.span, tokens)
    {
        ranges.insert((handler_span.start.offset, handler_span.end.offset));
    }

    match &expr.kind {
        ExprKind::TypeApply { callee, .. }
        | ExprKind::FieldAccess { base: callee, .. }
        | ExprKind::Try { expr: callee, .. }
        | ExprKind::Prefix { expr: callee, .. }
        | ExprKind::SchemaEncode { value: callee, .. } => {
            collect_handler_reference_ranges(
                callee,
                diagnostics,
                tokens,
                argument_delimiters,
                ranges,
            );
        }
        ExprKind::Call { callee, args } | ExprKind::Handle { body: callee, args, .. } => {
            collect_handler_reference_ranges(
                callee,
                diagnostics,
                tokens,
                argument_delimiters,
                ranges,
            );
            for arg in args {
                collect_handler_reference_ranges(
                    arg,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    ranges,
                );
            }
        }
        ExprKind::Perform { args, .. } | ExprKind::List(args) => {
            for arg in args {
                collect_handler_reference_ranges(
                    arg,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    ranges,
                );
            }
        }
        ExprKind::SchemaDecode { input, base, .. }
        | ExprKind::Binary {
            left: input,
            right: base,
            ..
        } => {
            collect_handler_reference_ranges(
                input,
                diagnostics,
                tokens,
                argument_delimiters,
                ranges,
            );
            collect_handler_reference_ranges(
                base,
                diagnostics,
                tokens,
                argument_delimiters,
                ranges,
            );
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_handler_reference_ranges(
                    &field.expr,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    ranges,
                );
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_handler_reference_ranges(
                    &entry.key,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    ranges,
                );
                collect_handler_reference_ranges(
                    &entry.value,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    ranges,
                );
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_handler_reference_ranges(
                scrutinee,
                diagnostics,
                tokens,
                argument_delimiters,
                ranges,
            );
            for arm in arms {
                collect_handler_reference_ranges(
                    &arm.expr,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    ranges,
                );
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_handler_reference_ranges(
                condition,
                diagnostics,
                tokens,
                argument_delimiters,
                ranges,
            );
            collect_handler_reference_ranges(
                then_branch,
                diagnostics,
                tokens,
                argument_delimiters,
                ranges,
            );
            for branch in else_if_branches {
                collect_handler_reference_ranges(
                    &branch.condition,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    ranges,
                );
                collect_handler_reference_ranges(
                    &branch.expr,
                    diagnostics,
                    tokens,
                    argument_delimiters,
                    ranges,
                );
            }
            collect_handler_reference_ranges(
                else_branch,
                diagnostics,
                tokens,
                argument_delimiters,
                ranges,
            );
        }
        ExprKind::Begin { body, .. } => collect_body_handler_reference_ranges(
            body,
            diagnostics,
            tokens,
            argument_delimiters,
            ranges,
        ),
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

struct HandlerArgumentDelimiters {
    closing_paren_ends: Vec<Option<usize>>,
    following_lparen_by_token_end: HashMap<usize, usize>,
}

impl HandlerArgumentDelimiters {
    fn new(tokens: &[Token]) -> Self {
        let mut closing_paren_ends = vec![None; tokens.len()];
        let mut open_parens = Vec::new();
        for (index, token) in tokens.iter().enumerate() {
            #[cfg(test)]
            record_handler_reference_token_visit();
            match token.kind {
                TokenKind::LParen => {
                    open_parens.push(index);
                }
                TokenKind::RParen => {
                    if let Some(open_index) = open_parens.pop() {
                        closing_paren_ends[open_index] = Some(token.range.end);
                    }
                }
                _ => {}
            }
        }

        let mut following_lparen_by_token_end = HashMap::with_capacity(tokens.len());
        let mut next_lparen = None;
        for (index, token) in tokens.iter().enumerate().rev() {
            #[cfg(test)]
            record_handler_reference_token_visit();
            if let Some(lparen_index) = next_lparen {
                following_lparen_by_token_end.insert(token.range.end, lparen_index);
            }
            if token.kind == TokenKind::LParen {
                next_lparen = Some(index);
            }
        }
        Self {
            closing_paren_ends,
            following_lparen_by_token_end,
        }
    }

    fn complete_arguments_follow(
        &self,
        handler_span: &SourceSpan,
        expr_span: &SourceSpan,
        tokens: &[Token],
    ) -> bool {
        let Some(&lparen_index) = self
            .following_lparen_by_token_end
            .get(&handler_span.end.offset)
        else {
            return false;
        };
        tokens[lparen_index].range.start < expr_span.end.offset
            && self.closing_paren_ends[lparen_index]
                .is_some_and(|closing_end| closing_end <= expr_span.end.offset)
    }
}

fn handler_operation_clause_body_ranges(
    source: &SourceFile,
    tokens: &[Token],
) -> Vec<(usize, usize)> {
    let defer_block_openers = defer_block_openers(tokens);
    let clause_headers =
        handler_operation_clause_headers_with(tokens, &defer_block_openers, || {});
    clause_headers
        .iter()
        .enumerate()
        .filter(|(_, is_header)| **is_header)
        .map(|(arrow_index, _)| {
            #[cfg(test)]
            record_handler_clause_body_range_index_entry();
            (
                tokens[arrow_index].range.end,
                handler_operation_clause_body_end_with_defer_openers(
                    tokens,
                    arrow_index,
                    source.text().len(),
                    &defer_block_openers,
                    &clause_headers,
                ),
            )
        })
        .collect()
}

fn handler_clause_bindings_by_name(syntax: &SyntaxTree) -> BTreeMap<String, Vec<ClauseBinding>> {
    let mut by_name = BTreeMap::new();
    for item in &syntax.items {
        let SyntaxItem::Handler(handler) = item else {
            continue;
        };
        for param in &handler.params {
            let binding = ClauseBinding {
                name: param.name.clone(),
                declaration: param.name_span.clone(),
                start: handler.span.start.offset,
                end: handler.span.end.offset,
                kind: LocalSymbolKind::HandlerContextParameter,
            };
            by_name
                .entry(binding.name.clone())
                .or_insert_with(Vec::new)
                .push(binding);
        }
        for (clause_index, clause) in handler.operation_clauses.iter().enumerate() {
            let clause_scope_end = handler
                .operation_clauses
                .get(clause_index + 1)
                .map_or(handler.span.end.offset, |next| next.span.start.offset);
            for param in &clause.params {
                let binding = ClauseBinding {
                    name: param.name.clone(),
                    declaration: param.name_span.clone(),
                    start: clause.span.start.offset,
                    end: clause_scope_end,
                    kind: LocalSymbolKind::HandlerOperationClauseParameter,
                };
                by_name
                    .entry(binding.name.clone())
                    .or_insert_with(Vec::new)
                    .push(binding);
            }
        }
    }
    by_name
}
