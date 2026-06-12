use super::*;
use crate::prelude::PRELUDE_MODULE;
use veln_ast::{
    CodecDecl, CodecDirection, CodecImplementationClause, CodecImplementationKind, PublicAliasKind,
    SchemaDecl, SchemaField, SchemaMappingAssignment, SchemaMappingClause, UseDecl,
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
            CodecSchemaResolution::Resolved => {}
            CodecSchemaResolution::Private => {
                diagnostics.push(private_codec_schema_diagnostic(codec, schema_name));
            }
            CodecSchemaResolution::WrongKind(actual_kind) => {
                diagnostics.push(codec_schema_kind_mismatch_diagnostic(
                    codec,
                    schema_name,
                    actual_kind,
                ));
            }
            CodecSchemaResolution::Unresolved => {
                diagnostics.push(unresolved_codec_schema_diagnostic(codec, schema_name));
            }
        }
    }

    diagnostics
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CodecSchemaResolution {
    Resolved,
    Private,
    WrongKind(&'static str),
    Unresolved,
}

fn resolve_codec_schema_reference(
    module: &SurfaceModule,
    codec: &CodecDecl,
    schema_name: &str,
) -> CodecSchemaResolution {
    let current_module = codec.module_name.as_deref();
    let segments = schema_name
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    match segments.as_slice() {
        [name] => resolve_local_codec_schema_reference(module, current_module, name),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            ) else {
                return CodecSchemaResolution::Unresolved;
            };
            resolve_imported_codec_schema_reference(module, use_decl, name)
        }
        _ => CodecSchemaResolution::Unresolved,
    }
}

fn resolve_local_codec_schema_reference(
    module: &SurfaceModule,
    current_module: Option<&str>,
    name: &str,
) -> CodecSchemaResolution {
    if module.schemas.iter().any(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == current_module
    }) {
        return CodecSchemaResolution::Resolved;
    }
    codec_schema_wrong_kind(module, current_module, name).map_or(
        CodecSchemaResolution::Unresolved,
        CodecSchemaResolution::WrongKind,
    )
}

fn resolve_imported_codec_schema_reference(
    module: &SurfaceModule,
    use_decl: &UseDecl,
    name: &str,
) -> CodecSchemaResolution {
    let target_module = Some(use_decl.name.as_str());
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == target_module
    }) {
        return if schema.visibility == Visibility::Public {
            CodecSchemaResolution::Resolved
        } else {
            CodecSchemaResolution::Private
        };
    }
    codec_schema_wrong_kind(module, target_module, name).map_or(
        CodecSchemaResolution::Unresolved,
        CodecSchemaResolution::WrongKind,
    )
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
    None
}

pub(crate) fn check_codec_decode_signatures(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for codec in &module.codecs {
        for implementation in codec.implementations.iter().filter(|implementation| {
            implementation.direction == CodecDirection::Decode
                && matches!(implementation.kind, CodecImplementationKind::With { .. })
        }) {
            let CodecImplementationKind::With { function } = &implementation.kind else {
                continue;
            };
            let Some(function_name) = function else {
                continue;
            };
            let Some(function) = codec_same_module_function(module, codec, function_name) else {
                diagnostics.push(unresolved_codec_decode_function_diagnostic(
                    codec,
                    implementation,
                    function_name,
                ));
                continue;
            };

            diagnostics.extend(codec_decode_signature_diagnostics(
                codec,
                implementation,
                function,
                function_name,
            ));
        }
    }

    diagnostics
}

pub(crate) fn check_codec_encode_signatures(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for codec in &module.codecs {
        for implementation in codec.implementations.iter().filter(|implementation| {
            implementation.direction == CodecDirection::Encode
                && matches!(implementation.kind, CodecImplementationKind::With { .. })
        }) {
            let CodecImplementationKind::With { function } = &implementation.kind else {
                continue;
            };
            let Some(function_name) = function else {
                continue;
            };
            let Some(function) = codec_same_module_function(module, codec, function_name) else {
                diagnostics.push(unresolved_codec_encode_function_diagnostic(
                    codec,
                    implementation,
                    function_name,
                ));
                continue;
            };

            diagnostics.extend(codec_encode_signature_diagnostics(
                codec,
                implementation,
                function,
                function_name,
            ));
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
    }

    diagnostics
}

fn is_decode_step_return(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { name, args } if name == "DecodeStep" && args.len() == 1
    )
}

