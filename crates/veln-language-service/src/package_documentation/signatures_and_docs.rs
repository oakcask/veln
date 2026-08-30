use super::*;

pub(super) fn function_signature(function: &FunctionDecl) -> String {
    veln_syntax::declaration_function_signature(function, true)
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
    render_documentation_lines(documentation_block_before(source, target_line))
}

pub(super) fn module_doc(source: &SourceFile, tree: &veln_syntax::SyntaxTree) -> Vec<String> {
    tree.module
        .as_ref()
        .map(|module| doc_block_before(source, module.span.start.line))
        .unwrap_or_default()
}

pub(super) fn doc_schema_references_before(
    source: &SourceFile,
    target_line: usize,
) -> Vec<veln_syntax::DocumentationSchemaReference> {
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
    if documentation_lines_are_adr_lite(docs.iter().map(|(_, line)| *line)) {
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
                references.extend(extract_documentation_schema_references(
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
