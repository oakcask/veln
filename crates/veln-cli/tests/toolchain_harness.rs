use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use veln_analysis::{derive_source_module_path, load_surface_module};
use veln_ast::{FunctionKind, PublicAliasKind, SurfaceModule, UseDecl, Visibility};
use veln_project::Project;

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

include!(concat!(env!("OUT_DIR"), "/toolchain_cases.rs"));

fn toolchain_case_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn run_case(case_dir: &Path) {
    let manifest = CaseManifest::read(&case_dir.join("case.toml"));
    if let Some(reason) = manifest.skip_reason() {
        eprintln!("skipping {}: {reason}", case_dir.display());
        return;
    }

    let project = TestProject::new(case_name(case_dir), &manifest.tools);
    project.copy_fixtures(case_dir);
    project.setup_tools(&manifest.tools);
    if let Some(expected_error) = &manifest.manifest_error {
        let panic = std::panic::catch_unwind(|| {
            manifest.validate_fixture_schema_references(&project.root);
        })
        .expect_err("case should fail manifest validation");
        let message = panic_message(panic);
        expected_error.assert_matches(case_dir, &message);
        return;
    }

    manifest.validate_fixture_schema_references(&project.root);

    for run_index in 0..manifest.invocation.repeat {
        let context = CaseRunContext {
            case_dir,
            run_number: run_index + 1,
        };
        let output = CapturedOutput::read(
            &context,
            project.veln(
                &manifest.invocation.command,
                &manifest.invocation.env,
                manifest.invocation.stdin.as_deref(),
            ),
        );
        manifest.expectations.assert_matches(&context, &output);
        manifest
            .expectations
            .assert_files_match(&context, &project.root);
    }
}

fn case_name(case_dir: &Path) -> String {
    case_dir
        .components()
        .rev()
        .take(2)
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("-")
}

struct TestProject {
    root: PathBuf,
    tool_path: Option<PathBuf>,
}

impl TestProject {
    fn new(name: String, tools: &ToolSetup) -> Self {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-toolchain-{name}-{}-{nanos}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("test project directory should be created");
        let tool_path = tools.needs_path().then(|| root.join(".veln-harness-tools"));
        Self { root, tool_path }
    }

    fn copy_fixtures(&self, case_dir: &Path) {
        copy_fixture_dir(case_dir, case_dir, &self.root);
    }

    fn veln(&self, args: &[String], env: &[(String, String)], stdin: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(&self.root);
        command.args(args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        if let Some(path) = &self.tool_path {
            command.env("PATH", path);
        }
        for (name, value) in env {
            command.env(name, value);
        }
        if stdin.is_some() {
            command.stdin(Stdio::piped());
        }
        let mut child = command.spawn().expect("veln should spawn");
        if let Some(input) = stdin {
            let mut child_stdin = child.stdin.take().expect("veln stdin should be piped");
            child_stdin
                .write_all(input.as_bytes())
                .expect("veln stdin should be written");
        }
        child.wait_with_output().expect("veln should run")
    }

    fn setup_tools(&self, tools: &ToolSetup) {
        let Some(tool_path) = &self.tool_path else {
            return;
        };
        fs::create_dir_all(tool_path).expect("tool directory should be created");

        for tool in tools.configured() {
            tool.setup(tool_path);
        }
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn copy_fixture_dir(base: &Path, dir: &Path, target_root: &Path) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: failed to read fixtures: {error}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("{}: failed to read fixture entry: {error}", dir.display())
        });
        let source = entry.path();
        let relative = source
            .strip_prefix(base)
            .expect("fixture should be under case directory");
        if relative == Path::new("case.toml") {
            continue;
        }

        let target = target_root.join(relative);
        if source.is_dir() {
            fs::create_dir_all(&target).unwrap_or_else(|error| {
                panic!(
                    "{}: failed to create fixture directory: {error}",
                    target.display()
                )
            });
            copy_fixture_dir(base, &source, target_root);
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|error| {
                    panic!(
                        "{}: failed to create fixture parent: {error}",
                        parent.display()
                    )
                });
            }
            fs::copy(&source, &target).unwrap_or_else(|error| {
                panic!(
                    "{}: failed to copy fixture to {}: {error}",
                    source.display(),
                    target.display()
                )
            });
        }
    }
}

fn setup_tool(tool_path: &Path, name: &str, availability: ToolAvailability) {
    match availability {
        ToolAvailability::Missing => {}
        ToolAvailability::FakeSuccess => write_fake_success_tool(tool_path, name),
        ToolAvailability::FakeGitRevParse => write_fake_git_rev_parse_tool(tool_path, name),
        ToolAvailability::Real => {
            let host_tool = find_host_tool(name)
                .unwrap_or_else(|| panic!("host tool `{name}` should be available"));
            install_real_tool(tool_path, name, &host_tool);
        }
    }
}

const FAKE_GIT_RESOLVED_REV: &str = "0123456789abcdef0123456789abcdef01234567";

