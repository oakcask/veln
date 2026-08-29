use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};

use veln_ast::{
    BodyLine, BodyLineKind, CodecImplementationKind, Expr, ExprKind, Function, FunctionKind,
    Pattern, PatternKind, PublicAliasKind, SurfaceModule, UseDecl, Visibility,
};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, SourceSpan};
use veln_syntax::{TokenKind, lex};

mod call_resolution;
mod callee_collection;
mod index;
mod recovery_collection;
mod recovery_index;
mod recovery_resolution;
mod targets;

use call_resolution::*;
use callee_collection::*;
pub(crate) use index::ReachabilityCache;
use index::{FunctionTargetIndex, ReachabilityIndex, ReachabilityInputs};
use recovery_index::*;
use recovery_resolution::*;
use targets::*;

#[cfg(test)]
pub(crate) fn reachable_entry_module(
    module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
) -> SurfaceModule {
    reachable_entry_module_with_cache(module, entry, entry_kind, &ReachabilityCache::default())
}

#[cfg(test)]
pub(crate) mod reachability_counters {
    use std::cell::Cell;

    thread_local! {
        static FUNCTION_LOOKUP_SCANS: Cell<usize> = const { Cell::new(0) };
        static TARGET_RESOLUTION_SCANS: Cell<usize> = const { Cell::new(0) };
        static MATERIALIZED_FUNCTION_BODIES: Cell<usize> = const { Cell::new(0) };
        static RECOVERY_SELECTOR_CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        FUNCTION_LOOKUP_SCANS.set(0);
        TARGET_RESOLUTION_SCANS.set(0);
        MATERIALIZED_FUNCTION_BODIES.set(0);
        RECOVERY_SELECTOR_CANDIDATE_SCANS.set(0);
    }

    pub(crate) fn record_function_lookup_scan() {
        FUNCTION_LOOKUP_SCANS.set(FUNCTION_LOOKUP_SCANS.get() + 1);
    }

    pub(crate) fn record_target_resolution_scan() {
        TARGET_RESOLUTION_SCANS.set(TARGET_RESOLUTION_SCANS.get() + 1);
    }

    pub(crate) fn record_materialized_function_body() {
        MATERIALIZED_FUNCTION_BODIES.set(MATERIALIZED_FUNCTION_BODIES.get() + 1);
    }

    pub(crate) fn record_recovery_selector_candidate_scan() {
        RECOVERY_SELECTOR_CANDIDATE_SCANS.set(RECOVERY_SELECTOR_CANDIDATE_SCANS.get() + 1);
    }

    pub(crate) fn snapshot() -> (usize, usize, usize, usize) {
        (
            FUNCTION_LOOKUP_SCANS.get(),
            TARGET_RESOLUTION_SCANS.get(),
            MATERIALIZED_FUNCTION_BODIES.get(),
            RECOVERY_SELECTOR_CANDIDATE_SCANS.get(),
        )
    }
}

#[cfg(test)]
pub(crate) fn reachable_entry_module_with_cache(
    module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
    cache: &ReachabilityCache,
) -> SurfaceModule {
    let inputs = ReachabilityInputs::combined(module);
    let reachability_index = cache
        .function_targets
        .get_or_init(|| reachable_function_targets(&inputs));
    let companion_access_targets = companion_function_access_targets(&inputs);
    let reachable = reachable_functions(
        &inputs,
        entry,
        entry_kind,
        reachability_index,
        &companion_access_targets,
        cache,
    );
    module_with_reachable_functions(&inputs, &reachable)
}

