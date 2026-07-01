use super::*;
use crate::prelude::PRELUDE_MODULE;
use crate::types::{
    ByteViewLengthExpr, LowercaseSchemaPrimitiveError, SchemaDispatchCasePayload,
    SchemaDispatchSpec, SchemaRepeatPayload, byte_view_multiple_constraint,
    byte_view_schema_primitive, closed_dispatch_schema_primitive, exact_width_schema_primitive,
    exact_width_schema_primitive_bit_width, extension_dispatch_schema_primitive,
    flag_schema_primitive, format_neutral_schema_field_type,
    lowercase_reserved_bits_schema_primitive, lowercase_schema_primitive,
    lowercase_schema_primitive_nested_payloads,
    recursive_dispatch_decode_only_payload_case_is_eligible,
    recursive_dispatch_payload_case_is_eligible, recursive_dispatch_payload_is_eligible,
    repeat_schema_primitive, reserved_bits_schema_primitive, schema_decode_step_function_name,
    schema_decode_value_type, schema_dispatch_payload_accepts_lowercase_primitive,
    schema_dispatch_payload_schema, schema_encode_function_name, schema_encode_value_type,
    schema_has_eligible_recursive_dispatch_payload, schema_has_recursive_dispatch_payload,
    schema_length_expression_references, schema_payload_name_last_segment,
    schema_payload_name_path, schema_recursive_dispatch_helper_payload_type,
    schema_recursive_dispatch_payload_type, supported_encode_reserved_bits,
};
use std::collections::BTreeSet;
use veln_ast::{PublicAliasKind, SchemaDecl, SchemaField, SchemaValidationClause, UseDecl};

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

#[derive(Clone, Copy, Debug)]
enum SchemaAliasCheckResolution {
    Resolved,
    Private,
    WrongKind(&'static str),
    Unresolved,
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

    diagnostics
}

pub(crate) fn check_schema_field_primitives(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for schema in &module.schemas {
        let format_name = schema.format.as_ref().map(|format| format.name.as_str());
        let mut decoded_fields = BTreeMap::<String, Type>::new();
        for field in &schema.fields {
            if let Some(reserved) = lowercase_reserved_bits_schema_primitive(&field.ty) {
                match (format_name, reserved) {
                    (Some("binary"), Ok(reserved)) => {
                        check_schema_non_byte_view_multiple(schema, field, &mut diagnostics);
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
                                schema,
                                field,
                                field_index,
                                reserved,
                            ));
                        }
                    }
                    (Some("binary"), Err(reason)) => {
                        diagnostics.push(lowercase_schema_primitive_diagnostic(
                            &field.ty,
                            Some(schema),
                            Some(field),
                            field.node_id.display("schema-field"),
                            field.span.clone(),
                            reason,
                        ));
                    }
                    (_, Ok(_)) => {
                        diagnostics.push(lowercase_schema_primitive_position_diagnostic(
                            &field.ty,
                            Some(schema),
                            Some(field),
                            field.node_id.display("schema-field"),
                            field.span.clone(),
                            "non_binary_format",
                        ));
                    }
                    (_, Err(reason)) => {
                        diagnostics.push(lowercase_schema_primitive_diagnostic(
                            &field.ty,
                            Some(schema),
                            Some(field),
                            field.node_id.display("schema-field"),
                            field.span.clone(),
                            reason,
                        ));
                    }
                }
                continue;
            }
            if let Some(primitive) = lowercase_schema_primitive(&field.ty) {
                match (format_name, primitive) {
                    (Some("binary"), Ok(primitive)) => {
                        check_schema_non_byte_view_multiple(schema, field, &mut diagnostics);
                        if primitive.family == "flag" {
                            decoded_fields.insert(
                                field.name.clone(),
                                Type::named(primitive.canonical_name(), Vec::new()),
                            );
                        } else {
                            decoded_fields.insert(field.name.clone(), Type::int());
                        }
                    }
                    (Some("binary"), Err(reason)) => {
                        diagnostics.push(lowercase_schema_primitive_diagnostic(
                            &field.ty,
                            Some(schema),
                            Some(field),
                            field.node_id.display("schema-field"),
                            field.span.clone(),
                            reason,
                        ));
                    }
                    (_, Ok(_)) => {
                        diagnostics.push(lowercase_schema_primitive_position_diagnostic(
                            &field.ty,
                            Some(schema),
                            Some(field),
                            field.node_id.display("schema-field"),
                            field.span.clone(),
                            "non_binary_format",
                        ));
                    }
                    (_, Err(reason)) => {
                        diagnostics.push(lowercase_schema_primitive_diagnostic(
                            &field.ty,
                            Some(schema),
                            Some(field),
                            field.node_id.display("schema-field"),
                            field.span.clone(),
                            reason,
                        ));
                    }
                }
                continue;
            }
            if let Some(flag_type) = flag_schema_primitive(&field.ty) {
                if format_name == Some("binary") {
                    check_schema_non_byte_view_multiple(schema, field, &mut diagnostics);
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
                    check_schema_non_byte_view_multiple(schema, field, &mut diagnostics);
                    decoded_fields.insert(field.name.clone(), Type::int());
                }
                continue;
            }
            let lowercase_nested_payloads = lowercase_schema_primitive_nested_payloads(&field.ty);
            if !lowercase_nested_payloads.is_empty() {
                let mut pushed_diagnostic = false;
                for (primitive, reason) in lowercase_nested_payloads {
                    if format_name == Some("binary")
                        && reason == "dispatch_payload"
                        && schema_dispatch_payload_accepts_lowercase_primitive(primitive)
                    {
                        continue;
                    }
                    let reason = if format_name == Some("binary") {
                        reason
                    } else {
                        "non_binary_format"
                    };
                    diagnostics.push(lowercase_schema_primitive_position_diagnostic(
                        primitive,
                        Some(schema),
                        Some(field),
                        field.node_id.display("schema-field"),
                        field.span.clone(),
                        reason,
                    ));
                    pushed_diagnostic = true;
                }
                if pushed_diagnostic {
                    continue;
                }
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
                ) && check_schema_byte_view_multiple(
                    schema,
                    field,
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
                check_schema_non_byte_view_multiple(schema, field, &mut diagnostics);
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
                check_schema_non_byte_view_multiple(schema, field, &mut diagnostics);
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
                        check_schema_non_byte_view_multiple(schema, field, &mut diagnostics);
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
                                schema,
                                field,
                                field_index,
                                reserved,
                            ));
                        }
                    }
                }
                continue;
            }
            if format_name.is_none() {
                if let Some(field_ty) = format_neutral_schema_field_type(&field.ty) {
                    decoded_fields.insert(field.name.clone(), field_ty);
                } else {
                    diagnostics.push(format_neutral_schema_helper_diagnostic(schema, field));
                }
                continue;
            }
            check_schema_non_byte_view_multiple(schema, field, &mut diagnostics);
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

