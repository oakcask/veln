mod effect_call_resolution;
mod effect_inference;
pub(crate) mod environment;
mod expression_effects;
pub(crate) mod private_inference;
pub(crate) mod schema_types;
mod signature_collection;
pub(crate) mod signatures;
mod standard_environment;
mod symbols;

pub(crate) use effect_call_resolution::companion_access_targets;
use effect_call_resolution::*;
use effect_inference::{EffectDependencyNode, infer_function_and_private_handler_effects};
pub(crate) use effect_inference::{canonical_user_effect_label, imported_effect_is_visible};
pub(crate) use environment::*;
use expression_effects::*;
pub(crate) use private_inference::*;
use private_inference::{
    collect_pattern_bindings, function_signature_params, function_signature_path,
};
pub(crate) use schema_types::*;
pub(crate) use signatures::*;
#[cfg(test)]
use standard_environment::is_standard_module_name;
pub use standard_environment::*;
use standard_environment::{
    application_module_is_empty, module_standard_names,
    module_without_reusable_standard_declarations, reusable_standard_module_names_for,
    selected_standard_access_targets, selected_standard_facts, valid_value_binding_name,
};
use symbols::*;

use crate::schema::*;

#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use veln_ast::{
    BodyLineKind, CodecDecl, CodecDirection, CodecImplementationKind, DictEntry, EffectDecl, Expr,
    ExprKind, Function, FunctionKind, HandlerDecl, IfBranch, InvalidName, MatchArm, PublicAlias,
    PublicAliasKind, RecordField, SchemaDecl, SchemaField, SurfaceModule, TypeDecl, UseDecl,
    Visibility, lower_surface_ast_with_module_identity,
};
use veln_project::{classify_companion_source, companion_access_target};
use veln_source::{SourceFile, SourceSpan, TextRange};

use crate::semantic_model::{Binding, FunctionKey, Type};

#[cfg(test)]
pub(crate) mod effect_inference_counters {
    use super::*;

    thread_local! {
        static DEPENDENCY_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static FUNCTION_BODY_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
        static HANDLER_OPERATION_CLAUSE_EVALUATIONS: Cell<usize> = const { Cell::new(0) };
        static CHANGED_REEVALUATIONS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(crate) struct Snapshot {
        pub(crate) dependency_discovery_scans: usize,
        pub(crate) function_body_collections: usize,
        pub(crate) handler_operation_clause_evaluations: usize,
        pub(crate) changed_reevaluations: usize,
    }

    pub(crate) fn reset() {
        DEPENDENCY_DISCOVERY_SCANS.set(0);
        FUNCTION_BODY_COLLECTIONS.set(0);
        HANDLER_OPERATION_CLAUSE_EVALUATIONS.set(0);
        CHANGED_REEVALUATIONS.set(0);
    }

    pub(crate) fn snapshot() -> Snapshot {
        Snapshot {
            dependency_discovery_scans: DEPENDENCY_DISCOVERY_SCANS.get(),
            function_body_collections: FUNCTION_BODY_COLLECTIONS.get(),
            handler_operation_clause_evaluations: HANDLER_OPERATION_CLAUSE_EVALUATIONS.get(),
            changed_reevaluations: CHANGED_REEVALUATIONS.get(),
        }
    }

    pub(super) fn record_dependency_discovery_scan() {
        DEPENDENCY_DISCOVERY_SCANS.set(DEPENDENCY_DISCOVERY_SCANS.get() + 1);
    }

    pub(super) fn record_function_body_collection() {
        FUNCTION_BODY_COLLECTIONS.set(FUNCTION_BODY_COLLECTIONS.get() + 1);
    }

    pub(super) fn record_handler_operation_clause_evaluation() {
        HANDLER_OPERATION_CLAUSE_EVALUATIONS.set(HANDLER_OPERATION_CLAUSE_EVALUATIONS.get() + 1);
    }

    pub(super) fn record_changed_reevaluation() {
        CHANGED_REEVALUATIONS.set(CHANGED_REEVALUATIONS.get() + 1);
    }
}

#[cfg(test)]
#[path = "types/tests.rs"]
mod tests;
