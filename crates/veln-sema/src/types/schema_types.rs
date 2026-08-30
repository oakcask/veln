use super::*;
use crate::name_recovery::normal_imported_use_for_path;

mod binary;
mod format_neutral;
mod schema_encode;

pub(crate) use binary::*;
use binary::{
    schema_decode_value_type_inner, schema_encode_dispatch_case_type, schema_repeat_payload_type,
};
pub(crate) use format_neutral::*;
use format_neutral::{
    format_neutral_schema_decode_function_signature_for_schema,
    format_neutral_schema_encode_record_fields,
};

pub(super) fn codec_call_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<CodecCallSignature> {
    module
        .codecs
        .iter()
        .flat_map(|codec| {
            let name = codec.name.clone()?;
            Some(
                codec
                    .implementations
                    .iter()
                    .flat_map(move |implementation| {
                        match (&implementation.direction, &implementation.kind) {
                            (
                                CodecDirection::Decode,
                                CodecImplementationKind::With {
                                    function: Some(function_name),
                                },
                            ) => codec_with_signature(
                                codec,
                                functions,
                                name.clone(),
                                function_name,
                                CodecCallBoundary::HandWrittenDecode,
                            )
                            .into_iter()
                            .collect(),
                            (
                                CodecDirection::Encode,
                                CodecImplementationKind::With {
                                    function: Some(function_name),
                                },
                            ) => codec_with_signature(
                                codec,
                                functions,
                                name.clone(),
                                function_name,
                                CodecCallBoundary::Direct,
                            )
                            .into_iter()
                            .collect(),
                            (CodecDirection::Decode, CodecImplementationKind::Derive) => {
                                codec_derive_decode_signature(
                                    module,
                                    functions,
                                    codec,
                                    name.clone(),
                                )
                                .into_iter()
                                .collect()
                            }
                            (CodecDirection::Encode, CodecImplementationKind::Derive) => {
                                codec_derive_encode_signatures(
                                    module,
                                    functions,
                                    codec,
                                    name.clone(),
                                )
                            }
                            (_, CodecImplementationKind::With { function: None }) => Vec::new(),
                        }
                    }),
            )
        })
        .flatten()
        .collect()
}

fn codec_with_signature(
    codec: &CodecDecl,
    functions: &[FunctionSignature],
    name: String,
    function_name: &str,
    boundary: CodecCallBoundary,
) -> Option<CodecCallSignature> {
    let function = functions.iter().find(|function| {
        function.name == function_name && function.module_name == codec.module_name
    })?;
    Some(codec_call_signature(codec, function, name, boundary))
}

fn codec_call_signature(
    codec: &CodecDecl,
    function: &FunctionSignature,
    name: String,
    boundary: CodecCallBoundary,
) -> CodecCallSignature {
    CodecCallSignature {
        name,
        target_name: function.target_name.clone(),
        boundary,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    }
}

fn codec_derive_decode_signature(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    codec: &CodecDecl,
    name: String,
) -> Option<CodecCallSignature> {
    let schema = codec_referenced_schema(module, codec)?;
    let schema_name = schema.name.as_ref()?;
    let step_name = schema_decode_step_function_name(schema_name);
    let function = functions.iter().find(|function| {
        function.name == step_name && function.module_name == schema.module_name
    })?;
    Some(codec_call_signature(
        codec,
        function,
        name,
        CodecCallBoundary::Direct,
    ))
}

