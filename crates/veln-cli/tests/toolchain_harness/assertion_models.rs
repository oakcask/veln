use super::*;

#[derive(Debug)]
pub(super) struct JsonAssertion {
    pub(super) path: String,
    pub(super) operation: Option<ValueAssertionOperation>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ValueAssertionOperation {
    Equals(JsonValue),
    EqualsFile(JsonValue),
    EqualsJsonFile(JsonValue),
    Contains(String),
    Length(usize),
    Missing,
    WorkspaceFileUri(String),
}

#[derive(Debug)]
pub(super) struct LspAssertion {
    pub(super) id: Option<JsonValue>,
    pub(super) method: Option<String>,
    pub(super) occurrence: Option<usize>,
    pub(super) path: String,
    pub(super) path_present: bool,
    pub(super) pointer_tokens: Vec<String>,
    pub(super) operation: Option<RpcAssertionOperation>,
    pub(super) operation_count: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum RpcAssertionOperation {
    Equals(JsonValue),
    EqualsFile(String),
    EqualsFileRef(CaseTextReference),
    EqualsJsonFile(JsonValue),
    EqualsJsonFileRef(CaseTextReference),
    Contains(String),
    Length(usize),
    Missing(bool),
    WorkspaceFileUri(String),
}

#[derive(Debug)]
pub(super) struct McpAssertion {
    pub(super) id: Option<JsonValue>,
    pub(super) path: String,
    pub(super) path_present: bool,
    pub(super) pointer_tokens: Vec<String>,
    pub(super) operation: Option<RpcAssertionOperation>,
    pub(super) operation_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CaseTextReference {
    pub(super) line_number: usize,
    pub(super) relative: String,
}

impl LspAssertion {
    pub(super) fn validate(&self, path: &Path, index: usize) {
        if self.id.is_some() == self.method.is_some() {
            manifest_error(
                path,
                0,
                format!("lsp_assert {index} needs exactly one of `id` or `method`"),
            );
        }
        if !self.path_present {
            manifest_error(path, 0, format!("lsp_assert {index} is missing `path`"));
        }
        if self.occurrence.is_some() && self.method.is_none() {
            manifest_error(
                path,
                0,
                format!("lsp_assert {index} `occurrence` is valid only with `method`"),
            );
        }
        if matches!(self.operation, Some(RpcAssertionOperation::Missing(false))) {
            manifest_error(
                path,
                0,
                format!("lsp_assert {index} `missing` must be true when present"),
            );
        }
        if self.operation_count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "lsp_assert {index} needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
                ),
            );
        }
    }

    pub(super) fn selector(&self) -> String {
        if let Some(id) = &self.id {
            format!("response id {}", id.to_compact_string())
        } else {
            format!(
                "notification method {:?} occurrence {}",
                self.method.as_deref().expect("validated method selector"),
                self.occurrence.unwrap_or(0)
            )
        }
    }
}

impl JsonAssertion {
    pub(super) fn validate(&self, path: &Path, index: usize) {
        if self.path.is_empty() {
            manifest_error(path, 0, format!("json_assert {index} is missing `path`"));
        }
    }
}

impl McpAssertion {
    pub(super) fn validate(&self, path: &Path, index: usize) {
        if self.id.is_none() {
            manifest_error(path, 0, format!("mcp_assert {index} is missing `id`"));
        }
        if !self.path_present {
            manifest_error(path, 0, format!("mcp_assert {index} is missing `path`"));
        }
        if matches!(self.operation, Some(RpcAssertionOperation::Missing(false))) {
            manifest_error(
                path,
                0,
                format!("mcp_assert {index} `missing` must be true when present"),
            );
        }
        if self.operation_count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "mcp_assert {index} needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
                ),
            );
        }
    }

    pub(super) fn selector(&self) -> String {
        format!(
            "response id {}",
            self.id
                .as_ref()
                .expect("validated MCP id")
                .to_compact_string()
        )
    }
}

#[derive(Debug)]
pub(super) struct ResultValueAssertion {
    pub(super) value_path: String,
    pub(super) path: String,
    pub(super) operation: Option<ValueAssertionOperation>,
}

