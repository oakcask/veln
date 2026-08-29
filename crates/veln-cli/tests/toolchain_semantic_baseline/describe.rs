use super::*;

pub(super) fn describe(manifest: &CaseManifest) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    describe_invocation(&mut fields, manifest);
    describe_expectations(&mut fields, manifest);
    describe_execution_gates(&mut fields, manifest);
    describe_skip(&mut fields, manifest);
    fields
}

fn describe_invocation(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    string_list(fields, "invocation.command", &manifest.invocation.command);
    optional_path(fields, "invocation.cwd", manifest.invocation.cwd.as_deref());
    optional_text(
        fields,
        "invocation.stdin",
        manifest.invocation.stdin.as_deref(),
    );
    optional_text(
        fields,
        "invocation.stdin_jsonrpc_file",
        manifest.invocation.stdin_jsonrpc_file.as_deref(),
    );
    scalar(fields, "invocation.repeat", manifest.invocation.repeat);
    for (index, (name, value)) in manifest.invocation.env.iter().enumerate() {
        text(fields, &format!("invocation.env[{index}].name"), name);
        text(fields, &format!("invocation.env[{index}].value"), value);
    }
}

fn describe_expectations(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    scalar(fields, "expectations.exit", manifest.expectations.exit);
    stream(fields, "expectations.stdout", &manifest.expectations.stdout);
    stream(fields, "expectations.stderr", &manifest.expectations.stderr);
    optional_help(fields, manifest.expectations.help.as_ref());
    describe_json_assertions(fields, manifest);
    describe_result_value_assertions(fields, manifest);
    describe_lsp_assertions(fields, manifest);
    describe_mcp_assertions(fields, manifest);
    describe_file_assertions(fields, manifest);
    describe_diagnostics(fields, manifest);
    binary_fixtures(fields, &manifest.expectations.binary_fixtures);
    output_chunks(fields, &manifest.expectations.output_chunk_lists);
}

fn describe_json_assertions(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    for (index, assertion) in manifest.expectations.json_assertions.iter().enumerate() {
        let base = format!("expectations.json_assertions[{index}]");
        text(fields, &format!("{base}.path"), &assertion.path);
        value_assertion_operation(fields, &base, assertion.operation.as_ref());
    }
}

fn describe_result_value_assertions(
    fields: &mut BTreeMap<String, String>,
    manifest: &CaseManifest,
) {
    for (index, assertion) in manifest
        .expectations
        .result_value_assertions
        .iter()
        .enumerate()
    {
        let base = format!("expectations.result_value_assertions[{index}]");
        text(fields, &format!("{base}.value_path"), &assertion.value_path);
        text(fields, &format!("{base}.path"), &assertion.path);
        value_assertion_operation(fields, &base, assertion.operation.as_ref());
    }
}

fn describe_lsp_assertions(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    for (index, assertion) in manifest.expectations.lsp_assertions.iter().enumerate() {
        let base = format!("expectations.lsp_assertions[{index}]");
        if let Some(id) = &assertion.id {
            fields.insert(
                format!("{base}.id"),
                canonical_json(id, &format!("{base}.id")),
            );
        }
        optional_text(
            fields,
            &format!("{base}.method"),
            assertion.method.as_deref(),
        );
        if assertion.method.is_some() {
            scalar(
                fields,
                &format!("{base}.occurrence"),
                assertion.occurrence.unwrap_or(0),
            );
        }
        text(fields, &format!("{base}.path"), &assertion.path);
        describe_protocol_assertion_operation(
            fields,
            &base,
            assertion
                .operation
                .as_ref()
                .expect("validated LSP assertion operation"),
        );
    }
}

fn describe_mcp_assertions(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    for (index, assertion) in manifest.expectations.mcp_assertions.iter().enumerate() {
        let base = format!("expectations.mcp_assertions[{index}]");
        fields.insert(
            format!("{base}.id"),
            canonical_json(
                assertion.id.as_ref().expect("validated MCP id"),
                &format!("{base}.id"),
            ),
        );
        text(fields, &format!("{base}.path"), &assertion.path);
        describe_protocol_assertion_operation(
            fields,
            &base,
            assertion
                .operation
                .as_ref()
                .expect("validated MCP assertion operation"),
        );
    }
}