#[cfg(unix)]
fn write_fake_success_tool(tool_path: &Path, name: &str) {
    let tool = tool_path.join(name);
    fs::write(&tool, "#!/bin/sh\nexit 0\n").expect("fake tool should be written");
    let mut permissions = fs::metadata(&tool)
        .expect("fake tool metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tool, permissions).expect("fake tool should be executable");
}

#[cfg(windows)]
fn write_fake_success_tool(tool_path: &Path, name: &str) {
    for extension in ["bat", "cmd"] {
        let tool = tool_path.join(format!("{name}.{extension}"));
        fs::write(&tool, "@echo off\r\nexit /b 0\r\n").expect("fake tool should be written");
    }
}

#[cfg(not(any(unix, windows)))]
fn write_fake_success_tool(_tool_path: &Path, name: &str) {
    panic!("fake tool `{name}` is not supported on this platform");
}

#[cfg(unix)]
fn write_fake_git_rev_parse_tool(tool_path: &Path, name: &str) {
    assert_eq!(name, "git", "fake git rev-parse is only valid for git");
    let tool = tool_path.join(name);
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nset -eu\nif [ \"$1\" = \"clone\" ]; then\n  shift\n  if [ \"$1\" = \"--no-checkout\" ]; then\n    shift\n  fi\n  url=\"$1\"\n  dest=\"$2\"\n  name=\"${{url##*/}}\"\n  name=\"${{name%.git}}\"\n  remote=\"$PWD/.fake-git-remotes/$name\"\n  command -p mkdir -p \"$dest\"\n  command -p cp -R \"$remote/.\" \"$dest/\"\n  exit 0\nfi\nif [ \"$1\" = \"-C\" ]; then\n  shift 2\n  if [ \"$1\" = \"fetch\" ] || [ \"$1\" = \"checkout\" ] || [ \"$1\" = \"clean\" ]; then\n    exit 0\n  fi\n  if [ \"$1\" = \"rev-parse\" ] && [ \"$2\" = \"--verify\" ]; then\n    echo \"{FAKE_GIT_RESOLVED_REV}\"\n    exit 0\n  fi\nfi\nexit 1\n"
        ),
    )
    .expect("fake git should be written");
    let mut permissions = fs::metadata(&tool)
        .expect("fake git metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tool, permissions).expect("fake git should be executable");
}

#[cfg(windows)]
fn write_fake_git_rev_parse_tool(tool_path: &Path, name: &str) {
    assert_eq!(name, "git", "fake git rev-parse is only valid for git");
    for extension in ["bat", "cmd"] {
        let tool = tool_path.join(format!("{name}.{extension}"));
        fs::write(
            &tool,
            format!(
                "@echo off\r\nif \"%1\"==\"-C\" if \"%3\"==\"rev-parse\" if \"%4\"==\"--verify\" (\r\n  echo {FAKE_GIT_RESOLVED_REV}\r\n  exit /b 0\r\n)\r\nexit /b 1\r\n"
            ),
        )
        .expect("fake git should be written");
    }
}

#[cfg(not(any(unix, windows)))]
fn write_fake_git_rev_parse_tool(_tool_path: &Path, name: &str) {
    panic!("fake git `{name}` is not supported on this platform");
}

fn find_host_tool(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for candidate_name in host_tool_names(name) {
            let candidate = dir.join(candidate_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(windows)]
fn host_tool_names(name: &str) -> Vec<String> {
    vec![
        format!("{name}.exe"),
        format!("{name}.cmd"),
        format!("{name}.bat"),
        name.to_string(),
    ]
}

#[cfg(not(windows))]
fn host_tool_names(name: &str) -> Vec<String> {
    vec![name.to_string()]
}

#[cfg(unix)]
fn install_real_tool(tool_path: &Path, name: &str, host_tool: &Path) {
    std::os::unix::fs::symlink(host_tool, tool_path.join(name))
        .expect("real tool symlink should be created");
}

#[cfg(windows)]
fn install_real_tool(tool_path: &Path, name: &str, host_tool: &Path) {
    let extension = host_tool
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("exe");
    fs::copy(host_tool, tool_path.join(format!("{name}.{extension}")))
        .expect("real tool should be copied");
}

#[cfg(not(any(unix, windows)))]
fn install_real_tool(_tool_path: &Path, name: &str, _host_tool: &Path) {
    panic!("real tool `{name}` is not supported on this platform");
}

#[derive(Debug)]
struct CaseInvocation {
    command: Vec<String>,
    stdin: Option<String>,
    repeat: usize,
    env: Vec<(String, String)>,
}

#[derive(Debug)]
struct CaseExpectations {
    exit: i32,
    stdout: StreamExpectation,
    stderr: StreamExpectation,
    help: Option<HelpExpectation>,
    json_assertions: Vec<JsonAssertion>,
    result_value_assertions: Vec<ResultValueAssertion>,
    file_assertions: Vec<FileAssertion>,
    diagnostics: Vec<DiagnosticExpectation>,
    binary_fixtures: Vec<BinaryFixtureExpectation>,
    output_chunk_lists: Vec<OutputChunkListExpectation>,
}

#[derive(Debug)]
struct CaseManifest {
    invocation: CaseInvocation,
    expectations: CaseExpectations,
    manifest_error: Option<ManifestErrorExpectation>,
    tools: ToolSetup,
    requires: Requirements,
    skip: SkipRules,
}

impl CaseManifest {
    fn read(path: &Path) -> Self {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{}: failed to read manifest: {error}", path.display()));
        parse_manifest(path, &text)
    }

    fn skip_reason(&self) -> Option<String> {
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

    fn requires_jdk(&self) -> bool {
        self.requires.jdk || self.tools.requires_jdk()
    }

    fn validate_fixture_schema_references(&self, project_root: &Path) {
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

impl CaseExpectations {
    fn assert_matches(&self, context: &CaseRunContext<'_>, output: &CapturedOutput) {
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

        assert_stream(context, "stdout", &self.stdout, &output.stdout);
        assert_stream(context, "stderr", &self.stderr, &output.stderr);
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
            for assertion in &self.json_assertions {
                assert_json_path(context, json, assertion);
            }
            for assertion in &self.result_value_assertions {
                assert_result_value_path(context, json, assertion);
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

    fn needs_stdout_json(&self) -> bool {
        self.stdout.format == Some(StreamFormat::Json)
            || !self.json_assertions.is_empty()
            || !self.result_value_assertions.is_empty()
            || !self.diagnostics.is_empty()
            || !self.binary_fixtures.is_empty()
            || !self.output_chunk_lists.is_empty()
    }

    fn assert_files_match(&self, context: &CaseRunContext<'_>, project_root: &Path) {
        for assertion in &self.file_assertions {
            let path = project_root.join(&assertion.path);
            let actual = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!(
                    "{}: failed to read asserted file `{}`: {error}",
                    context.label(),
                    assertion.path
                )
            });
            assert_eq!(
                actual,
                assertion.equals,
                "{}: file `{}` contents mismatch",
                context.label(),
                assertion.path
            );
        }
    }
}

#[derive(Debug, Default)]
struct ManifestErrorExpectation {
    contains: Vec<String>,
}

impl ManifestErrorExpectation {
    fn assert_matches(&self, case_dir: &Path, message: &str) {
        for expected in &self.contains {
            assert!(
                message.contains(expected),
                "{}: manifest error should contain `{expected}`, got `{message}`",
                case_dir.display()
            );
        }
    }

    fn has_assertion(&self) -> bool {
        !self.contains.is_empty()
    }
}

struct CaseRunContext<'a> {
    case_dir: &'a Path,
    run_number: usize,
}

impl CaseRunContext<'_> {
    fn label(&self) -> String {
        format!("{} run {}", self.case_dir.display(), self.run_number)
    }
}

struct CapturedOutput {
    exit: Option<i32>,
    stdout: String,
    stderr: String,
}

impl CapturedOutput {
    fn read(context: &CaseRunContext<'_>, output: Output) -> Self {
        Self {
            exit: output.status.code(),
            stdout: stream_text(output.stdout, context, "stdout"),
            stderr: stream_text(output.stderr, context, "stderr"),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct StreamExpectation {
    format: Option<StreamFormat>,
    contains: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamFormat {
    Empty,
    Text,
    Json,
}

#[derive(Debug)]
struct HelpExpectation {
    stream: OutputStream,
    summary: Option<String>,
    usage: Option<String>,
    commands: Vec<String>,
    arguments: Vec<String>,
    options: Vec<String>,
    contains: Vec<String>,
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
    fn assert_matches(&self, context: &CaseRunContext<'_>, output: &CapturedOutput) {
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

    fn has_assertion(&self) -> bool {
        self.summary.is_some()
            || self.usage.is_some()
            || !self.commands.is_empty()
            || !self.arguments.is_empty()
            || !self.options.is_empty()
            || !self.contains.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputStream {
    Stdout,
    Stderr,
}

impl OutputStream {
    fn name(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }

    fn text(self, output: &CapturedOutput) -> &str {
        match self {
            Self::Stdout => &output.stdout,
            Self::Stderr => &output.stderr,
        }
    }

    fn parse(path: &Path, line_number: usize, value: &str) -> Self {
        let value = parse_string(path, line_number, value);
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

#[derive(Debug)]
struct JsonAssertion {
    path: String,
    equals: Option<JsonValue>,
    missing: bool,
}

#[derive(Debug)]
struct ResultValueAssertion {
    value_path: String,
    path: String,
    equals: Option<JsonValue>,
    missing: bool,
}

#[derive(Debug)]
struct FileAssertion {
    path: String,
    equals: String,
}

#[derive(Debug)]
struct DiagnosticExpectation {
    id: String,
    severity: Option<String>,
    kind: Option<String>,
    message: Option<String>,
    span: Option<SpanExpectation>,
}

#[derive(Debug)]
struct BinaryFixtureExpectation {
    name: String,
    schema: Option<String>,
    bytes: Option<BinaryFixtureBytes>,
    consumed: Option<usize>,
    error: Option<String>,
    byte_diagnostic: Option<BinaryFixtureByteDiagnostic>,
}

#[derive(Debug)]
struct BinaryFixtureBytes {
    hex: String,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct OutputChunkListExpectation {
    name: String,
    chunks: Option<Vec<BinaryFixtureBytes>>,
}

#[derive(Debug, Default)]
struct BinaryFixtureByteDiagnostic {
    diagnostic_id: Option<String>,
    byte_offset: Option<usize>,
    expected_count: Option<usize>,
    available_count: Option<usize>,
    readiness: Option<String>,
    field_path: Option<JsonValue>,
}

#[derive(Debug, Default)]
struct SpanExpectation {
    file: Option<String>,
    line: Option<i64>,
    column: Option<i64>,
}

#[derive(Debug, Default)]
struct Requirements {
    jdk: bool,
}

#[derive(Debug, Default)]
struct ToolSetup {
    java: Option<ToolAvailability>,
    git: Option<ToolAvailability>,
}

impl ToolSetup {
    fn needs_path(&self) -> bool {
        self.configured().next().is_some()
    }

    fn requires_jdk(&self) -> bool {
        self.configured().any(ToolConfig::requires_jdk)
    }

    fn configured(&self) -> impl Iterator<Item = ToolConfig> {
        [
            self.java
                .map(|availability| ToolName::Java.config(availability)),
            self.git
                .map(|availability| ToolName::Git.config(availability)),
        ]
        .into_iter()
        .flatten()
    }

    fn set(&mut self, name: ToolName, availability: ToolAvailability) {
        match name {
            ToolName::Java => self.java = Some(availability),
            ToolName::Git => self.git = Some(availability),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ToolConfig {
    name: ToolName,
    availability: ToolAvailability,
}

impl ToolConfig {
    fn requires_jdk(self) -> bool {
        self.name == ToolName::Java && self.availability == ToolAvailability::Real
    }

    fn setup(self, tool_path: &Path) {
        setup_tool(tool_path, self.name.as_str(), self.availability);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolName {
    Java,
    Git,
}

impl ToolName {
    fn as_str(self) -> &'static str {
        match self {
            Self::Java => "java",
            Self::Git => "git",
        }
    }

    fn config(self, availability: ToolAvailability) -> ToolConfig {
        ToolConfig {
            name: self,
            availability,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolAvailability {
    Missing,
    FakeSuccess,
    FakeGitRevParse,
    Real,
}

#[derive(Debug, Default)]
struct SkipRules {
    platforms: Vec<SkipPlatform>,
    reason: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkipPlatform {
    Unix,
    Windows,
    Macos,
    Linux,
}

impl SkipPlatform {
    fn matches(self) -> bool {
        match self {
            Self::Unix => cfg!(unix),
            Self::Windows => cfg!(windows),
            Self::Macos => cfg!(target_os = "macos"),
            Self::Linux => cfg!(target_os = "linux"),
        }
    }
}

#[derive(Clone, Copy)]
enum Section {
    Root,
    Stdout,
    Stderr,
    Help,
    JsonAssert(usize),
    ResultValueAssert(usize),
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

fn parse_manifest(path: &Path, text: &str) -> CaseManifest {
    let mut parser = ManifestParser::new(path);
    for (line_index, raw_line) in text.lines().enumerate() {
        parser.parse_line(line_index + 1, raw_line);
    }
    parser.finish()
}

struct ManifestParser<'a> {
    path: &'a Path,
    command: Option<Vec<String>>,
    stdin: Option<String>,
    exit: Option<i32>,
    repeat: usize,
    env: Vec<(String, String)>,
    stdout: StreamExpectation,
    stderr: StreamExpectation,
    help: Option<HelpExpectation>,
    json_assertions: Vec<JsonAssertion>,
    result_value_assertions: Vec<ResultValueAssertion>,
    file_assertions: Vec<FileAssertion>,
    diagnostics: Vec<DiagnosticExpectation>,
    manifest_error: Option<ManifestErrorExpectation>,
    binary_fixtures: Vec<BinaryFixtureExpectation>,
    output_chunk_lists: Vec<OutputChunkListExpectation>,
    tools: ToolSetup,
    requires: Requirements,
    skip: SkipRules,
    section: Section,
}

impl<'a> ManifestParser<'a> {
    fn new(path: &'a Path) -> Self {
        Self {
            path,
            command: None,
            stdin: None,
            exit: None,
            repeat: 1,
            env: Vec::new(),
            stdout: StreamExpectation::default(),
            stderr: StreamExpectation::default(),
            help: None,
            json_assertions: Vec::new(),
            result_value_assertions: Vec::new(),
            file_assertions: Vec::new(),
            diagnostics: Vec::new(),
            manifest_error: None,
            binary_fixtures: Vec::new(),
            output_chunk_lists: Vec::new(),
            tools: ToolSetup::default(),
            requires: Requirements::default(),
            skip: SkipRules::default(),
            section: Section::Root,
        }
    }

    fn parse_line(&mut self, line_number: usize, raw_line: &str) {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            return;
        }

        if line.starts_with('[') {
            self.parse_section_header(line, line_number);
            return;
        }

        let (key, value) = line.split_once('=').unwrap_or_else(|| {
            manifest_error(self.path, line_number, "expected `key = value`");
        });
        self.parse_section_key(line_number, key.trim(), value.trim());
    }

    fn parse_section_header(&mut self, line: &str, line_number: usize) {
        self.section = match line {
            "[stdout]" => Section::Stdout,
            "[stderr]" => Section::Stderr,
            "[help]" => self.parse_help_header(line_number),
            "[requires]" => Section::Requires,
            "[skip]" => Section::Skip,
            "[env]" => Section::Env,
            "[tools]" => Section::Tools,
            "[[json_assert]]" => self.parse_json_assert_header(),
            "[[result_value_assert]]" => self.parse_result_value_assert_header(),
            "[[file_assert]]" => self.parse_file_assert_header(),
            "[[diagnostics]]" => self.parse_diagnostic_header(),
            "[diagnostics.span]" => self.parse_diagnostic_span_header(line_number),
            "[manifest_error]" => self.parse_manifest_error_header(line_number),
            "[[binary_fixture]]" => self.parse_binary_fixture_header(),
            "[[output_chunk_list]]" => self.parse_output_chunk_list_header(),
            _ => manifest_error(self.path, line_number, format!("unknown section `{line}`")),
        };
    }

    fn parse_help_header(&mut self, line_number: usize) -> Section {
        if self.help.is_some() {
            manifest_error(self.path, line_number, "duplicate help section");
        }
        self.help = Some(HelpExpectation::default());
        Section::Help
    }

    fn parse_json_assert_header(&mut self) -> Section {
        self.json_assertions.push(JsonAssertion {
            path: String::new(),
            equals: None,
            missing: false,
        });
        Section::JsonAssert(self.json_assertions.len() - 1)
    }

    fn parse_file_assert_header(&mut self) -> Section {
        self.file_assertions.push(FileAssertion {
            path: String::new(),
            equals: String::new(),
        });
        Section::FileAssert(self.file_assertions.len() - 1)
    }

    fn parse_result_value_assert_header(&mut self) -> Section {
        self.result_value_assertions.push(ResultValueAssertion {
            value_path: String::new(),
            path: String::new(),
            equals: None,
            missing: false,
        });
        Section::ResultValueAssert(self.result_value_assertions.len() - 1)
    }

    fn parse_diagnostic_header(&mut self) -> Section {
        self.diagnostics.push(DiagnosticExpectation {
            id: String::new(),
            severity: None,
            kind: None,
            message: None,
            span: None,
        });
        Section::Diagnostic(self.diagnostics.len() - 1)
    }

    fn parse_diagnostic_span_header(&mut self, line_number: usize) -> Section {
        let Some(index) = self.diagnostics.len().checked_sub(1) else {
            manifest_error(
                self.path,
                line_number,
                "diagnostics.span needs a diagnostic",
            );
        };
        if self.diagnostics[index].span.is_none() {
            self.diagnostics[index].span = Some(SpanExpectation::default());
        }
        Section::DiagnosticSpan(index)
    }

    fn parse_manifest_error_header(&mut self, line_number: usize) -> Section {
        if self.manifest_error.is_some() {
            manifest_error(self.path, line_number, "duplicate manifest_error section");
        }
        self.manifest_error = Some(ManifestErrorExpectation::default());
        Section::ManifestError
    }

    fn parse_binary_fixture_header(&mut self) -> Section {
        self.binary_fixtures.push(BinaryFixtureExpectation {
            name: String::new(),
            schema: None,
            bytes: None,
            consumed: None,
            error: None,
            byte_diagnostic: None,
        });
        Section::BinaryFixture(self.binary_fixtures.len() - 1)
    }

    fn parse_output_chunk_list_header(&mut self) -> Section {
        self.output_chunk_lists.push(OutputChunkListExpectation {
            name: String::new(),
            chunks: None,
        });
        Section::OutputChunkList(self.output_chunk_lists.len() - 1)
    }

    fn parse_section_key(&mut self, line_number: usize, key: &str, value: &str) {
        match self.section {
            Section::Root => self.parse_root_key(line_number, key, value),
            Section::Stdout => {
                parse_stream_key(self.path, line_number, &mut self.stdout, key, value, true)
            }
            Section::Stderr => {
                parse_stream_key(self.path, line_number, &mut self.stderr, key, value, false)
            }
            Section::Help => parse_help_key(
                self.path,
                line_number,
                self.help.as_mut().expect("help section should exist"),
                key,
                value,
            ),
            Section::Requires => self.parse_requires_key(line_number, key, value),
            Section::Skip => self.parse_skip_key(line_number, key, value),
            Section::Env => self
                .env
                .push((key.to_string(), parse_string(self.path, line_number, value))),
            Section::Tools => self.parse_tools_key(line_number, key, value),
            Section::JsonAssert(index) => {
                self.parse_json_assert_key(index, line_number, key, value)
            }
            Section::ResultValueAssert(index) => {
                self.parse_result_value_assert_key(index, line_number, key, value)
            }
            Section::FileAssert(index) => {
                self.parse_file_assert_key(index, line_number, key, value)
            }
            Section::Diagnostic(index) => self.parse_diagnostic_key(index, line_number, key, value),
            Section::DiagnosticSpan(index) => {
                self.parse_diagnostic_span_key(index, line_number, key, value);
            }
            Section::ManifestError => self.parse_manifest_error_key(line_number, key, value),
            Section::BinaryFixture(index) => {
                self.parse_binary_fixture_key(index, line_number, key, value)
            }
            Section::OutputChunkList(index) => {
                self.parse_output_chunk_list_key(index, line_number, key, value)
            }
        }
    }

    fn parse_root_key(&mut self, line_number: usize, key: &str, value: &str) {
        match key {
            "command" => self.command = Some(parse_string_array(self.path, line_number, value)),
            "stdin" => self.stdin = Some(parse_string(self.path, line_number, value)),
            "exit" => self.exit = Some(parse_i32(self.path, line_number, value)),
            "repeat" => self.repeat = parse_positive_usize(self.path, line_number, value),
            _ => manifest_error(self.path, line_number, format!("unknown root key `{key}`")),
        }
    }

    fn parse_requires_key(&mut self, line_number: usize, key: &str, value: &str) {
        match key {
            "jdk" => self.requires.jdk = parse_bool(self.path, line_number, value),
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown requires key `{key}`"),
            ),
        }
    }

    fn parse_skip_key(&mut self, line_number: usize, key: &str, value: &str) {
        match key {
            "platforms" => {
                self.skip.platforms = parse_string_array(self.path, line_number, value)
                    .into_iter()
                    .map(|platform| parse_skip_platform(self.path, line_number, &platform))
                    .collect();
            }
            "reason" => self.skip.reason = Some(parse_string(self.path, line_number, value)),
            _ => manifest_error(self.path, line_number, format!("unknown skip key `{key}`")),
        }
    }

    fn parse_tools_key(&mut self, line_number: usize, key: &str, value: &str) {
        match key {
            "java" => {
                self.tools.set(
                    ToolName::Java,
                    parse_tool_availability(self.path, line_number, value),
                );
            }
            "git" => {
                self.tools.set(
                    ToolName::Git,
                    parse_tool_availability(self.path, line_number, value),
                );
            }
            _ => manifest_error(self.path, line_number, format!("unknown tools key `{key}`")),
        }
    }

    fn parse_json_assert_key(&mut self, index: usize, line_number: usize, key: &str, value: &str) {
        match key {
            "path" => {
                self.json_assertions[index].path = parse_string(self.path, line_number, value)
            }
            "equals" => {
                self.json_assertions[index].equals =
                    Some(parse_manifest_json_value(self.path, line_number, value))
            }
            "missing" => {
                self.json_assertions[index].missing = parse_bool(self.path, line_number, value)
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown json_assert key `{key}`"),
            ),
        }
    }

    fn parse_result_value_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &str,
    ) {
        match key {
            "value_path" => {
                self.result_value_assertions[index].value_path =
                    parse_string(self.path, line_number, value)
            }
            "path" => {
                self.result_value_assertions[index].path =
                    parse_string(self.path, line_number, value)
            }
            "equals" => {
                self.result_value_assertions[index].equals =
                    Some(parse_manifest_json_value(self.path, line_number, value))
            }
            "missing" => {
                self.result_value_assertions[index].missing =
                    parse_bool(self.path, line_number, value)
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown result_value_assert key `{key}`"),
            ),
        }
    }

    fn parse_file_assert_key(&mut self, index: usize, line_number: usize, key: &str, value: &str) {
        match key {
            "path" => {
                self.file_assertions[index].path = parse_string(self.path, line_number, value)
            }
            "equals" => {
                self.file_assertions[index].equals = parse_string(self.path, line_number, value)
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown file_assert key `{key}`"),
            ),
        }
    }

    fn parse_diagnostic_key(&mut self, index: usize, line_number: usize, key: &str, value: &str) {
        match key {
            "id" => self.diagnostics[index].id = parse_string(self.path, line_number, value),
            "severity" => {
                self.diagnostics[index].severity =
                    Some(parse_string(self.path, line_number, value));
            }
            "kind" => {
                self.diagnostics[index].kind = Some(parse_string(self.path, line_number, value))
            }
            "message" => {
                self.diagnostics[index].message = Some(parse_string(self.path, line_number, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown diagnostics key `{key}`"),
            ),
        }
    }

    fn parse_diagnostic_span_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &str,
    ) {
        let span = self.diagnostics[index]
            .span
            .as_mut()
            .expect("diagnostic span should exist");
        match key {
            "file" => span.file = Some(parse_string(self.path, line_number, value)),
            "line" => span.line = Some(parse_i64(self.path, line_number, value)),
            "column" => span.column = Some(parse_i64(self.path, line_number, value)),
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown diagnostics.span key `{key}`"),
            ),
        }
    }

    fn parse_manifest_error_key(&mut self, line_number: usize, key: &str, value: &str) {
        let expectation = self
            .manifest_error
            .as_mut()
            .expect("manifest_error section should exist");
        match key {
            "contains" => {
                expectation.contains = parse_string_array(self.path, line_number, value);
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown manifest_error key `{key}`"),
            ),
        }
    }

    fn parse_binary_fixture_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &str,
    ) {
        let fixture = &mut self.binary_fixtures[index];
        match key {
            "name" => fixture.name = parse_string(self.path, line_number, value),
            "schema" => fixture.schema = Some(parse_string(self.path, line_number, value)),
            "hex" => {
                fixture.bytes = Some(parse_binary_fixture_hex(self.path, line_number, value));
            }
            "consumed" => {
                fixture.consumed = Some(parse_nonnegative_usize(self.path, line_number, value));
            }
            "error" => fixture.error = Some(parse_string(self.path, line_number, value)),
            "diagnostic_id" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .diagnostic_id = Some(parse_string(self.path, line_number, value));
            }
            "byte_offset" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .byte_offset = Some(parse_nonnegative_usize(self.path, line_number, value));
            }
            "expected_count" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .expected_count = Some(parse_nonnegative_usize(self.path, line_number, value));
            }
            "available_count" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .available_count = Some(parse_nonnegative_usize(self.path, line_number, value));
            }
            "readiness" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .readiness = Some(parse_string(self.path, line_number, value));
            }
            "field_path" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .field_path = Some(parse_manifest_json_value(self.path, line_number, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown binary_fixture key `{key}`"),
            ),
        }
    }

    fn parse_output_chunk_list_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &str,
    ) {
        let chunks = &mut self.output_chunk_lists[index];
        match key {
            "name" => chunks.name = parse_string(self.path, line_number, value),
            "chunks" => {
                chunks.chunks = Some(parse_binary_fixture_hex_array(
                    self.path,
                    line_number,
                    value,
                ));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown output_chunk_list key `{key}`"),
            ),
        }
    }

    fn finish(self) -> CaseManifest {
        let manifest = CaseManifest {
            invocation: CaseInvocation {
                command: self
                    .command
                    .unwrap_or_else(|| manifest_error(self.path, 0, "missing `command`")),
                stdin: self.stdin,
                repeat: self.repeat,
                env: self.env,
            },
            expectations: CaseExpectations {
                exit: self
                    .exit
                    .unwrap_or_else(|| manifest_error(self.path, 0, "missing `exit`")),
                stdout: self.stdout,
                stderr: self.stderr,
                help: self.help,
                json_assertions: self.json_assertions,
                result_value_assertions: self.result_value_assertions,
                file_assertions: self.file_assertions,
                diagnostics: self.diagnostics,
                binary_fixtures: self.binary_fixtures,
                output_chunk_lists: self.output_chunk_lists,
            },
            manifest_error: self.manifest_error,
            tools: self.tools,
            requires: self.requires,
            skip: self.skip,
        };

        for (index, assertion) in manifest.expectations.json_assertions.iter().enumerate() {
            if assertion.path.is_empty() {
                manifest_error(
                    self.path,
                    0,
                    format!("json_assert {index} is missing `path`"),
                );
            }
            if assertion.missing == assertion.equals.is_some() {
                manifest_error(
                    self.path,
                    0,
                    format!(
                        "json_assert {index} needs exactly one of `equals` or `missing = true`"
                    ),
                );
            }
        }
        for (index, assertion) in manifest
            .expectations
            .result_value_assertions
            .iter()
            .enumerate()
        {
            if assertion.value_path.is_empty() {
                manifest_error(
                    self.path,
                    0,
                    format!("result_value_assert {index} is missing `value_path`"),
                );
            }
            if assertion.path.is_empty() {
                manifest_error(
                    self.path,
                    0,
                    format!("result_value_assert {index} is missing `path`"),
                );
            }
            if assertion.missing == assertion.equals.is_some() {
                manifest_error(
                    self.path,
                    0,
                    format!(
                        "result_value_assert {index} needs exactly one of `equals` or `missing = true`"
                    ),
                );
            }
        }
        if let Some(help) = &manifest.expectations.help
            && !help.has_assertion()
        {
            manifest_error(self.path, 0, "help section has no assertion");
        }
        for (index, assertion) in manifest.expectations.file_assertions.iter().enumerate() {
            if assertion.path.is_empty() {
                manifest_error(
                    self.path,
                    0,
                    format!("file_assert {index} is missing `path`"),
                );
            }
        }
        for (index, diagnostic) in manifest.expectations.diagnostics.iter().enumerate() {
            if diagnostic.id.is_empty() {
                manifest_error(self.path, 0, format!("diagnostics {index} is missing `id`"));
            }
        }
        if let Some(manifest_error_expectation) = &manifest.manifest_error
            && !manifest_error_expectation.has_assertion()
        {
            manifest_error(self.path, 0, "manifest_error section has no assertion");
        }
        for (index, fixture) in manifest.expectations.binary_fixtures.iter().enumerate() {
            if fixture.name.is_empty() {
                manifest_error(
                    self.path,
                    0,
                    format!("binary_fixture {index} is missing `name`"),
                );
            }
            match (&fixture.bytes, &fixture.error) {
                (Some(_), None) => {}
                (None, Some(_)) if fixture.consumed.is_none() => {}
                (Some(_), Some(_)) => manifest_error(
                    self.path,
                    0,
                    format!("binary_fixture {index} cannot specify both `hex` and `error`"),
                ),
                (None, Some(_)) => manifest_error(
                    self.path,
                    0,
                    format!("binary_fixture {index} with `error` cannot specify `consumed`"),
                ),
                (None, None) => manifest_error(
                    self.path,
                    0,
                    format!("binary_fixture {index} needs `hex` or `error`"),
                ),
            }
            if let (Some(bytes), Some(consumed)) = (&fixture.bytes, fixture.consumed)
                && consumed > bytes.bytes.len()
            {
                manifest_error(
                    self.path,
                    0,
                    format!("binary_fixture {index} `consumed` exceeds decoded byte count"),
                );
            }
            if let Some(byte_diagnostic) = &fixture.byte_diagnostic {
                if fixture.bytes.is_none() {
                    manifest_error(
                        self.path,
                        0,
                        format!("binary_fixture {index} byte diagnostic metadata needs `hex`"),
                    );
                }
                if byte_diagnostic.byte_offset.is_none() || byte_diagnostic.field_path.is_none() {
                    manifest_error(
                        self.path,
                        0,
                        format!("binary_fixture {index} has incomplete byte diagnostic metadata"),
                    );
                }
                validate_binary_fixture_field_path(
                    self.path,
                    index,
                    byte_diagnostic.field_path.as_ref(),
                );
                let has_count_metadata = byte_diagnostic.expected_count.is_some()
                    || byte_diagnostic.available_count.is_some()
                    || byte_diagnostic.readiness.is_some();
                if has_count_metadata
                    && (byte_diagnostic.expected_count.is_none()
                        || byte_diagnostic.available_count.is_none()
                        || byte_diagnostic.readiness.is_none())
                {
                    manifest_error(
                        self.path,
                        0,
                        format!("binary_fixture {index} has incomplete byte count metadata"),
                    );
                }
                if byte_diagnostic.diagnostic_id.is_none() && !has_count_metadata {
                    manifest_error(
                        self.path,
                        0,
                        format!("binary_fixture {index} needs `diagnostic_id` for field metadata"),
                    );
                }
            }
        }
        for (index, chunks) in manifest.expectations.output_chunk_lists.iter().enumerate() {
            if chunks.name.is_empty() {
                manifest_error(
                    self.path,
                    0,
                    format!("output_chunk_list {index} is missing `name`"),
                );
            }
            if chunks.chunks.is_none() {
                manifest_error(
                    self.path,
                    0,
                    format!("output_chunk_list {index} is missing `chunks`"),
                );
            }
        }

        manifest
    }
}

fn validate_binary_fixture_field_path(
    path: &Path,
    fixture_index: usize,
    field_path: Option<&JsonValue>,
) {
    let Some(JsonValue::Array(segments)) = field_path else {
        manifest_error(
            path,
            0,
            format!("binary_fixture {fixture_index} `field_path` must be a JSON array"),
        );
    };
    for (segment_index, segment) in segments.iter().enumerate() {
        let JsonValue::Object(_) = segment else {
            manifest_error(
                path,
                0,
                format!(
                    "binary_fixture {fixture_index} `field_path` segment {segment_index} must be an object"
                ),
            );
        };
        if segment
            .object_field("kind")
            .and_then(JsonValue::as_str)
            .is_none()
        {
            manifest_error(
                path,
                0,
                format!(
                    "binary_fixture {fixture_index} `field_path` segment {segment_index} is missing string `kind`"
                ),
            );
        }
        if segment
            .object_field("name")
            .and_then(JsonValue::as_str)
            .is_none()
        {
            manifest_error(
                path,
                0,
                format!(
                    "binary_fixture {fixture_index} `field_path` segment {segment_index} is missing string `name`"
                ),
            );
        }
    }
}

fn command_source_inputs(command: &[String]) -> Vec<PathBuf> {
    let Some(command_name) = command.first().map(String::as_str) else {
        return Vec::new();
    };
    match command_name {
        "run" => run_command_source_inputs(&command[1..]),
        "check" | "doc" | "fmt" | "test" => source_inputs_after_flags(&command[1..]),
        _ => Vec::new(),
    }
}

fn run_command_source_inputs(args: &[String]) -> Vec<PathBuf> {
    let mut saw_entry = false;
    let mut inputs = Vec::new();
    for arg in args {
        if arg == "--" {
            break;
        }
        if arg == "--json" {
            continue;
        }
        if !saw_entry {
            saw_entry = true;
            continue;
        }
        inputs.push(PathBuf::from(arg));
    }
    inputs
}

fn source_inputs_after_flags(args: &[String]) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    for arg in args {
        if arg == "--" {
            break;
        }
        if arg == "--json" {
            continue;
        }
        inputs.push(PathBuf::from(arg));
    }
    inputs
}

fn fixture_reference_module(project: &Project, first_input: Option<&PathBuf>) -> Option<String> {
    if let Some(first_input) = first_input {
        let source_path = if first_input.is_absolute() {
            first_input.clone()
        } else {
            project.root.join(first_input)
        };
        if source_path.is_file()
            && let Ok(source) = veln_source::SourceFile::read(&project.root, &source_path)
            && let Ok(module) = derive_source_module_path(&source)
        {
            return Some(module);
        }
    }
    project
        .files
        .first()
        .and_then(|source| derive_source_module_path(source).ok())
}

fn validate_binary_fixture_schema_references(
    path: &Path,
    module: &SurfaceModule,
    current_module: Option<&str>,
    fixtures: &[BinaryFixtureExpectation],
) {
    let mut errors = Vec::new();
    for (index, fixture) in fixtures.iter().enumerate() {
        let Some(schema) = &fixture.schema else {
            continue;
        };
        match resolve_fixture_schema_reference(module, schema, current_module) {
            FixtureSchemaResolution::Resolved { name } => {
                if let Some(error) =
                    validate_binary_fixture_schema_field_path(index, &name, fixture)
                {
                    errors.push(error);
                }
            }
            FixtureSchemaResolution::Private => errors.push(format!(
                "binary_fixture {index} schema reference `{schema}` is private"
            )),
            FixtureSchemaResolution::WrongKind(kind) => errors.push(format!(
                "binary_fixture {index} schema reference `{schema}` is a {kind}, not a schema"
            )),
            FixtureSchemaResolution::Unresolved => errors.push(format!(
                "unresolved binary_fixture {index} schema reference `{schema}`"
            )),
        }
    }
    if !errors.is_empty() {
        manifest_error(path, 0, errors.join("\n"));
    }
}

fn validate_binary_fixture_schema_field_path(
    fixture_index: usize,
    schema_name: &str,
    fixture: &BinaryFixtureExpectation,
) -> Option<String> {
    let field_path = fixture
        .byte_diagnostic
        .as_ref()
        .and_then(|diagnostic| diagnostic.field_path.as_ref())?;
    let segments = field_path.as_array()?;
    let first_schema = segments
        .first()
        .and_then(|segment| match segment.object_field("kind") {
            Some(kind) if kind.as_str() == Some("schema") => segment.object_field("name"),
            _ => None,
        })
        .and_then(JsonValue::as_str);
    if first_schema != Some(schema_name) {
        return Some(format!(
            "binary_fixture {fixture_index} `field_path` first segment must name schema `{schema_name}`"
        ));
    }
    None
}

enum FixtureSchemaResolution {
    Resolved { name: String },
    Private,
    WrongKind(&'static str),
    Unresolved,
}

fn resolve_fixture_schema_reference(
    module: &SurfaceModule,
    target: &str,
    current_module: Option<&str>,
) -> FixtureSchemaResolution {
    let segments = target.split("::").map(str::to_string).collect::<Vec<_>>();
    resolve_fixture_schema_segments(module, &segments, current_module, true, &mut Vec::new())
}

fn resolve_fixture_schema_segments(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> FixtureSchemaResolution {
    match segments {
        [name] => resolve_fixture_schema_in_module(
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
                return FixtureSchemaResolution::Unresolved;
            };
            resolve_fixture_schema_in_module(
                module,
                Some(&use_decl.name),
                name,
                false,
                visited_aliases,
            )
        }
        _ => FixtureSchemaResolution::Unresolved,
    }
}

fn resolve_fixture_schema_in_module(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> FixtureSchemaResolution {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return if allow_private_schema || schema.visibility == Visibility::Public {
            FixtureSchemaResolution::Resolved {
                name: schema.name.clone().expect("schema should have a name"),
            }
        } else {
            FixtureSchemaResolution::Private
        };
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    }) {
        return resolve_fixture_schema_alias_target(module, alias, visited_aliases);
    }
    fixture_schema_wrong_kind(module, module_name, name).map_or(
        FixtureSchemaResolution::Unresolved,
        FixtureSchemaResolution::WrongKind,
    )
}

fn resolve_fixture_schema_alias_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> FixtureSchemaResolution {
    let Some(name) = &alias.name else {
        return FixtureSchemaResolution::Unresolved;
    };
    let key = (alias.module_name.clone(), name.clone());
    if visited_aliases.contains(&key) {
        return FixtureSchemaResolution::Unresolved;
    }
    visited_aliases.push(key);
    let resolution = resolve_fixture_schema_segments(
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

fn fixture_schema_wrong_kind(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
) -> Option<&'static str> {
    if module.functions.iter().any(|function| {
        function.kind == FunctionKind::Function
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
            PublicAliasKind::Function => Some("function"),
            PublicAliasKind::Type => Some("type"),
            PublicAliasKind::Schema => None,
        };
    }
    None
}

fn parse_help_key(
    path: &Path,
    line_number: usize,
    help: &mut HelpExpectation,
    key: &str,
    value: &str,
) {
    match key {
        "stream" => help.stream = OutputStream::parse(path, line_number, value),
        "summary" => help.summary = Some(parse_string(path, line_number, value)),
        "usage" => help.usage = Some(parse_string(path, line_number, value)),
        "commands" => help.commands = parse_string_array(path, line_number, value),
        "arguments" => help.arguments = parse_string_array(path, line_number, value),
        "options" => help.options = parse_string_array(path, line_number, value),
        "contains" => help.contains = parse_string_array(path, line_number, value),
        _ => manifest_error(path, line_number, format!("unknown help key `{key}`")),
    }
}

fn parse_stream_key(
    path: &Path,
    line_number: usize,
    stream: &mut StreamExpectation,
    key: &str,
    value: &str,
    allow_json: bool,
) {
    match key {
        "format" => {
            let format = parse_string(path, line_number, value);
            stream.format = Some(match format.as_str() {
                "empty" => StreamFormat::Empty,
                "text" => StreamFormat::Text,
                "json" if allow_json => StreamFormat::Json,
                _ => manifest_error(
                    path,
                    line_number,
                    format!("unknown stream format `{format}`"),
                ),
            });
        }
        "contains" => stream.contains = parse_string_array(path, line_number, value),
        _ => manifest_error(path, line_number, format!("unknown stream key `{key}`")),
    }
}

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
        } else if ch == '#' {
            return &line[..index];
        }
    }
    line
}

fn parse_manifest_json_value(path: &Path, line_number: usize, value: &str) -> JsonValue {
    if value.starts_with('"') {
        JsonValue::String(parse_string(path, line_number, value))
    } else if value == "true" {
        JsonValue::Bool(true)
    } else if value == "false" {
        JsonValue::Bool(false)
    } else if value == "null" {
        JsonValue::Null
    } else if value.starts_with('[') || value.starts_with('{') {
        parse_json(value).unwrap_or_else(|error| {
            manifest_error(
                path,
                line_number,
                format!("invalid json assertion value: {error}"),
            )
        })
    } else {
        JsonValue::Number(parse_i64(path, line_number, value))
    }
}

fn parse_binary_fixture_hex(path: &Path, line_number: usize, value: &str) -> BinaryFixtureBytes {
    let hex = parse_string(path, line_number, value);
    let bytes = decode_lowercase_hex(path, line_number, &hex);
    BinaryFixtureBytes { hex, bytes }
}

fn parse_binary_fixture_hex_array(
    path: &Path,
    line_number: usize,
    value: &str,
) -> Vec<BinaryFixtureBytes> {
    parse_string_array(path, line_number, value)
        .into_iter()
        .map(|hex| {
            let bytes = decode_lowercase_hex(path, line_number, &hex);
            BinaryFixtureBytes { hex, bytes }
        })
        .collect()
}

fn decode_lowercase_hex(path: &Path, line_number: usize, hex: &str) -> Vec<u8> {
    let bytes = hex.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        manifest_error(
            path,
            line_number,
            "expected complete lowercase hex byte pairs",
        );
    }

    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = lowercase_hex_nibble(pair[0])
            .unwrap_or_else(|| manifest_error(path, line_number, "expected lowercase hex"));
        let low = lowercase_hex_nibble(pair[1])
            .unwrap_or_else(|| manifest_error(path, line_number, "expected lowercase hex"));
        decoded.push((high << 4) | low);
    }
    decoded
}

fn lowercase_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn parse_bool(path: &Path, line_number: usize, value: &str) -> bool {
    match value {
        "true" => true,
        "false" => false,
        _ => manifest_error(path, line_number, "expected bool"),
    }
}

fn parse_skip_platform(path: &Path, line_number: usize, value: &str) -> SkipPlatform {
    match value {
        "unix" => SkipPlatform::Unix,
        "windows" => SkipPlatform::Windows,
        "macos" => SkipPlatform::Macos,
        "linux" => SkipPlatform::Linux,
        _ => manifest_error(
            path,
            line_number,
            format!("unknown skip platform `{value}`"),
        ),
    }
}

fn parse_tool_availability(path: &Path, line_number: usize, value: &str) -> ToolAvailability {
    let value = parse_string(path, line_number, value);
    match value.as_str() {
        "missing" => ToolAvailability::Missing,
        "fake-success" => ToolAvailability::FakeSuccess,
        "fake-git-rev-parse" => ToolAvailability::FakeGitRevParse,
        "real" => ToolAvailability::Real,
        _ => manifest_error(
            path,
            line_number,
            format!("unknown tool availability `{value}`"),
        ),
    }
}

fn parse_string_array(path: &Path, line_number: usize, value: &str) -> Vec<String> {
    let Some(inner) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        manifest_error(path, line_number, "expected string array");
    };
    let inner = inner.trim();
    if inner.is_empty() {
        return Vec::new();
    }
    split_array_values(inner)
        .into_iter()
        .map(|item| parse_string(path, line_number, item.trim()))
        .collect()
}

fn split_array_values(value: &str) -> Vec<&str> {
    let mut values = Vec::new();
    let mut start = 0;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
        } else if ch == ',' {
            values.push(&value[start..index]);
            start = index + 1;
        }
    }
    values.push(&value[start..]);
    values
}

fn parse_string(path: &Path, line_number: usize, value: &str) -> String {
    let Some(inner) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        manifest_error(path, line_number, "expected string");
    };

    let mut parsed = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            parsed.push(ch);
            continue;
        }

        let Some(escaped) = chars.next() else {
            manifest_error(path, line_number, "unterminated string escape");
        };
        match escaped {
            '"' => parsed.push('"'),
            '\\' => parsed.push('\\'),
            'n' => parsed.push('\n'),
            'r' => parsed.push('\r'),
            't' => parsed.push('\t'),
            _ => manifest_error(
                path,
                line_number,
                format!("unsupported string escape `{escaped}`"),
            ),
        }
    }
    parsed
}

fn parse_i32(path: &Path, line_number: usize, value: &str) -> i32 {
    value
        .parse()
        .unwrap_or_else(|_| manifest_error(path, line_number, "expected i32"))
}

fn parse_i64(path: &Path, line_number: usize, value: &str) -> i64 {
    value
        .parse()
        .unwrap_or_else(|_| manifest_error(path, line_number, "expected integer"))
}

fn parse_positive_usize(path: &Path, line_number: usize, value: &str) -> usize {
    let parsed = value
        .parse()
        .unwrap_or_else(|_| manifest_error(path, line_number, "expected positive integer"));
    if parsed == 0 {
        manifest_error(path, line_number, "expected positive integer");
    }
    parsed
}

fn parse_nonnegative_usize(path: &Path, line_number: usize, value: &str) -> usize {
    let parsed = parse_i64(path, line_number, value);
    if parsed < 0 {
        manifest_error(path, line_number, "expected non-negative integer");
    }
    usize::try_from(parsed).unwrap_or_else(|_| {
        manifest_error(
            path,
            line_number,
            "expected non-negative integer within range",
        )
    })
}

fn manifest_error(path: &Path, line_number: usize, message: impl std::fmt::Display) -> ! {
    if line_number == 0 {
        panic!("{}: {message}", path.display());
    }
    panic!("{}:{line_number}: {message}", path.display());
}

fn stream_text(bytes: Vec<u8>, context: &CaseRunContext<'_>, stream: &str) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|error| panic!("{}: {stream} should be UTF-8: {error}", context.label()))
}

fn assert_stream(
    context: &CaseRunContext<'_>,
    name: &str,
    expectation: &StreamExpectation,
    actual: &str,
) {
    match expectation.format {
        Some(StreamFormat::Empty) => assert_eq!(
            actual,
            "",
            "{}: expected {name} to be empty, got:\n{actual}",
            context.label()
        ),
        Some(StreamFormat::Text) | Some(StreamFormat::Json) | None => {}
    }

    for fragment in &expectation.contains {
        assert_contains_fragment(context, name, actual, fragment);
    }
}

fn assert_help_section(
    context: &CaseRunContext<'_>,
    surface: &str,
    stream: &str,
    section: &str,
    fragments: &[String],
) {
    if fragments.is_empty() {
        return;
    }
    assert_contains_fragment(context, surface, stream, &format!("{section}:\n"));
    for fragment in fragments {
        assert_contains_fragment(context, surface, stream, fragment);
    }
}

fn assert_contains_fragment(
    context: &CaseRunContext<'_>,
    surface: &str,
    actual: &str,
    fragment: &str,
) {
    assert!(
        actual.contains(fragment),
        "{}: expected {surface} to contain `{fragment}`, got:\n{actual}",
        context.label()
    );
}

fn assert_binary_fixture(
    context: &CaseRunContext<'_>,
    stdout: &str,
    fixture: &BinaryFixtureExpectation,
) {
    let expected = expected_binary_fixture_line(fixture);
    assert!(
        stdout.lines().any(|line| line == expected),
        "{}: expected binary fixture line `{expected}`, got:\n{stdout}",
        context.label()
    );
}

fn assert_output_chunk_list(
    context: &CaseRunContext<'_>,
    stdout: &str,
    chunks: &OutputChunkListExpectation,
) {
    let expected = expected_output_chunk_list_lines(chunks);
    let actual = stdout.lines().collect::<Vec<_>>();
    let matches = actual.windows(expected.len()).any(|window| {
        window
            .iter()
            .zip(&expected)
            .all(|(actual, expected)| *actual == expected.as_str())
    });
    assert!(
        matches,
        "{}: expected output chunk list:\n{}\ngot:\n{stdout}",
        context.label(),
        expected.join("\n")
    );
}

fn expected_binary_fixture_line(fixture: &BinaryFixtureExpectation) -> String {
    if let Some(bytes) = &fixture.bytes {
        let consumed = fixture
            .consumed
            .map_or_else(|| "none".to_string(), |value| value.to_string());
        let mut line = format!(
            "fixture {} hex {} count {} consumed {}",
            fixture.name,
            bytes.hex,
            bytes.bytes.len(),
            consumed
        );
        if let Some(byte_diagnostic) = &fixture.byte_diagnostic {
            if let Some(diagnostic_id) = &byte_diagnostic.diagnostic_id {
                line.push_str(&format!(" diagnostic {diagnostic_id}"));
            }
            if let Some(byte_offset) = byte_diagnostic.byte_offset {
                line.push_str(&format!(" offset {byte_offset}"));
            }
            if let Some(expected_count) = byte_diagnostic.expected_count {
                line.push_str(&format!(" expected {expected_count}"));
            }
            if let Some(available_count) = byte_diagnostic.available_count {
                line.push_str(&format!(" available {available_count}"));
            }
            if let Some(readiness) = &byte_diagnostic.readiness {
                line.push_str(&format!(" readiness {readiness}"));
            }
            if let Some(field_path) = &byte_diagnostic.field_path {
                line.push_str(&format!(" field_path {}", field_path.to_compact_string()));
            }
        }
        return line;
    }

    format!(
        "fixture {} error {}",
        fixture.name,
        fixture
            .error
            .as_deref()
            .expect("binary fixture error should be present")
    )
}

fn expected_output_chunk_list_lines(chunks: &OutputChunkListExpectation) -> Vec<String> {
    let chunk_values = chunks
        .chunks
        .as_deref()
        .expect("output chunk list chunks should be present");
    let mut lines = vec![format!(
        "output_chunk_list {} count {}",
        chunks.name,
        chunk_values.len()
    )];
    for (index, chunk) in chunk_values.iter().enumerate() {
        lines.push(format!(
            "output_chunk {} index {} hex \"{}\" count {}",
            chunks.name,
            index,
            chunk.hex,
            chunk.bytes.len()
        ));
    }
    lines
}

fn jdk_is_available() -> bool {
    Command::new("java").arg("-version").output().is_ok()
        && Command::new("java")
            .arg("--list-modules")
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("jdk.compiler"))
}

#[test]
fn manifest_tools_parse_controlled_java_availability() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[tools]
java = "fake-success"
git = "fake-git-rev-parse"
"#,
    );

    assert!(manifest.tools.needs_path());
    assert!(!manifest.tools.requires_jdk());
    assert_eq!(manifest.tools.java, Some(ToolAvailability::FakeSuccess));
    assert_eq!(manifest.tools.git, Some(ToolAvailability::FakeGitRevParse));

    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[tools]
