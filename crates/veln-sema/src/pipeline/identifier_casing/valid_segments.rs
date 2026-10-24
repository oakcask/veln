use super::*;
use crate::semantic_model::Type;

type SuppressedQualifiedPathLocations = HashMap<String, HashSet<(usize, usize, usize)>>;

thread_local! {
    static VARIANT_REFINEMENT_CLASSIFICATION_WORK:
        std::cell::Cell<Option<VariantRefinementClassificationWork>> = const { std::cell::Cell::new(None) };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VariantRefinementClassificationWork {
    pub rendered_annotations: usize,
    pub rendered_bytes: usize,
}

fn record_variant_refinement_classification_work(bytes: usize) {
    VARIANT_REFINEMENT_CLASSIFICATION_WORK.with(|work| {
        if let Some(mut current) = work.get() {
            current.rendered_annotations += 1;
            current.rendered_bytes += bytes;
            work.set(Some(current));
        }
    });
}

pub(super) fn valid_qualified_path_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    include_variant_refinements: bool,
    require_valid_variant_refinements: bool,
) -> (Vec<QualifiedPathSegment>, SuppressedQualifiedPathLocations) {
    let mut segments = Vec::new();
    let mut suppressed_locations = SuppressedQualifiedPathLocations::new();
    for type_decl in &module.types {
        let current_module = type_decl.module_name.as_deref();
        for variant in &type_decl.variants {
            for field in &variant.fields {
                collect_type_path_segments(
                    &field.ty_paths,
                    &field.ty_refinements,
                    current_module,
                    environment,
                    &mut segments,
                    &mut suppressed_locations,
                );
                if include_variant_refinements {
                    collect_variant_refinement_segments(
                        &field.ty_refinements,
                        current_module,
                        environment,
                        &type_decl.params,
                        require_valid_variant_refinements,
                        &mut segments,
                        &mut suppressed_locations,
                    );
                }
            }
        }
    }
    for effect in &module.effects {
        let current_module = effect.module_name.as_deref();
        for operation in &effect.operations {
            for param in &operation.params {
                collect_type_path_segments(
                    &param.ty_paths,
                    &param.ty_refinements,
                    current_module,
                    environment,
                    &mut segments,
                    &mut suppressed_locations,
                );
                if include_variant_refinements {
                    collect_variant_refinement_segments(
                        &param.ty_refinements,
                        current_module,
                        environment,
                        &[],
                        require_valid_variant_refinements,
                        &mut segments,
                        &mut suppressed_locations,
                    );
                }
            }
            collect_type_path_segments(
                &operation.return_type_paths,
                &operation.return_type_refinements,
                current_module,
                environment,
                &mut segments,
                &mut suppressed_locations,
            );
            if include_variant_refinements {
                collect_variant_refinement_segments(
                    &operation.return_type_refinements,
                    current_module,
                    environment,
                    &[],
                    require_valid_variant_refinements,
                    &mut segments,
                    &mut suppressed_locations,
                );
            }
        }
    }
    for schema in &module.schemas {
        let current_module = schema.module_name.as_deref();
        for field in &schema.fields {
            collect_type_path_segments(
                &field.ty_paths,
                &field.ty_refinements,
                current_module,
                environment,
                &mut segments,
                &mut suppressed_locations,
            );
            if include_variant_refinements {
                collect_variant_refinement_segments(
                    &field.ty_refinements,
                    current_module,
                    environment,
                    &[],
                    require_valid_variant_refinements,
                    &mut segments,
                    &mut suppressed_locations,
                );
            }
        }
    }
    for function in &module.functions {
        crate::pipeline::record_variant_refinement_casing_work(1);
        let current_module = function.module_name.as_deref();
        collect_type_path_segments(
            &function.return_type_paths,
            &function.return_type_refinements,
            current_module,
            environment,
            &mut segments,
            &mut suppressed_locations,
        );
        if include_variant_refinements {
            collect_variant_refinement_segments(
                &function.return_type_refinements,
                current_module,
                environment,
                &[],
                require_valid_variant_refinements,
                &mut segments,
                &mut suppressed_locations,
            );
        }
        for param in &function.params {
            crate::pipeline::record_variant_refinement_casing_work(1);
            collect_type_path_segments(
                &param.ty_paths,
                &param.ty_refinements,
                current_module,
                environment,
                &mut segments,
                &mut suppressed_locations,
            );
            if include_variant_refinements {
                collect_variant_refinement_segments(
                    &param.ty_refinements,
                    current_module,
                    environment,
                    &[],
                    require_valid_variant_refinements,
                    &mut segments,
                    &mut suppressed_locations,
                );
            }
        }
        for line in &function.body {
            crate::pipeline::record_variant_refinement_casing_work(1);
            collect_valid_segments_from_body_line(
                line,
                current_module,
                environment,
                include_variant_refinements,
                require_valid_variant_refinements,
                &mut segments,
                &mut suppressed_locations,
            );
        }
    }
    collect_handler_segments(
        module,
        environment,
        include_variant_refinements,
        require_valid_variant_refinements,
        &mut segments,
        &mut suppressed_locations,
    );
    (segments, suppressed_locations)
}

