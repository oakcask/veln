use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::*;

const SCHEMA: &str = "veln-toolchain-case-semantics/v0";
const ROOTS: [(&str, &str); 2] = [
    (
        "crates/veln-cli/tests/toolchain_cases",
        "tests/toolchain_cases",
    ),
    ("examples/specification", "../../examples/specification"),
];
const BASELINE: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/toolchain-case-semantics.baseline"));
const LARGE_TEXT_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Inventory {
    schema: String,
    roots: Vec<String>,
    source_git_tree: String,
    cases: BTreeMap<String, BTreeMap<String, String>>,
}

impl Inventory {
    fn current(source_git_tree: &str) -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut cases = BTreeMap::new();
        let inventory = toolchain_case_inventory::run_preflight(&manifest_dir)
            .unwrap_or_else(|error| panic!("{error}"));
        for case in inventory.cases {
            let path = manifest_dir.join(&case.manifest_relative).join("case.toml");
            let manifest = CaseManifest::read(&path);
            assert!(
                cases.insert(case.id.clone(), describe(&manifest)).is_none(),
                "duplicate semantic case identifier `{}`",
                case.id
            );
        }
        Self {
            schema: SCHEMA.to_string(),
            roots: ROOTS.iter().map(|(root, _)| (*root).to_string()).collect(),
            source_git_tree: source_git_tree.to_string(),
            cases,
        }
    }

    fn render(&self) -> String {
        let mut output = String::new();
        line(&mut output, "schema", &json_string(&self.schema));
        line(
            &mut output,
            "source_git_tree",
            &json_string(&self.source_git_tree),
        );
        for root in &self.roots {
            line(&mut output, "root", &json_string(root));
        }
        line(&mut output, "case_count", &self.cases.len().to_string());
        for (id, fields) in &self.cases {
            line(&mut output, "case", &json_string(id));
            for (path, value) in fields {
                output.push_str("field\t");
                output.push_str(path);
                output.push('\t');
                output.push_str(value);
                output.push('\n');
            }
            line(
                &mut output,
                "case_digest",
                &json_string(&fields_digest(fields)),
            );
        }
        line(
            &mut output,
            "aggregate_digest",
            &json_string(&aggregate_digest(&self.cases)),
        );
        output
    }

    fn parse(text: &str) -> Result<Self, String> {
        BaselineParser::default().parse(text)
    }
}

#[derive(Default)]
struct BaselineParser {
    schema: Option<String>,
    source_git_tree: Option<String>,
    roots: Vec<String>,
    declared_count: Option<usize>,
    declared_aggregate: Option<String>,
    cases: BTreeMap<String, BTreeMap<String, String>>,
    current: Option<(String, BTreeMap<String, String>)>,
}

impl BaselineParser {
    fn parse(mut self, text: &str) -> Result<Inventory, String> {
        for (index, raw_line) in text.lines().enumerate() {
            self.parse_line(index + 1, raw_line)?;
        }
        self.finish_open_case(text.lines().count() + 1, None)?;
        self.into_inventory()
    }

    fn parse_line(&mut self, line_number: usize, raw_line: &str) -> Result<(), String> {
        let mut parts = raw_line.splitn(3, '\t');
        let kind = parts.next().unwrap_or_default();
        let first = parts
            .next()
            .ok_or_else(|| format!("baseline line {line_number} has no value"))?;
        match kind {
            "schema" => self.schema = Some(parse_json_string(first, line_number)?),
            "source_git_tree" => {
                self.source_git_tree = Some(parse_json_string(first, line_number)?);
            }
            "root" => self.roots.push(parse_json_string(first, line_number)?),
            "case_count" => self.declared_count = Some(parse_count(first, line_number)?),
            "case" => self.start_case(first, line_number)?,
            "field" => self.insert_field(first, parts.next(), line_number)?,
            "case_digest" => self.finish_case_digest(first, line_number)?,
            "aggregate_digest" => self.finish_aggregate(first, line_number)?,
            _ => {
                return Err(format!(
                    "baseline line {line_number} has unknown record `{kind}`"
                ));
            }
        }
        Ok(())
    }

