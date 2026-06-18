use super::*;
use crate::prelude::PRELUDE_MODULE;
use crate::types::{
    ByteViewLengthExpr, SchemaDispatchCasePayload, SchemaDispatchSpec, SchemaMappingConverterInput,
    SchemaMappingSelectorComparison, SchemaMappingSelectorPredicate, SchemaRepeatPayload,
    byte_view_schema_primitive, closed_dispatch_schema_primitive,
    extension_dispatch_schema_primitive, flag_schema_primitive,
    recursive_dispatch_payload_case_is_eligible, recursive_dispatch_payload_is_eligible,
    repeat_schema_primitive, schema_decode_record_type, schema_decode_step_function_name,
    schema_decode_value_type, schema_dispatch_payload_schema, schema_encode_function_name,
    schema_encode_value_type, schema_length_expression_references,
    schema_mapping_assignment_expr_typed, schema_mapping_selector_predicate,
    schema_mapping_selectors_overlap, schema_mapping_source_field_types,
    schema_payload_name_last_segment, schema_payload_name_path,
    schema_recursive_dispatch_payload_type, selected_mappings_cover_closed_dispatch,
    supported_encode_reserved_bits,
};
use std::collections::BTreeSet;
use veln_ast::{
    CodecDecl, CodecDirection, CodecImplementationClause, CodecImplementationKind, PublicAliasKind,
    SchemaDecl, SchemaField, SchemaMappingAssignment, SchemaMappingClause, SchemaValidationClause,
    UseDecl,
};

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

pub(crate) fn check_declared_effect_labels(function: &Function) -> Vec<Diagnostic> {
    let Some(declared_effects) = &function.effects else {
        return Vec::new();
    };
    let boundary = declared_effect_boundary(function);
    let node_prefix = function.kind.node_prefix();

    if declared_effects.is_empty() {
        return vec![empty_declared_effect_diagnostic(
            function,
            node_prefix,
            boundary,
        )];
    }

    declared_effects
        .iter()
        .filter(|effect| !KNOWN_EFFECT_LABELS.contains(&effect.as_str()))
        .map(|effect| unknown_declared_effect_diagnostic(function, effect, node_prefix, boundary))
        .collect()
}

fn declared_effect_boundary(function: &Function) -> &'static str {
    match function.kind {
        FunctionKind::Test => "test_declaration",
        FunctionKind::Function if function.visibility == Visibility::Public => "public_function",
        FunctionKind::Function => "private_function",
    }
}

fn empty_declared_effect_diagnostic(
    function: &Function,
    node_prefix: &'static str,
    boundary: &'static str,
) -> Diagnostic {
    let subject = match function.kind {
        FunctionKind::Test => "test declaration",
        FunctionKind::Function => "function declaration",
    };
    let mut diagnostic = Diagnostic::new(
        "effect.empty_declaration",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("empty effects list is not allowed on a {subject}"),
        Some(function.span.clone()),
        effect_details(function.node_id.display(node_prefix), boundary),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string("Remove the clause when the inferred effect set is empty."),
        ),
    ]));
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string(
                "Replace the empty list with non-empty effect labels when the body performs effects.",
            ),
        ),
    ]));
    diagnostic
}

fn unknown_declared_effect_diagnostic(
    function: &Function,
    effect: &str,
    node_prefix: &'static str,
    boundary: &'static str,
) -> Diagnostic {
    let declared_effects = function
        .effects
        .as_ref()
        .expect("unknown effect diagnostics require a declared effects clause");
    let mut diagnostic = Diagnostic::new(
        "effect.unknown",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("declared effect `{effect}` is not known"),
        Some(function.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("effect")),
            (
                "node_id",
                JsonValue::string(function.node_id.display(node_prefix)),
            ),
            ("effect", JsonValue::string(effect.to_string())),
            ("boundary", JsonValue::string(boundary)),
            (
                "declared_effects",
                JsonValue::array(declared_effects.iter().cloned().map(JsonValue::string)),
            ),
            (
                "known_effects",
                JsonValue::array(KNOWN_EFFECT_LABELS.iter().copied().map(JsonValue::string)),
            ),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string("Use a known effect label or remove the declaration."),
        ),
    ]));
    diagnostic
}

pub(crate) fn check_duplicate_function_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for function in &module.functions {
        let Some(name) = &function.name else {
            continue;
        };
        let key = (function.module_name.clone(), name.clone());
        let node_id = function.node_id.display(function.kind.node_prefix());
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "function",
                "function declaration",
                node_id,
                function.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, function.span.clone()));
        }
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        let Some(name) = &alias.name else {
            continue;
        };
        let key = (alias.module_name.clone(), name.clone());
        let node_id = alias.node_id.display("alias");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "function",
                "function alias",
                node_id,
                alias.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, alias.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_duplicate_type_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for type_decl in &module.types {
        let Some(name) = &type_decl.name else {
            continue;
        };
        let key = (type_decl.module_name.clone(), name.clone());
        let node_id = type_decl.node_id.display("type");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "type",
                "type declaration",
                node_id,
                type_decl.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, type_decl.span.clone()));
        }
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Type)
    {
        let Some(name) = &alias.name else {
            continue;
        };
        let key = (alias.module_name.clone(), name.clone());
        let node_id = alias.node_id.display("alias");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "type",
                "type alias",
                node_id,
                alias.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, alias.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_duplicate_schema_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for schema in &module.schemas {
        let Some(name) = &schema.name else {
            continue;
        };
        let key = (schema.module_name.clone(), name.clone());
        let node_id = schema.node_id.display("schema");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "schema",
                "schema declaration",
                node_id,
                schema.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, schema.span.clone()));
        }
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Schema)
    {
        let Some(name) = &alias.name else {
            continue;
        };
        let key = (alias.module_name.clone(), name.clone());
        let node_id = alias.node_id.display("alias");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "schema",
                "schema alias",
                node_id,
                alias.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, alias.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_duplicate_codec_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for codec in &module.codecs {
        let Some(name) = &codec.name else {
            continue;
        };
        let key = (codec.module_name.clone(), name.clone());
        let node_id = codec.node_id.display("codec");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "codec",
                "codec declaration",
                node_id,
                codec.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, codec.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_codec_schema_references(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for codec in &module.codecs {
        let Some(schema_name) = &codec.schema else {
            continue;
        };
        match resolve_codec_schema_reference(module, codec, schema_name) {
            SchemaResolution::Resolved(_) => {}
            SchemaResolution::Private => {
                diagnostics.push(private_codec_schema_diagnostic(codec, schema_name));
            }
            SchemaResolution::WrongKind(actual_kind) => {
                diagnostics.push(codec_schema_kind_mismatch_diagnostic(
                    codec,
                    schema_name,
                    actual_kind,
                ));
            }
            SchemaResolution::Unresolved => {
                diagnostics.push(unresolved_codec_schema_diagnostic(codec, schema_name));
            }
        }
    }

    diagnostics
}

#[derive(Clone, Copy, Debug)]
enum SchemaResolution<'a> {
    Resolved(&'a SchemaDecl),
    Private,
    WrongKind(&'static str),
    Unresolved,
}

#[derive(Clone, Copy, Debug)]
enum SchemaAliasCheckResolution {
    Resolved,
    Private,
    WrongKind(&'static str),
    Unresolved,
}

fn resolve_codec_schema_reference<'a>(
    module: &'a SurfaceModule,
    codec: &CodecDecl,
    schema_name: &str,
) -> SchemaResolution<'a> {
    let current_module = codec.module_name.as_deref();
    let segments = schema_name
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    resolve_schema_reference(module, &segments, current_module, true, &mut Vec::new())
}

fn resolve_schema_reference<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> SchemaResolution<'a> {
    match segments {
        [name] => resolve_schema_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
        ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            ) else {
                return SchemaResolution::Unresolved;
            };
            resolve_schema_in_module(module, Some(&use_decl.name), name, false, visited_aliases)
        }
        _ => SchemaResolution::Unresolved,
    }
}

fn resolve_schema_in_module<'a>(
    module: &'a SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> SchemaResolution<'a> {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return if allow_private_schema || schema.visibility == Visibility::Public {
            SchemaResolution::Resolved(schema)
        } else {
            SchemaResolution::Private
        };
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    }) {
        return resolve_schema_alias_target(module, alias, visited_aliases);
    }
    codec_schema_wrong_kind(module, module_name, name)
        .map_or(SchemaResolution::Unresolved, SchemaResolution::WrongKind)
}

fn resolve_schema_alias_target<'a>(
    module: &'a SurfaceModule,
    alias: &veln_ast::PublicAlias,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> SchemaResolution<'a> {
    let Some(name) = &alias.name else {
        return SchemaResolution::Unresolved;
    };
    let key = (alias.module_name.clone(), name.clone());
    if visited_aliases.contains(&key) {
        return SchemaResolution::Unresolved;
    }
    visited_aliases.push(key);
    let resolution = resolve_schema_reference(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
    );
    visited_aliases.pop();
    resolution
}

fn codec_schema_wrong_kind(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
) -> Option<&'static str> {
    if module.functions.iter().any(|function| {
        function.kind == FunctionKind::Function
            && function.name.as_deref() == Some(name)
            && function.module_name.as_deref() == module_name
    }) {
        return Some("function");
    }
    if module.types.iter().any(|type_decl| {
        type_decl.name.as_deref() == Some(name) && type_decl.module_name.as_deref() == module_name
    }) {
        return Some("type");
    }
    if module.codecs.iter().any(|codec| {
        codec.name.as_deref() == Some(name) && codec.module_name.as_deref() == module_name
    }) {
        return Some("codec");
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.name.as_deref() == Some(name) && alias.module_name.as_deref() == module_name
    }) {
        return match alias.kind {
            PublicAliasKind::Function => Some("function"),
            PublicAliasKind::Type => Some("type"),
            PublicAliasKind::Schema => None,
        };
    }
    None
}

pub(crate) fn check_codec_decode_signatures(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for codec in &module.codecs {
        for implementation in codec
            .implementations
            .iter()
            .filter(|implementation| implementation.direction == CodecDirection::Decode)
        {
            match &implementation.kind {
                CodecImplementationKind::With { function } => {
                    let Some(function_name) = function else {
                        continue;
                    };
                    let Some(function) = codec_same_module_function(module, codec, function_name)
                    else {
                        diagnostics.push(unresolved_codec_decode_function_diagnostic(
                            codec,
                            implementation,
                            function_name,
                        ));
                        continue;
                    };

                    diagnostics.extend(codec_decode_signature_diagnostics(
                        module,
                        codec,
                        implementation,
                        function,
                        function_name,
                    ));
                }
                CodecImplementationKind::Derive => {
                    diagnostics.extend(codec_derive_decode_value_type_diagnostics(
                        module,
                        codec,
                        implementation,
                    ));
                }
            }
        }
    }

    diagnostics
}

pub(crate) fn check_codec_encode_signatures(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for codec in &module.codecs {
        for implementation in codec
            .implementations
            .iter()
            .filter(|implementation| implementation.direction == CodecDirection::Encode)
        {
            match &implementation.kind {
                CodecImplementationKind::With { function } => {
                    let Some(function_name) = function else {
                        continue;
                    };
                    let Some(function) = codec_same_module_function(module, codec, function_name)
                    else {
                        diagnostics.push(unresolved_codec_encode_function_diagnostic(
                            codec,
                            implementation,
                            function_name,
                        ));
                        continue;
                    };

                    diagnostics.extend(codec_encode_signature_diagnostics(
                        module,
                        codec,
                        implementation,
                        function,
                        function_name,
                    ));
                }
                CodecImplementationKind::Derive => {
                    diagnostics.extend(codec_derive_encode_value_type_diagnostics(
                        module,
                        codec,
                        implementation,
                    ));
                }
            }
        }
    }

    diagnostics
}

fn codec_same_module_function<'a>(
    module: &'a SurfaceModule,
    codec: &CodecDecl,
    function_name: &str,
) -> Option<&'a Function> {
    module.functions.iter().find(|function| {
        function.kind == FunctionKind::Function
            && function.module_name == codec.module_name
            && function.name.as_deref() == Some(function_name)
    })
}

