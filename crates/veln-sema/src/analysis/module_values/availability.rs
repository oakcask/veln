use std::collections::{BTreeSet, HashMap};

use veln_ast::{Expr, ExprKind, SurfaceModule};

use super::{check_module_value, colliding_module_value_identities, module_value_indices};
use crate::analysis::VariantDiagnosticInterner;
use crate::types::{ModuleValueIdentity, TypeEnvironment};

type ReverseDependencies = HashMap<ModuleValueIdentity, Vec<usize>>;

pub(super) fn stabilize_module_value_availability(
    module: &SurfaceModule,
    environment: &mut TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
    mut unavailable: BTreeSet<ModuleValueIdentity>,
    mut pending_dependencies: BTreeSet<ModuleValueIdentity>,
) -> BTreeSet<ModuleValueIdentity> {
    let mut ever_unavailable = unavailable.clone();
    record_retained_history_insertions(ever_unavailable.len());
    let reverse_dependencies = reverse_dependencies(module, environment);
    let colliding = colliding_module_value_identities(module);
    let value_indices = module_value_indices(module, &colliding);
    record_peak_availability_work(&pending_dependencies, &unavailable, &ever_unavailable);

    while let Some(changed) = pending_dependencies.pop_first() {
        let Some(dependents) = reverse_dependencies.get(&changed) else {
            continue;
        };
        for index in dependents {
            record_declaration_recheck_lookup();
            let value = &module.module_values[*index];
            let Some(identity) = value.name.as_ref().map(|name| {
                (
                    value.package_name.clone(),
                    value.module_name.clone(),
                    name.clone(),
                )
            }) else {
                continue;
            };
            if unavailable.contains(&identity) {
                continue;
            }
            let mut dependencies = Vec::new();
            let mut diagnostics = Vec::new();
            #[cfg(test)]
            super::update_module_value_availability_work(|work| work.semantic_visits += 1);
            if !check_module_value(
                value,
                *index,
                &value_indices,
                environment,
                variant_diagnostics,
                &mut dependencies,
                &mut diagnostics,
            ) {
                continue;
            }
            unavailable.insert(identity.clone());
            if ever_unavailable.insert(identity.clone()) {
                record_retained_history_insertions(1);
            }
            let _ = environment.set_module_value_availability(&identity, false);
            pending_dependencies.insert(identity);
        }
        record_peak_availability_work(&pending_dependencies, &unavailable, &ever_unavailable);
    }

    unavailable
}

pub(super) fn apply_availability_changes(
    current: &mut BTreeSet<ModuleValueIdentity>,
    next: &BTreeSet<ModuleValueIdentity>,
    environment: &mut TypeEnvironment,
    pending_dependencies: &mut BTreeSet<ModuleValueIdentity>,
) {
    let changes = current
        .symmetric_difference(next)
        .cloned()
        .collect::<Vec<_>>();
    for identity in changes {
        let available = !next.contains(&identity);
        let _ = environment.set_module_value_availability(&identity, available);
        pending_dependencies.insert(identity);
    }
    current.clone_from(next);
}

fn reverse_dependencies(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> ReverseDependencies {
    let mut reverse = HashMap::<ModuleValueIdentity, BTreeSet<usize>>::new();
    for (index, value) in module.module_values.iter().enumerate() {
        collect_reverse_dependencies(
            &value.initializer,
            value.module_name.as_deref(),
            value.package_name.as_deref(),
            index,
            environment,
            &mut reverse,
        );
    }
    reverse
        .into_iter()
        .map(|(identity, dependents)| (identity, dependents.into_iter().collect()))
        .collect()
}

fn collect_reverse_dependencies(
    expr: &Expr,
    owner_module: Option<&str>,
    owner_package: Option<&str>,
    dependent: usize,
    environment: &TypeEnvironment,
    reverse: &mut HashMap<ModuleValueIdentity, BTreeSet<usize>>,
) {
    if let ExprKind::NamePath { segments, .. } = &expr.kind
        && let Some(identity) = environment.module_value_dependency_identity_in_owner(
            segments,
            owner_module,
            owner_package,
        )
    {
        let inserted = reverse.entry(identity).or_default().insert(dependent);
        if inserted {
            record_dependency_index_entry();
        }
    }
    expr.for_each_child(&mut |child| {
        collect_reverse_dependencies(
            child,
            owner_module,
            owner_package,
            dependent,
            environment,
            reverse,
        );
    });
}

#[cfg(test)]
fn record_peak_availability_work(
    pending_dependencies: &BTreeSet<ModuleValueIdentity>,
    unavailable: &BTreeSet<ModuleValueIdentity>,
    ever_unavailable: &BTreeSet<ModuleValueIdentity>,
) {
    super::update_module_value_availability_work(|work| {
        work.peak_pending_owners = work.peak_pending_owners.max(pending_dependencies.len());
        work.peak_retained_availability_identities = work
            .peak_retained_availability_identities
            .max(unavailable.len() + ever_unavailable.len());
    });
}

#[cfg(not(test))]
fn record_peak_availability_work(
    _pending_dependencies: &BTreeSet<ModuleValueIdentity>,
    _unavailable: &BTreeSet<ModuleValueIdentity>,
    _ever_unavailable: &BTreeSet<ModuleValueIdentity>,
) {
}

#[cfg(test)]
fn record_dependency_index_entry() {
    super::update_module_value_availability_work(|work| work.dependency_index_entries += 1);
}

#[cfg(not(test))]
fn record_dependency_index_entry() {}

#[cfg(test)]
fn record_declaration_recheck_lookup() {
    super::update_module_value_availability_work(|work| work.declaration_recheck_lookups += 1);
}

#[cfg(not(test))]
fn record_declaration_recheck_lookup() {}

#[cfg(test)]
fn record_retained_history_insertions(count: usize) {
    super::update_module_value_availability_work(|work| {
        work.retained_history_insertions += count;
    });
}

#[cfg(not(test))]
fn record_retained_history_insertions(_count: usize) {}
