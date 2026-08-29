use super::*;

pub(super) fn catalog_to_json(catalog: &PackageDocCatalog) -> String {
    let mut out = String::new();
    out.push('{');
    field(&mut out, "schema_version", &catalog.schema_version, false);
    field(
        &mut out,
        "generator_contract",
        &catalog.generator_contract,
        true,
    );
    field(
        &mut out,
        "package_identity",
        &catalog.package_identity,
        true,
    );
    field(&mut out, "snapshot_digest", &catalog.snapshot_digest, true);
    out.push_str(",\"metadata\":");
    metadata_json(&mut out, &catalog.metadata);
    out.push_str(",\"modules\":[");
    for (index, module) in catalog.modules.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        module_json(&mut out, module);
    }
    out.push_str("],\"status\":");
    status_json(&mut out, &catalog.status);
    out.push('}');
    out
}

pub(super) fn status_to_json(
    identity: &str,
    snapshot_digest: &str,
    generator_contract: &str,
    status: &PackageDocGenerationStatus,
) -> String {
    let mut out = String::new();
    out.push('{');
    field(&mut out, "schema_version", SCHEMA_VERSION, false);
    field(&mut out, "generator_contract", generator_contract, true);
    field(&mut out, "package_identity", identity, true);
    field(&mut out, "snapshot_digest", snapshot_digest, true);
    out.push_str(",\"status\":");
    status_json(&mut out, status);
    out.push('}');
    out
}

pub(super) fn metadata_json(out: &mut String, metadata: &PackageDocMetadata) {
    out.push('{');
    field(out, "identity", &metadata.identity, false);
    optional_field(out, "package_name", metadata.package_name.as_deref());
    optional_field(out, "version", metadata.version.as_deref());
    optional_field(out, "description", metadata.description.as_deref());
    optional_field(out, "license", metadata.license.as_deref());
    string_array_field(out, "authors", &metadata.authors);
    string_array_field(out, "keywords", &metadata.keywords);
    string_array_field(out, "exported_modules", &metadata.exported_modules);
    out.push('}');
}

pub(super) fn module_json(out: &mut String, module: &PackageDocModule) {
    out.push('{');
    field(out, "id", &module.id, false);
    field(out, "name", &module.name, true);
    field(out, "source_path", &module.source_path, true);
    string_array_field(out, "doc", &module.doc);
    out.push_str(",\"doctests\":[");
    doctest_array_json(out, &module.doctests);
    out.push_str("],\"references\":[");
    reference_array_json(out, &module.references);
    out.push_str("],\"declarations\":[");
    for (index, declaration) in module.declarations.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        declaration_json(out, declaration);
    }
    out.push_str("]}");
}

pub(super) fn declaration_json(out: &mut String, declaration: &PackageDocDeclaration) {
    out.push('{');
    field(out, "id", &declaration.id, false);
    field(out, "kind", &declaration.kind, true);
    field(out, "name", &declaration.name, true);
    field(out, "signature", &declaration.signature, true);
    string_array_field(out, "doc", &declaration.doc);
    out.push_str(",\"contracts\":[");
    contract_array_json(out, &declaration.contracts);
    out.push_str("],\"constructors\":[");
    constructor_array_json(out, &declaration.constructors);
    out.push_str("],\"alias\":");
    alias_json(out, declaration.alias.as_ref());
    out.push_str(",\"doctests\":[");
    doctest_array_json(out, &declaration.doctests);
    out.push_str("],\"references\":[");
    reference_array_json(out, &declaration.references);
    out.push_str("]}");
}

pub(super) fn contract_array_json(out: &mut String, contracts: &[PackageDocFunctionContract]) {
    for (index, contract) in contracts.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "kind", &contract.kind, false);
        field(out, "text", &contract.text, true);
        out.push('}');
    }
}

pub(super) fn constructor_array_json(out: &mut String, constructors: &[PackageDocTypeConstructor]) {
    for (index, constructor) in constructors.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "name", &constructor.name, false);
        field(out, "signature", &constructor.signature, true);
        string_array_field(out, "doc", &constructor.doc);
        out.push_str(",\"doctests\":[");
        doctest_array_json(out, &constructor.doctests);
        out.push_str("],\"references\":[");
        reference_array_json(out, &constructor.references);
        out.push(']');
        out.push('}');
    }
}

pub(super) fn alias_json(out: &mut String, alias: Option<&PackageDocAlias>) {
    if let Some(alias) = alias {
        out.push('{');
        field(out, "kind", &alias.kind, false);
        string_array_field(out, "target", &alias.target);
        out.push('}');
    } else {
        out.push_str("null");
    }
}

pub(super) fn doctest_array_json(out: &mut String, doctests: &[PackageDocDoctest]) {
    for (index, doctest) in doctests.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "kind", &doctest.kind, false);
        field(out, "code", &doctest.code, true);
        optional_field(out, "expected_error", doctest.expected_error.as_deref());
        bool_field(out, "should_fail", doctest.should_fail);
        out.push_str(",\"expected_output\":[");
        expected_output_array_json(out, &doctest.expected_output);
        out.push(']');
        out.push('}');
    }
}

pub(super) fn expected_output_array_json(out: &mut String, outputs: &[PackageDocExpectedOutput]) {
    for (index, output) in outputs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "stream", &output.stream, false);
        string_array_field(out, "lines", &output.lines);
        out.push('}');
    }
}

pub(super) fn reference_array_json(out: &mut String, references: &[PackageDocReference]) {
    for (index, reference) in references.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "kind", &reference.kind, false);
        field(out, "marker", &reference.marker, true);
        field(
            out,
            "target_declaration_id",
            &reference.target_declaration_id,
            true,
        );
        out.push('}');
    }
}

pub(super) fn status_json(out: &mut String, status: &PackageDocGenerationStatus) {
    out.push('{');
    field(
        out,
        "state",
        match status.state {
            PackageDocGeneration::Complete => "complete",
            PackageDocGeneration::Failed => "failed",
        },
        false,
    );
    out.push_str(",\"diagnostics\":[");
    for (index, diagnostic) in status.diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(out, "gate", &diagnostic.gate, false);
        field(out, "code", &diagnostic.code, true);
        field(out, "message", &diagnostic.message, true);
        out.push_str(",\"span\":");
        if let Some(span) = &diagnostic.span {
            out.push('{');
            field(out, "source_uri", &span.source_uri, false);
            number_field(out, "line", span.line);
            number_field(out, "column", span.column);
            number_field(out, "offset", span.offset);
            out.push('}');
        } else {
            out.push_str("null");
        }
        out.push('}');
    }
    out.push_str("]}");
}

pub(super) fn field(out: &mut String, key: &str, value: &str, comma: bool) {
    if comma {
        out.push(',');
    }
    string(out, key);
    out.push(':');
    string(out, value);
}

pub(super) fn optional_field(out: &mut String, key: &str, value: Option<&str>) {
    out.push(',');
    string(out, key);
    out.push(':');
    if let Some(value) = value {
        string(out, value);
    } else {
        out.push_str("null");
    }
}

pub(super) fn string_array_field(out: &mut String, key: &str, values: &[String]) {
    out.push(',');
    string(out, key);
    out.push_str(":[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        string(out, value);
    }
    out.push(']');
}

pub(super) fn number_field(out: &mut String, key: &str, value: usize) {
    out.push(',');
    string(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

pub(super) fn bool_field(out: &mut String, key: &str, value: bool) {
    out.push(',');
    string(out, key);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
}

pub(super) fn string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}
