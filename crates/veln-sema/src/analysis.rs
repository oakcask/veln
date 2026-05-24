use std::collections::BTreeMap;

use veln_ast::{
    BinaryOp, BodyLineKind, ContractKind, DictEntry, Expr, ExprKind, Function, FunctionKind,
    MatchArm, NodeId, Pattern, PatternKind, RecordField, SatisfyClause, SurfaceModule, Visibility,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::contracts::{
    ContractCall, ContractValidation, contract_calls, contract_kind_text, is_contract_keyword,
    missing_contract_field, predicate_is_boolean, predicate_rendered_type, referenced_names,
};
use crate::diagnostics::{
    contract_details, effect_details, effect_missing_public_details, module_details, span_json,
    type_details,
};
use crate::effects::stdio_signature;
use crate::prelude::{
    float_arithmetic_prelude_name, float_comparison_prelude_name, float_prefix_prelude_name,
    prelude_signature,
};
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
            effect_details(function.node_id.display("fn"), "public_function"),
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

pub(crate) fn check_duplicate_function_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<String, (String, SourceSpan)>::new();

    for function in &module.functions {
        let Some(name) = &function.name else {
            continue;
        };
        let node_id = function.node_id.display(function.kind.node_prefix());
        if let Some((first_node_id, first_span)) = seen.get(name) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "function",
                "function declaration",
                node_id,
                function.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(name.clone(), (node_id, function.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_duplicate_use_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<String, (String, SourceSpan)>::new();

    for use_decl in &module.uses {
        let node_id = use_decl.node_id.display("use");
        if let Some((first_node_id, first_span)) = seen.get(&use_decl.alias) {
            diagnostics.push(duplicate_name_diagnostic(
                &use_decl.alias,
                "module",
                "import alias",
                node_id,
                use_decl.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(use_decl.alias.clone(), (node_id, use_decl.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_module_boundary(module: &SurfaceModule) -> Vec<Diagnostic> {
    if module.module.is_some() || module.uses.is_empty() {
        return Vec::new();
    }

    let first_use = &module.uses[0];
    let mut diagnostic = Diagnostic::new(
        "module.missing_identity",
        Severity::Error,
        DiagnosticKind::Module,
        "module import requires a module identity",
        Some(first_use.span.clone()),
        module_details(
            first_use.node_id.display("use"),
            "module_identity",
            "source",
            "missing",
        ),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string("Add a `mod` declaration before `use` declarations."),
        ),
    ]));
    vec![diagnostic]
}

fn duplicate_name_diagnostic(
    name: &str,
    namespace: &'static str,
    declaration_kind: &'static str,
    node_id: String,
    span: SourceSpan,
    first_node_id: String,
    first_span: &SourceSpan,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "name.duplicate",
        Severity::Error,
        DiagnosticKind::Name,
        format!("duplicate {declaration_kind} name `{name}`"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(node_id)),
            ("name", JsonValue::string(name)),
            ("namespace", JsonValue::string(namespace)),
            ("first_node_id", JsonValue::string(first_node_id)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("duplicate_origin")),
        (
            "message",
            JsonValue::string(format!("First {declaration_kind} with this name is here.")),
        ),
        ("span", span_json(first_span)),
    ]));
    diagnostic
}

pub(crate) fn check_test_declaration_boundary(function: &Function) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let node_id = function.node_id.display(function.kind.node_prefix());

    if let Some(param) = function.params.first() {
        let mut diagnostic = Diagnostic::new(
            "test.parameters",
            Severity::Error,
            DiagnosticKind::Type,
            "test declaration has parameters",
            Some(param.span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("test")),
                ("node_id", JsonValue::string(node_id.clone())),
                ("expected_parameters", JsonValue::Number(0)),
                (
                    "actual_parameters",
                    JsonValue::Number(function.params.len() as i64),
                ),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("test_shape")),
            (
                "message",
                JsonValue::string("A test declaration uses an empty parameter list."),
            ),
            ("span", span_json(&function.span)),
        ]));
        diagnostics.push(diagnostic);
    }

    match function.return_type.as_deref() {
        Some(return_type) => {
            if let Ok(ty) = parse_type_annotation(return_type) {
                if !is_allowed_test_return(&ty) {
                    diagnostics.push(test_return_diagnostic(
                        function,
                        &node_id,
                        format!("test declaration returns `{}`", ty.render()),
                        ty.render(),
                    ));
                }
            }
        }
        None => diagnostics.push(test_return_diagnostic(
            function,
            &node_id,
            "test declaration has no return type annotation".to_string(),
            "missing".to_string(),
        )),
    }

    if function.effects.is_none() {
        let mut diagnostic = Diagnostic::new(
            "effect.missing_test",
            Severity::Error,
            DiagnosticKind::Effect,
            "test declaration has no effects annotation",
            Some(function.span.clone()),
            effect_details(node_id, "test_declaration"),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("repair_hint")),
            (
                "message",
                JsonValue::string("Use `effects []` for a pure test declaration."),
            ),
        ]));
        diagnostics.push(diagnostic);
    }

    diagnostics
}