impl ResultValueAssertion {
    pub(super) fn validate(&self, path: &Path, index: usize) {
        if self.value_path.is_empty() {
            manifest_error(
                path,
                0,
                format!("result_value_assert {index} is missing `value_path`"),
            );
        }
        if self.path.is_empty() {
            manifest_error(
                path,
                0,
                format!("result_value_assert {index} is missing `path`"),
            );
        }
    }
}

#[derive(Debug)]
pub(super) struct FileAssertion {
    pub(super) path: String,
    pub(super) equals: Option<String>,
    pub(super) missing: bool,
    pub(super) operation_count: usize,
}

impl FileAssertion {
    pub(super) fn validate(&self, path: &Path, index: usize) {
        if self.path.is_empty() {
            manifest_error(path, 0, format!("file_assert {index} is missing `path`"));
        }
        if self.operation_count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "file_assert {index} needs exactly one of `equals`, `equals_file`, or `missing = true`"
                ),
            );
        }
    }
}

#[derive(Debug)]
pub(super) struct DiagnosticExpectation {
    pub(super) id: String,
    pub(super) severity: Option<String>,
    pub(super) kind: Option<String>,
    pub(super) message: Option<String>,
    pub(super) span: Option<SpanExpectation>,
}

impl DiagnosticExpectation {
    pub(super) fn validate(&self, path: &Path, index: usize) {
        if self.id.is_empty() {
            manifest_error(path, 0, format!("diagnostics {index} is missing `id`"));
        }
    }
}

#[derive(Debug)]
pub(super) struct BinaryFixtureExpectation {
    pub(super) name: String,
    pub(super) schema: Option<String>,
    pub(super) bytes: Option<BinaryFixtureBytes>,
    pub(super) consumed: Option<usize>,
    pub(super) error: Option<String>,
    pub(super) byte_diagnostic: Option<BinaryFixtureByteDiagnostic>,
}

impl BinaryFixtureExpectation {
    pub(super) fn validate(&self, path: &Path, index: usize) {
        if self.name.is_empty() {
            manifest_error(path, 0, format!("binary_fixture {index} is missing `name`"));
        }
        match (&self.bytes, &self.error) {
            (Some(_), None) => {}
            (None, Some(_)) if self.consumed.is_none() => {}
            (Some(_), Some(_)) => manifest_error(
                path,
                0,
                format!("binary_fixture {index} cannot specify both `hex` and `error`"),
            ),
            (None, Some(_)) => manifest_error(
                path,
                0,
                format!("binary_fixture {index} with `error` cannot specify `consumed`"),
            ),
            (None, None) => manifest_error(
                path,
                0,
                format!("binary_fixture {index} needs `hex` or `error`"),
            ),
        }
        if let (Some(bytes), Some(consumed)) = (&self.bytes, self.consumed)
            && consumed > bytes.bytes.len()
        {
            manifest_error(
                path,
                0,
                format!("binary_fixture {index} `consumed` exceeds decoded byte count"),
            );
        }
        if let Some(diagnostic) = &self.byte_diagnostic {
            diagnostic.validate(path, index, self.bytes.is_some());
        }
    }
}

