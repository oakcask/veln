use std::{collections::HashSet, sync::Arc};

use veln_ast::{
    Function, Param, TypeDecl, TypePathSegments, TypeVariantDecl, TypeVariantField,
    VariantRefinementType, Visibility,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::adt::descriptors::{AdtDescriptor, AdtVariantDescriptor};
use crate::adt::registry::TypeAliasExposurePath;
use crate::diagnostics::span_json;
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation;
use crate::types::TypeEnvironment;

pub(crate) fn check_public_variant_refinement_visibility(
    function: &Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for param in &function.params {
        collect_root_exposures(
            RootAnnotation::new(
                param.ty_span.as_ref(),
                param.ty.as_deref(),
                &param.ty_refinements,
                &param.ty_paths,
            ),
            PublicBoundary::Parameter { function, param },
            function.module_name.as_deref(),
            environment,
            &mut diagnostics,
        );
    }
    collect_root_exposures(
        RootAnnotation::new(
            function.return_type_span.as_ref(),
            function.return_type.as_deref(),
            &function.return_type_refinements,
            &function.return_type_paths,
        ),
        PublicBoundary::Result {
            function,
            span: function.return_type_span.as_ref(),
        },
        function.module_name.as_deref(),
        environment,
        &mut diagnostics,
    );
    diagnostics
}

pub(crate) fn check_public_adt_variant_refinement_visibility(
    declaration: &TypeDecl,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    if declaration.visibility != Visibility::Public {
        return Vec::new();
    }
    let mut diagnostics = Vec::new();
    for variant in declaration
        .variants
        .iter()
        .filter(|variant| variant.visibility == Visibility::Public)
    {
        for field in &variant.fields {
            collect_root_exposures(
                RootAnnotation::new(
                    Some(&field.ty_span),
                    Some(&field.ty),
                    &field.ty_refinements,
                    &field.ty_paths,
                ),
                PublicBoundary::AdtPayload {
                    declaration,
                    variant,
                    field,
                },
                declaration.module_name.as_deref(),
                environment,
                &mut diagnostics,
            );
        }
    }
    diagnostics
}

pub(crate) fn adt_has_private_variant_refinement_exposure(
    declaration: &TypeDecl,
    environment: &TypeEnvironment,
) -> bool {
    declaration.visibility == Visibility::Public
        && declaration
            .variants
            .iter()
            .filter(|variant| variant.visibility == Visibility::Public)
            .flat_map(|variant| &variant.fields)
            .any(|field| {
                root_exposures(
                    RootAnnotation::new(
                        Some(&field.ty_span),
                        Some(&field.ty),
                        &field.ty_refinements,
                        &field.ty_paths,
                    ),
                    declaration.module_name.as_deref(),
                    environment,
                )
                .next()
                .is_some()
            })
}

pub(crate) fn function_has_private_variant_refinement_exposure(
    function: &Function,
    environment: &TypeEnvironment,
) -> bool {
    function.visibility == Visibility::Public
        && (function.params.iter().any(|param| {
            root_exposures(
                RootAnnotation::new(
                    param.ty_span.as_ref(),
                    param.ty.as_deref(),
                    &param.ty_refinements,
                    &param.ty_paths,
                ),
                function.module_name.as_deref(),
                environment,
            )
            .next()
            .is_some()
        }) || root_exposures(
            RootAnnotation::new(
                function.return_type_span.as_ref(),
                function.return_type.as_deref(),
                &function.return_type_refinements,
                &function.return_type_paths,
            ),
            function.module_name.as_deref(),
            environment,
        )
        .next()
        .is_some())
}

fn collect_root_exposures(
    annotation: RootAnnotation<'_>,
    boundary: PublicBoundary<'_>,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for exposure in root_exposures(annotation, current_module, environment) {
        diagnostics.push(exposure.diagnostic(boundary));
    }
}

fn root_exposures<'a>(
    annotation: RootAnnotation<'a>,
    current_module: Option<&'a str>,
    environment: &'a TypeEnvironment,
) -> std::vec::IntoIter<PrivateExposure<'a>> {
    let Some(annotation_span) = annotation.span else {
        return Vec::new().into_iter();
    };
    let mut exposures = root_exposure_candidates(
        annotation_span,
        annotation.text,
        annotation.refinements,
        annotation.paths,
    )
    .into_iter()
    .filter_map(|candidate| private_exposure(candidate, current_module, environment))
    .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    exposures.retain(|exposure| {
        record_visibility_work(1);
        let mut structural_path = exposure.structural_path.clone();
        if matches!(
            structural_path.last(),
            Some(StructuralExposureSegment::UnionAlternative(_))
        ) {
            structural_path.pop();
        }
        seen.insert((
            exposure
                .record_path
                .as_ref()
                .map(RecordFieldPathRef::node_identity),
            structural_path,
            exposure.descriptor.identity(),
            exposure.variant.name.clone(),
            exposure.alternative_identity.clone(),
            exposure
                .alias_declarations
                .as_ref()
                .map(TypeAliasExposurePath::outer_identity)
                .map(str::to_string),
        ))
    });
    exposures.into_iter()
}

#[derive(Clone, Copy)]
struct RootAnnotation<'a> {
    span: Option<&'a SourceSpan>,
    text: Option<&'a str>,
    refinements: &'a [VariantRefinementType],
    paths: &'a [TypePathSegments],
}

