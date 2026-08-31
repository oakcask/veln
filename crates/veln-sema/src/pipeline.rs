use std::collections::BTreeSet;

use veln_ast::{FunctionKind, InvalidName, NameClass, NameOccurrence, SurfaceModule, Visibility};
use veln_core::CheckedProgram;
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_ir::{TypedProgram, lower_checked_core};

use crate::analysis::{
    check_declared_effect_labels, check_duplicate_constructor_names, check_duplicate_effect_names,
    check_duplicate_function_names, check_duplicate_schema_names, check_duplicate_type_names,
    check_duplicate_use_aliases, check_function_body, check_handler_declarations,
    check_module_boundary, check_public_aliases, check_public_function_boundary,
    check_reserved_prelude_aliases, check_schema_field_primitives, check_schema_type_references,
    check_test_declaration_boundary,
};
use crate::lowering::{lower_project_surface_module_to_core, lower_surface_module_to_core};
use crate::schema;
use crate::source_less_lookup::validate_source_less_lookup_registries;
use crate::types::{
    ReusableStandardEnvironment, TypeEnvironment, prepare_current_reusable_standard_environment,
    prepare_reusable_standard_environment,
};

#[derive(Clone, Debug)]
pub struct LoweredSurfaceModule {
    pub diagnostics: Vec<Diagnostic>,
    pub core: Option<CheckedProgram>,
    pub ir: Option<TypedProgram>,
}

pub fn analyze_surface_module(module: &SurfaceModule) -> Vec<Diagnostic> {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return vec![failure.diagnostic()];
    }
    let environment = TypeEnvironment::from_module(module);
    analyze_surface_module_with_environment(module, &environment, true)
}

#[cfg(test)]
pub(crate) fn analyze_surface_module_with_base_for_test(
    module: &SurfaceModule,
    base: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let environment = TypeEnvironment::from_module_with_base_for_test(module, base);
    analyze_surface_module_with_environment(module, &environment, true)
}

pub fn check_project_surface_module(
    module: &SurfaceModule,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    if let Err(failure) = validate_source_less_lookup_registries() {
        let diagnostics = vec![failure.diagnostic()];
        return (diagnostics.clone(), lowered_internal_failure(diagnostics));
    }
    let environment = TypeEnvironment::from_module(module);
    check_project_surface_module_with_environment(module, environment)
}

pub fn check_project_surface_module_with_standard_environment(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    if let Err(failure) = validate_source_less_lookup_registries() {
        let diagnostics = vec![failure.diagnostic()];
        return (diagnostics.clone(), lowered_internal_failure(diagnostics));
    }
    let environment = TypeEnvironment::from_module_with_standard(module, standard);
    check_project_surface_module_with_environment(module, environment)
}

pub fn check_project_surface_modules_with_standard_environment(
    application_module: &SurfaceModule,
    selected_standard_module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    if let Err(failure) = validate_source_less_lookup_registries() {
        let diagnostics = vec![failure.diagnostic()];
        return (diagnostics.clone(), lowered_internal_failure(diagnostics));
    }
    let environment = TypeEnvironment::from_application_module_with_standard(
        application_module,
        selected_standard_module,
        standard,
    );
    check_project_surface_module_with_environment(application_module, environment)
}

pub fn check_project_surface_module_with_standard_modules_environment(
    application_module: &SurfaceModule,
    selected_standard_module_names: &BTreeSet<String>,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    if let Err(failure) = validate_source_less_lookup_registries() {
        let diagnostics = vec![failure.diagnostic()];
        return (diagnostics.clone(), lowered_internal_failure(diagnostics));
    }
    let environment = TypeEnvironment::from_application_module_with_standard_module_names(
        application_module,
        selected_standard_module_names,
        standard,
    );
    check_project_surface_module_with_environment(application_module, environment)
}

pub fn prepare_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    validate_source_less_lookup_registries().expect("source-less lookup registries are valid");
    prepare_reusable_standard_environment(module)
}

pub fn validate_standard_symbol_registry_diagnostic() -> Result<(), Box<Diagnostic>> {
    validate_source_less_lookup_registries().map_err(|failure| Box::new(failure.diagnostic()))
}

pub fn try_prepare_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> Result<ReusableStandardEnvironment, Box<Diagnostic>> {
    validate_standard_symbol_registry_diagnostic()?;
    Ok(prepare_reusable_standard_environment(module))
}

