use super::*;

pub(super) fn abc_subject(
    source: &SourceFile,
    path: &str,
    function: &FunctionDecl,
) -> AbcSubjectMetric {
    let vector = abc_vector(function);
    let kind = match function.kind {
        FunctionKind::Function => AbcSubjectKind::Function,
        FunctionKind::Test => AbcSubjectKind::Test,
    };
    let name = function
        .name
        .clone()
        .unwrap_or_else(|| "<anonymous>".to_string());
    AbcSubjectMetric {
        identity: format!("{path}::{name}"),
        path: path.to_string(),
        name,
        kind,
        vector,
        magnitude: vector.magnitude(),
        contracts_included: false,
        generated: false,
        span: source.span(veln_source::TextRange::new(
            function.span.start.offset,
            function.span.end.offset,
        )),
    }
}

pub(super) fn is_generated_or_doctest_path(path: &str) -> bool {
    path.contains("#doctest-") || path.split('/').any(|segment| segment == "target")
}

pub(super) fn abc_vector(function: &FunctionDecl) -> AbcVector {
    let mut vector = AbcVector::default();
    count_body(&function.body, &mut vector);
    vector
}

fn count_body(body: &[BodyLine], vector: &mut AbcVector) {
    for line in body {
        match line {
            BodyLine::Let { expr, .. } => {
                vector.assignments += 1;
                count_expr(expr, vector);
            }
            BodyLine::Expr { expr, .. } => count_expr(expr, vector),
            BodyLine::Defer { body, .. } => count_body(body, vector),
        }
    }
}

pub(super) fn count_expr(expr: &Expr, vector: &mut AbcVector) {
    let contribution = expr_contribution(expr);
    vector.branches += contribution.branches;
    vector.conditionals += contribution.conditionals;
    count_expr_children(expr, vector);
}

#[derive(Clone, Copy, Default)]
struct ExprContribution {
    branches: usize,
    conditionals: usize,
}

fn expr_contribution(expr: &Expr) -> ExprContribution {
    match &expr.kind {
        ExprKind::Call { .. }
        | ExprKind::Perform { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaEncode { .. } => ExprContribution {
            branches: 1,
            ..ExprContribution::default()
        },
        ExprKind::Try { .. } => ExprContribution {
            conditionals: 1,
            ..ExprContribution::default()
        },
        ExprKind::Match { arms, .. } => ExprContribution {
            conditionals: 1 + arms.len(),
            ..ExprContribution::default()
        },
        ExprKind::If {
            else_if_branches, ..
        } => ExprContribution {
            conditionals: 1 + else_if_branches.len(),
            ..ExprContribution::default()
        },
        ExprKind::Binary {
            op: BinaryOp::And | BinaryOp::Or,
            ..
        } => ExprContribution {
            conditionals: 1,
            ..ExprContribution::default()
        },
        _ => ExprContribution::default(),
    }
}

fn count_expr_children(expr: &Expr, vector: &mut AbcVector) {
    match &expr.kind {
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
        ExprKind::TypeApply { callee: child, .. }
        | ExprKind::SchemaEncode { value: child, .. }
        | ExprKind::FieldAccess { base: child, .. }
        | ExprKind::Try { expr: child, .. }
        | ExprKind::Prefix { expr: child, .. } => count_expr(child, vector),
        ExprKind::SchemaDecode {
            input: first,
            base: second,
            ..
        }
        | ExprKind::Binary {
            left: first,
            right: second,
            ..
        } => count_exprs([first.as_ref(), second.as_ref()], vector),
        ExprKind::Perform { args, .. } | ExprKind::List(args) => count_exprs(args, vector),
        ExprKind::Call { callee, args } => {
            count_expr(callee, vector);
            count_exprs(args, vector);
        }
        ExprKind::Handle { body, args, .. } => {
            count_expr(body, vector);
            count_exprs(args, vector);
        }
        ExprKind::Record(fields) => count_exprs(fields.iter().map(|field| &field.expr), vector),
        ExprKind::Dict(entries) => count_exprs(
            entries.iter().flat_map(|entry| [&entry.key, &entry.value]),
            vector,
        ),
        ExprKind::Match { scrutinee, arms } => {
            count_expr(scrutinee, vector);
            count_exprs(arms.iter().map(|arm| &arm.expr), vector);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            count_exprs([condition.as_ref(), then_branch.as_ref()], vector);
            count_exprs(
                else_if_branches
                    .iter()
                    .flat_map(|branch| [&branch.condition, &branch.expr]),
                vector,
            );
            count_expr(else_branch, vector);
        }
        ExprKind::Begin { body, .. } => count_body(body, vector),
    }
}

fn count_exprs<'a>(exprs: impl IntoIterator<Item = &'a Expr>, vector: &mut AbcVector) {
    for expr in exprs {
        count_expr(expr, vector);
    }
}

pub(super) fn compare_abc_subjects(
    left: &AbcSubjectMetric,
    right: &AbcSubjectMetric,
) -> std::cmp::Ordering {
    right
        .magnitude
        .partial_cmp(&left.magnitude)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.span.start.offset.cmp(&right.span.start.offset))
        .then_with(|| left.kind.as_str().cmp(right.kind.as_str()))
}

pub(super) fn compare_module_metrics(
    left: &ModuleMetric,
    right: &ModuleMetric,
) -> std::cmp::Ordering {
    right
        .dependency_pressure
        .cmp(&left.dependency_pressure)
        .then_with(|| right.fan_out.cmp(&left.fan_out))
        .then_with(|| right.fan_in.cmp(&left.fan_in))
        .then_with(|| left.module.cmp(&right.module))
}
