use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, ExitCode, ExitStatus, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use veln_ast::{Expr, ExprKind, Function, SurfaceModule, lower_surface_ast};
use veln_backend_jvm::generate_java_with_entry;
use veln_diagnostics::{
    Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity, ToolInfo,
    diagnostic_to_json,
};
use veln_project::Project;
use veln_sema::{analyze_surface_module, lower_checked_surface_module};
use veln_syntax::{ParseDiagnostic, format_tree, parse};

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(exit_code) => exit_code,
        Err(message) => {
            eprintln!("veln: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    let command = Command::parse(args)?;
    match command {
        Command::Check { json, inputs } => check(json, inputs),
        Command::Fmt { inputs } => fmt(inputs),
        Command::Run { entry, inputs } => run_entry(entry, inputs),
        Command::Test { json, targets } => test(json, targets),
        Command::Help => {
            print_help();
            Ok(ExitCode::SUCCESS)
        }
        Command::Version => {
            println!("veln {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn fmt(inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let mut formatted = Vec::new();
    let mut diagnostics = Vec::new();

    for source in &project.files {
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().cloned());
        formatted.push((
            source.path().as_str().to_string(),
            format_tree(&parsed.tree),
        ));
    }

    if !diagnostics.is_empty() {
        for diagnostic in &diagnostics {
            print_parse_diagnostic_human(diagnostic);
        }
        return Ok(ExitCode::from(1));
    }

    for (path, text) in formatted {
        let path = project.root.join(path);
        fs::write(path, text).map_err(|error| error.to_string())?;
    }

    Ok(ExitCode::SUCCESS)
}

fn test(json: bool, targets: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let explicit = !targets.is_empty();
    let project = Project::discover(root, &targets).map_err(|error| error.to_string())?;
    let (module, mut diagnostics) = load_surface_module(&project);
    let test_files = selected_test_files(&project, explicit);
    let mut cases = discover_test_cases(&module, &test_files);
    let mut suite_errors = Vec::new();

    if !has_error(&diagnostics) {
        diagnostics.extend(analyze_surface_module(&module));
    }

    if cases.is_empty() && !has_error(&diagnostics) {
        suite_errors.push(SuiteError {
            kind: "discovery".to_string(),
            message: "no zero-argument test functions were discovered".to_string(),
        });
    }

    if has_error(&diagnostics) {
        for case in &mut cases {
            case.status = TestCaseStatus::Blocked;
            case.reason = Some("static_gate".to_string());
        }
    } else if suite_errors.is_empty() {
        for case in &mut cases {
            run_test_case(&module, case)?;
        }
    }

    let report = TestReport {
        status: test_run_status(&cases, &diagnostics, &suite_errors),
        selection: TestSelection {
            mode_name: if explicit { "explicit" } else { "discovered" }.to_string(),
            targets: selection_targets(&project, &test_files),
            confidence: "complete".to_string(),
            reason: if explicit {
                "user_selected".to_string()
            } else {
                "pattern_discovery".to_string()
            },
        },
        diagnostics,
        suite_errors,
        cases,
    };

    if json {
        println!("{}", report.to_json());
    } else {
        print_test_human(&report)?;
    }

    Ok(if report.status == TestRunStatus::Passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn run_entry(entry: String, inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let (module, diagnostics) = load_surface_module(&project);

    if has_error(&diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), diagnostics))?;
        return Ok(ExitCode::from(1));
    }

    let diagnostics = analyze_surface_module(&module);
    if has_error(&diagnostics) {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), diagnostics))?;
        return Ok(ExitCode::from(1));
    }

    let Some(entry_function) = module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some(entry.as_str()))
    else {
        eprintln!("veln: run entry `{entry}` was not found");
        return Ok(ExitCode::from(1));
    };
    if !entry_function.params.is_empty() {
        eprintln!("veln: run entry `{entry}` must not declare parameters in this slice");
        return Ok(ExitCode::from(1));
    }

    let reachable_module = reachable_entry_module(&module, &entry);
    let lowered = lower_checked_surface_module(&reachable_module);
    let Some(ir) = lowered.ir else {
        print_human_stderr(&DiagnosticEnvelope::new(tool_info(), lowered.diagnostics))?;
        eprintln!("veln: run blocked: checked program is not executable");
        return Ok(ExitCode::from(1));
    };

    let java = generate_java_with_entry(&ir, &entry);
    let build_dir = create_build_dir("veln-run").map_err(|error| error.to_string())?;
    let result = compile_and_run_java(&build_dir, &java);
    let cleanup_result = fs::remove_dir_all(&build_dir);
    if let Err(error) = cleanup_result {
        eprintln!(
            "veln: warning: failed to remove build directory `{}`: {error}",
            build_dir.display()
        );
    }
    result
}

