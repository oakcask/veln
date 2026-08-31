use super::*;

impl<'a> FunctionChecker<'a> {
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
        let mut key_type = expected_key;
        let mut value_type = expected_value;
        for entry in entries {
            let key_expected = collection_item_expected(
                key_type.clone(),
                expected,
                expr.node_id,
                expr.span.clone(),
                "Dict key type inferred here.",
            );
            let actual_key = self.infer_expr(&entry.key, Some(&key_expected));
            self.check_assignable(
                &entry.key,
                &key_expected.ty,
                &actual_key,
                &key_expected,
                "dict_key",
            );
            if key_type == Type::Unknown {
                key_type = actual_key;
            }
            let value_expected = collection_item_expected(
                value_type.clone(),
                expected,
                expr.node_id,
                expr.span.clone(),
                "Dict value type inferred here.",
            );
            let actual_value = self.infer_expr(&entry.value, Some(&value_expected));
            self.check_assignable(
                &entry.value,
                &value_expected.ty,
                &actual_value,
                &value_expected,
                "dict_value",
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
            .map(|ty| {
                self.environment
                    .canonicalize_type_annotation(ty, self.function.module_name.as_deref())
            })
            .and_then(|return_type| {
                adt::result_parts(&return_type).map(|(value, error)| (value.clone(), error.clone()))
            });
        let (value_type, error_type) = match (expected, return_result) {
            (Some(expected), Some((_, error_type))) => (expected.ty.clone(), error_type),
            (Some(expected), None) => (expected.ty.clone(), Type::Unknown),
            (None, Some((_, error_type))) => (Type::Unknown, error_type),
            (None, None) => (Type::Unknown, Type::Unknown),
        };
        let mut inner_expected = ExpectedType {
            ty: adt::result_type(value_type.clone(), error_type.clone()),
            source: ExpectedTypeSource::Inferred,
            origin_node_id: expected.map_or(expr.node_id, |expected| expected.origin_node_id),
            origin_span: expected.and_then(|expected| expected.origin_span.clone()),
            origin_message: expected.map_or(
                "Result propagation expected type inferred here.",
                |expected| expected.origin_message,
            ),
        };
        let actual = self.infer_expr(inner, Some(&inner_expected));
        if expected.is_none()
            && let Some((actual_value, _)) = adt::result_parts(&actual)
        {
            inner_expected.ty = adt::result_type(actual_value.clone(), error_type);
        }
        self.check_assignable(
            inner,
            &inner_expected.ty,
            &actual,
            &inner_expected,
            "return_value",
        );
        expected.map_or_else(
            || {
                adt::result_parts(&actual)
                    .map(|(value, _)| value.clone())
                    .unwrap_or(Type::Unknown)
            },
            |_| value_type,
        )
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
            veln_ast::PrefixOp::BitwiseNot => Type::int(),
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

        if let Some(count) = invalid_literal_shift_count(op, right) {
            let operator = shift_operator_text(op).expect("shift operator should have text");
            self.diagnostics.push(Diagnostic::new(
                "type.invalid_shift_count",
                Severity::Error,
                DiagnosticKind::Type,
                format!("shift count {count} is outside the permitted range 0 through 63"),
                Some(right.span.clone()),
                JsonValue::object([
                    ("operator", JsonValue::string(operator)),
                    ("actual_count", JsonValue::Number(count)),
                    ("minimum_count", JsonValue::Number(0)),
                    ("maximum_count", JsonValue::Number(63)),
                ]),
            ));
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
            BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::BitwiseAnd
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
            | BinaryOp::ShiftRightLogical => (Type::int(), Type::int()),
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
        if !matches!(callee.kind, ExprKind::NamePath { segments: _, .. }) {
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
}
