use super::*;

pub(super) fn format_expr_at_indent(expr: &Expr, indent: usize) -> String {
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
        ExprKind::NamePath { segments, .. } => segments.join("::"),
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
        ExprKind::Perform {
            effect,
            operation,
            args,
            ..
        } => {
            let args = format_expr_args(args, indent);
            format!("perform {}::{}({args})", effect.join("::"), operation)
        }
        ExprKind::Handle {
            body,
            handler,
            args,
            ..
        } => {
            let args = format_expr_args(args, indent);
            format!(
                "handle {} with {}({args})",
                format_expr_at_indent(body, indent),
                handler.join("::")
            )
        }
        ExprKind::SchemaDecode {
            schema,
            input,
            base,
        } => format!(
            "decode {} from {} at {}",
            schema.join("::"),
            format_expr_at_indent(input, indent),
            format_expr_at_indent(base, indent)
        ),
        ExprKind::SchemaEncode { schema, value } => format!(
            "encode {} from {}",
            schema.join("::"),
            format_expr_at_indent(value, indent)
        ),
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
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => format_if_expr(
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            indent,
        ),
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
    let args = format_expr_args(args, indent);
    format!(
        "{}({args})",
        format_expr_prec(callee, prec, ExprSide::Left, indent)
    )
}

fn format_expr_args(args: &[Expr], indent: usize) -> String {
    args.iter()
        .map(|arg| format_expr_at_indent(arg, indent))
        .collect::<Vec<_>>()
        .join(", ")
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
    if let Some(rewrite) = literal_match_rewrite(scrutinee, arms) {
        return format_literal_match_rewrite(&rewrite, indent);
    }
    if let Some(rewrite) = bool_match_rewrite(arms) {
        return format_bool_match_rewrite(scrutinee, &rewrite, indent);
    }

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

pub(super) struct LiteralMatchRewrite<'a> {
    scrutinee: &'a Expr,
    arms: Vec<(String, &'a Expr)>,
    fallback: &'a Expr,
}

pub(super) struct BoolMatchRewrite<'a> {
    true_expr: &'a Expr,
    false_expr: &'a Expr,
}

pub(super) fn bool_match_rewrite(arms: &[crate::MatchArm]) -> Option<BoolMatchRewrite<'_>> {
    let (true_arm, false_arm) = bool_match_arms(arms)?;
    Some(BoolMatchRewrite {
        true_expr: &true_arm.expr,
        false_expr: &false_arm.expr,
    })
}

pub(super) fn literal_match_rewrite<'a>(
    scrutinee: &'a Expr,
    arms: &'a [crate::MatchArm],
) -> Option<LiteralMatchRewrite<'a>> {
    let mut literal_arms = Vec::new();
    let (rewritten_scrutinee, fallback) =
        collect_literal_match_chain(scrutinee, arms, None, &mut literal_arms)?;
    let mut seen = std::collections::BTreeSet::new();
    if literal_arms
        .iter()
        .any(|(literal, _)| !seen.insert(literal.clone()))
    {
        return None;
    }

    Some(LiteralMatchRewrite {
        scrutinee: rewritten_scrutinee,
        arms: literal_arms,
        fallback,
    })
}

fn collect_literal_match_chain<'a>(
    condition: &'a Expr,
    arms: &'a [crate::MatchArm],
    expected_scrutinee: Option<&'a Expr>,
    literal_arms: &mut Vec<(String, &'a Expr)>,
) -> Option<(&'a Expr, &'a Expr)> {
    let (true_arm, false_arm) = bool_match_arms(arms)?;
    let condition_literals = literal_match_conditions(condition)?;
    let active_scrutinee = condition_literals.first()?.0;

    if let Some(expected) = expected_scrutinee
        && !exprs_equivalent(expected, active_scrutinee)
    {
        return None;
    }
    if condition_literals
        .iter()
        .any(|(scrutinee, _)| !exprs_equivalent(active_scrutinee, scrutinee))
    {
        return None;
    }

    for (_, literal) in condition_literals {
        literal_arms.push((literal, &true_arm.expr));
    }

    if let ExprKind::Match {
        scrutinee: next_condition,
        arms: next_arms,
    } = &false_arm.expr.kind
    {
        let mut nested_arms = Vec::new();
        if let Some((_, fallback)) = collect_literal_match_chain(
            next_condition,
            next_arms,
            Some(active_scrutinee),
            &mut nested_arms,
        ) {
            literal_arms.extend(nested_arms);
            return Some((active_scrutinee, fallback));
        }
    }

    Some((active_scrutinee, &false_arm.expr))
}

fn bool_match_arms(arms: &[crate::MatchArm]) -> Option<(&crate::MatchArm, &crate::MatchArm)> {
    if arms.len() != 2 {
        return None;
    }

    let mut true_arm = None;
    let mut false_arm = None;
    for arm in arms {
        match arm.pattern.kind {
            PatternKind::BoolLiteral(true) if true_arm.is_none() => true_arm = Some(arm),
            PatternKind::BoolLiteral(false) if false_arm.is_none() => false_arm = Some(arm),
            _ => return None,
        }
    }

    Some((true_arm?, false_arm?))
}