fn load_surface_module(project: &Project) -> (SurfaceModule, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut functions = Vec::new();

    for source in &project.files {
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if parsed.diagnostics.is_empty() {
            functions.extend(lower_surface_ast(&parsed.tree).functions);
        }
    }

    (SurfaceModule { functions }, diagnostics)
}

fn selected_test_files(project: &Project, explicit: bool) -> BTreeSet<String> {
    project
        .files
        .iter()
        .filter(|source| explicit || source.path().as_str().ends_with("_test.veln"))
        .map(|source| source.path().as_str().to_string())
        .collect()
}

fn discover_test_cases(module: &SurfaceModule, test_files: &BTreeSet<String>) -> Vec<TestCase> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.params.is_empty() && test_files.contains(function.span.file.as_str())
        })
        .enumerate()
        .map(|(index, function)| TestCase {
            id: format!("case-{}", index + 1),
            name: function
                .name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string()),
            kind: "test".to_string(),
            status: TestCaseStatus::Passed,
            source: TestCaseSource {
                file: function.span.file.as_str().to_string(),
                node_id: function.node_id.display("fn"),
                span: function.span.clone(),
            },
            reason: None,
            failure: None,
            events: Vec::new(),
            diagnostics: Vec::new(),
        })
        .collect()
}

fn run_test_case(module: &SurfaceModule, case: &mut TestCase) -> Result<(), String> {
    let reachable_module = reachable_entry_module(module, &case.name);
    let lowered = lower_checked_surface_module(&reachable_module);
    let Some(ir) = lowered.ir else {
        case.status = TestCaseStatus::Blocked;
        case.reason = Some("static_gate".to_string());
        case.diagnostics = lowered.diagnostics;
        return Ok(());
    };

    let java = generate_java_with_entry(&ir, &case.name);
    let build_dir = create_build_dir("veln-test").map_err(|error| error.to_string())?;
    let result = compile_and_run_java_capture(&build_dir, &java, "veln test");
    let cleanup_result = fs::remove_dir_all(&build_dir);
    if let Err(error) = cleanup_result {
        eprintln!(
            "veln: warning: failed to remove build directory `{}`: {error}",
            build_dir.display()
        );
    }
    let output = match result? {
        JavaRunResult::Ran(output) => output,
        JavaRunResult::ToolError(message) => {
            case.status = TestCaseStatus::Error;
            case.reason = Some("runner_error".to_string());
            case.failure = Some(TestFailure {
                kind: "runtime".to_string(),
                message,
            });
            return Ok(());
        }
    };

    case.events = stdio_events_from_output(&output, &case.source);
    if output.status.success() {
        case.status = TestCaseStatus::Passed;
    } else {
        case.status = TestCaseStatus::Failed;
        case.failure = Some(TestFailure {
            kind: "runtime".to_string(),
            message: format!("test process exited with status {}", output.status),
        });
    }
    Ok(())
}

