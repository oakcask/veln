use veln_ast::{
    Expr, ExprKind, Function, FunctionKind, Pattern, PatternKind, SurfaceModule, UseDecl,
    lower_surface_ast,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_project::ManifestModule;
use veln_project::Project;
use veln_source::SourceFile;
use veln_syntax::{TokenKind, lex, parse};

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
    let function_targets = module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            Some(FunctionTarget {
                name: function.name.clone()?,
                module_name: function.module_name.clone(),
                arity: function.params.len(),
            })
        })
        .collect::<Vec<_>>();
    let mut reachable = Vec::<ReachableFunction>::new();
    let mut stack = vec![ReachableFunction {
        kind: entry_kind,
        name: entry.to_string(),
        module_name: None,
    }];

    while let Some(key) = stack.pop() {
        if reachable.iter().any(|known| known == &key) {
            continue;
        }
        reachable.push(key.clone());
        for function in module.functions.iter().filter(|function| {
            function.name.as_deref() == Some(key.name.as_str())
                && function.kind == key.kind
                && key
                    .module_name
                    .as_ref()
                    .is_none_or(|module_name| function.module_name.as_ref() == Some(module_name))
        }) {
            for callee in direct_function_callees(function, &module.uses, &function_targets) {
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
                    reachable.iter().any(|known| {
                        known.name == *name
                            && known.kind == function.kind
                            && known.module_name.as_ref().is_none_or(|module_name| {
                                function.module_name.as_ref() == Some(module_name)
                            })
                    })
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
    module_name: Option<String>,
}

#[derive(Clone, Debug)]
struct FunctionTarget {
    name: String,
    module_name: Option<String>,
    arity: usize,
}

#[derive(Clone, Debug)]
struct LocalBinding {
    name: String,
    function_arity: Option<usize>,
}

fn direct_function_callees(
    function: &Function,
    uses: &[UseDecl],
    function_targets: &[FunctionTarget],
) -> Vec<ReachableFunction> {
    let mut callees = Vec::new();
    let current_module = function.module_name.as_deref();
    let mut local_bindings = function
        .params
        .iter()
        .map(|param| LocalBinding {
            name: param.name.clone(),
            function_arity: param.ty.as_deref().and_then(function_type_arity),
        })
        .collect::<Vec<_>>();
    for contract in &function.contracts {
        collect_contract_callees(
            &contract.text,
            current_module,
            uses,
            function_targets,
            &mut callees,
        );
    }
    for line in &function.body {
        match &line.kind {
            veln_ast::BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                collect_function_callees(
                    expr,
                    current_module,
                    uses,
                    function_targets,
                    &local_bindings,
                    &mut callees,
                );
                collect_pattern_bindings(
                    pattern,
                    annotation.as_deref().and_then(function_type_arity),
                    &mut local_bindings,
                );
            }
            veln_ast::BodyLineKind::Expr { expr } => {
                collect_function_callees(
                    expr,
                    current_module,
                    uses,
                    function_targets,
                    &local_bindings,
                    &mut callees,
                );
            }
        }
    }
    callees
}

fn collect_contract_callees(
    predicate: &str,
    current_module: Option<&str>,
    uses: &[UseDecl],
    function_targets: &[FunctionTarget],
    callees: &mut Vec<ReachableFunction>,
) {
    let source = SourceFile::new("<contract>", predicate);
    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .collect::<Vec<_>>();
    let mut index = 0usize;
    while index < tokens.len() {
        let name = &tokens[index];
        if name.kind != TokenKind::Ident {
            index += 1;
            continue;
        }
        let mut callee = name.text.clone();
        let mut next_index = index + 1;
        while next_index + 1 < tokens.len()
            && tokens[next_index].kind == TokenKind::DoubleColon
            && tokens[next_index + 1].kind == TokenKind::Ident
        {
            callee = tokens[next_index + 1].text.clone();
            next_index += 2;
        }
        let Some(next) = tokens.get(next_index) else {
            break;
        };
        if next.kind != TokenKind::LParen {
            index += 1;
            continue;
        }
        let segments = if callee == name.text {
            vec![callee]
        } else {
            vec![name.text.clone(), callee]
        };
        for callee in resolve_function_reference(&segments, current_module, uses, function_targets)
        {
            push_reachable(callees, callee);
        }
        index = next_index + 1;
    }
    collect_contract_function_value_references(
        &tokens,
        current_module,
        uses,
        function_targets,
        callees,
    );
}

fn collect_contract_function_value_references(
    tokens: &[veln_syntax::Token],
    current_module: Option<&str>,
    uses: &[UseDecl],
    function_targets: &[FunctionTarget],
    callees: &mut Vec<ReachableFunction>,
) {
    let mut index = 0usize;
    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Ident {
            index += 1;
            continue;
        }
        if index > 0
            && matches!(
                tokens[index - 1].kind,
                TokenKind::Dot | TokenKind::DoubleColon
            )
        {
            index += 1;
            continue;
        }
        if tokens
            .get(index + 1)
            .is_some_and(|token| matches!(token.kind, TokenKind::Dot | TokenKind::LParen))
        {
            index += 1;
            continue;
        }
        let segments = if tokens
            .get(index + 1)
            .is_some_and(|token| token.kind == TokenKind::DoubleColon)
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.kind == TokenKind::Ident)
        {
            let segments = vec![tokens[index].text.clone(), tokens[index + 2].text.clone()];
            index += 3;
            segments
        } else {
            let segments = vec![tokens[index].text.clone()];
            index += 1;
            segments
        };
        for callee in resolve_function_reference(&segments, current_module, uses, function_targets)
        {
            push_reachable(callees, callee);
        }
    }
}