java = "real"
"#,
    );

    assert!(manifest.tools.needs_path());
    assert!(manifest.tools.requires_jdk());
    assert_eq!(manifest.tools.java, Some(ToolAvailability::Real));
}

#[test]
fn manifest_json_assertions_support_missing_paths() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[json_assert]]
path = "error.details.byte_diagnostic.byte_preview"
missing = true
"#,
    );

    let assertion = &manifest.expectations.json_assertions[0];
    assert_eq!(assertion.path, "error.details.byte_diagnostic.byte_preview");
    assert!(assertion.missing);
    assert!(assertion.equals.is_none());
}

#[test]
#[should_panic(expected = "json_assert 0 needs exactly one of `equals` or `missing = true`")]
fn manifest_json_assertions_reject_mixed_equals_and_missing() {
    parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[json_assert]]
path = "status"
equals = "failed"
missing = true
"#,
    );
}

#[test]
fn manifest_binary_fixtures_parse_named_bytes_and_errors() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[binary_fixture]]
name = "short-u24"
hex = "0001"
consumed = 2
byte_offset = 2
expected_count = 3
available_count = 2
readiness = "need_bytes"
field_path = []

[[binary_fixture]]
name = "invalid-frame-kind"
schema = "DemoPacket"
hex = "ff0001"
consumed = 1
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"DemoPacket"},{"kind":"field","name":"kind"}]

