use super::*;

impl<'a> CoreLowerer<'a> {
    pub(super) fn lower_field_access(&mut self, expr: &Expr, base: &Expr, field: &str) -> CoreExpr {
        let base = self.lower_expr(base, None);
        let ty = base
            .ty
            .record_field(field)
            .cloned()
            .unwrap_or(CoreType::Unknown);
        self.core_expr(
            expr,
            ty,
            CoreExprKind::FieldAccess {
                base: Box::new(base),
                field: field.to_string(),
            },
        )
    }

    pub(super) fn lower_try(
        &mut self,
        expr: &Expr,
        inner: &Expr,
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let return_result = self
            .function
            .return_type
            .as_deref()
            .and_then(|return_type| parse_type_annotation(return_type).ok())
            .map(|ty| core_type(&ty))
            .and_then(|ty| {
                adt::core_result_parts(&ty).map(|(value, error)| (value.clone(), error.clone()))
            });
        let (value_type, error_type) = match (expected, return_result) {
            (Some(expected), Some((_, error))) => (expected.clone(), error),
            (Some(expected), None) => (expected.clone(), CoreType::Unknown),
            (None, Some((value, error))) => (value, error),
            (None, None) => (CoreType::Unknown, CoreType::Unknown),
        };
        let inner_expected = adt::core_result_type(value_type.clone(), error_type);
        let inner = self.lower_expr(inner, Some(&inner_expected));
        self.core_expr(expr, value_type, CoreExprKind::Try(Box::new(inner)))
    }

    pub(super) fn lower_record(
        &mut self,
        expr: &Expr,
        fields: &[RecordField],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if fields.is_empty()
            && let Some(expected) = expected
            && expected.dict_parts().is_some()
        {
            return self.core_expr(expr, expected.clone(), CoreExprKind::Dict(Vec::new()));
        }
        let fields = fields
            .iter()
            .map(|field| {
                let expected = expected.and_then(|expected| expected.record_field(&field.name));
                let expr = self.lower_expr(&field.expr, expected);
                CoreRecordField {
                    node_id: field.node_id,
                    name: field.name.clone(),
                    span: field.span.clone(),
                    expr,
                }
            })
            .collect::<Vec<_>>();
        let ty = expected.cloned().unwrap_or_else(|| {
            CoreType::Record(
                fields
                    .iter()
                    .map(|field| (field.name.clone(), field.expr.ty.clone()))
                    .collect(),
            )
        });
        self.core_expr(expr, ty, CoreExprKind::Record(fields))
    }

    pub(super) fn lower_dict(
        &mut self,
        expr: &Expr,
        entries: &[DictEntry],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let (key_expected, value_expected) = expected
            .and_then(CoreType::dict_parts)
            .map_or((None, None), |(key, value)| (Some(key), Some(value)));
        let entries = entries
            .iter()
            .map(|entry| CoreDictEntry {
                node_id: entry.node_id,
                key: self.lower_expr(&entry.key, key_expected),
                value: self.lower_expr(&entry.value, value_expected),
                span: entry.span.clone(),
            })
            .collect::<Vec<_>>();
        let ty = expected.cloned().unwrap_or_else(|| {
            let key_type = entries
                .first()
                .map_or(CoreType::Unknown, |entry| entry.key.ty.clone());
            let value_type = entries
                .first()
                .map_or(CoreType::Unknown, |entry| entry.value.ty.clone());
            CoreType::dict(key_type, value_type)
        });
        self.core_expr(expr, ty, CoreExprKind::Dict(entries))
    }

    pub(super) fn lower_list(
        &mut self,
        expr: &Expr,
        items: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let item_expected = expected.and_then(CoreType::vec_part).cloned();
        let items = items
            .iter()
            .map(|item| self.lower_expr(item, item_expected.as_ref()))
            .collect::<Vec<_>>();
        let item_type = item_expected.unwrap_or_else(|| {
            items
                .first()
                .map_or(CoreType::Unknown, |item| item.ty.clone())
        });
        self.core_expr(expr, CoreType::vec(item_type), CoreExprKind::List(items))
    }

