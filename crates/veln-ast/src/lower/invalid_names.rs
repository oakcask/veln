use super::*;

pub(super) fn collect_invalid_alias_name(
    alias: &SyntaxPublicAlias,
    invalid: &mut Vec<InvalidName>,
) {
    let class = match alias.kind {
        SyntaxPublicAliasKind::Function => NameClass::Function,
        SyntaxPublicAliasKind::Type => NameClass::Type,
        SyntaxPublicAliasKind::Schema => return,
    };
    push_invalid_name(
        invalid,
        alias.name.as_deref(),
        alias.name_span.as_ref(),
        class,
        NameOccurrence::Declaration,
        None,
        None,
    );
    push_invalid_name(
        invalid,
        alias.target.last().map(String::as_str),
        alias.target_spans.last(),
        class,
        NameOccurrence::AliasTarget,
        None,
        None,
    );
}

pub(super) fn collect_invalid_type_names(
    type_decl: &SyntaxTypeDecl,
    invalid: &mut Vec<InvalidName>,
) {
    push_invalid_name(
        invalid,
        type_decl.name.as_deref(),
        type_decl.name_span.as_ref(),
        NameClass::Type,
        NameOccurrence::Declaration,
        None,
        None,
    );
    for variant in &type_decl.variants {
        push_invalid_name(
            invalid,
            variant.name.as_deref(),
            variant.name_span.as_ref(),
            NameClass::Constructor,
            NameOccurrence::Declaration,
            None,
            None,
        );
    }
}

pub(super) fn collect_invalid_use_name(use_decl: &SyntaxUse, invalid: &mut Vec<InvalidName>) {
    for (index, (segment, span)) in module_path_segments(&use_decl.name)
        .into_iter()
        .zip(use_decl.name_spans.iter())
        .enumerate()
    {
        push_invalid_name(
            invalid,
            Some(segment),
            Some(span),
            NameClass::Module,
            NameOccurrence::PathSegment,
            None,
            Some(index),
        );
    }
}

fn module_path_segments(name: &str) -> Vec<&str> {
    name.split(&[':', '.'])
        .filter(|segment| !segment.is_empty())
        .collect()
}

pub(super) fn collect_invalid_function_names(
    function: &SyntaxFunction,
    invalid: &mut Vec<InvalidName>,
) {
    let enclosing = Some(function.span.clone());
    push_invalid_name(
        invalid,
        function.name.as_deref(),
        function.name_span.as_ref(),
        NameClass::Function,
        NameOccurrence::Declaration,
        enclosing.clone(),
        None,
    );
    for param in &function.params {
        push_invalid_name(
            invalid,
            Some(&param.name),
            Some(&param.name_span),
            NameClass::ValueBinding,
            NameOccurrence::Binding,
            enclosing.clone(),
            None,
        );
        collect_invalid_type_path_names(
            param.ty.as_deref(),
            param.ty_span.as_ref(),
            invalid,
            enclosing.clone(),
        );
    }
    if let Some(binding) = &function.return_binding {
        push_invalid_name(
            invalid,
            Some(&binding.name),
            Some(&binding.span),
            NameClass::ValueBinding,
            NameOccurrence::Binding,
            enclosing.clone(),
            None,
        );
    }
    collect_invalid_type_path_names(
        function.return_type.as_deref(),
        function.return_type_span.as_ref(),
        invalid,
        enclosing.clone(),
    );
    for line in &function.body {
        match line {
            SyntaxBodyLine::Let {
                pattern,
                annotation,
                expr,
                span,
                ..
            } => {
                collect_invalid_pattern_names(pattern, invalid, enclosing.clone());
                collect_invalid_type_path_names(
                    annotation.as_deref(),
                    annotation
                        .as_ref()
                        .map(|annotation| inferred_let_annotation_span(annotation, expr, span))
                        .as_ref(),
                    invalid,
                    enclosing.clone(),
                );
                collect_invalid_expr_names(expr, invalid, enclosing.clone());
            }
            SyntaxBodyLine::Expr { expr, .. } => {
                collect_invalid_expr_names(expr, invalid, enclosing.clone());
            }
        }
    }
}

