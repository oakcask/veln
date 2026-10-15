use std::collections::HashSet;

use veln_ast::{Function, Param, VariantRefinementType, Visibility};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::adt::descriptors::{AdtDescriptor, AdtVariantDescriptor};
use crate::diagnostics::span_json;
use crate::types::TypeEnvironment;

pub(crate) fn check_public_variant_refinement_visibility(
    function: &Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for param in &function.params {
        collect_root_exposures(
            param.ty_span.as_ref(),
            &param.ty_refinements,
            PublicBoundary::Parameter(param),
            function,
            environment,
            &mut diagnostics,
        );
    }
    collect_root_exposures(
        function.return_type_span.as_ref(),
        &function.return_type_refinements,
        PublicBoundary::Result(function.return_type_span.as_ref()),
        function,
        environment,
        &mut diagnostics,
    );
    diagnostics
}

pub(crate) fn function_has_private_variant_refinement_exposure(
    function: &Function,
    environment: &TypeEnvironment,
) -> bool {
    function.visibility == Visibility::Public
        && (function.params.iter().any(|param| {
            root_exposures(
                param.ty_span.as_ref(),
                &param.ty_refinements,
                function.module_name.as_deref(),
                environment,
            )
            .next()
            .is_some()
        }) || root_exposures(
            function.return_type_span.as_ref(),
            &function.return_type_refinements,
            function.module_name.as_deref(),
            environment,
        )
        .next()
        .is_some())
}

fn collect_root_exposures(
    annotation_span: Option<&SourceSpan>,
    refinements: &[VariantRefinementType],
    boundary: PublicBoundary<'_>,
    function: &Function,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if function.visibility != Visibility::Public {
        return;
    }
    for exposure in root_exposures(
        annotation_span,
        refinements,
        function.module_name.as_deref(),
        environment,
    ) {
        diagnostics.push(exposure.diagnostic(boundary, function));
    }
}

fn root_exposures<'a>(
    annotation_span: Option<&SourceSpan>,
    refinements: &'a [VariantRefinementType],
    current_module: Option<&'a str>,
    environment: &'a TypeEnvironment,
) -> impl Iterator<Item = PrivateExposure<'a>> {
    let refinements = annotation_span
        .and_then(|annotation_span| {
            refinements
                .iter()
                .find(|refinement| refinement.span == *annotation_span)
        })
        .into_iter();
    refinements.flat_map(move |refinement| {
        let mut seen = HashSet::new();
        refinement
            .alternatives
            .iter()
            .filter_map(move |alternative| {
                private_exposure(alternative, current_module, environment)
            })
            .filter(move |exposure| {
                record_visibility_work(1);
                seen.insert((
                    exposure.descriptor.identity(),
                    exposure.variant.name.clone(),
                ))
            })
    })
}

fn private_exposure<'a>(
    alternative: &'a veln_ast::VariantRefinementAlternative,
    current_module: Option<&'a str>,
    environment: &'a TypeEnvironment,
) -> Option<PrivateExposure<'a>> {
    let written_base = alternative.base.segments.join("::");
    record_visibility_work(1);
    let descriptor = environment
        .adts
        .descriptor_for_type_path(
            &written_base,
            alternative.type_arguments.len(),
            current_module,
            &environment.uses,
        )
        .or_else(|| {
            environment.adts.unique_descriptor_for_type_path_any_arity(
                &written_base,
                current_module,
                &environment.uses,
            )
        })?;
    if environment.adts.descriptor_is_type_alias(descriptor) {
        return None;
    }
    record_visibility_work(1);
    let variant = environment
        .adts
        .variant_for_descriptor(descriptor, &alternative.variant)?;
    let private_base = descriptor.visibility == Visibility::Private;
    if !private_base && variant.visibility == Visibility::Public {
        return None;
    }
    record_visibility_work(1);
    Some(PrivateExposure {
        alternative,
        descriptor,
        variant,
        declaration_span: private_declaration_span(environment, descriptor, variant, private_base),
        private_base,
    })
}

fn private_declaration_span(
    environment: &TypeEnvironment,
    descriptor: &AdtDescriptor,
    variant: &AdtVariantDescriptor,
    private_base: bool,
) -> Option<SourceSpan> {
    if private_base {
        return environment.adts.declaration_span(descriptor).cloned();
    }
    environment
        .adts
        .variant_declaration_span(descriptor, variant)
        .cloned()
}

