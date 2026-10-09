use veln_source::{SourceFile, SourceSpan, TextRange};

use crate::{
    FunctionDecl, TypeDecl, TypeVariantDecl, VariantRefinementType, canonical_type_text,
    format::canonical_structured_type_text,
};

#[derive(Clone, Debug)]
pub struct DocumentationSchemaReference {
    pub target: String,
    pub span: SourceSpan,
}

pub fn extract_documentation_schema_references(
    source: &SourceFile,
    text: &str,
    text_start: usize,
) -> Vec<DocumentationSchemaReference> {
    let mut references = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = text[cursor..].find("{@schema") {
        let marker_start = cursor + relative_start;
        let after_marker = marker_start + "{@schema".len();
        let Some(next) = text[after_marker..].chars().next() else {
            break;
        };
        if !next.is_whitespace() {
            cursor = after_marker;
            continue;
        }
        let after_space = after_marker
            + text[after_marker..]
                .char_indices()
                .find(|(_, character)| !character.is_whitespace())
                .map_or(0, |(index, _)| index);
        let Some(relative_end) = text[after_space..].find('}') else {
            break;
        };
        let marker_end = after_space + relative_end;
        let target_text = &text[after_space..marker_end];
        let leading_trim = target_text.len() - target_text.trim_start().len();
        let trailing_trim = target_text.trim_end().len();
        let target = target_text.trim().to_string();
        if !target.is_empty() {
            references.push(DocumentationSchemaReference {
                target,
                span: source.span(TextRange::new(
                    text_start + after_space + leading_trim,
                    text_start + after_space + trailing_trim,
                )),
            });
        }
        cursor = marker_end + 1;
    }
    references
}

pub fn declaration_function_signature(
    function: &FunctionDecl,
    include_effect_binder: bool,
) -> String {
    declaration_function_signature_inner(function, include_effect_binder, None)
}

pub fn declaration_function_signature_from_source(
    function: &FunctionDecl,
    include_effect_binder: bool,
    source: &SourceFile,
) -> String {
    declaration_function_signature_inner(function, include_effect_binder, Some(source.text()))
}

fn declaration_function_signature_inner(
    function: &FunctionDecl,
    include_effect_binder: bool,
    source: Option<&str>,
) -> String {
    let mut signature = String::from("fn ");
    signature.push_str(function.name.as_deref().unwrap_or("<anonymous>"));
    if include_effect_binder && let Some(binder) = &function.effect_binder {
        signature.push_str("<effect ");
        signature.push_str(&binder.name);
        signature.push('>');
    }
    signature.push('(');
    signature.push_str(
        &function
            .params
            .iter()
            .map(|param| match &param.ty {
                Some(ty) if param.is_variadic => format!(
                    "{}: ...{}",
                    param.name,
                    documentation_type_text(ty, &param.ty_refinements, source)
                ),
                Some(ty) => format!(
                    "{}: {}",
                    param.name,
                    documentation_type_text(ty, &param.ty_refinements, source)
                ),
                None => param.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    signature.push(')');
    if let Some(return_type) = &function.return_type {
        signature.push_str(" -> ");
        if let Some(binding) = &function.return_binding {
            signature.push_str(&binding.name);
            signature.push_str(": ");
        }
        signature.push_str(&documentation_type_text(
            return_type,
            &function.return_type_refinements,
            source,
        ));
    }
    if let Some(effects) = &function.effects {
        signature.push_str(" effects [");
        signature.push_str(&effects.join(", "));
        signature.push(']');
    }
    if function.callsite.is_some() {
        signature.push_str(" callsite");
    }
    signature
}

fn documentation_type_text(
    text: &str,
    refinements: &[VariantRefinementType],
    source: Option<&str>,
) -> String {
    source.map_or_else(
        || canonical_type_text(text),
        |source| canonical_type_text(&canonical_structured_type_text(text, refinements, source)),
    )
}

pub fn declaration_type_signature(type_decl: &TypeDecl) -> String {
    let mut signature = String::from("type ");
    signature.push_str(type_decl.name.as_deref().unwrap_or("<anonymous>"));
    if !type_decl.params.is_empty() {
        signature.push('<');
        signature.push_str(&type_decl.params.join(", "));
        signature.push('>');
    }
    signature
}

pub fn declaration_variant_signature(variant: &TypeVariantDecl) -> String {
    declaration_variant_signature_inner(variant, None)
}

pub fn declaration_variant_signature_from_source(
    variant: &TypeVariantDecl,
    source: &SourceFile,
) -> String {
    declaration_variant_signature_inner(variant, Some(source.text()))
}

fn declaration_variant_signature_inner(variant: &TypeVariantDecl, source: Option<&str>) -> String {
    let name = variant.name.as_deref().unwrap_or("<anonymous>");
    if variant.fields.is_empty() {
        return name.to_string();
    }
    if variant.fields.iter().all(|field| !field.name.is_empty()) {
        let fields = variant
            .fields
            .iter()
            .map(|field| {
                format!(
                    "{}: {}",
                    field.name,
                    documentation_type_text(&field.ty, &field.ty_refinements, source)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        return format!("{name} {{ {fields} }}");
    }
    let fields = variant
        .fields
        .iter()
        .map(|field| documentation_type_text(&field.ty, &field.ty_refinements, source))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{name}({fields})")
}

pub fn documentation_lines_are_adr_lite<'a>(lines: impl IntoIterator<Item = &'a str>) -> bool {
    lines
        .into_iter()
        .filter_map(|line| line.trim_start().strip_prefix("##"))
        .map(str::trim_start)
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| matches!(line.trim(), "@adr" | "@adr-lite"))
}

pub fn documentation_block_before(source: &SourceFile, target_line: usize) -> Vec<String> {
    if target_line <= 1 {
        return Vec::new();
    }
    let lines = source.text().lines().collect::<Vec<_>>();
    let mut index = target_line - 2;
    let mut docs = Vec::new();

    while let Some(line) = lines.get(index) {
        let trimmed = line.trim_start();
        if let Some(content) = trimmed.strip_prefix("##") {
            docs.push(content.trim_start().to_string());
        } else {
            break;
        }
        if index == 0 {
            break;
        }
        index -= 1;
    }

    docs.reverse();
    if docs
        .iter()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| matches!(line.trim(), "@adr" | "@adr-lite"))
    {
        return Vec::new();
    }
    docs
}

pub fn render_documentation_lines(lines: Vec<String>) -> Vec<String> {
    let mut rendered = Vec::new();
    let mut in_veln_fence = false;
    for line in lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_veln_fence =
                trimmed.starts_with("```veln") && !trimmed.starts_with("```veln-output");
            rendered.push(line);
            continue;
        }
        if in_veln_fence && line.starts_with("> ") {
            continue;
        }
        rendered.push(line);
    }
    rendered
}