fn format_neutral_schema_helper_diagnostic(schema: &SchemaDecl, field: &SchemaField) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let supported = "supported scalar, top-level List<Int>, List<Bool>, List<Float>, or List<String>, top-level Dict<String, Int>, Dict<String, Bool>, or Dict<String, String>, Option, or record-shaped field type";
    let boundary_message = format!(
        "Generated format-neutral decode helpers for schema `{schema_name}` accept only scalar fields, top-level List<Int>, List<Bool>, List<Float>, or List<String> fields, top-level Dict<String, Int>, Dict<String, Bool>, or Dict<String, String> fields, supported Option fields, and nested record-shaped fields."
    );
    let mut diagnostic = Diagnostic::new(
        "schema.format_neutral_decode_helper",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "format-neutral schema field `{}` cannot expose a generated decode helper because `{}` is not a {supported}",
            field.name, field.ty,
        ),
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("field_type", JsonValue::string(field.ty.clone())),
            (
                "reason",
                JsonValue::string("unsupported_format_neutral_field_type"),
            ),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("schema_helper_boundary")),
        ("span", span_json(&schema.span)),
        ("message", JsonValue::string(boundary_message)),
    ]));
    diagnostic
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
    length_expr: &str,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(references) = schema_length_expression_references(length_expr) else {
        return false;
    };
    let mut valid = true;
    for reference in references {
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
                    "repeat ByteView length operand `{reference}` must be an earlier decoded `Int` field"
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
                    "repeat ByteView length operand `{reference}` decodes as `{}`, not `Int`",
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

fn check_schema_byte_view_multiple(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(where_clause) = &field.where_clause else {
        return true;
    };
    let Some(constraint) = byte_view_multiple_constraint(&where_clause.predicate) else {
        diagnostics.push(schema_byte_view_multiple_diagnostic(
            schema,
            field,
            &where_clause.predicate,
            "unsupported_multiple_predicate",
            "ByteView field validation must use `payload_count multiple of <field-or-positive-integer>`".to_string(),
            [],
        ));
        return false;
    };
    let Some(reference) = constraint.reference() else {
        return true;
    };
    let Some(ty) = decoded_fields.get(reference) else {
        let reason = if schema_field_declared_after(schema, field, reference) {
            "forward_field_reference"
        } else {
            "unknown_field_reference"
        };
        let mut diagnostic = schema_byte_view_multiple_diagnostic(
            schema,
            field,
            reference,
            reason,
            format!(
                "ByteView multiple operand `{reference}` must be an earlier decoded `Int` field"
            ),
            [],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, "multiple");
        diagnostics.push(diagnostic);
        return false;
    };
    if ty != &Type::int() {
        let mut diagnostic = schema_byte_view_multiple_diagnostic(
            schema,
            field,
            reference,
            "incompatible_field_reference",
            format!(
                "ByteView multiple operand `{reference}` decodes as `{}`, not `Int`",
                ty.render()
            ),
            [("actual", JsonValue::string(ty.render()))],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, "multiple");
        diagnostics.push(diagnostic);
        return false;
    }
    true
}

fn check_schema_non_byte_view_multiple(
    schema: &SchemaDecl,
    field: &SchemaField,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(where_clause) = &field.where_clause else {
        return;
    };
    if !where_clause
        .predicate
        .trim()
        .starts_with("payload_count multiple of ")
    {
        return;
    }
    diagnostics.push(schema_byte_view_multiple_diagnostic(
        schema,
        field,
        &where_clause.predicate,
        "invalid_field_kind",
        "ByteView multiple validation can only be used on length-bounded `ByteView` fields"
            .to_string(),
        [],
    ));
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

fn schema_byte_view_multiple_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("multiple")));
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
            SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
            SchemaDispatchCasePayload::Schema { schema_name } => {
                if schema.name.as_deref() == Some(schema_name.as_str()) {
                    if !recursive_dispatch_payload_is_eligible(schema, field, dispatch, schema_name)
                    {
                        let blocker = recursive_dispatch_payload_blocker(
                            schema,
                            field,
                            dispatch,
                            schema_name,
                            schema,
                        );
                        diagnostics.push(schema_dispatch_payload_diagnostic(
                            schema,
                            field,
                            case.tag,
                            schema_name,
                            blocker.reason,
                            blocker.message,
                            [(
                                "recursive_helper_fact",
                                JsonValue::string(blocker.fact.to_string()),
                            )],
                        ));
                        None
                    } else {
                        schema_recursive_dispatch_payload_type(module, schema).or_else(|| {
                            diagnostics.push(incompatible_schema_dispatch_payload_diagnostic(
                                module,
                                schema,
                                field,
                                case.tag,
                                schema_name,
                                schema,
                                SchemaHelperAvailability {
                                    decode: false,
                                    encode: false,
                                },
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
                        if schema_has_recursive_dispatch_payload(payload_schema)
                            && !(recursive_dispatch_payload_case_is_eligible(
                                module,
                                schema,
                                field,
                                dispatch,
                                schema_name,
                            ) || recursive_dispatch_decode_only_payload_case_is_eligible(
                                module,
                                schema,
                                dispatch,
                                schema_name,
                            ))
                        {
                            let blocker = recursive_dispatch_payload_blocker(
                                schema,
                                field,
                                dispatch,
                                schema_name,
                                payload_schema,
                            );
                            diagnostics.push(schema_dispatch_payload_diagnostic(
                                schema,
                                field,
                                case.tag,
                                schema_name,
                                blocker.reason,
                                blocker.message,
                                [(
                                    "recursive_helper_fact",
                                    JsonValue::string(blocker.fact.to_string()),
                                )],
                            ));
                            return None;
                        }
                        schema_dispatch_payload_helper_type(
                            module,
                            schema,
                            field,
                            case.tag,
                            schema_name,
                            payload_schema,
                            diagnostics,
                        )
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
                    ) || recursive_dispatch_decode_only_payload_case_is_eligible(
                        module,
                        schema,
                        dispatch,
                        schema_name,
                    )
        )
    });
    if mixed_payload_type && recursive_dispatch_payload {
        valid = !payload_resolution_failed;
    } else if mixed_payload_type && payload_resolution_failed {
        valid = false;
    } else if mixed_payload_type {
        let expected = expected_payload_type.as_ref()?;
        if let Some((case, payload_ty)) = dispatch.cases.iter().find_map(|case| {
            let payload_ty = match &case.payload {
                SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
                SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
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
    let payload_ty = if recursive_dispatch_payload {
        schema_recursive_dispatch_helper_payload_type(module, schema, dispatch)?
    } else {
        expected_payload_type?
    };
    if dispatch.preserves_unknown {
        Some(Type::named("SchemaDispatchPayload", vec![payload_ty]))
    } else {
        Some(payload_ty)
    }
}

struct RecursiveDispatchPayloadBlocker {
    reason: &'static str,
    fact: &'static str,
    message: String,
}

fn recursive_dispatch_payload_blocker(
    _parent_schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
    payload_schema: &SchemaDecl,
) -> RecursiveDispatchPayloadBlocker {
    if dispatch.length_field.is_none() {
        return RecursiveDispatchPayloadBlocker {
            reason: "recursive_payload_missing_length_bound",
            fact: "recursive dispatch payloads require a length-bounded parent dispatch field",
            message: format!(
                "dispatch payload schema `{schema_name}` requires parent dispatch field `{}` to include a length field",
                field.name
            ),
        };
    }
    if !dispatch
        .cases
        .iter()
        .any(|case| matches!(case.payload, SchemaDispatchCasePayload::Primitive { .. }))
    {
        return RecursiveDispatchPayloadBlocker {
            reason: "recursive_payload_missing_primitive_base_case",
            fact: "recursive dispatch parents require a non-recursive primitive base case",
            message: format!(
                "dispatch payload schema `{schema_name}` requires parent dispatch field `{}` to include a non-recursive primitive case",
                field.name
            ),
        };
    }
    if !schema_has_eligible_recursive_dispatch_payload(payload_schema) {
        return RecursiveDispatchPayloadBlocker {
            reason: "recursive_payload_missing_bounded_helper",
            fact: "recursive dispatch payload schemas must expose a bounded recursive helper",
            message: format!(
                "dispatch payload schema `{schema_name}` does not expose a bounded recursive helper"
            ),
        };
    }
    RecursiveDispatchPayloadBlocker {
        reason: "recursive_payload_ineligible_parent",
        fact: "recursive dispatch payloads require a length-bounded parent with recursive helper support and a non-recursive base case",
        message: format!(
            "dispatch payload schema `{schema_name}` does not satisfy recursive dispatch helper requirements"
        ),
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
        SchemaDispatchCasePayload::ReservedBits { .. } => "<reserved>",
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

fn incompatible_schema_dispatch_payload_diagnostic(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    payload_schema: &SchemaDecl,
    helper_availability: SchemaHelperAvailability,
) -> Diagnostic {
    let payload_schema_name = schema_payload_name_last_segment(payload_name);
    let decode_helper = schema_decode_step_function_name(payload_schema_name);
    let encode_helper = schema_encode_function_name(payload_schema_name);
    let unsupported_blocker =
        unsupported_dispatch_payload_helper_blocker(module, payload_schema, helper_availability);
    let mut diagnostic = schema_dispatch_payload_diagnostic(
        schema,
        field,
        tag,
        payload_name,
        "incompatible_payload_schema",
        format!(
            "dispatch payload schema `{payload_schema_name}` is outside the generated binary schema helper slice"
        ),
        [
            (
                "expected_decode_helper",
                JsonValue::string(decode_helper.clone()),
            ),
            (
                "decode_helper_boundary",
                JsonValue::string("generated_binary_schema_decode_step"),
            ),
            (
                "expected_encode_helper",
                JsonValue::string(encode_helper.clone()),
            ),
            (
                "encode_helper_boundary",
                JsonValue::string("generated_binary_schema_encode"),
            ),
        ],
    );
    diagnostic.details =
        add_dispatch_payload_helper_unavailable_details(diagnostic.details, helper_availability);
    if let Some(unsupported_blocker) = &unsupported_blocker {
        diagnostic.details = add_dispatch_payload_unsupported_blocker_details(
            diagnostic.details,
            unsupported_blocker,
        );
    }
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("schema_declaration")),
        (
            "message",
            JsonValue::string(dispatch_payload_schema_declaration_message(
                payload_schema_name,
                &decode_helper,
                &encode_helper,
                helper_availability,
            )),
        ),
        ("span", span_json(&payload_schema.span)),
    ]));
    if let Some(unsupported_blocker) = unsupported_blocker {
        diagnostic
            .related
            .push(dispatch_payload_unsupported_blocker_related(
                &unsupported_blocker,
            ));
    }
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("helper_boundary")),
        (
            "message",
            JsonValue::string(dispatch_payload_helper_boundary_message(
                &decode_helper,
                &encode_helper,
                helper_availability,
            )),
        ),
    ]));
    diagnostic
}

