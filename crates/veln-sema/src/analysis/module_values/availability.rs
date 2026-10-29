use std::collections::{BTreeSet, HashMap};

use veln_ast::{Expr, ExprKind, SurfaceModule};

use super::{check_module_value, colliding_module_value_identities, module_value_indices};
use crate::analysis::VariantDiagnosticInterner;
use crate::types::{
    BareModuleValueExposureIdentity, ModuleValueDependency, ModuleValueIdentity, TypeEnvironment,
};

pub(super) struct ReverseDependencies {
    identities: HashMap<ModuleValueIdentity, Vec<usize>>,
    bare_exposures: HashMap<BareModuleValueExposureIdentity, BareExposureDependencies>,
}

struct BareExposureDependencies {
    candidates: Vec<ModuleValueIdentity>,
    dependents: Vec<usize>,
}

impl ReverseDependencies {
    fn retained_entry_count(&self) -> usize {
        self.identities.values().map(Vec::len).sum::<usize>()
            + self
                .bare_exposures
                .values()
                .map(|dependencies| dependencies.candidates.len() + dependencies.dependents.len())
                .sum::<usize>()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AvailabilityChange {
    Identity(ModuleValueIdentity),
    BareExposure(BareModuleValueExposureIdentity),
}

pub(super) struct StabilizedAvailability {
    pub(super) unavailable: BTreeSet<ModuleValueIdentity>,
    pub(super) rejected: BTreeSet<ModuleValueIdentity>,
    pub(super) recurrent_diagnostics: Vec<veln_diagnostics::Diagnostic>,
}

struct CyclicDependencyComponents {
    component_by_identity: HashMap<ModuleValueIdentity, usize>,
    components: Vec<BTreeSet<ModuleValueIdentity>>,
}

impl CyclicDependencyComponents {
    fn get(&self, identity: &ModuleValueIdentity) -> Option<&BTreeSet<ModuleValueIdentity>> {
        self.component_by_identity
            .get(identity)
            .and_then(|index| self.components.get(*index))
    }

    fn retained_identity_count(&self) -> usize {
        self.component_by_identity.len() + self.components.iter().map(BTreeSet::len).sum::<usize>()
    }
}

pub(super) fn stabilize_module_value_availability(
    module: &SurfaceModule,
    environment: &mut TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
    mut unavailable: BTreeSet<ModuleValueIdentity>,
    mut pending_dependencies: BTreeSet<AvailabilityChange>,
    reverse_dependencies: &ReverseDependencies,
    intrinsic_invalid: &BTreeSet<ModuleValueIdentity>,
) -> StabilizedAvailability {
    let mut ever_unavailable = unavailable.clone();
    let mut recurrently_unavailable = BTreeSet::new();
    let mut recurrent_diagnostics = Vec::new();
    let cyclic_components = cyclic_dependency_components(module, reverse_dependencies);
    record_retained_cyclic_component_identities(cyclic_components.retained_identity_count());
    record_retained_history_insertions(ever_unavailable.len());
    let colliding = colliding_module_value_identities(module);
    let value_indices = module_value_indices(module, &colliding);
    record_peak_availability_work(&pending_dependencies, &unavailable, &ever_unavailable);

    while !pending_dependencies.is_empty() {
        let changes = std::mem::take(&mut pending_dependencies);
        let mut dirty_declarations = BTreeSet::new();
        for changed in changes {
            let dependents = match &changed {
                AvailabilityChange::Identity(identity) => {
                    reverse_dependencies.identities.get(identity)
                }
                AvailabilityChange::BareExposure(exposure) => reverse_dependencies
                    .bare_exposures
                    .get(exposure)
                    .map(|dependencies| &dependencies.dependents),
            };
            if let Some(dependents) = dependents {
                dirty_declarations.extend(dependents.iter().copied());
            }
        }
        for index in dirty_declarations {
            record_declaration_recheck_lookup();
            let value = &module.module_values[index];
            let Some(identity) = value.name.as_ref().map(|name| {
                (
                    value.package_name.clone(),
                    value.module_name.clone(),
                    name.clone(),
                )
            }) else {
                continue;
            };
            if intrinsic_invalid.contains(&identity) || recurrently_unavailable.contains(&identity)
            {
                continue;
            }
            let mut dependencies = Vec::new();
            let mut diagnostics = Vec::new();
            #[cfg(test)]
            super::update_module_value_availability_work(|work| work.semantic_visits += 1);
            let mut invalid = check_module_value(
                value,
                index,
                &value_indices,
                environment,
                variant_diagnostics,
                &mut dependencies,
                &mut diagnostics,
            );
            // Same-module reads reserve unavailable names without producing a
            // lookup diagnostic, so their dependency status must be checked too.
            invalid |= dependencies.iter().any(|dependency| {
                let target = &module.module_values[dependency.target];
                target.name.as_ref().is_some_and(|name| {
                    unavailable.contains(&(
                        target.package_name.clone(),
                        target.module_name.clone(),
                        name.clone(),
                    ))
                })
            });
            if invalid == unavailable.contains(&identity) {
                continue;
            }
            if invalid {
                // A declaration that fails, recovers, and then fails again can
                // participate in a lookup graph with no Boolean fixed point.
                // Quarantine that recurring failure locally. Each declaration
                // can therefore recover once, while a non-convergent component
                // has a finite number of state transitions.
                if ever_unavailable.contains(&identity)
                    && let Some(component) = cyclic_components.get(&identity)
                {
                    recurrent_diagnostics.extend(diagnostics);
                    for rejected in component {
                        recurrently_unavailable.insert(rejected.clone());
                        unavailable.insert(rejected.clone());
                        if ever_unavailable.insert(rejected.clone()) {
                            record_retained_history_insertions(1);
                        }
                        queue_availability_change(
                            rejected,
                            false,
                            environment,
                            &mut pending_dependencies,
                        );
                    }
                    continue;
                }
                unavailable.insert(identity.clone());
                if ever_unavailable.insert(identity.clone()) {
                    record_retained_history_insertions(1);
                }
            } else {
                unavailable.remove(&identity);
            }
            queue_availability_change(&identity, !invalid, environment, &mut pending_dependencies);
        }
        record_peak_availability_work(&pending_dependencies, &unavailable, &ever_unavailable);
    }

    StabilizedAvailability {
        unavailable,
        rejected: recurrently_unavailable,
        recurrent_diagnostics,
    }
}

fn cyclic_dependency_components(
    module: &SurfaceModule,
    reverse_dependencies: &ReverseDependencies,
) -> CyclicDependencyComponents {
    let identities = module
        .module_values
        .iter()
        .map(|value| {
            value.name.as_ref().map(|name| {
                (
                    value.package_name.clone(),
                    value.module_name.clone(),
                    name.clone(),
                )
            })
        })
        .collect::<Vec<_>>();
    let indices = identities
        .iter()
        .enumerate()
        .filter_map(|(index, identity)| Some((identity.clone()?, index)))
        .collect::<HashMap<_, _>>();
    let mut graph = vec![Vec::new(); module.module_values.len()];
    for (dependency, dependents) in &reverse_dependencies.identities {
        let Some(source) = indices.get(dependency) else {
            continue;
        };
        for index in dependents {
            graph[*source].push(*index);
        }
    }
    for dependencies in reverse_dependencies.bare_exposures.values() {
        let exposure = graph.len();
        graph.push(dependencies.dependents.clone());
        for candidate in &dependencies.candidates {
            if let Some(source) = indices.get(candidate) {
                graph[*source].push(exposure);
            }
        }
    }
    let retained_graph_state = graph.len() + graph.iter().map(Vec::len).sum::<usize>();
    record_peak_retained_dependency_state(
        reverse_dependencies.retained_entry_count() + retained_graph_state,
    );

    let mut component_by_identity = HashMap::new();
    let mut components = Vec::new();
    for component in super::strongly_connected_index_components(&graph) {
        let cyclic = component.len() > 1
            || component
                .first()
                .is_some_and(|index| graph[*index].contains(index));
        if !cyclic {
            continue;
        }
        let identities = component
            .into_iter()
            .filter_map(|index| identities.get(index).cloned().flatten())
            .collect::<BTreeSet<_>>();
        let component_index = components.len();
        for identity in &identities {
            component_by_identity.insert(identity.clone(), component_index);
        }
        components.push(identities);
    }
    CyclicDependencyComponents {
        component_by_identity,
        components,
    }
}

pub(super) fn apply_availability_changes(
    current: &mut BTreeSet<ModuleValueIdentity>,
    next: &BTreeSet<ModuleValueIdentity>,
    environment: &mut TypeEnvironment,
    pending_dependencies: &mut BTreeSet<AvailabilityChange>,
) {
    let changes = current
        .symmetric_difference(next)
        .cloned()
        .collect::<Vec<_>>();
    for identity in changes {
        let available = !next.contains(&identity);
        queue_availability_change(&identity, available, environment, pending_dependencies);
    }
    current.clone_from(next);
}

fn queue_availability_change(
    identity: &ModuleValueIdentity,
    available: bool,
    environment: &mut TypeEnvironment,
    pending_dependencies: &mut BTreeSet<AvailabilityChange>,
) {
    let owners = environment.set_module_value_availability(identity, available);
    pending_dependencies.insert(AvailabilityChange::Identity(identity.clone()));
    for (package, module) in owners {
        pending_dependencies.insert(AvailabilityChange::BareExposure((
            package,
            module,
            identity.2.clone(),
        )));
    }
}

pub(super) fn reverse_dependencies(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> ReverseDependencies {
    let mut identities = HashMap::<ModuleValueIdentity, BTreeSet<usize>>::new();
    let mut bare_exposures = HashMap::<BareModuleValueExposureIdentity, BTreeSet<usize>>::new();
    for (index, value) in module.module_values.iter().enumerate() {
        collect_reverse_dependencies(
            &value.initializer,
            value.module_name.as_deref(),
            value.package_name.as_deref(),
            index,
            environment,
            &mut identities,
            &mut bare_exposures,
        );
    }
    let identities = identities
        .into_iter()
        .map(|(identity, dependents)| (identity, dependents.into_iter().collect()))
        .collect();
    let bare_exposures = bare_exposures
        .into_iter()
        .map(|(exposure, dependents)| {
            let candidates = environment
                .module_value_candidates_for_bare_exposure(&exposure)
                .into_iter()
                .collect::<Vec<_>>();
            record_dependency_index_entries(candidates.len());
            (
                exposure,
                BareExposureDependencies {
                    candidates,
                    dependents: dependents.into_iter().collect(),
                },
            )
        })
        .collect();
    ReverseDependencies {
        identities,
        bare_exposures,
    }
}

fn collect_reverse_dependencies(
    expr: &Expr,
    owner_module: Option<&str>,
    owner_package: Option<&str>,
    dependent: usize,
    environment: &TypeEnvironment,
    identities: &mut HashMap<ModuleValueIdentity, BTreeSet<usize>>,
    bare_exposures: &mut HashMap<BareModuleValueExposureIdentity, BTreeSet<usize>>,
) {
    if let ExprKind::NamePath { segments, .. } = &expr.kind {
        let dependency =
            environment.module_value_dependency_in_owner(segments, owner_module, owner_package);
        if let Some(dependency) = dependency {
            let dependents = match dependency {
                ModuleValueDependency::Identity(identity) => {
                    identities.entry(identity).or_default()
                }
                ModuleValueDependency::BareExposure(exposure) => {
                    bare_exposures.entry(exposure).or_default()
                }
            };
            let inserted = dependents.insert(dependent);
            if inserted {
                record_dependency_index_entry();
            }
        }
    }
    expr.for_each_child(&mut |child| {
        collect_reverse_dependencies(
            child,
            owner_module,
            owner_package,
            dependent,
            environment,
            identities,
            bare_exposures,
        );
    });
}

#[cfg(test)]
fn record_peak_availability_work(
    pending_dependencies: &BTreeSet<AvailabilityChange>,
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

#[cfg(test)]
fn record_retained_cyclic_component_identities(count: usize) {
    super::update_module_value_availability_work(|work| {
        work.retained_cyclic_component_identities = count;
    });
}

#[cfg(not(test))]
fn record_retained_cyclic_component_identities(_count: usize) {}

#[cfg(not(test))]
fn record_peak_availability_work(
    _pending_dependencies: &BTreeSet<AvailabilityChange>,
    _unavailable: &BTreeSet<ModuleValueIdentity>,
    _ever_unavailable: &BTreeSet<ModuleValueIdentity>,
) {
}

#[cfg(test)]
fn record_dependency_index_entry() {
    super::update_module_value_availability_work(|work| work.dependency_index_entries += 1);
}

#[cfg(test)]
fn record_dependency_index_entries(count: usize) {
    super::update_module_value_availability_work(|work| work.dependency_index_entries += count);
}

#[cfg(test)]
fn record_peak_retained_dependency_state(count: usize) {
    super::update_module_value_availability_work(|work| {
        work.peak_retained_dependency_state = work.peak_retained_dependency_state.max(count);
    });
}

#[cfg(not(test))]
fn record_dependency_index_entry() {}

#[cfg(not(test))]
fn record_dependency_index_entries(_count: usize) {}

#[cfg(not(test))]
fn record_peak_retained_dependency_state(_count: usize) {}

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