pub fn prepare_current_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    validate_source_less_lookup_registries().expect("source-less lookup registries are valid");
    prepare_current_reusable_standard_environment(module)
}

pub fn try_prepare_current_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> Result<ReusableStandardEnvironment, Box<Diagnostic>> {
    validate_standard_symbol_registry_diagnostic()?;
    Ok(prepare_current_reusable_standard_environment(module))
}

fn check_project_surface_module_with_environment(
    module: &SurfaceModule,
    environment: TypeEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    if let Err(failure) = validate_source_less_lookup_registries() {
        let diagnostics = vec![failure.diagnostic()];
        return (diagnostics.clone(), lowered_internal_failure(diagnostics));
    }
    let validate_standard_bodies = should_validate_standard_bodies(module);
    let semantic_diagnostics =
        analyze_surface_module_with_environment(module, &environment, validate_standard_bodies);
    let checked = lower_analyzed_surface_module_with_environment(
        module,
        semantic_diagnostics.clone(),
        &environment,
        true,
    );
    (semantic_diagnostics, checked)
}

fn analyze_surface_module_with_environment(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    validate_standard_bodies: bool,
) -> Vec<Diagnostic> {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return vec![failure.diagnostic()];
    }
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_module_declarations(module, environment));

    for function in &module.functions {
        if !validate_standard_bodies
            && function
                .module_name
                .as_deref()
                .is_some_and(|module| module.starts_with("std::"))
        {
            continue;
        }
        diagnostics.extend(check_function_declaration_and_body(function, environment));
    }

    diagnostics
}

fn check_module_declarations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_invalid_name_casing(module, environment));
    diagnostics.extend(check_duplicate_function_names(module));
    diagnostics.extend(check_duplicate_type_names(module));
    diagnostics.extend(check_duplicate_effect_names(module));
    diagnostics.extend(check_duplicate_schema_names(module));
    diagnostics.extend(check_duplicate_constructor_names(module));
    diagnostics.extend(check_module_boundary(module));
    diagnostics.extend(check_duplicate_use_aliases(module));
    diagnostics.extend(check_reserved_prelude_aliases(module));
    diagnostics.extend(check_public_aliases(module));
    diagnostics.extend(check_schema_field_primitives(module));
    diagnostics.extend(check_schema_type_references(module));
    diagnostics.extend(check_handler_declarations(module, environment));

    diagnostics
}

fn check_function_declaration_and_body(
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_declared_effect_labels(function, environment));
    if function.visibility == Visibility::Public {
        diagnostics.extend(check_public_function_boundary(function));
    }
    if function.kind == FunctionKind::Test {
        diagnostics.extend(check_test_declaration_boundary(function));
    }
    diagnostics.extend(check_function_body(function, environment));

    diagnostics
}

fn check_invalid_name_casing(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut invalid_names = module
        .invalid_names
        .iter()
        .filter(|invalid| !invalid_name_is_valid_constructor_pattern(invalid, module, environment))
        .filter(|invalid| !invalid_name_repeats_quarantined_import_alias(invalid, module))
        .filter(|invalid| {
            !invalid_type_segment_lacks_constructor_role(invalid, module, environment)
        })
        .filter(|invalid| {
            !invalid_constructor_segment_lacks_constructor_role(invalid, module, environment)
        })
        .filter(|invalid| {
            !invalid_function_segment_lacks_function_role(invalid, module, environment)
        })
        .filter(|invalid| !invalid_value_segment_lacks_value_role(invalid, module, environment))
        .collect::<Vec<_>>();
    invalid_names.sort_by_key(|invalid| (invalid.span.start.offset, invalid.span.end.offset));
    invalid_names
        .into_iter()
        .map(invalid_name_diagnostic)
        .collect()
}

fn invalid_function_segment_lacks_function_role(
    invalid: &InvalidName,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Function
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    module.functions.iter().any(|function| {
        function.body.iter().any(|line| {
            invalid_function_segment_lacks_function_role_in_body_line(
                invalid,
                line,
                function.module_name.as_deref(),
                environment,
            )
        })
    }) || module.handlers.iter().any(|handler| {
        handler.operation_clauses.iter().any(|clause| {
            invalid_function_segment_lacks_function_role_in_expr(
                invalid,
                &clause.body,
                handler.module_name.as_deref(),
                environment,
            )
        })
    })
}

