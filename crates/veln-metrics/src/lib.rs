//! Report-only Veln source metrics.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use veln_analysis::{derive_source_module_path, load_surface_module};
use veln_diagnostics::{Diagnostic, JsonValue, Severity, ToolInfo};
use veln_project::{Project, discover_source_paths};
use veln_source::{SourceFile, SourceSpan};
use veln_syntax::parse;

pub const JSON_SCHEMA_VERSION: &str = "veln-metrics-json/v0";

#[derive(Clone, Debug)]
pub struct MetricsReport {
    pub project: ProjectIdentity,
    pub modules: Vec<ModuleMetric>,
    pub edges: Vec<DependencyEdge>,
    pub cycles: Vec<DependencyCycle>,
    pub summary: MetricsSummary,
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
pub struct MetricsSummary {
    pub selected_module_count: usize,
    pub project_module_count: usize,
    pub internal_edge_count: usize,
    pub cycle_count: usize,
    pub external_dependency_count: usize,
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
        ProjectIdentity {
            root: ".".to_string(),
            selected_paths: selected_paths.iter().cloned().collect(),
        },
        &selected_paths,
    ))
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

    fn report(self, project: ProjectIdentity, selected_paths: &BTreeSet<String>) -> MetricsReport {
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
        let summary = MetricsSummary {
            selected_module_count: modules.len(),
            project_module_count: self.nodes.len(),
            internal_edge_count: self.edges.len(),
            cycle_count: cycles.len(),
            external_dependency_count,
        };
        MetricsReport {
            project,
            modules,
            edges,
            cycles,
            summary,
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

pub fn render_human(report: &MetricsReport) -> String {
    let mut out = String::new();
    out.push_str("Veln dependency metrics (advisory)\n");
    out.push_str(&format!(
        "project modules: {}, selected modules: {}, internal edges: {}, cycles: {}, external dependencies: {}\n\n",
        report.summary.project_module_count,
        report.summary.selected_module_count,
        report.summary.internal_edge_count,
        report.summary.cycle_count,
        report.summary.external_dependency_count
    ));
    out.push_str("Modules\n");
    if report.modules.is_empty() {
        out.push_str("  no project modules selected\n");
    } else {
        for module in &report.modules {
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
    out.push_str("\nCycles\n");
    if report.cycles.is_empty() {
        out.push_str("  none\n");
    } else {
        for cycle in &report.cycles {
            out.push_str(&format!(
                "  {} | path: {}\n",
                cycle.members.join(", "),
                cycle.path.join(" -> ")
            ));
        }
    }
    out
}

pub fn report_to_json(report: &MetricsReport, tool: ToolInfo) -> JsonValue {
    JsonValue::object([
        ("schema_version", JsonValue::string(JSON_SCHEMA_VERSION)),
        (
            "tool",
            JsonValue::object([
                ("name", JsonValue::string(tool.name)),
                ("version", JsonValue::string(tool.version)),
            ]),
        ),
        ("command", JsonValue::string("metrics")),
        ("status", JsonValue::string("ok")),
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
        ("summary", summary_to_json(&report.summary)),
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
    ])
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

#[cfg(test)]
mod tests {
    use super::*;

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
            ProjectIdentity {
                root: ".".to_string(),
                selected_paths: Vec::new(),
            },
            &selected,
        );

        assert_eq!(report.summary.internal_edge_count, 3);
        assert_eq!(report.summary.external_dependency_count, 1);
        assert_eq!(report.summary.cycle_count, 1);
        assert_eq!(report.cycles[0].path.first().unwrap(), "app");
        assert_eq!(report.cycles[0].path.last().unwrap(), "app");
        assert_eq!(report.modules[0].module, "util");
        assert_eq!(report.modules[0].dependency_pressure, 4);
    }
}
