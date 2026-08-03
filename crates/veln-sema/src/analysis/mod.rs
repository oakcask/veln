use std::collections::BTreeMap;

#[path = "../holes.rs"]
mod holes;

use veln_ast::{
    BinaryOp, BodyLineKind, ContractKind, DictEntry, Expr, ExprKind, Function, FunctionKind,
    IfBranch, MatchArm, NodeId, Pattern, PatternKind, RecordField, SatisfyClause, SurfaceModule,
    Visibility,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::adt::{self, AdtVariantKind, ConstructorLookup};
use crate::contracts::{
    ContractCall, ContractValidation, contract_calls, contract_kind_text,
    contract_predicate_is_statically_true, is_contract_keyword, missing_contract_field,
    predicate_is_boolean_with_calls, predicate_is_statically_false, predicate_is_statically_true,
    predicate_is_statically_true_with_literal_bounds, predicate_rendered_type_with_calls,
    predicate_type_with_calls, referenced_names,
};
use crate::diagnostics::{
    contract_details, effect_details, effect_missing_public_details, handler_details,
    module_details, span_json, type_details,
};
use crate::effects::KNOWN_EFFECT_LABELS;
use crate::prelude::{
    float_arithmetic_prelude_name, float_comparison_prelude_name, float_prefix_prelude_name,
    prelude_signature, prelude_signature_with_input,
    qualified_prelude_builtin_signature_with_input, qualified_prelude_signature,
    qualified_prelude_signature_with_input,
};
use crate::repair_candidates::{
    APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED, APPLICATION_STATUS_UNAPPLIED,
    CANDIDATE_STATUS_QUERY_ONLY, SATISFY_STATUS_BLOCKED_UNTIL_DISCHARGED,
    SATISFY_STATUS_STATICALLY_SATISFIED, application_policy, candidate_blocking_obligations,
    candidate_evidence, candidate_known_limits, candidate_satisfy_status,
};
use crate::semantic_model::{
    Binding, CallOrigin, EffectUse, ExpectedType, ExpectedTypeSource, Type,
};
use crate::standard_symbols::prelude_symbol;
use crate::type_relations::is_assignable;
use crate::type_syntax::parse_type_annotation;
use crate::types::{
    CompanionAccessTarget, EffectSignature, FunctionLookup, HandlerPathResolution,
    HandlerSignature, MatchScrutineePatternInference, TypeEnvironment,
    infer_match_scrutinee_type_from_constructor_patterns,
};

mod body;
mod boundary;
mod handlers;
mod repair_reasoning;

pub(in crate::analysis) use body::FunctionChecker;
pub(crate) use body::check_function_body;
pub(crate) use boundary::{
    check_declared_effect_labels, check_duplicate_constructor_names, check_duplicate_effect_names,
    check_duplicate_function_names, check_duplicate_schema_names, check_duplicate_type_names,
    check_duplicate_use_aliases, check_module_boundary, check_public_aliases,
    check_public_function_boundary, check_reserved_prelude_aliases, check_schema_field_primitives,
    check_schema_type_references, check_test_declaration_boundary,
};
pub(crate) use handlers::check_handler_declarations;

fn private_companion_effect_target_diagnostic(
    node_id: String,
    boundary: &'static str,
    effect_path: &str,
    effect: &EffectSignature,
    access: &CompanionAccessTarget,
    span: SourceSpan,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "effect.private_companion_target",
        Severity::Error,
        DiagnosticKind::Effect,
        format!(
            "private effect `{effect_path}` belongs to `{}` instead of companion target `{}`",
            effect.module_name.clone().unwrap_or_default(),
            access.target_module
        ),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("effect")),
            ("node_id", JsonValue::string(node_id)),
            ("boundary", JsonValue::string(boundary)),
            ("effect", JsonValue::string(effect_path.to_string())),
            (
                "resolved_effect",
                JsonValue::string(effect.qualified_name.clone()),
            ),
            (
                "companion_path",
                JsonValue::string(access.companion_path.clone()),
            ),
            (
                "companion_target_module",
                JsonValue::string(access.target_module.clone()),
            ),
            (
                "effect_module",
                JsonValue::string(effect.module_name.clone().unwrap_or_default()),
            ),
            ("reason", JsonValue::string("companion_target_mismatch")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("companion_target")),
        (
            "message",
            JsonValue::string(format!(
                "This test companion may access private effects only from target module `{}`.",
                access.target_module
            )),
        ),
        (
            "companion_path",
            JsonValue::string(access.companion_path.clone()),
        ),
        (
            "companion_target_module",
            JsonValue::string(access.target_module.clone()),
        ),
    ]));
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("effect_declaration")),
        (
            "message",
            JsonValue::string(format!(
                "Private effect `{}` is declared here.",
                effect.qualified_name
            )),
        ),
        ("effect", JsonValue::string(effect.qualified_name.clone())),
        ("span", span_json(&effect.span)),
    ]));
    diagnostic
}

fn private_companion_handler_target_diagnostic(
    node_id: String,
    boundary: &'static str,
    handler_path: &str,
    handler: &HandlerSignature,
    access: &CompanionAccessTarget,
    span: SourceSpan,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "handler.private_companion_target",
        Severity::Error,
        DiagnosticKind::Effect,
        format!(
            "private handler `{handler_path}` belongs to `{}` instead of companion target `{}`",
            handler.module_name.clone().unwrap_or_default(),
            access.target_module
        ),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("effect")),
            ("node_id", JsonValue::string(node_id)),
            ("boundary", JsonValue::string(boundary)),
            ("handler", JsonValue::string(handler_path.to_string())),
            (
                "resolved_handler",
                JsonValue::string(handler.qualified_name.clone()),
            ),
            (
                "companion_path",
                JsonValue::string(access.companion_path.clone()),
            ),
            (
                "companion_target_module",
                JsonValue::string(access.target_module.clone()),
            ),
            (
                "handler_module",
                JsonValue::string(handler.module_name.clone().unwrap_or_default()),
            ),
            ("reason", JsonValue::string("companion_target_mismatch")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("companion_target")),
        (
            "message",
            JsonValue::string(format!(
                "This test companion may access private handlers only from target module `{}`.",
                access.target_module
            )),
        ),
        (
            "companion_path",
            JsonValue::string(access.companion_path.clone()),
        ),
        (
            "companion_target_module",
            JsonValue::string(access.target_module.clone()),
        ),
    ]));
    diagnostic
}