fn codec_decode_signature_diagnostics(
    module: &SurfaceModule,
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    function: &Function,
    function_name: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if function.params.len() != 2 {
        diagnostics.push(codec_decode_signature_diagnostic(
            codec,
            implementation,
            Some(function),
            function_name,
            "parameter_count",
            "decode function must take `ByteView` and `ByteOffset` parameters",
            format!(
                "fn({}) -> {}",
                parameter_types_text(function),
                return_type_text(function)
            ),
        ));
        return diagnostics;
    }

    for (index, expected_type, expected_text) in [
        (0usize, Type::named("ByteView", Vec::new()), "ByteView"),
        (1usize, Type::named("ByteOffset", Vec::new()), "ByteOffset"),
    ] {
        let actual_type = function
            .params
            .get(index)
            .and_then(|param| param.ty.as_deref())
            .and_then(|annotation| parse_type_annotation(annotation).ok())
            .unwrap_or(Type::Unknown);
        if actual_type != expected_type {
            let ordinal = if index == 0 { "first" } else { "second" };
            diagnostics.push(codec_decode_signature_diagnostic(
                codec,
                implementation,
                Some(function),
                function_name,
                if index == 0 {
                    "input_view_type"
                } else {
                    "base_offset_type"
                },
                format!("decode function {ordinal} parameter must be `{expected_text}`"),
                actual_type.render(),
            ));
        }
    }

    let return_type = function
        .return_type
        .as_deref()
        .and_then(|annotation| parse_type_annotation(annotation).ok())
        .unwrap_or(Type::Unknown);
    if !is_decode_step_return(&return_type) {
        diagnostics.push(codec_decode_signature_diagnostic(
            codec,
            implementation,
            Some(function),
            function_name,
            "return_type",
            "decode function must return `DecodeStep<T>`",
            return_type.render(),
        ));
    } else if let Some(expected_value_type) = codec_mapping_value_type(module, codec) {
        let actual_value_type = decode_step_value_type(&return_type)
            .expect("DecodeStep return value type is available after shape check");
        if !types_match(&expected_value_type, actual_value_type) {
            diagnostics.push(codec_decode_value_type_diagnostic(
                codec,
                implementation,
                function,
                function_name,
                &expected_value_type,
                actual_value_type,
            ));
        }
    }

    diagnostics
}

fn is_decode_step_return(ty: &Type) -> bool {
    decode_step_value_type(ty).is_some()
}

fn decode_step_value_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Named { name, args } if name == "DecodeStep" && args.len() == 1 => Some(&args[0]),
        _ => None,
    }
}

fn codec_encode_signature_diagnostics(
    module: &SurfaceModule,
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    function: &Function,
    function_name: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(expected_value_type) = codec_mapping_value_type(module, codec) {
        match function.params.first() {
            Some(param) => {
                let actual_value_type = param
                    .ty
                    .as_deref()
                    .and_then(|annotation| parse_type_annotation(annotation).ok())
                    .unwrap_or(Type::Unknown);
                if !types_match(&expected_value_type, &actual_value_type) {
                    diagnostics.push(codec_encode_value_type_diagnostic(
                        codec,
                        implementation,
                        function,
                        function_name,
                        &expected_value_type,
                        EncodeValueTypeMismatch {
                            reason: "value_parameter_type",
                            message:
                                "encode function value parameter must match schema mapping value type",
                            actual_value_type: &actual_value_type,
                        },
                    ));
                }
            }
            None => {
                diagnostics.push(codec_encode_value_type_diagnostic(
                    codec,
                    implementation,
                    function,
                    function_name,
                    &expected_value_type,
                    EncodeValueTypeMismatch {
                        reason: "missing_value_parameter",
                        message: "encode function must take a schema mapping value parameter",
                        actual_value_type: &Type::Unknown,
                    },
                ));
            }
        }
    }

    let return_type = function
        .return_type
        .as_deref()
        .and_then(|annotation| parse_type_annotation(annotation).ok())
        .unwrap_or(Type::Unknown);

    if !is_encode_step_return(&return_type) {
        diagnostics.push(codec_encode_signature_diagnostic(
            codec,
            implementation,
            Some(function),
            function_name,
            "return_type",
            "encode function must return `EncodeStep<TState>`",
            return_type.render(),
        ));
    }

    diagnostics
}

fn codec_derive_decode_value_type_diagnostics(
    module: &SurfaceModule,
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
) -> Vec<Diagnostic> {
    let Some(schema) = codec_referenced_schema(module, codec) else {
        return Vec::new();
    };
    let Some(actual_value_type) = schema_decode_value_type(module, schema) else {
        return vec![codec_derive_helper_unsupported_diagnostic(
            codec,
            implementation,
            schema,
            CodecDirection::Decode,
        )];
    };
    let Some(expected_value_type) = codec_declared_mapping_value_type(module, codec) else {
        return Vec::new();
    };
    if types_match(&expected_value_type, &actual_value_type) {
        return Vec::new();
    }

    vec![codec_derive_decode_value_type_diagnostic(
        codec,
        implementation,
        &expected_value_type,
        &actual_value_type,
    )]
}

fn codec_derive_encode_value_type_diagnostics(
    module: &SurfaceModule,
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
) -> Vec<Diagnostic> {
    let Some(schema) = codec_referenced_schema(module, codec) else {
        return Vec::new();
    };
    let Some(actual_value_type) = schema_encode_value_type(module, schema) else {
        return vec![codec_derive_helper_unsupported_diagnostic(
            codec,
            implementation,
            schema,
            CodecDirection::Encode,
        )];
    };
    let Some(expected_value_type) = codec_declared_mapping_value_type(module, codec) else {
        return Vec::new();
    };
    if types_match(&expected_value_type, &actual_value_type) {
        return Vec::new();
    }

    vec![codec_derive_encode_value_type_diagnostic(
        codec,
        implementation,
        &expected_value_type,
        &actual_value_type,
    )]
}

fn is_encode_step_return(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { name, args } if name == "EncodeStep" && args.len() == 1
    )
}

fn codec_mapping_value_type(module: &SurfaceModule, codec: &CodecDecl) -> Option<Type> {
    let parts = codec_single_mapping_value_type_parts(
        module,
        codec,
        MappingValueTypeEligibility::ImplementedRuntimeSlice,
    )?;
    if !mapping_is_implemented_value_slice(
        module,
        parts.schema,
        &parts.schema_fields,
        parts.mapping,
        &parts.target_fields,
    ) {
        return None;
    }
    Some(Type::Record(parts.target_fields))
}

fn codec_declared_mapping_value_type(module: &SurfaceModule, codec: &CodecDecl) -> Option<Type> {
    let parts = codec_single_mapping_value_type_parts(
        module,
        codec,
        MappingValueTypeEligibility::Declared,
    )?;
    Some(Type::Record(parts.target_fields))
}

struct CodecMappingValueTypeParts<'a> {
    schema_fields: BTreeMap<String, Type>,
    target_fields: Vec<(String, Type)>,
    schema: &'a SchemaDecl,
    mapping: &'a SchemaMappingClause,
}

fn codec_single_mapping_value_type_parts<'a>(
    module: &'a SurfaceModule,
    codec: &CodecDecl,
    eligibility: MappingValueTypeEligibility,
) -> Option<CodecMappingValueTypeParts<'a>> {
    let schema = codec_referenced_schema(module, codec)?;
    let [mapping, rest @ ..] = schema.mappings.as_slice() else {
        return None;
    };
    let schema_fields = generated_schema_field_types(module, schema)?;
    let target_fields = schema_mapping_target_record_fields(module, schema, mapping)?;
    for candidate in rest {
        candidate.selector.as_ref()?;
        let candidate_target_fields =
            schema_mapping_target_record_fields(module, schema, candidate)?;
        if candidate_target_fields != target_fields {
            return None;
        }
        let candidate_matches = match eligibility {
            MappingValueTypeEligibility::Declared => mapping_matches_declared_value_type(
                module,
                schema,
                &schema_fields,
                candidate,
                &candidate_target_fields,
            ),
            MappingValueTypeEligibility::ImplementedRuntimeSlice => {
                mapping_is_implemented_value_slice(
                    module,
                    schema,
                    &schema_fields,
                    candidate,
                    &candidate_target_fields,
                )
            }
        };
        if !candidate_matches {
            return None;
        }
    }
    if eligibility == MappingValueTypeEligibility::Declared
        && !mapping_matches_declared_value_type(
            module,
            schema,
            &schema_fields,
            mapping,
            &target_fields,
        )
    {
        return None;
    }
    Some(CodecMappingValueTypeParts {
        schema_fields,
        target_fields,
        schema,
        mapping,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MappingValueTypeEligibility {
    Declared,
    ImplementedRuntimeSlice,
}

fn mapping_matches_declared_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &BTreeMap<String, Type>,
    mapping: &SchemaMappingClause,
    target_fields: &[(String, Type)],
) -> bool {
    let target_field_types = target_fields
        .iter()
        .cloned()
        .collect::<BTreeMap<String, Type>>();
    let mut seen_targets = BTreeMap::<&str, ()>::new();
    for assignment in &mapping.assignments {
        let Some(target_ty) = target_field_types.get(&assignment.target) else {
            return false;
        };
        if schema_mapping_expr_typed(module, schema, schema_fields, &assignment.expr, target_ty)
            .is_err()
        {
            return false;
        }
        if seen_targets
            .insert(assignment.target.as_str(), ())
            .is_some()
        {
            return false;
        }
    }

    target_fields
        .iter()
        .all(|(target_field, _)| seen_targets.contains_key(target_field.as_str()))
}

fn mapping_is_implemented_value_slice(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &BTreeMap<String, Type>,
    mapping: &SchemaMappingClause,
    target_fields: &[(String, Type)],
) -> bool {
    let mut seen_targets = BTreeMap::<&str, ()>::new();
    for assignment in &mapping.assignments {
        let Some((_, target_ty)) = target_fields
            .iter()
            .find(|(target_field, _)| target_field == &assignment.target)
        else {
            return false;
        };
        if schema_mapping_expr_typed(module, schema, schema_fields, &assignment.expr, target_ty)
            .is_err()
        {
            return false;
        }
        if seen_targets
            .insert(assignment.target.as_str(), ())
            .is_some()
        {
            return false;
        }
    }

    target_fields
        .iter()
        .all(|(target_field, _)| seen_targets.contains_key(target_field.as_str()))
}

fn codec_referenced_schema<'a>(
    module: &'a SurfaceModule,
    codec: &CodecDecl,
) -> Option<&'a SchemaDecl> {
    let schema_name = codec.schema.as_ref()?;
    match resolve_codec_schema_reference(module, codec, schema_name) {
        SchemaResolution::Resolved(schema) => Some(schema),
        SchemaResolution::Private
        | SchemaResolution::WrongKind(_)
        | SchemaResolution::Unresolved => None,
    }
}

fn types_match(expected: &Type, actual: &Type) -> bool {
    actual != &Type::Unknown && is_assignable(expected, actual) && is_assignable(actual, expected)
}

fn parameter_types_text(function: &Function) -> String {
    function
        .params
        .iter()
        .map(|param| {
            param
                .ty
                .as_deref()
                .and_then(|annotation| parse_type_annotation(annotation).ok())
                .unwrap_or(Type::Unknown)
                .render()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn return_type_text(function: &Function) -> String {
    function
        .return_type
        .as_deref()
        .and_then(|annotation| parse_type_annotation(annotation).ok())
        .unwrap_or(Type::Unknown)
        .render()
}

fn unresolved_codec_decode_function_diagnostic(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    function_name: &str,
) -> Diagnostic {
    Diagnostic::new(
        "name.unresolved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("unresolved decode function `{function_name}`"),
        Some(implementation.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("codec")),
            (
                "node_id",
                JsonValue::string(implementation.node_id.display("codec-impl")),
            ),
            (
                "codec",
                JsonValue::string(codec.name.as_deref().unwrap_or("<missing>")),
            ),
            ("direction", JsonValue::string("decode")),
            ("expected_kind", JsonValue::string("function")),
            ("target", JsonValue::string(function_name.to_string())),
        ]),
    )
}

fn unresolved_codec_encode_function_diagnostic(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    function_name: &str,
) -> Diagnostic {
    Diagnostic::new(
        "name.unresolved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("unresolved encode function `{function_name}`"),
        Some(implementation.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("codec")),
            (
                "node_id",
                JsonValue::string(implementation.node_id.display("codec-impl")),
            ),
            (
                "codec",
                JsonValue::string(codec.name.as_deref().unwrap_or("<missing>")),
            ),
            ("direction", JsonValue::string("encode")),
            ("expected_kind", JsonValue::string("function")),
            ("target", JsonValue::string(function_name.to_string())),
        ]),
    )
}

fn codec_decode_signature_diagnostic(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    function: Option<&Function>,
    function_name: &str,
    reason: &'static str,
    message: impl Into<String>,
    actual_signature: impl Into<String>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "codec.decode_signature",
        Severity::Error,
        DiagnosticKind::Type,
        message.into(),
        Some(implementation.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("codec")),
            (
                "node_id",
                JsonValue::string(implementation.node_id.display("codec-impl")),
            ),
            (
                "codec",
                JsonValue::string(codec.name.as_deref().unwrap_or("<missing>")),
            ),
            ("direction", JsonValue::string("decode")),
            ("function", JsonValue::string(function_name.to_string())),
            ("reason", JsonValue::string(reason)),
            (
                "expected_signature",
                JsonValue::string("fn(ByteView, ByteOffset) -> DecodeStep<T>"),
            ),
            (
                "actual_signature",
                JsonValue::string(actual_signature.into()),
            ),
        ]),
    );
    if let Some(function) = function {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("function_signature")),
            (
                "message",
                JsonValue::string(format!(
                    "Referenced function `{function_name}` is declared here."
                )),
            ),
            ("span", span_json(&function.span)),
        ]));
    }
    diagnostic
}

