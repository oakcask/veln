use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};

mod dependency_graph;

const DEFAULT_THRESHOLD: f64 = 30.0;
const DEFAULT_FILE_LINE_THRESHOLD: usize = 700;
const DEFAULT_DEPENDENCY_HOTSPOTS: usize = 10;
const DEFAULT_DEPENDENCY_CYCLE_LIMIT: usize = 5;
const DEFAULT_MAX_WARNINGS: usize = 50;

fn main() {
    if help_requested(env::args().skip(1)) {
        println!("{}", usage());
        return;
    }

    let config = match Config::parse(env::args().skip(1)) {
        Ok(config) => config,
        Err(message) => exit_with_message(2, message),
    };

    let mut findings = match collect_findings(&config) {
        Ok(findings) => findings,
        Err(message) => exit_with_message(1, message),
    };

    findings.sort_by(Finding::compare);
    emit_findings(&findings, &config);

    let dependency_cycles = if config.dependency_summary {
        emit_dependency_summary(&config)
    } else {
        0
    };

    if findings.iter().any(Finding::blocks_merge)
        || config.deny_dependency_cycles && dependency_cycles > 0
    {
        if config.deny_dependency_cycles && dependency_cycles > 0 {
            eprintln!(
                "dependency graph contains {dependency_cycles} strongly connected group(s); inspect Dependency Graph Refactor Signal and remove an ownership cycle because cyclic boundaries force changes to coordinate in both directions"
            );
        }
        std::process::exit(1);
    }
}

fn help_requested(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().any(|arg| arg == "--help" || arg == "-h")
}

fn exit_with_message(code: i32, message: String) -> ! {
    eprintln!("{message}");
    std::process::exit(code);
}

fn emit_findings(findings: &[Finding], config: &Config) {
    let shown = findings.len().min(config.max_warnings);
    for finding in findings.iter().take(config.max_warnings) {
        if config.github_annotations {
            println!("{}", finding.github_annotation());
        } else {
            println!("{finding}");
        }
    }

    if config.github_annotations && findings.len() > shown {
        println!(
            "::notice title=Code metrics truncated::Inspect the first {shown} code metric warnings; CI showed the highest-ranked findings and truncated {} total warnings to keep annotations usable.",
            findings.len()
        );
    }
}

fn emit_dependency_summary(config: &Config) -> usize {
    let files = match collect_configured_rust_files(config) {
        Ok(files) => files,
        Err(message) => exit_with_message(1, message),
    };
    let summary = match dependency_graph::collect_summary(
        files,
        config.dependency_hotspots,
        config.dependency_cycle_limit,
    ) {
        Ok(summary) => summary,
        Err(message) => exit_with_message(1, message),
    };
    if let Err(message) = dependency_graph::emit_summary(&summary.text) {
        exit_with_message(1, message);
    }
    summary.cycle_count
}

#[derive(Debug)]
struct Config {
    dependency_cycle_limit: usize,
    dependency_hotspots: usize,
    dependency_summary: bool,
    deny_dependency_cycles: bool,
    deny_numbered_split_files: bool,
    github_annotations: bool,
    file_line_threshold: usize,
    max_warnings: usize,
    roots: Vec<PathBuf>,
    threshold: f64,
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
    dependency_summary: bool,
    deny_dependency_cycles: bool,
    deny_numbered_split_files: bool,
    github_annotations: bool,
    file_line_threshold: usize,
    max_warnings: usize,
    roots: Vec<PathBuf>,
    threshold: f64,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            dependency_cycle_limit: DEFAULT_DEPENDENCY_CYCLE_LIMIT,
            dependency_hotspots: DEFAULT_DEPENDENCY_HOTSPOTS,
            dependency_summary: false,
            deny_dependency_cycles: false,
            deny_numbered_split_files: false,
            github_annotations: false,
            file_line_threshold: DEFAULT_FILE_LINE_THRESHOLD,
            max_warnings: DEFAULT_MAX_WARNINGS,
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
            "--dependency-summary" => self.dependency_summary = true,
            "--deny-dependency-cycles" => {
                self.dependency_summary = true;
                self.deny_dependency_cycles = true;
            }
            "--dependency-cycle-limit" => {
                self.dependency_cycle_limit = parse_next_usize(args, "--dependency-cycle-limit")?;
            }
            "--dependency-hotspots" => {
                self.dependency_hotspots = parse_next_usize(args, "--dependency-hotspots")?;
            }
            "--deny-numbered-split-files" => self.deny_numbered_split_files = true,
            "--github-annotations" => self.github_annotations = true,
            "--file-line-threshold" => {
                self.file_line_threshold = parse_next_usize(args, "--file-line-threshold")?;
            }
            "--max-warnings" => self.max_warnings = parse_next_usize(args, "--max-warnings")?,
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
            dependency_summary: self.dependency_summary,
            deny_dependency_cycles: self.deny_dependency_cycles,
            deny_numbered_split_files: self.deny_numbered_split_files,
            github_annotations: self.github_annotations,
            file_line_threshold: self.file_line_threshold,
            max_warnings: self.max_warnings,
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
    let threshold = value
        .parse()
        .map_err(|_| format!("--threshold must be a number, got {value:?}"))?;
    if threshold <= 0.0 {
        return Err("--threshold must be greater than zero".to_string());
    }
    Ok(threshold)
}

