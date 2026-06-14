use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_analysis::{
    derive_source_module_path, validate_manifest_dependencies, validate_manifest_exports,
};
use veln_diagnostics::DiagnosticEnvelope;
use veln_project::{ManifestField, Project};
use veln_source::SourceFile;
use veln_syntax::{
    AdrLiteAnchor, ContractClause, ContractKind, FunctionDecl, FunctionKind, PublicAliasDecl,
    PublicAliasKind, SyntaxItem, TypeDecl, TypeVariantDecl, Visibility, canonical_type_text, parse,
};

use crate::diagnostics::{parse_diagnostic_to_envelope, print_human_stderr, tool_info};

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
    let mut sections = Vec::new();

    for source in &project.files {
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        match derive_source_module_path(source) {
            Ok(_) => {}
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
            }
        }
        sections.push(source_docs(source, &parsed.tree));
    }
    diagnostics.extend(validate_manifest_exports(project));
    diagnostics.extend(validate_manifest_dependencies(project));

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
    if sections.is_empty() {
        push_paragraph(&mut out, "No source modules selected.");
    } else {
        for section in sections {
            out.push_str(&section);
        }
    }

    GeneratedDocs {
        markdown: out,
        diagnostics,
    }
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

    if !public_types.is_empty() || !public_functions.is_empty() || !public_aliases.is_empty() {
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
        out.push_str(&line);
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