#[derive(Clone, Copy)]
struct SchemaHelperAvailability {
    decode: bool,
    encode: bool,
}

fn schema_dispatch_payload_helper_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    payload_schema: &SchemaDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    let decode_type = schema_decode_value_type(module, payload_schema);
    match decode_type {
        Some(payload_ty) => Some(payload_ty),
        None => {
            let encode_type = schema_encode_value_type(module, payload_schema);
            diagnostics.push(incompatible_schema_dispatch_payload_diagnostic(
                module,
                schema,
                field,
                tag,
                payload_name,
                payload_schema,
                SchemaHelperAvailability {
                    decode: false,
                    encode: encode_type.is_some(),
                },
            ));
            None
        }
    }
}

fn dispatch_payload_schema_declaration_message(
    payload_schema_name: &str,
    decode_helper: &str,
    encode_helper: &str,
    helper_availability: SchemaHelperAvailability,
) -> String {
    match (helper_availability.decode, helper_availability.encode) {
        (false, false) => format!(
            "Schema `{payload_schema_name}` is declared here and does not expose the generated `{decode_helper}` helper required for dispatch payload decoding."
        ),
        (false, true) => format!(
            "Schema `{payload_schema_name}` is declared here and does not expose the generated `{decode_helper}` helper required for dispatch payload decoding."
        ),
        (true, false) => format!(
            "Schema `{payload_schema_name}` is declared here and does not expose the generated `{encode_helper}` helper required for dispatch payload encoding."
        ),
        (true, true) => format!(
            "Schema `{payload_schema_name}` is declared here and exposes the generated dispatch payload helpers."
        ),
    }
}

