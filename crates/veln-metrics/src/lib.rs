//! Report-only Veln source metrics.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use veln_analysis::{
    derive_source_module_path, invalid_case_rejected_visible_module_path, load_surface_module,
};
use veln_diagnostics::{
    Diagnostic, JsonValue, Severity, ToolInfo, diagnostic_to_json, parse_json_value,
    source_span_to_json as span_to_json,
};
use veln_project::{ManifestField, Project, ProjectManifest, discover_source_paths};
use veln_source::{SourceFile, SourceSpan, TextRange};
use veln_syntax::{
    BinaryOp, BodyLine, Expr, ExprKind, FunctionDecl, FunctionKind, SyntaxItem, Token, TokenKind,
    lex, parse,
};

pub const JSON_SCHEMA_VERSION: &str = "veln-metrics-json/v0";
pub const BASELINE_SCHEMA_VERSION: &str = "veln-metrics-baseline/v0";
pub const METRIC_MODEL_VERSION: &str = "veln-metrics-model/v0";
pub const DEFAULT_SIMILARITY_MIN_TOKENS: usize = 60;
pub const DEFAULT_HUMAN_OUTPUT_MAX_FINDINGS: usize = 50;

#[derive(Clone, Debug)]
pub struct MetricsReport {
    pub project: ProjectIdentity,
    pub diagnostics: Vec<Diagnostic>,
    pub completeness: MetricsCompleteness,
    pub modules: Vec<ModuleMetric>,
    pub edges: Vec<DependencyEdge>,
    pub cycles: Vec<DependencyCycle>,
    pub abc_subjects: Vec<AbcSubjectMetric>,
    pub similarities: Vec<SimilarityInstanceMetric>,
    pub summary: MetricsSummary,
    pub human_output_max_findings: usize,
}

#[derive(Clone, Debug)]
pub struct MetricsCheckReport {
    pub report: MetricsReport,
    pub policy: MetricsPolicy,
    pub violations: Vec<MetricsPolicyViolation>,
    pub baseline: Option<BaselineComparison>,
}

#[derive(Clone, Debug, Default)]
pub struct MetricsCompleteness {
    pub excluded_sources: Vec<ExcludedSource>,
    pub excluded_baseline_subjects: Vec<String>,
}