fn describe_protocol_assertion_operation(
    fields: &mut BTreeMap<String, String>,
    base: &str,
    operation: &RpcAssertionOperation,
) {
    match operation {
        RpcAssertionOperation::Equals(value) => {
            enum_value(fields, &format!("{base}.operation"), "equals");
            fields.insert(
                format!("{base}.equals"),
                canonical_json(value, &format!("{base}.equals")),
            );
        }
        RpcAssertionOperation::EqualsFile(value) => {
            enum_value(fields, &format!("{base}.operation"), "equals_file");
            text(fields, &format!("{base}.equals_file"), value);
        }
        RpcAssertionOperation::EqualsFileRef(_) => {
            unreachable!("manifest finish resolves protocol equals_file operands")
        }
        RpcAssertionOperation::EqualsJsonFile(value) => {
            enum_value(fields, &format!("{base}.operation"), "equals_json_file");
            fields.insert(
                format!("{base}.equals_json_file"),
                canonical_json(value, &format!("{base}.equals_json_file")),
            );
        }
        RpcAssertionOperation::EqualsJsonFileRef(_) => {
            unreachable!("manifest finish resolves protocol equals_json_file operands")
        }
        RpcAssertionOperation::Contains(value) => {
            enum_value(fields, &format!("{base}.operation"), "contains");
            text(fields, &format!("{base}.contains"), value);
        }
        RpcAssertionOperation::Length(value) => {
            enum_value(fields, &format!("{base}.operation"), "length");
            scalar(fields, &format!("{base}.length"), *value);
        }
        RpcAssertionOperation::Missing(true) => {
            enum_value(fields, &format!("{base}.operation"), "missing");
        }
        RpcAssertionOperation::WorkspaceFileUri(value) => {
            enum_value(fields, &format!("{base}.operation"), "workspace_file_uri");
            text(fields, &format!("{base}.workspace_file_uri"), value);
        }
        RpcAssertionOperation::Missing(false) => unreachable!("validated missing operation"),
    }
}

fn describe_file_assertions(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    for (index, assertion) in manifest.expectations.file_assertions.iter().enumerate() {
        let base = format!("expectations.file_assertions[{index}]");
        text(fields, &format!("{base}.path"), &assertion.path);
        if assertion.missing {
            enum_value(fields, &format!("{base}.operation"), "missing");
        } else {
            enum_value(fields, &format!("{base}.operation"), "equals");
            text(
                fields,
                &format!("{base}.equals"),
                assertion
                    .equals
                    .as_deref()
                    .expect("file assertion should have expected text"),
            );
        }
    }
}

fn describe_diagnostics(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    for (index, diagnostic) in manifest.expectations.diagnostics.iter().enumerate() {
        let base = format!("expectations.diagnostics[{index}]");
        text(fields, &format!("{base}.id"), &diagnostic.id);
        optional_text(
            fields,
            &format!("{base}.severity"),
            diagnostic.severity.as_deref(),
        );
        optional_text(fields, &format!("{base}.kind"), diagnostic.kind.as_deref());
        optional_text(
            fields,
            &format!("{base}.message"),
            diagnostic.message.as_deref(),
        );
        if let Some(span) = &diagnostic.span {
            optional_text(fields, &format!("{base}.span.file"), span.file.as_deref());
            optional_scalar(fields, &format!("{base}.span.line"), span.line);
            optional_scalar(fields, &format!("{base}.span.column"), span.column);
        }
    }
}

fn describe_execution_gates(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    enum_value(
        fields,
        "source_errors",
        source_error_name(manifest.source_errors),
    );
    if let Some(expectation) = &manifest.manifest_error {
        string_list(fields, "manifest_error.contains", &expectation.contains);
    }
    if manifest.requires.jdk {
        scalar(fields, "requirements.jdk", true);
    }
    optional_enum(
        fields,
        "tools.java",
        manifest.tools.java.map(tool_availability),
    );
    optional_enum(
        fields,
        "tools.git",
        manifest.tools.git.map(tool_availability),
    );
}

fn describe_skip(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    let platforms = manifest
        .skip
        .platforms
        .iter()
        .map(|platform| skip_platform_name(*platform).to_string())
        .collect::<Vec<_>>();
    string_list(fields, "skip.platforms", &platforms);
    optional_text(fields, "skip.reason", manifest.skip.reason.as_deref());
}

fn source_error_name(value: SourceErrorExpectation) -> &'static str {
    match value {
        SourceErrorExpectation::Forbidden => "forbidden",
        SourceErrorExpectation::Expected => "expected",
    }
}