[[binary_fixture]]
name = "bad-separator"
error = "fixture.hex.invalid_character"
"#,
    );

    assert!(manifest.expectations.needs_stdout_json());
    let fixtures = &manifest.expectations.binary_fixtures;
    assert_eq!(fixtures.len(), 3);
    assert_eq!(fixtures[0].name, "short-u24");
    assert_eq!(fixtures[0].bytes.as_ref().unwrap().hex, "0001");
    assert_eq!(fixtures[0].bytes.as_ref().unwrap().bytes, [0, 1]);
    assert_eq!(fixtures[0].consumed, Some(2));
    assert_eq!(
        expected_binary_fixture_line(&fixtures[0]),
        "fixture short-u24 hex 0001 count 2 consumed 2 offset 2 expected 3 available 2 readiness need_bytes field_path []"
    );
    assert_eq!(fixtures[1].name, "invalid-frame-kind");
    assert_eq!(fixtures[1].schema.as_deref(), Some("DemoPacket"));
    assert_eq!(fixtures[1].bytes.as_ref().unwrap().hex, "ff0001");
    assert_eq!(fixtures[1].consumed, Some(1));
    assert_eq!(
        expected_binary_fixture_line(&fixtures[1]),
        "fixture invalid-frame-kind hex ff0001 count 3 consumed 1 diagnostic schema.invalid_field_value offset 0 field_path [{\"kind\":\"schema\",\"name\":\"DemoPacket\"},{\"kind\":\"field\",\"name\":\"kind\"}]"
    );
    assert_eq!(fixtures[2].name, "bad-separator");
    assert_eq!(
        fixtures[2].error.as_deref(),
        Some("fixture.hex.invalid_character")
    );
    assert_eq!(
        expected_binary_fixture_line(&fixtures[2]),
        "fixture bad-separator error fixture.hex.invalid_character"
    );
}

