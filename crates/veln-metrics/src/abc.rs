use super::*;

pub(super) fn abc_subjects(
    project: &Project,
    selected_paths: &BTreeSet<String>,
) -> Vec<AbcSubjectMetric> {
    let mut subjects = Vec::new();
    for source in &project.files {
        let path = source.path().as_str().to_string();
        if !selected_paths.contains(&path) {
            continue;
        }
        if is_generated_or_doctest_path(&path) {
            continue;
        }
        let parsed = parse(source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        for item in parsed.tree.items {
            let SyntaxItem::Function(function) = item else {
                continue;
            };
            subjects.push(abc_subject(source, &path, &function));
        }
    }
    subjects.sort_by(compare_abc_subjects);
    subjects
}

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

pub(super) fn abc_contract_subject_count(
    project: &Project,
    selected_paths: &BTreeSet<String>,
) -> usize {
    let mut count = 0;
    for source in &project.files {
        let path = source.path().as_str().to_string();
        if !selected_paths.contains(&path) || is_generated_or_doctest_path(&path) {
            continue;
        }
        let parsed = parse(source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        count += parsed
            .tree
            .items
            .into_iter()
            .filter_map(|item| match item {
                SyntaxItem::Function(function) => Some(function),
                _ => None,
            })
            .filter(|function| !function.contracts.is_empty())
            .count();
    }
    count
}

pub(super) fn abc_vector(function: &FunctionDecl) -> AbcVector {
    let mut vector = AbcVector::default();
    for line in &function.body {
        match line {
            BodyLine::Let { expr, .. } => {
                vector.assignments += 1;
                count_expr(expr, &mut vector);
            }
            BodyLine::Expr { expr, .. } => count_expr(expr, &mut vector),
        }
    }
    vector
}

pub(super) fn count_expr(expr: &Expr, vector: &mut AbcVector) {
    match &expr.kind {
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath { segments: _, .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
        ExprKind::TypeApply { callee, .. } => count_expr(callee, vector),
        ExprKind::Call { callee, args } => {
            vector.branches += 1;
            count_expr(callee, vector);
            for arg in args {
                count_expr(arg, vector);
            }
        }
        ExprKind::Perform { args, .. } => {
            vector.branches += 1;
            for arg in args {
                count_expr(arg, vector);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            vector.branches += 1;
            count_expr(body, vector);
            for arg in args {
                count_expr(arg, vector);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            vector.branches += 1;
            count_expr(input, vector);
            count_expr(base, vector);
        }
        ExprKind::SchemaEncode { value, .. } => {
            vector.branches += 1;
            count_expr(value, vector);
        }
        ExprKind::FieldAccess { base, .. } | ExprKind::Try(base) => {
            if matches!(expr.kind, ExprKind::Try(_)) {
                vector.conditionals += 1;
            }
            count_expr(base, vector);
        }
        ExprKind::Record(fields) => {
            for field in fields {
                count_expr(&field.expr, vector);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                count_expr(&entry.key, vector);
                count_expr(&entry.value, vector);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                count_expr(item, vector);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            vector.conditionals += 1 + arms.len();
            count_expr(scrutinee, vector);
            for arm in arms {
                count_expr(&arm.expr, vector);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            vector.conditionals += 1 + else_if_branches.len();
            count_expr(condition, vector);
            count_expr(then_branch, vector);
            for branch in else_if_branches {
                count_expr(&branch.condition, vector);
                count_expr(&branch.expr, vector);
            }
            count_expr(else_branch, vector);
        }
        ExprKind::Prefix { expr, .. } => count_expr(expr, vector),
        ExprKind::Binary { op, left, right } => {
            if matches!(op, BinaryOp::And | BinaryOp::Or) {
                vector.conditionals += 1;
            }
            count_expr(left, vector);
            count_expr(right, vector);
        }
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
