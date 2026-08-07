//! Report-only Veln source metrics.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use veln_analysis::{derive_source_module_path, load_surface_module};
use veln_diagnostics::{Diagnostic, JsonValue, Severity, ToolInfo, parse_json_value};
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
    let full_project = Project::discover(root.clone(), &[]).map_err(|error| {
        vec![metrics_io_diagnostic(format!(
            "source discovery failed: {error}"
        ))]
    })?;
    let config = read_metrics_config(full_project.manifest.as_ref())?;
    analyze_project_metrics_from_project(root, inputs, full_project, config)
}

pub fn check_project_metrics(
    root: PathBuf,
    inputs: &[PathBuf],
) -> Result<MetricsCheckReport, Vec<Diagnostic>> {
    let full_project = Project::discover(root.clone(), &[]).map_err(|error| {
        vec![metrics_io_diagnostic(format!(
            "source discovery failed: {error}"
        ))]
    })?;
    let config = read_metrics_config(full_project.manifest.as_ref())?;
    let policy = config.policy;
    if !policy.deny_cycles {
        return Err(vec![metrics_policy_diagnostic(
            "metrics.policy.no_enabled",
            "metrics check requires at least one enabled policy".to_string(),
            None,
            JsonValue::object([("policy", JsonValue::string("none"))]),
        )]);
    }
    let report = analyze_project_metrics_from_project(root, inputs, full_project, config)?;
    Ok(evaluate_metrics_check(report, policy))
}

pub fn check_project_metrics_with_baseline(
    root: PathBuf,
    inputs: &[PathBuf],
    baseline: MetricsBaseline,
    baseline_path: String,
) -> Result<MetricsCheckReport, Vec<Diagnostic>> {
    let full_project = Project::discover(root.clone(), &[]).map_err(|error| {
        vec![metrics_io_diagnostic(format!(
            "source discovery failed: {error}"
        ))]
    })?;
    let config = read_metrics_config(full_project.manifest.as_ref())?;
    let policy = config.policy;
    if !policy.deny_cycles {
        return Err(vec![metrics_policy_diagnostic(
            "metrics.policy.no_enabled",
            "metrics check requires at least one enabled policy".to_string(),
            None,
            JsonValue::object([("policy", JsonValue::string("none"))]),
        )]);
    }
    let report = analyze_project_metrics_from_project(root, inputs, full_project, config)?;
    Ok(evaluate_metrics_check_with_baseline(
        report,
        policy,
        baseline,
        baseline_path,
    ))
}

fn analyze_project_metrics_from_project(
    root: PathBuf,
    inputs: &[PathBuf],
    full_project: Project,
    config: MetricsConfig,
) -> Result<MetricsReport, Vec<Diagnostic>> {
    let project = project_owned_sources(full_project);
    let source_diagnostics = source_graph_diagnostics(&project);
    if has_error(&source_diagnostics) {
        return Err(source_diagnostics);
    }

    let selected_paths = selected_source_paths(&root, inputs)?;
    let selected_paths = if inputs.is_empty() {
        project
            .files
            .iter()
            .map(|source| source.path().as_str().to_string())
            .collect()
    } else {
        selected_paths
    };
    let selected_paths = selected_paths.into_iter().collect::<BTreeSet<_>>();
    let graph = DependencyGraph::from_project(&project)?;
    Ok(graph.report(
        &project,
        ProjectIdentity {
            root: ".".to_string(),
            selected_paths: selected_paths.iter().cloned().collect(),
        },
        &selected_paths,
        config,
    ))
}

fn read_metrics_config(
    manifest: Option<&ProjectManifest>,
) -> Result<MetricsConfig, Vec<Diagnostic>> {
    let mut config = default_metrics_config();
    let Some(tool) = manifest
        .into_iter()
        .flat_map(|manifest| &manifest.tools)
        .find(|tool| tool.name == "metrics")
    else {
        return Ok(config);
    };

    let diagnostics = tool
        .fields
        .iter()
        .filter_map(|field| config.apply_manifest_field(field))
        .collect::<Vec<_>>();

    if diagnostics.is_empty() {
        Ok(config)
    } else {
        Err(diagnostics)
    }
}

fn default_metrics_config() -> MetricsConfig {
    MetricsConfig {
        policy: MetricsPolicy { deny_cycles: false },
        similarity_min_tokens: DEFAULT_SIMILARITY_MIN_TOKENS,
        human_output_max_findings: DEFAULT_HUMAN_OUTPUT_MAX_FINDINGS,
    }
}

fn parse_boolean_metrics_field(field: &ManifestField) -> Option<bool> {
    match field.value.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_positive_metrics_field(field: &ManifestField, maximum: usize) -> Option<usize> {
    field
        .value
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0 && *value <= maximum)
}

fn apply_metrics_field<Value>(
    target: &mut Value,
    value: Option<Value>,
    field: &ManifestField,
    allowed: JsonValue,
) -> Option<Diagnostic> {
    let Some(value) = value else {
        return Some(invalid_metrics_field_diagnostic(field, allowed));
    };
    *target = value;
    None
}

fn invalid_metrics_field_diagnostic(field: &ManifestField, allowed: JsonValue) -> Diagnostic {
    metrics_policy_diagnostic(
        "metrics.policy.invalid_value",
        format!(
            "invalid metrics policy value `{}` for `{}`",
            field.value, field.key
        ),
        Some(field.value_span.clone()),
        JsonValue::object([
            ("field", JsonValue::string(field.key.clone())),
            ("allowed", allowed),
        ]),
    )
}

fn unsupported_metrics_field_diagnostic(field: &ManifestField) -> Diagnostic {
    metrics_policy_diagnostic(
        "metrics.policy.unsupported_field",
        format!("unsupported metrics policy field `{}`", field.key),
        Some(field.key_span.clone()),
        JsonValue::object([
            ("field", JsonValue::string(field.key.clone())),
            ("tool", JsonValue::string("metrics")),
        ]),
    )
}

#[cfg(test)]
fn read_metrics_policy(
    manifest: Option<&ProjectManifest>,
) -> Result<MetricsPolicy, Vec<Diagnostic>> {
    read_metrics_config(manifest).map(|config| config.policy)
}

