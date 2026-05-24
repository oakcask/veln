use veln_ast::{Expr, ExprKind, Function, FunctionKind, SurfaceModule, lower_surface_ast};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_project::ManifestModule;
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
            diagnostics.extend(validate_manifest_module(
                project,
                source.path().as_str(),
                &lowered,
            ));
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

fn validate_manifest_module(
    project: &Project,
    source_path: &str,
    module: &SurfaceModule,
) -> Vec<Diagnostic> {
    let Some(manifest_module) = project.manifest.as_ref().and_then(|manifest| {
        manifest
            .modules
            .iter()
            .find(|entry| entry.path == source_path)
    }) else {
        return Vec::new();
    };

    let Some(source_module) = &module.module else {
        return vec![manifest_without_source_owner(manifest_module)];
    };

    if manifest_module.name == source_module.name {
        return Vec::new();
    }

    let mut diagnostic = Diagnostic::new(
        "module.metadata_drift",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "manifest module name `{}` does not match source module `{}`",
            manifest_module.name, source_module.name
        ),
        Some(manifest_module.name_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("canonical_owner", JsonValue::string("source")),
            ("derived_owner", JsonValue::string("manifest")),
            (
                "expected_value",
                JsonValue::string(source_module.name.clone()),
            ),
            (
                "observed_value",
                JsonValue::string(manifest_module.name.clone()),
            ),
            ("manifest_path", JsonValue::string("veln.toml")),
            ("source_path", JsonValue::string(source_path)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("canonical_owner")),
        (
            "message",
            JsonValue::string(
                "The source `mod` declaration owns the compiler-visible module name.",
            ),
        ),
        (
            "span",
            JsonValue::object([
                ("file", JsonValue::string(source_module.span.file.as_str())),
                (
                    "start",
                    JsonValue::object([
                        (
                            "line",
                            JsonValue::Number(source_module.span.start.line as i64),
                        ),
                        (
                            "column",
                            JsonValue::Number(source_module.span.start.column as i64),
                        ),
                        (
                            "offset",
                            JsonValue::Number(source_module.span.start.offset as i64),
                        ),
                    ]),
                ),
                (
                    "end",
                    JsonValue::object([
                        (
                            "line",
                            JsonValue::Number(source_module.span.end.line as i64),
                        ),
                        (
                            "column",
                            JsonValue::Number(source_module.span.end.column as i64),
                        ),
                        (
                            "offset",
                            JsonValue::Number(source_module.span.end.offset as i64),
                        ),
                    ]),
                ),
            ]),
        ),
    ]));
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Update the manifest entry or remove the duplicated module name."),
    )]));
    vec![diagnostic]
}

fn manifest_without_source_owner(manifest_module: &ManifestModule) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.metadata_drift",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "manifest module name `{}` has no source `mod` owner",
            manifest_module.name
        ),
        Some(manifest_module.name_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("canonical_owner", JsonValue::string("source")),
            ("derived_owner", JsonValue::string("manifest")),
            (
                "observed_value",
                JsonValue::string(manifest_module.name.clone()),
            ),
            ("manifest_path", JsonValue::string("veln.toml")),
            (
                "source_path",
                JsonValue::string(manifest_module.path.clone()),
            ),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(
            "Add a `mod` declaration to the source file or remove the manifest module name.",
        ),
    )]));
    diagnostic
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
        ExprKind::NamePath(segments) => {
            collect_function_name_reference(segments, function_names, callees);
        }
        ExprKind::Call { callee, args } => {
            if let ExprKind::NamePath(segments) = &callee.kind {
                collect_function_name_reference(segments, function_names, callees);
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
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_function_callees(&entry.key, function_names, callees);
                collect_function_callees(&entry.value, function_names, callees);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_function_callees(item, function_names, callees);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_function_callees(scrutinee, function_names, callees);
            for arm in arms {
                collect_function_callees(&arm.expr, function_names, callees);
            }
        }
        ExprKind::Prefix { expr, .. } => collect_function_callees(expr, function_names, callees),
        ExprKind::Binary { left, right, .. } => {
            collect_function_callees(left, function_names, callees);
            collect_function_callees(right, function_names, callees);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn collect_function_name_reference(
    segments: &[String],
    function_names: &[String],
    callees: &mut Vec<String>,
) {
    let Some(name) = segments.last() else {
        return;
    };
    if function_names
        .iter()
        .any(|function_name| function_name == name)
        && !callees.iter().any(|callee| callee == name)
    {
        callees.push(name.clone());
    }
}

#[cfg(test)]
mod tests {
    use veln_ast::{FunctionKind, SurfaceModule, lower_surface_ast};
    use veln_project::{ManifestModule, Project, ProjectManifest};
    use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan};
    use veln_syntax::parse;

    use super::{load_surface_module, reachable_entry_module};

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
    fn test_entry_can_reach_function_value_reference() {
        let module = lower(concat!(
            "test foo() -> () effects []\n",
            "  list_map([1], stringify)\n",
            "  ()\n",
            "end\n",
            "fn stringify(value: Int) -> String effects []\n",
            "  \"ok\"\n",
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
                (FunctionKind::Function, Some("stringify")),
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

    #[test]
    fn manifest_module_name_cannot_override_source_mod() {
        let source = SourceFile::new(
            "src/main.veln",
            "mod app.main\nfn main() -> () effects []\n  ()\nend\n",
        );
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                modules: vec![ManifestModule {
                    path: "src/main.veln".to_string(),
                    name: "manifest.main".to_string(),
                    path_span: span("veln.toml", 2, 2, 11),
                    name_span: span("veln.toml", 2, 20, 33),
                }],
            }),
        };

        let (module, diagnostics) = load_surface_module(&project);

        assert_eq!(module.module.as_ref().unwrap().name, "app.main");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "module.metadata_drift");
        assert_eq!(
            diagnostics[0].message,
            "manifest module name `manifest.main` does not match source module `app.main`"
        );
    }

    fn span(file: &str, line: usize, start_column: usize, end_column: usize) -> SourceSpan {
        SourceSpan {
            file: SourcePath::new(file),
            start: LineCol {
                line,
                column: start_column,
                offset: 0,
            },
            end: LineCol {
                line,
                column: end_column,
                offset: 0,
            },
        }
    }
}