fn skip_platform_name(value: SkipPlatform) -> &'static str {
    match value {
        SkipPlatform::Unix => "unix",
        SkipPlatform::Windows => "windows",
        SkipPlatform::Macos => "macos",
        SkipPlatform::Linux => "linux",
    }
}

fn stream(fields: &mut BTreeMap<String, String>, base: &str, stream: &StreamExpectation) {
    optional_enum(
        fields,
        &format!("{base}.format"),
        stream.format.map(|format| match format {
            StreamFormat::Empty => "empty",
            StreamFormat::Text => "text",
            StreamFormat::Json => "json",
        }),
    );
    string_list(fields, &format!("{base}.contains"), &stream.contains);
    string_list(
        fields,
        &format!("{base}.not_contains"),
        &stream.not_contains,
    );
}

fn optional_help(fields: &mut BTreeMap<String, String>, help: Option<&HelpExpectation>) {
    let Some(help) = help else {
        return;
    };
    enum_value(fields, "expectations.help.stream", help.stream.name());
    optional_text(fields, "expectations.help.summary", help.summary.as_deref());
    optional_text(fields, "expectations.help.usage", help.usage.as_deref());
    string_list(fields, "expectations.help.commands", &help.commands);
    string_list(fields, "expectations.help.arguments", &help.arguments);
    string_list(fields, "expectations.help.options", &help.options);
    string_list(fields, "expectations.help.contains", &help.contains);
}

fn value_assertion_operation(
    fields: &mut BTreeMap<String, String>,
    base: &str,
    operation: Option<&ValueAssertionOperation>,
) {
    let operation = operation.expect("preflight requires one value assertion operation");
    let (name, operand) = match operation {
        ValueAssertionOperation::Equals(value) => ("equals", Some(value)),
        ValueAssertionOperation::EqualsFile(value) => ("equals_file", Some(value)),
        ValueAssertionOperation::EqualsJsonFile(value) => ("equals_json_file", Some(value)),
        ValueAssertionOperation::Contains(value) => {
            enum_value(fields, &format!("{base}.operation"), "contains");
            text(fields, &format!("{base}.contains"), value);
            return;
        }
        ValueAssertionOperation::Length(value) => {
            enum_value(fields, &format!("{base}.operation"), "length");
            scalar(fields, &format!("{base}.length"), *value);
            return;
        }
        ValueAssertionOperation::Missing => {
            enum_value(fields, &format!("{base}.operation"), "missing");
            return;
        }
        ValueAssertionOperation::WorkspaceFileUri(value) => {
            enum_value(fields, &format!("{base}.operation"), "workspace_file_uri");
            text(fields, &format!("{base}.workspace_file_uri"), value);
            return;
        }
    };
    enum_value(fields, &format!("{base}.operation"), name);
    let path = format!("{base}.{name}");
    fields.insert(
        path.clone(),
        canonical_json(operand.expect("equality operation has operand"), &path),
    );
}

fn binary_fixtures(fields: &mut BTreeMap<String, String>, fixtures: &[BinaryFixtureExpectation]) {
    for (index, fixture) in fixtures.iter().enumerate() {
        let base = format!("expectations.binary_fixtures[{index}]");
        text(fields, &format!("{base}.name"), &fixture.name);
        optional_text(fields, &format!("{base}.schema"), fixture.schema.as_deref());
        if let Some(bytes) = &fixture.bytes {
            bytes_value(fields, &format!("{base}.bytes"), &bytes.bytes);
        }
        optional_scalar(fields, &format!("{base}.consumed"), fixture.consumed);
        optional_text(fields, &format!("{base}.error"), fixture.error.as_deref());
        if let Some(diagnostic) = &fixture.byte_diagnostic {
            optional_text(
                fields,
                &format!("{base}.byte_diagnostic.diagnostic_id"),
                diagnostic.diagnostic_id.as_deref(),
            );
            optional_scalar(
                fields,
                &format!("{base}.byte_diagnostic.byte_offset"),
                diagnostic.byte_offset,
            );
            optional_scalar(
                fields,
                &format!("{base}.byte_diagnostic.expected_count"),
                diagnostic.expected_count,
            );
            optional_scalar(
                fields,
                &format!("{base}.byte_diagnostic.available_count"),
                diagnostic.available_count,
            );
            optional_text(
                fields,
                &format!("{base}.byte_diagnostic.readiness"),
                diagnostic.readiness.as_deref(),
            );
            if let Some(value) = &diagnostic.field_path {
                let path = format!("{base}.byte_diagnostic.field_path");
                fields.insert(path.clone(), canonical_json(value, &path));
            }
        }
    }
}

