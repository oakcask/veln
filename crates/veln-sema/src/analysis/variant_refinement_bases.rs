use std::collections::{HashMap, HashSet};

use veln_ast::{BodyLine, BodyLineKind, Expr, ExprKind, SurfaceModule, TypePathSegments};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::diagnostics::{span_json, type_details};
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation;
use crate::types::{TypeEnvironment, VariantRefinementBaseFailure};

pub(crate) struct VariantRefinementBaseDiagnostic {
    pub(crate) diagnostic: Diagnostic,
    pub(crate) ordering_scope: SourceSpan,
    pub(crate) union_scope: Option<SourceSpan>,
}

struct VariantUnionBaseFailure {
    expected: Type,
    actual: Type,
    expected_span: SourceSpan,
    actual_span: SourceSpan,
    union_scope: SourceSpan,
}

#[derive(Default)]
struct AnnotationRecovery {
    errors: Vec<(String, String, SourceSpan)>,
    union_base_failures: Vec<VariantUnionBaseFailure>,
}

pub(crate) fn check_variant_refinement_bases(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<VariantRefinementBaseDiagnostic> {
    let mut diagnostics = Vec::new();

    collect_type_annotations(module, environment, &mut diagnostics);
    collect_effect_annotations(module, environment, &mut diagnostics);
    collect_schema_annotations(module, environment, &mut diagnostics);
    collect_function_annotations(module, environment, &mut diagnostics);
    collect_handler_annotations(module, environment, &mut diagnostics);

    diagnostics
}

fn annotation_has_variant_union_base_failure(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
) -> bool {
    if !annotation_contains_union(refinements) {
        return false;
    }
    !analyze_annotation(
        refinements,
        &[],
        current_module,
        environment,
        type_parameters,
    )
    .union_base_failures
    .is_empty()
}

fn annotation_contains_union(refinements: &[veln_ast::VariantRefinementType]) -> bool {
    refinements.iter().any(|refinement| {
        refinement.alternatives.len() > 1
            || refinement.alternatives.iter().any(|alternative| {
                alternative
                    .type_arguments
                    .iter()
                    .any(|argument| annotation_contains_union(&argument.ty_refinements))
            })
    })
}

fn collect_type_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for decl in &module.types {
        for variant in &decl.variants {
            for field in &variant.fields {
                collect_declaration_annotation(
                    &field.ty_refinements,
                    &field.ty_paths,
                    decl.module_name.as_deref(),
                    environment,
                    &decl.params,
                    field.node_id.display("field"),
                    [
                        decl.node_id.display("type"),
                        variant.node_id.display("variant"),
                        field.node_id.display("field"),
                    ],
                    diagnostics,
                );
            }
        }
    }
}

fn collect_effect_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for effect in &module.effects {
        for operation in &effect.operations {
            for param in &operation.params {
                collect_declaration_annotation(
                    &param.ty_refinements,
                    &param.ty_paths,
                    effect.module_name.as_deref(),
                    environment,
                    &[],
                    param.node_id.display("param"),
                    [
                        effect.node_id.display("effect"),
                        operation.node_id.display("operation"),
                        param.node_id.display("param"),
                    ],
                    diagnostics,
                );
            }
            collect_declaration_annotation(
                &operation.return_type_refinements,
                &operation.return_type_paths,
                effect.module_name.as_deref(),
                environment,
                &[],
                operation.node_id.display("operation"),
                [
                    effect.node_id.display("effect"),
                    operation.node_id.display("operation"),
                ],
                diagnostics,
            );
        }
    }
}

fn collect_schema_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for schema in &module.schemas {
        for field in &schema.fields {
            collect_declaration_annotation(
                &field.ty_refinements,
                &field.ty_paths,
                schema.module_name.as_deref(),
                environment,
                &[],
                field.node_id.display("schema-field"),
                [
                    schema.node_id.display("schema"),
                    field.node_id.display("schema-field"),
                ],
                diagnostics,
            );
        }
    }
}

fn collect_function_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for function in &module.functions {
        crate::pipeline::record_variant_refinement_annotation_work(1);
        for param in &function.params {
            crate::pipeline::record_variant_refinement_annotation_work(1);
            collect_annotation(
                &param.ty_refinements,
                &param.ty_paths,
                function.module_name.as_deref(),
                environment,
                &[],
                diagnostics,
            );
        }
        collect_annotation(
            &function.return_type_refinements,
            &function.return_type_paths,
            function.module_name.as_deref(),
            environment,
            &[],
            diagnostics,
        );
        for line in &function.body {
            crate::pipeline::record_variant_refinement_annotation_work(1);
            collect_body_line(
                line,
                function.module_name.as_deref(),
                environment,
                diagnostics,
            );
        }
    }
}

