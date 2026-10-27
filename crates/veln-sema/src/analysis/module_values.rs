use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use veln_ast::{
    BodyLine, BodyLineKind, Function, FunctionKind, NameClass, NameOccurrence, PublicAliasKind,
    SurfaceModule, Visibility,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};

use super::{FunctionChecker, VariantDiagnosticInterner, check_type_effect_labels};
use crate::diagnostics::span_json;
use crate::type_syntax::parse_type_annotation;
use crate::types::{ModuleValueIndex, TypeEnvironment};

mod admission;

use admission::{check_admitted_expr, collect_dependencies};

#[derive(Clone)]
pub(super) struct Dependency {
    target: usize,
    span: veln_source::SourceSpan,
}

pub(crate) struct ModuleValueAnalysis {
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) invalid: BTreeSet<(Option<String>, String)>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleValueAnnotationDedupWork {
    pub(crate) indexed_diagnostics: usize,
    pub(crate) path_lookups: usize,
}

#[cfg(test)]
thread_local! {
    static MODULE_VALUE_ANNOTATION_DEDUP_WORK:
        std::cell::Cell<ModuleValueAnnotationDedupWork> =
        const { std::cell::Cell::new(ModuleValueAnnotationDedupWork {
            indexed_diagnostics: 0,
            path_lookups: 0,
        }) };
}

#[cfg(test)]
pub(crate) fn reset_module_value_annotation_dedup_work() {
    MODULE_VALUE_ANNOTATION_DEDUP_WORK.set(ModuleValueAnnotationDedupWork::default());
}

#[cfg(test)]
pub(crate) fn take_module_value_annotation_dedup_work() -> ModuleValueAnnotationDedupWork {
    MODULE_VALUE_ANNOTATION_DEDUP_WORK.replace(ModuleValueAnnotationDedupWork::default())
}

#[cfg(test)]
fn record_module_value_annotation_dedup_work(
    update: impl FnOnce(&mut ModuleValueAnnotationDedupWork),
) {
    MODULE_VALUE_ANNOTATION_DEDUP_WORK.set({
        let mut work = MODULE_VALUE_ANNOTATION_DEDUP_WORK.get();
        update(&mut work);
        work
    });
}

pub(crate) fn check_module_values(
    module: &SurfaceModule,
    environment: &mut TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
) -> ModuleValueAnalysis {
    let mut diagnostics = Vec::new();
    let mut dependencies = vec![Vec::new(); module.module_values.len()];
    let colliding_identities = colliding_module_value_identities(module);
    let mut initially_invalid = colliding_identities.clone();
    initially_invalid.extend(invalid_cased_module_value_identities(module));
    environment.quarantine_module_values(&initially_invalid);
    let mut invalid = initially_invalid_values(module, &initially_invalid);
    let value_indices = module_value_indices(module, &colliding_identities);

    for (index, value) in module.module_values.iter().enumerate() {
        invalid[index] |= check_module_value(
            value,
            index,
            &value_indices,
            environment,
            variant_diagnostics,
            &mut dependencies[index],
            &mut diagnostics,
        );
    }

    let (cycle_diagnostics, cyclic) = cycle_diagnostics(module, &dependencies);
    diagnostics.extend(cycle_diagnostics);
    for index in cyclic {
        invalid[index] = true;
    }
    propagate_invalid_dependencies(&dependencies, &mut invalid);
    ModuleValueAnalysis {
        diagnostics,
        invalid: module
            .module_values
            .iter()
            .enumerate()
            .filter(|(index, value)| invalid[*index] && value.name.is_some())
            .map(|(_, value)| {
                (
                    value.module_name.clone(),
                    value.name.clone().expect("filtered module value name"),
                )
            })
            .collect(),
    }
}

fn initially_invalid_values(
    module: &SurfaceModule,
    colliding_identities: &BTreeSet<(Option<String>, String)>,
) -> Vec<bool> {
    module
        .module_values
        .iter()
        .map(|value| {
            value.name.as_ref().is_some_and(|name| {
                colliding_identities.contains(&(value.module_name.clone(), name.clone()))
            })
        })
        .collect()
}