fn codec_encode_signature_diagnostic(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    function: Option<&Function>,
    function_name: &str,
    reason: &'static str,
    message: impl Into<String>,
    actual_signature: impl Into<String>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "codec.encode_signature",
        Severity::Error,
        DiagnosticKind::Type,
        message.into(),
        Some(implementation.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("codec")),
            (
                "node_id",
                JsonValue::string(implementation.node_id.display("codec-impl")),
            ),
            (
                "codec",
                JsonValue::string(codec.name.as_deref().unwrap_or("<missing>")),
            ),
            ("direction", JsonValue::string("encode")),
            ("function", JsonValue::string(function_name.to_string())),
            ("reason", JsonValue::string(reason)),
            (
                "expected_signature",
                JsonValue::string("fn(...) -> EncodeStep<TState>"),
            ),
            (
                "actual_signature",
                JsonValue::string(actual_signature.into()),
            ),
        ]),
    );
    if let Some(function) = function {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("function_signature")),
            (
                "message",
                JsonValue::string(format!(
                    "Referenced function `{function_name}` is declared here."
                )),
            ),
            ("span", span_json(&function.span)),
        ]));
    }
    diagnostic
}

fn codec_decode_value_type_diagnostic(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    function: &Function,
    function_name: &str,
    expected_value_type: &Type,
    actual_value_type: &Type,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "codec.decode_value_type",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "decode function value type is `{}`, but schema mapping value type is `{}`",
            actual_value_type.render(),
            expected_value_type.render()
        ),
        Some(implementation.span.clone()),
        codec_mapping_value_details(
            implementation,
            codec,
            "decode",
            function_name,
            "return_value_type",
            expected_value_type,
            actual_value_type,
        ),
    );
    diagnostic
        .related
        .push(codec_function_related(function, function_name));
    diagnostic
}

fn codec_encode_value_type_diagnostic(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    function: &Function,
    function_name: &str,
    expected_value_type: &Type,
    mismatch: EncodeValueTypeMismatch<'_>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "codec.encode_value_type",
        Severity::Error,
        DiagnosticKind::Type,
        mismatch.message,
        Some(implementation.span.clone()),
        codec_mapping_value_details(
            implementation,
            codec,
            "encode",
            function_name,
            mismatch.reason,
            expected_value_type,
            mismatch.actual_value_type,
        ),
    );
    diagnostic
        .related
        .push(codec_function_related(function, function_name));
    diagnostic
}

fn codec_derive_decode_value_type_diagnostic(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    expected_value_type: &Type,
    actual_value_type: &Type,
) -> Diagnostic {
    Diagnostic::new(
        "codec.decode_value_type",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "derived decode value type is `{}`, but schema mapping value type is `{}`",
            actual_value_type.render(),
            expected_value_type.render()
        ),
        Some(implementation.span.clone()),
        codec_mapping_value_details(
            implementation,
            codec,
            "decode",
            "<derived>",
            "generated_decode_value_type",
            expected_value_type,
            actual_value_type,
        ),
    )
}

fn codec_derive_encode_value_type_diagnostic(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    expected_value_type: &Type,
    actual_value_type: &Type,
) -> Diagnostic {
    Diagnostic::new(
        "codec.encode_value_type",
        Severity::Error,
        DiagnosticKind::Type,
        "derived encode value parameter must match schema mapping value type",
        Some(implementation.span.clone()),
        codec_mapping_value_details(
            implementation,
            codec,
            "encode",
            "<derived>",
            "generated_encode_value_type",
            expected_value_type,
            actual_value_type,
        ),
    )
}

fn codec_derive_helper_unsupported_diagnostic(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    schema: &SchemaDecl,
    direction: CodecDirection,
) -> Diagnostic {
    let direction_text = direction.as_str();
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let (reason, helper, boundary) = match direction {
        CodecDirection::Decode => (
            "generated_decode_helper_unavailable",
            schema_decode_step_function_name(schema_name),
            "generated_binary_schema_decode_step",
        ),
        CodecDirection::Encode => (
            "generated_encode_helper_unavailable",
            schema_encode_function_name(schema_name),
            "generated_binary_schema_encode",
        ),
    };
    let mut diagnostic = Diagnostic::new(
        "codec.derive_helper_unsupported",
        Severity::Error,
        DiagnosticKind::Type,
        format!("derived {direction_text} is not available for this schema"),
        Some(implementation.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("codec")),
            (
                "node_id",
                JsonValue::string(implementation.node_id.display("codec-impl")),
            ),
            (
                "codec",
                JsonValue::string(codec.name.as_deref().unwrap_or("<missing>")),
            ),
            ("direction", JsonValue::string(direction_text)),
            ("reason", JsonValue::string(reason)),
            ("schema", JsonValue::string(schema_name)),
            ("expected_helper", JsonValue::string(helper.clone())),
            ("helper_boundary", JsonValue::string(boundary)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("schema_declaration")),
        (
            "message",
            JsonValue::string(format!(
                "Schema `{schema_name}` is declared here and is outside the generated {direction_text} helper slice."
            )),
        ),
        ("span", span_json(&schema.span)),
    ]));
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("helper_boundary")),
        (
            "message",
            JsonValue::string(format!(
                "`derive {direction_text}` requires the named schema to expose the generated `{helper}` helper."
            )),
        ),
    ]));
    diagnostic
}

struct EncodeValueTypeMismatch<'a> {
    reason: &'static str,
    message: &'static str,
    actual_value_type: &'a Type,
}

fn codec_mapping_value_details(
    implementation: &CodecImplementationClause,
    codec: &CodecDecl,
    direction: &'static str,
    function_name: &str,
    reason: &'static str,
    expected_value_type: &Type,
    actual_value_type: &Type,
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("codec")),
        (
            "node_id",
            JsonValue::string(implementation.node_id.display("codec-impl")),
        ),
        (
            "codec",
            JsonValue::string(codec.name.as_deref().unwrap_or("<missing>")),
        ),
        ("direction", JsonValue::string(direction)),
        ("function", JsonValue::string(function_name.to_string())),
        ("reason", JsonValue::string(reason)),
        (
            "schema",
            JsonValue::string(codec.schema.as_deref().unwrap_or("<missing>")),
        ),
        (
            "expected_value_type",
            JsonValue::string(expected_value_type.render()),
        ),
        (
            "actual_value_type",
            JsonValue::string(actual_value_type.render()),
        ),
    ])
}

fn codec_function_related(function: &Function, function_name: &str) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("function_signature")),
        (
            "message",
            JsonValue::string(format!(
                "Referenced function `{function_name}` is declared here."
            )),
        ),
        ("span", span_json(&function.span)),
    ])
}

pub(crate) fn check_public_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut schema_alias_cache = BTreeMap::new();
    for alias in &module.aliases {
        if alias.name.is_none() {
            continue;
        }
        match alias.kind {
            PublicAliasKind::Function => {
                if function_target(module, &alias.target, alias.module_name.as_deref()).is_none()
                    && type_target(module, &alias.target, alias.module_name.as_deref()).is_some()
                {
                    diagnostics.push(alias_kind_mismatch_diagnostic(alias, "function", "type"));
                } else if function_target(module, &alias.target, alias.module_name.as_deref())
                    .is_none()
                {
                    diagnostics.push(unresolved_alias_diagnostic(alias, "function"));
                }
            }
            PublicAliasKind::Type => {
                if type_target(module, &alias.target, alias.module_name.as_deref()).is_none()
                    && function_target(module, &alias.target, alias.module_name.as_deref())
                        .is_some()
                {
                    diagnostics.push(alias_kind_mismatch_diagnostic(alias, "type", "function"));
                } else if type_target(module, &alias.target, alias.module_name.as_deref()).is_none()
                {
                    diagnostics.push(unresolved_alias_diagnostic(alias, "type"));
                }
            }
            PublicAliasKind::Schema => {
                match resolve_schema_alias_check_reference(
                    module,
                    &alias.target,
                    alias.module_name.as_deref(),
                    false,
                    &mut Vec::new(),
                    &mut schema_alias_cache,
                ) {
                    SchemaAliasCheckResolution::Resolved => {}
                    SchemaAliasCheckResolution::Private => {
                        diagnostics.push(private_alias_diagnostic(alias));
                    }
                    SchemaAliasCheckResolution::WrongKind(actual_kind) => {
                        diagnostics.push(alias_kind_mismatch_diagnostic(
                            alias,
                            "schema",
                            actual_kind,
                        ));
                    }
                    SchemaAliasCheckResolution::Unresolved => {
                        diagnostics.push(unresolved_alias_diagnostic(alias, "schema"));
                    }
                }
            }
        }
    }
    diagnostics
}

fn resolve_schema_alias_check_reference(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
    cache: &mut BTreeMap<(Option<String>, String), SchemaAliasCheckResolution>,
) -> SchemaAliasCheckResolution {
    match segments {
        [name] => resolve_schema_alias_check_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
            cache,
        ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            ) else {
                return SchemaAliasCheckResolution::Unresolved;
            };
            resolve_schema_alias_check_in_module(
                module,
                Some(&use_decl.name),
                name,
                false,
                visited_aliases,
                cache,
            )
        }
        _ => SchemaAliasCheckResolution::Unresolved,
    }
}

fn resolve_schema_alias_check_in_module(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
    cache: &mut BTreeMap<(Option<String>, String), SchemaAliasCheckResolution>,
) -> SchemaAliasCheckResolution {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return if allow_private_schema || schema.visibility == Visibility::Public {
            SchemaAliasCheckResolution::Resolved
        } else {
            SchemaAliasCheckResolution::Private
        };
    }

    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    }) {
        return resolve_schema_alias_check_target(module, alias, visited_aliases, cache);
    }

    codec_schema_wrong_kind(module, module_name, name).map_or(
        SchemaAliasCheckResolution::Unresolved,
        SchemaAliasCheckResolution::WrongKind,
    )
}

fn resolve_schema_alias_check_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    visited_aliases: &mut Vec<(Option<String>, String)>,
    cache: &mut BTreeMap<(Option<String>, String), SchemaAliasCheckResolution>,
) -> SchemaAliasCheckResolution {
    let Some(name) = &alias.name else {
        return SchemaAliasCheckResolution::Unresolved;
    };
    let key = (alias.module_name.clone(), name.clone());
    if let Some(resolution) = cache.get(&key) {
        return *resolution;
    }
    if visited_aliases.contains(&key) {
        return SchemaAliasCheckResolution::Unresolved;
    }

    visited_aliases.push(key.clone());
    let resolution = resolve_schema_alias_check_reference(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
        cache,
    );
    visited_aliases.pop();
    cache.insert(key, resolution);
    resolution
}

fn unresolved_codec_schema_diagnostic(codec: &CodecDecl, schema_name: &str) -> Diagnostic {
    Diagnostic::new(
        "name.unresolved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("unresolved codec schema `{schema_name}`"),
        Some(codec.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(codec.node_id.display("codec"))),
            ("expected_kind", JsonValue::string("schema")),
            ("target", JsonValue::string(schema_name.to_string())),
        ]),
    )
}

fn private_codec_schema_diagnostic(codec: &CodecDecl, schema_name: &str) -> Diagnostic {
    Diagnostic::new(
        "name.visibility",
        Severity::Error,
        DiagnosticKind::Name,
        format!("codec schema `{schema_name}` is private"),
        Some(codec.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(codec.node_id.display("codec"))),
            ("expected_kind", JsonValue::string("schema")),
            ("target", JsonValue::string(schema_name.to_string())),
            ("visibility", JsonValue::string("private")),
        ]),
    )
}

fn codec_schema_kind_mismatch_diagnostic(
    codec: &CodecDecl,
    schema_name: &str,
    actual_kind: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "name.kind_mismatch",
        Severity::Error,
        DiagnosticKind::Name,
        format!("codec schema target `{schema_name}` is a {actual_kind}, not a schema"),
        Some(codec.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(codec.node_id.display("codec"))),
            ("expected_kind", JsonValue::string("schema")),
            ("actual_kind", JsonValue::string(actual_kind)),
            ("target", JsonValue::string(schema_name.to_string())),
        ]),
    )
}

