use super::*;

pub fn format_tree(tree: &SyntaxTree) -> String {
    let comments = LineComments::from_tree(tree);
    if comments.requires_lossless_preservation {
        return lossless_text(tree);
    }
    if tree_has_commented_match_rewrite(tree, &comments) {
        return lossless_text(tree);
    }

    let mut out = String::new();
    if let Some(module) = &tree.module {
        push_source_line(
            &mut out,
            &comments,
            module.span.start.line,
            0,
            format!("mod {}", module.name),
        );
    }
    for use_decl in &tree.uses {
        let source = match &use_decl.package {
            Some(package) => format!("use {} from \"{}\"", use_decl.name, package.name),
            None => format!("use {}", use_decl.name),
        };
        push_source_line(&mut out, &comments, use_decl.span.start.line, 0, source);
    }
    if (tree.module.is_some() || !tree.uses.is_empty()) && !tree.items.is_empty() {
        out.push('\n');
    }

    for (index, item) in tree.items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        match item {
            SyntaxItem::Function(function) => format_function(&mut out, &comments, function),
            SyntaxItem::Effect(effect) => format_effect_decl(&mut out, &comments, effect),
            SyntaxItem::Handler(handler) => format_handler_decl(&mut out, &comments, handler),
            SyntaxItem::Type(type_decl) => format_type_decl(&mut out, &comments, type_decl),
            SyntaxItem::Schema(schema) => format_schema_decl(&mut out, &comments, schema),
            SyntaxItem::PublicAlias(alias) => {
                push_source_line(
                    &mut out,
                    &comments,
                    alias.span.start.line,
                    0,
                    format_alias(alias),
                );
            }
        }
    }

    if !comments.all_emitted() {
        return lossless_text(tree);
    }

    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn format_effect_decl(out: &mut String, comments: &LineComments, effect: &crate::EffectDecl) {
    let mut header = String::new();
    if effect.visibility == crate::Visibility::Public {
        header.push_str("pub ");
    }
    header.push_str("effect ");
    header.push_str(effect.name.as_deref().unwrap_or("<missing>"));
    push_source_line(out, comments, effect.span.start.line, 0, header);
    for operation in &effect.operations {
        let mut line = String::new();
        line.push_str(operation.name.as_deref().unwrap_or("<missing>"));
        line.push('(');
        for (index, param) in operation.params.iter().enumerate() {
            if index > 0 {
                line.push_str(", ");
            }
            line.push_str(&param.name);
            line.push_str(": ");
            line.push_str(
                param
                    .ty
                    .as_deref()
                    .map(canonical_type_text)
                    .unwrap_or_else(|| "unknown".to_string())
                    .as_str(),
            );
        }
        line.push_str(") -> ");
        line.push_str(
            operation
                .return_type
                .as_deref()
                .map(canonical_type_text)
                .unwrap_or_else(|| "unknown".to_string())
                .as_str(),
        );
        push_source_line(out, comments, operation.span.start.line, 1, line);
    }
    push_source_line(out, comments, effect.span.end.line, 0, String::from("end"));
}

fn format_handler_decl(out: &mut String, comments: &LineComments, handler: &HandlerDecl) {
    let mut header = String::new();
    if handler.visibility == Visibility::Public {
        header.push_str("pub ");
    }
    header.push_str("handler ");
    header.push_str(handler.name.as_deref().unwrap_or("<missing>"));
    header.push('(');
    for (index, param) in handler.params.iter().enumerate() {
        if index > 0 {
            header.push_str(", ");
        }
        header.push_str(&param.name);
        header.push_str(": ");
        header.push_str(
            param
                .ty
                .as_deref()
                .map(canonical_type_text)
                .unwrap_or_else(|| "unknown".to_string())
                .as_str(),
        );
    }
    header.push_str(") handles ");
    header.push_str(&handler.effect.join("::"));
    if let Some(effects) = &handler.effects {
        header.push_str(" effects [");
        header.push_str(&effects.join(", "));
        header.push(']');
    }
    push_source_line(out, comments, handler.span.start.line, 0, header);
    for clause in &handler.operation_clauses {
        let params = clause
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        push_source_line(
            out,
            comments,
            clause.span.start.line,
            1,
            format!(
                "{}({}) => {}",
                clause.operation.as_deref().unwrap_or("<missing>"),
                params,
                format_expr_at_indent(&clause.body, 1)
            ),
        );
    }
    push_source_line(
        out,
        comments,
        handler_end_line(handler),
        0,
        String::from("end"),
    );
}

