use super::*;
use veln_project::{ManifestLib, ManifestPackage, ManifestTool};

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
        Vec::new(),
        MetricsCompleteness::default(),
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
        self.unrelated_count + self.large_group_count + (self.pair_count * 2) + self.prefix_count
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

fn mark_report_partial(report: &mut MetricsReport, path: &str) {
    report.completeness.excluded_sources.push(ExcludedSource {
        path: path.to_string(),
        reason: "invalid_module_identity".to_string(),
    });
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
        Vec::new(),
        MetricsCompleteness::default(),
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
        source_bytes: text.into_bytes(),
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

mod config_and_human_output;
mod graph_and_abc;
mod ordering_and_similarity;
mod partial_analysis;
mod policy_and_baseline;
