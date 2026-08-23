use veln_ast::{BodyLineKind, Expr, ExprKind, Function, Pattern, PatternKind, SurfaceModule};
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

pub(crate) fn valid_function_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|initial| initial.is_ascii_lowercase())
}

pub(crate) fn valid_type_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|initial| initial.is_ascii_uppercase())
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
        let Some((role, symbol)) = unresolved_recovery_role_and_symbol(&diagnostic.message) else {
            return true;
        };
        recovery_count(module, role, symbol, span.file.as_str()) != 1
    });
}

#[derive(Clone, Copy)]
enum RecoveryRole {
    Type,
    Callable,
}

fn unresolved_recovery_role_and_symbol(message: &str) -> Option<(RecoveryRole, &str)> {
    let (prefix, suffix) = message.split_once('`')?;
    let symbol = suffix.strip_suffix('`')?;
    let role = if prefix.contains("call_target") || prefix.contains("value") {
        RecoveryRole::Callable
    } else if prefix.contains("type") {
        RecoveryRole::Type
    } else {
        return None;
    };
    Some((role, symbol))
}

fn recovery_count(module: &SurfaceModule, role: RecoveryRole, symbol: &str, file: &str) -> usize {
    let invalid_functions = module.functions.iter().filter(|function| {
        function.name.as_deref() == Some(symbol)
            && !valid_function_name(symbol)
            && function.span.file.as_str() == file
    });
    let invalid_types = module.types.iter().filter(|type_decl| {
        type_decl.name.as_deref() == Some(symbol)
            && !valid_type_name(symbol)
            && type_decl.span.file.as_str() == file
    });
    let invalid_variants = module
        .types
        .iter()
        .flat_map(|type_decl| &type_decl.variants)
        .filter(|variant| {
            variant.name.as_deref() == Some(symbol)
                && !valid_type_name(symbol)
                && variant.span.file.as_str() == file
        });
    match role {
        RecoveryRole::Type => invalid_types.count(),
        RecoveryRole::Callable => invalid_functions.count() + invalid_variants.count(),
    }
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
