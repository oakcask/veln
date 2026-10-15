use std::{collections::HashSet, sync::Arc};

use veln_ast::{Function, Param, TypePathSegments, VariantRefinementType, Visibility};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::adt::descriptors::{AdtDescriptor, AdtVariantDescriptor};
use crate::diagnostics::span_json;
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation;
use crate::types::TypeEnvironment;

pub(crate) fn check_public_variant_refinement_visibility(
    function: &Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for param in &function.params {
        collect_root_exposures(
            RootAnnotation::new(
                param.ty_span.as_ref(),
                param.ty.as_deref(),
                &param.ty_refinements,
                &param.ty_paths,
            ),
            PublicBoundary::Parameter(param),
            function,
            environment,
            &mut diagnostics,
        );
    }
    collect_root_exposures(
        RootAnnotation::new(
            function.return_type_span.as_ref(),
            function.return_type.as_deref(),
            &function.return_type_refinements,
            &function.return_type_paths,
        ),
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
                RootAnnotation::new(
                    param.ty_span.as_ref(),
                    param.ty.as_deref(),
                    &param.ty_refinements,
                    &param.ty_paths,
                ),
                function.module_name.as_deref(),
                environment,
            )
            .next()
            .is_some()
        }) || root_exposures(
            RootAnnotation::new(
                function.return_type_span.as_ref(),
                function.return_type.as_deref(),
                &function.return_type_refinements,
                &function.return_type_paths,
            ),
            function.module_name.as_deref(),
            environment,
        )
        .next()
        .is_some())
}

fn collect_root_exposures(
    annotation: RootAnnotation<'_>,
    boundary: PublicBoundary<'_>,
    function: &Function,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if function.visibility != Visibility::Public {
        return;
    }
    for exposure in root_exposures(annotation, function.module_name.as_deref(), environment) {
        diagnostics.push(exposure.diagnostic(boundary, function));
    }
}

fn root_exposures<'a>(
    annotation: RootAnnotation<'a>,
    current_module: Option<&'a str>,
    environment: &'a TypeEnvironment,
) -> std::vec::IntoIter<PrivateExposure<'a>> {
    let Some(annotation_span) = annotation.span else {
        return Vec::new().into_iter();
    };
    let mut exposures = root_exposure_candidates(
        annotation_span,
        annotation.text,
        annotation.refinements,
        annotation.paths,
    )
    .into_iter()
    .filter_map(|candidate| private_exposure(candidate, current_module, environment))
    .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    exposures.retain(|exposure| {
        record_visibility_work(1);
        seen.insert((
            exposure.descriptor.identity(),
            exposure.variant.name.clone(),
            exposure.alternative_identity.clone(),
        ))
    });
    exposures.into_iter()
}

#[derive(Clone, Copy)]
struct RootAnnotation<'a> {
    span: Option<&'a SourceSpan>,
    text: Option<&'a str>,
    refinements: &'a [VariantRefinementType],
    paths: &'a [TypePathSegments],
}

impl<'a> RootAnnotation<'a> {
    fn new(
        span: Option<&'a SourceSpan>,
        text: Option<&'a str>,
        refinements: &'a [VariantRefinementType],
        paths: &'a [TypePathSegments],
    ) -> Self {
        Self {
            span,
            text,
            refinements,
            paths,
        }
    }
}

fn root_exposure_candidates(
    annotation_span: &SourceSpan,
    annotation_text: Option<&str>,
    refinements: &[VariantRefinementType],
    paths: &[TypePathSegments],
) -> Vec<RefinementExposureCandidate> {
    if let Some(refinement) = refinements
        .iter()
        .find(|refinement| refinement.span == *annotation_span)
    {
        record_visibility_candidate_construction(refinement.alternatives.len());
        return refinement
            .alternatives
            .iter()
            .map(refinement_candidate)
            .collect();
    }
    let Some(annotation_text) = annotation_text else {
        return Vec::new();
    };
    record_visibility_root_classification(annotation_text.len());
    if matches!(
        parse_type_annotation(annotation_text),
        Ok(Type::Record(_) | Type::Function { .. })
    ) {
        return Vec::new();
    }
    recovered_root_exposure_candidates(annotation_span, annotation_text, refinements, paths)
}

