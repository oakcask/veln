use super::*;

pub(super) fn analyze_project_metrics_from_project(
    root: PathBuf,
    inputs: &[PathBuf],
    full_project: Project,
    config: MetricsConfig,
) -> Result<MetricsReport, Vec<Diagnostic>> {
    let project = project_owned_sources(full_project);
    let source_diagnostics = source_graph_diagnostics(&project);
    let partial = partial_source_analysis(&project, &source_diagnostics)?;

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
    let graph_project = if partial.completeness.is_partial() {
        project_without_paths(project.clone(), &partial.excluded_paths)
    } else {
        project.clone()
    };
    let graph = DependencyGraph::from_project(&graph_project)?;
    Ok(graph.report(
        &project,
        ProjectIdentity {
            root: ".".to_string(),
            selected_paths: selected_paths.iter().cloned().collect(),
        },
        &selected_paths,
        config,
        partial.diagnostics,
        partial.completeness,
    ))
}

#[derive(Debug)]
pub(super) struct PartialSourceAnalysis {
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) completeness: MetricsCompleteness,
    pub(super) excluded_paths: BTreeSet<String>,
}

pub(super) fn partial_source_analysis(
    project: &Project,
    diagnostics: &[Diagnostic],
) -> Result<PartialSourceAnalysis, Vec<Diagnostic>> {
    if !has_error(diagnostics) {
        return Ok(PartialSourceAnalysis {
            diagnostics: Vec::new(),
            completeness: MetricsCompleteness::default(),
            excluded_paths: BTreeSet::new(),
        });
    }

    let qualifying = diagnostics
        .iter()
        .filter(|diagnostic| is_source_path_invalid_case(diagnostic))
        .cloned()
        .collect::<Vec<_>>();
    if qualifying.is_empty() {
        return Err(diagnostics.to_vec());
    }

    let excluded_paths = qualifying
        .iter()
        .filter_map(|diagnostic| json_string_field(&diagnostic.details, "source_path"))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let excluded_modules = project
        .files
        .iter()
        .filter(|source| excluded_paths.contains(source.path().as_str()))
        .filter_map(invalid_case_rejected_visible_module_path)
        .collect::<BTreeSet<_>>();
    let project_modules = project
        .files
        .iter()
        .filter_map(source_visible_module_path)
        .collect::<BTreeSet<_>>();

    let disallowed = diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Error
            && !is_source_path_invalid_case(diagnostic)
            && !is_exclusion_caused_unresolved_import(
                diagnostic,
                &excluded_paths,
                &excluded_modules,
                &project_modules,
            )
    });
    if disallowed {
        return Err(diagnostics.to_vec());
    }

    Ok(PartialSourceAnalysis {
        diagnostics: qualifying,
        completeness: MetricsCompleteness {
            excluded_sources: excluded_paths
                .iter()
                .map(|path| ExcludedSource {
                    path: path.clone(),
                    reason: "invalid_module_identity".to_string(),
                })
                .collect(),
            excluded_baseline_subjects: Vec::new(),
        },
        excluded_paths,
    })
}

pub(super) fn project_without_paths(
    mut project: Project,
    excluded_paths: &BTreeSet<String>,
) -> Project {
    project
        .files
        .retain(|source| !excluded_paths.contains(source.path().as_str()));
    project
}

pub(super) fn is_source_path_invalid_case(diagnostic: &Diagnostic) -> bool {
    diagnostic.id == "name.invalid_case"
        && diagnostic.severity == Severity::Error
        && json_string_field(&diagnostic.details, "origin") == Some("source_path")
}

pub(super) fn is_exclusion_caused_unresolved_import(
    diagnostic: &Diagnostic,
    excluded_paths: &BTreeSet<String>,
    excluded_modules: &BTreeSet<String>,
    project_modules: &BTreeSet<String>,
) -> bool {
    if diagnostic.id != "module.unresolved_import" {
        return false;
    }
    let Some(module_path) = json_string_field(&diagnostic.details, "module_path") else {
        return false;
    };
    if excluded_modules.contains(module_path) {
        return true;
    }
    diagnostic
        .span
        .as_ref()
        .is_some_and(|span| excluded_paths.contains(span.file.as_str()))
        && project_modules.contains(module_path)
}

