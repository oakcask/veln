fn valid_effect_operation_ranges(
    tokens: &[Token],
    effect_reference_ranges: &BTreeSet<(usize, usize)>,
    parsed: &ParseOutput,
) -> BTreeSet<(usize, usize)> {
    let closing_parentheses = closing_parenthesis_indexes(tokens);
    let mut diagnostic_offsets = parsed
        .diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.offset))
        .collect::<Vec<_>>();
    diagnostic_offsets.sort_unstable();
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| {
            effect_reference_ranges.contains(&(token.range.start, token.range.end))
        })
        .filter_map(|(index, _)| next_path_segment_index(tokens, index))
        .filter(|index| {
            !effect_operation_arguments_are_recovered(
                tokens,
                *index,
                &closing_parentheses,
                &diagnostic_offsets,
            )
        })
        .map(|index| {
            let range = tokens[index].range;
            (range.start, range.end)
        })
        .collect()
}

fn closing_parenthesis_indexes(tokens: &[Token]) -> Vec<Option<usize>> {
    let mut closing_parentheses = vec![None; tokens.len()];
    let mut open_parentheses = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LParen => open_parentheses.push(index),
            TokenKind::RParen => {
                if let Some(open_index) = open_parentheses.pop() {
                    closing_parentheses[open_index] = Some(index);
                }
            }
            _ => {}
        }
    }
    closing_parentheses
}

fn effect_operation_arguments_are_recovered(
    tokens: &[Token],
    operation_index: usize,
    closing_parentheses: &[Option<usize>],
    diagnostic_offsets: &[usize],
) -> bool {
    let Some(open_index) = next_non_layout_index(tokens, operation_index) else {
        return true;
    };
    if tokens[open_index].kind != TokenKind::LParen {
        return true;
    }
    let Some(close_index) = closing_parentheses[open_index] else {
        return true;
    };
    let start = tokens[operation_index].range.start;
    let end = tokens[close_index].range.end;
    let diagnostic_index = diagnostic_offsets.partition_point(|offset| *offset < start);
    diagnostic_offsets
        .get(diagnostic_index)
        .is_some_and(|offset| *offset <= end)
}

fn generic_effect_binders(syntax: &SyntaxTree) -> Vec<GenericEffectBinder> {
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Function(function) => function.effect_binder.as_ref().map(|binder| {
                GenericEffectBinder {
                    name: binder.name.clone(),
                    start: function.span.start.offset,
                    end: function.span.end.offset,
                }
            }),
            _ => None,
        })
        .collect()
}

fn recovered_effect_declarations(syntax: &SyntaxTree) -> Vec<SourceSpan> {
    syntax
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Effect(effect) if effect.recovered => Some(effect.span.clone()),
            _ => None,
        })
        .collect()
}

fn valid_effect_reference_ranges(
    tokens: &[Token],
    effect_list_membership: &[bool],
    syntax: &SyntaxTree,
) -> BTreeSet<(usize, usize)> {
    let path_roots = path_root_indices(tokens);
    let mut regions = Vec::new();
    let mut handler_target_ranges = BTreeSet::new();
    let mut handler_target_token_cursor = 0usize;
    for item in &syntax.items {
        match item {
            SyntaxItem::Function(function) => {
                collect_function_effect_reference_regions(function, &mut regions);
            }
            SyntaxItem::Handler(handler) => {
                collect_handler_effect_reference_regions(handler, &mut regions);
                if let Some(range) = handler_effect_leaf_range(
                    tokens,
                    &mut handler_target_token_cursor,
                    handler,
                )
                {
                    handler_target_ranges.insert(range);
                }
            }
            SyntaxItem::Effect(effect) => {
                collect_effect_declaration_reference_regions(tokens, effect, &mut regions);
            }
            SyntaxItem::Type(ty) => {
                collect_type_declaration_reference_regions(tokens, ty, &mut regions);
            }
            SyntaxItem::Schema(schema) => {
                collect_schema_declaration_reference_regions(tokens, schema, &mut regions);
            }
            SyntaxItem::PublicAlias(_) => {}
        }
    }

    let merged_regions = merge_regions(regions);
    collect_effect_reference_token_ranges(
        tokens,
        effect_list_membership,
        &path_roots,
        &handler_target_ranges,
        &merged_regions,
    )
}

fn handler_effect_leaf_range(
    tokens: &[Token],
    token_cursor: &mut usize,
    handler: &veln_syntax::HandlerDecl,
) -> Option<(usize, usize)> {
    let mut leaf = None;
    while let Some(token) = tokens.get(*token_cursor) {
        if token.range.end > handler.effect_span.end.offset {
            break;
        }
        if !handler.effect_recovered
            && token.kind == TokenKind::Ident
            && handler.effect_span.start.offset <= token.range.start
        {
            leaf = Some((token.range.start, token.range.end));
        }
        *token_cursor += 1;
    }
    leaf
}