fn collect_handler_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    include_variant_refinements: bool,
    require_valid_variant_refinements: bool,
    segments: &mut Vec<QualifiedPathSegment>,
    suppressed_locations: &mut HashMap<String, HashSet<(usize, usize, usize)>>,
) {
    for handler in &module.handlers {
        let current_module = handler.module_name.as_deref();
        for param in &handler.params {
            collect_type_path_segments(
                &param.ty_paths,
                &param.ty_refinements,
                current_module,
                environment,
                segments,
                suppressed_locations,
            );
            if include_variant_refinements {
                collect_variant_refinement_segments(
                    &param.ty_refinements,
                    current_module,
                    environment,
                    &[],
                    require_valid_variant_refinements,
                    segments,
                    suppressed_locations,
                );
            }
        }
        for clause in &handler.operation_clauses {
            for param in &clause.params {
                collect_type_path_segments(
                    &param.ty_paths,
                    &param.ty_refinements,
                    current_module,
                    environment,
                    segments,
                    suppressed_locations,
                );
                if include_variant_refinements {
                    collect_variant_refinement_segments(
                        &param.ty_refinements,
                        current_module,
                        environment,
                        &[],
                        require_valid_variant_refinements,
                        segments,
                        suppressed_locations,
                    );
                }
            }
            collect_valid_segments_from_expr(
                &clause.body,
                current_module,
                environment,
                include_variant_refinements,
                require_valid_variant_refinements,
                segments,
                suppressed_locations,
            );
        }
    }
}

