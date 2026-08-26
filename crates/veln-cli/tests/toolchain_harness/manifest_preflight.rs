use std::collections::BTreeSet;
use std::path::Path;

use super::{
    JsonValue, ManifestStatement, ManifestValue, Section, assertion_base_context, manifest_error,
    parse_bool, parse_manifest_json_value, parse_manifest_json_value_allow_decimal,
    parse_nonnegative_usize, parse_nonnegative_usize_raw_with_context, parse_string,
    validate_workspace_file_uri_operand_with_context, value_assertion_base_context,
};

pub(super) fn validate(path: &Path, statements: &[ManifestStatement<'_>]) {
    let mut preflight = ManifestAssignmentPreflight::new(path);
    for statement in statements {
        preflight.observe(statement);
    }
    preflight.finish();
}

struct ManifestAssignmentPreflight<'a> {
    path: &'a Path,
    section: Section,
    seen: BTreeSet<String>,
    root_stdin_operands: usize,
    json_operations: Vec<ValueAssertionPreflight>,
    result_value_operations: Vec<ValueAssertionPreflight>,
    lsp_operations: Vec<LspAssertionPreflight>,
    mcp_operations: Vec<McpAssertionPreflight>,
    file_assert_operations: Vec<usize>,
    file_assert_missing_false: Vec<bool>,
}

impl<'a> ManifestAssignmentPreflight<'a> {
    fn new(path: &'a Path) -> Self {
        Self {
            path,
            section: Section::Root,
            seen: BTreeSet::new(),
            root_stdin_operands: 0,
            json_operations: Vec::new(),
            result_value_operations: Vec::new(),
            lsp_operations: Vec::new(),
            mcp_operations: Vec::new(),
            file_assert_operations: Vec::new(),
            file_assert_missing_false: Vec::new(),
        }
    }

    fn observe(&mut self, statement: &ManifestStatement<'_>) {
        match statement {
            ManifestStatement::Section { name, .. } => self.enter_section(name),
            ManifestStatement::Assignment { key, line, value } => {
                self.record_assignment(key, *line, value);
            }
        }
    }

    fn enter_section(&mut self, name: &str) {
        let section = match name {
            "[[json_assert]]" => {
                self.json_operations
                    .push(ValueAssertionPreflight::default());
                Section::JsonAssert(self.json_operations.len() - 1)
            }
            "[[result_value_assert]]" => {
                self.result_value_operations
                    .push(ValueAssertionPreflight::default());
                Section::ResultValueAssert(self.result_value_operations.len() - 1)
            }
            "[[lsp_assert]]" => {
                self.lsp_operations.push(LspAssertionPreflight::default());
                Section::LspAssert(self.lsp_operations.len() - 1)
            }
            "[[mcp_assert]]" => {
                self.mcp_operations.push(McpAssertionPreflight::default());
                Section::McpAssert(self.mcp_operations.len() - 1)
            }
            "[[file_assert]]" => {
                self.file_assert_operations.push(0);
                self.file_assert_missing_false.push(false);
                Section::FileAssert(self.file_assert_operations.len() - 1)
            }
            _ => match fixed_section(name) {
                Some(section) => section,
                None => return,
            },
        };
        self.section = section;
        if matches!(
            self.section,
            Section::Diagnostic(0)
                | Section::DiagnosticSpan(0)
                | Section::BinaryFixture(0)
                | Section::OutputChunkList(0)
        ) {
            self.seen.clear();
        }
    }

    fn record_assignment(&mut self, key: &str, line: usize, value: &ManifestValue<'_>) {
        self.reject_duplicate_assignment(key, line);
        match self.section {
            Section::Root if matches!(key, "stdin" | "stdin_file" | "stdin_jsonrpc_file") => {
                self.root_stdin_operands += 1;
            }
            Section::JsonAssert(index) => {
                self.json_operations[index].record(self.path, key, value);
            }
            Section::ResultValueAssert(index) => {
                self.result_value_operations[index].record(self.path, key, value);
            }
            Section::LspAssert(index) if AssertionOperationPreflight::accepts_key(key) => {
                self.lsp_operations[index]
                    .operation
                    .record(self.path, key, value);
            }
            Section::McpAssert(index) if AssertionOperationPreflight::accepts_key(key) => {
                self.mcp_operations[index]
                    .operation
                    .record(self.path, key, value);
            }
            Section::LspAssert(index) => {
                self.lsp_operations[index].record_selector_or_path(self.path, key, value);
            }
            Section::McpAssert(index) => {
                self.mcp_operations[index].record_selector_or_path(self.path, key, value);
            }
            Section::FileAssert(index) if matches!(key, "equals" | "equals_file" | "missing") => {
                self.file_assert_operations[index] += 1;
                if key == "missing" && !parse_bool(self.path, value) {
                    self.file_assert_missing_false[index] = true;
                }
            }
            _ => {}
        }
    }

