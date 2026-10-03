use std::collections::{BTreeSet, HashMap};

use veln_ast::{BodyLine, Param};

use super::boundary::{
    duplicate_name_diagnostic, exact_width_binary_primitive_name,
    exact_width_schema_primitive_diagnostic, format_neutral_schema_encode_helper_diagnostic,
    lowercase_schema_primitive_position_diagnostic, type_contains_unknown,
};
use super::repair_reasoning::*;
use super::*;
use crate::effect_rows::{collect_effect_row_substitution, instantiate_effect_rows};
use crate::effects::prelude_effect_origin;
use crate::schema::primitives::lowercase_schema_primitive;
use crate::source_less_lookup::qualified_symbol;
use crate::types::signatures::{
    FunctionSignature, SchemaReferenceErrorKind, UserEffectPathResolution,
};

type LocalNameDeclaration = (String, SourceSpan);
type ScopedLocalName = (String, Option<LocalNameDeclaration>);

pub(crate) fn check_function_body(
    function: &Function,
    environment: &TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
) -> Vec<Diagnostic> {
    let mut checker =
        FunctionChecker::for_source_declaration(function, environment, variant_diagnostics);
    checker.check_body();
    checker.diagnostics
}

fn json_string_field_is(value: &JsonValue, field: &str, expected: &str) -> bool {
    matches!(
        value,
        JsonValue::Object(entries) if entries.iter().any(|(name, value)| {
            name == field && value.as_text() == Some(expected)
        })
    )
}

fn valid_value_binding_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

fn invalid_value_binding_name(name: &str) -> bool {
    !valid_value_binding_name(name)
}

fn type_contains_variant_refinement(ty: &Type) -> bool {
    match ty {
        Type::VariantRefinement { .. } => true,
        Type::Named { args, .. } => args.iter().any(type_contains_variant_refinement),
        Type::Record(fields) => fields
            .iter()
            .any(|(_, ty)| type_contains_variant_refinement(ty)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(type_contains_variant_refinement)
                || variadic
                    .as_deref()
                    .is_some_and(type_contains_variant_refinement)
                || type_contains_variant_refinement(return_type)
        }
        Type::Unknown => false,
    }
}

fn inferred_control_flow_result_type(ty: Type) -> Type {
    match ty {
        Type::VariantRefinement {
            name,
            identity,
            args,
            ..
        } => Type::resolved_named(name, identity, args),
        ty => ty,
    }
}

fn inferred_aggregate_member_type(ty: Type) -> Type {
    match ty {
        Type::VariantRefinement {
            name,
            identity,
            args,
            ..
        } => Type::resolved_named(name, identity, args),
        ty => ty,
    }
}

fn inferred_aggregate_member_type_with_expected(ty: Type, expected: &Type) -> Type {
    if matches!(expected, Type::VariantRefinement { .. }) {
        ty
    } else {
        inferred_aggregate_member_type(ty)
    }
}

fn common_variant_base(left: &Type, right: &Type) -> Option<Type> {
    match (left, right) {
        (
            Type::VariantRefinement {
                name: left_name,
                identity: left_identity,
                args: left_args,
                ..
            },
            Type::VariantRefinement {
                identity: right_identity,
                args: right_args,
                ..
            },
        ) if left_identity == right_identity && left_args == right_args => Some(
            Type::resolved_named(left_name.clone(), left_identity.clone(), left_args.clone()),
        ),
        _ => None,
    }
}

pub(in crate::analysis) struct FunctionChecker<'a> {
    pub(super) function: &'a Function,
    pub(super) environment: &'a TypeEnvironment,
    pub(super) supports_callsite_modifier: bool,
    pub(super) bindings: Vec<Binding>,
    invalid_binding_recoveries: Vec<InvalidBindingRecovery>,
    omitted_local_bindings: Vec<OmittedLocalBinding>,
    pub(super) local_names: BTreeMap<String, LocalNameDeclaration>,
    local_name_scopes: Vec<Vec<ScopedLocalName>>,
    captured_local_bindings: BTreeSet<String>,
    defer_capture_boundaries: Vec<usize>,
    pub(super) inferred_effects: Vec<EffectUse>,
    pub(super) inferred_return_type: Option<Type>,
    inferred_return_unknown_is_diagnosed: bool,
    pub(super) diagnostics: Vec<Diagnostic>,
    suppressed_diagnostic_indices: BTreeSet<usize>,
    defer_blocks: Vec<SourceSpan>,
    variant_diagnostics: &'a mut VariantDiagnosticInterner,
}

pub(in crate::analysis) struct PatternBinding {
    name: String,
    ty: Type,
    node_id: NodeId,
    span: SourceSpan,
}

struct InvalidBindingRecovery {
    name: String,
    ty: Type,
}

struct OmittedLocalBinding {
    name: String,
    node_id: NodeId,
    span: SourceSpan,
    deferred_initializer_diagnostic: Option<usize>,
}

struct EffectBoundary {
    kind: &'static str,
    diagnostic_id: &'static str,
    subject: &'static str,
}

pub(super) struct DeferRestrictionDiagnostic {
    pub(super) id: &'static str,
    pub(super) message: String,
    pub(super) node_id: String,
    pub(super) span: SourceSpan,
    pub(super) reason: &'static str,
    pub(super) repair: &'static str,
    pub(super) repair_span: SourceSpan,
}

