use super::*;

pub fn report_to_json(report: &MetricsReport, tool: ToolInfo) -> JsonValue {
    JsonValue::object(metrics_report_json_entries(
        report,
        tool,
        report_status(report),
        None,
        true,
    ))
}

pub fn baseline_to_json(report: &MetricsReport, tool: ToolInfo) -> JsonValue {
    let mut entries = metrics_report_json_entries(report, tool, "ok", None, false);
    replace_json_entry(
        &mut entries,
        "schema_version",
        JsonValue::string(BASELINE_SCHEMA_VERSION),
    );
    entries.retain(|(key, _)| *key != "diagnostics" && *key != "completeness");
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
    let status = if check.has_violations() {
        "policy_violation"
    } else if check.report.completeness.is_partial() {
        "incomplete"
    } else {
        "ok"
    };
    JsonValue::object(metrics_report_json_entries(
        &check.report,
        tool,
        status,
        Some(check_to_json(check)),
        true,
    ))
}

pub(super) fn metrics_report_json_entries(
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
            "diagnostics",
            JsonValue::array(report.diagnostics.iter().map(diagnostic_to_json)),
        ),
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
    if report.completeness.is_partial() {
        entries.push(("completeness", completeness_to_json(&report.completeness)));
    }
    if include_human_output {
        entries.push(("human_output", human_output_to_json(report, check.as_ref())));
    }
    if let Some(check) = check {
        entries.push(("check", check));
    }
    entries
}

pub(super) fn human_output_to_json(report: &MetricsReport, check: Option<&JsonValue>) -> JsonValue {
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
    let omitted_findings = policy_violation_count.saturating_sub(report.human_output_max_findings)
        + omitted_report_finding_count(report);
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

pub(super) fn usize_to_json_number(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(super) fn max_json_usize() -> usize {
    usize::try_from(i64::MAX).unwrap_or(usize::MAX)
}

pub(super) fn check_to_json(check: &MetricsCheckReport) -> JsonValue {
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
            JsonValue::string(if check.has_violations() {
                "fail"
            } else if check.report.completeness.is_partial() {
                "incomplete"
            } else {
                "pass"
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

pub(super) fn replace_json_entry(
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

pub(super) fn baseline_comparison_to_json(baseline: &BaselineComparison) -> JsonValue {
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

pub(super) fn report_status(report: &MetricsReport) -> &'static str {
    if report.completeness.is_partial() {
        "incomplete"
    } else {
        "ok"
    }
}

pub(super) fn completeness_to_json(completeness: &MetricsCompleteness) -> JsonValue {
    let mut entries = vec![
        ("status", JsonValue::string("partial")),
        (
            "excluded_sources",
            JsonValue::array(
                completeness
                    .excluded_sources
                    .iter()
                    .map(excluded_source_to_json),
            ),
        ),
    ];
    if !completeness.excluded_baseline_subjects.is_empty() {
        entries.push((
            "excluded_baseline_subjects",
            JsonValue::array(
                completeness
                    .excluded_baseline_subjects
                    .iter()
                    .map(|subject| JsonValue::string(subject.clone())),
            ),
        ));
    }
    JsonValue::object(entries)
}

pub(super) fn excluded_source_to_json(source: &ExcludedSource) -> JsonValue {
    JsonValue::object([
        ("path", JsonValue::string(source.path.clone())),
        ("reason", JsonValue::string(source.reason.clone())),
    ])
}

pub(super) fn policy_violation_to_json(violation: &MetricsPolicyViolation) -> JsonValue {
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

pub(super) fn module_to_json(module: &ModuleMetric) -> JsonValue {
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

pub(super) fn edge_to_json(edge: &DependencyEdge) -> JsonValue {
    JsonValue::object([
        ("source", JsonValue::string(edge.source.clone())),
        ("target", JsonValue::string(edge.target.clone())),
        ("span", span_to_json(&edge.span)),
    ])
}

pub(super) fn cycle_to_json(cycle: &DependencyCycle) -> JsonValue {
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

pub(super) fn abc_subject_to_json(subject: &AbcSubjectMetric) -> JsonValue {
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

pub(super) fn similarity_to_json(instance: &SimilarityInstanceMetric) -> JsonValue {
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

pub(super) fn similarity_declaration_to_json(
    declaration: &SimilarityDeclarationMetric,
) -> JsonValue {
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

pub(super) fn summary_to_json(summary: &MetricsSummary) -> JsonValue {
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

pub(super) fn parse_baseline_value(value: &JsonValue) -> Result<MetricsBaseline, Vec<Diagnostic>> {
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

pub(super) fn parse_baseline_module(value: &JsonValue) -> Option<BaselineModule> {
    Some(BaselineModule {
        module: json_string_field(value, "module")?.to_string(),
        path: json_string_field(value, "path")?.to_string(),
    })
}

pub(super) fn parse_baseline_edge(value: &JsonValue) -> Option<BaselineEdge> {
    Some(BaselineEdge {
        source: json_string_field(value, "source")?.to_string(),
        target: json_string_field(value, "target")?.to_string(),
    })
}

pub(super) fn parse_baseline_cycle(value: &JsonValue) -> Option<BaselineCycle> {
    Some(BaselineCycle {
        members: json_array_field(value, "members")?
            .iter()
            .filter_map(json_string)
            .map(str::to_string)
            .collect(),
    })
}

pub(super) fn json_string_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    json_object_field(value, key).and_then(json_string)
}

pub(super) fn json_array_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a [JsonValue]> {
    match json_object_field(value, key)? {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }
}

pub(super) fn json_object_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find_map(|(field, value)| (field == key).then_some(value))
}

pub(super) fn json_string(value: &JsonValue) -> Option<&str> {
    match value {
        JsonValue::String(value) => Some(value),
        _ => None,
    }
}

pub(super) fn metrics_io_diagnostic(message: String) -> Diagnostic {
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

pub(super) fn metrics_policy_diagnostic(
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
