#[derive(Default)]
struct VariantRefinementSourceIndex {
    final_ranges: BTreeSet<(usize, usize)>,
    final_range_by_base_range: BTreeMap<(usize, usize), (usize, usize)>,
    type_argument_count_by_final_range: BTreeMap<(usize, usize), usize>,
    union_group_index_by_final_range: BTreeMap<(usize, usize), usize>,
    union_final_range_groups: Vec<Vec<(usize, usize)>>,
    type_argument_annotations_by_final_range: BTreeMap<(usize, usize), Vec<String>>,
    type_parameter_contexts: Vec<Vec<String>>,
    type_parameter_context_index_by_final_range: BTreeMap<(usize, usize), usize>,
}
fn variant_refinement_source_index(syntax: &SyntaxTree) -> VariantRefinementSourceIndex {
    let mut index = VariantRefinementSourceIndex::default();
    let no_type_parameters = Vec::new();
    for item in &syntax.items {
        match item {
            SyntaxItem::Function(function) => {
                collect_param_refinement_ranges(
                    &function.params,
                    &no_type_parameters,
                    &mut index,
                );
                collect_refinement_ranges(
                    &function.return_type_refinements,
                    &no_type_parameters,
                    &mut index,
                );
                collect_body_refinement_ranges(&function.body, &no_type_parameters, &mut index);
            }
            SyntaxItem::Effect(effect) => {
                for operation in &effect.operations {
                    collect_param_refinement_ranges(
                        &operation.params,
                        &no_type_parameters,
                        &mut index,
                    );
                    collect_refinement_ranges(
                        &operation.return_type_refinements,
                        &no_type_parameters,
                        &mut index,
                    );
                }
            }
            SyntaxItem::Handler(handler) => {
                collect_param_refinement_ranges(
                    &handler.params,
                    &no_type_parameters,
                    &mut index,
                );
                for clause in &handler.operation_clauses {
                    collect_param_refinement_ranges(
                        &clause.params,
                        &no_type_parameters,
                        &mut index,
                    );
                    collect_expr_refinement_ranges(
                        &clause.body,
                        &no_type_parameters,
                        &mut index,
                    );
                }
            }
            SyntaxItem::Type(ty) => {
                for variant in &ty.variants {
                    for field in &variant.fields {
                        collect_refinement_ranges(&field.ty_refinements, &ty.params, &mut index);
                    }
                }
            }
            SyntaxItem::Schema(schema) => {
                for field in &schema.fields {
                    collect_refinement_ranges(
                        &field.ty_refinements,
                        &no_type_parameters,
                        &mut index,
                    );
                }
            }
            SyntaxItem::PublicAlias(_) => {}
        }
    }
    index
}
fn constructor_reference_declaration_ranges(
    syntax: &SyntaxTree,
    tokens: &[Token],
) -> BTreeSet<(usize, usize)> {
    let mut ranges = BTreeSet::new();
    for item in &syntax.items {
        match item {
            SyntaxItem::Type(ty) => {
                for variant in &ty.variants {
                    let Some(name) = variant.name.as_deref() else {
                        continue;
                    };
                    if let Some(span) = variant.name_span.as_ref() {
                        ranges.insert((span.start.offset, span.end.offset));
                    } else if let Some(token) = tokens.iter().find(|token| {
                        token.kind == TokenKind::Ident
                            && token.text == name
                            && token.range.start >= variant.span.start.offset
                            && token.range.end <= variant.span.end.offset
                    }) {
                        ranges.insert((token.range.start, token.range.end));
                    }
                }
            }
            SyntaxItem::Effect(effect) => {
                for operation in &effect.operations {
                    if operation.name.is_some() {
                        ranges.insert((
                            operation.name_span.start.offset,
                            operation.name_span.end.offset,
                        ));
                    }
                }
            }
            SyntaxItem::Handler(handler) => {
                for clause in &handler.operation_clauses {
                    if clause.operation.is_some() {
                        ranges.insert((
                            clause.operation_span.start.offset,
                            clause.operation_span.end.offset,
                        ));
                    }
                }
            }
            SyntaxItem::Function(_) | SyntaxItem::Schema(_) | SyntaxItem::PublicAlias(_) => {}
        }
    }
    ranges
}

fn collect_param_refinement_ranges(
    params: &[veln_syntax::Param],
    type_parameters: &[String],
    index: &mut VariantRefinementSourceIndex,
) {
    for param in params {
        collect_refinement_ranges(&param.ty_refinements, type_parameters, index);
    }
}

fn collect_body_refinement_ranges(
    body: &[BodyLine],
    type_parameters: &[String],
    index: &mut VariantRefinementSourceIndex,
) {
    for line in body {
        match line {
            BodyLine::Let {
                annotation_refinements,
                expr,
                ..
            } => {
                collect_refinement_ranges(annotation_refinements, type_parameters, index);
                collect_expr_refinement_ranges(expr, type_parameters, index);
            }
            BodyLine::Expr { expr, .. } => {
                collect_expr_refinement_ranges(expr, type_parameters, index)
            }
            BodyLine::Defer { body, .. } => {
                collect_body_refinement_ranges(body, type_parameters, index)
            }
        }
    }
}