impl<'a> RootAnnotation<'a> {
    fn new(
        span: Option<&'a SourceSpan>,
        text: Option<&'a str>,
        refinements: &'a [VariantRefinementType],
        paths: &'a [TypePathSegments],
    ) -> Self {
        Self {
            span,
            text,
            refinements,
            paths,
        }
    }
}

fn root_exposure_candidates(
    annotation_span: &SourceSpan,
    annotation_text: Option<&str>,
    refinements: &[VariantRefinementType],
    paths: &[TypePathSegments],
) -> Vec<RefinementExposureCandidate> {
    let Some(annotation_text) = annotation_text else {
        return Vec::new();
    };
    record_visibility_root_classification(annotation_text.len());
    if annotation_text.trim_start().starts_with('{')
        && !annotation_text.contains('<')
        && !annotation_text.contains("fn")
    {
        return record_exposure_candidates(refinements);
    }
    let Ok(ty) = parse_type_annotation(annotation_text) else {
        let recovered = recovered_root_exposure_candidates(
            annotation_span,
            annotation_text,
            refinements,
            paths,
        );
        if !recovered.is_empty() {
            return recovered;
        }
        return if annotation_text.trim_start().starts_with('{') {
            record_exposure_candidates(refinements)
        } else {
            Vec::new()
        };
    };
    let candidates = structured_exposure_candidates(annotation_span, &ty, refinements, paths);
    if candidates.is_empty() {
        recovered_root_exposure_candidates(annotation_span, annotation_text, refinements, paths)
    } else {
        candidates
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum StructuralExposureSegment {
    TypeArgument(usize),
    FunctionParameter(usize),
    FunctionResult,
    RecordField(String),
    UnionAlternative(usize),
}

#[derive(Clone, Copy)]
struct ExposureTraversalContext<'a> {
    annotation_span: &'a SourceSpan,
    placeholders: bool,
    enclosing_structural_span: Option<&'a SourceSpan>,
}

fn structured_exposure_candidates(
    annotation_span: &SourceSpan,
    ty: &Type,
    refinements: &[VariantRefinementType],
    paths: &[TypePathSegments],
) -> Vec<RefinementExposureCandidate> {
    let mut candidates = Vec::new();
    let mut refinements = refinements.iter().peekable();
    collect_type_exposures(
        ExposureTraversalContext {
            annotation_span,
            placeholders: false,
            enclosing_structural_span: None,
        },
        ty,
        &mut refinements,
        &[],
        &mut candidates,
    );
    candidates.sort_by_key(|candidate| candidate.occurrence_span.start.offset);
    assign_outer_structural_spans(annotation_span, paths, &mut candidates);
    candidates
}

fn collect_type_exposures<'a>(
    context: ExposureTraversalContext<'_>,
    ty: &Type,
    refinements: &mut std::iter::Peekable<std::slice::Iter<'a, VariantRefinementType>>,
    path: &[StructuralExposureSegment],
    candidates: &mut Vec<RefinementExposureCandidate>,
) {
    match ty {
        Type::Named { name, args, .. } => {
            collect_named_type_exposures(context, name, args, refinements, path, candidates);
        }
        Type::Record(fields) => {
            for (name, field) in fields {
                collect_nested_type_exposures(
                    context,
                    field,
                    refinements,
                    path,
                    StructuralExposureSegment::RecordField(name.clone()),
                    candidates,
                );
            }
        }
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            collect_function_type_exposures(
                context,
                params,
                variadic.as_deref(),
                return_type,
                refinements,
                path,
                candidates,
            );
        }
        Type::VariantRefinement { .. } => {
            let Some(refinement) = refinements.next() else {
                return;
            };
            collect_refinement_exposures(context, refinement, path, candidates);
        }
        Type::Unknown => {}
    }
}

fn collect_named_type_exposures<'a>(
    context: ExposureTraversalContext<'_>,
    name: &str,
    args: &[Type],
    refinements: &mut std::iter::Peekable<std::slice::Iter<'a, VariantRefinementType>>,
    path: &[StructuralExposureSegment],
    candidates: &mut Vec<RefinementExposureCandidate>,
) {
    let is_refinement = refinements.peek().is_some_and(|refinement| {
        context.placeholders && name == "Placeholder::Variant"
            || args.is_empty() && plain_refinement_name(refinement).as_deref() == Some(name)
    });
    if is_refinement {
        let refinement = refinements.next().expect("peeked refinement");
        collect_refinement_exposures(context, refinement, path, candidates);
        return;
    }
    for (index, argument) in args.iter().enumerate() {
        collect_nested_type_exposures(
            context,
            argument,
            refinements,
            path,
            StructuralExposureSegment::TypeArgument(index),
            candidates,
        );
    }
}

fn collect_function_type_exposures<'a>(
    context: ExposureTraversalContext<'_>,
    params: &[Type],
    variadic: Option<&Type>,
    return_type: &Type,
    refinements: &mut std::iter::Peekable<std::slice::Iter<'a, VariantRefinementType>>,
    path: &[StructuralExposureSegment],
    candidates: &mut Vec<RefinementExposureCandidate>,
) {
    for (index, parameter) in params.iter().enumerate() {
        collect_nested_type_exposures(
            context,
            parameter,
            refinements,
            path,
            StructuralExposureSegment::FunctionParameter(index),
            candidates,
        );
    }
    if let Some(variadic) = variadic {
        collect_nested_type_exposures(
            context,
            variadic,
            refinements,
            path,
            StructuralExposureSegment::FunctionParameter(params.len()),
            candidates,
        );
    }
    collect_nested_type_exposures(
        context,
        return_type,
        refinements,
        path,
        StructuralExposureSegment::FunctionResult,
        candidates,
    );
}

