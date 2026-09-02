use super::*;

pub(super) fn collect_invalid_module_header(module: &SyntaxModule, invalid: &mut Vec<InvalidName>) {
    push_invalid_name(
        invalid,
        Some(&module.name),
        module.name_spans.first(),
        NameClass::Module,
        NameOccurrence::Declaration,
        None,
        None,
    );
}

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
        for field in &variant.fields {
            collect_invalid_type_path_names(&field.ty_paths, invalid, None);
        }
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
        collect_invalid_type_path_names(&param.ty_paths, invalid, enclosing.clone());
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
    collect_invalid_type_path_names(&function.return_type_paths, invalid, enclosing.clone());
    for line in &function.body {
        match line {
            SyntaxBodyLine::Let {
                pattern,
                annotation_paths,
                expr,
                ..
            } => {
                collect_invalid_pattern_names(pattern, invalid, enclosing.clone());
                collect_invalid_type_path_names(annotation_paths, invalid, enclosing.clone());
                collect_invalid_expr_names(expr, invalid, enclosing.clone());
            }
            SyntaxBodyLine::Expr { expr, .. } => {
                collect_invalid_expr_names(expr, invalid, enclosing.clone());
            }
        }
    }
}

fn collect_invalid_type_path_names(
    paths: &[veln_syntax::TypePathSegments],
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    for path in paths {
        for index in 0..path.segments.len() {
            let class = if index + 1 == path.segments.len() {
                NameClass::Type
            } else {
                NameClass::Module
            };
            push_invalid_name(
                invalid,
                Some(&path.segments[index]),
                path.segment_spans.get(index),
                class,
                NameOccurrence::PathSegment,
                enclosing.clone(),
                Some(index),
            );
        }
    }
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
        collect_invalid_type_path_names(&param.ty_paths, invalid, None);
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
            collect_invalid_type_path_names(&param.ty_paths, invalid, None);
        }
        collect_invalid_expr_names(&clause.body, invalid, None);
    }
}

pub(super) fn collect_invalid_effect_names(
    effect: &SyntaxEffectDecl,
    invalid: &mut Vec<InvalidName>,
) {
    for operation in &effect.operations {
        for param in &operation.params {
            collect_invalid_type_path_names(&param.ty_paths, invalid, None);
        }
        collect_invalid_type_path_names(&operation.return_type_paths, invalid, None);
    }
}

pub(super) fn collect_invalid_schema_names(
    schema: &SyntaxSchemaDecl,
    invalid: &mut Vec<InvalidName>,
) {
    for field in &schema.fields {
        collect_invalid_type_path_names(&field.ty_paths, invalid, None);
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
            collect_invalid_expr_list(args, invalid, enclosing);
        }
        SyntaxExprKind::Perform { args, .. } => collect_invalid_expr_list(args, invalid, enclosing),
        SyntaxExprKind::Handle { body, args, .. } => {
            collect_invalid_expr_names(body, invalid, enclosing.clone());
            collect_invalid_expr_list(args, invalid, enclosing);
        }
        SyntaxExprKind::SchemaDecode { input, base, .. } => {
            collect_invalid_expr_names(input, invalid, enclosing.clone());
            collect_invalid_expr_names(base, invalid, enclosing);
        }
        SyntaxExprKind::SchemaEncode { value, .. } => {
            collect_invalid_expr_names(value, invalid, enclosing);
        }
        SyntaxExprKind::Record(fields) => {
            collect_invalid_record_field_expr_names(fields, invalid, enclosing);
        }
        SyntaxExprKind::Dict(entries) => {
            collect_invalid_dict_entry_expr_names(entries, invalid, enclosing);
        }
        SyntaxExprKind::List(items) => collect_invalid_expr_list(items, invalid, enclosing),
        SyntaxExprKind::Match { scrutinee, arms } => {
            collect_invalid_match_expr_names(scrutinee, arms, invalid, enclosing);
        }
        SyntaxExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_invalid_if_expr_names(
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                invalid,
                enclosing,
            );
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

fn collect_invalid_expr_list(
    exprs: &[SyntaxExpr],
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    for expr in exprs {
        collect_invalid_expr_names(expr, invalid, enclosing.clone());
    }
}

fn collect_invalid_record_field_expr_names(
    fields: &[SyntaxRecordField],
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    for field in fields {
        collect_invalid_expr_names(&field.expr, invalid, enclosing.clone());
    }
}

fn collect_invalid_dict_entry_expr_names(
    entries: &[SyntaxDictEntry],
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    for entry in entries {
        collect_invalid_expr_names(&entry.key, invalid, enclosing.clone());
        collect_invalid_expr_names(&entry.value, invalid, enclosing.clone());
    }
}

fn collect_invalid_match_expr_names(
    scrutinee: &SyntaxExpr,
    arms: &[SyntaxMatchArm],
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    collect_invalid_expr_names(scrutinee, invalid, enclosing.clone());
    for arm in arms {
        collect_invalid_pattern_names(&arm.pattern, invalid, enclosing.clone());
        collect_invalid_expr_names(&arm.expr, invalid, enclosing.clone());
    }
}

fn collect_invalid_if_expr_names(
    condition: &SyntaxExpr,
    then_branch: &SyntaxExpr,
    else_if_branches: &[SyntaxIfBranch],
    else_branch: &SyntaxExpr,
    invalid: &mut Vec<InvalidName>,
    enclosing: Option<SourceSpan>,
) {
    collect_invalid_expr_names(condition, invalid, enclosing.clone());
    collect_invalid_expr_names(then_branch, invalid, enclosing.clone());
    for branch in else_if_branches {
        collect_invalid_expr_names(&branch.condition, invalid, enclosing.clone());
        collect_invalid_expr_names(&branch.expr, invalid, enclosing.clone());
    }
    collect_invalid_expr_names(else_branch, invalid, enclosing);
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