fn stdio_events_from_output(output: &Output, source: &TestCaseSource) -> Vec<JsonValue> {
    let mut events = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        events.push(stdio_event(
            "stdout",
            stdout.as_ref(),
            events.len() + 1,
            source,
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        events.push(stdio_event(
            "stderr",
            stderr.as_ref(),
            events.len() + 1,
            source,
        ));
    }
    events
}

fn stdio_event(stream: &str, text: &str, sequence: usize, source: &TestCaseSource) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("stdio")),
        ("stream", JsonValue::string(stream)),
        ("operation", JsonValue::string("print")),
        ("text", JsonValue::string(text)),
        ("terminator", JsonValue::string("none")),
        ("sequence", JsonValue::Number(sequence as i64)),
        ("node_id", JsonValue::string(source.node_id.clone())),
        ("span", source_span_to_json(&source.span)),
    ])
}

fn reachable_entry_module(module: &SurfaceModule, entry: &str) -> SurfaceModule {
    let function_names = module
        .functions
        .iter()
        .filter_map(|function| function.name.as_deref())
        .collect::<Vec<_>>();
    let mut reachable = Vec::<String>::new();
    let mut stack = vec![entry.to_string()];

    while let Some(name) = stack.pop() {
        if reachable.iter().any(|known| known == &name) {
            continue;
        }
        reachable.push(name.clone());
        for function in module
            .functions
            .iter()
            .filter(|function| function.name.as_deref() == Some(name.as_str()))
        {
            for callee in direct_function_callees(function, &function_names) {
                if !reachable.iter().any(|known| known == &callee) {
                    stack.push(callee);
                }
            }
        }
    }

    SurfaceModule {
        functions: module
            .functions
            .iter()
            .filter(|function| {
                function
                    .name
                    .as_ref()
                    .is_some_and(|name| reachable.iter().any(|known| known == name))
            })
            .cloned()
            .collect(),
    }
}

fn direct_function_callees(function: &Function, function_names: &[&str]) -> Vec<String> {
    let mut callees = Vec::new();
    for line in &function.body {
        match &line.kind {
            veln_ast::BodyLineKind::Let { expr, .. } | veln_ast::BodyLineKind::Expr { expr } => {
                collect_function_callees(expr, function_names, &mut callees);
            }
        }
    }
    callees
}