    fn reject_duplicate_assignment(&mut self, key: &str, line: usize) {
        if super::is_accumulating_manifest_key(self.section, key) {
            return;
        }
        let assignment = format!("{:?}:{key}", self.section);
        if !self.seen.insert(assignment) {
            manifest_error(self.path, line, format!("duplicate key `{key}`"));
        }
    }

    fn finish(self) {
        if self.root_stdin_operands > 1 {
            manifest_error(
                self.path,
                0,
                "root invocation needs at most one of `stdin`, `stdin_file`, or `stdin_jsonrpc_file`",
            );
        }
        for (index, assertion) in self.json_operations.iter().enumerate() {
            assertion.validate(self.path, "json_assert", index);
        }
        for (index, assertion) in self.result_value_operations.iter().enumerate() {
            assertion.validate(self.path, "result_value_assert", index);
        }
        for (index, assertion) in self.lsp_operations.iter().enumerate() {
            assertion.validate(self.path, index);
        }
        for (index, assertion) in self.mcp_operations.iter().enumerate() {
            assertion.validate(self.path, index);
        }
        for (index, count) in self.file_assert_operations.iter().enumerate() {
            if self.file_assert_missing_false[index] {
                manifest_error(
                    self.path,
                    0,
                    format!("file_assert {index} `missing` must be true when present"),
                );
            }
            if *count != 1 {
                manifest_error(
                    self.path,
                    0,
                    format!(
                        "file_assert {index} needs exactly one of `equals`, `equals_file`, or `missing = true`"
                    ),
                );
            }
        }
    }
}

fn fixed_section(name: &str) -> Option<Section> {
    Some(match name {
        "[stdout]" => Section::Stdout,
        "[stderr]" => Section::Stderr,
        "[help]" => Section::Help,
        "[requires]" => Section::Requires,
        "[skip]" => Section::Skip,
        "[env]" => Section::Env,
        "[tools]" => Section::Tools,
        "[[diagnostics]]" => Section::Diagnostic(0),
        "[diagnostics.span]" => Section::DiagnosticSpan(0),
        "[manifest_error]" => Section::ManifestError,
        "[[binary_fixture]]" => Section::BinaryFixture(0),
        "[[output_chunk_list]]" => Section::OutputChunkList(0),
        _ => return None,
    })
}

#[derive(Default)]
struct ValueAssertionPreflight {
    selected_path: Option<String>,
    operation: AssertionOperationPreflight,
}

impl ValueAssertionPreflight {
    fn record(&mut self, path: &Path, key: &str, value: &ManifestValue<'_>) {
        match key {
            "path" => self.selected_path = Some(parse_string(path, value)),
            key if AssertionOperationPreflight::accepts_key(key) => {
                self.operation.record(path, key, value);
            }
            _ => {}
        }
    }

    fn validate(&self, path: &Path, section: &str, index: usize) {
        self.operation.validate(
            path,
            section,
            index,
            &value_assertion_base_context(
                section,
                index,
                self.selected_path.as_deref().unwrap_or(""),
            ),
        );
    }
}

#[derive(Default)]
struct AssertionOperationPreflight {
    count: usize,
    missing_false: bool,
    length: Option<PreflightLengthOperand>,
    workspace_file_uri: Option<PreflightWorkspaceFileUriOperand>,
}

impl AssertionOperationPreflight {
    fn accepts_key(key: &str) -> bool {
        matches!(
            key,
            "equals"
                | "equals_file"
                | "equals_json_file"
                | "contains"
                | "length"
                | "workspace_file_uri"
                | "missing"
        )
    }

