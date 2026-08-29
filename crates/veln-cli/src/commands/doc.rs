use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_analysis::{
    DoctestMode, analyze_project, derive_source_module_path, load_surface_module,
    parse_diagnostic_to_envelope, validate_manifest_dependencies, validate_manifest_exports,
};
use veln_ast::{PublicAliasKind as AstPublicAliasKind, SurfaceModule, UseDecl};
use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity};
use veln_project::{
    ManifestField, Project, ProjectManifest, classify_companion_source, production_analysis_inputs,
    read_manifest,
};
use veln_source::{SourceFile, TextRange};
use veln_syntax::{
    AdrLiteAnchor, ContractClause, ContractKind, FunctionDecl, FunctionKind, PublicAliasDecl,
    PublicAliasKind, SchemaDecl, SyntaxItem, TypeDecl, TypeVariantDecl, Visibility,
    canonical_type_text, parse,
};

use crate::diagnostics::{print_human_stderr, tool_info};

mod schema_references;

use schema_references::{doc_schema_reference_diagnostics, render_doc_schema_references};

pub(crate) fn doc(
    start: super::CommandAnalysisStart,
    inputs: Vec<PathBuf>,
) -> Result<ExitCode, String> {
    let inputs = start.resolve_inputs(inputs);
    let root = start.package_root;
    let project = discover_doc_project(root, &inputs).map_err(|error| error.to_string())?;
    let generated = generate_markdown(&project);
    if !generated.diagnostics.is_empty() {
        let envelope = DiagnosticEnvelope::new(tool_info(), generated.diagnostics);
        print_human_stderr(&envelope)?;
        return Ok(ExitCode::from(1));
    }

    print!("{}", generated.markdown);
    Ok(ExitCode::SUCCESS)
}

fn discover_doc_project(root: PathBuf, inputs: &[PathBuf]) -> io::Result<Project> {
    let paths = production_analysis_inputs(&root, inputs)?;
    let mut files = Vec::new();
    for path in paths {
        files.push(SourceFile::read(&root, &path)?);
    }
    let manifest = read_manifest(&root)?;
    Ok(Project {
        root,
        files,
        manifest,
    })
}

struct GeneratedDocs {
    markdown: String,
    diagnostics: Vec<veln_diagnostics::Diagnostic>,
}

fn generate_markdown(project: &Project) -> GeneratedDocs {
    let mut diagnostics = Vec::new();
    let sources = collect_doc_sources(project, &mut diagnostics);
    validate_doc_sources(project, &sources, &mut diagnostics);
    if !diagnostics.is_empty() {
        return GeneratedDocs {
            markdown: String::new(),
            diagnostics,
        };
    }

    GeneratedDocs {
        markdown: render_project_docs(project, &sources),
        diagnostics,
    }
}

