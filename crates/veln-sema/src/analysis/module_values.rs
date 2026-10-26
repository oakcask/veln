use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use veln_ast::{
    BodyLine, BodyLineKind, Expr, ExprKind, Function, FunctionKind, PublicAliasKind, SurfaceModule,
    Visibility,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};

use super::{FunctionChecker, VariantDiagnosticInterner};
use crate::adt::registry::ConstructorLookup;
use crate::adt::{descriptors::AdtConstructor, type_operations as adt};
use crate::diagnostics::span_json;
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation;
use crate::types::TypeEnvironment;

#[derive(Clone)]
struct Dependency {
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
    environment.quarantine_module_values(&colliding_identities);
    let mut invalid = module
        .module_values
        .iter()
        .map(|value| {
            value.name.as_ref().is_some_and(|name| {
                colliding_identities.contains(&(value.module_name.clone(), name.clone()))
            })
        })
        .collect::<Vec<_>>();
    let mut value_indices = HashMap::new();
    for (index, value) in module.module_values.iter().enumerate() {
        if let Some(name) = &value.name {
            value_indices
                .entry((value.module_name.clone(), name.clone()))
                .or_insert(index);
        }
    }

    for (index, value) in module.module_values.iter().enumerate() {
        collect_dependencies(
            &value.initializer,
            value.module_name.as_deref(),
            &value_indices,
            &mut Vec::new(),
            &mut dependencies[index],
        );
        let diagnostic_count = diagnostics.len();
        let expected = parse_type_annotation(&value.annotation)
            .ok()
            .map(|ty| environment.canonicalize_type_annotation(ty, value.module_name.as_deref()));
        check_admitted_expr(
            &value.initializer,
            value,
            &value_indices,
            environment,
            expected.as_ref(),
            &mut diagnostics,
        );

        let synthetic = Function {
            node_id: value.node_id,
            module_name: value.module_name.clone(),
            package_name: None,
            kind: FunctionKind::Function,
            visibility: Visibility::Private,
            name: Some(format!("__module_value_{}", index)),
            effect_binder: None,
            params: Vec::new(),
            return_binding: None,
            return_type: Some(value.annotation.clone()),
            return_type_span: Some(value.span.clone()),
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
        };
        let mut checker = FunctionChecker::for_synthetic_declaration(
            &synthetic,
            environment,
            variant_diagnostics,
        );
        checker.check_body();
        diagnostics.append(&mut checker.diagnostics);
        let annotation_is_closed =
            parse_type_annotation(&value.annotation).is_ok_and(|annotation| {
                environment.type_annotation_is_fully_resolved(
                    &annotation,
                    value.module_name.as_deref(),
                    &[],
                )
            });
        if !annotation_is_closed {
            push_unresolved_annotation_paths(
                value,
                environment,
                diagnostic_count,
                &mut diagnostics,
            );
        }
        invalid[index] |= diagnostics.len() != diagnostic_count || !annotation_is_closed;
    }

