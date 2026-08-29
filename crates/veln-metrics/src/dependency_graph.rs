use super::*;

#[derive(Debug)]
pub(super) struct DependencyGraph {
    nodes: Vec<DependencyNode>,
    incoming: Vec<BTreeSet<usize>>,
    outgoing: Vec<BTreeSet<usize>>,
    edges: Vec<DependencyEdgeIndex>,
}

#[derive(Debug)]
pub(super) struct DependencyNode {
    module: String,
    path: String,
    span: SourceSpan,
    external_dependencies: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub(super) struct DependencyEdgeIndex {
    source: usize,
    target: usize,
    span: SourceSpan,
}

impl DependencyGraph {
    pub(super) fn from_project(project: &Project) -> Result<Self, Vec<Diagnostic>> {
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

    pub(super) fn report(
        self,
        source_project: &Project,
        project: ProjectIdentity,
        selected_paths: &BTreeSet<String>,
        config: MetricsConfig,
        diagnostics: Vec<Diagnostic>,
        completeness: MetricsCompleteness,
    ) -> MetricsReport {
        let selected_modules = self.selected_modules(selected_paths);
        let modules = self.selected_module_metrics(&selected_modules);
        let edges = self.selected_edges(&selected_modules);
        let cycles = self.selected_cycles(&selected_modules);
        let abc_subjects = abc_subjects(source_project, selected_paths);
        let abc_contract_subject_count = abc_contract_subject_count(source_project, selected_paths);
        let (similarities, similarity_fingerprint_count) =
            similarity_instances(source_project, selected_paths, config.similarity_min_tokens);
        let summary = self.summary(
            &modules,
            &cycles,
            &abc_subjects,
            abc_contract_subject_count,
            &similarities,
            similarity_fingerprint_count,
        );
        MetricsReport {
            project,
            diagnostics,
            completeness,
            modules,
            edges,
            cycles,
            abc_subjects,
            similarities,
            summary,
            human_output_max_findings: config.human_output_max_findings,
        }
    }

    fn selected_modules(&self, selected_paths: &BTreeSet<String>) -> BTreeSet<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| selected_paths.contains(&node.path).then_some(index))
            .collect()
    }

    fn selected_module_metrics(&self, selected_modules: &BTreeSet<usize>) -> Vec<ModuleMetric> {
        let mut modules = selected_modules
            .iter()
            .map(|index| self.module_metric(*index))
            .collect::<Vec<_>>();
        modules.sort_by(compare_module_metrics);
        modules
    }

    fn selected_edges(&self, selected_modules: &BTreeSet<usize>) -> Vec<DependencyEdge> {
        self.edges
            .iter()
            .filter(|edge| {
                selected_modules.contains(&edge.source) || selected_modules.contains(&edge.target)
            })
            .map(|edge| DependencyEdge {
                source: self.nodes[edge.source].module.clone(),
                target: self.nodes[edge.target].module.clone(),
                span: edge.span.clone(),
            })
            .collect()
    }

    fn selected_cycles(&self, selected_modules: &BTreeSet<usize>) -> Vec<DependencyCycle> {
        self.cycles()
            .into_iter()
            .filter(|cycle| cycle.iter().any(|index| selected_modules.contains(index)))
            .map(|cycle| self.dependency_cycle(cycle))
            .collect()
    }

    fn summary(
        &self,
        modules: &[ModuleMetric],
        cycles: &[DependencyCycle],
        abc_subjects: &[AbcSubjectMetric],
        abc_contract_subject_count: usize,
        similarities: &[SimilarityInstanceMetric],
        similarity_fingerprint_count: usize,
    ) -> MetricsSummary {
        let external_dependency_count = modules
            .iter()
            .map(|module| module.external_dependency_count)
            .sum();
        let similarity_region_count = similarities
            .iter()
            .map(|instance| instance.declarations.len())
            .sum();
        MetricsSummary {
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

pub(super) struct Tarjan<'a> {
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
