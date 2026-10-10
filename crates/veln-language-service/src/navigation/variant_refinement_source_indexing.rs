fn variant_refinement_final_ranges(syntax: &SyntaxTree) -> BTreeSet<(usize, usize)> {
    let mut ranges = BTreeSet::new();
    for item in &syntax.items {
        match item {
            SyntaxItem::Function(function) => {
                collect_param_refinement_ranges(&function.params, &mut ranges);
                collect_refinement_ranges(&function.return_type_refinements, &mut ranges);
                collect_body_refinement_ranges(&function.body, &mut ranges);
            }
            SyntaxItem::Effect(effect) => {
                for operation in &effect.operations {
                    collect_param_refinement_ranges(&operation.params, &mut ranges);
                    collect_refinement_ranges(&operation.return_type_refinements, &mut ranges);
                }
            }
            SyntaxItem::Handler(handler) => {
                collect_param_refinement_ranges(&handler.params, &mut ranges);
                for clause in &handler.operation_clauses {
                    collect_param_refinement_ranges(&clause.params, &mut ranges);
                    collect_expr_refinement_ranges(&clause.body, &mut ranges);
                }
            }
            SyntaxItem::Type(ty) => {
                for variant in &ty.variants {
                    for field in &variant.fields {
                        collect_refinement_ranges(&field.ty_refinements, &mut ranges);
                    }
                }
            }
            SyntaxItem::Schema(schema) => {
                for field in &schema.fields {
                    collect_refinement_ranges(&field.ty_refinements, &mut ranges);
                }
            }
            SyntaxItem::PublicAlias(_) => {}
        }
    }
    ranges
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
    ranges: &mut BTreeSet<(usize, usize)>,
) {
    for param in params {
        collect_refinement_ranges(&param.ty_refinements, ranges);
    }
}

fn collect_body_refinement_ranges(body: &[BodyLine], ranges: &mut BTreeSet<(usize, usize)>) {
    for line in body {
        match line {
            BodyLine::Let {
                annotation_refinements,
                expr,
                ..
            } => {
                collect_refinement_ranges(annotation_refinements, ranges);
                collect_expr_refinement_ranges(expr, ranges);
            }
            BodyLine::Expr { expr, .. } => collect_expr_refinement_ranges(expr, ranges),
            BodyLine::Defer { body, .. } => collect_body_refinement_ranges(body, ranges),
        }
    }
}

fn collect_expr_refinement_ranges(expr: &Expr, ranges: &mut BTreeSet<(usize, usize)>) {
    match &expr.kind {
        ExprKind::TypeApply {
            callee,
            type_arg_refinements,
            ..
        } => {
            collect_expr_refinement_ranges(callee, ranges);
            for refinements in type_arg_refinements {
                collect_refinement_ranges(refinements, ranges);
            }
        }
        ExprKind::Call { callee, args } => {
            collect_expr_refinement_ranges(callee, ranges);
            collect_exprs_refinement_ranges(args, ranges);
        }
        ExprKind::Perform { args, .. } | ExprKind::List(args) => {
            collect_exprs_refinement_ranges(args, ranges);
        }
        ExprKind::Handle { body, args, .. } => {
            collect_expr_refinement_ranges(body, ranges);
            collect_exprs_refinement_ranges(args, ranges);
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_expr_refinement_ranges(input, ranges);
            collect_expr_refinement_ranges(base, ranges);
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::Prefix { expr: value, .. }
        | ExprKind::Try { expr: value, .. }
        | ExprKind::FieldAccess { base: value, .. } => {
            collect_expr_refinement_ranges(value, ranges);
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_expr_refinement_ranges(&field.expr, ranges);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_expr_refinement_ranges(&entry.key, ranges);
                collect_expr_refinement_ranges(&entry.value, ranges);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_refinement_ranges(scrutinee, ranges);
            for arm in arms {
                collect_expr_refinement_ranges(&arm.expr, ranges);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_expr_refinement_ranges(condition, ranges);
            collect_expr_refinement_ranges(then_branch, ranges);
            for branch in else_if_branches {
                collect_expr_refinement_ranges(&branch.condition, ranges);
                collect_expr_refinement_ranges(&branch.expr, ranges);
            }
            collect_expr_refinement_ranges(else_branch, ranges);
        }
        ExprKind::Begin { body, .. } => collect_body_refinement_ranges(body, ranges),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_refinement_ranges(left, ranges);
            collect_expr_refinement_ranges(right, ranges);
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

fn collect_exprs_refinement_ranges(exprs: &[Expr], ranges: &mut BTreeSet<(usize, usize)>) {
    for expr in exprs {
        collect_expr_refinement_ranges(expr, ranges);
    }
}

fn collect_refinement_ranges(
    refinements: &[veln_syntax::VariantRefinementType],
    ranges: &mut BTreeSet<(usize, usize)>,
) {
    for refinement in refinements {
        for alternative in &refinement.alternatives {
            ranges.insert((
                alternative.variant_span.start.offset,
                alternative.variant_span.end.offset,
            ));
            for argument in &alternative.type_arguments {
                collect_refinement_ranges(&argument.ty_refinements, ranges);
            }
        }
    }
}