fn collect_type_path_segments(
    paths: &[veln_ast::TypePathSegments],
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
    suppressed_locations: &mut HashMap<String, HashSet<(usize, usize, usize)>>,
) {
    let selected_refinement_fallbacks = refinements
        .iter()
        .flat_map(|refinement| &refinement.alternatives)
        .filter(|alternative| {
            environment.variant_refinement_syntax_candidate_is_selected(
                &alternative.base.segments,
                alternative.type_arguments.len(),
                &alternative.variant,
                current_module,
            ) && !single_segment_base_is_module_qualifier(alternative, current_module, environment)
        })
        .map(|alternative| {
            (
                alternative.span.file.as_str().to_string(),
                alternative.span.start.offset,
                alternative.span.end.offset,
            )
        })
        .collect::<HashSet<_>>();
    for path in paths {
        let selected_refinement_fallback = path
            .segment_spans
            .first()
            .zip(path.segment_spans.last())
            .is_some_and(|(first, last)| {
                selected_refinement_fallbacks.contains(&(
                    first.file.as_str().to_string(),
                    first.start.offset,
                    last.end.offset,
                ))
            });
        if selected_refinement_fallback {
            for (segment_index, span) in path.segment_spans.iter().enumerate() {
                suppressed_locations
                    .entry(span.file.as_str().to_string())
                    .or_default()
                    .insert((span.start.offset, span.end.offset, segment_index));
            }
            continue;
        }
        if path.segments.len() < 2 {
            continue;
        }
        if environment.recovered_variant_refinement_candidate(&path.segments, current_module) {
            let base_index = path.segments.len() - 2;
            for (index, (segment, span)) in
                path.segments.iter().zip(&path.segment_spans).enumerate()
            {
                let role = if index == base_index {
                    NameClass::Type
                } else if index + 1 == path.segments.len() {
                    NameClass::Constructor
                } else {
                    NameClass::Module
                };
                output.push(qualified_path_segment_from_parts(
                    segment,
                    role,
                    span,
                    index,
                    QualifiedPathSegmentEvidence::UniqueRecovery,
                ));
            }
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

fn single_segment_base_is_module_qualifier(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    environment
        .variant_refinement_base_is_module_qualifier(&alternative.base.segments, current_module)
}

fn collect_variant_refinement_segments(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
    require_valid_variant_refinements: bool,
    output: &mut Vec<QualifiedPathSegment>,
    suppressed_locations: &mut HashMap<String, HashSet<(usize, usize, usize)>>,
) {
    let type_parameter_context = crate::types::TypeParameterIdentityContext::new(type_parameters);
    let mut pending = refinements
        .iter()
        .rev()
        .map(|refinement| (refinement, false))
        .collect::<Vec<_>>();
    while let Some((refinement, valid_ancestor)) = pending.pop() {
        let refinement_is_valid = !require_valid_variant_refinements
            || valid_ancestor
            || variant_refinement_is_valid(
                refinement,
                current_module,
                environment,
                type_parameters,
            );
        let mut nested_refinements = Vec::new();
        for alternative in &refinement.alternatives {
            if !environment.variant_refinement_syntax_candidate_is_selected(
                &alternative.base.segments,
                alternative.type_arguments.len(),
                &alternative.variant,
                current_module,
            ) {
                continue;
            }
            let base = alternative.base.segments.join("::");
            if !base.contains("::") && type_parameter_context.ordinal(&base).is_some() {
                for argument in &alternative.type_arguments {
                    collect_type_path_segments(
                        &argument.ty_paths,
                        &argument.ty_refinements,
                        current_module,
                        environment,
                        output,
                        suppressed_locations,
                    );
                    nested_refinements.extend(&argument.ty_refinements);
                }
                continue;
            }
            collect_variant_refinement_alternative(
                alternative,
                refinement_is_valid,
                current_module,
                environment,
                output,
                &mut nested_refinements,
                suppressed_locations,
            );
        }
        pending.extend(
            nested_refinements
                .into_iter()
                .rev()
                .map(|nested| (nested, refinement_is_valid)),
        );
    }
}

fn collect_variant_refinement_alternative<'a>(
    alternative: &'a veln_ast::VariantRefinementAlternative,
    refinement_is_valid: bool,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
    nested_refinements: &mut Vec<&'a veln_ast::VariantRefinementType>,
    suppressed_locations: &mut HashMap<String, HashSet<(usize, usize, usize)>>,
) {
    let mut segments = alternative.base.segments.clone();
    segments.push(alternative.variant.clone());
    let mut spans = alternative.base.segment_spans.clone();
    spans.push(alternative.variant_span.clone());
    let base = alternative.base.segments.join("::");
    let recovered = environment.recovered_variant_refinement_base_resolves(&base, current_module);
    let constructor = environment
        .adts
        .constructor(&segments, current_module, &environment.uses);
    if matches!(
        constructor,
        crate::adt::registry::ConstructorLookup::Found(_)
    ) {
        if refinement_is_valid {
            push_constructor_path_segments(&segments, &spans, current_module, environment, output);
        }
    } else if environment
        .variant_refinement_descriptor_identity(
            &base,
            alternative.type_arguments.len(),
            current_module,
        )
        .is_some()
        || environment.non_adt_refinement_base_resolves(
            &base,
            alternative.type_arguments.len(),
            current_module,
        )
        || recovered
    {
        let evidence = if recovered {
            QualifiedPathSegmentEvidence::UniqueRecovery
        } else {
            QualifiedPathSegmentEvidence::Resolved
        };
        push_variant_refinement_path_segments(&segments, &spans, evidence, output);
    }
    for argument in &alternative.type_arguments {
        collect_type_path_segments(
            &argument.ty_paths,
            &argument.ty_refinements,
            current_module,
            environment,
            output,
            suppressed_locations,
        );
        nested_refinements.extend(&argument.ty_refinements);
    }
}

fn variant_refinement_is_valid(
    refinement: &veln_ast::VariantRefinementType,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
) -> bool {
    let text = render_variant_refinement(refinement);
    record_variant_refinement_classification_work(text.len());
    crate::type_syntax::parse_type_annotation(&text)
        .ok()
        .is_some_and(|ty| {
            environment
                .variant_refinement_annotation_error(&ty, current_module)
                .is_none()
                && refinement.alternatives.iter().all(|alternative| {
                    alternative.type_arguments.iter().all(|argument| {
                        crate::type_syntax::parse_type_annotation(
                            &render_variant_refinement_type_argument(argument),
                        )
                        .ok()
                        .is_some_and(|argument| {
                            environment.type_annotation_is_fully_resolved(
                                &argument,
                                current_module,
                                type_parameters,
                            )
                        })
                    })
                })
        })
}

fn render_variant_refinement(refinement: &veln_ast::VariantRefinementType) -> String {
    refinement
        .alternatives
        .iter()
        .map(render_variant_refinement_alternative)
        .collect::<Vec<_>>()
        .join(" | ")
}

fn render_variant_refinement_alternative(
    alternative: &veln_ast::VariantRefinementAlternative,
) -> String {
    let mut text = alternative.base.segments.join("::");
    if !alternative.type_arguments.is_empty() {
        text.push('<');
        text.push_str(
            &alternative
                .type_arguments
                .iter()
                .map(render_variant_refinement_type_argument)
                .collect::<Vec<_>>()
                .join(", "),
        );
        text.push('>');
    }
    text.push_str("::");
    text.push_str(&alternative.variant);
    text
}

fn render_variant_refinement_type_argument(
    argument: &veln_ast::VariantRefinementTypeArgument,
) -> String {
    let mut text = String::new();
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        text.push_str(fragment);
        if let Some(refinement) = argument.ty_refinements.get(index) {
            text.push_str(&render_variant_refinement(refinement));
        }
    }
    text
}

fn push_variant_refinement_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    evidence: QualifiedPathSegmentEvidence,
    output: &mut Vec<QualifiedPathSegment>,
) {
    for (index, (segment, span)) in segments.iter().zip(segment_spans).enumerate() {
        let role = if index + 1 == segments.len() {
            NameClass::Constructor
        } else if index + 2 == segments.len() {
            NameClass::Type
        } else {
            NameClass::Module
        };
        output.push(qualified_path_segment_from_parts(
            segment, role, span, index, evidence,
        ));
    }
}