fn collect_doc_sources<'a>(
    project: &'a Project,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ParsedDocSource<'a>> {
    let mut sources = Vec::new();

    for source in &project.files {
        if classify_companion_source(source.path().as_str()).is_some() {
            continue;
        }
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

    sources
}

fn validate_doc_sources(
    project: &Project,
    sources: &[ParsedDocSource<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    diagnostics.extend(validate_manifest_exports(project));
    diagnostics.extend(validate_manifest_dependencies(project));
    if diagnostics.is_empty() {
        diagnostics
            .extend(analyze_project(project.clone(), DoctestMode::Exclude).semantic_diagnostics());
    }
    if diagnostics.is_empty() {
        let (surface_module, _) = load_surface_module(project);
        diagnostics.extend(doc_schema_reference_diagnostics(&surface_module, sources));
    }
}

fn render_project_docs(project: &Project, sources: &[ParsedDocSource<'_>]) -> String {
    let mut out = String::new();
    let title = project
        .manifest
        .as_ref()
        .and_then(|manifest| manifest_field(&manifest.package.fields, "name"))
        .unwrap_or("Veln Project");
    push_heading(&mut out, 1, title);
    if let Some(manifest) = &project.manifest {
        push_manifest_docs(&mut out, manifest);
    }

    push_heading(&mut out, 2, "Modules");
    if sources.is_empty() {
        push_paragraph(&mut out, "No source modules selected.");
    } else {
        for source in sources {
            out.push_str(&source_docs(source.source, &source.tree));
        }
    }

    out
}

fn push_manifest_docs(out: &mut String, manifest: &ProjectManifest) {
    if let Some(description) = manifest_field(&manifest.package.fields, "description") {
        push_paragraph(out, description);
    }
    if !manifest.package.fields.is_empty() {
        push_heading(out, 2, "Package");
        push_field_list(out, &manifest.package.fields);
    }
    if !manifest.tools.is_empty() {
        push_heading(out, 2, "Tool Metadata");
        for tool in &manifest.tools {
            push_heading(out, 3, &tool.name);
            push_field_list(out, &tool.fields);
        }
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
    let public_api = PublicApi::from_tree(tree);
    if public_api.is_empty() {
        return;
    }

    push_heading(out, 4, "Public API");
    for type_decl in public_api.types {
        push_public_type(out, source, type_decl);
    }
    for schema in public_api.schemas {
        push_public_schema(out, source, schema);
    }
    for function in public_api.functions {
        push_public_function(out, source, function);
    }
    for alias in public_api.aliases {
        push_public_alias(out, source, alias);
    }
}

struct PublicApi<'a> {
    types: Vec<&'a TypeDecl>,
    schemas: Vec<&'a SchemaDecl>,
    functions: Vec<&'a FunctionDecl>,
    aliases: Vec<&'a PublicAliasDecl>,
}

impl<'a> PublicApi<'a> {
    fn from_tree(tree: &'a veln_syntax::SyntaxTree) -> Self {
        let mut public_api = Self {
            types: Vec::new(),
            schemas: Vec::new(),
            functions: Vec::new(),
            aliases: Vec::new(),
        };
        for item in &tree.items {
            match item {
                SyntaxItem::Type(type_decl) if type_decl.visibility == Visibility::Public => {
                    public_api.types.push(type_decl);
                }
                SyntaxItem::Schema(schema) if schema.visibility == Visibility::Public => {
                    public_api.schemas.push(schema);
                }
                SyntaxItem::Function(function)
                    if function.kind == FunctionKind::Function
                        && function.visibility == Visibility::Public =>
                {
                    public_api.functions.push(function);
                }
                SyntaxItem::PublicAlias(alias) => public_api.aliases.push(alias),
                _ => {}
            }
        }
        public_api
    }

    fn is_empty(&self) -> bool {
        self.types.is_empty()
            && self.schemas.is_empty()
            && self.functions.is_empty()
            && self.aliases.is_empty()
    }
}

fn push_public_type(out: &mut String, source: &SourceFile, type_decl: &TypeDecl) {
    push_heading(out, 5, &type_signature(type_decl));
    push_doc_block(out, doc_block_before(source, type_decl.span.start.line));
    let public_variants = type_decl
        .variants
        .iter()
        .filter(|variant| variant.visibility == Visibility::Public)
        .collect::<Vec<_>>();
    if public_variants.is_empty() {
        return;
    }

    out.push_str("Public constructors:\n\n");
    for variant in public_variants {
        out.push_str(&format!("- `{}`\n", variant_signature(variant)));
    }
    out.push('\n');
}

fn push_public_schema(out: &mut String, source: &SourceFile, schema: &SchemaDecl) {
    push_heading(out, 5, &schema_signature(schema));
    push_doc_block(out, doc_block_before(source, schema.span.start.line));
}

fn push_public_function(out: &mut String, source: &SourceFile, function: &FunctionDecl) {
    push_heading(out, 5, &function_signature(function));
    push_doc_block(out, doc_block_before(source, function.span.start.line));
    push_contracts(out, &function.contracts);
}

fn push_public_alias(out: &mut String, source: &SourceFile, alias: &PublicAliasDecl) {
    push_heading(out, 5, &alias_signature(alias));
    push_doc_block(out, doc_block_before(source, alias.span.start.line));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_doc_generation_preserves_default_structure() {
        let project = Project {
            root: PathBuf::from("."),
            files: vec![SourceFile::new(
                "sample.veln",
                concat!(
                    "## Returns the supplied value.\n",
                    "pub fn identity(value: Int) -> Int\n",
                    "\tvalue\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };

        let generated = generate_markdown(&project);

        assert!(generated.diagnostics.is_empty());
        assert_eq!(
            generated.markdown,
            concat!(
                "# Veln Project\n\n",
                "## Modules\n\n",
                "### sample\n\n",
                "Source: `sample.veln`\n\n",
                "#### Public API\n\n",
                "##### fn identity(value: Int) -> Int\n\n",
                "Returns the supplied value.\n\n",
            )
        );
    }

    #[test]
    fn public_api_rendering_preserves_category_order_and_visibility() {
        let source = SourceFile::new(
            "sample.veln",
            concat!(
                "pub fn exported() -> Int\n",
                "\t1\n",
                "end\n",
                "fn hidden() -> Int\n",
                "\t2\n",
                "end\n",
                "pub schema Packet\n",
                "\tformat binary\n",
                "\n",
                "\tvalue: UInt8\n",
                "end\n",
                "pub type Visible\n",
                "\tpub Wrap(Int)\n",
                "\tHidden(Int)\n",
                "end\n",
                "pub type Alias = Visible\n",
            ),
        );
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "fixture should parse: {:#?}",
            parsed.diagnostics
        );

        let markdown = source_docs(&source, &parsed.tree);
        let type_position = markdown.find("##### type Visible").expect("public type");
        let schema_position = markdown.find("##### schema Packet").expect("public schema");
        let function_position = markdown
            .find("##### fn exported() -> Int")
            .expect("public function");
        let alias_position = markdown
            .find("##### type Alias = Visible")
            .expect("public alias");

        assert!(type_position < schema_position);
        assert!(schema_position < function_position);
        assert!(function_position < alias_position);
        assert!(
            markdown.contains("Public constructors:\n\n- `Wrap { value: Int }`"),
            "rendered markdown:\n{markdown}"
        );
        assert!(!markdown.contains("Hidden { value: Int }"));
        assert!(!markdown.contains("hidden()"));
    }
}
