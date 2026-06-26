use super::boundary::{
    duplicate_name_diagnostic, exact_width_binary_primitive_name,
    exact_width_schema_primitive_diagnostic, type_contains_unknown,
};
use super::repair_reasoning::*;
use super::*;
use crate::standard_symbols::qualified_symbol;

pub(crate) fn check_function_body(
    function: &Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut checker = FunctionChecker::new(function, environment);
    checker.check_body();
    checker.diagnostics
}

pub(in crate::analysis) struct FunctionChecker<'a> {
    pub(super) function: &'a Function,
    pub(super) environment: &'a TypeEnvironment,
    pub(super) bindings: Vec<Binding>,
    omitted_local_bindings: Vec<OmittedLocalBinding>,
    pub(super) local_names: BTreeMap<String, (String, SourceSpan)>,
    pub(super) inferred_effects: Vec<EffectUse>,
    pub(super) inferred_return_type: Option<Type>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

pub(in crate::analysis) struct PatternBinding {
    name: String,
    ty: Type,
    node_id: NodeId,
    span: SourceSpan,
}

struct OmittedLocalBinding {
    name: String,
    node_id: NodeId,
    span: SourceSpan,
}

#[derive(Clone, Copy)]
enum MatchDomain {
    Bool,
    Adt,
}

impl MatchDomain {
    pub(super) fn from_type(ty: &Type, environment: &TypeEnvironment) -> Option<Self> {
        match ty {
            Type::Named { name, args } if name == "Bool" && args.is_empty() => Some(Self::Bool),
            _ => environment.adts.descriptor_for_type(ty).map(|_| Self::Adt),
        }
    }

    pub(super) fn cases(self, ty: &Type, environment: &TypeEnvironment) -> Vec<String> {
        match self {
            Self::Bool => vec!["false".to_string(), "true".to_string()],
            Self::Adt => environment
                .adts
                .descriptor_for_type(ty)
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
            let case = match domain {
                MatchDomain::Adt => environment
                    .adts
                    .descriptor_for_type(scrutinee_type)
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
            omitted_local_bindings: Vec::new(),
            local_names: BTreeMap::new(),
            inferred_effects: Vec::new(),
            inferred_return_type: None,
            diagnostics: Vec::new(),
        }
    }

    pub(super) fn check_body(&mut self) {
        self.check_function_annotations();
        self.check_contracts();
        for (index, line) in self.function.body.iter().enumerate() {
            match &line.kind {
                BodyLineKind::Let {
                    pattern,
                    annotation,
                    expr,
                } => {
                    let expected = annotation.as_deref().and_then(|annotation| {
                        self.parse_annotation(
                            annotation,
                            line.node_id,
                            &line.span,
                            ExpectedTypeSource::LocalAnnotation,
                            "Type annotation declared here.",
                        )
                    });
                    let initializer_diagnostic_count = self.diagnostics.len();
                    let actual = self.infer_expr(expr, expected.as_ref());
                    let initializer_has_diagnostic =
                        self.diagnostics.len() != initializer_diagnostic_count;
                    if let Some(expected) = &expected {
                        self.check_assignable(expr, &expected.ty, &actual, expected, "assignable");
                    }
                    self.check_let_pattern_supported(pattern);
                    let binding_type = expected
                        .as_ref()
                        .map_or_else(|| actual.clone(), |expected| expected.ty.clone());
                    let pattern_bindings = self.pattern_bindings(pattern, &binding_type);
                    for binding in pattern_bindings {
                        if !self.declare_local_name(
                            &binding.name,
                            binding.node_id.display("pattern"),
                            binding.span.clone(),
                            "local binding",
                        ) {
                            continue;
                        }
                        self.bindings.push(Binding {
                            name: binding.name.clone(),
                            ty: binding.ty.clone(),
                        });
                        if annotation.is_none()
                            && !initializer_has_diagnostic
                            && type_contains_unknown(&binding.ty)
                        {
                            self.omitted_local_bindings.push(OmittedLocalBinding {
                                name: binding.name,
                                node_id: binding.node_id,
                                span: binding.span,
                            });
                        }
                    }
                }
                BodyLineKind::Expr { expr } => {
                    let expected = self.return_expected(line.node_id);
                    let actual = self.infer_expr(expr, expected.as_ref());
                    if index + 1 == self.function.body.len() {
                        self.inferred_return_type = Some(actual.clone());
                        if let Some(expected) = &expected {
                            self.check_assignable(
                                expr,
                                &expected.ty,
                                &actual,
                                expected,
                                "return_value",
                            );
                        }
                    }
                }
            }
        }
        self.check_implicit_unit_return();
        self.check_omitted_local_inference_complete();
        self.check_private_inference_complete();
        self.check_effect_boundaries();
    }

    pub(super) fn check_implicit_unit_return(&mut self) {
        if matches!(
            self.function.body.last().map(|line| &line.kind),
            Some(BodyLineKind::Expr { .. })
        ) {
            return;
        }
        let Some(expected) = self.return_expected(self.function.node_id) else {
            self.inferred_return_type = Some(Type::unit());
            return;
        };
        let actual = Type::unit();
        if is_assignable(&expected.ty, &actual) {
            return;
        }
        self.diagnostics.push(Diagnostic::new(
            "type.mismatch",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "expected `{}`, but found `{}`",
                expected.ty.render(),
                actual.render()
            ),
            Some(self.function.span.clone()),
            type_details(
                self.function.node_id.display("fn"),
                expected.ty.render(),
                actual.render(),
                expected.source.as_type_source(),
                "implicit_unit",
                "return_value",
                [
                    self.function.node_id.display("fn"),
                    expected.origin_node_id.display("fn"),
                ],
            ),
        ));
    }

