use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};

mod dependency_graph;
mod output;

const DEFAULT_THRESHOLD: f64 = 30.0;
const DEFAULT_FILE_LINE_THRESHOLD: usize = 700;
const DEFAULT_DEPENDENCY_HOTSPOTS: usize = 10;
const DEFAULT_DEPENDENCY_CYCLE_LIMIT: usize = 5;
const DEFAULT_MAX_FINDINGS: usize = 50;

fn main() {
    if help_requested(env::args().skip(1)) {
        println!("{}", usage());
        return;
    }

    let config = match Config::parse(env::args().skip(1)) {
        Ok(config) => config,
        Err(message) => exit_with_message(2, message),
    };

    let report = match collect_report(&config) {
        Ok(report) => report,
        Err(message) => exit_with_message(1, message),
    };
    match config.format {
        OutputFormat::Human => print!("{}", output::render_human(&report, &config)),
        OutputFormat::Json => println!("{}", output::render_json(&report, &config)),
    }
}

fn help_requested(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().any(|arg| arg == "--help" || arg == "-h")
}

fn exit_with_message(code: i32, message: String) -> ! {
    eprintln!("{message}");
    std::process::exit(code);
}

#[derive(Debug)]
struct Config {
    dependency_cycle_limit: usize,
    dependency_hotspots: usize,
    dependency_graph: bool,
    file_line_threshold: usize,
    format: OutputFormat,
    max_findings: usize,
    roots: Vec<PathBuf>,
    threshold: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum OutputFormat {
    #[default]
    Human,
    Json,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "human" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            _ => Err(format!("--format must be `human` or `json`, got {value:?}")),
        }
    }
}

impl Config {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut builder = ConfigBuilder::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            builder.parse_arg(arg, &mut args)?;
        }

        Ok(builder.finish())
    }
}

#[derive(Debug)]
struct ConfigBuilder {
    dependency_cycle_limit: usize,
    dependency_hotspots: usize,
    dependency_graph: bool,
    file_line_threshold: usize,
    format: OutputFormat,
    max_findings: usize,
    roots: Vec<PathBuf>,
    threshold: f64,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            dependency_cycle_limit: DEFAULT_DEPENDENCY_CYCLE_LIMIT,
            dependency_hotspots: DEFAULT_DEPENDENCY_HOTSPOTS,
            dependency_graph: false,
            file_line_threshold: DEFAULT_FILE_LINE_THRESHOLD,
            format: OutputFormat::Human,
            max_findings: DEFAULT_MAX_FINDINGS,
            roots: Vec::new(),
            threshold: DEFAULT_THRESHOLD,
        }
    }
}

impl ConfigBuilder {
    fn parse_arg(
        &mut self,
        arg: String,
        args: &mut impl Iterator<Item = String>,
    ) -> Result<(), String> {
        match arg.as_str() {
            "--dependency-graph" => self.dependency_graph = true,
            "--dependency-cycle-limit" => {
                self.dependency_cycle_limit = parse_next_usize(args, "--dependency-cycle-limit")?;
            }
            "--dependency-hotspots" => {
                self.dependency_hotspots = parse_next_usize(args, "--dependency-hotspots")?;
            }
            "--file-line-threshold" => {
                self.file_line_threshold = parse_next_usize(args, "--file-line-threshold")?;
            }
            "--format" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--format requires a value".to_string())?;
                self.format = OutputFormat::parse(&value)?;
            }
            "--max-findings" => {
                self.max_findings = parse_next_usize(args, "--max-findings")?;
            }
            "--threshold" => {
                self.threshold = parse_next_threshold(args)?;
            }
            _ if arg.starts_with('-') => return Err(format!("unknown option {arg:?}")),
            _ => self.roots.push(PathBuf::from(arg)),
        }
        Ok(())
    }

    fn finish(mut self) -> Config {
        if self.roots.is_empty() {
            self.roots.push(PathBuf::from("crates"));
        }

        Config {
            dependency_cycle_limit: self.dependency_cycle_limit,
            dependency_hotspots: self.dependency_hotspots,
            dependency_graph: self.dependency_graph,
            file_line_threshold: self.file_line_threshold,
            format: self.format,
            max_findings: self.max_findings,
            roots: self.roots,
            threshold: self.threshold,
        }
    }
}

