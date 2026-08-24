use veln_ast::{
    BodyLineKind, Expr, ExprKind, Function, FunctionKind, Pattern, PatternKind, PublicAliasKind,
    SurfaceModule,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

#[derive(Clone, Copy)]
enum NameClass {
    Type,
    Constructor,
    Function,
    ValueBinding,
}

impl NameClass {
    fn detail(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Constructor => "constructor",
            Self::Function => "function",
            Self::ValueBinding => "value_binding",
        }
    }

    fn subject(self) -> &'static str {
        match self {
            Self::Type => "type name",
            Self::Constructor => "constructor name",
            Self::Function => "function name",
            Self::ValueBinding => "binding name",
        }
    }

    fn requires_uppercase(self) -> bool {
        matches!(self, Self::Type | Self::Constructor)
    }
}

pub(crate) fn check_source_identifier_casing(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for type_decl in &module.types {
        check_name(
            &mut diagnostics,
            type_decl.name.as_deref(),
            type_decl.name_span.as_ref(),
            NameClass::Type,
            "declaration",
        );
        for variant in &type_decl.variants {
            check_name(
                &mut diagnostics,
                variant.name.as_deref(),
                variant.name_span.as_ref(),
                NameClass::Constructor,
                "declaration",
            );
        }
    }
    for function in &module.functions {
        check_name(
            &mut diagnostics,
            function.name.as_deref(),
            function.name_span.as_ref(),
            NameClass::Function,
            "declaration",
        );
        check_function_bindings(&mut diagnostics, function);
    }
    for alias in &module.aliases {
        match alias.kind {
            PublicAliasKind::Function => check_name(
                &mut diagnostics,
                alias.name.as_deref(),
                alias.name_span.as_ref(),
                NameClass::Function,
                "declaration",
            ),
            PublicAliasKind::Type => check_name(
                &mut diagnostics,
                alias.name.as_deref(),
                alias.name_span.as_ref(),
                NameClass::Type,
                "declaration",
            ),
            PublicAliasKind::Schema => {}
        }
    }
    for handler in &module.handlers {
        for param in &handler.params {
            check_name(
                &mut diagnostics,
                Some(&param.name),
                Some(&param.name_span),
                NameClass::ValueBinding,
                "binding",
            );
        }
        for clause in &handler.operation_clauses {
            for param in &clause.params {
                check_name(
                    &mut diagnostics,
                    Some(&param.name),
                    Some(&param.name_span),
                    NameClass::ValueBinding,
                    "binding",
                );
            }
            check_expr_bindings(&mut diagnostics, &clause.body);
        }
    }
    diagnostics
}

pub fn valid_function_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|initial| initial.is_ascii_lowercase())
}

pub fn valid_type_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|initial| initial.is_ascii_uppercase())
}

pub fn valid_public_alias_name(kind: PublicAliasKind, name: &str) -> bool {
    match kind {
        PublicAliasKind::Function => valid_function_name(name),
        PublicAliasKind::Type => valid_type_name(name),
        PublicAliasKind::Schema => true,
    }
}

pub(crate) fn valid_value_binding_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|initial| initial.is_ascii_lowercase())
}

pub(crate) fn suppress_unique_local_recovery_derivatives(
    module: &SurfaceModule,
    diagnostics: &mut Vec<Diagnostic>,
) {
    diagnostics.retain(|diagnostic| {
        if diagnostic.id != "name.unresolved" {
            return true;
        }
        let Some(span) = &diagnostic.span else {
            return true;
        };
        let Some((role, symbol)) = unresolved_recovery_role_and_symbol(diagnostic) else {
            return true;
        };
        !has_unique_recovery_candidate(module, role, symbol, span)
    });
}

pub(crate) fn suppress_quarantined_type_alias_derivatives(
    module: &SurfaceModule,
    diagnostics: &mut Vec<Diagnostic>,
) {
    suppress_unique_alias_recovery_derivatives(module, diagnostics, PublicAliasKind::Type);
}

pub(crate) fn suppress_quarantined_function_alias_derivatives(
    module: &SurfaceModule,
    diagnostics: &mut Vec<Diagnostic>,
) {
    suppress_unique_alias_recovery_derivatives(module, diagnostics, PublicAliasKind::Function);
}