fn invalid_function_segment_lacks_function_role_in_body_line(
    invalid: &InvalidName,
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    match &line.kind {
        veln_ast::BodyLineKind::Let { pattern, expr, .. } => {
            invalid_function_segment_lacks_function_role_in_pattern(
                invalid,
                pattern,
                current_module,
                environment,
            ) || invalid_function_segment_lacks_function_role_in_expr(
                invalid,
                expr,
                current_module,
                environment,
            )
        }
        veln_ast::BodyLineKind::Expr { expr } => {
            invalid_function_segment_lacks_function_role_in_expr(
                invalid,
                expr,
                current_module,
                environment,
            )
        }
    }
}

fn invalid_function_segment_lacks_function_role_in_pattern(
    _invalid: &InvalidName,
    pattern: &veln_ast::Pattern,
    _current_module: Option<&str>,
    _environment: &TypeEnvironment,
) -> bool {
    match &pattern.kind {
        veln_ast::PatternKind::Constructor { args, .. } => args.iter().any(|arg| {
            invalid_function_segment_lacks_function_role_in_pattern(
                _invalid,
                arg,
                _current_module,
                _environment,
            )
        }),
        veln_ast::PatternKind::Record(fields) => fields.iter().any(|field| {
            invalid_function_segment_lacks_function_role_in_pattern(
                _invalid,
                &field.pattern,
                _current_module,
                _environment,
            )
        }),
        _ => false,
    }
}

fn invalid_function_segment_lacks_function_role_in_expr(
    invalid: &InvalidName,
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    invalid_segment_lacks_role_in_expr(
        invalid,
        expr,
        current_module,
        environment,
        invalid_function_segment_lacks_function_role_for_path,
        invalid_function_segment_lacks_function_role_in_pattern,
    )
}

type ExprPathRolePredicate =
    fn(&InvalidName, &[String], &[veln_source::SourceSpan], Option<&str>, &TypeEnvironment) -> bool;

type PatternRolePredicate =
    fn(&InvalidName, &veln_ast::Pattern, Option<&str>, &TypeEnvironment) -> bool;

fn invalid_segment_lacks_role_in_expr(
    invalid: &InvalidName,
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    path_predicate: ExprPathRolePredicate,
    pattern_predicate: PatternRolePredicate,
) -> bool {
    RoleExprScan {
        invalid,
        current_module,
        environment,
        path_predicate,
        pattern_predicate,
    }
    .expr(expr)
}

struct RoleExprScan<'a> {
    invalid: &'a InvalidName,
    current_module: Option<&'a str>,
    environment: &'a TypeEnvironment,
    path_predicate: ExprPathRolePredicate,
    pattern_predicate: PatternRolePredicate,
}

impl RoleExprScan<'_> {
    fn expr(&self, expr: &veln_ast::Expr) -> bool {
        match &expr.kind {
            veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } => (self.path_predicate)(
                self.invalid,
                segments,
                segment_spans,
                self.current_module,
                self.environment,
            ),
            veln_ast::ExprKind::Call { callee, args } => self.call(callee, args),
            veln_ast::ExprKind::TypeApply { callee, .. }
            | veln_ast::ExprKind::FieldAccess { base: callee, .. }
            | veln_ast::ExprKind::Try(callee)
            | veln_ast::ExprKind::Prefix { expr: callee, .. } => self.expr(callee),
            veln_ast::ExprKind::Binary { left, right, .. } => self.binary(left, right),
            veln_ast::ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => self.if_expr(condition, then_branch, else_if_branches, else_branch),
            veln_ast::ExprKind::Record(fields) => {
                self.any_expr(fields.iter().map(|field| &field.expr))
            }
            veln_ast::ExprKind::List(items) | veln_ast::ExprKind::Perform { args: items, .. } => {
                self.any_expr(items.iter())
            }
            veln_ast::ExprKind::Dict(entries) => entries
                .iter()
                .any(|entry| self.binary(&entry.key, &entry.value)),
            veln_ast::ExprKind::Handle { body, args, .. } => {
                self.expr(body) || self.any_expr(args.iter())
            }
            veln_ast::ExprKind::SchemaDecode { input, base, .. } => self.binary(input, base),
            veln_ast::ExprKind::SchemaEncode { value, .. } => self.expr(value),
            veln_ast::ExprKind::Match { scrutinee, arms } => self.match_expr(scrutinee, arms),
            _ => false,
        }
    }

    fn call(&self, callee: &veln_ast::Expr, args: &[veln_ast::Expr]) -> bool {
        self.expr(callee) || self.any_expr(args.iter())
    }

    fn binary(&self, left: &veln_ast::Expr, right: &veln_ast::Expr) -> bool {
        self.expr(left) || self.expr(right)
    }

    fn if_expr(
        &self,
        condition: &veln_ast::Expr,
        then_branch: &veln_ast::Expr,
        else_if_branches: &[veln_ast::IfBranch],
        else_branch: &veln_ast::Expr,
    ) -> bool {
        self.binary(condition, then_branch)
            || else_if_branches
                .iter()
                .any(|branch| self.binary(&branch.condition, &branch.expr))
            || self.expr(else_branch)
    }

    fn match_expr(&self, scrutinee: &veln_ast::Expr, arms: &[veln_ast::MatchArm]) -> bool {
        self.expr(scrutinee)
            || arms.iter().any(|arm| {
                (self.pattern_predicate)(
                    self.invalid,
                    &arm.pattern,
                    self.current_module,
                    self.environment,
                ) || self.expr(&arm.expr)
            })
    }

    fn any_expr<'a>(&self, mut exprs: impl Iterator<Item = &'a veln_ast::Expr>) -> bool {
        exprs.any(|expr| self.expr(expr))
    }
}

