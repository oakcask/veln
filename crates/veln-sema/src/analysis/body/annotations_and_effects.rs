use super::*;

impl<'a> FunctionChecker<'a> {
    pub(super) fn check_parameter_annotation(
        &mut self,
        param: &veln_ast::Param,
        index: usize,
        variadic_count: usize,
        signature: Option<&FunctionSignature>,
    ) {
        let private_omitted_parameter = self.function.visibility == Visibility::Private
            && self.function.kind == FunctionKind::Function
            && parameter_annotation_is_omitted(param);
        let inferred_private_param = self.inferred_private_parameter_type(param);
        let declared_type = param
            .ty
            .as_deref()
            .filter(|annotation| {
                !(param.is_variadic && annotation.is_empty() && private_omitted_parameter)
            })
            .and_then(|annotation| {
                self.parse_annotation(
                    annotation,
                    param.node_id,
                    &param.span,
                    ExpectedTypeSource::DeclaredParameter,
                    "Parameter type declared here.",
                )
            });

        self.check_variadic_parameter_shape(param, variadic_count, private_omitted_parameter);
        let binding_type = signature
            .and_then(|signature| signature.params.get(index).cloned())
            .or(inferred_private_param.filter(|ty| !type_contains_unknown(ty)))
            .unwrap_or_else(|| {
                declared_type.map_or(Type::Unknown, |expected| {
                    if param.is_variadic {
                        Type::named("List", vec![expected.ty])
                    } else {
                        expected.ty
                    }
                })
            });
        self.admit_value_binding(
            &param.name,
            binding_type,
            param.node_id.display("param"),
            param.span.clone(),
            "parameter",
        );
    }

    pub(super) fn check_variadic_parameter_shape(
        &mut self,
        param: &veln_ast::Param,
        variadic_count: usize,
        private_omitted_parameter: bool,
    ) {
        if !param.is_variadic {
            return;
        }
        if param.ty.as_deref().is_none_or(str::is_empty) && !private_omitted_parameter {
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

    pub(super) fn check_return_annotation(&mut self) {
        if let Some(return_type) = &self.function.return_type {
            self.parse_annotation(
                return_type,
                self.function.node_id,
                &self.function.span,
                ExpectedTypeSource::DeclaredReturn,
                "Return type declared here.",
            );
        }
    }

    pub(super) fn check_result_binding_name(&mut self) {
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

    pub(super) fn inferred_private_parameter_type(&self, param: &veln_ast::Param) -> Option<Type> {
        if self.function.visibility != Visibility::Private
            || self.function.kind != FunctionKind::Function
            || !parameter_annotation_is_omitted(param)
        {
            return None;
        }
        let signature = self.environment.function_for(self.function)?;
        if param.is_variadic {
            return signature
                .variadic
                .clone()
                .map(|ty| Type::named("List", vec![ty]));
        }
        let index = self
            .function
            .params
            .iter()
            .take_while(|candidate| candidate.node_id != param.node_id)
            .filter(|candidate| !candidate.is_variadic)
            .count();
        signature.params.get(index).cloned()
    }

    pub(super) fn push_variadic_parameter_diagnostic(
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

    pub(super) fn admit_value_binding(
        &mut self,
        name: &str,
        ty: Type,
        node_id: String,
        span: SourceSpan,
        declaration_kind: &'static str,
    ) {
        if invalid_value_binding_name(name) {
            self.invalid_binding_recoveries
                .push(InvalidBindingRecovery {
                    name: name.to_string(),
                    ty,
                });
            return;
        }
        if !self.declare_local_name(name, node_id, span, declaration_kind) {
            return;
        }
        self.bindings.push(Binding::new(name.to_string(), ty));
    }

    pub(in crate::analysis) fn admit_value_binding_without_duplicate_diagnostic(
        &mut self,
        name: &str,
        ty: Type,
    ) {
        if invalid_value_binding_name(name) {
            self.invalid_binding_recoveries
                .push(InvalidBindingRecovery {
                    name: name.to_string(),
                    ty,
                });
            return;
        }
        self.bindings.push(Binding::new(name.to_string(), ty));
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
        let Some(boundary) = EffectBoundary::for_function(self.function) else {
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
        let declared_effects = self.declared_boundary_effects();
        let inferred_effects = self.inferred_boundary_effects();

        for effect in &inferred_effects {
            if !declared_effects.contains(effect) {
                let diagnostic = self.missing_effect_diagnostic(
                    &boundary,
                    effect,
                    &declared_effects,
                    &inferred_effects,
                );
                self.diagnostics.push(diagnostic);
            }
        }
    }

    pub(super) fn declared_boundary_effects(&self) -> Vec<String> {
        self.function
            .effects
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|effect| {
                if effect.starts_with("...") {
                    return effect.clone();
                }
                let segments = effect.split("::").map(str::to_string).collect::<Vec<_>>();
                self.environment
                    .user_effect_path(&segments, self.function.module_name.as_deref())
                    .map(|effect| effect.qualified_name.clone())
                    .unwrap_or_else(|| effect.clone())
            })
            .collect()
    }

    pub(super) fn inferred_boundary_effects(&self) -> Vec<String> {
        let mut inferred_effects = Vec::<String>::new();
        for effect_use in &self.inferred_effects {
            if !inferred_effects.contains(&effect_use.effect) {
                inferred_effects.push(effect_use.effect.clone());
            }
        }
        inferred_effects
    }

    pub(super) fn missing_effect_diagnostic(
        &self,
        boundary: &EffectBoundary,
        effect: &str,
        declared_effects: &[String],
        inferred_effects: &[String],
    ) -> Diagnostic {
        let provenance = self
            .inferred_effects
            .iter()
            .filter(|effect_use| effect_use.effect == effect)
            .take(3)
            .cloned()
            .collect::<Vec<_>>();
        let matching_path_count = self
            .inferred_effects
            .iter()
            .filter(|effect_use| effect_use.effect == effect)
            .count();
        let omitted_path_count = matching_path_count.saturating_sub(provenance.len());
        let mut diagnostic = Diagnostic::new(
            boundary.diagnostic_id,
            Severity::Error,
            DiagnosticKind::Effect,
            format!("{} uses undeclared effect `{effect}`", boundary.subject),
            Some(self.function.span.clone()),
            effect_missing_public_details(
                self.function
                    .node_id
                    .display(self.function.kind.node_prefix()),
                self.function.name.as_deref().unwrap_or("<missing>"),
                &self.function.span,
                effect,
                boundary.kind,
                declared_effects,
                inferred_effects,
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
        diagnostic
    }
}