pub(crate) fn reachable_entry_module_with_standard_cache(
    standard_module: &SurfaceModule,
    application_module: &SurfaceModule,
    entry: &str,
    entry_kind: FunctionKind,
    cache: &ReachabilityCache,
) -> SurfaceModule {
    let inputs = ReachabilityInputs::separated(standard_module, application_module);
    let reachability_index = cache
        .separated_function_targets
        .get_or_init(|| reachable_function_targets(&inputs));
    let companion_access_targets = companion_function_access_targets(&inputs);
    let reachable = reachable_functions(
        &inputs,
        entry,
        entry_kind,
        reachability_index,
        &companion_access_targets,
        cache,
    );
    module_with_reachable_functions(&inputs, &reachable)
}

fn reachable_function_targets(inputs: &ReachabilityInputs<'_>) -> ReachabilityIndex {
    let mut function_targets = function_targets(inputs);
    function_targets.extend(function_alias_targets(inputs, &function_targets));
    function_targets.extend(codec_with_targets(inputs));
    ReachabilityIndex::new(inputs, function_targets)
}

fn function_targets(inputs: &ReachabilityInputs<'_>) -> Vec<FunctionTarget> {
    inputs
        .functions()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(function_target)
        .collect()
}

fn function_target(function: &Function) -> Option<FunctionTarget> {
    let name = function.name.clone()?;
    let recovery = !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase);
    Some(FunctionTarget {
        name: name.clone(),
        module_name: function.module_name.clone(),
        target_name: name,
        target_module_name: function.module_name.clone(),
        target_node_id: function.node_id,
        visibility: function.visibility,
        shape: function_shape(function),
        bare_importable: true,
        requires_public_import: false,
        recovery,
    })
}

fn function_shape(function: &Function) -> FunctionShape {
    let mut fixed_arity = 0usize;
    let mut variadic = None;
    for param in &function.params {
        if param.is_variadic {
            variadic = param.ty.clone();
        } else {
            fixed_arity += 1;
        }
    }
    FunctionShape {
        fixed_arity,
        variadic,
    }
}

fn codec_with_targets(inputs: &ReachabilityInputs<'_>) -> Vec<FunctionTarget> {
    inputs
        .codecs()
        .flat_map(|codec| {
            let name = codec.name.clone()?;
            Some(
                codec
                    .implementations
                    .iter()
                    .filter_map(move |implementation| {
                        let CodecImplementationKind::With {
                            function: Some(function_name),
                        } = &implementation.kind
                        else {
                            return None;
                        };
                        let target = inputs.functions().find(|function| {
                            function.kind == FunctionKind::Function
                                && function.name.as_deref() == Some(function_name.as_str())
                                && function.module_name == codec.module_name
                        })?;
                        Some(FunctionTarget {
                            name: name.clone(),
                            module_name: codec.module_name.clone(),
                            target_name: function_name.clone(),
                            target_module_name: target.module_name.clone(),
                            target_node_id: target.node_id,
                            visibility: codec.visibility,
                            shape: function_shape(target),
                            bare_importable: false,
                            requires_public_import: true,
                            recovery: false,
                        })
                    }),
            )
        })
        .flatten()
        .collect()
}

fn reachable_functions(
    inputs: &ReachabilityInputs<'_>,
    entry: &str,
    entry_kind: FunctionKind,
    reachability_index: &ReachabilityIndex,
    companion_access_targets: &HashMap<String, String>,
    cache: &ReachabilityCache,
) -> HashSet<ReachableFunction> {
    let mut reachable = HashSet::<ReachableFunction>::new();
    let mut stack = vec![ReachableFunction {
        kind: entry_kind,
        name: entry.to_string(),
        module_name: None,
        node_id: None,
    }];

    while let Some(key) = stack.pop() {
        if !reachable.insert(key.clone()) {
            continue;
        }
        let cached_callees = cache.direct_callees.borrow().get(&key).cloned();
        let callees = cached_callees.unwrap_or_else(|| {
            let callees = reachability_index
                .function_refs(&key)
                .iter()
                .map(|function_ref| {
                    #[cfg(test)]
                    reachability_counters::record_function_lookup_scan();
                    inputs.function(*function_ref)
                })
                .filter(|function| {
                    key.node_id
                        .is_none_or(|node_id| function.node_id == node_id)
                })
                .flat_map(|function| {
                    direct_function_callees(
                        function,
                        inputs,
                        &reachability_index.function_targets,
                        companion_access_targets,
                    )
                })
                .collect::<Vec<_>>();
            cache
                .direct_callees
                .borrow_mut()
                .insert(key.clone(), callees.clone());
            callees
        });
        for callee in callees {
            if !reachable.contains(&callee) {
                stack.push(callee);
            }
        }
    }
    reachable
}