fn suppress_unique_alias_recovery_derivatives(
    module: &SurfaceModule,
    diagnostics: &mut Vec<Diagnostic>,
    kind: PublicAliasKind,
) {
    diagnostics.retain(|diagnostic| {
        if suppress_valid_invalid_alias_target_derivative(module, diagnostic, kind) {
            return false;
        }
        if diagnostic.id != "name.unresolved" {
            return true;
        }
        let Some(span) = &diagnostic.span else {
            return true;
        };
        let Some((role, symbol)) = unresolved_recovery_role_and_symbol(diagnostic) else {
            return true;
        };
        !has_unique_alias_recovery_candidate(module, kind, role, symbol, span)
    });
}

fn suppress_valid_invalid_alias_target_derivative(
    module: &SurfaceModule,
    diagnostic: &Diagnostic,
    kind: PublicAliasKind,
) -> bool {
    if diagnostic.id != "name.unresolved" {
        return false;
    }
    let Some(span) = &diagnostic.span else {
        return false;
    };
    let Some(expected_kind) = diagnostic_string_detail(diagnostic, "expected_kind") else {
        return false;
    };
    if !alias_expected_kind_matches(kind, expected_kind) {
        return false;
    }
    module.aliases.iter().any(|alias| {
        alias.kind == kind
            && alias.span.file == span.file
            && alias.span.start.offset == span.start.offset
            && alias.span.end.offset == span.end.offset
            && alias.name.as_deref().is_some_and(|name| {
                !valid_public_alias_name(alias.kind, name) && alias_target_exists(module, alias)
            })
    })
}

fn alias_expected_kind_matches(kind: PublicAliasKind, expected_kind: &str) -> bool {
    matches!(
        (kind, expected_kind),
        (PublicAliasKind::Function, "function") | (PublicAliasKind::Type, "type")
    )
}

fn alias_target_exists(module: &SurfaceModule, alias: &veln_ast::PublicAlias) -> bool {
    match alias.kind {
        PublicAliasKind::Function => alias_target_function_exists(module, alias),
        PublicAliasKind::Type => alias_target_type_exists(module, alias),
        PublicAliasKind::Schema => false,
    }
}

fn alias_target_function_exists(module: &SurfaceModule, alias: &veln_ast::PublicAlias) -> bool {
    let Some(target_name) = alias.target.last() else {
        return false;
    };
    if !valid_function_name(target_name) {
        return false;
    }
    let target_module = alias_target_module(module, alias);
    module.functions.iter().any(|function| {
        function.kind == FunctionKind::Function
            && function.name.as_ref() == Some(target_name)
            && function.module_name == target_module
    })
}

fn alias_target_type_exists(module: &SurfaceModule, alias: &veln_ast::PublicAlias) -> bool {
    let Some(target_name) = alias.target.last() else {
        return false;
    };
    if !valid_type_name(target_name) {
        return false;
    }
    let target_module = alias_target_module(module, alias);
    module.types.iter().any(|type_decl| {
        type_decl.name.as_ref() == Some(target_name) && type_decl.module_name == target_module
    })
}

fn alias_target_module(module: &SurfaceModule, alias: &veln_ast::PublicAlias) -> Option<String> {
    match alias.target.as_slice() {
        [_] => alias.module_name.clone(),
        [_, .., _] => module
            .uses
            .iter()
            .find(|use_decl| {
                use_decl.module_name.as_deref() == alias.module_name.as_deref()
                    && (use_decl.alias == alias.target[..alias.target.len() - 1].join("::")
                        || use_decl.name == alias.target[..alias.target.len() - 1].join("::"))
            })
            .map(|use_decl| use_decl.name.clone()),
        _ => None,
    }
}