fn collect_invalid_type_path_names(
    ty: Option<&str>,
    span: Option<&SourceSpan>,
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    let (Some(ty), Some(span)) = (ty, span) else {
        return;
    };
    let Some((segments, segment_spans)) = type_name_path_segments(ty, span) else {
        return;
    };
    if segments.len() <= 1 {
        return;
    }
    for index in 0..segments.len() {
        let class = if index + 1 == segments.len() {
            NameClass::Type
        } else {
            NameClass::Module
        };
        push_invalid_name(
            invalid,
            Some(&segments[index]),
            segment_spans.get(index),
            class,
            NameOccurrence::PathSegment,
            enclosing.clone(),
            Some(index),
        );
    }
}

fn type_name_path_segments(ty: &str, span: &SourceSpan) -> Option<(Vec<String>, Vec<SourceSpan>)> {
    if !ty.contains("::") {
        return None;
    }
    let mut segments = Vec::new();
    let mut spans = Vec::new();
    let bytes = ty.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if !is_ident_start(bytes[cursor]) {
            cursor += 1;
            continue;
        }
        let start = cursor;
        cursor += 1;
        while cursor < bytes.len() && is_ident_continue(bytes[cursor]) {
            cursor += 1;
        }
        let end = cursor;
        let preceded_by_path = start >= 2 && &ty[start - 2..start] == "::";
        let followed_by_path = ty.get(end..end + 2) == Some("::");
        if preceded_by_path || followed_by_path {
            segments.push(ty[start..end].to_string());
            spans.push(offset_span_within(span, start, end));
        }
    }
    (segments.len() > 1).then_some((segments, spans))
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn offset_span_within(span: &SourceSpan, start: usize, end: usize) -> SourceSpan {
    let mut next = span.clone();
    next.start.offset = span.start.offset + start;
    next.end.offset = span.start.offset + end;
    next.start.column = span.start.column + start;
    next.end.column = span.start.column + end;
    next
}

fn inferred_let_annotation_span(
    annotation: &str,
    expr: &SyntaxExpr,
    line_span: &SourceSpan,
) -> SourceSpan {
    let annotation_end = expr.span.start.offset.saturating_sub(3);
    let annotation_start = annotation_end.saturating_sub(annotation.len());
    let mut span = line_span.clone();
    span.start.offset = annotation_start;
    span.end.offset = annotation_end;
    span.start.line = expr.span.start.line;
    span.end.line = expr.span.start.line;
    span.start.column = expr.span.start.column.saturating_sub(2 + annotation.len());
    span.end.column = expr.span.start.column.saturating_sub(2);
    span
}

pub(super) fn collect_invalid_handler_names(
    handler: &SyntaxHandlerDecl,
    invalid: &mut Vec<InvalidName>,
) {
    for param in &handler.params {
        push_invalid_name(
            invalid,
            Some(&param.name),
            Some(&param.name_span),
            NameClass::ValueBinding,
            NameOccurrence::Binding,
            None,
            None,
        );
    }
    for clause in &handler.operation_clauses {
        for param in &clause.params {
            push_invalid_name(
                invalid,
                Some(&param.name),
                Some(&param.name_span),
                NameClass::ValueBinding,
                NameOccurrence::Binding,
                None,
                None,
            );
        }
        collect_invalid_expr_names(&clause.body, invalid, None);
    }
}

