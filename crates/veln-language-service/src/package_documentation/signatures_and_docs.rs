use super::*;

pub(super) fn doc_lines_are_adr_lite<'a>(lines: impl IntoIterator<Item = &'a str>) -> bool {
    lines
        .into_iter()
        .filter_map(|line| line.trim_start().strip_prefix("##"))
        .map(str::trim_start)
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| matches!(line.trim(), "@adr" | "@adr-lite"))
}

pub(super) fn function_signature(function: &FunctionDecl) -> String {
    let mut signature = String::from("fn ");
    signature.push_str(function.name.as_deref().unwrap_or("<anonymous>"));
    if let Some(binder) = &function.effect_binder {
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
                Some(ty) if param.is_variadic => {
                    format!("{}: ...{}", param.name, canonical_type_text(ty))
                }
                Some(ty) => format!("{}: {}", param.name, canonical_type_text(ty)),
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
        signature.push_str(&canonical_type_text(return_type));
    }
    if let Some(effects) = &function.effects {
        signature.push_str(" effects [");
        signature.push_str(&effects.join(", "));
        signature.push(']');
    }
    signature
}

pub(super) fn public_documentation_lines(tree: &veln_syntax::SyntaxTree) -> Vec<usize> {
    let mut lines = Vec::new();
    if let Some(module) = &tree.module {
        lines.push(module.span.start.line);
    }
    lines.extend(
        tree.items
            .iter()
            .flat_map(|item| match item {
                SyntaxItem::Type(type_decl) if type_decl.visibility == Visibility::Public => {
                    let mut lines = Vec::with_capacity(type_decl.variants.len() + 1);
                    lines.push(type_decl.span.start.line);
                    lines.extend(
                        type_decl
                            .variants
                            .iter()
                            .filter(|variant| variant.visibility == Visibility::Public)
                            .map(|variant| variant.span.start.line),
                    );
                    lines
                }
                SyntaxItem::Schema(schema) if schema.visibility == Visibility::Public => {
                    vec![schema.span.start.line]
                }
                SyntaxItem::Function(function)
                    if function.kind == FunctionKind::Function
                        && function.visibility == Visibility::Public =>
                {
                    vec![function.span.start.line]
                }
                SyntaxItem::PublicAlias(alias) => vec![alias.span.start.line],
                _ => Vec::new(),
            })
            .collect::<Vec<_>>(),
    );
    lines.sort_unstable();
    lines.dedup();
    lines
}

pub(super) fn record_declaration_location(
    source: &SourceFile,
    source_uri: &str,
    declaration_locations: &mut BTreeMap<PackageDocLocationKey, String>,
    declaration_id: &str,
    declaration_span: &SourceSpan,
    declaration_name: Option<&str>,
) {
    declaration_locations.insert(
        PackageDocLocationKey::new(source_uri, declaration_span),
        declaration_id.to_string(),
    );
    if let Some(name_span) = declaration_name
        .and_then(|name| name_span_in(source, declaration_span, name))
        .filter(|name_span| name_span.start.offset != declaration_span.start.offset)
    {
        declaration_locations.insert(
            PackageDocLocationKey::new(source_uri, &name_span),
            declaration_id.to_string(),
        );
    }
}

pub(super) fn name_span_in(
    source: &SourceFile,
    span: &SourceSpan,
    name: &str,
) -> Option<SourceSpan> {
    lex(source)
        .tokens
        .into_iter()
        .find(|token| {
            token.kind == TokenKind::Ident
                && token.text == name
                && token.range.start >= span.start.offset
                && token.range.end <= span.end.offset
        })
        .map(|token| source.span(token.range))
}

pub(super) fn type_signature(type_decl: &TypeDecl) -> String {
    let mut signature = String::from("type ");
    signature.push_str(type_decl.name.as_deref().unwrap_or("<anonymous>"));
    if !type_decl.params.is_empty() {
        signature.push('<');
        signature.push_str(&type_decl.params.join(", "));
        signature.push('>');
    }
    signature
}