    let (cycle_diagnostics, cyclic) = cycle_diagnostics(module, &dependencies);
    diagnostics.extend(cycle_diagnostics);
    for index in cyclic {
        invalid[index] = true;
    }
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

fn check_admitted_expr(
    expr: &Expr,
    declaration: &veln_ast::ModuleValueDecl,
    value_indices: &HashMap<(Option<String>, String), usize>,
    environment: &TypeEnvironment,
    expected: Option<&Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &expr.kind {
        ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
        ExprKind::Record(fields) => {
            if fields.is_empty() && expected.is_some_and(|ty| ty.dict_parts().is_some()) {
                diagnostics.push(unsupported_diagnostic(expr, declaration));
                return;
            }
            for field in fields {
                check_admitted_expr(
                    &field.expr,
                    declaration,
                    value_indices,
                    environment,
                    expected.and_then(|ty| ty.record_field(&field.name)),
                    diagnostics,
                );
            }
        }
        ExprKind::Prefix {
            op: veln_ast::PrefixOp::Negate,
            expr: inner,
        } if matches!(inner.kind, ExprKind::IntLiteral(_)) => {}
        ExprKind::NamePath { segments, .. } => {
            if module_value_index(value_indices, segments, declaration.module_name.as_deref())
                .is_some()
            {
                return;
            }
            if matches!(
                environment.adts.nullary_constructor(
                    segments,
                    declaration.module_name.as_deref(),
                    &environment.uses,
                ),
                ConstructorLookup::Found(_)
            ) {
                return;
            }
            if known_function(environment, segments, declaration.module_name.as_deref()) {
                diagnostics.push(unsupported_diagnostic(expr, declaration));
            }
        }
        ExprKind::Call { callee, args } => {
            match constructor_call(
                environment,
                callee,
                declaration.module_name.as_deref(),
                expected,
            ) {
                ConstructorCall::Found {
                    constructor,
                    diagnostic_name,
                    reason,
                } => {
                    let expected_count = constructor.variant.payload_fields.len();
                    if args.len() != expected_count {
                        diagnostics.push(constructor_arity_diagnostic(
                            expr,
                            expected_count,
                            args.len(),
                            diagnostic_name,
                            reason,
                        ));
                    }
                    for (index, arg) in args.iter().enumerate() {
                        let payload_expected = expected
                            .and_then(|expected| adt::payload_type(expected, constructor, index))
                            .or_else(|| {
                                adt::payload_type_with_resolved_args(constructor, index, |_| {
                                    Type::Unknown
                                })
                            });
                        check_admitted_expr(
                            arg,
                            declaration,
                            value_indices,
                            environment,
                            payload_expected.as_ref(),
                            diagnostics,
                        );
                    }
                }
                ConstructorCall::Ambiguous => {
                    for arg in args {
                        check_admitted_expr(
                            arg,
                            declaration,
                            value_indices,
                            environment,
                            None,
                            diagnostics,
                        );
                    }
                }
                ConstructorCall::Missing => {
                    let known_module_value = callee.callee_name_path().is_some_and(|segments| {
                        module_value_index(
                            value_indices,
                            segments,
                            declaration.module_name.as_deref(),
                        )
                        .is_some()
                    });
                    let known_function_call = callee.callee_name_path().is_some_and(|segments| {
                        known_function(environment, segments, declaration.module_name.as_deref())
                    });
                    if known_module_value || known_function_call {
                        diagnostics.push(unsupported_diagnostic(expr, declaration));
                    } else {
                        for arg in args {
                            check_admitted_expr(
                                arg,
                                declaration,
                                value_indices,
                                environment,
                                None,
                                diagnostics,
                            );
                        }
                    }
                }
            }
        }
        ExprKind::Missing => {}
        _ => diagnostics.push(unsupported_diagnostic(expr, declaration)),
    }
}

fn collect_dependencies(
    expr: &Expr,
    current_module: Option<&str>,
    value_indices: &HashMap<(Option<String>, String), usize>,
    bindings: &mut Vec<String>,
    dependencies: &mut Vec<Dependency>,
) {
    if let ExprKind::NamePath { segments, .. } = &expr.kind
        && !matches!(segments.as_slice(), [name] if bindings.iter().rev().any(|binding| binding == name))
        && let Some(target) = module_value_index(value_indices, segments, current_module)
    {
        dependencies.push(Dependency {
            target,
            span: expr.span.clone(),
        });
    }
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            collect_dependencies(
                scrutinee,
                current_module,
                value_indices,
                bindings,
                dependencies,
            );
            for arm in arms {
                let binding_count = bindings.len();
                arm.pattern
                    .for_each_binding(&mut |name| bindings.push(name.to_string()));
                collect_dependencies(
                    &arm.expr,
                    current_module,
                    value_indices,
                    bindings,
                    dependencies,
                );
                bindings.truncate(binding_count);
            }
        }
        ExprKind::Begin { body, .. } => {
            let binding_count = bindings.len();
            collect_body_dependencies(body, current_module, value_indices, bindings, dependencies);
            bindings.truncate(binding_count);
        }
        _ => expr.for_each_child(&mut |child| {
            collect_dependencies(child, current_module, value_indices, bindings, dependencies);
        }),
    }
}

fn collect_body_dependencies(
    body: &[BodyLine],
    current_module: Option<&str>,
    value_indices: &HashMap<(Option<String>, String), usize>,
    bindings: &mut Vec<String>,
    dependencies: &mut Vec<Dependency>,
) {
    for line in body {
        match &line.kind {
            BodyLineKind::Let { pattern, expr, .. } => {
                collect_dependencies(expr, current_module, value_indices, bindings, dependencies);
                pattern.for_each_binding(&mut |name| bindings.push(name.to_string()));
            }
            BodyLineKind::Expr { expr } => {
                collect_dependencies(expr, current_module, value_indices, bindings, dependencies)
            }
            BodyLineKind::Defer { body, .. } => {
                let binding_count = bindings.len();
                collect_body_dependencies(
                    body,
                    current_module,
                    value_indices,
                    bindings,
                    dependencies,
                );
                bindings.truncate(binding_count);
            }
        }
    }
}

