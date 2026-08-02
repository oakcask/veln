use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_analysis::parse_diagnostic_to_envelope;
use veln_analysis::{
    derive_source_module_path, load_surface_module, validate_manifest_dependencies,
    validate_manifest_exports,
};
use veln_ast::{PublicAliasKind as AstPublicAliasKind, SurfaceModule, UseDecl};
use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity};
use veln_project::{ManifestField, Project};
use veln_source::{SourceFile, TextRange};
use veln_syntax::{
    AdrLiteAnchor, ContractClause, ContractKind, FunctionDecl, FunctionKind, PublicAliasDecl,
    PublicAliasKind, SchemaDecl, SyntaxItem, TypeDecl, TypeVariantDecl, Visibility,
    canonical_type_text, parse,
};

use crate::diagnostics::{print_human_stderr, tool_info};

pub(crate) fn doc(inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let generated = generate_markdown(&project);
    if !generated.diagnostics.is_empty() {
        let envelope = DiagnosticEnvelope::new(tool_info(), generated.diagnostics);
        print_human_stderr(&envelope)?;
        return Ok(ExitCode::from(1));
    }

    print!("{}", generated.markdown);
    Ok(ExitCode::SUCCESS)
}

struct GeneratedDocs {
    markdown: String,
    diagnostics: Vec<veln_diagnostics::Diagnostic>,
}

fn generate_markdown(project: &Project) -> GeneratedDocs {
    let mut diagnostics = Vec::new();
    let mut sources = Vec::new();

    for source in &project.files {
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        let module_name = match derive_source_module_path(source) {
            Ok(module_name) => Some(module_name),
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
                None
            }
        };
        sources.push(ParsedDocSource {
            source,
            tree: parsed.tree,
            module_name,
        });
    }
    diagnostics.extend(validate_manifest_exports(project));
    diagnostics.extend(validate_manifest_dependencies(project));
    if diagnostics.is_empty() {
        let (surface_module, _) = load_surface_module(project);
        diagnostics.extend(doc_schema_reference_diagnostics(&surface_module, &sources));
    }

    if !diagnostics.is_empty() {
        return GeneratedDocs {
            markdown: String::new(),
            diagnostics,
        };
    }

    let mut out = String::new();
    let title = project
        .manifest
        .as_ref()
        .and_then(|manifest| manifest_field(&manifest.package.fields, "name"))
        .unwrap_or("Veln Project");
    push_heading(&mut out, 1, title);
    if let Some(manifest) = &project.manifest {
        if let Some(description) = manifest_field(&manifest.package.fields, "description") {
            push_paragraph(&mut out, description);
        }
        if !manifest.package.fields.is_empty() {
            push_heading(&mut out, 2, "Package");
            push_field_list(&mut out, &manifest.package.fields);
        }
        if !manifest.tools.is_empty() {
            push_heading(&mut out, 2, "Tool Metadata");
            for tool in &manifest.tools {
                push_heading(&mut out, 3, &tool.name);
                push_field_list(&mut out, &tool.fields);
            }
        }
    }

    push_heading(&mut out, 2, "Modules");
    if sources.is_empty() {
        push_paragraph(&mut out, "No source modules selected.");
    } else {
        for source in sources {
            out.push_str(&source_docs(source.source, &source.tree));
        }
    }

    GeneratedDocs {
        markdown: out,
        diagnostics,
    }
}

struct ParsedDocSource<'a> {
    source: &'a SourceFile,
    tree: veln_syntax::SyntaxTree,
    module_name: Option<String>,
}

fn source_docs(source: &SourceFile, tree: &veln_syntax::SyntaxTree) -> String {
    let mut out = String::new();
    push_module_header(&mut out, source);
    push_imports(&mut out, tree);
    push_public_api(&mut out, source, tree);
    push_adr_lite_records(&mut out, tree);
    out
}

fn push_module_header(out: &mut String, source: &SourceFile) {
    let module_name = derive_source_module_path(source).unwrap_or_else(|_| "<invalid>".to_string());
    push_heading(out, 3, &module_name);
    push_paragraph(out, &format!("Source: `{}`", source.path().as_str()));

    push_doc_block(out, doc_block_before(source, 1));
}

