use veln_ast::{Expr, ExprKind, Function, SurfaceModule, lower_surface_ast};
use veln_diagnostics::Diagnostic;
use veln_project::Project;
use veln_syntax::parse;

use crate::diagnostics::parse_diagnostic_to_envelope;

pub(crate) fn load_surface_module(project: &Project) -> (SurfaceModule, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut functions = Vec::new();

    for source in &project.files {
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if parsed.diagnostics.is_empty() {
            functions.extend(lower_surface_ast(&parsed.tree).functions);
        }
    }

    (SurfaceModule { functions }, diagnostics)
}

pub(crate) fn reachable_entry_module(module: &SurfaceModule, entry: &str) -> SurfaceModule {
    let function_names = module
        .functions
        .iter()
        .filter_map(|function| function.name.as_deref())
        .collect::<Vec<_>>();
    let mut reachable = Vec::<String>::new();
    let mut stack = vec![entry.to_string()];

    while let Some(name) = stack.pop() {
        if reachable.iter().any(|known| known == &name) {
            continue;
        }
        reachable.push(name.clone());
        for function in module
            .functions
            .iter()
            .filter(|function| function.name.as_deref() == Some(name.as_str()))
        {
            for callee in direct_function_callees(function, &function_names) {
                if !reachable.iter().any(|known| known == &callee) {
                    stack.push(callee);
                }
            }
        }
    }

    SurfaceModule {
        functions: module
            .functions
            .iter()
            .filter(|function| {
                function
                    .name
                    .as_ref()
                    .is_some_and(|name| reachable.iter().any(|known| known == name))
            })
            .cloned()
            .collect(),
    }
}

fn direct_function_callees(function: &Function, function_names: &[&str]) -> Vec<String> {
    let mut callees = Vec::new();
    for line in &function.body {
        match &line.kind {
            veln_ast::BodyLineKind::Let { expr, .. } | veln_ast::BodyLineKind::Expr { expr } => {
                collect_function_callees(expr, function_names, &mut callees);
            }
        }
    }
    callees
}

fn collect_function_callees(expr: &Expr, function_names: &[&str], callees: &mut Vec<String>) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let ExprKind::NamePath(segments) = &callee.kind {
                if let Some(name) = segments.last() {
                    if function_names
                        .iter()
                        .any(|function_name| function_name == name)
                        && !callees.iter().any(|callee| callee == name)
                    {
                        callees.push(name.clone());
                    }
                }
            }
            collect_function_callees(callee, function_names, callees);
            for arg in args {
                collect_function_callees(arg, function_names, callees);
            }
        }
        ExprKind::Try(inner) => collect_function_callees(inner, function_names, callees),
        ExprKind::Record(fields) => {
            for field in fields {
                collect_function_callees(&field.expr, function_names, callees);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_function_callees(item, function_names, callees);
            }
        }
        ExprKind::Prefix { expr, .. } => collect_function_callees(expr, function_names, callees),
        ExprKind::Binary { left, right, .. } => {
            collect_function_callees(left, function_names, callees);
            collect_function_callees(right, function_names, callees);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::Unit => {}
    }
}