    pub(super) fn lower_match(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[MatchArm],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let scrutinee = self.lower_expr(scrutinee, None);
        let mut result_type = expected.cloned().unwrap_or(CoreType::Unknown);
        let mut lowered_arms = Vec::new();
        if arms.is_empty() {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: expr.node_id,
                reason: "empty_match".to_string(),
            });
        }
        for arm in arms {
            let saved_bindings = self.bindings.len();
            for binding in self.pattern_bindings(&arm.pattern, &scrutinee.ty) {
                self.bindings.push(binding);
            }
            let arm_expected = if result_type == CoreType::Unknown {
                None
            } else {
                Some(&result_type)
            };
            let lowered_expr = self.lower_expr(&arm.expr, arm_expected);
            if result_type == CoreType::Unknown {
                result_type = lowered_expr.ty.clone();
            }
            lowered_arms.push(CoreMatchArm {
                node_id: arm.node_id,
                pattern: self.lower_pattern(&arm.pattern, Some(&scrutinee.ty)),
                expr: lowered_expr,
                span: arm.span.clone(),
            });
            self.bindings.truncate(saved_bindings);
        }
        self.core_expr(
            expr,
            result_type,
            CoreExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms: lowered_arms,
            },
        )
    }

    pub(super) fn lower_if(
        &mut self,
        expr: &Expr,
        condition: &Expr,
        then_branch: &Expr,
        else_if_branches: &[IfBranch],
        else_branch: &Expr,
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let mut result_type = expected.cloned().unwrap_or(CoreType::Unknown);
        let mut lowered = self.lower_if_chain(
            IfLoweringTarget {
                node_id: expr.node_id,
                span: &expr.span,
            },
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            &mut result_type,
        );
        lowered.node_id = expr.node_id;
        lowered.span = expr.span.clone();
        lowered.ty = result_type;
        lowered
    }

    pub(super) fn lower_if_chain(
        &mut self,
        target: IfLoweringTarget<'_>,
        condition: &Expr,
        then_branch: &Expr,
        else_if_branches: &[IfBranch],
        else_branch: &Expr,
        result_type: &mut CoreType,
    ) -> CoreExpr {
        let scrutinee = self.lower_expr(condition, None);
        let then_expected = (*result_type != CoreType::Unknown).then(|| result_type.clone());
        let lowered_then = self.lower_expr(then_branch, then_expected.as_ref());
        if *result_type == CoreType::Unknown {
            *result_type = lowered_then.ty.clone();
        }

        let (false_expr, false_span, false_node_id) = if let Some((next_branch, rest)) =
            else_if_branches.split_first()
        {
            (
                self.lower_if_chain(
                    IfLoweringTarget {
                        node_id: next_branch.node_id,
                        span: &next_branch.span,
                    },
                    &next_branch.condition,
                    &next_branch.expr,
                    rest,
                    else_branch,
                    result_type,
                ),
                next_branch.span.clone(),
                next_branch.node_id,
            )
        } else {
            let else_expected = (*result_type != CoreType::Unknown).then(|| result_type.clone());
            let lowered_else = self.lower_expr(else_branch, else_expected.as_ref());
            if *result_type == CoreType::Unknown {
                *result_type = lowered_else.ty.clone();
            }
            (lowered_else, else_branch.span.clone(), else_branch.node_id)
        };

        self.core_expr(
            &Expr {
                node_id: target.node_id,
                kind: ExprKind::Missing,
                span: target.span.clone(),
            },
            result_type.clone(),
            CoreExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms: vec![
                    CoreMatchArm {
                        node_id: then_branch.node_id,
                        pattern: self.lower_bool_pattern(
                            true,
                            then_branch.node_id,
                            &then_branch.span,
                        ),
                        expr: lowered_then,
                        span: then_branch.span.clone(),
                    },
                    CoreMatchArm {
                        node_id: false_node_id,
                        pattern: self.lower_bool_pattern(false, false_node_id, &false_span),
                        expr: false_expr,
                        span: false_span,
                    },
                ],
            },
        )
    }

    pub(super) fn lower_bool_pattern(
        &self,
        value: bool,
        node_id: veln_ast::NodeId,
        span: &veln_source::SourceSpan,
    ) -> CorePattern {
        CorePattern {
            node_id,
            kind: CorePatternKind::BoolLiteral(value),
            span: span.clone(),
        }
    }

    pub(super) fn pattern_bindings(
        &self,
        pattern: &Pattern,
        scrutinee_type: &CoreType,
    ) -> Vec<CoreBinding> {
        match &pattern.kind {
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => Vec::new(),
            PatternKind::Binding(name) => vec![CoreBinding {
                name: name.clone(),
                ty: scrutinee_type.clone(),
            }],
            PatternKind::Record(fields) => fields
                .iter()
                .flat_map(|field| {
                    let field_type = scrutinee_type
                        .record_field(&field.name)
                        .unwrap_or(&CoreType::Unknown);
                    self.pattern_bindings(&field.pattern, field_type)
                })
                .collect(),
            PatternKind::Constructor { name, args, .. } => {
                let Some(descriptor) = self
                    .environment
                    .adts
                    .descriptor_for_core_type(scrutinee_type)
                else {
                    return args
                        .iter()
                        .flat_map(|pattern| self.pattern_bindings(pattern, &CoreType::Unknown))
                        .collect();
                };
                let Some(constructor) = self.environment.adts.constructor_for_descriptor(
                    name,
                    descriptor,
                    self.function.module_name.as_deref(),
                    &self.environment.uses,
                ) else {
                    return args
                        .iter()
                        .flat_map(|pattern| self.pattern_bindings(pattern, &CoreType::Unknown))
                        .collect();
                };
                args.iter()
                    .enumerate()
                    .flat_map(|(index, pattern)| {
                        let ty = adt::core_payload_type(scrutinee_type, constructor, index)
                            .unwrap_or(CoreType::Unknown);
                        self.pattern_bindings(pattern, &ty)
                    })
                    .collect()
            }
        }
    }

    pub(super) fn lower_pattern(
        &self,
        pattern: &Pattern,
        scrutinee_type: Option<&CoreType>,
    ) -> CorePattern {
        CorePattern {
            node_id: pattern.node_id,
            kind: match &pattern.kind {
                PatternKind::Wildcard => CorePatternKind::Wildcard,
                PatternKind::Binding(name) => CorePatternKind::Binding(name.clone()),
                PatternKind::StringLiteral(value) => CorePatternKind::StringLiteral(value.clone()),
                PatternKind::IntLiteral(value) => CorePatternKind::IntLiteral(
                    parse_integer_literal(value)
                        .map(|literal| literal.value.to_string())
                        .unwrap_or_else(|_| value.clone()),
                ),
                PatternKind::FloatLiteral(value) => CorePatternKind::FloatLiteral(value.clone()),
                PatternKind::BoolLiteral(value) => CorePatternKind::BoolLiteral(*value),
                PatternKind::Unit => CorePatternKind::Unit,
                PatternKind::Record(fields) => CorePatternKind::Record(
                    fields
                        .iter()
                        .map(|field| CorePatternField {
                            node_id: field.node_id,
                            name: field.name.clone(),
                            pattern: self.lower_pattern(
                                &field.pattern,
                                scrutinee_type.and_then(|ty| ty.record_field(&field.name)),
                            ),
                            span: field.span.clone(),
                        })
                        .collect(),
                ),
                PatternKind::Constructor { name, args, .. } => {
                    let constructor = scrutinee_type
                        .and_then(|ty| self.environment.adts.descriptor_for_core_type(ty))
                        .and_then(|descriptor| {
                            self.environment.adts.constructor_for_descriptor(
                                name,
                                descriptor,
                                self.function.module_name.as_deref(),
                                &self.environment.uses,
                            )
                        });
                    CorePatternKind::Constructor {
                        name: constructor
                            .map(|constructor| {
                                vec![
                                    constructor.descriptor.type_name.clone(),
                                    constructor.variant.name.clone(),
                                ]
                            })
                            .unwrap_or_else(|| self.canonical_constructor_name(name)),
                        args: args
                            .iter()
                            .enumerate()
                            .map(|(index, arg)| {
                                let payload_type = scrutinee_type.and_then(|ty| {
                                    constructor.and_then(|constructor| {
                                        adt::core_payload_type(ty, constructor, index)
                                    })
                                });
                                self.lower_pattern(arg, payload_type.as_ref())
                            })
                            .collect(),
                    }
                }
            },
            span: pattern.span.clone(),
        }
    }

    pub(super) fn canonical_constructor_name(&self, name: &[String]) -> Vec<String> {
        match self.environment.adts.constructor(
            name,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        ) {
            ConstructorLookup::Found(constructor) => vec![
                constructor.descriptor.type_name.clone(),
                constructor.variant.name.clone(),
            ],
            ConstructorLookup::Ambiguous | ConstructorLookup::Missing => name.to_vec(),
        }
    }

    pub(super) fn core_call_signature(
        &self,
        callee: &Expr,
        expected: Option<&CoreType>,
        arg_count: Option<usize>,
    ) -> Option<CoreCallSignature> {
        let bindings = self
            .bindings
            .iter()
            .map(|binding| crate::call_resolution::CoreBinding {
                name: &binding.name,
                ty: &binding.ty,
            })
            .collect::<Vec<_>>();
        crate::call_resolution::core_call_signature(
            callee,
            expected,
            arg_count,
            &bindings,
            self.environment,
            self.function.module_name.as_deref(),
        )
    }

    pub(super) fn core_expr(&self, expr: &Expr, ty: CoreType, kind: CoreExprKind) -> CoreExpr {
        CoreExpr {
            node_id: expr.node_id,
            ty,
            kind,
            span: expr.span.clone(),
        }
    }
}