fn collect_function_effect_reference_regions(
    function: &veln_syntax::FunctionDecl,
    regions: &mut Vec<(usize, usize)>,
) {
    collect_parameter_type_regions(&function.params, regions);
    if let (Some(return_type), Some(span)) =
        (&function.return_type, &function.return_type_span)
    {
        push_valid_type_region(return_type, span, regions);
    }
    if !function.effects_recovered {
        extend_optional_spans(&function.effect_spans, regions);
    }
    for contract in &function.contracts {
        extend_spans(&contract.perform_effect_spans, regions);
    }
    for line in &function.body {
        collect_body_line_effect_reference_regions(line, regions);
    }
}

fn collect_body_line_effect_reference_regions(
    line: &BodyLine,
    regions: &mut Vec<(usize, usize)>,
) {
    match line {
        BodyLine::Let {
            annotation, expr, span, ..
        } => {
            if let Some(annotation) = annotation
                && valid_type_syntax(annotation)
            {
                regions.push((span.start.offset, expr.span.start.offset));
            }
            collect_perform_effect_regions(expr, regions);
        }
        BodyLine::Expr { expr, .. } => collect_perform_effect_regions(expr, regions),
        BodyLine::Defer { body, .. } => {
            for line in body {
                collect_body_line_effect_reference_regions(line, regions);
            }
        }
    }
}

fn collect_handler_effect_reference_regions(
    handler: &veln_syntax::HandlerDecl,
    regions: &mut Vec<(usize, usize)>,
) {
    collect_parameter_type_regions(&handler.params, regions);
    if !handler.effect_recovered {
        regions.push((
            handler.effect_span.start.offset,
            handler.effect_span.end.offset,
        ));
    }
    if !handler.effects_recovered {
        extend_optional_spans(&handler.effect_spans, regions);
    }
    for clause in &handler.operation_clauses {
        collect_parameter_type_regions(&clause.params, regions);
        collect_perform_effect_regions(&clause.body, regions);
    }
}

fn collect_effect_declaration_reference_regions(
    tokens: &[Token],
    effect: &veln_syntax::EffectDecl,
    regions: &mut Vec<(usize, usize)>,
) {
    for operation in &effect.operations {
        collect_parameter_type_regions(&operation.params, regions);
        if let Some(return_type) = &operation.return_type
            && valid_type_syntax(return_type)
            && let Some(start) = offset_after_token(tokens, &operation.span, TokenKind::Arrow)
        {
            regions.push((start, operation.span.end.offset));
        }
    }
}

fn collect_type_declaration_reference_regions(
    tokens: &[Token],
    ty: &veln_syntax::TypeDecl,
    regions: &mut Vec<(usize, usize)>,
) {
    for field in ty.variants.iter().flat_map(|variant| &variant.fields) {
        if valid_type_syntax(&field.ty) {
            let start = offset_after_token(tokens, &field.span, TokenKind::Colon)
                .unwrap_or(field.span.start.offset);
            regions.push((start, field.span.end.offset));
        }
    }
}

fn collect_schema_declaration_reference_regions(
    tokens: &[Token],
    schema: &veln_syntax::SchemaDecl,
    regions: &mut Vec<(usize, usize)>,
) {
    for field in &schema.fields {
        if valid_type_syntax(&field.ty) {
            let start = offset_after_token(tokens, &field.span, TokenKind::Colon)
                .unwrap_or(field.span.start.offset);
            let end = field
                .where_clause
                .as_ref()
                .map_or(field.span.end.offset, |clause| clause.span.start.offset);
            regions.push((start, end));
        }
        if let Some(clause) = &field.where_clause {
            extend_spans(&clause.perform_effect_spans, regions);
        }
    }
    for validation in &schema.validations {
        extend_spans(&validation.perform_effect_spans, regions);
    }
}

fn merge_regions(mut regions: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    regions.sort_unstable();
    let mut merged_regions = Vec::<(usize, usize)>::new();
    for (start, end) in regions {
        if let Some((_, merged_end)) = merged_regions.last_mut()
            && start <= *merged_end
        {
            *merged_end = (*merged_end).max(end);
        } else {
            merged_regions.push((start, end));
        }
    }
    merged_regions
}

fn collect_effect_reference_token_ranges(
    tokens: &[Token],
    effect_list_membership: &[bool],
    path_roots: &[usize],
    handler_target_ranges: &BTreeSet<(usize, usize)>,
    merged_regions: &[(usize, usize)],
) -> BTreeSet<(usize, usize)> {
    let mut ranges = BTreeSet::new();
    let mut region_index = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        while region_index < merged_regions.len()
            && merged_regions[region_index].1 < token.range.end
        {
            region_index += 1;
        }
        if region_index == merged_regions.len() {
            break;
        }
        let (start, end) = merged_regions[region_index];
        if start <= token.range.start
            && token.range.end <= end
            && token.kind == TokenKind::Ident
            && (is_effect_list_member_token(tokens, effect_list_membership, index)
                || handler_target_ranges.contains(&(token.range.start, token.range.end))
                || is_handler_handled_effect_token(tokens, path_roots, index)
                || is_perform_effect_qualifier_token(tokens, path_roots, index))
        {
            ranges.insert((token.range.start, token.range.end));
        }
    }
    ranges
}