fn invalid_function_segment_lacks_function_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    if index + 1 != segments.len() {
        return false;
    }
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
    {
        return false;
    }
    environment
        .function_path(segments, current_module)
        .is_none()
        && environment
            .codec_call_path(segments, current_module)
            .is_empty()
        && !lowercase_corrected_function_path_resolves(
            invalid,
            segments,
            current_module,
            environment,
        )
}

fn lowercase_corrected_function_path_resolves(
    invalid: &InvalidName,
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    environment
        .function_path(&corrected, current_module)
        .is_some()
        || !environment
            .codec_call_path(&corrected, current_module)
            .is_empty()
}

fn invalid_value_segment_lacks_value_role(
    invalid: &InvalidName,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> bool {
    if !matches!(invalid.class, NameClass::Module | NameClass::ValueBinding)
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    module.functions.iter().any(|function| {
        function.body.iter().any(|line| {
            invalid_value_segment_lacks_value_role_in_body_line(
                invalid,
                line,
                function.module_name.as_deref(),
                environment,
            )
        })
    }) || module.handlers.iter().any(|handler| {
        handler.operation_clauses.iter().any(|clause| {
            invalid_value_segment_lacks_value_role_in_expr(
                invalid,
                &clause.body,
                handler.module_name.as_deref(),
                environment,
            )
        })
    })
}

fn invalid_value_segment_lacks_value_role_in_body_line(
    invalid: &InvalidName,
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    match &line.kind {
        veln_ast::BodyLineKind::Let { pattern, expr, .. } => {
            invalid_value_segment_lacks_value_role_in_pattern(
                invalid,
                pattern,
                current_module,
                environment,
            ) || invalid_value_segment_lacks_value_role_in_expr(
                invalid,
                expr,
                current_module,
                environment,
            )
        }
        veln_ast::BodyLineKind::Expr { expr } => invalid_value_segment_lacks_value_role_in_expr(
            invalid,
            expr,
            current_module,
            environment,
        ),
    }
}

fn invalid_value_segment_lacks_value_role_in_pattern(
    _invalid: &InvalidName,
    pattern: &veln_ast::Pattern,
    _current_module: Option<&str>,
    _environment: &TypeEnvironment,
) -> bool {
    match &pattern.kind {
        veln_ast::PatternKind::Constructor { args, .. } => args.iter().any(|arg| {
            invalid_value_segment_lacks_value_role_in_pattern(
                _invalid,
                arg,
                _current_module,
                _environment,
            )
        }),
        veln_ast::PatternKind::Record(fields) => fields.iter().any(|field| {
            invalid_value_segment_lacks_value_role_in_pattern(
                _invalid,
                &field.pattern,
                _current_module,
                _environment,
            )
        }),
        _ => false,
    }
}

fn invalid_value_segment_lacks_value_role_in_expr(
    invalid: &InvalidName,
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    match &expr.kind {
        veln_ast::ExprKind::NamePath {
            segments,
            segment_spans,
        } => invalid_value_segment_lacks_value_role_for_path(
            invalid,
            segments,
            segment_spans,
            current_module,
            environment,
        ),
        veln_ast::ExprKind::Call { callee, args } => {
            invalid_value_segment_lacks_value_role_in_expr(
                invalid,
                callee,
                current_module,
                environment,
            ) || args.iter().any(|arg| {
                invalid_value_segment_lacks_value_role_in_expr(
                    invalid,
                    arg,
                    current_module,
                    environment,
                )
            })
        }
        veln_ast::ExprKind::Record(fields) => fields.iter().any(|field| {
            invalid_value_segment_lacks_value_role_in_expr(
                invalid,
                &field.expr,
                current_module,
                environment,
            )
        }),
        veln_ast::ExprKind::List(items) => items.iter().any(|item| {
            invalid_value_segment_lacks_value_role_in_expr(
                invalid,
                item,
                current_module,
                environment,
            )
        }),
        veln_ast::ExprKind::Match { scrutinee, arms } => {
            invalid_value_segment_lacks_value_role_in_expr(
                invalid,
                scrutinee,
                current_module,
                environment,
            ) || arms.iter().any(|arm| {
                invalid_value_segment_lacks_value_role_in_pattern(
                    invalid,
                    &arm.pattern,
                    current_module,
                    environment,
                ) || invalid_value_segment_lacks_value_role_in_expr(
                    invalid,
                    &arm.expr,
                    current_module,
                    environment,
                )
            })
        }
        _ => false,
    }
}

fn invalid_value_segment_lacks_value_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
    {
        return false;
    }
    if invalid.class == NameClass::Module {
        return type_qualified_constructor_path(invalid, segments, current_module, environment);
    }
    if !path_resolves_as_value(segments, current_module, environment)
        && !lowercase_corrected_value_path_resolves(invalid, segments, current_module, environment)
    {
        return true;
    }
    matches!(
        environment
            .adts
            .nullary_constructor(segments, current_module, &environment.uses),
        crate::adt::registry::ConstructorLookup::Found(_)
            | crate::adt::registry::ConstructorLookup::Ambiguous
    )
}

