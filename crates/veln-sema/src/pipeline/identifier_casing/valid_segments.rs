use super::*;

pub(super) fn valid_qualified_path_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    let mut segments = Vec::new();
    for type_decl in &module.types {
        let current_module = type_decl.module_name.as_deref();
        for variant in &type_decl.variants {
            for field in &variant.fields {
                collect_type_path_segments(
                    &field.ty_paths,
                    current_module,
                    environment,
                    &mut segments,
                );
            }
        }
    }
    for effect in &module.effects {
        let current_module = effect.module_name.as_deref();
        for operation in &effect.operations {
            for param in &operation.params {
                collect_type_path_segments(
                    &param.ty_paths,
                    current_module,
                    environment,
                    &mut segments,
                );
            }
            collect_type_path_segments(
                &operation.return_type_paths,
                current_module,
                environment,
                &mut segments,
            );
        }
    }
    for schema in &module.schemas {
        let current_module = schema.module_name.as_deref();
        for field in &schema.fields {
            collect_type_path_segments(&field.ty_paths, current_module, environment, &mut segments);
        }
    }
    for function in &module.functions {
        let current_module = function.module_name.as_deref();
        collect_type_path_segments(
            &function.return_type_paths,
            current_module,
            environment,
            &mut segments,
        );
        for param in &function.params {
            collect_type_path_segments(&param.ty_paths, current_module, environment, &mut segments);
        }
        for line in &function.body {
            collect_valid_segments_from_body_line(line, current_module, environment, &mut segments);
        }
    }
    for handler in &module.handlers {
        let current_module = handler.module_name.as_deref();
        for param in &handler.params {
            collect_type_path_segments(&param.ty_paths, current_module, environment, &mut segments);
        }
        for clause in &handler.operation_clauses {
            for param in &clause.params {
                collect_type_path_segments(
                    &param.ty_paths,
                    current_module,
                    environment,
                    &mut segments,
                );
            }
            collect_valid_segments_from_expr(
                &clause.body,
                current_module,
                environment,
                &mut segments,
            );
        }
    }
    segments
}