pub(super) fn schema_signature(schema: &SchemaDecl) -> String {
    format!("schema {}", schema.name.as_deref().unwrap_or("<anonymous>"))
}

pub(super) fn alias_signature(alias: &PublicAliasDecl) -> String {
    format!(
        "{} {} = {}",
        alias_kind(alias.kind),
        alias.name.as_deref().unwrap_or("<anonymous>"),
        alias.target.join("::")
    )
}

pub(super) fn alias_kind(kind: PublicAliasKind) -> &'static str {
    match kind {
        PublicAliasKind::Function => "function",
        PublicAliasKind::Type => "type",
        PublicAliasKind::Schema => "schema",
    }
}

pub(super) fn variant_signature(variant: &TypeVariantDecl) -> String {
    let name = variant.name.as_deref().unwrap_or("<anonymous>");
    if variant.fields.is_empty() {
        return name.to_string();
    }
    if variant.fields.iter().all(|field| !field.name.is_empty()) {
        let fields = variant
            .fields
            .iter()
            .map(|field| format!("{}: {}", field.name, canonical_type_text(&field.ty)))
            .collect::<Vec<_>>()
            .join(", ");
        return format!("{name} {{ {fields} }}");
    }
    let fields = variant
        .fields
        .iter()
        .map(|field| canonical_type_text(&field.ty))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{name}({fields})")
}

pub(super) fn function_contract(contract: &ContractClause) -> PackageDocFunctionContract {
    PackageDocFunctionContract {
        kind: match contract.kind {
            ContractKind::Require => "require",
            ContractKind::Ensure => "ensure",
            ContractKind::Invariant => "invariant",
        }
        .to_string(),
        text: contract.text.clone(),
    }
}

pub(super) fn doc_block_before(source: &SourceFile, target_line: usize) -> Vec<String> {
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
    rendered_doc_lines(docs)
}

pub(super) fn module_doc(source: &SourceFile, tree: &veln_syntax::SyntaxTree) -> Vec<String> {
    tree.module
        .as_ref()
        .map(|module| doc_block_before(source, module.span.start.line))
        .unwrap_or_default()
}

pub(super) fn rendered_doc_lines(lines: Vec<String>) -> Vec<String> {
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

#[derive(Clone)]
pub(super) struct DocSchemaReference {
    pub(super) target: String,
    pub(super) span: SourceSpan,
}

pub(super) fn doc_schema_references_before(
    source: &SourceFile,
    target_line: usize,
) -> Vec<DocSchemaReference> {
    if target_line <= 1 {
        return Vec::new();
    }
    let lines = source.text().split_inclusive('\n').collect::<Vec<_>>();
    let mut index = target_line - 2;
    let mut docs = Vec::new();
    while let Some(line) = lines.get(index) {
        let trimmed = line.trim_start();
        if trimmed.strip_prefix("##").is_none() {
            break;
        }
        docs.push((index, *line));
        if index == 0 {
            break;
        }
        index -= 1;
    }
    docs.reverse();
    if doc_lines_are_adr_lite(docs.iter().map(|(_, line)| *line)) {
        return Vec::new();
    }

    let mut references = Vec::new();
    let mut line_start = 0;
    for (line_index, line) in lines.iter().enumerate() {
        if docs.iter().any(|(index, _)| *index == line_index) {
            let trimmed = line.trim_start();
            let indent_len = line.len() - trimmed.len();
            if let Some(content) = trimmed.strip_prefix("##") {
                let content_start = line_start + indent_len + "##".len();
                references.extend(extract_doc_schema_references(
                    source,
                    content,
                    content_start,
                ));
            }
        }
        line_start += line.len();
    }
    references
}

pub(super) fn extract_doc_schema_references(
    source: &SourceFile,
    text: &str,
    text_start: usize,
) -> Vec<DocSchemaReference> {
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
                .find(|(_, ch)| !ch.is_whitespace())
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
            references.push(DocSchemaReference {
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