fn module_with_reachable_functions(
    inputs: &ReachabilityInputs<'_>,
    reachable: &HashSet<ReachableFunction>,
) -> SurfaceModule {
    let mut functions = materialize_reachable_functions(inputs, reachable);
    let reachable_invalid_name_spans = reachable_invalid_name_spans(inputs, &functions);
    functions.extend(materialize_quarantined_import_proof_functions(
        inputs,
        &functions,
        &reachable_invalid_name_spans,
    ));
    let invalid_names_by_declaration = inputs.cloned_declarations(|module| &module.invalid_names);
    let invalid_names = inputs
        .cloned_declarations(|module| &module.invalid_names)
        .into_iter()
        .filter(|invalid| invalid_name_is_reachable(invalid, &reachable_invalid_name_spans))
        .collect();
    SurfaceModule {
        module: inputs.module_header(),
        uses: inputs.cloned_declarations(|module| &module.uses),
        aliases: inputs
            .cloned_declarations(|module| &module.aliases)
            .into_iter()
            .filter(|alias| {
                !declaration_contains_invalid_name(&alias.span, &invalid_names_by_declaration)
                    || reachable_invalid_name_spans
                        .iter()
                        .any(|span| span.is_declaration(&alias.span))
            })
            .collect(),
        effects: inputs.cloned_declarations(|module| &module.effects),
        handlers: inputs
            .cloned_declarations(|module| &module.handlers)
            .into_iter()
            .filter(|handler| {
                reachable_invalid_name_spans
                    .iter()
                    .any(|span| span.is_declaration(&handler.span))
            })
            .collect(),
        types: inputs.cloned_declarations(|module| &module.types),
        schemas: inputs.cloned_declarations(|module| &module.schemas),
        codecs: inputs.cloned_declarations(|module| &module.codecs),
        functions,
        invalid_names,
    }
}

fn declaration_contains_invalid_name(
    declaration: &SourceSpan,
    invalid_names: &[veln_ast::InvalidName],
) -> bool {
    invalid_names
        .iter()
        .any(|invalid| span_contains(declaration, &invalid.span))
}

fn invalid_name_is_reachable(
    invalid: &veln_ast::InvalidName,
    reachable_spans: &[ReachableInvalidNameSpan],
) -> bool {
    if let Some(span) = &invalid.enclosing_function_span {
        return reachable_spans
            .iter()
            .any(|reachable| reachable.is_declaration(span));
    }
    reachable_spans.iter().any(|reachable| match reachable {
        ReachableInvalidNameSpan::Declaration(span) => span_contains(span, &invalid.span),
        ReachableInvalidNameSpan::Name(span) => span == &invalid.span,
    })
}

fn invalid_import_path_segment_spans(
    use_decl: &UseDecl,
    invalid_names: &[&veln_ast::InvalidName],
) -> Vec<ReachableInvalidNameSpan> {
    invalid_names
        .iter()
        .copied()
        .filter(move |invalid| invalid_module_segment_in_use_decl(use_decl, invalid))
        .map(|invalid| ReachableInvalidNameSpan::Name(invalid.span.clone()))
        .collect()
}

fn use_decl_has_invalid_module_segment(
    use_decl: &UseDecl,
    invalid_names: &[&veln_ast::InvalidName],
) -> bool {
    invalid_names
        .iter()
        .copied()
        .any(|invalid| invalid_module_segment_in_use_decl(use_decl, invalid))
}

