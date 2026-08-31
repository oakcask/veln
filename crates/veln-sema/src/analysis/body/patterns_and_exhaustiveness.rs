use super::*;

impl<'a> FunctionChecker<'a> {
    pub(super) fn check_match_exhaustiveness(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        scrutinee_type: &Type,
        arms: &[MatchArm],
    ) {
        let Some(domain) = MatchDomain::from_type(
            scrutinee_type,
            self.environment,
            self.function.module_name.as_deref(),
        ) else {
            return;
        };
        let mut covered = Vec::new();
        let mut invalid_recovery_covered = Vec::new();
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
            invalid_recovery_covered.extend(invalid_qualified_constructor_recovery_cases(
                &arm.pattern,
                &domain,
                scrutinee_type,
                self.environment,
                self.function.module_name.as_deref(),
            ));
            for case in coverage.cases {
                if !covered.contains(&case) {
                    covered.push(case.clone());
                    proving_arms.push((case, arm.pattern.span.clone()));
                }
            }
        }

        let cases = domain.cases(
            scrutinee_type,
            self.environment,
            self.function.module_name.as_deref(),
        );
        let Some(missing_case) = cases
            .iter()
            .find(|case| !covered.contains(case) && !invalid_recovery_covered.contains(case))
            .cloned()
        else {
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
        self.pattern_bindings_with_recovery(pattern, scrutinee_type, false)
    }