    fn start_case(&mut self, value: &str, line_number: usize) -> Result<(), String> {
        self.finish_open_case(line_number, None)?;
        self.current = Some((parse_json_string(value, line_number)?, BTreeMap::new()));
        Ok(())
    }

    fn insert_field(
        &mut self,
        path: &str,
        value: Option<&str>,
        line_number: usize,
    ) -> Result<(), String> {
        let value =
            value.ok_or_else(|| format!("baseline line {line_number} has no field value"))?;
        let (_, fields) = self
            .current
            .as_mut()
            .ok_or_else(|| format!("baseline line {line_number} has a field outside a case"))?;
        if fields.insert(path.to_string(), value.to_string()).is_some() {
            return Err(format!(
                "baseline line {line_number} repeats field `{path}`"
            ));
        }
        Ok(())
    }

    fn finish_case_digest(&mut self, value: &str, line_number: usize) -> Result<(), String> {
        let digest = parse_json_string(value, line_number)?;
        self.finish_open_case(line_number, Some(&digest))
    }

    fn finish_aggregate(&mut self, value: &str, line_number: usize) -> Result<(), String> {
        self.finish_open_case(line_number, None)?;
        self.declared_aggregate = Some(parse_json_string(value, line_number)?);
        Ok(())
    }

    fn finish_open_case(
        &mut self,
        line_number: usize,
        declared_digest: Option<&str>,
    ) -> Result<(), String> {
        finish_case(
            &mut self.current,
            &mut self.cases,
            declared_digest,
            line_number,
        )
    }

    fn into_inventory(self) -> Result<Inventory, String> {
        let schema = self
            .schema
            .ok_or_else(|| "baseline is missing schema".to_string())?;
        validate_schema(&schema)?;
        validate_case_count(self.declared_count, self.cases.len())?;
        validate_aggregate(self.declared_aggregate.as_deref(), &self.cases)?;
        Ok(Inventory {
            schema,
            roots: self.roots,
            source_git_tree: self
                .source_git_tree
                .ok_or_else(|| "baseline is missing source Git tree".to_string())?,
            cases: self.cases,
        })
    }
}

fn parse_count(value: &str, line_number: usize) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("baseline line {line_number} has invalid case count: {error}"))
}

fn validate_schema(schema: &str) -> Result<(), String> {
    if schema == SCHEMA {
        Ok(())
    } else {
        Err(format!("unsupported baseline schema `{schema}`"))
    }
}

fn validate_case_count(declared_count: Option<usize>, actual_count: usize) -> Result<(), String> {
    if declared_count == Some(actual_count) {
        Ok(())
    } else {
        Err(format!(
            "baseline case count mismatch: declared {declared_count:?}, found {actual_count}"
        ))
    }
}

fn validate_aggregate(
    declared_aggregate: Option<&str>,
    cases: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<(), String> {
    let actual_aggregate = aggregate_digest(cases);
    if declared_aggregate == Some(actual_aggregate.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "baseline aggregate digest mismatch: declared {declared_aggregate:?}, computed {actual_aggregate}"
        ))
    }
}

fn finish_case(
    current: &mut Option<(String, BTreeMap<String, String>)>,
    cases: &mut BTreeMap<String, BTreeMap<String, String>>,
    declared_digest: Option<&str>,
    line_number: usize,
) -> Result<(), String> {
    let Some((id, fields)) = current.take() else {
        if declared_digest.is_some() {
            return Err(format!(
                "baseline line {line_number} has a digest outside a case"
            ));
        }
        return Ok(());
    };
    if let Some(declared_digest) = declared_digest {
        let actual = fields_digest(&fields);
        if declared_digest != actual {
            return Err(format!(
                "baseline case `{id}` digest mismatch: declared {declared_digest}, computed {actual}"
            ));
        }
    }
    if cases.insert(id.clone(), fields).is_some() {
        return Err(format!("baseline repeats case `{id}`"));
    }
    Ok(())
}