fn format_schema_decl(out: &mut String, comments: &LineComments, schema: &SchemaDecl) {
    let mut header = String::new();
    if schema.visibility == Visibility::Public {
        header.push_str("pub ");
    }
    header.push_str("schema ");
    header.push_str(schema.name.as_deref().unwrap_or("<missing>"));
    push_source_line(out, comments, schema.span.start.line, 0, header);

    if let Some(format) = &schema.format {
        push_source_line(
            out,
            comments,
            format.span.start.line,
            1,
            format!("format {}", format.name),
        );
    }

    if !schema.fields.is_empty() {
        out.push('\n');
    }
    for field in &schema.fields {
        let mut line = format!(
            "{}: {}",
            field.name,
            canonical_schema_field_type_text(&field.ty, schema_format_is_binary(schema))
        );
        if let Some(where_clause) = &field.where_clause {
            line.push_str(" where ");
            line.push_str(&canonical_predicate_text(&where_clause.predicate));
        }
        push_source_line(out, comments, field.span.start.line, 1, line);
    }

    if !schema.validations.is_empty() && !schema.fields.is_empty() {
        out.push('\n');
    }
    for validation in &schema.validations {
        format_schema_validation(out, comments, validation);
    }

    push_source_line(
        out,
        comments,
        schema_end_line(schema),
        0,
        String::from("end"),
    );
}

fn format_schema_validation(
    out: &mut String,
    comments: &LineComments,
    validation: &SchemaValidationClause,
) {
    push_source_line(
        out,
        comments,
        validation.span.start.line,
        1,
        format!(
            "validate {}",
            canonical_predicate_text(&validation.predicate)
        ),
    );
}

fn schema_format_is_binary(schema: &SchemaDecl) -> bool {
    schema
        .format
        .as_ref()
        .is_some_and(|format| format.name == "binary")
}

fn format_alias(alias: &crate::PublicAliasDecl) -> String {
    let kind = match alias.kind {
        crate::PublicAliasKind::Function => "fn",
        crate::PublicAliasKind::Type => "type",
        crate::PublicAliasKind::Schema => "schema",
    };
    format!(
        "pub {kind} {} = {}",
        alias.name.as_deref().unwrap_or("<missing>"),
        alias.target.join("::")
    )
}

fn format_type_decl(out: &mut String, comments: &LineComments, type_decl: &TypeDecl) {
    let mut header = String::new();
    if type_decl.visibility == Visibility::Public {
        header.push_str("pub ");
    }
    header.push_str("type ");
    header.push_str(type_decl.name.as_deref().unwrap_or("<missing>"));
    if !type_decl.params.is_empty() {
        header.push('<');
        header.push_str(&type_decl.params.join(", "));
        header.push('>');
    }
    push_source_line(out, comments, type_decl.span.start.line, 0, header);

    for variant in &type_decl.variants {
        push_source_line(
            out,
            comments,
            variant.span.start.line,
            1,
            format_type_variant(variant),
        );
    }

    let end_line = type_end_line(type_decl);
    comments.emit_before_first_after(type_body_end_line(type_decl), end_line, out, 1);
    push_source_line(out, comments, end_line, 0, String::from("end"));
}

fn format_type_variant(variant: &TypeVariantDecl) -> String {
    let mut line = String::new();
    if variant.visibility == Visibility::Public {
        line.push_str("pub ");
    }
    line.push_str(variant.name.as_deref().unwrap_or("<missing>"));
    if variant.fields.is_empty() {
        return line;
    }

    match variant
        .field_delimiter
        .unwrap_or(TypeVariantFieldDelimiter::Tuple)
    {
        TypeVariantFieldDelimiter::Tuple => {
            line.push('(');
            for (index, field) in variant.fields.iter().enumerate() {
                if index > 0 {
                    line.push_str(", ");
                }
                if !is_default_positional_field(index, &field.name) {
                    line.push_str(&field.name);
                    line.push_str(": ");
                }
                line.push_str(&canonical_type_text(&field.ty));
            }
            line.push(')');
        }
        TypeVariantFieldDelimiter::Record => {
            line.push_str(" { ");
            for (index, field) in variant.fields.iter().enumerate() {
                if index > 0 {
                    line.push_str(", ");
                }
                line.push_str(&field.name);
                line.push_str(": ");
                line.push_str(&canonical_type_text(&field.ty));
            }
            line.push_str(" }");
        }
    }
    line
}

fn is_default_positional_field(index: usize, name: &str) -> bool {
    if index == 0 {
        name == "value"
    } else {
        name == format!("_{index}")
    }
}