fn collect_nested_type_exposures<'a>(
    context: ExposureTraversalContext<'_>,
    ty: &Type,
    refinements: &mut std::iter::Peekable<std::slice::Iter<'a, VariantRefinementType>>,
    path: &[StructuralExposureSegment],
    segment: StructuralExposureSegment,
    candidates: &mut Vec<RefinementExposureCandidate>,
) {
    record_visibility_work(path.len() + 1);
    let mut nested = path.to_vec();
    nested.push(segment);
    collect_type_exposures(context, ty, refinements, &nested, candidates);
}

fn plain_refinement_name(refinement: &VariantRefinementType) -> Option<String> {
    let alternative = refinement.alternatives.first()?;
    if refinement.alternatives.len() != 1 || !alternative.type_arguments.is_empty() {
        return None;
    }
    let mut name = alternative.base.segments.join("::");
    name.push_str("::");
    name.push_str(&alternative.variant);
    Some(name)
}

fn collect_refinement_exposures(
    context: ExposureTraversalContext<'_>,
    refinement: &VariantRefinementType,
    path: &[StructuralExposureSegment],
    candidates: &mut Vec<RefinementExposureCandidate>,
) {
    record_visibility_candidate_construction(refinement.alternatives.len());
    let mut alternative_paths = std::collections::HashMap::<String, usize>::new();
    for (alternative_index, alternative) in refinement.alternatives.iter().enumerate() {
        let mut candidate = refinement_candidate(alternative);
        let alternative_path = refinement_alternative_path(
            refinement,
            path,
            alternative_index,
            &candidate.written_type,
            &mut alternative_paths,
        );
        candidate.structural_path = alternative_path.clone();
        candidate.structural_span = refinement_structural_span(context, refinement, path);
        candidates.push(candidate);
        for (argument_index, argument) in alternative.type_arguments.iter().enumerate() {
            collect_type_argument_exposures(
                context,
                refinement,
                argument,
                argument_index,
                &alternative_path,
                candidates,
            );
        }
    }
}

fn refinement_alternative_path(
    refinement: &VariantRefinementType,
    path: &[StructuralExposureSegment],
    alternative_index: usize,
    written_type: &str,
    alternative_paths: &mut std::collections::HashMap<String, usize>,
) -> Vec<StructuralExposureSegment> {
    record_visibility_work(path.len() + 1);
    let mut alternative_path = path.to_vec();
    if refinement.alternatives.len() > 1 {
        let path_index = *alternative_paths
            .entry(written_type.to_string())
            .or_insert(alternative_index);
        alternative_path.push(StructuralExposureSegment::UnionAlternative(path_index));
    }
    alternative_path
}

fn refinement_structural_span(
    context: ExposureTraversalContext<'_>,
    refinement: &VariantRefinementType,
    path: &[StructuralExposureSegment],
) -> Option<SourceSpan> {
    context.enclosing_structural_span.cloned().or_else(|| {
        path.iter()
            .any(|segment| matches!(segment, StructuralExposureSegment::RecordField(_)))
            .then(|| refinement.span.clone())
    })
}

fn collect_type_argument_exposures(
    context: ExposureTraversalContext<'_>,
    refinement: &VariantRefinementType,
    argument: &veln_ast::VariantRefinementTypeArgument,
    argument_index: usize,
    alternative_path: &[StructuralExposureSegment],
    candidates: &mut Vec<RefinementExposureCandidate>,
) {
    record_visibility_work(alternative_path.len() + 1);
    let mut nested = alternative_path.to_vec();
    nested.push(StructuralExposureSegment::TypeArgument(argument_index));
    let child_context = ExposureTraversalContext {
        annotation_span: context.annotation_span,
        placeholders: true,
        enclosing_structural_span: context.enclosing_structural_span.or(Some(&refinement.span)),
    };
    let skeleton = type_argument_skeleton(argument);
    if let Ok(ty) = parse_type_annotation(&skeleton) {
        let mut child_refinements = argument.ty_refinements.iter().peekable();
        collect_type_exposures(
            child_context,
            &ty,
            &mut child_refinements,
            &nested,
            candidates,
        );
        return;
    }
    for child in &argument.ty_refinements {
        collect_refinement_exposures(child_context, child, &nested, candidates);
    }
}

fn type_argument_skeleton(argument: &veln_ast::VariantRefinementTypeArgument) -> String {
    let mut text = String::new();
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        record_visibility_work(fragment.len());
        text.push_str(fragment);
        if index < argument.ty_refinements.len() {
            text.push_str("Placeholder::Variant");
        }
    }
    text
}

