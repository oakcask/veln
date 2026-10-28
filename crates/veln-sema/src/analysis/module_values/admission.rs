use std::collections::HashMap;

use veln_ast::{BodyLine, BodyLineKind, Expr, ExprKind};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};

use super::Dependency;
use crate::adt::registry::ConstructorLookup;
use crate::adt::{descriptors::AdtConstructor, type_operations as adt};
use crate::diagnostics::span_json;
use crate::semantic_model::Type;
use crate::types::{ModuleValueIdentity, ModuleValueIndex, ModuleValueLookup, TypeEnvironment};

pub(super) fn check_admitted_expr(
    expr: &Expr,
    declaration: &veln_ast::ModuleValueDecl,
    value_indices: &HashMap<ModuleValueIdentity, ModuleValueIndex>,
    environment: &TypeEnvironment,
    expected: Option<&Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &expr.kind {
        ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
        ExprKind::Record(fields) => {
            if fields.is_empty() && expected.is_some_and(|ty| ty.dict_parts().is_some()) {
                diagnostics.push(unsupported_diagnostic(expr, declaration));
                return;
            }
            for field in fields {
                check_admitted_expr(
                    &field.expr,
                    declaration,
                    value_indices,
                    environment,
                    expected.and_then(|ty| ty.record_field(&field.name)),
                    diagnostics,
                );
            }
        }
        ExprKind::Prefix {
            op: veln_ast::PrefixOp::Negate,
            expr: inner,
        } if matches!(inner.kind, ExprKind::IntLiteral(_)) => {}
        ExprKind::NamePath { segments, .. } => check_admitted_name(
            expr,
            segments,
            declaration,
            value_indices,
            environment,
            diagnostics,
        ),
        ExprKind::Call { callee, args } => {
            let scope = AdmissionScope {
                declaration,
                value_indices,
                environment,
            };
            check_admitted_call(expr, callee, args, &scope, expected, diagnostics);
        }
        ExprKind::Missing => {}
        _ => diagnostics.push(unsupported_diagnostic(expr, declaration)),
    }
}

fn check_admitted_name(
    expr: &Expr,
    segments: &[String],
    declaration: &veln_ast::ModuleValueDecl,
    _value_indices: &HashMap<ModuleValueIdentity, ModuleValueIndex>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let current_module = declaration.module_name.as_deref();
    if !matches!(
        environment.module_value_path_in_owner(
            segments,
            current_module,
            declaration.package_name.as_deref(),
        ),
        ModuleValueLookup::Missing
    ) || matches!(
        environment
            .adts
            .nullary_constructor(segments, current_module, &environment.uses),
        ConstructorLookup::Found(_)
    ) {
        return;
    }
    if known_function(
        environment,
        segments,
        current_module,
        declaration.package_name.as_deref(),
    ) {
        diagnostics.push(unsupported_diagnostic(expr, declaration));
    }
}

fn check_admitted_call(
    expr: &Expr,
    callee: &Expr,
    args: &[Expr],
    scope: &AdmissionScope<'_>,
    expected: Option<&Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match constructor_call(
        scope.environment,
        callee,
        scope.declaration.module_name.as_deref(),
        expected,
    ) {
        found @ ConstructorCall::Found { .. } => {
            check_constructor_arguments(expr, args, scope, expected, found, diagnostics)
        }
        ConstructorCall::Ambiguous => check_admitted_arguments(args, scope, diagnostics),
        ConstructorCall::Missing => {
            let Some(segments) = callee.callee_name_path() else {
                diagnostics.push(unsupported_diagnostic(expr, scope.declaration));
                return;
            };
            let known_module_value = {
                !matches!(
                    scope.environment.module_value_path_in_owner(
                        segments,
                        scope.declaration.module_name.as_deref(),
                        scope.declaration.package_name.as_deref(),
                    ),
                    ModuleValueLookup::Missing
                )
            };
            let known_function_call = known_function(
                scope.environment,
                segments,
                scope.declaration.module_name.as_deref(),
                scope.declaration.package_name.as_deref(),
            );
            if known_module_value || known_function_call {
                diagnostics.push(unsupported_diagnostic(expr, scope.declaration));
            } else {
                check_admitted_arguments(args, scope, diagnostics);
            }
        }
    }
}

struct AdmissionScope<'a> {
    declaration: &'a veln_ast::ModuleValueDecl,
    value_indices: &'a HashMap<ModuleValueIdentity, ModuleValueIndex>,
    environment: &'a TypeEnvironment,
}