fn collect_function_callees(expr: &Expr, function_names: &[&str], callees: &mut Vec<String>) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let ExprKind::NamePath(segments) = &callee.kind {
                if let Some(name) = segments.last() {
                    if function_names
                        .iter()
                        .any(|function_name| function_name == name)
                        && !callees.iter().any(|callee| callee == name)
                    {
                        callees.push(name.clone());
                    }
                }
            }
            collect_function_callees(callee, function_names, callees);
            for arg in args {
                collect_function_callees(arg, function_names, callees);
            }
        }
        ExprKind::Try(inner) => collect_function_callees(inner, function_names, callees),
        ExprKind::Record(fields) => {
            for field in fields {
                collect_function_callees(&field.expr, function_names, callees);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_function_callees(item, function_names, callees);
            }
        }
        ExprKind::Prefix { expr, .. } => collect_function_callees(expr, function_names, callees),
        ExprKind::Binary { left, right, .. } => {
            collect_function_callees(left, function_names, callees);
            collect_function_callees(right, function_names, callees);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn check(json: bool, inputs: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let mut diagnostics = Vec::new();

    for source in &project.files {
        let parsed = parse(source);
        let surface_ast = lower_surface_ast(&parsed.tree);
        let has_parse_diagnostics = !parsed.diagnostics.is_empty();
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if !has_parse_diagnostics {
            diagnostics.extend(analyze_surface_module(&surface_ast));
        }
    }

    let has_errors = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    let envelope = DiagnosticEnvelope::new(
        ToolInfo::new("veln", env!("CARGO_PKG_VERSION")),
        diagnostics,
    );

    if json {
        println!("{}", envelope.to_json());
    } else {
        print_human(&envelope);
    }

    Ok(if has_errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn compile_and_run_java(
    build_dir: &std::path::Path,
    java: &veln_backend_jvm::JavaProgram,
) -> Result<ExitCode, String> {
    let result = compile_and_run_java_capture(build_dir, java, "veln run")?;
    let output = match result {
        JavaRunResult::Ran(output) => output,
        JavaRunResult::ToolError(message) => {
            eprintln!("{message}");
            return Ok(ExitCode::from(1));
        }
    };
    forward_process_output(&output)?;
    Ok(exit_code_from_status(output.status))
}

enum JavaRunResult {
    Ran(Output),
    ToolError(String),
}

fn compile_and_run_java_capture(
    build_dir: &std::path::Path,
    java: &veln_backend_jvm::JavaProgram,
    command_name: &str,
) -> Result<JavaRunResult, String> {
    for source in &java.sources {
        fs::write(build_dir.join(&source.path), &source.contents)
            .map_err(|error| error.to_string())?;
    }

    let javac_output = ProcessCommand::new("javac")
        .args(java.sources.iter().map(|source| source.path.as_str()))
        .current_dir(build_dir)
        .output();
    let javac_output = match javac_output {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(JavaRunResult::ToolError(format!(
                "veln: `javac` was not found; install a JDK to use `{command_name}`"
            )));
        }
        Err(error) => return Err(error.to_string()),
    };
    if !javac_output.status.success() {
        return Ok(JavaRunResult::ToolError(format!(
            "veln: javac failed with status {}",
            javac_output.status
        )));
    }

    let java_output = ProcessCommand::new("java")
        .arg("-cp")
        .arg(build_dir)
        .arg("VelnEntry")
        .output();
    let java_output = match java_output {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(JavaRunResult::ToolError(format!(
                "veln: `java` was not found; install a JDK to use `{command_name}`"
            )));
        }
        Err(error) => return Err(error.to_string()),
    };
    Ok(JavaRunResult::Ran(java_output))
}

fn create_build_dir(prefix: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    for attempt in 0..100 {
        let candidate = if attempt == 0 {
            base.clone()
        } else {
            env::temp_dir().join(format!("{prefix}-{}-{nanos}-{attempt}", std::process::id()))
        };
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate build directory",
    ))
}

fn forward_process_output(output: &std::process::Output) -> Result<(), String> {
    io::stdout()
        .write_all(&output.stdout)
        .map_err(|error| error.to_string())?;
    io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn exit_code_from_status(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) if (0..=255).contains(&code) => ExitCode::from(code as u8),
        Some(_) | None => ExitCode::from(1),
    }
}

fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestRunStatus {
    Passed,
    Failed,
    Blocked,
    Error,
}

impl TestRunStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestCaseStatus {
    Passed,
    Failed,
    Blocked,
    Error,
}

impl TestCaseStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Error => "error",
        }
    }
}

struct TestSelection {
    mode_name: String,
    targets: Vec<String>,
    confidence: String,
    reason: String,
}

struct TestReport {
    status: TestRunStatus,
    selection: TestSelection,
    diagnostics: Vec<Diagnostic>,
    suite_errors: Vec<SuiteError>,
    cases: Vec<TestCase>,
}

impl TestReport {
    fn to_json(&self) -> String {
        JsonValue::object([
            ("schema_version", JsonValue::string("veln-test-json/v0")),
            ("command", JsonValue::string("test")),
            ("status", JsonValue::string(self.status.as_str())),
            ("selection", self.selection.to_json()),
            (
                "summary",
                test_summary_to_json(&self.cases, &self.suite_errors),
            ),
            (
                "diagnostics",
                JsonValue::array(self.diagnostics.iter().map(diagnostic_to_json)),
            ),
            (
                "suite_errors",
                JsonValue::array(self.suite_errors.iter().map(SuiteError::to_json)),
            ),
            (
                "cases",
                JsonValue::array(self.cases.iter().map(TestCase::to_json)),
            ),
        ])
        .to_json()
    }
}

