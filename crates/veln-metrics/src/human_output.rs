use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct HumanOutputProjection {
    omitted: usize,
}

impl HumanOutputProjection {
    fn for_report(report: &MetricsReport) -> Self {
        Self {
            omitted: omitted_report_finding_count(report),
        }
    }

    fn for_check(check: &MetricsCheckReport) -> Self {
        Self {
            omitted: check
                .violations
                .len()
                .saturating_sub(check.report.human_output_max_findings)
                + omitted_report_finding_count(&check.report),
        }
    }

    fn append_summary(self, out: &mut String) {
        if self.omitted > 0 {
            out.push_str(&format!(
                "\nDetailed findings omitted: {}; use veln metrics --json for complete evidence.\n",
                self.omitted
            ));
        }
    }
}

pub(super) struct ReportHumanSelection {
    modules: Vec<bool>,
    cycles: Vec<bool>,
    abc_subjects: Vec<bool>,
    similarities: Vec<bool>,
}

pub(super) fn select_report_findings(report: &MetricsReport) -> ReportHumanSelection {
    let limit = report.human_output_max_findings;
    let cycles = selected_prefix(report.cycles.len(), limit);
    let modules = selected_prefix(report.modules.len(), limit);
    let abc_subjects = selected_prefix(report.abc_subjects.len(), limit);
    let similarities = selected_prefix(report.similarities.len(), limit);
    ReportHumanSelection {
        modules,
        cycles,
        abc_subjects,
        similarities,
    }
}

pub(super) fn detailed_report_finding_count(report: &MetricsReport) -> usize {
    report.modules.len()
        + report.cycles.len()
        + report.abc_subjects.len()
        + report.similarities.len()
}

pub(super) fn omitted_report_finding_count(report: &MetricsReport) -> usize {
    let limit = report.human_output_max_findings;
    [
        report.cycles.len(),
        report.modules.len(),
        report.abc_subjects.len(),
        report.similarities.len(),
    ]
    .into_iter()
    .map(|count| count.saturating_sub(limit))
    .sum()
}

pub(super) fn selected_prefix(count: usize, limit: usize) -> Vec<bool> {
    (0..count).map(|index| index < limit).collect()
}

pub(super) fn append_section_truncation(
    out: &mut String,
    count: usize,
    limit: usize,
    subject: &str,
) {
    let omitted = count.saturating_sub(limit);
    if omitted > 0 {
        out.push_str(&format!(
            "  showing {} of {count} {subject}; {omitted} omitted; use veln metrics --json for complete evidence.\n",
            count.min(limit)
        ));
    }
}

pub fn render_human(report: &MetricsReport) -> String {
    let projection = HumanOutputProjection::for_report(report);
    let selection = select_report_findings(report);
    let mut out = render_human_with_selection(report, &selection);
    projection.append_summary(&mut out);
    out
}

pub(super) fn render_human_with_selection(
    report: &MetricsReport,
    selection: &ReportHumanSelection,
) -> String {
    let mut out = String::new();
    append_report_summary(&mut out, report);
    append_cycles(&mut out, report, selection);
    append_modules(&mut out, report, selection);
    append_abc_subjects(&mut out, report, selection);
    append_similarities(&mut out, report, selection);
    out
}

pub(super) fn append_report_summary(out: &mut String, report: &MetricsReport) {
    out.push_str("Veln dependency metrics (advisory)\n");
    if report.completeness.is_partial() {
        out.push_str("analysis status: incomplete\n");
        out.push_str("excluded sources:\n");
        for source in &report.completeness.excluded_sources {
            out.push_str(&format!("  {} ({})\n", source.path, source.reason));
        }
    }
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
}

pub(super) fn append_cycles(
    out: &mut String,
    report: &MetricsReport,
    selection: &ReportHumanSelection,
) {
    out.push_str("Cycles\n");
    if report.cycles.is_empty() {
        out.push_str("  none\n");
    } else {
        append_section_truncation(
            out,
            report.cycles.len(),
            report.human_output_max_findings,
            "cycles",
        );
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
}

pub(super) fn append_modules(
    out: &mut String,
    report: &MetricsReport,
    selection: &ReportHumanSelection,
) {
    out.push_str("\nModules\n");
    if report.modules.is_empty() {
        out.push_str("  no project modules selected\n");
    } else {
        append_section_truncation(
            out,
            report.modules.len(),
            report.human_output_max_findings,
            "module rows",
        );
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
}

pub(super) fn append_abc_subjects(
    out: &mut String,
    report: &MetricsReport,
    selection: &ReportHumanSelection,
) {
    out.push_str("\nABC size\n");
    if report.abc_subjects.is_empty() {
        out.push_str("  no function or test subjects selected\n");
    } else {
        append_section_truncation(
            out,
            report.abc_subjects.len(),
            report.human_output_max_findings,
            "ABC subjects",
        );
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
}

pub(super) fn append_similarities(
    out: &mut String,
    report: &MetricsReport,
    selection: &ReportHumanSelection,
) {
    out.push_str("\nWhole-body similarity (experimental)\n");
    out.push_str("  Similarity is advisory; it never creates a metrics policy violation.\n");
    out.push_str("  Review repeated bodies manually; the report does not prescribe automatic deduplication.\n");
    if report.similarities.is_empty() {
        out.push_str("  none\n");
    } else {
        append_section_truncation(
            out,
            report.similarities.len(),
            report.human_output_max_findings,
            "similarity instances",
        );
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
}

pub(super) fn span_label(span: &SourceSpan) -> String {
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
    let projection = HumanOutputProjection::for_check(check);
    let limit = check.report.human_output_max_findings;
    let selected_violations = selected_prefix(check.violations.len(), limit);
    let report_selection = select_report_findings(&check.report);
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
        if !check
            .report
            .completeness
            .excluded_baseline_subjects
            .is_empty()
        {
            out.push_str(&format!(
                "baseline excluded subjects: {}\n",
                check
                    .report
                    .completeness
                    .excluded_baseline_subjects
                    .join(", ")
            ));
        }
    }
    if check.has_violations() {
        out.push_str("policy result: fail\n\n");
        out.push_str("Policy violations\n");
        append_section_truncation(&mut out, check.violations.len(), limit, "policy violations");
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
    } else if check.report.completeness.is_partial() {
        out.push_str("policy result: incomplete\n\n");
    } else {
        out.push_str("policy result: pass\n\n");
    }
    out.push_str(&render_human_with_selection(
        &check.report,
        &report_selection,
    ));
    projection.append_summary(&mut out);
    out
}