fn check_constructor_arguments(
    expr: &Expr,
    args: &[Expr],
    scope: &AdmissionScope<'_>,
    expected: Option<&Type>,
    found: ConstructorCall<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let ConstructorCall::Found {
        constructor,
        diagnostic_name,
        reason,
    } = found
    else {
        unreachable!("constructor argument checking requires a resolved constructor");
    };
    let expected_count = constructor.variant.payload_fields.len();
    if args.len() != expected_count {
        diagnostics.push(constructor_arity_diagnostic(
            expr,
            expected_count,
            args.len(),
            diagnostic_name,
            reason,
        ));
    }
    for (index, arg) in args.iter().enumerate() {
        let payload_expected = expected
            .and_then(|expected| adt::payload_type(expected, constructor, index))
            .or_else(|| {
                adt::payload_type_with_resolved_args(constructor, index, |_| Type::Unknown)
            });
        check_admitted_expr(
            arg,
            scope.declaration,
            scope.value_indices,
            scope.environment,
            payload_expected.as_ref(),
            diagnostics,
        );
    }
}

fn check_admitted_arguments(
    args: &[Expr],
    scope: &AdmissionScope<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for arg in args {
        check_admitted_expr(
            arg,
            scope.declaration,
            scope.value_indices,
            scope.environment,
            None,
            diagnostics,
        );
    }
}

pub(super) fn collect_dependencies(
    expr: &Expr,
    declaration: &veln_ast::ModuleValueDecl,
    value_indices: &HashMap<ModuleValueIdentity, ModuleValueIndex>,
    environment: &TypeEnvironment,
    bindings: &mut Vec<String>,
    dependencies: &mut Vec<Dependency>,
    has_invalid_dependency: &mut bool,
) {
    if let ExprKind::NamePath { segments, .. } = &expr.kind
        && !matches!(segments.as_slice(), [name] if bindings.iter().rev().any(|binding| binding == name))
        && let Some(target) = module_value_index(
            value_indices,
            segments,
            declaration.module_name.as_deref(),
            declaration.package_name.as_deref(),
            environment,
        )
    {
        match target {
            ModuleValueIndex::Unique(target) => dependencies.push(Dependency {
                target,
                span: expr.span.clone(),
            }),
            ModuleValueIndex::Collision => *has_invalid_dependency = true,
        }
    }
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            collect_dependencies(
                scrutinee,
                declaration,
                value_indices,
                environment,
                bindings,
                dependencies,
                has_invalid_dependency,
            );
            for arm in arms {
                let binding_count = bindings.len();
                arm.pattern
                    .for_each_binding(&mut |name| bindings.push(name.to_string()));
                collect_dependencies(
                    &arm.expr,
                    declaration,
                    value_indices,
                    environment,
                    bindings,
                    dependencies,
                    has_invalid_dependency,
                );
                bindings.truncate(binding_count);
            }
        }
        ExprKind::Begin { body, .. } => {
            let binding_count = bindings.len();
            collect_body_dependencies(
                body,
                declaration,
                value_indices,
                environment,
                bindings,
                dependencies,
                has_invalid_dependency,
            );
            bindings.truncate(binding_count);
        }
        _ => expr.for_each_child(&mut |child| {
            collect_dependencies(
                child,
                declaration,
                value_indices,
                environment,
                bindings,
                dependencies,
                has_invalid_dependency,
            );
        }),
    }
}

fn collect_body_dependencies(
    body: &[BodyLine],
    declaration: &veln_ast::ModuleValueDecl,
    value_indices: &HashMap<ModuleValueIdentity, ModuleValueIndex>,
    environment: &TypeEnvironment,
    bindings: &mut Vec<String>,
    dependencies: &mut Vec<Dependency>,
    has_invalid_dependency: &mut bool,
) {
    for line in body {
        match &line.kind {
            BodyLineKind::Let { pattern, expr, .. } => {
                collect_dependencies(
                    expr,
                    declaration,
                    value_indices,
                    environment,
                    bindings,
                    dependencies,
                    has_invalid_dependency,
                );
                pattern.for_each_binding(&mut |name| bindings.push(name.to_string()));
            }
            BodyLineKind::Expr { expr } => collect_dependencies(
                expr,
                declaration,
                value_indices,
                environment,
                bindings,
                dependencies,
                has_invalid_dependency,
            ),
            BodyLineKind::Defer { body, .. } => {
                let binding_count = bindings.len();
                collect_body_dependencies(
                    body,
                    declaration,
                    value_indices,
                    environment,
                    bindings,
                    dependencies,
                    has_invalid_dependency,
                );
                bindings.truncate(binding_count);
            }
        }
    }
}