fn collect_handler_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for handler in &module.handlers {
        for param in &handler.params {
            collect_declaration_annotation(
                &param.ty_refinements,
                &param.ty_paths,
                handler.module_name.as_deref(),
                environment,
                &[],
                param.node_id.display("param"),
                [
                    handler.node_id.display("handler"),
                    param.node_id.display("param"),
                ],
                diagnostics,
            );
        }
        for clause in &handler.operation_clauses {
            for param in &clause.params {
                collect_declaration_annotation(
                    &param.ty_refinements,
                    &param.ty_paths,
                    handler.module_name.as_deref(),
                    environment,
                    &[],
                    param.node_id.display("param"),
                    [
                        handler.node_id.display("handler"),
                        clause.node_id.display("clause"),
                        param.node_id.display("param"),
                    ],
                    diagnostics,
                );
            }
            collect_expr(
                &clause.body,
                handler.module_name.as_deref(),
                environment,
                diagnostics,
            );
        }
    }
}

pub(crate) fn annotation_has_invalid_refinement(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
) -> bool {
    annotation_has_base_failure(refinements, paths, current_module, environment)
        || annotation_has_variant_union_base_failure(
            refinements,
            current_module,
            environment,
            type_parameters,
        )
}

fn annotation_has_base_failure(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    paths.iter().any(|path| {
        environment
            .recovered_variant_refinement_base_failure(&path.segments, current_module)
            .is_some()
    }) || refinements.iter().any(|refinement| {
        refinement.alternatives.iter().any(|alternative| {
            let base = alternative.base.segments.join("::");
            environment
                .variant_refinement_base_failure(
                    &base,
                    alternative.type_arguments.len(),
                    current_module,
                )
                .is_some()
                || alternative.type_arguments.iter().any(|argument| {
                    annotation_has_base_failure(
                        &argument.ty_refinements,
                        &argument.ty_paths,
                        current_module,
                        environment,
                    )
                })
        })
    })
}

