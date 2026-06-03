use veln_ast::{
    Expr, ExprKind, Function, FunctionKind, Pattern, PatternKind, PublicAliasKind, SurfaceModule,
    UseDecl, lower_surface_ast, lower_surface_ast_with_module_identity,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_project::Project;
use veln_source::{SourceFile, TextRange};
use veln_syntax::{TokenKind, lex, parse};

use crate::diagnostics::parse_diagnostic_to_envelope;

pub(crate) fn load_surface_module(project: &Project) -> (SurfaceModule, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut module = None;
    let mut uses = Vec::new();
    let mut aliases = Vec::new();
    let mut types = Vec::new();
    let mut functions = Vec::new();
    let mut derived_modules = Vec::<(String, SourceFile)>::new();

    for source in &project.files {
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if parsed.diagnostics.is_empty() {
            if let Some(module) = &parsed.tree.module {
                diagnostics.push(source_mod_decl_diagnostic(module));
            }
            for use_decl in &parsed.tree.uses {
                if use_decl.name.contains('.') {
                    diagnostics.push(dotted_use_decl_diagnostic(use_decl));
                }
            }

            let derived_module = derive_source_module_path(source);
            match &derived_module {
                Ok(module_name) => {
                    if module_name == "prelude" {
                        diagnostics.push(reserved_source_module_diagnostic(source, module_name));
                    }
                    if !is_doctest_source(source)
                        && let Some((_, first_source)) = derived_modules
                            .iter()
                            .find(|(known_module, _)| known_module == module_name)
                    {
                        diagnostics.push(duplicate_derived_module_diagnostic(
                            module_name,
                            source,
                            first_source,
                        ));
                    } else if !is_doctest_source(source) {
                        derived_modules.push((module_name.clone(), source.clone()));
                    }
                }
                Err(diagnostic) => diagnostics.push((**diagnostic).clone()),
            }
            let lowered = match derived_module {
                Ok(module_name) => lower_surface_ast_with_module_identity(
                    &parsed.tree,
                    module_name,
                    source.span(TextRange::new(0, 0)),
                ),
                Err(_) => lower_surface_ast(&parsed.tree),
            };
            diagnostics.extend(validate_manifest_module(
                project,
                source.path().as_str(),
                &lowered,
            ));
            module = module.or(lowered.module);
            uses.extend(lowered.uses);
            aliases.extend(lowered.aliases);
            types.extend(lowered.types);
            functions.extend(lowered.functions);
        }
    }
    diagnostics.extend(unresolved_local_import_diagnostics(&uses, &derived_modules));

    (
        SurfaceModule {
            module,
            uses,
            aliases,
            types,
            functions,
        },
        diagnostics,
    )
}

pub(crate) fn derive_source_module_path(source: &SourceFile) -> Result<String, Box<Diagnostic>> {
    let path = source.path().as_str();
    if let Some(module_name) = derive_doctest_module_path(path) {
        return Ok(module_name);
    }
    let Some(without_extension) = path.strip_suffix(".veln") else {
        return Err(Box::new(invalid_source_module_path_diagnostic(
            source,
            path,
            "source module files must use the `.veln` extension",
        )));
    };
    let mut segments = Vec::new();
    for segment in without_extension.split('/') {
        if is_module_identifier(segment) {
            segments.push(segment);
        } else {
            return Err(Box::new(invalid_source_module_path_diagnostic(
                source,
                segment,
                "source path segment cannot be used as a module identifier",
            )));
        }
    }
    Ok(segments.join("::"))
}

fn derive_doctest_module_path(path: &str) -> Option<String> {
    let (source_path, _) = path.split_once("#doctest-")?;
    let source_stem = source_path.strip_suffix(".veln")?;
    let mut segments = Vec::new();
    for segment in source_stem.split('/') {
        if is_module_identifier(segment) {
            segments.push(segment.to_string());
        } else {
            return None;
        }
    }
    Some(segments.join("::"))
}

fn is_doctest_source(source: &SourceFile) -> bool {
    source.path().as_str().contains("#doctest-")
}

fn source_mod_decl_diagnostic(module: &veln_syntax::ModuleDecl) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.source_mod",
        Severity::Error,
        DiagnosticKind::Module,
        "source `mod` declarations are not supported",
        Some(module.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("module_path", JsonValue::string(module.name.clone())),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(
            "Move or rename the source file so its package-relative path derives the intended module path.",
        ),
    )]));
    diagnostic
}