fn recovered_root_exposure_candidates(
    annotation_span: &SourceSpan,
    annotation_text: &str,
    refinements: &[VariantRefinementType],
    paths: &[TypePathSegments],
) -> Vec<RefinementExposureCandidate> {
    record_visibility_candidate_construction(
        refinements
            .iter()
            .map(|refinement| refinement.alternatives.len())
            .sum(),
    );
    let mut candidates = refinements
        .iter()
        .flat_map(|refinement| refinement.alternatives.iter())
        .map(refinement_candidate)
        .collect::<Vec<_>>();
    merge_recovered_type_paths(annotation_span, paths, &mut candidates);
    candidates.sort_by_key(|candidate| candidate.occurrence_span.start.offset);
    if direct_root_sequence(annotation_text, &candidates) {
        candidates
    } else {
        Vec::new()
    }
}

fn merge_recovered_type_paths(
    annotation_span: &SourceSpan,
    paths: &[TypePathSegments],
    candidates: &mut Vec<RefinementExposureCandidate>,
) {
    let mut candidate_occurrences = candidates
        .iter()
        .map(|candidate| occurrence_key(&candidate.occurrence_span))
        .collect::<HashSet<_>>();
    for path in paths {
        if path.segments.len() < 2 || !span_within(&path.application_span, annotation_span) {
            continue;
        }
        record_visibility_occurrence_lookup(1);
        if !candidate_occurrences.insert(occurrence_key(&path.application_span)) {
            continue;
        }
        let variant_index = path.segments.len() - 1;
        record_visibility_candidate_construction(1);
        candidates.push(RefinementExposureCandidate {
            written_base: path.segments[..variant_index].join("::"),
            type_argument_count: path.type_argument_count,
            written_variant: path.segments[variant_index].clone(),
            written_type: path.written.clone(),
            base_span: path.segment_spans[variant_index - 1].clone(),
            variant_span: path.segment_spans[variant_index].clone(),
            occurrence_span: path.application_span.clone(),
        });
    }
}

fn refinement_candidate(
    alternative: &veln_ast::VariantRefinementAlternative,
) -> RefinementExposureCandidate {
    RefinementExposureCandidate {
        written_base: alternative.base.segments.join("::"),
        type_argument_count: alternative.type_arguments.len(),
        written_variant: alternative.variant.clone(),
        written_type: format!("{}::{}", alternative.base.written, alternative.variant),
        base_span: alternative
            .base
            .segment_spans
            .last()
            .expect("type path has a segment")
            .clone(),
        variant_span: alternative.variant_span.clone(),
        occurrence_span: alternative.span.clone(),
    }
}

fn span_within(candidate: &SourceSpan, enclosing: &SourceSpan) -> bool {
    candidate.file == enclosing.file
        && candidate.start.offset >= enclosing.start.offset
        && candidate.end.offset <= enclosing.end.offset
}

fn occurrence_key(span: &SourceSpan) -> (veln_source::SourcePath, usize, usize) {
    (span.file.clone(), span.start.offset, span.end.offset)
}

fn direct_root_sequence(annotation_text: &str, candidates: &[RefinementExposureCandidate]) -> bool {
    if candidates.is_empty() {
        return false;
    }
    let expected = candidates
        .iter()
        .map(|candidate| compact_type_text(&candidate.written_type))
        .collect::<Vec<_>>()
        .join("|");
    compact_type_text(annotation_text) == expected
}