fn collect_function_callees(
    expr: &Expr,
    current_module: Option<&str>,
    uses: &[UseDecl],
    function_targets: &[FunctionTarget],
    local_bindings: &[LocalBinding],
    callees: &mut Vec<ReachableFunction>,
) {
    match &expr.kind {
        ExprKind::NamePath(segments) => {
            collect_function_name_reference(
                segments,
                current_module,
                uses,
                function_targets,
                local_bindings,
                callees,
            );
        }
        ExprKind::TypeApply { callee, .. } => {
            collect_function_callees(
                callee,
                current_module,
                uses,
                function_targets,
                local_bindings,
                callees,
            );
        }
        ExprKind::Call { callee, args } => {
            if let Some(segments) = callee_name_path(callee) {
                collect_function_name_reference(
                    segments,
                    current_module,
                    uses,
                    function_targets,
                    local_bindings,
                    callees,
                );
            }
            collect_function_callees(
                callee,
                current_module,
                uses,
                function_targets,
                local_bindings,
                callees,
            );
            for arg in args {
                collect_function_callees(
                    arg,
                    current_module,
                    uses,
                    function_targets,
                    local_bindings,
                    callees,
                );
            }
        }
        ExprKind::FieldAccess { base, .. } => {
            collect_function_callees(
                base,
                current_module,
                uses,
                function_targets,
                local_bindings,
                callees,
            );
        }
        ExprKind::Try(inner) => collect_function_callees(
            inner,
            current_module,
            uses,
            function_targets,
            local_bindings,
            callees,
        ),
        ExprKind::Record(fields) => {
            for field in fields {
                collect_function_callees(
                    &field.expr,
                    current_module,
                    uses,
                    function_targets,
                    local_bindings,
                    callees,
                );
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_function_callees(
                    &entry.key,
                    current_module,
                    uses,
                    function_targets,
                    local_bindings,
                    callees,
                );
                collect_function_callees(
                    &entry.value,
                    current_module,
                    uses,
                    function_targets,
                    local_bindings,
                    callees,
                );
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_function_callees(
                    item,
                    current_module,
                    uses,
                    function_targets,
                    local_bindings,
                    callees,
                );
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_function_callees(
                scrutinee,
                current_module,
                uses,
                function_targets,
                local_bindings,
                callees,
            );
            for arm in arms {
                let mut arm_bindings = local_bindings.to_vec();
                collect_pattern_bindings(&arm.pattern, None, &mut arm_bindings);
                collect_function_callees(
                    &arm.expr,
                    current_module,
                    uses,
                    function_targets,
                    &arm_bindings,
                    callees,
                );
            }
        }
        ExprKind::Prefix { expr, .. } => {
            collect_function_callees(
                expr,
                current_module,
                uses,
                function_targets,
                local_bindings,
                callees,
            );
        }
        ExprKind::Binary { left, right, .. } => {
            collect_function_callees(
                left,
                current_module,
                uses,
                function_targets,
                local_bindings,
                callees,
            );
            collect_function_callees(
                right,
                current_module,
                uses,
                function_targets,
                local_bindings,
                callees,
            );
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn collect_pattern_bindings(
    pattern: &Pattern,
    function_arity: Option<usize>,
    bindings: &mut Vec<LocalBinding>,
) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(LocalBinding {
            name: name.clone(),
            function_arity,
        }),
        PatternKind::Record(fields) => {
            for field in fields {
                collect_pattern_bindings(&field.pattern, None, bindings);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_pattern_bindings(arg, None, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

fn callee_name_path(callee: &Expr) -> Option<&Vec<String>> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments),
        ExprKind::TypeApply { callee, .. } => callee_name_path(callee),
        _ => None,
    }
}

fn collect_opaque_function_value_callees(
    arity: usize,
    current_module: Option<&str>,
    uses: &[UseDecl],
    function_targets: &[FunctionTarget],
    callees: &mut Vec<ReachableFunction>,
) {
    for target in function_targets.iter().filter(|target| {
        target.arity == arity
            && visible_from_current_module(target.module_name.as_deref(), current_module, uses)
    }) {
        push_reachable(
            callees,
            ReachableFunction {
                kind: FunctionKind::Function,
                name: target.name.clone(),
                module_name: target.module_name.clone(),
            },
        );
    }
}

fn visible_from_current_module(
    target_module: Option<&str>,
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> bool {
    if current_module.is_none() || target_module == current_module {
        return true;
    }
    target_module
        .is_some_and(|module_name| uses.iter().any(|use_decl| use_decl.name == module_name))
}

fn function_type_arity(annotation: &str) -> Option<usize> {
    let params = annotation.trim().strip_prefix("fn")?.trim_start();
    let params = params.strip_prefix('(')?;
    let mut depth = 0usize;
    let mut split_at = None;
    for (index, ch) in params.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' if depth == 0 => {
                split_at = Some(index);
                break;
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let params = &params[..split_at?].trim();
    if params.is_empty() {
        return Some(0);
    }
    Some(split_top_level_commas(params).len())
}

fn split_top_level_commas(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

fn collect_function_name_reference(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
    function_targets: &[FunctionTarget],
    local_bindings: &[LocalBinding],
    callees: &mut Vec<ReachableFunction>,
) {
    if let [name] = segments {
        if let Some(binding) = local_bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
        {
            if let Some(arity) = binding.function_arity {
                collect_opaque_function_value_callees(
                    arity,
                    current_module,
                    uses,
                    function_targets,
                    callees,
                );
            }
            return;
        }
    }
    for callee in resolve_function_reference(segments, current_module, uses, function_targets) {
        push_reachable(callees, callee);
    }
}

fn resolve_function_reference(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
    function_targets: &[FunctionTarget],
) -> Vec<ReachableFunction> {
    match segments {
        [name] => function_targets
            .iter()
            .filter(|target| {
                target.name == *name
                    && current_module.is_none_or(|module_name| {
                        target.module_name.as_deref() == Some(module_name)
                    })
            })
            .map(|target| ReachableFunction {
                kind: FunctionKind::Function,
                name: target.name.clone(),
                module_name: target.module_name.clone(),
            })
            .collect(),
        [alias, name] => {
            let Some(module_name) = uses
                .iter()
                .find(|use_decl| use_decl.alias == *alias)
                .map(|use_decl| use_decl.name.as_str())
            else {
                return Vec::new();
            };
            function_targets
                .iter()
                .filter(|target| {
                    target.name == *name && target.module_name.as_deref() == Some(module_name)
                })
                .map(|target| ReachableFunction {
                    kind: FunctionKind::Function,
                    name: target.name.clone(),
                    module_name: target.module_name.clone(),
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn push_reachable(callees: &mut Vec<ReachableFunction>, callee: ReachableFunction) {
    if !callees.iter().any(|known| known == &callee) {
        callees.push(callee);
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
    fn test_entry_conservatively_reaches_opaque_function_value_call_targets() {
        let module = lower(concat!(
            "test foo() -> Bool effects []\n",
            "  invoke(ready)\n",
            "end\n",
            "fn invoke(job: fn() -> Bool) -> Bool effects []\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool effects []\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool effects []\n",
            "  _\n",
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
                (FunctionKind::Function, Some("invoke")),
                (FunctionKind::Function, Some("ready")),
                (FunctionKind::Function, Some("risky")),
            ]
        );
    }

    #[test]
    fn test_entry_reaches_opaque_function_value_call_targets_with_spaced_type() {
        let module = lower(concat!(
            "test foo() -> Bool effects []\n",
            "  invoke(ready)\n",
            "end\n",
            "fn invoke(job: fn () -> Bool) -> Bool effects []\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool effects []\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool effects []\n",
            "  _\n",
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
                (FunctionKind::Function, Some("invoke")),
                (FunctionKind::Function, Some("ready")),
                (FunctionKind::Function, Some("risky")),
            ]
        );
    }

    #[test]
    fn test_entry_conservatively_reaches_opaque_local_function_value_call_targets() {
        let module = lower(concat!(
            "test foo() -> Bool effects []\n",
            "  invoke()\n",
            "end\n",
            "fn invoke() -> Bool effects []\n",
            "  let job: fn() -> Bool = ready\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool effects []\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool effects []\n",
            "  _\n",
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
                (FunctionKind::Function, Some("invoke")),
                (FunctionKind::Function, Some("ready")),
                (FunctionKind::Function, Some("risky")),
            ]
        );
    }

    #[test]
    fn test_entry_can_reach_qualified_function_value_reference() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "main_test.veln",
                    concat!(
                        "mod app.main\n",
                        "use app.text\n",
                        "test foo() -> () effects []\n",
                        "  list_map([1], text::stringify)\n",
                        "  ()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "text.veln",
                    concat!(
                        "mod app.text\n",
                        "fn stringify(value: Int) -> String effects []\n",
                        "  \"ok\"\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app.main"), FunctionKind::Test, Some("foo")),
                (Some("app.text"), FunctionKind::Function, Some("stringify")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_contract_helper() {
        let module = lower(concat!(
            "fn positive(value: Int) -> Bool effects []\n",
            "  value > 0\n",
            "end\n",
            "pub fn main(value: Int) -> output: Int effects []\n",
            "  ensure positive(output)\n",
            "  value\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Function, Some("positive")),
                (FunctionKind::Function, Some("main")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_contract_function_value() {
        let module = lower(concat!(
            "fn accepts(job: fn() -> Bool) -> Bool effects []\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool effects []\n",
            "  true\n",
            "end\n",
            "pub fn main() -> () effects []\n",
            "  require accepts(ready)\n",
            "  ()\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (FunctionKind::Function, Some("accepts")),
                (FunctionKind::Function, Some("ready")),
                (FunctionKind::Function, Some("main")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_qualified_contract_helper() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "main.veln",
                    concat!(
                        "mod app.main\n",
                        "use app.rules\n",
                        "pub fn main(value: Int) -> output: Int effects []\n",
                        "  ensure rules::positive(output)\n",
                        "  value\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "rules.veln",
                    concat!(
                        "mod app.rules\n",
                        "fn positive(value: Int) -> Bool effects []\n",
                        "  value > 0\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app.main"), FunctionKind::Function, Some("main")),
                (Some("app.rules"), FunctionKind::Function, Some("positive")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_imported_qualified_call() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "main.veln",
                    concat!(
                        "mod app.main\n",
                        "use app.util\n",
                        "pub fn main() -> Int effects []\n",
                        "  util::value()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "util.veln",
                    concat!(
                        "mod app.util\n",
                        "fn value() -> Int effects []\n",
                        "  1\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app.main"), FunctionKind::Function, Some("main")),
                (Some("app.util"), FunctionKind::Function, Some("value")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_qualified_contract_function_value() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "main.veln",
                    concat!(
                        "mod app.main\n",
                        "use app.rules\n",
                        "fn accepts(job: fn() -> Bool) -> Bool effects []\n",
                        "  job()\n",
                        "end\n",
                        "pub fn main() -> () effects []\n",
                        "  require accepts(rules::ready)\n",
                        "  ()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "rules.veln",
                    concat!(
                        "mod app.rules\n",
                        "fn ready() -> Bool effects []\n",
                        "  true\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app.main"), FunctionKind::Function, Some("accepts")),
                (Some("app.main"), FunctionKind::Function, Some("main")),
                (Some("app.rules"), FunctionKind::Function, Some("ready")),
            ]
        );
    }

    #[test]
    fn imported_reachability_keeps_module_specific_function_names() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "main.veln",
                    concat!(
                        "mod app.main\n",
                        "use app.util\n",
                        "fn value() -> Int effects []\n",
                        "  _\n",
                        "end\n",
                        "pub fn main() -> Int effects []\n",
                        "  util::value()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "util.veln",
                    concat!(
                        "mod app.util\n",
                        "fn value() -> Int effects []\n",
                        "  1\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app.main"), FunctionKind::Function, Some("main")),
                (Some("app.util"), FunctionKind::Function, Some("value")),
            ]
        );
    }

    #[test]
    fn bare_reachability_keeps_current_module_function_names() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "main.veln",
                    concat!(
                        "mod app.main\n",
                        "fn value() -> Int effects []\n",
                        "  1\n",
                        "end\n",
                        "pub fn main() -> Int effects []\n",
                        "  value()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "other.veln",
                    concat!(
                        "mod app.other\n",
                        "fn value() -> Int effects []\n",
                        "  _\n",
                        "end\n",
                    ),
                ),
            ],
            manifest: None,
        };
        let (module, diagnostics) = load_surface_module(&project);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| {
                (
                    function.module_name.as_deref(),
                    function.kind,
                    function.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            functions,
            vec![
                (Some("app.main"), FunctionKind::Function, Some("value")),
                (Some("app.main"), FunctionKind::Function, Some("main")),
            ]
        );
    }

    #[test]
    fn local_binding_shadowing_function_name_does_not_reach_function() {
        let module = lower(concat!(
            "fn helper() -> Int effects []\n",
            "  _\n",
            "end\n",
            "pub fn main() -> Int effects []\n",
            "  let helper = 1\n",
            "  helper\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
    }

    #[test]
    fn match_binding_shadowing_function_name_does_not_reach_function() {
        let module = lower(concat!(
            "fn helper() -> Int effects []\n",
            "  _\n",
            "end\n",
            "pub fn main(value: Option(Int)) -> Int effects []\n",
            "  match value\n",
            "    Some(helper) => helper\n",
            "    None => 0\n",
            "  end\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
    }

    #[test]
    fn run_entry_does_not_reach_qualified_call_without_import_alias() {
        let module = lower(concat!(
            "mod app.main\n",
            "pub fn main() -> Int effects []\n",
            "  util::value()\n",
            "end\n",
            "fn value() -> Int effects []\n",
            "  _\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
    }

    #[test]
    fn contract_reachability_ignores_function_names_inside_strings() {
        let module = lower(concat!(
            "fn positive(value: Int) -> Bool effects []\n",
            "  value > 0\n",
            "end\n",
            "pub fn main() -> output: String effects []\n",
            "  ensure \"positive(\" == output\n",
            "  \"positive(\"\n",
            "end\n",
        ));

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        let functions = reachable
            .functions
            .iter()
            .map(|function| (function.kind, function.name.as_deref()))
            .collect::<Vec<_>>();

        assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
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

    #[test]
    fn matching_manifest_module_name_does_not_report_drift() {
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
                    name: "app.main".to_string(),
                    path_span: span("veln.toml", 2, 2, 11),
                    name_span: span("veln.toml", 2, 20, 28),
                }],
            }),
        };

        let (module, diagnostics) = load_surface_module(&project);

        assert_eq!(module.module.as_ref().unwrap().name, "app.main");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "module.metadata_drift"),
            "{diagnostics:#?}"
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