pub(super) fn source_visible_module_path(source: &SourceFile) -> Option<String> {
    derive_source_module_path(source)
        .ok()
        .or_else(|| invalid_case_rejected_visible_module_path(source))
}

pub(super) fn read_metrics_config(
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

pub(super) fn default_metrics_config() -> MetricsConfig {
    MetricsConfig {
        policy: MetricsPolicy { deny_cycles: false },
        similarity_min_tokens: DEFAULT_SIMILARITY_MIN_TOKENS,
        human_output_max_findings: DEFAULT_HUMAN_OUTPUT_MAX_FINDINGS,
    }
}

pub(super) fn parse_boolean_metrics_field(field: &ManifestField) -> Option<bool> {
    match field.value.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

pub(super) fn parse_positive_metrics_field(field: &ManifestField, maximum: usize) -> Option<usize> {
    field
        .value
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0 && *value <= maximum)
}

pub(super) fn apply_metrics_field<Value>(
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

pub(super) fn invalid_metrics_field_diagnostic(
    field: &ManifestField,
    allowed: JsonValue,
) -> Diagnostic {
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

pub(super) fn unsupported_metrics_field_diagnostic(field: &ManifestField) -> Diagnostic {
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
pub(super) fn read_metrics_policy(
    manifest: Option<&ProjectManifest>,
) -> Result<MetricsPolicy, Vec<Diagnostic>> {
    read_metrics_config(manifest).map(|config| config.policy)
}

pub(super) fn evaluate_metrics_check(
    report: MetricsReport,
    policy: MetricsPolicy,
) -> MetricsCheckReport {
    let violations = cycle_violations(&report, &policy, |_| true);
    MetricsCheckReport {
        report,
        policy,
        violations,
        baseline: None,
    }
}

pub(super) fn evaluate_metrics_check_with_baseline(
    mut report: MetricsReport,
    policy: MetricsPolicy,
    baseline: MetricsBaseline,
    baseline_path: String,
) -> MetricsCheckReport {
    let violations = cycle_violations(&report, &policy, |cycle| {
        !baseline_allows_cycle(&report, cycle, &baseline)
    });
    let current_subjects = report
        .modules
        .iter()
        .map(|module| module.module.as_str())
        .collect::<BTreeSet<_>>();
    let excluded_paths = report
        .completeness
        .excluded_sources
        .iter()
        .map(|source| source.path.as_str())
        .collect::<BTreeSet<_>>();
    let stale_subjects = baseline
        .modules
        .iter()
        .filter(|module| {
            !current_subjects.contains(module.module.as_str())
                && !excluded_paths.contains(module.path.as_str())
        })
        .map(|module| module.module.clone())
        .collect();
    let mut excluded_baseline_subjects: Vec<String> = baseline
        .modules
        .iter()
        .filter(|module| {
            !current_subjects.contains(module.module.as_str())
                && excluded_paths.contains(module.path.as_str())
        })
        .map(|module| module.module.clone())
        .collect();
    excluded_baseline_subjects.sort();
    report.completeness.excluded_baseline_subjects = excluded_baseline_subjects;
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

fn cycle_violations(
    report: &MetricsReport,
    policy: &MetricsPolicy,
    include: impl Fn(&DependencyCycle) -> bool,
) -> Vec<MetricsPolicyViolation> {
    if !policy.deny_cycles {
        return Vec::new();
    }
    report
        .cycles
        .iter()
        .filter(|cycle| include(cycle))
        .map(|cycle| MetricsPolicyViolation {
            policy: "deny_cycles".to_string(),
            cycle_members: cycle.members.clone(),
            path: cycle.path.clone(),
        })
        .collect()
}

pub(super) fn baseline_allows_cycle(
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

pub(super) fn cyclic_edges<Edge>(
    edges: &[Edge],
    members: &BTreeSet<String>,
) -> BTreeSet<BaselineEdge>
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

pub(super) trait DependencyEdgeLike {
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

pub(super) fn source_graph_diagnostics(project: &Project) -> Vec<Diagnostic> {
    let (_, diagnostics) = load_surface_module(project);
    diagnostics
}

pub(super) fn project_owned_sources(mut project: Project) -> Project {
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

pub(super) fn dependency_path_prefixes(project: &Project) -> Vec<String> {
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

pub(super) fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

pub(super) fn selected_source_paths(
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
