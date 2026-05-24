use crate::{
    BinaryOp, BodyLine, ContractKind, Expr, ExprKind, FunctionDecl, PrefixOp, SyntaxItem,
    SyntaxTree, TokenKind, Visibility,
};

pub fn format_tree(tree: &SyntaxTree) -> String {
    if tree
        .lossless_tokens()
        .any(|token| token.kind == TokenKind::Comment)
    {
        return tree
            .lossless_tokens()
            .filter(|token| token.kind != TokenKind::Eof)
            .map(|token| token.text.as_str())
            .collect();
    }

    let mut out = String::new();
    if let Some(module) = &tree.module {
        push_line(&mut out, format_args!("mod {}", module.name));
    }
    for use_decl in &tree.uses {
        push_line(&mut out, format_args!("use {}", use_decl.name));
    }
    if (tree.module.is_some() || !tree.uses.is_empty()) && !tree.items.is_empty() {
        out.push('\n');
    }

    for (index, item) in tree.items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let SyntaxItem::Function(function) = item;
        format_function(&mut out, function);
    }

    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn format_function(out: &mut String, function: &FunctionDecl) {
    if function.visibility == Visibility::Public {
        out.push_str("pub ");
    }
    out.push_str("fn ");
    out.push_str(function.name.as_deref().unwrap_or("<missing>"));
    out.push('(');
    for (index, param) in function.params.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&param.name);
        if let Some(ty) = &param.ty {
            out.push_str(": ");
            out.push_str(&canonical_type_text(ty));
        }
    }
    out.push(')');
    if let Some(return_type) = &function.return_type {
        out.push_str(" -> ");
        out.push_str(&canonical_type_text(return_type));
    }
    if let Some(effects) = &function.effects {
        out.push_str(" effects [");
        for (index, effect) in effects.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            out.push_str(effect);
        }
        out.push(']');
    }
    out.push('\n');

    for contract in &function.contracts {
        out.push_str("  ");
        out.push_str(match contract.kind {
            ContractKind::Require => "require",
            ContractKind::Ensure => "ensure",
        });
        if !contract.text.is_empty() {
            out.push(' ');
            out.push_str(&contract.text);
        }
        out.push('\n');
    }

    for line in &function.body {
        out.push_str("  ");
        match line {
            BodyLine::Let {
                name,
                annotation,
                expr,
                ..
            } => {
                out.push_str("let ");
                out.push_str(name.as_deref().unwrap_or("<missing>"));
                if let Some(annotation) = annotation {
                    out.push_str(": ");
                    out.push_str(&canonical_type_text(annotation));
                }
                out.push_str(" = ");
                out.push_str(&format_expr(expr));
            }
            BodyLine::Expr { expr, .. } => out.push_str(&format_expr(expr)),
        }
        out.push('\n');
    }
    out.push_str("end\n");
}

fn push_line(out: &mut String, args: std::fmt::Arguments<'_>) {
    use std::fmt::Write as _;

    out.write_fmt(args)
        .expect("writing to String should not fail");
    out.push('\n');
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

fn format_expr(expr: &Expr) -> String {
    format_expr_prec(expr, 0, ExprSide::Root)
}

#[derive(Clone, Copy)]
enum ExprSide {
    Root,
    Left,
    Right,
}

fn format_expr_prec(expr: &Expr, parent_prec: u8, side: ExprSide) -> String {
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
        ExprKind::Unit => "()".to_string(),
        ExprKind::Call { callee, args } => {
            let args = args.iter().map(format_expr).collect::<Vec<_>>().join(", ");
            format!(
                "{}({args})",
                format_expr_prec(callee, expr_prec(expr), ExprSide::Left)
            )
        }
        ExprKind::Try(inner) => format!(
            "{}?",
            format_expr_prec(inner, expr_prec(expr), ExprSide::Left)
        ),
        ExprKind::Record(fields) => {
            if fields.is_empty() {
                return "{}".to_string();
            }
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, format_expr(&field.expr)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        ExprKind::List(items) => {
            let items = items.iter().map(format_expr).collect::<Vec<_>>().join(", ");
            format!("[{items}]")
        }
        ExprKind::Prefix { op, expr: inner } => match op {
            PrefixOp::Not => format!(
                "not {}",
                format_expr_prec(inner, expr_prec(expr), ExprSide::Right)
            ),
            PrefixOp::Negate => format!(
                "-{}",
                format_expr_prec(inner, expr_prec(expr), ExprSide::Right)
            ),
        },
        ExprKind::Binary { op, left, right } => {
            let op_text = binary_op_text(*op);
            format!(
                "{} {op_text} {}",
                format_expr_prec(left, expr_prec(expr), ExprSide::Left),
                format_expr_prec(right, expr_prec(expr), ExprSide::Right)
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
        ExprKind::Call { .. } | ExprKind::Try(_) => 17,
        _ => 19,
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