fn push_imports(out: &mut String, tree: &veln_syntax::SyntaxTree) {
    if !tree.uses.is_empty() {
        push_heading(out, 4, "Imports");
        for import in &tree.uses {
            match &import.package {
                Some(package) => {
                    out.push_str(&format!("- `{} from \"{}\"`\n", import.name, package.name));
                }
                None => out.push_str(&format!("- `{}`\n", import.name)),
            }
        }
        out.push('\n');
    }
}

fn push_public_api(out: &mut String, source: &SourceFile, tree: &veln_syntax::SyntaxTree) {
    let public_types = tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Type(type_decl) if type_decl.visibility == Visibility::Public => {
                Some(type_decl)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let public_schemas = tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Schema(schema) if schema.visibility == Visibility::Public => Some(schema),
            _ => None,
        })
        .collect::<Vec<_>>();
    let public_functions = tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Function(function)
                if function.kind == FunctionKind::Function
                    && function.visibility == Visibility::Public =>
            {
                Some(function)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let public_aliases = tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::PublicAlias(alias) => Some(alias),
            _ => None,
        })
        .collect::<Vec<_>>();

    if !public_types.is_empty()
        || !public_schemas.is_empty()
        || !public_functions.is_empty()
        || !public_aliases.is_empty()
    {
        push_heading(out, 4, "Public API");
    }
    for type_decl in public_types {
        push_heading(out, 5, &type_signature(type_decl));
        push_doc_block(out, doc_block_before(source, type_decl.span.start.line));
        let variants = type_decl
            .variants
            .iter()
            .filter(|variant| variant.visibility == Visibility::Public)
            .collect::<Vec<_>>();
        if !variants.is_empty() {
            out.push_str("Public constructors:\n\n");
            for variant in variants {
                out.push_str(&format!("- `{}`\n", variant_signature(variant)));
            }
            out.push('\n');
        }
    }
    for schema in public_schemas {
        push_heading(out, 5, &schema_signature(schema));
        push_doc_block(out, doc_block_before(source, schema.span.start.line));
    }
    for function in public_functions {
        push_heading(out, 5, &function_signature(function));
        push_doc_block(out, doc_block_before(source, function.span.start.line));
        push_contracts(out, &function.contracts);
    }
    for alias in public_aliases {
        push_heading(out, 5, &alias_signature(alias));
        push_doc_block(out, doc_block_before(source, alias.span.start.line));
    }
}

fn push_adr_lite_records(out: &mut String, tree: &veln_syntax::SyntaxTree) {
    if !tree.adr_lite_records.is_empty() {
        push_heading(out, 4, "ADR-Lite Records");
        for record in &tree.adr_lite_records {
            out.push_str(&format!("- `{}` ({})", record.id, record.status));
            if let Some(anchor) = &record.anchor {
                out.push_str(&format!(" anchored to {}", anchor_text(anchor)));
            }
            out.push('\n');
            out.push_str(&format!("  - scope: {}\n", record.scope));
            out.push_str(&format!("  - decision: {}\n", record.decision));
            out.push_str(&format!("  - consequences: {}\n", record.consequences));
        }
        out.push('\n');
    }
}

fn push_heading(out: &mut String, level: usize, text: &str) {
    out.push_str(&format!("{} {text}\n\n", "#".repeat(level)));
}

fn push_paragraph(out: &mut String, text: &str) {
    out.push_str(text);
    out.push_str("\n\n");
}

fn push_field_list(out: &mut String, fields: &[ManifestField]) {
    for field in fields {
        out.push_str(&format!("- {}: `{}`\n", field.key, field.value));
    }
    out.push('\n');
}

fn push_doc_block(out: &mut String, lines: Vec<String>) {
    let lines = rendered_doc_lines(lines);
    if lines.is_empty() {
        return;
    }
    for line in lines {
        out.push_str(&render_doc_schema_references(&line));
        out.push('\n');
    }
    out.push('\n');
}

fn push_contracts(out: &mut String, contracts: &[ContractClause]) {
    if contracts.is_empty() {
        return;
    }
    out.push_str("Contracts:\n\n");
    for contract in contracts {
        out.push_str(&format!(
            "- `{}` {}\n",
            contract_kind_text(contract.kind),
            contract.text
        ));
    }
    out.push('\n');
}