pub(crate) fn check_schema_type_references(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for function in &module.functions {
        let current_module = function.module_name.as_deref();
        for param in &function.params {
            if let Some(annotation) = &param.ty {
                push_schema_type_reference_diagnostics(
                    module,
                    current_module,
                    annotation,
                    param.node_id.display("param"),
                    param.span.clone(),
                    "parameter_type",
                    &mut diagnostics,
                );
            }
        }
        if let Some(return_type) = &function.return_type {
            push_schema_type_reference_diagnostics(
                module,
                current_module,
                return_type,
                function.node_id.display(function.kind.node_prefix()),
                function.span.clone(),
                "return_type",
                &mut diagnostics,
            );
        }
        for line in &function.body {
            let BodyLineKind::Let {
                annotation: Some(annotation),
                ..
            } = &line.kind
            else {
                continue;
            };
            push_schema_type_reference_diagnostics(
                module,
                current_module,
                annotation,
                line.node_id.display("let"),
                line.span.clone(),
                "local_annotation",
                &mut diagnostics,
            );
        }
    }

    for type_decl in &module.types {
        let current_module = type_decl.module_name.as_deref();
        for variant in &type_decl.variants {
            for field in &variant.fields {
                push_schema_type_reference_diagnostics(
                    module,
                    current_module,
                    &field.ty,
                    field.node_id.display("field"),
                    field.span.clone(),
                    "type_variant_field",
                    &mut diagnostics,
                );
            }
        }
    }

    for schema in &module.schemas {
        let current_module = schema.module_name.as_deref();
        for mapping in &schema.mappings {
            let Some(target) = &mapping.target else {
                continue;
            };
            push_schema_type_reference_diagnostics(
                module,
                current_module,
                target,
                mapping.node_id.display("schema-mapping"),
                mapping.span.clone(),
                "schema_mapping_target",
                &mut diagnostics,
            );
        }
    }

    diagnostics
}

pub(crate) fn check_schema_field_primitives(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for schema in &module.schemas {
        let format_name = schema.format.as_ref().map(|format| format.name.as_str());
        let mut decoded_fields = BTreeMap::<String, Type>::new();
        for field in &schema.fields {
            if let Some(flag_type) = flag_schema_primitive(&field.ty) {
                if format_name == Some("binary") {
                    decoded_fields.insert(field.name.clone(), Type::named(flag_type, Vec::new()));
                } else {
                    diagnostics.push(exact_width_schema_primitive_diagnostic(
                        flag_type,
                        Some(schema),
                        Some(field),
                        field.node_id.display("schema-field"),
                        field.span.clone(),
                        "non_binary_format",
                    ));
                }
                continue;
            }
            if let Some(primitive) = exact_width_binary_primitive_name(&field.ty) {
                if format_name != Some("binary") {
                    diagnostics.push(exact_width_schema_primitive_diagnostic(
                        primitive,
                        Some(schema),
                        Some(field),
                        field.node_id.display("schema-field"),
                        field.span.clone(),
                        "non_binary_format",
                    ));
                } else {
                    decoded_fields.insert(field.name.clone(), Type::int());
                }
                continue;
            }
            if format_name == Some("binary")
                && let Some(length_expr) = byte_view_schema_primitive(&field.ty)
            {
                if check_schema_byte_view_reference(
                    schema,
                    field,
                    &length_expr,
                    &decoded_fields,
                    &mut diagnostics,
                ) {
                    decoded_fields.insert(field.name.clone(), Type::named("ByteView", Vec::new()));
                }
                continue;
            }
            if format_name == Some("binary")
                && let Some(repeat) = repeat_schema_primitive(&field.ty)
            {
                if let Some(field_ty) = check_schema_repeat_field(
                    module,
                    schema,
                    field,
                    &repeat,
                    &decoded_fields,
                    &mut diagnostics,
                ) {
                    decoded_fields.insert(field.name.clone(), field_ty);
                }
                continue;
            }
            if format_name == Some("binary")
                && let Some(dispatch) = closed_dispatch_schema_primitive(&field.ty)
                    .or_else(|| extension_dispatch_schema_primitive(&field.ty))
            {
                if let Some(field_ty) = check_schema_dispatch_field(
                    module,
                    schema,
                    field,
                    &dispatch,
                    &decoded_fields,
                    &mut diagnostics,
                ) {
                    decoded_fields.insert(field.name.clone(), field_ty);
                }
                continue;
            }
            if let Some(primitive) = reserved_bits_primitive(&field.ty) {
                if format_name != Some("binary") {
                    diagnostics.push(reserved_bits_format_diagnostic(schema, field));
                    continue;
                }
                match primitive {
                    Err(reason) => {
                        diagnostics.push(reserved_bits_argument_diagnostic(schema, field, reason))
                    }
                    Ok(reserved) => {
                        let field_index = schema
                            .fields
                            .iter()
                            .position(|schema_field| schema_field.node_id == field.node_id);
                        if field_index
                            .and_then(|index| {
                                supported_encode_reserved_bits(&schema.fields, index, reserved)
                            })
                            .is_none()
                        {
                            diagnostics.push(reserved_bits_encode_shape_diagnostic(
                                schema, field, reserved,
                            ));
                        }
                    }
                }
            }
        }
        for (index, validation) in schema.validations.iter().enumerate() {
            if index > 0 {
                diagnostics.push(schema_validation_duplicate_diagnostic(schema, validation));
                continue;
            }
            check_schema_validation_clause(schema, validation, &decoded_fields, &mut diagnostics);
        }
    }

    diagnostics
}

fn check_schema_validation_clause(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for reference in schema_validation_references(&validation.predicate) {
        let Some(ty) = decoded_fields.get(&reference) else {
            diagnostics.push(schema_validation_reference_diagnostic(
                schema,
                validation,
                &reference,
                "unknown_field_reference",
                format!("schema validation reference `{reference}` is not a decoded schema field"),
                [],
            ));
            continue;
        };
        if ty != &Type::int() {
            diagnostics.push(schema_validation_reference_diagnostic(
                schema,
                validation,
                &reference,
                "incompatible_field_reference",
                format!(
                    "schema validation reference `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            ));
        }
    }
}

fn schema_validation_references(predicate: &str) -> Vec<String> {
    let mut references = BTreeSet::new();
    let mut chars = predicate.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if !(ch.is_ascii_alphabetic() || ch == '_') {
            continue;
        }
        let mut end = start + ch.len_utf8();
        while let Some((index, next)) = chars.peek().copied() {
            if next.is_ascii_alphanumeric() || next == '_' {
                chars.next();
                end = index + next.len_utf8();
            } else {
                break;
            }
        }
        let ident = &predicate[start..end];
        if !matches!(ident, "true" | "false" | "and" | "or" | "not") {
            references.insert(ident.to_string());
        }
    }
    references.into_iter().collect()
}

fn check_schema_repeat_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    repeat: &crate::types::SchemaRepeatSpec,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if !check_schema_repeat_references(
        schema,
        field,
        decoded_fields,
        &repeat.count_field,
        diagnostics,
    ) {
        return None;
    }
    let element_ty = match &repeat.payload {
        SchemaRepeatPayload::Primitive { .. } => Type::int(),
        SchemaRepeatPayload::ByteView { length_field } => {
            if !check_schema_repeat_byte_view_reference(
                schema,
                field,
                length_field,
                decoded_fields,
                diagnostics,
            ) {
                return None;
            }
            Type::named("ByteView", Vec::new())
        }
        SchemaRepeatPayload::Schema { schema_name } => {
            let payload_schema = resolve_schema_repeat_payload_schema(
                module,
                schema,
                field,
                schema_name,
                diagnostics,
            )?;
            schema_decode_value_type(module, payload_schema).or_else(|| {
                diagnostics.push(schema_repeat_payload_diagnostic(
                    schema,
                    field,
                    schema_name,
                    "incompatible_payload_schema",
                    format!(
                        "repeat payload schema `{}` is not a supported decoded binary schema",
                        schema_payload_name_last_segment(schema_name)
                    ),
                    [],
                ));
                None
            })?
        }
    };
    Some(Type::named("List", vec![element_ty]))
}

fn check_schema_repeat_byte_view_reference(
    schema: &SchemaDecl,
    field: &SchemaField,
    length_field: &str,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(ty) = decoded_fields.get(length_field) else {
        let reason = if schema_field_declared_after(schema, field, length_field) {
            "forward_field_reference"
        } else {
            "unknown_field_reference"
        };
        let mut diagnostic = schema_repeat_payload_diagnostic(
            schema,
            field,
            length_field,
            reason,
            format!(
                "repeat ByteView length field `{length_field}` must be an earlier decoded `Int` field"
            ),
            [],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, "length");
        diagnostics.push(diagnostic);
        return false;
    };
    if ty != &Type::int() {
        let mut diagnostic = schema_repeat_payload_diagnostic(
            schema,
            field,
            length_field,
            "incompatible_field_reference",
            format!(
                "repeat ByteView length field `{length_field}` decodes as `{}`, not `Int`",
                ty.render()
            ),
            [("actual", JsonValue::string(ty.render()))],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, "length");
        diagnostics.push(diagnostic);
        return false;
    }
    true
}

fn check_schema_byte_view_reference(
    schema: &SchemaDecl,
    field: &SchemaField,
    length_expr: &ByteViewLengthExpr,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut valid = true;
    for reference in length_expr.references() {
        let Some(ty) = decoded_fields.get(reference) else {
            let reason = if schema_field_declared_after(schema, field, reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                reason,
                format!(
                    "ByteView length operand `{reference}` must be an earlier decoded `Int` field"
                ),
                [],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
            continue;
        };
        if ty != &Type::int() {
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                "incompatible_field_reference",
                format!(
                    "ByteView length operand `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
        }
    }
    valid
}

fn check_schema_repeat_references(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    count_expr: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(references) = schema_length_expression_references(count_expr) else {
        return false;
    };
    let label = if references.len() == 1 {
        "repeat count field"
    } else {
        "repeat count operand"
    };
    let mut valid = true;
    for reference in references {
        let Some(ty) = decoded_fields.get(reference) else {
            let reason = if schema_field_declared_after(schema, field, reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            let mut diagnostic = schema_repeat_reference_diagnostic(
                schema,
                field,
                reference,
                reason,
                format!("{label} `{reference}` must be an earlier decoded `Int` field"),
                [],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "count",
            );
            diagnostics.push(diagnostic);
            valid = false;
            continue;
        };
        if ty != &Type::int() {
            let mut diagnostic = schema_repeat_reference_diagnostic(
                schema,
                field,
                reference,
                "incompatible_field_reference",
                format!(
                    "{label} `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "count",
            );
            diagnostics.push(diagnostic);
            valid = false;
        }
    }
    valid
}

fn resolve_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    payload_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let Some(segments) = schema_payload_name_path(payload_name) else {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            payload_name,
            "invalid_payload_name",
            format!("repeat payload schema `{payload_name}` is not a valid schema path"),
            [],
        ));
        return None;
    };
    match segments.as_slice() {
        [name] => {
            resolve_local_schema_repeat_payload_schema(module, schema, field, name, diagnostics)
        }
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            ) else {
                diagnostics.push(schema_repeat_payload_diagnostic(
                    schema,
                    field,
                    payload_name,
                    "unknown_import",
                    format!("repeat payload schema `{payload_name}` is not declared"),
                    [],
                ));
                return None;
            };
            resolve_imported_schema_repeat_payload_schema(
                module,
                schema,
                field,
                use_decl,
                payload_name,
                name,
                diagnostics,
            )
        }
        _ => None,
    }
}

fn resolve_local_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| candidate.node_id == schema.node_id)?;
    if let Some((index, candidate)) = module.schemas.iter().enumerate().find(|(_, candidate)| {
        candidate.name.as_deref() == Some(name)
            && candidate.module_name.as_deref() == schema.module_name.as_deref()
    }) {
        if index == current_index {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                name,
                "self_payload_schema",
                format!("repeat payload schema `{name}` cannot reference itself"),
                [],
            ));
            return None;
        }
        if index > current_index {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                name,
                "forward_payload_schema",
                format!(
                    "repeat payload schema `{name}` must be declared before schema `{}`",
                    schema.name.as_deref().unwrap_or("<missing>")
                ),
                [],
            ));
            return None;
        }
        if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                name,
                "non_binary_payload_schema",
                format!("repeat payload schema `{name}` must use `format binary`"),
                [],
            ));
            return None;
        }
        return Some(candidate);
    }
    if let Some(kind) = codec_schema_wrong_kind(module, schema.module_name.as_deref(), name) {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            name,
            "non_schema_payload",
            format!("repeat payload `{name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            name,
            "unknown_payload_schema",
            format!("repeat payload schema `{name}` is not declared"),
            [],
        ));
    }
    None
}

fn resolve_imported_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    use_decl: &UseDecl,
    payload_name: &str,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let target_module = Some(use_decl.name.as_str());
    if let Some(candidate) = module.schemas.iter().find(|candidate| {
        candidate.name.as_deref() == Some(name) && candidate.module_name.as_deref() == target_module
    }) {
        if candidate.visibility != Visibility::Public {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                payload_name,
                "private_imported_payload_schema",
                format!("imported repeat payload schema `{payload_name}` is private"),
                [],
            ));
            return None;
        }
        if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                payload_name,
                "non_binary_payload_schema",
                format!("repeat payload schema `{payload_name}` must use `format binary`"),
                [],
            ));
            return None;
        }
        return Some(candidate);
    }
    if let Some(kind) = codec_schema_wrong_kind(module, target_module, name) {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            payload_name,
            "non_schema_payload",
            format!("repeat payload `{payload_name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            payload_name,
            "unknown_payload_schema",
            format!("repeat payload schema `{payload_name}` is not declared"),
            [],
        ));
    }
    None
}