fn collect_parameter_type_regions(params: &[veln_syntax::Param], regions: &mut Vec<(usize, usize)>) {
    for param in params {
        if let (Some(ty), Some(span)) = (&param.ty, &param.ty_span) {
            push_valid_type_region(ty, span, regions);
        }
    }
}

fn push_valid_type_region(ty: &str, span: &SourceSpan, regions: &mut Vec<(usize, usize)>) {
    if valid_type_syntax(ty) {
        regions.push((span.start.offset, span.end.offset));
    }
}

fn valid_type_syntax(ty: &str) -> bool {
    veln_sema::type_annotation_reference_paths(ty).is_ok()
}

fn offset_after_token(tokens: &[Token], span: &SourceSpan, kind: TokenKind) -> Option<usize> {
    let start = tokens.partition_point(|token| token.range.end <= span.start.offset);
    tokens[start..]
        .iter()
        .take_while(|token| token.range.start < span.end.offset)
        .find(|token| token.kind == kind)
        .map(|token| token.range.end)
}

fn extend_optional_spans(spans: &Option<Vec<SourceSpan>>, regions: &mut Vec<(usize, usize)>) {
    if let Some(spans) = spans {
        extend_spans(spans, regions);
    }
}

fn extend_spans(spans: &[SourceSpan], regions: &mut Vec<(usize, usize)>) {
    regions.extend(
        spans
            .iter()
            .map(|span| (span.start.offset, span.end.offset)),
    );
}

fn collect_perform_effect_regions(expr: &Expr, regions: &mut Vec<(usize, usize)>) {
    match &expr.kind {
        ExprKind::Perform {
            effect_span,
            recovered,
            args,
            ..
        } => {
            if !recovered {
                regions.push((effect_span.start.offset, effect_span.end.offset));
            }
            for arg in args {
                collect_perform_effect_regions(arg, regions);
            }
        }
        ExprKind::TypeApply { callee, .. }
        | ExprKind::FieldAccess { base: callee, .. }
        | ExprKind::Try { expr: callee, .. }
        | ExprKind::Prefix { expr: callee, .. } => {
            collect_perform_effect_regions(callee, regions);
        }
        ExprKind::Call { callee, args } => {
            collect_perform_effect_regions(callee, regions);
            for arg in args {
                collect_perform_effect_regions(arg, regions);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_perform_effect_regions(body, regions);
            for arg in args {
                collect_perform_effect_regions(arg, regions);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_perform_effect_regions(input, regions);
            collect_perform_effect_regions(base, regions);
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_perform_effect_regions(value, regions);
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_perform_effect_regions(&field.expr, regions);
            }
        }
        ExprKind::Dict(entries) => collect_dict_perform_effect_regions(entries, regions),
        ExprKind::List(items) => {
            for item in items {
                collect_perform_effect_regions(item, regions);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_match_perform_effect_regions(scrutinee, arms, regions);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => collect_if_perform_effect_regions(
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            regions,
        ),
        ExprKind::Begin { body, .. } => {
            for line in body {
                collect_body_line_effect_reference_regions(line, regions);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_perform_effect_regions(left, regions);
            collect_perform_effect_regions(right, regions);
        }
        ExprKind::Hole { satisfy, .. } => {
            if let Some(clause) = satisfy {
                extend_spans(&clause.perform_effect_spans, regions);
            }
        }
        ExprKind::Missing
        | ExprKind::NamePath { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn collect_dict_perform_effect_regions(
    entries: &[veln_syntax::DictEntry],
    regions: &mut Vec<(usize, usize)>,
) {
    for entry in entries {
        collect_perform_effect_regions(&entry.key, regions);
        collect_perform_effect_regions(&entry.value, regions);
    }
}

fn collect_match_perform_effect_regions(
    scrutinee: &Expr,
    arms: &[veln_syntax::MatchArm],
    regions: &mut Vec<(usize, usize)>,
) {
    collect_perform_effect_regions(scrutinee, regions);
    for arm in arms {
        collect_perform_effect_regions(&arm.expr, regions);
    }
}

fn collect_if_perform_effect_regions(
    condition: &Expr,
    then_branch: &Expr,
    else_if_branches: &[veln_syntax::IfBranch],
    else_branch: &Expr,
    regions: &mut Vec<(usize, usize)>,
) {
    collect_perform_effect_regions(condition, regions);
    collect_perform_effect_regions(then_branch, regions);
    for branch in else_if_branches {
        collect_perform_effect_regions(&branch.condition, regions);
        collect_perform_effect_regions(&branch.expr, regions);
    }
    collect_perform_effect_regions(else_branch, regions);
}
