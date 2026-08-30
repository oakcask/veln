use super::*;

use super::schema_dispatch_resolution::schema_dispatch_payload_diagnostic;
use super::schema_repeat_resolution::{
    companion_private_schema_access_allowed, schema_repeat_payload_diagnostic,
};

#[derive(Clone, Copy)]
pub(super) enum SchemaPayloadKind {
    Dispatch { tag: i64 },
    Repeat,
}

impl SchemaPayloadKind {
    fn name(self) -> &'static str {
        match self {
            Self::Dispatch { .. } => "dispatch",
            Self::Repeat => "repeat",
        }
    }

    fn diagnostic<const N: usize>(
        self,
        schema: &SchemaDecl,
        field: &SchemaField,
        payload_name: &str,
        reason: &'static str,
        message: String,
        extra: [(&'static str, JsonValue); N],
    ) -> Diagnostic {
        match self {
            Self::Dispatch { tag } => schema_dispatch_payload_diagnostic(
                schema,
                field,
                tag,
                payload_name,
                reason,
                message,
                extra,
            ),
            Self::Repeat => schema_repeat_payload_diagnostic(
                schema,
                field,
                payload_name,
                reason,
                message,
                extra,
            ),
        }
    }
}

pub(super) fn resolve_schema_payload<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    kind: SchemaPayloadKind,
    payload_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let Some(segments) = schema_payload_name_path(payload_name) else {
        diagnostics.push(kind.diagnostic(
            schema,
            field,
            payload_name,
            "invalid_payload_name",
            format!(
                "{} payload schema `{payload_name}` is not a valid schema path",
                kind.name()
            ),
            [],
        ));
        return None;
    };
    match segments.as_slice() {
        [name] => resolve_local_schema_payload(module, schema, field, kind, name, diagnostics),
        [_, .., name] => {
            let Some(use_decl) = normal_imported_use_for_path(
                module,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            ) else {
                diagnostics.push(kind.diagnostic(
                    schema,
                    field,
                    payload_name,
                    "unknown_import",
                    format!(
                        "{} payload schema `{payload_name}` is not declared",
                        kind.name()
                    ),
                    [],
                ));
                return None;
            };
            resolve_imported_schema_payload(
                module,
                schema,
                field,
                kind,
                use_decl,
                payload_name,
                name,
                diagnostics,
            )
        }
        _ => None,
    }
}

fn resolve_local_schema_payload<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    kind: SchemaPayloadKind,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| candidate.node_id == schema.node_id)?;
    if let Some((candidate_index, candidate)) =
        module.schemas.iter().enumerate().find(|(_, candidate)| {
            candidate.name.as_deref() == Some(name)
                && candidate.module_name.as_deref() == schema.module_name.as_deref()
        })
    {
        return validate_local_schema_payload_candidate(
            schema,
            field,
            kind,
            name,
            current_index,
            candidate_index,
            candidate,
            diagnostics,
        );
    }
    report_missing_schema_payload(
        module,
        schema,
        field,
        kind,
        schema.module_name.as_deref(),
        name,
        name,
        diagnostics,
    );
    None
}

#[allow(clippy::too_many_arguments)]
fn validate_local_schema_payload_candidate<'a>(
    schema: &SchemaDecl,
    field: &SchemaField,
    kind: SchemaPayloadKind,
    name: &str,
    current_index: usize,
    candidate_index: usize,
    candidate: &'a SchemaDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let failure = if candidate_index == current_index {
        Some((
            "self_payload_schema",
            format!(
                "{} payload schema `{name}` cannot reference itself",
                kind.name()
            ),
        ))
    } else if candidate_index > current_index {
        Some((
            "forward_payload_schema",
            format!(
                "{} payload schema `{name}` must be declared before schema `{}`",
                kind.name(),
                schema.name.as_deref().unwrap_or("<missing>")
            ),
        ))
    } else if !is_binary_schema(candidate) {
        Some((
            "non_binary_payload_schema",
            format!(
                "{} payload schema `{name}` must use `format binary`",
                kind.name()
            ),
        ))
    } else {
        None
    };
    if let Some((reason, message)) = failure {
        diagnostics.push(kind.diagnostic(schema, field, name, reason, message, []));
        return None;
    }
    Some(candidate)
}

#[allow(clippy::too_many_arguments)]
fn resolve_imported_schema_payload<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    kind: SchemaPayloadKind,
    use_decl: &UseDecl,
    payload_name: &str,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let target_module = Some(use_decl.name.as_str());
    if let Some(candidate) = module.schemas.iter().find(|candidate| {
        candidate.name.as_deref() == Some(name) && candidate.module_name.as_deref() == target_module
    }) {
        let failure = if candidate.visibility != Visibility::Public
            && !companion_private_schema_access_allowed(module, schema, use_decl)
        {
            Some((
                "private_imported_payload_schema",
                format!(
                    "imported {} payload schema `{payload_name}` is private",
                    kind.name()
                ),
            ))
        } else if !is_binary_schema(candidate) {
            Some((
                "non_binary_payload_schema",
                format!(
                    "{} payload schema `{payload_name}` must use `format binary`",
                    kind.name()
                ),
            ))
        } else {
            None
        };
        if let Some((reason, message)) = failure {
            diagnostics.push(kind.diagnostic(schema, field, payload_name, reason, message, []));
            return None;
        }
        return Some(candidate);
    }
    report_missing_schema_payload(
        module,
        schema,
        field,
        kind,
        target_module,
        payload_name,
        name,
        diagnostics,
    );
    None
}

#[allow(clippy::too_many_arguments)]
fn report_missing_schema_payload(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    kind: SchemaPayloadKind,
    target_module: Option<&str>,
    payload_name: &str,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(resolved_kind) = codec_schema_wrong_kind(module, target_module, name) {
        diagnostics.push(kind.diagnostic(
            schema,
            field,
            payload_name,
            "non_schema_payload",
            format!(
                "{} payload `{payload_name}` resolves to a {resolved_kind}, not a schema",
                kind.name()
            ),
            [("resolved_kind", JsonValue::string(resolved_kind))],
        ));
    } else {
        diagnostics.push(kind.diagnostic(
            schema,
            field,
            payload_name,
            "unknown_payload_schema",
            format!(
                "{} payload schema `{payload_name}` is not declared",
                kind.name()
            ),
            [],
        ));
    }
}

fn is_binary_schema(schema: &SchemaDecl) -> bool {
    schema.format.as_ref().map(|format| format.name.as_str()) == Some("binary")
}