fn usage() -> String {
    "usage: veln-code-metrics [--github-annotations] [--dependency-summary] [--dependency-hotspots N] [--dependency-cycle-limit N] [--deny-dependency-cycles] [--deny-numbered-split-files] [--file-line-threshold N] [--max-warnings N] [--threshold N] [PATH ...]"
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
    NumberedSplitFile(NumberedSplitFileFinding),
}

impl Finding {
    fn blocks_merge(&self) -> bool {
        matches!(self, Self::NumberedSplitFile(_))
    }

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
            Self::NumberedSplitFile(finding) => &finding.file,
        }
    }

    fn github_annotation(&self) -> String {
        match self {
            Self::Function(finding) => finding.github_warning(),
            Self::File(finding) => finding.github_warning(),
            Self::NumberedSplitFile(finding) => finding.github_error(),
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::Function(finding) => &finding.name,
            Self::File(_) => "file",
            Self::NumberedSplitFile(_) => "numbered split file",
        }
    }

    fn line(&self) -> usize {
        match self {
            Self::Function(finding) => finding.line,
            Self::File(finding) => finding.line,
            Self::NumberedSplitFile(finding) => finding.line,
        }
    }

    fn rank(&self) -> f64 {
        match self {
            Self::Function(finding) => finding.metrics.score(),
            Self::File(finding) => finding.lines as f64 / 100.0,
            Self::NumberedSplitFile(_) => f64::INFINITY,
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Function(finding) => write!(formatter, "{finding}"),
            Self::File(finding) => write!(formatter, "{finding}"),
            Self::NumberedSplitFile(finding) => write!(formatter, "{finding}"),
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

impl FunctionFinding {
    fn github_warning(&self) -> String {
        format!(
            "::warning file={},line={},title={}::{}",
            annotation_property_escape(self.file.to_string_lossy().as_ref()),
            self.line,
            annotation_property_escape("High ABC complexity"),
            annotation_message_escape(&format!(
                "{} has ABC {:.1} (A={}, B={}, C={}); when touching this function, improve cohesion around one concern, clarify ownership boundaries, or decouple distinct concepts so complexity decreases for reviewers instead of moving mechanically",
                self.name,
                self.metrics.score(),
                self.metrics.assignments,
                self.metrics.branches,
                self.metrics.conditionals
            ))
        )
    }
}

#[derive(Debug, PartialEq)]
struct FileFinding {
    file: PathBuf,
    line: usize,
    lines: usize,
}

impl FileFinding {
    fn github_warning(&self) -> String {
        format!(
            "::warning file={},line={},title={}::{}",
            annotation_property_escape(self.file.to_string_lossy().as_ref()),
            self.line,
            annotation_property_escape("Large Rust file"),
            annotation_message_escape(&format!(
                "{} has {} lines; when touching this file, check whether its responsibilities still share one cohesive owner, or move distinct concepts behind clearer boundaries so review scope stays understandable",
                self.file.display(),
                self.lines
            ))
        )
    }
}

#[derive(Debug, PartialEq)]
struct NumberedSplitFileFinding {
    file: PathBuf,
    line: usize,
}

impl NumberedSplitFileFinding {
    fn github_error(&self) -> String {
        format!(
            "::error file={},line={},title={}::{}",
            annotation_property_escape(self.file.to_string_lossy().as_ref()),
            self.line,
            annotation_property_escape("Numbered split file"),
            annotation_message_escape(
                "Rename this Rust file to describe its responsibility before merging; numbered bucket files hide ownership and make code-metric refactors mechanical"
            )
        )
    }
}

impl fmt::Display for NumberedSplitFileFinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: numbered split file name; rename this Rust file to describe its responsibility",
            self.file.display(),
            self.line
        )
    }
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

