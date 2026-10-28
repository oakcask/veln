use std::collections::{BTreeSet, HashMap};

use veln_ast::SurfaceModule;

use super::check_module_values_once;
use crate::analysis::VariantDiagnosticInterner;
use crate::types::{ModuleValueIdentity, TypeEnvironment};

type ModuleValueOwner = (Option<String>, Option<String>);

struct ModuleValueAvailabilityState<'a> {
    unavailable: &'a mut BTreeSet<ModuleValueIdentity>,
    invalid_by_owner: &'a mut HashMap<ModuleValueOwner, BTreeSet<ModuleValueIdentity>>,
    pending_owners: &'a mut BTreeSet<ModuleValueOwner>,
    ever_unavailable: &'a mut BTreeSet<ModuleValueIdentity>,
}

pub(super) fn stabilize_module_value_availability(
    module: &SurfaceModule,
    environment: &mut TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
    mut unavailable: BTreeSet<ModuleValueIdentity>,
    mut pending_owners: BTreeSet<ModuleValueOwner>,
) -> BTreeSet<ModuleValueIdentity> {
    let mut ever_unavailable = unavailable.clone();
    record_retained_history_insertions(ever_unavailable.len());
    record_peak_availability_work(&pending_owners, &unavailable, &ever_unavailable);
    let mut invalid_by_owner = invalid_identities_by_owner(&unavailable);
    let modules_by_owner = module_values_by_owner(module);
    let mut owner_updates = 0usize;
    let update_limit = module.module_values.len().saturating_mul(4).max(1);

    while let Some(owner) = pending_owners.pop_first() {
        if !recheck_owner_availability(
            &modules_by_owner,
            &owner,
            environment,
            variant_diagnostics,
            &mut ModuleValueAvailabilityState {
                unavailable: &mut unavailable,
                invalid_by_owner: &mut invalid_by_owner,
                pending_owners: &mut pending_owners,
                ever_unavailable: &mut ever_unavailable,
            },
        ) {
            continue;
        }
        record_peak_availability_work(&pending_owners, &unavailable, &ever_unavailable);
        owner_updates += 1;
        if owner_updates > update_limit {
            unavailable = ever_unavailable;
            environment.quarantine_module_values(&unavailable);
            break;
        }
    }

    unavailable
}

fn recheck_owner_availability(
    modules_by_owner: &HashMap<ModuleValueOwner, SurfaceModule>,
    owner: &ModuleValueOwner,
    environment: &mut TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
    state: &mut ModuleValueAvailabilityState<'_>,
) -> bool {
    record_owner_recheck_lookup();
    let Some(owner_module) = modules_by_owner.get(owner) else {
        return false;
    };
    let owner_analysis = check_module_values_once(owner_module, environment, variant_diagnostics);
    apply_owner_availability_changes(
        owner,
        state.unavailable,
        state.invalid_by_owner,
        owner_analysis.invalid,
        environment,
        state.pending_owners,
        state.ever_unavailable,
    );
    true
}

pub(super) fn apply_availability_changes(
    current: &mut BTreeSet<ModuleValueIdentity>,
    next: &BTreeSet<ModuleValueIdentity>,
    environment: &mut TypeEnvironment,
    pending_owners: &mut BTreeSet<ModuleValueOwner>,
) {
    let changes = current
        .symmetric_difference(next)
        .cloned()
        .collect::<Vec<_>>();
    for identity in changes {
        let available = !next.contains(&identity);
        pending_owners.extend(environment.set_module_value_availability(&identity, available));
    }
    current.clone_from(next);
}

fn apply_owner_availability_changes(
    owner: &ModuleValueOwner,
    current: &mut BTreeSet<ModuleValueIdentity>,
    invalid_by_owner: &mut HashMap<ModuleValueOwner, BTreeSet<ModuleValueIdentity>>,
    next_owner: BTreeSet<ModuleValueIdentity>,
    environment: &mut TypeEnvironment,
    pending_owners: &mut BTreeSet<ModuleValueOwner>,
    ever_unavailable: &mut BTreeSet<ModuleValueIdentity>,
) {
    let current_owner = invalid_by_owner.entry(owner.clone()).or_default();
    let changes = current_owner
        .symmetric_difference(&next_owner)
        .cloned()
        .collect::<Vec<_>>();
    for identity in changes {
        let available = !next_owner.contains(&identity);
        if available {
            current.remove(&identity);
        } else {
            current.insert(identity.clone());
            record_retained_history_insertions(1);
            ever_unavailable.insert(identity.clone());
        }
        pending_owners.extend(environment.set_module_value_availability(&identity, available));
    }
    *current_owner = next_owner;
}