fn compare(expected: &Inventory, actual: &Inventory) -> Result<(), String> {
    compare_metadata(expected, actual)?;
    let expected_ids = compare_case_ids(expected, actual)?;
    let differences = collect_field_differences(expected, actual, expected_ids);
    if differences.is_empty() {
        Ok(())
    } else {
        Err(differences.join("\n"))
    }
}

fn compare_metadata(expected: &Inventory, actual: &Inventory) -> Result<(), String> {
    if expected.schema != actual.schema {
        return Err(format!(
            "schema changed: expected `{}`, got `{}`",
            expected.schema, actual.schema
        ));
    }
    if expected.roots != actual.roots {
        return Err(format!(
            "authoritative roots changed: expected {:?}, got {:?}",
            expected.roots, actual.roots
        ));
    }
    Ok(())
}

fn compare_case_ids(expected: &Inventory, actual: &Inventory) -> Result<BTreeSet<String>, String> {
    let expected_ids = expected.cases.keys().cloned().collect::<BTreeSet<_>>();
    let actual_ids = actual.cases.keys().cloned().collect::<BTreeSet<_>>();
    if expected_ids == actual_ids {
        Ok(expected_ids)
    } else {
        Err(case_set_difference_message(&expected_ids, &actual_ids))
    }
}

fn case_set_difference_message(
    expected_ids: &BTreeSet<String>,
    actual_ids: &BTreeSet<String>,
) -> String {
    let removed = expected_ids
        .difference(actual_ids)
        .cloned()
        .collect::<Vec<_>>();
    let added = actual_ids
        .difference(expected_ids)
        .cloned()
        .collect::<Vec<_>>();
    format!("case set changed; removed={removed:?}; added={added:?}")
}

fn collect_field_differences(
    expected: &Inventory,
    actual: &Inventory,
    expected_ids: BTreeSet<String>,
) -> Vec<String> {
    let mut differences = Vec::new();
    for id in expected_ids {
        append_case_field_differences(expected, actual, &id, &mut differences);
        if differences.len() > 20 {
            break;
        }
    }
    differences
}

fn append_case_field_differences(
    expected: &Inventory,
    actual: &Inventory,
    id: &str,
    differences: &mut Vec<String>,
) {
    let expected_fields = &expected.cases[id];
    let actual_fields = &actual.cases[id];
    let paths = expected_fields
        .keys()
        .chain(actual_fields.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for path in paths {
        if let Some(difference) = field_difference(id, &path, expected_fields, actual_fields) {
            differences.push(difference);
        }
        if differences.len() == 20 {
            differences.push("additional differences omitted".to_string());
            break;
        }
    }
}

fn field_difference(
    id: &str,
    path: &str,
    expected_fields: &BTreeMap<String, String>,
    actual_fields: &BTreeMap<String, String>,
) -> Option<String> {
    match (expected_fields.get(path), actual_fields.get(path)) {
        (Some(expected), Some(actual)) if expected != actual => Some(format!(
            "{id} field `{path}` changed: expected {expected}, got {actual}"
        )),
        (Some(expected), None) => Some(format!(
            "{id} field `{path}` was removed (expected {expected})"
        )),
        (None, Some(actual)) => Some(format!("{id} field `{path}` was added ({actual})")),
        _ => None,
    }
}

fn describe(manifest: &CaseManifest) -> BTreeMap<String, String> {
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
        assertion_operation(
            fields,
            &base,
            assertion.equals.as_ref(),
            assertion.missing == Some(true),
        );
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
        assertion_operation(
            fields,
            &base,
            assertion.equals.as_ref(),
            assertion.missing == Some(true),
        );
    }
}

