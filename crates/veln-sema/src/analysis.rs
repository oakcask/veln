use veln_ast::{
    BinaryOp, BodyLineKind, ContractKind, Expr, ExprKind, Function, NodeId, RecordField,
    SatisfyClause, Visibility,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::contracts::{
    ContractValidation, contains_call_like_construct, contract_kind_text, is_contract_keyword,
    predicate_is_boolean, predicate_rendered_type, referenced_names,
};
use crate::diagnostics::{
    contract_details, effect_details, effect_missing_public_details, span_json, type_details,
};
use crate::effects::stdio_signature;
use crate::types::{
    Binding, CallOrigin, EffectUse, ExpectedType, ExpectedTypeSource, Type, TypeEnvironment,
    is_assignable, parse_type_annotation,
};

pub(crate) fn check_public_function_boundary(function: &Function) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for param in &function.params {
        if param.ty.is_none() {
            diagnostics.push(Diagnostic::new(
                "type.public_signature_missing",
                Severity::Error,
                DiagnosticKind::Type,
                format!("public parameter `{}` has no type annotation", param.name),
                Some(param.span.clone()),
                type_details(
                    param.node_id.display("param"),
                    "explicit",
                    "missing",
                    "declared_parameter",
                    "source",
                    "assignable",
                    [function.node_id.display("fn")],
                ),
            ));
        }
    }

    if function.return_type.is_none() {
        diagnostics.push(Diagnostic::new(
            "type.public_signature_missing",
            Severity::Error,
            DiagnosticKind::Type,
            "public function has no return type annotation",
            Some(function.span.clone()),
            type_details(
                function.node_id.display("fn"),
                "explicit",
                "missing",
                "declared_return",
                "source",
                "return_value",
                [function.node_id.display("fn")],
            ),
        ));
    }

    if function.effects.is_none() {
        let mut diagnostic = Diagnostic::new(
            "effect.missing_public",
            Severity::Error,
            DiagnosticKind::Effect,
            "public function has no effects annotation",
            Some(function.span.clone()),
            effect_details(function.node_id.display("fn")),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("repair_hint")),
            (
                "message",
                JsonValue::string("Use `effects []` for a pure public function."),
            ),
        ]));
        diagnostics.push(diagnostic);
    }

    diagnostics
}

pub(crate) fn check_function_body(
    function: &Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut checker = FunctionChecker::new(function, environment);
    checker.check_body();
    checker.diagnostics
}