#[test]
fn binary_fixture_schema_references_resolve_from_command_sources() {
    let root = test_temp_root("fixture-schema-references");
    write_fixture_schema_sources(&root);
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln", "wire.veln", "facade.veln"]
exit = 0

[[binary_fixture]]
name = "local-private"
schema = "LocalPacket"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"LocalPacket"}]

[[binary_fixture]]
name = "imported-public"
schema = "wire::PublicPacket"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"PublicPacket"}]

[[binary_fixture]]
name = "imported-alias"
schema = "facade::AliasPacket"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"PublicPacket"}]
"#,
    );

    manifest.validate_fixture_schema_references(&root);
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn binary_fixture_schema_references_reject_wrong_targets() {
    assert_fixture_schema_error(
        "MissingPacket",
        Some(r#"[{"kind":"schema","name":"MissingPacket"}]"#),
        "unresolved binary_fixture 0 schema reference `MissingPacket`",
    );
    assert_fixture_schema_error(
        "PrivatePacket",
        Some(r#"[{"kind":"schema","name":"PrivatePacket"}]"#),
        "unresolved binary_fixture 0 schema reference `PrivatePacket`",
    );
    assert_fixture_schema_error(
        "wire::PrivatePacket",
        Some(r#"[{"kind":"schema","name":"PrivatePacket"}]"#),
        "binary_fixture 0 schema reference `wire::PrivatePacket` is private",
    );
    assert_fixture_schema_error(
        "wire::make_packet",
        Some(r#"[{"kind":"schema","name":"make_packet"}]"#),
        "binary_fixture 0 schema reference `wire::make_packet` is a function, not a schema",
    );
    assert_fixture_schema_error(
        "wire::PacketShape",
        Some(r#"[{"kind":"schema","name":"PacketShape"}]"#),
        "binary_fixture 0 schema reference `wire::PacketShape` is a type, not a schema",
    );
    assert_fixture_schema_error(
        "wire::PacketCodec",
        Some(r#"[{"kind":"schema","name":"PacketCodec"}]"#),
        "unresolved binary_fixture 0 schema reference `wire::PacketCodec`",
    );
    assert_fixture_schema_error(
        "wire::byte_decode_public_packet",
        Some(r#"[{"kind":"schema","name":"PublicPacket"}]"#),
        "unresolved binary_fixture 0 schema reference `wire::byte_decode_public_packet`",
    );
    assert_fixture_schema_error(
        "other::PublicPacket",
        Some(r#"[{"kind":"schema","name":"PublicPacket"}]"#),
        "unresolved binary_fixture 0 schema reference `other::PublicPacket`",
    );
    assert_fixture_schema_error(
        "wire::PublicPacket",
        Some(r#"[{"kind":"schema","name":"OtherPacket"}]"#),
        "binary_fixture 0 `field_path` first segment must name schema `PublicPacket`",
    );
}

#[test]
fn manifest_result_value_assertions_parse_paths() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.details.value"
path = "value.id"
equals = "codec.incomplete_input"

[[result_value_assert]]
value_path = "error.details.value"
path = "value.detail.preview"
missing = true
"#,
    );

    assert!(manifest.expectations.needs_stdout_json());
    let assertions = &manifest.expectations.result_value_assertions;
    assert_eq!(assertions.len(), 2);
    assert_eq!(assertions[0].value_path, "error.details.value");
    assert_eq!(assertions[0].path, "value.id");
    assert_eq!(
        assertions[0].equals,
        Some(JsonValue::String("codec.incomplete_input".to_string()))
    );
    assert!(assertions[1].missing);
}

#[test]
fn result_value_parser_exposes_runtime_diagnostic_shape() {
    let parsed = parse_result_value(
        "RuntimeDiagnostic(codec.incomplete_input, byte read requires 3 bytes but view has 2, RuntimeByteDiagnostic(ByteOffset(2), Cons(RuntimeDiagnosticFieldPathSegment(schema, Payload), Cons(RuntimeDiagnosticFieldPathSegment(field, body), Nil)), RuntimeByteCountFacts(ByteCount(3), ByteCount(2), need_bytes), RuntimeBytePreview(0001, ByteCount(2), ByteCount(2), false)))",
    )
    .expect("runtime diagnostic value should parse");

    assert_eq!(
        json_path(&parsed, "constructor"),
        Some(&JsonValue::String("Err".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.constructor"),
        Some(&JsonValue::String("RuntimeDiagnostic".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.field_path.1.name"),
        Some(&JsonValue::String("body".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.facts.expected_count.value"),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.preview.truncated"),
        Some(&JsonValue::Bool(false))
    );
}

#[test]
fn result_value_parser_exposes_runtime_value_diagnostic_shape() {
    let parsed = parse_result_value(
        "RuntimeDiagnostic(schema.encode_value_unrepresentable, encode value is unrepresentable, RuntimeValueDiagnostic(Cons(RuntimeDiagnosticFieldPathSegment(schema, RuntimeValuePacket), Cons(RuntimeDiagnosticFieldPathSegment(field, value), Nil)), value must be between 0 and 255))",
    )
    .expect("runtime value diagnostic should parse");

    assert_eq!(
        json_path(&parsed, "value.detail.constructor"),
        Some(&JsonValue::String("RuntimeValueDiagnostic".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.field_path.1.name"),
        Some(&JsonValue::String("value".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.reason"),
        Some(&JsonValue::String(
            "value must be between 0 and 255".to_string()
        ))
    );
}

#[test]
fn result_value_parser_exposes_hpack_fixture_runtime_diagnostics() {
    let fixture = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.malformed_raw_string_value, HPACK fixture malformed raw string value at byte offset 9, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(9, 5, 8, fixture HPACK raw string value, hpack_fixture, ByteChunk([Byte(8), Byte(3), Byte(50), Byte(31), Byte(48)]))))",
    )
    .expect("HPACK fixture runtime diagnostic value should parse");
    let dynamic_index = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.dynamic_index_out_of_range, HPACK dynamic index out of range at byte offset 27, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicIndexDiagnostic(27, 1, 190, 0, 0, fixture dynamic indexed header, hpack_fixture, ByteChunk([Byte(190)]))))",
    )
    .expect("HPACK dynamic-index runtime diagnostic value should parse");
    let dynamic_name = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.dynamic_name_continuation_out_of_range, HPACK dynamic-name continuation out of range at byte offset 98, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicNameDiagnostic(98, 8, 127, 3, 3, fixture dynamic-name continuation range, hpack_fixture, ByteChunk([Byte(127), Byte(2), Byte(5), Byte(80), Byte(65), Byte(84), Byte(67), Byte(72)]))))",
    )
    .expect("HPACK dynamic-name runtime diagnostic value should parse");
    let table_size = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.table_size_update_not_at_start, HPACK fixture table-size update after header field at byte offset 10, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureTableSizeUpdateDiagnostic(10, 2, 62, 30, 1, 1, hpack-fixture, fixture HPACK table-size update at header block start, hpack_fixture, ByteChunk([Byte(130), Byte(62)]))))",
    )
    .expect("HPACK table-size runtime diagnostic value should parse");
    let table_size_malformed = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.table_size_update_malformed, HPACK fixture malformed table-size update integer at byte offset 77, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(77, 2, 63, fixture HPACK malformed table-size update integer, hpack_fixture, ByteChunk([Byte(63), Byte(128)]))))",
    )
    .expect("HPACK table-size malformed runtime diagnostic value should parse");

    assert_eq!(
        json_path(&fixture, "value.detail.constructor"),
        Some(&JsonValue::String(
            "RuntimeHttp2HpackDiagnostic".to_string()
        ))
    );
    assert_eq!(
        json_path(&fixture, "value.detail.detail.expected_fixture"),
        Some(&JsonValue::String(
            "fixture HPACK raw string value".to_string()
        ))
    );
    assert_eq!(
        json_path(&fixture, "value.detail.detail.preview.bytes.2.value"),
        Some(&JsonValue::Number(50))
    );
    assert_eq!(
        json_path(
            &dynamic_index,
            "value.detail.detail.requested_dynamic_index"
        ),
        Some(&JsonValue::Number(0))
    );
    assert_eq!(
        json_path(&dynamic_index, "value.detail.detail.preview.bytes.0.value"),
        Some(&JsonValue::Number(190))
    );
    assert_eq!(
        json_path(&dynamic_name, "value.detail.detail.requested_dynamic_index"),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(
            &dynamic_name,
            "value.detail.detail.dynamic_table_entry_count"
        ),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(
            &table_size,
            "value.detail.detail.observed_header_table_size"
        ),
        Some(&JsonValue::Number(30))
    );
    assert_eq!(
        json_path(&table_size, "value.detail.detail.active_state"),
        Some(&JsonValue::String("hpack-fixture".to_string()))
    );
    assert_eq!(
        json_path(
            &table_size_malformed,
            "value.detail.detail.expected_fixture"
        ),
        Some(&JsonValue::String(
            "fixture HPACK malformed table-size update integer".to_string()
        ))
    );
    assert_eq!(
        json_path(
            &table_size_malformed,
            "value.detail.detail.preview.bytes.1.value"
        ),
        Some(&JsonValue::Number(128))
    );
}

#[test]
fn result_value_parser_exposes_http2_peer_limit_runtime_diagnostics() {
    let header_table = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.header_table_size_exceeded, HTTP/2 header table size exceeds receive maximum at byte offset 35, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(35, 289, 160, 9, 1, local_configuration, hpack_dynamic_table_size_update, ByteChunk([Byte(63), Byte(129), Byte(1)]))))",
    )
    .expect("header-table runtime diagnostic value should parse");
    let concurrent_streams = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.concurrent_streams_exceeded, HTTP/2 concurrent stream receive limit exceeded at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(9, 3, 2, 1, server, open-stream, local_configuration, peer_created_stream_receive_limit, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(4), Byte(0), Byte(0), Byte(0), Byte(3)]))))",
    )
    .expect("concurrent-stream runtime diagnostic value should parse");

    assert_eq!(
        json_path(&header_table, "value.detail.constructor"),
        Some(&JsonValue::String("RuntimeHttp2Diagnostic".to_string()))
    );
    assert_eq!(
        json_path(
            &header_table,
            "value.detail.detail.observed_header_table_size"
        ),
        Some(&JsonValue::Number(289))
    );
    assert_eq!(
        json_path(&header_table, "value.detail.detail.preview.bytes.1.value"),
        Some(&JsonValue::Number(129))
    );
    assert_eq!(
        json_path(
            &concurrent_streams,
            "value.detail.detail.attempted_concurrent_stream_count"
        ),
        Some(&JsonValue::Number(2))
    );
    assert_eq!(
        json_path(
            &concurrent_streams,
            "value.detail.detail.receive_limit_provenance"
        ),
        Some(&JsonValue::String("local_configuration".to_string()))
    );
    assert_eq!(
        json_path(
            &concurrent_streams,
            "value.detail.detail.preview.bytes.8.value"
        ),
        Some(&JsonValue::Number(3))
    );
}