fn assign_outer_structural_spans(
    annotation_span: &SourceSpan,
    paths: &[TypePathSegments],
    candidates: &mut [RefinementExposureCandidate],
) {
    let mut containers = paths
        .iter()
        .filter(|path| path.type_argument_count > 0)
        .map(|path| &path.application_span)
        .collect::<Vec<_>>();
    containers.sort_by_key(|span| (span.start.offset, usize::MAX - span.end.offset));
    let mut next = 0;
    let mut active = Vec::<&SourceSpan>::new();
    for candidate in candidates {
        if candidate.structural_path.iter().any(|segment| {
            matches!(
                segment,
                StructuralExposureSegment::FunctionParameter(_)
                    | StructuralExposureSegment::FunctionResult
            )
        }) {
            candidate.structural_span = Some(annotation_span.clone());
            continue;
        }
        if !candidate
            .structural_path
            .iter()
            .any(|segment| matches!(segment, StructuralExposureSegment::TypeArgument(_)))
        {
            if candidate.structural_span.is_none()
                && candidate
                    .structural_path
                    .iter()
                    .any(|segment| matches!(segment, StructuralExposureSegment::RecordField(_)))
            {
                candidate.structural_span = Some(candidate.occurrence_span.clone());
            }
            continue;
        }
        while containers
            .get(next)
            .is_some_and(|span| span.start.offset < candidate.occurrence_span.start.offset)
        {
            record_visibility_work(1);
            active.push(containers[next]);
            next += 1;
        }
        active.retain(|span| {
            record_visibility_work(1);
            span.end.offset >= candidate.occurrence_span.end.offset
        });
        if let Some(span) = active
            .iter()
            .max_by_key(|span| span.end.offset.saturating_sub(span.start.offset))
        {
            candidate.structural_span = Some((*span).clone());
        }
    }
}

fn record_exposure_candidates(
    refinements: &[VariantRefinementType],
) -> Vec<RefinementExposureCandidate> {
    let mut candidates = Vec::new();
    let mut candidate_tails = Vec::new();
    let mut path_nodes = Vec::new();
    let mut record_path = Vec::<usize>::new();
    for refinement in refinements {
        let Some(path) = &refinement.record_field_path else {
            continue;
        };
        if path.shared_prefix_len > record_path.len() {
            record_path.clear();
            continue;
        }
        record_path.truncate(path.shared_prefix_len);
        for field in &path.suffix {
            record_visibility_path_node_construction(1);
            path_nodes.push(RecordFieldPathNode {
                parent: record_path.last().copied(),
                field: field.clone(),
            });
            record_path.push(path_nodes.len() - 1);
        }
        let record_path_tail = record_path.last().copied();
        record_visibility_candidate_construction(refinement.alternatives.len());
        for (alternative_index, alternative) in refinement.alternatives.iter().enumerate() {
            let mut candidate = refinement_candidate(alternative);
            candidate.structural_span = Some(refinement.span.clone());
            if refinement.alternatives.len() > 1 {
                candidate
                    .structural_path
                    .push(StructuralExposureSegment::UnionAlternative(
                        alternative_index,
                    ));
            }
            candidates.push(candidate);
            candidate_tails.push(record_path_tail);
        }
    }
    let arena = Arc::<[RecordFieldPathNode]>::from(path_nodes);
    for (candidate, tail) in candidates.iter_mut().zip(candidate_tails) {
        candidate.record_path = tail.map(|tail| RecordFieldPathRef {
            arena: Arc::clone(&arena),
            tail,
        });
    }
    candidates
}

fn recovered_root_exposure_candidates(
    annotation_span: &SourceSpan,
    annotation_text: &str,
    refinements: &[VariantRefinementType],
    paths: &[TypePathSegments],
) -> Vec<RefinementExposureCandidate> {
    record_visibility_candidate_construction(
        refinements
            .iter()
            .map(|refinement| refinement.alternatives.len())
            .sum(),
    );
    let structured_candidates = refinements
        .iter()
        .flat_map(|refinement| refinement.alternatives.iter())
        .map(refinement_candidate)
        .collect::<Vec<_>>();
    let nested_path_occurrences = nested_type_argument_occurrences(refinements);
    let recovered_candidates = recovered_type_path_candidates(
        annotation_span,
        annotation_text,
        paths,
        &nested_path_occurrences,
        &structured_candidates,
    );
    let mut candidates = structured_candidates;
    candidates.extend(recovered_candidates);
    candidates.sort_by_key(|candidate| candidate.occurrence_span.start.offset);
    if direct_root_sequence(annotation_text, &candidates) {
        candidates
    } else {
        Vec::new()
    }
}

fn recovered_type_path_candidates(
    annotation_span: &SourceSpan,
    annotation_text: &str,
    paths: &[TypePathSegments],
    nested_path_occurrences: &HashSet<(veln_source::SourcePath, usize, usize)>,
    structured_candidates: &[RefinementExposureCandidate],
) -> Vec<RefinementExposureCandidate> {
    let mut candidate_occurrences = structured_candidates
        .iter()
        .map(|candidate| occurrence_key(&candidate.occurrence_span))
        .collect::<HashSet<_>>();
    let mut candidates = Vec::new();
    for path in paths {
        if path.segments.len() < 2
            || !span_within(&path.application_span, annotation_span)
            || !path.segments.last().is_some_and(|segment| {
                segment
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_uppercase)
            })
        {
            continue;
        }
        record_visibility_occurrence_lookup(1);
        if nested_path_occurrences.contains(&occurrence_key(&path.application_span)) {
            continue;
        }
        if !candidate_occurrences.insert(occurrence_key(&path.application_span)) {
            continue;
        }
        let variant_index = path.segments.len() - 1;
        let written_type =
            annotation_occurrence_text(annotation_span, annotation_text, &path.application_span)
                .unwrap_or_else(|| path.written.clone());
        record_visibility_candidate_construction(1);
        candidates.push(RefinementExposureCandidate {
            written_base: path.segments[..variant_index].join("::"),
            type_argument_count: path.type_argument_count,
            written_variant: path.segments[variant_index].clone(),
            written_type,
            type_arguments: None,
            base_span: path.segment_spans[variant_index - 1].clone(),
            variant_span: path.segment_spans[variant_index].clone(),
            occurrence_span: path.application_span.clone(),
            record_path: None,
            structural_span: None,
            structural_path: Vec::new(),
        });
    }
    candidates
}