fn parse_next_usize(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<usize, String> {
    let value = args
        .next()
        .ok_or_else(|| format!("{option} requires a value"))?;
    parse_positive_usize(option, &value)
}

fn parse_next_threshold(args: &mut impl Iterator<Item = String>) -> Result<f64, String> {
    let value = args
        .next()
        .ok_or_else(|| "--threshold requires a value".to_string())?;
    let threshold: f64 = value
        .parse()
        .map_err(|_| format!("--threshold must be a number, got {value:?}"))?;
    if !threshold.is_finite() || threshold <= 0.0 {
        return Err("--threshold must be a finite number greater than zero".to_string());
    }
    Ok(threshold)
}

fn usage() -> String {
    "usage: veln-repo-metrics [--format human|json] [--dependency-graph] [--dependency-hotspots N] [--dependency-cycle-limit N] [--file-line-threshold N] [--max-findings N] [--threshold N] [PATH ...]"
        .to_string()
}

fn parse_positive_usize(option: &str, value: &str) -> Result<usize, String> {
    let parsed = value
        .parse()
        .map_err(|_| format!("{option} must be a positive integer, got {value:?}"))?;
    if parsed == 0 {
        return Err(format!("{option} must be greater than zero"));
    }
    Ok(parsed)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct AbcMetrics {
    assignments: usize,
    branches: usize,
    conditionals: usize,
}

impl AbcMetrics {
    fn score(self) -> f64 {
        let assignments = self.assignments as f64;
        let branches = self.branches as f64;
        let conditionals = self.conditionals as f64;
        (assignments.mul_add(
            assignments,
            branches.mul_add(branches, conditionals * conditionals),
        ))
        .sqrt()
    }
}

#[derive(Debug, PartialEq)]
enum Finding {
    Function(FunctionFinding),
    File(FileFinding),
}

impl Finding {
    fn compare(left: &Self, right: &Self) -> std::cmp::Ordering {
        right
            .rank()
            .total_cmp(&left.rank())
            .then_with(|| left.file().cmp(right.file()))
            .then_with(|| left.line().cmp(&right.line()))
            .then_with(|| left.label().cmp(right.label()))
    }

    fn file(&self) -> &Path {
        match self {
            Self::Function(finding) => &finding.file,
            Self::File(finding) => &finding.file,
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::Function(finding) => &finding.name,
            Self::File(_) => "file",
        }
    }

    fn line(&self) -> usize {
        match self {
            Self::Function(finding) => finding.line,
            Self::File(finding) => finding.line,
        }
    }

    fn rank(&self) -> f64 {
        match self {
            Self::Function(finding) => finding.metrics.score(),
            Self::File(finding) => finding.lines as f64 / 100.0,
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function(finding) => write!(formatter, "{finding}"),
            Self::File(finding) => write!(formatter, "{finding}"),
        }
    }
}

#[derive(Debug, PartialEq)]
struct FunctionFinding {
    file: PathBuf,
    line: usize,
    name: String,
    metrics: AbcMetrics,
}

#[derive(Debug, PartialEq)]
struct FileFinding {
    file: PathBuf,
    line: usize,
    lines: usize,
}

impl fmt::Display for FileFinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: file has {} lines",
            self.file.display(),
            self.line,
            self.lines
        )
    }
}

impl fmt::Display for FunctionFinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {} ABC {:.1} (A={}, B={}, C={})",
            self.file.display(),
            self.line,
            self.name,
            self.metrics.score(),
            self.metrics.assignments,
            self.metrics.branches,
            self.metrics.conditionals
        )
    }
}

struct Report {
    files: Vec<PathBuf>,
    findings: Vec<Finding>,
    dependency_graph: Option<dependency_graph::DependencyReport>,
}