#[test]
fn result_value_parser_exposes_http2_data_flow_content_length_diagnostics() {
    let data_padding = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_data_padding, HTTP/2 invalid DATA padding at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(9, 1, 2, 0, open-stream, rfc9113_data_padding, ByteChunk([Byte(2)]))))",
    )
    .expect("DATA padding runtime diagnostic value should parse");
    let flow_control = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.flow_control_window_exceeded, HTTP/2 flow-control window exceeded at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(0, 4, 3, 0, 1, open-stream, stream_receive_window, ByteChunk([Byte(1), Byte(2), Byte(3), Byte(4)]))))",
    )
    .expect("flow-control runtime diagnostic value should parse");
    let content_length = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.content_length_mismatch, HTTP/2 content-length body length mismatch at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(9, 0, 1, 5, 3, open-stream, rfc9113_content_length_body, ByteChunk([Byte(170), Byte(187), Byte(204)]))))",
    )
    .expect("content-length runtime diagnostic value should parse");

    assert_eq!(
        json_path(&data_padding, "value.detail.detail.pad_length"),
        Some(&JsonValue::Number(2))
    );
    assert_eq!(
        json_path(&data_padding, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
    assert_eq!(
        json_path(&flow_control, "value.detail.detail.allowed_window_credit"),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(&flow_control, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String("stream_receive_window".to_string()))
    );
    assert_eq!(
        json_path(
            &content_length,
            "value.detail.detail.expected_content_length"
        ),
        Some(&JsonValue::Number(5))
    );
    assert_eq!(
        json_path(&content_length, "value.detail.detail.observed_body_length"),
        Some(&JsonValue::Number(3))
    );
}

#[test]
fn result_value_parser_exposes_http2_header_list_runtime_diagnostics() {
    let request = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_request_header_list, HTTP/2 request header list is missing :method at byte offset 12, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(12, 9, 1, missing_required_pseudo_header, :method, headers, request-headers, rfc9113_request_pseudo_headers, ByteChunk([Byte(130), Byte(132), Byte(134)]))))",
    )
    .expect("request header-list runtime diagnostic value should parse");
    let response = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_response_header_list, HTTP/2 response header list is missing :status at byte offset 12, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(12, 9, 1, missing_required_pseudo_header, :status, server, response-headers, rfc9113_response_pseudo_headers, ByteChunk([Byte(136)]))))",
    )
    .expect("response header-list runtime diagnostic value should parse");

    assert_eq!(
        json_path(&request, "value.detail.detail.failed_header_fact"),
        Some(&JsonValue::String(
            "missing_required_pseudo_header".to_string()
        ))
    );
    assert_eq!(
        json_path(&request, "value.detail.detail.decoded_header_names"),
        Some(&JsonValue::String("headers".to_string()))
    );
    assert_eq!(
        json_path(&request, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
    assert_eq!(
        json_path(&response, "value.detail.detail.constructor"),
        Some(&JsonValue::String(
            "RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic".to_string()
        ))
    );
    assert_eq!(
        json_path(&response, "value.detail.detail.header_name"),
        Some(&JsonValue::String(":status".to_string()))
    );
    assert_eq!(
        json_path(&response, "value.detail.detail.preview.bytes.0.value"),
        Some(&JsonValue::Number(136))
    );
}

#[test]
fn result_value_parser_exposes_http2_preface_runtime_diagnostics() {
    let partial = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.partial_preface, HTTP/2 input ended with partial client connection preface at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPartialPrefaceDiagnostic(0, 12, 24, connection-preface, rfc9113_client_connection_preface, ByteChunk([Byte(80), Byte(82), Byte(73), Byte(32), Byte(42), Byte(32)]))))",
    )
    .expect("partial preface runtime diagnostic value should parse");
    let invalid = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_preface, HTTP/2 invalid client connection preface at byte offset 4, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(4, 42, 43, 4, 24, connection-preface, rfc9113_client_connection_preface, ByteChunk([Byte(80), Byte(82), Byte(73), Byte(32), Byte(43)]))))",
    )
    .expect("invalid preface runtime diagnostic value should parse");

    assert_eq!(
        json_path(&partial, "value.detail.detail.pending_count"),
        Some(&JsonValue::Number(12))
    );
    assert_eq!(
        json_path(&partial, "value.detail.detail.active_state"),
        Some(&JsonValue::String("connection-preface".to_string()))
    );
    assert_eq!(
        json_path(&invalid, "value.detail.detail.expected_byte"),
        Some(&JsonValue::Number(42))
    );
    assert_eq!(
        json_path(&invalid, "value.detail.detail.actual_byte"),
        Some(&JsonValue::Number(43))
    );
    assert_eq!(
        json_path(&invalid, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
}

#[test]
fn result_value_parser_exposes_http2_control_runtime_diagnostics() {
    let closed = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.closed_with_pending, HTTP/2 input ended with 4 pending byte(s) at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolClosedWithPendingDiagnostic(0, 4, none, 0, 0, 0, 0, none, ByteChunk([Byte(1), Byte(2), Byte(3), Byte(4)]))))",
    )
    .expect("closed-input runtime diagnostic value should parse");
    let continuation = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.continuation_expected, HTTP/2 expected CONTINUATION frame at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContinuationExpectedDiagnostic(9, 0, 1, 1, 1, 0, headers, 3, rfc9113_continuation_sequence, ByteChunk([Byte(0), Byte(0), Byte(0)]))))",
    )
    .expect("continuation runtime diagnostic value should parse");
    let frame_kind = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_frame_kind, HTTP/2 invalid frame kind at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(0, 0, 1, 1, idle-stream, idle_streams_require_headers, ByteChunk([Byte(0)]))))",
    )
    .expect("invalid frame-kind runtime diagnostic value should parse");
    let stream_id = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_stream_id, HTTP/2 invalid stream id at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(0, 1, 2, nonzero client-initiated stream id, server, stream-id-domain, server_receives_client_initiated_streams, ByteChunk([Byte(0)]))))",
    )
    .expect("invalid stream-id runtime diagnostic value should parse");

    assert_eq!(
        json_path(&closed, "value.detail.detail.pending_count"),
        Some(&JsonValue::Number(4))
    );
    assert_eq!(
        json_path(&closed, "value.detail.detail.active_continuation"),
        Some(&JsonValue::String("none".to_string()))
    );
    assert_eq!(
        json_path(&closed, "value.detail.detail.expected_stream_id"),
        Some(&JsonValue::Number(0))
    );
    assert_eq!(
        json_path(&closed, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String("none".to_string()))
    );
    assert_eq!(
        json_path(&continuation, "value.detail.detail.expected_stream_id"),
        Some(&JsonValue::Number(1))
    );
    assert_eq!(
        json_path(&continuation, "value.detail.detail.active_continuation"),
        Some(&JsonValue::String("headers".to_string()))
    );
    assert_eq!(
        json_path(
            &continuation,
            "value.detail.detail.accumulated_header_block_bytes"
        ),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(&continuation, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String(
            "rfc9113_continuation_sequence".to_string()
        ))
    );
    assert_eq!(
        json_path(&frame_kind, "value.detail.detail.expected_frame_kind"),
        Some(&JsonValue::Number(1))
    );
    assert_eq!(
        json_path(&frame_kind, "value.detail.detail.active_state"),
        Some(&JsonValue::String("idle-stream".to_string()))
    );
    assert_eq!(
        json_path(&stream_id, "value.detail.detail.required_stream_id_domain"),
        Some(&JsonValue::String(
            "nonzero client-initiated stream id".to_string()
        ))
    );
    assert_eq!(
        json_path(&stream_id, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String(
            "server_receives_client_initiated_streams".to_string()
        ))
    );
}

#[test]
fn result_value_parser_exposes_http2_limit_and_shutdown_runtime_diagnostics() {
    let payload_length = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_payload_length, HTTP/2 invalid payload length at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(9, 8, 0, 3, 4, connection-flow-control, rfc9113_window_update_payload_length, ByteChunk([Byte(1), Byte(2), Byte(3)]))))",
    )
    .expect("invalid payload-length runtime diagnostic value should parse");
    let settings_value = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.settings_value_out_of_range, HTTP/2 SETTINGS value outside accepted range at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitSettingsValueDiagnostic(9, 5, SETTINGS_MAX_FRAME_SIZE, 16383, 16384, 16777215, peer_settings, ByteChunk([Byte(0), Byte(5)]))))",
    )
    .expect("settings value runtime diagnostic value should parse");
    let window_update = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_window_update_increment, HTTP/2 invalid WINDOW_UPDATE increment at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(0, 0, 0, 1, 2147483647, connection-flow-control, window_update_increment_nonzero, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(0)]))))",
    )
    .expect("window-update runtime diagnostic value should parse");
    let priority = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_priority_dependency, HTTP/2 invalid PRIORITY dependency at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPriorityDependencyDiagnostic(0, 1, 1, stream-control, rfc9113_priority_dependency, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(15)]))))",
    )
    .expect("priority runtime diagnostic value should parse");
    let goaway = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.stream_after_goaway, HTTP/2 stream opened after graceful shutdown at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(9, 7, 5, graceful_shutdown, server, goaway_last_stream_id, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(4), Byte(0), Byte(0), Byte(0), Byte(7)]))))",
    )
    .expect("stream-after-GOAWAY runtime diagnostic value should parse");

    assert_eq!(
        json_path(
            &payload_length,
            "value.detail.detail.observed_payload_length"
        ),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(
            &payload_length,
            "value.detail.detail.expected_payload_length"
        ),
        Some(&JsonValue::Number(4))
    );
    assert_eq!(
        json_path(&settings_value, "value.detail.detail.setting_name"),
        Some(&JsonValue::String("SETTINGS_MAX_FRAME_SIZE".to_string()))
    );
    assert_eq!(
        json_path(&settings_value, "value.detail.detail.peer_limit_provenance"),
        Some(&JsonValue::String("peer_settings".to_string()))
    );
    assert_eq!(
        json_path(
            &window_update,
            "value.detail.detail.accepted_max_window_increment"
        ),
        Some(&JsonValue::Number(2147483647))
    );
    assert_eq!(
        json_path(&priority, "value.detail.detail.dependency_stream_id"),
        Some(&JsonValue::Number(1))
    );
    assert_eq!(
        json_path(&priority, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
    assert_eq!(
        json_path(&goaway, "value.detail.detail.shutdown_state"),
        Some(&JsonValue::String("graceful_shutdown".to_string()))
    );
    assert_eq!(
        json_path(&goaway, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String("goaway_last_stream_id".to_string()))
    );
    assert_eq!(
        json_path(&goaway, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
}

#[test]
fn manifest_output_chunk_lists_parse_ordered_hex_chunks() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
chunks = ["0001ff", "00040000000f000001"]

[[output_chunk_list]]
name = "empty-chunk"
chunks = [""]

[[output_chunk_list]]
name = "no-output"
chunks = []
"#,
    );

    assert!(manifest.expectations.needs_stdout_json());
    let chunk_lists = &manifest.expectations.output_chunk_lists;
    assert_eq!(chunk_lists.len(), 3);
    assert_eq!(chunk_lists[0].name, "protocol-output");
    assert_eq!(
        chunk_lists[0].chunks.as_ref().unwrap()[0].bytes,
        [0, 1, 255]
    );
    assert_eq!(
        expected_output_chunk_list_lines(&chunk_lists[0]),
        [
            "output_chunk_list protocol-output count 2",
            "output_chunk protocol-output index 0 hex \"0001ff\" count 3",
            "output_chunk protocol-output index 1 hex \"00040000000f000001\" count 9",
        ]
    );
    assert_eq!(
        expected_output_chunk_list_lines(&chunk_lists[1]),
        [
            "output_chunk_list empty-chunk count 1",
            "output_chunk empty-chunk index 0 hex \"\" count 0",
        ]
    );
    assert_eq!(
        expected_output_chunk_list_lines(&chunk_lists[2]),
        ["output_chunk_list no-output count 0"]
    );
}

#[test]
#[should_panic(expected = "expected lowercase hex")]
fn manifest_output_chunk_lists_reject_uppercase_hex() {
    let _manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
chunks = ["00FF"]
"#,
    );
}

#[test]
#[should_panic(expected = "expected complete lowercase hex byte pairs")]
fn manifest_output_chunk_lists_reject_odd_length_hex() {
    let _manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
chunks = ["001"]
"#,
    );
}

