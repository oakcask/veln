use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedPackageDocResource {
    pub uri: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub mime_type: &'static str,
    pub text: String,
    pub listed: bool,
}

pub fn render_package_documentation(result: &PackageDocResult) -> Vec<RenderedPackageDocResource> {
    match result.kind() {
        PackageDocResultKind::Catalog(catalog) => render_catalog(result, catalog),
        PackageDocResultKind::Status(status) => vec![render_status(result, status, true)],
    }
}

fn render_catalog(
    result: &PackageDocResult,
    catalog: &PackageDocCatalog,
) -> Vec<RenderedPackageDocResource> {
    let mut resources = vec![RenderedPackageDocResource {
        uri: catalog.index_uri.clone(),
        name: package_index_name(catalog),
        title: format!("Veln package documentation: {}", catalog.package_identity),
        description: catalog.metadata.description.clone(),
        mime_type: PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE,
        text: render_index(result, catalog),
        listed: true,
    }];
    for module in &catalog.modules {
        resources.push(RenderedPackageDocResource {
            uri: module.uri.clone(),
            name: module.name.clone(),
            title: format!("Veln package module: {}", module.name),
            description: first_doc_line(&module.doc),
            mime_type: PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE,
            text: render_module(result, catalog, module),
            listed: false,
        });
        for declaration in &module.declarations {
            resources.push(RenderedPackageDocResource {
                uri: declaration.uri.clone(),
                name: declaration.name.clone(),
                title: format!(
                    "Veln package declaration: {} {}",
                    declaration.kind, declaration.name
                ),
                description: first_doc_line(&declaration.doc),
                mime_type: PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE,
                text: render_declaration(result, catalog, module, declaration),
                listed: false,
            });
        }
    }
    resources
}

fn render_index(result: &PackageDocResult, catalog: &PackageDocCatalog) -> String {
    let mut out = String::new();
    heading(
        &mut out,
        1,
        &format!("Package Documentation: {}", catalog.package_identity),
    );
    facts(
        &mut out,
        [
            ("Schema version", catalog.schema_version.as_str()),
            ("Generator contract", catalog.generator_contract.as_str()),
            ("Package identity", catalog.package_identity.as_str()),
            ("Snapshot digest", catalog.snapshot_digest.as_str()),
            ("Documentation digest", result.doc_digest()),
            ("Status URI", catalog.status_uri.as_str()),
        ],
    );
    metadata(&mut out, &catalog.metadata);
    heading(&mut out, 2, "Modules");
    if catalog.modules.is_empty() {
        out.push_str("No public modules are published.\n");
    } else {
        for module in &catalog.modules {
            out.push_str("- [");
            out.push_str(&module.name);
            out.push_str("](");
            out.push_str(&module.uri);
            out.push_str(") - ");
            out.push_str(&module.source_path);
            out.push('\n');
        }
    }
    out.push('\n');
    status_summary(&mut out, &catalog.status);
    out
}

fn render_module(
    result: &PackageDocResult,
    catalog: &PackageDocCatalog,
    module: &PackageDocModule,
) -> String {
    let mut out = String::new();
    heading(&mut out, 1, &format!("Module {}", module.name));
    facts(
        &mut out,
        [
            ("Package identity", catalog.package_identity.as_str()),
            ("Snapshot digest", result.snapshot_digest()),
            ("Documentation digest", result.doc_digest()),
            ("Module id", module.id.as_str()),
            ("Source path", module.source_path.as_str()),
        ],
    );
    documentation(&mut out, &module.doc);
    doctests(&mut out, &module.doctests);
    references(&mut out, &module.references);
    heading(&mut out, 2, "Declarations");
    if module.declarations.is_empty() {
        out.push_str("No public declarations are published.\n");
    } else {
        for declaration in &module.declarations {
            out.push_str("- [");
            out.push_str(&declaration.kind);
            out.push(' ');
            out.push_str(&declaration.name);
            out.push_str("](");
            out.push_str(&declaration.uri);
            out.push_str(") - `");
            out.push_str(&declaration.signature);
            out.push_str("`\n");
        }
    }
    out
}

