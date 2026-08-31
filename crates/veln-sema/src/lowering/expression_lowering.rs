use super::*;

impl<'a> CoreLowerer<'a> {
    pub(super) fn lower_expr(&mut self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
            ExprKind::Missing => self.lower_missing_expr(expr, expected),
            ExprKind::Hole { name, .. } => self.lower_hole_expr(expr, expected, name),
            ExprKind::NamePath { segments, .. } => self.lower_name_path(expr, segments, expected),
            ExprKind::StringLiteral(value) => self.lower_string_literal(expr, value),
            ExprKind::IntLiteral(value) => self.lower_int_literal(expr, value),
            ExprKind::FloatLiteral(value) => self.lower_float_literal(expr, value),
            ExprKind::BoolLiteral(value) => self.lower_bool_literal(expr, *value),
            ExprKind::Unit => self.lower_unit_literal(expr),
            ExprKind::TypeApply { .. } => {
                self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
            }
            ExprKind::Call { callee, args } => self.lower_call(expr, callee, args, expected),
            ExprKind::Perform {
                effect,
                operation,
                args,
                ..
            } => self.lower_perform(expr, effect, operation, args),
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => self.lower_handle(expr, body, handler, args, expected),
            ExprKind::SchemaDecode {
                schema,
                input,
                base,
            } => self.lower_schema_decode(expr, schema, input, base),
            ExprKind::SchemaEncode { schema, value } => {
                self.lower_schema_encode(expr, schema, value)
            }
            ExprKind::FieldAccess { base, field, .. } => self.lower_field_access(expr, base, field),
            ExprKind::Try(inner) => self.lower_try(expr, inner, expected),
            ExprKind::Record(fields) => self.lower_record(expr, fields, expected),
            ExprKind::Dict(entries) => self.lower_dict(expr, entries, expected),
            ExprKind::List(items) => self.lower_list(expr, items, expected),
            ExprKind::Match { scrutinee, arms } => {
                self.lower_match(expr, scrutinee, arms, expected)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => self.lower_if(
                expr,
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                expected,
            ),
            ExprKind::Prefix { op, expr: inner } => self.lower_prefix(expr, *op, inner, expected),
            ExprKind::Binary { op, left, right } => {
                self.lower_binary(expr, *op, left, right, expected)
            }
        }
    }

    pub(super) fn lower_missing_expr(
        &mut self,
        expr: &Expr,
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        self.missing_expression(expr, expected, "missing_expression");
        self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
    }

    pub(super) fn lower_hole_expr(
        &mut self,
        expr: &Expr,
        expected: Option<&CoreType>,
        name: &Option<String>,
    ) -> CoreExpr {
        self.blockers.push(CoreBlocker::Hole {
            node_id: expr.node_id,
        });
        self.core_expr(
            expr,
            expected.cloned().unwrap_or(CoreType::Unknown),
            CoreExprKind::Hole {
                label: name.clone(),
            },
        )
    }

    pub(super) fn lower_string_literal(&self, expr: &Expr, value: &str) -> CoreExpr {
        self.core_expr(
            expr,
            CoreType::string(),
            CoreExprKind::StringLiteral(value.to_string()),
        )
    }

    pub(super) fn lower_int_literal(&self, expr: &Expr, value: &str) -> CoreExpr {
        let value = parse_integer_literal(value)
            .map(|literal| literal.value.to_string())
            .unwrap_or_else(|_| value.to_string());
        self.core_expr(expr, CoreType::int(), CoreExprKind::IntLiteral(value))
    }

    pub(super) fn lower_float_literal(&self, expr: &Expr, value: &str) -> CoreExpr {
        self.core_expr(
            expr,
            CoreType::float(),
            CoreExprKind::FloatLiteral(value.to_string()),
        )
    }

    pub(super) fn lower_bool_literal(&self, expr: &Expr, value: bool) -> CoreExpr {
        self.core_expr(expr, CoreType::bool(), CoreExprKind::BoolLiteral(value))
    }

    pub(super) fn lower_unit_literal(&self, expr: &Expr) -> CoreExpr {
        self.core_expr(expr, CoreType::unit(), CoreExprKind::Unit)
    }