fn nested_type_argument_occurrences(
    refinements: &[VariantRefinementType],
) -> HashSet<(veln_source::SourcePath, usize, usize)> {
    let mut occurrences = HashSet::new();
    for path in refinements
        .iter()
        .flat_map(|refinement| refinement.alternatives.iter())
        .flat_map(|alternative| alternative.type_arguments.iter())
        .flat_map(|argument| argument.ty_paths.iter())
    {
        record_visibility_occurrence_lookup(1);
        occurrences.insert(occurrence_key(&path.application_span));
    }
    occurrences
}

fn refinement_candidate(
    alternative: &veln_ast::VariantRefinementAlternative,
) -> RefinementExposureCandidate {
    let type_arguments = alternative
        .type_arguments
        .iter()
        .map(render_type_argument)
        .collect::<Vec<_>>();
    let mut written_type = alternative.base.segments.join("::");
    if !type_arguments.is_empty() {
        written_type.push('<');
        written_type.push_str(&type_arguments.join(", "));
        written_type.push('>');
    }
    written_type.push_str("::");
    written_type.push_str(&alternative.variant);
    RefinementExposureCandidate {
        written_base: alternative.base.segments.join("::"),
        type_argument_count: alternative.type_arguments.len(),
        written_variant: alternative.variant.clone(),
        written_type,
        type_arguments: Some(type_arguments),
        base_span: alternative
            .base
            .segment_spans
            .last()
            .expect("type path has a segment")
            .clone(),
        variant_span: alternative.variant_span.clone(),
        occurrence_span: alternative.span.clone(),
        record_path: None,
        structural_span: None,
        structural_path: Vec::new(),
    }
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
        .map(|alternative| refinement_candidate(alternative).written_type)
        .collect::<Vec<_>>()
        .join(" | ")
}

fn annotation_occurrence_text(
    annotation_span: &SourceSpan,
    annotation_text: &str,
    occurrence_span: &SourceSpan,
) -> Option<String> {
    if occurrence_span.file != annotation_span.file {
        return None;
    }
    let start = occurrence_span
        .start
        .offset
        .checked_sub(annotation_span.start.offset)?;
    let end = occurrence_span
        .end
        .offset
        .checked_sub(annotation_span.start.offset)?;
    annotation_text.get(start..end).map(str::to_string)
}

fn span_within(candidate: &SourceSpan, enclosing: &SourceSpan) -> bool {
    candidate.file == enclosing.file
        && candidate.start.offset >= enclosing.start.offset
        && candidate.end.offset <= enclosing.end.offset
}

fn occurrence_key(span: &SourceSpan) -> (veln_source::SourcePath, usize, usize) {
    (span.file.clone(), span.start.offset, span.end.offset)
}

fn direct_root_sequence(annotation_text: &str, candidates: &[RefinementExposureCandidate]) -> bool {
    if candidates.is_empty() {
        return false;
    }
    let expected = candidates
        .iter()
        .map(|candidate| compact_type_text(&candidate.written_type))
        .collect::<Vec<_>>()
        .join("|");
    compact_type_text(annotation_text) == expected
}

fn compact_type_text(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

struct RefinementExposureCandidate {
    written_base: String,
    type_argument_count: usize,
    written_variant: String,
    written_type: String,
    type_arguments: Option<Vec<String>>,
    base_span: SourceSpan,
    variant_span: SourceSpan,
    occurrence_span: SourceSpan,
    record_path: Option<RecordFieldPathRef>,
    structural_span: Option<SourceSpan>,
    structural_path: Vec<StructuralExposureSegment>,
}

struct RecordFieldPathNode {
    parent: Option<usize>,
    field: String,
}

#[derive(Clone)]
struct RecordFieldPathRef {
    arena: Arc<[RecordFieldPathNode]>,
    tail: usize,
}

impl RecordFieldPathRef {
    fn node_identity(&self) -> usize {
        std::ptr::from_ref(&self.arena[self.tail]) as usize
    }
}

fn private_exposure<'a>(
    candidate: RefinementExposureCandidate,
    current_module: Option<&'a str>,
    environment: &'a TypeEnvironment,
) -> Option<PrivateExposure<'a>> {
    record_visibility_work(1);
    let written_descriptor = environment.variant_refinement_descriptor_identity(
        &candidate.written_base,
        candidate.type_argument_count,
        current_module,
    )?;
    let descriptor = environment
        .adts
        .terminal_descriptor_for_alias(written_descriptor)
        .unwrap_or_else(|| {
            environment
                .adts
                .canonical_descriptor_for(written_descriptor)
        });
    record_visibility_work(1);
    let variant =
        environment.variant_refinement_variant_identity(descriptor, &candidate.written_variant)?;
    let private_base = written_descriptor.visibility == Visibility::Private
        || descriptor.visibility == Visibility::Private;
    if !private_base && variant.visibility == Visibility::Public {
        return None;
    }
    let alias_declarations = environment
        .adts
        .type_alias_exposure_path(written_descriptor);
    record_visibility_work(1);
    let alternative_identity =
        refinement_alternative_identity(&candidate, &variant.name, current_module, environment);
    Some(PrivateExposure {
        written_type: candidate.written_type,
        base_span: candidate.base_span,
        variant_span: candidate.variant_span,
        descriptor,
        variant,
        declaration_span: private_declaration_span(environment, descriptor, variant, private_base),
        private_base,
        alternative_identity,
        alias_declarations,
        record_path: candidate.record_path,
        structural_span: candidate.structural_span,
        structural_path: candidate.structural_path,
    })
}