impl EffectBoundary {
    fn for_function(function: &Function) -> Option<Self> {
        if function.kind == FunctionKind::Test {
            return Some(Self {
                kind: "test_declaration",
                diagnostic_id: "effect.missing_test",
                subject: "test declaration",
            });
        }
        if function.visibility == Visibility::Public {
            return Some(Self {
                kind: "public_function",
                diagnostic_id: "effect.missing_public",
                subject: "public function",
            });
        }
        None
    }
}

#[derive(Clone, Copy)]
enum MatchDomain {
    Bool,
    Adt,
}

impl MatchDomain {
    pub(super) fn from_type(
        ty: &Type,
        environment: &TypeEnvironment,
        current_module: Option<&str>,
    ) -> Option<Self> {
        match ty {
            Type::Named { name, args, .. } if name == "Bool" && args.is_empty() => Some(Self::Bool),
            _ => environment
                .adts
                .descriptor_for_type_prefer_module(ty, current_module)
                .map(|_| Self::Adt),
        }
    }

    pub(super) fn cases(
        self,
        ty: &Type,
        environment: &TypeEnvironment,
        current_module: Option<&str>,
    ) -> Vec<String> {
        match self {
            Self::Bool => vec!["false".to_string(), "true".to_string()],
            Self::Adt => environment
                .adts
                .descriptor_for_type_prefer_module(ty, current_module)
                .into_iter()
                .flat_map(|descriptor| descriptor.variants.iter())
                .map(|variant| variant.coverage_case.clone())
                .collect(),
        }
    }
}

struct PatternCoverage {
    catches_all: bool,
    cases: Vec<String>,
}

fn match_pattern_coverage(
    pattern: &Pattern,
    domain: &MatchDomain,
    scrutinee_type: &Type,
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> PatternCoverage {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Binding(_) => PatternCoverage {
            catches_all: true,
            cases: Vec::new(),
        },
        PatternKind::BoolLiteral(value) if matches!(domain, MatchDomain::Bool) => PatternCoverage {
            catches_all: false,
            cases: vec![(if *value { "true" } else { "false" }).to_string()],
        },
        PatternKind::Constructor { name, .. } => {
            if invalid_qualified_constructor_pattern(name) {
                return PatternCoverage {
                    catches_all: false,
                    cases: Vec::new(),
                };
            }
            let case = match domain {
                MatchDomain::Adt => environment
                    .adts
                    .descriptor_for_type_prefer_module(scrutinee_type, current_module)
                    .and_then(|descriptor| {
                        environment
                            .adts
                            .constructor_for_descriptor(
                                name,
                                descriptor,
                                current_module,
                                &environment.uses,
                            )
                            .map(|constructor| constructor.variant.coverage_case.clone())
                    }),
                MatchDomain::Bool => None,
            };
            PatternCoverage {
                catches_all: false,
                cases: case.into_iter().collect(),
            }
        }
        _ => PatternCoverage {
            catches_all: false,
            cases: Vec::new(),
        },
    }
}

impl<'a> FunctionChecker<'a> {
    pub(super) fn for_source_declaration(
        function: &'a Function,
        environment: &'a TypeEnvironment,
        variant_diagnostics: &'a mut VariantDiagnosticInterner,
    ) -> Self {
        Self::new(
            function,
            environment,
            function.kind == FunctionKind::Function,
            variant_diagnostics,
        )
    }

    pub(super) fn for_synthetic_declaration(
        function: &'a Function,
        environment: &'a TypeEnvironment,
        variant_diagnostics: &'a mut VariantDiagnosticInterner,
    ) -> Self {
        Self::new(function, environment, false, variant_diagnostics)
    }

    fn new(
        function: &'a Function,
        environment: &'a TypeEnvironment,
        supports_callsite_modifier: bool,
        variant_diagnostics: &'a mut VariantDiagnosticInterner,
    ) -> Self {
        Self {
            function,
            environment,
            supports_callsite_modifier,
            bindings: Vec::new(),
            invalid_binding_recoveries: Vec::new(),
            omitted_local_bindings: Vec::new(),
            local_names: BTreeMap::new(),
            local_name_scopes: Vec::new(),
            captured_local_bindings: BTreeSet::new(),
            defer_capture_boundaries: Vec::new(),
            inferred_effects: Vec::new(),
            inferred_return_type: None,
            inferred_return_unknown_is_diagnosed: false,
            diagnostics: Vec::new(),
            suppressed_diagnostic_indices: BTreeSet::new(),
            defer_blocks: Vec::new(),
            variant_diagnostics,
        }
    }

    pub(super) fn check_body(&mut self) {
        self.check_function_annotations();
        self.check_contracts();
        let function = self.function;
        for (index, line) in function.body.iter().enumerate() {
            self.check_body_line(index, line);
        }
        self.check_implicit_unit_return();
        self.check_omitted_local_inference_complete();
        self.check_private_inference_complete();
        self.check_effect_boundaries();
        self.remove_suppressed_diagnostics();
    }
}

mod adt_and_match;
mod annotations_and_effects;
mod body_lines;
mod collections_and_operators;
mod contract_validation;
mod diagnostics_and_repairs;
pub(crate) use diagnostics_and_repairs::VariantDiagnosticInterner;
#[cfg(test)]
pub(crate) use diagnostics_and_repairs::{
    reset_retained_variant_diagnostic_key_variants, take_retained_variant_diagnostic_key_variants,
};
mod expression_effects;
mod name_and_declared_calls;
mod patterns_and_exhaustiveness;
mod prelude_and_unresolved_calls;
mod recovery_helpers;

use recovery_helpers::*;
