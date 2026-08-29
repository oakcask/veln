use super::*;

impl<'a> FunctionChecker<'a> {
    pub(super) fn validate_contract_predicate(
        &self,
        kind: ContractKind,
        predicate: &str,
    ) -> ContractValidation {
        self.validate_predicate_with_bindings(predicate, &self.contract_bindings(kind))
    }

    pub(in crate::analysis) fn validate_predicate_with_bindings(
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
        if trimmed.contains("perform ") {
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
                if signature.module_name.as_deref() == Some("std::prelude")
                    && let Some((params, return_type)) = prelude_signature(&signature.name, None)
                {
                    return (params, return_type, signature.effects.clone());
                }
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
        if veln_literals::parse_integer_literal(trimmed).is_ok() {
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
                    .function_path_for_value(&segments, self.function.module_name.as_deref())
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
                        .is_some_and(|binding| {
                            binding.name == name && valid_value_binding_name(&binding.name)
                        })
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
            && valid_value_binding_name(&result_binding.name)
        {
            bindings.push(Binding::new(
                result_binding.name.clone(),
                self.function
                    .return_type
                    .as_deref()
                    .and_then(|return_type| parse_type_annotation(return_type).ok())
                    .map(|ty| {
                        self.environment
                            .canonicalize_type_annotation(ty, self.function.module_name.as_deref())
                    })
                    .unwrap_or(Type::Unknown),
            ));
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
                ty: self
                    .environment
                    .canonicalize_type_annotation(ty, self.function.module_name.as_deref()),
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
            .map(|ty| {
                self.environment
                    .canonicalize_type_annotation(ty, self.function.module_name.as_deref())
            })
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
                            .function_for(self.function)
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
}
