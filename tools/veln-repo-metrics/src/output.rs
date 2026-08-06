use std::path::{Path, PathBuf};

use crate::{Config, Finding, Report, dependency_graph};

const JSON_SCHEMA_VERSION: &str = "veln-repo-metrics-json/v0";

pub(crate) fn render_human(report: &Report, config: &Config) -> String {
    let mut output = format!(
        "Rust repository metrics\n  files analyzed: {}\n  findings: {}\n",
        report.files.len(),
        report.findings.len()
    );
    output.push_str("\nFindings\n");
    if report.findings.is_empty() {
        output.push_str("  none\n");
    } else {
        for finding in report.findings.iter().take(config.max_findings) {
            output.push_str(&format!("  {finding}\n"));
        }
        if report.findings.len() > config.max_findings {
            output.push_str(&format!(
                "  ... {} more finding(s); use --format json for the complete report\n",
                report.findings.len() - config.max_findings
            ));
        }
    }
    if let Some(graph) = &report.dependency_graph {
        output.push('\n');
        output.push_str(
            &graph.render_human(config.dependency_hotspots, config.dependency_cycle_limit),
        );
    }
    output
}

pub(crate) fn render_json(report: &Report, config: &Config) -> String {
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "schema_version", JSON_SCHEMA_VERSION, false);
    push_tool_json(&mut output);
    push_inputs_json(&mut output, config);
    push_summary_json(&mut output, report);
    output.push_str(",\"files\":");
    push_json_paths(&mut output, &report.files);
    push_findings_json(&mut output, &report.findings);
    push_optional_dependency_json(&mut output, report.dependency_graph.as_ref());
    output.push('}');
    output
}

fn push_tool_json(output: &mut String) {
    output.push_str(",\"tool\":{");
    push_json_field(output, "name", "veln-repo-metrics", false);
    push_json_field(output, "version", env!("CARGO_PKG_VERSION"), true);
    output.push('}');
}

fn push_inputs_json(output: &mut String, config: &Config) {
    output.push_str(",\"inputs\":{");
    output.push_str("\"roots\":");
    push_json_paths(output, &config.roots);
    output.push_str(&format!(
        ",\"abc_threshold\":{:.6},\"file_line_threshold\":{}",
        config.threshold, config.file_line_threshold
    ));
    output.push('}');
}

fn push_summary_json(output: &mut String, report: &Report) {
    output.push_str(",\"summary\":{");
    output.push_str(&format!(
        "\"rust_file_count\":{},\"finding_count\":{}",
        report.files.len(),
        report.findings.len()
    ));
    output.push('}');
}

fn push_findings_json(output: &mut String, findings: &[Finding]) {
    output.push_str(",\"findings\":[");
    for (index, finding) in findings.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_finding_json(output, finding);
    }
    output.push(']');
}

fn push_optional_dependency_json(
    output: &mut String,
    graph: Option<&dependency_graph::DependencyReport>,
) {
    output.push_str(",\"dependency_graph\":");
    match graph {
        Some(graph) => push_dependency_json(output, graph),
        None => output.push_str("null"),
    }
}

fn push_finding_json(output: &mut String, finding: &Finding) {
    match finding {
        Finding::Function(finding) => {
            output.push('{');
            push_json_field(output, "kind", "abc_complexity", false);
            push_json_path_field(output, "path", &finding.file, true);
            output.push_str(&format!(",\"line\":{}", finding.line));
            push_json_field(output, "subject", &finding.name, true);
            output.push_str(&format!(
                ",\"abc\":{{\"assignments\":{},\"branches\":{},\"conditionals\":{},\"magnitude\":{:.6}}}",
                finding.metrics.assignments,
                finding.metrics.branches,
                finding.metrics.conditionals,
                finding.metrics.score()
            ));
            output.push('}');
        }
        Finding::File(finding) => {
            output.push('{');
            push_json_field(output, "kind", "file_line_count", false);
            push_json_path_field(output, "path", &finding.file, true);
            output.push_str(&format!(
                ",\"line\":{},\"lines\":{}",
                finding.line, finding.lines
            ));
            output.push('}');
        }
    }
}

fn push_dependency_json(output: &mut String, graph: &dependency_graph::DependencyReport) {
    output.push('{');
    output.push_str(&format!(
        "\"file_count\":{},\"edge_count\":{}",
        graph.file_count, graph.edge_count
    ));
    output.push_str(",\"hotspots\":[");
    for (index, hotspot) in graph.hotspots.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push('{');
        push_json_path_field(output, "path", &hotspot.path, false);
        output.push_str(&format!(
            ",\"incoming\":{},\"outgoing\":{},\"pressure\":{}",
            hotspot.incoming, hotspot.outgoing, hotspot.pressure
        ));
        output.push('}');
    }
    output.push_str("],\"cycles\":[");
    for (index, cycle) in graph.cycles.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_json_paths(output, cycle);
    }
    output.push_str("]}");
}

fn push_json_paths(output: &mut String, paths: &[PathBuf]) {
    output.push('[');
    for (index, path) in paths.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_json_string(output, &path.to_string_lossy());
    }
    output.push(']');
}

fn push_json_path_field(output: &mut String, name: &str, path: &Path, comma: bool) {
    push_json_field(output, name, &path.to_string_lossy(), comma);
}

fn push_json_field(output: &mut String, name: &str, value: &str, comma: bool) {
    if comma {
        output.push(',');
    }
    push_json_string(output, name);
    output.push(':');
    push_json_string(output, value);
}

pub(crate) fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0C}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control.is_control() => {
                output.push_str(&format!("\\u{:04X}", control as u32));
            }
            printable => output.push(printable),
        }
    }
    output.push('"');
}