pub(crate) fn independent_annotation_errors(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> Vec<(String, String, SourceSpan)> {
    analyze_annotation(refinements, paths, current_module, environment, &[]).errors
}

fn analyze_annotation(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
) -> AnnotationRecovery {
    let mut recovery = AnnotationRecovery::default();
    let mut visited_paths = HashMap::new();
    let mut materialized_refinements = HashMap::new();
    collect_independent_refinement_errors(
        refinements,
        paths,
        current_module,
        environment,
        &mut recovery.errors,
        &mut visited_paths,
        &mut recovery.union_base_failures,
        type_parameters,
        &mut materialized_refinements,
    );
    collect_independent_structural_arity_errors(
        paths,
        current_module,
        environment,
        &mut recovery.errors,
        &mut visited_paths,
    );
    recovery
}

fn collect_independent_structural_arity_errors(
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
) {
    for path in paths {
        crate::pipeline::record_variant_refinement_recovery_work(1);
        let path_span = joined_span(&path.segment_spans).resolved_or_generated();
        let occurrence = (
            path_span.file.clone(),
            path_span.start.offset,
            path_span.end.offset,
        );
        if visited_paths.get(&occurrence).copied() == Some(true) {
            continue;
        }
        let name = path.segments.join("::");
        if let Some(error) =
            environment.type_path_arity_error(&name, path.type_argument_count, current_module)
        {
            errors.push((path.written.clone(), error, path.application_span.clone()));
        }
    }
}

type PathOccurrence = (veln_source::SourcePath, usize, usize);

#[allow(clippy::too_many_arguments)]
fn collect_independent_refinement_errors(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    union_base_failures: &mut Vec<VariantUnionBaseFailure>,
    type_parameters: &[String],
    materialized_refinements: &mut HashMap<PathOccurrence, Option<Type>>,
) -> bool {
    let mut annotation_is_invalid = false;
    for path in paths {
        let span = joined_span(&path.segment_spans).resolved_or_generated();
        let occurrence = (span.file, span.start.offset, span.end.offset);
        if let Some(path_is_invalid) = visited_paths.get(&occurrence) {
            annotation_is_invalid |= path_is_invalid;
            continue;
        }
        if path.segments.len() == 1
            && path.type_argument_count == 0
            && type_parameters
                .iter()
                .any(|parameter| parameter == &path.segments[0])
        {
            visited_paths.insert(occurrence, false);
            continue;
        }
        let path_is_invalid = if environment
            .recovered_variant_refinement_base_failure(&path.segments, current_module)
            .is_some()
        {
            true
        } else if let Some(error) = environment.recovered_variant_refinement_annotation_error(
            std::slice::from_ref(path),
            current_module,
        ) {
            errors.push((
                path.segments.join("::"),
                error,
                joined_span(&path.segment_spans),
            ));
            true
        } else {
            false
        };
        visited_paths.insert(occurrence, path_is_invalid);
        annotation_is_invalid |= path_is_invalid;
    }
    for refinement in refinements {
        let mut expected_union_base: Option<(Type, SourceSpan)> = None;
        let mut recovered_refinement = None;
        let mut recovered_variants = HashSet::new();
        let mut refinement_is_invalid = false;
        for alternative in &refinement.alternatives {
            let Some(canonical) = collect_refinement_alternative_type(
                alternative,
                current_module,
                environment,
                errors,
                visited_paths,
                union_base_failures,
                type_parameters,
                materialized_refinements,
            ) else {
                annotation_is_invalid = true;
                refinement_is_invalid = true;
                continue;
            };
            if variant_refinement_base_identity_and_arguments(&canonical).is_none() {
                annotation_is_invalid = true;
                refinement_is_invalid = true;
                continue;
            }
            if let Some((expected, expected_span)) = &expected_union_base {
                crate::pipeline::record_variant_refinement_recovery_work(1);
                if !same_variant_refinement_base(expected, &canonical) {
                    union_base_failures.push(VariantUnionBaseFailure {
                        expected: expected.clone(),
                        actual: canonical,
                        expected_span: expected_span.clone(),
                        actual_span: alternative_span(alternative),
                        union_scope: refinement.span.clone(),
                    });
                    annotation_is_invalid = true;
                    refinement_is_invalid = true;
                } else if let Some(expected) = recovered_refinement.take() {
                    recovered_refinement = merge_refinement_alternative(
                        expected,
                        canonical.clone(),
                        &mut recovered_variants,
                    );
                }
            } else {
                expected_union_base = Some((canonical, alternative_span(alternative)));
                recovered_refinement = expected_union_base.as_ref().map(|(ty, _)| ty.clone());
                if let Some(Type::VariantRefinement { variants, .. }) = &recovered_refinement {
                    recovered_variants.extend(variants.iter().cloned());
                }
            }
        }
        materialized_refinements.insert(
            span_occurrence(&refinement.span),
            (!refinement_is_invalid)
                .then_some(recovered_refinement)
                .flatten(),
        );
    }
    annotation_is_invalid
}

#[allow(clippy::too_many_arguments)]
fn collect_refinement_alternative_type(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    union_base_failures: &mut Vec<VariantUnionBaseFailure>,
    type_parameters: &[String],
    materialized_refinements: &mut HashMap<PathOccurrence, Option<Type>>,
) -> Option<Type> {
    crate::pipeline::record_variant_refinement_recovery_work(1);
    let base = alternative.base.segments.join("::");
    let own_base_failure = environment
        .variant_refinement_base_failure(&base, alternative.type_arguments.len(), current_module)
        .is_some();
    let argument_types = collect_refinement_argument_types(
        &alternative.type_arguments,
        current_module,
        environment,
        errors,
        visited_paths,
        union_base_failures,
        type_parameters,
        materialized_refinements,
    );
    if let Some(error) =
        environment.type_path_arity_error(&base, alternative.type_arguments.len(), current_module)
    {
        let written = format!(
            "{}::{}",
            written_refinement_base(alternative),
            alternative.variant
        );
        crate::pipeline::record_variant_refinement_recovery_materialization(written.len());
        errors.push((written, error, alternative_span(alternative)));
        return None;
    }
    if own_base_failure || argument_types.is_none() {
        return None;
    }
    let argument_types = argument_types.expect("checked recovered argument types");
    let shell = Type::variant_refinement(
        base,
        vec![Type::Unknown; argument_types.len()],
        vec![alternative.variant.clone()],
    );
    if let Some(error) = environment.variant_refinement_annotation_error(&shell, current_module) {
        let written = format!(
            "{}::{}",
            written_refinement_base(alternative),
            alternative.variant
        );
        crate::pipeline::record_variant_refinement_recovery_materialization(written.len());
        errors.push((written, error, alternative_span(alternative)));
        return None;
    }
    crate::pipeline::record_variant_refinement_recovery_materialization(argument_types.len() + 1);
    let canonical = refinement_with_arguments(
        environment.canonicalize_type_annotation(shell, current_module),
        argument_types,
    )?;
    Some(canonical)
}

#[allow(clippy::too_many_arguments)]
fn collect_refinement_argument_types(
    arguments: &[veln_ast::VariantRefinementTypeArgument],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    union_base_failures: &mut Vec<VariantUnionBaseFailure>,
    type_parameters: &[String],
    materialized_refinements: &mut HashMap<PathOccurrence, Option<Type>>,
) -> Option<Vec<Type>> {
    let mut recovered = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let argument_errors_start = errors.len();
        let argument_is_invalid = collect_independent_refinement_errors(
            &argument.ty_refinements,
            &argument.ty_paths,
            current_module,
            environment,
            errors,
            visited_paths,
            union_base_failures,
            type_parameters,
            materialized_refinements,
        );
        if argument_is_invalid || errors.len() != argument_errors_start {
            return None;
        }
        let skeleton = type_argument_skeleton(argument);
        crate::pipeline::record_variant_refinement_recovery_materialization(skeleton.len());
        let ty = match parse_type_annotation(&skeleton) {
            Ok(ty) => ty,
            Err(error) => {
                let written = render_type_argument(argument);
                errors.push((written, error, argument.span.clone()));
                return None;
            }
        };
        if let Some(error) = environment.variant_refinement_arity_error(&ty, current_module) {
            let written = render_type_argument(argument);
            errors.push((written, error, argument.span.clone()));
            return None;
        }
        let mut resolution_parameters = type_parameters.to_vec();
        resolution_parameters.push("Placeholder::Variant".to_string());
        let fully_resolved = environment.type_annotation_is_fully_resolved(
            &ty,
            current_module,
            &resolution_parameters,
        );
        if !fully_resolved {
            return None;
        }
        let mut child_refinements = argument.ty_refinements.iter().map(|refinement| {
            materialized_refinements
                .get(&span_occurrence(&refinement.span))
                .cloned()
                .flatten()
        });
        let mut replacements = Vec::new();
        if !collect_recovery_replacements(&ty, &mut child_refinements, &mut replacements)
            || child_refinements.next().is_some()
        {
            return None;
        }
        let mut replacements = replacements.into_iter();
        let canonical = environment.canonicalize_type_annotation(ty, current_module);
        crate::pipeline::record_variant_refinement_recovery_materialization(type_node_count(
            &canonical,
        ));
        let recovered_argument = replace_recovery_unknowns(canonical, &mut replacements);
        if replacements.next().is_some() {
            return None;
        }
        recovered.push(recovered_argument);
    }
    Some(recovered)
}