fn type_qualified_constructor_path(
    invalid: &InvalidName,
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    invalid.segment_index == Some(0)
        && environment
            .adts
            .constructor_candidates(segments, current_module, &environment.uses)
            .iter()
            .any(|constructor| constructor.descriptor.type_name == invalid.name)
}

fn path_resolves_as_value(
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    environment
        .function_path_for_value(segments, current_module)
        .is_some()
        || matches!(
            environment
                .adts
                .nullary_constructor(segments, current_module, &environment.uses),
            crate::adt::registry::ConstructorLookup::Found(_)
                | crate::adt::registry::ConstructorLookup::Ambiguous
        )
}

fn lowercase_corrected_value_path_resolves(
    invalid: &InvalidName,
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    path_resolves_as_value(&corrected, current_module, environment)
}

fn invalid_constructor_segment_lacks_constructor_role(
    invalid: &InvalidName,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> bool {
    if !matches!(invalid.class, NameClass::Constructor | NameClass::Function)
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    module.functions.iter().any(|function| {
        function.body.iter().any(|line| {
            invalid_constructor_segment_lacks_constructor_role_in_body_line(
                invalid,
                line,
                function.module_name.as_deref(),
                environment,
            )
        })
    }) || module.handlers.iter().any(|handler| {
        handler.operation_clauses.iter().any(|clause| {
            invalid_constructor_segment_lacks_constructor_role_in_expr(
                invalid,
                &clause.body,
                handler.module_name.as_deref(),
                environment,
            )
        })
    })
}

fn invalid_constructor_segment_lacks_constructor_role_in_body_line(
    invalid: &InvalidName,
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    match &line.kind {
        veln_ast::BodyLineKind::Let { pattern, expr, .. } => {
            invalid_constructor_segment_lacks_constructor_role_in_pattern(
                invalid,
                pattern,
                current_module,
                environment,
            ) || invalid_constructor_segment_lacks_constructor_role_in_expr(
                invalid,
                expr,
                current_module,
                environment,
            )
        }
        veln_ast::BodyLineKind::Expr { expr } => {
            invalid_constructor_segment_lacks_constructor_role_in_expr(
                invalid,
                expr,
                current_module,
                environment,
            )
        }
    }
}

fn invalid_constructor_segment_lacks_constructor_role_in_pattern(
    _invalid: &InvalidName,
    pattern: &veln_ast::Pattern,
    _current_module: Option<&str>,
    _environment: &TypeEnvironment,
) -> bool {
    match &pattern.kind {
        veln_ast::PatternKind::Constructor { args, .. } => args.iter().any(|arg| {
            invalid_constructor_segment_lacks_constructor_role_in_pattern(
                _invalid,
                arg,
                _current_module,
                _environment,
            )
        }),
        veln_ast::PatternKind::Record(fields) => fields.iter().any(|field| {
            invalid_constructor_segment_lacks_constructor_role_in_pattern(
                _invalid,
                &field.pattern,
                _current_module,
                _environment,
            )
        }),
        _ => false,
    }
}

fn invalid_constructor_segment_lacks_constructor_role_in_expr(
    invalid: &InvalidName,
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    invalid_segment_lacks_role_in_expr(
        invalid,
        expr,
        current_module,
        environment,
        invalid_constructor_segment_lacks_constructor_role_for_path,
        invalid_constructor_segment_lacks_constructor_role_in_pattern,
    )
}

fn invalid_constructor_segment_lacks_constructor_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    if index + 1 != segments.len() {
        return false;
    }
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
    {
        return false;
    }
    match invalid.class {
        NameClass::Constructor => {
            if segments.get(index.saturating_sub(1)).is_none_or(|segment| {
                !segment
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_uppercase)
            }) {
                return true;
            }
            environment
                .function_path(segments, current_module)
                .is_some()
                || !environment
                    .codec_call_path(segments, current_module)
                    .is_empty()
        }
        NameClass::Function => {
            matches!(
                environment
                    .adts
                    .constructor(segments, current_module, &environment.uses),
                crate::adt::registry::ConstructorLookup::Found(_)
                    | crate::adt::registry::ConstructorLookup::Ambiguous
            ) || !environment
                .codec_call_path(segments, current_module)
                .is_empty()
        }
        NameClass::Type | NameClass::Module | NameClass::ValueBinding => false,
    }
}

