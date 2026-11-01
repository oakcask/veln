use super::*;

pub(super) fn cycle_diagnostics(
    module: &SurfaceModule,
    dependencies: &[Vec<Dependency>],
) -> (Vec<(usize, Diagnostic)>, BTreeSet<usize>) {
    let components = strongly_connected_components(dependencies);
    let mut cyclic = components
        .into_iter()
        .filter(|component| {
            component.len() > 1
                || dependencies[component[0]]
                    .iter()
                    .any(|dependency| dependency.target == component[0])
        })
        .collect::<Vec<_>>();
    cyclic.sort_by_key(|component| {
        component
            .iter()
            .map(|index| value_key(module, *index))
            .min()
    });

    let invalid = cyclic.iter().flatten().copied().collect();
    let diagnostics = cyclic
        .into_iter()
        .filter_map(|component| {
            let owner = component.iter().copied().min()?;
            cycle_diagnostic(module, dependencies, &component).map(|diagnostic| (owner, diagnostic))
        })
        .collect();
    (diagnostics, invalid)
}

fn cycle_diagnostic(
    module: &SurfaceModule,
    dependencies: &[Vec<Dependency>],
    component: &[usize],
) -> Option<Diagnostic> {
    let members = component.iter().copied().collect::<BTreeSet<_>>();
    let (source, edge) = closing_cycle_reference(module, dependencies, component, &members)?;
    let source_value = &module.module_values[source];
    let target_value = &module.module_values[edge.target];
    let source_name = source_value.name.as_deref().unwrap_or("<missing>");
    let target_name = target_value.name.as_deref().unwrap_or("<missing>");
    let mut diagnostic = Diagnostic::new(
        "value.module_initializer_cycle",
        Severity::Error,
        DiagnosticKind::Type,
        format!("module value reference `{target_name}` closes an initializer cycle"),
        Some(edge.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("type_check")),
            ("name", JsonValue::string(source_name)),
            ("target", JsonValue::string(target_name)),
        ]),
    );
    append_cycle_declarations(module, component, &mut diagnostic);
    append_cycle_references(module, dependencies, component, &members, &mut diagnostic);
    Some(diagnostic)
}

fn closing_cycle_reference(
    module: &SurfaceModule,
    dependencies: &[Vec<Dependency>],
    component: &[usize],
    members: &BTreeSet<usize>,
) -> Option<(usize, Dependency)> {
    let mut ordered = component.to_vec();
    ordered.sort_by_key(|index| value_key(module, *index));
    let mut colors = BTreeMap::<usize, u8>::new();
    for root in ordered {
        if colors.get(&root).copied().unwrap_or(0) != 0 {
            continue;
        }
        colors.insert(root, 1);
        let mut stack = vec![(
            root,
            sorted_component_edges(module, root, dependencies, members),
            0,
        )];
        while let Some((node, edges, edge_index)) = stack.last_mut() {
            if let Some(edge) = edges.get(*edge_index).cloned() {
                *edge_index += 1;
                match colors.get(&edge.target).copied().unwrap_or(0) {
                    0 => {
                        colors.insert(edge.target, 1);
                        stack.push((
                            edge.target,
                            sorted_component_edges(module, edge.target, dependencies, members),
                            0,
                        ));
                    }
                    1 => return Some((*node, edge)),
                    _ => {}
                }
            } else {
                colors.insert(*node, 2);
                stack.pop();
            }
        }
    }
    None
}

fn append_cycle_declarations(
    module: &SurfaceModule,
    component: &[usize],
    diagnostic: &mut Diagnostic,
) {
    let mut related_declarations = component.to_vec();
    related_declarations.sort_by_key(|index| value_key(module, *index));
    for index in related_declarations {
        let value = &module.module_values[index];
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("cycle_declaration")),
            (
                "message",
                JsonValue::string(format!(
                    "Module value `{}` participates in this initializer cycle.",
                    value.name.as_deref().unwrap_or("<missing>")
                )),
            ),
            ("span", span_json(&value.span)),
        ]));
    }
}

fn append_cycle_references(
    module: &SurfaceModule,
    dependencies: &[Vec<Dependency>],
    component: &[usize],
    members: &BTreeSet<usize>,
    diagnostic: &mut Diagnostic,
) {
    let mut related_references = component
        .iter()
        .flat_map(|source| {
            dependencies[*source]
                .iter()
                .filter(|edge| members.contains(&edge.target))
                .map(|edge| (*source, edge.clone()))
        })
        .collect::<Vec<_>>();
    related_references.sort_by_key(|(source, edge)| {
        (
            value_key(module, *source),
            value_key(module, edge.target),
            edge.span.file.as_str().to_string(),
            edge.span.start.offset,
        )
    });
    for (source, edge) in related_references {
        let source_name = module.module_values[source]
            .name
            .as_deref()
            .unwrap_or("<missing>");
        let target_name = module.module_values[edge.target]
            .name
            .as_deref()
            .unwrap_or("<missing>");
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("cycle_reference")),
            (
                "message",
                JsonValue::string(format!(
                    "Module value `{source_name}` depends on `{target_name}` here."
                )),
            ),
            ("span", span_json(&edge.span)),
        ]));
    }
}

fn sorted_component_edges(
    module: &SurfaceModule,
    node: usize,
    dependencies: &[Vec<Dependency>],
    members: &BTreeSet<usize>,
) -> Vec<Dependency> {
    let mut edges = dependencies[node]
        .iter()
        .filter(|edge| members.contains(&edge.target))
        .cloned()
        .collect::<Vec<_>>();
    edges.sort_by_key(|edge| (value_key(module, edge.target), edge.span.start.offset));
    edges
}

fn value_key(module: &SurfaceModule, index: usize) -> (String, String, String, String, usize) {
    let value = &module.module_values[index];
    (
        value.package_name.clone().unwrap_or_default(),
        value.module_name.clone().unwrap_or_default(),
        value.name.clone().unwrap_or_default(),
        value.span.file.as_str().to_string(),
        value.span.start.offset,
    )
}

fn strongly_connected_components(dependencies: &[Vec<Dependency>]) -> Vec<Vec<usize>> {
    let targets = dependencies
        .iter()
        .map(|edges| edges.iter().map(|edge| edge.target).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    strongly_connected_index_components(&targets)
}

pub(super) fn strongly_connected_index_components(dependencies: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let count = dependencies.len();
    let mut visited = vec![false; count];
    let mut finished = Vec::with_capacity(count);
    for root in 0..count {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack = vec![(root, 0)];
        while let Some((node, edge_index)) = stack.last_mut() {
            if let Some(target) = dependencies[*node].get(*edge_index) {
                *edge_index += 1;
                if !visited[*target] {
                    visited[*target] = true;
                    stack.push((*target, 0));
                }
            } else {
                finished.push(*node);
                stack.pop();
            }
        }
    }

    let mut reverse = vec![Vec::new(); count];
    for (source, edges) in dependencies.iter().enumerate() {
        for target in edges {
            reverse[*target].push(source);
        }
    }
    visited.fill(false);
    let mut components = Vec::new();
    while let Some(root) = finished.pop() {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut component = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            component.push(node);
            for predecessor in &reverse[node] {
                if !visited[*predecessor] {
                    visited[*predecessor] = true;
                    stack.push(*predecessor);
                }
            }
        }
        components.push(component);
    }
    components
}