fn dotted_use_decl_diagnostic(use_decl: &veln_syntax::UseDecl) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.invalid_import_path",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "module import `{}` uses `.`; source module paths use `::`",
            use_decl.name
        ),
        Some(use_decl.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("import_path")),
            ("module_path", JsonValue::string(use_decl.name.clone())),
            ("expected_delimiter", JsonValue::string("::")),
            ("observed_delimiter", JsonValue::string(".")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Rewrite the import with `::` between module path segments."),
    )]));
    diagnostic
}

fn is_module_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn invalid_source_module_path_diagnostic(
    source: &SourceFile,
    segment: &str,
    message: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "module.invalid_source_path",
        Severity::Error,
        DiagnosticKind::Module,
        format!("{message}: `{segment}`"),
        Some(source.span(veln_source::TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("source_path", JsonValue::string(source.path().as_str())),
            ("segment", JsonValue::string(segment)),
        ]),
    )
}

fn duplicate_derived_module_diagnostic(
    module_name: &str,
    source: &SourceFile,
    first_source: &SourceFile,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.duplicate_source_path",
        Severity::Error,
        DiagnosticKind::Module,
        format!("multiple source files derive module path `{module_name}`"),
        Some(source.span(veln_source::TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("module_path", JsonValue::string(module_name)),
            ("source_path", JsonValue::string(source.path().as_str())),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("duplicate_origin")),
        (
            "message",
            JsonValue::string(format!(
                "The first source file deriving `{module_name}` is here."
            )),
        ),
        (
            "span",
            JsonValue::object([
                ("file", JsonValue::string(first_source.path().as_str())),
                (
                    "start",
                    JsonValue::object([
                        ("line", JsonValue::Number(1)),
                        ("column", JsonValue::Number(1)),
                        ("offset", JsonValue::Number(0)),
                    ]),
                ),
                (
                    "end",
                    JsonValue::object([
                        ("line", JsonValue::Number(1)),
                        ("column", JsonValue::Number(1)),
                        ("offset", JsonValue::Number(0)),
                    ]),
                ),
            ]),
        ),
    ]));
    diagnostic
}

fn reserved_source_module_diagnostic(source: &SourceFile, module_name: &str) -> Diagnostic {
    Diagnostic::new(
        "name.reserved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("module identity `{module_name}` conflicts with the standard prelude"),
        Some(source.span(TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("name", JsonValue::string(module_name)),
            ("namespace", JsonValue::string("module")),
            ("reserved_for", JsonValue::string("standard_prelude")),
        ]),
    )
}

pub(crate) fn validate_manifest_module(
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
        return Vec::new();
    };

    if manifest_module.name == source_module.name {
        return Vec::new();
    }

    let mut diagnostic = Diagnostic::new(
        "module.metadata_drift",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "manifest module name `{}` does not match derived module `{}`",
            manifest_module.name, source_module.name
        ),
        Some(manifest_module.name_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("canonical_owner", JsonValue::string("source_path")),
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
                "The package-relative source path owns the compiler-visible module name.",
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

fn unresolved_local_import_diagnostics(
    uses: &[UseDecl],
    derived_modules: &[(String, SourceFile)],
) -> Vec<Diagnostic> {
    uses.iter()
        .filter(|use_decl| {
            use_decl.name.contains("::")
                && !derived_modules
                    .iter()
                    .any(|(module_name, _)| module_name == &use_decl.name)
        })
        .map(|use_decl| unresolved_local_import_diagnostic(use_decl, derived_modules))
        .collect()
}

fn unresolved_local_import_diagnostic(
    use_decl: &UseDecl,
    derived_modules: &[(String, SourceFile)],
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.unresolved_import",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "local import `{}` has no matching selected source file",
            use_decl.name
        ),
        Some(use_decl.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("import_path")),
            ("module_path", JsonValue::string(use_decl.name.clone())),
        ]),
    );
    for (module_name, source) in derived_modules {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("selected_source_module")),
            (
                "message",
                JsonValue::string(format!(
                    "Selected source `{}` derives `{module_name}`.",
                    source.path().as_str()
                )),
            ),
        ]));
    }
    diagnostic
}