fn is_allowed_test_return(ty: &Type) -> bool {
    ty == &Type::unit()
        || ty
            .result_parts()
            .is_some_and(|(value, _)| value == &Type::unit())
}

fn test_return_diagnostic(
    function: &Function,
    node_id: &str,
    message: String,
    actual_type: String,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "test.return_type",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(function.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("test")),
            ("node_id", JsonValue::string(node_id)),
            ("expected_type", JsonValue::string("() or Result((), E)")),
            ("actual_type", JsonValue::string(actual_type)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("test_shape")),
        (
            "message",
            JsonValue::string("A test declaration returns `()` or `Result((), E)`."),
        ),
        ("span", span_json(&function.span)),
    ]));
    diagnostic
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
    local_names: BTreeMap<String, (String, SourceSpan)>,
    inferred_effects: Vec<EffectUse>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> FunctionChecker<'a> {
    fn new(function: &'a Function, environment: &'a TypeEnvironment) -> Self {
        Self {
            function,
            environment,
            bindings: Vec::new(),
            local_names: BTreeMap::new(),
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
                        if !self.declare_local_name(
                            name,
                            line.node_id.display("let"),
                            line.span.clone(),
                            "local binding",
                        ) {
                            continue;
                        }
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
        self.check_effect_boundaries();
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
            if !self.declare_local_name(
                &param.name,
                param.node_id.display("param"),
                param.span.clone(),
                "parameter",
            ) {
                continue;
            }
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

        if let Some(result_binding) = &self.function.return_binding {
            if let Some(param) = self
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
    }

    fn declare_local_name(
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

    fn check_effect_boundaries(&mut self) {
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
        let calls = contract_calls(trimmed);
        for (call_index, call) in calls.iter().enumerate() {
            if call.callee.contains("::") {
                return ContractValidation::UnsupportedConstruct {
                    reason: "unsupported_call",
                };
            }
            let Some(signature) = self.environment.function(&call.callee) else {
                return ContractValidation::UnresolvedName {
                    name: call.callee.clone(),
                };
            };
            if !signature.effects.is_empty() {
                return ContractValidation::UnsupportedConstruct {
                    reason: "effectful_operation",
                };
            }
            if signature.return_type != Type::bool()
                && !contract_call_result_is_compared(trimmed, call.start, call.end)
                && !contract_call_is_argument(&calls, call_index)
            {
                return ContractValidation::NonBoolean {
                    actual_type: signature.return_type.render(),
                };
            }
            if call.args.len() != signature.params.len() {
                return ContractValidation::UnsupportedConstruct {
                    reason: "call_arity",
                };
            }
            for (arg, expected) in call.args.iter().zip(&signature.params) {
                let arg_calls = contract_calls(arg);
                for name in referenced_names(arg) {
                    if is_contract_keyword(&name) || name == "true" || name == "false" {
                        continue;
                    }
                    if arg_calls.iter().any(|call| call.callee == name) {
                        continue;
                    }
                    if self
                        .contract_bindings(kind)
                        .iter()
                        .any(|binding| binding.name == name)
                    {
                        continue;
                    }
                    return ContractValidation::UnresolvedName { name };
                }
                let actual_type = self.contract_arg_type(kind, arg);
                if !is_assignable(expected, &actual_type) {
                    return ContractValidation::UnsupportedConstruct {
                        reason: "call_argument_type",
                    };
                }
            }
        }
        for name in referenced_names(trimmed) {
            if is_contract_keyword(&name) || name == "true" || name == "false" {
                continue;
            }
            if calls.iter().any(|call| call.callee == name) {
                continue;
            }
            if self
                .contract_bindings(kind)
                .iter()
                .any(|binding| binding.name == name)
            {
                continue;
            }
            return ContractValidation::UnresolvedName { name };
        }
        if let Some(call) = calls
            .iter()
            .find(|call| call.start == 0 && call.end == trimmed.len())
        {
            let return_type = self
                .environment
                .function(&call.callee)
                .map(|signature| signature.return_type.clone())
                .unwrap_or(Type::Unknown);
            return if return_type == Type::bool() {
                ContractValidation::Valid
            } else {
                ContractValidation::NonBoolean {
                    actual_type: return_type.render(),
                }
            };
        }
        if let Some((base_type, field)) =
            missing_contract_field(trimmed, &self.contract_bindings(kind))
        {
            return ContractValidation::MissingField { base_type, field };
        }
        if predicate_is_boolean(trimmed, &self.contract_bindings(kind)) {
            ContractValidation::Valid
        } else {
            ContractValidation::NonBoolean {
                actual_type: predicate_rendered_type(trimmed, &self.contract_bindings(kind)),
            }
        }
    }

    fn contract_arg_type(&self, kind: ContractKind, arg: &str) -> Type {
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
        if let [call] = contract_calls(trimmed).as_slice() {
            if call.start == 0 && call.end == trimmed.len() {
                return self
                    .environment
                    .function(&call.callee)
                    .map(|signature| signature.return_type.clone())
                    .unwrap_or(Type::Unknown);
            }
        }
        let mut parts = trimmed.split('.');
        let Some(base) = parts.next() else {
            return Type::Unknown;
        };
        let Some(binding) = self
            .contract_bindings(kind)
            .into_iter()
            .find(|binding| binding.name == base)
        else {
            return Type::Unknown;
        };
        let mut current = binding.ty;
        for field in parts {
            let Some(next) = current.record_field(field) else {
                return Type::Unknown;
            };
            current = next.clone();
        }
        current
    }

    fn contract_referenced_bindings(&self, kind: ContractKind, predicate: &str) -> Vec<JsonValue> {
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

    fn contract_bindings(&self, kind: ContractKind) -> Vec<Binding> {
        let mut bindings = self.bindings.clone();
        if kind == ContractKind::Ensure {
            if let Some(result_binding) = &self.function.return_binding {
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
        }
        bindings
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
                if let Some(satisfy) = satisfy {
                    self.check_satisfy_clause(expr, satisfy);
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
            ExprKind::Unit => Type::unit(),
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
            ExprKind::Prefix { op, expr } => self.infer_prefix(*op, expr, expected),
            ExprKind::Binary { op, left, right } => self.infer_binary(*op, left, right, expected),
        }
    }

    fn infer_name_path(
        &mut self,
        segments: &[String],
        expr: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Type {
        match segments {
            [name] if name == "true" || name == "false" => Type::bool(),
            [name] if name == "None" => expected
                .and_then(|expected| expected.ty.option_part().map(|_| expected.ty.clone()))
                .unwrap_or_else(|| Type::named("Option", vec![Type::Unknown])),
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
            if let [name] = segments.as_slice() {
                if let Some((params, return_type)) =
                    prelude_signature(name, expected.map(|expected| &expected.ty))
                {
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
                        self.check_assignable(
                            arg,
                            &expected.ty,
                            &actual,
                            &expected,
                            "call_argument",
                        );
                    }
                    return return_type;
                }
            }
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

    fn infer_field_access(
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
                let effects = binding.ty.function_effects().unwrap_or_default().to_vec();
                Some((
                    params.to_vec(),
                    return_type.clone(),
                    CallOrigin {
                        node_id: callee.node_id,
                        span: callee.span.clone(),
                        symbol: name.clone(),
                        effects,
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

    fn infer_match(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[MatchArm],
        expected: Option<&ExpectedType>,
    ) -> Type {
        let scrutinee_type = self.infer_expr(scrutinee, None);
        if arms.is_empty() {
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
                let _ = self.declare_local_name(
                    &binding.name,
                    arm.pattern.node_id.display("pattern"),
                    arm.pattern.span.clone(),
                    "pattern binding",
                );
                self.bindings.push(binding);
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

        result_type
    }

    fn pattern_bindings(&self, pattern: &Pattern, scrutinee_type: &Type) -> Vec<Binding> {
        match &pattern.kind {
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => Vec::new(),
            PatternKind::Binding(name) => vec![Binding {
                name: name.clone(),
                ty: scrutinee_type.clone(),
            }],
            PatternKind::Record(fields) => fields
                .iter()
                .flat_map(|field| {
                    let field_type = scrutinee_type
                        .record_field(&field.name)
                        .unwrap_or(&Type::Unknown);
                    self.pattern_bindings(&field.pattern, field_type)
                })
                .collect(),
            PatternKind::Constructor { name, args } => match name.as_slice() {
                [constructor] if constructor == "Some" => scrutinee_type
                    .option_part()
                    .zip(args.first())
                    .map_or_else(Vec::new, |(inner, pattern)| {
                        self.pattern_bindings(pattern, inner)
                    }),
                [constructor] if constructor == "Ok" => scrutinee_type
                    .result_parts()
                    .zip(args.first())
                    .map_or_else(Vec::new, |((value, _), pattern)| {
                        self.pattern_bindings(pattern, value)
                    }),
                [constructor] if constructor == "Err" => scrutinee_type
                    .result_parts()
                    .zip(args.first())
                    .map_or_else(Vec::new, |((_, error), pattern)| {
                        self.pattern_bindings(pattern, error)
                    }),
                _ => Vec::new(),
            },
        }
    }

    fn infer_record(
        &mut self,
        _expr: &Expr,
        fields: &[RecordField],
        expected: Option<&ExpectedType>,
    ) -> Type {
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
        if let Some(expected) = expected {
            if matches!(expected.ty, Type::Record(_)) {
                return expected.ty.clone();
            }
        }
        Type::Record(actual_fields)
    }

    fn infer_dict(
        &mut self,
        expr: &Expr,
        entries: &[DictEntry],
        expected: Option<&ExpectedType>,
    ) -> Type {
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

    fn infer_prefix(
        &mut self,
        op: veln_ast::PrefixOp,
        expr: &Expr,
        expected_result: Option<&ExpectedType>,
    ) -> Type {
        let operand_type = match op {
            veln_ast::PrefixOp::Not => Type::bool(),
            veln_ast::PrefixOp::Negate => self.numeric_operand_type(expected_result, &[expr]),
        };
        if operand_type == Type::float() {
            if let Some(name) = float_prefix_prelude_name(op) {
                return self.infer_builtin_unary_call(name, expr);
            }
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

    fn infer_binary(
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

    fn infer_pipeline(
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

        let mut piped_args = Vec::with_capacity(args.len() + 1);
        piped_args.push(left.clone());
        piped_args.extend(args.iter().cloned());
        self.infer_call(right, callee, &piped_args, expected_result)
    }

    fn infer_builtin_unary_call(&mut self, name: &str, arg: &Expr) -> Type {
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

    fn infer_builtin_binary_call(&mut self, name: &str, left: &Expr, right: &Expr) -> Type {
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

    fn check_numeric_operator_assignable(
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

    fn numeric_operand_type(
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

    fn shallow_expr_type(&self, expr: &Expr) -> Option<Type> {
        match &expr.kind {
            ExprKind::IntLiteral(_) => Some(Type::int()),
            ExprKind::FloatLiteral(_) => Some(Type::float()),
            ExprKind::NamePath(segments) => match segments.as_slice() {
                [name] => self
                    .bindings
                    .iter()
                    .rev()
                    .find(|binding| binding.name == *name)
                    .map(|binding| binding.ty.clone()),
                _ => None,
            },
            ExprKind::Call { callee, .. } => self
                .call_signature(callee)
                .map(|(_, return_type, _)| return_type),
            _ => None,
        }
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

    fn check_satisfy_clause(&mut self, expr: &Expr, satisfy: &SatisfyClause) {
        let Some(candidate) = satisfy.candidate.as_deref() else {
            return;
        };
        let candidate_span = satisfy
            .candidate_span
            .clone()
            .unwrap_or_else(|| satisfy.span.clone());

        if let Some((origin_kind, origin_span)) = self.satisfy_shadow_origin(candidate) {
            let mut diagnostic = Diagnostic::new(
                "hole.satisfy_candidate_shadow",
                Severity::Error,
                DiagnosticKind::Hole,
                format!("satisfy candidate `{candidate}` shadows a visible binding"),
                Some(candidate_span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("hole")),
                    ("node_id", JsonValue::string(expr.node_id.display("hole"))),
                    ("candidate_binding", JsonValue::string(candidate)),
                    ("shadowed_binding_kind", JsonValue::string(origin_kind)),
                ]),
            );
            if let Some(origin_span) = origin_span {
                diagnostic.related.push(JsonValue::object([
                    ("kind", JsonValue::string("shadow_origin")),
                    (
                        "message",
                        JsonValue::string("Visible binding with this name is here."),
                    ),
                    ("span", span_json(&origin_span)),
                ]));
            } else {
                diagnostic.related.push(JsonValue::object([
                    ("kind", JsonValue::string("shadow_origin")),
                    (
                        "message",
                        JsonValue::string(
                            "Prelude helper names cannot be reused as satisfy candidates.",
                        ),
                    ),
                ]));
            }
            self.diagnostics.push(diagnostic);
        }

        if !referenced_names(&satisfy.predicate)
            .iter()
            .any(|name| name == candidate)
        {
            let mut diagnostic = Diagnostic::new(
                "hole.satisfy_candidate_unused",
                Severity::Error,
                DiagnosticKind::Hole,
                format!("satisfy predicate does not reference candidate `{candidate}`"),
                Some(candidate_span),
                JsonValue::object([
                    ("phase", JsonValue::string("hole")),
                    ("node_id", JsonValue::string(expr.node_id.display("hole"))),
                    ("candidate_binding", JsonValue::string(candidate)),
                    (
                        "predicate_text",
                        JsonValue::string(satisfy.predicate.clone()),
                    ),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("constraint_origin")),
                (
                    "message",
                    JsonValue::string("The predicate for this satisfy clause is here."),
                ),
                ("span", span_json(&satisfy.span)),
            ]));
            self.diagnostics.push(diagnostic);
        }
    }

    fn satisfy_shadow_origin(&self, candidate: &str) -> Option<(&'static str, Option<SourceSpan>)> {
        if let Some((_, span)) = self.local_names.get(candidate) {
            return Some(("local", Some(span.clone())));
        }
        if let Some(result_binding) = &self.function.return_binding {
            if result_binding.name == candidate {
                return Some(("result", Some(result_binding.span.clone())));
            }
        }
        if prelude_signature(candidate, None).is_some() {
            return Some(("prelude", None));
        }
        None
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
            ("candidate_status", JsonValue::string("query_only")),
            (
                "application_policy",
                JsonValue::string("manual_review_required"),
            ),
            (
                "query",
                JsonValue::string(format!("fn({argument_types}) -> {}", expected.render())),
            ),
        ])]
    }
}

fn is_ordering_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
    )
}

fn contract_call_result_is_compared(predicate: &str, start: usize, end: usize) -> bool {
    let before = predicate[..start].trim_end();
    let after = predicate[end..].trim_start();
    before.ends_with("==")
        || before.ends_with("!=")
        || before.ends_with("<=")
        || before.ends_with(">=")
        || before.ends_with('<')
        || before.ends_with('>')
        || after.starts_with("==")
        || after.starts_with("!=")
        || after.starts_with("<=")
        || after.starts_with(">=")
        || after.starts_with('<')
        || after.starts_with('>')
}

fn contract_call_is_argument(calls: &[ContractCall], call_index: usize) -> bool {
    let call = &calls[call_index];
    calls.iter().enumerate().any(|(index, outer)| {
        index != call_index && outer.start < call.start && call.end < outer.end
    })
}