fn module_value_indices(
    module: &SurfaceModule,
    colliding_identities: &BTreeSet<(Option<String>, String)>,
) -> HashMap<(Option<String>, String), ModuleValueIndex> {
    let mut indices = HashMap::new();
    for (index, value) in module.module_values.iter().enumerate() {
        if let Some(name) = &value.name {
            let identity = (value.module_name.clone(), name.clone());
            let indexed = if colliding_identities.contains(&identity) {
                ModuleValueIndex::Collision
            } else {
                ModuleValueIndex::Unique(index)
            };
            indices.entry(identity).or_insert(indexed);
        }
    }
    indices
}

fn invalid_cased_module_value_identities(
    module: &SurfaceModule,
) -> BTreeSet<(Option<String>, String)> {
    let invalid_declaration_spans = module
        .invalid_names
        .iter()
        .filter(|invalid| {
            invalid.class == NameClass::ValueBinding
                && invalid.occurrence == NameOccurrence::Declaration
        })
        .map(|invalid| {
            (
                invalid.span.file.clone(),
                invalid.span.start.offset,
                invalid.span.end.offset,
            )
        })
        .collect::<BTreeSet<_>>();
    module
        .module_values
        .iter()
        .filter_map(|value| {
            let name = value.name.as_ref()?;
            let name_span = value.name_span.as_ref()?;
            invalid_declaration_spans
                .contains(&(
                    name_span.file.clone(),
                    name_span.start.offset,
                    name_span.end.offset,
                ))
                .then(|| (value.module_name.clone(), name.clone()))
        })
        .collect()
}

fn check_module_value(
    value: &veln_ast::ModuleValueDecl,
    index: usize,
    value_indices: &HashMap<(Option<String>, String), ModuleValueIndex>,
    environment: &TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
    dependencies: &mut Vec<Dependency>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut has_invalid_dependency = false;
    collect_dependencies(
        &value.initializer,
        value.module_name.as_deref(),
        value_indices,
        environment,
        &mut Vec::new(),
        dependencies,
        &mut has_invalid_dependency,
    );
    let diagnostic_start = diagnostics.len();
    let parsed_annotation = parse_type_annotation(&value.annotation).ok();
    let expected = parsed_annotation
        .clone()
        .map(|ty| environment.canonicalize_type_annotation(ty, value.module_name.as_deref()));
    check_admitted_expr(
        &value.initializer,
        value,
        value_indices,
        environment,
        expected.as_ref(),
        diagnostics,
    );

    let synthetic = synthetic_module_value_function(value, index);
    diagnostics.extend(check_type_effect_labels(
        &value.annotation,
        &value.annotation_span,
        &synthetic,
        environment,
        "module_value_type",
    ));
    let mut checker =
        FunctionChecker::for_synthetic_declaration(&synthetic, environment, variant_diagnostics);
    checker.check_body();
    diagnostics.append(&mut checker.diagnostics);

    let annotation_is_closed = parsed_annotation.is_some_and(|annotation| {
        environment.type_annotation_is_fully_resolved(
            &annotation,
            value.module_name.as_deref(),
            &[],
        )
    });
    if !annotation_is_closed {
        push_unresolved_annotation_paths(value, environment, diagnostic_start, diagnostics);
    }
    diagnostics.len() != diagnostic_start || !annotation_is_closed || has_invalid_dependency
}

fn synthetic_module_value_function(value: &veln_ast::ModuleValueDecl, index: usize) -> Function {
    Function {
        node_id: value.node_id,
        module_name: value.module_name.clone(),
        package_name: value.package_name.clone(),
        kind: FunctionKind::Function,
        visibility: Visibility::Private,
        name: Some(format!("__module_value_{index}")),
        effect_binder: None,
        params: Vec::new(),
        return_binding: None,
        return_type: Some(value.annotation.clone()),
        return_type_span: Some(value.annotation_span.clone()),
        return_type_paths: value.annotation_paths.clone(),
        return_type_refinements: value.annotation_refinements.clone(),
        effects: Some(Vec::new()),
        effect_spans: Some(Vec::new()),
        callsite: None,
        contracts: Vec::new(),
        body: vec![BodyLine {
            node_id: value.node_id,
            kind: BodyLineKind::Expr {
                expr: value.initializer.clone(),
            },
            span: value.initializer.span.clone(),
        }],
        span: value.span.clone(),
    }
}

