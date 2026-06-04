use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn toolchain_cases_pass() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cases_roots = [
        manifest_dir.join("tests/toolchain_cases"),
        manifest_dir.join("../../examples/specification"),
    ];
    let cases = discover_cases(&cases_roots);
    assert!(!cases.is_empty(), "expected at least one toolchain case");

    for case_dir in cases {
        run_case(&case_dir);
    }
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

fn discover_cases(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut cases = Vec::new();
    for root in roots {
        collect_cases(root, &mut cases);
    }
    cases.sort();
    cases
}

fn collect_cases(dir: &Path, cases: &mut Vec<PathBuf>) {
    if dir.join("case.toml").is_file() {
        cases.push(dir.to_path_buf());
        return;
    }

    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: failed to read cases: {error}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("{}: failed to read case entry: {error}", dir.display())
        });
        let path = entry.path();
        if path.is_dir() {
            collect_cases(&path, cases);
        }
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
    file_assertions: Vec<FileAssertion>,
    diagnostics: Vec<DiagnosticExpectation>,
}

#[derive(Debug)]
struct CaseManifest {
    invocation: CaseInvocation,
    expectations: CaseExpectations,
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
            for diagnostic in &self.diagnostics {
                assert_diagnostic(context, json, diagnostic);
            }
        }
    }

    fn needs_stdout_json(&self) -> bool {
        self.stdout.format == Some(StreamFormat::Json)
            || !self.json_assertions.is_empty()
            || !self.diagnostics.is_empty()
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
    equals: JsonValue,
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
    FileAssert(usize),
    Diagnostic(usize),
    DiagnosticSpan(usize),
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
    file_assertions: Vec<FileAssertion>,
    diagnostics: Vec<DiagnosticExpectation>,
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
            file_assertions: Vec::new(),
            diagnostics: Vec::new(),
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
            "[[file_assert]]" => self.parse_file_assert_header(),
            "[[diagnostics]]" => self.parse_diagnostic_header(),
            "[diagnostics.span]" => self.parse_diagnostic_span_header(line_number),
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
            equals: JsonValue::Null,
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
            Section::FileAssert(index) => {
                self.parse_file_assert_key(index, line_number, key, value)
            }
            Section::Diagnostic(index) => self.parse_diagnostic_key(index, line_number, key, value),
            Section::DiagnosticSpan(index) => {
                self.parse_diagnostic_span_key(index, line_number, key, value);
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
                    parse_manifest_json_value(self.path, line_number, value)
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown json_assert key `{key}`"),
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
                file_assertions: self.file_assertions,
                diagnostics: self.diagnostics,
            },
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

        manifest
    }
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
    } else {
        JsonValue::Number(parse_i64(path, line_number, value))
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
        &assertion.equals,
        "{}: JSON path `{}` mismatch",
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