fn doc_block_before(source: &SourceFile, target_line: usize) -> Vec<String> {
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

fn rendered_doc_lines(lines: Vec<String>) -> Vec<String> {
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

fn function_signature(function: &FunctionDecl) -> String {
    let mut signature = String::from("fn ");
    signature.push_str(function.name.as_deref().unwrap_or("<anonymous>"));
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

fn type_signature(type_decl: &TypeDecl) -> String {
    let mut signature = String::from("type ");
    signature.push_str(type_decl.name.as_deref().unwrap_or("<anonymous>"));
    if !type_decl.params.is_empty() {
        signature.push('<');
        signature.push_str(&type_decl.params.join(", "));
        signature.push('>');
    }
    signature
}

fn alias_signature(alias: &PublicAliasDecl) -> String {
    let kind = match alias.kind {
        PublicAliasKind::Function => "fn",
        PublicAliasKind::Type => "type",
        PublicAliasKind::Schema => "schema",
    };
    format!(
        "{kind} {} = {}",
        alias.name.as_deref().unwrap_or("<anonymous>"),
        alias.target.join("::")
    )
}

fn schema_signature(schema: &SchemaDecl) -> String {
    let mut signature = String::from("schema ");
    signature.push_str(schema.name.as_deref().unwrap_or("<anonymous>"));
    signature
}

fn variant_signature(variant: &TypeVariantDecl) -> String {
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

#[derive(Clone, Debug)]
struct DocSchemaReference {
    target: String,
    span: veln_source::SourceSpan,
}

#[derive(Clone, Copy, Debug)]
enum DocSchemaResolution {
    Resolved,
    Private,
    WrongKind(&'static str),
    Unresolved,
}

fn doc_schema_reference_diagnostics(
    module: &SurfaceModule,
    sources: &[ParsedDocSource<'_>],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for source in sources {
        for reference in doc_schema_references(source.source) {
            match resolve_doc_schema_reference(
                module,
                &reference.target,
                source.module_name.as_deref(),
                &mut Vec::new(),
            ) {
                DocSchemaResolution::Resolved => {}
                DocSchemaResolution::Private => diagnostics.push(private_doc_schema_diagnostic(
                    &reference.target,
                    reference.span,
                )),
                DocSchemaResolution::WrongKind(actual_kind) => {
                    diagnostics.push(doc_schema_kind_mismatch_diagnostic(
                        &reference.target,
                        actual_kind,
                        reference.span,
                    ))
                }
                DocSchemaResolution::Unresolved => diagnostics.push(
                    unresolved_doc_schema_diagnostic(&reference.target, reference.span),
                ),
            }
        }
    }
    diagnostics
}

fn doc_schema_references(source: &SourceFile) -> Vec<DocSchemaReference> {
    let mut references = Vec::new();
    let mut line_start = 0;
    for line in source.text().split_inclusive('\n') {
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
        line_start += line.len();
    }
    references
}

fn extract_doc_schema_references(
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
            let start = text_start + after_space + leading_trim;
            let end = text_start + after_space + trailing_trim;
            references.push(DocSchemaReference {
                target,
                span: source.span(TextRange::new(start, end)),
            });
        }
        cursor = marker_end + 1;
    }
    references
}

fn render_doc_schema_references(line: &str) -> String {
    let mut rendered = String::new();
    let mut cursor = 0;
    while let Some(relative_start) = line[cursor..].find("{@schema") {
        let marker_start = cursor + relative_start;
        let after_marker = marker_start + "{@schema".len();
        let Some(next) = line[after_marker..].chars().next() else {
            break;
        };
        if !next.is_whitespace() {
            rendered.push_str(&line[cursor..after_marker]);
            cursor = after_marker;
            continue;
        }
        let after_space = after_marker
            + line[after_marker..]
                .char_indices()
                .find(|(_, ch)| !ch.is_whitespace())
                .map_or(0, |(index, _)| index);
        let Some(relative_end) = line[after_space..].find('}') else {
            break;
        };
        let marker_end = after_space + relative_end;
        let target = line[after_space..marker_end].trim();
        rendered.push_str(&line[cursor..marker_start]);
        if target.is_empty() {
            rendered.push_str(&line[marker_start..=marker_end]);
        } else {
            rendered.push('`');
            rendered.push_str(target);
            rendered.push('`');
        }
        cursor = marker_end + 1;
    }
    rendered.push_str(&line[cursor..]);
    rendered
}

fn resolve_doc_schema_reference(
    module: &SurfaceModule,
    target: &str,
    current_module: Option<&str>,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> DocSchemaResolution {
    let segments = target.split("::").map(str::to_string).collect::<Vec<_>>();
    resolve_doc_schema_segments(module, &segments, current_module, true, visited_aliases)
}

fn resolve_doc_schema_segments(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> DocSchemaResolution {
    match segments {
        [name] => resolve_doc_schema_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
        ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            ) else {
                return DocSchemaResolution::Unresolved;
            };
            resolve_doc_schema_in_module(module, Some(&use_decl.name), name, false, visited_aliases)
        }
        _ => DocSchemaResolution::Unresolved,
    }
}

fn resolve_doc_schema_in_module(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> DocSchemaResolution {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return if allow_private_schema || schema.visibility == veln_ast::Visibility::Public {
            DocSchemaResolution::Resolved
        } else {
            DocSchemaResolution::Private
        };
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.kind == AstPublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    }) {
        return resolve_doc_schema_alias_target(module, alias, visited_aliases);
    }
    doc_schema_wrong_kind(module, module_name, name).map_or(
        DocSchemaResolution::Unresolved,
        DocSchemaResolution::WrongKind,
    )
}

fn resolve_doc_schema_alias_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> DocSchemaResolution {
    let Some(name) = &alias.name else {
        return DocSchemaResolution::Unresolved;
    };
    let key = (alias.module_name.clone(), name.clone());
    if visited_aliases.contains(&key) {
        return DocSchemaResolution::Unresolved;
    }
    visited_aliases.push(key);
    let resolution = resolve_doc_schema_segments(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
    );
    visited_aliases.pop();
    resolution
}

fn imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

fn doc_schema_wrong_kind(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
) -> Option<&'static str> {
    if module.functions.iter().any(|function| {
        function.kind == veln_ast::FunctionKind::Function
            && function.name.as_deref() == Some(name)
            && function.module_name.as_deref() == module_name
    }) {
        return Some("function");
    }
    if module.types.iter().any(|type_decl| {
        type_decl.name.as_deref() == Some(name) && type_decl.module_name.as_deref() == module_name
    }) {
        return Some("type");
    }
    if module.codecs.iter().any(|codec| {
        codec.name.as_deref() == Some(name) && codec.module_name.as_deref() == module_name
    }) {
        return Some("codec");
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.name.as_deref() == Some(name) && alias.module_name.as_deref() == module_name
    }) {
        return match alias.kind {
            AstPublicAliasKind::Function => Some("function"),
            AstPublicAliasKind::Type => Some("type"),
            AstPublicAliasKind::Schema => None,
        };
    }
    None
}