pub(crate) fn reachable_entry_module(
    module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
) -> SurfaceModule {
    let mut function_targets = module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            let name = function.name.clone()?;
            Some(FunctionTarget {
                name: name.clone(),
                module_name: function.module_name.clone(),
                target_name: name,
                target_module_name: function.module_name.clone(),
                arity: function.params.len(),
            })
        })
        .collect::<Vec<_>>();
    let aliases = function_alias_targets(module, &function_targets);
    function_targets.extend(aliases);
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
        aliases: module.aliases.clone(),
        types: module.types.clone(),
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
    target_name: String,
    target_module_name: Option<String>,
    arity: usize,
}

fn function_alias_targets(
    module: &SurfaceModule,
    function_targets: &[FunctionTarget],
) -> Vec<FunctionTarget> {
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            let target = target_for_alias_path(
                &alias.target,
                &module.uses,
                function_targets,
                alias.module_name.as_deref(),
            )?;
            Some(FunctionTarget {
                name,
                module_name: alias.module_name.clone(),
                target_name: target.target_name.clone(),
                target_module_name: target.target_module_name.clone(),
                arity: target.arity,
            })
        })
        .collect()
}

fn target_for_alias_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    function_targets: &'a [FunctionTarget],
    current_module: Option<&str>,
) -> Option<&'a FunctionTarget> {
    match segments {
        [name] => function_targets.iter().find(|target| target.name == *name),
        [_, .., name] => {
            let module_name =
                imported_module_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            function_targets.iter().find(|target| {
                target.name == *name && target.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
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
        let mut segments = vec![name.text.clone()];
        let mut next_index = index + 1;
        while next_index + 1 < tokens.len()
            && tokens[next_index].kind == TokenKind::DoubleColon
            && tokens[next_index + 1].kind == TokenKind::Ident
        {
            segments.push(tokens[next_index + 1].text.clone());
            next_index += 2;
        }
        let Some(next) = tokens.get(next_index) else {
            break;
        };
        if next.kind != TokenKind::LParen {
            index += 1;
            continue;
        }
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
            let mut segments = vec![tokens[index].text.clone()];
            index += 1;
            while tokens
                .get(index)
                .is_some_and(|token| token.kind == TokenKind::DoubleColon)
                && tokens
                    .get(index + 1)
                    .is_some_and(|token| token.kind == TokenKind::Ident)
            {
                segments.push(tokens[index + 1].text.clone());
                index += 2;
            }
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
    target_module.is_some_and(|module_name| {
        uses.iter().any(|use_decl| {
            use_decl.module_name.as_deref() == current_module && use_decl.name == module_name
        })
    })
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
    if let [name] = segments
        && let Some(binding) = local_bindings
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
                name: target.target_name.clone(),
                module_name: target.target_module_name.clone(),
            })
            .collect(),
        [_, .., name] => {
            let Some(module_name) =
                imported_module_for_path(uses, &segments[..segments.len() - 1], current_module)
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
                    name: target.target_name.clone(),
                    module_name: target.target_module_name.clone(),
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn imported_module_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a str> {
    let module_path = segments.join("::");
    uses.iter()
        .find(|use_decl| {
            use_decl.module_name.as_deref() == current_module
                && (use_decl.name == module_path || use_decl.alias == module_path)
        })
        .map(|use_decl| use_decl.name.as_str())
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
            "test foo() -> ()\n",
            "  helper()\n",
            "end\n",
            "fn helper() -> ()\n",
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
            "test foo() -> ()\n",
            "  vec_map([1], stringify)\n",
            "  ()\n",
            "end\n",
            "fn stringify(value: Int) -> String\n",
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
            "test foo() -> Bool\n",
            "  invoke(ready)\n",
            "end\n",
            "fn invoke(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
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
            "test foo() -> Bool\n",
            "  invoke(ready)\n",
            "end\n",
            "fn invoke(job: fn () -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
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
            "test foo() -> Bool\n",
            "  invoke()\n",
            "end\n",
            "fn invoke() -> Bool\n",
            "  let job: fn() -> Bool = ready\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
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
                    "app/main_test.veln",
                    concat!(
                        "use app::text\n",
                        "test foo() -> ()\n",
                        "  vec_map([1], app::text::stringify)\n",
                        "  ()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/text.veln",
                    concat!(
                        "fn stringify(value: Int) -> String\n",
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
                (Some("app::main_test"), FunctionKind::Test, Some("foo")),
                (Some("app::text"), FunctionKind::Function, Some("stringify")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_contract_helper() {
        let module = lower(concat!(
            "fn positive(value: Int) -> Bool\n",
            "  value > 0\n",
            "end\n",
            "pub fn main(value: Int) -> output: Int\n",
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
            "fn accepts(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "pub fn main() -> ()\n",
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
                    "app/main.veln",
                    concat!(
                        "use app::rules\n",
                        "pub fn main(value: Int) -> output: Int\n",
                        "  ensure app::rules::positive(output)\n",
                        "  value\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/rules.veln",
                    concat!(
                        "fn positive(value: Int) -> Bool\n",
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
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::rules"), FunctionKind::Function, Some("positive")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_imported_qualified_call() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "use app::util\n",
                        "pub fn main() -> Int\n",
                        "  app::util::value()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/util.veln",
                    concat!("fn value() -> Int\n", "  1\n", "end\n",),
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
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::util"), FunctionKind::Function, Some("value")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_imported_alias_target() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "use app::api\n",
                        "pub fn main() -> Int\n",
                        "  app::api::twice(21)\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/api.veln",
                    concat!("use app::impl\n", "pub fn twice = app::impl::double\n",),
                ),
                SourceFile::new(
                    "app/impl.veln",
                    concat!(
                        "fn double(value: Int) -> Int\n",
                        "  value + value\n",
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
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::impl"), FunctionKind::Function, Some("double")),
            ]
        );
    }

    #[test]
    fn run_entry_can_reach_qualified_contract_function_value() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "use app::rules\n",
                        "fn accepts(job: fn() -> Bool) -> Bool\n",
                        "  job()\n",
                        "end\n",
                        "pub fn main() -> ()\n",
                        "  require accepts(app::rules::ready)\n",
                        "  ()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/rules.veln",
                    concat!("fn ready() -> Bool\n", "  true\n", "end\n",),
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
                (Some("app::main"), FunctionKind::Function, Some("accepts")),
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::rules"), FunctionKind::Function, Some("ready")),
            ]
        );
    }

    #[test]
    fn imported_reachability_keeps_module_specific_function_names() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "use app::util\n",
                        "fn value() -> Int\n",
                        "  _\n",
                        "end\n",
                        "pub fn main() -> Int\n",
                        "  app::util::value()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/util.veln",
                    concat!("fn value() -> Int\n", "  1\n", "end\n",),
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
                (Some("app::main"), FunctionKind::Function, Some("main")),
                (Some("app::util"), FunctionKind::Function, Some("value")),
            ]
        );
    }

    #[test]
    fn bare_reachability_keeps_current_module_function_names() {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app/main.veln",
                    concat!(
                        "fn value() -> Int\n",
                        "  1\n",
                        "end\n",
                        "pub fn main() -> Int\n",
                        "  value()\n",
                        "end\n",
                    ),
                ),
                SourceFile::new(
                    "app/other.veln",
                    concat!("fn value() -> Int\n", "  _\n", "end\n",),
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
                (Some("app::main"), FunctionKind::Function, Some("value")),
                (Some("app::main"), FunctionKind::Function, Some("main")),
            ]
        );
    }

    #[test]
    fn local_binding_shadowing_function_name_does_not_reach_function() {
        let module = lower(concat!(
            "fn helper() -> Int\n",
            "  _\n",
            "end\n",
            "pub fn main() -> Int\n",
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
            "fn helper() -> Int\n",
            "  _\n",
            "end\n",
            "pub fn main(value: Option<Int>) -> Int\n",
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
            "pub fn main() -> Int\n",
            "  util::value()\n",
            "end\n",
            "fn value() -> Int\n",
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
            "fn positive(value: Int) -> Bool\n",
            "  value > 0\n",
            "end\n",
            "pub fn main() -> output: String\n",
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
            "test helper() -> ()\n",
            "  ()\n",
            "end\n",
            "fn foo() -> ()\n",
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
    fn manifest_module_name_cannot_override_derived_source_path() {
        let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                package: Default::default(),
                modules: vec![ManifestModule {
                    path: "src/main.veln".to_string(),
                    name: "manifest.main".to_string(),
                    path_span: span("veln.toml", 2, 2, 11),
                    name_span: span("veln.toml", 2, 20, 33),
                }],
                tools: Vec::new(),
            }),
        };

        let (module, diagnostics) = load_surface_module(&project);

        assert_eq!(module.module.as_ref().unwrap().name, "src::main");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "module.metadata_drift");
        assert_eq!(
            diagnostics[0].message,
            "manifest module name `manifest.main` does not match derived module `src::main`"
        );
    }

    #[test]
    fn source_mod_declaration_reports_module_diagnostic() {
        let source = SourceFile::new(
            "src/main.veln",
            "mod app.main\nfn main() -> ()\n  ()\nend\n",
        );
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: None,
        };

        let (_, diagnostics) = load_surface_module(&project);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "module.source_mod");
        assert_eq!(
            diagnostics[0].message,
            "source `mod` declarations are not supported"
        );
    }

    #[test]
    fn matching_manifest_module_name_does_not_report_drift() {
        let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                package: Default::default(),
                modules: vec![ManifestModule {
                    path: "src/main.veln".to_string(),
                    name: "src::main".to_string(),
                    path_span: span("veln.toml", 2, 2, 11),
                    name_span: span("veln.toml", 2, 20, 28),
                }],
                tools: Vec::new(),
            }),
        };

        let (module, diagnostics) = load_surface_module(&project);

        assert_eq!(module.module.as_ref().unwrap().name, "src::main");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "module.metadata_drift"),
            "{diagnostics:#?}"
        );
    }

    #[test]
    fn unselected_manifest_module_entries_do_not_report_drift() {
        let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
        let project = Project {
            root: ".".into(),
            files: vec![source],
            manifest: Some(ProjectManifest {
                path: SourcePath::new("veln.toml"),
                package: Default::default(),
                modules: vec![
                    ManifestModule {
                        path: "src/other.veln".to_string(),
                        name: "manifest.other".to_string(),
                        path_span: span("veln.toml", 2, 2, 12),
                        name_span: span("veln.toml", 2, 21, 35),
                    },
                    ManifestModule {
                        path: "src/main.veln".to_string(),
                        name: "src::main".to_string(),
                        path_span: span("veln.toml", 3, 2, 11),
                        name_span: span("veln.toml", 3, 20, 28),
                    },
                ],
                tools: Vec::new(),
            }),
        };

        let (_, diagnostics) = load_surface_module(&project);

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