fn type_argument_skeleton(argument: &veln_ast::VariantRefinementTypeArgument) -> String {
    let mut text = String::new();
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        text.push_str(fragment);
        if index < argument.ty_refinements.len() {
            text.push_str("Placeholder::Variant");
        }
    }
    text
}

fn collect_recovery_replacements(
    ty: &Type,
    child_refinements: &mut impl Iterator<Item = Option<Type>>,
    replacements: &mut Vec<Type>,
) -> bool {
    match ty {
        Type::Named { name, args, .. } => {
            if args.is_empty() && name == "Placeholder::Variant" {
                let Some(Some(child)) = child_refinements.next() else {
                    return false;
                };
                replacements.push(child);
                return true;
            }
            args.iter()
                .all(|arg| collect_recovery_replacements(arg, child_refinements, replacements))
        }
        Type::VariantRefinement { args, .. } => args
            .iter()
            .all(|arg| collect_recovery_replacements(arg, child_refinements, replacements)),
        Type::Record(fields) => fields
            .iter()
            .all(|(_, ty)| collect_recovery_replacements(ty, child_refinements, replacements)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params
                .iter()
                .all(|ty| collect_recovery_replacements(ty, child_refinements, replacements))
                && variadic.as_deref().is_none_or(|ty| {
                    collect_recovery_replacements(ty, child_refinements, replacements)
                })
                && collect_recovery_replacements(return_type, child_refinements, replacements)
        }
        Type::Unknown => true,
    }
}

fn replace_recovery_unknowns(ty: Type, replacements: &mut impl Iterator<Item = Type>) -> Type {
    match ty {
        Type::Unknown => replacements.next().unwrap_or(Type::Unknown),
        Type::Named {
            name,
            identity,
            args,
        } => Type::Named {
            name,
            identity,
            args: args
                .into_iter()
                .map(|arg| replace_recovery_unknowns(arg, replacements))
                .collect(),
        },
        Type::VariantRefinement {
            name,
            identity,
            args,
            variants,
            unresolved_alternatives,
        } => Type::VariantRefinement {
            name,
            identity,
            args: args
                .into_iter()
                .map(|arg| replace_recovery_unknowns(arg, replacements))
                .collect(),
            variants,
            unresolved_alternatives,
        },
        Type::Record(fields) => Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name, replace_recovery_unknowns(ty, replacements)))
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => Type::Function {
            params: params
                .into_iter()
                .map(|ty| replace_recovery_unknowns(ty, replacements))
                .collect(),
            variadic: variadic.map(|ty| Box::new(replace_recovery_unknowns(*ty, replacements))),
            return_type: Box::new(replace_recovery_unknowns(*return_type, replacements)),
            effects,
        },
    }
}

fn span_occurrence(span: &SourceSpan) -> PathOccurrence {
    (span.file.clone(), span.start.offset, span.end.offset)
}

