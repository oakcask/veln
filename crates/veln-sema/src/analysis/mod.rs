use std::collections::BTreeMap;

#[path = "../holes.rs"]
mod holes;

use veln_ast::{
    BinaryOp, BodyLineKind, ContractKind, DictEntry, Expr, ExprKind, Function, FunctionKind,
    MatchArm, NodeId, Pattern, PatternKind, RecordField, SatisfyClause, SurfaceModule, Visibility,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::adt::{self, ConstructorLookup};
use crate::contracts::{
    ContractCall, ContractValidation, contract_calls, contract_kind_text,
    contract_predicate_is_statically_true, is_contract_keyword, missing_contract_field,
    predicate_is_boolean_with_calls, predicate_is_statically_false, predicate_is_statically_true,
    predicate_is_statically_true_with_literal_bounds, predicate_rendered_type_with_calls,
    predicate_type_with_calls, referenced_names,
};
use crate::diagnostics::{
    contract_details, effect_details, effect_missing_public_details, module_details, span_json,
    type_details,
};
use crate::effects::KNOWN_EFFECT_LABELS;
use crate::prelude::{
    float_arithmetic_prelude_name, float_comparison_prelude_name, float_prefix_prelude_name,
    prelude_signature, qualified_prelude_builtin_signature, qualified_prelude_signature,
};
use crate::repair_candidates::{
    APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED, APPLICATION_STATUS_UNAPPLIED,
    CANDIDATE_STATUS_QUERY_ONLY, SATISFY_STATUS_BLOCKED_UNTIL_DISCHARGED,
    SATISFY_STATUS_STATICALLY_SATISFIED, application_policy, candidate_blocking_obligations,
    candidate_evidence, candidate_known_limits, candidate_satisfy_status,
};
use crate::standard_symbols::prelude_symbol;
use crate::types::{
    Binding, CallOrigin, EffectUse, ExpectedType, ExpectedTypeSource, FunctionLookup, Type,
    TypeEnvironment, is_assignable, parse_type_annotation,
};

mod body;
mod boundary;
mod repair_reasoning;

pub(in crate::analysis) use body::FunctionChecker;
pub(crate) use body::check_function_body;
pub(crate) use boundary::{
    check_declared_effect_labels, check_duplicate_constructor_names,
    check_duplicate_function_names, check_duplicate_type_names, check_duplicate_use_aliases,
    check_module_boundary, check_public_aliases, check_public_function_boundary,
    check_reserved_prelude_aliases, check_schema_type_references, check_test_declaration_boundary,
};