fn materialize_record_field_path(path: &RecordFieldPathRef) -> Vec<String> {
    let mut fields = Vec::new();
    let mut node = Some(path.tail);
    while let Some(index) = node {
        let current = &path.arena[index];
        fields.push(current.field.clone());
        node = current.parent;
    }
    fields.reverse();
    fields
}

fn refinement_alternative_identity(
    candidate: &RefinementExposureCandidate,
    resolved_variant: &str,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> RefinementAlternativeIdentity {
    let parsed_arguments = candidate.type_arguments.as_ref().and_then(|arguments| {
        arguments
            .iter()
            .map(|argument| parse_type_annotation(argument).ok())
            .collect::<Option<Vec<_>>>()
    });
    let parsed_arguments = parsed_arguments.or_else(|| {
        parse_type_annotation(&candidate.written_type)
            .ok()
            .and_then(|ty| match ty {
                Type::VariantRefinement { args, .. } => Some(args),
                _ => None,
            })
    });
    if let Some(args) = parsed_arguments {
        let args = args
            .into_iter()
            .map(|argument| {
                canonical_type_identity(
                    environment.canonicalize_type_annotation(argument, current_module),
                )
            })
            .collect::<Option<Vec<_>>>();
        if let Some(args) = args {
            return RefinementAlternativeIdentity::Resolved {
                args,
                variants: vec![resolved_variant.to_string()],
            };
        }
    }
    RefinementAlternativeIdentity::Written(candidate.written_type.clone())
}

fn canonical_type_identity(ty: Type) -> Option<Type> {
    match ty {
        Type::Named { identity, args, .. } => canonical_named_identity(identity, args),
        Type::VariantRefinement {
            identity,
            args,
            variants,
            unresolved_alternatives,
            ..
        } if unresolved_alternatives.is_empty() => {
            canonical_refinement_identity(identity, args, variants)
        }
        Type::VariantRefinement { .. } => None,
        Type::Record(fields) => canonical_record_identity(fields),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => canonical_function_identity(params, variadic, *return_type, effects),
        Type::Unknown => None,
    }
}

fn canonical_named_identity(identity: String, args: Vec<Type>) -> Option<Type> {
    Some(Type::resolved_named(
        identity.clone(),
        identity,
        canonical_type_identities(args)?,
    ))
}

fn canonical_refinement_identity(
    identity: String,
    args: Vec<Type>,
    variants: Arc<[String]>,
) -> Option<Type> {
    Some(Type::VariantRefinement {
        name: identity.clone(),
        identity,
        args: canonical_type_identities(args)?,
        variants,
        unresolved_alternatives: Vec::new(),
    })
}

fn canonical_record_identity(fields: Vec<(String, Type)>) -> Option<Type> {
    Some(Type::Record(
        fields
            .into_iter()
            .map(|(name, ty)| Some((name, canonical_type_identity(ty)?)))
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn canonical_function_identity(
    params: Vec<Type>,
    variadic: Option<Box<Type>>,
    return_type: Type,
    effects: Vec<String>,
) -> Option<Type> {
    Some(Type::Function {
        params: canonical_type_identities(params)?,
        variadic: canonical_optional_type(variadic)?,
        return_type: Box::new(canonical_type_identity(return_type)?),
        effects,
    })
}

fn canonical_type_identities(types: Vec<Type>) -> Option<Vec<Type>> {
    types.into_iter().map(canonical_type_identity).collect()
}

fn canonical_optional_type(ty: Option<Box<Type>>) -> Option<Option<Box<Type>>> {
    match ty {
        Some(ty) => Some(Some(Box::new(canonical_type_identity(*ty)?))),
        None => Some(None),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum RefinementAlternativeIdentity {
    Resolved {
        args: Vec<Type>,
        variants: Vec<String>,
    },
    Written(String),
}

fn private_declaration_span(
    environment: &TypeEnvironment,
    descriptor: &AdtDescriptor,
    variant: &AdtVariantDescriptor,
    private_base: bool,
) -> Option<SourceSpan> {
    if private_base {
        return environment.adts.declaration_span(descriptor).cloned();
    }
    environment
        .adts
        .variant_declaration_span(descriptor, variant)
        .cloned()
}

#[cfg(test)]
thread_local! {
    static VISIBILITY_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static VISIBILITY_CANDIDATE_WORK: std::cell::Cell<VisibilityCandidateWork> =
        const { std::cell::Cell::new(VisibilityCandidateWork::new()) };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VisibilityCandidateWork {
    pub(crate) root_classification: usize,
    pub(crate) candidate_construction: usize,
    pub(crate) occurrence_lookup: usize,
    pub(crate) record_path_nodes: usize,
}

#[cfg(test)]
impl VisibilityCandidateWork {
    const fn new() -> Self {
        Self {
            root_classification: 0,
            candidate_construction: 0,
            occurrence_lookup: 0,
            record_path_nodes: 0,
        }
    }
}

#[cfg(test)]
fn record_visibility_work(amount: usize) {
    VISIBILITY_WORK.with(|work| work.set(work.get() + amount));
}

#[cfg(test)]
fn record_visibility_root_classification(amount: usize) {
    VISIBILITY_CANDIDATE_WORK.with(|work| {
        let mut next = work.get();
        next.root_classification += amount;
        work.set(next);
    });
}

#[cfg(not(test))]
fn record_visibility_root_classification(_amount: usize) {}

#[cfg(test)]
fn record_visibility_candidate_construction(amount: usize) {
    VISIBILITY_CANDIDATE_WORK.with(|work| {
        let mut next = work.get();
        next.candidate_construction += amount;
        work.set(next);
    });
}

#[cfg(not(test))]
fn record_visibility_candidate_construction(_amount: usize) {}

#[cfg(test)]
fn record_visibility_occurrence_lookup(amount: usize) {
    VISIBILITY_CANDIDATE_WORK.with(|work| {
        let mut next = work.get();
        next.occurrence_lookup += amount;
        work.set(next);
    });
}

#[cfg(not(test))]
fn record_visibility_occurrence_lookup(_amount: usize) {}

#[cfg(test)]
fn record_visibility_path_node_construction(amount: usize) {
    VISIBILITY_CANDIDATE_WORK.with(|work| {
        let mut next = work.get();
        next.record_path_nodes += amount;
        work.set(next);
    });
}

#[cfg(not(test))]
fn record_visibility_path_node_construction(_amount: usize) {}

#[cfg(not(test))]
fn record_visibility_work(_amount: usize) {}

#[cfg(test)]
pub(crate) fn reset_visibility_work() {
    VISIBILITY_WORK.with(|work| work.set(0));
    VISIBILITY_CANDIDATE_WORK.with(|work| work.set(VisibilityCandidateWork::new()));
}

#[cfg(test)]
pub(crate) fn take_visibility_work() -> usize {
    VISIBILITY_WORK.with(|work| work.replace(0))
}

#[cfg(test)]
pub(crate) fn take_visibility_candidate_work() -> VisibilityCandidateWork {
    VISIBILITY_CANDIDATE_WORK.with(|work| work.replace(VisibilityCandidateWork::new()))
}

struct PrivateExposure<'a> {
    written_type: String,
    base_span: SourceSpan,
    variant_span: SourceSpan,
    descriptor: &'a AdtDescriptor,
    variant: &'a AdtVariantDescriptor,
    declaration_span: Option<SourceSpan>,
    private_base: bool,
    alternative_identity: RefinementAlternativeIdentity,
    alias_declarations: Option<TypeAliasExposurePath<'a>>,
    record_path: Option<RecordFieldPathRef>,
    structural_span: Option<SourceSpan>,
    structural_path: Vec<StructuralExposureSegment>,
}

impl PrivateExposure<'_> {
    fn diagnostic(&self, boundary: PublicBoundary<'_>) -> Diagnostic {
        let base_type = self.descriptor.identity();
        let (span, message, declaration_kind, declaration_message) = self.primary_context();
        let record_fields = self
            .record_path
            .as_ref()
            .map(materialize_record_field_path)
            .unwrap_or_default();
        let mut diagnostic = Diagnostic::new(
            "type.variant_refinement_private",
            Severity::Error,
            DiagnosticKind::Type,
            message,
            Some(span),
            JsonValue::object([
                ("written_type", JsonValue::string(self.written_type.clone())),
                ("base_type", JsonValue::string(base_type)),
                ("variant", JsonValue::string(self.variant.name.clone())),
                ("boundary", JsonValue::string(boundary.name())),
                (
                    "exposure_path",
                    boundary.exposure_path(&record_fields, &self.structural_path),
                ),
            ]),
        );
        if let Some(path) = &self.alias_declarations {
            let aliases = path.declarations().collect::<Vec<_>>();
            for alias in aliases {
                diagnostic.related.push(JsonValue::object([
                    ("kind", JsonValue::string("type_alias_declaration")),
                    (
                        "message",
                        JsonValue::string(format!(
                            "Public type alias `{}` is declared here.",
                            alias.identity
                        )),
                    ),
                    ("span", span_json(&alias.span)),
                ]));
            }
        }
        if let Some(span) = &self.declaration_span {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string(declaration_kind)),
                ("message", JsonValue::string(declaration_message)),
                ("span", span_json(span)),
            ]));
        }
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string(boundary.related_kind())),
            ("message", JsonValue::string(boundary.related_message())),
            ("span", span_json(boundary.span())),
        ]));
        diagnostic
    }

    fn primary_context(&self) -> (SourceSpan, String, &'static str, String) {
        let base_type = self.descriptor.identity();
        if let Some(span) = &self.structural_span {
            let declaration_kind = if self.private_base {
                "private_type_declaration"
            } else {
                "private_variant_declaration"
            };
            let declaration_message = if self.private_base {
                format!("Private type `{base_type}` is declared here.")
            } else {
                format!(
                    "Private variant `{}::{}` is declared here.",
                    base_type, self.variant.name
                )
            };
            return (
                span.clone(),
                if self.private_base {
                    format!("public refinement exposes private type `{base_type}`")
                } else {
                    format!(
                        "public refinement exposes private variant `{}::{}`",
                        base_type, self.variant.name
                    )
                },
                declaration_kind,
                declaration_message,
            );
        }
        if self.alias_declarations.is_some() {
            let declaration_kind = if self.private_base {
                "private_type_declaration"
            } else {
                "private_variant_declaration"
            };
            let declaration_message = if self.private_base {
                format!("Private type `{base_type}` is declared here.")
            } else {
                format!(
                    "Private variant `{}::{}` is declared here.",
                    base_type, self.variant.name
                )
            };
            return (
                self.base_span.clone(),
                if self.private_base {
                    format!("public refinement exposes private type `{base_type}`")
                } else {
                    format!(
                        "public refinement exposes private variant `{}::{}`",
                        base_type, self.variant.name
                    )
                },
                declaration_kind,
                declaration_message,
            );
        }
        if self.private_base {
            (
                self.base_span.clone(),
                format!("public refinement exposes private type `{base_type}`"),
                "private_type_declaration",
                format!("Private type `{base_type}` is declared here."),
            )
        } else {
            (
                self.variant_span.clone(),
                format!(
                    "public refinement exposes private variant `{}::{}`",
                    base_type, self.variant.name
                ),
                "private_variant_declaration",
                format!(
                    "Private variant `{}::{}` is declared here.",
                    base_type, self.variant.name
                ),
            )
        }
    }
}