fn collect_invalid_pattern_names(
    pattern: &SyntaxPattern,
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    match &pattern.kind {
        SyntaxPatternKind::Binding(name) => push_invalid_name(
            invalid,
            Some(name),
            Some(&pattern.span),
            NameClass::ValueBinding,
            NameOccurrence::PatternHead,
            enclosing,
            None,
        ),
        SyntaxPatternKind::Record(fields) => {
            for field in fields {
                collect_invalid_pattern_names(&field.pattern, invalid, enclosing.clone());
            }
        }
        SyntaxPatternKind::Constructor {
            name,
            name_spans,
            args,
        } => {
            collect_invalid_constructor_path_names(name, name_spans, invalid, enclosing.clone());
            if let [name] = name.as_slice()
                && args.is_empty()
            {
                push_invalid_name(
                    invalid,
                    Some(name),
                    Some(&pattern.span),
                    NameClass::ValueBinding,
                    NameOccurrence::PatternHead,
                    enclosing.clone(),
                    None,
                );
            }
            for arg in args {
                collect_invalid_pattern_names(arg, invalid, enclosing.clone());
            }
        }
        SyntaxPatternKind::Wildcard
        | SyntaxPatternKind::StringLiteral(_)
        | SyntaxPatternKind::IntLiteral(_)
        | SyntaxPatternKind::FloatLiteral(_)
        | SyntaxPatternKind::BoolLiteral(_)
        | SyntaxPatternKind::Unit => {}
    }
}

fn collect_invalid_expr_names(
    expr: &SyntaxExpr,
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    match &expr.kind {
        SyntaxExprKind::Hole {
            satisfy: Some(clause),
            ..
        } => push_invalid_name(
            invalid,
            clause.candidate.as_deref(),
            clause.candidate_span.as_ref(),
            NameClass::ValueBinding,
            NameOccurrence::Binding,
            enclosing,
            None,
        ),
        SyntaxExprKind::TypeApply { callee, .. }
        | SyntaxExprKind::FieldAccess { base: callee, .. }
        | SyntaxExprKind::Try(callee)
        | SyntaxExprKind::Prefix { expr: callee, .. } => {
            collect_invalid_expr_names(callee, invalid, enclosing);
        }
        SyntaxExprKind::Call { callee, args } => {
            collect_invalid_call_callee_names(callee, invalid, enclosing.clone());
            for arg in args {
                collect_invalid_expr_names(arg, invalid, enclosing.clone());
            }
        }
        SyntaxExprKind::Perform { args, .. } => {
            for arg in args {
                collect_invalid_expr_names(arg, invalid, enclosing.clone());
            }
        }
        SyntaxExprKind::Handle { body, args, .. } => {
            collect_invalid_expr_names(body, invalid, enclosing.clone());
            for arg in args {
                collect_invalid_expr_names(arg, invalid, enclosing.clone());
            }
        }
        SyntaxExprKind::SchemaDecode { input, base, .. } => {
            collect_invalid_expr_names(input, invalid, enclosing.clone());
            collect_invalid_expr_names(base, invalid, enclosing);
        }
        SyntaxExprKind::SchemaEncode { value, .. } => {
            collect_invalid_expr_names(value, invalid, enclosing);
        }
        SyntaxExprKind::Record(fields) => {
            for field in fields {
                collect_invalid_expr_names(&field.expr, invalid, enclosing.clone());
            }
        }
        SyntaxExprKind::Dict(entries) => {
            for entry in entries {
                collect_invalid_expr_names(&entry.key, invalid, enclosing.clone());
                collect_invalid_expr_names(&entry.value, invalid, enclosing.clone());
            }
        }
        SyntaxExprKind::List(items) => {
            for item in items {
                collect_invalid_expr_names(item, invalid, enclosing.clone());
            }
        }
        SyntaxExprKind::Match { scrutinee, arms } => {
            collect_invalid_expr_names(scrutinee, invalid, enclosing.clone());
            for arm in arms {
                collect_invalid_pattern_names(&arm.pattern, invalid, enclosing.clone());
                collect_invalid_expr_names(&arm.expr, invalid, enclosing.clone());
            }
        }
        SyntaxExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_invalid_expr_names(condition, invalid, enclosing.clone());
            collect_invalid_expr_names(then_branch, invalid, enclosing.clone());
            for branch in else_if_branches {
                collect_invalid_expr_names(&branch.condition, invalid, enclosing.clone());
                collect_invalid_expr_names(&branch.expr, invalid, enclosing.clone());
            }
            collect_invalid_expr_names(else_branch, invalid, enclosing);
        }
        SyntaxExprKind::Binary { left, right, .. } => {
            collect_invalid_expr_names(left, invalid, enclosing.clone());
            collect_invalid_expr_names(right, invalid, enclosing);
        }
        SyntaxExprKind::NamePath {
            segments,
            segment_spans,
        } => {
            collect_invalid_value_path_names(
                segments,
                segment_spans,
                invalid,
                enclosing.clone(),
                NameClass::ValueBinding,
            );
        }
        SyntaxExprKind::StringLiteral(_)
        | SyntaxExprKind::IntLiteral(_)
        | SyntaxExprKind::FloatLiteral(_)
        | SyntaxExprKind::BoolLiteral(_)
        | SyntaxExprKind::Missing
        | SyntaxExprKind::Hole { .. }
        | SyntaxExprKind::Unit => {}
    }
}