fn literal_match_conditions(condition: &Expr) -> Option<Vec<(&Expr, String)>> {
    match &condition.kind {
        ExprKind::Binary {
            op: BinaryOp::Or,
            left,
            right,
        } => {
            let mut conditions = literal_match_conditions(left)?;
            conditions.extend(literal_match_conditions(right)?);
            Some(conditions)
        }
        ExprKind::Binary {
            op: BinaryOp::Equal,
            left,
            right,
        } => literal_equality_condition(left, right).map(|condition| vec![condition]),
        _ => None,
    }
}

fn literal_equality_condition<'a>(left: &'a Expr, right: &'a Expr) -> Option<(&'a Expr, String)> {
    if let Some(literal) = literal_pattern_text(right) {
        return Some((left, literal));
    }
    literal_pattern_text(left).map(|literal| (right, literal))
}

fn literal_pattern_text(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => Some(value.clone()),
        ExprKind::Unit => Some("()".to_string()),
        _ => None,
    }
}

fn exprs_equivalent(left: &Expr, right: &Expr) -> bool {
    format_expr_at_indent(left, 0) == format_expr_at_indent(right, 0)
}

fn format_literal_match_rewrite(rewrite: &LiteralMatchRewrite<'_>, indent: usize) -> String {
    let mut text = format!(
        "match {}\n",
        format_expr_at_indent(rewrite.scrutinee, indent)
    );
    for (literal, expr) in &rewrite.arms {
        push_indent(&mut text, indent + 1);
        text.push_str(literal);
        text.push_str(" => ");
        text.push_str(&format_expr_at_indent(expr, indent + 1));
        text.push('\n');
    }
    push_indent(&mut text, indent + 1);
    text.push_str("_ => ");
    text.push_str(&format_expr_at_indent(rewrite.fallback, indent + 1));
    text.push('\n');
    push_indent(&mut text, indent);
    text.push_str("end");
    text
}

fn format_bool_match_rewrite(
    condition: &Expr,
    rewrite: &BoolMatchRewrite<'_>,
    indent: usize,
) -> String {
    let mut text = format!("if {}\n", format_expr_at_indent(condition, indent));
    push_indent(&mut text, indent + 1);
    text.push_str(&format_expr_at_indent(rewrite.true_expr, indent + 1));
    text.push('\n');
    format_bool_match_else(&mut text, rewrite.false_expr, indent);
    text
}

fn format_bool_match_else(text: &mut String, false_expr: &Expr, indent: usize) {
    if let ExprKind::Match {
        scrutinee,
        arms: nested_arms,
    } = &false_expr.kind
        && literal_match_rewrite(scrutinee, nested_arms).is_none()
        && let Some(rewrite) = bool_match_rewrite(nested_arms)
    {
        push_indent(text, indent);
        text.push_str("else if ");
        text.push_str(&format_expr_at_indent(scrutinee, indent));
        text.push('\n');
        push_indent(text, indent + 1);
        text.push_str(&format_expr_at_indent(rewrite.true_expr, indent + 1));
        text.push('\n');
        format_bool_match_else(text, rewrite.false_expr, indent);
        return;
    }

    push_indent(text, indent);
    text.push_str("else\n");
    push_indent(text, indent + 1);
    text.push_str(&format_expr_at_indent(false_expr, indent + 1));
    text.push('\n');
    push_indent(text, indent);
    text.push_str("end");
}

fn format_if_expr(
    condition: &Expr,
    then_branch: &Expr,
    else_if_branches: &[crate::IfBranch],
    else_branch: &Expr,
    indent: usize,
) -> String {
    let mut text = format!("if {}\n", format_expr_at_indent(condition, indent));
    push_indent(&mut text, indent + 1);
    text.push_str(&format_expr_at_indent(then_branch, indent + 1));
    text.push('\n');
    for branch in else_if_branches {
        push_indent(&mut text, indent);
        text.push_str("else if ");
        text.push_str(&format_expr_at_indent(&branch.condition, indent));
        text.push('\n');
        push_indent(&mut text, indent + 1);
        text.push_str(&format_expr_at_indent(&branch.expr, indent + 1));
        text.push('\n');
    }
    push_indent(&mut text, indent);
    text.push_str("else\n");
    push_indent(&mut text, indent + 1);
    text.push_str(&format_expr_at_indent(else_branch, indent + 1));
    text.push('\n');
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
        PrefixOp::BitwiseNot => format!(
            "~{}",
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
            BinaryOp::BitwiseOr => 7,
            BinaryOp::BitwiseXor => 9,
            BinaryOp::BitwiseAnd => 11,
            BinaryOp::Equal | BinaryOp::NotEqual => 13,
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => 15,
            BinaryOp::ShiftLeft | BinaryOp::ShiftRight | BinaryOp::ShiftRightLogical => 17,
            BinaryOp::Add | BinaryOp::Subtract => 19,
            BinaryOp::Multiply | BinaryOp::Divide => 21,
        },
        ExprKind::Prefix { .. } => 25,
        ExprKind::Call { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaEncode { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try(_) => 27,
        ExprKind::Match { .. } | ExprKind::If { .. } => 29,
        _ => 29,
    }
}

pub(super) fn format_pattern(pattern: &Pattern) -> String {
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
        PatternKind::Constructor { name, args, .. } => {
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
        BinaryOp::BitwiseOr => "|",
        BinaryOp::BitwiseXor => "^",
        BinaryOp::BitwiseAnd => "&",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::ShiftLeft => "<<",
        BinaryOp::ShiftRight => ">>",
        BinaryOp::ShiftRightLogical => ">>>",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
    }
}