struct FunctionChecker<'a> {
    function: &'a Function,
    environment: &'a TypeEnvironment,
    bindings: Vec<Binding>,
    inferred_effects: Vec<EffectUse>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> FunctionChecker<'a> {
    fn new(function: &'a Function, environment: &'a TypeEnvironment) -> Self {
        Self {
            function,
            environment,
            bindings: Vec::new(),
            inferred_effects: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn check_body(&mut self) {
        self.check_function_annotations();
        self.check_contracts();
        for (index, line) in self.function.body.iter().enumerate() {
            match &line.kind {
                BodyLineKind::Let {
                    name,
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
                    let actual = self.infer_expr(expr, expected.as_ref());
                    if let Some(expected) = &expected {
                        self.check_assignable(expr, &expected.ty, &actual, expected, "assignable");
                    }
                    if let Some(name) = name {
                        self.bindings.push(Binding {
                            name: name.clone(),
                            ty: expected.map_or(actual, |expected| expected.ty),
                        });
                    }
                }
                BodyLineKind::Expr { expr } => {
                    let expected = self.return_expected(line.node_id);
                    let actual = self.infer_expr(expr, expected.as_ref());
                    if index + 1 == self.function.body.len() {
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
        self.check_public_effect_boundary();
    }

    fn check_function_annotations(&mut self) {
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
            self.bindings.push(Binding {
                name: param.name.clone(),
                ty: ty.map_or(Type::Unknown, |expected| expected.ty),
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
    }

    fn check_contracts(&mut self) {
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

    fn check_public_effect_boundary(&mut self) {
        if self.function.visibility != Visibility::Public {
            return;
        }
        let Some(declared_effects) = &self.function.effects else {
            return;
        };

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
            let mut diagnostic = Diagnostic::new(
                "effect.missing_public",
                Severity::Error,
                DiagnosticKind::Effect,
                format!("public function uses undeclared effect `{effect}`"),
                Some(self.function.span.clone()),
                effect_missing_public_details(
                    self.function.node_id.display("fn"),
                    effect,
                    declared_effects,
                    &inferred_effects,
                    &provenance,
                    self.inferred_effects.len() > provenance.len(),
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

    fn validate_contract_predicate(
        &self,
        kind: ContractKind,
        predicate: &str,
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
        if contains_call_like_construct(trimmed) {
            return ContractValidation::UnsupportedConstruct {
                reason: "unsupported_call",
            };
        }
        for name in referenced_names(trimmed) {
            if is_contract_keyword(&name) || name == "true" || name == "false" {
                continue;
            }
            if kind == ContractKind::Ensure && name == "result" {
                continue;
            }
            if self.bindings.iter().any(|binding| binding.name == name) {
                continue;
            }
            return ContractValidation::UnresolvedName { name };
        }
        if predicate_is_boolean(trimmed, &self.bindings) {
            ContractValidation::Valid
        } else {
            ContractValidation::NonBoolean {
                actual_type: predicate_rendered_type(trimmed, &self.bindings),
            }
        }
    }

    fn contract_referenced_bindings(&self, kind: ContractKind, predicate: &str) -> Vec<JsonValue> {
        referenced_names(predicate)
            .into_iter()
            .filter_map(|name| {
                if kind == ContractKind::Ensure && name == "result" {
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

    fn parse_annotation(
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

    fn return_expected(&self, origin_node_id: NodeId) -> Option<ExpectedType> {
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
    }

    fn infer_expr(&mut self, expr: &Expr, expected: Option<&ExpectedType>) -> Type {
        match &expr.kind {
            ExprKind::Missing => Type::Unknown,
            ExprKind::Hole { name, satisfy } => {
                self.push_hole_diagnostic(expr, name.as_deref(), satisfy.as_ref(), expected);
                expected
                    .map(|expected| expected.ty.clone())
                    .unwrap_or(Type::Unknown)
            }
            ExprKind::NamePath(segments) => self.infer_name_path(segments, expr),
            ExprKind::StringLiteral(_) => Type::string(),
            ExprKind::IntLiteral(_) => Type::int(),
            ExprKind::FloatLiteral(_) => Type::float(),
            ExprKind::Unit => Type::unit(),
            ExprKind::Call { callee, args } => self.infer_call(expr, callee, args, expected),
            ExprKind::Try(inner) => self.infer_try(expr, inner, expected),
            ExprKind::Record(fields) => self.infer_record(expr, fields, expected),
            ExprKind::List(items) => self.infer_list(expr, items, expected),
            ExprKind::Prefix { op, expr } => self.infer_prefix(*op, expr),
            ExprKind::Binary { op, left, right } => self.infer_binary(*op, left, right),
        }
    }

    fn infer_name_path(&mut self, segments: &[String], expr: &Expr) -> Type {
        match segments {
            [name] if name == "true" || name == "false" => Type::bool(),
            [name] => {
                if let Some(binding) = self
                    .bindings
                    .iter()
                    .rev()
                    .find(|binding| binding.name == *name)
                {
                    binding.ty.clone()
                } else {
                    self.push_unresolved_name(expr.node_id, expr.span.clone(), name, "value");
                    Type::Unknown
                }
            }
            _ => {
                let symbol = segments.join("::");
                self.push_unresolved_name(expr.node_id, expr.span.clone(), &symbol, "value");
                Type::Unknown
            }
        }
    }

    fn infer_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Type {
        if let ExprKind::NamePath(segments) = &callee.kind {
            if matches!(segments.as_slice(), [name] if name == "Ok") {
                return self.infer_result_constructor(expr, args, expected, true);
            }
            if matches!(segments.as_slice(), [name] if name == "Err") {
                return self.infer_result_constructor(expr, args, expected, false);
            }
            if matches!(segments.as_slice(), [name] if name == "Some") {
                return self.infer_option_constructor(expr, args, expected);
            }
        }

        if let Some((params, return_type, origin)) = self.call_signature(callee) {
            for effect in &origin.effects {
                self.inferred_effects.push(EffectUse {
                    effect: effect.clone(),
                    node_id: expr.node_id,
                    span: expr.span.clone(),
                    kind: "direct_call",
                    symbol: origin.symbol.clone(),
                });
            }
            for (index, arg) in args.iter().enumerate() {
                let Some(param_type) = params.get(index) else {
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
            return return_type;
        }

        if let ExprKind::NamePath(segments) = &callee.kind {
            let symbol = segments.join("::");
            self.push_unresolved_name(callee.node_id, callee.span.clone(), &symbol, "call_target");
        }
        for arg in args {
            self.infer_expr(arg, None);
        }
        Type::Unknown
    }

    fn call_signature(&self, callee: &Expr) -> Option<(Vec<Type>, Type, CallOrigin)> {
        match &callee.kind {
            ExprKind::NamePath(segments) => {
                if let Some(origin) = stdio_signature(segments, callee) {
                    return Some((vec![Type::string()], Type::unit(), origin));
                }
                let name = segments.last()?;
                if let Some(function) = self.environment.function(name) {
                    return Some((
                        function.params.clone(),
                        function.return_type.clone(),
                        CallOrigin {
                            node_id: function.node_id,
                            span: function.span.clone(),
                            symbol: function.name.clone(),
                            effects: function.effects.clone(),
                        },
                    ));
                }
                let binding = self
                    .bindings
                    .iter()
                    .rev()
                    .find(|binding| binding.name == *name)?;
                let (params, return_type) = binding.ty.function_parts()?;
                Some((
                    params.to_vec(),
                    return_type.clone(),
                    CallOrigin {
                        node_id: callee.node_id,
                        span: callee.span.clone(),
                        symbol: name.clone(),
                        effects: Vec::new(),
                    },
                ))
            }
            _ => None,
        }
    }

    fn infer_result_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
        is_ok: bool,
    ) -> Type {
        let (expected_value, expected_error) = expected
            .and_then(|expected| expected.ty.result_parts())
            .map_or((Type::Unknown, Type::Unknown), |(value, error)| {
                (value.clone(), error.clone())
            });

        let arg_expected = ExpectedType {
            ty: if is_ok {
                expected_value.clone()
            } else {
                expected_error.clone()
            },
            source: expected.map_or(ExpectedTypeSource::Unknown, |expected| expected.source),
            origin_node_id: expected.map_or(expr.node_id, |expected| expected.origin_node_id),
            origin_span: expected.and_then(|expected| expected.origin_span.clone()),
            origin_message: expected.map_or("Expected type inferred here.", |expected| {
                expected.origin_message
            }),
        };

        let actual_arg = if let Some(arg) = args.first() {
            let actual_arg = self.infer_expr(arg, Some(&arg_expected));
            self.check_assignable(
                arg,
                &arg_expected.ty,
                &actual_arg,
                &arg_expected,
                "call_argument",
            );
            actual_arg
        } else {
            Type::Unknown
        };
        for arg in args.iter().skip(1) {
            self.infer_expr(arg, None);
        }

        if expected
            .and_then(|expected| expected.ty.result_parts())
            .is_some()
        {
            return Type::result(expected_value, expected_error);
        }

        if is_ok {
            Type::result(actual_arg, Type::Unknown)
        } else {
            Type::result(Type::Unknown, actual_arg)
        }
    }

    fn infer_option_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Type {
        let expected_item = expected
            .and_then(|expected| expected.ty.option_part())
            .cloned()
            .unwrap_or(Type::Unknown);
        let arg_expected = ExpectedType {
            ty: expected_item.clone(),
            source: expected.map_or(ExpectedTypeSource::Unknown, |expected| expected.source),
            origin_node_id: expected.map_or(expr.node_id, |expected| expected.origin_node_id),
            origin_span: expected.and_then(|expected| expected.origin_span.clone()),
            origin_message: expected.map_or("Expected type inferred here.", |expected| {
                expected.origin_message
            }),
        };
        let actual_item = if let Some(arg) = args.first() {
            let actual_item = self.infer_expr(arg, Some(&arg_expected));
            self.check_assignable(
                arg,
                &arg_expected.ty,
                &actual_item,
                &arg_expected,
                "call_argument",
            );
            actual_item
        } else {
            Type::Unknown
        };
        for arg in args.iter().skip(1) {
            self.infer_expr(arg, None);
        }
        Type::named(
            "Option",
            vec![if expected_item == Type::Unknown {
                actual_item
            } else {
                expected_item
            }],
        )
    }

    fn infer_list(&mut self, expr: &Expr, items: &[Expr], expected: Option<&ExpectedType>) -> Type {
        let expected_item = expected
            .and_then(|expected| expected.ty.list_part())
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
        Type::list(item_type)
    }

    fn infer_record(
        &mut self,
        _expr: &Expr,
        fields: &[RecordField],
        expected: Option<&ExpectedType>,
    ) -> Type {
        let mut actual_fields = Vec::new();
        for field in fields {
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
        if let Some(expected) = expected {
            if matches!(expected.ty, Type::Record(_)) {
                return expected.ty.clone();
            }
        }
        Type::Record(actual_fields)
    }

    fn infer_try(&mut self, expr: &Expr, inner: &Expr, expected: Option<&ExpectedType>) -> Type {
        let return_result = self
            .function
            .return_type
            .as_deref()
            .and_then(|return_type| parse_type_annotation(return_type).ok())
            .and_then(|return_type| {
                return_type
                    .result_parts()
                    .map(|(value, error)| (value.clone(), error.clone()))
            });
        let (value_type, error_type) = match (expected, return_result) {
            (Some(expected), Some((_, error_type))) => (expected.ty.clone(), error_type),
            (Some(expected), None) => (expected.ty.clone(), Type::Unknown),
            (None, Some((value_type, error_type))) => (value_type, error_type),
            (None, None) => (Type::Unknown, Type::Unknown),
        };
        let inner_expected = ExpectedType {
            ty: Type::result(value_type.clone(), error_type),
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

    fn infer_prefix(&mut self, op: veln_ast::PrefixOp, expr: &Expr) -> Type {
        let expected = ExpectedType {
            ty: match op {
                veln_ast::PrefixOp::Not => Type::bool(),
                veln_ast::PrefixOp::Negate => Type::int(),
            },
            source: ExpectedTypeSource::Inferred,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Operator operand type inferred here.",
        };
        self.infer_expr(expr, Some(&expected));
        expected.ty
    }

    fn infer_binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> Type {
        let (operand_type, result_type) = match op {
            BinaryOp::Or | BinaryOp::And => (Type::bool(), Type::bool()),
            BinaryOp::Equal | BinaryOp::NotEqual => (Type::Unknown, Type::bool()),
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                (Type::int(), Type::bool())
            }
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                (Type::int(), Type::int())
            }
            BinaryOp::PipeGreater => (Type::Unknown, Type::Unknown),
        };
        let expected = ExpectedType {
            ty: operand_type,
            source: ExpectedTypeSource::Inferred,
            origin_node_id: left.node_id,
            origin_span: Some(left.span.clone()),
            origin_message: "Operator operand type inferred here.",
        };
        self.infer_expr(left, Some(&expected));
        self.infer_expr(right, Some(&expected));
        result_type
    }

    fn check_assignable(
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

    fn push_invalid_type_annotation(
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

    fn push_unresolved_name(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        symbol: &str,
        namespace: &'static str,
    ) {
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

    fn push_hole_diagnostic(
        &mut self,
        expr: &Expr,
        name: Option<&str>,
        satisfy: Option<&SatisfyClause>,
        expected: Option<&ExpectedType>,
    ) {
        let expected_type = expected
            .map(|expected| expected.ty.render())
            .unwrap_or_else(|| "unknown".to_string());
        let expected_source =
            expected.map_or(ExpectedTypeSource::Unknown, |expected| expected.source);
        let candidate_queries = self.candidate_queries(expected.map(|expected| &expected.ty));
        let constraints = self.hole_constraints(satisfy);
        let mut diagnostic = Diagnostic::new(
            "hole.unfilled",
            Severity::Hint,
            DiagnosticKind::Hole,
            if expected_type == "unknown" {
                "hole requires a value of unknown type".to_string()
            } else {
                format!("hole requires a `{expected_type}` value")
            },
            Some(expr.span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("hole")),
                ("node_id", JsonValue::string(expr.node_id.display("hole"))),
                (
                    "label",
                    name.map_or(JsonValue::Null, |name| {
                        JsonValue::string(format!("_{name}"))
                    }),
                ),
                ("expected_type", JsonValue::string(expected_type)),
                (
                    "expected_type_source",
                    JsonValue::string(expected_source.as_hole_source()),
                ),
                ("constraints", JsonValue::array(constraints)),
                (
                    "local_bindings",
                    JsonValue::array(self.bindings.iter().map(|binding| {
                        JsonValue::object([
                            ("name", JsonValue::string(binding.name.clone())),
                            ("type", JsonValue::string(binding.ty.render())),
                        ])
                    })),
                ),
                ("candidate_queries", JsonValue::array(candidate_queries)),
            ]),
        );
        if let Some(expected) = expected {
            if let Some(span) = &expected.origin_span {
                diagnostic.related.push(JsonValue::object([
                    ("kind", JsonValue::string("expected_type_origin")),
                    ("message", JsonValue::string(expected.origin_message)),
                    ("span", span_json(span)),
                ]));
            }
        }
        for contract in &self.function.contracts {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("constraint_origin")),
                (
                    "message",
                    JsonValue::string(format!(
                        "{} contract contributes a repair constraint.",
                        contract_kind_text(contract.kind)
                    )),
                ),
                ("span", span_json(&contract.span)),
            ]));
        }
        if let Some(satisfy) = satisfy {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("constraint_origin")),
                (
                    "message",
                    JsonValue::string("Satisfy predicate contributes a repair constraint."),
                ),
                ("span", span_json(&satisfy.span)),
            ]));
        }
        self.diagnostics.push(diagnostic);
    }

    fn hole_constraints(&self, satisfy: Option<&SatisfyClause>) -> Vec<JsonValue> {
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
            constraints.push(JsonValue::object([
                ("kind", JsonValue::string("satisfy")),
                ("text", JsonValue::string(satisfy.predicate.clone())),
                (
                    "candidate_binding",
                    satisfy
                        .candidate
                        .as_ref()
                        .map_or(JsonValue::Null, |candidate| JsonValue::string(candidate)),
                ),
                ("validation_status", JsonValue::string("valid_unknown")),
                (
                    "repair_status",
                    JsonValue::string("blocked_until_discharged"),
                ),
            ]));
        }
        constraints
    }

    fn candidate_queries(&self, expected: Option<&Type>) -> Vec<JsonValue> {
        let Some(expected) = expected.filter(|expected| **expected != Type::Unknown) else {
            return Vec::new();
        };
        let argument_types = self
            .bindings
            .iter()
            .map(|binding| binding.ty.render())
            .collect::<Vec<_>>()
            .join(", ");
        vec![JsonValue::object([
            ("kind", JsonValue::string("symbol")),
            (
                "query",
                JsonValue::string(format!("fn({argument_types}) -> {}", expected.render())),
            ),
        ])]
    }
}