fn annotation_property_escape(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

fn annotation_message_escape(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn collect_findings(config: &Config) -> Result<Vec<Finding>, String> {
    let files = collect_configured_rust_files(config)?;

    let mut findings = Vec::new();
    for file in &files {
        findings.extend(analyze_file(file, config)?);
    }
    if config.deny_numbered_split_files {
        findings.extend(numbered_split_file_findings(&files));
    }
    Ok(findings)
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

fn numbered_split_file_findings(files: &[PathBuf]) -> Vec<Finding> {
    let mut groups: BTreeMap<(PathBuf, String), Vec<PathBuf>> = BTreeMap::new();
    for file in files {
        if let Some((directory, prefix)) = numbered_suffix_group(file) {
            groups
                .entry((directory, prefix))
                .or_default()
                .push(file.clone());
        }
    }

    groups
        .into_values()
        .filter(|group| group.len() > 1)
        .flatten()
        .map(|file| Finding::NumberedSplitFile(NumberedSplitFileFinding { file, line: 1 }))
        .collect()
}

fn numbered_suffix_group(path: &Path) -> Option<(PathBuf, String)> {
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|stem| !stem.is_empty())?;
    let prefix_end = stem
        .bytes()
        .rposition(|byte| !byte.is_ascii_digit())
        .map(|index| index + 1)?;
    if prefix_end == stem.len() || prefix_end == 0 {
        return None;
    }

    let directory = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
    Some((directory, stem[..prefix_end].to_string()))
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
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn reports_function_over_threshold() {
        let source = r#"
fn complex(input: Result<i32, ()>) -> i32 {
    let mut total = 0;
    total += helper();
    if total > 1 && helper() > 2 {
        total = helper();
    }
    match input {
        Ok(value) => value,
        Err(_) => fallback(),
    }
}

fn helper() -> i32 {
    1
}

fn fallback() -> i32 {
    0
}
"#;

        let findings = analyze_source(Path::new("sample.rs"), source, 5.0).unwrap();

        assert_eq!(findings.len(), 1);
        let Finding::Function(finding) = &findings[0] else {
            panic!("expected function finding");
        };
        assert_eq!(finding.name, "complex");
        assert_eq!(
            finding.metrics,
            AbcMetrics {
                assignments: 3,
                branches: 4,
                conditionals: 5,
            }
        );
    }

    #[test]
    fn escapes_github_annotation_properties() {
        assert_eq!(
            annotation_property_escape("a:b,c%\r\n"),
            "a%3Ab%2Cc%25%0D%0A"
        );
    }

    #[test]
    fn leaves_github_annotation_message_punctuation_readable() {
        assert_eq!(
            annotation_message_escape("A=1, B=2: check 100%\r\n"),
            "A=1, B=2: check 100%25%0D%0A"
        );
    }

    #[test]
    fn reports_file_over_line_threshold() {
        let config = Config {
            dependency_cycle_limit: DEFAULT_DEPENDENCY_CYCLE_LIMIT,
            dependency_hotspots: DEFAULT_DEPENDENCY_HOTSPOTS,
            dependency_summary: false,
            deny_dependency_cycles: false,
            deny_numbered_split_files: false,
            github_annotations: false,
            file_line_threshold: 3,
            max_warnings: 10,
            roots: Vec::new(),
            threshold: 100.0,
        };
        let source = "fn tiny() {}\n\nfn other() {}\n";
        let path = Path::new("sample.rs");

        let mut findings = analyze_source(path, source, config.threshold).unwrap();
        let lines = source.lines().count();
        if lines >= config.file_line_threshold {
            findings.push(Finding::File(FileFinding {
                file: path.to_path_buf(),
                line: 1,
                lines,
            }));
        }

        assert!(matches!(findings.last(), Some(Finding::File(finding)) if finding.lines == 3));
    }

    #[test]
    fn parses_dependency_summary_options() {
        let config = Config::parse([
            "--dependency-summary".to_string(),
            "--dependency-hotspots".to_string(),
            "3".to_string(),
            "--dependency-cycle-limit".to_string(),
            "2".to_string(),
            "--deny-dependency-cycles".to_string(),
            "crates".to_string(),
        ])
        .unwrap();

        assert!(config.dependency_summary);
        assert!(config.deny_dependency_cycles);
        assert_eq!(config.dependency_hotspots, 3);
        assert_eq!(config.dependency_cycle_limit, 2);
        assert_eq!(config.roots, vec![PathBuf::from("crates")]);
    }

    #[test]
    fn identifies_numbered_suffix_groups() {
        assert_eq!(
            numbered_suffix_group(Path::new("nested/parser01.rs")),
            Some((PathBuf::from("nested"), "parser".to_string()))
        );
        assert_eq!(
            numbered_suffix_group(Path::new("sha256.rs")),
            Some((PathBuf::from(""), "sha".to_string()))
        );
        assert_eq!(numbered_suffix_group(Path::new("partitions.rs")), None);
        assert_eq!(numbered_suffix_group(Path::new("part.rs")), None);
        assert_eq!(numbered_suffix_group(Path::new("123.rs")), None);
    }

    #[test]
    fn reports_numbered_suffix_series_with_shared_prefix() {
        let findings = numbered_split_file_findings(&[
            PathBuf::from("parser01.rs"),
            PathBuf::from("parser02.rs"),
            PathBuf::from("sha256.rs"),
            PathBuf::from("nested/parser03.rs"),
            PathBuf::from("nested/parser04.rs"),
        ]);

        let files = findings.iter().map(Finding::file).collect::<Vec<_>>();
        assert_eq!(
            files,
            vec![
                Path::new("parser01.rs"),
                Path::new("parser02.rs"),
                Path::new("nested/parser03.rs"),
                Path::new("nested/parser04.rs"),
            ]
        );
    }

    #[test]
    fn numbered_split_file_finding_blocks_merge() {
        let finding = Finding::NumberedSplitFile(NumberedSplitFileFinding {
            file: PathBuf::from("part01.rs"),
            line: 1,
        });

        assert!(finding.blocks_merge());
        assert!(
            finding
                .github_annotation()
                .contains("Rename this Rust file"),
            "annotation should tell maintainers what action is required"
        );
    }
}