fn dispatch_payload_helper_boundary_message(
    decode_helper: &str,
    encode_helper: &str,
    helper_availability: SchemaHelperAvailability,
) -> String {
    match (helper_availability.decode, helper_availability.encode) {
        (false, false) | (true, true) => format!(
            "Dispatch payload schemas must expose generated decode helpers before parent decode helpers can use them, and generated encode helpers before parent encode helpers can use them; expected `{decode_helper}` and `{encode_helper}`."
        ),
        (false, true) => format!(
            "Dispatch payload schemas must expose generated decode helpers before parent decode helpers can use them; expected `{decode_helper}`."
        ),
        (true, false) => format!(
            "Dispatch payload schemas must expose generated encode helpers before parent encode helpers can use them; expected `{encode_helper}`."
        ),
    }
}

struct UnsupportedDispatchPayloadHelperField<'a> {
    schema_name: String,
    field: &'a SchemaField,
    field_path_display: String,
    layout_fact: String,
    reason: &'static str,
}

enum UnsupportedDispatchPayloadHelperBlocker<'a> {
    Field(UnsupportedDispatchPayloadHelperField<'a>),
}

fn unsupported_dispatch_payload_helper_blocker<'a>(
    _module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    _helper_availability: SchemaHelperAvailability,
) -> Option<UnsupportedDispatchPayloadHelperBlocker<'a>> {
    unsupported_dispatch_payload_helper_field(schema)
        .map(UnsupportedDispatchPayloadHelperBlocker::Field)
}