#[derive(Clone, Copy)]
enum PublicBoundary<'a> {
    Parameter {
        function: &'a Function,
        param: &'a Param,
    },
    Result {
        function: &'a Function,
        span: Option<&'a SourceSpan>,
    },
    AdtPayload {
        declaration: &'a TypeDecl,
        variant: &'a TypeVariantDecl,
        field: &'a TypeVariantField,
    },
}

impl<'a> PublicBoundary<'a> {
    fn name(self) -> &'static str {
        match self {
            Self::Parameter { .. } => "public_parameter",
            Self::Result { .. } => "public_result",
            Self::AdtPayload { .. } => "public_adt_payload",
        }
    }

    fn exposure_path(
        self,
        record_fields: &[String],
        structural_path: &[StructuralExposureSegment],
    ) -> JsonValue {
        let mut segments = match self {
            Self::Parameter { param, .. } => vec![JsonValue::object([
                ("kind", JsonValue::string("parameter")),
                ("name", JsonValue::string(param.name.clone())),
            ])],
            Self::Result { .. } => {
                vec![JsonValue::object([("kind", JsonValue::string("result"))])]
            }
            Self::AdtPayload {
                declaration,
                variant,
                field,
            } => vec![JsonValue::object([
                ("kind", JsonValue::string("adt_payload")),
                (
                    "type",
                    JsonValue::string(declaration.name.clone().unwrap_or_default()),
                ),
                (
                    "variant",
                    JsonValue::string(variant.name.clone().unwrap_or_default()),
                ),
                ("field", JsonValue::string(field.name.clone())),
            ])],
        };
        segments.extend(record_fields.iter().map(|field| {
            JsonValue::object([
                ("kind", JsonValue::string("record_field")),
                ("name", JsonValue::string(field.clone())),
            ])
        }));
        segments.extend(structural_path.iter().map(|segment| match segment {
            StructuralExposureSegment::TypeArgument(index) => JsonValue::object([
                ("kind", JsonValue::string("type_argument")),
                ("index", JsonValue::Number(*index as i64)),
            ]),
            StructuralExposureSegment::FunctionParameter(index) => JsonValue::object([
                ("kind", JsonValue::string("function_parameter")),
                ("index", JsonValue::Number(*index as i64)),
            ]),
            StructuralExposureSegment::FunctionResult => {
                JsonValue::object([("kind", JsonValue::string("function_result"))])
            }
            StructuralExposureSegment::RecordField(field) => JsonValue::object([
                ("kind", JsonValue::string("record_field")),
                ("name", JsonValue::string(field.clone())),
            ]),
            StructuralExposureSegment::UnionAlternative(index) => JsonValue::object([
                ("kind", JsonValue::string("union_alternative")),
                ("index", JsonValue::Number(*index as i64)),
            ]),
        }));
        JsonValue::array(segments)
    }

    fn related_kind(self) -> &'static str {
        match self {
            Self::Parameter { .. } => "public_parameter_boundary",
            Self::Result { .. } => "public_result_boundary",
            Self::AdtPayload { .. } => "public_adt_payload_boundary",
        }
    }

    fn related_message(self) -> String {
        match self {
            Self::Parameter { function, param } => format!(
                "Public parameter `{}` of function `{function_name}` exposes this refinement.",
                param.name,
                function_name = function.name.as_deref().unwrap_or("<invalid>")
            ),
            Self::Result { function, .. } => {
                let function_name = function.name.as_deref().unwrap_or("<invalid>");
                format!("Public result of function `{function_name}` exposes this refinement.")
            }
            Self::AdtPayload {
                declaration,
                variant,
                field,
            } => format!(
                "Public payload `{}` of variant `{}::{}` exposes this refinement.",
                field.name,
                declaration.name.as_deref().unwrap_or("<invalid>"),
                variant.name.as_deref().unwrap_or("<invalid>")
            ),
        }
    }

    fn span(self) -> &'a SourceSpan {
        match self {
            Self::Parameter { param, .. } => &param.span,
            Self::Result {
                span: Some(span), ..
            } => span,
            Self::Result { function, .. } => &function.span,
            Self::AdtPayload { field, .. } => &field.span,
        }
    }
}
