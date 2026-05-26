use crate::{
    BinaryOp, BodyLine, ContractKind, Expr, ExprKind, FunctionDecl, FunctionKind, Pattern,
    PatternKind, PrefixOp, SyntaxItem, SyntaxTree, TokenKind, Visibility,
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
        push_source_line(
            &mut out,
            &comments,
            use_decl.span.start.line,
            0,
            format!("use {}", use_decl.name),
        );
    }
    if (tree.module.is_some() || !tree.uses.is_empty()) && !tree.items.is_empty() {
        out.push('\n');
    }

    for (index, item) in tree.items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let SyntaxItem::Function(function) = item;
        format_function(&mut out, &comments, function);
    }

    if !comments.all_emitted() {
        return lossless_text(tree);
    }

    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn format_function(out: &mut String, comments: &LineComments, function: &FunctionDecl) {
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
    push_source_line(out, comments, function.span.start.line, 0, signature);

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

    for line in &function.body {
        let (source_line, content) = match line {
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
        };
        push_source_line(out, comments, source_line, 1, content);
    }
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

fn canonical_type_text(text: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    while cursor < text.len() {
        let ch = text[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = cursor;
            cursor += ch.len_utf8();
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
            let ident = &text[start..cursor];
            let namespaced_before = text[..start].ends_with("::");
            let namespaced_after = text[cursor..].starts_with("::");
            if ident == "Unit" && !namespaced_before && !namespaced_after {
                out.push_str("()");
            } else {
                out.push_str(ident);
            }
        } else {
            out.push(ch);
            cursor += ch.len_utf8();
        }
    }
    out
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
    let mut rendered = match &expr.kind {
        ExprKind::Missing => "_".to_string(),
        ExprKind::Hole { name, satisfy } => {
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
        ExprKind::NamePath(segments) => segments.join("::"),
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => value.clone(),
        ExprKind::BoolLiteral(true) => "true".to_string(),
        ExprKind::BoolLiteral(false) => "false".to_string(),
        ExprKind::Unit => "()".to_string(),
        ExprKind::TypeApply { callee, type_args } => {
            format!(
                "{}[{}]",
                format_expr_at_indent(callee, indent),
                type_args.join(", ")
            )
        }
        ExprKind::Call { callee, args } => {
            let args = args
                .iter()
                .map(|arg| format_expr_at_indent(arg, indent))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{}({args})",
                format_expr_prec(callee, expr_prec(expr), ExprSide::Left, indent)
            )
        }
        ExprKind::FieldAccess { base, field, .. } => {
            format!(
                "{}.{field}",
                format_expr_prec(base, expr_prec(expr), ExprSide::Left, indent)
            )
        }
        ExprKind::Try(inner) => format!(
            "{}?",
            format_expr_prec(inner, expr_prec(expr), ExprSide::Left, indent)
        ),
        ExprKind::Record(fields) => {
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
        ExprKind::Dict(entries) => {
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
        ExprKind::List(items) => {
            let items = items
                .iter()
                .map(|item| format_expr_at_indent(item, indent))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{items}]")
        }
        ExprKind::Match { scrutinee, arms } => {
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
        ExprKind::Prefix { op, expr: inner } => match op {
            PrefixOp::Not => format!(
                "not {}",
                format_expr_prec(inner, expr_prec(expr), ExprSide::Right, indent)
            ),
            PrefixOp::Negate => format!(
                "-{}",
                format_expr_prec(inner, expr_prec(expr), ExprSide::Right, indent)
            ),
        },
        ExprKind::Binary { op, left, right } => {
            let op_text = binary_op_text(*op);
            format!(
                "{} {op_text} {}",
                format_expr_prec(left, expr_prec(expr), ExprSide::Left, indent),
                format_expr_prec(right, expr_prec(expr), ExprSide::Right, indent)
            )
        }
    };

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