fn unsupported_dispatch_payload_helper_field(
    schema: &SchemaDecl,
) -> Option<UnsupportedDispatchPayloadHelperField<'_>> {
    let schema_name = schema.name.clone().unwrap_or_default();
    let mut decoded_fields = BTreeMap::<String, Type>::new();
    for (index, field) in schema.fields.iter().enumerate() {
        if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
            if supported_encode_reserved_bits(&schema.fields, index, reserved).is_none() {
                let layout =
                    reserved_bits_unsupported_layout_context(schema, Some(index), reserved.0);
                return Some(UnsupportedDispatchPayloadHelperField {
                    field,
                    field_path_display: format!("{schema_name}.{}", field.name),
                    layout_fact: format!(
                        "`ReservedBits({}, {})` is outside the supported `{}` layout: {}",
                        reserved.0,
                        reserved.1,
                        layout.supported_layout_family,
                        layout.human_supported_note
                    ),
                    reason: "unsupported_reserved_bits_layout",
                    schema_name,
                });
            }
            continue;
        }
        if let Some(width) = exact_width_schema_primitive(&field.ty) {
            let ty = if let Some(flag_type) = flag_schema_primitive(&field.ty) {
                Type::named(flag_type, Vec::new())
            } else {
                Type::int()
            };
            decoded_fields.insert(field.name.clone(), ty);
            if width == 0 {
                return None;
            }
            continue;
        }
        if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
            if let Some(reference) = length_expr
                .references()
                .into_iter()
                .find(|reference| decoded_fields.get(*reference) != Some(&Type::int()))
            {
                let reference_fact =
                    byte_view_ineligible_length_fact(schema, field, &decoded_fields, reference);
                return Some(UnsupportedDispatchPayloadHelperField {
                    field,
                    field_path_display: format!("{schema_name}.{}", field.name),
                    layout_fact: format!(
                        "`ByteView({})` requires {reference_fact}",
                        length_expr.render()
                    ),
                    reason: "ineligible_byte_view_length_reference",
                    schema_name,
                });
            }
            decoded_fields.insert(field.name.clone(), Type::named("ByteView", Vec::new()));
            continue;
        }
    }
    None
}

fn byte_view_ineligible_length_fact(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    reference: &str,
) -> String {
    if let Some(actual) = decoded_fields.get(reference) {
        format!(
            "length reference `{reference}` to decode as `Int`; it decodes as `{}`",
            actual.render()
        )
    } else if schema_field_declared_after(schema, field, reference) {
        format!(
            "length reference `{reference}` to be declared before field `{}`",
            field.name
        )
    } else {
        format!("length reference `{reference}` to name an earlier decoded `Int` field")
    }
}

fn add_dispatch_payload_helper_unavailable_details(
    details: JsonValue,
    helper_availability: SchemaHelperAvailability,
) -> JsonValue {
    let JsonValue::Object(mut fields) = details else {
        return details;
    };
    fields.push((
        "unavailable_helper_directions".to_string(),
        JsonValue::array(dispatch_payload_unavailable_helper_directions(
            helper_availability,
        )),
    ));
    JsonValue::Object(fields)
}

