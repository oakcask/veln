use crate::{BodyLineKind, Expr, ExprKind, Function, Pattern, PatternKind, SurfaceModule};

pub(super) fn collect_module_node_ids(module: &SurfaceModule) -> Vec<u32> {
    let mut ids = Vec::new();
    for function in &module.functions {
        collect_function_node_ids(function, &mut ids);
    }
    ids
}

fn collect_function_node_ids(function: &Function, ids: &mut Vec<u32>) {
    ids.push(function.node_id.as_u32());
    ids.extend(function.params.iter().map(|param| param.node_id.as_u32()));
    ids.extend(
        function
            .return_binding
            .iter()
            .map(|binding| binding.node_id.as_u32()),
    );
    ids.extend(
        function
            .contracts
            .iter()
            .map(|contract| contract.node_id.as_u32()),
    );
    for line in &function.body {
        ids.push(line.node_id.as_u32());
        match &line.kind {
            BodyLineKind::Let { expr, .. } | BodyLineKind::Expr { expr } => {
                collect_expr_node_ids(expr, ids);
            }
        }
    }
}

fn collect_expr_node_ids(expr: &Expr, ids: &mut Vec<u32>) {
    ids.push(expr.node_id.as_u32());
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            collect_expr_node_ids(callee, ids);
            for arg in args {
                collect_expr_node_ids(arg, ids);
            }
        }
        ExprKind::TypeApply { callee, .. } => collect_expr_node_ids(callee, ids),
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_expr_node_ids(arg, ids);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_expr_node_ids(body, ids);
            for arg in args {
                collect_expr_node_ids(arg, ids);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_expr_node_ids(input, ids);
            collect_expr_node_ids(base, ids);
        }
        ExprKind::SchemaEncode { value, .. } => collect_expr_node_ids(value, ids),
        ExprKind::FieldAccess { base, .. } => collect_expr_node_ids(base, ids),
        ExprKind::Try(expr) => collect_expr_node_ids(expr, ids),
        ExprKind::Record(fields) => {
            for field in fields {
                ids.push(field.node_id.as_u32());
                collect_expr_node_ids(&field.expr, ids);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                ids.push(entry.node_id.as_u32());
                collect_expr_node_ids(&entry.key, ids);
                collect_expr_node_ids(&entry.value, ids);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_expr_node_ids(item, ids);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_node_ids(scrutinee, ids);
            for arm in arms {
                ids.push(arm.node_id.as_u32());
                collect_pattern_node_ids(&arm.pattern, ids);
                collect_expr_node_ids(&arm.expr, ids);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_expr_node_ids(condition, ids);
            collect_expr_node_ids(then_branch, ids);
            for branch in else_if_branches {
                ids.push(branch.node_id.as_u32());
                collect_expr_node_ids(&branch.condition, ids);
                collect_expr_node_ids(&branch.expr, ids);
            }
            collect_expr_node_ids(else_branch, ids);
        }
        ExprKind::Prefix { expr, .. } => collect_expr_node_ids(expr, ids),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_node_ids(left, ids);
            collect_expr_node_ids(right, ids);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn collect_pattern_node_ids(pattern: &Pattern, ids: &mut Vec<u32>) {
    ids.push(pattern.node_id.as_u32());
    match &pattern.kind {
        PatternKind::Record(fields) => {
            for field in fields {
                ids.push(field.node_id.as_u32());
                collect_pattern_node_ids(&field.pattern, ids);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_pattern_node_ids(arg, ids);
            }
        }
        PatternKind::Wildcard
        | PatternKind::Binding(_)
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}