    pub(super) fn let_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        scrutinee_type: &Type,
    ) -> Vec<PatternBinding> {
        self.pattern_bindings_with_recovery(pattern, scrutinee_type, true)
    }

    pub(super) fn pattern_bindings_with_recovery(
        &mut self,
        pattern: &Pattern,
        scrutinee_type: &Type,
        recover_unknown_bare_constructor: bool,
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
            PatternKind::Record(fields) => self.record_pattern_bindings(
                pattern,
                fields,
                scrutinee_type,
                recover_unknown_bare_constructor,
            ),
            PatternKind::Constructor { name, args, .. } => {
                if invalid_qualified_constructor_pattern(name) {
                    self.report_invalid_qualified_constructor_pattern_mismatch(
                        pattern,
                        name,
                        scrutinee_type,
                    );
                    return self.unknown_pattern_bindings(args);
                }
                if recover_unknown_bare_constructor
                    && let [binding] = name.as_slice()
                    && args.is_empty()
                    && invalid_value_binding_name(binding)
                    && !self.constructor_pattern_resolves(name)
                {
                    return vec![PatternBinding {
                        name: binding.clone(),
                        ty: scrutinee_type.clone(),
                        node_id: pattern.node_id,
                        span: pattern.span.clone(),
                    }];
                }
                self.constructor_pattern_bindings(pattern, name, args, scrutinee_type)
            }
        }
    }

    pub(super) fn record_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        fields: &[PatternField],
        scrutinee_type: &Type,
        recover_unknown_bare_constructor: bool,
    ) -> Vec<PatternBinding> {
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
            let field_type = self.record_pattern_field_type(pattern, field, scrutinee_type);
            bindings.extend(self.pattern_bindings_with_recovery(
                &field.pattern,
                &field_type,
                recover_unknown_bare_constructor,
            ));
        }
        bindings
    }

    pub(super) fn constructor_pattern_resolves(&self, name: &[String]) -> bool {
        matches!(
            self.environment.adts.constructor(
                name,
                self.function.module_name.as_deref(),
                &self.environment.uses,
            ),
            ConstructorLookup::Found(_) | ConstructorLookup::Ambiguous
        )
    }

    pub(super) fn record_pattern_field_type(
        &mut self,
        pattern: &Pattern,
        field: &PatternField,
        scrutinee_type: &Type,
    ) -> Type {
        if let Some(field_type) = scrutinee_type.record_field(&field.name) {
            return field_type.clone();
        }
        if scrutinee_type != &Type::Unknown {
            self.diagnostics.push(Diagnostic::new(
                "type.field_missing",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "type `{}` has no field `{}`",
                    scrutinee_type.render(),
                    field.name
                ),
                Some(field.span.clone()),
                type_details(
                    field.node_id.display("field"),
                    format!("record field `{}`", field.name),
                    scrutinee_type.render(),
                    "record_pattern",
                    "inferred_expression",
                    "record_pattern",
                    [
                        self.function.node_id.display("fn"),
                        pattern.node_id.display("pattern"),
                    ],
                ),
            ));
        }
        Type::Unknown
    }

    pub(super) fn constructor_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        name: &[String],
        args: &[Pattern],
        scrutinee_type: &Type,
    ) -> Vec<PatternBinding> {
        let Some(descriptor) = self.environment.adts.descriptor_for_type_prefer_module(
            scrutinee_type,
            self.function.module_name.as_deref(),
        ) else {
            return self.unknown_pattern_bindings(args);
        };
        if let Some(constructor) = self.environment.adts.constructor_for_descriptor(
            name,
            descriptor,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        ) {
            return args
                .iter()
                .enumerate()
                .flat_map(|(index, pattern)| {
                    let ty = adt::payload_type(scrutinee_type, constructor, index)
                        .unwrap_or(Type::Unknown);
                    self.pattern_bindings(pattern, &ty)
                })
                .collect();
        }
        self.report_constructor_pattern_mismatch(pattern, name, scrutinee_type);
        self.unknown_pattern_bindings(args)
    }

    pub(super) fn unknown_pattern_bindings(&mut self, patterns: &[Pattern]) -> Vec<PatternBinding> {
        patterns
            .iter()
            .flat_map(|pattern| self.pattern_bindings(pattern, &Type::Unknown))
            .collect()
    }

    pub(super) fn report_constructor_pattern_mismatch(
        &mut self,
        pattern: &Pattern,
        name: &[String],
        scrutinee_type: &Type,
    ) {
        let ConstructorLookup::Found(constructor) = self.environment.adts.constructor(
            name,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        ) else {
            return;
        };
        self.report_constructor_type_mismatch(pattern, scrutinee_type, constructor);
    }

    pub(super) fn report_invalid_qualified_constructor_pattern_mismatch(
        &mut self,
        pattern: &Pattern,
        name: &[String],
        scrutinee_type: &Type,
    ) {
        let Some(descriptor) = self.environment.adts.descriptor_for_type_prefer_module(
            scrutinee_type,
            self.function.module_name.as_deref(),
        ) else {
            return;
        };
        let Some(recovered) = initial_uppercase_qualified_constructor_name(name) else {
            return;
        };
        if self
            .environment
            .adts
            .constructor_for_descriptor(
                &recovered,
                descriptor,
                self.function.module_name.as_deref(),
                &self.environment.uses,
            )
            .is_some()
        {
            return;
        }
        let ConstructorLookup::Found(constructor) = self.environment.adts.constructor(
            &recovered,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        ) else {
            return;
        };
        self.report_constructor_type_mismatch(pattern, scrutinee_type, constructor);
    }

    fn report_constructor_type_mismatch(
        &mut self,
        pattern: &Pattern,
        scrutinee_type: &Type,
        constructor: AdtConstructor<'_>,
    ) {
        let actual = adt::constructed_type_from_args(
            constructor,
            &vec![Type::Unknown; constructor.descriptor.type_parameters.len()],
        );
        self.diagnostics.push(Diagnostic::new(
            "type.mismatch",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "expected `{}`, but found `{}`",
                scrutinee_type.render(),
                actual.render()
            ),
            Some(pattern.span.clone()),
            type_details(
                pattern.node_id.display("pattern"),
                scrutinee_type.render(),
                actual.render(),
                "inferred_expression",
                "constructor_pattern",
                "constructor_pattern",
                [
                    self.function.node_id.display("fn"),
                    pattern.node_id.display("pattern"),
                ],
            ),
        ));
    }
}