fn collect_expr_refinement_ranges(
    expr: &Expr,
    type_parameters: &[String],
    index: &mut VariantRefinementSourceIndex,
) {
    match &expr.kind {
        ExprKind::TypeApply {
            callee,
            type_arg_refinements,
            ..
        } => {
            collect_expr_refinement_ranges(callee, type_parameters, index);
            for refinements in type_arg_refinements {
                collect_refinement_ranges(refinements, type_parameters, index);
            }
        }
        ExprKind::Call { callee, args } => {
            collect_expr_refinement_ranges(callee, type_parameters, index);
            collect_exprs_refinement_ranges(args, type_parameters, index);
        }
        ExprKind::Perform { args, .. } | ExprKind::List(args) => {
            collect_exprs_refinement_ranges(args, type_parameters, index);
        }
        ExprKind::Handle { body, args, .. } => {
            collect_expr_refinement_ranges(body, type_parameters, index);
            collect_exprs_refinement_ranges(args, type_parameters, index);
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_expr_refinement_ranges(input, type_parameters, index);
            collect_expr_refinement_ranges(base, type_parameters, index);
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::Prefix { expr: value, .. }
        | ExprKind::Try { expr: value, .. }
        | ExprKind::FieldAccess { base: value, .. } => {
            collect_expr_refinement_ranges(value, type_parameters, index);
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_expr_refinement_ranges(&field.expr, type_parameters, index);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_expr_refinement_ranges(&entry.key, type_parameters, index);
                collect_expr_refinement_ranges(&entry.value, type_parameters, index);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_refinement_ranges(scrutinee, type_parameters, index);
            for arm in arms {
                collect_expr_refinement_ranges(&arm.expr, type_parameters, index);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_expr_refinement_ranges(condition, type_parameters, index);
            collect_expr_refinement_ranges(then_branch, type_parameters, index);
            for branch in else_if_branches {
                collect_expr_refinement_ranges(&branch.condition, type_parameters, index);
                collect_expr_refinement_ranges(&branch.expr, type_parameters, index);
            }
            collect_expr_refinement_ranges(else_branch, type_parameters, index);
        }
        ExprKind::Begin { body, .. } => {
            collect_body_refinement_ranges(body, type_parameters, index)
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_refinement_ranges(left, type_parameters, index);
            collect_expr_refinement_ranges(right, type_parameters, index);
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

fn collect_exprs_refinement_ranges(
    exprs: &[Expr],
    type_parameters: &[String],
    index: &mut VariantRefinementSourceIndex,
) {
    for expr in exprs {
        collect_expr_refinement_ranges(expr, type_parameters, index);
    }
}

fn collect_refinement_ranges(
    refinements: &[veln_syntax::VariantRefinementType],
    type_parameters: &[String],
    index: &mut VariantRefinementSourceIndex,
) {
    for refinement in refinements {
        let type_parameter_context_index = (!type_parameters.is_empty()
            && refinement
                .alternatives
                .iter()
                .any(|alternative| !alternative.type_arguments.is_empty()))
        .then(|| {
            let context_index = index.type_parameter_contexts.len();
            index.type_parameter_contexts.push(type_parameters.to_vec());
            context_index
        });
        let union_final_ranges = refinement
            .alternatives
            .iter()
            .map(|alternative| {
                (
                    alternative.variant_span.start.offset,
                    alternative.variant_span.end.offset,
                )
            })
            .collect::<Vec<_>>();
        let union_group_index = index.union_final_range_groups.len();
        index.union_final_range_groups.push(union_final_ranges);
        for alternative in &refinement.alternatives {
            let final_range = (
                alternative.variant_span.start.offset,
                alternative.variant_span.end.offset,
            );
            index.final_ranges.insert(final_range);
            index
                .type_argument_count_by_final_range
                .entry(final_range)
                .or_insert(alternative.type_arguments.len());
            index
                .union_group_index_by_final_range
                .entry(final_range)
                .or_insert(union_group_index);
            index
                .type_argument_annotations_by_final_range
                .entry(final_range)
                .or_insert_with(|| {
                    alternative
                        .type_arguments
                        .iter()
                        .map(variant_refinement_type_argument_annotation)
                        .collect()
                });
            if !alternative.type_arguments.is_empty()
                && let Some(context_index) = type_parameter_context_index
            {
                index
                    .type_parameter_context_index_by_final_range
                    .entry(final_range)
                    .or_insert(context_index);
            }
            if let Some(base_span) = alternative.base.segment_spans.last() {
                let base_range = (base_span.start.offset, base_span.end.offset);
                index.final_range_by_base_range.insert(
                    base_range,
                    final_range,
                );
            }
            for argument in &alternative.type_arguments {
                collect_refinement_ranges(&argument.ty_refinements, type_parameters, index);
            }
        }
    }
}

fn variant_refinement_type_argument_annotation(
    argument: &veln_syntax::VariantRefinementTypeArgument,
) -> String {
    let mut text = String::new();
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        text.push_str(fragment);
        if let Some(refinement) = argument.ty_refinements.get(index) {
            text.push_str(&variant_refinement_annotation(refinement));
        }
    }
    text
}

fn variant_refinement_annotation(refinement: &veln_syntax::VariantRefinementType) -> String {
    refinement
        .alternatives
        .iter()
        .map(|alternative| {
            let mut text = alternative.base.segments.join("::");
            if !alternative.type_arguments.is_empty() {
                text.push('<');
                text.push_str(
                    &alternative
                        .type_arguments
                        .iter()
                        .map(variant_refinement_type_argument_annotation)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                text.push('>');
            }
            text.push_str("::");
            text.push_str(&alternative.variant);
            text
        })
        .collect::<Vec<_>>()
        .join(" | ")
}
