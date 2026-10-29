use std::collections::BTreeSet;

use veln_ast::{ModuleValueDecl, TypePathSegments, Visibility};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};

use crate::adt::descriptors::AdtDescriptor;
use crate::diagnostics::span_json;
use crate::types::TypeEnvironment;

pub(super) fn public_named_type_diagnostics(
    value: &ModuleValueDecl,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    if value.visibility != Visibility::Public {
        return Vec::new();
    }
    let mut seen = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for path in &value.annotation_paths {
        let written = path.segments.join("::");
        let Some(descriptors) = resolved_descriptors(value, path, environment, &written) else {
            continue;
        };
        for descriptor in descriptors {
            let identity = descriptor.declaration_identity();
            if descriptor.visibility == Visibility::Private && seen.insert(identity.clone()) {
                diagnostics.push(exposure_diagnostic(
                    value,
                    path,
                    environment,
                    descriptor,
                    written.clone(),
                    identity,
                ));
            }
        }
    }
    diagnostics
}

fn resolved_descriptors<'a>(
    value: &ModuleValueDecl,
    path: &TypePathSegments,
    environment: &'a TypeEnvironment,
    written: &str,
) -> Option<[&'a AdtDescriptor; 2]> {
    let descriptor = environment.annotation_type_descriptor_in_owner(
        written,
        path.type_argument_count,
        value.module_name.as_deref(),
        value.package_name.as_deref(),
    )?;
    Some([
        descriptor,
        environment
            .adts
            .terminal_descriptor_for_alias(descriptor)
            .unwrap_or_else(|| environment.adts.canonical_descriptor_for(descriptor)),
    ])
}

fn exposure_diagnostic(
    value: &ModuleValueDecl,
    path: &TypePathSegments,
    environment: &TypeEnvironment,
    descriptor: &AdtDescriptor,
    written: String,
    identity: String,
) -> Diagnostic {
    let name = value.name.as_deref().unwrap_or("<invalid>");
    let mut diagnostic = Diagnostic::new(
        "type.public_annotation_private",
        Severity::Error,
        DiagnosticKind::Type,
        format!("public module value annotation exposes private type `{identity}`"),
        Some(path.application_span.clone()),
        JsonValue::object([
            ("boundary", JsonValue::string("public_module_value")),
            ("name", JsonValue::string(name)),
            ("written_type", JsonValue::string(written)),
            ("resolved_type", JsonValue::string(identity.clone())),
        ]),
    );
    append_private_declaration(&mut diagnostic, environment, descriptor, &identity);
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("public_module_value_boundary")),
        (
            "message",
            JsonValue::string(format!("Public module value `{name}` exposes this type.")),
        ),
        ("span", span_json(&value.annotation_span)),
    ]));
    diagnostic
}

fn append_private_declaration(
    diagnostic: &mut Diagnostic,
    environment: &TypeEnvironment,
    descriptor: &AdtDescriptor,
    identity: &str,
) {
    let alias_path = environment.adts.type_alias_exposure_path(descriptor);
    let alias_span = alias_path.and_then(|path| {
        path.declarations()
            .next()
            .map(|declaration| declaration.span.clone())
    });
    let Some(span) = alias_span
        .as_ref()
        .or_else(|| environment.adts.declaration_span(descriptor))
    else {
        return;
    };
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("private_type_declaration")),
        (
            "message",
            JsonValue::string(format!("Private type `{identity}` is declared here.")),
        ),
        ("span", span_json(span)),
    ]));
}
