use super::module_value_lowering::module_value_identity;
use super::*;
use crate::types::ModuleValueLookup;

impl<'a> CoreLowerer<'a> {
    pub(super) fn lower_expr(&mut self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
            ExprKind::Missing | ExprKind::Hole { .. } | ExprKind::NamePath { .. } => {
                self.lower_reference_expr(expr, expected)
            }
            ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit
            | ExprKind::TypeApply { .. } => self.lower_literal_expr(expr),
            ExprKind::Call { .. }
            | ExprKind::Perform { .. }
            | ExprKind::Handle { .. }
            | ExprKind::SchemaDecode { .. }
            | ExprKind::SchemaEncode { .. }
            | ExprKind::SchemaValidate { .. }
            | ExprKind::FieldAccess { .. }
            | ExprKind::Try { .. } => self.lower_application_expr(expr, expected),
            ExprKind::Record(_)
            | ExprKind::Dict(_)
            | ExprKind::List(_)
            | ExprKind::Match { .. }
            | ExprKind::If { .. }
            | ExprKind::Begin { .. } => self.lower_structured_expr(expr, expected),
            ExprKind::Prefix { .. } | ExprKind::Binary { .. } => {
                self.lower_operator_expr(expr, expected)
            }
        }
    }

    fn lower_reference_expr(&mut self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
            ExprKind::Missing => self.lower_missing_expr(expr, expected),
            ExprKind::Hole { name, .. } => self.lower_hole_expr(expr, expected, name),
            ExprKind::NamePath { segments, .. } => self.lower_name_path(expr, segments, expected),
            _ => unreachable!("reference expression dispatcher received another expression kind"),
        }
    }

    fn lower_literal_expr(&self, expr: &Expr) -> CoreExpr {
        match &expr.kind {
            ExprKind::StringLiteral(value) => self.lower_string_literal(expr, value),
            ExprKind::IntLiteral(value) => self.lower_int_literal(expr, value),
            ExprKind::FloatLiteral(value) => self.lower_float_literal(expr, value),
            ExprKind::BoolLiteral(value) => self.lower_bool_literal(expr, *value),
            ExprKind::Unit => self.lower_unit_literal(expr),
            ExprKind::TypeApply { .. } => {
                self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
            }
            _ => unreachable!("literal expression dispatcher received another expression kind"),
        }
    }

    fn lower_application_expr(&mut self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
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
            ExprKind::SchemaValidate {
                schema,
                value,
                base,
                ..
            } => self.lower_schema_validate(expr, schema, value, base),
            ExprKind::FieldAccess { base, field, .. } => self.lower_field_access(expr, base, field),
            ExprKind::Try { expr: inner, .. } => self.lower_try(expr, inner, expected),
            _ => unreachable!("application expression dispatcher received another expression kind"),
        }
    }

    fn lower_structured_expr(&mut self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
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
            ExprKind::Begin { body, .. } => {
                let (body, ty) = self.lower_scoped_body(body, expected);
                self.core_expr(
                    expr,
                    ty,
                    CoreExprKind::CleanupRegion {
                        region: CoreCleanupRegion::new(body),
                    },
                )
            }
            _ => unreachable!("structured expression dispatcher received another expression kind"),
        }
    }

    fn lower_operator_expr(&mut self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
            ExprKind::Prefix { op, expr: inner } => self.lower_prefix(expr, *op, inner, expected),
            ExprKind::Binary { op, left, right } => {
                self.lower_binary(expr, *op, left, right, expected)
            }
            _ => unreachable!("operator expression dispatcher received another expression kind"),
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

        let numeric_type = if op == BinaryOp::Remainder {
            CoreType::int()
        } else {
            self.binary_numeric_operand_type(op, left, right, expected)
        };
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
        if !matches!(callee.kind, ExprKind::NamePath { .. }) {
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
        self.lower_call_with_callsite(expr, right, callee, &piped_args, expected)
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
        if let Some(cached) = self.shallow_type_cache.borrow().get(&expr.node_id) {
            return cached.clone();
        }
        let ty = self.uncached_shallow_expr_type(expr);
        self.shallow_type_cache
            .borrow_mut()
            .insert(expr.node_id, ty.clone());
        ty
    }

    fn uncached_shallow_expr_type(&self, expr: &Expr) -> Option<CoreType> {
        match &expr.kind {
            ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => self.shallow_literal_type(expr),
            ExprKind::Record(_) | ExprKind::List(_) => self.shallow_collection_type(expr),
            ExprKind::NamePath { segments, .. } => self.shallow_name_path_type(segments),
            ExprKind::Call { callee, args } => self.shallow_call_type(callee, args),
            _ => None,
        }
    }

    fn shallow_literal_type(&self, expr: &Expr) -> Option<CoreType> {
        match &expr.kind {
            ExprKind::StringLiteral(_) => Some(CoreType::string()),
            ExprKind::IntLiteral(_) => Some(CoreType::int()),
            ExprKind::FloatLiteral(_) => Some(CoreType::float()),
            ExprKind::BoolLiteral(_) => Some(CoreType::bool()),
            ExprKind::Unit => Some(CoreType::unit()),
            _ => unreachable!("shallow literal dispatcher received another expression kind"),
        }
    }

    fn shallow_collection_type(&self, expr: &Expr) -> Option<CoreType> {
        match &expr.kind {
            ExprKind::Record(fields) => fields
                .iter()
                .map(|field| {
                    self.shallow_expr_type(&field.expr)
                        .map(|ty| (field.name.clone(), ty))
                })
                .collect::<Option<Vec<_>>>()
                .map(CoreType::Record),
            ExprKind::List(items) => items
                .first()
                .and_then(|item| self.shallow_expr_type(item))
                .map(CoreType::vec),
            _ => unreachable!("shallow collection dispatcher received another expression kind"),
        }
    }

    fn shallow_name_path_type(&self, segments: &[String]) -> Option<CoreType> {
        match segments {
            [name] => self
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.name == *name)
                .map(|binding| binding.ty.clone())
                .or_else(|| {
                    self.environment
                        .unqualified_function_in_owner(
                            name,
                            self.function.module_name.as_deref(),
                            self.function.package_name.as_deref(),
                        )
                        .found()
                        .map(|function| core_type(&function.ty()))
                }),
            _ => self
                .environment
                .function_path_in_owner(
                    segments,
                    self.function.module_name.as_deref(),
                    self.function.package_name.as_deref(),
                )
                .map(|function| core_type(&function.ty())),
        }
    }

    fn shallow_call_type(&self, callee: &Expr, args: &[Expr]) -> Option<CoreType> {
        if let Some(list_type) = self.shallow_list_cons_type(callee, args) {
            return Some(list_type);
        }
        if let Some(constructed_type) = self.shallow_constructor_type(callee, args) {
            return Some(constructed_type);
        }
        self.shallow_callable_return_type(callee, args)
    }

    fn shallow_list_cons_type(&self, callee: &Expr, args: &[Expr]) -> Option<CoreType> {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            return None;
        };
        if !matches!(segments.as_slice(), [name] if name == "list_cons") {
            return None;
        }
        args.first()
            .and_then(|arg| self.shallow_expr_type(arg))
            .map(|item| CoreType::named("List", vec![item]))
    }

    fn shallow_constructor_type(&self, callee: &Expr, args: &[Expr]) -> Option<CoreType> {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            return None;
        };
        let ConstructorLookup::Found(constructor) = self.environment.adts.constructor(
            segments,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        ) else {
            return None;
        };
        let payloads = args
            .iter()
            .map(|arg| self.shallow_expr_type(arg).unwrap_or(CoreType::Unknown))
            .collect::<Vec<_>>();
        Some(adt::core_constructed_type(constructor, &payloads))
    }

    fn shallow_callable_return_type(&self, callee: &Expr, args: &[Expr]) -> Option<CoreType> {
        let first = args.first().and_then(|arg| self.shallow_expr_type(arg));
        let second = args.get(1).and_then(|arg| self.shallow_expr_type(arg));
        let third = args.get(2).and_then(|arg| self.shallow_expr_type(arg));
        self.core_call_signature(
            callee,
            None,
            crate::call_resolution::CallArgumentEvidence {
                first: first.as_ref(),
                second: second.as_ref(),
                third: third.as_ref(),
            },
        )
        .map(|signature| signature.return_type)
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
        match self.environment.adts.constructor_for_expected_type(
            segments,
            expected,
            self.function.module_name.as_deref(),
            &self.environment.uses,
            ConstructorShape::Nullary,
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
            ConstructorLookup::Ambiguous => Some(self.core_expr(
                expr,
                CoreType::Unknown,
                CoreExprKind::Local(segments.join("::")),
            )),
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
            let local = self.lower_local_name(expr, name, index, expected);
            self.record_defer_capture(index);
            return local;
        }

        if let ModuleValueLookup::Available(value) = self.environment.module_value_path_in_owner(
            &[name.to_string()],
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
            return self.core_expr(
                expr,
                core_type(&value.ty),
                CoreExprKind::ModuleValue(module_value_identity(&value.declaration)),
            );
        }

        match self.environment.unqualified_function_in_owner(
            name,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
            FunctionLookup::Found(function) => self.core_expr(
                expr,
                core_type(&function.ty()),
                CoreExprKind::FunctionValue {
                    name: function.target_name.clone(),
                    callsite: function.callsite,
                },
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

    pub(super) fn lower_qualified_name(&mut self, expr: &Expr, segments: &[String]) -> CoreExpr {
        if let ModuleValueLookup::Available(value) = self.environment.module_value_path_in_owner(
            segments,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
            self.core_expr(
                expr,
                core_type(&value.ty),
                CoreExprKind::ModuleValue(module_value_identity(&value.declaration)),
            )
        } else if let Some(function) = self.environment.function_path_for_value_in_owner(
            segments,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
            self.core_expr(
                expr,
                core_type(&function.ty()),
                CoreExprKind::FunctionValue {
                    name: function.target_name.clone(),
                    callsite: function.callsite,
                },
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