fn invalid_type_segment_lacks_constructor_role(
    invalid: &InvalidName,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Type
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    module.functions.iter().any(|function| {
        function.body.iter().any(|line| {
            invalid_type_segment_lacks_constructor_role_in_body_line(
                invalid,
                line,
                function.module_name.as_deref(),
                environment,
            )
        })
    }) || module.handlers.iter().any(|handler| {
        handler.operation_clauses.iter().any(|clause| {
            invalid_type_segment_lacks_constructor_role_in_expr(
                invalid,
                &clause.body,
                handler.module_name.as_deref(),
                environment,
            )
        })
    })
}

fn invalid_type_segment_lacks_constructor_role_in_body_line(
    invalid: &InvalidName,
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    match &line.kind {
        veln_ast::BodyLineKind::Let { pattern, expr, .. } => {
            invalid_type_segment_lacks_constructor_role_in_pattern(
                invalid,
                pattern,
                current_module,
                environment,
            ) || invalid_type_segment_lacks_constructor_role_in_expr(
                invalid,
                expr,
                current_module,
                environment,
            )
        }
        veln_ast::BodyLineKind::Expr { expr } => {
            invalid_type_segment_lacks_constructor_role_in_expr(
                invalid,
                expr,
                current_module,
                environment,
            )
        }
    }
}

fn invalid_type_segment_lacks_constructor_role_in_pattern(
    invalid: &InvalidName,
    pattern: &veln_ast::Pattern,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    match &pattern.kind {
        veln_ast::PatternKind::Constructor { name, args } => {
            invalid_type_segment_lacks_constructor_role_for_pattern_path(
                invalid,
                name,
                pattern,
                current_module,
                environment,
            ) || args.iter().any(|arg| {
                invalid_type_segment_lacks_constructor_role_in_pattern(
                    invalid,
                    arg,
                    current_module,
                    environment,
                )
            })
        }
        veln_ast::PatternKind::Record(fields) => fields.iter().any(|field| {
            invalid_type_segment_lacks_constructor_role_in_pattern(
                invalid,
                &field.pattern,
                current_module,
                environment,
            )
        }),
        _ => false,
    }
}

