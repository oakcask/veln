use std::collections::BTreeSet;

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

pub(crate) fn check_function_body(
    function: &Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut checker = FunctionChecker::new(function, environment);
    checker.check_body();
    checker.diagnostics
}

fn json_string_field_is(value: &JsonValue, field: &str, expected: &str) -> bool {
    matches!(
        value,
        JsonValue::Object(entries) if entries.iter().any(|(name, value)| {
            name == field && matches!(value, JsonValue::String(actual) if actual == expected)
        })
    )
}

fn valid_value_binding_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

fn invalid_value_binding_name(name: &str) -> bool {
    !valid_value_binding_name(name)
}

pub(in crate::analysis) struct FunctionChecker<'a> {
    pub(super) function: &'a Function,
    pub(super) environment: &'a TypeEnvironment,
    pub(super) bindings: Vec<Binding>,
    invalid_binding_recoveries: Vec<InvalidBindingRecovery>,
    omitted_local_bindings: Vec<OmittedLocalBinding>,
    pub(super) local_names: BTreeMap<String, (String, SourceSpan)>,
    pub(super) inferred_effects: Vec<EffectUse>,
    pub(super) inferred_return_type: Option<Type>,
    pub(super) diagnostics: Vec<Diagnostic>,
    suppressed_diagnostic_indices: BTreeSet<usize>,
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
            Type::Named { name, args } if name == "Bool" && args.is_empty() => Some(Self::Bool),
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
    pub(super) fn new(function: &'a Function, environment: &'a TypeEnvironment) -> Self {
        Self {
            function,
            environment,
            bindings: Vec::new(),
            invalid_binding_recoveries: Vec::new(),
            omitted_local_bindings: Vec::new(),
            local_names: BTreeMap::new(),
            inferred_effects: Vec::new(),
            inferred_return_type: None,
            diagnostics: Vec::new(),
            suppressed_diagnostic_indices: BTreeSet::new(),
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
mod expression_effects;
mod name_and_declared_calls;
mod patterns_and_exhaustiveness;
mod prelude_and_unresolved_calls;
mod recovery_helpers;

use recovery_helpers::*;