impl MetricsCompleteness {
    pub fn is_partial(&self) -> bool {
        !self.excluded_sources.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExcludedSource {
    pub path: String,
    pub reason: String,
}

impl MetricsCheckReport {
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetricsPolicy {
    pub deny_cycles: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetricsConfig {
    pub policy: MetricsPolicy,
    pub similarity_min_tokens: usize,
    pub human_output_max_findings: usize,
}

impl MetricsConfig {
    fn apply_manifest_field(&mut self, field: &ManifestField) -> Option<Diagnostic> {
        match field.key.as_str() {
            "deny_cycles" => apply_metrics_field(
                &mut self.policy.deny_cycles,
                parse_boolean_metrics_field(field),
                field,
                JsonValue::array([JsonValue::string("true"), JsonValue::string("false")]),
            ),
            "similarity_min_tokens" => apply_metrics_field(
                &mut self.similarity_min_tokens,
                parse_positive_metrics_field(field, usize::MAX),
                field,
                JsonValue::string("positive integer string"),
            ),
            "max_findings" => apply_metrics_field(
                &mut self.human_output_max_findings,
                parse_positive_metrics_field(field, max_json_usize()),
                field,
                JsonValue::string("positive integer string"),
            ),
            _ => Some(unsupported_metrics_field_diagnostic(field)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricsPolicyViolation {
    pub policy: String,
    pub cycle_members: Vec<String>,
    pub path: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselineComparison {
    pub path: String,
    pub stale_subjects: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct MetricsBaseline {
    pub modules: Vec<BaselineModule>,
    pub edges: Vec<BaselineEdge>,
    pub cycles: Vec<BaselineCycle>,
}

#[derive(Clone, Debug)]
pub struct BaselineModule {
    pub module: String,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BaselineEdge {
    pub source: String,
    pub target: String,
}

#[derive(Clone, Debug)]
pub struct BaselineCycle {
    pub members: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ProjectIdentity {
    pub root: String,
    pub selected_paths: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ModuleMetric {
    pub module: String,
    pub path: String,
    pub generated: bool,
    pub fan_in: usize,
    pub fan_out: usize,
    pub dependency_pressure: usize,
    pub external_dependency_count: usize,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyEdge {
    pub source: String,
    pub target: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct DependencyCycle {
    pub members: Vec<String>,
    pub path: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AbcSubjectMetric {
    pub identity: String,
    pub path: String,
    pub name: String,
    pub kind: AbcSubjectKind,
    pub vector: AbcVector,
    pub magnitude: f64,
    pub contracts_included: bool,
    pub generated: bool,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SimilarityInstanceMetric {
    pub identity: String,
    pub fingerprint: String,
    pub token_count: usize,
    pub experimental: bool,
    pub declarations: Vec<SimilarityDeclarationMetric>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimilarityDeclarationMetric {
    pub identity: String,
    pub path: String,
    pub name: String,
    pub kind: AbcSubjectKind,
    pub generated: bool,
    pub span: SourceSpan,
    pub body_span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbcSubjectKind {
    Function,
    Test,
}

impl AbcSubjectKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Test => "test",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AbcVector {
    pub assignments: usize,
    pub branches: usize,
    pub conditionals: usize,
}

impl AbcVector {
    fn magnitude(self) -> f64 {
        ((self.assignments * self.assignments
            + self.branches * self.branches
            + self.conditionals * self.conditionals) as f64)
            .sqrt()
    }
}

#[derive(Clone, Debug)]
pub struct MetricsSummary {
    pub selected_module_count: usize,
    pub project_module_count: usize,
    pub internal_edge_count: usize,
    pub cycle_count: usize,
    pub external_dependency_count: usize,
    pub abc_subject_count: usize,
    pub abc_contract_subject_count: usize,
    pub similarity_fingerprint_count: usize,
    pub similarity_instance_count: usize,
    pub similarity_region_count: usize,
}

pub fn analyze_project_metrics(
    root: PathBuf,
    inputs: &[PathBuf],
) -> Result<MetricsReport, Vec<Diagnostic>> {
    let (full_project, config) = discover_metrics_project(&root)?;
    analyze_project_metrics_from_project(root, inputs, full_project, config)
}

pub fn check_project_metrics(
    root: PathBuf,
    inputs: &[PathBuf],
) -> Result<MetricsCheckReport, Vec<Diagnostic>> {
    let (report, policy) = analyze_project_metrics_for_check(root, inputs)?;
    Ok(evaluate_metrics_check(report, policy))
}

pub fn check_project_metrics_with_baseline(
    root: PathBuf,
    inputs: &[PathBuf],
    baseline: MetricsBaseline,
    baseline_path: String,
) -> Result<MetricsCheckReport, Vec<Diagnostic>> {
    let (report, policy) = analyze_project_metrics_for_check(root, inputs)?;
    Ok(evaluate_metrics_check_with_baseline(
        report,
        policy,
        baseline,
        baseline_path,
    ))
}

fn analyze_project_metrics_for_check(
    root: PathBuf,
    inputs: &[PathBuf],
) -> Result<(MetricsReport, MetricsPolicy), Vec<Diagnostic>> {
    let (full_project, config) = discover_metrics_project(&root)?;
    let policy = config.policy;
    require_enabled_metrics_policy(policy)?;
    let report = analyze_project_metrics_from_project(root, inputs, full_project, config)?;
    Ok((report, policy))
}

fn discover_metrics_project(root: &Path) -> Result<(Project, MetricsConfig), Vec<Diagnostic>> {
    let project = Project::discover(root.to_path_buf(), &[]).map_err(|error| {
        vec![metrics_io_diagnostic(format!(
            "source discovery failed: {error}"
        ))]
    })?;
    let config = read_metrics_config(project.manifest.as_ref())?;
    Ok((project, config))
}

fn require_enabled_metrics_policy(policy: MetricsPolicy) -> Result<(), Vec<Diagnostic>> {
    if !policy.deny_cycles {
        return Err(vec![metrics_policy_diagnostic(
            "metrics.policy.no_enabled",
            "metrics check requires at least one enabled policy".to_string(),
            None,
            JsonValue::object([("policy", JsonValue::string("none"))]),
        )]);
    }
    Ok(())
}

mod abc;
mod analysis;
mod dependency_graph;
mod human_output;
mod json_output;
mod similarity;

use abc::*;
use analysis::*;
use dependency_graph::*;
use human_output::{detailed_report_finding_count, omitted_report_finding_count};
use json_output::{
    json_string_field, max_json_usize, metrics_io_diagnostic, metrics_policy_diagnostic,
};
use similarity::*;

pub use human_output::{render_check_human, render_human};
pub use json_output::{baseline_from_json, baseline_to_json, report_check_to_json, report_to_json};

#[cfg(test)]
mod tests;