impl TestSelection {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("mode", JsonValue::string(self.mode_name.clone())),
            (
                "targets",
                JsonValue::array(self.targets.iter().map(|target| JsonValue::string(target))),
            ),
            ("confidence", JsonValue::string(self.confidence.clone())),
            ("reason", JsonValue::string(self.reason.clone())),
        ])
    }
}

struct SuiteError {
    kind: String,
    message: String,
}

impl SuiteError {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string(self.kind.clone())),
            ("message", JsonValue::string(self.message.clone())),
        ])
    }
}

struct TestCase {
    id: String,
    name: String,
    kind: String,
    status: TestCaseStatus,
    source: TestCaseSource,
    reason: Option<String>,
    failure: Option<TestFailure>,
    events: Vec<JsonValue>,
    diagnostics: Vec<Diagnostic>,
}

impl TestCase {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("id", JsonValue::string(self.id.clone())),
            ("name", JsonValue::string(self.name.clone())),
            ("kind", JsonValue::string(self.kind.clone())),
            ("status", JsonValue::string(self.status.as_str())),
            (
                "source",
                JsonValue::object([
                    ("file", JsonValue::string(self.source.file.clone())),
                    ("node_id", JsonValue::string(self.source.node_id.clone())),
                    ("span", source_span_range_to_json(&self.source.span)),
                ]),
            ),
            (
                "reason",
                self.reason
                    .as_ref()
                    .map_or(JsonValue::Null, |reason| JsonValue::string(reason)),
            ),
            (
                "failure",
                self.failure
                    .as_ref()
                    .map_or(JsonValue::Null, TestFailure::to_json),
            ),
            ("events", JsonValue::Array(self.events.clone())),
            (
                "diagnostics",
                JsonValue::array(self.diagnostics.iter().map(diagnostic_to_json)),
            ),
        ])
    }
}

struct TestCaseSource {
    file: String,
    node_id: String,
    span: veln_source::SourceSpan,
}

struct TestFailure {
    kind: String,
    message: String,
}

impl TestFailure {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string(self.kind.clone())),
            ("message", JsonValue::string(self.message.clone())),
            ("expected", JsonValue::Null),
            ("actual", JsonValue::Null),
            ("span", JsonValue::Null),
            (
                "details",
                JsonValue::object(Vec::<(String, JsonValue)>::new()),
            ),
        ])
    }
}

fn test_run_status(
    cases: &[TestCase],
    diagnostics: &[Diagnostic],
    suite_errors: &[SuiteError],
) -> TestRunStatus {
    if cases
        .iter()
        .any(|case| case.status == TestCaseStatus::Error)
    {
        TestRunStatus::Error
    } else if !suite_errors.is_empty() && cases.is_empty() {
        TestRunStatus::Blocked
    } else if has_error(diagnostics)
        || cases
            .iter()
            .any(|case| case.status == TestCaseStatus::Blocked)
    {
        TestRunStatus::Blocked
    } else if cases
        .iter()
        .any(|case| case.status == TestCaseStatus::Failed)
    {
        TestRunStatus::Failed
    } else {
        TestRunStatus::Passed
    }
}

fn test_summary_to_json(cases: &[TestCase], suite_errors: &[SuiteError]) -> JsonValue {
    let count = |status| cases.iter().filter(|case| case.status == status).count() as i64;
    JsonValue::object([
        ("total", JsonValue::Number(cases.len() as i64)),
        ("passed", JsonValue::Number(count(TestCaseStatus::Passed))),
        ("failed", JsonValue::Number(count(TestCaseStatus::Failed))),
        ("skipped", JsonValue::Number(0)),
        ("todo", JsonValue::Number(0)),
        ("blocked", JsonValue::Number(count(TestCaseStatus::Blocked))),
        (
            "errors",
            JsonValue::Number(count(TestCaseStatus::Error) + suite_errors.len() as i64),
        ),
    ])
}

