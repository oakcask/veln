use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{
    FunctionKind, InvalidName, NameClass, NameOccurrence, QualifiedPathSegment,
    QualifiedPathSegmentEvidence, SurfaceModule, Visibility,
};
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
use crate::prelude::{qualified_prelude_builtin_signature_with_input, qualified_prelude_signature};
use crate::schema;
use crate::source_less_lookup::validate_source_less_lookup_registries;
use crate::types::{
    ReusableStandardEnvironment, TypeEnvironment, prepare_current_reusable_standard_environment,
    prepare_reusable_standard_environment,
};

type RecoveredQualifiedSegmentPush = fn(
    &[String],
    &[veln_source::SourceSpan],
    Option<&str>,
    &veln_source::SourceSpan,
    &TypeEnvironment,
    &mut Vec<InvalidName>,
);

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
    let mut invalid_names = classified_invalid_names(module, environment);
    invalid_names.sort_by_key(|invalid| (invalid.span.start.offset, invalid.span.end.offset));
    invalid_names.dedup_by(|left, right| {
        left.class == right.class
            && left.occurrence == right.occurrence
            && left.span.file == right.span.file
            && left.span.start.offset == right.span.start.offset
            && left.span.end.offset == right.span.end.offset
    });
    invalid_names.iter().map(invalid_name_diagnostic).collect()
}

fn classified_invalid_names(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<InvalidName> {
    let classified_segments = classified_qualified_path_segments(module, environment);
    let mut invalid_names = module
        .invalid_names
        .iter()
        .filter(|invalid| invalid.occurrence != NameOccurrence::PathSegment)
        .filter(|invalid| !invalid_name_is_valid_constructor_pattern(invalid, module, environment))
        .cloned()
        .collect::<Vec<_>>();
    invalid_names.extend(
        classified_segments
            .into_iter()
            .filter(|segment| !name_satisfies_class(&segment.name, segment.role))
            .map(|segment| {
                let enclosing_function_span = enclosing_function_span_for_segment(module, &segment);
                InvalidName {
                    name: segment.name,
                    class: segment.role,
                    occurrence: segment.occurrence,
                    span: segment.span,
                    enclosing_function_span,
                    segment_index: Some(segment.segment_index),
                }
            }),
    );
    invalid_names
}

fn classified_qualified_path_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    let mut segments = valid_qualified_path_segments(module, environment);
    segments.extend(recovered_qualified_type_segments(module, environment));
    segments.extend(recovered_qualified_module_segments(module, environment));
    segments.extend(recovered_qualified_function_segments(module, environment));
    let classified_keys = segments
        .iter()
        .map(classified_segment_key)
        .collect::<BTreeSet<_>>();
    let occurrence_index = QualifiedPathOccurrenceIndex::new(module);
    segments.extend(
        module
            .invalid_names
            .iter()
            .filter(|invalid| invalid.occurrence == NameOccurrence::PathSegment)
            .filter(|invalid| {
                !invalid_path_segment_is_already_classified(invalid, &classified_keys)
            })
            .filter_map(|invalid| {
                classified_invalid_path_segment(invalid, &occurrence_index, module, environment)
            }),
    );
    segments.sort_by_key(|segment| (segment.span.start.offset, segment.span.end.offset));
    segments.dedup_by(|left, right| {
        left.role == right.role
            && left.occurrence == right.occurrence
            && left.span.file == right.span.file
            && left.span.start.offset == right.span.start.offset
            && left.span.end.offset == right.span.end.offset
    });
    segments
}

fn classified_segment_key(
    segment: &QualifiedPathSegment,
) -> (String, usize, usize, usize, &'static str) {
    (
        segment.span.file.as_str().to_string(),
        segment.span.start.offset,
        segment.span.end.offset,
        segment.segment_index,
        segment.role.as_str(),
    )
}

fn invalid_path_segment_is_already_classified(
    invalid: &InvalidName,
    classified_keys: &BTreeSet<(String, usize, usize, usize, &'static str)>,
) -> bool {
    let Some(segment_index) = invalid.segment_index else {
        return false;
    };
    classified_keys.contains(&(
        invalid.span.file.as_str().to_string(),
        invalid.span.start.offset,
        invalid.span.end.offset,
        segment_index,
        invalid.class.as_str(),
    ))
}

fn valid_qualified_path_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    let mut segments = Vec::new();
    for type_decl in &module.types {
        let current_module = type_decl.module_name.as_deref();
        for variant in &type_decl.variants {
            for field in &variant.fields {
                collect_type_path_segments(
                    &field.ty_paths,
                    current_module,
                    environment,
                    &mut segments,
                );
            }
        }
    }
    for effect in &module.effects {
        let current_module = effect.module_name.as_deref();
        for operation in &effect.operations {
            for param in &operation.params {
                collect_type_path_segments(
                    &param.ty_paths,
                    current_module,
                    environment,
                    &mut segments,
                );
            }
            collect_type_path_segments(
                &operation.return_type_paths,
                current_module,
                environment,
                &mut segments,
            );
        }
    }
    for schema in &module.schemas {
        let current_module = schema.module_name.as_deref();
        for field in &schema.fields {
            collect_type_path_segments(&field.ty_paths, current_module, environment, &mut segments);
        }
    }
    for function in &module.functions {
        let current_module = function.module_name.as_deref();
        collect_type_path_segments(
            &function.return_type_paths,
            current_module,
            environment,
            &mut segments,
        );
        for param in &function.params {
            collect_type_path_segments(&param.ty_paths, current_module, environment, &mut segments);
        }
        for line in &function.body {
            collect_valid_segments_from_body_line(line, current_module, environment, &mut segments);
        }
    }
    for handler in &module.handlers {
        let current_module = handler.module_name.as_deref();
        for param in &handler.params {
            collect_type_path_segments(&param.ty_paths, current_module, environment, &mut segments);
        }
        for clause in &handler.operation_clauses {
            for param in &clause.params {
                collect_type_path_segments(
                    &param.ty_paths,
                    current_module,
                    environment,
                    &mut segments,
                );
            }
            collect_valid_segments_from_expr(
                &clause.body,
                current_module,
                environment,
                &mut segments,
            );
        }
    }
    segments
}

