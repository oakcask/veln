use super::*;
use crate::adt::AdtRegistry;
use crate::name_recovery::public_alias_has_invalid_target_leaf;
use crate::schema::dispatch::{
    SchemaDispatchCase, SchemaDispatchCasePayload, SchemaDispatchSpec,
    closed_dispatch_schema_primitive, extension_dispatch_schema_primitive,
    lowercase_schema_primitive_nested_payloads,
    schema_dispatch_payload_accepts_lowercase_primitive,
};
use crate::schema::primitives::{
    ByteViewLengthExpr, LowercaseSchemaPrimitive, LowercaseSchemaPrimitiveError,
    SchemaRepeatPayload, SchemaRepeatSpec, byte_view_multiple_constraint,
    byte_view_schema_primitive, exact_width_schema_primitive,
    exact_width_schema_primitive_bit_width, lowercase_reserved_bits_schema_primitive,
    lowercase_schema_primitive, repeat_schema_primitive, reserved_bits_schema_primitive,
    schema_field_reference_type, schema_length_expression_references,
    schema_payload_name_last_segment, schema_payload_name_path,
    schema_repeat_payload_accepts_lowercase_primitive,
};
use crate::schema::reserved_layout::{
    schema_field_uses_generalized_reserved_byte_prefix,
    schema_payload_has_generalized_reserved_byte_prefix, supported_encode_reserved_bits,
};
use crate::types::companion_access_targets;
use crate::types::schema_types::{
    binary_schema_anonymous_record_decode_type,
    format_neutral_schema_encode_field_is_source_adt_candidate,
    format_neutral_schema_encode_field_type_for_schema,
    format_neutral_schema_field_type_for_schema,
    recursive_dispatch_decode_only_payload_case_is_eligible,
    recursive_dispatch_payload_case_is_eligible, recursive_dispatch_payload_is_eligible,
    schema_decode_step_function_name, schema_decode_value_type,
    schema_dispatch_has_recursive_payload, schema_dispatch_payload_schema,
    schema_encode_function_name, schema_encode_value_type, schema_field_target,
    schema_field_uses_existing_grammar, schema_has_eligible_recursive_dispatch_payload,
    schema_has_recursive_dispatch_payload, schema_recursive_dispatch_helper_payload_type,
    schema_recursive_dispatch_payload_type,
};
use std::collections::{BTreeMap, BTreeSet};
use veln_ast::{
    NameClass, PublicAliasKind, SchemaDecl, SchemaField, SchemaValidationClause, UseDecl,
};
use veln_literals::parse_integer_literal;

mod effects;
mod module_boundaries;
mod names_and_aliases;
mod schema_composition;
mod schema_dispatch;
mod schema_dispatch_helpers;
mod schema_dispatch_resolution;
mod schema_entrypoints;
mod schema_payload_resolution;
mod schema_repeat;
mod schema_repeat_resolution;
mod schema_type_references;
mod schema_validation;

pub(crate) use effects::check_declared_effect_labels;
pub(crate) use module_boundaries::{
    check_duplicate_constructor_names, check_duplicate_use_aliases, check_module_boundary,
    check_reserved_prelude_aliases, check_test_declaration_boundary,
};
pub(in crate::analysis) use module_boundaries::{duplicate_name_diagnostic, type_contains_unknown};
pub(crate) use names_and_aliases::{
    check_duplicate_effect_names, check_duplicate_function_names, check_duplicate_schema_names,
    check_duplicate_type_names, check_public_aliases,
};
pub(in crate::analysis) use schema_composition::format_neutral_schema_encode_helper_diagnostic;
pub(crate) use schema_entrypoints::{check_schema_field_primitives, check_schema_type_references};
pub(in crate::analysis) use schema_type_references::{
    exact_width_binary_primitive_name, exact_width_schema_primitive_diagnostic,
    lowercase_schema_primitive_position_diagnostic,
};

use module_boundaries::*;
use names_and_aliases::*;
use schema_composition::*;
use schema_dispatch::*;
use schema_dispatch_helpers::*;
use schema_dispatch_resolution::*;
use schema_payload_resolution::*;
use schema_repeat::*;
use schema_repeat_resolution::*;
use schema_type_references::*;
use schema_validation::*;

pub(crate) fn check_public_function_boundary(function: &Function) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for param in &function.params {
        if param.ty.is_none() {
            diagnostics.push(Diagnostic::new(
                "type.public_signature_missing",
                Severity::Error,
                DiagnosticKind::Type,
                format!("public parameter `{}` has no type annotation", param.name),
                Some(param.span.clone()),
                type_details(
                    param.node_id.display("param"),
                    "explicit",
                    "missing",
                    "declared_parameter",
                    "source",
                    "assignable",
                    [function.node_id.display("fn")],
                ),
            ));
        }
    }

    if function.return_type.is_none() {
        diagnostics.push(Diagnostic::new(
            "type.public_signature_missing",
            Severity::Error,
            DiagnosticKind::Type,
            "public function has no return type annotation",
            Some(function.span.clone()),
            type_details(
                function.node_id.display("fn"),
                "explicit",
                "missing",
                "declared_return",
                "source",
                "return_value",
                [function.node_id.display("fn")],
            ),
        ));
    }

    diagnostics
}
