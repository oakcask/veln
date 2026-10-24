use super::*;

struct UnsupportedRefinement {
    depth: usize,
    start_offset: usize,
    target_type: String,
    base_type: String,
    expected_variants: Vec<String>,
    span: veln_source::SourceSpan,
}

pub(super) fn check_binary_schema_variant_refinement_decode(
    schema: &SchemaDecl,
    field: &SchemaField,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary")
        || field.ty_refinements.is_empty()
    {
        return false;
    }

    let current_module = schema.module_name.as_deref();
    let mut candidates = Vec::new();
    collect_unsupported_refinements(
        &field.ty_refinements,
        0,
        current_module,
        environment,
        &mut candidates,
    );
    candidates.sort_by_key(|candidate| (usize::MAX - candidate.depth, candidate.start_offset));

    if let Some(candidate) = candidates.into_iter().next() {
        diagnostics.push(variant_refinement_decode_unsupported_diagnostic(
            schema, candidate,
        ));
        return true;
    }

    // Preserve the existing composition and refinement diagnostics when no
    // finite, valid refinement target can establish this capability failure.
    false
}

fn collect_unsupported_refinements(
    refinements: &[veln_ast::VariantRefinementType],
    depth: usize,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    candidates: &mut Vec<UnsupportedRefinement>,
) {
    for refinement in refinements {
        if let Some(candidate) =
            unsupported_refinement(refinement, depth, current_module, environment)
        {
            candidates.push(candidate);
        }
        for alternative in &refinement.alternatives {
            for argument in &alternative.type_arguments {
                collect_unsupported_refinements(
                    &argument.ty_refinements,
                    depth + 1,
                    current_module,
                    environment,
                    candidates,
                );
            }
        }
    }
}

fn unsupported_refinement(
    refinement: &veln_ast::VariantRefinementType,
    depth: usize,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> Option<UnsupportedRefinement> {
    let first = refinement.alternatives.first()?;
    let first_base = written_refinement_base(first);
    let descriptor = environment.variant_refinement_descriptor_identity(
        &first.base.segments.join("::"),
        first.type_arguments.len(),
        current_module,
    )?;
    if descriptor.type_parameters.len() != first.type_arguments.len()
        || descriptor.refinement_availability
            != crate::adt::descriptors::VariantRefinementAvailability::Finite
    {
        return None;
    }
    let descriptor_identity = descriptor.identity();
    let mut expected_variants = Vec::with_capacity(refinement.alternatives.len());
    for alternative in &refinement.alternatives {
        let alternative_descriptor = environment.variant_refinement_descriptor_identity(
            &alternative.base.segments.join("::"),
            alternative.type_arguments.len(),
            current_module,
        )?;
        if alternative_descriptor.type_parameters.len() != alternative.type_arguments.len()
            || alternative_descriptor.identity() != descriptor_identity
        {
            return None;
        }
        let variant = environment
            .variant_refinement_variant_identity(alternative_descriptor, &alternative.variant)?;
        expected_variants.push(variant.name.clone());
    }

    Some(UnsupportedRefinement {
        depth,
        start_offset: refinement.span.start.offset,
        target_type: render_refinement(refinement),
        base_type: first_base,
        expected_variants,
        span: refinement.span.clone(),
    })
}

fn variant_refinement_decode_unsupported_diagnostic(
    schema: &SchemaDecl,
    candidate: UnsupportedRefinement,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let decoder = schema_decode_function_name(schema_name);
    let expected = candidate.expected_variants.join(" | ");
    let mut diagnostic = Diagnostic::new(
        "schema.variant_refinement_decode_unsupported",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "generated binary decoder `{decoder}` cannot establish variant refinement `{}`",
            candidate.target_type
        ),
        Some(candidate.span),
        JsonValue::object([
            ("target_type", JsonValue::string(candidate.target_type)),
            ("base_type", JsonValue::string(candidate.base_type.clone())),
            (
                "expected_variants",
                JsonValue::array(
                    candidate
                        .expected_variants
                        .into_iter()
                        .map(JsonValue::string),
                ),
            ),
            ("decoder", JsonValue::string(decoder.clone())),
        ]),
    );
    let mut related = vec![
        ("kind", JsonValue::string("decoder_provenance")),
        (
            "message",
            JsonValue::string(format!(
                "`format binary` selects generated decoder `{decoder}`, which cannot construct an admitted `{}` variant or validate its decoded tag ({expected}).",
                candidate.base_type
            )),
        ),
        ("decoder", JsonValue::string(decoder)),
        ("provenance", JsonValue::string("generated_binary_schema")),
    ];
    if let Some(format) = &schema.format {
        related.push(("span", span_json(&format.span)));
    }
    diagnostic.related.push(JsonValue::object(related));
    diagnostic
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