fn type_node_count(ty: &Type) -> usize {
    match ty {
        Type::Unknown => 1,
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
            1 + args.iter().map(type_node_count).sum::<usize>()
        }
        Type::Record(fields) => {
            1 + fields
                .iter()
                .map(|(_, field)| type_node_count(field))
                .sum::<usize>()
        }
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            1 + params.iter().map(type_node_count).sum::<usize>()
                + variadic.as_deref().map(type_node_count).unwrap_or_default()
                + type_node_count(return_type)
        }
    }
}

fn refinement_with_arguments(ty: Type, arguments: Vec<Type>) -> Option<Type> {
    match ty {
        Type::Named { name, identity, .. } => Some(Type::Named {
            name,
            identity,
            args: arguments,
        }),
        Type::VariantRefinement {
            name,
            identity,
            variants,
            unresolved_alternatives,
            ..
        } => Some(Type::VariantRefinement {
            name,
            identity,
            args: arguments,
            variants,
            unresolved_alternatives,
        }),
        _ => None,
    }
}

fn merge_refinement_alternative(
    expected: Type,
    actual: Type,
    seen_variants: &mut HashSet<String>,
) -> Option<Type> {
    if matches!(expected, Type::Named { .. }) {
        return Some(expected);
    }
    let Type::VariantRefinement {
        name,
        identity,
        args,
        variants,
        unresolved_alternatives,
    } = expected
    else {
        return None;
    };
    let Type::VariantRefinement {
        variants: actual_variants,
        ..
    } = actual
    else {
        return None;
    };
    let mut variants = variants.to_vec();
    for variant in actual_variants.iter() {
        if seen_variants.insert(variant.clone()) {
            variants.push(variant.clone());
        }
    }
    Some(Type::VariantRefinement {
        name,
        identity,
        args,
        variants: variants.into(),
        unresolved_alternatives,
    })
}

fn variant_refinement_base_identity_and_arguments(ty: &Type) -> Option<(&str, &[Type])> {
    match ty {
        Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
            Some((identity, args))
        }
        _ => None,
    }
}

fn same_variant_refinement_base(expected: &Type, actual: &Type) -> bool {
    match (
        variant_refinement_base_identity_and_arguments(expected),
        variant_refinement_base_identity_and_arguments(actual),
    ) {
        (Some((expected_identity, expected_args)), Some((actual_identity, actual_args))) => {
            expected_identity == actual_identity
                && crate::type_relations::invariant_args_match(expected_args, actual_args)
        }
        _ => false,
    }
}

fn alternative_span(alternative: &veln_ast::VariantRefinementAlternative) -> SourceSpan {
    let mut span = joined_span(&alternative.base.segment_spans);
    span.end = alternative.variant_span.end;
    span
}

fn variant_refinement_base_renders(expected: &Type, actual: &Type) -> (String, String) {
    let (
        Type::Named {
            name: expected_name,
            identity: expected_identity,
            args: expected_args,
        }
        | Type::VariantRefinement {
            name: expected_name,
            identity: expected_identity,
            args: expected_args,
            ..
        },
        Type::Named {
            name: actual_name,
            identity: actual_identity,
            args: actual_args,
        }
        | Type::VariantRefinement {
            name: actual_name,
            identity: actual_identity,
            args: actual_args,
            ..
        },
    ) = (expected, actual)
    else {
        unreachable!("successfully instantiated refinement has a named ADT base")
    };
    let (expected_name, actual_name) =
        if expected_name == actual_name && expected_identity != actual_identity {
            (expected_identity.as_str(), actual_identity.as_str())
        } else {
            (expected_name.as_str(), actual_name.as_str())
        };
    let mut expected_arguments = Vec::with_capacity(expected_args.len());
    let mut actual_arguments = Vec::with_capacity(actual_args.len());
    for (index, expected_argument) in expected_args.iter().enumerate() {
        if let Some(actual_argument) = actual_args.get(index) {
            let (expected_render, actual_render) =
                diagnostic_type_renders((expected_argument, actual_argument));
            expected_arguments.push(expected_render);
            actual_arguments.push(actual_render);
        } else {
            expected_arguments.push(expected_argument.render());
        }
    }
    actual_arguments.extend(
        actual_args
            .iter()
            .skip(expected_args.len())
            .map(Type::render),
    );
    let expected_render = if expected_arguments.is_empty() {
        expected_name.to_string()
    } else {
        format!("{expected_name}<{}>", expected_arguments.join(", "))
    };
    let actual_render = if actual_arguments.is_empty() {
        actual_name.to_string()
    } else {
        format!("{actual_name}<{}>", actual_arguments.join(", "))
    };
    (expected_render, actual_render)
}