fn schema_repeat_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("count")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.repeat_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_byte_view_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("length")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.byte_view_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_validation_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = vec![
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>").to_string()),
        ),
        ("reason", JsonValue::string(reason)),
        ("reference", JsonValue::string(reference.to_string())),
    ];
    fields.extend(extra);
    Diagnostic::new(
        "schema.validation_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(validation.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_validation_duplicate_diagnostic(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
) -> Diagnostic {
    Diagnostic::new(
        "schema.validation_duplicate",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "schema `{}` can declare only one schema-level validation",
            schema.name.as_deref().unwrap_or("<missing>")
        ),
        Some(validation.span.clone()),
        JsonValue::object([
            (
                "schema",
                JsonValue::string(schema.name.as_deref().unwrap_or("<missing>").to_string()),
            ),
            ("reason", JsonValue::string("duplicate_validation")),
        ]),
    )
}

fn schema_repeat_payload_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    payload_name: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("payload", JsonValue::string(payload_name.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.repeat_payload",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn check_schema_dispatch_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    let mut valid = true;
    if !check_schema_dispatch_reference(
        schema,
        field,
        decoded_fields,
        &dispatch.tag_field,
        "tag",
        diagnostics,
    ) {
        valid = false;
    }
    if let Some(length_field) = &dispatch.length_field
        && !check_schema_dispatch_reference(
            schema,
            field,
            decoded_fields,
            length_field,
            "length",
            diagnostics,
        )
    {
        valid = false;
    }

    let mut expected_payload_type = None::<Type>;
    let mut mixed_payload_type = false;
    let mut payload_resolution_failed = false;
    for case in &dispatch.cases {
        let payload_ty = match &case.payload {
            SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
            SchemaDispatchCasePayload::Schema { schema_name } => {
                if schema.name.as_deref() == Some(schema_name.as_str()) {
                    if !recursive_dispatch_payload_is_eligible(schema, field, dispatch, schema_name)
                    {
                        diagnostics.push(schema_dispatch_payload_diagnostic(
                            schema,
                            field,
                            case.tag,
                            schema_name,
                            "self_payload_schema",
                            format!(
                                "dispatch payload schema `{schema_name}` cannot reference itself"
                            ),
                            [],
                        ));
                        None
                    } else {
                        schema_recursive_dispatch_payload_type(module, schema).or_else(|| {
                            diagnostics.push(schema_dispatch_payload_diagnostic(
                                schema,
                                field,
                                case.tag,
                                schema_name,
                                "incompatible_payload_schema",
                                format!(
                                    "dispatch payload schema `{}` is not a supported decoded binary schema",
                                    schema_payload_name_last_segment(schema_name)
                                ),
                                [],
                            ));
                            None
                        })
                    }
                } else {
                    resolve_schema_dispatch_payload_schema(
                        module,
                        schema,
                        field,
                        case.tag,
                        schema_name,
                        diagnostics,
                    )
                    .and_then(|payload_schema| {
                        schema_decode_value_type(module, payload_schema).or_else(|| {
                            diagnostics.push(schema_dispatch_payload_diagnostic(
                                schema,
                                field,
                                case.tag,
                                schema_name,
                                "incompatible_payload_schema",
                                format!(
                                    "dispatch payload schema `{}` is not a supported decoded binary schema",
                                    schema_payload_name_last_segment(schema_name)
                                ),
                                [],
                            ));
                            None
                        })
                    })
                }
            }
        };
        let Some(payload_ty) = payload_ty else {
            valid = false;
            payload_resolution_failed = true;
            continue;
        };
        if let Some(expected) = &expected_payload_type {
            if expected != &payload_ty {
                mixed_payload_type = true;
                valid = false;
            }
        } else {
            expected_payload_type = Some(payload_ty);
        }
    }

    let recursive_dispatch_payload = dispatch.cases.iter().any(|case| {
        matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name }
                if recursive_dispatch_payload_case_is_eligible(
                    module,
                    schema,
                    field,
                    dispatch,
                    schema_name,
                )
        )
    });
    if mixed_payload_type
        && (selected_mappings_cover_closed_dispatch(schema, dispatch) || recursive_dispatch_payload)
    {
        valid = !payload_resolution_failed;
    } else if mixed_payload_type && payload_resolution_failed {
        valid = false;
    } else if mixed_payload_type {
        let expected = expected_payload_type.as_ref()?;
        if let Some((case, payload_ty)) = dispatch.cases.iter().find_map(|case| {
            let payload_ty = match &case.payload {
                SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
                SchemaDispatchCasePayload::Schema { schema_name } => {
                    if schema.name.as_deref() == Some(schema_name.as_str()) {
                        schema_recursive_dispatch_payload_type(module, schema)
                    } else {
                        schema_dispatch_payload_schema(module, schema, schema_name).and_then(
                            |payload_schema| schema_decode_value_type(module, payload_schema),
                        )
                    }
                }
            }?;
            (&payload_ty != expected).then_some((case, payload_ty))
        }) {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                schema,
                field,
                case.tag,
                schema_dispatch_case_payload_name(&case.payload),
                "incompatible_payload_type",
                format!(
                    "dispatch payload case `{}` decodes as `{}`, but earlier cases decode as `{}`",
                    case.tag,
                    payload_ty.render(),
                    expected.render()
                ),
                [
                    ("expected", JsonValue::string(expected.render())),
                    ("actual", JsonValue::string(payload_ty.render())),
                ],
            ));
        }
    }

    if !valid {
        return None;
    }
    let payload_ty = expected_payload_type?;
    if dispatch.preserves_unknown {
        Some(Type::named("SchemaDispatchPayload", vec![payload_ty]))
    } else {
        Some(payload_ty)
    }
}

fn check_schema_dispatch_reference(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    reference: &str,
    role: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(ty) = decoded_fields.get(reference) else {
        let reason = if schema_field_declared_after(schema, field, reference) {
            "forward_field_reference"
        } else {
            "unknown_field_reference"
        };
        let mut diagnostic = schema_dispatch_reference_diagnostic(
            schema,
            field,
            reference,
            role,
            reason,
            format!("dispatch {role} field `{reference}` must be an earlier decoded `Int` field"),
            [],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, role);
        diagnostics.push(diagnostic);
        return false;
    };
    if ty != &Type::int() {
        let mut diagnostic = schema_dispatch_reference_diagnostic(
            schema,
            field,
            reference,
            role,
            "incompatible_field_reference",
            format!(
                "dispatch {role} field `{reference}` decodes as `{}`, not `Int`",
                ty.render()
            ),
            [("actual", JsonValue::string(ty.render()))],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, role);
        diagnostics.push(diagnostic);
        return false;
    }
    true
}

fn resolve_schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let context = SchemaDispatchPayloadContext { schema, field, tag };
    let Some(segments) = schema_payload_name_path(payload_name) else {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            schema,
            field,
            tag,
            payload_name,
            "invalid_payload_name",
            format!("dispatch payload schema `{payload_name}` is not a valid schema path"),
            [],
        ));
        return None;
    };
    match segments.as_slice() {
        [name] => resolve_local_schema_dispatch_payload_schema(
            module,
            schema,
            field,
            tag,
            name,
            diagnostics,
        ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            ) else {
                diagnostics.push(schema_dispatch_payload_diagnostic(
                    schema,
                    field,
                    tag,
                    payload_name,
                    "unknown_import",
                    format!("dispatch payload schema `{payload_name}` is not declared"),
                    [],
                ));
                return None;
            };
            resolve_imported_schema_dispatch_payload_schema(
                module,
                context,
                use_decl,
                payload_name,
                name,
                diagnostics,
            )
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct SchemaDispatchPayloadContext<'a> {
    schema: &'a SchemaDecl,
    field: &'a SchemaField,
    tag: i64,
}

fn resolve_local_schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| candidate.node_id == schema.node_id)?;
    if let Some((index, candidate)) = module.schemas.iter().enumerate().find(|(_, candidate)| {
        candidate.name.as_deref() == Some(name)
            && candidate.module_name.as_deref() == schema.module_name.as_deref()
    }) {
        if index == current_index {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                schema,
                field,
                tag,
                name,
                "self_payload_schema",
                format!("dispatch payload schema `{name}` cannot reference itself"),
                [],
            ));
            return None;
        }
        if index > current_index {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                schema,
                field,
                tag,
                name,
                "forward_payload_schema",
                format!(
                    "dispatch payload schema `{name}` must be declared before schema `{}`",
                    schema.name.as_deref().unwrap_or("<missing>")
                ),
                [],
            ));
            return None;
        }
        if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                schema,
                field,
                tag,
                name,
                "non_binary_payload_schema",
                format!("dispatch payload schema `{name}` must use `format binary`"),
                [],
            ));
            return None;
        }
        return Some(candidate);
    }
    if let Some(kind) = codec_schema_wrong_kind(module, schema.module_name.as_deref(), name) {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            schema,
            field,
            tag,
            name,
            "non_schema_payload",
            format!("dispatch payload `{name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            schema,
            field,
            tag,
            name,
            "unknown_payload_schema",
            format!("dispatch payload schema `{name}` is not declared"),
            [],
        ));
    }
    None
}

fn resolve_imported_schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    context: SchemaDispatchPayloadContext<'_>,
    use_decl: &UseDecl,
    payload_name: &str,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let target_module = Some(use_decl.name.as_str());
    if let Some(candidate) = module.schemas.iter().find(|candidate| {
        candidate.name.as_deref() == Some(name) && candidate.module_name.as_deref() == target_module
    }) {
        if candidate.visibility != Visibility::Public {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                context.schema,
                context.field,
                context.tag,
                payload_name,
                "private_imported_payload_schema",
                format!("imported dispatch payload schema `{payload_name}` is private"),
                [],
            ));
            return None;
        }
        if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                context.schema,
                context.field,
                context.tag,
                payload_name,
                "non_binary_payload_schema",
                format!("dispatch payload schema `{payload_name}` must use `format binary`"),
                [],
            ));
            return None;
        }
        return Some(candidate);
    }
    if let Some(kind) = codec_schema_wrong_kind(module, target_module, name) {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            context.schema,
            context.field,
            context.tag,
            payload_name,
            "non_schema_payload",
            format!("dispatch payload `{payload_name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            context.schema,
            context.field,
            context.tag,
            payload_name,
            "unknown_payload_schema",
            format!("dispatch payload schema `{payload_name}` is not declared"),
            [],
        ));
    }
    None
}

fn schema_dispatch_case_payload_name(payload: &SchemaDispatchCasePayload) -> &str {
    match payload {
        SchemaDispatchCasePayload::Primitive { .. } => "<primitive>",
        SchemaDispatchCasePayload::Schema { schema_name } => schema_name,
    }
}

fn schema_field_declared_after(schema: &SchemaDecl, field: &SchemaField, reference: &str) -> bool {
    let current_index = schema
        .fields
        .iter()
        .position(|candidate| candidate.node_id == field.node_id);
    let reference_index = schema
        .fields
        .iter()
        .position(|candidate| candidate.name == reference);
    matches!((current_index, reference_index), (Some(current), Some(reference)) if reference > current)
}

fn add_compatible_prior_int_field_related(
    diagnostic: &mut Diagnostic,
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    role: &str,
) {
    let expected_type = Type::int();
    let int_field = |field: &&SchemaField| decoded_fields.get(&field.name) == Some(&expected_type);
    let Some(candidate_field) = schema
        .fields
        .iter()
        .find(|field| int_field(field) && field.name.contains(role))
        .or_else(|| schema.fields.iter().find(int_field))
    else {
        return;
    };
    let candidate_name = &candidate_field.name;
    diagnostic.related.push(JsonValue::object([
        ("span", span_json(&candidate_field.span)),
        (
            "message",
            JsonValue::string(format!(
                "Compatible earlier {role} field `{candidate_name}` is declared here."
            )),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    (
                        "name",
                        JsonValue::string(
                            schema.name.as_deref().unwrap_or("<missing>").to_string(),
                        ),
                    ),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string(candidate_name.clone())),
                ]),
            ]),
        ),
    ]));
}