pub(crate) fn type_alias_targets_invalid_source_type(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> bool {
    match segments {
        [] => true,
        [name] => module.types.iter().any(|type_decl| {
            type_decl.name.as_deref() == Some(name)
                && type_decl.module_name.as_deref() == current_module
                && !valid_type_name(name)
        }),
        [_, .., name] => {
            let Some(module_name) = module
                .uses
                .iter()
                .find(|use_decl| {
                    use_decl.module_name.as_deref() == current_module
                        && use_decl.alias == segments[..segments.len() - 1].join("::")
                })
                .map(|use_decl| use_decl.name.as_str())
            else {
                return false;
            };
            module.types.iter().any(|type_decl| {
                type_decl.name.as_deref() == Some(name)
                    && type_decl.module_name.as_deref() == Some(module_name)
                    && !valid_type_name(name)
            })
        }
    }
}

#[derive(Clone, Copy)]
enum RecoveryRole {
    Type,
    Value,
    CallTarget,
    ContractPredicate,
    SatisfyPredicate,
}

fn unresolved_recovery_role_and_symbol(diagnostic: &Diagnostic) -> Option<(RecoveryRole, &str)> {
    let namespace = diagnostic_string_detail(diagnostic, "namespace")?;
    let symbol = diagnostic_string_detail(diagnostic, "symbol")?;
    let role = match namespace {
        "type" => RecoveryRole::Type,
        "value" => RecoveryRole::Value,
        "call_target" => RecoveryRole::CallTarget,
        "contract_predicate" => RecoveryRole::ContractPredicate,
        "satisfy_predicate" => RecoveryRole::SatisfyPredicate,
        _ => return None,
    };
    Some((role, symbol))
}

fn diagnostic_string_detail<'a>(diagnostic: &'a Diagnostic, key: &str) -> Option<&'a str> {
    let JsonValue::Object(entries) = &diagnostic.details else {
        return None;
    };
    entries.iter().find_map(|(entry_key, value)| {
        if entry_key == key
            && let JsonValue::String(value) = value
        {
            Some(value.as_str())
        } else {
            None
        }
    })
}

fn has_unique_recovery_candidate(
    module: &SurfaceModule,
    role: RecoveryRole,
    symbol: &str,
    use_span: &SourceSpan,
) -> bool {
    recovery_count(module, role, symbol, use_span) == 1
}

fn recovery_count(
    module: &SurfaceModule,
    role: RecoveryRole,
    symbol: &str,
    use_span: &SourceSpan,
) -> usize {
    let invalid_functions = module.functions.iter().filter(|function| {
        function.kind == FunctionKind::Function
            && function.name.as_deref() == Some(symbol)
            && !valid_function_name(symbol)
            && function.span.file == use_span.file
    });
    let invalid_types = module.types.iter().filter(|type_decl| {
        type_decl.name.as_deref() == Some(symbol)
            && !valid_type_name(symbol)
            && type_decl.span.file == use_span.file
    });
    let invalid_variants = module
        .types
        .iter()
        .flat_map(|type_decl| &type_decl.variants)
        .filter(|variant| {
            variant.name.as_deref() == Some(symbol)
                && !valid_type_name(symbol)
                && variant.span.file == use_span.file
        });
    let invalid_type_aliases = module.aliases.iter().filter(|alias| {
        alias.kind == PublicAliasKind::Type
            && alias.name.as_deref() == Some(symbol)
            && !valid_public_alias_name(alias.kind, symbol)
            && alias.span.file == use_span.file
    });
    let invalid_function_aliases = module.aliases.iter().filter(|alias| {
        alias.kind == PublicAliasKind::Function
            && alias.name.as_deref() == Some(symbol)
            && !valid_public_alias_name(alias.kind, symbol)
            && alias.span.file == use_span.file
    });
    let invalid_value_bindings = local_recovery_binding_count(module, role, symbol, use_span);
    match role {
        RecoveryRole::Type => invalid_types.count() + invalid_type_aliases.count(),
        RecoveryRole::Value | RecoveryRole::ContractPredicate | RecoveryRole::SatisfyPredicate => {
            invalid_value_bindings
        }
        RecoveryRole::CallTarget => {
            invalid_functions.count()
                + invalid_variants.count()
                + invalid_function_aliases.count()
                + invalid_value_bindings
        }
    }
}

fn has_unique_alias_recovery_candidate(
    module: &SurfaceModule,
    kind: PublicAliasKind,
    role: RecoveryRole,
    symbol: &str,
    use_span: &SourceSpan,
) -> bool {
    let expected_role = match kind {
        PublicAliasKind::Function => RecoveryRole::CallTarget,
        PublicAliasKind::Type => RecoveryRole::Type,
        PublicAliasKind::Schema => return false,
    };
    if !matches_recovery_role(role, expected_role) {
        return false;
    }
    if recovery_count(module, role, symbol, use_span) != 1 {
        return false;
    }
    module
        .aliases
        .iter()
        .filter(|alias| {
            alias.kind == kind
                && alias.name.as_deref() == Some(symbol)
                && !valid_public_alias_name(alias.kind, symbol)
                && alias.span.file == use_span.file
        })
        .take(2)
        .count()
        == 1
}

