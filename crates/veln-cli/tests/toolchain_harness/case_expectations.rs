use super::*;

#[derive(Debug)]
pub(super) struct CaseInvocation {
    pub(super) command: Vec<String>,
    pub(super) cwd: Option<PathBuf>,
    pub(super) stdin: Option<String>,
    pub(super) stdin_jsonrpc_file: Option<String>,
    pub(super) stdin_jsonrpc_workspace_file_uri_directives: Vec<WorkspaceFileUriDirective>,
    pub(super) repeat: usize,
    pub(super) env: Vec<(String, String)>,
}

impl CaseInvocation {
    pub(super) fn materialized_stdin(&self, project_root: &Path) -> Option<String> {
        let input = self.stdin.as_deref()?;
        if self.stdin_jsonrpc_file.is_some() {
            Some(materialize_jsonrpc_workspace_file_uri_directives(
                input,
                &self.stdin_jsonrpc_workspace_file_uri_directives,
                project_root,
            ))
        } else {
            Some(input.to_string())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkspaceFileUriDirective {
    pub(super) message_index: usize,
    pub(super) pointer_route: Vec<JsonPointerRouteSegment>,
    pub(super) relative: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum JsonPointerRouteSegment {
    ArrayIndex(usize),
    ObjectMember { key: String, occurrence: usize },
}

#[derive(Debug)]
pub(super) struct CaseExpectations {
    pub(super) exit: i32,
    pub(super) stdout: StreamExpectation,
    pub(super) stderr: StreamExpectation,
    pub(super) help: Option<HelpExpectation>,
    pub(super) json_assertions: Vec<JsonAssertion>,
    pub(super) result_value_assertions: Vec<ResultValueAssertion>,
    pub(super) lsp_assertions: Vec<LspAssertion>,
    pub(super) mcp_assertions: Vec<McpAssertion>,
    pub(super) file_assertions: Vec<FileAssertion>,
    pub(super) diagnostics: Vec<DiagnosticExpectation>,
    pub(super) binary_fixtures: Vec<BinaryFixtureExpectation>,
    pub(super) output_chunk_lists: Vec<OutputChunkListExpectation>,
}

#[derive(Debug)]
pub(super) struct CaseManifest {
    pub(super) invocation: CaseInvocation,
    pub(super) expectations: CaseExpectations,
    pub(super) source_errors: SourceErrorExpectation,
    pub(super) manifest_error: Option<ManifestErrorExpectation>,
    pub(super) tools: ToolSetup,
    pub(super) requires: Requirements,
    pub(super) skip: SkipRules,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum SourceErrorExpectation {
    #[default]
    Forbidden,
    Expected,
}

impl CaseManifest {
    pub(super) fn read(path: &Path) -> Self {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{}: failed to read manifest: {error}", path.display()));
        parse_manifest(path, &text)
    }

    pub(super) fn validate(&self, path: &Path) {
        self.expectations.validate(path);
        if !self.expectations.lsp_assertions.is_empty()
            && self.invocation.command.first().map(String::as_str) != Some("lsp")
        {
            manifest_error(path, 0, "lsp_assert requires `command = [\"lsp\", ...]`");
        }
        if !self.expectations.mcp_assertions.is_empty()
            && self.invocation.command.first().map(String::as_str) != Some("mcp")
        {
            manifest_error(path, 0, "mcp_assert requires `command = [\"mcp\", ...]`");
        }
        if let Some(expectation) = &self.manifest_error
            && !expectation.has_assertion()
        {
            manifest_error(path, 0, "manifest_error section has no assertion");
        }
    }

    pub(super) fn skip_reason(&self) -> Option<String> {
        if self.requires_jdk() && !jdk_is_available() {
            return Some("requires a real JDK".to_string());
        }
        if self
            .skip
            .platforms
            .iter()
            .any(|platform| platform.matches())
        {
            let reason = self
                .skip
                .reason
                .as_deref()
                .unwrap_or("case is skipped on this platform");
            return Some(reason.to_string());
        }
        None
    }

    pub(super) fn requires_jdk(&self) -> bool {
        self.requires.jdk || self.tools.requires_jdk()
    }

    pub(super) fn assert_no_unexpected_example_source_errors(
        &self,
        case_dir: &Path,
        project_root: &Path,
    ) {
        if !is_specification_example(case_dir) || !self.needs_independent_source_error_guard() {
            return;
        }
        if self.command_explicitly_expects_source_errors()
            && self.source_errors == SourceErrorExpectation::Forbidden
        {
            return;
        }

        let project = Project::discover(project_root.to_path_buf(), &[]).unwrap_or_else(|error| {
            panic!(
                "{}: inspect the example project inputs; source-error guard discovery failed: {error}",
                case_dir.display()
            )
        });
        let errors = checked_project_diagnostics(project, DoctestMode::Include)
            .into_iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .collect::<Vec<_>>();

        match (self.source_errors, errors.is_empty()) {
            (SourceErrorExpectation::Forbidden, false) => {
                panic!(
                    "{}: remove unexpected source error diagnostics, or set `source_errors = \"expected\"` in case.toml when this example exists to exercise them; clean examples prevent unrelated editor errors.\n{}",
                    case_dir.display(),
                    source_error_evidence(&errors)
                );
            }
            (SourceErrorExpectation::Expected, true) => {
                panic!(
                    "{}: remove stale `source_errors = \"expected\"` from case.toml; the example no longer produces a source error diagnostic",
                    case_dir.display()
                );
            }
            _ => {}
        }
    }

    pub(super) fn assert_no_unexpected_command_source_errors(
        &self,
        context: &CaseRunContext<'_>,
        evidence: &CommandSourceDiagnosticEvidence,
    ) {
        if evidence.error_count == 0 {
            return;
        }
        panic!(
            "{}: remove unexpected source error diagnostics, or set `source_errors = \"expected\"` in case.toml when this example exists to exercise them; clean examples prevent unrelated editor errors.\n{}",
            context.label(),
            evidence.message
        );
    }

    pub(super) fn needs_pre_command_source_error_guard(&self, case_dir: &Path) -> bool {
        is_specification_example(case_dir)
            && self.needs_independent_source_error_guard()
            && !self.needs_command_source_error_guard(case_dir)
            && !(self.command_explicitly_expects_source_errors()
                && self.source_errors == SourceErrorExpectation::Forbidden)
    }

    pub(super) fn needs_command_source_error_guard(&self, case_dir: &Path) -> bool {
        is_specification_example(case_dir)
            && self.source_errors == SourceErrorExpectation::Forbidden
            && !self.command_explicitly_expects_source_errors()
            && matches!(
                self.invocation.command.first().map(String::as_str),
                Some("check" | "run" | "test")
            )
    }

    pub(super) fn needs_independent_source_error_guard(&self) -> bool {
        matches!(
            self.invocation.command.first().map(String::as_str),
            Some("check" | "doc" | "fmt" | "lsp" | "metrics" | "repair" | "run" | "test")
        )
    }

    pub(super) fn command_explicitly_expects_source_errors(&self) -> bool {
        self.expectations.exit != 0
            && matches!(
                self.invocation.command.first().map(String::as_str),
                Some("check" | "doc" | "fmt" | "repair")
            )
            || self.invocation.command.first().map(String::as_str) == Some("run")
                && self.expectations.exit != 0
                && self
                    .expectations
                    .stderr
                    .contains
                    .iter()
                    .any(|fragment| fragment.contains("runnable entry retains user-defined effect"))
    }

    pub(super) fn validate_fixture_schema_references(&self, project_root: &Path) {
        if self
            .expectations
            .binary_fixtures
            .iter()
            .all(|fixture| fixture.schema.is_none())
        {
            return;
        }

        let inputs = command_source_inputs(&self.invocation.command);
        let project = Project::discover(project_root.to_path_buf(), &inputs)
            .unwrap_or_else(|error| manifest_error(project_root, 0, error));
        let current_module = fixture_reference_module(&project, inputs.first());
        let (module, _) = load_surface_module(&project);
        validate_binary_fixture_schema_references(
            project_root,
            &module,
            current_module.as_deref(),
            &self.expectations.binary_fixtures,
        );
    }
}

pub(super) fn is_specification_example(case_dir: &Path) -> bool {
    let components = case_dir.components().collect::<Vec<_>>();
    components
        .windows(2)
        .any(|pair| pair[0].as_os_str() == "examples" && pair[1].as_os_str() == "specification")
}

pub(super) fn source_error_evidence(errors: &[Diagnostic]) -> String {
    errors
        .iter()
        .map(|diagnostic| {
            let location = diagnostic.span.as_ref().map_or_else(
                || "<unknown>".to_string(),
                |span| {
                    format!(
                        "{}:{}:{}",
                        span.file.as_str(),
                        span.start.line,
                        span.start.column
                    )
                },
            );
            format!(
                "{location}: error[{}]: {}",
                diagnostic.id, diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) struct CommandSourceDiagnosticEvidence {
    pub(super) error_count: usize,
    pub(super) message: String,
}

impl CommandSourceDiagnosticEvidence {
    pub(super) fn read(context: &CaseRunContext<'_>, path: &Path) -> Self {
        let text = fs::read_to_string(path).unwrap_or_else(|error| {
            panic!(
                "{}: command did not write source diagnostic artifact `{}`: {error}",
                context.label(),
                path.display()
            )
        });
        let json = parse_json(&text).unwrap_or_else(|error| {
            panic!(
                "{}: source diagnostic artifact JSON parse failed: {error}\n{}",
                context.label(),
                text
            )
        });
        let diagnostics = json
            .object_field("diagnostics")
            .and_then(JsonValue::as_array)
            .unwrap_or_else(|| {
                panic!(
                    "{}: source diagnostic artifact is missing diagnostics array",
                    context.label()
                )
            });
        let errors = diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic
                    .object_field("severity")
                    .and_then(JsonValue::as_str)
                    == Some("error")
            })
            .collect::<Vec<_>>();
        Self {
            error_count: errors.len(),
            message: command_source_error_evidence(&errors),
        }
    }
}

pub(super) fn command_source_error_evidence(errors: &[&JsonValue]) -> String {
    errors
        .iter()
        .map(|diagnostic| {
            let location = diagnostic
                .object_field("span")
                .and_then(command_span_evidence)
                .unwrap_or_else(|| "<unknown>".to_string());
            let id = diagnostic
                .object_field("id")
                .and_then(JsonValue::as_str)
                .unwrap_or("<unknown>");
            let message = diagnostic
                .object_field("message")
                .and_then(JsonValue::as_str)
                .unwrap_or("<missing message>");
            format!("{location}: error[{id}]: {message}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn command_span_evidence(span: &JsonValue) -> Option<String> {
    if matches!(span, JsonValue::Null) {
        return None;
    }
    let file = span.object_field("file")?.as_str()?;
    let start = span.object_field("start")?;
    let line = start.object_field("line")?.as_i64()?;
    let column = start.object_field("column")?.as_i64()?;
    Some(format!("{file}:{line}:{column}"))
}

impl CaseExpectations {
    pub(super) fn validate(&self, path: &Path) {
        for (index, assertion) in self.json_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        for (index, assertion) in self.result_value_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        for (index, assertion) in self.lsp_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        for (index, assertion) in self.mcp_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        if let Some(help) = &self.help
            && !help.has_assertion()
        {
            manifest_error(path, 0, "help section has no assertion");
        }
        for (index, assertion) in self.file_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            diagnostic.validate(path, index);
        }
        for (index, fixture) in self.binary_fixtures.iter().enumerate() {
            fixture.validate(path, index);
        }
        for (index, chunks) in self.output_chunk_lists.iter().enumerate() {
            chunks.validate(path, index);
        }
    }

    pub(super) fn assert_matches(
        &self,
        context: &CaseRunContext<'_>,
        output: &CapturedOutput,
        project_root: &Path,
    ) {
        let independent_failures = self.independent_failure_messages(context, output, project_root);
        if !independent_failures.is_empty() {
            panic!("{}", independent_failures.join("\n"));
        }
        if let Some(help) = &self.help {
            help.assert_matches(context, output);
        }

        let json = if self.needs_stdout_json() {
            Some(parse_json(&output.stdout).unwrap_or_else(|error| {
                panic!(
                    "{}: stdout JSON parse failed: {error}\n{}",
                    context.label(),
                    output.stdout
                )
            }))
        } else {
            None
        };

        if let Some(json) = json.as_ref() {
            for (index, assertion) in self.json_assertions.iter().enumerate() {
                assert_json_path_in_workspace(context, json, assertion, index, project_root);
            }
            for (index, assertion) in self.result_value_assertions.iter().enumerate() {
                assert_result_value_path_in_workspace(
                    context,
                    json,
                    assertion,
                    index,
                    project_root,
                );
            }
            for diagnostic in &self.diagnostics {
                assert_diagnostic(context, json, diagnostic);
            }
        }

        if !self.binary_fixtures.is_empty() || !self.output_chunk_lists.is_empty() {
            let program_stdout = json
                .as_ref()
                .and_then(|json| json_path(json, "stdout"))
                .and_then(JsonValue::as_str)
                .unwrap_or(&output.stdout);
            for fixture in &self.binary_fixtures {
                assert_binary_fixture(context, program_stdout, fixture);
            }
            for chunks in &self.output_chunk_lists {
                assert_output_chunk_list(context, program_stdout, chunks);
            }
        }
    }

    pub(super) fn independent_failure_messages(
        &self,
        context: &CaseRunContext<'_>,
        output: &CapturedOutput,
        project_root: &Path,
    ) -> Vec<String> {
        let mut failures = Vec::new();
        collect_panic_failure(&mut failures, || {
            assert_eq!(
                output.exit,
                Some(self.exit),
                "{}: expected exit {}, got {:?}\nstdout:\n{}\nstderr:\n{}",
                context.label(),
                self.exit,
                output.exit,
                output.stdout,
                output.stderr
            );
        });
        collect_panic_failure(&mut failures, || {
            assert_stream(context, "stdout", &self.stdout, &output.stdout)
        });
        collect_panic_failure(&mut failures, || {
            assert_stream(context, "stderr", &self.stderr, &output.stderr)
        });
        collect_panic_failure(&mut failures, || {
            assert_lsp_assertions_in_workspace(
                context,
                &output.stdout,
                &self.lsp_assertions,
                project_root,
            )
        });
        collect_panic_failure(&mut failures, || {
            assert_mcp_assertions(context, &output.stdout, &self.mcp_assertions, project_root)
        });
        failures
    }

    pub(super) fn needs_stdout_json(&self) -> bool {
        self.stdout.format == Some(StreamFormat::Json)
            || !self.json_assertions.is_empty()
            || !self.result_value_assertions.is_empty()
            || !self.diagnostics.is_empty()
            || !self.binary_fixtures.is_empty()
            || !self.output_chunk_lists.is_empty()
    }

    pub(super) fn assert_files_match(&self, context: &CaseRunContext<'_>, project_root: &Path) {
        for assertion in &self.file_assertions {
            let path = project_root.join(&assertion.path);
            if assertion.missing {
                assert!(
                    !path.exists(),
                    "{}: file `{}` exists but should be missing",
                    context.label(),
                    assertion.path
                );
                continue;
            }
            let actual = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!(
                    "{}: failed to read asserted file `{}`: {error}",
                    context.label(),
                    assertion.path
                )
            });
            assert_eq!(
                actual,
                assertion
                    .equals
                    .as_ref()
                    .expect("file assertion should have expected text")
                    .as_str(),
                "{}: file `{}` contents mismatch",
                context.label(),
                assertion.path
            );
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct ManifestErrorExpectation {
    pub(super) contains: Vec<String>,
}

impl ManifestErrorExpectation {
    pub(super) fn assert_matches(&self, case_dir: &Path, message: &str) {
        for expected in &self.contains {
            assert!(
                message.contains(expected),
                "{}: manifest error should contain `{expected}`, got `{message}`",
                case_dir.display()
            );
        }
    }

    pub(super) fn has_assertion(&self) -> bool {
        !self.contains.is_empty()
    }
}

pub(super) struct CaseRunContext<'a> {
    pub(super) case_dir: &'a Path,
    pub(super) run_number: usize,
}

impl CaseRunContext<'_> {
    pub(super) fn label(&self) -> String {
        format!("{} run {}", self.case_dir.display(), self.run_number)
    }
}

pub(super) struct CapturedOutput {
    pub(super) exit: Option<i32>,
    pub(super) stdout: String,
    pub(super) stderr: String,
}

impl CapturedOutput {
    pub(super) fn read(context: &CaseRunContext<'_>, output: Output) -> Self {
        Self {
            exit: output.status.code(),
            stdout: stream_text(output.stdout, context, "stdout"),
            stderr: stream_text(output.stderr, context, "stderr"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct StreamExpectation {
    pub(super) format: Option<StreamFormat>,
    pub(super) contains: Vec<String>,
    pub(super) not_contains: Vec<String>,
    pub(super) equals: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StreamFormat {
    Empty,
    Text,
    Json,
}

#[derive(Debug)]
pub(super) struct HelpExpectation {
    pub(super) stream: OutputStream,
    pub(super) summary: Option<String>,
    pub(super) usage: Option<String>,
    pub(super) commands: Vec<String>,
    pub(super) arguments: Vec<String>,
    pub(super) options: Vec<String>,
    pub(super) contains: Vec<String>,
}

impl Default for HelpExpectation {
    fn default() -> Self {
        Self {
            stream: OutputStream::Stdout,
            summary: None,
            usage: None,
            commands: Vec::new(),
            arguments: Vec::new(),
            options: Vec::new(),
            contains: Vec::new(),
        }
    }
}

impl HelpExpectation {
    pub(super) fn assert_matches(&self, context: &CaseRunContext<'_>, output: &CapturedOutput) {
        let stream = self.stream.text(output);
        let stream_name = self.stream.name();
        let help_surface = format!("help {stream_name}");
        if let Some(summary) = &self.summary {
            assert_eq!(
                stream.lines().next(),
                Some(summary.as_str()),
                "{}: help summary mismatch on {}",
                context.label(),
                stream_name
            );
        }
        if let Some(usage) = &self.usage {
            assert_contains_fragment(context, &help_surface, stream, &format!("Usage: {usage}\n"));
        }
        assert_help_section(context, &help_surface, stream, "Commands", &self.commands);
        assert_help_section(context, &help_surface, stream, "Arguments", &self.arguments);
        assert_help_section(context, &help_surface, stream, "Options", &self.options);
        for fragment in &self.contains {
            assert_contains_fragment(context, &help_surface, stream, fragment);
        }
    }

    pub(super) fn has_assertion(&self) -> bool {
        self.summary.is_some()
            || self.usage.is_some()
            || !self.commands.is_empty()
            || !self.arguments.is_empty()
            || !self.options.is_empty()
            || !self.contains.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OutputStream {
    Stdout,
    Stderr,
}

impl OutputStream {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }

    pub(super) fn text(self, output: &CapturedOutput) -> &str {
        match self {
            Self::Stdout => &output.stdout,
            Self::Stderr => &output.stderr,
        }
    }

    pub(super) fn parse(path: &Path, value: &ManifestValue<'_>) -> Self {
        let line_number = value.line();
        let value = parse_string(path, value);
        match value.as_str() {
            "stdout" => Self::Stdout,
            "stderr" => Self::Stderr,
            _ => manifest_error(
                path,
                line_number,
                format!("unknown output stream `{value}`"),
            ),
        }
    }
}