fn collect_type_path_segments(
    paths: &[veln_ast::TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    for path in paths {
        if path.segments.len() < 2 {
            continue;
        }
        let quarantined_import_lacks_leaf = environment
            .quarantined_import_type_path_lacks_visible_leaf(&path.segments, current_module);
        if quarantined_import_lacks_leaf
            && !environment
                .quarantined_import_type_path_uses_nested_alias(&path.segments, current_module)
        {
            continue;
        }
        for index in 0..path.segments.len() {
            if quarantined_import_lacks_leaf && index + 1 == path.segments.len() {
                continue;
            }
            let Some(span) = path.segment_spans.get(index) else {
                continue;
            };
            let role = if index + 1 == path.segments.len() {
                NameClass::Type
            } else {
                NameClass::Module
            };
            output.push(qualified_path_segment_from_parts(
                &path.segments[index],
                role,
                span,
                index,
                QualifiedPathSegmentEvidence::Syntax,
            ));
        }
    }
}

fn collect_valid_segments_from_body_line(
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    match &line.kind {
        veln_ast::BodyLineKind::Let {
            pattern,
            annotation_paths,
            expr,
            ..
        } => {
            collect_valid_segments_from_pattern(pattern, current_module, environment, output);
            collect_type_path_segments(annotation_paths, current_module, environment, output);
            collect_valid_segments_from_expr(expr, current_module, environment, output);
        }
        veln_ast::BodyLineKind::Expr { expr } => {
            collect_valid_segments_from_expr(expr, current_module, environment, output);
        }
    }
}

fn collect_valid_segments_from_expr(
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    match &expr.kind {
        veln_ast::ExprKind::NamePath {
            segments,
            segment_spans,
        } => {
            collect_valid_expr_path_segments(
                segments,
                segment_spans,
                current_module,
                environment,
                output,
            );
        }
        veln_ast::ExprKind::Call { callee, args } => {
            if let veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } = &callee.kind
            {
                collect_valid_call_path_segments(
                    segments,
                    segment_spans,
                    current_module,
                    environment,
                    output,
                );
            } else {
                collect_valid_segments_from_expr(callee, current_module, environment, output);
            }
            for arg in args {
                collect_valid_segments_from_expr(arg, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::TypeApply { callee, .. }
        | veln_ast::ExprKind::FieldAccess { base: callee, .. }
        | veln_ast::ExprKind::Try(callee)
        | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
            collect_valid_segments_from_expr(callee, current_module, environment, output);
        }
        veln_ast::ExprKind::Binary { left, right, .. } => {
            collect_valid_segments_from_expr(left, current_module, environment, output);
            collect_valid_segments_from_expr(right, current_module, environment, output);
        }
        veln_ast::ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_valid_segments_from_expr(condition, current_module, environment, output);
            collect_valid_segments_from_expr(then_branch, current_module, environment, output);
            for branch in else_if_branches {
                collect_valid_segments_from_expr(
                    &branch.condition,
                    current_module,
                    environment,
                    output,
                );
                collect_valid_segments_from_expr(&branch.expr, current_module, environment, output);
            }
            collect_valid_segments_from_expr(else_branch, current_module, environment, output);
        }
        veln_ast::ExprKind::Record(fields) => {
            for field in fields {
                collect_valid_segments_from_expr(&field.expr, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::Dict(entries) => {
            for entry in entries {
                collect_valid_segments_from_expr(&entry.key, current_module, environment, output);
                collect_valid_segments_from_expr(&entry.value, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::List(items) | veln_ast::ExprKind::Perform { args: items, .. } => {
            for item in items {
                collect_valid_segments_from_expr(item, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::Handle { body, args, .. } => {
            collect_valid_segments_from_expr(body, current_module, environment, output);
            for arg in args {
                collect_valid_segments_from_expr(arg, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::SchemaDecode { input, base, .. } => {
            collect_valid_segments_from_expr(input, current_module, environment, output);
            collect_valid_segments_from_expr(base, current_module, environment, output);
        }
        veln_ast::ExprKind::SchemaEncode { value, .. } => {
            collect_valid_segments_from_expr(value, current_module, environment, output);
        }
        veln_ast::ExprKind::Match { scrutinee, arms } => {
            collect_valid_segments_from_expr(scrutinee, current_module, environment, output);
            for arm in arms {
                collect_valid_segments_from_pattern(
                    &arm.pattern,
                    current_module,
                    environment,
                    output,
                );
                collect_valid_segments_from_expr(&arm.expr, current_module, environment, output);
            }
        }
        veln_ast::ExprKind::Missing
        | veln_ast::ExprKind::Hole { .. }
        | veln_ast::ExprKind::StringLiteral(_)
        | veln_ast::ExprKind::IntLiteral(_)
        | veln_ast::ExprKind::FloatLiteral(_)
        | veln_ast::ExprKind::BoolLiteral(_)
        | veln_ast::ExprKind::Unit => {}
    }
}

fn collect_valid_segments_from_pattern(
    pattern: &veln_ast::Pattern,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    match &pattern.kind {
        veln_ast::PatternKind::Constructor {
            name,
            name_spans,
            args,
        } => {
            collect_valid_constructor_path_segments(
                name,
                name_spans,
                current_module,
                environment,
                output,
            );
            for arg in args {
                collect_valid_segments_from_pattern(arg, current_module, environment, output);
            }
        }
        veln_ast::PatternKind::Record(fields) => {
            for field in fields {
                collect_valid_segments_from_pattern(
                    &field.pattern,
                    current_module,
                    environment,
                    output,
                );
            }
        }
        _ => {}
    }
}

fn collect_valid_expr_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    if segments.len() < 2 {
        return;
    }
    if environment
        .function_path_for_value(segments, current_module)
        .is_some()
        || qualified_prelude_signature(segments, None).is_some()
        || qualified_prelude_builtin_signature_with_input(segments, None, None).is_some()
    {
        push_module_prefix_and_leaf(segments, segment_spans, NameClass::ValueBinding, output);
        return;
    }
    if matches!(
        environment
            .adts
            .nullary_constructor(segments, current_module, &environment.uses),
        crate::adt::registry::ConstructorLookup::Found(_)
    ) {
        push_constructor_path_segments(
            segments,
            segment_spans,
            current_module,
            environment,
            output,
        );
    }
}

fn collect_valid_call_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    if segments.len() < 2 {
        return;
    }
    if !environment
        .codec_call_path(segments, current_module)
        .is_empty()
    {
        return;
    }
    if environment
        .function_path(segments, current_module)
        .is_some()
        || qualified_prelude_signature(segments, None).is_some()
        || qualified_prelude_builtin_signature_with_input(segments, None, None).is_some()
    {
        push_module_prefix_and_leaf(segments, segment_spans, NameClass::Function, output);
    } else if path_resolves_as_constructor(segments, current_module, environment) {
        push_constructor_path_segments(
            segments,
            segment_spans,
            current_module,
            environment,
            output,
        );
    }
}

fn collect_valid_constructor_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    if segments.len() < 2 || !path_resolves_as_constructor(segments, current_module, environment) {
        return;
    }
    push_constructor_path_segments(segments, segment_spans, current_module, environment, output);
}

fn push_constructor_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    let type_index = constructor_type_segment_index(segments, current_module, environment);
    for index in 0..segments.len() {
        let Some(span) = segment_spans.get(index) else {
            continue;
        };
        let role = if Some(index) == type_index {
            NameClass::Type
        } else if index + 1 == segments.len() {
            NameClass::Constructor
        } else {
            NameClass::Module
        };
        output.push(qualified_path_segment_from_parts(
            &segments[index],
            role,
            span,
            index,
            QualifiedPathSegmentEvidence::Resolved,
        ));
    }
}

fn constructor_type_segment_index(
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> Option<usize> {
    let crate::adt::registry::ConstructorLookup::Found(constructor) =
        environment
            .adts
            .constructor(segments, current_module, &environment.uses)
    else {
        return None;
    };
    segments[..segments.len().saturating_sub(1)]
        .iter()
        .rposition(|segment| segment == &constructor.descriptor.type_name)
}

fn push_module_prefix_and_leaf(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    leaf_role: NameClass,
    output: &mut Vec<QualifiedPathSegment>,
) {
    for index in 0..segments.len() {
        let Some(span) = segment_spans.get(index) else {
            continue;
        };
        let role = if index + 1 == segments.len() {
            leaf_role
        } else {
            NameClass::Module
        };
        output.push(qualified_path_segment_from_parts(
            &segments[index],
            role,
            span,
            index,
            QualifiedPathSegmentEvidence::Resolved,
        ));
    }
}

fn qualified_path_segment_from_parts(
    name: &str,
    role: NameClass,
    span: &veln_source::SourceSpan,
    segment_index: usize,
    evidence: QualifiedPathSegmentEvidence,
) -> QualifiedPathSegment {
    QualifiedPathSegment {
        name: name.to_string(),
        role,
        occurrence: NameOccurrence::PathSegment,
        span: span.clone(),
        segment_index,
        evidence,
    }
}

pub fn classified_project_qualified_path_segments(
    module: &SurfaceModule,
) -> Vec<QualifiedPathSegment> {
    let environment = TypeEnvironment::from_module(module);
    classified_qualified_path_segments(module, &environment)
}

fn enclosing_function_span_for_segment(
    module: &SurfaceModule,
    segment: &QualifiedPathSegment,
) -> Option<veln_source::SourceSpan> {
    module
        .invalid_names
        .iter()
        .find(|invalid| {
            invalid.occurrence == NameOccurrence::PathSegment
                && invalid.segment_index == Some(segment.segment_index)
                && invalid.span.file == segment.span.file
                && invalid.span.start.offset == segment.span.start.offset
                && invalid.span.end.offset == segment.span.end.offset
        })
        .and_then(|invalid| invalid.enclosing_function_span.clone())
        .or_else(|| function_span_for_segment(module, &segment.span))
}

fn function_span_for_segment(
    module: &SurfaceModule,
    span: &veln_source::SourceSpan,
) -> Option<veln_source::SourceSpan> {
    module
        .functions
        .iter()
        .find(|function| {
            function.span.file == span.file
                && function.span.start.offset <= span.start.offset
                && function.span.end.offset >= span.end.offset
        })
        .map(|function| function.span.clone())
}

fn classified_invalid_path_segment(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Option<QualifiedPathSegment> {
    if invalid_name_repeats_quarantined_import_alias(invalid, module) {
        return None;
    }
    match invalid.class {
        NameClass::Module => {
            if invalid_segment_is_constructor_type_qualifier(invalid, occurrences, environment) {
                return Some(classified_path_segment(
                    invalid,
                    NameClass::Type,
                    QualifiedPathSegmentEvidence::Resolved,
                ));
            }
            if invalid_value_segment_lacks_value_role(invalid, occurrences, environment) {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::Module,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
        NameClass::Type => {
            if invalid_type_segment_has_module_role(invalid, occurrences, environment) {
                return Some(classified_path_segment(
                    invalid,
                    NameClass::Module,
                    QualifiedPathSegmentEvidence::Resolved,
                ));
            }
            if invalid_type_segment_lacks_constructor_role(invalid, occurrences, environment) {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::Type,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
        NameClass::Constructor => {
            if invalid_constructor_segment_has_function_role(invalid, occurrences, environment) {
                return Some(classified_path_segment(
                    invalid,
                    NameClass::Function,
                    QualifiedPathSegmentEvidence::UniqueRecovery,
                ));
            }
            if invalid_constructor_segment_lacks_constructor_role(invalid, occurrences, environment)
            {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::Constructor,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
        NameClass::Function => {
            if invalid_function_segment_lacks_function_role(invalid, occurrences, environment)
                || invalid_constructor_segment_lacks_constructor_role(
                    invalid,
                    occurrences,
                    environment,
                )
            {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::Function,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
        NameClass::ValueBinding => {
            if invalid_value_segment_lacks_value_role(invalid, occurrences, environment) {
                None
            } else {
                Some(classified_path_segment(
                    invalid,
                    NameClass::ValueBinding,
                    QualifiedPathSegmentEvidence::Resolved,
                ))
            }
        }
    }
}

#[derive(Clone)]
struct QualifiedPathOccurrence {
    segments: Vec<String>,
    segment_spans: Vec<veln_source::SourceSpan>,
    current_module: Option<String>,
    call_role: bool,
    pattern_role: bool,
}

#[derive(Default)]
struct QualifiedPathOccurrenceIndex {
    by_segment: BTreeMap<(String, usize, usize, usize), Vec<QualifiedPathOccurrence>>,
}

impl QualifiedPathOccurrenceIndex {
    fn new(module: &SurfaceModule) -> Self {
        let mut index = Self::default();
        for function in &module.functions {
            for line in &function.body {
                index.collect_body_line(line, function.module_name.as_deref());
            }
        }
        for handler in &module.handlers {
            for clause in &handler.operation_clauses {
                index.collect_expr(&clause.body, handler.module_name.as_deref(), false);
            }
        }
        index
    }

    fn occurrences_for(&self, invalid: &InvalidName) -> &[QualifiedPathOccurrence] {
        let Some(segment_index) = invalid.segment_index else {
            return &[];
        };
        self.by_segment
            .get(&(
                invalid.span.file.as_str().to_string(),
                invalid.span.start.offset,
                invalid.span.end.offset,
                segment_index,
            ))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn collect_body_line(&mut self, line: &veln_ast::BodyLine, current_module: Option<&str>) {
        match &line.kind {
            veln_ast::BodyLineKind::Let { pattern, expr, .. } => {
                self.collect_pattern(pattern, current_module);
                self.collect_expr(expr, current_module, false);
            }
            veln_ast::BodyLineKind::Expr { expr } => self.collect_expr(expr, current_module, false),
        }
    }

    fn collect_expr(
        &mut self,
        expr: &veln_ast::Expr,
        current_module: Option<&str>,
        call_role: bool,
    ) {
        match &expr.kind {
            veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } => self.insert(segments, segment_spans, current_module, call_role, false),
            veln_ast::ExprKind::Call { callee, args } => {
                self.collect_expr(callee, current_module, true);
                for arg in args {
                    self.collect_expr(arg, current_module, false);
                }
            }
            veln_ast::ExprKind::TypeApply { callee, .. }
            | veln_ast::ExprKind::FieldAccess { base: callee, .. }
            | veln_ast::ExprKind::Try(callee)
            | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
                self.collect_expr(callee, current_module, call_role);
            }
            veln_ast::ExprKind::Binary { left, right, .. } => {
                self.collect_expr(left, current_module, false);
                self.collect_expr(right, current_module, false);
            }
            veln_ast::ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.collect_expr(condition, current_module, false);
                self.collect_expr(then_branch, current_module, false);
                for branch in else_if_branches {
                    self.collect_expr(&branch.condition, current_module, false);
                    self.collect_expr(&branch.expr, current_module, false);
                }
                self.collect_expr(else_branch, current_module, false);
            }
            veln_ast::ExprKind::Record(fields) => {
                for field in fields {
                    self.collect_expr(&field.expr, current_module, false);
                }
            }
            veln_ast::ExprKind::Dict(entries) => {
                for entry in entries {
                    self.collect_expr(&entry.key, current_module, false);
                    self.collect_expr(&entry.value, current_module, false);
                }
            }
            veln_ast::ExprKind::List(items) | veln_ast::ExprKind::Perform { args: items, .. } => {
                for item in items {
                    self.collect_expr(item, current_module, false);
                }
            }
            veln_ast::ExprKind::Handle { body, args, .. } => {
                self.collect_expr(body, current_module, false);
                for arg in args {
                    self.collect_expr(arg, current_module, false);
                }
            }
            veln_ast::ExprKind::SchemaDecode { input, base, .. } => {
                self.collect_expr(input, current_module, false);
                self.collect_expr(base, current_module, false);
            }
            veln_ast::ExprKind::SchemaEncode { value, .. } => {
                self.collect_expr(value, current_module, false);
            }
            veln_ast::ExprKind::Match { scrutinee, arms } => {
                self.collect_expr(scrutinee, current_module, false);
                for arm in arms {
                    self.collect_pattern(&arm.pattern, current_module);
                    self.collect_expr(&arm.expr, current_module, false);
                }
            }
            _ => {}
        }
    }

    fn collect_pattern(&mut self, pattern: &veln_ast::Pattern, current_module: Option<&str>) {
        match &pattern.kind {
            veln_ast::PatternKind::Constructor {
                name,
                name_spans,
                args,
            } => {
                self.insert(name, name_spans, current_module, false, true);
                for arg in args {
                    self.collect_pattern(arg, current_module);
                }
            }
            veln_ast::PatternKind::Record(fields) => {
                for field in fields {
                    self.collect_pattern(&field.pattern, current_module);
                }
            }
            _ => {}
        }
    }

    fn insert(
        &mut self,
        segments: &[String],
        segment_spans: &[veln_source::SourceSpan],
        current_module: Option<&str>,
        call_role: bool,
        pattern_role: bool,
    ) {
        if segments.len() < 2 {
            return;
        }
        let occurrence = QualifiedPathOccurrence {
            segments: segments.to_vec(),
            segment_spans: segment_spans.to_vec(),
            current_module: current_module.map(str::to_string),
            call_role,
            pattern_role,
        };
        for (index, span) in segment_spans.iter().enumerate() {
            self.by_segment
                .entry((
                    span.file.as_str().to_string(),
                    span.start.offset,
                    span.end.offset,
                    index,
                ))
                .or_default()
                .push(occurrence.clone());
        }
    }
}

fn name_satisfies_class(name: &str, class: NameClass) -> bool {
    match class {
        NameClass::Type | NameClass::Constructor => {
            name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        }
        NameClass::Module | NameClass::Function | NameClass::ValueBinding => {
            name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        }
    }
}

fn classified_path_segment(
    invalid: &InvalidName,
    role: NameClass,
    evidence: QualifiedPathSegmentEvidence,
) -> QualifiedPathSegment {
    QualifiedPathSegment {
        name: invalid.name.clone(),
        role,
        occurrence: invalid.occurrence,
        span: invalid.span.clone(),
        segment_index: invalid
            .segment_index
            .expect("classified path segment has segment index"),
        evidence,
    }
}

fn invalid_segment_is_constructor_type_qualifier(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    occurrences
        .occurrences_for(invalid)
        .iter()
        .any(|occurrence| {
            !occurrence.pattern_role
                && invalid.segment_index.is_some_and(|index| {
                    index + 2 == occurrence.segments.len()
                        && type_qualified_constructor_path(
                            invalid,
                            &occurrence.segments,
                            occurrence.current_module.as_deref(),
                            environment,
                        )
                })
        })
}

fn invalid_value_segment_lacks_value_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if !matches!(invalid.class, NameClass::Module | NameClass::ValueBinding)
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .any(|occurrence| {
            !occurrence.pattern_role
                && invalid_value_segment_lacks_value_role_for_path(
                    invalid,
                    &occurrence.segments,
                    &occurrence.segment_spans,
                    occurrence.current_module.as_deref(),
                    environment,
                )
        })
}

fn recovered_qualified_type_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    let mut invalid = Vec::new();
    for function in &module.functions {
        for line in &function.body {
            collect_recovered_qualified_type_segments_from_body_line(
                line,
                function.module_name.as_deref(),
                &function.span,
                environment,
                &mut invalid,
            );
        }
    }
    for handler in &module.handlers {
        for clause in &handler.operation_clauses {
            collect_recovered_qualified_type_segments_from_expr(
                &clause.body,
                handler.module_name.as_deref(),
                &handler.span,
                environment,
                &mut invalid,
            );
        }
    }
    invalid
        .into_iter()
        .map(|invalid| {
            invalid_name_to_classified_segment(
                &invalid,
                QualifiedPathSegmentEvidence::UniqueRecovery,
            )
        })
        .collect()
}

fn recovered_qualified_module_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    recovered_qualified_segments(module, environment, push_recovered_module_segment)
}

fn recovered_qualified_function_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    let mut invalid = Vec::new();
    for function in &module.functions {
        for line in &function.body {
            collect_recovered_qualified_function_segments_from_body_line(
                line,
                function.module_name.as_deref(),
                &function.span,
                environment,
                &mut invalid,
            );
        }
    }
    for handler in &module.handlers {
        for clause in &handler.operation_clauses {
            collect_recovered_qualified_function_segments_from_expr(
                &clause.body,
                handler.module_name.as_deref(),
                &handler.span,
                environment,
                &mut invalid,
            );
        }
    }
    invalid
        .into_iter()
        .map(|invalid| {
            invalid_name_to_classified_segment(
                &invalid,
                QualifiedPathSegmentEvidence::UniqueRecovery,
            )
        })
        .collect()
}

fn recovered_qualified_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    push: RecoveredQualifiedSegmentPush,
) -> Vec<QualifiedPathSegment> {
    let mut invalid = Vec::new();
    for function in &module.functions {
        for line in &function.body {
            collect_recovered_qualified_segments_from_body_line(
                line,
                function.module_name.as_deref(),
                &function.span,
                environment,
                push,
                &mut invalid,
            );
        }
    }
    for handler in &module.handlers {
        for clause in &handler.operation_clauses {
            collect_recovered_qualified_segments_from_expr(
                &clause.body,
                handler.module_name.as_deref(),
                &handler.span,
                environment,
                push,
                &mut invalid,
            );
        }
    }
    invalid
        .into_iter()
        .map(|invalid| {
            invalid_name_to_classified_segment(
                &invalid,
                QualifiedPathSegmentEvidence::UniqueRecovery,
            )
        })
        .collect()
}

fn invalid_name_to_classified_segment(
    invalid: &InvalidName,
    evidence: QualifiedPathSegmentEvidence,
) -> QualifiedPathSegment {
    QualifiedPathSegment {
        name: invalid.name.clone(),
        role: invalid.class,
        occurrence: invalid.occurrence,
        span: invalid.span.clone(),
        segment_index: invalid
            .segment_index
            .expect("classified path segment has segment index"),
        evidence,
    }
}

fn collect_recovered_qualified_segments_from_body_line(
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    push: RecoveredQualifiedSegmentPush,
    invalid: &mut Vec<InvalidName>,
) {
    match &line.kind {
        veln_ast::BodyLineKind::Let { expr, .. } | veln_ast::BodyLineKind::Expr { expr } => {
            collect_recovered_qualified_segments_from_expr(
                expr,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
        }
    }
}

fn collect_recovered_qualified_segments_from_expr(
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    push: RecoveredQualifiedSegmentPush,
    invalid: &mut Vec<InvalidName>,
) {
    match &expr.kind {
        veln_ast::ExprKind::NamePath {
            segments,
            segment_spans,
        } => push(
            segments,
            segment_spans,
            current_module,
            enclosing_function_span,
            environment,
            invalid,
        ),
        veln_ast::ExprKind::Call { callee, args } => {
            collect_recovered_qualified_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
            for arg in args {
                collect_recovered_qualified_segments_from_expr(
                    arg,
                    current_module,
                    enclosing_function_span,
                    environment,
                    push,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::TypeApply { callee, .. }
        | veln_ast::ExprKind::FieldAccess { base: callee, .. }
        | veln_ast::ExprKind::Try(callee)
        | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
            collect_recovered_qualified_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
        }
        veln_ast::ExprKind::Binary { left, right, .. } => {
            collect_recovered_qualified_segments_from_expr(
                left,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
            collect_recovered_qualified_segments_from_expr(
                right,
                current_module,
                enclosing_function_span,
                environment,
                push,
                invalid,
            );
        }
        _ => {}
    }
}

fn collect_recovered_qualified_function_segments_from_body_line(
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    match &line.kind {
        veln_ast::BodyLineKind::Let { expr, .. } | veln_ast::BodyLineKind::Expr { expr } => {
            collect_recovered_qualified_function_segments_from_expr(
                expr,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
    }
}

fn collect_recovered_qualified_function_segments_from_expr(
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    match &expr.kind {
        veln_ast::ExprKind::Call { callee, args } => {
            if let veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } = &callee.kind
            {
                push_recovered_function_segment(
                    segments,
                    segment_spans,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
            collect_recovered_qualified_function_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for arg in args {
                collect_recovered_qualified_function_segments_from_expr(
                    arg,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::TypeApply { callee, .. }
        | veln_ast::ExprKind::FieldAccess { base: callee, .. }
        | veln_ast::ExprKind::Try(callee)
        | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
            collect_recovered_qualified_function_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Binary { left, right, .. } => {
            collect_recovered_qualified_function_segments_from_expr(
                left,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            collect_recovered_qualified_function_segments_from_expr(
                right,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Record(fields) => {
            for field in fields {
                collect_recovered_qualified_function_segments_from_expr(
                    &field.expr,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::List(items) | veln_ast::ExprKind::Perform { args: items, .. } => {
            for item in items {
                collect_recovered_qualified_function_segments_from_expr(
                    item,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        _ => {}
    }
}

fn collect_recovered_qualified_type_segments_from_body_line(
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    match &line.kind {
        veln_ast::BodyLineKind::Let { expr, .. } | veln_ast::BodyLineKind::Expr { expr } => {
            collect_recovered_qualified_type_segments_from_expr(
                expr,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
    }
}

fn collect_recovered_qualified_type_segments_from_expr(
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    match &expr.kind {
        veln_ast::ExprKind::Call { callee, args } => {
            if let veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } = &callee.kind
            {
                push_recovered_qualified_type_segment(
                    segments,
                    segment_spans,
                    Some(args.len()),
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
            collect_recovered_qualified_type_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for arg in args {
                collect_recovered_qualified_type_segments_from_expr(
                    arg,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::NamePath {
            segments,
            segment_spans,
        } => {
            push_recovered_qualified_type_segment(
                segments,
                segment_spans,
                None,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::TypeApply { callee, .. }
        | veln_ast::ExprKind::FieldAccess { base: callee, .. }
        | veln_ast::ExprKind::Try(callee)
        | veln_ast::ExprKind::Prefix { expr: callee, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                callee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Binary { left, right, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                left,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            collect_recovered_qualified_type_segments_from_expr(
                right,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_recovered_qualified_type_segments_from_expr(
                condition,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            collect_recovered_qualified_type_segments_from_expr(
                then_branch,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for branch in else_if_branches {
                collect_recovered_qualified_type_segments_from_expr(
                    &branch.condition,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
                collect_recovered_qualified_type_segments_from_expr(
                    &branch.expr,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
            collect_recovered_qualified_type_segments_from_expr(
                else_branch,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Record(fields) => {
            for field in fields {
                collect_recovered_qualified_type_segments_from_expr(
                    &field.expr,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::Dict(entries) => {
            for entry in entries {
                collect_recovered_qualified_type_segments_from_expr(
                    &entry.key,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
                collect_recovered_qualified_type_segments_from_expr(
                    &entry.value,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::List(items) | veln_ast::ExprKind::Perform { args: items, .. } => {
            for item in items {
                collect_recovered_qualified_type_segments_from_expr(
                    item,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::Handle { body, args, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                body,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for arg in args {
                collect_recovered_qualified_type_segments_from_expr(
                    arg,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::SchemaDecode { input, base, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                input,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            collect_recovered_qualified_type_segments_from_expr(
                base,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::SchemaEncode { value, .. } => {
            collect_recovered_qualified_type_segments_from_expr(
                value,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
        }
        veln_ast::ExprKind::Match { scrutinee, arms } => {
            collect_recovered_qualified_type_segments_from_expr(
                scrutinee,
                current_module,
                enclosing_function_span,
                environment,
                invalid,
            );
            for arm in arms {
                collect_recovered_qualified_type_segments_from_expr(
                    &arm.expr,
                    current_module,
                    enclosing_function_span,
                    environment,
                    invalid,
                );
            }
        }
        veln_ast::ExprKind::Missing
        | veln_ast::ExprKind::Hole { .. }
        | veln_ast::ExprKind::StringLiteral(_)
        | veln_ast::ExprKind::IntLiteral(_)
        | veln_ast::ExprKind::FloatLiteral(_)
        | veln_ast::ExprKind::BoolLiteral(_)
        | veln_ast::ExprKind::Unit => {}
    }
}

fn push_recovered_qualified_type_segment(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    arg_count: Option<usize>,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    if segments.len() < 2 {
        return;
    }
    let type_index = segments.len() - 2;
    let type_name = &segments[type_index];
    if type_name
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_uppercase)
    {
        return;
    }
    let Some(span) = segment_spans.get(type_index) else {
        return;
    };
    let mut corrected = segments.to_vec();
    corrected[type_index] = uppercase_initial(type_name);
    let recovered = match arg_count {
        Some(arg_count) => {
            matches!(
                environment
                    .adts
                    .constructor(&corrected, current_module, &environment.uses),
                crate::adt::registry::ConstructorLookup::Found(_)
            ) || environment.quarantined_import_constructor_recovery_candidate_count(
                &corrected,
                current_module,
                Some(arg_count),
            ) == 1
        }
        None => matches!(
            environment
                .adts
                .nullary_constructor(&corrected, current_module, &environment.uses),
            crate::adt::registry::ConstructorLookup::Found(_)
        ),
    };
    if !recovered {
        return;
    }
    invalid.push(InvalidName {
        name: type_name.clone(),
        class: NameClass::Type,
        occurrence: NameOccurrence::PathSegment,
        span: span.clone(),
        enclosing_function_span: Some(enclosing_function_span.clone()),
        segment_index: Some(type_index),
    });
}

fn push_recovered_module_segment(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    if segments.len() < 2 {
        return;
    }
    for index in 0..segments.len() - 1 {
        let name = &segments[index];
        if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
            continue;
        }
        let Some(span) = segment_spans.get(index) else {
            continue;
        };
        let mut corrected = segments.to_vec();
        corrected[index] = lowercase_initial(name);
        let probe = InvalidName {
            name: name.clone(),
            class: NameClass::Module,
            occurrence: NameOccurrence::PathSegment,
            span: span.clone(),
            enclosing_function_span: Some(enclosing_function_span.clone()),
            segment_index: Some(index),
        };
        if index + 2 == corrected.len()
            && path_resolves_as_constructor(&corrected, current_module, environment)
        {
            continue;
        }
        if module_segment_role_is_fixed(&probe, &corrected, current_module, environment) {
            invalid.push(probe);
        }
    }
}

fn push_recovered_function_segment(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    if segments.len() < 2 {
        return;
    }
    let index = segments.len() - 1;
    let name = &segments[index];
    if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
        return;
    }
    let Some(span) = segment_spans.get(index) else {
        return;
    };
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(name);
    if !environment
        .codec_call_path(segments, current_module)
        .is_empty()
    {
        return;
    }
    if environment
        .function_path(&corrected, current_module)
        .is_none()
    {
        return;
    }
    invalid.push(InvalidName {
        name: name.clone(),
        class: NameClass::Function,
        occurrence: NameOccurrence::PathSegment,
        span: span.clone(),
        enclosing_function_span: Some(enclosing_function_span.clone()),
        segment_index: Some(index),
    });
}

fn invalid_function_segment_lacks_function_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Function
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role)
        .any(|occurrence| {
            invalid_function_segment_lacks_function_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
            )
        })
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
        return !module_segment_role_is_fixed(invalid, segments, current_module, environment);
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

fn module_segment_role_is_fixed(
    invalid: &InvalidName,
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    if index + 1 >= segments.len() {
        return false;
    }
    if type_qualified_constructor_path(invalid, segments, current_module, environment)
        || path_resolves_as_value(segments, current_module, environment)
        || environment
            .function_path(segments, current_module)
            .is_some()
        || !environment
            .codec_call_path(segments, current_module)
            .is_empty()
        || environment.quarantined_import_value_recovery_candidate_count(segments, current_module)
            == 1
        || environment.quarantined_import_constructor_recovery_candidate_count(
            segments,
            current_module,
            None,
        ) == 1
        || qualified_prelude_signature(segments, None).is_some()
        || qualified_prelude_builtin_signature_with_input(segments, None, None).is_some()
    {
        return true;
    }
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    path_resolves_as_value(&corrected, current_module, environment)
        || matches!(
            environment
                .adts
                .constructor(&corrected, current_module, &environment.uses),
            crate::adt::registry::ConstructorLookup::Found(_)
        )
        || environment
            .function_path(&corrected, current_module)
            .is_some()
        || !environment
            .codec_call_path(&corrected, current_module)
            .is_empty()
        || environment.quarantined_import_value_recovery_candidate_count(&corrected, current_module)
            == 1
        || environment.quarantined_import_constructor_recovery_candidate_count(
            &corrected,
            current_module,
            None,
        ) == 1
        || qualified_prelude_signature(&corrected, None).is_some()
        || qualified_prelude_builtin_signature_with_input(&corrected, None, None).is_some()
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
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if !matches!(invalid.class, NameClass::Constructor | NameClass::Function)
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role || occurrence.pattern_role)
        .any(|occurrence| {
            invalid_constructor_segment_lacks_constructor_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
                occurrence.pattern_role,
            )
        })
}

fn invalid_constructor_segment_has_function_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Constructor
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role)
        .any(|occurrence| {
            invalid_constructor_segment_has_function_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
            )
        })
}

fn invalid_constructor_segment_has_function_role_for_path(
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
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    for segment in corrected.iter_mut().take(index) {
        if segment
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_uppercase)
        {
            *segment = lowercase_initial(segment);
        }
    }
    environment
        .function_path(&corrected, current_module)
        .is_some()
        || !environment
            .codec_call_path(&corrected, current_module)
            .is_empty()
}

fn invalid_constructor_segment_lacks_constructor_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    pattern_role: bool,
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
            if !pattern_role
                && segments.get(index.saturating_sub(1)).is_none_or(|segment| {
                    !segment
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_uppercase)
                })
            {
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
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Type
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role || occurrence.pattern_role)
        .any(|occurrence| {
            invalid_type_segment_lacks_constructor_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
            )
        })
}

fn invalid_type_segment_has_module_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Type
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role || occurrence.pattern_role)
        .any(|occurrence| {
            invalid_type_segment_has_module_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
            )
        })
}

fn invalid_type_segment_has_module_role_for_path(
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
        || index + 1 >= segments.len()
    {
        return false;
    }
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    if index + 2 == corrected.len()
        && path_resolves_as_constructor(&corrected, current_module, environment)
    {
        return false;
    }
    module_segment_role_is_fixed(invalid, &corrected, current_module, environment)
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
