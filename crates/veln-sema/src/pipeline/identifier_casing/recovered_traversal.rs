use super::*;

pub(super) fn collect_recovered_qualified_segments_from_body_line(
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    push: RecoveredQualifiedSegmentPush,
    invalid: &mut Vec<InvalidName>,
) {
    match &line.kind {
        veln_ast::BodyLineKind::Let { expr, .. } | veln_ast::BodyLineKind::Expr { expr } => {
            collect_recovered_qualified_segments_from_expr(
                expr,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
        }
    }
}

pub(super) fn collect_recovered_qualified_segments_from_expr(
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    push: RecoveredQualifiedSegmentPush,
    invalid: &mut Vec<InvalidName>,
) {
    match &expr.kind {
        veln_ast::ExprKind::NamePath {
            segments,
            segment_spans,
        } => push(
            segments,
            segment_spans,
            current_module,
            enclosing_function_span,
            environment,
            invalid,
        ),
        veln_ast::ExprKind::Call { callee, args } => {
            collect_recovered_qualified_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
            for arg in args {
                collect_recovered_qualified_segments_from_expr(
                    arg,
                    current_module,
                    enclosing_function_span,
                    environment,
                    push,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::TypeApply { callee, .. }
        | veln_ast::ExprKind::FieldAccess { base: callee, .. }
        | veln_ast::ExprKind::Try(callee)
        | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
            collect_recovered_qualified_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
        }
        veln_ast::ExprKind::Binary { left, right, .. } => {
            collect_recovered_qualified_segments_from_expr(
                left,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
            collect_recovered_qualified_segments_from_expr(
                right,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
        }
        _ => {}
    }
}

pub(super) fn collect_recovered_qualified_function_segments_from_body_line(
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    match &line.kind {
        veln_ast::BodyLineKind::Let { expr, .. } | veln_ast::BodyLineKind::Expr { expr } => {
            collect_recovered_qualified_function_segments_from_expr(
                expr,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
    }
}

pub(super) fn collect_recovered_qualified_function_segments_from_expr(
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    match &expr.kind {
        veln_ast::ExprKind::Call { callee, args } => {
            if let veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } = &callee.kind
            {
                push_recovered_function_segment(
                    segments,
                    segment_spans,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
            collect_recovered_qualified_function_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for arg in args {
                collect_recovered_qualified_function_segments_from_expr(
                    arg,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::TypeApply { callee, .. }
        | veln_ast::ExprKind::FieldAccess { base: callee, .. }
        | veln_ast::ExprKind::Try(callee)
        | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
            collect_recovered_qualified_function_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Binary { left, right, .. } => {
            collect_recovered_qualified_function_segments_from_expr(
                left,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            collect_recovered_qualified_function_segments_from_expr(
                right,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Record(fields) => {
            for field in fields {
                collect_recovered_qualified_function_segments_from_expr(
                    &field.expr,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::List(items) | veln_ast::ExprKind::Perform { args: items, .. } => {
            for item in items {
                collect_recovered_qualified_function_segments_from_expr(
                    item,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        _ => {}
    }
}

pub(super) fn collect_recovered_qualified_type_segments_from_body_line(
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    match &line.kind {
        veln_ast::BodyLineKind::Let { expr, .. } | veln_ast::BodyLineKind::Expr { expr } => {
            collect_recovered_qualified_type_segments_from_expr(
                expr,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
    }
}

pub(super) fn collect_recovered_qualified_type_segments_from_expr(
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    match &expr.kind {
        veln_ast::ExprKind::Call { callee, args } => {
            if let veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } = &callee.kind
            {
                push_recovered_qualified_type_segment(
                    segments,
                    segment_spans,
                    Some(args.len()),
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
            collect_recovered_qualified_type_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for arg in args {
                collect_recovered_qualified_type_segments_from_expr(
                    arg,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::NamePath {
            segments,
            segment_spans,
        } => {
            push_recovered_qualified_type_segment(
                segments,
                segment_spans,
                None,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::TypeApply { callee, .. }
        | veln_ast::ExprKind::FieldAccess { base: callee, .. }
        | veln_ast::ExprKind::Try(callee)
        | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Binary { left, right, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                left,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            collect_recovered_qualified_type_segments_from_expr(
                right,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_recovered_qualified_type_segments_from_expr(
                condition,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            collect_recovered_qualified_type_segments_from_expr(
                then_branch,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for branch in else_if_branches {
                collect_recovered_qualified_type_segments_from_expr(
                    &branch.condition,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
                collect_recovered_qualified_type_segments_from_expr(
                    &branch.expr,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
            collect_recovered_qualified_type_segments_from_expr(
                else_branch,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Record(fields) => {
            for field in fields {
                collect_recovered_qualified_type_segments_from_expr(
                    &field.expr,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::Dict(entries) => {
            for entry in entries {
                collect_recovered_qualified_type_segments_from_expr(
                    &entry.key,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
                collect_recovered_qualified_type_segments_from_expr(
                    &entry.value,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::List(items) | veln_ast::ExprKind::Perform { args: items, .. } => {
            for item in items {
                collect_recovered_qualified_type_segments_from_expr(
                    item,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::Handle { body, args, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                body,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for arg in args {
                collect_recovered_qualified_type_segments_from_expr(
                    arg,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::SchemaDecode { input, base, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                input,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            collect_recovered_qualified_type_segments_from_expr(
                base,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::SchemaEncode { value, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                value,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Match { scrutinee, arms } => {
            collect_recovered_qualified_type_segments_from_expr(
                scrutinee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for arm in arms {
                collect_recovered_qualified_type_segments_from_expr(
                    &arm.expr,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::Missing
        | veln_ast::ExprKind::Hole { .. }
        | veln_ast::ExprKind::StringLiteral(_)
        | veln_ast::ExprKind::IntLiteral(_)
        | veln_ast::ExprKind::FloatLiteral(_)
        | veln_ast::ExprKind::BoolLiteral(_)
        | veln_ast::ExprKind::Unit => {}
    }
}