fn compact_type_text(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

struct RefinementExposureCandidate {
    written_base: String,
    type_argument_count: usize,
    written_variant: String,
    written_type: String,
    base_span: SourceSpan,
    variant_span: SourceSpan,
    occurrence_span: SourceSpan,
}

fn private_exposure<'a>(
    candidate: RefinementExposureCandidate,
    current_module: Option<&'a str>,
    environment: &'a TypeEnvironment,
) -> Option<PrivateExposure<'a>> {
    record_visibility_work(1);
    let descriptor = environment.variant_refinement_descriptor_identity(
        &candidate.written_base,
        candidate.type_argument_count,
        current_module,
    )?;
    if environment.adts.descriptor_is_type_alias(descriptor) {
        return None;
    }
    record_visibility_work(1);
    let variant =
        environment.variant_refinement_variant_identity(descriptor, &candidate.written_variant)?;
    let private_base = descriptor.visibility == Visibility::Private;
    if !private_base && variant.visibility == Visibility::Public {
        return None;
    }
    record_visibility_work(1);
    let alternative_identity = refinement_alternative_identity(
        &candidate.written_type,
        &variant.name,
        current_module,
        environment,
    );
    Some(PrivateExposure {
        written_type: candidate.written_type,
        base_span: candidate.base_span,
        variant_span: candidate.variant_span,
        descriptor,
        variant,
        declaration_span: private_declaration_span(environment, descriptor, variant, private_base),
        private_base,
        alternative_identity,
    })
}