#[cfg(test)]
thread_local! {
    static VISIBILITY_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_visibility_work(amount: usize) {
    VISIBILITY_WORK.with(|work| work.set(work.get() + amount));
}

#[cfg(not(test))]
fn record_visibility_work(_amount: usize) {}

#[cfg(test)]
pub(crate) fn reset_visibility_work() {
    VISIBILITY_WORK.with(|work| work.set(0));
}

#[cfg(test)]
pub(crate) fn take_visibility_work() -> usize {
    VISIBILITY_WORK.with(|work| work.replace(0))
}

struct PrivateExposure<'a> {
    alternative: &'a veln_ast::VariantRefinementAlternative,
    descriptor: &'a AdtDescriptor,
    variant: &'a AdtVariantDescriptor,
    declaration_span: Option<SourceSpan>,
    private_base: bool,
}

impl PrivateExposure<'_> {
    fn diagnostic(&self, boundary: PublicBoundary<'_>, function: &Function) -> Diagnostic {
        let base_type = self.descriptor.identity();
        let written_type = format!(
            "{}::{}",
            self.alternative.base.written, self.alternative.variant
        );
        let (span, message, declaration_kind, declaration_message) = self.primary_context();
        let mut diagnostic = Diagnostic::new(
            "type.variant_refinement_private",
            Severity::Error,
            DiagnosticKind::Type,
            message,
            Some(span),
            JsonValue::object([
                ("written_type", JsonValue::string(written_type)),
                ("base_type", JsonValue::string(base_type)),
                ("variant", JsonValue::string(self.variant.name.clone())),
                ("boundary", JsonValue::string(boundary.name())),
                ("exposure_path", boundary.exposure_path()),
            ]),
        );
        if let Some(span) = &self.declaration_span {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string(declaration_kind)),
                ("message", JsonValue::string(declaration_message)),
                ("span", span_json(span)),
            ]));
        }
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string(boundary.related_kind())),
            (
                "message",
                JsonValue::string(boundary.related_message(function)),
            ),
            ("span", span_json(boundary.span(function))),
        ]));
        diagnostic
    }

    fn primary_context(&self) -> (SourceSpan, String, &'static str, String) {
        let base_type = self.descriptor.identity();
        if self.private_base {
            (
                self.alternative
                    .base
                    .segment_spans
                    .last()
                    .expect("type path has a segment")
                    .clone(),
                format!("public refinement exposes private type `{base_type}`"),
                "private_type_declaration",
                format!("Private type `{base_type}` is declared here."),
            )
        } else {
            (
                self.alternative.variant_span.clone(),
                format!(
                    "public refinement exposes private variant `{}::{}`",
                    base_type, self.variant.name
                ),
                "private_variant_declaration",
                format!(
                    "Private variant `{}::{}` is declared here.",
                    base_type, self.variant.name
                ),
            )
        }
    }
}

#[derive(Clone, Copy)]
enum PublicBoundary<'a> {
    Parameter(&'a Param),
    Result(Option<&'a SourceSpan>),
}

impl PublicBoundary<'_> {
    fn name(self) -> &'static str {
        match self {
            Self::Parameter(_) => "public_parameter",
            Self::Result(_) => "public_result",
        }
    }

    fn exposure_path(self) -> JsonValue {
        match self {
            Self::Parameter(param) => JsonValue::array([JsonValue::object([
                ("kind", JsonValue::string("parameter")),
                ("name", JsonValue::string(param.name.clone())),
            ])]),
            Self::Result(_) => {
                JsonValue::array([JsonValue::object([("kind", JsonValue::string("result"))])])
            }
        }
    }

    fn related_kind(self) -> &'static str {
        match self {
            Self::Parameter(_) => "public_parameter_boundary",
            Self::Result(_) => "public_result_boundary",
        }
    }

    fn related_message(self, function: &Function) -> String {
        let function_name = function.name.as_deref().unwrap_or("<invalid>");
        match self {
            Self::Parameter(param) => format!(
                "Public parameter `{}` of function `{function_name}` exposes this refinement.",
                param.name
            ),
            Self::Result(_) => {
                format!("Public result of function `{function_name}` exposes this refinement.")
            }
        }
    }

    fn span<'a>(self, function: &'a Function) -> &'a SourceSpan
    where
        Self: 'a,
    {
        match self {
            Self::Parameter(param) => &param.span,
            Self::Result(Some(span)) => span,
            Self::Result(None) => &function.span,
        }
    }
}
