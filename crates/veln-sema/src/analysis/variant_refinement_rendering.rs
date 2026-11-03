use veln_ast::{
    VariantRefinementAlternative, VariantRefinementType, VariantRefinementTypeArgument,
};

pub(in crate::analysis) fn written_refinement_base(
    alternative: &VariantRefinementAlternative,
) -> String {
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

pub(in crate::analysis) fn render_refinement_alternative(
    alternative: &VariantRefinementAlternative,
) -> String {
    format!(
        "{}::{}",
        written_refinement_base(alternative),
        alternative.variant
    )
}

pub(in crate::analysis) fn render_type_argument(
    argument: &VariantRefinementTypeArgument,
) -> String {
    let mut text = String::new();
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        text.push_str(fragment);
        if let Some(refinement) = argument.ty_refinements.get(index) {
            text.push_str(&render_refinement(refinement));
        }
    }
    text
}

pub(in crate::analysis) fn render_refinement(refinement: &VariantRefinementType) -> String {
    refinement
        .alternatives
        .iter()
        .map(render_refinement_alternative)
        .collect::<Vec<_>>()
        .join(" | ")
}