fn invalid_identities_by_owner(
    invalid: &BTreeSet<ModuleValueIdentity>,
) -> HashMap<ModuleValueOwner, BTreeSet<ModuleValueIdentity>> {
    let mut by_owner = HashMap::new();
    for identity in invalid {
        by_owner
            .entry((identity.0.clone(), identity.1.clone()))
            .or_insert_with(BTreeSet::new)
            .insert(identity.clone());
    }
    by_owner
}

fn module_values_by_owner(module: &SurfaceModule) -> HashMap<ModuleValueOwner, SurfaceModule> {
    let mut owners = HashMap::new();
    add_aliases_by_owner(&mut owners, module);
    add_functions_by_owner(&mut owners, module);
    add_values_by_owner(&mut owners, module);
    owners
}

fn add_aliases_by_owner(
    owners: &mut HashMap<ModuleValueOwner, SurfaceModule>,
    module: &SurfaceModule,
) {
    for alias in &module.aliases {
        record_owner_index_entry();
        owner_module(
            owners,
            module,
            (alias.package_name.clone(), alias.module_name.clone()),
        )
        .aliases
        .push(alias.clone());
    }
}

fn add_functions_by_owner(
    owners: &mut HashMap<ModuleValueOwner, SurfaceModule>,
    module: &SurfaceModule,
) {
    for function in &module.functions {
        record_owner_index_entry();
        owner_module(
            owners,
            module,
            (function.package_name.clone(), function.module_name.clone()),
        )
        .functions
        .push(function.clone());
    }
}

fn add_values_by_owner(
    owners: &mut HashMap<ModuleValueOwner, SurfaceModule>,
    module: &SurfaceModule,
) {
    let invalid_names_by_span = module
        .invalid_names
        .iter()
        .map(|invalid| {
            record_owner_index_entry();
            (
                (
                    invalid.span.file.clone(),
                    invalid.span.start.offset,
                    invalid.span.end.offset,
                ),
                invalid,
            )
        })
        .collect::<HashMap<_, _>>();

    for value in &module.module_values {
        record_owner_index_entry();
        let owner = owner_module(
            owners,
            module,
            (value.package_name.clone(), value.module_name.clone()),
        );
        owner.module_values.push(value.clone());
        let Some(name_span) = &value.name_span else {
            continue;
        };
        let key = (
            name_span.file.clone(),
            name_span.start.offset,
            name_span.end.offset,
        );
        if let Some(invalid) = invalid_names_by_span.get(&key) {
            record_owner_index_entry();
            owner.invalid_names.push((*invalid).clone());
        }
    }
}

fn owner_module<'a>(
    owners: &'a mut HashMap<ModuleValueOwner, SurfaceModule>,
    module: &SurfaceModule,
    owner: ModuleValueOwner,
) -> &'a mut SurfaceModule {
    owners.entry(owner).or_insert_with(|| SurfaceModule {
        module: module.module.clone(),
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        module_values: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    })
}

#[cfg(test)]
fn record_peak_availability_work(
    pending_owners: &BTreeSet<ModuleValueOwner>,
    unavailable: &BTreeSet<ModuleValueIdentity>,
    ever_unavailable: &BTreeSet<ModuleValueIdentity>,
) {
    super::update_module_value_availability_work(|work| {
        work.peak_pending_owners = work.peak_pending_owners.max(pending_owners.len());
        work.peak_retained_availability_identities = work
            .peak_retained_availability_identities
            .max(unavailable.len() + ever_unavailable.len());
    });
}

#[cfg(not(test))]
fn record_peak_availability_work(
    _pending_owners: &BTreeSet<ModuleValueOwner>,
    _unavailable: &BTreeSet<ModuleValueIdentity>,
    _ever_unavailable: &BTreeSet<ModuleValueIdentity>,
) {
}

#[cfg(test)]
fn record_owner_index_entry() {
    super::update_module_value_availability_work(|work| work.owner_index_entries += 1);
}

#[cfg(not(test))]
fn record_owner_index_entry() {}

#[cfg(test)]
fn record_owner_recheck_lookup() {
    super::update_module_value_availability_work(|work| work.owner_recheck_lookups += 1);
}

#[cfg(not(test))]
fn record_owner_recheck_lookup() {}

#[cfg(test)]
fn record_retained_history_insertions(count: usize) {
    super::update_module_value_availability_work(|work| work.retained_history_insertions += count);
}

#[cfg(not(test))]
fn record_retained_history_insertions(_count: usize) {}