fn render_declaration(
    result: &PackageDocResult,
    catalog: &PackageDocCatalog,
    module: &PackageDocModule,
    declaration: &PackageDocDeclaration,
) -> String {
    let mut out = String::new();
    heading(
        &mut out,
        1,
        &format!("{} {}", title_case(&declaration.kind), declaration.name),
    );
    facts(
        &mut out,
        [
            ("Package identity", catalog.package_identity.as_str()),
            ("Snapshot digest", result.snapshot_digest()),
            ("Documentation digest", result.doc_digest()),
            ("Module", module.name.as_str()),
            ("Declaration id", declaration.id.as_str()),
            ("Kind", declaration.kind.as_str()),
            ("Signature", declaration.signature.as_str()),
        ],
    );
    documentation(&mut out, &declaration.doc);
    if !declaration.contracts.is_empty() {
        heading(&mut out, 2, "Contracts");
        for contract in &declaration.contracts {
            out.push_str("- ");
            out.push_str(&contract.kind);
            out.push_str(": `");
            out.push_str(&contract.text);
            out.push_str("`\n");
        }
        out.push('\n');
    }
    if let Some(alias) = &declaration.alias {
        heading(&mut out, 2, "Alias");
        out.push_str("- Kind: ");
        out.push_str(&alias.kind);
        out.push('\n');
        out.push_str("- Target: ");
        out.push_str(&alias.target.join("::"));
        out.push_str("\n\n");
    }
    constructors(&mut out, &declaration.constructors);
    doctests(&mut out, &declaration.doctests);
    references(&mut out, &declaration.references);
    out
}

fn render_status(
    result: &PackageDocResult,
    status: &PackageDocGenerationStatus,
    listed: bool,
) -> RenderedPackageDocResource {
    RenderedPackageDocResource {
        uri: result.status_uri().to_string(),
        name: package_status_name(result),
        title: format!("Veln package documentation status: {}", result.identity()),
        description: None,
        mime_type: PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE,
        text: render_status_text(result, status),
        listed,
    }
}

fn render_status_text(result: &PackageDocResult, status: &PackageDocGenerationStatus) -> String {
    let mut out = String::new();
    heading(
        &mut out,
        1,
        &format!("Package Documentation Status: {}", result.identity()),
    );
    facts(
        &mut out,
        [
            ("Package identity", result.identity()),
            ("Snapshot digest", result.snapshot_digest()),
            ("Documentation digest", result.doc_digest()),
            ("State", generation_state(status.state)),
        ],
    );
    diagnostics(&mut out, &status.diagnostics);
    out
}

fn metadata(out: &mut String, metadata: &PackageDocMetadata) {
    heading(out, 2, "Metadata");
    optional_fact(out, "Identity", Some(metadata.identity.as_str()));
    optional_fact(out, "Package name", metadata.package_name.as_deref());
    optional_fact(out, "Version", metadata.version.as_deref());
    optional_fact(out, "Description", metadata.description.as_deref());
    optional_fact(out, "License", metadata.license.as_deref());
    list_fact(out, "Authors", &metadata.authors);
    list_fact(out, "Keywords", &metadata.keywords);
    list_fact(out, "Exported modules", &metadata.exported_modules);
    out.push('\n');
}

fn documentation(out: &mut String, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    heading(out, 2, "Documentation");
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
}

fn constructors(out: &mut String, constructors: &[PackageDocTypeConstructor]) {
    if constructors.is_empty() {
        return;
    }
    heading(out, 2, "Constructors");
    for constructor in constructors {
        heading(out, 3, &constructor.name);
        facts(
            out,
            [
                ("Name", constructor.name.as_str()),
                ("Signature", constructor.signature.as_str()),
            ],
        );
        documentation(out, &constructor.doc);
        doctests(out, &constructor.doctests);
        references(out, &constructor.references);
    }
}

fn doctests(out: &mut String, doctests: &[PackageDocDoctest]) {
    if doctests.is_empty() {
        return;
    }
    heading(out, 2, "Doctests");
    for (index, doctest) in doctests.iter().enumerate() {
        heading(out, 3, &format!("Doctest {}", index + 1));
        optional_fact(out, "Kind", Some(doctest.kind.as_str()));
        optional_fact(out, "Expected error", doctest.expected_error.as_deref());
        optional_fact(
            out,
            "Should fail",
            Some(if doctest.should_fail { "true" } else { "false" }),
        );
        out.push('\n');
        out.push_str("```veln\n");
        out.push_str(&doctest.code);
        if !doctest.code.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```\n\n");
        expected_output(out, &doctest.expected_output);
    }
}

fn expected_output(out: &mut String, outputs: &[PackageDocExpectedOutput]) {
    if outputs.is_empty() {
        return;
    }
    heading(out, 4, "Expected Output");
    for output in outputs {
        out.push_str("- Stream: ");
        out.push_str(&output.stream);
        out.push('\n');
        out.push_str("```text\n");
        for line in &output.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("```\n\n");
    }
}