fn collect_type_path_segments(
    paths: &[veln_ast::TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    for path in paths {
        if path.segments.len() < 2 {
            continue;
        }
        let quarantined_import_lacks_leaf = environment
            .quarantined_import_type_path_lacks_visible_leaf(&path.segments, current_module);
        if quarantined_import_lacks_leaf
            && !environment
                .quarantined_import_type_path_uses_nested_alias(&path.segments, current_module)
        {
            continue;
        }
        for index in 0..path.segments.len() {
            if quarantined_import_lacks_leaf && index + 1 == path.segments.len() {
                continue;
            }
            let Some(span) = path.segment_spans.get(index) else {
                continue;
            };
            let role = if index + 1 == path.segments.len() {
                NameClass::Type
            } else {
                NameClass::Module
            };
            output.push(qualified_path_segment_from_parts(
                &path.segments[index],
                role,
                span,
                index,
                QualifiedPathSegmentEvidence::Syntax,
            ));
        }
    }
}

fn collect_valid_segments_from_body_line(
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    match &line.kind {
        veln_ast::BodyLineKind::Let {
            pattern,
            annotation_paths,
            expr,
            ..
        } => {
            collect_valid_segments_from_pattern(pattern, current_module, environment, output);
            collect_type_path_segments(annotation_paths, current_module, environment, output);
            collect_valid_segments_from_expr(expr, current_module, environment, output);
        }
        veln_ast::BodyLineKind::Expr { expr } => {
            collect_valid_segments_from_expr(expr, current_module, environment, output);
        }
    }
}

fn collect_valid_segments_from_expr(
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    match &expr.kind {
        veln_ast::ExprKind::NamePath {
            segments,
            segment_spans,
        } => {
            collect_valid_expr_path_segments(
                segments,
                segment_spans,
                current_module,
                environment,
                output,
            );
        }
        veln_ast::ExprKind::Call { callee, args } => {
            if let veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } = &callee.kind
            {
                collect_valid_call_path_segments(
                    segments,
                    segment_spans,
                    current_module,
                    environment,
                    output,
                );
            } else {
                collect_valid_segments_from_expr(callee, current_module, environment, output);
            }
            for arg in args {
                collect_valid_segments_from_expr(arg, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::TypeApply { callee, .. }
        | veln_ast::ExprKind::FieldAccess { base: callee, .. }
        | veln_ast::ExprKind::Try(callee)
        | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
            collect_valid_segments_from_expr(callee, current_module, environment, output);
        }
        veln_ast::ExprKind::Binary { left, right, .. } => {
            collect_valid_segments_from_expr(left, current_module, environment, output);
            collect_valid_segments_from_expr(right, current_module, environment, output);
        }
        veln_ast::ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_valid_segments_from_expr(condition, current_module, environment, output);
            collect_valid_segments_from_expr(then_branch, current_module, environment, output);
            for branch in else_if_branches {
                collect_valid_segments_from_expr(
                    &branch.condition,
                    current_module,
                    environment,
                    output,
                );
                collect_valid_segments_from_expr(&branch.expr, current_module, environment, output);
            }
            collect_valid_segments_from_expr(else_branch, current_module, environment, output);
        }
        veln_ast::ExprKind::Record(fields) => {
            for field in fields {
                collect_valid_segments_from_expr(&field.expr, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::Dict(entries) => {
            for entry in entries {
                collect_valid_segments_from_expr(&entry.key, current_module, environment, output);
                collect_valid_segments_from_expr(&entry.value, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::List(items) | veln_ast::ExprKind::Perform { args: items, .. } => {
            for item in items {
                collect_valid_segments_from_expr(item, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::Handle { body, args, .. } => {
            collect_valid_segments_from_expr(body, current_module, environment, output);
            for arg in args {
                collect_valid_segments_from_expr(arg, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::SchemaDecode { input, base, .. } => {
            collect_valid_segments_from_expr(input, current_module, environment, output);
            collect_valid_segments_from_expr(base, current_module, environment, output);
        }
        veln_ast::ExprKind::SchemaEncode { value, .. } => {
            collect_valid_segments_from_expr(value, current_module, environment, output);
        }
        veln_ast::ExprKind::Match { scrutinee, arms } => {
            collect_valid_segments_from_expr(scrutinee, current_module, environment, output);
            for arm in arms {
                collect_valid_segments_from_pattern(
                    &arm.pattern,
                    current_module,
                    environment,
                    output,
                );
                collect_valid_segments_from_expr(&arm.expr, current_module, environment, output);
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

fn collect_valid_segments_from_pattern(
    pattern: &veln_ast::Pattern,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    match &pattern.kind {
        veln_ast::PatternKind::Constructor {
            name,
            name_spans,
            args,
        } => {
            collect_valid_constructor_path_segments(
                name,
                name_spans,
                current_module,
                environment,
                output,
            );
            for arg in args {
                collect_valid_segments_from_pattern(arg, current_module, environment, output);
            }
        }
        veln_ast::PatternKind::Record(fields) => {
            for field in fields {
                collect_valid_segments_from_pattern(
                    &field.pattern,
                    current_module,
                    environment,
                    output,
                );
            }
        }
        _ => {}
    }
}

fn collect_valid_expr_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    if segments.len() < 2 {
        return;
    }
    if environment
        .function_path_for_value(segments, current_module)
        .is_some()
        || qualified_prelude_signature(segments, None).is_some()
        || qualified_prelude_builtin_signature_with_input(segments, None, None).is_some()
    {
        push_module_prefix_and_leaf(segments, segment_spans, NameClass::ValueBinding, output);
        return;
    }
    if matches!(
        environment
            .adts
            .nullary_constructor(segments, current_module, &environment.uses),
        crate::adt::registry::ConstructorLookup::Found(_)
    ) {
        push_constructor_path_segments(
            segments,
            segment_spans,
            current_module,
            environment,
            output,
        );
    }
}

fn collect_valid_call_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    if segments.len() < 2 {
        return;
    }
    if environment
        .function_path(segments, current_module)
        .is_some()
        || qualified_prelude_signature(segments, None).is_some()
        || qualified_prelude_builtin_signature_with_input(segments, None, None).is_some()
    {
        push_module_prefix_and_leaf(segments, segment_spans, NameClass::Function, output);
    } else if path_resolves_as_constructor(segments, current_module, environment) {
        push_constructor_path_segments(
            segments,
            segment_spans,
            current_module,
            environment,
            output,
        );
    }
}

fn collect_valid_constructor_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    if segments.len() < 2 || !path_resolves_as_constructor(segments, current_module, environment) {
        return;
    }
    push_constructor_path_segments(segments, segment_spans, current_module, environment, output);
}

fn push_constructor_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    let type_index = constructor_type_segment_index(segments, current_module, environment);
    for index in 0..segments.len() {
        let Some(span) = segment_spans.get(index) else {
            continue;
        };
        let role = if Some(index) == type_index {
            NameClass::Type
        } else if index + 1 == segments.len() {
            NameClass::Constructor
        } else {
            NameClass::Module
        };
        output.push(qualified_path_segment_from_parts(
            &segments[index],
            role,
            span,
            index,
            QualifiedPathSegmentEvidence::Resolved,
        ));
    }
}

fn constructor_type_segment_index(
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> Option<usize> {
    let crate::adt::registry::ConstructorLookup::Found(constructor) =
        environment
            .adts
            .constructor(segments, current_module, &environment.uses)
    else {
        return None;
    };
    segments[..segments.len().saturating_sub(1)]
        .iter()
        .rposition(|segment| segment == &constructor.descriptor.type_name)
}

fn push_module_prefix_and_leaf(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    leaf_role: NameClass,
    output: &mut Vec<QualifiedPathSegment>,
) {
    for index in 0..segments.len() {
        let Some(span) = segment_spans.get(index) else {
            continue;
        };
        let role = if index + 1 == segments.len() {
            leaf_role
        } else {
            NameClass::Module
        };
        output.push(qualified_path_segment_from_parts(
            &segments[index],
            role,
            span,
            index,
            QualifiedPathSegmentEvidence::Resolved,
        ));
    }
}

fn qualified_path_segment_from_parts(
    name: &str,
    role: NameClass,
    span: &veln_source::SourceSpan,
    segment_index: usize,
    evidence: QualifiedPathSegmentEvidence,
) -> QualifiedPathSegment {
    QualifiedPathSegment {
        name: name.to_string(),
        role,
        occurrence: NameOccurrence::PathSegment,
        span: span.clone(),
        segment_index,
        evidence,
    }
}

pub fn classified_project_qualified_path_segments(
    module: &SurfaceModule,
) -> Vec<QualifiedPathSegment> {
    classified_project_qualified_path_segments_with_context(module, module)
}

pub fn classified_project_qualified_path_segments_with_context(
    module: &SurfaceModule,
    project: &SurfaceModule,
) -> Vec<QualifiedPathSegment> {
    let environment = TypeEnvironment::from_module(project);
    classified_qualified_path_segments(module, &environment)
}