mod traversal;
use traversal::{
    collect_valid_segments_from_body_line, collect_valid_segments_from_expr,
    push_constructor_path_segments, qualified_path_segment_from_parts,
};

pub fn classified_project_qualified_path_segments(
    module: &SurfaceModule,
) -> Vec<QualifiedPathSegment> {
    classified_project_qualified_path_segments_with_context(module, module)
}

pub fn classified_project_qualified_path_segments_with_context(
    module: &SurfaceModule,
    project: &SurfaceModule,
) -> Vec<QualifiedPathSegment> {
    let environment = TypeEnvironment::for_path_classification(project);
    classified_qualified_path_segments_for_navigation(module, &environment)
}

pub fn classified_project_qualified_path_segments_with_context_and_work(
    module: &SurfaceModule,
    project: &SurfaceModule,
) -> (
    Vec<QualifiedPathSegment>,
    VariantRefinementClassificationWork,
) {
    VARIANT_REFINEMENT_CLASSIFICATION_WORK
        .with(|work| work.set(Some(VariantRefinementClassificationWork::default())));
    let segments = classified_project_qualified_path_segments_with_context(module, project);
    let work =
        VARIANT_REFINEMENT_CLASSIFICATION_WORK.with(|work| work.replace(None).unwrap_or_default());
    (segments, work)
}

mod canonical;
pub use canonical::{
    CanonicalTypeAnnotationIdentity, CanonicalTypeAnnotationWork,
    canonical_type_annotation_identities_with_context,
    canonical_type_annotation_identities_with_context_and_work,
};