fn schema_end_line(schema: &SchemaDecl) -> usize {
    if schema.end_present {
        schema.span.end.line
    } else {
        schema.span.start.line.max(schema_body_end_line(schema))
    }
}

fn schema_body_end_line(schema: &SchemaDecl) -> usize {
    let field_end = schema
        .fields
        .last()
        .map(|field| field.span.end.line)
        .unwrap_or(schema.span.start.line);
    let validation_end = schema
        .validations
        .last()
        .map(|validation| validation.span.end.line)
        .unwrap_or(schema.span.start.line);
    let format_end = schema
        .format
        .as_ref()
        .map(|format| format.span.end.line)
        .unwrap_or(schema.span.start.line);
    field_end.max(validation_end).max(format_end)
}

fn handler_end_line(handler: &HandlerDecl) -> usize {
    if handler.end_present && handler.span.end.column == 1 {
        handler.span.end.line.saturating_sub(1)
    } else {
        handler.span.end.line
    }
}

fn format_function(out: &mut String, comments: &LineComments, function: &FunctionDecl) {
    push_source_line(
        out,
        comments,
        function.span.start.line,
        0,
        format_function_signature(function),
    );
    format_function_contracts(out, comments, function);
    format_function_body(out, comments, function);
    format_function_end(out, comments, function);
}

fn format_function_signature(function: &FunctionDecl) -> String {
    let mut signature = String::new();
    if function.kind == FunctionKind::Test {
        signature.push_str("test ");
    } else {
        if function.visibility == Visibility::Public {
            signature.push_str("pub ");
        }
        signature.push_str("fn ");
    }
    signature.push_str(function.name.as_deref().unwrap_or("<missing>"));
    if let Some(binder) = &function.effect_binder {
        signature.push_str("<effect ");
        signature.push_str(&binder.name);
        signature.push('>');
    }
    signature.push('(');
    for (index, param) in function.params.iter().enumerate() {
        if index > 0 {
            signature.push_str(", ");
        }
        signature.push_str(&param.name);
        if let Some(ty) = &param.ty {
            signature.push_str(": ");
            if param.is_variadic {
                signature.push_str("...");
            }
            signature.push_str(&canonical_type_text(ty));
        }
    }
    signature.push(')');
    if let Some(return_type) = &function.return_type {
        signature.push_str(" -> ");
        if let Some(result_binding) = &function.return_binding {
            signature.push_str(&result_binding.name);
            signature.push_str(": ");
        }
        signature.push_str(&canonical_type_text(return_type));
    }
    if let Some(effects) = &function.effects {
        signature.push_str(" effects [");
        for (index, effect) in effects.iter().enumerate() {
            if index > 0 {
                signature.push_str(", ");
            }
            signature.push_str(effect);
        }
        signature.push(']');
    }
    signature
}

fn format_function_contracts(out: &mut String, comments: &LineComments, function: &FunctionDecl) {
    for contract in &function.contracts {
        let mut line = String::new();
        line.push_str(match contract.kind {
            ContractKind::Require => "require",
            ContractKind::Ensure => "ensure",
            ContractKind::Invariant => "invariant",
        });
        if !contract.text.is_empty() {
            line.push(' ');
            line.push_str(&contract.text);
        }
        push_source_line(out, comments, contract.span.start.line, 1, line);
    }
}

fn format_function_body(out: &mut String, comments: &LineComments, function: &FunctionDecl) {
    for line in &function.body {
        let (source_line, content) = format_body_line(line);
        push_source_line(out, comments, source_line, 1, content);
    }
}

fn format_body_line(line: &BodyLine) -> (usize, String) {
    match line {
        BodyLine::Let {
            pattern,
            annotation,
            expr,
            span,
            ..
        } => {
            let mut content = String::from("let ");
            content.push_str(&format_pattern(pattern));
            if let Some(annotation) = annotation {
                content.push_str(": ");
                content.push_str(&canonical_type_text(annotation));
            }
            content.push_str(" = ");
            content.push_str(&format_expr_at_indent(expr, 1));
            (span.start.line, content)
        }
        BodyLine::Expr { expr, span } => (span.start.line, format_expr_at_indent(expr, 1)),
    }
}

fn format_function_end(out: &mut String, comments: &LineComments, function: &FunctionDecl) {
    let end_line = function_end_line(function);
    comments.emit_before_first_after(function_body_end_line(function), end_line, out, 1);
    push_source_line(out, comments, end_line, 0, String::from("end"));
}