fn schema_dispatch_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    role: &'static str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string(role)));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.dispatch_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_dispatch_payload_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("case_tag", JsonValue::Number(tag)));
    fields.push(("payload", JsonValue::string(payload_name.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.dispatch_payload",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_dispatch_details(
    schema: &SchemaDecl,
    field: &SchemaField,
    reason: &'static str,
) -> Vec<(&'static str, JsonValue)> {
    vec![
        ("phase", JsonValue::string("schema")),
        (
            "node_id",
            JsonValue::string(field.node_id.display("schema-field")),
        ),
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ),
        ("field", JsonValue::string(field.name.clone())),
        ("reason", JsonValue::string(reason)),
    ]
}

pub(crate) fn check_schema_mappings(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for schema in &module.schemas {
        let Some(schema_fields) = generated_schema_field_types(module, schema) else {
            continue;
        };
        let selection_diagnostics =
            schema_mapping_selection_diagnostics(module, schema, &schema_fields);
        let has_selection_errors = !selection_diagnostics.is_empty();
        diagnostics.extend(selection_diagnostics);
        if has_selection_errors {
            continue;
        }
        for mapping in &schema.mappings {
            let Some(target) = &mapping.target else {
                continue;
            };
            let target_fields = schema_mapping_target_record_fields(module, schema, mapping);
            let Some(target_fields) = target_fields else {
                diagnostics.push(schema_mapping_target_diagnostic(schema, mapping, target));
                continue;
            };
            let target_field_types = target_fields.into_iter().collect::<BTreeMap<_, _>>();
            let mapping_schema_fields =
                schema_mapping_source_field_types(module, schema, &schema_fields, mapping)
                    .unwrap_or_else(|| schema_fields.clone());
            let mut assigned_targets = BTreeMap::<String, SourceSpan>::new();
            for assignment in &mapping.assignments {
                let Some(target_ty) = target_field_types.get(&assignment.target) else {
                    diagnostics.push(schema_mapping_target_field_diagnostic(
                        schema, mapping, assignment,
                    ));
                    continue;
                };
                if let Err(error) = schema_mapping_assignment_expr_typed(
                    module,
                    schema,
                    &mapping_schema_fields,
                    assignment,
                    target_ty,
                ) {
                    diagnostics.push(schema_mapping_expr_diagnostic(
                        schema, mapping, assignment, *error,
                    ));
                }
                if let Some(first_span) =
                    assigned_targets.insert(assignment.target.clone(), assignment.span.clone())
                {
                    diagnostics.push(schema_mapping_duplicate_target_diagnostic(
                        schema,
                        mapping,
                        assignment,
                        &first_span,
                    ));
                }
            }
            for target_field in target_field_types.keys() {
                if !assigned_targets.contains_key(target_field) {
                    diagnostics.push(schema_mapping_missing_target_diagnostic(
                        schema,
                        mapping,
                        target_field,
                    ));
                }
            }
        }
    }

    diagnostics
}

fn schema_mapping_selection_diagnostics(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &BTreeMap<String, Type>,
) -> Vec<Diagnostic> {
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return Vec::new();
    }
    if schema.mappings.len() <= 1 {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();
    let mut seen_selectors = Vec::<(
        &veln_ast::SchemaMappingSelector,
        SchemaMappingSelectorPredicate,
        SourceSpan,
    )>::new();
    let mut target_fields = None::<Vec<(String, Type)>>;
    for mapping in &schema.mappings {
        let Some(selector) = &mapping.selector else {
            diagnostics.push(schema_mapping_selection_required_diagnostic(
                schema, mapping,
            ));
            continue;
        };
        let Ok(predicate) = schema_mapping_selector_predicate(selector) else {
            diagnostics.push(schema_mapping_selection_unsupported_diagnostic(
                schema, mapping, selector,
            ));
            continue;
        };
        let mut selector_fields = BTreeSet::<String>::new();
        predicate.collect_fields(&mut selector_fields);
        let mut selector_valid = true;
        for selector_field in selector_fields {
            if !schema
                .fields
                .iter()
                .any(|field| field.name == selector_field)
            {
                selector_valid = false;
                diagnostics.push(schema_mapping_selection_unknown_field_diagnostic(
                    schema,
                    mapping,
                    selector,
                    &selector_field,
                ));
            } else if schema_fields.get(&selector_field) != Some(&Type::int()) {
                selector_valid = false;
                diagnostics.push(schema_mapping_selection_field_diagnostic(
                    schema,
                    mapping,
                    selector,
                    &selector_field,
                ));
            }
        }
        if !selector_valid {
            continue;
        }
        if let Some((previous, previous_predicate, first_span)) =
            seen_selectors.iter().find(|(_, previous_predicate, _)| {
                schema_mapping_selectors_overlap(previous_predicate, &predicate)
            })
        {
            diagnostics.push(schema_mapping_selection_ambiguous_diagnostic(
                schema,
                mapping,
                selector,
                &predicate,
                previous,
                previous_predicate,
                first_span,
            ));
        }
        seen_selectors.push((selector, predicate, selector.span.clone()));
        if let Some(fields) = schema_mapping_target_record_fields(module, schema, mapping) {
            if let Some(first_fields) = &target_fields {
                if first_fields != &fields {
                    diagnostics.push(schema_mapping_selection_target_diagnostic(schema, mapping));
                }
            } else {
                target_fields = Some(fields);
            }
        }
    }
    diagnostics
}

fn schema_mapping_selection_required_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_selection_required",
        Severity::Error,
        DiagnosticKind::Type,
        "schema mapping clause needs a selector",
        Some(mapping.span.clone()),
        schema_mapping_details(
            mapping.node_id.display("schema-mapping"),
            schema,
            mapping,
            [("reason", JsonValue::string("missing_mapping_selector"))],
        ),
    )
}

fn schema_mapping_selection_field_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    selector: &veln_ast::SchemaMappingSelector,
    selector_field: &str,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_selection",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "schema mapping selector field `{}` is not a decoded Int field",
            selector_field
        ),
        Some(selector.span.clone()),
        schema_mapping_details(
            selector.node_id.display("schema-mapping-selector"),
            schema,
            mapping,
            [
                ("reason", JsonValue::string("selector_field")),
                (
                    "selector_field",
                    JsonValue::string(selector_field.to_string()),
                ),
                (
                    "selector_expression",
                    JsonValue::string(selector.text.clone()),
                ),
            ],
        ),
    )
}

fn schema_mapping_selection_unknown_field_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    selector: &veln_ast::SchemaMappingSelector,
    selector_field: &str,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_selection",
        Severity::Error,
        DiagnosticKind::Type,
        format!("schema mapping selector field `{selector_field}` is not declared"),
        Some(selector.span.clone()),
        schema_mapping_details(
            selector.node_id.display("schema-mapping-selector"),
            schema,
            mapping,
            [
                ("reason", JsonValue::string("unknown_selector_field")),
                (
                    "selector_field",
                    JsonValue::string(selector_field.to_string()),
                ),
                (
                    "selector_expression",
                    JsonValue::string(selector.text.clone()),
                ),
            ],
        ),
    )
}

fn schema_mapping_selection_unsupported_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    selector: &veln_ast::SchemaMappingSelector,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_selection_unsupported",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "schema mapping selector expression `{}` is not supported",
            selector.text
        ),
        Some(selector.span.clone()),
        schema_mapping_details(
            selector.node_id.display("schema-mapping-selector"),
            schema,
            mapping,
            [
                (
                    "reason",
                    JsonValue::string("unsupported_selector_expression"),
                ),
                (
                    "selector_expression",
                    JsonValue::string(selector.text.clone()),
                ),
            ],
        ),
    )
}

fn schema_mapping_selection_ambiguous_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    selector: &veln_ast::SchemaMappingSelector,
    predicate: &SchemaMappingSelectorPredicate,
    previous: &veln_ast::SchemaMappingSelector,
    previous_predicate: &SchemaMappingSelectorPredicate,
    first_span: &SourceSpan,
) -> Diagnostic {
    let duplicate = selector.text == previous.text;
    let message = if duplicate {
        format!("schema mapping selector `{}` is duplicated", selector.text)
    } else {
        format!(
            "schema mapping selector `{}` overlaps `{}`",
            selector.text, previous.text
        )
    };
    let mut details = vec![
        (
            "reason",
            JsonValue::string(if duplicate {
                "duplicate_selector"
            } else {
                "overlapping_selector"
            }),
        ),
        (
            "selector_expression",
            JsonValue::string(selector.text.clone()),
        ),
    ];
    if let Some((field, op, value)) = predicate.as_simple_comparison() {
        details.push(("selector_field", JsonValue::string(field.to_string())));
        details.push((
            "selector_operator",
            JsonValue::string(schema_mapping_selector_op_text(op)),
        ));
        details.push(("selector_value", JsonValue::Number(value)));
    }
    if let Some((field, op, value)) = previous_predicate.as_simple_comparison() {
        details.push((
            "previous_selector_field",
            JsonValue::string(field.to_string()),
        ));
        details.push((
            "previous_selector_operator",
            JsonValue::string(schema_mapping_selector_op_text(op)),
        ));
        details.push(("previous_selector_value", JsonValue::Number(value)));
    }
    let mut diagnostic = Diagnostic::new(
        "schema.mapping_selection_ambiguous",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(selector.span.clone()),
        schema_mapping_details(
            selector.node_id.display("schema-mapping-selector"),
            schema,
            mapping,
            details,
        ),
    );
    diagnostic.related.push(JsonValue::object([
        ("span", span_json(first_span)),
        (
            "message",
            JsonValue::string("Previous overlapping selector is here."),
        ),
    ]));
    diagnostic
}

fn schema_mapping_selector_op_text(op: SchemaMappingSelectorComparison) -> &'static str {
    match op {
        SchemaMappingSelectorComparison::Equal => "==",
        SchemaMappingSelectorComparison::NotEqual => "!=",
    }
}

fn schema_mapping_selection_target_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_selection_unsupported",
        Severity::Error,
        DiagnosticKind::Type,
        "schema mapping selection targets must decode to the same record shape",
        Some(mapping.span.clone()),
        schema_mapping_details(
            mapping.node_id.display("schema-mapping"),
            schema,
            mapping,
            [("reason", JsonValue::string("target_shape_mismatch"))],
        ),
    )
}

fn generated_schema_field_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<BTreeMap<String, Type>> {
    let Type::Record(fields) = schema_decode_record_type(module, schema)? else {
        return None;
    };
    Some(fields.into_iter().collect())
}

fn schema_mapping_target_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    target: &str,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_target",
        Severity::Error,
        DiagnosticKind::Type,
        format!("schema mapping target `{target}` is not a supported record target"),
        Some(mapping.span.clone()),
        schema_mapping_details(
            mapping.node_id.display("schema-mapping"),
            schema,
            mapping,
            [
                ("reason", JsonValue::string("unsupported_target")),
                ("target", JsonValue::string(target.to_string())),
            ],
        ),
    )
}

