use super::*;

fn diagnostic_summary<'a>(
    diagnostics: impl IntoIterator<Item = &'a Diagnostic>,
) -> Vec<(String, String)> {
    diagnostics
        .into_iter()
        .map(|diagnostic| (diagnostic.id.to_string(), diagnostic.message.clone()))
        .collect()
}

fn call_argument_types(expr: &veln_core::CoreExpr) -> Vec<CoreType> {
    match &expr.kind {
        CoreExprKind::Call { args, .. } => args.iter().map(|arg| arg.ty.clone()).collect(),
        CoreExprKind::Binary { left, right, .. } => {
            let mut types = call_argument_types(left);
            types.extend(call_argument_types(right));
            types
        }
        _ => Vec::new(),
    }
}

mod builtin_exhaustiveness;
mod collections;
mod list_exhaustiveness;
mod pattern_typing;
mod returns_and_results;
mod satisfy_repairs;
mod source_adt_basics;
mod source_adt_visibility;
