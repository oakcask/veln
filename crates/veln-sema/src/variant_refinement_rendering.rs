use veln_ast::{
    VariantRefinementAlternative, VariantRefinementType, VariantRefinementTypeArgument,
};

pub(crate) fn written_refinement_base(alternative: &VariantRefinementAlternative) -> String {
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

pub(crate) fn render_refinement_alternative(alternative: &VariantRefinementAlternative) -> String {
    format!(
        "{}::{}",
        written_refinement_base(alternative),
        alternative.variant
    )
}

pub(crate) fn render_type_argument(argument: &VariantRefinementTypeArgument) -> String {
    let mut text = String::new();
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        text.push_str(fragment);
        if let Some(refinement) = argument.ty_refinements.get(index) {
            text.push_str(&render_refinement(refinement));
        }
    }
    text
}

pub(crate) fn render_refinement(refinement: &VariantRefinementType) -> String {
    refinement
        .alternatives
        .iter()
        .map(render_refinement_alternative)
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use veln_source::SourceFile;

    #[test]
    fn renders_nested_refinements_in_type_arguments() {
        let source = SourceFile::new(
            "main.veln",
            "fn nested(value: Pair<State::Ready | State::Closed, Box<Int>::Full>::Made) -> ()\n  ()\nend\n",
        );
        let parsed = veln_syntax::parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = veln_ast::lower_surface_ast(&parsed.tree);
        let refinement = &module.functions[0].params[0].ty_refinements[0];

        assert_eq!(
            render_refinement(refinement),
            "Pair<State::Ready | State::Closed, Box<Int>::Full>::Made"
        );
        assert_eq!(
            render_type_argument(&refinement.alternatives[0].type_arguments[0]),
            "State::Ready | State::Closed"
        );
    }
}