fn describe_lsp_assertions(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    for (index, assertion) in manifest.expectations.lsp_assertions.iter().enumerate() {
        let base = format!("expectations.lsp_assertions[{index}]");
        if let Some(id) = &assertion.id {
            fields.insert(format!("{base}.id"), canonical_json(id, &format!("{base}.id")));
        }
        optional_text(fields, &format!("{base}.method"), assertion.method.as_deref());
        if assertion.method.is_some() {
            scalar(
                fields,
                &format!("{base}.occurrence"),
                assertion.occurrence.unwrap_or(0),
            );
        }
        text(fields, &format!("{base}.path"), &assertion.path);
        match assertion
            .operation
            .as_ref()
            .expect("validated LSP assertion operation")
        {
            LspAssertionOperation::Equals(value) => {
                enum_value(fields, &format!("{base}.operation"), "equals");
                fields.insert(
                    format!("{base}.equals"),
                    canonical_json(value, &format!("{base}.equals")),
                );
            }
            LspAssertionOperation::EqualsFile(value) => {
                enum_value(fields, &format!("{base}.operation"), "equals_file");
                text(fields, &format!("{base}.equals_file"), value);
            }
            LspAssertionOperation::Contains(value) => {
                enum_value(fields, &format!("{base}.operation"), "contains");
                text(fields, &format!("{base}.contains"), value);
            }
            LspAssertionOperation::Missing(true) => {
                enum_value(fields, &format!("{base}.operation"), "missing");
            }
            LspAssertionOperation::Missing(false) => unreachable!("validated missing operation"),
        }
    }
}

fn describe_mcp_assertions(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    for (index, assertion) in manifest.expectations.mcp_assertions.iter().enumerate() {
        let base = format!("expectations.mcp_assertions[{index}]");
        fields.insert(
            format!("{base}.id"),
            canonical_json(
                assertion.id.as_ref().expect("validated MCP assertion id"),
                &format!("{base}.id"),
            ),
        );
        text(fields, &format!("{base}.path"), &assertion.path);
        match assertion
            .operation
            .as_ref()
            .expect("validated MCP assertion operation")
        {
            McpAssertionOperation::Equals(value) => {
                enum_value(fields, &format!("{base}.operation"), "equals");
                fields.insert(
                    format!("{base}.equals"),
                    canonical_json(value, &format!("{base}.equals")),
                );
            }
            McpAssertionOperation::ArrayLen(value) => {
                enum_value(fields, &format!("{base}.operation"), "array_len");
                scalar(fields, &format!("{base}.array_len"), value);
            }
            McpAssertionOperation::Missing(true) => {
                enum_value(fields, &format!("{base}.operation"), "missing");
            }
            McpAssertionOperation::Missing(false) => unreachable!("validated missing operation"),
            McpAssertionOperation::WorkspaceUri(value) => {
                enum_value(fields, &format!("{base}.operation"), "workspace_uri");
                text(fields, &format!("{base}.workspace_uri"), value);
            }
        }
    }
}

