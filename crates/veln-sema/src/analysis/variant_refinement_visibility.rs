use std::collections::BTreeSet;

use veln_ast::{Function, Param, SurfaceModule, VariantRefinementType, Visibility};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::adt::descriptors::{AdtDescriptor, AdtVariantDescriptor};
use crate::diagnostics::span_json;
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation;
use crate::types::TypeEnvironment;

pub(crate) fn check_public_variant_refinement_visibility(
    function: &Function,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for param in &function.params {
        collect_root_exposures(
            param.ty.as_deref(),
            &param.ty_refinements,
            PublicBoundary::Parameter(param),
            function,
            module,
            environment,
            &mut diagnostics,
        );
    }
    collect_root_exposures(
        function.return_type.as_deref(),
        &function.return_type_refinements,
        PublicBoundary::Result(function.return_type_span.as_ref()),
        function,
        module,
        environment,
        &mut diagnostics,
    );
    diagnostics
}

pub(crate) fn function_has_private_variant_refinement_exposure(
    function: &Function,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> bool {
    function.visibility == Visibility::Public
        && (function.params.iter().any(|param| {
            root_exposures(
                param.ty.as_deref(),
                &param.ty_refinements,
                module,
                function.module_name.as_deref(),
                environment,
            )
            .next()
            .is_some()
        }) || root_exposures(
            function.return_type.as_deref(),
            &function.return_type_refinements,
            module,
            function.module_name.as_deref(),
            environment,
        )
        .next()
        .is_some())
}

fn collect_root_exposures(
    annotation: Option<&str>,
    refinements: &[VariantRefinementType],
    boundary: PublicBoundary<'_>,
    function: &Function,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if function.visibility != Visibility::Public {
        return;
    }
    for exposure in root_exposures(
        annotation,
        refinements,
        module,
        function.module_name.as_deref(),
        environment,
    ) {
        diagnostics.push(exposure.diagnostic(boundary, function));
    }
}

fn root_exposures<'a>(
    annotation: Option<&str>,
    refinements: &'a [VariantRefinementType],
    module: &'a SurfaceModule,
    current_module: Option<&'a str>,
    environment: &'a TypeEnvironment,
) -> impl Iterator<Item = PrivateExposure<'a>> {
    let is_root_refinement = annotation
        .and_then(|text| parse_type_annotation(text).ok())
        .is_some_and(|ty| match ty {
            Type::VariantRefinement { .. } => true,
            Type::Named { name, .. } => refinements
                .first()
                .and_then(|refinement| refinement.alternatives.first())
                .is_some_and(|alternative| {
                    name == format!(
                        "{}::{}",
                        alternative.base.segments.join("::"),
                        alternative.variant
                    )
                }),
            _ => false,
        });
    let refinements = is_root_refinement
        .then_some(refinements.first())
        .flatten()
        .into_iter();
    refinements.flat_map(move |refinement| {
        let mut seen = BTreeSet::new();
        refinement
            .alternatives
            .iter()
            .filter_map(move |alternative| {
                let written_base = alternative.base.segments.join("::");
                let descriptor = environment.adts.descriptor_for_type_path(
                    &written_base,
                    alternative.type_arguments.len(),
                    current_module,
                    &environment.uses,
                )?;
                if environment.adts.descriptor_is_type_alias(descriptor) {
                    return None;
                }
                let variant = descriptor
                    .variants
                    .iter()
                    .find(|variant| variant.name == alternative.variant)?;
                let private_base = descriptor.visibility == Visibility::Private;
                if !private_base && variant.visibility == Visibility::Public {
                    return None;
                }
                if !seen.insert((descriptor.identity(), variant.name.clone())) {
                    return None;
                }
                Some(PrivateExposure {
                    alternative,
                    descriptor,
                    variant,
                    declaration_span: private_declaration_span(
                        module,
                        descriptor,
                        variant,
                        private_base,
                    ),
                    private_base,
                })
            })
    })
}

fn private_declaration_span(
    module: &SurfaceModule,
    descriptor: &AdtDescriptor,
    variant: &AdtVariantDescriptor,
    private_base: bool,
) -> Option<SourceSpan> {
    let declaration = module.types.iter().find(|declaration| {
        declaration.name.as_deref() == Some(descriptor.type_name.as_str())
            && declaration.module_name == descriptor.module_name
    })?;
    if private_base {
        return Some(declaration.span.clone());
    }
    declaration
        .variants
        .iter()
        .find(|declaration| declaration.name.as_deref() == Some(variant.name.as_str()))
        .map(|declaration| declaration.span.clone())
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
        let (span, message, declaration_kind, declaration_message) = if self.private_base {
            (
                joined_span(&self.alternative.base.segment_spans),
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
        };
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

fn joined_span(spans: &[SourceSpan]) -> SourceSpan {
    let mut span = spans.first().expect("type path has a segment").clone();
    span.end = spans.last().expect("type path has a segment").end;
    span
}