fn matches_recovery_role(left: RecoveryRole, right: RecoveryRole) -> bool {
    std::mem::discriminant(&left) == std::mem::discriminant(&right)
}

fn local_recovery_binding_count(
    module: &SurfaceModule,
    role: RecoveryRole,
    symbol: &str,
    use_span: &SourceSpan,
) -> usize {
    module
        .functions
        .iter()
        .filter(|function| {
            function.span.file == use_span.file && span_contains(&function.span, use_span)
        })
        .map(|function| {
            function
                .params
                .iter()
                .filter(|param| {
                    param.name == symbol
                        && !valid_value_binding_name(&param.name)
                        && span_starts_not_after(&param.name_span, use_span)
                        && recovery_binding_matches_role(param.ty.as_deref(), role)
                })
                .count()
                + function
                    .body
                    .iter()
                    .filter(|line| span_starts_not_after(&line.span, use_span))
                    .map(|line| local_recovery_bindings_in_line(line, role, symbol, use_span))
                    .sum::<usize>()
        })
        .sum()
}

fn local_recovery_bindings_in_line(
    line: &veln_ast::BodyLine,
    role: RecoveryRole,
    symbol: &str,
    use_span: &SourceSpan,
) -> usize {
    let BodyLineKind::Let {
        pattern,
        annotation,
        ..
    } = &line.kind
    else {
        return 0;
    };
    pattern_bindings(pattern)
        .into_iter()
        .filter(|(name, span)| {
            *name == symbol
                && !valid_value_binding_name(name)
                && span_starts_not_after(span, use_span)
                && recovery_binding_matches_role(annotation.as_deref(), role)
        })
        .count()
}

fn pattern_bindings(pattern: &Pattern) -> Vec<(&str, &SourceSpan)> {
    match &pattern.kind {
        PatternKind::Binding(name) => vec![(name.as_str(), &pattern.span)],
        PatternKind::Record(fields) => fields
            .iter()
            .flat_map(|field| pattern_bindings(&field.pattern))
            .collect(),
        PatternKind::Constructor { args, .. } => args.iter().flat_map(pattern_bindings).collect(),
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => Vec::new(),
    }
}

fn recovery_binding_matches_role(annotation: Option<&str>, role: RecoveryRole) -> bool {
    match role {
        RecoveryRole::CallTarget => annotation.is_some_and(annotation_is_function_type),
        RecoveryRole::Value | RecoveryRole::ContractPredicate | RecoveryRole::SatisfyPredicate => {
            true
        }
        RecoveryRole::Type => false,
    }
}

fn annotation_is_function_type(annotation: &str) -> bool {
    annotation.trim_start().starts_with("fn(")
}

fn span_contains(region: &SourceSpan, span: &SourceSpan) -> bool {
    region.file == span.file
        && span.start.offset >= region.start.offset
        && span.end.offset <= region.end.offset
}

fn span_starts_not_after(binding_span: &SourceSpan, use_span: &SourceSpan) -> bool {
    binding_span.file == use_span.file && binding_span.start.offset <= use_span.start.offset
}

fn check_function_bindings(diagnostics: &mut Vec<Diagnostic>, function: &Function) {
    for param in &function.params {
        check_name(
            diagnostics,
            Some(&param.name),
            Some(&param.name_span),
            NameClass::ValueBinding,
            "binding",
        );
    }
    if let Some(binding) = &function.return_binding {
        check_name(
            diagnostics,
            Some(&binding.name),
            Some(&binding.span),
            NameClass::ValueBinding,
            "binding",
        );
    }
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let { pattern, expr, .. } => {
                check_pattern_bindings(diagnostics, pattern);
                check_expr_bindings(diagnostics, expr);
            }
            BodyLineKind::Expr { expr } => check_expr_bindings(diagnostics, expr),
        }
    }
}