    pub(super) fn check_private_inference_complete(&mut self) {
        if self.function.visibility == Visibility::Public
            || self.function.kind != FunctionKind::Function
        {
            return;
        }
        for param in &self.function.params {
            if param.ty.is_some() {
                continue;
            }
            let inferred = self
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.name == param.name)
                .map(|binding| &binding.ty)
                .unwrap_or(&Type::Unknown);
            if inferred == &Type::Unknown {
                let mut diagnostic = Diagnostic::new(
                    "type.private_inference_incomplete",
                    Severity::Error,
                    DiagnosticKind::Type,
                    format!("private parameter `{}` has no inferred type", param.name),
                    Some(param.span.clone()),
                    JsonValue::object([
                        ("phase", JsonValue::string("type_check")),
                        ("node_id", JsonValue::string(param.node_id.display("param"))),
                        ("boundary", JsonValue::string("private_function")),
                        ("missing_fact", JsonValue::string("parameter_type")),
                        ("inferred_type", JsonValue::string("unknown")),
                    ]),
                );
                diagnostic.related.push(JsonValue::object([
                    ("kind", JsonValue::string("repair_hint")),
                    (
                        "message",
                        JsonValue::string("Add a parameter type annotation."),
                    ),
                    ("span", span_json(&param.span)),
                ]));
                self.diagnostics.push(diagnostic);
            }
        }
        if self.function.return_type.is_some() {
            return;
        }
        if self.inferred_return_type.as_ref() == Some(&Type::Unknown) {
            let mut diagnostic = Diagnostic::new(
                "type.private_inference_incomplete",
                Severity::Error,
                DiagnosticKind::Type,
                "private function has no inferred return type",
                Some(self.function.span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("type_check")),
                    (
                        "node_id",
                        JsonValue::string(self.function.node_id.display("fn")),
                    ),
                    ("boundary", JsonValue::string("private_function")),
                    ("missing_fact", JsonValue::string("return_type")),
                    ("inferred_type", JsonValue::string("unknown")),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("repair_hint")),
                (
                    "message",
                    JsonValue::string("Add a return type annotation."),
                ),
                ("span", span_json(&self.function.span)),
            ]));
            self.diagnostics.push(diagnostic);
        }
    }

    fn check_omitted_local_inference_complete(&mut self) {
        for omitted in &self.omitted_local_bindings {
            let inferred = self
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.name == omitted.name)
                .map(|binding| &binding.ty)
                .unwrap_or(&Type::Unknown);
            if !type_contains_unknown(inferred) {
                continue;
            }
            let mut diagnostic = Diagnostic::new(
                "type.local_inference_incomplete",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "omitted local binding `{}` has no concrete inferred type",
                    omitted.name
                ),
                Some(omitted.span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("type_check")),
                    (
                        "node_id",
                        JsonValue::string(omitted.node_id.display("pattern")),
                    ),
                    ("slot_kind", JsonValue::string("local_binding")),
                    ("binding", JsonValue::string(omitted.name.clone())),
                    ("inferred_type", JsonValue::string(inferred.render())),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("repair_hint")),
                (
                    "message",
                    JsonValue::string(
                        "Add a type annotation or a later same-function use that fixes the type.",
                    ),
                ),
                ("span", span_json(&omitted.span)),
            ]));
            self.diagnostics.push(diagnostic);
        }
    }

    pub(super) fn check_let_pattern_supported(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Binding(_) => {}
            PatternKind::Record(fields) => {
                for field in fields {
                    self.check_let_pattern_supported(&field.pattern);
                }
            }
            PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit
            | PatternKind::Constructor { .. } => {
                let mut diagnostic = Diagnostic::new(
                    "pattern.refutable_let",
                    Severity::Error,
                    DiagnosticKind::Type,
                    "refutable let pattern is not supported",
                    Some(pattern.span.clone()),
                    JsonValue::object([
                        ("phase", JsonValue::string("type_check")),
                        (
                            "node_id",
                            JsonValue::string(pattern.node_id.display("pattern")),
                        ),
                    ]),
                );
                diagnostic.related.push(JsonValue::object([
                    ("kind", JsonValue::string("let_pattern")),
                    (
                        "message",
                        JsonValue::string(
                            "Use a binding, wildcard, or record pattern in a let statement.",
                        ),
                    ),
                    ("span", span_json(&pattern.span)),
                ]));
                self.diagnostics.push(diagnostic);
            }
        }
    }

    pub(super) fn check_function_annotations(&mut self) {
        let variadic_count = self
            .function
            .params
            .iter()
            .filter(|param| param.is_variadic)
            .count();
        for param in &self.function.params {
            let ty = param.ty.as_deref().and_then(|annotation| {
                self.parse_annotation(
                    annotation,
                    param.node_id,
                    &param.span,
                    ExpectedTypeSource::DeclaredParameter,
                    "Parameter type declared here.",
                )
            });
            if param.is_variadic {
                if param.ty.as_deref().is_none_or(str::is_empty) {
                    self.push_variadic_parameter_diagnostic(
                        param.node_id,
                        param.span.clone(),
                        "type.variadic_parameter_type",
                        format!(
                            "variadic parameter `{}` is missing an element type",
                            param.name
                        ),
                        "element_type",
                    );
                }
                if self
                    .function
                    .params
                    .last()
                    .is_some_and(|last| last.node_id != param.node_id)
                {
                    self.push_variadic_parameter_diagnostic(
                        param.node_id,
                        param.span.clone(),
                        "type.variadic_parameter_position",
                        format!(
                            "variadic parameter `{}` must be the final parameter",
                            param.name
                        ),
                        "final_parameter",
                    );
                }
                if variadic_count > 1 {
                    self.push_variadic_parameter_diagnostic(
                        param.node_id,
                        param.span.clone(),
                        "type.variadic_parameter_duplicate",
                        "function parameter list has more than one variadic parameter".to_string(),
                        "single_variadic_parameter",
                    );
                }
            }
            if !self.declare_local_name(
                &param.name,
                param.node_id.display("param"),
                param.span.clone(),
                "parameter",
            ) {
                continue;
            }
            let inferred_private_param = (self.function.visibility == Visibility::Private
                && self.function.kind == FunctionKind::Function
                && param.ty.is_none())
            .then(|| {
                self.environment
                    .function_by_node_id(self.function.node_id)
                    .and_then(|function| {
                        self.function
                            .params
                            .iter()
                            .position(|candidate| candidate.node_id == param.node_id)
                            .and_then(|index| function.params.get(index).cloned())
                    })
            })
            .flatten()
            .filter(|ty| !type_contains_unknown(ty));
            self.bindings.push(Binding {
                name: param.name.clone(),
                ty: inferred_private_param.unwrap_or_else(|| {
                    ty.map_or(Type::Unknown, |expected| {
                        if param.is_variadic {
                            Type::named("List", vec![expected.ty])
                        } else {
                            expected.ty
                        }
                    })
                }),
            });
        }

        if let Some(return_type) = &self.function.return_type {
            self.parse_annotation(
                return_type,
                self.function.node_id,
                &self.function.span,
                ExpectedTypeSource::DeclaredReturn,
                "Return type declared here.",
            );
        }

        if let Some(result_binding) = &self.function.return_binding
            && let Some(param) = self
                .function
                .params
                .iter()
                .find(|param| param.name == result_binding.name)
        {
            let mut diagnostic = Diagnostic::new(
                "name.duplicate",
                Severity::Error,
                DiagnosticKind::Name,
                format!("duplicate result binding name `{}`", result_binding.name),
                Some(result_binding.span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("name")),
                    (
                        "node_id",
                        JsonValue::string(result_binding.node_id.display("result")),
                    ),
                    ("name", JsonValue::string(result_binding.name.clone())),
                    ("namespace", JsonValue::string("value")),
                    (
                        "first_node_id",
                        JsonValue::string(param.node_id.display("param")),
                    ),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("duplicate_origin")),
                (
                    "message",
                    JsonValue::string("Parameter with this name is here."),
                ),
                ("span", span_json(&param.span)),
            ]));
            self.diagnostics.push(diagnostic);
        }
    }

    fn push_variadic_parameter_diagnostic(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        id: &'static str,
        message: String,
        expected: &'static str,
    ) {
        self.diagnostics.push(Diagnostic::new(
            id,
            Severity::Error,
            DiagnosticKind::Type,
            message,
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("type")),
                ("node_id", JsonValue::string(node_id.display("param"))),
                ("expected", JsonValue::string(expected)),
                ("actual", JsonValue::string("variadic_parameter")),
                ("constraint", JsonValue::string("function_parameter_shape")),
            ]),
        ));
    }

    pub(super) fn declare_local_name(
        &mut self,
        name: &str,
        node_id: String,
        span: SourceSpan,
        declaration_kind: &'static str,
    ) -> bool {
        if let Some((first_node_id, first_span)) = self.local_names.get(name) {
            self.diagnostics.push(duplicate_name_diagnostic(
                name,
                "value",
                declaration_kind,
                node_id,
                span,
                first_node_id.clone(),
                first_span,
            ));
            false
        } else {
            self.local_names
                .insert(name.to_string(), (node_id, span.clone()));
            true
        }
    }

    pub(super) fn check_contracts(&mut self) {
        for contract in &self.function.contracts {
            let validation = self.validate_contract_predicate(contract.kind, &contract.text);
            match validation {
                ContractValidation::Valid => {}
                ContractValidation::NonBoolean { actual_type } => {
                    self.diagnostics.push(Diagnostic::new(
                        "contract.type_mismatch",
                        Severity::Error,
                        DiagnosticKind::Contract,
                        "contract predicate is not `Bool`",
                        Some(contract.span.clone()),
                        contract_details(
                            contract.node_id.display("contract"),
                            contract.kind,
                            &contract.text,
                            "invalid",
                            "failed_static",
                            "non_boolean_predicate",
                            false,
                            self.contract_referenced_bindings(contract.kind, &contract.text),
                        ),
                    ));
                    self.diagnostics.push(Diagnostic::new(
                        "type.mismatch",
                        Severity::Error,
                        DiagnosticKind::Type,
                        format!("expected `Bool`, but found `{actual_type}`"),
                        Some(contract.span.clone()),
                        type_details(
                            contract.node_id.display("contract"),
                            "Bool",
                            actual_type,
                            "contract_predicate",
                            "inferred_expression",
                            "contract_predicate",
                            [
                                self.function.node_id.display("fn"),
                                contract.node_id.display("contract"),
                            ],
                        ),
                    ));
                }
                ContractValidation::UnsupportedConstruct { reason } => {
                    self.diagnostics.push(Diagnostic::new(
                        "contract.unsupported_construct",
                        Severity::Error,
                        DiagnosticKind::Contract,
                        "contract predicate contains an unsupported construct",
                        Some(contract.span.clone()),
                        contract_details(
                            contract.node_id.display("contract"),
                            contract.kind,
                            &contract.text,
                            "invalid",
                            "failed_static",
                            reason,
                            false,
                            self.contract_referenced_bindings(contract.kind, &contract.text),
                        ),
                    ));
                }
                ContractValidation::MissingField { base_type, field } => {
                    self.diagnostics.push(Diagnostic::new(
                        "contract.field_missing",
                        Severity::Error,
                        DiagnosticKind::Contract,
                        format!("contract field `{field}` is not present on `{base_type}`"),
                        Some(contract.span.clone()),
                        contract_details(
                            contract.node_id.display("contract"),
                            contract.kind,
                            &contract.text,
                            "invalid",
                            "failed_static",
                            "missing_field",
                            false,
                            self.contract_referenced_bindings(contract.kind, &contract.text),
                        ),
                    ));
                }
                ContractValidation::UnresolvedName { name } => {
                    self.push_unresolved_name(
                        contract.node_id,
                        contract.span.clone(),
                        &name,
                        "contract_predicate",
                    );
                }
            }
        }
    }

    pub(super) fn check_effect_boundaries(&mut self) {
        let boundary = if self.function.kind == FunctionKind::Test {
            Some((
                "test_declaration",
                "effect.missing_test",
                "test declaration",
            ))
        } else if self.function.visibility == Visibility::Public {
            Some((
                "public_function",
                "effect.missing_public",
                "public function",
            ))
        } else {
            None
        };
        let Some((boundary, diagnostic_id, subject)) = boundary else {
            return;
        };
        if self
            .function
            .effects
            .as_ref()
            .is_some_and(|declared_effects| declared_effects.is_empty())
        {
            return;
        }
        let empty_declared_effects = Vec::new();
        let declared_effects = self
            .function
            .effects
            .as_ref()
            .unwrap_or(&empty_declared_effects);

        let mut inferred_effects = Vec::<String>::new();
        for effect_use in &self.inferred_effects {
            if !inferred_effects.contains(&effect_use.effect) {
                inferred_effects.push(effect_use.effect.clone());
            }
        }

        for effect in &inferred_effects {
            if declared_effects.iter().any(|declared| declared == effect) {
                continue;
            }
            let provenance = self
                .inferred_effects
                .iter()
                .filter(|effect_use| &effect_use.effect == effect)
                .take(3)
                .cloned()
                .collect::<Vec<_>>();
            let matching_path_count = self
                .inferred_effects
                .iter()
                .filter(|effect_use| &effect_use.effect == effect)
                .count();
            let omitted_path_count = matching_path_count.saturating_sub(provenance.len());
            let mut diagnostic = Diagnostic::new(
                diagnostic_id,
                Severity::Error,
                DiagnosticKind::Effect,
                format!("{subject} uses undeclared effect `{effect}`"),
                Some(self.function.span.clone()),
                effect_missing_public_details(
                    self.function
                        .node_id
                        .display(self.function.kind.node_prefix()),
                    self.function.name.as_deref().unwrap_or("<missing>"),
                    &self.function.span,
                    effect,
                    boundary,
                    declared_effects,
                    &inferred_effects,
                    &provenance,
                    matching_path_count > provenance.len(),
                    omitted_path_count,
                ),
            );
            for effect_use in provenance {
                diagnostic.related.push(JsonValue::object([
                    ("kind", JsonValue::string("effect_provenance")),
                    (
                        "message",
                        JsonValue::string(format!(
                            "Call to `{}` requires this effect.",
                            effect_use.symbol
                        )),
                    ),
                    ("span", span_json(&effect_use.span)),
                ]));
            }
            self.diagnostics.push(diagnostic);
        }
    }

    pub(super) fn validate_contract_predicate(
        &self,
        kind: ContractKind,
        predicate: &str,
    ) -> ContractValidation {
        self.validate_predicate_with_bindings(predicate, &self.contract_bindings(kind))
    }

    pub(super) fn validate_predicate_with_bindings(
        &self,
        predicate: &str,
        bindings: &[Binding],
    ) -> ContractValidation {
        let trimmed = predicate.trim();
        if trimmed.is_empty() {
            return ContractValidation::UnsupportedConstruct {
                reason: "empty_predicate",
            };
        }
        if trimmed.contains("stdio::") {
            return ContractValidation::UnsupportedConstruct {
                reason: "effectful_operation",
            };
        }
        let calls = contract_calls(trimmed)
            .into_iter()
            .filter(|call| !is_contract_keyword(&call.callee))
            .collect::<Vec<_>>();
        if let Some(validation) = self.validate_contract_calls(trimmed, bindings, &calls) {
            return validation;
        }
        if let Some(validation) = self.validate_contract_referenced_names(trimmed, bindings, &calls)
        {
            return validation;
        }
        if let Some(validation) = self.validate_whole_contract_call(trimmed, &calls) {
            return validation;
        }
        if let Some(validation) = self.validate_missing_contract_field(trimmed, bindings) {
            return validation;
        }
        self.validate_boolean_contract_predicate(trimmed, bindings)
    }

    pub(super) fn validate_contract_calls(
        &self,
        predicate: &str,
        bindings: &[Binding],
        calls: &[ContractCall],
    ) -> Option<ContractValidation> {
        for (call_index, call) in calls.iter().enumerate() {
            if let Some(validation) =
                self.validate_contract_call(predicate, bindings, calls, call_index, call)
            {
                return Some(validation);
            }
        }
        None
    }

    pub(super) fn validate_contract_call(
        &self,
        predicate: &str,
        bindings: &[Binding],
        calls: &[ContractCall],
        call_index: usize,
        call: &ContractCall,
    ) -> Option<ContractValidation> {
        let Some((params, return_type, effects)) = self.contract_call_signature(&call.callee)
        else {
            return Some(ContractValidation::UnresolvedName {
                name: call.callee.clone(),
            });
        };
        if !effects.is_empty() {
            return Some(ContractValidation::UnsupportedConstruct {
                reason: "effectful_operation",
            });
        }
        if return_type != Type::bool()
            && !contract_call_result_is_compared(predicate, call.start, call.end)
            && !contract_call_result_feeds_boolean_predicate(predicate, call.start, call.end)
            && !contract_call_result_has_field_access(predicate, call.end)
            && !contract_call_is_argument(calls, call_index)
        {
            return Some(ContractValidation::NonBoolean {
                actual_type: return_type.render(),
            });
        }
        if call.args.len() != params.len() {
            return Some(ContractValidation::UnsupportedConstruct {
                reason: "call_arity",
            });
        }
        for (arg, expected) in call.args.iter().zip(&params) {
            if let Some(validation) = self.validate_contract_call_argument(arg, expected, bindings)
            {
                return Some(validation);
            }
        }
        None
    }

    pub(super) fn validate_contract_call_argument(
        &self,
        arg: &str,
        expected: &Type,
        bindings: &[Binding],
    ) -> Option<ContractValidation> {
        let arg_calls = contract_calls(arg);
        if let Some(name) = referenced_names(arg)
            .into_iter()
            .find(|name| !self.contract_reference_is_resolved(name, bindings, &arg_calls))
        {
            return Some(ContractValidation::UnresolvedName { name });
        }
        let actual_type = self.predicate_arg_type(arg, bindings);
        (!is_assignable(expected, &actual_type)).then_some(
            ContractValidation::UnsupportedConstruct {
                reason: "call_argument_type",
            },
        )
    }

    pub(super) fn validate_contract_referenced_names(
        &self,
        predicate: &str,
        bindings: &[Binding],
        calls: &[ContractCall],
    ) -> Option<ContractValidation> {
        referenced_names(predicate)
            .into_iter()
            .find(|name| !self.contract_reference_is_resolved(name, bindings, calls))
            .map(|name| ContractValidation::UnresolvedName { name })
    }

    pub(super) fn contract_reference_is_resolved(
        &self,
        name: &str,
        bindings: &[Binding],
        calls: &[ContractCall],
    ) -> bool {
        is_contract_keyword(name)
            || name == "true"
            || name == "false"
            || calls.iter().any(|call| call.callee == name)
            || bindings.iter().any(|binding| binding.name == name)
            || matches!(
                self.environment
                    .unqualified_function(name, self.function.module_name.as_deref()),
                FunctionLookup::Found(_)
            )
    }

    pub(super) fn validate_whole_contract_call(
        &self,
        predicate: &str,
        calls: &[ContractCall],
    ) -> Option<ContractValidation> {
        let call = calls
            .iter()
            .find(|call| call.start == 0 && call.end == predicate.len())?;
        let return_type = self
            .contract_call_signature(&call.callee)
            .map(|(_, return_type, _)| return_type)
            .unwrap_or(Type::Unknown);
        Some(if return_type == Type::bool() {
            ContractValidation::Valid
        } else {
            ContractValidation::NonBoolean {
                actual_type: return_type.render(),
            }
        })
    }

    pub(super) fn validate_missing_contract_field(
        &self,
        predicate: &str,
        bindings: &[Binding],
    ) -> Option<ContractValidation> {
        missing_contract_field(predicate, bindings, &|callee| {
            self.contract_call_signature(callee)
                .map(|(_, return_type, _)| return_type)
        })
        .map(|(base_type, field)| ContractValidation::MissingField { base_type, field })
    }

    pub(super) fn validate_boolean_contract_predicate(
        &self,
        predicate: &str,
        bindings: &[Binding],
    ) -> ContractValidation {
        if predicate_is_boolean_with_calls(predicate, bindings, &|callee| {
            self.contract_call_signature(callee)
                .map(|(_, return_type, _)| return_type)
        }) {
            ContractValidation::Valid
        } else {
            ContractValidation::NonBoolean {
                actual_type: predicate_rendered_type_with_calls(predicate, bindings, &|callee| {
                    self.contract_call_signature(callee)
                        .map(|(_, return_type, _)| return_type)
                }),
            }
        }
    }

    pub(super) fn contract_call_signature(
        &self,
        callee: &str,
    ) -> Option<(Vec<Type>, Type, Vec<String>)> {
        let segments = contract_callee_segments(callee);
        let signature = match segments.as_slice() {
            [name] => self
                .environment
                .unqualified_function(name, self.function.module_name.as_deref())
                .found(),
            _ => self
                .environment
                .function_path(&segments, self.function.module_name.as_deref()),
        };
        signature
            .map(|signature| {
                (
                    signature.params.clone(),
                    signature.return_type.clone(),
                    signature.effects.clone(),
                )
            })
            .or_else(|| match segments.as_slice() {
                [name] if !self.bare_prelude_import_is_ambiguous(name) => {
                    prelude_signature(name, None)
                        .map(|(params, return_type)| (params, return_type, Vec::new()))
                }
                _ => qualified_prelude_signature(&segments, None)
                    .map(|(_, params, return_type)| (params, return_type, Vec::new())),
            })
    }

    pub(super) fn predicate_arg_type(&self, arg: &str, bindings: &[Binding]) -> Type {
        let trimmed = arg.trim();
        if trimmed.starts_with('"') {
            return Type::string();
        }
        if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
            return Type::int();
        }
        if matches!(trimmed, "true" | "false") {
            return Type::bool();
        }
        if let [call] = contract_calls(trimmed).as_slice()
            && call.start == 0
            && call.end == trimmed.len()
        {
            return self
                .contract_call_signature(&call.callee)
                .map(|(_, return_type, _)| return_type)
                .unwrap_or(Type::Unknown);
        }
        if let Some(ty) = predicate_type_with_calls(trimmed, bindings, &|callee| {
            self.contract_call_signature(callee)
                .map(|(_, return_type, _)| return_type)
        }) {
            return ty;
        }
        let segments = contract_callee_segments(trimmed);
        match segments.as_slice() {
            [name] => {
                if let FunctionLookup::Found(function) = self
                    .environment
                    .unqualified_function(name, self.function.module_name.as_deref())
                {
                    return function.ty();
                }
            }
            _ => {
                if let Some(function) = self
                    .environment
                    .function_path(&segments, self.function.module_name.as_deref())
                {
                    return function.ty();
                }
            }
        }
        let mut parts = trimmed.split('.');
        let Some(base) = parts.next() else {
            return Type::Unknown;
        };
        let Some(binding) = bindings.iter().find(|binding| binding.name == base) else {
            return Type::Unknown;
        };
        let mut current = binding.ty.clone();
        for field in parts {
            let Some(next) = current.record_field(field) else {
                return Type::Unknown;
            };
            current = next.clone();
        }
        current
    }

    pub(super) fn contract_referenced_bindings(
        &self,
        kind: ContractKind,
        predicate: &str,
    ) -> Vec<JsonValue> {
        referenced_names(predicate)
            .into_iter()
            .filter_map(|name| {
                if kind == ContractKind::Ensure
                    && self
                        .function
                        .return_binding
                        .as_ref()
                        .is_some_and(|binding| binding.name == name)
                {
                    return Some(JsonValue::object([
                        ("name", JsonValue::string(name)),
                        ("kind", JsonValue::string("result")),
                    ]));
                }
                self.bindings
                    .iter()
                    .any(|binding| binding.name == name)
                    .then(|| {
                        JsonValue::object([
                            ("name", JsonValue::string(name)),
                            ("kind", JsonValue::string("local")),
                        ])
                    })
            })
            .collect()
    }

    pub(super) fn contract_bindings(&self, kind: ContractKind) -> Vec<Binding> {
        let mut bindings = self.bindings.clone();
        if kind == ContractKind::Ensure
            && let Some(result_binding) = &self.function.return_binding
        {
            bindings.push(Binding {
                name: result_binding.name.clone(),
                ty: self
                    .function
                    .return_type
                    .as_deref()
                    .and_then(|return_type| parse_type_annotation(return_type).ok())
                    .unwrap_or(Type::Unknown),
            });
        }
        bindings
    }

    pub(super) fn parse_annotation(
        &mut self,
        annotation: &str,
        origin_node_id: NodeId,
        origin_span: &SourceSpan,
        source: ExpectedTypeSource,
        origin_message: &'static str,
    ) -> Option<ExpectedType> {
        match parse_type_annotation(annotation) {
            Ok(ty) => Some(ExpectedType {
                ty,
                source,
                origin_node_id,
                origin_span: Some(origin_span.clone()),
                origin_message,
            }),
            Err(error) => {
                self.push_invalid_type_annotation(
                    annotation,
                    &error,
                    origin_node_id,
                    origin_span.clone(),
                );
                None
            }
        }
    }

    pub(super) fn return_expected(&self, origin_node_id: NodeId) -> Option<ExpectedType> {
        self.function
            .return_type
            .as_deref()
            .and_then(|return_type| parse_type_annotation(return_type).ok())
            .map(|ty| ExpectedType {
                ty,
                source: ExpectedTypeSource::DeclaredReturn,
                origin_node_id,
                origin_span: Some(self.function.span.clone()),
                origin_message: "Return type declared here.",
            })
            .or_else(|| {
                (self.function.visibility == Visibility::Private
                    && self.function.kind == FunctionKind::Function)
                    .then(|| {
                        self.environment
                            .function_by_node_id(self.function.node_id)
                            .map(|function| function.return_type.clone())
                    })
                    .flatten()
                    .filter(|ty| !type_contains_unknown(ty))
                    .map(|ty| ExpectedType {
                        ty,
                        source: ExpectedTypeSource::Inferred,
                        origin_node_id,
                        origin_span: Some(self.function.span.clone()),
                        origin_message: "Private return type inferred here.",
                    })
            })
    }

    pub(super) fn infer_expr(&mut self, expr: &Expr, expected: Option<&ExpectedType>) -> Type {
        match &expr.kind {
            ExprKind::Missing => Type::Unknown,
            ExprKind::Hole { name, satisfy } => {
                if let Some(satisfy) = satisfy {
                    self.check_satisfy_clause(expr, satisfy, expected);
                }
                self.push_hole_diagnostic(expr, name.as_deref(), satisfy.as_ref(), expected);
                expected
                    .map(|expected| expected.ty.clone())
                    .unwrap_or(Type::Unknown)
            }
            ExprKind::NamePath(segments) => self.infer_name_path(segments, expr, expected),
            ExprKind::StringLiteral(_) => Type::string(),
            ExprKind::IntLiteral(_) => Type::int(),
            ExprKind::FloatLiteral(_) => Type::float(),
            ExprKind::BoolLiteral(_) => Type::bool(),
            ExprKind::Unit => Type::unit(),
            ExprKind::TypeApply { .. } => Type::Unknown,
            ExprKind::Call { callee, args } => self.infer_call(expr, callee, args, expected),
            ExprKind::FieldAccess {
                base,
                field,
                field_span,
            } => self.infer_field_access(expr, base, field, field_span),
            ExprKind::Try(inner) => self.infer_try(expr, inner, expected),
            ExprKind::Record(fields) => self.infer_record(expr, fields, expected),
            ExprKind::Dict(entries) => self.infer_dict(expr, entries, expected),
            ExprKind::List(items) => self.infer_list(expr, items, expected),
            ExprKind::Match { scrutinee, arms } => {
                self.infer_match(expr, scrutinee, arms, expected)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => self.infer_if(
                expr,
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                expected,
            ),
            ExprKind::Prefix { op, expr } => self.infer_prefix(*op, expr, expected),
            ExprKind::Binary { op, left, right } => self.infer_binary(*op, left, right, expected),
        }
    }

    pub(super) fn infer_name_path(
        &mut self,
        segments: &[String],
        expr: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Type {
        match self.environment.adts.nullary_constructor(
            segments,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        ) {
            ConstructorLookup::Found(constructor) => {
                let inferred = expected
                    .and_then(|expected| {
                        adt::adt_args(&expected.ty, constructor.descriptor)
                            .map(|_| expected.ty.clone())
                    })
                    .unwrap_or_else(|| adt::constructed_type(constructor, &[]));
                if type_contains_unknown(&inferred)
                    && ((expected.is_none() && constructor.variant.kind != AdtVariantKind::ListNil)
                        || (expected.is_some()
                            && constructor.variant.kind == AdtVariantKind::ListNil))
                {
                    self.push_ambiguous_constructor_type(
                        expr.node_id,
                        expr.span.clone(),
                        &segments.join("::"),
                        &inferred,
                    );
                }
                inferred
            }
            ConstructorLookup::Ambiguous => {
                self.push_ambiguous_name(
                    expr.node_id,
                    expr.span.clone(),
                    &segments.join("::"),
                    "value",
                );
                Type::Unknown
            }
            ConstructorLookup::Missing => match segments {
                [name] => {
                    if let Some(ty) = self.infer_local_binding_name(name, expected) {
                        ty
                    } else if self.bare_prelude_import_is_ambiguous(name) {
                        self.push_ambiguous_unqualified_function_import(
                            expr.node_id,
                            expr.span.clone(),
                            name,
                            "value",
                        );
                        Type::Unknown
                    } else {
                        match self
                            .environment
                            .unqualified_function(name, self.function.module_name.as_deref())
                        {
                            FunctionLookup::Found(function) => function.ty(),
                            FunctionLookup::Ambiguous => {
                                self.push_ambiguous_unqualified_function_import(
                                    expr.node_id,
                                    expr.span.clone(),
                                    name,
                                    "value",
                                );
                                Type::Unknown
                            }
                            FunctionLookup::Missing => {
                                self.push_unresolved_name(
                                    expr.node_id,
                                    expr.span.clone(),
                                    name,
                                    "value",
                                );
                                Type::Unknown
                            }
                        }
                    }
                }
                _ => {
                    if let Some(function) = self
                        .environment
                        .function_path(segments, self.function.module_name.as_deref())
                    {
                        return function.ty();
                    }
                    let symbol = segments.join("::");
                    self.push_unresolved_name(expr.node_id, expr.span.clone(), &symbol, "value");
                    Type::Unknown
                }
            },
        }
    }

    fn infer_local_binding_name(
        &mut self,
        name: &str,
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        let index = self
            .bindings
            .iter()
            .rposition(|binding| binding.name == name)?;
        let current = self.bindings[index].ty.clone();
        if matches!(current, Type::Record(ref fields) if fields.is_empty())
            && let Some(expected) = expected
            && expected.ty.dict_parts().is_some()
            && !type_contains_unknown(&expected.ty)
        {
            self.bindings[index].ty = expected.ty.clone();
            return Some(expected.ty.clone());
        }
        if type_contains_unknown(&current)
            && let Some(expected) = expected
            && !type_contains_unknown(&expected.ty)
        {
            self.bindings[index].ty = expected.ty.clone();
            return Some(expected.ty.clone());
        }
        Some(current)
    }

    pub(super) fn infer_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Type {
        if let Some(ty) = self.infer_constructor_call(expr, callee, args, expected) {
            return ty;
        }
        if let Some(ty) = self.infer_declared_call(expr, callee, args, expected) {
            return ty;
        }
        if let Some(ty) = self.infer_prelude_call(callee, args, expected) {
            return ty;
        }
        if let Some(ty) = self.diagnose_method_call(expr, callee, args) {
            return ty;
        }
        self.infer_unresolved_call(callee, args)
    }

    pub(super) fn infer_constructor_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        if let ExprKind::NamePath(segments) = &callee.kind {
            match self.environment.adts.constructor(
                segments,
                self.function.module_name.as_deref(),
                &self.environment.uses,
            ) {
                ConstructorLookup::Found(constructor)
                    if !constructor.variant.payload_fields.is_empty() =>
                {
                    return Some(self.infer_adt_constructor(expr, args, expected, constructor));
                }
                ConstructorLookup::Ambiguous => {
                    if let Some(constructor) = expected
                        .and_then(|expected| {
                            self.environment.adts.descriptor_for_type(&expected.ty)
                        })
                        .and_then(|descriptor| {
                            self.environment.adts.constructor_for_descriptor(
                                segments,
                                descriptor,
                                self.function.module_name.as_deref(),
                                &self.environment.uses,
                            )
                        })
                        .filter(|constructor| !constructor.variant.payload_fields.is_empty())
                    {
                        return Some(self.infer_adt_constructor(expr, args, expected, constructor));
                    }
                    self.push_ambiguous_name(
                        callee.node_id,
                        callee.span.clone(),
                        &segments.join("::"),
                        "call_target",
                    );
                    return Some(Type::Unknown);
                }
                _ => {}
            }
        }
        None
    }

    pub(super) fn infer_declared_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        if self.bare_call_is_ambiguous(callee) {
            if let ExprKind::NamePath(segments) = &callee.kind
                && let [name] = segments.as_slice()
            {
                self.push_ambiguous_unqualified_function_import(
                    callee.node_id,
                    callee.span.clone(),
                    name,
                    "call_target",
                );
            }
            for arg in args {
                self.infer_expr(arg, None);
            }
            return Some(Type::Unknown);
        }

        let (params, variadic, return_type, origin) = self.call_signature(
            callee,
            expected.map(|expected| &expected.ty),
            args.first()
                .and_then(|arg| self.shallow_expr_type(arg))
                .as_ref(),
            Some(args.len()),
        )?;

        for effect in &origin.effects {
            self.inferred_effects.push(EffectUse {
                effect: effect.clone(),
                node_id: expr.node_id,
                span: expr.span.clone(),
                kind: "direct_call",
                symbol: origin.symbol.clone(),
            });
        }
        self.check_call_arguments(args, &params, variadic.as_ref(), &origin);
        Some(return_type)
    }

    fn check_call_arguments(
        &mut self,
        args: &[Expr],
        params: &[Type],
        variadic: Option<&Type>,
        origin: &CallOrigin,
    ) {
        for (index, arg) in args.iter().enumerate() {
            let param_type = params.get(index).or(variadic);
            let Some(param_type) = param_type else {
                self.infer_expr(arg, None);
                continue;
            };
            let expected = ExpectedType {
                ty: param_type.clone(),
                source: ExpectedTypeSource::DeclaredParameter,
                origin_node_id: origin.node_id,
                origin_span: Some(origin.span.clone()),
                origin_message: "Callee parameter type declared here.",
            };
            let actual = self.infer_expr(arg, Some(&expected));
            self.check_assignable(arg, &expected.ty, &actual, &expected, "call_argument");
        }
    }

    pub(super) fn infer_prelude_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        let ExprKind::NamePath(segments) = &callee.kind else {
            return None;
        };
        let (name, params, return_type) = if let [name] = segments.as_slice() {
            if self.bare_name_is_shadowed(name) || self.bare_prelude_import_is_ambiguous(name) {
                return None;
            }
            let input_type =
                prelude_input_arg(args, name).and_then(|arg| self.shallow_expr_type(arg));
            let (params, return_type) = prelude_signature_with_input(
                name,
                expected.map(|expected| &expected.ty),
                input_type.as_ref(),
            )?;
            (name.clone(), params, return_type)
        } else if let Some((name, params, return_type)) = segments.last().and_then(|name| {
            let input_type =
                prelude_input_arg(args, name).and_then(|arg| self.shallow_expr_type(arg));
            qualified_prelude_signature_with_input(
                segments,
                expected.map(|expected| &expected.ty),
                input_type.as_ref(),
            )
        }) {
            (name, params, return_type)
        } else {
            segments.last().and_then(|name| {
                let input_type =
                    prelude_input_arg(args, name).and_then(|arg| self.shallow_expr_type(arg));
                qualified_prelude_builtin_signature_with_input(
                    segments,
                    expected.map(|expected| &expected.ty),
                    input_type.as_ref(),
                )
            })?
        };

        for (index, arg) in args.iter().enumerate() {
            let Some(param_type) = params.get(index) else {
                self.infer_expr(arg, None);
                continue;
            };
            let expected = ExpectedType {
                ty: param_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: callee.node_id,
                origin_span: Some(callee.span.clone()),
                origin_message: "Prelude helper parameter type inferred here.",
            };
            let actual = self.infer_expr(arg, Some(&expected));
            self.check_prelude_argument_assignable(&name, index, arg, &expected, &actual);
        }
        Some(return_type)
    }

    pub(super) fn diagnose_method_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
    ) -> Option<Type> {
        if let ExprKind::FieldAccess {
            base,
            field,
            field_span,
        } = &callee.kind
        {
            self.infer_expr(base, None);
            for arg in args {
                self.infer_expr(arg, None);
            }
            let mut diagnostic = Diagnostic::new(
                "type.method_call",
                Severity::Error,
                DiagnosticKind::Type,
                "method call syntax is not supported",
                Some(field_span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("type")),
                    ("node_id", JsonValue::string(expr.node_id.display("expr"))),
                    ("expected", JsonValue::string("function_call")),
                    ("actual", JsonValue::string("method_call")),
                    ("constraint", JsonValue::string("call_target")),
                    ("method", JsonValue::string(field.clone())),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("call_style")),
                (
                    "message",
                    JsonValue::string(
                        "Use a named function call with the receiver as an explicit argument.",
                    ),
                ),
                ("span", span_json(&callee.span)),
            ]));
            self.diagnostics.push(diagnostic);
            return Some(Type::Unknown);
        }
        None
    }

    pub(super) fn infer_unresolved_call(&mut self, callee: &Expr, args: &[Expr]) -> Type {
        if let Some((segments, type_args)) = callee_name_path_and_type_args(callee)
            && !known_concurrency_type_arg_overflow(segments, type_args)
        {
            let symbol = segments.join("::");
            self.push_unresolved_name(callee.node_id, callee.span.clone(), &symbol, "call_target");
        }
        for arg in args {
            self.infer_expr(arg, None);
        }
        Type::Unknown
    }

    pub(super) fn infer_field_access(
        &mut self,
        expr: &Expr,
        base: &Expr,
        field: &str,
        field_span: &SourceSpan,
    ) -> Type {
        let base_type = self.infer_expr(base, None);
        if let Some(field_type) = base_type.record_field(field) {
            return field_type.clone();
        }
        if base_type == Type::Unknown {
            return Type::Unknown;
        }
        let mut diagnostic = Diagnostic::new(
            "type.field_missing",
            Severity::Error,
            DiagnosticKind::Type,
            format!("type `{}` has no field `{field}`", base_type.render()),
            Some(field_span.clone()),
            type_details(
                expr.node_id.display("expr"),
                format!("record field `{field}`"),
                base_type.render(),
                "field_access",
                "inferred_expression",
                "field_access",
                [
                    self.function.node_id.display("fn"),
                    base.node_id.display("expr"),
                ],
            ),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("field_base")),
            (
                "message",
                JsonValue::string(format!(
                    "Field access base has type `{}`.",
                    base_type.render()
                )),
            ),
            ("span", span_json(&base.span)),
        ]));
        self.diagnostics.push(diagnostic);
        Type::Unknown
    }

    pub(super) fn call_signature(
        &self,
        callee: &Expr,
        expected: Option<&Type>,
        handle_type: Option<&Type>,
        arg_count: Option<usize>,
    ) -> Option<(Vec<Type>, Option<Type>, Type, CallOrigin)> {
        let bindings = self
            .bindings
            .iter()
            .map(|binding| crate::call_resolution::TypeBinding {
                name: &binding.name,
                ty: &binding.ty,
            })
            .collect::<Vec<_>>();
        let signature = crate::call_resolution::type_call_signature(
            callee,
            expected,
            handle_type,
            arg_count,
            &bindings,
            self.environment,
            self.function.module_name.as_deref(),
        )?;
        Some((
            signature.params,
            signature.variadic,
            signature.return_type,
            signature.origin,
        ))
    }

    fn bare_call_is_ambiguous(&self, callee: &Expr) -> bool {
        let ExprKind::NamePath(segments) = &callee.kind else {
            return false;
        };
        let [name] = segments.as_slice() else {
            return false;
        };
        if self.bare_name_is_shadowed(name) {
            return false;
        }
        if self.bare_prelude_import_is_ambiguous(name) {
            return true;
        }
        matches!(
            self.environment
                .unqualified_function(name, self.function.module_name.as_deref()),
            FunctionLookup::Ambiguous
        )
    }

    fn bare_name_is_shadowed(&self, name: &str) -> bool {
        self.bindings
            .iter()
            .rev()
            .any(|binding| binding.name == name)
            || matches!(
                self.environment
                    .unqualified_function(name, self.function.module_name.as_deref()),
                FunctionLookup::Found(function)
                    if function.module_name.as_deref() == self.function.module_name.as_deref()
            )
    }

    fn bare_prelude_import_is_ambiguous(&self, name: &str) -> bool {
        prelude_symbol(name).is_some()
            && !self
                .environment
                .unqualified_function_import_candidates(name, self.function.module_name.as_deref())
                .is_empty()
    }

    fn push_ambiguous_unqualified_function_import(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        name: &str,
        namespace: &'static str,
    ) {
        let mut diagnostic = Diagnostic::new(
            "name.ambiguous",
            Severity::Error,
            DiagnosticKind::Name,
            format!("ambiguous {namespace} `{name}`"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("name")),
                ("node_id", JsonValue::string(node_id.display("name"))),
                ("symbol", JsonValue::string(name)),
                ("namespace", JsonValue::string(namespace)),
                ("resolution_status", JsonValue::string("ambiguous")),
            ]),
        );
        for candidate in self
            .environment
            .unqualified_function_import_candidates(name, self.function.module_name.as_deref())
        {
            let Some(module_name) = candidate.module_name.as_deref() else {
                continue;
            };
            let Some(use_decl) = self
                .environment
                .uses
                .iter()
                .find(|use_decl| use_decl.name == module_name)
            else {
                continue;
            };
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("import_candidate")),
                (
                    "message",
                    JsonValue::string(format!(
                        "Imported module `{module_name}` exports `{name}`; use `{}::{name}` to select it.",
                        use_decl.alias
                    )),
                ),
                ("span", span_json(&use_decl.span)),
            ]));
        }
        if prelude_symbol(name).is_some() {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("import_candidate")),
                (
                    "message",
                    JsonValue::string(format!(
                        "The standard prelude exports `{name}`; use `prelude::{name}` to select it.",
                    )),
                ),
            ]));
        }
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn infer_adt_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
        constructor: adt::AdtConstructor,
    ) -> Type {
        let mut actual_args = Vec::new();
        for (index, _) in constructor.variant.payload_fields.iter().enumerate() {
            let expected_payload = expected
                .and_then(|expected| adt::payload_type(&expected.ty, constructor, index))
                .unwrap_or(Type::Unknown);
            let arg_expected = ExpectedType {
                ty: expected_payload,
                source: expected.map_or(ExpectedTypeSource::Unknown, |expected| expected.source),
                origin_node_id: expected.map_or(expr.node_id, |expected| expected.origin_node_id),
                origin_span: expected.and_then(|expected| expected.origin_span.clone()),
                origin_message: expected.map_or("Expected type inferred here.", |expected| {
                    expected.origin_message
                }),
            };
            let Some(arg) = args.get(index) else {
                continue;
            };
            let actual_arg = self.infer_expr(arg, Some(&arg_expected));
            self.check_assignable(
                arg,
                &arg_expected.ty,
                &actual_arg,
                &arg_expected,
                "call_argument",
            );
            actual_args.push(actual_arg);
        }
        for arg in args.iter().skip(constructor.variant.payload_fields.len()) {
            self.infer_expr(arg, None);
        }

        if expected
            .and_then(|expected| adt::adt_args(&expected.ty, constructor.descriptor))
            .is_some()
        {
            return expected
                .map(|expected| expected.ty.clone())
                .unwrap_or(Type::Unknown);
        }
        adt::constructed_type(constructor, &actual_args)
    }

    pub(super) fn infer_list(
        &mut self,
        expr: &Expr,
        items: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Type {
        if items.is_empty()
            && let Some(expected) = expected
            && expected.ty.vec_part().is_some()
            && type_contains_unknown(&expected.ty)
        {
            self.push_ambiguous_empty_collection_type(
                expr.node_id,
                expr.span.clone(),
                "Vec",
                &expected.ty,
            );
        }
        let expected_item = expected
            .and_then(|expected| expected.ty.vec_part())
            .cloned()
            .unwrap_or(Type::Unknown);
        let item_expected = ExpectedType {
            ty: expected_item.clone(),
            source: expected.map_or(ExpectedTypeSource::Unknown, |expected| expected.source),
            origin_node_id: expected.map_or(expr.node_id, |expected| expected.origin_node_id),
            origin_span: expected.and_then(|expected| expected.origin_span.clone()),
            origin_message: expected.map_or("Expected type inferred here.", |expected| {
                expected.origin_message
            }),
        };
        let mut item_type = expected_item.clone();
        for item in items {
            let actual = self.infer_expr(item, Some(&item_expected));
            self.check_assignable(
                item,
                &item_expected.ty,
                &actual,
                &item_expected,
                "assignable",
            );
            if item_type == Type::Unknown {
                item_type = actual;
            }
        }
        Type::vec(item_type)
    }

    pub(super) fn infer_match(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[MatchArm],
        expected: Option<&ExpectedType>,
    ) -> Type {
        let scrutinee_type = self.infer_expr(scrutinee, None);
        if arms.is_empty() {
            self.check_match_exhaustiveness(expr, scrutinee, &scrutinee_type, arms);
            return expected
                .map(|expected| expected.ty.clone())
                .unwrap_or(Type::Unknown);
        }

        let mut result_type = expected
            .map(|expected| expected.ty.clone())
            .unwrap_or(Type::Unknown);
        for arm in arms {
            let saved_bindings = self.bindings.len();
            let saved_names = self.local_names.clone();
            let pattern_bindings = self.pattern_bindings(&arm.pattern, &scrutinee_type);
            for binding in pattern_bindings {
                if !self.declare_local_name(
                    &binding.name,
                    binding.node_id.display("pattern"),
                    binding.span,
                    "pattern binding",
                ) {
                    continue;
                }
                self.bindings.push(Binding {
                    name: binding.name,
                    ty: binding.ty,
                });
            }

            let arm_expected = if let Some(expected) = expected {
                Some(expected.clone())
            } else if result_type != Type::Unknown {
                Some(ExpectedType {
                    ty: result_type.clone(),
                    source: ExpectedTypeSource::Inferred,
                    origin_node_id: expr.node_id,
                    origin_span: Some(expr.span.clone()),
                    origin_message: "Match result type inferred here.",
                })
            } else {
                None
            };
            let actual = self.infer_expr(&arm.expr, arm_expected.as_ref());
            if let Some(expected) = &arm_expected {
                self.check_assignable(&arm.expr, &expected.ty, &actual, expected, "match_arm");
            }
            if result_type == Type::Unknown {
                result_type = actual;
            }

            self.bindings.truncate(saved_bindings);
            self.local_names = saved_names;
        }

        self.check_match_exhaustiveness(expr, scrutinee, &scrutinee_type, arms);
        result_type
    }

    pub(super) fn infer_if(
        &mut self,
        expr: &Expr,
        condition: &Expr,
        then_branch: &Expr,
        else_if_branches: &[IfBranch],
        else_branch: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Type {
        self.check_if_condition(expr, condition);

        let mut result_type = expected
            .map(|expected| expected.ty.clone())
            .unwrap_or(Type::Unknown);
        self.infer_if_branch(expr, then_branch, expected, &mut result_type);
        for branch in else_if_branches {
            self.check_if_condition(expr, &branch.condition);
            self.infer_if_branch(expr, &branch.expr, expected, &mut result_type);
        }
        self.infer_if_branch(expr, else_branch, expected, &mut result_type);
        result_type
    }

    fn check_if_condition(&mut self, if_expr: &Expr, condition: &Expr) {
        let expected = ExpectedType {
            ty: Type::bool(),
            source: ExpectedTypeSource::Inferred,
            origin_node_id: if_expr.node_id,
            origin_span: Some(if_expr.span.clone()),
            origin_message: "If condition expected `Bool` here.",
        };
        let actual = self.infer_expr(condition, Some(&expected));
        self.check_assignable(condition, &expected.ty, &actual, &expected, "if_condition");
    }

    fn infer_if_branch(
        &mut self,
        if_expr: &Expr,
        branch_expr: &Expr,
        expected: Option<&ExpectedType>,
        result_type: &mut Type,
    ) {
        let branch_expected = if let Some(expected) = expected {
            Some(expected.clone())
        } else if *result_type != Type::Unknown {
            Some(ExpectedType {
                ty: result_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: if_expr.node_id,
                origin_span: Some(if_expr.span.clone()),
                origin_message: "If result type inferred here.",
            })
        } else {
            None
        };
        let actual = self.infer_expr(branch_expr, branch_expected.as_ref());
        if let Some(expected) = &branch_expected {
            self.check_assignable(branch_expr, &expected.ty, &actual, expected, "if_branch");
        }
        if *result_type == Type::Unknown {
            *result_type = actual;
        }
    }

    pub(super) fn check_match_exhaustiveness(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        scrutinee_type: &Type,
        arms: &[MatchArm],
    ) {
        let Some(domain) = MatchDomain::from_type(scrutinee_type, self.environment) else {
            return;
        };
        let mut covered = Vec::new();
        let mut proving_arms = Vec::new();
        for arm in arms {
            let coverage = match_pattern_coverage(
                &arm.pattern,
                &domain,
                scrutinee_type,
                self.environment,
                self.function.module_name.as_deref(),
            );
            if coverage.catches_all {
                return;
            }
            for case in coverage.cases {
                if !covered.contains(&case) {
                    covered.push(case.clone());
                    proving_arms.push((case, arm.pattern.span.clone()));
                }
            }
        }

        let cases = domain.cases(scrutinee_type, self.environment);
        let Some(missing_case) = cases.iter().find(|case| !covered.contains(case)).cloned() else {
            return;
        };

        let mut diagnostic = Diagnostic::new(
            "type.match_non_exhaustive",
            Severity::Error,
            DiagnosticKind::Type,
            format!("match is missing case {missing_case}"),
            Some(expr.span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("type")),
                ("node_id", JsonValue::string(expr.node_id.display("match"))),
                ("scrutinee_type", JsonValue::string(scrutinee_type.render())),
                ("missing_case", JsonValue::string(missing_case)),
                ("constraint", JsonValue::string("match_exhaustiveness")),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("scrutinee_type")),
            (
                "message",
                JsonValue::string(format!("Scrutinee has type `{}`.", scrutinee_type.render())),
            ),
            ("span", span_json(&scrutinee.span)),
        ]));
        for (case, span) in proving_arms {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("covered_case")),
                (
                    "message",
                    JsonValue::string(format!("This arm covers {case}.")),
                ),
                ("span", span_json(&span)),
            ]));
        }
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn pattern_bindings(
        &mut self,
        pattern: &Pattern,
        scrutinee_type: &Type,
    ) -> Vec<PatternBinding> {
        match &pattern.kind {
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => Vec::new(),
            PatternKind::Binding(name) => vec![PatternBinding {
                name: name.clone(),
                ty: scrutinee_type.clone(),
                node_id: pattern.node_id,
                span: pattern.span.clone(),
            }],
            PatternKind::Record(fields) => {
                let mut bindings = Vec::new();
                let mut seen_fields = BTreeMap::<String, (String, SourceSpan)>::new();
                for field in fields {
                    if let Some((first_node_id, first_span)) = seen_fields.get(&field.name) {
                        self.diagnostics.push(duplicate_name_diagnostic(
                            &field.name,
                            "record_field",
                            "record pattern field",
                            field.node_id.display("field"),
                            field.span.clone(),
                            first_node_id.clone(),
                            first_span,
                        ));
                    } else {
                        seen_fields.insert(
                            field.name.clone(),
                            (field.node_id.display("field"), field.span.clone()),
                        );
                    }
                    let field_type = scrutinee_type
                        .record_field(&field.name)
                        .unwrap_or(&Type::Unknown);
                    bindings.extend(self.pattern_bindings(&field.pattern, field_type));
                }
                bindings
            }
            PatternKind::Constructor { name, args } => {
                let Some(descriptor) = self.environment.adts.descriptor_for_type(scrutinee_type)
                else {
                    return Vec::new();
                };
                let Some(constructor) = self.environment.adts.constructor_for_descriptor(
                    name,
                    descriptor,
                    self.function.module_name.as_deref(),
                    &self.environment.uses,
                ) else {
                    return Vec::new();
                };
                args.iter()
                    .enumerate()
                    .flat_map(|(index, pattern)| {
                        let ty = adt::payload_type(scrutinee_type, constructor, index)
                            .unwrap_or(Type::Unknown);
                        self.pattern_bindings(pattern, &ty)
                    })
                    .collect()
            }
        }
    }

    pub(super) fn infer_record(
        &mut self,
        expr: &Expr,
        fields: &[RecordField],
        expected: Option<&ExpectedType>,
    ) -> Type {
        if fields.is_empty()
            && let Some(expected) = expected
            && expected.ty.dict_parts().is_some()
        {
            if type_contains_unknown(&expected.ty) {
                self.push_ambiguous_empty_collection_type(
                    expr.node_id,
                    expr.span.clone(),
                    "Dict",
                    &expected.ty,
                );
            }
            return expected.ty.clone();
        }
        let mut actual_fields = Vec::new();
        let mut seen_fields = BTreeMap::<String, (String, SourceSpan)>::new();
        for field in fields {
            if let Some((first_node_id, first_span)) = seen_fields.get(&field.name) {
                self.diagnostics.push(duplicate_name_diagnostic(
                    &field.name,
                    "record_field",
                    "record field",
                    field.node_id.display("field"),
                    field.span.clone(),
                    first_node_id.clone(),
                    first_span,
                ));
            } else {
                seen_fields.insert(
                    field.name.clone(),
                    (field.node_id.display("field"), field.span.clone()),
                );
            }
            let field_expected = expected
                .and_then(|expected| expected.ty.record_field(&field.name))
                .cloned()
                .map(|ty| ExpectedType {
                    ty,
                    source: expected
                        .map_or(ExpectedTypeSource::Unknown, |expected| expected.source),
                    origin_node_id: expected
                        .map_or(field.node_id, |expected| expected.origin_node_id),
                    origin_span: expected.and_then(|expected| expected.origin_span.clone()),
                    origin_message: expected.map_or("Expected type inferred here.", |expected| {
                        expected.origin_message
                    }),
                });
            let actual = self.infer_expr(&field.expr, field_expected.as_ref());
            if let Some(field_expected) = &field_expected {
                self.check_assignable(
                    &field.expr,
                    &field_expected.ty,
                    &actual,
                    field_expected,
                    "assignable",
                );
            }
            actual_fields.push((field.name.clone(), actual));
        }
        if let Some(expected) = expected
            && matches!(expected.ty, Type::Record(_))
        {
            return expected.ty.clone();
        }
        Type::Record(actual_fields)
    }

    pub(super) fn infer_dict(
        &mut self,
        expr: &Expr,
        entries: &[DictEntry],
        expected: Option<&ExpectedType>,
    ) -> Type {
        if entries.is_empty()
            && let Some(expected) = expected
            && expected.ty.dict_parts().is_some()
            && type_contains_unknown(&expected.ty)
        {
            self.push_ambiguous_empty_collection_type(
                expr.node_id,
                expr.span.clone(),
                "Dict",
                &expected.ty,
            );
        }
        let (expected_key, expected_value) = expected
            .and_then(|expected| expected.ty.dict_parts())
            .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
                (key.clone(), value.clone())
            });
        let key_expected = ExpectedType {
            ty: expected_key.clone(),
            source: expected.map_or(ExpectedTypeSource::Unknown, |expected| expected.source),
            origin_node_id: expected.map_or(expr.node_id, |expected| expected.origin_node_id),
            origin_span: expected.and_then(|expected| expected.origin_span.clone()),
            origin_message: expected.map_or("Expected type inferred here.", |expected| {
                expected.origin_message
            }),
        };
        let value_expected = ExpectedType {
            ty: expected_value.clone(),
            source: expected.map_or(ExpectedTypeSource::Unknown, |expected| expected.source),
            origin_node_id: expected.map_or(expr.node_id, |expected| expected.origin_node_id),
            origin_span: expected.and_then(|expected| expected.origin_span.clone()),
            origin_message: expected.map_or("Expected type inferred here.", |expected| {
                expected.origin_message
            }),
        };
        let mut key_type = expected_key;
        let mut value_type = expected_value;
        for entry in entries {
            let actual_key = self.infer_expr(&entry.key, Some(&key_expected));
            self.check_assignable(
                &entry.key,
                &key_expected.ty,
                &actual_key,
                &key_expected,
                "assignable",
            );
            if key_type == Type::Unknown {
                key_type = actual_key;
            }
            let actual_value = self.infer_expr(&entry.value, Some(&value_expected));
            self.check_assignable(
                &entry.value,
                &value_expected.ty,
                &actual_value,
                &value_expected,
                "assignable",
            );
            if value_type == Type::Unknown {
                value_type = actual_value;
            }
        }
        Type::dict(key_type, value_type)
    }

    pub(super) fn infer_try(
        &mut self,
        expr: &Expr,
        inner: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Type {
        let return_result = self
            .function
            .return_type
            .as_deref()
            .and_then(|return_type| parse_type_annotation(return_type).ok())
            .and_then(|return_type| {
                adt::result_parts(&return_type).map(|(value, error)| (value.clone(), error.clone()))
            });
        let (value_type, error_type) = match (expected, return_result) {
            (Some(expected), Some((_, error_type))) => (expected.ty.clone(), error_type),
            (Some(expected), None) => (expected.ty.clone(), Type::Unknown),
            (None, Some((value_type, error_type))) => (value_type, error_type),
            (None, None) => (Type::Unknown, Type::Unknown),
        };
        let inner_expected = ExpectedType {
            ty: adt::result_type(value_type.clone(), error_type),
            source: ExpectedTypeSource::Inferred,
            origin_node_id: expected.map_or(expr.node_id, |expected| expected.origin_node_id),
            origin_span: expected.and_then(|expected| expected.origin_span.clone()),
            origin_message: expected.map_or(
                "Result propagation expected type inferred here.",
                |expected| expected.origin_message,
            ),
        };
        let actual = self.infer_expr(inner, Some(&inner_expected));
        self.check_assignable(
            inner,
            &inner_expected.ty,
            &actual,
            &inner_expected,
            "return_value",
        );
        value_type
    }

    pub(super) fn infer_prefix(
        &mut self,
        op: veln_ast::PrefixOp,
        expr: &Expr,
        expected_result: Option<&ExpectedType>,
    ) -> Type {
        let operand_type = match op {
            veln_ast::PrefixOp::Not => Type::bool(),
            veln_ast::PrefixOp::Negate => self.numeric_operand_type(expected_result, &[expr]),
        };
        if operand_type == Type::float()
            && let Some(name) = float_prefix_prelude_name(op)
        {
            return self.infer_builtin_unary_call(name, expr);
        }
        let expected = ExpectedType {
            ty: operand_type,
            source: ExpectedTypeSource::Inferred,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Operator operand type inferred here.",
        };
        let actual = self.infer_expr(expr, Some(&expected));
        self.check_assignable(expr, &expected.ty, &actual, &expected, "operator_operand");
        expected.ty
    }

    pub(super) fn infer_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        expected_result: Option<&ExpectedType>,
    ) -> Type {
        if op == BinaryOp::PipeGreater {
            return self.infer_pipeline(left, right, expected_result);
        }

        let numeric_type = if is_ordering_op(op) {
            self.numeric_operand_type(None, &[left, right])
        } else {
            self.numeric_operand_type(expected_result, &[left, right])
        };
        if numeric_type == Type::float() {
            if let Some(name) = float_comparison_prelude_name(op) {
                return self.infer_builtin_binary_call(name, left, right);
            }
            if let Some(name) = float_arithmetic_prelude_name(op) {
                return self.infer_builtin_binary_call(name, left, right);
            }
        }
        let (operand_type, result_type) = match op {
            BinaryOp::Or | BinaryOp::And => (Type::bool(), Type::bool()),
            BinaryOp::Equal | BinaryOp::NotEqual => (Type::Unknown, Type::bool()),
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                (numeric_type, Type::bool())
            }
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                (numeric_type.clone(), numeric_type)
            }
            BinaryOp::PipeGreater => unreachable!("pipeline handled before binary operators"),
        };
        let expected = ExpectedType {
            ty: operand_type,
            source: ExpectedTypeSource::Inferred,
            origin_node_id: left.node_id,
            origin_span: Some(left.span.clone()),
            origin_message: "Operator operand type inferred here.",
        };
        let actual_left = self.infer_expr(left, Some(&expected));
        self.check_assignable(
            left,
            &expected.ty,
            &actual_left,
            &expected,
            "operator_operand",
        );
        let actual_right = self.infer_expr(right, Some(&expected));
        self.check_assignable(
            right,
            &expected.ty,
            &actual_right,
            &expected,
            "operator_operand",
        );
        result_type
    }

    pub(super) fn infer_pipeline(
        &mut self,
        left: &Expr,
        right: &Expr,
        expected_result: Option<&ExpectedType>,
    ) -> Type {
        let ExprKind::Call { callee, args } = &right.kind else {
            self.infer_expr(left, None);
            self.infer_expr(right, None);
            self.diagnostics.push(Diagnostic::new(
                "type.pipeline_target",
                Severity::Error,
                DiagnosticKind::Type,
                "pipeline target is not a call",
                Some(right.span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("type")),
                    ("node_id", JsonValue::string(right.node_id.display("expr"))),
                    ("expected", JsonValue::string("call")),
                    ("actual", JsonValue::string("expression")),
                    ("constraint", JsonValue::string("pipeline_target")),
                ]),
            ));
            return Type::Unknown;
        };
        if !matches!(callee.kind, ExprKind::NamePath(_)) {
            self.infer_expr(left, None);
            self.infer_expr(right, expected_result);
            self.diagnostics.push(Diagnostic::new(
                "type.pipeline_target",
                Severity::Error,
                DiagnosticKind::Type,
                "pipeline target is not a named call",
                Some(right.span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("type")),
                    ("node_id", JsonValue::string(right.node_id.display("expr"))),
                    ("expected", JsonValue::string("named_call")),
                    ("actual", JsonValue::string("call")),
                    ("constraint", JsonValue::string("pipeline_target")),
                ]),
            ));
            return Type::Unknown;
        }

        let mut piped_args = Vec::with_capacity(args.len() + 1);
        piped_args.push(left.clone());
        piped_args.extend(args.iter().cloned());
        self.infer_call(right, callee, &piped_args, expected_result)
    }

    pub(super) fn infer_builtin_unary_call(&mut self, name: &str, arg: &Expr) -> Type {
        let Some((params, return_type)) = prelude_signature(name, None) else {
            return Type::Unknown;
        };
        let Some(param_type) = params.first() else {
            return return_type;
        };
        let expected = ExpectedType {
            ty: param_type.clone(),
            source: ExpectedTypeSource::Inferred,
            origin_node_id: arg.node_id,
            origin_span: Some(arg.span.clone()),
            origin_message: "Builtin operator parameter type inferred here.",
        };
        let actual = self.infer_expr(arg, Some(&expected));
        self.check_numeric_operator_assignable(arg, &expected.ty, &actual, &expected);
        return_type
    }

    pub(super) fn infer_builtin_binary_call(
        &mut self,
        name: &str,
        left: &Expr,
        right: &Expr,
    ) -> Type {
        let Some((params, return_type)) = prelude_signature(name, None) else {
            return Type::Unknown;
        };
        for (arg, param_type) in [left, right].into_iter().zip(params) {
            let expected = ExpectedType {
                ty: param_type,
                source: ExpectedTypeSource::Inferred,
                origin_node_id: arg.node_id,
                origin_span: Some(arg.span.clone()),
                origin_message: "Builtin operator parameter type inferred here.",
            };
            let actual = self.infer_expr(arg, Some(&expected));
            self.check_numeric_operator_assignable(arg, &expected.ty, &actual, &expected);
        }
        return_type
    }

    pub(super) fn check_numeric_operator_assignable(
        &mut self,
        expr: &Expr,
        expected: &Type,
        actual: &Type,
        expected_context: &ExpectedType,
    ) {
        if expected == &Type::float() && actual == &Type::int() {
            return;
        }
        self.check_assignable(expr, expected, actual, expected_context, "operator_operand");
    }

    pub(super) fn check_prelude_argument_assignable(
        &mut self,
        helper_name: &str,
        arg_index: usize,
        arg: &Expr,
        expected: &ExpectedType,
        actual: &Type,
    ) {
        if is_assignable(&expected.ty, actual) {
            return;
        }
        let mut diagnostic = Diagnostic::new(
            "type.mismatch",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "expected `{}`, but found `{}`",
                expected.ty.render(),
                actual.render()
            ),
            Some(arg.span.clone()),
            type_details(
                arg.node_id.display("expr"),
                expected.ty.render(),
                actual.render(),
                expected.source.as_type_source(),
                "inferred_expression",
                "call_argument",
                [
                    self.function.node_id.display("fn"),
                    expected.origin_node_id.display("expr"),
                    arg.node_id.display("expr"),
                ],
            ),
        );
        if helper_name == "vec_map"
            && arg_index == 1
            && function_returns_result(&expected.ty).is_none()
            && function_returns_result(actual).is_some()
        {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("repair_hint")),
                (
                    "message",
                    JsonValue::string("Use `vec_try_map` when the callback returns `Result`."),
                ),
                ("span", span_json(&arg.span)),
            ]));
        }
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn numeric_operand_type(
        &self,
        expected_result: Option<&ExpectedType>,
        operands: &[&Expr],
    ) -> Type {
        if expected_result.is_some_and(|expected| expected.ty == Type::float()) {
            return Type::float();
        }
        if operands.iter().any(|expr| {
            self.shallow_expr_type(expr)
                .is_some_and(|ty| ty == Type::float())
        }) {
            return Type::float();
        }
        Type::int()
    }

    pub(super) fn shallow_expr_type(&self, expr: &Expr) -> Option<Type> {
        match &expr.kind {
            ExprKind::IntLiteral(_) => Some(Type::int()),
            ExprKind::FloatLiteral(_) => Some(Type::float()),
            ExprKind::BoolLiteral(_) => Some(Type::bool()),
            ExprKind::NamePath(segments) => match segments.as_slice() {
                [name] => self
                    .bindings
                    .iter()
                    .rev()
                    .find(|binding| binding.name == *name)
                    .map(|binding| binding.ty.clone())
                    .or_else(|| {
                        self.environment
                            .unqualified_function(name, self.function.module_name.as_deref())
                            .found()
                            .map(|function| function.ty())
                    }),
                _ => None,
            },
            ExprKind::Call { callee, .. } => self
                .call_signature(callee, None, None, None)
                .map(|(_, _, return_type, _)| return_type),
            ExprKind::List(items) => items
                .first()
                .and_then(|first| self.shallow_expr_type(first))
                .map(Type::vec),
            _ => None,
        }
    }

    pub(super) fn check_assignable(
        &mut self,
        expr: &Expr,
        expected: &Type,
        actual: &Type,
        expected_source: &ExpectedType,
        constraint: &'static str,
    ) {
        if is_assignable(expected, actual) {
            return;
        }
        self.diagnostics.push(Diagnostic::new(
            "type.mismatch",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "expected `{}`, but found `{}`",
                expected.render(),
                actual.render()
            ),
            Some(expr.span.clone()),
            type_details(
                expr.node_id.display("expr"),
                expected.render(),
                actual.render(),
                expected_source.source.as_type_source(),
                "inferred_expression",
                constraint,
                [
                    self.function.node_id.display("fn"),
                    expected_source.origin_node_id.display("expr"),
                    expr.node_id.display("expr"),
                ],
            ),
        ));
    }

    pub(super) fn push_invalid_type_annotation(
        &mut self,
        annotation: &str,
        error: &str,
        origin_node_id: NodeId,
        span: SourceSpan,
    ) {
        self.diagnostics.push(Diagnostic::new(
            "type.invalid_annotation",
            Severity::Error,
            DiagnosticKind::Type,
            format!("invalid type annotation `{annotation}`: {error}"),
            Some(span),
            type_details(
                origin_node_id.display("expr"),
                "valid_type",
                annotation,
                "source",
                "source",
                "assignable",
                [
                    self.function.node_id.display("fn"),
                    origin_node_id.display("expr"),
                ],
            ),
        ));
    }

    pub(super) fn push_unresolved_name(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        symbol: &str,
        namespace: &'static str,
    ) {
        if namespace == "value"
            && let Some(primitive) = exact_width_binary_primitive_name(symbol)
        {
            self.diagnostics
                .push(exact_width_schema_primitive_diagnostic(
                    primitive,
                    None,
                    None,
                    node_id.display("name"),
                    span,
                    "value_position",
                ));
            return;
        }
        self.diagnostics.push(Diagnostic::new(
            "name.unresolved",
            Severity::Error,
            DiagnosticKind::Name,
            format!("unresolved {namespace} `{symbol}`"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("name")),
                ("node_id", JsonValue::string(node_id.display("name"))),
                ("symbol", JsonValue::string(symbol)),
                ("namespace", JsonValue::string(namespace)),
                ("resolution_status", JsonValue::string("unresolved")),
                ("candidates", JsonValue::array([])),
            ]),
        ));
    }

    pub(super) fn push_ambiguous_name(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        symbol: &str,
        namespace: &'static str,
    ) {
        self.diagnostics.push(Diagnostic::new(
            "name.ambiguous",
            Severity::Error,
            DiagnosticKind::Name,
            format!("ambiguous {namespace} `{symbol}`"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("name")),
                ("node_id", JsonValue::string(node_id.display("name"))),
                ("symbol", JsonValue::string(symbol)),
                ("namespace", JsonValue::string(namespace)),
                ("resolution_status", JsonValue::string("ambiguous")),
            ]),
        ));
    }

    pub(super) fn push_ambiguous_constructor_type(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        symbol: &str,
        ty: &Type,
    ) {
        self.diagnostics.push(Diagnostic::new(
            "type.inference_ambiguous",
            Severity::Error,
            DiagnosticKind::Type,
            format!("constructor `{symbol}` needs type context"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("type")),
                ("node_id", JsonValue::string(node_id.display("expr"))),
                ("constructor", JsonValue::string(symbol)),
                ("inferred_type", JsonValue::string(ty.render())),
                ("constraint", JsonValue::string("constructor_type_context")),
            ]),
        ));
    }

    pub(super) fn push_ambiguous_empty_collection_type(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        collection: &str,
        ty: &Type,
    ) {
        self.diagnostics.push(Diagnostic::new(
            "type.inference_ambiguous",
            Severity::Error,
            DiagnosticKind::Type,
            format!("empty {collection} literal needs concrete type context"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("type")),
                ("node_id", JsonValue::string(node_id.display("expr"))),
                ("collection", JsonValue::string(collection)),
                ("inferred_type", JsonValue::string(ty.render())),
                (
                    "constraint",
                    JsonValue::string("empty_collection_type_context"),
                ),
            ]),
        ));
    }

    pub(super) fn hole_constraints(
        &self,
        satisfy: Option<&SatisfyClause>,
        expected: Option<&Type>,
    ) -> Vec<JsonValue> {
        let mut constraints = self
            .function
            .contracts
            .iter()
            .map(|contract| {
                JsonValue::object([
                    ("kind", JsonValue::string("contract")),
                    (
                        "clause",
                        JsonValue::string(contract_kind_text(contract.kind)),
                    ),
                    ("text", JsonValue::string(contract.text.clone())),
                    ("validation_status", JsonValue::string("valid_unknown")),
                    (
                        "source_node_id",
                        JsonValue::string(contract.node_id.display("contract")),
                    ),
                ])
            })
            .collect::<Vec<_>>();
        if let Some(satisfy) = satisfy {
            let repair_status = expected
                .and_then(|expected| self.satisfy_repair_constraint(satisfy, expected))
                .filter(|constraint| self.constraint_has_assignable_candidate(expected, constraint))
                .map_or(SATISFY_STATUS_BLOCKED_UNTIL_DISCHARGED, |_| {
                    SATISFY_STATUS_STATICALLY_SATISFIED
                });
            constraints.push(JsonValue::object([
                ("kind", JsonValue::string("satisfy")),
                ("text", JsonValue::string(satisfy.predicate.clone())),
                (
                    "candidate_binding",
                    satisfy
                        .candidate
                        .as_ref()
                        .map_or(JsonValue::Null, JsonValue::string),
                ),
                ("validation_status", JsonValue::string("valid_unknown")),
                ("repair_status", JsonValue::string(repair_status)),
            ]));
        }
        constraints
    }

    pub(super) fn constraint_has_assignable_candidate(
        &self,
        expected: Option<&Type>,
        constraint: &SatisfyRepairConstraint,
    ) -> bool {
        let Some(expected) = expected.filter(|expected| **expected != Type::Unknown) else {
            return false;
        };
        self.bindings.iter().any(|binding| {
            is_assignable(expected, &binding.ty)
                && constraint.reason_for(binding.name.as_str()).is_some()
        })
    }

    pub(super) fn candidate_queries(
        &self,
        expected: Option<&Type>,
        hole: &Expr,
        satisfy: Option<&SatisfyClause>,
    ) -> Vec<JsonValue> {
        let Some(expected) = expected.filter(|expected| **expected != Type::Unknown) else {
            return Vec::new();
        };
        let argument_types = self
            .bindings
            .iter()
            .map(|binding| binding.ty.render())
            .collect::<Vec<_>>()
            .join(", ");
        let repair_constraint =
            satisfy.and_then(|satisfy| self.satisfy_repair_constraint(satisfy, expected));
        let ranked_candidates =
            self.ranked_symbol_candidates(expected, hole, repair_constraint.as_ref());
        let mut query = vec![
            ("kind", JsonValue::string("symbol")),
            (
                "candidate_status",
                JsonValue::string(CANDIDATE_STATUS_QUERY_ONLY),
            ),
            (
                "application_policy",
                JsonValue::string(APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED),
            ),
            (
                "query",
                JsonValue::string(format!("fn({argument_types}) -> {}", expected.render())),
            ),
        ];
        if let Some(satisfy) = satisfy {
            query.push((
                "satisfy_predicate",
                JsonValue::string(satisfy.predicate.clone()),
            ));
            query.push((
                "satisfy_candidate_binding",
                satisfy
                    .candidate
                    .as_ref()
                    .map_or(JsonValue::Null, JsonValue::string),
            ));
        }
        if !ranked_candidates.is_empty() {
            query.push(("candidates", JsonValue::array(ranked_candidates)));
        }
        vec![JsonValue::object(query)]
    }

    pub(super) fn ranked_symbol_candidates(
        &self,
        expected: &Type,
        hole: &Expr,
        satisfy: Option<&SatisfyRepairConstraint>,
    ) -> Vec<JsonValue> {
        let mut candidates = self
            .bindings
            .iter()
            .rev()
            .enumerate()
            .filter(|(_, binding)| is_assignable(expected, &binding.ty))
            .map(|(distance, binding)| {
                let score = if binding.ty == *expected { 0 } else { 1 };
                (score, distance, binding)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then(left.1.cmp(&right.1))
                .then(left.2.name.cmp(&right.2.name))
        });
        candidates
            .into_iter()
            .enumerate()
            .filter_map(|(sorted_index, candidate)| {
                let static_satisfy =
                    satisfy.and_then(|satisfy| satisfy.reason_for(candidate.2.name.as_str()));
                (sorted_index < 5 || static_satisfy.is_some())
                    .then_some((candidate, static_satisfy))
            })
            .enumerate()
            .map(|(index, ((score, _, binding), static_satisfy))| {
                let rank = index + 1;
                let reason = if let Some(reason) = static_satisfy {
                    reason
                } else if score == 0 {
                    "exact_type_match"
                } else {
                    "assignable_type_match"
                };
                let policy = application_policy(static_satisfy.is_some());
                let satisfy_status =
                    candidate_satisfy_status(satisfy.is_some(), static_satisfy.is_some());
                let mut candidate = vec![
                    ("candidate_id", JsonValue::string(format!("symbol-{rank}"))),
                    ("name", JsonValue::string(binding.name.clone())),
                    ("type", JsonValue::string(binding.ty.render())),
                    ("rank", JsonValue::Number(rank as i64)),
                    ("reason", JsonValue::string(reason)),
                    ("application_policy", JsonValue::string(policy)),
                    (
                        "edits",
                        JsonValue::array([JsonValue::object([
                            ("kind", JsonValue::string("replace")),
                            ("span", span_json(&hole.span)),
                            ("replacement", JsonValue::string(binding.name.clone())),
                        ])]),
                    ),
                    (
                        "target",
                        JsonValue::object([
                            ("node_id", JsonValue::string(hole.node_id.display("hole"))),
                            ("span", span_json(&hole.span)),
                        ]),
                    ),
                    (
                        "edit_summary",
                        JsonValue::string(format!("Replace hole with `{}`", binding.name)),
                    ),
                    (
                        "evidence",
                        candidate_evidence(expected, &binding.ty, rank, reason, satisfy_status),
                    ),
                    ("known_limits", candidate_known_limits(satisfy_status)),
                    (
                        "blocking_obligations",
                        candidate_blocking_obligations(policy, satisfy_status),
                    ),
                    (
                        "verification_hint",
                        JsonValue::object([
                            (
                                "command",
                                JsonValue::string(format!(
                                    "veln check --json {}",
                                    hole.span.file.as_str()
                                )),
                            ),
                            ("scope", JsonValue::string("after_applying_candidate_edit")),
                        ]),
                    ),
                    (
                        "application_status",
                        JsonValue::string(APPLICATION_STATUS_UNAPPLIED),
                    ),
                ];
                if let Some(satisfy_status) = satisfy_status {
                    candidate.push(("satisfy_status", JsonValue::string(satisfy_status)));
                }
                JsonValue::object(candidate)
            })
            .collect()
    }

    pub(super) fn satisfy_repair_constraint(
        &self,
        satisfy: &SatisfyClause,
        expected: &Type,
    ) -> Option<SatisfyRepairConstraint> {
        let allow_static_truth = self.valid_static_satisfy_predicate(satisfy, expected);
        let direct_constraint = SatisfyRepairConstraint::from_satisfy(satisfy, allow_static_truth);
        if direct_constraint
            .as_ref()
            .is_some_and(SatisfyRepairConstraint::allows_any_binding)
        {
            return direct_constraint;
        }
        let candidate = satisfy.candidate.as_ref()?;
        let static_allowed_bindings = if allow_static_truth {
            self.bindings
                .iter()
                .filter(|binding| {
                    let replaced = replace_identifier(&satisfy.predicate, candidate, &binding.name);
                    predicate_is_statically_true_with_literal_bounds(&replaced)
                })
                .map(|binding| SatisfyAllowedBinding {
                    name: binding.name.clone(),
                    reason: "satisfy_tautology",
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let required_predicates = self
            .function
            .contracts
            .iter()
            .filter(|contract| contract.kind != ContractKind::Ensure)
            .filter(|contract| {
                matches!(
                    self.validate_contract_predicate(contract.kind, &contract.text),
                    ContractValidation::Valid
                )
            })
            .map(|contract| contract.text.clone())
            .collect::<Vec<_>>();
        if required_predicates.is_empty() {
            if !static_allowed_bindings.is_empty() {
                let Some(mut constraint) = direct_constraint else {
                    return Some(SatisfyRepairConstraint {
                        allowed_bindings: Some(static_allowed_bindings),
                        reason: "satisfy_tautology",
                    });
                };
                constraint.extend_allowed_bindings(static_allowed_bindings);
                return Some(constraint);
            }
            return direct_constraint;
        }
        let require_allowed_bindings = self
            .bindings
            .iter()
            .filter(|binding| {
                let replaced = replace_identifier(&satisfy.predicate, candidate, &binding.name);
                predicate_guaranteed_by_required_predicates(&replaced, &required_predicates)
                    || (binding.ty == Type::int()
                        && int_successor_predicate_guaranteed_by_required_predicates(
                            &replaced,
                            &required_predicates,
                        ))
            })
            .map(|binding| SatisfyAllowedBinding {
                name: binding.name.clone(),
                reason: "satisfy_require_match",
            })
            .collect::<Vec<_>>();
        let Some(mut constraint) = direct_constraint else {
            let mut allowed_bindings = static_allowed_bindings;
            allowed_bindings.extend(require_allowed_bindings);
            return (!allowed_bindings.is_empty()).then_some(SatisfyRepairConstraint {
                allowed_bindings: Some(allowed_bindings),
                reason: "satisfy_require_match",
            });
        };
        constraint.extend_allowed_bindings(static_allowed_bindings);
        constraint.extend_allowed_bindings(require_allowed_bindings);
        Some(constraint)
    }

    pub(super) fn valid_static_satisfy_predicate(
        &self,
        satisfy: &SatisfyClause,
        expected: &Type,
    ) -> bool {
        let Some(candidate) = satisfy.candidate.as_ref() else {
            return false;
        };
        let mut predicate_bindings = self.bindings.clone();
        predicate_bindings.push(Binding {
            name: candidate.clone(),
            ty: expected.clone(),
        });
        matches!(
            self.validate_predicate_with_bindings(&satisfy.predicate, &predicate_bindings),
            ContractValidation::Valid
        )
    }
}

fn prelude_input_arg<'a>(args: &'a [Expr], helper_name: &str) -> Option<&'a Expr> {
    if helper_name == "vec_try_map_with" {
        args.get(1)
    } else {
        args.first()
    }
}

fn known_concurrency_type_arg_overflow(segments: &[String], type_args: Option<&[String]>) -> bool {
    let Some(type_args) = type_args else {
        return false;
    };
    let limit = match segments {
        [module, name] if module == "task" && name == "spawn_with" => 2,
        [module, _] if module == "channel" || module == "task" => 1,
        _ => return false,
    };
    if type_args.len() <= limit {
        return false;
    }
    qualified_symbol(segments).is_some_and(|symbol| symbol.effects.contains(&"concurrency"))
}