fn dispatch_payload_unavailable_helper_directions(
    helper_availability: SchemaHelperAvailability,
) -> Vec<JsonValue> {
    let mut directions = Vec::new();
    if !helper_availability.decode {
        directions.push(JsonValue::string("decode"));
    }
    if !helper_availability.encode {
        directions.push(JsonValue::string("encode"));
    }
    directions
}

fn add_dispatch_payload_unsupported_blocker_details(
    details: JsonValue,
    unsupported: &UnsupportedDispatchPayloadHelperBlocker<'_>,
) -> JsonValue {
    match unsupported {
        UnsupportedDispatchPayloadHelperBlocker::Field(field) => {
            add_dispatch_payload_unsupported_field_details(details, field)
        }
    }
}

fn add_dispatch_payload_unsupported_field_details(
    details: JsonValue,
    unsupported: &UnsupportedDispatchPayloadHelperField<'_>,
) -> JsonValue {
    let JsonValue::Object(mut fields) = details else {
        return details;
    };
    fields.push((
        "unsupported_nested_schema".to_string(),
        JsonValue::string(unsupported.schema_name.clone()),
    ));
    fields.push((
        "unsupported_nested_field".to_string(),
        JsonValue::string(unsupported.field.name.clone()),
    ));
    fields.push((
        "unsupported_nested_field_path".to_string(),
        dispatch_payload_unsupported_field_path(unsupported),
    ));
    fields.push((
        "unsupported_nested_layout_reason".to_string(),
        JsonValue::string(unsupported.reason),
    ));
    fields.push((
        "unsupported_nested_layout_fact".to_string(),
        JsonValue::string(unsupported.layout_fact.clone()),
    ));
    JsonValue::Object(fields)
}

fn dispatch_payload_unsupported_blocker_related(
    unsupported: &UnsupportedDispatchPayloadHelperBlocker<'_>,
) -> JsonValue {
    match unsupported {
        UnsupportedDispatchPayloadHelperBlocker::Field(field) => {
            dispatch_payload_unsupported_field_related(field)
        }
    }
}

fn dispatch_payload_unsupported_field_related(
    unsupported: &UnsupportedDispatchPayloadHelperField<'_>,
) -> JsonValue {
    let layout_fact = unsupported.layout_fact.trim_end_matches('.');
    JsonValue::object([
        ("kind", JsonValue::string("unsupported_nested_field")),
        ("span", span_json(&unsupported.field.span)),
        (
            "field_path",
            dispatch_payload_unsupported_field_path(unsupported),
        ),
        (
            "message",
            JsonValue::string(format!(
                "Nested dispatch payload field `{}` prevents generated decode and encode helpers: {}.",
                unsupported.field_path_display, layout_fact
            )),
        ),
    ])
}

fn dispatch_payload_unsupported_field_path(
    unsupported: &UnsupportedDispatchPayloadHelperField<'_>,
) -> JsonValue {
    JsonValue::array([
        JsonValue::object([
            ("kind", JsonValue::string("schema")),
            ("name", JsonValue::string(unsupported.schema_name.clone())),
        ]),
        JsonValue::object([
            ("kind", JsonValue::string("field")),
            ("name", JsonValue::string(unsupported.field.name.clone())),
        ]),
    ])
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
        ("field_path", schema_dispatch_field_path(schema, field)),
        ("reason", JsonValue::string(reason)),
    ]
}