fn diagnostic_type_renders(expected: (&Type, &Type)) -> (String, String) {
    let (expected, actual) = expected;
    diagnostic_named_type_renders(expected, actual)
        .or_else(|| diagnostic_variant_type_renders(expected, actual))
        .or_else(|| diagnostic_record_type_renders(expected, actual))
        .or_else(|| diagnostic_function_type_renders(expected, actual))
        .unwrap_or_else(|| (expected.render(), actual.render()))
}

fn diagnostic_named_type_renders(expected: &Type, actual: &Type) -> Option<(String, String)> {
    match (expected, actual) {
        (
            Type::Named {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
            },
            Type::Named {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
            },
        ) if expected_args.len() == actual_args.len() => {
            let (expected_name, actual_name) = diagnostic_type_names(
                expected_name,
                expected_identity,
                actual_name,
                actual_identity,
            );
            let (expected_arguments, actual_arguments) =
                diagnostic_type_arguments(expected_args, actual_args);
            if expected_arguments.is_empty() && expected_name == "Unit" && actual_name == "Unit" {
                Some(("()".to_string(), "()".to_string()))
            } else {
                Some((
                    diagnostic_type_application(expected_name, &expected_arguments),
                    diagnostic_type_application(actual_name, &actual_arguments),
                ))
            }
        }
        _ => None,
    }
}

fn diagnostic_variant_type_renders(expected: &Type, actual: &Type) -> Option<(String, String)> {
    match (expected, actual) {
        (
            Type::VariantRefinement {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
                variants: expected_variants,
                ..
            },
            Type::VariantRefinement {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
                variants: actual_variants,
                ..
            },
        ) if expected_args.len() == actual_args.len() => {
            let (expected_name, actual_name) = diagnostic_type_names(
                expected_name,
                expected_identity,
                actual_name,
                actual_identity,
            );
            let (expected_arguments, actual_arguments) =
                diagnostic_type_arguments(expected_args, actual_args);
            let expected_base = diagnostic_type_application(expected_name, &expected_arguments);
            let actual_base = diagnostic_type_application(actual_name, &actual_arguments);
            Some((
                expected_variants
                    .iter()
                    .map(|variant| format!("{expected_base}::{variant}"))
                    .collect::<Vec<_>>()
                    .join(" | "),
                actual_variants
                    .iter()
                    .map(|variant| format!("{actual_base}::{variant}"))
                    .collect::<Vec<_>>()
                    .join(" | "),
            ))
        }
        _ => None,
    }
}

fn diagnostic_record_type_renders(expected: &Type, actual: &Type) -> Option<(String, String)> {
    match (expected, actual) {
        (Type::Record(expected_fields), Type::Record(actual_fields))
            if expected_fields.len() == actual_fields.len()
                && expected_fields
                    .iter()
                    .zip(actual_fields)
                    .all(|((expected_name, _), (actual_name, _))| expected_name == actual_name) =>
        {
            let (expected_fields, actual_fields): (Vec<_>, Vec<_>) = expected_fields
                .iter()
                .zip(actual_fields)
                .map(|((name, expected), (_, actual))| {
                    let (expected, actual) = diagnostic_type_renders((expected, actual));
                    (format!("{name}: {expected}"), format!("{name}: {actual}"))
                })
                .unzip();
            Some((
                format!("{{{}}}", expected_fields.join(", ")),
                format!("{{{}}}", actual_fields.join(", ")),
            ))
        }
        _ => None,
    }
}

fn diagnostic_function_type_renders(expected: &Type, actual: &Type) -> Option<(String, String)> {
    match (expected, actual) {
        (
            Type::Function {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                effects: expected_effects,
            },
            Type::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: actual_effects,
            },
        ) if expected_params.len() == actual_params.len()
            && expected_variadic.is_some() == actual_variadic.is_some() =>
        {
            let (mut expected_params, mut actual_params): (Vec<_>, Vec<_>) = expected_params
                .iter()
                .zip(actual_params)
                .map(diagnostic_type_renders)
                .unzip();
            if let (Some(expected), Some(actual)) = (expected_variadic, actual_variadic) {
                let (expected, actual) = diagnostic_type_renders((expected, actual));
                expected_params.push(format!("...{expected}"));
                actual_params.push(format!("...{actual}"));
            }
            let (expected_return, actual_return) =
                diagnostic_type_renders((expected_return, actual_return));
            let expected_effects = if expected_effects.is_empty() {
                String::new()
            } else {
                format!(" effects [{}]", expected_effects.join(", "))
            };
            let actual_effects = if actual_effects.is_empty() {
                String::new()
            } else {
                format!(" effects [{}]", actual_effects.join(", "))
            };
            Some((
                format!(
                    "fn({}) -> {expected_return}{expected_effects}",
                    expected_params.join(", ")
                ),
                format!(
                    "fn({}) -> {actual_return}{actual_effects}",
                    actual_params.join(", ")
                ),
            ))
        }
        _ => None,
    }
}