fn invalid_type_segment_lacks_constructor_role_for_pattern_path(
    invalid: &InvalidName,
    segments: &[String],
    pattern: &veln_ast::Pattern,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    if index >= segments.len()
        || segments[index] != invalid.name
        || invalid.span.file != pattern.span.file
        || invalid.span.start.offset < pattern.span.start.offset
        || pattern.span.end.offset < invalid.span.end.offset
    {
        return false;
    }
    if path_resolves_as_constructor(segments, current_module, environment) {
        return true;
    }
    if segments.len() < 3 {
        return true;
    }
    if index + 2 != segments.len() {
        return false;
    }
    let mut corrected = segments.to_vec();
    corrected[index] = uppercase_initial(&invalid.name);
    !path_resolves_as_constructor(&corrected, current_module, environment)
        && environment.quarantined_import_constructor_recovery_candidate_count(
            &corrected,
            current_module,
            None,
        ) != 1
}

fn invalid_type_segment_lacks_constructor_role_in_expr(
    invalid: &InvalidName,
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    invalid_segment_lacks_role_in_expr(
        invalid,
        expr,
        current_module,
        environment,
        invalid_type_segment_lacks_constructor_role_for_path,
        invalid_type_segment_lacks_constructor_role_in_pattern,
    )
}

fn invalid_type_segment_lacks_constructor_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
    {
        return false;
    }
    if path_resolves_as_constructor(segments, current_module, environment) {
        return true;
    }
    if environment
        .function_path(segments, current_module)
        .is_some()
        || !environment
            .codec_call_path(segments, current_module)
            .is_empty()
    {
        return true;
    }
    if segments
        .last()
        .is_some_and(|leaf| leaf.as_bytes().first().is_some_and(u8::is_ascii_lowercase))
    {
        return true;
    }
    if segments.len() < 3 {
        return true;
    }
    if index + 2 != segments.len() {
        return false;
    }
    let mut corrected = segments.to_vec();
    corrected[index] = uppercase_initial(&invalid.name);
    !path_resolves_as_constructor(&corrected, current_module, environment)
        && environment.quarantined_import_constructor_recovery_candidate_count(
            &corrected,
            current_module,
            None,
        ) != 1
}

fn path_resolves_as_constructor(
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    matches!(
        environment
            .adts
            .constructor(segments, current_module, &environment.uses),
        crate::adt::registry::ConstructorLookup::Found(_)
            | crate::adt::registry::ConstructorLookup::Ambiguous
    )
}

fn uppercase_initial(name: &str) -> String {
    let Some((_, first)) = name.char_indices().next() else {
        return String::new();
    };
    let rest = &name[first.len_utf8()..];
    first.to_ascii_uppercase().to_string() + rest
}

fn lowercase_initial(name: &str) -> String {
    let Some((_, first)) = name.char_indices().next() else {
        return String::new();
    };
    let rest = &name[first.len_utf8()..];
    first.to_ascii_lowercase().to_string() + rest
}

fn invalid_name_repeats_quarantined_import_alias(
    invalid: &InvalidName,
    module: &SurfaceModule,
) -> bool {
    if invalid.class != NameClass::Module
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index != Some(0)
    {
        return false;
    }
    if module.uses.iter().any(|use_decl| {
        use_decl.span.file == invalid.span.file
            && use_decl.span.start.offset <= invalid.span.start.offset
            && invalid.span.end.offset <= use_decl.span.end.offset
    }) {
        return false;
    }
    module.uses.iter().any(|use_decl| {
        let alias = use_decl
            .name
            .rsplit("::")
            .next()
            .unwrap_or(use_decl.name.as_str());
        crate::name_recovery::use_decl_has_invalid_module_segment(module, use_decl)
            && use_decl.span.file == invalid.span.file
            && (use_decl.alias == invalid.name || alias == invalid.name)
    })
}

fn invalid_name_is_valid_constructor_pattern(
    invalid: &InvalidName,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::ValueBinding || invalid.occurrence != NameOccurrence::PatternHead
    {
        return false;
    }
    let current_module = invalid.enclosing_function_span.as_ref().and_then(|span| {
        module
            .functions
            .iter()
            .find(|function| &function.span == span)
            .and_then(|function| function.module_name.as_deref())
    });
    matches!(
        environment.adts.constructor(
            std::slice::from_ref(&invalid.name),
            current_module,
            &environment.uses,
        ),
        crate::adt::registry::ConstructorLookup::Found(_)
            | crate::adt::registry::ConstructorLookup::Ambiguous
    )
}

