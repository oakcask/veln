use veln_ast::{Expr, ExprKind, Function, FunctionKind, SurfaceModule, lower_surface_ast};
use veln_diagnostics::Diagnostic;
use veln_project::Project;
use veln_syntax::parse;

use crate::diagnostics::parse_diagnostic_to_envelope;

pub(crate) fn load_surface_module(project: &Project) -> (SurfaceModule, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut module = None;
    let mut uses = Vec::new();
    let mut functions = Vec::new();

    for source in &project.files {
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if parsed.diagnostics.is_empty() {
            let lowered = lower_surface_ast(&parsed.tree);
            module = module.or(lowered.module);
            uses.extend(lowered.uses);
            functions.extend(lowered.functions);
        }
    }

    (
        SurfaceModule {
            module,
            uses,
            functions,
        },
        diagnostics,
    )
}

pub(crate) fn reachable_entry_module(
    module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
) -> SurfaceModule {
    let function_names = module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| function.name.clone())
        .collect::<Vec<_>>();
    let mut reachable = Vec::<ReachableFunction>::new();
    let mut stack = vec![ReachableFunction {
        kind: entry_kind,
        name: entry.to_string(),
    }];

    while let Some(key) = stack.pop() {
        if reachable.iter().any(|known| known == &key) {
            continue;
        }
        reachable.push(key.clone());
        for function in module.functions.iter().filter(|function| {
            function.name.as_deref() == Some(key.name.as_str()) && function.kind == key.kind
        }) {
            for callee in direct_function_callees(function, &function_names) {
                let callee = ReachableFunction {
                    kind: FunctionKind::Function,
                    name: callee,
                };
                if !reachable.iter().any(|known| known == &callee) {
                    stack.push(callee);
                }
            }
        }
    }

    SurfaceModule {
        module: module.module.clone(),
        uses: module.uses.clone(),
        functions: module
            .functions
            .iter()
            .filter(|function| {
                function.name.as_ref().is_some_and(|name| {
                    reachable
                        .iter()
                        .any(|known| known.name == *name && known.kind == function.kind)
                })
            })
            .cloned()
            .collect(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReachableFunction {
    kind: FunctionKind,
    name: String,
}

fn direct_function_callees(function: &Function, function_names: &[String]) -> Vec<String> {
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

fn collect_function_callees(expr: &Expr, function_names: &[String], callees: &mut Vec<String>) {
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
        ExprKind::FieldAccess { base, .. } => {
            collect_function_callees(base, function_names, callees);
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

#[cfg(test)]
mod tests {
    use veln_ast::{FunctionKind, SurfaceModule, lower_surface_ast};
    use veln_source::SourceFile;
    use veln_syntax::parse;

    use super::reachable_entry_module;

    fn lower(text: &str) -> SurfaceModule {
        let source = SourceFile::new("main_test.veln", text);
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}",
            parsed.diagnostics
        );
        lower_surface_ast(&parsed.tree)
    }

    #[test]
    fn test_entry_can_reach_function_callee() {
        let module = lower(concat!(
            "test foo() -> () effects []\n",
            "  helper()\n",
            "end\n",
            "fn helper() -> () effects []\n",
            "  ()\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Test, Some("foo")),
                (FunctionKind::Function, Some("helper")),
            ]
        );
    }

    #[test]
    fn run_entry_does_not_include_tests() {
        let module = lower(concat!(
            "test helper() -> () effects []\n",
            "  ()\n",
            "end\n",
            "fn foo() -> () effects []\n",
            "  ()\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("foo"))]);
    }
}