    pub(super) fn lower_prefix(
        &mut self,
        expr: &Expr,
        op: veln_ast::PrefixOp,
        inner: &Expr,
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let expected_operand = match op {
            veln_ast::PrefixOp::Not => CoreType::bool(),
            veln_ast::PrefixOp::Negate => self.numeric_operand_type(expected, &[inner]),
            veln_ast::PrefixOp::BitwiseNot => CoreType::int(),
        };
        if expected_operand == CoreType::float()
            && let Some(name) = float_prefix_prelude_name(op)
        {
            let arg = self.lower_expr(inner, Some(&CoreType::float()));
            return self.core_expr(
                expr,
                CoreType::float(),
                CoreExprKind::Call {
                    target: CoreCallTarget::PreludeBuiltin(name.to_string()),
                    args: vec![arg],
                },
            );
        }
        let lowered = self.lower_expr(inner, Some(&expected_operand));
        self.core_expr(
            expr,
            expected_operand,
            CoreExprKind::Prefix {
                op,
                expr: Box::new(lowered),
            },
        )
    }

    pub(super) fn lower_binary(
        &mut self,
        expr: &Expr,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if op == BinaryOp::PipeGreater {
            return self.lower_pipeline(expr, left, right, expected);
        }

        let numeric_type = self.binary_numeric_operand_type(op, left, right, expected);
        self.lower_float_binary_prelude_call(expr, op, left, right, &numeric_type)
            .unwrap_or_else(|| self.lower_regular_binary(expr, op, left, right, numeric_type))
    }

    pub(super) fn binary_numeric_operand_type(
        &self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        expected: Option<&CoreType>,
    ) -> CoreType {
        if is_ordering_op(op) {
            self.numeric_operand_type(None, &[left, right])
        } else {
            self.numeric_operand_type(expected, &[left, right])
        }
    }

    pub(super) fn lower_float_binary_prelude_call(
        &mut self,
        expr: &Expr,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        numeric_type: &CoreType,
    ) -> Option<CoreExpr> {
        if numeric_type != &CoreType::float() {
            return None;
        }
        let (name, return_type) = float_comparison_prelude_name(op)
            .map(|name| (name, CoreType::bool()))
            .or_else(|| float_arithmetic_prelude_name(op).map(|name| (name, CoreType::float())))?;
        let left = self.lower_expr(left, Some(&CoreType::float()));
        let right = self.lower_expr(right, Some(&CoreType::float()));
        Some(self.core_expr(
            expr,
            return_type,
            CoreExprKind::Call {
                target: CoreCallTarget::PreludeBuiltin(name.to_string()),
                args: vec![left, right],
            },
        ))
    }

    pub(super) fn lower_regular_binary(
        &mut self,
        expr: &Expr,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        numeric_type: CoreType,
    ) -> CoreExpr {
        let (operand, result) = binary_operand_and_result(op, numeric_type);
        let left = self.lower_expr(left, Some(&operand));
        let right = self.lower_expr(right, Some(&operand));
        self.core_expr(
            expr,
            result,
            CoreExprKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
        )
    }