fn invalid_module_segment_in_use_decl(use_decl: &UseDecl, invalid: &veln_ast::InvalidName) -> bool {
    invalid.class == veln_ast::NameClass::Module
        && invalid.occurrence == veln_ast::NameOccurrence::PathSegment
        && span_contains(&use_decl.span, &invalid.span)
}

fn reachable_invalid_name_spans(
    inputs: &ReachabilityInputs<'_>,
    functions: &[Function],
) -> Vec<ReachableInvalidNameSpan> {
    let mut selector = ReachableInvalidNameSelector::new(inputs);
    let mut spans = Vec::new();
    spans.extend(
        functions
            .iter()
            .map(|function| function.span.clone())
            .map(ReachableInvalidNameSpan::Declaration),
    );
    for function in functions {
        selector.collect_function(function, &mut spans);
    }
    dedup_reachable_invalid_name_spans(&mut spans);
    spans
}

fn materialize_reachable_functions(
    inputs: &ReachabilityInputs<'_>,
    reachable: &HashSet<ReachableFunction>,
) -> Vec<Function> {
    inputs
        .functions()
        .filter(|function| {
            function.name.as_ref().is_some_and(|name| {
                reachable.contains(&ReachableFunction {
                    kind: function.kind,
                    name: name.clone(),
                    module_name: None,
                    node_id: None,
                }) || reachable.contains(&ReachableFunction {
                    kind: function.kind,
                    name: name.clone(),
                    module_name: function.module_name.clone(),
                    node_id: None,
                }) || reachable.contains(&ReachableFunction {
                    kind: function.kind,
                    name: name.clone(),
                    module_name: function.module_name.clone(),
                    node_id: Some(function.node_id),
                })
            })
        })
        .inspect(|_function| {
            #[cfg(test)]
            if !_function.body.is_empty() {
                reachability_counters::record_materialized_function_body();
            }
        })
        .cloned()
        .collect()
}

fn materialize_quarantined_import_proof_functions(
    inputs: &ReachabilityInputs<'_>,
    reachable_functions: &[Function],
    reachable_invalid_name_spans: &[ReachableInvalidNameSpan],
) -> Vec<Function> {
    let invalid_names = inputs.invalid_names().collect::<Vec<_>>();
    let quarantined_modules = inputs
        .all_uses()
        .into_iter()
        .filter(|use_decl| use_decl_has_invalid_module_segment(use_decl, &invalid_names))
        .filter(|use_decl| {
            invalid_import_path_segment_spans(use_decl, &invalid_names)
                .iter()
                .any(|span| {
                    reachable_invalid_name_spans
                        .iter()
                        .any(|reachable| reachable == span)
                })
        })
        .map(|use_decl| use_decl.name.as_str())
        .collect::<HashSet<_>>();
    if quarantined_modules.is_empty() {
        return Vec::new();
    }
    inputs
        .functions()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Public
                && function.module_name.as_deref().is_some_and(|module_name| {
                    quarantined_modules.contains(module_name)
                        && !reachable_functions.iter().any(|reachable| {
                            reachable.module_name.as_deref() == Some(module_name)
                                && reachable.name == function.name
                                && reachable.node_id == function.node_id
                        })
                })
        })
        .map(quarantined_import_proof_function)
        .collect()
}

fn quarantined_import_proof_function(function: &Function) -> Function {
    let mut proof = function.clone();
    proof.contracts.clear();
    proof.return_type = Some("()".to_string());
    for param in &mut proof.params {
        param.ty = Some("()".to_string());
    }
    proof.body = vec![BodyLine {
        node_id: function.node_id,
        kind: BodyLineKind::Expr {
            expr: Expr {
                node_id: function.node_id,
                kind: ExprKind::Unit,
                span: function.span.clone(),
            },
        },
        span: function.span.clone(),
    }];
    proof
}