fn diagnostic_type_names<'a>(
    expected_name: &'a str,
    expected_identity: &'a str,
    actual_name: &'a str,
    actual_identity: &'a str,
) -> (&'a str, &'a str) {
    if expected_name == actual_name && expected_identity != actual_identity {
        (expected_identity, actual_identity)
    } else {
        (expected_name, actual_name)
    }
}

fn diagnostic_type_arguments(expected: &[Type], actual: &[Type]) -> (Vec<String>, Vec<String>) {
    expected
        .iter()
        .zip(actual)
        .map(diagnostic_type_renders)
        .unzip()
}

fn diagnostic_type_application(name: &str, arguments: &[String]) -> String {
    if arguments.is_empty() {
        name.to_string()
    } else {
        format!("{name}<{}>", arguments.join(", "))
    }
}

fn variant_union_base_diagnostic(
    expected: &Type,
    actual: &Type,
    expected_span: &SourceSpan,
    actual_span: SourceSpan,
    union_scope: SourceSpan,
) -> VariantRefinementBaseDiagnostic {
    let (expected_base_type, actual_base_type) = variant_refinement_base_renders(expected, actual);
    let mut diagnostic = Diagnostic::new(
        "type.variant_union_base",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "variant-refinement base `{actual_base_type}` does not match expected base `{expected_base_type}`"
        ),
        Some(actual_span.clone()),
        JsonValue::object([
            (
                "expected_base_type",
                JsonValue::string(expected_base_type.clone()),
            ),
            ("actual_base_type", JsonValue::string(actual_base_type)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("expected_variant_union_base")),
        (
            "message",
            JsonValue::string(format!(
                "The first successfully instantiated alternative established `{expected_base_type}` as the union base."
            )),
        ),
        ("span", span_json(expected_span)),
    ]));
    VariantRefinementBaseDiagnostic {
        diagnostic,
        ordering_scope: actual_span,
        union_scope: Some(union_scope),
    }
}

fn collect_annotation(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    crate::pipeline::record_variant_refinement_annotation_work(refinements.len() + paths.len() + 1);
    let recovery = if annotation_contains_union(refinements) {
        analyze_annotation(
            refinements,
            &[],
            current_module,
            environment,
            type_parameters,
        )
    } else {
        AnnotationRecovery::default()
    };
    collect_annotation_diagnostics(
        refinements,
        paths,
        current_module,
        environment,
        &recovery,
        diagnostics,
    );
}