fn evaluate_metrics_check(report: MetricsReport, policy: MetricsPolicy) -> MetricsCheckReport {
    let violations = if policy.deny_cycles {
        report
            .cycles
            .iter()
            .map(|cycle| MetricsPolicyViolation {
                policy: "deny_cycles".to_string(),
                cycle_members: cycle.members.clone(),
                path: cycle.path.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    MetricsCheckReport {
        report,
        policy,
        violations,
        baseline: None,
    }
}

fn evaluate_metrics_check_with_baseline(
    report: MetricsReport,
    policy: MetricsPolicy,
    baseline: MetricsBaseline,
    baseline_path: String,
) -> MetricsCheckReport {
    let violations = if policy.deny_cycles {
        report
            .cycles
            .iter()
            .filter(|cycle| !baseline_allows_cycle(&report, cycle, &baseline))
            .map(|cycle| MetricsPolicyViolation {
                policy: "deny_cycles".to_string(),
                cycle_members: cycle.members.clone(),
                path: cycle.path.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    let current_subjects = report
        .modules
        .iter()
        .map(|module| module.module.as_str())
        .collect::<BTreeSet<_>>();
    let stale_subjects = baseline
        .modules
        .iter()
        .filter(|module| !current_subjects.contains(module.module.as_str()))
        .map(|module| module.module.clone())
        .collect();
    MetricsCheckReport {
        report,
        policy,
        violations,
        baseline: Some(BaselineComparison {
            path: baseline_path,
            stale_subjects,
        }),
    }
}

fn baseline_allows_cycle(
    report: &MetricsReport,
    cycle: &DependencyCycle,
    baseline: &MetricsBaseline,
) -> bool {
    let current_members = cycle.members.iter().cloned().collect::<BTreeSet<_>>();
    let current_edges = cyclic_edges(&report.edges, &current_members);
    baseline.cycles.iter().any(|baseline_cycle| {
        let baseline_members = baseline_cycle
            .members
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        current_members.is_subset(&baseline_members)
            && current_edges.is_subset(&cyclic_edges(&baseline.edges, &baseline_members))
    })
}

fn cyclic_edges<Edge>(edges: &[Edge], members: &BTreeSet<String>) -> BTreeSet<BaselineEdge>
where
    Edge: DependencyEdgeLike,
{
    edges
        .iter()
        .filter_map(|edge| {
            let source = edge.source();
            let target = edge.target();
            (members.contains(source) && members.contains(target)).then(|| BaselineEdge {
                source: source.to_string(),
                target: target.to_string(),
            })
        })
        .collect()
}

trait DependencyEdgeLike {
    fn source(&self) -> &str;
    fn target(&self) -> &str;
}

impl DependencyEdgeLike for DependencyEdge {
    fn source(&self) -> &str {
        &self.source
    }

    fn target(&self) -> &str {
        &self.target
    }
}

impl DependencyEdgeLike for BaselineEdge {
    fn source(&self) -> &str {
        &self.source
    }

    fn target(&self) -> &str {
        &self.target
    }
}

fn source_graph_diagnostics(project: &Project) -> Vec<Diagnostic> {
    let (_, diagnostics) = load_surface_module(project);
    diagnostics
}

fn project_owned_sources(mut project: Project) -> Project {
    let prefixes = dependency_path_prefixes(&project);
    if prefixes.is_empty() {
        return project;
    }
    project.files.retain(|source| {
        let path = source.path().as_str();
        !prefixes
            .iter()
            .any(|prefix| path == prefix || path.starts_with(&format!("{prefix}/")))
    });
    project
}

fn dependency_path_prefixes(project: &Project) -> Vec<String> {
    project
        .manifest
        .as_ref()
        .into_iter()
        .flat_map(|manifest| &manifest.dependencies)
        .filter_map(|dependency| dependency.path.as_ref())
        .map(|field| field.value.trim_matches('/').to_string())
        .filter(|path| !path.is_empty())
        .collect()
}

fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

fn selected_source_paths(
    root: &std::path::Path,
    inputs: &[PathBuf],
) -> Result<Vec<String>, Vec<Diagnostic>> {
    discover_source_paths(root, inputs)
        .map_err(|error| {
            vec![metrics_io_diagnostic(format!(
                "source selection failed: {error}"
            ))]
        })?
        .into_iter()
        .map(|path| {
            SourceFile::read(root, &path)
                .map(|source| source.path().as_str().to_string())
                .map_err(|error| {
                    vec![metrics_io_diagnostic(format!(
                        "selected source could not be read: {error}"
                    ))]
                })
        })
        .collect()
}

#[derive(Debug)]
struct DependencyGraph {
    nodes: Vec<DependencyNode>,
    incoming: Vec<BTreeSet<usize>>,
    outgoing: Vec<BTreeSet<usize>>,
    edges: Vec<DependencyEdgeIndex>,
}

#[derive(Debug)]
struct DependencyNode {
    module: String,
    path: String,
    span: SourceSpan,
    external_dependencies: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct DependencyEdgeIndex {
    source: usize,
    target: usize,
    span: SourceSpan,
}

impl DependencyGraph {
    fn from_project(project: &Project) -> Result<Self, Vec<Diagnostic>> {
        let mut nodes = Vec::new();
        let mut module_index = BTreeMap::new();
        for source in &project.files {
            let module =
                derive_source_module_path(source).map_err(|diagnostic| vec![*diagnostic])?;
            let index = nodes.len();
            module_index.insert(module.clone(), index);
            nodes.push(DependencyNode {
                module,
                path: source.path().as_str().to_string(),
                span: source.span(veln_source::TextRange::new(0, 0)),
                external_dependencies: BTreeSet::new(),
            });
        }

        let mut edge_keys = BTreeMap::<(usize, usize), SourceSpan>::new();
        for source in &project.files {
            let module =
                derive_source_module_path(source).map_err(|diagnostic| vec![*diagnostic])?;
            let source_index = module_index[&module];
            let parsed = parse(source);
            if !parsed.diagnostics.is_empty() {
                continue;
            }
            for use_decl in parsed.tree.uses {
                if let Some(package) = use_decl.package {
                    if package.name != veln_stdlib::PACKAGE_NAME {
                        nodes[source_index]
                            .external_dependencies
                            .insert(format!("{}::{}", package.name, use_decl.name));
                    }
                    continue;
                }
                if let Some(target_index) = module_index.get(&use_decl.name) {
                    edge_keys
                        .entry((source_index, *target_index))
                        .or_insert(use_decl.span);
                }
            }
        }

        let mut incoming = vec![BTreeSet::new(); nodes.len()];
        let mut outgoing = vec![BTreeSet::new(); nodes.len()];
        let mut edges = Vec::new();
        for ((source, target), span) in edge_keys {
            outgoing[source].insert(target);
            incoming[target].insert(source);
            edges.push(DependencyEdgeIndex {
                source,
                target,
                span,
            });
        }
        edges.sort_by(|left, right| {
            nodes[left.source]
                .module
                .cmp(&nodes[right.source].module)
                .then_with(|| nodes[left.target].module.cmp(&nodes[right.target].module))
        });

        Ok(Self {
            nodes,
            incoming,
            outgoing,
            edges,
        })
    }

    fn report(
        self,
        source_project: &Project,
        project: ProjectIdentity,
        selected_paths: &BTreeSet<String>,
        config: MetricsConfig,
    ) -> MetricsReport {
        let selected_modules = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| selected_paths.contains(&node.path).then_some(index))
            .collect::<BTreeSet<_>>();
        let mut modules = selected_modules
            .iter()
            .map(|index| self.module_metric(*index))
            .collect::<Vec<_>>();
        modules.sort_by(compare_module_metrics);

        let edges = self
            .edges
            .iter()
            .filter(|edge| {
                selected_modules.contains(&edge.source) || selected_modules.contains(&edge.target)
            })
            .map(|edge| DependencyEdge {
                source: self.nodes[edge.source].module.clone(),
                target: self.nodes[edge.target].module.clone(),
                span: edge.span.clone(),
            })
            .collect::<Vec<_>>();

        let cycles = self
            .cycles()
            .into_iter()
            .filter(|cycle| cycle.iter().any(|index| selected_modules.contains(index)))
            .map(|cycle| self.dependency_cycle(cycle))
            .collect::<Vec<_>>();
        let external_dependency_count = modules
            .iter()
            .map(|module| module.external_dependency_count)
            .sum();
        let abc_subjects = abc_subjects(source_project, selected_paths);
        let abc_contract_subject_count = abc_contract_subject_count(source_project, selected_paths);
        let (similarities, similarity_fingerprint_count) =
            similarity_instances(source_project, selected_paths, config.similarity_min_tokens);
        let similarity_region_count = similarities
            .iter()
            .map(|instance| instance.declarations.len())
            .sum();
        let summary = MetricsSummary {
            selected_module_count: modules.len(),
            project_module_count: self.nodes.len(),
            internal_edge_count: self.edges.len(),
            cycle_count: cycles.len(),
            external_dependency_count,
            abc_subject_count: abc_subjects.len(),
            abc_contract_subject_count,
            similarity_fingerprint_count,
            similarity_instance_count: similarities.len(),
            similarity_region_count,
        };
        MetricsReport {
            project,
            modules,
            edges,
            cycles,
            abc_subjects,
            similarities,
            summary,
            human_output_max_findings: config.human_output_max_findings,
        }
    }

    fn module_metric(&self, index: usize) -> ModuleMetric {
        let fan_in = self.incoming[index].len();
        let fan_out = self.outgoing[index].len();
        let node = &self.nodes[index];
        ModuleMetric {
            module: node.module.clone(),
            path: node.path.clone(),
            generated: false,
            fan_in,
            fan_out,
            dependency_pressure: fan_in * fan_out,
            external_dependency_count: node.external_dependencies.len(),
            span: node.span.clone(),
        }
    }

    fn cycles(&self) -> Vec<Vec<usize>> {
        let mut tarjan = Tarjan::new(&self.outgoing);
        let mut cycles = tarjan.components();
        cycles.retain(|component| {
            component.len() > 1
                || component
                    .first()
                    .is_some_and(|index| self.outgoing[*index].contains(index))
        });
        for cycle in &mut cycles {
            cycle.sort_by(|left, right| self.nodes[*left].module.cmp(&self.nodes[*right].module));
        }
        cycles.sort_by(|left, right| {
            self.nodes[left[0]]
                .module
                .cmp(&self.nodes[right[0]].module)
                .then_with(|| left.len().cmp(&right.len()))
        });
        cycles
    }

    fn dependency_cycle(&self, members: Vec<usize>) -> DependencyCycle {
        let member_set = members.iter().copied().collect::<BTreeSet<_>>();
        let start = *members
            .iter()
            .min_by(|left, right| self.nodes[**left].module.cmp(&self.nodes[**right].module))
            .expect("cycles have at least one member");
        let mut path = Vec::new();
        let mut visited = BTreeSet::new();
        let _ = self.find_closed_path(start, start, &member_set, &mut visited, &mut path);
        let members = members
            .into_iter()
            .map(|index| self.nodes[index].module.clone())
            .collect();
        DependencyCycle { members, path }
    }

    fn find_closed_path(
        &self,
        current: usize,
        target: usize,
        members: &BTreeSet<usize>,
        visited: &mut BTreeSet<usize>,
        path: &mut Vec<String>,
    ) -> bool {
        path.push(self.nodes[current].module.clone());
        visited.insert(current);
        for next in self.outgoing[current]
            .iter()
            .copied()
            .filter(|next| members.contains(next))
        {
            if next == target {
                path.push(self.nodes[target].module.clone());
                return true;
            }
            if !visited.contains(&next)
                && self.find_closed_path(next, target, members, visited, path)
            {
                return true;
            }
        }
        path.pop();
        false
    }
}

#[derive(Clone, Debug)]
struct SimilarityCandidate {
    declaration: SimilarityDeclarationMetric,
    tokens: Vec<NormalizedToken>,
    fingerprint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct NormalizedToken {
    kind: TokenKind,
    text: String,
}

fn similarity_instances(
    project: &Project,
    selected_paths: &BTreeSet<String>,
    min_tokens: usize,
) -> (Vec<SimilarityInstanceMetric>, usize) {
    let candidates = similarity_candidates(project, selected_paths)
        .into_iter()
        .filter(|candidate| candidate.tokens.len() >= min_tokens)
        .collect::<Vec<_>>();
    let fingerprint_count = candidates.len();
    (
        similarity_instances_from_candidates(candidates),
        fingerprint_count,
    )
}

fn similarity_instances_from_candidates(
    candidates: Vec<SimilarityCandidate>,
) -> Vec<SimilarityInstanceMetric> {
    let mut by_fingerprint = BTreeMap::<String, Vec<SimilarityCandidate>>::new();
    for candidate in candidates {
        by_fingerprint
            .entry(candidate.fingerprint.clone())
            .or_default()
            .push(candidate);
    }

    let mut instances = Vec::new();
    for candidates in by_fingerprint.into_values() {
        let mut by_tokens = Vec::<Vec<SimilarityCandidate>>::new();
        for candidate in candidates {
            if let Some(group) = by_tokens.iter_mut().find(|group| {
                group
                    .first()
                    .is_some_and(|first| first.tokens == candidate.tokens)
            }) {
                group.push(candidate);
            } else {
                by_tokens.push(vec![candidate]);
            }
        }
        for mut group in by_tokens {
            if group.len() < 2 {
                continue;
            }
            group.sort_by(compare_similarity_candidates);
            let fingerprint = group[0].fingerprint.clone();
            let token_count = group[0].tokens.len();
            let declarations = group
                .into_iter()
                .map(|candidate| candidate.declaration)
                .collect::<Vec<_>>();
            instances.push(SimilarityInstanceMetric {
                identity: format!("similarity:{fingerprint}"),
                fingerprint,
                token_count,
                experimental: true,
                declarations,
            });
        }
    }
    instances.sort_by(compare_similarity_instances);
    instances
}

fn similarity_candidates(
    project: &Project,
    selected_paths: &BTreeSet<String>,
) -> Vec<SimilarityCandidate> {
    let mut candidates = Vec::new();
    for source in &project.files {
        let path = source.path().as_str().to_string();
        if !selected_paths.contains(&path) || is_generated_or_doctest_path(&path) {
            continue;
        }
        let parsed = parse(source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        let lexed = lex(source);
        for item in parsed.tree.items {
            let SyntaxItem::Function(function) = item else {
                continue;
            };
            let Some(body_range) = function_body_range(&function) else {
                continue;
            };
            let tokens = normalized_body_tokens(&lexed.tokens, body_range);
            if tokens.is_empty() {
                continue;
            }
            let declaration = similarity_declaration(source, &path, &function, body_range);
            let fingerprint = similarity_fingerprint(&tokens);
            candidates.push(SimilarityCandidate {
                declaration,
                tokens,
                fingerprint,
            });
        }
    }
    candidates
}

fn function_body_range(function: &FunctionDecl) -> Option<TextRange> {
    function
        .body
        .iter()
        .map(body_line_range)
        .reduce(TextRange::cover)
}

fn body_line_range(line: &BodyLine) -> TextRange {
    let span = match line {
        BodyLine::Let { span, .. } | BodyLine::Expr { span, .. } => span,
    };
    TextRange::new(span.start.offset, span.end.offset)
}

fn normalized_body_tokens(tokens: &[Token], range: TextRange) -> Vec<NormalizedToken> {
    tokens
        .iter()
        .filter(|token| token.range.start >= range.start && token.range.end <= range.end)
        .filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Comment | TokenKind::Newline | TokenKind::Eof
            )
        })
        .map(|token| NormalizedToken {
            kind: token.kind,
            text: token.text.clone(),
        })
        .collect()
}

fn similarity_declaration(
    source: &SourceFile,
    path: &str,
    function: &FunctionDecl,
    body_range: TextRange,
) -> SimilarityDeclarationMetric {
    let kind = match function.kind {
        FunctionKind::Function => AbcSubjectKind::Function,
        FunctionKind::Test => AbcSubjectKind::Test,
    };
    let name = function
        .name
        .clone()
        .unwrap_or_else(|| "<anonymous>".to_string());
    SimilarityDeclarationMetric {
        identity: format!("{path}::{name}"),
        path: path.to_string(),
        name,
        kind,
        generated: false,
        span: source.span(TextRange::new(
            function.span.start.offset,
            function.span.end.offset,
        )),
        body_span: source.span(body_range),
    }
}

fn similarity_fingerprint(tokens: &[NormalizedToken]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for token in tokens {
        hash = fnv1a(hash, &[token.kind as u8]);
        hash = fnv1a(hash, &[0]);
        hash = fnv1a(hash, token.text.as_bytes());
        hash = fnv1a(hash, &[0xff]);
    }
    format!("{hash:016x}")
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn compare_similarity_candidates(
    left: &SimilarityCandidate,
    right: &SimilarityCandidate,
) -> std::cmp::Ordering {
    compare_similarity_declarations(&left.declaration, &right.declaration)
}

fn compare_similarity_declarations(
    left: &SimilarityDeclarationMetric,
    right: &SimilarityDeclarationMetric,
) -> std::cmp::Ordering {
    left.path
        .cmp(&right.path)
        .then_with(|| left.span.start.offset.cmp(&right.span.start.offset))
        .then_with(|| left.kind.as_str().cmp(right.kind.as_str()))
}

fn compare_similarity_instances(
    left: &SimilarityInstanceMetric,
    right: &SimilarityInstanceMetric,
) -> std::cmp::Ordering {
    right
        .token_count
        .cmp(&left.token_count)
        .then_with(|| {
            compare_similarity_declarations(&left.declarations[0], &right.declarations[0])
        })
        .then_with(|| left.fingerprint.cmp(&right.fingerprint))
}

fn abc_subjects(project: &Project, selected_paths: &BTreeSet<String>) -> Vec<AbcSubjectMetric> {
    let mut subjects = Vec::new();
    for source in &project.files {
        let path = source.path().as_str().to_string();
        if !selected_paths.contains(&path) {
            continue;
        }
        if is_generated_or_doctest_path(&path) {
            continue;
        }
        let parsed = parse(source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        for item in parsed.tree.items {
            let SyntaxItem::Function(function) = item else {
                continue;
            };
            subjects.push(abc_subject(source, &path, &function));
        }
    }
    subjects.sort_by(compare_abc_subjects);
    subjects
}

fn abc_subject(source: &SourceFile, path: &str, function: &FunctionDecl) -> AbcSubjectMetric {
    let vector = abc_vector(function);
    let kind = match function.kind {
        FunctionKind::Function => AbcSubjectKind::Function,
        FunctionKind::Test => AbcSubjectKind::Test,
    };
    let name = function
        .name
        .clone()
        .unwrap_or_else(|| "<anonymous>".to_string());
    AbcSubjectMetric {
        identity: format!("{path}::{name}"),
        path: path.to_string(),
        name,
        kind,
        vector,
        magnitude: vector.magnitude(),
        contracts_included: false,
        generated: false,
        span: source.span(veln_source::TextRange::new(
            function.span.start.offset,
            function.span.end.offset,
        )),
    }
}

fn is_generated_or_doctest_path(path: &str) -> bool {
    path.contains("#doctest-") || path.split('/').any(|segment| segment == "target")
}

fn abc_contract_subject_count(project: &Project, selected_paths: &BTreeSet<String>) -> usize {
    let mut count = 0;
    for source in &project.files {
        let path = source.path().as_str().to_string();
        if !selected_paths.contains(&path) || is_generated_or_doctest_path(&path) {
            continue;
        }
        let parsed = parse(source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        count += parsed
            .tree
            .items
            .into_iter()
            .filter_map(|item| match item {
                SyntaxItem::Function(function) => Some(function),
                _ => None,
            })
            .filter(|function| !function.contracts.is_empty())
            .count();
    }
    count
}

fn abc_vector(function: &FunctionDecl) -> AbcVector {
    let mut vector = AbcVector::default();
    for line in &function.body {
        match line {
            BodyLine::Let { expr, .. } => {
                vector.assignments += 1;
                count_expr(expr, &mut vector);
            }
            BodyLine::Expr { expr, .. } => count_expr(expr, &mut vector),
        }
    }
    vector
}

fn count_expr(expr: &Expr, vector: &mut AbcVector) {
    match &expr.kind {
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
        ExprKind::TypeApply { callee, .. } => count_expr(callee, vector),
        ExprKind::Call { callee, args } => {
            vector.branches += 1;
            count_expr(callee, vector);
            for arg in args {
                count_expr(arg, vector);
            }
        }
        ExprKind::Perform { args, .. } => {
            vector.branches += 1;
            for arg in args {
                count_expr(arg, vector);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            vector.branches += 1;
            count_expr(body, vector);
            for arg in args {
                count_expr(arg, vector);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            vector.branches += 1;
            count_expr(input, vector);
            count_expr(base, vector);
        }
        ExprKind::SchemaEncode { value, .. } => {
            vector.branches += 1;
            count_expr(value, vector);
        }
        ExprKind::FieldAccess { base, .. } | ExprKind::Try(base) => {
            if matches!(expr.kind, ExprKind::Try(_)) {
                vector.conditionals += 1;
            }
            count_expr(base, vector);
        }
        ExprKind::Record(fields) => {
            for field in fields {
                count_expr(&field.expr, vector);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                count_expr(&entry.key, vector);
                count_expr(&entry.value, vector);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                count_expr(item, vector);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            vector.conditionals += 1 + arms.len();
            count_expr(scrutinee, vector);
            for arm in arms {
                count_expr(&arm.expr, vector);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            vector.conditionals += 1 + else_if_branches.len();
            count_expr(condition, vector);
            count_expr(then_branch, vector);
            for branch in else_if_branches {
                count_expr(&branch.condition, vector);
                count_expr(&branch.expr, vector);
            }
            count_expr(else_branch, vector);
        }
        ExprKind::Prefix { expr, .. } => count_expr(expr, vector),
        ExprKind::Binary { op, left, right } => {
            if matches!(op, BinaryOp::And | BinaryOp::Or) {
                vector.conditionals += 1;
            }
            count_expr(left, vector);
            count_expr(right, vector);
        }
    }
}

fn compare_abc_subjects(left: &AbcSubjectMetric, right: &AbcSubjectMetric) -> std::cmp::Ordering {
    right
        .magnitude
        .partial_cmp(&left.magnitude)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.span.start.offset.cmp(&right.span.start.offset))
        .then_with(|| left.kind.as_str().cmp(right.kind.as_str()))
}

fn compare_module_metrics(left: &ModuleMetric, right: &ModuleMetric) -> std::cmp::Ordering {
    right
        .dependency_pressure
        .cmp(&left.dependency_pressure)
        .then_with(|| right.fan_out.cmp(&left.fan_out))
        .then_with(|| right.fan_in.cmp(&left.fan_in))
        .then_with(|| left.module.cmp(&right.module))
}

struct Tarjan<'a> {
    edges: &'a [BTreeSet<usize>],
    index: usize,
    stack: Vec<usize>,
    on_stack: BTreeSet<usize>,
    indices: Vec<Option<usize>>,
    lowlinks: Vec<usize>,
    components: Vec<Vec<usize>>,
}

impl<'a> Tarjan<'a> {
    fn new(edges: &'a [BTreeSet<usize>]) -> Self {
        Self {
            edges,
            index: 0,
            stack: Vec::new(),
            on_stack: BTreeSet::new(),
            indices: vec![None; edges.len()],
            lowlinks: vec![0; edges.len()],
            components: Vec::new(),
        }
    }

    fn components(&mut self) -> Vec<Vec<usize>> {
        for node in 0..self.edges.len() {
            if self.indices[node].is_none() {
                self.visit(node);
            }
        }
        std::mem::take(&mut self.components)
    }

    fn visit(&mut self, node: usize) {
        self.indices[node] = Some(self.index);
        self.lowlinks[node] = self.index;
        self.index += 1;
        self.stack.push(node);
        self.on_stack.insert(node);

        for next in &self.edges[node] {
            if self.indices[*next].is_none() {
                self.visit(*next);
                self.lowlinks[node] = self.lowlinks[node].min(self.lowlinks[*next]);
            } else if self.on_stack.contains(next) {
                self.lowlinks[node] = self.lowlinks[node]
                    .min(self.indices[*next].expect("stack members always have indices"));
            }
        }

        if self.lowlinks[node] == self.indices[node].expect("visited node has index") {
            let mut component = Vec::new();
            loop {
                let member = self.stack.pop().expect("component root is on the stack");
                self.on_stack.remove(&member);
                component.push(member);
                if member == node {
                    break;
                }
            }
            self.components.push(component);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HumanOutputBudget {
    limit: usize,
    total: usize,
    shown: usize,
}

impl HumanOutputBudget {
    fn for_report(report: &MetricsReport) -> Self {
        Self {
            limit: report.human_output_max_findings,
            total: detailed_report_finding_count(report),
            shown: 0,
        }
    }

    fn for_check(check: &MetricsCheckReport) -> Self {
        Self {
            limit: check.report.human_output_max_findings,
            total: detailed_check_finding_count(check),
            shown: 0,
        }
    }

    fn allow(&mut self) -> bool {
        if self.shown < self.limit {
            self.shown += 1;
            true
        } else {
            false
        }
    }

    fn omitted(self) -> usize {
        self.total.saturating_sub(self.shown)
    }

    fn append_summary(self, out: &mut String) {
        let omitted = self.omitted();
        if omitted > 0 {
            out.push_str(&format!(
                "\nDetailed findings omitted: {omitted}; use veln metrics --json for complete evidence.\n"
            ));
        }
    }
}

struct ReportHumanSelection {
    modules: Vec<bool>,
    cycles: Vec<bool>,
    abc_subjects: Vec<bool>,
    similarities: Vec<bool>,
}

fn select_report_findings(
    report: &MetricsReport,
    budget: &mut HumanOutputBudget,
) -> ReportHumanSelection {
    let cycles = report.cycles.iter().map(|_| budget.allow()).collect();
    let modules = report.modules.iter().map(|_| budget.allow()).collect();
    let abc_subjects = report.abc_subjects.iter().map(|_| budget.allow()).collect();
    let similarities = report.similarities.iter().map(|_| budget.allow()).collect();
    ReportHumanSelection {
        modules,
        cycles,
        abc_subjects,
        similarities,
    }
}

fn detailed_report_finding_count(report: &MetricsReport) -> usize {
    report.modules.len()
        + report.cycles.len()
        + report.abc_subjects.len()
        + report.similarities.len()
}

fn detailed_check_finding_count(check: &MetricsCheckReport) -> usize {
    check.violations.len() + detailed_report_finding_count(&check.report)
}

pub fn render_human(report: &MetricsReport) -> String {
    let mut budget = HumanOutputBudget::for_report(report);
    let selection = select_report_findings(report, &mut budget);
    let mut out = render_human_with_selection(report, &selection);
    budget.append_summary(&mut out);
    out
}

fn render_human_with_selection(report: &MetricsReport, selection: &ReportHumanSelection) -> String {
    let mut out = String::new();
    out.push_str("Veln dependency metrics (advisory)\n");
    out.push_str(&format!(
        "project modules: {}, selected modules: {}, internal edges: {}, cycles: {}, external dependencies: {}, ABC subjects: {}, ABC contract subjects: {}, similarity fingerprints: {}, similarity instances: {}, similarity regions: {}\n\n",
        report.summary.project_module_count,
        report.summary.selected_module_count,
        report.summary.internal_edge_count,
        report.summary.cycle_count,
        report.summary.external_dependency_count,
        report.summary.abc_subject_count,
        report.summary.abc_contract_subject_count,
        report.summary.similarity_fingerprint_count,
        report.summary.similarity_instance_count,
        report.summary.similarity_region_count
    ));
    out.push_str("Cycles\n");
    if report.cycles.is_empty() {
        out.push_str("  none\n");
    } else {
        for (cycle, selected) in report.cycles.iter().zip(&selection.cycles) {
            if !selected {
                continue;
            }
            out.push_str(&format!(
                "  {} | path: {}\n",
                cycle.members.join(", "),
                cycle.path.join(" -> ")
            ));
        }
    }
    out.push_str("\nModules\n");
    if report.modules.is_empty() {
        out.push_str("  no project modules selected\n");
    } else {
        for (module, selected) in report.modules.iter().zip(&selection.modules) {
            if !selected {
                continue;
            }
            out.push_str(&format!(
                "  {} ({}) fan-in={} fan-out={} pressure={} external={}\n",
                module.module,
                module.path,
                module.fan_in,
                module.fan_out,
                module.dependency_pressure,
                module.external_dependency_count
            ));
        }
    }
    out.push_str("\nABC size\n");
    if report.abc_subjects.is_empty() {
        out.push_str("  no function or test subjects selected\n");
    } else {
        for (subject, selected) in report.abc_subjects.iter().zip(&selection.abc_subjects) {
            if !selected {
                continue;
            }
            out.push_str(&format!(
                "  {} ({}) {} ABC size={:.1} vector=({}, {}, {}) contracts_included={}\n",
                subject.identity,
                subject.path,
                subject.kind.as_str(),
                subject.magnitude,
                subject.vector.assignments,
                subject.vector.branches,
                subject.vector.conditionals,
                subject.contracts_included
            ));
        }
    }
    out.push_str("\nWhole-body similarity (experimental)\n");
    out.push_str("  Similarity is advisory; it never creates a metrics policy violation.\n");
    out.push_str("  Review repeated bodies manually; the report does not prescribe automatic deduplication.\n");
    if report.similarities.is_empty() {
        out.push_str("  none\n");
    } else {
        for (instance, selected) in report.similarities.iter().zip(&selection.similarities) {
            if !selected {
                continue;
            }
            let primary = &instance.declarations[0];
            out.push_str(&format!(
                "  {} token_count={} fingerprint={} primary={} at {} body {}\n",
                instance.identity,
                instance.token_count,
                instance.fingerprint,
                primary.identity,
                span_label(&primary.span),
                span_label(&primary.body_span)
            ));
            for declaration in instance.declarations.iter().skip(1) {
                out.push_str(&format!(
                    "    related: {} at {} body {}\n",
                    declaration.identity,
                    span_label(&declaration.span),
                    span_label(&declaration.body_span)
                ));
            }
        }
    }
    out
}

fn span_label(span: &SourceSpan) -> String {
    format!(
        "{}:{}:{}-{}:{}",
        span.file.as_str(),
        span.start.line,
        span.start.column,
        span.end.line,
        span.end.column
    )
}

pub fn render_check_human(check: &MetricsCheckReport) -> String {
    let mut budget = HumanOutputBudget::for_check(check);
    let selected_violations = check
        .violations
        .iter()
        .map(|_| budget.allow())
        .collect::<Vec<_>>();
    let report_selection = select_report_findings(&check.report, &mut budget);
    let mut out = String::new();
    out.push_str("Veln dependency metrics (check)\n");
    out.push_str("policy checks: deny_cycles\n");
    if let Some(baseline) = &check.baseline {
        out.push_str(&format!("baseline: {}\n", baseline.path));
        if baseline.stale_subjects.is_empty() {
            out.push_str("baseline stale subjects: none\n");
        } else {
            out.push_str(&format!(
                "baseline stale subjects: {}\n",
                baseline.stale_subjects.join(", ")
            ));
        }
    }
    if check.violations.is_empty() {
        out.push_str("policy result: pass\n\n");
    } else {
        out.push_str("policy result: fail\n\n");
        out.push_str("Policy violations\n");
        for (violation, selected) in check.violations.iter().zip(&selected_violations) {
            if !selected {
                continue;
            }
            out.push_str(&format!(
                "  {}: dependency cycle path: {}\n",
                violation.policy,
                violation.path.join(" -> ")
            ));
            out.push_str(&format!(
                "    members: {}; review module ownership and dependency direction\n",
                violation.cycle_members.join(", ")
            ));
        }
        out.push('\n');
    }
    out.push_str(&render_human_with_selection(
        &check.report,
        &report_selection,
    ));
    budget.append_summary(&mut out);
    out
}

pub fn report_to_json(report: &MetricsReport, tool: ToolInfo) -> JsonValue {
    JsonValue::object(metrics_report_json_entries(report, tool, "ok", None, true))
}

pub fn baseline_to_json(report: &MetricsReport, tool: ToolInfo) -> JsonValue {
    let mut entries = metrics_report_json_entries(report, tool, "ok", None, false);
    replace_json_entry(
        &mut entries,
        "schema_version",
        JsonValue::string(BASELINE_SCHEMA_VERSION),
    );
    entries.insert(1, ("metric_model", JsonValue::string(METRIC_MODEL_VERSION)));
    JsonValue::object(entries)
}

pub fn baseline_from_json(source: &str) -> Result<MetricsBaseline, Vec<Diagnostic>> {
    let value = parse_json_value(source).map_err(|error| {
        vec![metrics_policy_diagnostic(
            "metrics.baseline.invalid_json",
            format!("metrics baseline is not valid JSON: {error}"),
            None,
            JsonValue::object([("phase", JsonValue::string("baseline"))]),
        )]
    })?;
    parse_baseline_value(&value)
}

pub fn report_check_to_json(check: &MetricsCheckReport, tool: ToolInfo) -> JsonValue {
    let status = if check.violations.is_empty() {
        "ok"
    } else {
        "policy_violation"
    };
    JsonValue::object(metrics_report_json_entries(
        &check.report,
        tool,
        status,
        Some(check_to_json(check)),
        true,
    ))
}

fn metrics_report_json_entries(
    report: &MetricsReport,
    tool: ToolInfo,
    status: &str,
    check: Option<JsonValue>,
    include_human_output: bool,
) -> Vec<(&'static str, JsonValue)> {
    let mut entries = vec![
        ("schema_version", JsonValue::string(JSON_SCHEMA_VERSION)),
        (
            "tool",
            JsonValue::object([
                ("name", JsonValue::string(tool.name)),
                ("version", JsonValue::string(tool.version)),
            ]),
        ),
        ("command", JsonValue::string("metrics")),
        ("status", JsonValue::string(status)),
        (
            "project",
            JsonValue::object([
                ("root", JsonValue::string(report.project.root.clone())),
                (
                    "selected_paths",
                    JsonValue::array(
                        report
                            .project
                            .selected_paths
                            .iter()
                            .map(|path| JsonValue::string(path.clone())),
                    ),
                ),
            ]),
        ),
        (
            "modules",
            JsonValue::array(report.modules.iter().map(module_to_json)),
        ),
        (
            "edges",
            JsonValue::array(report.edges.iter().map(edge_to_json)),
        ),
        (
            "cycles",
            JsonValue::array(report.cycles.iter().map(cycle_to_json)),
        ),
        (
            "abc_subjects",
            JsonValue::array(report.abc_subjects.iter().map(abc_subject_to_json)),
        ),
        (
            "similarities",
            JsonValue::array(report.similarities.iter().map(similarity_to_json)),
        ),
        ("summary", summary_to_json(&report.summary)),
    ];
    if include_human_output {
        entries.push(("human_output", human_output_to_json(report, check.as_ref())));
    }
    if let Some(check) = check {
        entries.push(("check", check));
    }
    entries
}

fn human_output_to_json(report: &MetricsReport, check: Option<&JsonValue>) -> JsonValue {
    let policy_violation_count = match check {
        Some(JsonValue::Object(entries)) => entries
            .iter()
            .find_map(|(key, value)| (key == "violations").then_some(value))
            .and_then(|value| match value {
                JsonValue::Array(values) => Some(values.len()),
                _ => None,
            })
            .unwrap_or(0),
        _ => 0,
    };
    let total_findings = policy_violation_count + detailed_report_finding_count(report);
    let omitted_findings = total_findings.saturating_sub(report.human_output_max_findings);
    JsonValue::object([
        (
            "max_findings",
            JsonValue::Number(usize_to_json_number(report.human_output_max_findings)),
        ),
        (
            "total_findings",
            JsonValue::Number(usize_to_json_number(total_findings)),
        ),
        (
            "omitted_findings",
            JsonValue::Number(usize_to_json_number(omitted_findings)),
        ),
        ("truncated", JsonValue::Bool(omitted_findings > 0)),
    ])
}

fn usize_to_json_number(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn max_json_usize() -> usize {
    usize::try_from(i64::MAX).unwrap_or(usize::MAX)
}

fn check_to_json(check: &MetricsCheckReport) -> JsonValue {
    let mut entries = vec![
        ("mode", JsonValue::string("check")),
        (
            "enabled_policies",
            JsonValue::array(
                check
                    .policy
                    .deny_cycles
                    .then(|| JsonValue::string("deny_cycles")),
            ),
        ),
        (
            "result",
            JsonValue::string(if check.violations.is_empty() {
                "pass"
            } else {
                "fail"
            }),
        ),
        (
            "violations",
            JsonValue::array(check.violations.iter().map(policy_violation_to_json)),
        ),
    ];
    if let Some(baseline) = &check.baseline {
        entries.push(("baseline", baseline_comparison_to_json(baseline)));
    }
    JsonValue::object(entries)
}

fn replace_json_entry(
    entries: &mut Vec<(&'static str, JsonValue)>,
    key: &'static str,
    value: JsonValue,
) {
    if let Some((_, existing)) = entries.iter_mut().find(|(entry_key, _)| *entry_key == key) {
        *existing = value;
    } else {
        entries.push((key, value));
    }
}

fn baseline_comparison_to_json(baseline: &BaselineComparison) -> JsonValue {
    JsonValue::object([
        ("path", JsonValue::string(baseline.path.clone())),
        ("schema_version", JsonValue::string(BASELINE_SCHEMA_VERSION)),
        ("metric_model", JsonValue::string(METRIC_MODEL_VERSION)),
        (
            "stale_subjects",
            JsonValue::array(
                baseline
                    .stale_subjects
                    .iter()
                    .map(|subject| JsonValue::string(subject.clone())),
            ),
        ),
    ])
}

fn policy_violation_to_json(violation: &MetricsPolicyViolation) -> JsonValue {
    JsonValue::object([
        ("policy", JsonValue::string(violation.policy.clone())),
        (
            "cycle_members",
            JsonValue::array(
                violation
                    .cycle_members
                    .iter()
                    .map(|member| JsonValue::string(member.clone())),
            ),
        ),
        (
            "path",
            JsonValue::array(
                violation
                    .path
                    .iter()
                    .map(|member| JsonValue::string(member.clone())),
            ),
        ),
        (
            "guidance",
            JsonValue::string("review module ownership and dependency direction"),
        ),
    ])
}

fn module_to_json(module: &ModuleMetric) -> JsonValue {
    JsonValue::object([
        ("module", JsonValue::string(module.module.clone())),
        ("path", JsonValue::string(module.path.clone())),
        ("generated", JsonValue::Bool(module.generated)),
        ("fan_in", JsonValue::Number(module.fan_in as i64)),
        ("fan_out", JsonValue::Number(module.fan_out as i64)),
        (
            "dependency_pressure",
            JsonValue::Number(module.dependency_pressure as i64),
        ),
        (
            "external_dependency_count",
            JsonValue::Number(module.external_dependency_count as i64),
        ),
        ("span", span_to_json(&module.span)),
    ])
}

fn edge_to_json(edge: &DependencyEdge) -> JsonValue {
    JsonValue::object([
        ("source", JsonValue::string(edge.source.clone())),
        ("target", JsonValue::string(edge.target.clone())),
        ("span", span_to_json(&edge.span)),
    ])
}

fn cycle_to_json(cycle: &DependencyCycle) -> JsonValue {
    JsonValue::object([
        (
            "members",
            JsonValue::array(
                cycle
                    .members
                    .iter()
                    .map(|member| JsonValue::string(member.clone())),
            ),
        ),
        (
            "path",
            JsonValue::array(
                cycle
                    .path
                    .iter()
                    .map(|member| JsonValue::string(member.clone())),
            ),
        ),
    ])
}

fn abc_subject_to_json(subject: &AbcSubjectMetric) -> JsonValue {
    JsonValue::object([
        ("identity", JsonValue::string(subject.identity.clone())),
        ("path", JsonValue::string(subject.path.clone())),
        ("name", JsonValue::string(subject.name.clone())),
        ("kind", JsonValue::string(subject.kind.as_str())),
        ("generated", JsonValue::Bool(subject.generated)),
        (
            "contracts_included",
            JsonValue::Bool(subject.contracts_included),
        ),
        (
            "abc",
            JsonValue::object([
                (
                    "assignments",
                    JsonValue::Number(subject.vector.assignments as i64),
                ),
                (
                    "branches",
                    JsonValue::Number(subject.vector.branches as i64),
                ),
                (
                    "conditionals",
                    JsonValue::Number(subject.vector.conditionals as i64),
                ),
                (
                    "magnitude",
                    JsonValue::string(format!("{:.15}", subject.magnitude)),
                ),
            ]),
        ),
        ("span", span_to_json(&subject.span)),
    ])
}

fn similarity_to_json(instance: &SimilarityInstanceMetric) -> JsonValue {
    JsonValue::object([
        ("identity", JsonValue::string(instance.identity.clone())),
        (
            "fingerprint",
            JsonValue::string(instance.fingerprint.clone()),
        ),
        (
            "token_count",
            JsonValue::Number(instance.token_count as i64),
        ),
        ("experimental", JsonValue::Bool(instance.experimental)),
        (
            "declarations",
            JsonValue::array(
                instance
                    .declarations
                    .iter()
                    .map(similarity_declaration_to_json),
            ),
        ),
    ])
}

fn similarity_declaration_to_json(declaration: &SimilarityDeclarationMetric) -> JsonValue {
    JsonValue::object([
        ("identity", JsonValue::string(declaration.identity.clone())),
        ("path", JsonValue::string(declaration.path.clone())),
        ("name", JsonValue::string(declaration.name.clone())),
        ("kind", JsonValue::string(declaration.kind.as_str())),
        ("generated", JsonValue::Bool(declaration.generated)),
        ("span", span_to_json(&declaration.span)),
        ("body_span", span_to_json(&declaration.body_span)),
    ])
}

fn summary_to_json(summary: &MetricsSummary) -> JsonValue {
    JsonValue::object([
        (
            "selected_module_count",
            JsonValue::Number(summary.selected_module_count as i64),
        ),
        (
            "project_module_count",
            JsonValue::Number(summary.project_module_count as i64),
        ),
        (
            "internal_edge_count",
            JsonValue::Number(summary.internal_edge_count as i64),
        ),
        ("cycle_count", JsonValue::Number(summary.cycle_count as i64)),
        (
            "external_dependency_count",
            JsonValue::Number(summary.external_dependency_count as i64),
        ),
        (
            "abc_subject_count",
            JsonValue::Number(summary.abc_subject_count as i64),
        ),
        (
            "abc_contract_subject_count",
            JsonValue::Number(summary.abc_contract_subject_count as i64),
        ),
        (
            "similarity_fingerprint_count",
            JsonValue::Number(summary.similarity_fingerprint_count as i64),
        ),
        (
            "similarity_instance_count",
            JsonValue::Number(summary.similarity_instance_count as i64),
        ),
        (
            "similarity_region_count",
            JsonValue::Number(summary.similarity_region_count as i64),
        ),
    ])
}

fn parse_baseline_value(value: &JsonValue) -> Result<MetricsBaseline, Vec<Diagnostic>> {
    let schema_version = json_string_field(value, "schema_version");
    if schema_version != Some(BASELINE_SCHEMA_VERSION) {
        return Err(vec![metrics_policy_diagnostic(
            "metrics.baseline.unsupported_schema",
            format!(
                "unsupported metrics baseline schema `{}`",
                schema_version.unwrap_or("<missing>")
            ),
            None,
            JsonValue::object([
                (
                    "expected",
                    JsonValue::string(BASELINE_SCHEMA_VERSION.to_string()),
                ),
                (
                    "actual",
                    schema_version.map_or(JsonValue::Null, JsonValue::string),
                ),
            ]),
        )]);
    }
    let metric_model = json_string_field(value, "metric_model");
    if metric_model != Some(METRIC_MODEL_VERSION) {
        return Err(vec![metrics_policy_diagnostic(
            "metrics.baseline.unsupported_metric_model",
            format!(
                "unsupported metrics baseline metric model `{}`",
                metric_model.unwrap_or("<missing>")
            ),
            None,
            JsonValue::object([
                ("expected", JsonValue::string(METRIC_MODEL_VERSION)),
                (
                    "actual",
                    metric_model.map_or(JsonValue::Null, JsonValue::string),
                ),
            ]),
        )]);
    }

    Ok(MetricsBaseline {
        modules: json_array_field(value, "modules")
            .unwrap_or(&[])
            .iter()
            .filter_map(parse_baseline_module)
            .collect(),
        edges: json_array_field(value, "edges")
            .unwrap_or(&[])
            .iter()
            .filter_map(parse_baseline_edge)
            .collect(),
        cycles: json_array_field(value, "cycles")
            .unwrap_or(&[])
            .iter()
            .filter_map(parse_baseline_cycle)
            .collect(),
    })
}

fn parse_baseline_module(value: &JsonValue) -> Option<BaselineModule> {
    Some(BaselineModule {
        module: json_string_field(value, "module")?.to_string(),
        path: json_string_field(value, "path")?.to_string(),
    })
}

fn parse_baseline_edge(value: &JsonValue) -> Option<BaselineEdge> {
    Some(BaselineEdge {
        source: json_string_field(value, "source")?.to_string(),
        target: json_string_field(value, "target")?.to_string(),
    })
}

fn parse_baseline_cycle(value: &JsonValue) -> Option<BaselineCycle> {
    Some(BaselineCycle {
        members: json_array_field(value, "members")?
            .iter()
            .filter_map(json_string)
            .map(str::to_string)
            .collect(),
    })
}

fn json_string_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    json_object_field(value, key).and_then(json_string)
}

fn json_array_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a [JsonValue]> {
    match json_object_field(value, key)? {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }
}

fn json_object_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find_map(|(field, value)| (field == key).then_some(value))
}

fn json_string(value: &JsonValue) -> Option<&str> {
    match value {
        JsonValue::String(value) => Some(value),
        _ => None,
    }
}

fn span_to_json(span: &SourceSpan) -> JsonValue {
    JsonValue::object([
        ("file", JsonValue::string(span.file.as_str())),
        (
            "start",
            JsonValue::object([
                ("line", JsonValue::Number(span.start.line as i64)),
                ("column", JsonValue::Number(span.start.column as i64)),
                ("offset", JsonValue::Number(span.start.offset as i64)),
            ]),
        ),
        (
            "end",
            JsonValue::object([
                ("line", JsonValue::Number(span.end.line as i64)),
                ("column", JsonValue::Number(span.end.column as i64)),
                ("offset", JsonValue::Number(span.end.offset as i64)),
            ]),
        ),
    ])
}

fn metrics_io_diagnostic(message: String) -> Diagnostic {
    Diagnostic {
        id: "metrics.discovery".to_string(),
        severity: Severity::Error,
        kind: veln_diagnostics::DiagnosticKind::Module,
        message,
        span: None,
        details: JsonValue::object([("phase", JsonValue::string("metrics"))]),
        related: Vec::new(),
    }
}

fn metrics_policy_diagnostic(
    id: &str,
    message: String,
    span: Option<SourceSpan>,
    details: JsonValue,
) -> Diagnostic {
    Diagnostic {
        id: id.to_string(),
        severity: Severity::Error,
        kind: veln_diagnostics::DiagnosticKind::Module,
        message,
        span,
        details,
        related: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veln_project::{ManifestLib, ManifestPackage, ManifestTool};

    #[test]
    fn counts_internal_edges_external_imports_and_self_cycles() {
        let project = Project {
            root: ".".into(),
            manifest: None,
            files: vec![
                SourceFile::new(
                    "app.veln",
                    "use util\nuse vendor from \"example/pkg\"\nfn main() -> ()\n  ()\nend\n",
                ),
                SourceFile::new(
                    "util.veln",
                    "use app\nuse util\nfn value() -> ()\n  ()\nend\n",
                ),
            ],
        };

        let graph = DependencyGraph::from_project(&project).expect("graph");
        let selected = ["app.veln".to_string(), "util.veln".to_string()]
            .into_iter()
            .collect();
        let report = graph.report(
            &project,
            ProjectIdentity {
                root: ".".to_string(),
                selected_paths: Vec::new(),
            },
            &selected,
            default_metrics_config(),
        );

        assert_eq!(report.summary.internal_edge_count, 3);
        assert_eq!(report.summary.external_dependency_count, 1);
        assert_eq!(report.summary.cycle_count, 1);
        assert_eq!(report.cycles[0].path.first().unwrap(), "app");
        assert_eq!(report.cycles[0].path.last().unwrap(), "app");
        assert_eq!(report.modules[0].module, "util");
        assert_eq!(report.modules[0].dependency_pressure, 4);
    }

    #[test]
    fn counts_abc_constructs_from_function_bodies() {
        let cases = [
            (
                "let binding",
                "fn subject() -> Int\n  let value = 1\n  value\nend\n",
                AbcVector {
                    assignments: 1,
                    branches: 0,
                    conditionals: 0,
                },
            ),
            (
                "call",
                "fn subject() -> Int\n  add(1, 2)\nend\n",
                AbcVector {
                    assignments: 0,
                    branches: 1,
                    conditionals: 0,
                },
            ),
            (
                "perform",
                "fn subject() -> Int effects [Console]\n  perform Console::read()\nend\n",
                AbcVector {
                    assignments: 0,
                    branches: 1,
                    conditionals: 0,
                },
            ),
            (
                "handler application",
                "fn subject() -> Int effects [Ask]\n  handle perform Ask::value() with ask(1)\nend\n\neffect Ask\n  value() -> Int\nend\n\nhandler ask(context: Int) handles Ask\n  value() => provide(context)\nend\n",
                AbcVector {
                    assignments: 0,
                    branches: 2,
                    conditionals: 0,
                },
            ),
            (
                "schema decode and try",
                "fn subject() -> DecodeStep<{value: Int}>\n  decode Packet from view()? at offset()?\nend\n",
                AbcVector {
                    assignments: 0,
                    branches: 3,
                    conditionals: 2,
                },
            ),
            (
                "schema encode",
                "fn subject() -> ByteChunk\n  encode Packet from {value: make_value()}\nend\n",
                AbcVector {
                    assignments: 0,
                    branches: 2,
                    conditionals: 0,
                },
            ),
            (
                "if else-if and short circuit",
                "fn subject() -> Int\n  if left() and right()\n    1\n  else if fallback() or other()\n    2\n  else\n    3\n  end\nend\n",
                AbcVector {
                    assignments: 0,
                    branches: 4,
                    conditionals: 4,
                },
            ),
            (
                "match arms",
                "fn subject() -> Int\n  match value()\n    0 => one()\n    _ => two()\n  end\nend\n",
                AbcVector {
                    assignments: 0,
                    branches: 3,
                    conditionals: 3,
                },
            ),
            (
                "nested expressions",
                "fn subject() -> Int\n  let value = outer(if ready()\n    inner(1)\n  else\n    inner(2)\n  end)\n  value?\nend\n",
                AbcVector {
                    assignments: 1,
                    branches: 4,
                    conditionals: 2,
                },
            ),
        ];

        for (name, source, expected) in cases {
            assert_eq!(first_function_vector(source), expected, "{name}");
        }
    }

    #[test]
    fn excludes_annotations_contracts_and_preserves_subject_kind_identity() {
        let project = Project {
            root: ".".into(),
            manifest: None,
            files: vec![SourceFile::new(
                "app.veln",
                "fn same(value: Int) -> Result<Int, String>\n  ensure value >= 0\n  value\nend\n\ntest same() -> Result<Int, String>\n  require perform Console::read() == 1\n  let value: Int = compute()?\n  value\nend\n",
            )],
        };
        let selected = ["app.veln".to_string()].into_iter().collect();
        let subjects = abc_subjects(&project, &selected);

        assert_eq!(subjects.len(), 2);
        assert_eq!(subjects[0].kind, AbcSubjectKind::Test);
        assert_eq!(
            subjects[0].vector,
            AbcVector {
                assignments: 1,
                branches: 1,
                conditionals: 1,
            }
        );
        assert!(!subjects[0].contracts_included);
        assert_eq!(subjects[1].kind, AbcSubjectKind::Function);
        assert_eq!(subjects[1].vector, AbcVector::default());
        assert_eq!(subjects[0].identity, "app.veln::same");
        assert_eq!(subjects[1].identity, "app.veln::same");
        assert_eq!(abc_contract_subject_count(&project, &selected), 2);
    }

    #[test]
    fn excludes_generated_and_doctest_derived_abc_subjects() {
        let project = Project {
            root: ".".into(),
            manifest: None,
            files: vec![
                SourceFile::new("app.veln", "fn source() -> Int\n  call()\nend\n"),
                SourceFile::new(
                    "target/generated.veln",
                    "fn generated() -> Int\n  call()\nend\n",
                ),
                SourceFile::new(
                    "app.veln#doctest-1_test.veln",
                    "test doctest_1() -> Int\n  call()\nend\n",
                ),
            ],
        };
        let selected = [
            "app.veln".to_string(),
            "target/generated.veln".to_string(),
            "app.veln#doctest-1_test.veln".to_string(),
        ]
        .into_iter()
        .collect();
        let subjects = abc_subjects(&project, &selected);

        assert_eq!(subjects.len(), 1);
        assert_eq!(subjects[0].identity, "app.veln::source");
    }

    #[test]
    fn render_human_reports_abc_summary_counts() {
        let project = Project {
            root: ".".into(),
            manifest: None,
            files: vec![SourceFile::new(
                "app.veln",
                "fn plain() -> Int\n  call()\nend\n\nfn guarded(value: Int) -> Int\n  ensure value >= 0\n  value\nend\n",
            )],
        };
        let selected = ["app.veln".to_string()].into_iter().collect();
        let graph = DependencyGraph::from_project(&project).expect("graph");
        let report = graph.report(
            &project,
            ProjectIdentity {
                root: ".".to_string(),
                selected_paths: vec!["app.veln".to_string()],
            },
            &selected,
            default_metrics_config(),
        );
        let human = render_human(&report);

        assert!(human.contains("ABC subjects: 2, ABC contract subjects: 1"));
    }

    #[test]
    fn reads_metrics_cycle_policy_from_manifest_table() {
        let cases = [
            ("omitted", None, Ok(false)),
            ("false", Some("false"), Ok(false)),
            ("true", Some("true"), Ok(true)),
            ("invalid", Some("yes"), Err("metrics.policy.invalid_value")),
        ];

        for (name, value, expected) in cases {
            let manifest = value.map(|value| metrics_manifest(&[("deny_cycles", value)]));
            let actual = read_metrics_policy(manifest.as_ref());
            match expected {
                Ok(deny_cycles) => {
                    assert_eq!(actual.expect(name).deny_cycles, deny_cycles, "{name}");
                }
                Err(id) => {
                    let diagnostics = actual.expect_err(name);
                    assert_eq!(diagnostics[0].id, id, "{name}");
                    assert_eq!(
                        diagnostics[0].span.as_ref().unwrap().file.as_str(),
                        "veln.toml"
                    );
                }
            }
        }
    }

    #[test]
    fn reads_metrics_human_output_limit_from_manifest_table() {
        let cases = [
            ("omitted", None, Ok(DEFAULT_HUMAN_OUTPUT_MAX_FINDINGS)),
            ("valid", Some("5"), Ok(5)),
            ("zero", Some("0"), Err("metrics.policy.invalid_value")),
            (
                "malformed",
                Some("many"),
                Err("metrics.policy.invalid_value"),
            ),
            (
                "overflow",
                Some("184467440737095516160"),
                Err("metrics.policy.invalid_value"),
            ),
        ];

        for (name, value, expected) in cases {
            let manifest = value.map(|value| metrics_manifest(&[("max_findings", value)]));
            let actual = read_metrics_config(manifest.as_ref());
            match expected {
                Ok(max_findings) => {
                    assert_eq!(
                        actual.expect(name).human_output_max_findings,
                        max_findings,
                        "{name}"
                    );
                }
                Err(id) => {
                    let diagnostics = actual.expect_err(name);
                    assert_eq!(diagnostics[0].id, id, "{name}");
                    assert_eq!(
                        diagnostics[0].span.as_ref().unwrap().file.as_str(),
                        "veln.toml"
                    );
                }
            }
        }

        if usize::MAX > max_json_usize() {
            let manifest = metrics_manifest(&[("max_findings", "9223372036854775808")]);
            let diagnostics =
                read_metrics_config(Some(&manifest)).expect_err("above JSON number maximum");
            assert_eq!(diagnostics[0].id, "metrics.policy.invalid_value");
            assert_eq!(
                diagnostics[0].span.as_ref().unwrap().file.as_str(),
                "veln.toml"
            );
        }
    }

    #[test]
    fn rejects_unknown_metrics_policy_fields() {
        let manifest = metrics_manifest(&[("deny_cycles", "true"), ("unknown", "5")]);
        let diagnostics = read_metrics_policy(Some(&manifest)).expect_err("unsupported field");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "metrics.policy.unsupported_field");
        assert_eq!(
            diagnostics[0].span.as_ref().unwrap().file.as_str(),
            "veln.toml"
        );
    }

    #[test]
    fn reports_all_invalid_metrics_config_fields_in_manifest_order() {
        let manifest = metrics_manifest(&[
            ("deny_cycles", "yes"),
            ("similarity_min_tokens", "0"),
            ("max_findings", "many"),
            ("unknown", "5"),
        ]);

        let diagnostics = read_metrics_config(Some(&manifest)).expect_err("invalid fields");

        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "metrics.policy.invalid_value",
                "metrics.policy.invalid_value",
                "metrics.policy.invalid_value",
                "metrics.policy.unsupported_field",
            ]
        );
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.file.as_str() == "veln.toml")
        }));
        assert!(diagnostics[0].message.contains("`deny_cycles`"));
        assert!(diagnostics[1].message.contains("`similarity_min_tokens`"));
        assert!(diagnostics[2].message.contains("`max_findings`"));
        assert!(diagnostics[3].message.contains("`unknown`"));
    }

    #[test]
    fn render_human_truncates_stable_cross_section_prefix() {
        let mut report = report_from_edges(&[
            ("app", "util"),
            ("util", "app"),
            ("zeta", "app"),
            ("alpha", "app"),
        ]);
        report.human_output_max_findings = 3;

        let human = render_human(&report);

        assert!(human.contains("app (app.veln) fan-in=3 fan-out=1 pressure=3 external=0"));
        assert!(human.contains("util (util.veln) fan-in=1 fan-out=1 pressure=1 external=0"));
        assert!(!human.contains("alpha (alpha.veln) fan-in=0 fan-out=1 pressure=0 external=0"));
        assert!(!human.contains("zeta (zeta.veln) fan-in=0 fan-out=1 pressure=0 external=0"));
        assert!(human.contains("app, util | path: app -> util -> app"));
        assert_before(&human, "Cycles\n", "\nModules\n");
        assert!(human.contains(
            "Detailed findings omitted: 6; use veln metrics --json for complete evidence."
        ));
    }

    #[test]
    fn render_human_keeps_similarity_related_lines_at_truncation_boundary() {
        let project = Project {
            root: ".".into(),
            manifest: None,
            files: vec![SourceFile::new(
                "app.veln",
                "fn first() -> Int\n  let value = add(1, 2)\n  let other = add(value, 3)\n  other\nend\n\nfn second() -> Int\n  let value = add(1, 2)\n  let other = add(value, 3)\n  other\nend\n\nfn third() -> Int\n  let value = add(4, 5)\n  value\nend\n\nfn fourth() -> Int\n  let value = add(4, 5)\n  value\nend\n",
            )],
        };
        let selected = ["app.veln".to_string()].into_iter().collect();
        let graph = DependencyGraph::from_project(&project).expect("graph");
        let mut report = graph.report(
            &project,
            ProjectIdentity {
                root: ".".to_string(),
                selected_paths: vec!["app.veln".to_string()],
            },
            &selected,
            MetricsConfig {
                similarity_min_tokens: 8,
                ..default_metrics_config()
            },
        );
        report.human_output_max_findings = 6;

        let human = render_human(&report);

        assert!(human.contains("primary=app.veln::first"));
        assert!(human.contains("related: app.veln::second"));
        assert!(!human.contains("primary=app.veln::third"));
        assert!(!human.contains("related: app.veln::fourth"));
        assert!(human.contains(
            "Detailed findings omitted: 1; use veln metrics --json for complete evidence."
        ));
    }

    #[test]
    fn render_check_human_spends_budget_on_policy_violations_first() {
        let mut report = report_from_edges(&[("app", "util"), ("util", "app")]);
        report.human_output_max_findings = 1;
        let check = evaluate_metrics_check(report, MetricsPolicy { deny_cycles: true });

        let human = render_check_human(&check);

        assert!(human.contains("policy result: fail"));
        assert!(human.contains("deny_cycles: dependency cycle path: app -> util -> app"));
        assert!(!human.contains("app (app.veln) fan-in=1 fan-out=1 pressure=1 external=0"));
        assert!(human.contains(
            "Detailed findings omitted: 5; use veln metrics --json for complete evidence."
        ));
    }

    #[test]
    fn report_json_exposes_human_output_projection_metadata() {
        let mut report = report_from_edges(&[("app", "util"), ("util", "app")]);
        report.human_output_max_findings = 2;

        let json = report_to_json(&report, tool_info()).to_json();

        assert!(json.contains("\"human_output\":{\"max_findings\":2"));
        assert!(json.contains("\"total_findings\":5"));
        assert!(json.contains("\"omitted_findings\":3"));
        assert!(json.contains("\"truncated\":true"));
        assert!(json.contains("\"modules\":["));
        assert!(json.contains("\"cycles\":["));
        assert!(json.contains("\"abc_subjects\":["));
    }

    #[test]
    fn checked_json_counts_policy_violations_in_human_output_projection() {
        let mut report = report_from_edges(&[("app", "util"), ("util", "app")]);
        report.human_output_max_findings = 1;
        let check = evaluate_metrics_check(report, MetricsPolicy { deny_cycles: true });

        let json = report_check_to_json(&check, tool_info()).to_json();

        assert!(json.contains("\"human_output\":{\"max_findings\":1"));
        assert!(json.contains("\"total_findings\":6"));
        assert!(json.contains("\"omitted_findings\":5"));
        assert!(json.contains("\"status\":\"policy_violation\""));
        assert!(json.contains("\"violations\":["));
    }

    #[test]
    fn evaluates_cycle_policy_table_and_preserves_report_data() {
        let cases = [
            (
                "acyclic pass",
                "fn value() -> ()\n  ()\nend\n",
                0,
                false,
                "\"status\":\"ok\"",
            ),
            (
                "cycle violation",
                "use app\nfn value() -> ()\n  ()\nend\n",
                1,
                true,
                "\"status\":\"policy_violation\"",
            ),
        ];

        for (name, util_source, expected_cycles, expected_violation, expected_status) in cases {
            let project = Project {
                root: ".".into(),
                manifest: None,
                files: vec![
                    SourceFile::new("app.veln", "use util\nfn main() -> ()\n  ()\nend\n"),
                    SourceFile::new("util.veln", util_source),
                ],
            };
            let selected = ["app.veln".to_string(), "util.veln".to_string()]
                .into_iter()
                .collect();
            let graph = DependencyGraph::from_project(&project).expect(name);
            let report = graph.report(
                &project,
                ProjectIdentity {
                    root: ".".to_string(),
                    selected_paths: vec!["app.veln".to_string(), "util.veln".to_string()],
                },
                &selected,
                default_metrics_config(),
            );

            let check = evaluate_metrics_check(report, MetricsPolicy { deny_cycles: true });

            assert_eq!(check.has_violations(), expected_violation, "{name}");
            assert_eq!(check.report.summary.cycle_count, expected_cycles, "{name}");
            assert_eq!(check.report.modules.len(), 2, "{name}");
            assert!(
                report_check_to_json(&check, ToolInfo::new("veln", "0.1.0"))
                    .to_json()
                    .contains(expected_status),
                "{name}"
            );

            if expected_violation {
                assert_eq!(check.violations[0].policy, "deny_cycles", "{name}");
                assert_eq!(
                    check.violations[0].path.first().map(String::as_str),
                    Some("app"),
                    "{name}"
                );
                assert_eq!(
                    check.violations[0].path.last().map(String::as_str),
                    Some("app"),
                    "{name}"
                );
                assert!(
                    render_check_human(&check).contains("review module ownership"),
                    "{name}"
                );
            } else {
                assert!(
                    render_check_human(&check).contains("policy result: pass"),
                    "{name}"
                );
            }
        }
    }

    #[test]
    fn evaluates_cycle_policy_only_for_selected_cycle_subjects() {
        let cases = [
            (
                "unselected cycle is advisory only",
                vec![
                    SourceFile::new("app.veln", "use util\nfn main() -> ()\n  ()\nend\n"),
                    SourceFile::new("util.veln", "use app\nfn value() -> ()\n  ()\nend\n"),
                    SourceFile::new("entry.veln", "fn entry() -> ()\n  ()\nend\n"),
                ],
                vec!["entry.veln"],
                0,
                false,
            ),
            (
                "selected mutual cycle fails",
                vec![
                    SourceFile::new("app.veln", "use util\nfn main() -> ()\n  ()\nend\n"),
                    SourceFile::new("util.veln", "use app\nfn value() -> ()\n  ()\nend\n"),
                    SourceFile::new("entry.veln", "fn entry() -> ()\n  ()\nend\n"),
                ],
                vec!["app.veln"],
                1,
                true,
            ),
            (
                "selected self cycle fails",
                vec![SourceFile::new(
                    "self.veln",
                    "use self\nfn value() -> ()\n  ()\nend\n",
                )],
                vec!["self.veln"],
                1,
                true,
            ),
        ];

        for (name, files, selected, expected_cycles, expected_violation) in cases {
            let project = Project {
                root: ".".into(),
                manifest: None,
                files,
            };
            let selected = selected
                .into_iter()
                .map(str::to_string)
                .collect::<BTreeSet<_>>();
            let graph = DependencyGraph::from_project(&project).expect(name);
            let report = graph.report(
                &project,
                ProjectIdentity {
                    root: ".".to_string(),
                    selected_paths: selected.iter().cloned().collect(),
                },
                &selected,
                default_metrics_config(),
            );

            let check = evaluate_metrics_check(report, MetricsPolicy { deny_cycles: true });

            assert_eq!(check.report.summary.cycle_count, expected_cycles, "{name}");
            assert_eq!(check.has_violations(), expected_violation, "{name}");
        }
    }

    #[test]
    fn evaluates_cycle_policy_against_baseline_regressions() {
        let cases = [
            (
                "unchanged cycle is allowed",
                vec![("app", "util"), ("util", "app")],
                vec![vec!["app", "util"]],
                vec![("app", "util"), ("util", "app")],
                false,
            ),
            (
                "cycle that loses an edge is allowed",
                vec![
                    ("app", "util"),
                    ("util", "app"),
                    ("util", "core"),
                    ("core", "app"),
                ],
                vec![vec!["app", "core", "util"]],
                vec![("app", "util"), ("util", "app")],
                false,
            ),
            (
                "cycle with same members that loses only a cyclic edge is allowed",
                vec![
                    ("app", "util"),
                    ("app", "core"),
                    ("util", "core"),
                    ("core", "app"),
                ],
                vec![vec!["app", "core", "util"]],
                vec![("app", "util"), ("util", "core"), ("core", "app")],
                false,
            ),
            (
                "cycle that adds an edge fails",
                vec![("app", "util"), ("util", "app")],
                vec![vec!["app", "util"]],
                vec![("app", "util"), ("util", "core"), ("core", "app")],
                true,
            ),
            (
                "cycle with same members that adds only a cyclic edge fails",
                vec![("app", "util"), ("util", "core"), ("core", "app")],
                vec![vec!["app", "core", "util"]],
                vec![
                    ("app", "util"),
                    ("util", "core"),
                    ("core", "app"),
                    ("core", "util"),
                ],
                true,
            ),
            (
                "renamed cycle fails",
                vec![("app", "util"), ("util", "app")],
                vec![vec!["app", "util"]],
                vec![("renamed", "util"), ("util", "renamed")],
                true,
            ),
            (
                "new self cycle fails",
                vec![("app", "util"), ("util", "app")],
                vec![vec!["app", "util"]],
                vec![("selfish", "selfish")],
                true,
            ),
        ];

        for (name, baseline_edges, baseline_cycles, current_edges, expected_violation) in cases {
            let report = report_from_edges(&current_edges);
            let baseline = baseline_from_edges(&baseline_edges, &baseline_cycles);
            let check = evaluate_metrics_check_with_baseline(
                report,
                MetricsPolicy { deny_cycles: true },
                baseline,
                "metrics.baseline.json".to_string(),
            );

            assert_eq!(check.has_violations(), expected_violation, "{name}");
        }
    }

    #[test]
    fn reports_stale_baseline_subjects_without_failing() {
        let report = report_from_edges(&[("app", "util"), ("util", "app")]);
        let baseline = MetricsBaseline {
            modules: vec![
                BaselineModule {
                    module: "app".to_string(),
                    path: "app.veln".to_string(),
                },
                BaselineModule {
                    module: "util".to_string(),
                    path: "util.veln".to_string(),
                },
                BaselineModule {
                    module: "deleted".to_string(),
                    path: "deleted.veln".to_string(),
                },
            ],
            edges: vec![
                BaselineEdge {
                    source: "app".to_string(),
                    target: "util".to_string(),
                },
                BaselineEdge {
                    source: "util".to_string(),
                    target: "app".to_string(),
                },
            ],
            cycles: vec![BaselineCycle {
                members: vec!["app".to_string(), "util".to_string()],
            }],
        };

        let check = evaluate_metrics_check_with_baseline(
            report,
            MetricsPolicy { deny_cycles: true },
            baseline,
            "metrics.baseline.json".to_string(),
        );

        assert!(!check.has_violations());
        assert_eq!(
            check.baseline.unwrap().stale_subjects,
            vec!["deleted".to_string()]
        );
    }

    #[test]
    fn parses_baseline_schema_and_metric_model_versions() {
        let report = report_from_edges(&[("app", "util"), ("util", "app")]);
        let json = baseline_to_json(&report, ToolInfo::new("veln", "0.1.0")).to_json();
        let baseline = baseline_from_json(&json).expect("baseline should parse");

        assert_eq!(baseline.modules.len(), 2);
        assert_eq!(baseline.cycles.len(), 1);

        let unsupported_schema =
            json.replace(BASELINE_SCHEMA_VERSION, "veln-metrics-baseline/v999");
        assert_eq!(
            baseline_from_json(&unsupported_schema).unwrap_err()[0].id,
            "metrics.baseline.unsupported_schema"
        );

        let unsupported_model = json.replace(METRIC_MODEL_VERSION, "veln-metrics-model/v999");
        assert_eq!(
            baseline_from_json(&unsupported_model).unwrap_err()[0].id,
            "metrics.baseline.unsupported_metric_model"
        );
    }

    #[test]
    fn canonical_path_ordering_survives_source_insertion_order_and_separators() {
        let forward = stable_ordering_project(vec![
            SourceFile::new("nested\\util.veln", STABLE_UTIL_SOURCE),
            SourceFile::new("nested/app.veln", STABLE_APP_SOURCE),
        ]);
        let reversed = stable_ordering_project(vec![
            SourceFile::new("nested\\app.veln", STABLE_APP_SOURCE),
            SourceFile::new("nested/util.veln", STABLE_UTIL_SOURCE),
        ]);
        let selected = [
            "nested/app.veln".to_string(),
            "nested/util.veln".to_string(),
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();

        let forward_report = metrics_report_from_project(&forward, &selected);
        let reversed_report = metrics_report_from_project(&reversed, &selected);
        let forward_json = report_to_json(&forward_report, tool_info()).to_json();
        let reversed_json = report_to_json(&reversed_report, tool_info()).to_json();
        let forward_human = render_human(&forward_report);
        let reversed_human = render_human(&reversed_report);
        let baseline_json = baseline_to_json(&forward_report, tool_info()).to_json();

        assert_eq!(forward_json, reversed_json);
        assert_eq!(forward_human, reversed_human);
        assert!(!forward_json.contains('\\'));
        assert!(!forward_human.contains('\\'));
        assert!(!baseline_json.contains('\\'));
        assert_eq!(
            forward_report.project.selected_paths,
            ["nested/app.veln", "nested/util.veln"]
        );
        assert_eq!(
            forward_report
                .modules
                .iter()
                .map(|module| module.module.as_str())
                .collect::<Vec<_>>(),
            ["nested::app", "nested::util"]
        );
        assert_eq!(
            forward_report
                .edges
                .iter()
                .map(|edge| (edge.source.as_str(), edge.target.as_str()))
                .collect::<Vec<_>>(),
            [
                ("nested::app", "nested::util"),
                ("nested::util", "nested::app")
            ]
        );
        assert_eq!(
            forward_report
                .abc_subjects
                .iter()
                .map(|subject| subject.identity.as_str())
                .collect::<Vec<_>>(),
            [
                "nested/app.veln::duplicate_app",
                "nested/app.veln::variant_app",
                "nested/util.veln::duplicate_util",
                "nested/util.veln::variant_util",
                "nested/app.veln::add",
                "nested/util.veln::add"
            ]
        );
        assert_eq!(forward_report.similarities.len(), 2);
        assert_eq!(
            forward_report.similarities[0]
                .declarations
                .iter()
                .map(|declaration| declaration.identity.as_str())
                .collect::<Vec<_>>(),
            [
                "nested/app.veln::duplicate_app",
                "nested/util.veln::duplicate_util"
            ]
        );
        assert_eq!(
            forward_report.similarities[1]
                .declarations
                .iter()
                .map(|declaration| declaration.identity.as_str())
                .collect::<Vec<_>>(),
            [
                "nested/app.veln::variant_app",
                "nested/util.veln::variant_util"
            ]
        );
        assert_eq!(
            forward_report
                .similarities
                .iter()
                .map(|instance| instance.token_count)
                .collect::<Vec<_>>(),
            [19, 19]
        );
    }

    #[test]
    fn orders_abc_subjects_deterministically() {
        let project = Project {
            root: ".".into(),
            manifest: None,
            files: vec![
                SourceFile::new("b.veln", "fn small() -> Int\n  call()\nend\n"),
                SourceFile::new(
                    "a.veln",
                    "fn large() -> Int\n  let value = call()\n  if value == 1\n    call()\n  else\n    call()\n  end\nend\n",
                ),
            ],
        };
        let selected = ["a.veln".to_string(), "b.veln".to_string()]
            .into_iter()
            .collect();
        let subjects = abc_subjects(&project, &selected);

        assert_eq!(
            subjects
                .iter()
                .map(|subject| subject.identity.as_str())
                .collect::<Vec<_>>(),
            ["a.veln::large", "b.veln::small"]
        );
    }

    #[test]
    fn detects_similarity_from_table_driven_source_boundaries() {
        struct Case {
            name: &'static str,
            source: &'static str,
            min_tokens: usize,
            fingerprint_count: usize,
            groups: Vec<Vec<&'static str>>,
            token_counts: Vec<usize>,
        }

        let cases = [
            Case {
                name: "formatting and comments are ignored",
                source: "fn first() -> Int\n  # ignored\n  let value = add(1, 2)\n  value\nend\n\nfn second() -> Int\n  let value=add(\n    1,\n    2\n  )\n  value\nend\n",
                min_tokens: 8,
                fingerprint_count: 2,
                groups: vec![vec!["app.veln::first", "app.veln::second"]],
                token_counts: vec![10],
            },
            Case {
                name: "identifier spelling is significant",
                source: "fn first() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn renamed() -> Int\n  let other = add(1, 2)\n  other\nend\n",
                min_tokens: 8,
                fingerprint_count: 2,
                groups: vec![],
                token_counts: vec![],
            },
            Case {
                name: "literal spelling is significant",
                source: "fn one() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn two() -> Int\n  let value = add(1, 3)\n  value\nend\n",
                min_tokens: 8,
                fingerprint_count: 2,
                groups: vec![],
                token_counts: vec![],
            },
            Case {
                name: "only complete bodies match",
                source: "fn left() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn right() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn partial() -> Int\n  let value = add(1, 2)\n  value + 1\nend\n",
                min_tokens: 10,
                fingerprint_count: 3,
                groups: vec![vec!["app.veln::left", "app.veln::right"]],
                token_counts: vec![10],
            },
            Case {
                name: "minimum token boundary excludes short bodies",
                source: "fn left() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn right() -> Int\n  let value = add(1, 2)\n  value\nend\n",
                min_tokens: 11,
                fingerprint_count: 0,
                groups: vec![],
                token_counts: vec![],
            },
        ];

        for case in cases {
            let project = Project {
                root: ".".into(),
                manifest: None,
                files: vec![SourceFile::new("app.veln", case.source)],
            };
            let selected = ["app.veln".to_string()].into_iter().collect();
            let (instances, fingerprint_count) =
                similarity_instances(&project, &selected, case.min_tokens);
            let groups = instances
                .iter()
                .map(|instance| {
                    instance
                        .declarations
                        .iter()
                        .map(|declaration| declaration.identity.as_str())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let token_counts = instances
                .iter()
                .map(|instance| instance.token_count)
                .collect::<Vec<_>>();

            assert_eq!(fingerprint_count, case.fingerprint_count, "{}", case.name);
            assert_eq!(groups, case.groups, "{}", case.name);
            assert_eq!(token_counts, case.token_counts, "{}", case.name);
        }
    }

    #[test]
    fn groups_similarity_candidates_by_complete_tokens_after_fingerprint_matches() {
        let candidates = vec![
            similarity_candidate(
                "a.veln",
                "first",
                "same",
                vec![("Let", "let"), ("Ident", "a")],
            ),
            similarity_candidate(
                "b.veln",
                "second",
                "same",
                vec![("Let", "let"), ("Ident", "b")],
            ),
            similarity_candidate(
                "c.veln",
                "third",
                "same",
                vec![("Let", "let"), ("Ident", "a")],
            ),
            similarity_candidate(
                "d.veln",
                "fourth",
                "same",
                vec![("Let", "let"), ("Ident", "b")],
            ),
        ];
        let instances = similarity_instances_from_candidates(candidates);

        assert_eq!(instances.len(), 2);
        assert_eq!(
            instances
                .iter()
                .map(|instance| {
                    instance
                        .declarations
                        .iter()
                        .map(|declaration| declaration.identity.as_str())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>(),
            [
                vec!["a.veln::first", "c.veln::third"],
                vec!["b.veln::second", "d.veln::fourth"]
            ]
        );
    }

    #[test]
    fn orders_similarity_instances_and_respects_structural_bounds() {
        let candidates = vec![
            similarity_candidate(
                "ignored.veln",
                "unique_a",
                "unique-a",
                token_texts(&["u", "a"]),
            ),
            similarity_candidate(
                "ignored.veln",
                "unique_b",
                "unique-b",
                token_texts(&["u", "b"]),
            ),
            similarity_candidate(
                "z.veln",
                "large_z",
                "large",
                token_texts(&["l", "a", "r", "g", "e"]),
            ),
            similarity_candidate(
                "a.veln",
                "large_a",
                "large",
                token_texts(&["l", "a", "r", "g", "e"]),
            ),
            similarity_candidate(
                "m.veln",
                "large_m",
                "large",
                token_texts(&["l", "a", "r", "g", "e"]),
            ),
            similarity_candidate(
                "b.veln",
                "large_b",
                "large",
                token_texts(&["l", "a", "r", "g", "e"]),
            ),
            similarity_candidate(
                "dir\\z.veln",
                "pair_z",
                "pair-one",
                token_texts(&["p", "a", "i", "r"]),
            ),
            similarity_candidate(
                "dir/a.veln",
                "pair_a",
                "pair-one",
                token_texts(&["p", "a", "i", "r"]),
            ),
            similarity_candidate(
                "c.veln",
                "pair_c",
                "pair-two",
                token_texts(&["t", "w", "o", "2"]),
            ),
            similarity_candidate(
                "d.veln",
                "pair_d",
                "pair-two",
                token_texts(&["t", "w", "o", "2"]),
            ),
            similarity_candidate("e.veln", "pair_e", "pair-three", token_texts(&["o", "k"])),
            similarity_candidate("f.veln", "pair_f", "pair-three", token_texts(&["o", "k"])),
        ];
        let fingerprint_count = candidates.len();
        let instances = similarity_instances_from_candidates(candidates);
        let region_count = instances
            .iter()
            .map(|instance| instance.declarations.len())
            .sum::<usize>();

        assert_eq!(instances.len(), 4);
        assert_eq!(region_count, 10);
        assert!(region_count <= fingerprint_count);
        assert!(instances.len() <= fingerprint_count / 2);
        assert_eq!(
            instances
                .iter()
                .map(|instance| {
                    (
                        instance.token_count,
                        instance.declarations[0].identity.as_str(),
                        instance.declarations.len(),
                    )
                })
                .collect::<Vec<_>>(),
            [
                (5, "a.veln::large_a", 4),
                (4, "c.veln::pair_c", 2),
                (4, "dir/a.veln::pair_a", 2),
                (2, "e.veln::pair_e", 2)
            ]
        );

        let mut report = report_from_edges(&[]);
        report.summary.similarity_fingerprint_count = fingerprint_count;
        report.summary.similarity_instance_count = instances.len();
        report.summary.similarity_region_count = region_count;
        report.similarities = instances;

        let json = report_to_json(&report, tool_info()).to_json();
        let human = render_human(&report);

        assert_before(
            &json,
            "\"identity\":\"c.veln::pair_c\"",
            "\"identity\":\"dir/a.veln::pair_a\"",
        );
        assert_before(
            &human,
            "primary=c.veln::pair_c",
            "primary=dir/a.veln::pair_a",
        );
    }

    #[test]
    fn renders_similarity_fingerprint_tiebreak_order_in_public_outputs() {
        let instances = similarity_instances_from_candidates(vec![
            similarity_candidate(
                "same.veln",
                "primary",
                "fingerprint-b",
                token_texts(&["b", "b"]),
            ),
            similarity_candidate(
                "z.veln",
                "related_b",
                "fingerprint-b",
                token_texts(&["b", "b"]),
            ),
            similarity_candidate(
                "same.veln",
                "primary",
                "fingerprint-a",
                token_texts(&["a", "a"]),
            ),
            similarity_candidate(
                "z.veln",
                "related_a",
                "fingerprint-a",
                token_texts(&["a", "a"]),
            ),
        ]);
        let region_count = instances
            .iter()
            .map(|instance| instance.declarations.len())
            .sum::<usize>();
        let mut report = report_from_edges(&[]);
        report.summary.similarity_fingerprint_count = 4;
        report.summary.similarity_instance_count = instances.len();
        report.summary.similarity_region_count = region_count;
        report.similarities = instances;

        let json = report_to_json(&report, tool_info()).to_json();
        let human = render_human(&report);

        assert_before(
            &json,
            "\"fingerprint\":\"fingerprint-a\"",
            "\"fingerprint\":\"fingerprint-b\"",
        );
        assert_before(
            &human,
            "fingerprint=fingerprint-a",
            "fingerprint=fingerprint-b",
        );
    }

    #[test]
    fn generated_similarity_workload_preserves_pipeline_bounds() {
        let workload = GeneratedSimilarityWorkload {
            unrelated_count: 8,
            large_group_count: 5,
            pair_count: 6,
            prefix_count: 7,
            min_tokens: 8,
        };
        let project = Project {
            root: ".".into(),
            manifest: None,
            files: vec![SourceFile::new("app.veln", workload.source())],
        };
        let selected = ["app.veln".to_string()].into_iter().collect();
        let graph = DependencyGraph::from_project(&project).expect("graph");
        let report = graph.report(
            &project,
            ProjectIdentity {
                root: ".".to_string(),
                selected_paths: vec!["app.veln".to_string()],
            },
            &selected,
            MetricsConfig {
                similarity_min_tokens: workload.min_tokens,
                ..default_metrics_config()
            },
        );
        let eligible_declarations = workload.eligible_declaration_count();

        assert_eq!(
            report.summary.abc_subject_count, eligible_declarations,
            "the workload should exercise the full parsed function population"
        );
        assert_eq!(
            report.summary.similarity_fingerprint_count,
            eligible_declarations
        );
        assert_eq!(
            report.summary.similarity_instance_count,
            workload.expected_instance_count()
        );
        assert_eq!(
            report.summary.similarity_region_count,
            workload.expected_region_count()
        );
        assert!(report.summary.similarity_region_count <= eligible_declarations);
        assert!(report.summary.similarity_instance_count <= eligible_declarations / 2);

        let mut seen_declarations = BTreeSet::new();
        for instance in &report.similarities {
            for declaration in &instance.declarations {
                assert!(
                    seen_declarations.insert(declaration.identity.clone()),
                    "declaration reported in more than one similarity instance: {}",
                    declaration.identity
                );
            }
        }
    }

    fn first_function_vector(source: &str) -> AbcVector {
        let source = SourceFile::new("case.veln", source);
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let function = parsed
            .tree
            .items
            .into_iter()
            .find_map(|item| match item {
                SyntaxItem::Function(function) => Some(function),
                _ => None,
            })
            .expect("function");
        abc_vector(&function)
    }

    const STABLE_APP_SOURCE: &str = "use nested::util\n\nfn add(left: Int, right: Int) -> Int\n  left + right\nend\n\nfn duplicate_app() -> Int\n  let value = add(1, 2)\n  let other = add(value, 3)\n  other\nend\n\nfn variant_app() -> Int\n  let value = add(4, 5)\n  let other = add(value, 6)\n  other\nend\n";
    const STABLE_UTIL_SOURCE: &str = "use nested::app\n\nfn add(left: Int, right: Int) -> Int\n  left + right\nend\n\nfn duplicate_util() -> Int\n  let value = add(1, 2)\n  let other = add(value, 3)\n  other\nend\n\nfn variant_util() -> Int\n  let value = add(4, 5)\n  let other = add(value, 6)\n  other\nend\n";

    fn stable_ordering_project(files: Vec<SourceFile>) -> Project {
        Project {
            root: ".".into(),
            manifest: None,
            files,
        }
    }

    fn metrics_report_from_project(
        project: &Project,
        selected_paths: &BTreeSet<String>,
    ) -> MetricsReport {
        let graph = DependencyGraph::from_project(project).expect("graph");
        graph.report(
            project,
            ProjectIdentity {
                root: ".".to_string(),
                selected_paths: selected_paths.iter().cloned().collect(),
            },
            selected_paths,
            MetricsConfig {
                similarity_min_tokens: 8,
                ..default_metrics_config()
            },
        )
    }

    fn similarity_candidate(
        path: &str,
        name: &str,
        fingerprint: &str,
        tokens: Vec<(&str, &str)>,
    ) -> SimilarityCandidate {
        let source = SourceFile::new(path, "");
        SimilarityCandidate {
            declaration: SimilarityDeclarationMetric {
                identity: format!("{path}::{name}"),
                path: path.to_string(),
                name: name.to_string(),
                kind: AbcSubjectKind::Function,
                generated: false,
                span: source.span(TextRange::new(0, 0)),
                body_span: source.span(TextRange::new(0, 0)),
            },
            tokens: tokens
                .into_iter()
                .map(|(kind, text)| NormalizedToken {
                    kind: match kind {
                        "Let" => TokenKind::Let,
                        "Ident" => TokenKind::Ident,
                        _ => TokenKind::Invalid,
                    },
                    text: text.to_string(),
                })
                .collect(),
            fingerprint: fingerprint.to_string(),
        }
    }

    struct GeneratedSimilarityWorkload {
        unrelated_count: usize,
        large_group_count: usize,
        pair_count: usize,
        prefix_count: usize,
        min_tokens: usize,
    }

    impl GeneratedSimilarityWorkload {
        fn source(&self) -> String {
            let mut source = String::new();
            for index in 0..self.unrelated_count {
                push_function(&mut source, &format!("unrelated_{index}"), index);
            }
            for index in 0..self.large_group_count {
                push_repeated_function(&mut source, &format!("large_{index}"), 1000);
            }
            for pair in 0..self.pair_count {
                let seed = 2000 + pair;
                push_repeated_function(&mut source, &format!("pair_{pair}_left"), seed);
                push_repeated_function(&mut source, &format!("pair_{pair}_right"), seed);
            }
            for index in 0..self.prefix_count {
                push_prefixed_function(&mut source, &format!("prefix_{index}"), 3000 + index);
            }
            source
        }

        fn eligible_declaration_count(&self) -> usize {
            self.unrelated_count
                + self.large_group_count
                + (self.pair_count * 2)
                + self.prefix_count
        }

        fn expected_instance_count(&self) -> usize {
            usize::from(self.large_group_count >= 2) + self.pair_count
        }

        fn expected_region_count(&self) -> usize {
            (if self.large_group_count >= 2 {
                self.large_group_count
            } else {
                0
            }) + (self.pair_count * 2)
        }
    }

    fn push_function(source: &mut String, name: &str, seed: usize) {
        source.push_str(&format!(
            "fn {name}() -> Int\n  let seed = {seed}\n  let left = seed + {}\n  let right = left + {}\n  right\nend\n\n",
            seed + 1,
            seed + 2
        ));
    }

    fn push_repeated_function(source: &mut String, name: &str, seed: usize) {
        source.push_str(&format!(
            "fn {name}() -> Int\n  let seed = {seed}\n  let left = seed + 1\n  let right = left + 2\n  right\nend\n\n"
        ));
    }

    fn push_prefixed_function(source: &mut String, name: &str, seed: usize) {
        source.push_str(&format!(
            "fn {name}() -> Int\n  let seed = 4000\n  let left = seed + 1\n  let right = left + {seed}\n  right\nend\n\n"
        ));
    }

    fn token_texts(texts: &[&'static str]) -> Vec<(&'static str, &'static str)> {
        texts.iter().map(|text| ("Ident", *text)).collect()
    }

    fn tool_info() -> ToolInfo {
        ToolInfo::new("veln", "test")
    }

    fn report_from_edges(edges: &[(&str, &str)]) -> MetricsReport {
        let mut modules = edges
            .iter()
            .flat_map(|(source, target)| [*source, *target])
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|module| ModuleMetric {
                module: module.to_string(),
                path: format!("{module}.veln"),
                generated: false,
                fan_in: 0,
                fan_out: 0,
                dependency_pressure: 0,
                external_dependency_count: 0,
                span: SourceFile::new(format!("{module}.veln"), "")
                    .span(veln_source::TextRange::new(0, 0)),
            })
            .collect::<Vec<_>>();
        modules.sort_by(compare_module_metrics);
        let edges = edges
            .iter()
            .map(|(source, target)| DependencyEdge {
                source: (*source).to_string(),
                target: (*target).to_string(),
                span: SourceFile::new(format!("{source}.veln"), "")
                    .span(veln_source::TextRange::new(0, 0)),
            })
            .collect::<Vec<_>>();
        let graph_project = Project {
            root: ".".into(),
            manifest: None,
            files: modules
                .iter()
                .map(|module| {
                    SourceFile::new(
                        module.path.as_str(),
                        source_for_module(&module.module, &edges),
                    )
                })
                .collect(),
        };
        let graph = DependencyGraph::from_project(&graph_project).expect("graph");
        let selected = modules
            .iter()
            .map(|module| module.path.clone())
            .collect::<BTreeSet<_>>();
        graph.report(
            &graph_project,
            ProjectIdentity {
                root: ".".to_string(),
                selected_paths: selected.iter().cloned().collect(),
            },
            &selected,
            default_metrics_config(),
        )
    }

    fn source_for_module(module: &str, edges: &[DependencyEdge]) -> String {
        let mut source = String::new();
        for edge in edges.iter().filter(|edge| edge.source == module) {
            source.push_str(&format!("use {}\n", edge.target));
        }
        source.push_str(&format!("fn {}_value() -> ()\n  ()\nend\n", module));
        source
    }

    fn baseline_from_edges(edges: &[(&str, &str)], cycles: &[Vec<&str>]) -> MetricsBaseline {
        MetricsBaseline {
            modules: edges
                .iter()
                .flat_map(|(source, target)| [*source, *target])
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|module| BaselineModule {
                    module: module.to_string(),
                    path: format!("{module}.veln"),
                })
                .collect(),
            edges: edges
                .iter()
                .map(|(source, target)| BaselineEdge {
                    source: (*source).to_string(),
                    target: (*target).to_string(),
                })
                .collect(),
            cycles: cycles
                .iter()
                .map(|members| BaselineCycle {
                    members: members.iter().map(|member| (*member).to_string()).collect(),
                })
                .collect(),
        }
    }

    fn metrics_manifest(fields: &[(&str, &str)]) -> ProjectManifest {
        let mut text = String::from("[tool.metrics]\n");
        for (key, value) in fields {
            text.push_str(&format!("{key} = \"{value}\"\n"));
        }
        let source = SourceFile::new("veln.toml", &text);
        let mut offset = "[tool.metrics]\n".len();
        let fields = fields
            .iter()
            .map(|(key, value)| {
                let key_start = offset;
                let value_start = offset + key.len() + " = \"".len();
                offset += key.len() + " = \"".len() + value.len() + "\"\n".len();
                veln_project::ManifestField {
                    key: (*key).to_string(),
                    value: (*value).to_string(),
                    key_span: source.span(veln_source::TextRange::new(
                        key_start,
                        key_start + key.len(),
                    )),
                    value_span: source.span(veln_source::TextRange::new(
                        value_start,
                        value_start + value.len(),
                    )),
                }
            })
            .collect();

        ProjectManifest {
            path: source.path().clone(),
            package: ManifestPackage::default(),
            lib: ManifestLib {
                exports: Vec::new(),
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: vec![ManifestTool {
                name: "metrics".to_string(),
                fields,
            }],
        }
    }

    fn assert_before(haystack: &str, first: &str, second: &str) {
        let first_index = haystack.find(first).expect("first fragment");
        let second_index = haystack.find(second).expect("second fragment");
        assert!(
            first_index < second_index,
            "expected `{first}` before `{second}` in:\n{haystack}"
        );
    }
}