fn unresolved_doc_schema_diagnostic(target: &str, span: veln_source::SourceSpan) -> Diagnostic {
    Diagnostic::new(
        "name.unresolved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("unresolved documentation schema reference `{target}`"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("expected_kind", JsonValue::string("schema")),
            ("target", JsonValue::string(target.to_string())),
        ]),
    )
}

fn private_doc_schema_diagnostic(target: &str, span: veln_source::SourceSpan) -> Diagnostic {
    Diagnostic::new(
        "name.visibility",
        Severity::Error,
        DiagnosticKind::Name,
        format!("documentation schema reference `{target}` is private"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("expected_kind", JsonValue::string("schema")),
            ("target", JsonValue::string(target.to_string())),
            ("visibility", JsonValue::string("private")),
        ]),
    )
}

fn doc_schema_kind_mismatch_diagnostic(
    target: &str,
    actual_kind: &'static str,
    span: veln_source::SourceSpan,
) -> Diagnostic {
    Diagnostic::new(
        "name.kind_mismatch",
        Severity::Error,
        DiagnosticKind::Name,
        format!("documentation schema reference `{target}` is a {actual_kind}, not a schema"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("expected_kind", JsonValue::string("schema")),
            ("actual_kind", JsonValue::string(actual_kind)),
            ("target", JsonValue::string(target.to_string())),
        ]),
    )
}

fn contract_kind_text(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::Require => "require",
        ContractKind::Ensure => "ensure",
        ContractKind::Invariant => "invariant",
    }
}

fn anchor_text(anchor: &AdrLiteAnchor) -> String {
    match anchor {
        AdrLiteAnchor::Module { name } => format!("module `{name}`"),
        AdrLiteAnchor::Function { name } => format!("function `{name}`"),
    }
}

fn manifest_field<'a>(fields: &'a [ManifestField], key: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|field| field.key == key)
        .map(|field| field.value.as_str())
}
