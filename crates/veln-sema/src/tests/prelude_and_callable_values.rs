use super::*;
use crate::schema::ir::schema_decode_specs;
use crate::schema::primitives::{SchemaRepeatPayload, repeat_schema_primitive};
use crate::semantic_model::Type;
use crate::types::schema_types::{schema_decode_value_type, schema_encode_value_type};

const FORMAT_NEUTRAL_HELPER_SUPPORTED: &str = "recursive format-neutral visible shape made from scalar leaves, anonymous record fields, Option<T>, List<T>, Vec<T>, Dict<String, T>, Result<recursive visible shape, recursive visible shape>, or same-module or public imported source ADTs whose constructor payloads are recursive visible shapes";

#[derive(Clone, Copy)]
enum ExpectedSchemaBoundary {
    DecodeStep,
    Encode,
}

fn assert_named_schema_boundary(
    lowered: &LoweredSurfaceModule,
    function_name: &str,
    schema_name: &str,
    expected: ExpectedSchemaBoundary,
) {
    let expected_argument_count = match expected {
        ExpectedSchemaBoundary::DecodeStep => 2,
        ExpectedSchemaBoundary::Encode => 1,
    };
    let core = lowered.core.as_ref().expect("checked core should be built");
    let function = core
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .unwrap_or_else(|| panic!("{function_name} should be lowered"));
    let CoreStmtKind::Return { expr } = &function.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { target, args } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert_eq!(args.len(), expected_argument_count);
    assert!(
        match (expected, target) {
            (ExpectedSchemaBoundary::DecodeStep, CoreCallTarget::SchemaDecodeStep(name))
            | (ExpectedSchemaBoundary::Encode, CoreCallTarget::SchemaEncode(name)) =>
                name == schema_name,
            _ => false,
        },
        "unexpected core call target: {target:?}"
    );

    let ir = lowered.ir.as_ref().expect("typed IR should be built");
    let function = ir
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .unwrap_or_else(|| panic!("{function_name} should be in IR"));
    let IrStmtKind::Return { value } = &function.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Call { target, args } = &value.kind else {
        panic!("tail expression should lower as IR call");
    };
    assert_eq!(args.len(), expected_argument_count);
    assert!(
        match (expected, target) {
            (ExpectedSchemaBoundary::DecodeStep, IrCallTarget::SchemaDecodeStep(name))
            | (ExpectedSchemaBoundary::Encode, IrCallTarget::SchemaEncode(name)) =>
                name == schema_name,
            _ => false,
        },
        "unexpected IR call target: {target:?}"
    );
}

fn assert_main_schema_boundary(
    source: &SourceFile,
    schema_name: &str,
    expected: ExpectedSchemaBoundary,
) {
    let parsed = parse(source);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert_named_schema_boundary(&lowered, "main", schema_name, expected);
}

mod byte_view_and_repeat_expressions;
mod callable_values_and_record_fields;
mod callback_diagnostics;
mod callback_expected_types;
mod callback_inference_contexts;
mod codec_derivation_reserved;
mod derived_repeat_boundaries;
mod dispatch_and_derived_encoding;
mod dispatch_metadata;
mod explicit_schema_operations;
mod format_neutral_decode_boundaries;
mod format_neutral_encode_containers;
mod format_neutral_source_adts;
mod imported_codecs_and_prelude_inference;
mod network_system_prelude_inference;
mod packed_reserved_suffix;
mod repeat_lengths_and_reserved_prefix;
mod repeat_operand_rejections;
mod reserved_prefix_groups;
mod reserved_shapes_and_visible_primitives;
mod reserved_split_groups;
mod schema_composition;
mod schema_composition_imports;
mod schema_validation_and_repeats;
mod standard_prelude_and_shadowing;