fn check_expr_bindings(diagnostics: &mut Vec<Diagnostic>, expr: &Expr) {
    match &expr.kind {
        ExprKind::Hole {
            satisfy: Some(satisfy),
            ..
        } => check_name(
            diagnostics,
            satisfy.candidate.as_deref(),
            satisfy.candidate_span.as_ref(),
            NameClass::ValueBinding,
            "binding",
        ),
        ExprKind::TypeApply { callee, .. }
        | ExprKind::FieldAccess { base: callee, .. }
        | ExprKind::Try(callee)
        | ExprKind::Prefix { expr: callee, .. } => check_expr_bindings(diagnostics, callee),
        ExprKind::Call { callee, args } => {
            check_expr_bindings(diagnostics, callee);
            for arg in args {
                check_expr_bindings(diagnostics, arg);
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                check_expr_bindings(diagnostics, arg);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            check_expr_bindings(diagnostics, body);
            for arg in args {
                check_expr_bindings(diagnostics, arg);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            check_expr_bindings(diagnostics, input);
            check_expr_bindings(diagnostics, base);
        }
        ExprKind::SchemaEncode { value, .. } => check_expr_bindings(diagnostics, value),
        ExprKind::Record(fields) => {
            for field in fields {
                check_expr_bindings(diagnostics, &field.expr);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                check_expr_bindings(diagnostics, &entry.key);
                check_expr_bindings(diagnostics, &entry.value);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                check_expr_bindings(diagnostics, item);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            check_expr_bindings(diagnostics, scrutinee);
            for arm in arms {
                check_pattern_bindings(diagnostics, &arm.pattern);
                check_expr_bindings(diagnostics, &arm.expr);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            check_expr_bindings(diagnostics, condition);
            check_expr_bindings(diagnostics, then_branch);
            for branch in else_if_branches {
                check_expr_bindings(diagnostics, &branch.condition);
                check_expr_bindings(diagnostics, &branch.expr);
            }
            check_expr_bindings(diagnostics, else_branch);
        }
        ExprKind::Binary { left, right, .. } => {
            check_expr_bindings(diagnostics, left);
            check_expr_bindings(diagnostics, right);
        }
        ExprKind::Missing
        | ExprKind::Hole { satisfy: None, .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn check_pattern_bindings(diagnostics: &mut Vec<Diagnostic>, pattern: &Pattern) {
    match &pattern.kind {
        PatternKind::Binding(name) => check_name(
            diagnostics,
            Some(name),
            Some(&pattern.span),
            NameClass::ValueBinding,
            "binding",
        ),
        PatternKind::Record(fields) => {
            for field in fields {
                check_pattern_bindings(diagnostics, &field.pattern);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                check_pattern_bindings(diagnostics, arg);
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

fn check_name(
    diagnostics: &mut Vec<Diagnostic>,
    name: Option<&str>,
    span: Option<&SourceSpan>,
    class: NameClass,
    occurrence: &'static str,
) {
    let (Some(name), Some(span)) = (name, span) else {
        return;
    };
    let valid = name.chars().next().is_some_and(|initial| {
        if class.requires_uppercase() {
            initial.is_ascii_uppercase()
        } else {
            initial.is_ascii_lowercase()
        }
    });
    if valid {
        return;
    }
    let required_initial = if class.requires_uppercase() {
        "ascii_uppercase"
    } else {
        "ascii_lowercase"
    };
    diagnostics.push(Diagnostic::new(
        "name.invalid_case",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "{} must start with an ASCII {} letter",
            class.subject(),
            if class.requires_uppercase() {
                "uppercase"
            } else {
                "lowercase"
            }
        ),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("origin", JsonValue::string("source")),
            ("occurrence", JsonValue::string(occurrence)),
            ("name", JsonValue::string(name)),
            ("name_class", JsonValue::string(class.detail())),
            ("required_initial", JsonValue::string(required_initial)),
            (
                "observed_initial",
                JsonValue::string(observed_initial(name)),
            ),
        ]),
    ));
}

fn observed_initial(name: &str) -> &'static str {
    match name.chars().next() {
        Some(initial) if initial.is_ascii_uppercase() => "ascii_uppercase",
        Some(initial) if initial.is_ascii_lowercase() => "ascii_lowercase",
        Some('_') => "underscore",
        _ => "other",
    }
}