fn module_value_index(
    value_indices: &HashMap<ModuleValueIdentity, ModuleValueIndex>,
    segments: &[String],
    current_module: Option<&str>,
    current_package: Option<&str>,
    environment: &TypeEnvironment,
) -> Option<ModuleValueIndex> {
    let (name, _) = segments.split_last()?;
    environment
        .same_module_value_path_matches_in_owner(segments, current_module, current_package)
        .then(|| {
            value_indices.get(&(
                current_package.map(str::to_owned),
                current_module.map(str::to_owned),
                name.clone(),
            ))
        })
        .flatten()
        .copied()
}

fn known_function(
    environment: &TypeEnvironment,
    segments: &[String],
    current_module: Option<&str>,
    current_package: Option<&str>,
) -> bool {
    match segments {
        [name] => !matches!(
            environment.unqualified_function_in_owner(name, current_module, current_package),
            crate::types::FunctionLookup::Missing
        ),
        _ => environment
            .function_path_for_value_in_owner(segments, current_module, current_package)
            .is_some(),
    }
}

enum ConstructorCall<'a> {
    Found {
        constructor: AdtConstructor<'a>,
        diagnostic_name: String,
        reason: &'static str,
    },
    Ambiguous,
    Missing,
}

fn constructor_call<'a>(
    environment: &'a TypeEnvironment,
    callee: &Expr,
    current_module: Option<&str>,
    expected: Option<&Type>,
) -> ConstructorCall<'a> {
    let Some((segments, _)) = callee.callee_name_path_and_type_args() else {
        return ConstructorCall::Missing;
    };
    let contextual = (segments.len() == 1)
        .then_some(expected)
        .flatten()
        .and_then(|expected| {
            environment
                .adts
                .descriptor_for_type_prefer_module(expected, current_module)
        })
        .and_then(|descriptor| {
            environment.adts.constructor_for_descriptor(
                segments,
                descriptor,
                current_module,
                &environment.uses,
            )
        });
    match contextual.map(ConstructorLookup::Found).unwrap_or_else(|| {
        environment
            .adts
            .constructor(segments, current_module, &environment.uses)
    }) {
        ConstructorLookup::Found(constructor) => ConstructorCall::Found {
            constructor,
            diagnostic_name: constructor.descriptor.diagnostic_name.clone(),
            reason: match constructor.descriptor.type_name.as_str() {
                "Option" => "option_constructor_arity_mismatch",
                "Result" => "result_constructor_arity_mismatch",
                _ => "constructor_arity_mismatch",
            },
        },
        ConstructorLookup::Ambiguous => ConstructorCall::Ambiguous,
        ConstructorLookup::Missing => ConstructorCall::Missing,
    }
}

fn constructor_arity_diagnostic(
    expr: &Expr,
    expected_count: usize,
    actual_count: usize,
    diagnostic_name: String,
    reason: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        format!("core.{reason}"),
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "{diagnostic_name} constructor expects {expected_count} argument, but got {actual_count}"
        ),
        Some(expr.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("core_lowering")),
            ("node_id", JsonValue::string(expr.node_id.display("expr"))),
            ("reason", JsonValue::string(reason)),
            (
                "facts",
                JsonValue::object([
                    (
                        "expected_argument_count",
                        JsonValue::Number(expected_count as i64),
                    ),
                    (
                        "actual_argument_count",
                        JsonValue::Number(actual_count as i64),
                    ),
                ]),
            ),
        ]),
    )
}

fn unsupported_diagnostic(expr: &Expr, declaration: &veln_ast::ModuleValueDecl) -> Diagnostic {
    let name = declaration.name.as_deref().unwrap_or("<missing>");
    let mut diagnostic = Diagnostic::new(
        "value.module_initializer_unsupported",
        Severity::Error,
        DiagnosticKind::Type,
        "module value initializer expression is not immutable data construction",
        Some(expr.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("type_check")),
            ("node_id", JsonValue::string(expr.node_id.display("expr"))),
            ("name", JsonValue::string(name)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("module_value_declaration")),
        (
            "message",
            JsonValue::string(format!("Module value `{name}` is declared here.")),
        ),
        ("span", span_json(&declaration.span)),
    ]));
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("permitted_initializers")),
        (
            "message",
            JsonValue::string(
                "Use literals, records, ADT constructors, or same-module module values.",
            ),
        ),
    ]));
    diagnostic
}