fn source_span_to_json(span: &veln_source::SourceSpan) -> JsonValue {
    JsonValue::object([
        ("file", JsonValue::string(span.file.as_str())),
        ("start", line_col_to_json(span.start)),
        ("end", line_col_to_json(span.end)),
    ])
}

fn source_span_range_to_json(span: &veln_source::SourceSpan) -> JsonValue {
    JsonValue::object([
        ("start", line_col_to_json(span.start)),
        ("end", line_col_to_json(span.end)),
    ])
}

fn line_col_to_json(line_col: veln_source::LineCol) -> JsonValue {
    JsonValue::object([
        ("line", JsonValue::Number(line_col.line as i64)),
        ("column", JsonValue::Number(line_col.column as i64)),
        ("offset", JsonValue::Number(line_col.offset as i64)),
    ])
}

fn selection_targets(project: &Project, test_files: &BTreeSet<String>) -> Vec<String> {
    project
        .files
        .iter()
        .filter_map(|source| {
            let path = source.path().as_str();
            test_files.contains(path).then(|| path.to_string())
        })
        .collect()
}

fn print_test_human(report: &TestReport) -> Result<(), String> {
    if !report.diagnostics.is_empty() {
        print_human_stderr(&DiagnosticEnvelope::new(
            tool_info(),
            report.diagnostics.clone(),
        ))?;
    }
    for suite_error in &report.suite_errors {
        eprintln!("veln: test {}: {}", suite_error.kind, suite_error.message);
    }
    for case in &report.cases {
        match case.status {
            TestCaseStatus::Passed => println!("ok {}", case.name),
            TestCaseStatus::Failed => println!("not ok {}", case.name),
            TestCaseStatus::Blocked => println!("blocked {}", case.name),
            TestCaseStatus::Error => println!("error {}", case.name),
        }
        for diagnostic in &case.diagnostics {
            print_human_stderr(&DiagnosticEnvelope::new(
                tool_info(),
                vec![diagnostic.clone()],
            ))?;
        }
        if let Some(failure) = &case.failure {
            eprintln!("veln: test `{}` failed: {}", case.name, failure.message);
        }
    }
    Ok(())
}

fn parse_diagnostic_to_envelope(diagnostic: &ParseDiagnostic) -> Diagnostic {
    Diagnostic::new(
        diagnostic.id,
        Severity::Error,
        DiagnosticKind::Parse,
        diagnostic.message.clone(),
        diagnostic.span.clone(),
        JsonValue::object([
            ("phase", JsonValue::string("parse")),
            ("node_id", JsonValue::Null),
            (
                "parser_context",
                JsonValue::string(diagnostic.parser_context),
            ),
            (
                "unexpected",
                JsonValue::object([
                    (
                        "kind",
                        JsonValue::string(diagnostic.unexpected.kind.clone()),
                    ),
                    (
                        "text",
                        JsonValue::string(diagnostic.unexpected.text.clone()),
                    ),
                ]),
            ),
            (
                "expected",
                JsonValue::array(
                    diagnostic
                        .expected
                        .iter()
                        .map(|expected| JsonValue::string(*expected)),
                ),
            ),
            (
                "recovery",
                JsonValue::object([
                    (
                        "strategy",
                        JsonValue::string(diagnostic.recovery.strategy.as_str()),
                    ),
                    (
                        "anchor",
                        diagnostic
                            .recovery
                            .anchor
                            .as_ref()
                            .map_or(JsonValue::Null, |anchor| JsonValue::string(anchor.clone())),
                    ),
                    (
                        "dropped_token_count",
                        JsonValue::Number(diagnostic.recovery.dropped_token_count as i64),
                    ),
                ]),
            ),
        ]),
    )
}