    fn record(&mut self, path: &Path, key: &str, value: &ManifestValue<'_>) {
        self.count += 1;
        if key == "missing" && !parse_bool(path, value) {
            self.missing_false = true;
        }
        if key == "length" {
            self.length = Some(PreflightLengthOperand {
                line_number: value.line(),
                raw: value.raw().to_string(),
            });
        }
        if key == "workspace_file_uri" {
            self.workspace_file_uri = Some(PreflightWorkspaceFileUriOperand {
                line_number: value.line(),
                relative: value.is_string().then(|| parse_string(path, value)),
                string_operand: value.is_string(),
            });
        }
    }

    fn validate(&self, path: &Path, section: &str, index: usize, base_context: &str) {
        if self.missing_false {
            manifest_error(
                path,
                0,
                format!("{section} {index} `missing` must be true when present"),
            );
        }
        if self.count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "{section} {index} needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
                ),
            );
        }
        self.validate_operands(path, base_context);
    }

    fn validate_operands(&self, path: &Path, base_context: &str) {
        if let Some(operand) = &self.length {
            let context = format!("{base_context} length");
            parse_nonnegative_usize_raw_with_context(
                path,
                operand.line_number,
                &operand.raw,
                Some(&context),
            );
        }
        if let Some(operand) = &self.workspace_file_uri {
            let context = format!("{base_context} workspace_file_uri");
            if !operand.string_operand {
                manifest_error(
                    path,
                    operand.line_number,
                    format!("{context}: expected string"),
                );
            }
            let relative = operand
                .relative
                .as_deref()
                .expect("validated workspace_file_uri string operand");
            validate_workspace_file_uri_operand_with_context(
                path,
                operand.line_number,
                relative,
                Some(&context),
            );
        }
    }
}

#[derive(Default)]
struct LspAssertionPreflight {
    id: Option<JsonValue>,
    method: Option<String>,
    occurrence: Option<usize>,
    path: Option<String>,
    operation: AssertionOperationPreflight,
}

impl LspAssertionPreflight {
    fn record_selector_or_path(&mut self, path: &Path, key: &str, value: &ManifestValue<'_>) {
        match key {
            "id" => self.id = Some(parse_manifest_json_value(path, value)),
            "method" => self.method = Some(parse_string(path, value)),
            "occurrence" => self.occurrence = Some(parse_nonnegative_usize(path, value)),
            "path" => self.path = Some(parse_string(path, value)),
            _ => {}
        }
    }

    fn validate(&self, path: &Path, index: usize) {
        self.operation.validate(
            path,
            "lsp_assert",
            index,
            &assertion_base_context(
                "lsp_assert",
                index,
                &self.selector(),
                self.path.as_deref().unwrap_or(""),
            ),
        );
    }

    fn selector(&self) -> String {
        if let Some(id) = &self.id {
            format!("response id {}", id.to_compact_string())
        } else if let Some(method) = &self.method {
            format!(
                "notification method {method:?} occurrence {}",
                self.occurrence.unwrap_or(0)
            )
        } else {
            "unresolved selector".to_string()
        }
    }
}

#[derive(Default)]
struct McpAssertionPreflight {
    id: Option<JsonValue>,
    path: Option<String>,
    operation: AssertionOperationPreflight,
}

impl McpAssertionPreflight {
    fn record_selector_or_path(&mut self, path: &Path, key: &str, value: &ManifestValue<'_>) {
        match key {
            "id" => self.id = Some(parse_manifest_json_value_allow_decimal(path, value)),
            "path" => self.path = Some(parse_string(path, value)),
            _ => {}
        }
    }

    fn validate(&self, path: &Path, index: usize) {
        self.operation.validate(
            path,
            "mcp_assert",
            index,
            &assertion_base_context(
                "mcp_assert",
                index,
                &self.selector(),
                self.path.as_deref().unwrap_or(""),
            ),
        );
    }

    fn selector(&self) -> String {
        if let Some(id) = &self.id {
            format!("response id {}", id.to_compact_string())
        } else {
            "unresolved selector".to_string()
        }
    }
}

#[derive(Default)]
struct PreflightLengthOperand {
    line_number: usize,
    raw: String,
}

#[derive(Default)]
struct PreflightWorkspaceFileUriOperand {
    line_number: usize,
    relative: Option<String>,
    string_operand: bool,
}
