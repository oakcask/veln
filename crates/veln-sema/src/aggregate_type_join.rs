use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::sync::Arc;

use crate::adt::descriptors::{AdtConstructor, Variance};
use crate::adt::registry::{AdtRegistry, VariantDeclarationOrder};
use crate::adt::{type_operations as adt, unification};
use crate::semantic_model::Type;
use crate::type_relations::{
    TypePresentationJoin, invariant_args_match, is_assignable_nested_with_registry,
    record_field_match_work, same_type_identity,
};

include!("aggregate_type_join/state.rs");
include!("aggregate_type_join/bounded_join_state.rs");
include!("aggregate_type_join/join_tree.rs");
include!("aggregate_type_join/join_materialization.rs");
include!("aggregate_type_join/constructor_constraints.rs");
include!("aggregate_type_join/constraint_traversal.rs");
include!("aggregate_type_join/bounded_type_resolution.rs");
include!("aggregate_type_join/bound_candidate_selection.rs");
include!("aggregate_type_join/structural_admission.rs");
include!("aggregate_type_join/candidate_canonicalization.rs");