fn describe_file_assertions(fields: &mut BTreeMap<String, String>, manifest: &CaseManifest) {
    for (index, assertion) in manifest.expectations.file_assertions.iter().enumerate() {
        let base = format!("expectations.file_assertions[{index}]");
        text(fields, &format!("{base}.path"), &assertion.path);
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

fn assertion_operation(
    fields: &mut BTreeMap<String, String>,
    base: &str,
    equals: Option<&JsonValue>,
    missing: bool,
) {
    if missing {
        enum_value(fields, &format!("{base}.operation"), "missing");
    } else {
        enum_value(fields, &format!("{base}.operation"), "equals");
        let path = format!("{base}.equals");
        fields.insert(
            path.clone(),
            canonical_json(
                equals.expect("validated assertion should have equals"),
                &path,
            ),
        );
    }
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
                fields.insert(
                    path.clone(),
                    canonical_json(value, &path),
                );
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

fn fields_digest(fields: &BTreeMap<String, String>) -> String {
    let mut bytes = Vec::new();
    for (path, value) in fields {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(b'\n');
    }
    sha256(&bytes)
}
fn aggregate_digest(cases: &BTreeMap<String, BTreeMap<String, String>>) -> String {
    let mut bytes = Vec::new();
    for (id, fields) in cases {
        bytes.extend_from_slice(id.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(fields_digest(fields).as_bytes());
        bytes.push(b'\n');
    }
    sha256(&bytes)
}
fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn line(output: &mut String, kind: &str, value: &str) {
    output.push_str(kind);
    output.push('\t');
    output.push_str(value);
    output.push('\n');
}
fn json_string(value: &str) -> String {
    format!("\"{}\"", escape_json_string(value))
}

fn parse_json_string(value: &str, line_number: usize) -> Result<String, String> {
    parse_json(value)
        .map_err(|error| format!("baseline line {line_number} has invalid JSON string: {error}"))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("baseline line {line_number} value is not a JSON string"))
}

fn sample_inventory() -> Inventory {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\", \"main.veln\"]\nstdin = \"a\\nb\"\nexit = 0\n[env]\nB = \"2\"\nA = \"1\"\n[[json_assert]]\npath = \"value\"\nequals = {\"b\": [1, 2], \"a\": true}\n[[json_assert]]\npath = \"other\"\nmissing = true\n[stdout]\ncontains = [\"first\", \"second\"]\n",
    );
    Inventory {
        schema: SCHEMA.to_string(),
        roots: ROOTS.iter().map(|(root, _)| (*root).to_string()).collect(),
        source_git_tree: "sample".to_string(),
        cases: BTreeMap::from([(
            "tests/toolchain_cases/sample".to_string(),
            describe(&manifest),
        )]),
    }
}

#[test]
fn semantic_export_is_deterministic_and_round_trips() {
    let inventory = sample_inventory();
    let first = inventory.render();
    let second = inventory.render();
    assert_eq!(first, second);
    assert_eq!(Inventory::parse(&first).unwrap(), inventory);
}

#[test]
fn semantic_export_records_structured_jsonrpc_source_and_framed_stdin() {
    let root = test_temp_root("semantic-jsonrpc-input");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("requests.json"),
        r#"[{"jsonrpc":"2.0","id":1,"method":"shutdown"}]"#,
    )
    .expect("JSON-RPC fixture should be written");
    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
    );
    let fields = describe(&manifest);
    assert_eq!(
        fields["invocation.stdin_jsonrpc_file"],
        json_string("requests.json")
    );
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#;
    assert_eq!(
        fields["invocation.stdin"],
        json_string(&format!("Content-Length: {}\r\n\r\n{body}", body.len()))
    );
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn semantic_export_normalizes_object_order_and_hashes_large_exact_text() {
    let left = parse_manifest(
        Path::new("case.toml"),
        &format!(
            "command = [\"check\"]\nstdin = {:?}\nexit = 0\n[[json_assert]]\npath = \"value\"\nequals = {{\"b\": 2, \"a\": 1}}\n",
            "x".repeat(LARGE_TEXT_BYTES)
        ),
    );
    let right = parse_manifest(
        Path::new("case.toml"),
        &format!(
            "command = [\"check\"]\nstdin = {:?}\nexit = 0\n[[json_assert]]\npath = \"value\"\nequals = {{\"a\": 1, \"b\": 2}}\n",
            "x".repeat(LARGE_TEXT_BYTES)
        ),
    );
    let left = describe(&left);
    let right = describe(&right);
    assert_eq!(left, right);
    assert_eq!(
        left["invocation.stdin"],
        format!(
            "{{\"logical_field\":\"invocation.stdin\",\"byte_length\":{LARGE_TEXT_BYTES},\"sha256\":{}}}",
            json_string(&sha256("x".repeat(LARGE_TEXT_BYTES).as_bytes()))
        )
    );
}

#[test]
fn semantic_export_hashes_large_typed_json_strings_with_logical_fields() {
    let large = "x".repeat(LARGE_TEXT_BYTES);
    let manifest = parse_manifest(
        Path::new("case.toml"),
        &format!(
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"value\"\nequals = {{\"outer\": [{{\"inner/key\": {:?}}}]}}\n[[json_assert]]\npath = \"top\"\nequals = {:?}\n",
            large,
            large
        ),
    );
    let fields = describe(&manifest);
    let digest = json_string(&sha256(large.as_bytes()));
    assert_eq!(
        fields["expectations.json_assertions[0].equals"],
        format!(
            "{{\"outer\":[{{\"inner/key\":{{\"logical_field\":\"expectations.json_assertions[0].equals/outer/0/inner~1key\",\"byte_length\":{LARGE_TEXT_BYTES},\"sha256\":{digest}}}}}]}}"
        )
    );
    assert_eq!(
        fields["expectations.json_assertions[1].equals"],
        format!(
            "{{\"logical_field\":\"expectations.json_assertions[1].equals\",\"byte_length\":{LARGE_TEXT_BYTES},\"sha256\":{digest}}}"
        )
    );
}

#[test]
fn checked_in_semantic_baseline_matches_authoritative_cases() {
    let expected =
        Inventory::parse(BASELINE).expect("checked-in semantic baseline should be valid");
    let actual = Inventory::current(&expected.source_git_tree);
    compare(&expected, &actual).unwrap_or_else(|difference| panic!("toolchain case semantic baseline changed:\n{difference}\nGenerate a candidate only for a deliberate contract review."));
}

#[test]
fn comparator_rejects_case_membership_changes() {
    let expected = sample_inventory();
    let mut actual = expected.clone();
    actual
        .cases
        .insert("examples/specification/added".to_string(), BTreeMap::new());
    let error = compare(&expected, &actual).unwrap_err();
    assert!(error.contains("case set changed"));
    assert!(error.contains("examples/specification/added"));
}

#[test]
fn comparator_rejects_invocation_and_order_changes_with_field_context() {
    let expected = sample_inventory();
    for field in ["invocation.command[0]", "invocation.env[0].name"] {
        let mut actual = expected.clone();
        actual
            .cases
            .values_mut()
            .next()
            .unwrap()
            .insert(field.to_string(), json_string("changed"));
        let error = compare(&expected, &actual).unwrap_err();
        assert!(error.contains(field), "{error}");
    }

    let mut reordered = expected.clone();
    let fields = reordered.cases.values_mut().next().unwrap();
    fields.insert(
        "expectations.stdout.contains[0]".to_string(),
        json_string("second"),
    );
    fields.insert(
        "expectations.stdout.contains[1]".to_string(),
        json_string("first"),
    );
    let error = compare(&expected, &reordered).unwrap_err();
    assert!(error.contains("expectations.stdout.contains[0]"), "{error}");
}

#[test]
fn comparator_rejects_assertion_operation_typed_value_and_exact_bytes() {
    let expected = sample_inventory();
    for (field, value) in [
        (
            "expectations.json_assertions[0].operation",
            json_string("missing"),
        ),
        (
            "expectations.json_assertions[0].equals",
            "{\"a\":true,\"b\":[1,\"2\"]}".to_string(),
        ),
        ("invocation.stdin", json_string("a\r\nb")),
    ] {
        let mut actual = expected.clone();
        actual
            .cases
            .values_mut()
            .next()
            .unwrap()
            .insert(field.to_string(), value);
        let error = compare(&expected, &actual).unwrap_err();
        assert!(error.contains(field), "{error}");
    }
}

#[test]
#[ignore = "writes a deliberate baseline candidate"]
fn generate_toolchain_semantic_baseline_candidate() {
    let destination = std::env::var_os("VELN_TOOLCHAIN_BASELINE_CANDIDATE")
        .expect("set VELN_TOOLCHAIN_BASELINE_CANDIDATE to a candidate output path");
    let source_git_tree = std::env::var("VELN_TOOLCHAIN_SOURCE_GIT_TREE")
        .expect("set VELN_TOOLCHAIN_SOURCE_GIT_TREE to the reviewed source tree identifier");
    fs::write(destination, Inventory::current(&source_git_tree).render())
        .expect("semantic baseline candidate should be written");
}