fn references(out: &mut String, references: &[PackageDocReference]) {
    if references.is_empty() {
        return;
    }
    heading(out, 2, "References");
    for reference in references {
        out.push_str("- ");
        out.push_str(&reference.kind);
        out.push(' ');
        out.push_str(&reference.marker);
        out.push_str(": [");
        out.push_str(&reference.target_declaration_id);
        out.push_str("](");
        out.push_str(&reference.target_uri);
        out.push_str(")\n");
    }
    out.push('\n');
}

fn diagnostics(out: &mut String, diagnostics: &[PackageDocDiagnostic]) {
    heading(out, 2, "Diagnostics");
    if diagnostics.is_empty() {
        out.push_str("No diagnostics.\n");
        return;
    }
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        heading(out, 3, &format!("Diagnostic {}", index + 1));
        facts(
            out,
            [
                ("Gate", diagnostic.gate.as_str()),
                ("Code", diagnostic.code.as_str()),
                ("Message", diagnostic.message.as_str()),
            ],
        );
        if let Some(span) = &diagnostic.span {
            facts(
                out,
                [
                    ("Source URI", span.source_uri.as_str()),
                    ("Line", &span.line.to_string()),
                    ("Column", &span.column.to_string()),
                    ("Offset", &span.offset.to_string()),
                ],
            );
        }
    }
}

fn status_summary(out: &mut String, status: &PackageDocGenerationStatus) {
    heading(out, 2, "Status");
    out.push_str("- State: ");
    out.push_str(generation_state(status.state));
    out.push('\n');
    out.push_str("- Diagnostics: ");
    out.push_str(&status.diagnostics.len().to_string());
    out.push('\n');
}

fn facts<'a>(out: &mut String, facts: impl IntoIterator<Item = (&'a str, &'a str)>) {
    for (key, value) in facts {
        optional_fact(out, key, Some(value));
    }
    out.push('\n');
}

fn optional_fact(out: &mut String, key: &str, value: Option<&str>) {
    out.push_str("- ");
    out.push_str(key);
    out.push_str(": ");
    out.push_str(value.unwrap_or("null"));
    out.push('\n');
}

fn list_fact(out: &mut String, key: &str, values: &[String]) {
    out.push_str("- ");
    out.push_str(key);
    out.push_str(": ");
    if values.is_empty() {
        out.push_str("[]\n");
    } else {
        out.push_str(&values.join(", "));
        out.push('\n');
    }
}

fn heading(out: &mut String, level: usize, text: &str) {
    out.push_str(&"#".repeat(level));
    out.push(' ');
    out.push_str(text);
    out.push_str("\n\n");
}

fn first_doc_line(lines: &[String]) -> Option<String> {
    lines.iter().find(|line| !line.trim().is_empty()).cloned()
}

fn generation_state(state: PackageDocGeneration) -> &'static str {
    match state {
        PackageDocGeneration::Complete => "complete",
        PackageDocGeneration::Failed => "failed",
    }
}

fn package_index_name(catalog: &PackageDocCatalog) -> String {
    format!(
        "{}-documentation-index",
        catalog.package_identity.replace('/', "-")
    )
}