fn propagate_invalid_dependencies(dependencies: &[Vec<Dependency>], invalid: &mut [bool]) {
    let mut reverse = vec![Vec::new(); invalid.len()];
    for (source, edges) in dependencies.iter().enumerate() {
        for edge in edges {
            reverse[edge.target].push(source);
        }
    }
    let mut pending = invalid
        .iter()
        .enumerate()
        .filter_map(|(index, invalid)| invalid.then_some(index))
        .collect::<VecDeque<_>>();
    while let Some(target) = pending.pop_front() {
        for dependent in &reverse[target] {
            if !invalid[*dependent] {
                invalid[*dependent] = true;
                pending.push_back(*dependent);
            }
        }
    }
}

fn colliding_module_value_identities(module: &SurfaceModule) -> BTreeSet<(Option<String>, String)> {
    let mut counts = HashMap::<(Option<String>, String), usize>::new();
    for function in &module.functions {
        if let Some(name) = &function.name {
            *counts
                .entry((function.module_name.clone(), name.clone()))
                .or_default() += 1;
        }
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        if let Some(name) = &alias.name {
            *counts
                .entry((alias.module_name.clone(), name.clone()))
                .or_default() += 1;
        }
    }
    for value in &module.module_values {
        if let Some(name) = &value.name {
            *counts
                .entry((value.module_name.clone(), name.clone()))
                .or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(identity, count)| (count > 1).then_some(identity))
        .collect()
}

fn push_unresolved_annotation_paths(
    declaration: &veln_ast::ModuleValueDecl,
    environment: &TypeEnvironment,
    diagnostic_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let unresolved_spans = diagnostics[diagnostic_start..]
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .filter_map(|diagnostic| diagnostic.span.as_ref())
        .map(span_identity)
        .collect::<BTreeSet<_>>();
    #[cfg(test)]
    record_module_value_annotation_dedup_work(|work| {
        work.indexed_diagnostics += diagnostics.len() - diagnostic_start;
    });
    for path in &declaration.annotation_paths {
        let symbol = path.segments.join("::");
        #[cfg(test)]
        record_module_value_annotation_dedup_work(|work| work.path_lookups += 1);
        if environment.type_path_is_resolved(&symbol, declaration.module_name.as_deref())
            || unresolved_spans.contains(&span_identity(&path.application_span))
        {
            continue;
        }
        diagnostics.push(Diagnostic::new(
            "name.unresolved",
            Severity::Error,
            DiagnosticKind::Name,
            format!("unresolved type `{symbol}`"),
            Some(path.application_span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("name")),
                ("node_id", JsonValue::Null),
                ("symbol", JsonValue::string(symbol)),
                ("namespace", JsonValue::string("type")),
                ("resolution_status", JsonValue::string("unresolved")),
                ("candidates", JsonValue::array([])),
            ]),
        ));
    }
}

type SpanIdentity = (
    veln_source::SourcePath,
    (usize, usize, usize),
    (usize, usize, usize),
    Option<veln_source::SourcePath>,
    Option<(usize, usize, usize)>,
    Option<(usize, usize, usize)>,
);

fn span_identity(span: &veln_source::SourceSpan) -> SpanIdentity {
    let position =
        |position: veln_source::LineCol| (position.line, position.column, position.offset);
    (
        span.file.clone(),
        position(span.start),
        position(span.end),
        span.original_path().cloned(),
        span.original_start().map(position),
        span.original_end().map(position),
    )
}

fn cycle_diagnostics(
    module: &SurfaceModule,
    dependencies: &[Vec<Dependency>],
) -> (Vec<Diagnostic>, BTreeSet<usize>) {
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
        .filter_map(|component| cycle_diagnostic(module, dependencies, &component))
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
                    1 => {
                        return Some((*node, edge));
                    }
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

fn value_key(module: &SurfaceModule, index: usize) -> (String, String, String, usize) {
    let value = &module.module_values[index];
    (
        value.module_name.clone().unwrap_or_default(),
        value.name.clone().unwrap_or_default(),
        value.span.file.as_str().to_string(),
        value.span.start.offset,
    )
}

fn strongly_connected_components(dependencies: &[Vec<Dependency>]) -> Vec<Vec<usize>> {
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
            if let Some(edge) = dependencies[*node].get(*edge_index) {
                *edge_index += 1;
                if !visited[edge.target] {
                    visited[edge.target] = true;
                    stack.push((edge.target, 0));
                }
            } else {
                finished.push(*node);
                stack.pop();
            }
        }
    }

    let mut reverse = vec![Vec::new(); count];
    for (source, edges) in dependencies.iter().enumerate() {
        for edge in edges {
            reverse[edge.target].push(source);
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