fn schema_dispatch_field_path(schema: &SchemaDecl, field: &SchemaField) -> JsonValue {
    JsonValue::array([
        JsonValue::object([
            ("kind", JsonValue::string("schema")),
            (
                "name",
                JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
            ),
        ]),
        JsonValue::object([
            ("kind", JsonValue::string("field")),
            ("name", JsonValue::string(field.name.clone())),
        ]),
    ])
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
    let mut lowercase_primitives = Vec::new();
    collect_lowercase_schema_primitive_references(&ty, &mut lowercase_primitives);
    for primitive in lowercase_primitives {
        diagnostics.push(lowercase_schema_primitive_position_diagnostic(
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

fn collect_lowercase_schema_primitive_references<'a>(ty: &'a Type, primitives: &mut Vec<&'a str>) {
    match ty {
        Type::Named { name, args } => {
            if lowercase_schema_primitive(name).is_some() {
                primitives.push(name);
            }
            for arg in args {
                collect_lowercase_schema_primitive_references(arg, primitives);
            }
        }
        Type::Record(fields) => {
            for (_, field_ty) in fields {
                collect_lowercase_schema_primitive_references(field_ty, primitives);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_lowercase_schema_primitive_references(param, primitives);
            }
            collect_lowercase_schema_primitive_references(return_type, primitives);
        }
        Type::Unknown => {}
    }
}

pub(in crate::analysis) fn lowercase_schema_primitive_diagnostic(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: LowercaseSchemaPrimitiveError,
) -> Diagnostic {
    let reason_text = match reason {
        LowercaseSchemaPrimitiveError::MissingWidth => "missing_width",
        LowercaseSchemaPrimitiveError::UnknownEndian => "unknown_endian",
        LowercaseSchemaPrimitiveError::MissingEndian => "missing_endian",
        LowercaseSchemaPrimitiveError::RedundantEndian => "redundant_endian",
        LowercaseSchemaPrimitiveError::UnsupportedWidth => "unsupported_width",
        LowercaseSchemaPrimitiveError::ReservesOnFlag => "reserves_on_flag",
        LowercaseSchemaPrimitiveError::ReservesValue => "reserves_value",
    };
    let message = match reason {
        LowercaseSchemaPrimitiveError::MissingWidth => {
            format!("binary schema primitive `{primitive}` must specify a width")
        }
        LowercaseSchemaPrimitiveError::UnknownEndian => {
            format!(
                "binary schema primitive `{primitive}` must end with `be` or `le` when it specifies byte order"
            )
        }
        LowercaseSchemaPrimitiveError::MissingEndian => {
            format!("binary schema primitive `{primitive}` requires byte order suffix `be` or `le`")
        }
        LowercaseSchemaPrimitiveError::RedundantEndian => {
            format!("binary schema primitive `{primitive}` must not specify byte order")
        }
        LowercaseSchemaPrimitiveError::UnsupportedWidth => {
            format!("binary schema primitive `{primitive}` uses an unsupported width")
        }
        LowercaseSchemaPrimitiveError::ReservesOnFlag => {
            format!("binary schema primitive `{primitive}` cannot use `reserves` on a flag field")
        }
        LowercaseSchemaPrimitiveError::ReservesValue => {
            format!(
                "binary schema primitive `{primitive}` requires `reserves` value to be a literal non-negative integer"
            )
        }
    };
    lowercase_schema_primitive_diagnostic_with_message(
        primitive,
        schema,
        field,
        node_id,
        span,
        reason_text,
        message,
    )
}

pub(in crate::analysis) fn lowercase_schema_primitive_position_diagnostic(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: &'static str,
) -> Diagnostic {
    let message = match reason {
        "repeat_payload" => format!(
            "binary schema primitive `{primitive}` is not yet supported in `Repeat` payload positions"
        ),
        "dispatch_payload" => format!(
            "binary schema primitive `{primitive}` is not yet supported in dispatch payload positions"
        ),
        _ => format!(
            "binary schema primitive `{primitive}` can only be used in a `format binary` schema field"
        ),
    };
    lowercase_schema_primitive_diagnostic_with_message(
        primitive, schema, field, node_id, span, reason, message,
    )
}

fn lowercase_schema_primitive_diagnostic_with_message(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: &'static str,
    message: String,
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
        "schema.lowercase_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(span),
        JsonValue::object(details),
    )
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
        "UInt40be" => Some("UInt40be"),
        "UInt40le" => Some("UInt40le"),
        "UInt48be" => Some("UInt48be"),
        "UInt48le" => Some("UInt48le"),
        "UInt56be" => Some("UInt56be"),
        "UInt56le" => Some("UInt56le"),
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
    field_index: Option<usize>,
    reserved: (i64, i64),
) -> Diagnostic {
    let layout = reserved_bits_unsupported_layout_context(schema, field_index, reserved.0);
    let mut details = vec![
        (
            "schema",
            JsonValue::string(schema.name.clone().unwrap_or_default()),
        ),
        ("field", JsonValue::string(field.name.clone())),
        ("primitive", JsonValue::string("ReservedBits")),
        ("bit_width", JsonValue::Number(reserved.0)),
        ("expected_value", JsonValue::Number(reserved.1)),
        ("reason", JsonValue::string("unsupported_encode_shape")),
        (
            "supported_layout_family",
            JsonValue::string(layout.supported_layout_family),
        ),
    ];
    if let Some(previous_width) = layout.previous_visible_bit_width {
        details.push((
            "previous_visible_bit_width",
            JsonValue::Number(i64::from(previous_width)),
        ));
    }
    if let Some(next_width) = layout.next_visible_bit_width {
        details.push((
            "next_visible_bit_width",
            JsonValue::Number(i64::from(next_width)),
        ));
    }

    let mut diagnostic = Diagnostic::new(
        "schema.reserved_bits_encode",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "`ReservedBits({}, {})` is outside the supported binary schema field layouts",
            reserved.0, reserved.1
        ),
        Some(field.span.clone()),
        JsonValue::object(details),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(format!(
            "Schema `{}` field `{}` declares ReservedBits({}, {}).",
            schema.name.clone().unwrap_or_default(),
            field.name,
            reserved.0,
            reserved.1
        )),
    )]));
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(layout.human_adjacent_note),
    )]));
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(format!(
            "Supported layout family: {}; {}",
            layout.supported_layout_family, layout.human_supported_note
        )),
    )]));
    diagnostic
}

