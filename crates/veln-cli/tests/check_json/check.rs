pub(super) use super::support::*;

#[path = "check/contracts_and_repairs.rs"]
mod contracts_and_repairs;
#[path = "check/core_execution.rs"]
mod core_execution;
#[path = "check/parsing_and_recovery.rs"]
mod parsing_and_recovery;
#[path = "check/selection_and_manifests.rs"]
mod selection_and_manifests;
#[path = "check/types_and_effects.rs"]
mod types_and_effects;
