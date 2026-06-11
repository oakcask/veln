use crate::{
    BinaryOp, BodyLine, ContractKind, Expr, ExprKind, FunctionDecl, FunctionKind, Pattern,
    PatternKind, PrefixOp, SchemaDecl, SyntaxItem, SyntaxTree, TokenKind, TypeDecl,
    TypeVariantDecl, TypeVariantFieldDelimiter, Visibility,
};

pub fn format_tree(tree: &SyntaxTree) -> String {
    let comments = LineComments::from_tree(tree);
    if comments.requires_lossless_preservation {
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
            canonical_schema_field_type_text(&field.ty)
        );
        if let Some(where_clause) = &field.where_clause {
            line.push_str(" where ");
            line.push_str(&canonical_predicate_text(&where_clause.predicate));
        }
        push_source_line(out, comments, field.span.start.line, 1, line);
    }

    push_source_line(
        out,
        comments,
        schema_end_line(schema),
        0,
        String::from("end"),
    );
}

fn format_alias(alias: &crate::PublicAliasDecl) -> String {
    let kind = match alias.kind {
        crate::PublicAliasKind::Function => "fn",
        crate::PublicAliasKind::Type => "type",
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
    schema
        .fields
        .last()
        .map(|field| field.span.end.line)
        .or_else(|| schema.format.as_ref().map(|format| format.span.end.line))
        .unwrap_or(schema.span.start.line)
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
    signature.push('(');
    for (index, param) in function.params.iter().enumerate() {
        if index > 0 {
            signature.push_str(", ");
        }
        signature.push_str(&param.name);
        if let Some(ty) = &param.ty {
            signature.push_str(": ");
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

fn lossless_text(tree: &SyntaxTree) -> String {
    tree.lossless_tokens()
        .filter(|token| token.kind != TokenKind::Eof)
        .map(|token| token.text.as_str())
        .collect()
}

fn push_source_line(
    out: &mut String,
    comments: &LineComments,
    source_line: usize,
    indent: usize,
    content: String,
) {
    comments.emit_before(source_line, out, indent);
    push_indent(out, indent);
    out.push_str(&content);
    comments.emit_after(source_line, out);
    out.push('\n');
}

fn push_indent(out: &mut String, level: usize) {
    out.push_str(&"\t".repeat(level));
}

#[derive(Default)]
struct LineComments {
    before: std::cell::RefCell<std::collections::BTreeMap<usize, Vec<String>>>,
    after: std::cell::RefCell<std::collections::BTreeMap<usize, Vec<String>>>,
    requires_lossless_preservation: bool,
}

impl LineComments {
    fn from_tree(tree: &SyntaxTree) -> Self {
        let mut line = 1usize;
        let mut seen_code_on_line = false;
        let mut pending = Vec::new();
        let mut before = std::collections::BTreeMap::<usize, Vec<String>>::new();
        let mut after = std::collections::BTreeMap::<usize, Vec<String>>::new();
        let mut requires_lossless_preservation = false;

        for token in tree.lossless_tokens() {
            match token.kind {
                TokenKind::Newline => {
                    line += 1;
                    seen_code_on_line = false;
                }
                TokenKind::Whitespace => {}
                TokenKind::Comment => {
                    if seen_code_on_line {
                        after
                            .entry(line)
                            .or_default()
                            .push(token.text.trim_start().to_string());
                    } else {
                        pending.push(token.text.trim_start().to_string());
                    }
                }
                TokenKind::Eof => {}
                _ => {
                    if !seen_code_on_line && !pending.is_empty() {
                        before.entry(line).or_default().append(&mut pending);
                    }
                    seen_code_on_line = true;
                }
            }
        }

        if !pending.is_empty() {
            requires_lossless_preservation = true;
        }

        Self {
            before: std::cell::RefCell::new(before),
            after: std::cell::RefCell::new(after),
            requires_lossless_preservation,
        }
    }

    fn emit_before(&self, source_line: usize, out: &mut String, indent: usize) {
        let Some(comments) = self.before.borrow_mut().remove(&source_line) else {
            return;
        };
        for comment in comments {
            push_indent(out, indent);
            out.push_str(&comment);
            out.push('\n');
        }
    }

    fn emit_before_first_after(
        &self,
        after_line: usize,
        through_line: usize,
        out: &mut String,
        indent: usize,
    ) {
        let Some(line) = self
            .before
            .borrow()
            .keys()
            .copied()
            .find(|line| *line > after_line && *line <= through_line)
        else {
            return;
        };
        self.emit_before(line, out, indent);
    }

    fn emit_after(&self, source_line: usize, out: &mut String) {
        let Some(comments) = self.after.borrow_mut().remove(&source_line) else {
            return;
        };
        for comment in comments {
            out.push_str("  ");
            out.push_str(&comment);
        }
    }

    fn all_emitted(&self) -> bool {
        self.before.borrow().is_empty() && self.after.borrow().is_empty()
    }
}

fn function_body_end_line(function: &FunctionDecl) -> usize {
    function
        .body
        .last()
        .map(|line| match line {
            BodyLine::Let { span, .. } | BodyLine::Expr { span, .. } => span.start.line,
        })
        .or_else(|| {
            function
                .contracts
                .last()
                .map(|contract| contract.span.start.line)
        })
        .unwrap_or(function.span.start.line)
}

fn function_end_line(function: &FunctionDecl) -> usize {
    if function.end_present && function.span.end.column == 1 {
        function.span.end.line.saturating_sub(1)
    } else {
        function.span.end.line
    }
}

fn type_body_end_line(type_decl: &TypeDecl) -> usize {
    type_decl
        .variants
        .last()
        .map(|variant| variant.span.start.line)
        .unwrap_or(type_decl.span.start.line)
}

fn type_end_line(type_decl: &TypeDecl) -> usize {
    if type_decl.end_present && type_decl.span.end.column == 1 {
        type_decl.span.end.line.saturating_sub(1)
    } else {
        type_decl.span.end.line
    }
}

pub fn canonical_type_text(text: &str) -> String {
    canonicalize_type_segment(text)
}

fn canonical_schema_field_type_text(text: &str) -> String {
    canonical_predicate_text(text)
}

fn canonical_predicate_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(" . ", ".")
        .replace("[ ", "[")
        .replace(" ]", "]")
        .replace(" ,", ",")
}

fn canonicalize_type_segment(text: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    while cursor < text.len() {
        let ch = text[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = cursor;
            cursor = consume_type_path(text, cursor);
            let path = &text[start..cursor];
            if path == "Unit" {
                out.push_str("()");
            } else {
                out.push_str(path);
            }

            if path != "fn" && text[cursor..].starts_with('(') {
                if let Some(close) = matching_delimiter(text, cursor, '(', ')') {
                    out.push('<');
                    out.push_str(&canonicalize_type_segment(&text[cursor + 1..close]));
                    out.push('>');
                    cursor = close + 1;
                }
            } else if text[cursor..].starts_with('<')
                && let Some(close) = matching_delimiter(text, cursor, '<', '>')
            {
                out.push('<');
                out.push_str(&canonicalize_type_segment(&text[cursor + 1..close]));
                out.push('>');
                cursor = close + 1;
            }
        } else {
            out.push(ch);
            cursor += ch.len_utf8();
        }
    }
    out
}

fn consume_type_path(text: &str, mut cursor: usize) -> usize {
    cursor = consume_ident(text, cursor);
    while text[cursor..].starts_with("::") {
        let segment_start = cursor + 2;
        let segment_end = consume_ident(text, segment_start);
        if segment_end == segment_start {
            break;
        }
        cursor = segment_end;
    }
    cursor
}

fn consume_ident(text: &str, mut cursor: usize) -> usize {
    while cursor < text.len() {
        let ch = text[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        if ch.is_ascii_alphanumeric() || ch == '_' {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    cursor
}

fn matching_delimiter(text: &str, open: usize, open_ch: char, close_ch: char) -> Option<usize> {
    let mut cursor = open;
    let mut depth = 0usize;
    while cursor < text.len() {
        let ch = text[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        if ch == open_ch {
            depth += 1;
        } else if ch == close_ch {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(cursor);
            }
        }
        cursor += ch.len_utf8();
    }
    None
}

fn format_expr_at_indent(expr: &Expr, indent: usize) -> String {
    format_expr_prec(expr, 0, ExprSide::Root, indent)
}

#[derive(Clone, Copy)]
enum ExprSide {
    Root,
    Left,
    Right,
}

fn format_expr_prec(expr: &Expr, parent_prec: u8, side: ExprSide, indent: usize) -> String {
    let prec = expr_prec(expr);
    let mut rendered = format_expr_inner(expr, prec, indent);

    let needs_parens = match side {
        ExprSide::Root | ExprSide::Left => prec < parent_prec,
        ExprSide::Right => prec <= parent_prec && matches!(expr.kind, ExprKind::Binary { .. }),
    };
    if needs_parens {
        rendered.insert(0, '(');
        rendered.push(')');
    }
    rendered
}

fn format_expr_inner(expr: &Expr, prec: u8, indent: usize) -> String {
    match &expr.kind {
        ExprKind::Missing => "_".to_string(),
        ExprKind::Hole { name, satisfy } => format_hole_expr(name.as_deref(), satisfy.as_ref()),
        ExprKind::NamePath(segments) => segments.join("::"),
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => value.clone(),
        ExprKind::BoolLiteral(true) => "true".to_string(),
        ExprKind::BoolLiteral(false) => "false".to_string(),
        ExprKind::Unit => "()".to_string(),
        ExprKind::TypeApply { callee, type_args } => {
            let type_args = type_args
                .iter()
                .map(|arg| canonical_type_text(arg))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", format_expr_at_indent(callee, indent), type_args)
        }
        ExprKind::Call { callee, args } => format_call_expr(callee, args, prec, indent),
        ExprKind::FieldAccess { base, field, .. } => {
            format!(
                "{}.{field}",
                format_expr_prec(base, prec, ExprSide::Left, indent)
            )
        }
        ExprKind::Try(inner) => {
            format!("{}?", format_expr_prec(inner, prec, ExprSide::Left, indent))
        }
        ExprKind::Record(fields) => format_record_expr(fields, indent),
        ExprKind::Dict(entries) => format_dict_expr(entries, indent),
        ExprKind::List(items) => format_list_expr(items, indent),
        ExprKind::Match { scrutinee, arms } => format_match_expr(scrutinee, arms, indent),
        ExprKind::Prefix { op, expr: inner } => format_prefix_expr(*op, inner, prec, indent),
        ExprKind::Binary { op, left, right } => format_binary_expr(*op, left, right, prec, indent),
    }
}

fn format_hole_expr(name: Option<&str>, satisfy: Option<&crate::SatisfyClause>) -> String {
    let mut text = String::from("_");
    if let Some(name) = name {
        text.push_str(name);
    }
    if let Some(satisfy) = satisfy {
        text.push_str(" satisfy");
        if let Some(candidate) = &satisfy.candidate {
            text.push(' ');
            text.push_str(candidate);
        }
        if !satisfy.predicate.is_empty() {
            text.push_str(" => ");
            text.push_str(&satisfy.predicate);
        }
    }
    text
}

fn format_call_expr(callee: &Expr, args: &[Expr], prec: u8, indent: usize) -> String {
    let args = args
        .iter()
        .map(|arg| format_expr_at_indent(arg, indent))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}({args})",
        format_expr_prec(callee, prec, ExprSide::Left, indent)
    )
}

fn format_record_expr(fields: &[crate::RecordField], indent: usize) -> String {
    if fields.is_empty() {
        return "{}".to_string();
    }
    let fields = fields
        .iter()
        .map(|field| {
            format!(
                "{}: {}",
                field.name,
                format_expr_at_indent(&field.expr, indent)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {fields} }}")
}

fn format_dict_expr(entries: &[crate::DictEntry], indent: usize) -> String {
    let entries = entries
        .iter()
        .map(|entry| {
            format!(
                "{}: {}",
                format_expr_at_indent(&entry.key, indent),
                format_expr_at_indent(&entry.value, indent)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {entries} }}")
}

fn format_list_expr(items: &[Expr], indent: usize) -> String {
    let items = items
        .iter()
        .map(|item| format_expr_at_indent(item, indent))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{items}]")
}

fn format_match_expr(scrutinee: &Expr, arms: &[crate::MatchArm], indent: usize) -> String {
    let mut text = format!("match {}\n", format_expr_at_indent(scrutinee, indent));
    for arm in arms {
        push_indent(&mut text, indent + 1);
        text.push_str(&format_pattern(&arm.pattern));
        text.push_str(" => ");
        text.push_str(&format_expr_at_indent(&arm.expr, indent + 1));
        text.push('\n');
    }
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

fn format_prefix_expr(op: PrefixOp, inner: &Expr, prec: u8, indent: usize) -> String {
    match op {
        PrefixOp::Not => format!(
            "not {}",
            format_expr_prec(inner, prec, ExprSide::Right, indent)
        ),
        PrefixOp::Negate => format!(
            "-{}",
            format_expr_prec(inner, prec, ExprSide::Right, indent)
        ),
    }
}

fn format_binary_expr(op: BinaryOp, left: &Expr, right: &Expr, prec: u8, indent: usize) -> String {
    let op_text = binary_op_text(op);
    format!(
        "{} {op_text} {}",
        format_expr_prec(left, prec, ExprSide::Left, indent),
        format_expr_prec(right, prec, ExprSide::Right, indent)
    )
}

fn expr_prec(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Binary { op, .. } => match op {
            BinaryOp::PipeGreater => 1,
            BinaryOp::Or => 3,
            BinaryOp::And => 5,
            BinaryOp::Equal | BinaryOp::NotEqual => 7,
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => 9,
            BinaryOp::Add | BinaryOp::Subtract => 11,
            BinaryOp::Multiply | BinaryOp::Divide => 13,
        },
        ExprKind::Prefix { .. } => 15,
        ExprKind::Call { .. } | ExprKind::FieldAccess { .. } | ExprKind::Try(_) => 17,
        ExprKind::Match { .. } => 19,
        _ => 19,
    }
}

fn format_pattern(pattern: &Pattern) -> String {
    match &pattern.kind {
        PatternKind::Wildcard => "_".to_string(),
        PatternKind::Binding(name) => name.clone(),
        PatternKind::StringLiteral(value)
        | PatternKind::IntLiteral(value)
        | PatternKind::FloatLiteral(value) => value.clone(),
        PatternKind::BoolLiteral(true) => "true".to_string(),
        PatternKind::BoolLiteral(false) => "false".to_string(),
        PatternKind::Unit => "()".to_string(),
        PatternKind::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, format_pattern(&field.pattern)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        PatternKind::Constructor { name, args } => {
            if args.is_empty() {
                name.join("::")
            } else {
                let args = args
                    .iter()
                    .map(format_pattern)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({args})", name.join("::"))
            }
        }
    }
}

fn binary_op_text(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::PipeGreater => "|>",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
    }
}