fn collect_annotation_diagnostics(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    recovery: &AnnotationRecovery,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    diagnostics.extend(recovery.union_base_failures.iter().map(|failure| {
        variant_union_base_diagnostic(
            &failure.expected,
            &failure.actual,
            &failure.expected_span,
            failure.actual_span.clone(),
            failure.union_scope.clone(),
        )
    }));
    collect_refinements(refinements, current_module, environment, diagnostics);
    for path in paths {
        if let Some(failure) =
            environment.recovered_variant_refinement_base_failure(&path.segments, current_module)
        {
            let base_segments = &path.segments[..path.segments.len() - 1];
            let written_type = base_segments.join("::");
            let span = joined_span(&path.segment_spans[..base_segments.len()]);
            diagnostics.push(base_diagnostic(
                written_type,
                span,
                joined_span(&path.segment_spans),
                None,
                failure,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_declaration_annotation(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
    node_id: String,
    origin_node_ids: impl IntoIterator<Item = String> + Clone,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    crate::pipeline::record_variant_refinement_annotation_work(refinements.len() + paths.len() + 1);
    let has_base_failure =
        annotation_has_base_failure(refinements, paths, current_module, environment);
    let recovery = if has_base_failure || annotation_contains_union(refinements) {
        analyze_annotation(
            refinements,
            paths,
            current_module,
            environment,
            type_parameters,
        )
    } else {
        AnnotationRecovery::default()
    };
    collect_annotation_diagnostics(
        refinements,
        paths,
        current_module,
        environment,
        &recovery,
        diagnostics,
    );
    if !has_base_failure && recovery.union_base_failures.is_empty() {
        return;
    }
    for (annotation, error, span) in recovery.errors {
        let diagnostic = Diagnostic::new(
            "type.invalid_annotation",
            Severity::Error,
            DiagnosticKind::Type,
            format!("invalid type annotation `{annotation}`: {error}"),
            Some(span.clone()),
            type_details(
                node_id.clone(),
                "valid_type",
                annotation,
                "source",
                "source",
                "assignable",
                origin_node_ids.clone(),
            ),
        );
        diagnostics.push(VariantRefinementBaseDiagnostic {
            diagnostic,
            ordering_scope: span,
            union_scope: None,
        });
    }
}

fn collect_refinements(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for refinement in refinements {
        crate::pipeline::record_variant_refinement_annotation_work(1);
        for alternative in &refinement.alternatives {
            crate::pipeline::record_variant_refinement_annotation_work(1);
            let base = alternative.base.segments.join("::");
            if let Some(failure) = environment.variant_refinement_base_failure(
                &base,
                alternative.type_arguments.len(),
                current_module,
            ) {
                diagnostics.push(base_diagnostic(
                    written_refinement_base(alternative),
                    joined_span(&alternative.base.segment_spans),
                    alternative.span.clone(),
                    Some(refinement.span.clone()),
                    failure,
                ));
            }
            for argument in &alternative.type_arguments {
                collect_refinements(
                    &argument.ty_refinements,
                    current_module,
                    environment,
                    diagnostics,
                );
            }
        }
    }
}

fn collect_body_line(
    line: &BodyLine,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    match &line.kind {
        BodyLineKind::Let {
            annotation_structure,
            expr,
            ..
        } => {
            collect_annotation(
                &annotation_structure.variant_refinements,
                &annotation_structure.paths,
                current_module,
                environment,
                &[],
                diagnostics,
            );
            collect_expr(expr, current_module, environment, diagnostics);
        }
        BodyLineKind::Expr { expr } => {
            collect_expr(expr, current_module, environment, diagnostics);
        }
        BodyLineKind::Defer { body, .. } => {
            for line in body {
                collect_body_line(line, current_module, environment, diagnostics);
            }
        }
    }
}

fn collect_expr(
    expr: &Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    if let ExprKind::TypeApply {
        type_arg_paths,
        type_arg_refinements,
        ..
    } = &expr.kind
    {
        for (refinements, paths) in type_arg_refinements.iter().zip(type_arg_paths) {
            collect_declaration_annotation(
                refinements,
                paths,
                current_module,
                environment,
                &[],
                expr.node_id.display("expr"),
                [expr.node_id.display("expr")],
                diagnostics,
            );
        }
    }
    if let ExprKind::Begin { body, .. } = &expr.kind {
        for line in body {
            collect_body_line(line, current_module, environment, diagnostics);
        }
    } else {
        expr.for_each_child(&mut |child| {
            collect_expr(child, current_module, environment, diagnostics)
        });
    }
}

fn base_diagnostic(
    written_type: String,
    span: SourceSpan,
    ordering_scope: SourceSpan,
    union_scope: Option<SourceSpan>,
    failure: VariantRefinementBaseFailure,
) -> VariantRefinementBaseDiagnostic {
    let message = match failure.reason {
        "not_adt" => format!("variant-refinement base `{written_type}` is not a finite ADT"),
        "opaque" => {
            format!("variant-refinement base `{written_type}` is opaque at this annotation")
        }
        _ => unreachable!("closed provider-independent base reason"),
    };
    let mut diagnostic = Diagnostic::new(
        "type.variant_refinement_base",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(span),
        JsonValue::object([
            ("written_type", JsonValue::string(written_type)),
            ("reason", JsonValue::string(failure.reason)),
        ]),
    );
    let mut related = vec![
        ("kind", JsonValue::string(failure.related_kind)),
        ("message", JsonValue::string(failure.related_message)),
        (
            "resolved_identity",
            JsonValue::string(failure.resolved_identity),
        ),
    ];
    if let Some(provider) = failure.provider {
        related.push(("provider", JsonValue::string(provider)));
    }
    if let Some(declaration_span) = failure.declaration_span {
        related.push(("span", span_json(&declaration_span)));
    }
    diagnostic.related.push(JsonValue::object(related));
    VariantRefinementBaseDiagnostic {
        diagnostic,
        ordering_scope,
        union_scope,
    }
}

fn joined_span(spans: &[SourceSpan]) -> SourceSpan {
    let mut span = spans.first().expect("type path has a segment").clone();
    span.end = spans.last().expect("type path has a segment").end;
    span
}

fn written_refinement_base(alternative: &veln_ast::VariantRefinementAlternative) -> String {
    let base = alternative.base.segments.join("::");
    if alternative.type_arguments.is_empty() {
        return base;
    }
    let arguments = alternative
        .type_arguments
        .iter()
        .map(render_type_argument)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{base}<{arguments}>")
}

fn render_type_argument(argument: &veln_ast::VariantRefinementTypeArgument) -> String {
    let mut text = String::new();
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        text.push_str(fragment);
        if let Some(refinement) = argument.ty_refinements.get(index) {
            text.push_str(&render_refinement(refinement));
        }
    }
    text
}

fn render_refinement(refinement: &veln_ast::VariantRefinementType) -> String {
    refinement
        .alternatives
        .iter()
        .map(|alternative| {
            format!(
                "{}::{}",
                written_refinement_base(alternative),
                alternative.variant
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}