fn codec_derive_encode_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    codec: &CodecDecl,
    name: String,
) -> Vec<CodecCallSignature> {
    let Some(schema) = codec_referenced_schema(module, codec) else {
        return Vec::new();
    };
    let Some(schema_name) = schema.name.as_ref() else {
        return Vec::new();
    };
    let encode_name = schema_encode_function_name(schema_name);
    let Some(function) = functions.iter().find(|function| {
        function.name == encode_name && function.module_name == schema.module_name
    }) else {
        return Vec::new();
    };
    let unbounded = CodecCallSignature {
        name,
        target_name: format!("{SCHEMA_ENCODE_STEP_TARGET_PREFIX}{schema_name}"),
        boundary: CodecCallBoundary::Direct,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: Type::named("EncodeStep", vec![Type::unit()]),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    };
    let Some(value_type) = function.params.first().cloned() else {
        return vec![unbounded];
    };
    let mut state_fields = match &value_type {
        Type::Record(fields) => fields.clone(),
        _ => Vec::new(),
    };
    state_fields.push((
        "encoded_offset".to_string(),
        Type::named("ByteCount", Vec::new()),
    ));
    let budgeted = CodecCallSignature {
        name: unbounded.name.clone(),
        target_name: unbounded.target_name.clone(),
        boundary: unbounded.boundary,
        module_name: unbounded.module_name.clone(),
        visibility: unbounded.visibility,
        params: vec![value_type, Type::named("ByteCount", Vec::new())],
        return_type: Type::named("EncodeStep", vec![Type::Record(state_fields)]),
        effects: unbounded.effects.clone(),
        node_id: unbounded.node_id,
        span: unbounded.span.clone(),
    };
    vec![unbounded, budgeted]
}

fn codec_referenced_schema<'a>(
    module: &'a SurfaceModule,
    codec: &CodecDecl,
) -> Option<&'a SchemaDecl> {
    let schema_name = codec.schema.as_ref()?;
    let segments = schema_name
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    schema_reference(
        module,
        &segments,
        codec.module_name.as_deref(),
        true,
        &mut Vec::new(),
    )
}

fn schema_reference<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> Option<&'a SchemaDecl> {
    let companion_access_targets = companion_access_targets(module);
    match segments {
        [name] => schema_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
        ),
        [_, .., name] => {
            let use_decl = normal_imported_use_for_path(
                module,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            schema_in_module(
                module,
                Some(&use_decl.name),
                name,
                companion_private_schema_access_allowed(
                    use_decl,
                    current_module,
                    &companion_access_targets,
                ),
                visited_aliases,
            )
        }
        _ => None,
    }
}

pub(crate) fn schema_field_target<'a>(
    module: &'a SurfaceModule,
    containing_schema: &SchemaDecl,
    text: &str,
) -> Option<&'a SchemaDecl> {
    if schema_field_uses_existing_grammar(containing_schema, text) {
        return None;
    }
    let segments = schema_payload_name_path(text)?;
    schema_reference(
        module,
        &segments,
        containing_schema.module_name.as_deref(),
        true,
        &mut Vec::new(),
    )
}

pub(crate) fn schema_field_uses_existing_grammar(schema: &SchemaDecl, text: &str) -> bool {
    match schema.format.as_ref().map(|format| format.name.as_str()) {
        None => matches!(text, "Int" | "Bool" | "Float" | "String"),
        Some("binary") => {
            exact_width_schema_primitive(text).is_some()
                || lowercase_reserved_bits_schema_primitive(text).is_some()
                || lowercase_schema_primitive(text).is_some()
                || !lowercase_schema_primitive_nested_payloads(text).is_empty()
                || byte_view_schema_primitive(text).is_some()
                || repeat_schema_primitive(text).is_some()
                || binary_schema_anonymous_record_decode_type(text).is_some()
                || closed_dispatch_schema_primitive(text).is_some()
                || extension_dispatch_schema_primitive(text).is_some()
                || reserved_bits_schema_primitive(text).is_some()
        }
        Some(_) => false,
    }
}

fn schema_in_module<'a>(
    module: &'a SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> Option<&'a SchemaDecl> {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return (allow_private_schema || schema.visibility == Visibility::Public).then_some(schema);
    }
    let alias = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    })?;
    let alias_name = alias.name.as_ref()?;
    let key = (alias.module_name.clone(), alias_name.clone());
    if visited_aliases.contains(&key) {
        return None;
    }
    visited_aliases.push(key);
    let schema = schema_reference(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
    );
    visited_aliases.pop();
    schema
}

pub(super) fn schema_decode_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .flat_map(|schema| schema_decode_function_signatures_for_schema(module, schema))
        .collect()
}