fn refinement_alternative_identity(
    written_type: &str,
    resolved_variant: &str,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> RefinementAlternativeIdentity {
    if let Ok(Type::VariantRefinement { args, .. }) = parse_type_annotation(written_type) {
        let args = args
            .into_iter()
            .map(|argument| {
                canonical_type_identity(
                    environment.canonicalize_type_annotation(argument, current_module),
                )
            })
            .collect::<Option<Vec<_>>>();
        if let Some(args) = args {
            return RefinementAlternativeIdentity::Resolved {
                args,
                variants: vec![resolved_variant.to_string()],
            };
        }
    }
    RefinementAlternativeIdentity::Written(written_type.to_string())
}

fn canonical_type_identity(ty: Type) -> Option<Type> {
    match ty {
        Type::Named { identity, args, .. } => canonical_named_identity(identity, args),
        Type::VariantRefinement {
            identity,
            args,
            variants,
            unresolved_alternatives,
            ..
        } if unresolved_alternatives.is_empty() => {
            canonical_refinement_identity(identity, args, variants)
        }
        Type::VariantRefinement { .. } => None,
        Type::Record(fields) => canonical_record_identity(fields),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => canonical_function_identity(params, variadic, *return_type, effects),
        Type::Unknown => None,
    }
}

fn canonical_named_identity(identity: String, args: Vec<Type>) -> Option<Type> {
    Some(Type::resolved_named(
        identity.clone(),
        identity,
        canonical_type_identities(args)?,
    ))
}

fn canonical_refinement_identity(
    identity: String,
    args: Vec<Type>,
    variants: Arc<[String]>,
) -> Option<Type> {
    Some(Type::VariantRefinement {
        name: identity.clone(),
        identity,
        args: canonical_type_identities(args)?,
        variants,
        unresolved_alternatives: Vec::new(),
    })
}

fn canonical_record_identity(fields: Vec<(String, Type)>) -> Option<Type> {
    Some(Type::Record(
        fields
            .into_iter()
            .map(|(name, ty)| Some((name, canonical_type_identity(ty)?)))
            .collect::<Option<Vec<_>>>()?,
    ))
}

fn canonical_function_identity(
    params: Vec<Type>,
    variadic: Option<Box<Type>>,
    return_type: Type,
    effects: Vec<String>,
) -> Option<Type> {
    Some(Type::Function {
        params: canonical_type_identities(params)?,
        variadic: canonical_optional_type(variadic)?,
        return_type: Box::new(canonical_type_identity(return_type)?),
        effects,
    })
}

fn canonical_type_identities(types: Vec<Type>) -> Option<Vec<Type>> {
    types.into_iter().map(canonical_type_identity).collect()
}

fn canonical_optional_type(ty: Option<Box<Type>>) -> Option<Option<Box<Type>>> {
    match ty {
        Some(ty) => Some(Some(Box::new(canonical_type_identity(*ty)?))),
        None => Some(None),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum RefinementAlternativeIdentity {
    Resolved {
        args: Vec<Type>,
        variants: Vec<String>,
    },
    Written(String),
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
    static VISIBILITY_CANDIDATE_WORK: std::cell::Cell<VisibilityCandidateWork> =
        const { std::cell::Cell::new(VisibilityCandidateWork::new()) };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VisibilityCandidateWork {
    pub(crate) root_classification: usize,
    pub(crate) candidate_construction: usize,
    pub(crate) occurrence_lookup: usize,
}

#[cfg(test)]
impl VisibilityCandidateWork {
    const fn new() -> Self {
        Self {
            root_classification: 0,
            candidate_construction: 0,
            occurrence_lookup: 0,
        }
    }
}

#[cfg(test)]
fn record_visibility_work(amount: usize) {
    VISIBILITY_WORK.with(|work| work.set(work.get() + amount));
}

#[cfg(test)]
fn record_visibility_root_classification(amount: usize) {
    VISIBILITY_CANDIDATE_WORK.with(|work| {
        let mut next = work.get();
        next.root_classification += amount;
        work.set(next);
    });
}

#[cfg(not(test))]
fn record_visibility_root_classification(_amount: usize) {}

#[cfg(test)]
fn record_visibility_candidate_construction(amount: usize) {
    VISIBILITY_CANDIDATE_WORK.with(|work| {
        let mut next = work.get();
        next.candidate_construction += amount;
        work.set(next);
    });
}

#[cfg(not(test))]
fn record_visibility_candidate_construction(_amount: usize) {}

#[cfg(test)]
fn record_visibility_occurrence_lookup(amount: usize) {
    VISIBILITY_CANDIDATE_WORK.with(|work| {
        let mut next = work.get();
        next.occurrence_lookup += amount;
        work.set(next);
    });
}

#[cfg(not(test))]
fn record_visibility_occurrence_lookup(_amount: usize) {}

#[cfg(not(test))]
fn record_visibility_work(_amount: usize) {}

#[cfg(test)]
pub(crate) fn reset_visibility_work() {
    VISIBILITY_WORK.with(|work| work.set(0));
    VISIBILITY_CANDIDATE_WORK.with(|work| work.set(VisibilityCandidateWork::new()));
}

#[cfg(test)]
pub(crate) fn take_visibility_work() -> usize {
    VISIBILITY_WORK.with(|work| work.replace(0))
}

#[cfg(test)]
pub(crate) fn take_visibility_candidate_work() -> VisibilityCandidateWork {
    VISIBILITY_CANDIDATE_WORK.with(|work| work.replace(VisibilityCandidateWork::new()))
}

struct PrivateExposure<'a> {
    written_type: String,
    base_span: SourceSpan,
    variant_span: SourceSpan,
    descriptor: &'a AdtDescriptor,
    variant: &'a AdtVariantDescriptor,
    declaration_span: Option<SourceSpan>,
    private_base: bool,
    alternative_identity: RefinementAlternativeIdentity,
}

impl PrivateExposure<'_> {
    fn diagnostic(&self, boundary: PublicBoundary<'_>, function: &Function) -> Diagnostic {
        let base_type = self.descriptor.identity();
        let (span, message, declaration_kind, declaration_message) = self.primary_context();
        let mut diagnostic = Diagnostic::new(
            "type.variant_refinement_private",
            Severity::Error,
            DiagnosticKind::Type,
            message,
            Some(span),
            JsonValue::object([
                ("written_type", JsonValue::string(self.written_type.clone())),
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
                self.base_span.clone(),
                format!("public refinement exposes private type `{base_type}`"),
                "private_type_declaration",
                format!("Private type `{base_type}` is declared here."),
            )
        } else {
            (
                self.variant_span.clone(),
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