fn schema_mapping_expr_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    assignment: &SchemaMappingAssignment,
    error: SchemaMappingExprError,
) -> Diagnostic {
    match error {
        SchemaMappingExprError::UnknownSchemaField { name, span } => {
            schema_mapping_source_diagnostic(schema, assignment, &name, span)
        }
        SchemaMappingExprError::Unsupported { text, span } => Diagnostic::new(
            "schema.mapping_expression_unsupported",
            Severity::Error,
            DiagnosticKind::Type,
            format!("schema mapping expression `{text}` is not supported"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("unsupported_expression")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("expression", JsonValue::string(text)),
                ],
            ),
        ),
        SchemaMappingExprError::UnresolvedConstructor { name, span } => Diagnostic::new(
            "schema.mapping_constructor",
            Severity::Error,
            DiagnosticKind::Name,
            format!("schema mapping constructor `{name}` is not resolved"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("unresolved_constructor")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("constructor", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::UnresolvedConverter { name, span } => Diagnostic::new(
            "schema.mapping_converter",
            Severity::Error,
            DiagnosticKind::Name,
            format!("schema mapping converter `{name}` is not resolved"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("unresolved_converter")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("converter", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::PrivateConverter {
            name,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_visibility",
                Severity::Error,
                DiagnosticKind::Name,
                format!("schema mapping converter `{name}` is private"),
                Some(span),
                schema_mapping_assignment_details(
                    assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    assignment,
                    [
                        ("reason", JsonValue::string("private_converter")),
                        (
                            "mapping_target",
                            JsonValue::string(mapping.target.clone().unwrap_or_default()),
                        ),
                        ("converter", JsonValue::string(name)),
                    ],
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ConstructorArity {
            name,
            expected,
            actual,
            span,
        } => Diagnostic::new(
            "schema.mapping_constructor_arity",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "schema mapping constructor `{name}` expects {expected} argument(s), but got {actual}"
            ),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("constructor_arity_mismatch")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("constructor", JsonValue::string(name)),
                    (
                        "expected_argument_count",
                        JsonValue::Number(expected as i64),
                    ),
                    ("actual_argument_count", JsonValue::Number(actual as i64)),
                ],
            ),
        ),
        SchemaMappingExprError::ConverterArity {
            name,
            expected,
            actual,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_arity",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "schema mapping converter `{name}` expects {expected} argument(s), but got {actual}"
                ),
                Some(span),
                schema_mapping_assignment_details(
                    assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    assignment,
                    [
                        ("reason", JsonValue::string("converter_arity_mismatch")),
                        (
                            "mapping_target",
                            JsonValue::string(mapping.target.clone().unwrap_or_default()),
                        ),
                        ("converter", JsonValue::string(name)),
                        (
                            "expected_argument_count",
                            JsonValue::Number(expected as i64),
                        ),
                        ("actual_argument_count", JsonValue::Number(actual as i64)),
                    ],
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ConverterInputType {
            name,
            expected,
            actual,
            input,
            span,
            function_span,
        } => {
            let message = match &input {
                SchemaMappingConverterInput::SourceField(source) => format!(
                    "schema mapping converter `{name}` expects `{}`, but source field `{source}` decodes as `{}`",
                    expected.render(),
                    actual.render()
                ),
                SchemaMappingConverterInput::Expression(text) => format!(
                    "schema mapping converter `{name}` expects `{}`, but argument expression `{text}` has type `{}`",
                    expected.render(),
                    actual.render()
                ),
            };
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_input",
                Severity::Error,
                DiagnosticKind::Type,
                message,
                Some(span),
                schema_mapping_converter_details(SchemaMappingConverterDetails {
                    node_id: assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    mapping,
                    assignment,
                    reason: "converter_input_type_mismatch",
                    converter: &name,
                    input: &input,
                    expected: &expected,
                    actual: &actual,
                }),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ConverterReturnType {
            name,
            expected,
            actual,
            input,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_return",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "schema mapping converter `{name}` returns `{}`, but target field `{}` expects `{}`",
                    actual.render(),
                    assignment.target,
                    expected.render()
                ),
                Some(span),
                schema_mapping_converter_details(SchemaMappingConverterDetails {
                    node_id: assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    mapping,
                    assignment,
                    reason: "converter_return_type_mismatch",
                    converter: &name,
                    input: &input,
                    expected: &expected,
                    actual: &actual,
                }),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ImpureConverter {
            name,
            effects,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_purity",
                Severity::Error,
                DiagnosticKind::Effect,
                format!("schema mapping converter `{name}` must be pure"),
                Some(span),
                schema_mapping_assignment_details(
                    assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    assignment,
                    [
                        ("reason", JsonValue::string("impure_converter")),
                        (
                            "mapping_target",
                            JsonValue::string(mapping.target.clone().unwrap_or_default()),
                        ),
                        ("converter", JsonValue::string(name)),
                        (
                            "effects",
                            JsonValue::array(effects.iter().cloned().map(JsonValue::string)),
                        ),
                    ],
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::RecordField { name, span } => Diagnostic::new(
            "schema.mapping_record_field",
            Severity::Error,
            DiagnosticKind::Name,
            format!("schema mapping record field `{name}` is not expected"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("unexpected_record_field")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("record_field", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::MissingRecordField { name, span } => Diagnostic::new(
            "schema.mapping_record_field",
            Severity::Error,
            DiagnosticKind::Name,
            format!("schema mapping record expression does not assign field `{name}`"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("missing_record_field")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("record_field", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::TypeMismatch {
            expected,
            actual,
            text,
            span,
        } => schema_mapping_type_diagnostic(
            schema, mapping, assignment, &expected, &actual, text, span,
        ),
    }
}

fn schema_mapping_source_diagnostic(
    schema: &SchemaDecl,
    assignment: &SchemaMappingAssignment,
    source: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_source_field",
        Severity::Error,
        DiagnosticKind::Name,
        format!("schema mapping source field `{source}` is not declared"),
        Some(span),
        schema_mapping_assignment_details(
            assignment.node_id.display("schema-mapping-assignment"),
            schema,
            assignment,
            [
                ("reason", JsonValue::string("unknown_source_field")),
                (
                    "missing_source_field",
                    JsonValue::string(source.to_string()),
                ),
            ],
        ),
    )
}

fn schema_mapping_target_field_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    assignment: &SchemaMappingAssignment,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_target_field",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "schema mapping target field `{}` is not declared",
            assignment.target
        ),
        Some(assignment.span.clone()),
        schema_mapping_assignment_details(
            assignment.node_id.display("schema-mapping-assignment"),
            schema,
            assignment,
            [
                ("reason", JsonValue::string("unknown_target_field")),
                (
                    "mapping_target",
                    JsonValue::string(mapping.target.clone().unwrap_or_default()),
                ),
            ],
        ),
    )
}

fn schema_mapping_type_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    assignment: &SchemaMappingAssignment,
    target_ty: &Type,
    source_ty: &Type,
    source_text: String,
    span: SourceSpan,
) -> Diagnostic {
    let message = if assignment.source == source_text {
        format!(
            "schema mapping target field `{}` expects `{}`, but source field `{}` decodes as `{}`",
            assignment.target,
            target_ty.render(),
            source_text,
            source_ty.render()
        )
    } else {
        format!(
            "schema mapping target field `{}` expects `{}`, but expression `{}` has type `{}`",
            assignment.target,
            target_ty.render(),
            source_text,
            source_ty.render()
        )
    };
    Diagnostic::new(
        "schema.mapping_type",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(span),
        schema_mapping_assignment_details(
            assignment.node_id.display("schema-mapping-assignment"),
            schema,
            assignment,
            [
                ("reason", JsonValue::string("field_type_mismatch")),
                (
                    "mapping_target",
                    JsonValue::string(mapping.target.clone().unwrap_or_default()),
                ),
                ("expected", JsonValue::string(target_ty.render())),
                ("actual", JsonValue::string(source_ty.render())),
                ("expression", JsonValue::string(source_text)),
            ],
        ),
    )
}

fn schema_mapping_duplicate_target_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    assignment: &SchemaMappingAssignment,
    first_span: &SourceSpan,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "schema.mapping_duplicate_target_field",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "schema mapping assigns target field `{}` more than once",
            assignment.target
        ),
        Some(assignment.span.clone()),
        schema_mapping_assignment_details(
            assignment.node_id.display("schema-mapping-assignment"),
            schema,
            assignment,
            [
                ("reason", JsonValue::string("duplicate_target_field")),
                (
                    "mapping_target",
                    JsonValue::string(mapping.target.clone().unwrap_or_default()),
                ),
            ],
        ),
    );
    diagnostic.related.push(JsonValue::object([
        ("span", span_json(first_span)),
        (
            "message",
            JsonValue::string(format!(
                "First assignment to `{}` is here.",
                assignment.target
            )),
        ),
    ]));
    diagnostic
}

fn schema_mapping_missing_target_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    target_field: &str,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_missing_target_field",
        Severity::Error,
        DiagnosticKind::Name,
        format!("schema mapping does not assign target field `{target_field}`"),
        Some(mapping.span.clone()),
        schema_mapping_details(
            mapping.node_id.display("schema-mapping"),
            schema,
            mapping,
            [
                ("reason", JsonValue::string("missing_target_field")),
                ("target_field", JsonValue::string(target_field.to_string())),
            ],
        ),
    )
}

fn schema_mapping_details(
    node_id: String,
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    extra: impl IntoIterator<Item = (&'static str, JsonValue)>,
) -> JsonValue {
    let mut fields = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(node_id)),
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ),
    ];
    if let Some(target) = &mapping.target {
        fields.push(("mapping_target", JsonValue::string(target.clone())));
    }
    fields.extend(extra);
    JsonValue::object(fields)
}

fn schema_mapping_assignment_details<const N: usize>(
    node_id: String,
    schema: &SchemaDecl,
    assignment: &SchemaMappingAssignment,
    extra: [(&'static str, JsonValue); N],
) -> JsonValue {
    let mut fields = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(node_id)),
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ),
        ("target_field", JsonValue::string(assignment.target.clone())),
        ("source_field", JsonValue::string(assignment.source.clone())),
    ];
    fields.extend(extra);
    JsonValue::object(fields)
}

struct SchemaMappingConverterDetails<'a> {
    node_id: String,
    schema: &'a SchemaDecl,
    mapping: &'a SchemaMappingClause,
    assignment: &'a SchemaMappingAssignment,
    reason: &'static str,
    converter: &'a str,
    input: &'a SchemaMappingConverterInput,
    expected: &'a Type,
    actual: &'a Type,
}

fn schema_mapping_converter_details(details: SchemaMappingConverterDetails<'_>) -> JsonValue {
    let mut fields = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(details.node_id)),
        (
            "schema",
            JsonValue::string(details.schema.name.as_deref().unwrap_or("<missing>")),
        ),
        (
            "target_field",
            JsonValue::string(details.assignment.target.clone()),
        ),
        (
            "source_field",
            JsonValue::string(details.assignment.source.clone()),
        ),
        ("reason", JsonValue::string(details.reason)),
        (
            "mapping_target",
            JsonValue::string(details.mapping.target.clone().unwrap_or_default()),
        ),
        (
            "converter",
            JsonValue::string(details.converter.to_string()),
        ),
        ("expected", JsonValue::string(details.expected.render())),
        ("actual", JsonValue::string(details.actual.render())),
    ];
    match details.input {
        SchemaMappingConverterInput::SourceField(source) => {
            fields.push(("input_source_field", JsonValue::string(source.clone())));
        }
        SchemaMappingConverterInput::Expression(text) => {
            fields.push(("input_expression", JsonValue::string(text.clone())));
        }
    }
    JsonValue::object(fields)
}

fn push_schema_type_reference_diagnostics(
    module: &SurfaceModule,
    current_module: Option<&str>,
    annotation: &str,
    node_id: String,
    span: SourceSpan,
    use_kind: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Ok(ty) = parse_type_annotation(annotation) else {
        return;
    };
    let mut schemas = Vec::new();
    collect_schema_type_references(module, current_module, &ty, &mut schemas);
    for schema in schemas {
        diagnostics.push(schema_type_reference_diagnostic(
            schema,
            node_id.clone(),
            span.clone(),
            use_kind,
        ));
    }
    let mut primitives = Vec::new();
    collect_exact_width_schema_primitive_references(&ty, &mut primitives);
    for primitive in primitives {
        diagnostics.push(exact_width_schema_primitive_diagnostic(
            primitive,
            None,
            None,
            node_id.clone(),
            span.clone(),
            use_kind,
        ));
    }
}

fn collect_schema_type_references<'a>(
    module: &'a SurfaceModule,
    current_module: Option<&str>,
    ty: &Type,
    schemas: &mut Vec<&'a SchemaDecl>,
) {
    match ty {
        Type::Named { name, args } => {
            if let Some(schema) = schema_for_type_name(module, current_module, name) {
                schemas.push(schema);
            }
            for arg in args {
                collect_schema_type_references(module, current_module, arg, schemas);
            }
        }
        Type::Record(fields) => {
            for (_, field_ty) in fields {
                collect_schema_type_references(module, current_module, field_ty, schemas);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_schema_type_references(module, current_module, param, schemas);
            }
            collect_schema_type_references(module, current_module, return_type, schemas);
        }
        Type::Unknown => {}
    }
}

fn collect_exact_width_schema_primitive_references<'a>(
    ty: &'a Type,
    primitives: &mut Vec<&'a str>,
) {
    match ty {
        Type::Named { name, args } => {
            if let Some(primitive) = exact_width_binary_primitive_name(name) {
                primitives.push(primitive);
            }
            for arg in args {
                collect_exact_width_schema_primitive_references(arg, primitives);
            }
        }
        Type::Record(fields) => {
            for (_, field_ty) in fields {
                collect_exact_width_schema_primitive_references(field_ty, primitives);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_exact_width_schema_primitive_references(param, primitives);
            }
            collect_exact_width_schema_primitive_references(return_type, primitives);
        }
        Type::Unknown => {}
    }
}

fn schema_for_type_name<'a>(
    module: &'a SurfaceModule,
    current_module: Option<&str>,
    name: &str,
) -> Option<&'a SchemaDecl> {
    let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
    match segments.as_slice() {
        [name] => module.schemas.iter().find(|schema| {
            schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == current_module
        }),
        [_, .., name] => {
            let module_name = imported_module_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            module.schemas.iter().find(|schema| {
                schema.name.as_deref() == Some(name)
                    && schema.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}

fn schema_type_reference_diagnostic(
    schema: &SchemaDecl,
    node_id: String,
    span: SourceSpan,
    use_kind: &'static str,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    Diagnostic::new(
        "type.schema_reference",
        Severity::Error,
        DiagnosticKind::Type,
        format!("schema `{schema_name}` cannot be used as an ordinary type"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("type")),
            ("node_id", JsonValue::string(node_id)),
            ("schema", JsonValue::string(schema_name)),
            ("use_kind", JsonValue::string(use_kind)),
        ]),
    )
}