#[test]
fn missing_tool_setup_leaves_isolated_tool_path_empty() {
    let root = test_temp_root("missing-tool");
    setup_tool(&root, "java", ToolAvailability::Missing);

    let entries = fs::read_dir(&root)
        .expect("tool root should be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("tool entries should be readable");
    assert!(entries.is_empty());

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn fake_success_tool_setup_installs_success_launcher() {
    let root = test_temp_root("fake-tool");
    setup_tool(&root, "java", ToolAvailability::FakeSuccess);

    let output = Command::new(fake_tool_path(&root, "java"))
        .output()
        .expect("fake tool should run");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"");

    fs::remove_dir_all(root).expect("test root should be removed");
}

fn assert_fixture_schema_error(schema: &str, field_path: Option<&str>, expected: &str) {
    let root = test_temp_root("fixture-schema-error");
    write_fixture_schema_sources(&root);
    let field_path = field_path
        .map(|value| format!("field_path = {value}"))
        .unwrap_or_default();
    let manifest = parse_manifest(
        Path::new("case.toml"),
        &format!(
            r#"
command = ["run", "--json", "main", "main.veln", "wire.veln", "facade.veln"]
exit = 0

[[binary_fixture]]
name = "schema-reference"
schema = "{schema}"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
{field_path}
"#
        ),
    );
    let panic = std::panic::catch_unwind(|| manifest.validate_fixture_schema_references(&root))
        .expect_err("schema reference should be rejected");
    let message = panic_message(panic);
    assert!(
        message.contains(expected),
        "expected panic to contain `{expected}`, got `{message}`"
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

fn write_fixture_schema_sources(root: &Path) {
    fs::write(
        root.join("main.veln"),
        r#"
use wire
use facade

schema LocalPacket
	format binary

	length: UInt8
end
"#,
    )
    .expect("main source should be written");
    fs::write(
        root.join("wire.veln"),
        r#"
pub schema PublicPacket
	format binary

	length: UInt8
end

schema PrivatePacket
	format binary

	length: UInt8
end

pub fn make_packet() -> Int
	1
end

pub type PacketShape
	pub Packet(Int)
end

"#,
    )
    .expect("wire source should be written");
    fs::write(
        root.join("facade.veln"),
        r#"
use wire

pub schema AliasPacket = wire::PublicPacket
"#,
    )
    .expect("facade source should be written");
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&str>() {
        return message.to_string();
    }
    "non-string panic".to_string()
}

fn test_temp_root(name: &str) -> PathBuf {
    let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "veln-toolchain-harness-test-{name}-{}-{nanos}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("test root should be created");
    root
}

#[cfg(windows)]
fn fake_tool_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{name}.cmd"))
}

#[cfg(not(windows))]
fn fake_tool_path(root: &Path, name: &str) -> PathBuf {
    root.join(name)
}

fn assert_json_path(context: &CaseRunContext<'_>, json: &JsonValue, assertion: &JsonAssertion) {
    if assertion.missing {
        assert!(
            json_path(json, &assertion.path).is_none(),
            "{}: JSON path `{}` should be missing in {:?}",
            context.label(),
            assertion.path,
            json
        );
        return;
    }

    let actual = json_path(json, &assertion.path).unwrap_or_else(|| {
        panic!(
            "{}: JSON path `{}` was not found in {:?}",
            context.label(),
            assertion.path,
            json
        )
    });
    assert_eq!(
        actual,
        assertion
            .equals
            .as_ref()
            .expect("non-missing json assertion should have expected value"),
        "{}: JSON path `{}` mismatch",
        context.label(),
        assertion.path
    );
}

fn assert_result_value_path(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    assertion: &ResultValueAssertion,
) {
    let rendered = json_path(json, &assertion.value_path)
        .and_then(JsonValue::as_str)
        .unwrap_or_else(|| {
            panic!(
                "{}: result value source path `{}` was not found as a string in {:?}",
                context.label(),
                assertion.value_path,
                json
            )
        });
    let parsed = parse_result_value(rendered).unwrap_or_else(|error| {
        panic!(
            "{}: result value at `{}` could not be parsed: {error}\nvalue: {rendered}",
            context.label(),
            assertion.value_path
        )
    });

    if assertion.missing {
        assert!(
            json_path(&parsed, &assertion.path).is_none(),
            "{}: result value path `{}` should be missing in {:?}",
            context.label(),
            assertion.path,
            parsed
        );
        return;
    }

    let actual = json_path(&parsed, &assertion.path).unwrap_or_else(|| {
        panic!(
            "{}: result value path `{}` was not found in {:?}",
            context.label(),
            assertion.path,
            parsed
        )
    });
    assert_eq!(
        actual,
        assertion
            .equals
            .as_ref()
            .expect("non-missing result value assertion should have expected value"),
        "{}: result value path `{}` mismatch",
        context.label(),
        assertion.path
    );
}

fn assert_diagnostic(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    expected: &DiagnosticExpectation,
) {
    let diagnostics = json_path(json, "diagnostics")
        .and_then(JsonValue::as_array)
        .unwrap_or_else(|| panic!("{}: JSON diagnostics array missing", context.label()));

    let mut matches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic_field(diagnostic, "id") == Some(expected.id.as_str()))
        .filter(|diagnostic| {
            expected
                .message
                .as_deref()
                .is_none_or(|message| diagnostic_field(diagnostic, "message") == Some(message))
        })
        .filter(|diagnostic| {
            expected
                .span
                .as_ref()
                .and_then(|span| span.file.as_deref())
                .is_none_or(|file| {
                    json_path(diagnostic, "span.file") == Some(&JsonValue::String(file.to_string()))
                })
        })
        .filter(|diagnostic| {
            expected
                .span
                .as_ref()
                .and_then(|span| span.line)
                .is_none_or(|line| {
                    json_path(diagnostic, "span.start.line") == Some(&JsonValue::Number(line))
                })
        });

    let diagnostic = matches.next().unwrap_or_else(|| {
        panic!(
            "{}: diagnostic `{}` was not found in {:?}",
            context.label(),
            expected.id,
            diagnostics
        )
    });
    assert!(
        matches.next().is_none(),
        "{}: diagnostic `{}` matched more than one JSON diagnostic",
        context.label(),
        expected.id
    );

    assert_diagnostic_field(
        context,
        diagnostic,
        &expected.id,
        "severity",
        &expected.severity,
    );
    assert_diagnostic_field(context, diagnostic, &expected.id, "kind", &expected.kind);
    assert_diagnostic_field(
        context,
        diagnostic,
        &expected.id,
        "message",
        &expected.message,
    );
    if let Some(span) = &expected.span {
        if let Some(file) = &span.file {
            assert_json_equals(
                context,
                diagnostic,
                &expected.id,
                "span.file",
                &JsonValue::String(file.clone()),
            );
        }
        if let Some(line) = span.line {
            assert_json_equals(
                context,
                diagnostic,
                &expected.id,
                "span.start.line",
                &JsonValue::Number(line),
            );
        }
        if let Some(column) = span.column {
            assert_json_equals(
                context,
                diagnostic,
                &expected.id,
                "span.start.column",
                &JsonValue::Number(column),
            );
        }
    }
}

fn assert_diagnostic_field(
    context: &CaseRunContext<'_>,
    diagnostic: &JsonValue,
    id: &str,
    field: &str,
    expected: &Option<String>,
) {
    if let Some(expected) = expected {
        assert_json_equals(
            context,
            diagnostic,
            id,
            field,
            &JsonValue::String(expected.clone()),
        );
    }
}

fn assert_json_equals(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    id: &str,
    path: &str,
    expected: &JsonValue,
) {
    let actual = json_path(json, path).unwrap_or_else(|| {
        panic!(
            "{}: diagnostic `{id}` JSON path `{path}` missing in {:?}",
            context.label(),
            json
        )
    });
    assert_eq!(
        actual,
        expected,
        "{}: diagnostic `{id}` JSON path `{path}` mismatch",
        context.label()
    );
}

fn diagnostic_field<'a>(diagnostic: &'a JsonValue, field: &str) -> Option<&'a str> {
    json_path(diagnostic, field).and_then(JsonValue::as_str)
}

fn json_path<'a>(mut value: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    for segment in path.split('.') {
        value = if let Ok(index) = segment.parse::<usize>() {
            value.as_array()?.get(index)?
        } else {
            value.object_field(segment)?
        };
    }
    Some(value)
}

fn parse_result_value(rendered_value: &str) -> Result<JsonValue, String> {
    let trimmed = rendered_value.trim();
    if let Some(inner) = constructor_arg(trimmed, "Err") {
        return parse_veln_value(trimmed).or_else(|_| {
            Ok(result_value_object(
                "Err",
                vec![("value", parse_veln_value(inner)?)],
            ))
        });
    }
    Ok(result_value_object(
        "Err",
        vec![("value", parse_veln_value(trimmed)?)],
    ))
}