    pub(super) fn lower_pipeline(
        &mut self,
        expr: &Expr,
        left: &Expr,
        right: &Expr,
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let ExprKind::Call { callee, args } = &right.kind else {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: right.node_id,
                reason: "pipeline_target_not_call".to_string(),
            });
            self.lower_expr(left, None);
            self.lower_expr(right, None);
            return self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing);
        };
        if !matches!(callee.kind, ExprKind::NamePath { segments: _, .. }) {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: right.node_id,
                reason: "pipeline_target_not_named_call".to_string(),
            });
            self.lower_expr(left, None);
            self.lower_expr(right, expected);
            return self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing);
        }

        let mut piped_args = Vec::with_capacity(args.len() + 1);
        piped_args.push(left.clone());
        piped_args.extend(args.iter().cloned());
        self.lower_call(expr, callee, &piped_args, expected)
    }

    pub(super) fn numeric_operand_type(
        &self,
        expected: Option<&CoreType>,
        operands: &[&Expr],
    ) -> CoreType {
        if expected.is_some_and(|expected| expected == &CoreType::float()) {
            return CoreType::float();
        }
        if operands.iter().any(|expr| {
            self.shallow_expr_type(expr)
                .is_some_and(|ty| ty == CoreType::float())
        }) {
            return CoreType::float();
        }
        CoreType::int()
    }

    pub(super) fn shallow_expr_type(&self, expr: &Expr) -> Option<CoreType> {
        match &expr.kind {
            ExprKind::IntLiteral(_) => Some(CoreType::int()),
            ExprKind::FloatLiteral(_) => Some(CoreType::float()),
            ExprKind::BoolLiteral(_) => Some(CoreType::bool()),
            ExprKind::NamePath { segments, .. } => match segments.as_slice() {
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
                            .map(|function| core_type(&function.ty()))
                    }),
                _ => self
                    .environment
                    .function_path(segments, self.function.module_name.as_deref())
                    .map(|function| core_type(&function.ty())),
            },
            ExprKind::Call { callee, .. } => self
                .core_call_signature(callee, None, None)
                .map(|signature| signature.return_type),
            _ => None,
        }
    }

    pub(super) fn lower_name_path(
        &mut self,
        expr: &Expr,
        segments: &[String],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if let Some(constructor) = self.lower_nullary_constructor(expr, segments, expected) {
            return constructor;
        }

        match segments {
            [name] => self.lower_unqualified_name(expr, name, expected),
            _ => self.lower_qualified_name(expr, segments),
        }
    }

    pub(super) fn lower_nullary_constructor(
        &self,
        expr: &Expr,
        segments: &[String],
        expected: Option<&CoreType>,
    ) -> Option<CoreExpr> {
        match self.environment.adts.nullary_constructor(
            segments,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        ) {
            ConstructorLookup::Found(constructor) => {
                let ty = expected
                    .filter(|expected| {
                        unification::adt_args(*expected, constructor.descriptor).is_some()
                    })
                    .cloned()
                    .unwrap_or_else(|| adt::core_constructed_type(constructor, &[]));
                Some(self.core_expr(expr, ty, core_nullary_constructor_kind(constructor)))
            }
            ConstructorLookup::Ambiguous => {
                if let Some(constructor) = expected
                    .and_then(|expected| self.environment.adts.descriptor_for_core_type(expected))
                    .and_then(|descriptor| {
                        self.environment.adts.constructor_for_descriptor(
                            segments,
                            descriptor,
                            self.function.module_name.as_deref(),
                            &self.environment.uses,
                        )
                    })
                    .filter(|constructor| constructor.variant.payload_fields.is_empty())
                {
                    return Some(self.core_expr(
                        expr,
                        expected.cloned().unwrap_or(CoreType::Unknown),
                        core_nullary_constructor_kind(constructor),
                    ));
                }
                Some(self.core_expr(
                    expr,
                    CoreType::Unknown,
                    CoreExprKind::Local(segments.join("::")),
                ))
            }
            ConstructorLookup::Missing => None,
        }
    }

    pub(super) fn lower_unqualified_name(
        &mut self,
        expr: &Expr,
        name: &str,
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if let Some(index) = self
            .bindings
            .iter()
            .rposition(|binding| binding.name == name)
        {
            return self.lower_local_name(expr, name, index, expected);
        }

        match self
            .environment
            .unqualified_function(name, self.function.module_name.as_deref())
        {
            FunctionLookup::Found(function) => self.core_expr(
                expr,
                core_type(&function.ty()),
                CoreExprKind::FunctionValue(function.target_name.clone()),
            ),
            FunctionLookup::Ambiguous | FunctionLookup::Missing => self.core_expr(
                expr,
                CoreType::Unknown,
                CoreExprKind::Local(name.to_string()),
            ),
        }
    }

    pub(super) fn lower_local_name(
        &mut self,
        expr: &Expr,
        name: &str,
        index: usize,
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let mut ty = self.bindings[index].ty.clone();
        if let Some(expected) = expected
            && !core_type_contains_unknown(expected)
            && (core_type_contains_unknown(&ty)
                || matches!(ty, CoreType::Record(ref fields) if fields.is_empty())
                    && expected.dict_parts().is_some())
        {
            ty = expected.clone();
            self.bindings[index].ty = ty.clone();
        }
        self.core_expr(expr, ty, CoreExprKind::Local(name.to_string()))
    }

    pub(super) fn lower_qualified_name(&self, expr: &Expr, segments: &[String]) -> CoreExpr {
        if let Some(function) = self
            .environment
            .function_path_for_value(segments, self.function.module_name.as_deref())
        {
            self.core_expr(
                expr,
                core_type(&function.ty()),
                CoreExprKind::FunctionValue(function.target_name.clone()),
            )
        } else {
            self.core_expr(
                expr,
                CoreType::Unknown,
                CoreExprKind::Local(segments.join("::")),
            )
        }
    }
}