pub(in crate::analysis) fn exact_width_binary_primitive_name(name: &str) -> Option<&'static str> {
    match name {
        "UInt1" => Some("UInt1"),
        "UInt2" => Some("UInt2"),
        "UInt3" => Some("UInt3"),
        "UInt4" => Some("UInt4"),
        "UInt5" => Some("UInt5"),
        "UInt6" => Some("UInt6"),
        "UInt7" => Some("UInt7"),
        "UInt8" => Some("UInt8"),
        "UInt16be" => Some("UInt16be"),
        "UInt16le" => Some("UInt16le"),
        "UInt24be" => Some("UInt24be"),
        "UInt24le" => Some("UInt24le"),
        "UInt31be" => Some("UInt31be"),
        "UInt31le" => Some("UInt31le"),
        "UInt32be" => Some("UInt32be"),
        "UInt32le" => Some("UInt32le"),
        "UInt48be" => Some("UInt48be"),
        "UInt48le" => Some("UInt48le"),
        "UInt64be" => Some("UInt64be"),
        "UInt64le" => Some("UInt64le"),
        _ => None,
    }
}

pub(in crate::analysis) fn exact_width_schema_primitive_diagnostic(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: &'static str,
) -> Diagnostic {
    let mut details = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(node_id)),
        ("primitive", JsonValue::string(primitive.to_string())),
        ("reason", JsonValue::string(reason)),
    ];
    if let Some(schema) = schema {
        details.push((
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ));
    }
    if let Some(field) = field {
        details.push(("field", JsonValue::string(field.name.clone())));
    }
    Diagnostic::new(
        "schema.exact_width_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "binary schema primitive `{primitive}` can only be used in a `format binary` schema field"
        ),
        Some(span),
        JsonValue::object(details),
    )
}

fn reserved_bits_primitive(ty: &str) -> Option<Result<(i64, i64), ReservedBitsArgumentReason>> {
    let rest = ty.strip_prefix("ReservedBits")?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return Some(Err(ReservedBitsArgumentReason::Arity));
    }
    if !rest.starts_with('(') {
        return None;
    }
    if !rest.ends_with(')') {
        return Some(Err(ReservedBitsArgumentReason::Arity));
    }
    let inner = rest[1..rest.len() - 1].trim();
    let args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    if args.len() != 2 {
        return Some(Err(ReservedBitsArgumentReason::Arity));
    }
    let Ok(width) = parse_reserved_bits_integer(args[0]) else {
        return Some(Err(ReservedBitsArgumentReason::Literal));
    };
    let Ok(value) = parse_reserved_bits_integer(args[1]) else {
        return Some(Err(ReservedBitsArgumentReason::Literal));
    };
    Some(Ok((width, value)))
}

fn parse_reserved_bits_integer(text: &str) -> Result<i64, ()> {
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(());
    }
    text.parse::<i64>().map_err(|_| ())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReservedBitsArgumentReason {
    Arity,
    Literal,
}

fn reserved_bits_format_diagnostic(schema: &SchemaDecl, field: &SchemaField) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    Diagnostic::new(
        "schema.reserved_bits_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        "`ReservedBits` can only be used in a `format binary` schema field",
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("primitive", JsonValue::string("ReservedBits")),
            ("reason", JsonValue::string("non_binary_format")),
        ]),
    )
}

fn reserved_bits_argument_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
    reason: ReservedBitsArgumentReason,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let reason_text = match reason {
        ReservedBitsArgumentReason::Arity => "argument_count",
        ReservedBitsArgumentReason::Literal => "non_literal_argument",
    };
    let message = match reason {
        ReservedBitsArgumentReason::Arity => {
            "`ReservedBits` requires width and value integer arguments"
        }
        ReservedBitsArgumentReason::Literal => {
            "`ReservedBits` arguments must be literal non-negative integers"
        }
    };
    Diagnostic::new(
        "schema.reserved_bits_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("primitive", JsonValue::string("ReservedBits")),
            ("reason", JsonValue::string(reason_text)),
        ]),
    )
}

fn reserved_bits_encode_shape_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
    reserved: (i64, i64),
) -> Diagnostic {
    Diagnostic::new(
        "schema.reserved_bits_encode",
        Severity::Error,
        DiagnosticKind::Type,
        "`ReservedBits` encode support does not cover this field layout",
        Some(field.span.clone()),
        JsonValue::object([
            (
                "schema",
                JsonValue::string(schema.name.clone().unwrap_or_default()),
            ),
            ("field", JsonValue::string(field.name.clone())),
            ("primitive", JsonValue::string("ReservedBits")),
            ("bit_width", JsonValue::Number(reserved.0)),
            ("expected_value", JsonValue::Number(reserved.1)),
            ("reason", JsonValue::string("unsupported_encode_shape")),
        ]),
    )
}

fn function_target<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a veln_ast::Function> {
    match segments {
        [name] => module.functions.iter().find(|function| {
            function.kind == FunctionKind::Function && function.name.as_deref() == Some(name)
        }),
        [_, .., name] => {
            let module_name = imported_module_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            module.functions.iter().find(|function| {
                function.kind == FunctionKind::Function
                    && function.name.as_deref() == Some(name)
                    && function.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}

fn type_target<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a veln_ast::TypeDecl> {
    match segments {
        [name] => module
            .types
            .iter()
            .find(|type_decl| type_decl.name.as_deref() == Some(name)),
        [_, .., name] => {
            let module_name = imported_module_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            module.types.iter().find(|type_decl| {
                type_decl.name.as_deref() == Some(name)
                    && type_decl.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}

fn imported_module_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a str> {
    imported_use_for_path(uses, segments, current_module).map(|use_decl| use_decl.name.as_str())
}

fn imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

fn unresolved_alias_diagnostic(
    alias: &veln_ast::PublicAlias,
    expected_kind: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "name.unresolved",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "unresolved {expected_kind} alias target `{}`",
            alias.target.join("::")
        ),
        Some(alias.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(alias.node_id.display("alias"))),
            ("expected_kind", JsonValue::string(expected_kind)),
            ("target", JsonValue::string(alias.target.join("::"))),
        ]),
    )
}

fn private_alias_diagnostic(alias: &veln_ast::PublicAlias) -> Diagnostic {
    Diagnostic::new(
        "name.visibility",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "schema alias target `{}` is private",
            alias.target.join("::")
        ),
        Some(alias.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(alias.node_id.display("alias"))),
            ("expected_kind", JsonValue::string("schema")),
            ("target", JsonValue::string(alias.target.join("::"))),
            ("reason", JsonValue::string("private")),
        ]),
    )
}

fn alias_kind_mismatch_diagnostic(
    alias: &veln_ast::PublicAlias,
    expected_kind: &'static str,
    actual_kind: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "name.kind_mismatch",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "public alias target `{}` is a {actual_kind}, not a {expected_kind}",
            alias.target.join("::")
        ),
        Some(alias.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(alias.node_id.display("alias"))),
            ("expected_kind", JsonValue::string(expected_kind)),
            ("actual_kind", JsonValue::string(actual_kind)),
            ("target", JsonValue::string(alias.target.join("::"))),
        ]),
    )
}

pub(crate) fn check_duplicate_use_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for use_decl in &module.uses {
        let node_id = use_decl.node_id.display("use");
        let key = (use_decl.module_name.clone(), use_decl.alias.clone());
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                &use_decl.alias,
                "module",
                "import alias",
                node_id,
                use_decl.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, use_decl.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_reserved_prelude_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(header) = &module.module
        && header.name == PRELUDE_MODULE
        && !is_standard_prelude_source(&header.span)
    {
        diagnostics.push(reserved_prelude_diagnostic(
            header.node_id.display("mod"),
            header.span.clone(),
            "module",
            "module identity",
            "Choose a non-conflicting module name.",
        ));
    }

    for use_decl in &module.uses {
        if use_decl.alias == PRELUDE_MODULE {
            diagnostics.push(reserved_prelude_diagnostic(
                use_decl.node_id.display("use"),
                use_decl.span.clone(),
                "module",
                "import alias",
                "Choose a non-conflicting import path.",
            ));
        }
    }

    diagnostics
}

fn is_standard_prelude_source(span: &SourceSpan) -> bool {
    span.file.as_str() == "prelude.veln"
}

fn reserved_prelude_diagnostic(
    node_id: String,
    span: SourceSpan,
    namespace: &'static str,
    declaration_kind: &'static str,
    hint: &'static str,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "name.reserved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("{declaration_kind} `prelude` conflicts with the standard prelude"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(node_id)),
            ("name", JsonValue::string(PRELUDE_MODULE)),
            ("namespace", JsonValue::string(namespace)),
            ("reserved_for", JsonValue::string("standard_prelude")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        ("message", JsonValue::string(hint)),
    ]));
    diagnostic
}

pub(crate) fn check_duplicate_constructor_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen =
        BTreeMap::<(Option<String>, Option<String>, String), (String, SourceSpan)>::new();

    for type_decl in &module.types {
        for variant in &type_decl.variants {
            let Some(name) = &variant.name else {
                continue;
            };
            let key = (
                type_decl.module_name.clone(),
                type_decl.name.clone(),
                name.clone(),
            );
            let node_id = variant.node_id.display("variant");
            if let Some((first_node_id, first_span)) = seen.get(&key) {
                diagnostics.push(duplicate_name_diagnostic(
                    name,
                    "constructor",
                    "constructor declaration",
                    node_id,
                    variant.span.clone(),
                    first_node_id.clone(),
                    first_span,
                ));
            } else {
                seen.insert(key, (node_id, variant.span.clone()));
            }
        }
    }

    diagnostics
}

pub(crate) fn check_module_boundary(module: &SurfaceModule) -> Vec<Diagnostic> {
    if module.module.is_some() || module.uses.is_empty() {
        return Vec::new();
    }

    let first_use = &module.uses[0];
    let mut diagnostic = Diagnostic::new(
        "module.missing_identity",
        Severity::Error,
        DiagnosticKind::Module,
        "module import requires a module identity",
        Some(first_use.span.clone()),
        module_details(
            first_use.node_id.display("use"),
            "module_identity",
            "source",
            "missing",
        ),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string("Add a `mod` declaration before `use` declarations."),
        ),
    ]));
    vec![diagnostic]
}

pub(super) fn duplicate_name_diagnostic(
    name: &str,
    namespace: &'static str,
    declaration_kind: &'static str,
    node_id: String,
    span: SourceSpan,
    first_node_id: String,
    first_span: &SourceSpan,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "name.duplicate",
        Severity::Error,
        DiagnosticKind::Name,
        format!("duplicate {declaration_kind} name `{name}`"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(node_id)),
            ("name", JsonValue::string(name)),
            ("namespace", JsonValue::string(namespace)),
            ("first_node_id", JsonValue::string(first_node_id)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("duplicate_origin")),
        (
            "message",
            JsonValue::string(format!("First {declaration_kind} with this name is here.")),
        ),
        ("span", span_json(first_span)),
    ]));
    diagnostic
}

pub(crate) fn check_test_declaration_boundary(function: &Function) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let node_id = function.node_id.display(function.kind.node_prefix());

    if let Some(param) = function.params.first() {
        let mut diagnostic = Diagnostic::new(
            "test.parameters",
            Severity::Error,
            DiagnosticKind::Type,
            "test declaration has parameters",
            Some(param.span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("test")),
                ("node_id", JsonValue::string(node_id.clone())),
                ("expected_parameters", JsonValue::Number(0)),
                (
                    "actual_parameters",
                    JsonValue::Number(function.params.len() as i64),
                ),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("test_shape")),
            (
                "message",
                JsonValue::string("A test declaration uses an empty parameter list."),
            ),
            ("span", span_json(&function.span)),
        ]));
        diagnostics.push(diagnostic);
    }

    match function.return_type.as_deref() {
        Some(return_type) => {
            if let Ok(ty) = parse_type_annotation(return_type)
                && !is_allowed_test_return(&ty)
            {
                diagnostics.push(test_return_diagnostic(
                    function,
                    &node_id,
                    format!("test declaration returns `{}`", ty.render()),
                    ty.render(),
                ));
            }
        }
        None => diagnostics.push(test_return_diagnostic(
            function,
            &node_id,
            "test declaration has no return type annotation".to_string(),
            "missing".to_string(),
        )),
    }

    diagnostics
}

fn is_allowed_test_return(ty: &Type) -> bool {
    ty == &Type::unit() || adt::result_parts(ty).is_some_and(|(value, _)| value == &Type::unit())
}

pub(super) fn type_contains_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } => args.iter().any(type_contains_unknown),
        Type::Record(fields) => fields.iter().any(|(_, ty)| type_contains_unknown(ty)),
        Type::Function {
            params,
            return_type,
            ..
        } => params.iter().any(type_contains_unknown) || type_contains_unknown(return_type),
    }
}

fn test_return_diagnostic(
    function: &Function,
    node_id: &str,
    message: String,
    actual_type: String,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "test.return_type",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(function.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("test")),
            ("node_id", JsonValue::string(node_id)),
            ("expected_type", JsonValue::string("() or Result<(), E>")),
            ("actual_type", JsonValue::string(actual_type)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("test_shape")),
        (
            "message",
            JsonValue::string("A test declaration returns `()` or `Result<(), E>`."),
        ),
        ("span", span_json(&function.span)),
    ]));
    diagnostic
}