fn schema_decode_function_signatures_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Vec<FunctionSignature> {
    let Some(schema_name) = schema.name.as_ref() else {
        return Vec::new();
    };
    if schema.format.is_none() {
        return format_neutral_schema_decode_function_signature_for_schema(module, schema)
            .into_iter()
            .collect();
    }
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return Vec::new();
    }
    let Some(fields) = schema_decode_record_fields(module, schema) else {
        return Vec::new();
    };
    let byte_view = Type::named("ByteView", Vec::new());
    let byte_offset = Type::named("ByteOffset", Vec::new());
    let decoded_type = Type::Record(fields.into_iter().map(|(name, ty, _)| (name, ty)).collect());
    let result = Type::named("Result", vec![decoded_type.clone(), Type::string()]);
    let step = Type::named("DecodeStep", vec![decoded_type]);
    vec![
        FunctionSignature {
            name: schema_decode_function_name(schema_name),
            target_name: format!("{SCHEMA_DECODE_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![byte_view.clone()],
            variadic: None,
            return_type: result,
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        },
        FunctionSignature {
            name: schema_decode_step_function_name(schema_name),
            target_name: format!("{SCHEMA_DECODE_STEP_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![byte_view, byte_offset],
            variadic: None,
            return_type: step,
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        },
    ]
}

pub(crate) fn schema_decode_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    schema_decode_value_type_inner(module, schema, &mut Vec::new())
}

pub(super) fn schema_encode_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_encode_function_signature_for_schema(module, schema))
        .collect()
}

pub(super) fn schema_validate_function_signatures(
    module: &SurfaceModule,
) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_validate_function_signature_for_schema(module, schema))
        .collect()
}

fn schema_validate_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let fields = schema_decode_record_fields(module, schema)?
        .into_iter()
        .map(|(name, ty, _)| (name, ty))
        .collect::<Vec<_>>();
    let decoded_type = Type::Record(fields);
    Some(FunctionSignature {
        name: schema_validate_function_name(schema_name),
        target_name: format!("{SCHEMA_VALIDATE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![decoded_type.clone()],
        variadic: None,
        return_type: Type::named("Result", vec![decoded_type, Type::string()]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

fn schema_encode_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    if schema.format.is_none() {
        let value_type = Type::Record(format_neutral_schema_encode_record_fields(module, schema)?);
        return Some(FunctionSignature {
            name: schema_encode_function_name(schema_name),
            target_name: format!("{SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![value_type.clone()],
            variadic: None,
            return_type: Type::named("Result", vec![value_type, Type::string()]),
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        });
    }
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let (fields, exact_width_field_names) =
        schema_encode::schema_encode_schema_fields(module, schema)?;
    let value_fields =
        schema_encode_value_fields(module, schema, &fields, &exact_width_field_names)?;
    let byte_chunk = Type::named("ByteChunk", Vec::new());
    let encode_error = Type::named("EncodeError", Vec::new());
    Some(FunctionSignature {
        name: schema_encode_function_name(schema_name),
        target_name: format!("{SCHEMA_ENCODE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![Type::Record(value_fields)],
        variadic: None,
        return_type: Type::named("Result", vec![byte_chunk, encode_error]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

pub(crate) fn schema_encode_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    schema_encode_function_signature_for_schema(module, schema)
        .and_then(|signature| signature.params.into_iter().next())
}

fn schema_encode_value_fields(
    _module: &SurfaceModule,
    _schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    _exact_width_field_names: &[String],
) -> Option<Vec<(String, Type)>> {
    Some(schema_fields.to_vec())
}

pub(crate) fn schema_decode_function_name(schema_name: &str) -> String {
    format!("byte_decode_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_decode_step_function_name(schema_name: &str) -> String {
    format!("byte_decode_step_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_encode_function_name(schema_name: &str) -> String {
    format!("byte_encode_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_validate_function_name(schema_name: &str) -> String {
    format!("validate_{}", snake_case_identifier(schema_name))
}

fn snake_case_identifier(name: &str) -> String {
    let mut out = String::new();
    let mut previous_was_lower_or_digit = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if previous_was_lower_or_digit && !out.ends_with('_') {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
                previous_was_lower_or_digit = false;
            } else {
                out.push(ch);
                previous_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
            }
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
            previous_was_lower_or_digit = false;
        }
    }
    out.trim_matches('_').to_string()
}