fn parse_veln_value(text: &str) -> Result<JsonValue, String> {
    let text = text.trim();
    if text == "Nil" {
        return Ok(JsonValue::Array(Vec::new()));
    }
    if text == "NoRuntimeBytePreview" {
        return Ok(result_value_object("NoRuntimeBytePreview", Vec::new()));
    }
    let Some((name, args)) = split_constructor_call(text) else {
        return Ok(parse_veln_atom(text));
    };

    match name {
        "Err" => {
            let args = expect_arity(name, args, 1)?;
            Ok(result_value_object(
                "Err",
                vec![("value", parse_veln_value(args[0])?)],
            ))
        }
        "RuntimeDiagnostic" => {
            let args = expect_arity(name, args, 3)?;
            Ok(result_value_object(
                "RuntimeDiagnostic",
                vec![
                    ("id", JsonValue::String(args[0].trim().to_string())),
                    ("message", JsonValue::String(args[1].trim().to_string())),
                    ("detail", parse_veln_value(args[2])?),
                ],
            ))
        }
        "RuntimeByteDiagnostic" => {
            let args = expect_arity(name, args, 4)?;
            Ok(result_value_object(
                "RuntimeByteDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("field_path", parse_veln_list(args[1])?),
                    ("facts", parse_veln_value(args[2])?),
                    ("preview", parse_veln_value(args[3])?),
                ],
            ))
        }
        "RuntimeValueDiagnostic" => {
            let args = expect_arity(name, args, 2)?;
            Ok(result_value_object(
                "RuntimeValueDiagnostic",
                vec![
                    ("field_path", parse_veln_list(args[0])?),
                    ("reason", JsonValue::String(args[1].trim().to_string())),
                ],
            ))
        }
        "RuntimeHttp2Diagnostic" | "RuntimeHttp2HpackDiagnostic" => {
            let args = expect_arity(name, args, 1)?;
            Ok(result_value_object(
                name,
                vec![("detail", parse_veln_value(args[0])?)],
            ))
        }
        "RuntimeHpackFixtureDiagnostic" => {
            let args = expect_arity(name, args, 6)?;
            Ok(result_value_object(
                "RuntimeHpackFixtureDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("observed_header_block_size", parse_veln_value(args[1])?),
                    ("observed_first_byte", parse_veln_value(args[2])?),
                    (
                        "expected_fixture",
                        JsonValue::String(args[3].trim().to_string()),
                    ),
                    (
                        "codec_module",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[5])?),
                ],
            ))
        }
        "RuntimeHpackFixtureDynamicIndexDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHpackFixtureDynamicIndexDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("observed_header_block_size", parse_veln_value(args[1])?),
                    ("observed_first_byte", parse_veln_value(args[2])?),
                    ("requested_dynamic_index", parse_veln_value(args[3])?),
                    ("dynamic_table_entry_count", parse_veln_value(args[4])?),
                    (
                        "expected_fixture",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "codec_module",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHpackFixtureDynamicNameDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHpackFixtureDynamicNameDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("observed_header_block_size", parse_veln_value(args[1])?),
                    ("observed_first_byte", parse_veln_value(args[2])?),
                    ("requested_dynamic_index", parse_veln_value(args[3])?),
                    ("dynamic_table_entry_count", parse_veln_value(args[4])?),
                    (
                        "expected_fixture",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "codec_module",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHpackFixtureTableSizeUpdateDiagnostic" => {
            let args = expect_arity(name, args, 10)?;
            Ok(result_value_object(
                "RuntimeHpackFixtureTableSizeUpdateDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("observed_header_block_size", parse_veln_value(args[1])?),
                    ("observed_first_byte", parse_veln_value(args[2])?),
                    ("observed_header_table_size", parse_veln_value(args[3])?),
                    ("frame_kind", parse_veln_value(args[4])?),
                    ("stream_id", parse_veln_value(args[5])?),
                    (
                        "active_state",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    (
                        "expected_fixture",
                        JsonValue::String(args[7].trim().to_string()),
                    ),
                    (
                        "codec_module",
                        JsonValue::String(args[8].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[9])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolPartialPrefaceDiagnostic" => {
            let args = expect_arity(name, args, 6)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolPartialPrefaceDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("pending_count", parse_veln_value(args[1])?),
                    ("expected_count", parse_veln_value(args[2])?),
                    (
                        "active_state",
                        JsonValue::String(args[3].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[5])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolInvalidPrefaceDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolInvalidPrefaceDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("expected_byte", parse_veln_value(args[1])?),
                    ("actual_byte", parse_veln_value(args[2])?),
                    ("matched_prefix_count", parse_veln_value(args[3])?),
                    ("expected_count", parse_veln_value(args[4])?),
                    (
                        "active_state",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolClosedWithPendingDiagnostic" => {
            let args = expect_arity(name, args, 9)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolClosedWithPendingDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("pending_count", parse_veln_value(args[1])?),
                    (
                        "active_continuation",
                        JsonValue::String(args[2].trim().to_string()),
                    ),
                    ("expected_stream_id", parse_veln_value(args[3])?),
                    ("started_frame_kind", parse_veln_value(args[4])?),
                    ("started_byte_offset", parse_veln_value(args[5])?),
                    ("accumulated_header_block_bytes", parse_veln_value(args[6])?),
                    (
                        "rule_provenance",
                        JsonValue::String(args[7].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[8])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolContinuationExpectedDiagnostic" => {
            let args = expect_arity(name, args, 10)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolContinuationExpectedDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("actual_frame_kind", parse_veln_value(args[1])?),
                    ("actual_stream_id", parse_veln_value(args[2])?),
                    ("expected_stream_id", parse_veln_value(args[3])?),
                    ("started_frame_kind", parse_veln_value(args[4])?),
                    ("started_byte_offset", parse_veln_value(args[5])?),
                    (
                        "active_continuation",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("accumulated_header_block_bytes", parse_veln_value(args[7])?),
                    (
                        "rule_provenance",
                        JsonValue::String(args[8].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[9])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolInvalidFrameKindDiagnostic" => {
            let args = expect_arity(name, args, 7)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolInvalidFrameKindDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("actual_frame_kind", parse_veln_value(args[1])?),
                    ("stream_id", parse_veln_value(args[2])?),
                    ("expected_frame_kind", parse_veln_value(args[3])?),
                    (
                        "active_state",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[6])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolInvalidStreamIdDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolInvalidStreamIdDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("frame_kind", parse_veln_value(args[1])?),
                    ("stream_id", parse_veln_value(args[2])?),
                    (
                        "required_stream_id_domain",
                        JsonValue::String(args[3].trim().to_string()),
                    ),
                    (
                        "endpoint_role",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    (
                        "active_state",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolPeerStreamIdNotIncreasingDiagnostic" => {
            let args = expect_arity(name, args, 7)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolPeerStreamIdNotIncreasingDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("stream_id", parse_veln_value(args[1])?),
                    ("previous_peer_stream_id", parse_veln_value(args[2])?),
                    (
                        "endpoint_role",
                        JsonValue::String(args[3].trim().to_string()),
                    ),
                    (
                        "active_state",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[6])?),
                ],
            ))
        }
        "RuntimeHttp2PeerLimitFrameSizeDiagnostic" => {
            let args = expect_arity(name, args, 7)?;
            Ok(result_value_object(
                "RuntimeHttp2PeerLimitFrameSizeDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("observed_payload_length", parse_veln_value(args[1])?),
                    ("allowed_max_frame_size", parse_veln_value(args[2])?),
                    ("frame_kind", parse_veln_value(args[3])?),
                    ("stream_id", parse_veln_value(args[4])?),
                    (
                        "receive_limit_provenance",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[6])?),
                ],
            ))
        }
        "RuntimeHttp2PeerLimitHeaderListSizeDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2PeerLimitHeaderListSizeDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("observed_header_list_size", parse_veln_value(args[1])?),
                    ("allowed_header_list_size", parse_veln_value(args[2])?),
                    ("frame_kind", parse_veln_value(args[3])?),
                    ("stream_id", parse_veln_value(args[4])?),
                    (
                        "receive_limit_provenance",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2PeerLimitSettingsValueDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2PeerLimitSettingsValueDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("setting_identifier", parse_veln_value(args[1])?),
                    (
                        "setting_name",
                        JsonValue::String(args[2].trim().to_string()),
                    ),
                    ("observed_value", parse_veln_value(args[3])?),
                    ("accepted_min_value", parse_veln_value(args[4])?),
                    ("accepted_max_value", parse_veln_value(args[5])?),
                    (
                        "peer_limit_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("frame_kind", parse_veln_value(args[1])?),
                    ("stream_id", parse_veln_value(args[2])?),
                    ("observed_payload_length", parse_veln_value(args[3])?),
                    ("expected_payload_length", parse_veln_value(args[4])?),
                    (
                        "active_state",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic" => {
            let args = expect_arity(name, args, 7)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("stream_id", parse_veln_value(args[1])?),
                    ("pad_length", parse_veln_value(args[2])?),
                    ("remaining_payload_length", parse_veln_value(args[3])?),
                    (
                        "active_state",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[6])?),
                ],
            ))
        }
        "RuntimeHttp2PeerLimitFlowControlWindowDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2PeerLimitFlowControlWindowDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("observed_payload_length", parse_veln_value(args[1])?),
                    ("allowed_window_credit", parse_veln_value(args[2])?),
                    ("frame_kind", parse_veln_value(args[3])?),
                    ("stream_id", parse_veln_value(args[4])?),
                    (
                        "active_state",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolContentLengthMismatchDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolContentLengthMismatchDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("frame_kind", parse_veln_value(args[1])?),
                    ("stream_id", parse_veln_value(args[2])?),
                    ("expected_content_length", parse_veln_value(args[3])?),
                    ("observed_body_length", parse_veln_value(args[4])?),
                    (
                        "active_state",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("observed_header_table_size", parse_veln_value(args[1])?),
                    ("allowed_header_table_size", parse_veln_value(args[2])?),
                    ("frame_kind", parse_veln_value(args[3])?),
                    ("stream_id", parse_veln_value(args[4])?),
                    (
                        "receive_limit_provenance",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic" => {
            let args = expect_arity(name, args, 9)?;
            Ok(result_value_object(
                "RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("stream_id", parse_veln_value(args[1])?),
                    (
                        "attempted_concurrent_stream_count",
                        parse_veln_value(args[2])?,
                    ),
                    (
                        "allowed_concurrent_stream_count",
                        parse_veln_value(args[3])?,
                    ),
                    (
                        "endpoint_role",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    (
                        "active_state",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "receive_limit_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[7].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[8])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic"
        | "RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic" => {
            let args = expect_arity(name, args, 9)?;
            Ok(result_value_object(
                name,
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("frame_kind", parse_veln_value(args[1])?),
                    ("stream_id", parse_veln_value(args[2])?),
                    (
                        "failed_header_fact",
                        JsonValue::String(args[3].trim().to_string()),
                    ),
                    ("header_name", JsonValue::String(args[4].trim().to_string())),
                    (
                        "decoded_header_names",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "active_state",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[7].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[8])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("stream_id", parse_veln_value(args[1])?),
                    ("observed_window_increment", parse_veln_value(args[2])?),
                    ("accepted_min_window_increment", parse_veln_value(args[3])?),
                    ("accepted_max_window_increment", parse_veln_value(args[4])?),
                    (
                        "active_state",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic" => {
            let args = expect_arity(name, args, 4)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    (
                        "active_state",
                        JsonValue::String(args[1].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[2].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[3])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolInitialPeerSettingsRequiredDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolInitialPeerSettingsRequiredDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("actual_frame_kind", parse_veln_value(args[1])?),
                    ("actual_flags", parse_veln_value(args[2])?),
                    ("stream_id", parse_veln_value(args[3])?),
                    (
                        "endpoint_role",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    (
                        "active_state",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolSettingsNotAllowedForEndpointDiagnostic" => {
            let args = expect_arity(name, args, 8)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolSettingsNotAllowedForEndpointDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("setting_identifier", parse_veln_value(args[1])?),
                    (
                        "setting_name",
                        JsonValue::String(args[2].trim().to_string()),
                    ),
                    (
                        "endpoint_role",
                        JsonValue::String(args[3].trim().to_string()),
                    ),
                    ("frame_kind", parse_veln_value(args[4])?),
                    (
                        "active_state",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[6].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[7])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolPriorityDependencyDiagnostic" => {
            let args = expect_arity(name, args, 6)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolPriorityDependencyDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("stream_id", parse_veln_value(args[1])?),
                    ("dependency_stream_id", parse_veln_value(args[2])?),
                    (
                        "active_state",
                        JsonValue::String(args[3].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[5])?),
                ],
            ))
        }
        "RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic" => {
            let args = expect_arity(name, args, 7)?;
            Ok(result_value_object(
                "RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic",
                vec![
                    ("byte_offset", parse_veln_value(args[0])?),
                    ("stream_id", parse_veln_value(args[1])?),
                    ("last_stream_id", parse_veln_value(args[2])?),
                    (
                        "shutdown_state",
                        JsonValue::String(args[3].trim().to_string()),
                    ),
                    (
                        "endpoint_role",
                        JsonValue::String(args[4].trim().to_string()),
                    ),
                    (
                        "rule_provenance",
                        JsonValue::String(args[5].trim().to_string()),
                    ),
                    ("preview", parse_veln_value(args[6])?),
                ],
            ))
        }
        "RuntimeDiagnosticFieldPathSegment" => {
            let args = expect_arity(name, args, 2)?;
            Ok(result_value_object(
                "RuntimeDiagnosticFieldPathSegment",
                vec![
                    ("kind", JsonValue::String(args[0].trim().to_string())),
                    ("name", JsonValue::String(args[1].trim().to_string())),
                ],
            ))
        }
        "RuntimeByteCountFacts" => {
            let args = expect_arity(name, args, 3)?;
            Ok(result_value_object(
                "RuntimeByteCountFacts",
                vec![
                    ("expected_count", parse_veln_value(args[0])?),
                    ("available_count", parse_veln_value(args[1])?),
                    ("readiness", JsonValue::String(args[2].trim().to_string())),
                ],
            ))
        }
        "RuntimeByteRangeFacts" => {
            let args = expect_arity(name, args, 2)?;
            Ok(result_value_object(
                "RuntimeByteRangeFacts",
                vec![
                    ("requested_count", parse_veln_value(args[0])?),
                    ("available_count", parse_veln_value(args[1])?),
                ],
            ))
        }
        "RuntimeByteFixedValueFacts" => {
            let args = expect_arity(name, args, 2)?;
            Ok(result_value_object(
                "RuntimeByteFixedValueFacts",
                vec![
                    ("expected_value", parse_veln_value(args[0])?),
                    ("actual_value", parse_veln_value(args[1])?),
                ],
            ))
        }
        "RuntimeByteReasonFacts" => {
            let args = expect_arity(name, args, 1)?;
            Ok(result_value_object(
                "RuntimeByteReasonFacts",
                vec![("reason", JsonValue::String(args[0].trim().to_string()))],
            ))
        }
        "RuntimeBytePreview" => {
            let args = expect_arity(name, args, 4)?;
            Ok(result_value_object(
                "RuntimeBytePreview",
                vec![
                    ("encoding", JsonValue::String("hex".to_string())),
                    ("data", JsonValue::String(args[0].trim().to_string())),
                    ("preview_byte_count", parse_veln_value(args[1])?),
                    ("total_byte_count", parse_veln_value(args[2])?),
                    ("truncated", parse_veln_value(args[3])?),
                ],
            ))
        }
        "ByteChunk" => {
            let args = expect_arity(name, args, 1)?;
            Ok(result_value_object(
                "ByteChunk",
                vec![("bytes", parse_veln_bracketed_list(args[0])?)],
            ))
        }
        "Byte" | "ByteOffset" | "ByteCount" => {
            let args = expect_arity(name, args, 1)?;
            Ok(result_value_object(
                name,
                vec![("value", parse_veln_nonnegative_integer(name, args[0])?)],
            ))
        }
        "Cons" => Ok(JsonValue::Array(parse_veln_list_items(text)?)),
        _ => Ok(result_value_object(
            name,
            vec![(
                "fields",
                JsonValue::Array(
                    args.into_iter()
                        .map(parse_veln_value)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            )],
        )),
    }
}

fn parse_veln_list(text: &str) -> Result<JsonValue, String> {
    Ok(JsonValue::Array(parse_veln_list_items(text)?))
}

fn parse_veln_list_items(text: &str) -> Result<Vec<JsonValue>, String> {
    let text = text.trim();
    if text == "Nil" {
        return Ok(Vec::new());
    }
    let Some((name, args)) = split_constructor_call(text) else {
        return Err(format!("expected list value, got `{text}`"));
    };
    if name != "Cons" {
        return Err(format!("expected `Cons` or `Nil`, got `{name}`"));
    }
    let args = expect_arity(name, args, 2)?;
    let mut values = vec![parse_veln_value(args[0])?];
    values.extend(parse_veln_list_items(args[1])?);
    Ok(values)
}

fn parse_veln_bracketed_list(text: &str) -> Result<JsonValue, String> {
    let text = text.trim();
    let inner = text
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("expected bracketed list, got `{text}`"))?;
    if inner.trim().is_empty() {
        return Ok(JsonValue::Array(Vec::new()));
    }
    Ok(JsonValue::Array(
        split_top_level_args(inner)
            .into_iter()
            .map(parse_veln_value)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn parse_veln_atom(text: &str) -> JsonValue {
    match text {
        "true" => JsonValue::Bool(true),
        "false" => JsonValue::Bool(false),
        _ => text
            .parse::<i64>()
            .map(JsonValue::Number)
            .unwrap_or_else(|_| JsonValue::String(text.to_string())),
    }
}

fn parse_veln_nonnegative_integer(name: &str, text: &str) -> Result<JsonValue, String> {
    let value = text
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("`{name}` expects an integer payload, got `{}`", text.trim()))?;
    Ok(JsonValue::Number(value))
}

fn split_constructor_call(text: &str) -> Option<(&str, Vec<&str>)> {
    let open = text.find('(')?;
    if !text.ends_with(')') {
        return None;
    }
    let name = text[..open].trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let inner = &text[open + 1..text.len() - 1];
    Some((name, split_top_level_args(inner)))
}

fn constructor_arg<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}(");
    text.strip_prefix(&prefix)?.strip_suffix(')')
}

fn split_top_level_args(text: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if paren_depth == 0 && bracket_depth == 0 => {
                args.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        args.push(tail);
    }
    args
}

fn expect_arity<'a>(
    name: &str,
    args: Vec<&'a str>,
    expected: usize,
) -> Result<Vec<&'a str>, String> {
    if args.len() == expected {
        Ok(args)
    } else {
        Err(format!(
            "`{name}` expects {expected} argument(s), got {}",
            args.len()
        ))
    }
}

fn result_value_object(constructor: &str, fields: Vec<(&str, JsonValue)>) -> JsonValue {
    let mut entries = vec![(
        "constructor".to_string(),
        JsonValue::String(constructor.to_string()),
    )];
    entries.extend(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );
    JsonValue::Object(entries)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn to_compact_string(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::String(value) => format!("\"{}\"", escape_json_string(value)),
            Self::Array(values) => {
                let values = values
                    .iter()
                    .map(JsonValue::to_compact_string)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("[{values}]")
            }
            Self::Object(entries) => {
                let entries = entries
                    .iter()
                    .map(|(key, value)| {
                        format!(
                            "\"{}\":{}",
                            escape_json_string(key),
                            value.to_compact_string()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{entries}}}")
            }
        }
    }

    fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn object_field(&self, name: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(entries) => entries
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            ch if ch.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn parse_json(text: &str) -> Result<JsonValue, String> {
    let mut parser = JsonParser { text, offset: 0 };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.offset == text.len() {
        Ok(value)
    } else {
        Err(format!(
            "unexpected trailing input at byte {}",
            parser.offset
        ))
    }
}

struct JsonParser<'a> {
    text: &'a str,
    offset: usize,
}

impl JsonParser<'_> {
    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'n') => {
                self.expect_literal("null")?;
                Ok(JsonValue::Null)
            }
            Some(b't') => {
                self.expect_literal("true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal("false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(byte) => Err(format!(
                "unexpected byte `{}` at byte {}",
                byte as char, self.offset
            )),
            None => Err("unexpected end of input".to_string()),
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.consume(b'[')?;
        let mut values = Vec::new();
        self.skip_ws();
        if self.consume_if(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_ws();
            if self.consume_if(b']') {
                break;
            }
            self.consume(b',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.consume(b'{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.consume_if(b'}') {
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.consume(b':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            if self.consume_if(b'}') {
                break;
            }
            self.consume(b',')?;
        }
        Ok(JsonValue::Object(entries))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.consume(b'"')?;
        let mut parsed = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '"' => return Ok(parsed),
                '\\' => parsed.push(self.parse_escape()?),
                ch if ch.is_control() => {
                    return Err(format!(
                        "control character in string at byte {}",
                        self.offset
                    ));
                }
                ch => parsed.push(ch),
            }
        }
        Err("unterminated string".to_string())
    }

    fn parse_escape(&mut self) -> Result<char, String> {
        let Some(ch) = self.next_char() else {
            return Err("unterminated escape".to_string());
        };
        match ch {
            '"' | '\\' | '/' => Ok(ch),
            'b' => Ok('\u{08}'),
            'f' => Ok('\u{0c}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'u' => self.parse_unicode_escape(),
            _ => Err(format!("unsupported escape `{ch}` at byte {}", self.offset)),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let start = self.offset;
        let end = start + 4;
        let Some(hex) = self.text.get(start..end) else {
            return Err("short unicode escape".to_string());
        };
        if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(format!("invalid unicode escape `{hex}`"));
        }
        self.offset = end;
        let codepoint = u32::from_str_radix(hex, 16).expect("hex was validated");
        char::from_u32(codepoint).ok_or_else(|| format!("invalid unicode codepoint `{hex}`"))
    }

    fn parse_number(&mut self) -> Result<i64, String> {
        let start = self.offset;
        self.consume_if(b'-');
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.offset += 1;
        }
        self.text[start..self.offset]
            .parse()
            .map_err(|_| format!("invalid integer at byte {start}"))
    }

    fn expect_literal(&mut self, literal: &str) -> Result<(), String> {
        if self.text[self.offset..].starts_with(literal) {
            self.offset += literal.len();
            Ok(())
        } else {
            Err(format!("expected `{literal}` at byte {}", self.offset))
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn consume(&mut self, expected: u8) -> Result<(), String> {
        if self.consume_if(expected) {
            Ok(())
        } else {
            Err(format!(
                "expected `{}` at byte {}",
                expected as char, self.offset
            ))
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.offset).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.text[self.offset..].chars().next()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }
}