fn invalid_name_diagnostic(invalid: &InvalidName) -> Diagnostic {
    let subject = match invalid.class {
        NameClass::Type => "type name",
        NameClass::Constructor => "constructor name",
        NameClass::Module => "module name",
        NameClass::Function => "function name",
        NameClass::ValueBinding => "binding name",
    };
    let subject = if invalid.occurrence == NameOccurrence::AliasTarget {
        match invalid.class {
            NameClass::Type => "type alias target",
            NameClass::Function => "function alias target",
            NameClass::Constructor | NameClass::Module | NameClass::ValueBinding => subject,
        }
    } else {
        subject
    };
    let required_letter = match invalid.class {
        NameClass::Type | NameClass::Constructor => "uppercase",
        NameClass::Module | NameClass::Function | NameClass::ValueBinding => "lowercase",
    };
    let observed_initial = invalid.name.as_bytes().first().map_or("other", |initial| {
        if initial.is_ascii_uppercase() {
            "ascii_uppercase"
        } else if initial.is_ascii_lowercase() {
            "ascii_lowercase"
        } else if *initial == b'_' {
            "underscore"
        } else {
            "other"
        }
    });
    let mut details = vec![
        ("phase", JsonValue::string("name")),
        ("origin", JsonValue::string("source")),
        ("occurrence", JsonValue::string(invalid.occurrence.as_str())),
        ("name", JsonValue::string(invalid.name.clone())),
        ("name_class", JsonValue::string(invalid.class.as_str())),
        (
            "required_initial",
            JsonValue::string(invalid.class.required_initial()),
        ),
        ("observed_initial", JsonValue::string(observed_initial)),
    ];
    if let Some(index) = invalid.segment_index {
        details.push(("segment_index", JsonValue::Number(index as i64)));
    }
    Diagnostic::new(
        "name.invalid_case",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "{subject} `{}` must start with an ASCII {required_letter} letter",
            invalid.name
        ),
        Some(invalid.span.clone()),
        JsonValue::object(details),
    )
}

pub fn lower_checked_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    lower_analyzed_surface_module(module, analyze_surface_module(module))
}

pub fn lower_project_reachable_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let environment = TypeEnvironment::from_module(module);
    lower_project_reachable_surface_module_with_environment(module, environment)
}

pub fn lower_project_reachable_surface_module_with_standard_environment(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let environment = TypeEnvironment::from_module_with_standard(module, standard);
    lower_project_reachable_surface_module_with_environment(module, environment)
}

pub fn lower_project_reachable_surface_modules_with_standard_environment(
    reachable_module: &SurfaceModule,
    selected_standard_module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let environment = TypeEnvironment::from_application_module_with_standard(
        reachable_module,
        selected_standard_module,
        standard,
    );
    lower_project_reachable_surface_module_with_environment(reachable_module, environment)
}

fn lower_project_reachable_surface_module_with_environment(
    module: &SurfaceModule,
    environment: TypeEnvironment,
) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let diagnostics = analyze_surface_module_with_environment(
        module,
        &environment,
        should_validate_standard_bodies(module),
    );
    lower_analyzed_surface_module_with_environment(module, diagnostics, &environment, false)
}

fn should_validate_standard_bodies(module: &SurfaceModule) -> bool {
    !module.functions.iter().any(|function| {
        !function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    })
}

pub fn lower_analyzed_surface_module(
    module: &SurfaceModule,
    diagnostics: Vec<Diagnostic>,
) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let environment = TypeEnvironment::from_module(module);
    lower_analyzed_surface_module_with_environment(module, diagnostics, &environment, false)
}

fn lowered_internal_failure(diagnostics: Vec<Diagnostic>) -> LoweredSurfaceModule {
    LoweredSurfaceModule {
        diagnostics,
        core: None,
        ir: None,
    }
}

fn lower_analyzed_surface_module_with_environment(
    module: &SurfaceModule,
    mut diagnostics: Vec<Diagnostic>,
    environment: &TypeEnvironment,
    project_check: bool,
) -> LoweredSurfaceModule {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return LoweredSurfaceModule {
            diagnostics,
            core: None,
            ir: None,
        };
    }

    let lowered_core = if project_check {
        lower_project_surface_module_to_core(module, environment)
    } else {
        lower_surface_module_to_core(module, environment)
    };
    diagnostics.extend(lowered_core.diagnostics);
    let ir = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        None
    } else {
        lower_checked_core(&lowered_core.program)
            .ok()
            .map(|mut ir| {
                ir.schema_decoders = schema::ir::schema_decode_specs(module);
                ir
            })
    };

    LoweredSurfaceModule {
        diagnostics,
        core: Some(lowered_core.program),
        ir,
    }
}
