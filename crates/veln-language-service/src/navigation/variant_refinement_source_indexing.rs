#[derive(Default)]
struct VariantRefinementSourceIndex {
    final_ranges: BTreeSet<(usize, usize)>,
    final_range_by_base_range: BTreeMap<(usize, usize), (usize, usize)>,
    type_argument_count_by_final_range: BTreeMap<(usize, usize), usize>,
}

fn variant_refinement_source_index(syntax: &SyntaxTree) -> VariantRefinementSourceIndex {
    let mut index = VariantRefinementSourceIndex::default();
    for item in &syntax.items {
        match item {
            SyntaxItem::Function(function) => {
                collect_param_refinement_ranges(&function.params, &mut index);
                collect_refinement_ranges(&function.return_type_refinements, &mut index);
                collect_body_refinement_ranges(&function.body, &mut index);
            }
            SyntaxItem::Effect(effect) => {
                for operation in &effect.operations {
                    collect_param_refinement_ranges(&operation.params, &mut index);
                    collect_refinement_ranges(&operation.return_type_refinements, &mut index);
                }
            }
            SyntaxItem::Handler(handler) => {
                collect_param_refinement_ranges(&handler.params, &mut index);
                for clause in &handler.operation_clauses {
                    collect_param_refinement_ranges(&clause.params, &mut index);
                    collect_expr_refinement_ranges(&clause.body, &mut index);
                }
            }
            SyntaxItem::Type(ty) => {
                for variant in &ty.variants {
                    for field in &variant.fields {
                        collect_refinement_ranges(&field.ty_refinements, &mut index);
                    }
                }
            }
            SyntaxItem::Schema(schema) => {
                for field in &schema.fields {
                    collect_refinement_ranges(&field.ty_refinements, &mut index);
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
    index: &mut VariantRefinementSourceIndex,
) {
    for param in params {
        collect_refinement_ranges(&param.ty_refinements, index);
    }
}

fn collect_body_refinement_ranges(body: &[BodyLine], index: &mut VariantRefinementSourceIndex) {
    for line in body {
        match line {
            BodyLine::Let {
                annotation_refinements,
                expr,
                ..
            } => {
                collect_refinement_ranges(annotation_refinements, index);
                collect_expr_refinement_ranges(expr, index);
            }
            BodyLine::Expr { expr, .. } => collect_expr_refinement_ranges(expr, index),
            BodyLine::Defer { body, .. } => collect_body_refinement_ranges(body, index),
        }
    }
}

fn collect_expr_refinement_ranges(expr: &Expr, index: &mut VariantRefinementSourceIndex) {
    match &expr.kind {
        ExprKind::TypeApply {
            callee,
            type_arg_refinements,
            ..
        } => {
            collect_expr_refinement_ranges(callee, index);
            for refinements in type_arg_refinements {
                collect_refinement_ranges(refinements, index);
            }
        }
        ExprKind::Call { callee, args } => {
            collect_expr_refinement_ranges(callee, index);
            collect_exprs_refinement_ranges(args, index);
        }
        ExprKind::Perform { args, .. } | ExprKind::List(args) => {
            collect_exprs_refinement_ranges(args, index);
        }
        ExprKind::Handle { body, args, .. } => {
            collect_expr_refinement_ranges(body, index);
            collect_exprs_refinement_ranges(args, index);
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_expr_refinement_ranges(input, index);
            collect_expr_refinement_ranges(base, index);
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::Prefix { expr: value, .. }
        | ExprKind::Try { expr: value, .. }
        | ExprKind::FieldAccess { base: value, .. } => {
            collect_expr_refinement_ranges(value, index);
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_expr_refinement_ranges(&field.expr, index);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_expr_refinement_ranges(&entry.key, index);
                collect_expr_refinement_ranges(&entry.value, index);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_refinement_ranges(scrutinee, index);
            for arm in arms {
                collect_expr_refinement_ranges(&arm.expr, index);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_expr_refinement_ranges(condition, index);
            collect_expr_refinement_ranges(then_branch, index);
            for branch in else_if_branches {
                collect_expr_refinement_ranges(&branch.condition, index);
                collect_expr_refinement_ranges(&branch.expr, index);
            }
            collect_expr_refinement_ranges(else_branch, index);
        }
        ExprKind::Begin { body, .. } => collect_body_refinement_ranges(body, index),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_refinement_ranges(left, index);
            collect_expr_refinement_ranges(right, index);
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

fn collect_exprs_refinement_ranges(exprs: &[Expr], index: &mut VariantRefinementSourceIndex) {
    for expr in exprs {
        collect_expr_refinement_ranges(expr, index);
    }
}

fn collect_refinement_ranges(
    refinements: &[veln_syntax::VariantRefinementType],
    index: &mut VariantRefinementSourceIndex,
) {
    for refinement in refinements {
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
            if let Some(base_span) = alternative.base.segment_spans.last() {
                let base_range = (base_span.start.offset, base_span.end.offset);
                index.final_range_by_base_range.insert(
                    base_range,
                    final_range,
                );
            }
            for argument in &alternative.type_arguments {
                collect_refinement_ranges(&argument.ty_refinements, index);
            }
        }
    }
}