fn codec_encode_signature_diagnostics(
    codec: &CodecDecl,
    implementation: &CodecImplementationClause,
    function: &Function,
    function_name: &str,
) -> Vec<Diagnostic> {
    let return_type = function
        .return_type
        .as_deref()
        .and_then(|annotation| parse_type_annotation(annotation).ok())
        .unwrap_or(Type::Unknown);

    if is_encode_step_return(&return_type) {
        return Vec::new();
    }

    vec![codec_encode_signature_diagnostic(
        codec,
        implementation,
        Some(function),
        function_name,
        "return_type",
        "encode function must return `EncodeStep<TState>`",
        return_type.render(),
    )]
}

fn is_encode_step_return(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { name, args } if name == "EncodeStep" && args.len() == 1
    )
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

pub(crate) fn check_public_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
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
        }
    }
    diagnostics
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
        for field in &schema.fields {
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
                }
                continue;
            }
            let Some(primitive) = reserved_bits_primitive(&field.ty) else {
                continue;
            };
            if format_name != Some("binary") {
                diagnostics.push(reserved_bits_format_diagnostic(schema, field));
                continue;
            }
            if let Err(reason) = primitive {
                diagnostics.push(reserved_bits_argument_diagnostic(schema, field, reason));
            }
        }
    }

    diagnostics
}

pub(crate) fn check_schema_mappings(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for schema in &module.schemas {
        let Some(schema_fields) = generated_schema_field_types(schema) else {
            continue;
        };
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
            let mut assigned_targets = BTreeMap::<String, SourceSpan>::new();
            for assignment in &mapping.assignments {
                if !schema_fields.contains_key(&assignment.source) {
                    diagnostics.push(schema_mapping_source_diagnostic(schema, assignment));
                }
                let Some(target_ty) = target_field_types.get(&assignment.target) else {
                    diagnostics.push(schema_mapping_target_field_diagnostic(
                        schema, mapping, assignment,
                    ));
                    continue;
                };
                if target_ty != &Type::int() {
                    diagnostics.push(schema_mapping_type_diagnostic(
                        schema, mapping, assignment, target_ty,
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

fn generated_schema_field_types(schema: &SchemaDecl) -> Option<BTreeMap<String, Type>> {
    if schema.format.as_ref()?.name != "binary" {
        return None;
    }
    schema
        .fields
        .iter()
        .map(|field| {
            exact_width_binary_primitive_name(&field.ty)?;
            Some((field.name.clone(), Type::int()))
        })
        .collect()
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

fn schema_mapping_source_diagnostic(
    schema: &SchemaDecl,
    assignment: &SchemaMappingAssignment,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_source_field",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "schema mapping source field `{}` is not declared",
            assignment.source
        ),
        Some(assignment.span.clone()),
        schema_mapping_assignment_details(
            assignment.node_id.display("schema-mapping-assignment"),
            schema,
            assignment,
            [("reason", JsonValue::string("unknown_source_field"))],
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
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_type",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "schema mapping target field `{}` expects `{}`, but source field `{}` decodes as `Int`",
            assignment.target,
            target_ty.render(),
            assignment.source
        ),
        Some(assignment.span.clone()),
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
                ("actual", JsonValue::string("Int")),
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

fn schema_mapping_details<const N: usize>(
    node_id: String,
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    extra: [(&'static str, JsonValue); N],
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
        "UInt8" => Some("UInt8"),
        "UInt16be" => Some("UInt16be"),
        "UInt24be" => Some("UInt24be"),
        "UInt31be" => Some("UInt31be"),
        "UInt32be" => Some("UInt32be"),
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
    let mut seen = BTreeMap::<String, (String, SourceSpan)>::new();

    for use_decl in &module.uses {
        let node_id = use_decl.node_id.display("use");
        if let Some((first_node_id, first_span)) = seen.get(&use_decl.alias) {
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
            seen.insert(use_decl.alias.clone(), (node_id, use_decl.span.clone()));
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