#[derive(Debug)]
pub(super) struct BinaryFixtureBytes {
    pub(super) hex: String,
    pub(super) bytes: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct OutputChunkListExpectation {
    pub(super) name: String,
    pub(super) chunks: Option<Vec<BinaryFixtureBytes>>,
}

impl OutputChunkListExpectation {
    pub(super) fn validate(&self, path: &Path, index: usize) {
        if self.name.is_empty() {
            manifest_error(
                path,
                0,
                format!("output_chunk_list {index} is missing `name`"),
            );
        }
        if self.chunks.is_none() {
            manifest_error(
                path,
                0,
                format!("output_chunk_list {index} is missing `chunks`"),
            );
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct BinaryFixtureByteDiagnostic {
    pub(super) diagnostic_id: Option<String>,
    pub(super) byte_offset: Option<usize>,
    pub(super) expected_count: Option<usize>,
    pub(super) available_count: Option<usize>,
    pub(super) readiness: Option<String>,
    pub(super) field_path: Option<JsonValue>,
}

impl BinaryFixtureByteDiagnostic {
    pub(super) fn validate(&self, path: &Path, fixture_index: usize, fixture_has_bytes: bool) {
        if !fixture_has_bytes {
            manifest_error(
                path,
                0,
                format!("binary_fixture {fixture_index} byte diagnostic metadata needs `hex`"),
            );
        }
        if self.byte_offset.is_none() || self.field_path.is_none() {
            manifest_error(
                path,
                0,
                format!("binary_fixture {fixture_index} has incomplete byte diagnostic metadata"),
            );
        }
        validate_binary_fixture_field_path(path, fixture_index, self.field_path.as_ref());

        let has_count_metadata = self.expected_count.is_some()
            || self.available_count.is_some()
            || self.readiness.is_some();
        if has_count_metadata
            && (self.expected_count.is_none()
                || self.available_count.is_none()
                || self.readiness.is_none())
        {
            manifest_error(
                path,
                0,
                format!("binary_fixture {fixture_index} has incomplete byte count metadata"),
            );
        }
        if self.diagnostic_id.is_none() && !has_count_metadata {
            manifest_error(
                path,
                0,
                format!("binary_fixture {fixture_index} needs `diagnostic_id` for field metadata"),
            );
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct SpanExpectation {
    pub(super) file: Option<String>,
    pub(super) line: Option<i64>,
    pub(super) column: Option<i64>,
}

#[derive(Debug, Default)]
pub(super) struct Requirements {
    pub(super) jdk: bool,
}

#[derive(Debug, Default)]
pub(super) struct ToolSetup {
    pub(super) java: Option<ToolAvailability>,
    pub(super) git: Option<ToolAvailability>,
}

impl ToolSetup {
    pub(super) fn needs_path(&self) -> bool {
        self.configured().next().is_some()
    }

    pub(super) fn requires_jdk(&self) -> bool {
        self.configured().any(ToolConfig::requires_jdk)
    }

    pub(super) fn configured(&self) -> impl Iterator<Item = ToolConfig> {
        [
            self.java
                .map(|availability| ToolName::Java.config(availability)),
            self.git
                .map(|availability| ToolName::Git.config(availability)),
        ]
        .into_iter()
        .flatten()
    }

    pub(super) fn set(&mut self, name: ToolName, availability: ToolAvailability) {
        match name {
            ToolName::Java => self.java = Some(availability),
            ToolName::Git => self.git = Some(availability),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ToolConfig {
    pub(super) name: ToolName,
    pub(super) availability: ToolAvailability,
}

impl ToolConfig {
    pub(super) fn requires_jdk(self) -> bool {
        self.name == ToolName::Java && self.availability == ToolAvailability::Real
    }

    pub(super) fn setup(self, tool_path: &Path) {
        setup_tool(tool_path, self.name.as_str(), self.availability);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolName {
    Java,
    Git,
}

impl ToolName {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Java => "java",
            Self::Git => "git",
        }
    }

    pub(super) fn config(self, availability: ToolAvailability) -> ToolConfig {
        ToolConfig {
            name: self,
            availability,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolAvailability {
    Missing,
    FakeSuccess,
    FakeGitRevParse,
    Real,
}

#[derive(Debug, Default)]
pub(super) struct SkipRules {
    pub(super) platforms: Vec<SkipPlatform>,
    pub(super) reason: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SkipPlatform {
    Unix,
    Windows,
    Macos,
    Linux,
}

impl SkipPlatform {
    pub(super) fn matches(self) -> bool {
        match self {
            Self::Unix => cfg!(unix),
            Self::Windows => cfg!(windows),
            Self::Macos => cfg!(target_os = "macos"),
            Self::Linux => cfg!(target_os = "linux"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Section {
    Root,
    Stdout,
    Stderr,
    Help,
    JsonAssert(usize),
    ResultValueAssert(usize),
    LspAssert(usize),
    McpAssert(usize),
    FileAssert(usize),
    Diagnostic(usize),
    DiagnosticSpan(usize),
    ManifestError,
    BinaryFixture(usize),
    OutputChunkList(usize),
    Requires,
    Skip,
    Env,
    Tools,
}