fn output_chunks(fields: &mut BTreeMap<String, String>, lists: &[OutputChunkListExpectation]) {
    for (index, list) in lists.iter().enumerate() {
        let base = format!("expectations.output_chunk_lists[{index}]");
        text(fields, &format!("{base}.name"), &list.name);
        let chunks = list
            .chunks
            .as_ref()
            .expect("validated output chunks should exist");
        for (chunk_index, chunk) in chunks.iter().enumerate() {
            bytes_value(
                fields,
                &format!("{base}.chunks[{chunk_index}]"),
                &chunk.bytes,
            );
        }
    }
}

fn canonical_json(value: &JsonValue, logical_field: &str) -> String {
    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(value) => value.to_string(),
        JsonValue::Number(value) => value.to_string(),
        JsonValue::Decimal(value) => value.clone(),
        JsonValue::String(value) => {
            if value.len() >= LARGE_TEXT_BYTES {
                large_text_descriptor(logical_field, value)
            } else {
                json_string(value)
            }
        }
        JsonValue::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    canonical_json(value, &format!("{logical_field}/{index}"))
                })
                .collect::<Vec<_>>()
                .join(",")
        ),
        JsonValue::Object(entries) => {
            let mut entries = entries
                .iter()
                .map(|(key, value)| {
                    (
                        key,
                        canonical_json(value, &json_logical_child(logical_field, key)),
                    )
                })
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            format!(
                "{{{}}}",
                entries
                    .into_iter()
                    .map(|(key, value)| format!("{}:{value}", json_string(key)))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}

fn json_logical_child(parent: &str, key: &str) -> String {
    let escaped = key.replace('~', "~0").replace('/', "~1");
    format!("{parent}/{escaped}")
}

fn string_list(fields: &mut BTreeMap<String, String>, base: &str, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        text(fields, &format!("{base}[{index}]"), value);
    }
}

fn text(fields: &mut BTreeMap<String, String>, path: &str, value: &str) {
    if value.len() >= LARGE_TEXT_BYTES {
        fields.insert(path.to_string(), large_text_descriptor(path, value));
    } else {
        fields.insert(path.to_string(), json_string(value));
    }
}

fn large_text_descriptor(logical_field: &str, value: &str) -> String {
    format!(
        "{{\"logical_field\":{},\"byte_length\":{},\"sha256\":{}}}",
        json_string(logical_field),
        value.len(),
        json_string(&sha256(value.as_bytes()))
    )
}

fn bytes_value(fields: &mut BTreeMap<String, String>, path: &str, value: &[u8]) {
    fields.insert(
        path.to_string(),
        format!(
            "{{\"byte_length\":{},\"sha256\":{}}}",
            value.len(),
            json_string(&sha256(value))
        ),
    );
}

fn optional_text(fields: &mut BTreeMap<String, String>, path: &str, value: Option<&str>) {
    if let Some(value) = value {
        text(fields, path, value);
    }
}
fn optional_path(fields: &mut BTreeMap<String, String>, path: &str, value: Option<&Path>) {
    optional_text(
        fields,
        path,
        value.map(|value| value.to_str().expect("manifest path should be UTF-8")),
    );
}
fn optional_enum(fields: &mut BTreeMap<String, String>, path: &str, value: Option<&str>) {
    if let Some(value) = value {
        enum_value(fields, path, value);
    }
}
fn scalar(fields: &mut BTreeMap<String, String>, path: &str, value: impl ToString) {
    fields.insert(path.to_string(), value.to_string());
}
fn optional_scalar<T: ToString + Copy>(
    fields: &mut BTreeMap<String, String>,
    path: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        scalar(fields, path, value);
    }
}
fn enum_value(fields: &mut BTreeMap<String, String>, path: &str, value: &str) {
    fields.insert(path.to_string(), json_string(value));
}
fn tool_availability(value: ToolAvailability) -> &'static str {
    match value {
        ToolAvailability::Missing => "missing",
        ToolAvailability::FakeSuccess => "fake_success",
        ToolAvailability::FakeGitRevParse => "fake_git_rev_parse",
        ToolAvailability::Real => "real",
    }
}