fn collect_invalid_call_callee_names(
    callee: &SyntaxExpr,
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    match &callee.kind {
        SyntaxExprKind::NamePath {
            segments,
            segment_spans,
        } if constructor_path_is_role_fixed(segments) => {
            collect_invalid_constructor_path_names(segments, segment_spans, invalid, enclosing);
        }
        SyntaxExprKind::NamePath {
            segments,
            segment_spans,
        } => {
            collect_invalid_value_path_names(
                segments,
                segment_spans,
                invalid,
                enclosing,
                NameClass::Function,
            );
        }
        _ => collect_invalid_expr_names(callee, invalid, enclosing),
    }
}

fn collect_invalid_constructor_path_names(
    segments: &[String],
    segment_spans: &[SourceSpan],
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    if segments.len() <= 1 {
        return;
    }
    for index in 0..segments.len() {
        let class = if index + 1 == segments.len() {
            NameClass::Constructor
        } else if index + 2 == segments.len() {
            NameClass::Type
        } else {
            NameClass::Module
        };
        push_invalid_name(
            invalid,
            Some(&segments[index]),
            segment_spans.get(index),
            class,
            NameOccurrence::PathSegment,
            enclosing.clone(),
            Some(index),
        );
    }
}

fn collect_invalid_value_path_names(
    segments: &[String],
    segment_spans: &[SourceSpan],
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
    leaf_class: NameClass,
) {
    if segments.len() <= 1 {
        return;
    }
    for index in 0..segments.len() {
        let class = if index + 1 == segments.len() {
            leaf_class
        } else {
            NameClass::Module
        };
        push_invalid_name(
            invalid,
            Some(&segments[index]),
            segment_spans.get(index),
            class,
            NameOccurrence::PathSegment,
            enclosing.clone(),
            Some(index),
        );
    }
}

fn constructor_path_is_role_fixed(segments: &[String]) -> bool {
    segments.len() >= 3
        || matches!(segments, [type_name, constructor] if type_name
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_uppercase)
            && constructor
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_uppercase))
}

fn push_invalid_name(
    invalid: &mut Vec<InvalidName>,
    name: Option<&str>,
    span: Option<&SourceSpan>,
    class: NameClass,
    occurrence: NameOccurrence,
    enclosing_function_span: Option<SourceSpan>,
    segment_index: Option<usize>,
) {
    let (Some(name), Some(span)) = (name, span) else {
        return;
    };
    let valid = match class {
        NameClass::Type | NameClass::Constructor => {
            name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        }
        NameClass::Module | NameClass::Function | NameClass::ValueBinding => {
            name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        }
    };
    if !valid {
        invalid.push(InvalidName {
            name: name.to_string(),
            class,
            occurrence,
            span: span.clone(),
            enclosing_function_span,
            segment_index,
        });
    }
}