fn module_value_index(
    value_indices: &HashMap<(Option<String>, String), usize>,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<usize> {
    let (name, qualifier) = segments.split_last()?;
    let qualifier_matches = qualifier.is_empty()
        || current_module.is_some_and(|module_name| {
            qualifier.join("::") == module_name
                || (qualifier.len() == 1
                    && qualifier
                        .last()
                        .is_some_and(|leaf| module_name.rsplit("::").next() == Some(leaf.as_str())))
        });
    qualifier_matches
        .then(|| value_indices.get(&(current_module.map(str::to_owned), name.clone())))
        .flatten()
        .copied()
}

fn known_function(
    environment: &TypeEnvironment,
    segments: &[String],
    current_module: Option<&str>,
) -> bool {
    match segments {
        [name] => !matches!(
            environment.unqualified_function(name, current_module),
            crate::types::FunctionLookup::Missing
        ),
        _ => environment
            .function_path_for_value(segments, current_module)
            .is_some(),
    }
}

enum ConstructorCall<'a> {
    Found {
        constructor: AdtConstructor<'a>,
        diagnostic_name: String,
        reason: &'static str,
    },
    Ambiguous,
    Missing,
}

fn constructor_call<'a>(
    environment: &'a TypeEnvironment,
    callee: &Expr,
    current_module: Option<&str>,
    expected: Option<&Type>,
) -> ConstructorCall<'a> {
    let Some((segments, _)) = callee.callee_name_path_and_type_args() else {
        return ConstructorCall::Missing;
    };
    let contextual = (segments.len() == 1)
        .then_some(expected)
        .flatten()
        .and_then(|expected| {
            environment
                .adts
                .descriptor_for_type_prefer_module(expected, current_module)
        })
        .and_then(|descriptor| {
            environment.adts.constructor_for_descriptor(
                segments,
                descriptor,
                current_module,
                &environment.uses,
            )
        });
    match contextual.map(ConstructorLookup::Found).unwrap_or_else(|| {
        environment
            .adts
            .constructor(segments, current_module, &environment.uses)
    }) {
        ConstructorLookup::Found(constructor) => ConstructorCall::Found {
            constructor,
            diagnostic_name: constructor.descriptor.diagnostic_name.clone(),
            reason: match constructor.descriptor.type_name.as_str() {
                "Option" => "option_constructor_arity_mismatch",
                "Result" => "result_constructor_arity_mismatch",
                _ => "constructor_arity_mismatch",
            },
        },
        ConstructorLookup::Ambiguous => ConstructorCall::Ambiguous,
        ConstructorLookup::Missing => ConstructorCall::Missing,
    }
}

fn constructor_arity_diagnostic(
    expr: &Expr,
    expected_count: usize,
    actual_count: usize,
    diagnostic_name: String,
    reason: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        format!("core.{reason}"),
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "{diagnostic_name} constructor expects {expected_count} argument, but got {actual_count}"
        ),
        Some(expr.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("core_lowering")),
            ("node_id", JsonValue::string(expr.node_id.display("expr"))),
            ("reason", JsonValue::string(reason)),
            (
                "facts",
                JsonValue::object([
                    (
                        "expected_argument_count",
                        JsonValue::Number(expected_count as i64),
                    ),
                    (
                        "actual_argument_count",
                        JsonValue::Number(actual_count as i64),
                    ),
                ]),
            ),
        ]),
    )
}

fn unsupported_diagnostic(expr: &Expr, declaration: &veln_ast::ModuleValueDecl) -> Diagnostic {
    let name = declaration.name.as_deref().unwrap_or("<missing>");
    let mut diagnostic = Diagnostic::new(
        "value.module_initializer_unsupported",
        Severity::Error,
        DiagnosticKind::Type,
        "module value initializer expression is not immutable data construction",
        Some(expr.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("type_check")),
            ("node_id", JsonValue::string(expr.node_id.display("expr"))),
            ("name", JsonValue::string(name)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("module_value_declaration")),
        (
            "message",
            JsonValue::string(format!("Module value `{name}` is declared here.")),
        ),
        ("span", span_json(&declaration.span)),
    ]));
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("permitted_initializers")),
        (
            "message",
            JsonValue::string(
                "Use literals, records, ADT constructors, or same-module module values.",
            ),
        ),
    ]));
    diagnostic
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
    let mut ordered = component.to_vec();
    ordered.sort_by_key(|index| value_key(module, *index));
    let mut colors = BTreeMap::<usize, u8>::new();
    let mut closing = None;
    for root in ordered {
        if colors.get(&root).copied().unwrap_or(0) != 0 {
            continue;
        }
        colors.insert(root, 1);
        let mut stack = vec![(
            root,
            sorted_component_edges(module, root, dependencies, &members),
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
                            sorted_component_edges(module, edge.target, dependencies, &members),
                            0,
                        ));
                    }
                    1 => {
                        closing = Some((*node, edge));
                        break;
                    }
                    _ => {}
                }
            } else {
                colors.insert(*node, 2);
                stack.pop();
            }
        }
        if closing.is_some() {
            break;
        }
    }
    let (source, edge) = closing?;
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
    Some(diagnostic)
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