struct ReservedBitsUnsupportedLayoutContext {
    supported_layout_family: &'static str,
    previous_visible_bit_width: Option<u8>,
    next_visible_bit_width: Option<u8>,
    human_adjacent_note: String,
    human_supported_note: &'static str,
}

fn reserved_bits_unsupported_layout_context(
    schema: &SchemaDecl,
    field_index: Option<usize>,
    bit_width: i64,
) -> ReservedBitsUnsupportedLayoutContext {
    let previous_field = field_index
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| schema.fields.get(index));
    let previous_previous_field = field_index
        .and_then(|index| index.checked_sub(2))
        .and_then(|index| schema.fields.get(index));
    let next_field = field_index.and_then(|index| schema.fields.get(index + 1));
    let previous_previous_visible_bit_width =
        previous_previous_field.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty));
    let previous_visible_bit_width =
        previous_field.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty));
    let next_visible_bit_width =
        next_field.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty));

    let (supported_layout_family, human_supported_note) = reserved_bits_supported_layout_family(
        bit_width,
        previous_previous_visible_bit_width,
        previous_visible_bit_width,
        next_visible_bit_width,
    );
    ReservedBitsUnsupportedLayoutContext {
        supported_layout_family,
        previous_visible_bit_width,
        next_visible_bit_width,
        human_adjacent_note: reserved_bits_adjacent_width_note(
            previous_field,
            previous_visible_bit_width,
            next_field,
            next_visible_bit_width,
        ),
        human_supported_note,
    }
}

fn reserved_bits_supported_layout_family(
    bit_width: i64,
    previous_previous_visible_bit_width: Option<u8>,
    previous_visible_bit_width: Option<u8>,
    next_visible_bit_width: Option<u8>,
) -> (&'static str, &'static str) {
    if previous_visible_bit_width.is_some() && next_visible_bit_width.is_some() {
        return (
            "middle_reserved_bits",
            "visible and reserved widths must complete one supported big-endian storage unit.",
        );
    }
    if previous_visible_bit_width.is_some()
        && suffix_packed_reserved_storage_bit_width(bit_width).is_some()
    {
        if previous_visible_bit_width == Some(8)
            && previous_previous_visible_bit_width
                .is_some_and(|width| i64::from(width) + 8 + bit_width == 16)
            && (1..=7).contains(&bit_width)
        {
            return (
                "suffix_reserved_group",
                "two visible widths plus the reserved width must complete the same two-byte big-endian storage unit.",
            );
        }
        return (
            "packed_reserved_suffix",
            "the previous visible width plus the reserved width must complete one supported big-endian storage unit.",
        );
    }
    if next_visible_bit_width.is_some() && packed_reserved_storage_bit_width(bit_width).is_some() {
        return (
            "packed_reserved_prefix",
            "the reserved width plus the next visible width must complete one supported big-endian storage unit.",
        );
    }
    if bit_width > 0 && bit_width <= 32 && bit_width % 8 == 0 {
        return (
            "byte_aligned_reserved_bits",
            "byte-aligned reserved fields are supported up to four bytes when the value fits the width.",
        );
    }
    (
        "bit_packed_reserved_group",
        "a bit-packed group must contain at least one visible field and complete one supported big-endian storage unit.",
    )
}

fn packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    if (1..=7).contains(&bit_width) {
        Some(8)
    } else if (9..=15).contains(&bit_width) {
        Some(16)
    } else if (17..=23).contains(&bit_width) {
        Some(24)
    } else if (25..=31).contains(&bit_width) {
        Some(32)
    } else {
        None
    }
}

fn suffix_packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    packed_reserved_storage_bit_width(bit_width).or_else(|| {
        if (33..=39).contains(&bit_width) {
            Some(40)
        } else if (41..=47).contains(&bit_width) {
            Some(48)
        } else {
            None
        }
    })
}

fn reserved_bits_adjacent_width_note(
    previous_field: Option<&SchemaField>,
    previous_visible_bit_width: Option<u8>,
    next_field: Option<&SchemaField>,
    next_visible_bit_width: Option<u8>,
) -> String {
    let mut parts = Vec::new();
    if let (Some(field), Some(width)) = (previous_field, previous_visible_bit_width) {
        parts.push(format!("previous `{}` is {} bit(s)", field.name, width));
    }
    if let (Some(field), Some(width)) = (next_field, next_visible_bit_width) {
        parts.push(format!("next `{}` is {} bit(s)", field.name, width));
    }
    if parts.is_empty() {
        "No adjacent visible exact-width field participates in this unsupported layout.".to_string()
    } else {
        format!("Adjacent visible field widths: {}.", parts.join("; "))
    }
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