fn print_parse_diagnostic_human(diagnostic: &ParseDiagnostic) {
    if let Some(span) = &diagnostic.span {
        eprintln!(
            "{}:{}:{}: error[{}]: {}",
            span.file.as_str(),
            span.start.line,
            span.start.column,
            diagnostic.id,
            diagnostic.message
        );
    } else {
        eprintln!("error[{}]: {}", diagnostic.id, diagnostic.message);
    }
}

fn print_human(envelope: &DiagnosticEnvelope) {
    if envelope.diagnostics.is_empty() {
        println!("ok");
        return;
    }

    for diagnostic in &envelope.diagnostics {
        if let Some(span) = &diagnostic.span {
            println!(
                "{}:{}:{}: {}[{}]: {}",
                span.file.as_str(),
                span.start.line,
                span.start.column,
                diagnostic.severity.as_str(),
                diagnostic.id,
                diagnostic.message
            );
        } else {
            println!(
                "{}[{}]: {}",
                diagnostic.severity.as_str(),
                diagnostic.id,
                diagnostic.message
            );
        }
    }
}

fn print_human_stderr(envelope: &DiagnosticEnvelope) -> Result<(), String> {
    let mut stderr = io::stderr();
    for diagnostic in &envelope.diagnostics {
        if let Some(span) = &diagnostic.span {
            writeln!(
                stderr,
                "{}:{}:{}: {}[{}]: {}",
                span.file.as_str(),
                span.start.line,
                span.start.column,
                diagnostic.severity.as_str(),
                diagnostic.id,
                diagnostic.message
            )
            .map_err(|error| error.to_string())?;
        } else {
            writeln!(
                stderr,
                "{}[{}]: {}",
                diagnostic.severity.as_str(),
                diagnostic.id,
                diagnostic.message
            )
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn print_help() {
    println!("veln check [--json] [path ...]");
    println!("veln fmt [path ...]");
    println!("veln run <entry> [path ...]");
    println!("veln test [--json] [target ...]");
}

fn tool_info() -> ToolInfo {
    ToolInfo::new("veln", env!("CARGO_PKG_VERSION"))
}

enum Command {
    Check { json: bool, inputs: Vec<PathBuf> },
    Fmt { inputs: Vec<PathBuf> },
    Run { entry: String, inputs: Vec<PathBuf> },
    Test { json: bool, targets: Vec<PathBuf> },
    Help,
    Version,
}

impl Command {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        let Some(first) = args.first() else {
            return Ok(Self::Help);
        };
        match first.as_str() {
            "check" => parse_check(args.into_iter().skip(1)),
            "fmt" => parse_fmt(args.into_iter().skip(1)),
            "run" => parse_run(args.into_iter().skip(1)),
            "test" => parse_test(args.into_iter().skip(1)),
            "--help" | "-h" | "help" => Ok(Self::Help),
            "--version" | "-V" | "version" => Ok(Self::Version),
            command => Err(format!("unknown command `{command}`")),
        }
    }
}

fn parse_check(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut json = false;
    let mut inputs = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "--help" | "-h" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("unknown check flag `{flag}`")),
            path => inputs.push(PathBuf::from(path)),
        }
    }
    Ok(Command::Check { json, inputs })
}

fn parse_fmt(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut inputs = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("unknown fmt flag `{flag}`")),
            path => inputs.push(PathBuf::from(path)),
        }
    }
    Ok(Command::Fmt { inputs })
}

fn parse_run(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut entry = None;
    let mut inputs = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("unknown run flag `{flag}`")),
            value if entry.is_none() => entry = Some(value.to_string()),
            path => inputs.push(PathBuf::from(path)),
        }
    }
    let Some(entry) = entry else {
        return Err("run requires an entry function name".to_string());
    };
    Ok(Command::Run { entry, inputs })
}

fn parse_test(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut json = false;
    let mut targets = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "--help" | "-h" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("unknown test flag `{flag}`")),
            path => targets.push(PathBuf::from(path)),
        }
    }
    Ok(Command::Test { json, targets })
}