fn package_status_name(result: &PackageDocResult) -> String {
    format!(
        "{}-documentation-status",
        result.identity().replace('/', "-")
    )
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veln_project::{
        PackageIdentity, PackageSnapshotSource, capture_embedded_package_snapshot,
        parse_manifest_text,
    };

    fn generate(manifest: &str, sources: &[(&str, &str)]) -> PackageDocResult {
        let snapshot = capture_embedded_package_snapshot(
            manifest.as_bytes(),
            sources
                .iter()
                .map(|(path, text)| PackageSnapshotSource::new(path, text.as_bytes())),
        )
        .unwrap();
        let manifest = parse_manifest_text("veln.toml", manifest);
        let identity = PackageIdentity::new(
            manifest
                .package
                .fields
                .iter()
                .find(|field| field.key == "name")
                .unwrap()
                .value
                .as_str(),
        )
        .unwrap();
        PackageDocResult::generate(
            &identity,
            &snapshot,
            &manifest,
            PackageDocGeneratorContract::new("contract-a"),
        )
    }

    #[test]
    fn successful_rendering_preserves_ordered_fields_and_links() {
        let result = generate(
            concat!(
                "[package]\n",
                "name = \"demo\"\n",
                "description = \"Package docs.\"\n",
                "authors = \"Ada, Bea\"\n",
                "keywords = \"docs, api\"\n",
                "[lib]\n",
                "exports = [\"main.veln\"]\n",
            ),
            &[(
                "main.veln",
                concat!(
                    "## Type docs mention {@schema Packet}.\n",
                    "pub type Choice\n",
                    "\t## Ready docs.\n",
                    "\tpub Ready(value: Int)\n",
                    "end\n",
                    "\n",
                    "pub schema Packet\n",
                    "\tvalue: UInt8\n",
                    "end\n",
                    "\n",
                    "## Function docs.\n",
                    "## ```veln\n",
                    "## fn sample() -> Int\n",
                    "## \t1\n",
                    "## end\n",
                    "## ```\n",
                    "## ```veln-output stream=stdout\n",
                    "## 1\n",
                    "## ```\n",
                    "pub fn value(input: Int) -> output: Int\n",
                    "\trequire input >= 0\n",
                    "\tensure output >= input\n",
                    "\tinput\n",
                    "end\n",
                ),
            )],
        );

        let resources = render_package_documentation(&result);
        let listed = resources
            .iter()
            .filter(|resource| resource.listed)
            .collect::<Vec<_>>();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].uri, result.catalog().unwrap().index_uri);
        assert_eq!(listed[0].description.as_deref(), Some("Package docs."));
        let index = &listed[0].text;
        assert!(index.contains("- Schema version: veln-package-doc-catalog/v1"));
        assert!(index.contains("- Authors: Ada, Bea"));
        assert!(index.contains("- Keywords: docs, api"));
        assert!(index.contains("- Exported modules: main"));
        assert!(index.contains("](veln-doc:///package/demo/"));

        let declaration = resources
            .iter()
            .find(|resource| resource.text.starts_with("# Function value\n\n"))
            .unwrap();
        let text = &declaration.text;
        assert!(text.contains("- Kind: function"));
        assert!(text.contains("- Signature: "));
        assert!(text.contains("value(input: Int)"));
        assert!(text.contains("Function docs."));
        assert!(text.contains("- require: `input >= 0`"));
        assert!(text.contains("- ensure: `output >= input`"));
        assert!(text.contains("## Doctests"));
        assert!(text.contains("#### Expected Output"));
        assert!(text.contains("- Stream: stdout"));

        let type_resource = resources
            .iter()
            .find(|resource| resource.text.starts_with("# Type Choice\n\n"))
            .unwrap();
        assert!(type_resource.text.contains("## Constructors"));
        assert!(type_resource.text.contains("### Ready"));
        assert!(type_resource.text.contains("## References"));
        assert!(type_resource.text.contains("Packet"));
        assert!(type_resource.text.contains("veln-doc:///package/demo/"));
    }

    #[test]
    fn status_rendering_preserves_diagnostics_and_disclosure_boundary() {
        let snapshot = capture_embedded_package_snapshot(
            b"[package]\nname = \"demo\"\nrepository = \"hidden\"\n[lib]\nexports = [\"main.veln\"]\n",
            [PackageSnapshotSource::new(
                "main.veln",
                b"pub fn broken(\n\t1\nend\n",
            )],
        )
        .unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            "[package]\nname = \"demo\"\nrepository = \"hidden\"\n[lib]\nexports = [\"main.veln\"]\n",
        );
        let result = PackageDocResult::generate(
            &PackageIdentity::new("demo").unwrap(),
            &snapshot,
            &manifest,
            PackageDocGeneratorContract::new("contract-a"),
        );

        let resources = render_package_documentation(&result);
        assert_eq!(resources.len(), 1);
        assert!(resources[0].listed);
        let text = &resources[0].text;
        assert!(text.starts_with("# Package Documentation Status: demo\n\n"));
        assert!(text.contains("- State: failed"));
        assert!(text.contains("- Gate: parse"));
        assert!(text.contains("- Code: "));
        assert!(text.contains("- Message: "));
        assert!(text.contains("- Source URI: veln-pkg:///demo/snapshot/"));
        assert!(!text.contains("repository"));
        assert!(!text.contains("hidden"));
    }
}