fn collect_report(config: &Config) -> Result<Report, String> {
    let files = collect_configured_rust_files(config)?;
    let mut findings = Vec::new();
    for file in &files {
        findings.extend(analyze_file(file, config)?);
    }
    findings.sort_by(Finding::compare);
    let dependency_graph = config
        .dependency_graph
        .then(|| dependency_graph::collect_report(files.clone()))
        .transpose()?;
    Ok(Report {
        files,
        findings,
        dependency_graph,
    })
}

fn collect_configured_rust_files(config: &Config) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for root in &config.roots {
        collect_rust_files(root, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_rust_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("failed to read metadata for {}: {error}", path.display()))?;
    if metadata.is_file() {
        if path.extension() == Some(OsStr::new("rs")) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    if !metadata.is_dir() {
        return Ok(());
    }

    for entry in
        fs::read_dir(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?
    {
        let entry = entry
            .map_err(|error| format!("failed to read entry in {}: {error}", path.display()))?;
        collect_rust_files(&entry.path(), files)?;
    }

    Ok(())
}

fn analyze_file(path: &Path, config: &Config) -> Result<Vec<Finding>, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut findings = analyze_source(path, &source, config.threshold)?;
    let lines = source.lines().count();
    if lines >= config.file_line_threshold {
        findings.push(Finding::File(FileFinding {
            file: path.to_path_buf(),
            line: 1,
            lines,
        }));
    }
    Ok(findings)
}

fn analyze_source(path: &Path, source: &str, threshold: f64) -> Result<Vec<Finding>, String> {
    let syntax = syn::parse_file(source)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    let mut analyzer = FileAnalyzer {
        file: path.to_path_buf(),
        findings: Vec::new(),
        threshold,
    };
    analyzer.visit_file(&syntax);
    Ok(analyzer.findings)
}

struct FileAnalyzer {
    file: PathBuf,
    findings: Vec<Finding>,
    threshold: f64,
}

impl FileAnalyzer {
    fn record_function(&mut self, name: String, line: usize, block: &syn::Block) {
        let mut visitor = AbcVisitor::default();
        visitor.visit_block(block);
        if visitor.metrics.score() >= self.threshold {
            self.findings.push(Finding::Function(FunctionFinding {
                file: self.file.clone(),
                line,
                name,
                metrics: visitor.metrics,
            }));
        }
    }
}

impl<'ast> Visit<'ast> for FileAnalyzer {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.record_function(
            item.sig.ident.to_string(),
            item.sig.ident.span().start().line,
            &item.block,
        );
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.record_function(
            item.sig.ident.to_string(),
            item.sig.ident.span().start().line,
            &item.block,
        );
        visit::visit_impl_item_fn(self, item);
    }
}

#[derive(Default)]
struct AbcVisitor {
    metrics: AbcMetrics,
}

impl<'ast> Visit<'ast> for AbcVisitor {
    fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
        self.metrics.assignments += 1;
        visit::visit_expr_assign(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if matches!(
            node.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        ) {
            self.metrics.assignments += 1;
        }

        if matches!(node.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            self.metrics.conditionals += 1;
        }

        visit::visit_expr_binary(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        self.metrics.assignments += 1;
        visit::visit_local(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        self.metrics.branches += 1;
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.metrics.branches += 1;
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        self.metrics.branches += 1;
        visit::visit_expr_macro(self, node);
    }

    fn visit_stmt_macro(&mut self, node: &'ast syn::StmtMacro) {
        self.metrics.branches += 1;
        visit::visit_stmt_macro(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.metrics.conditionals += 1;
        visit::visit_expr_if(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.metrics.conditionals += 1;
        visit::visit_expr_for_loop(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.metrics.conditionals += 1;
        visit::visit_expr_loop(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.metrics.conditionals += 1 + node.arms.len();
        visit::visit_expr_match(self, node);
    }

    fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
        self.metrics.conditionals += 1;
        visit::visit_expr_try(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.metrics.conditionals += 1;
        visit::visit_expr_while(self, node);
    }
}

#[cfg(test)]
mod tests;
