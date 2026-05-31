use std::collections::BTreeMap;

#[path = "holes.rs"]
mod holes;

use veln_ast::{
    BinaryOp, BodyLineKind, ContractKind, DictEntry, Expr, ExprKind, Function, FunctionKind,
    MatchArm, NodeId, Pattern, PatternKind, RecordField, SatisfyClause, SurfaceModule, Visibility,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::adt::{self, ConstructorLookup};
use crate::contracts::{
    ContractCall, ContractValidation, contract_calls, contract_kind_text,
    contract_predicate_is_statically_true, is_contract_keyword, missing_contract_field,
    predicate_is_boolean_with_calls, predicate_is_statically_false, predicate_is_statically_true,
    predicate_is_statically_true_with_literal_bounds, predicate_rendered_type_with_calls,
    predicate_type_with_calls, referenced_names,
};
use crate::diagnostics::{
    contract_details, effect_details, effect_missing_public_details, module_details, span_json,
    type_details,
};
use crate::effects::{
    KNOWN_EFFECT_LABELS, concurrency_origin, concurrency_signature, standard_library_origin,
    standard_library_signature, stdio_signature,
};
use crate::prelude::{
    float_arithmetic_prelude_name, float_comparison_prelude_name, float_prefix_prelude_name,
    prelude_signature,
};
use crate::repair_candidates::{
    APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED, APPLICATION_STATUS_UNAPPLIED,
    CANDIDATE_STATUS_QUERY_ONLY, SATISFY_STATUS_BLOCKED_UNTIL_DISCHARGED,
    SATISFY_STATUS_STATICALLY_SATISFIED, application_policy, candidate_blocking_obligations,
    candidate_evidence, candidate_known_limits, candidate_satisfy_status,
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

    diagnostics
}

pub(crate) fn check_declared_effect_labels(function: &Function) -> Vec<Diagnostic> {
    let Some(declared_effects) = &function.effects else {
        return Vec::new();
    };
    let boundary = declared_effect_boundary(function);
    let node_prefix = function.kind.node_prefix();

    if declared_effects.is_empty() {
        return vec![empty_declared_effect_diagnostic(
            function,
            node_prefix,
            boundary,
        )];
    }

    declared_effects
        .iter()
        .filter(|effect| !KNOWN_EFFECT_LABELS.contains(&effect.as_str()))
        .map(|effect| unknown_declared_effect_diagnostic(function, effect, node_prefix, boundary))
        .collect()
}

fn declared_effect_boundary(function: &Function) -> &'static str {
    match function.kind {
        FunctionKind::Test => "test_declaration",
        FunctionKind::Function if function.visibility == Visibility::Public => "public_function",
        FunctionKind::Function => "private_function",
    }
}

fn empty_declared_effect_diagnostic(
    function: &Function,
    node_prefix: &'static str,
    boundary: &'static str,
) -> Diagnostic {
    let subject = match function.kind {
        FunctionKind::Test => "test declaration",
        FunctionKind::Function => "function declaration",
    };
    let mut diagnostic = Diagnostic::new(
        "effect.empty_declaration",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("empty effects list is not allowed on a {subject}"),
        Some(function.span.clone()),
        effect_details(function.node_id.display(node_prefix), boundary),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string("Remove the clause when the inferred effect set is empty."),
        ),
    ]));
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string(
                "Replace the empty list with non-empty effect labels when the body performs effects.",
            ),
        ),
    ]));
    diagnostic
}

fn unknown_declared_effect_diagnostic(
    function: &Function,
    effect: &str,
    node_prefix: &'static str,
    boundary: &'static str,
) -> Diagnostic {
    let declared_effects = function
        .effects
        .as_ref()
        .expect("unknown effect diagnostics require a declared effects clause");
    let mut diagnostic = Diagnostic::new(
        "effect.unknown",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("declared effect `{effect}` is not known"),
        Some(function.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("effect")),
            (
                "node_id",
                JsonValue::string(function.node_id.display(node_prefix)),
            ),
            ("effect", JsonValue::string(effect.to_string())),
            ("boundary", JsonValue::string(boundary)),
            (
                "declared_effects",
                JsonValue::array(declared_effects.iter().cloned().map(JsonValue::string)),
            ),
            (
                "known_effects",
                JsonValue::array(KNOWN_EFFECT_LABELS.iter().copied().map(JsonValue::string)),
            ),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string("Use a known effect label or remove the declaration."),
        ),
    ]));
    diagnostic
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

pub(crate) fn check_duplicate_constructor_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen =
        BTreeMap::<(Option<String>, Option<String>, String), (String, SourceSpan)>::new();

    for type_decl in &module.types {
        for variant in &type_decl.variants {
            let Some(name) = &variant.name else {
                continue;
            };
            let key = (
                type_decl.module_name.clone(),
                type_decl.name.clone(),
                name.clone(),
            );
            let node_id = variant.node_id.display("variant");
            if let Some((first_node_id, first_span)) = seen.get(&key) {
                diagnostics.push(duplicate_name_diagnostic(
                    name,
                    "constructor",
                    "constructor declaration",
                    node_id,
                    variant.span.clone(),
                    first_node_id.clone(),
                    first_span,
                ));
            } else {
                seen.insert(key, (node_id, variant.span.clone()));
            }
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
            if let Ok(ty) = parse_type_annotation(return_type)
                && !is_allowed_test_return(&ty)
            {
                diagnostics.push(test_return_diagnostic(
                    function,
                    &node_id,
                    format!("test declaration returns `{}`", ty.render()),
                    ty.render(),
                ));
            }
        }
        None => diagnostics.push(test_return_diagnostic(
            function,
            &node_id,
            "test declaration has no return type annotation".to_string(),
            "missing".to_string(),
        )),
    }

    diagnostics
}

fn is_allowed_test_return(ty: &Type) -> bool {
    ty == &Type::unit() || adt::result_parts(ty).is_some_and(|(value, _)| value == &Type::unit())
}

fn type_contains_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } => args.iter().any(type_contains_unknown),
        Type::Record(fields) => fields.iter().any(|(_, ty)| type_contains_unknown(ty)),
        Type::Function {
            params,
            return_type,
            ..
        } => params.iter().any(type_contains_unknown) || type_contains_unknown(return_type),
    }
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
            ("expected_type", JsonValue::string("() or Result<(), E>")),
            ("actual_type", JsonValue::string(actual_type)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("test_shape")),
        (
            "message",
            JsonValue::string("A test declaration returns `()` or `Result<(), E>`."),
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
    inferred_return_type: Option<Type>,
    diagnostics: Vec<Diagnostic>,
}

struct PatternBinding {
    name: String,
    ty: Type,
    node_id: NodeId,
    span: SourceSpan,
}

#[derive(Clone, Copy)]
enum MatchDomain {
    Bool,
    Adt,
}

impl MatchDomain {
    fn from_type(ty: &Type, environment: &TypeEnvironment) -> Option<Self> {
        match ty {
            Type::Named { name, args } if name == "Bool" && args.is_empty() => Some(Self::Bool),
            _ => environment.adts.descriptor_for_type(ty).map(|_| Self::Adt),
        }
    }

    fn cases(self, ty: &Type, environment: &TypeEnvironment) -> Vec<String> {
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
    fn new(function: &'a Function, environment: &'a TypeEnvironment) -> Self {
        Self {
            function,
            environment,
            bindings: Vec::new(),
            local_names: BTreeMap::new(),
            inferred_effects: Vec::new(),
            inferred_return_type: None,
            diagnostics: Vec::new(),
        }
    }

    fn check_body(&mut self) {
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
                    let actual = self.infer_expr(expr, expected.as_ref());
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
                            binding.span,
                            "local binding",
                        ) {
                            continue;
                        }
                        self.bindings.push(Binding {
                            name: binding.name,
                            ty: binding.ty,
                        });
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
        self.check_private_inference_complete();
        self.check_effect_boundaries();
    }

    fn check_implicit_unit_return(&mut self) {
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

    fn check_private_inference_complete(&mut self) {
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

    fn check_let_pattern_supported(&mut self, pattern: &Pattern) {
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

    fn validate_contract_predicate(
        &self,
        kind: ContractKind,
        predicate: &str,
    ) -> ContractValidation {
        self.validate_predicate_with_bindings(predicate, &self.contract_bindings(kind))
    }

    fn validate_predicate_with_bindings(
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

    fn validate_contract_calls(
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

    fn validate_contract_call(
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

    fn validate_contract_call_argument(
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

    fn validate_contract_referenced_names(
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

    fn contract_reference_is_resolved(
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
            || self.environment.function(name).is_some()
    }

    fn validate_whole_contract_call(
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

    fn validate_missing_contract_field(
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

    fn validate_boolean_contract_predicate(
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

    fn contract_call_signature(&self, callee: &str) -> Option<(Vec<Type>, Type, Vec<String>)> {
        self.environment
            .function_path(&contract_callee_segments(callee))
            .map(|signature| {
                (
                    signature.params.clone(),
                    signature.return_type.clone(),
                    signature.effects.clone(),
                )
            })
            .or_else(|| {
                (!callee.contains("::"))
                    .then(|| {
                        prelude_signature(callee, None)
                            .map(|(params, return_type)| (params, return_type, Vec::new()))
                    })
                    .flatten()
            })
    }

    fn predicate_arg_type(&self, arg: &str, bindings: &[Binding]) -> Type {
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
        if let Some(function) = self.environment.function_path(&segments) {
            return function.ty();
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
                if expected.is_none() && type_contains_unknown(&inferred) {
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
                    if let Some(binding) = self
                        .bindings
                        .iter()
                        .rev()
                        .find(|binding| binding.name == *name)
                    {
                        binding.ty.clone()
                    } else if let Some(function) = self.environment.function(name) {
                        function.ty()
                    } else {
                        self.push_unresolved_name(expr.node_id, expr.span.clone(), name, "value");
                        Type::Unknown
                    }
                }
                _ => {
                    if let Some(function) = self.environment.function_path(segments) {
                        return function.ty();
                    }
                    let symbol = segments.join("::");
                    self.push_unresolved_name(expr.node_id, expr.span.clone(), &symbol, "value");
                    Type::Unknown
                }
            },
        }
    }

    fn infer_call(
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

    fn infer_constructor_call(
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

    fn infer_declared_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        let (params, return_type, origin) = self.call_signature(
            callee,
            expected.map(|expected| &expected.ty),
            args.first()
                .and_then(|arg| self.shallow_expr_type(arg))
                .as_ref(),
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
        Some(return_type)
    }

    fn infer_prelude_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        let ExprKind::NamePath(segments) = &callee.kind else {
            return None;
        };
        let [name] = segments.as_slice() else {
            return None;
        };
        let (params, return_type) = prelude_signature(name, expected.map(|expected| &expected.ty))?;

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
            self.check_prelude_argument_assignable(name, index, arg, &expected, &actual);
        }
        Some(return_type)
    }

    fn diagnose_method_call(&mut self, expr: &Expr, callee: &Expr, args: &[Expr]) -> Option<Type> {
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

    fn infer_unresolved_call(&mut self, callee: &Expr, args: &[Expr]) -> Type {
        if let Some((segments, _)) = callee_name_path_and_type_args(callee) {
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

    fn call_signature(
        &self,
        callee: &Expr,
        expected: Option<&Type>,
        handle_type: Option<&Type>,
    ) -> Option<(Vec<Type>, Type, CallOrigin)> {
        match &callee.kind {
            ExprKind::NamePath(segments) => {
                if let Some(origin) = stdio_signature(segments, callee) {
                    return Some((vec![Type::string()], Type::unit(), origin));
                }
                if let Some(origin) = concurrency_origin(segments, callee) {
                    let (params, return_type) =
                        concurrency_signature(segments, expected, handle_type, None)?;
                    return Some((params, return_type, origin));
                }
                if let Some(origin) = standard_library_origin(segments, callee) {
                    let (params, return_type) = standard_library_signature(segments)?;
                    return Some((params, return_type, origin));
                }
                if let [name] = segments.as_slice()
                    && let Some(binding) = self
                        .bindings
                        .iter()
                        .rev()
                        .find(|binding| binding.name == *name)
                {
                    let (params, return_type) = binding.ty.function_parts()?;
                    let effects = binding.ty.function_effects().unwrap_or_default().to_vec();
                    return Some((
                        params.to_vec(),
                        return_type.clone(),
                        CallOrigin {
                            node_id: callee.node_id,
                            span: callee.span.clone(),
                            symbol: name.clone(),
                            effects,
                        },
                    ));
                }
                self.environment.function_path(segments).map(|function| {
                    (
                        function.params.clone(),
                        function.return_type.clone(),
                        CallOrigin {
                            node_id: function.node_id,
                            span: function.span.clone(),
                            symbol: segments.join("::"),
                            effects: function.effects.clone(),
                        },
                    )
                })
            }
            ExprKind::TypeApply { .. } => {
                let (segments, type_args) = type_applied_name_path(callee)?;
                if let Some(origin) = concurrency_origin(segments, callee) {
                    let explicit_item = type_args
                        .first()
                        .and_then(|type_arg| parse_type_annotation(type_arg).ok());
                    let (params, return_type) = concurrency_signature(
                        segments,
                        expected,
                        handle_type,
                        explicit_item.as_ref(),
                    )?;
                    return Some((params, return_type, origin));
                }
                None
            }
            _ => None,
        }
    }

    fn infer_adt_constructor(
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

    fn infer_list(&mut self, expr: &Expr, items: &[Expr], expected: Option<&ExpectedType>) -> Type {
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

    fn infer_match(
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

    fn check_match_exhaustiveness(
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

    fn pattern_bindings(
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
                let ConstructorLookup::Found(constructor) = self.environment.adts.constructor(
                    name,
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
        if let Some(expected) = expected
            && matches!(expected.ty, Type::Record(_))
        {
            return expected.ty.clone();
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

    fn check_prelude_argument_assignable(
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
                            .function(name)
                            .map(|function| function.ty())
                    }),
                _ => None,
            },
            ExprKind::Call { callee, .. } => self
                .call_signature(callee, None, None)
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

    fn push_ambiguous_name(
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

    fn push_ambiguous_constructor_type(
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

    fn hole_constraints(
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

    fn constraint_has_assignable_candidate(
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

    fn candidate_queries(
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

    fn ranked_symbol_candidates(
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

    fn satisfy_repair_constraint(
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

    fn valid_static_satisfy_predicate(&self, satisfy: &SatisfyClause, expected: &Type) -> bool {
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

fn contract_callee_segments(callee: &str) -> Vec<String> {
    callee.split("::").map(ToString::to_string).collect()
}

struct SatisfyRepairConstraint {
    allowed_bindings: Option<Vec<SatisfyAllowedBinding>>,
    reason: &'static str,
}

struct SatisfyAllowedBinding {
    name: String,
    reason: &'static str,
}

impl SatisfyRepairConstraint {
    fn from_satisfy(satisfy: &SatisfyClause, allow_static_truth: bool) -> Option<Self> {
        let candidate = satisfy.candidate.as_ref()?;
        if let Some(tautology) = tautological_candidate_predicate(&satisfy.predicate, candidate) {
            return Some(Self {
                allowed_bindings: None,
                reason: tautology.reason,
            });
        }
        if allow_static_truth
            && predicate_is_statically_true_with_literal_bounds(&satisfy.predicate)
        {
            return Some(Self {
                allowed_bindings: None,
                reason: "satisfy_tautology",
            });
        }
        if let Some(bindings) = reflexive_candidate_disjunct_bindings(&satisfy.predicate, candidate)
        {
            return Some(Self {
                allowed_bindings: Some(bindings),
                reason: "satisfy_reflexive_match",
            });
        }
        reflexive_candidate_binding(&satisfy.predicate, candidate).map(|allowed| Self {
            allowed_bindings: Some(vec![SatisfyAllowedBinding {
                name: allowed.binding,
                reason: allowed.reason,
            }]),
            reason: allowed.reason,
        })
    }

    fn reason_for(&self, binding: &str) -> Option<&'static str> {
        match &self.allowed_bindings {
            Some(allowed) => allowed
                .iter()
                .find(|allowed_binding| allowed_binding.name == binding)
                .map(|allowed_binding| allowed_binding.reason),
            None => Some(self.reason),
        }
    }

    fn allows_any_binding(&self) -> bool {
        self.allowed_bindings.is_none()
    }

    fn extend_allowed_bindings(&mut self, bindings: Vec<SatisfyAllowedBinding>) {
        let Some(allowed) = &mut self.allowed_bindings else {
            return;
        };
        for binding in bindings {
            if !allowed.iter().any(|existing| existing.name == binding.name) {
                allowed.push(binding);
            }
        }
    }
}

fn reflexive_candidate_disjunct_bindings(
    predicate: &str,
    candidate: &str,
) -> Option<Vec<SatisfyAllowedBinding>> {
    if repair_relevant_negated_and_clauses(predicate).is_some() {
        return None;
    }
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.len() <= 1 {
        return reflexive_candidate_conjunction_bindings(
            repair_relevant_and_clauses(predicate),
            candidate,
        );
    }
    let mut bindings = Vec::new();
    for disjunct in disjuncts {
        let Some(direct) = reflexive_candidate_conjunction_bindings(
            repair_relevant_and_clauses(disjunct),
            candidate,
        ) else {
            continue;
        };
        for direct_allowed in direct {
            if !bindings
                .iter()
                .any(|binding: &SatisfyAllowedBinding| binding.name == direct_allowed.name)
            {
                bindings.push(direct_allowed);
            }
        }
    }
    (!bindings.is_empty()).then_some(bindings)
}

struct ReflexiveCandidateBinding {
    binding: String,
    reason: &'static str,
}

fn reflexive_candidate_binding(
    predicate: &str,
    candidate: &str,
) -> Option<ReflexiveCandidateBinding> {
    let expanded_disjuncts = repair_relevant_negated_and_clauses(predicate);
    let disjuncts = expanded_disjuncts.as_deref().map_or_else(
        || repair_relevant_or_clauses(predicate),
        |clauses| clauses.iter().map(String::as_str).collect(),
    );
    if disjuncts.is_empty() {
        return None;
    }
    if disjuncts.len() > 1 {
        return reflexive_candidate_disjunction(disjuncts, candidate);
    }
    reflexive_candidate_conjunction(repair_relevant_and_clauses(disjuncts[0]), candidate)
}

fn reflexive_candidate_disjunction(
    disjuncts: Vec<&str>,
    candidate: &str,
) -> Option<ReflexiveCandidateBinding> {
    let mut allowed_binding = None::<String>;
    let mut reason = "satisfy_equality_match";
    for disjunct in disjuncts {
        let direct =
            reflexive_candidate_conjunction(repair_relevant_and_clauses(disjunct), candidate)?;
        if let Some(existing) = &allowed_binding {
            if existing != &direct.binding {
                return None;
            }
        } else {
            allowed_binding = Some(direct.binding);
        }
        if direct.reason != "satisfy_equality_match" {
            reason = direct.reason;
        }
    }
    allowed_binding.map(|binding| ReflexiveCandidateBinding { binding, reason })
}

fn reflexive_candidate_conjunction(
    clauses: Vec<String>,
    candidate: &str,
) -> Option<ReflexiveCandidateBinding> {
    let allowed_bindings = reflexive_candidate_conjunction_bindings(clauses, candidate)?;
    let [allowed] = allowed_bindings.as_slice() else {
        return None;
    };
    Some(ReflexiveCandidateBinding {
        binding: allowed.name.clone(),
        reason: allowed.reason,
    })
}

fn reflexive_candidate_conjunction_bindings(
    clauses: Vec<String>,
    candidate: &str,
) -> Option<Vec<SatisfyAllowedBinding>> {
    let mut allowed_bindings = None::<Vec<SatisfyAllowedBinding>>;
    for clause in clauses {
        if is_surplus_tautology_clause(&clause, candidate) {
            continue;
        }
        let direct = reflexive_candidate_clause_bindings(&clause, candidate)?;
        if let Some(existing) = &mut allowed_bindings {
            existing.retain(|allowed| {
                direct
                    .iter()
                    .any(|direct_allowed| direct_allowed.name == allowed.name)
            });
            for allowed in existing.iter_mut() {
                if allowed.reason == "satisfy_equality_match"
                    && let Some(direct_allowed) = direct
                        .iter()
                        .find(|direct_allowed| direct_allowed.name == allowed.name)
                {
                    allowed.reason = direct_allowed.reason;
                }
            }
            if existing.is_empty() {
                return None;
            }
        } else {
            allowed_bindings = Some(direct);
        }
    }
    allowed_bindings
}

fn reflexive_candidate_clause_bindings(
    clause: &str,
    candidate: &str,
) -> Option<Vec<SatisfyAllowedBinding>> {
    let disjuncts = repair_relevant_or_clauses(clause);
    if disjuncts.len() > 1 {
        let bindings = disjuncts
            .into_iter()
            .filter_map(|disjunct| direct_reflexive_clause(disjunct, candidate))
            .fold(
                Vec::<SatisfyAllowedBinding>::new(),
                |mut bindings, direct| {
                    if !bindings
                        .iter()
                        .any(|binding| binding.name == direct.binding)
                    {
                        bindings.push(SatisfyAllowedBinding {
                            name: direct.binding,
                            reason: direct.reason,
                        });
                    }
                    bindings
                },
            );
        return (!bindings.is_empty()).then_some(bindings);
    }
    direct_reflexive_clause(clause, candidate).map(|direct| {
        vec![SatisfyAllowedBinding {
            name: direct.binding,
            reason: direct.reason,
        }]
    })
}

struct TautologicalCandidatePredicate {
    reason: &'static str,
}

fn tautological_candidate_predicate(
    predicate: &str,
    candidate: &str,
) -> Option<TautologicalCandidatePredicate> {
    if has_true_disjunct(predicate) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if repair_relevant_negated_and_clauses(predicate)
        .as_deref()
        .is_some_and(|clauses| clauses.iter().any(|clause| clause == "true"))
    {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if repair_relevant_negated_and_clauses(predicate)
        .as_deref()
        .is_some_and(|clauses| {
            let disjuncts = clauses.iter().map(String::as_str).collect::<Vec<_>>();
            has_complementary_candidate_disjuncts(&disjuncts, candidate)
        })
    {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if negated_and_clauses(predicate)
        .is_some_and(|clauses| has_exclusive_order_candidate_conjuncts(&clauses, candidate))
    {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if negated_and_clauses(predicate).is_some_and(|clauses| {
        has_exclusive_inclusive_order_candidate_conjuncts(&clauses, candidate)
    }) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.is_empty() {
        return None;
    }
    if has_complementary_candidate_disjuncts(&disjuncts, candidate) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if has_inclusive_total_order_candidate_disjuncts(&disjuncts, candidate) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if has_total_order_candidate_disjuncts(&disjuncts, candidate) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if disjuncts
        .into_iter()
        .any(|disjunct| is_candidate_tautology_disjunct(disjunct, candidate))
    {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    None
}

fn has_complementary_candidate_disjuncts(disjuncts: &[&str], candidate: &str) -> bool {
    if has_complementary_candidate_comparison_disjuncts(disjuncts, candidate) {
        return true;
    }
    let mut positive = Vec::<String>::new();
    let mut negative = Vec::<String>::new();
    for disjunct in disjuncts {
        let Some((negated, clause)) = complementary_disjunct_key(disjunct, candidate) else {
            continue;
        };
        let complements = if negated { &positive } else { &negative };
        if complements.iter().any(|existing| existing == &clause) {
            return true;
        }
        if negated {
            negative.push(clause);
        } else {
            positive.push(clause);
        }
    }
    false
}

fn has_complementary_candidate_comparison_disjuncts(disjuncts: &[&str], candidate: &str) -> bool {
    disjuncts.iter().enumerate().any(|(index, left)| {
        disjuncts
            .iter()
            .skip(index + 1)
            .any(|right| complementary_candidate_comparisons(left, right, candidate))
    })
}

fn complementary_candidate_comparisons(left: &str, right: &str, candidate: &str) -> bool {
    if !expression_references_identifier(left, candidate)
        && !expression_references_identifier(right, candidate)
    {
        return false;
    }
    let Some(left) = NormalizedRepairComparison::parse(left) else {
        return false;
    };
    let Some(right) = NormalizedRepairComparison::parse(right) else {
        return false;
    };
    match (left.operator, right.operator) {
        ("==", "!=") | ("!=", "==") => left.same_operands_unordered(&right),
        ("<", "<=") | ("<=", "<") => {
            left.same_operands_reversed(&right) || left.same_operands_unordered(&right)
        }
        _ => false,
    }
}

fn has_inclusive_total_order_candidate_disjuncts(disjuncts: &[&str], candidate: &str) -> bool {
    disjuncts.iter().enumerate().any(|(index, left)| {
        let Some(left) = inclusive_total_order_candidate_clause(left, candidate) else {
            return false;
        };
        disjuncts
            .iter()
            .skip(index + 1)
            .filter_map(|right| inclusive_total_order_candidate_clause(right, candidate))
            .any(|right| left.left == right.right && left.right == right.left)
    })
}

struct InclusiveTotalOrderCandidateClause {
    left: String,
    right: String,
}

fn inclusive_total_order_candidate_clause(
    disjunct: &str,
    candidate: &str,
) -> Option<InclusiveTotalOrderCandidateClause> {
    if !expression_references_identifier(disjunct, candidate) {
        return None;
    }
    let parsed = NormalizedRepairComparison::parse(disjunct)?;
    if parsed.operator != "<=" {
        return None;
    }
    let left = compact_predicate_text(parsed.left);
    let right = compact_predicate_text(parsed.right);
    (left != right).then_some(InclusiveTotalOrderCandidateClause { left, right })
}

fn has_total_order_candidate_disjuncts(disjuncts: &[&str], candidate: &str) -> bool {
    if disjuncts.len() < 3 {
        return false;
    }
    disjuncts.iter().enumerate().any(|(index, disjunct)| {
        let Some(first) = total_order_candidate_clause(disjunct, candidate) else {
            return false;
        };
        disjuncts
            .iter()
            .skip(index + 1)
            .filter_map(|other| total_order_candidate_clause(other, candidate))
            .filter(|other| other.left == first.left && other.right == first.right)
            .fold(first.relation.bit(), |mask, other| {
                mask | other.relation.bit()
            })
            == TotalOrderRelation::ALL_BITS
    })
}

fn has_exclusive_order_candidate_conjuncts(conjuncts: &[&str], candidate: &str) -> bool {
    if conjuncts.len() < 2 {
        return false;
    }
    conjuncts.iter().enumerate().any(|(index, conjunct)| {
        let Some(first) = total_order_candidate_clause(conjunct, candidate) else {
            return false;
        };
        conjuncts
            .iter()
            .skip(index + 1)
            .filter_map(|other| total_order_candidate_clause(other, candidate))
            .any(|other| {
                other.left == first.left
                    && other.right == first.right
                    && other.relation != first.relation
            })
    })
}

fn has_exclusive_inclusive_order_candidate_conjuncts(conjuncts: &[&str], candidate: &str) -> bool {
    if conjuncts.len() < 2 {
        return false;
    }
    conjuncts.iter().enumerate().any(|(index, conjunct)| {
        let Some(first) = order_bound_candidate_clause(conjunct, candidate) else {
            return false;
        };
        conjuncts
            .iter()
            .skip(index + 1)
            .filter_map(|other| order_bound_candidate_clause(other, candidate))
            .any(|other| {
                first.left == other.right
                    && first.right == other.left
                    && (first.strict || other.strict)
            })
    })
}

struct OrderBoundCandidateClause {
    left: String,
    right: String,
    strict: bool,
}

fn order_bound_candidate_clause(
    conjunct: &str,
    candidate: &str,
) -> Option<OrderBoundCandidateClause> {
    let conjunct = strip_balanced_outer_parens(conjunct);
    if !expression_references_identifier(conjunct, candidate) {
        return None;
    }
    let parsed = ParsedRepairComparison::parse(conjunct)?;
    let mut left = compact_predicate_text(parsed.left);
    let mut right = compact_predicate_text(parsed.right);
    if left == right {
        return None;
    }
    match parsed.operator {
        ">" | ">=" => std::mem::swap(&mut left, &mut right),
        "<" | "<=" => {}
        _ => return None,
    }
    Some(OrderBoundCandidateClause {
        left,
        right,
        strict: matches!(parsed.operator, "<" | ">"),
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TotalOrderRelation {
    Less,
    Equal,
    Greater,
}

impl TotalOrderRelation {
    const ALL_BITS: u8 = Self::Less.bit() | Self::Equal.bit() | Self::Greater.bit();

    const fn bit(self) -> u8 {
        match self {
            Self::Less => 0b001,
            Self::Equal => 0b010,
            Self::Greater => 0b100,
        }
    }

    fn invert(self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::Equal => Self::Equal,
            Self::Greater => Self::Less,
        }
    }
}

struct TotalOrderCandidateClause {
    left: String,
    right: String,
    relation: TotalOrderRelation,
}

fn total_order_candidate_clause(
    disjunct: &str,
    candidate: &str,
) -> Option<TotalOrderCandidateClause> {
    let disjunct = strip_balanced_outer_parens(disjunct);
    if !expression_references_identifier(disjunct, candidate) {
        return None;
    }
    let parsed = ParsedRepairComparison::parse(disjunct)?;
    let mut left = compact_predicate_text(parsed.left);
    let mut right = compact_predicate_text(parsed.right);
    if left == right {
        return None;
    }
    let mut relation = match parsed.operator {
        "==" => TotalOrderRelation::Equal,
        "<" => TotalOrderRelation::Less,
        ">" => TotalOrderRelation::Greater,
        _ => return None,
    };
    if right < left {
        std::mem::swap(&mut left, &mut right);
        relation = relation.invert();
    }
    Some(TotalOrderCandidateClause {
        left,
        right,
        relation,
    })
}

fn complementary_disjunct_key(disjunct: &str, candidate: &str) -> Option<(bool, String)> {
    let disjunct = strip_balanced_outer_parens(disjunct);
    let (negated, clause) = stripped_not_operand(disjunct)
        .map(|inner| (true, strip_balanced_outer_parens(inner)))
        .unwrap_or((false, disjunct));
    expression_references_identifier(clause, candidate)
        .then(|| (negated, canonical_repair_clause(clause)))
}

fn has_true_disjunct(predicate: &str) -> bool {
    split_top_level_keyword(strip_balanced_outer_parens(predicate), "or")
        .into_iter()
        .any(|clause| normalized_predicate_clause(clause) == "true")
}

fn is_candidate_tautology_disjunct(predicate: &str, candidate: &str) -> bool {
    let clauses = repair_relevant_and_clauses(predicate);
    !clauses.is_empty()
        && clauses.iter().all(|clause| {
            is_surplus_tautology_clause(clause, candidate)
                || is_candidate_tautology_clause(clause, candidate)
        })
}

fn is_surplus_tautology_clause(clause: &str, candidate: &str) -> bool {
    has_true_disjunct(clause)
        || predicate_is_statically_true(clause)
        || has_complementary_candidate_disjuncts(&repair_relevant_or_clauses(clause), candidate)
}

fn is_candidate_tautology_clause(predicate: &str, candidate: &str) -> bool {
    let predicate = single_repair_relevant_clause(predicate).unwrap_or(predicate);
    let predicate = canonical_repair_clause(predicate);
    ["==", "<="].iter().any(|operator| {
        let Some((left, right)) = predicate.split_once(operator) else {
            return false;
        };
        if tautological_candidate_expression(left, right, candidate) {
            return true;
        }
        let Some(left) = operand_path(left) else {
            return false;
        };
        let Some(right) = operand_path(right) else {
            return false;
        };
        left.first().is_some_and(|base| *base == candidate) && left == right
    })
}

fn tautological_candidate_expression(left: &str, right: &str, candidate: &str) -> bool {
    compact_direct_repair_expression_text(left) == compact_direct_repair_expression_text(right)
        && expression_references_identifier(left, candidate)
}

fn direct_reflexive_clause(predicate: &str, candidate: &str) -> Option<ReflexiveCandidateBinding> {
    let predicate = single_repair_relevant_clause(predicate).unwrap_or(predicate);
    let predicate = canonical_repair_clause(predicate);
    if let Some(binding) = reflexive_operand(&predicate, candidate, "==") {
        return Some(ReflexiveCandidateBinding {
            binding,
            reason: "satisfy_equality_match",
        });
    }
    if let Some(binding) = reflexive_expression_operand(&predicate, candidate, "==") {
        return Some(ReflexiveCandidateBinding {
            binding,
            reason: "satisfy_equality_match",
        });
    }
    if let Some(binding) = reflexive_operand(&predicate, candidate, "<=") {
        return Some(ReflexiveCandidateBinding {
            binding,
            reason: "satisfy_reflexive_match",
        });
    }
    if let Some(binding) = reflexive_expression_operand(&predicate, candidate, "<=") {
        return Some(ReflexiveCandidateBinding {
            binding,
            reason: "satisfy_reflexive_match",
        });
    }
    None
}

fn reflexive_operand(predicate: &str, candidate: &str, operator: &str) -> Option<String> {
    let (left, right) = predicate.split_once(operator)?;
    let left = operand_path(left)?;
    let right = operand_path(right)?;
    reflexive_path_binding(&left, &right, candidate)
        .or_else(|| reflexive_path_binding(&right, &left, candidate))
}

fn is_plain_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn operand_path(value: &str) -> Option<Vec<&str>> {
    value
        .trim()
        .split('.')
        .map(str::trim)
        .map(|segment| is_plain_identifier(segment).then_some(segment))
        .collect()
}

fn reflexive_path_binding(left: &[&str], right: &[&str], candidate: &str) -> Option<String> {
    let (Some(left_base), Some(right_base)) = (left.first(), right.first()) else {
        return None;
    };
    if *left_base != candidate || *right_base == candidate || !is_plain_identifier(right_base) {
        return None;
    }
    (left[1..] == right[1..]).then(|| (*right_base).to_string())
}

fn reflexive_expression_operand(
    predicate: &str,
    candidate: &str,
    operator: &str,
) -> Option<String> {
    let (left, right) = predicate.split_once(operator)?;
    reflexive_expression_binding(left, right, candidate)
        .or_else(|| reflexive_expression_binding(right, left, candidate))
}

fn reflexive_expression_binding(
    candidate_expr: &str,
    binding_expr: &str,
    candidate: &str,
) -> Option<String> {
    if !expression_references_identifier(candidate_expr, candidate)
        || expression_references_identifier(binding_expr, candidate)
    {
        return None;
    }
    let matching_bindings = expression_identifiers(binding_expr)
        .into_iter()
        .filter(|binding| *binding != candidate)
        .filter(|binding| {
            is_plain_identifier(binding)
                && compact_direct_repair_expression_text(&replace_identifier(
                    candidate_expr,
                    candidate,
                    binding,
                )) == compact_direct_repair_expression_text(binding_expr)
        })
        .collect::<Vec<_>>();
    match matching_bindings.as_slice() {
        [binding] => Some((*binding).to_string()),
        _ => None,
    }
}

fn expression_references_identifier(expression: &str, name: &str) -> bool {
    expression_identifiers(expression)
        .into_iter()
        .any(|identifier| identifier == name)
}

fn expression_identifiers(expression: &str) -> Vec<&str> {
    let mut identifiers = Vec::new();
    let mut chars = expression.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch == '"' {
            let mut escaped = false;
            for (_, string_ch) in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if string_ch == '\\' {
                    escaped = true;
                } else if string_ch == '"' {
                    break;
                }
            }
        } else if is_ident_start(ch) {
            let mut end = start + ch.len_utf8();
            while let Some((next, next_ch)) = chars.peek().copied() {
                if !is_ident_continue(next_ch) {
                    break;
                }
                chars.next();
                end = next + next_ch.len_utf8();
            }
            let ident = &expression[start..end];
            if is_value_identifier_position(expression, start, end)
                && !identifiers.iter().any(|existing| existing == &ident)
            {
                identifiers.push(ident);
            }
        }
    }
    identifiers
}

fn compact_predicate_text(predicate: &str) -> String {
    let mut output = String::with_capacity(predicate.len());
    let mut chars = predicate.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            output.push(ch);
            let mut escaped = false;
            for string_ch in chars.by_ref() {
                output.push(string_ch);
                if escaped {
                    escaped = false;
                } else if string_ch == '\\' {
                    escaped = true;
                } else if string_ch == '"' {
                    break;
                }
            }
        } else if !ch.is_whitespace() {
            output.push(ch);
        }
    }
    output
}

fn compact_direct_repair_expression_text(predicate: &str) -> String {
    let mut current = compact_predicate_text(predicate);
    loop {
        let stripped = strip_redundant_repair_atom_parens(&current);
        if stripped == current {
            return current;
        }
        current = stripped;
    }
}

fn strip_redundant_repair_atom_parens(predicate: &str) -> String {
    let mut output = String::with_capacity(predicate.len());
    let mut cursor = 0;
    while cursor < predicate.len() {
        let rest = &predicate[cursor..];
        if let Some(inner_start) = rest.strip_prefix('(')
            && let Some(end) = inner_start.find(')')
        {
            let inner = &inner_start[..end];
            if is_repair_atom_text(inner) {
                output.push_str(inner);
                cursor += end + 2;
                continue;
            }
        }
        let ch = rest
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        output.push(ch);
        cursor += ch.len_utf8();
    }
    output
}

fn is_repair_atom_text(text: &str) -> bool {
    operand_path(text).is_some() || repair_numeric_order_literal(text).is_some()
}

fn normalized_and_clauses(predicate: &str) -> Vec<String> {
    split_top_level_keyword(strip_balanced_outer_parens(predicate), "and")
        .into_iter()
        .map(normalized_predicate_clause)
        .filter(|clause| !clause.is_empty())
        .collect()
}

fn repair_relevant_and_clauses(predicate: &str) -> Vec<String> {
    normalized_and_clauses(predicate)
        .into_iter()
        .flat_map(|clause| {
            canonical_negated_disjunction_repair_clauses(&clause).unwrap_or_else(|| vec![clause])
        })
        .filter(|clause| clause != "true" && !contract_predicate_is_statically_true(clause))
        .collect()
}

fn repair_relevant_or_clauses(predicate: &str) -> Vec<&str> {
    let clauses = split_top_level_keyword(strip_balanced_outer_parens(predicate), "or");
    let has_disjunction = clauses.len() > 1;
    clauses
        .into_iter()
        .filter(|clause| {
            normalized_predicate_clause(clause) != "false"
                && (!has_disjunction || !predicate_is_statically_false(clause))
        })
        .collect()
}

fn single_repair_relevant_clause(predicate: &str) -> Option<&str> {
    let clauses = repair_relevant_or_clauses(predicate);
    match clauses.as_slice() {
        [clause] => Some(*clause),
        _ => None,
    }
}

fn repair_relevant_negated_and_clauses(predicate: &str) -> Option<Vec<String>> {
    let conjuncts = negated_and_clauses(predicate)?;
    if conjuncts.len() <= 1 {
        return None;
    }
    let clauses = conjuncts
        .into_iter()
        .map(|conjunct| canonical_negated_repair_or_atom_clause(&format!("not ({conjunct})")))
        .collect::<Option<Vec<_>>>()?;
    if clauses.iter().any(|clause| clause == "true") {
        return Some(vec!["true".to_string()]);
    }
    let clauses = clauses
        .into_iter()
        .filter(|clause| clause != "false")
        .collect::<Vec<_>>();
    Some(if clauses.is_empty() {
        vec!["false".to_string()]
    } else {
        clauses
    })
}

fn negated_and_clauses(predicate: &str) -> Option<Vec<&str>> {
    let trimmed = predicate.trim();
    let negated = if let Some(negated) = trimmed.strip_prefix("not ") {
        negated
    } else {
        trimmed
            .strip_prefix("not(")
            .map(|negated| negated.strip_suffix(')').unwrap_or(negated).trim())?
    };
    Some(flattened_repair_keyword_clauses(negated, "and"))
}

fn flattened_repair_keyword_clauses<'a>(predicate: &'a str, keyword: &str) -> Vec<&'a str> {
    let clauses = split_top_level_keyword(strip_balanced_outer_parens(predicate), keyword);
    if clauses.len() <= 1 {
        return clauses;
    }
    clauses
        .into_iter()
        .flat_map(|clause| flattened_repair_keyword_clauses(clause, keyword))
        .collect()
}

fn predicate_guaranteed_by_required_predicates(
    predicate: &str,
    required_predicates: &[String],
) -> bool {
    if required_predicate_set_statically_implies_predicate(required_predicates, predicate) {
        return true;
    }
    if required_predicates
        .iter()
        .any(|required| required_predicate_implies_disjunctive_predicate(required, predicate))
    {
        return true;
    }
    if required_predicate_set_implies_disjunctive_predicate(required_predicates, predicate) {
        return true;
    }
    repair_relevant_or_clause_strings(predicate)
        .into_iter()
        .map(|disjunct| repair_relevant_and_clauses(&disjunct))
        .any(|disjunct_clauses| {
            !disjunct_clauses.is_empty()
                && disjunct_clauses.iter().all(|clause| {
                    repair_clause_guaranteed_by_required_predicates(clause, required_predicates)
                })
        })
}

fn int_successor_predicate_guaranteed_by_required_predicates(
    predicate: &str,
    required_predicates: &[String],
) -> bool {
    repair_relevant_or_clause_strings(predicate)
        .into_iter()
        .map(|disjunct| repair_relevant_and_clauses(&disjunct))
        .any(|disjunct_clauses| {
            !disjunct_clauses.is_empty()
                && disjunct_clauses.iter().all(|clause| {
                    repair_clause_guaranteed_by_required_predicates(clause, required_predicates)
                        || int_successor_clause_guaranteed_by_required_predicates(
                            clause,
                            required_predicates,
                        )
                })
        })
}

fn int_successor_clause_guaranteed_by_required_predicates(
    clause: &str,
    required_predicates: &[String],
) -> bool {
    let required_clauses = required_predicates
        .iter()
        .flat_map(|predicate| repair_set_clauses(predicate))
        .collect::<Vec<_>>();
    if repair_clause_set_int_successor_implies_clause(&required_clauses, clause) {
        return true;
    }
    required_predicates
        .iter()
        .any(|required| required_predicate_int_successor_implies_clause(required, clause))
}

fn repair_clause_set_int_successor_implies_clause(
    required_clauses: &[String],
    wanted: &str,
) -> bool {
    let Some(wanted) = NormalizedRepairComparison::parse(wanted) else {
        return false;
    };
    let equivalences = repair_equivalences(required_clauses);
    required_clauses.iter().any(|required| {
        let Some(required) = NormalizedRepairComparison::parse(required) else {
            return false;
        };
        int_successor_repair_comparison_implies(&required, &wanted, &equivalences)
    })
}

fn required_predicate_int_successor_implies_clause(predicate: &str, wanted: &str) -> bool {
    let predicate = strip_balanced_outer_parens(predicate);
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.len() > 1 {
        return disjuncts
            .into_iter()
            .all(|disjunct| required_predicate_int_successor_implies_clause(disjunct, wanted));
    }
    if disjuncts.is_empty() {
        return false;
    }
    let conjuncts = split_top_level_keyword(disjuncts[0], "and");
    if conjuncts.len() > 1 {
        return conjuncts
            .into_iter()
            .any(|conjunct| required_predicate_int_successor_implies_clause(conjunct, wanted));
    }
    let canonical = canonical_repair_clause(disjuncts[0]);
    int_successor_repair_clause_implies(&canonical, wanted)
}

fn int_successor_repair_clause_implies(required: &str, wanted: &str) -> bool {
    let Some(required) = NormalizedRepairComparison::parse(required) else {
        return false;
    };
    let Some(wanted) = NormalizedRepairComparison::parse(wanted) else {
        return false;
    };
    int_successor_repair_comparison_implies(&required, &wanted, &RepairEquivalences::default())
}

fn int_successor_repair_comparison_implies(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    match (required.operator, wanted.operator) {
        ("<", "<=") => strict_int_bound_implies_adjacent_inclusive(required, wanted, equivalences),
        ("<=", "<") => inclusive_int_bound_implies_adjacent_strict(required, wanted, equivalences),
        _ => false,
    }
}

fn strict_int_bound_implies_adjacent_inclusive(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    strict_int_lower_bound_implies_adjacent_inclusive(required, wanted, equivalences)
        || strict_int_upper_bound_implies_adjacent_inclusive(required, wanted, equivalences)
}

fn inclusive_int_bound_implies_adjacent_strict(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    inclusive_int_lower_bound_implies_adjacent_strict(required, wanted, equivalences)
        || inclusive_int_upper_bound_implies_adjacent_strict(required, wanted, equivalences)
}

fn strict_int_lower_bound_implies_adjacent_inclusive(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required.right, wanted.right, equivalences)
        && repair_numeric_order_literal(required.left).is_some_and(|required_literal| {
            required_literal.is_integer()
                && repair_numeric_order_literal(wanted.left).is_some_and(|wanted_literal| {
                    wanted_literal.is_integer()
                        && Some(wanted_literal) == required_literal.add_int(1)
                })
        })
}

fn strict_int_upper_bound_implies_adjacent_inclusive(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required.left, wanted.left, equivalences)
        && repair_numeric_order_literal(required.right).is_some_and(|required_literal| {
            required_literal.is_integer()
                && repair_numeric_order_literal(wanted.right).is_some_and(|wanted_literal| {
                    wanted_literal.is_integer()
                        && Some(wanted_literal) == required_literal.add_int(-1)
                })
        })
}

fn inclusive_int_lower_bound_implies_adjacent_strict(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required.right, wanted.right, equivalences)
        && repair_numeric_order_literal(required.left).is_some_and(|required_literal| {
            required_literal.is_integer()
                && repair_numeric_order_literal(wanted.left).is_some_and(|wanted_literal| {
                    wanted_literal.is_integer()
                        && Some(wanted_literal) == required_literal.add_int(-1)
                })
        })
}

fn inclusive_int_upper_bound_implies_adjacent_strict(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required.left, wanted.left, equivalences)
        && repair_numeric_order_literal(required.right).is_some_and(|required_literal| {
            required_literal.is_integer()
                && repair_numeric_order_literal(wanted.right).is_some_and(|wanted_literal| {
                    wanted_literal.is_integer()
                        && Some(wanted_literal) == required_literal.add_int(1)
                })
        })
}

fn required_predicate_set_statically_implies_predicate(
    required_predicates: &[String],
    predicate: &str,
) -> bool {
    if required_predicates.is_empty() {
        return false;
    }
    let antecedent = required_predicates
        .iter()
        .map(|required| format!("({required})"))
        .collect::<Vec<_>>()
        .join(" and ");
    contract_predicate_is_statically_true(&format!("not ({antecedent}) or ({predicate})"))
}

fn required_predicate_implies_disjunctive_predicate(required: &str, wanted: &str) -> bool {
    let wanted_disjuncts = repair_relevant_or_clauses(wanted)
        .into_iter()
        .map(canonical_repair_clause)
        .collect::<Vec<_>>();
    if wanted_disjuncts.len() <= 1 {
        return false;
    }
    let required_disjuncts = repair_relevant_negated_and_clauses(required).unwrap_or_else(|| {
        repair_relevant_or_clauses(required)
            .into_iter()
            .map(canonical_repair_clause)
            .collect()
    });
    if required_disjuncts.len() <= 1 {
        return false;
    }
    required_disjuncts.iter().all(|required_disjunct| {
        wanted_disjuncts
            .iter()
            .any(|wanted_disjunct| repair_clause_implies(required_disjunct, wanted_disjunct))
    })
}

fn required_predicate_set_implies_disjunctive_predicate(
    required_predicates: &[String],
    wanted: &str,
) -> bool {
    let wanted_disjuncts = repair_relevant_or_clauses(wanted)
        .into_iter()
        .map(canonical_repair_clause)
        .collect::<Vec<_>>();
    if wanted_disjuncts.len() <= 1 {
        return false;
    }
    if disjunctive_branch_set_implies_disjunctive_predicate(required_predicates, &wanted_disjuncts)
    {
        return true;
    }
    let required_clauses = required_predicates
        .iter()
        .flat_map(|predicate| repair_set_clauses(predicate))
        .collect::<Vec<_>>();
    let equivalences = repair_equivalences(&required_clauses);
    required_clauses.iter().any(|required| {
        disequality_implies_numeric_ordering_disjunction(required, &wanted_disjuncts, &equivalences)
            || inclusive_bound_implies_order_or_equality_disjunction(
                required,
                &wanted_disjuncts,
                &equivalences,
            )
    })
}

fn disjunctive_branch_set_implies_disjunctive_predicate(
    required_predicates: &[String],
    wanted_disjuncts: &[String],
) -> bool {
    required_predicates
        .iter()
        .enumerate()
        .any(|(disjunctive_index, predicate)| {
            let disjuncts = repair_relevant_or_clauses(predicate);
            disjuncts.len() > 1
                && disjuncts.into_iter().all(|disjunct| {
                    let branch_clauses =
                        branch_required_clauses(required_predicates, disjunctive_index, disjunct);
                    wanted_disjuncts
                        .iter()
                        .any(|wanted| repair_clause_set_implies_clause(&branch_clauses, wanted))
                })
        })
}

fn repair_relevant_or_clause_strings(predicate: &str) -> Vec<String> {
    repair_relevant_negated_and_clauses(predicate).unwrap_or_else(|| {
        repair_relevant_or_clauses(predicate)
            .into_iter()
            .map(ToString::to_string)
            .collect()
    })
}

fn repair_clause_guaranteed_by_required_predicates(
    clause: &str,
    required_predicates: &[String],
) -> bool {
    if has_true_disjunct(clause) {
        return true;
    }
    let disjuncts = repair_relevant_or_clauses(clause);
    if disjuncts.len() > 1 {
        return disjuncts.into_iter().any(|disjunct| {
            repair_clause_guaranteed_by_required_predicates(disjunct, required_predicates)
        });
    }
    if disjuncts.is_empty() {
        return false;
    }
    let canonical = canonical_repair_clause(disjuncts[0]);
    required_predicates
        .iter()
        .any(|required| required_predicate_implies_clause(required, &canonical))
        || required_predicate_set_implies_clause(required_predicates, &canonical)
}

fn required_predicate_implies_clause(predicate: &str, wanted: &str) -> bool {
    let predicate = strip_balanced_outer_parens(predicate);
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.len() > 1 {
        return disjuncts
            .into_iter()
            .all(|disjunct| required_predicate_implies_clause(disjunct, wanted));
    }
    if disjuncts.is_empty() {
        return false;
    }
    let predicate = disjuncts[0];
    let conjuncts = split_top_level_keyword(predicate, "and");
    if conjuncts.len() > 1 {
        return conjuncts
            .into_iter()
            .any(|conjunct| required_predicate_implies_clause(conjunct, wanted));
    }
    let canonical = canonical_repair_clause(predicate);
    repair_clause_implies(&canonical, wanted)
        || repair_atoms_equivalent(&canonical, wanted, &RepairEquivalences::default())
}

fn repair_clause_implies(required: &str, wanted: &str) -> bool {
    if required == wanted {
        return true;
    }
    let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
        return false;
    };
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return boolean_atom_implies_literal_comparison(
            required,
            &wanted,
            &RepairEquivalences::default(),
        );
    };
    if required.left == wanted.left
        && required.right == wanted.right
        && matches!((required.operator, wanted.operator), ("<", "<="))
    {
        return true;
    }
    if required.operator == "<"
        && wanted.operator == "!="
        && same_repair_operands_unordered(required.left, required.right, wanted.left, wanted.right)
    {
        return true;
    }
    if equality_with_distinct_literal_implies_disequality(
        required.left,
        required.operator,
        required.right,
        wanted.left,
        wanted.operator,
        wanted.right,
        &RepairEquivalences::default(),
    ) {
        return true;
    }
    if literal_order_comparison_implies(&required, &wanted, &RepairEquivalences::default()) {
        return true;
    }
    if literal_equality_implies_order_comparison(&required, &wanted, &RepairEquivalences::default())
    {
        return true;
    }
    if literal_bound_implies_disequality(&required, &wanted, &RepairEquivalences::default()) {
        return true;
    }
    if boolean_literal_comparison_implies_comparison(
        &required,
        &wanted,
        &RepairEquivalences::default(),
    ) {
        return true;
    }
    required.operator == "=="
        && wanted.operator == "<="
        && same_repair_operands_unordered(required.left, required.right, wanted.left, wanted.right)
}

fn required_predicate_set_implies_clause(required_predicates: &[String], wanted: &str) -> bool {
    let required_clauses = required_predicates
        .iter()
        .flat_map(|predicate| repair_set_clauses(predicate))
        .collect::<Vec<_>>();
    let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
        return repair_clause_set_implies_clause(&required_clauses, wanted)
            || disjunctive_branch_set_implies_clause(required_predicates, wanted);
    };
    if repair_clause_set_implies_comparison(&required_clauses, &wanted)
        || disjunctive_branch_set_implies_clause(required_predicates, wanted.clause)
    {
        return true;
    }
    false
}

fn repair_clause_set_implies_clause(required_clauses: &[String], wanted: &str) -> bool {
    let equivalences = repair_equivalences(required_clauses);
    let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
        return required_clauses.iter().any(|required| {
            repair_atoms_equivalent(required, wanted, &equivalences)
                || boolean_literal_comparison_implies_atom(required, wanted, &equivalences)
        });
    };
    repair_clause_set_implies_comparison(required_clauses, &wanted)
}

fn repair_clause_set_implies_comparison(
    required_clauses: &[String],
    wanted: &ParsedRepairComparison<'_>,
) -> bool {
    let equivalences = repair_equivalences(required_clauses);
    if required_clauses
        .iter()
        .any(|required| repair_clause_implies_with_equivalences(required, wanted, &equivalences))
    {
        return true;
    }
    if boolean_disequality_alias_implies_comparison(required_clauses, wanted, &equivalences) {
        return true;
    }
    if ordering_path_implies_clause(required_clauses, wanted, &equivalences) {
        return true;
    }
    if wanted.operator != "==" {
        return false;
    }
    required_clauses.iter().any(|left| {
        required_clauses
            .iter()
            .any(|right| inclusive_bounds_imply_equality(left, right, wanted, &equivalences))
    })
}

fn disjunctive_branch_set_implies_clause(required_predicates: &[String], wanted: &str) -> bool {
    required_predicates
        .iter()
        .enumerate()
        .any(|(disjunctive_index, predicate)| {
            let disjuncts = repair_relevant_or_clauses(predicate);
            disjuncts.len() > 1
                && disjuncts.into_iter().all(|disjunct| {
                    let branch_clauses =
                        branch_required_clauses(required_predicates, disjunctive_index, disjunct);
                    repair_clause_set_implies_clause(&branch_clauses, wanted)
                })
        })
}

fn branch_required_clauses(
    required_predicates: &[String],
    disjunctive_index: usize,
    disjunct: &str,
) -> Vec<String> {
    required_predicates
        .iter()
        .enumerate()
        .flat_map(|(index, predicate)| {
            if index == disjunctive_index {
                repair_set_clauses(disjunct)
            } else {
                repair_set_clauses(predicate)
            }
        })
        .collect()
}

fn non_disjunctive_repair_clauses(predicate: &str) -> Vec<String> {
    let predicate = strip_balanced_outer_parens(predicate);
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.len() > 1 {
        return Vec::new();
    }
    let Some(predicate) = disjuncts.first().copied() else {
        return Vec::new();
    };
    split_top_level_keyword(predicate, "and")
        .into_iter()
        .flat_map(|clause| {
            let clause = strip_balanced_outer_parens(clause);
            if repair_relevant_or_clauses(clause).len() > 1 {
                Vec::new()
            } else {
                canonical_non_disjunctive_repair_clauses(clause)
            }
        })
        .collect()
}

fn repair_set_clauses(predicate: &str) -> Vec<String> {
    let predicate = strip_balanced_outer_parens(predicate);
    let mut clauses = non_disjunctive_repair_clauses(predicate);
    clauses.extend(disjunctive_common_repair_clauses(predicate));
    clauses
}

fn disjunctive_common_repair_clauses(predicate: &str) -> Vec<String> {
    let predicate = strip_balanced_outer_parens(predicate);
    let mut derived = Vec::new();
    for clause in split_top_level_keyword(predicate, "and") {
        let clause = strip_balanced_outer_parens(clause);
        let disjuncts = repair_relevant_or_clauses(clause);
        if disjuncts.len() <= 1 {
            continue;
        }
        let Some(first) = disjuncts.first().copied() else {
            continue;
        };
        for candidate in implied_clause_candidates(first) {
            if disjuncts
                .iter()
                .all(|disjunct| required_predicate_implies_clause(disjunct, &candidate))
                && !derived.iter().any(|existing| existing == &candidate)
            {
                derived.push(candidate);
            }
        }
    }
    derived
}

fn implied_clause_candidates(predicate: &str) -> Vec<String> {
    non_disjunctive_repair_clauses(predicate)
        .into_iter()
        .flat_map(|clause| {
            let Some(parsed) = ParsedRepairComparison::parse(&clause) else {
                return vec![clause];
            };
            vec![
                format!("{} == {}", parsed.left, parsed.right),
                format!("{} != {}", parsed.left, parsed.right),
                format!("{} < {}", parsed.left, parsed.right),
                format!("{} < {}", parsed.right, parsed.left),
                format!("{} <= {}", parsed.left, parsed.right),
                format!("{} <= {}", parsed.right, parsed.left),
            ]
        })
        .map(canonical_repair_clause)
        .fold(Vec::<String>::new(), |mut candidates, candidate| {
            if !candidates.iter().any(|existing| existing == &candidate) {
                candidates.push(candidate);
            }
            candidates
        })
}

fn canonical_non_disjunctive_repair_clauses(clause: &str) -> Vec<String> {
    if let Some(clauses) = repair_relevant_negated_and_clauses(clause) {
        return clauses;
    }
    canonical_negated_disjunction_repair_clauses(clause)
        .unwrap_or_else(|| vec![canonical_repair_clause(clause)])
}

fn canonical_negated_disjunction_repair_clauses(clause: &str) -> Option<Vec<String>> {
    let trimmed = clause.trim();
    let negated = if let Some(negated) = trimmed.strip_prefix("not ") {
        negated
    } else {
        trimmed
            .strip_prefix("not(")
            .map(|negated| negated.strip_suffix(')').unwrap_or(negated).trim())?
    };
    let negated = strip_balanced_outer_parens(negated);
    let disjuncts = split_top_level_keyword(negated, "or")
        .into_iter()
        .filter(|disjunct| !disjunct.trim().is_empty())
        .collect::<Vec<_>>();
    if disjuncts.len() <= 1 {
        return None;
    }
    let clauses = disjuncts
        .into_iter()
        .map(|disjunct| canonical_negated_repair_or_atom_clause(&format!("not ({disjunct})")))
        .collect::<Option<Vec<_>>>()?;
    if clauses.iter().any(|clause| clause == "false") {
        return Some(vec!["false".to_string()]);
    }
    let clauses = clauses
        .into_iter()
        .filter(|clause| clause != "true")
        .collect::<Vec<_>>();
    Some(if clauses.is_empty() {
        vec!["true".to_string()]
    } else {
        clauses
    })
}

fn inclusive_bounds_imply_equality(
    left: &str,
    right: &str,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(left) = ParsedRepairComparison::parse(left) else {
        return false;
    };
    let Some(right) = ParsedRepairComparison::parse(right) else {
        return false;
    };
    left.operator == "<="
        && right.operator == "<="
        && repair_operands_equivalent_ordered(
            left.left,
            left.right,
            wanted.left,
            wanted.right,
            equivalences,
        )
        && repair_operands_equivalent_ordered(
            right.left,
            right.right,
            wanted.right,
            wanted.left,
            equivalences,
        )
}

fn repair_clause_implies_with_equivalences(
    required: &str,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return boolean_atom_implies_literal_comparison(required, wanted, equivalences);
    };
    if required.operator == wanted.operator
        && repair_operands_equivalent_ordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        )
    {
        return true;
    }
    match (required.operator, wanted.operator) {
        ("<", "<=") => repair_operands_equivalent_ordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        ),
        ("<", "!=") | ("==", "<=") => repair_operands_equivalent_unordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        ),
        ("==", "!=") => equality_with_distinct_literal_implies_disequality(
            required.left,
            required.operator,
            required.right,
            wanted.left,
            wanted.operator,
            wanted.right,
            equivalences,
        ),
        _ => {
            literal_order_comparison_implies(&required, wanted, equivalences)
                || literal_equality_implies_order_comparison(&required, wanted, equivalences)
                || literal_bound_implies_disequality(&required, wanted, equivalences)
                || boolean_literal_comparison_implies_comparison(&required, wanted, equivalences)
        }
    }
}

fn literal_order_comparison_implies(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    if !matches!(required.operator, "<" | "<=") || !matches!(wanted.operator, "<" | "<=") {
        return false;
    }
    literal_lower_bound_implies(required, wanted, equivalences)
        || literal_upper_bound_implies(required, wanted, equivalences)
}

fn literal_lower_bound_implies(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required_literal) = repair_numeric_order_literal(required.left) else {
        return false;
    };
    let Some(wanted_literal) = repair_numeric_order_literal(wanted.left) else {
        return false;
    };
    repair_operands_equivalent(required.right, wanted.right, equivalences)
        && literal_order_strength_implies(
            required_literal,
            required.operator,
            wanted_literal,
            wanted.operator,
            true,
        )
}

fn literal_upper_bound_implies(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required_literal) = repair_numeric_order_literal(required.right) else {
        return false;
    };
    let Some(wanted_literal) = repair_numeric_order_literal(wanted.right) else {
        return false;
    };
    repair_operands_equivalent(required.left, wanted.left, equivalences)
        && literal_order_strength_implies(
            required_literal,
            required.operator,
            wanted_literal,
            wanted.operator,
            false,
        )
}

fn literal_equality_implies_order_comparison(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    if required.operator != "==" || !matches!(wanted.operator, "<" | "<=") {
        return false;
    }
    literal_equality_implies_lower_bound(required, wanted, equivalences)
        || literal_equality_implies_upper_bound(required, wanted, equivalences)
}

fn literal_equality_implies_lower_bound(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((required_subject, required_literal)) = literal_equality_subject(required) else {
        return false;
    };
    let Some(wanted_literal) = repair_numeric_order_literal(wanted.left) else {
        return false;
    };
    repair_operands_equivalent(required_subject, wanted.right, equivalences)
        && literal_order_strength_implies(
            required_literal,
            "<=",
            wanted_literal,
            wanted.operator,
            true,
        )
}

fn literal_equality_implies_upper_bound(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((required_subject, required_literal)) = literal_equality_subject(required) else {
        return false;
    };
    let Some(wanted_literal) = repair_numeric_order_literal(wanted.right) else {
        return false;
    };
    repair_operands_equivalent(required_subject, wanted.left, equivalences)
        && literal_order_strength_implies(
            required_literal,
            "<=",
            wanted_literal,
            wanted.operator,
            false,
        )
}

fn literal_equality_subject<'a>(
    required: &'a ParsedRepairComparison<'a>,
) -> Option<(&'a str, RepairRational)> {
    repair_numeric_order_literal(required.left)
        .map(|literal| (required.right, literal))
        .or_else(|| {
            repair_numeric_order_literal(required.right).map(|literal| (required.left, literal))
        })
}

fn literal_order_strength_implies<T: Ord>(
    required_literal: T,
    required_operator: &str,
    wanted_literal: T,
    wanted_operator: &str,
    lower_bound: bool,
) -> bool {
    match required_literal.cmp(&wanted_literal) {
        std::cmp::Ordering::Greater if lower_bound => true,
        std::cmp::Ordering::Less if !lower_bound => true,
        std::cmp::Ordering::Equal => required_operator == "<" || wanted_operator == "<=",
        _ => false,
    }
}

fn literal_bound_implies_disequality(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    if !matches!(required.operator, "<" | "<=") || wanted.operator != "!=" {
        return false;
    }
    literal_lower_bound_implies_disequality(required, wanted, equivalences)
        || literal_upper_bound_implies_disequality(required, wanted, equivalences)
}

fn literal_lower_bound_implies_disequality(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required_literal) = repair_numeric_order_literal(required.left) else {
        return false;
    };
    let Some(wanted_literal) =
        repair_disequality_literal_for_operand(wanted, required.right, equivalences)
    else {
        return false;
    };
    match wanted_literal.cmp(&required_literal) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Equal => required.operator == "<",
        std::cmp::Ordering::Greater => false,
    }
}

fn literal_upper_bound_implies_disequality(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required_literal) = repair_numeric_order_literal(required.right) else {
        return false;
    };
    let Some(wanted_literal) =
        repair_disequality_literal_for_operand(wanted, required.left, equivalences)
    else {
        return false;
    };
    match wanted_literal.cmp(&required_literal) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => required.operator == "<",
        std::cmp::Ordering::Less => false,
    }
}

fn repair_disequality_literal_for_operand(
    wanted: &ParsedRepairComparison<'_>,
    operand: &str,
    equivalences: &RepairEquivalences,
) -> Option<RepairRational> {
    if repair_operands_equivalent(wanted.left, operand, equivalences) {
        return repair_numeric_order_literal(wanted.right);
    }
    if repair_operands_equivalent(wanted.right, operand, equivalences) {
        return repair_numeric_order_literal(wanted.left);
    }
    None
}

fn disequality_implies_numeric_ordering_disjunction(
    required: &str,
    wanted_disjuncts: &[String],
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return false;
    };
    if required.operator != "!=" {
        return false;
    }
    let Some((subject, excluded)) = numeric_literal_comparison_side(&required) else {
        return false;
    };
    let mut has_lower_side = false;
    let mut has_upper_side = false;
    for wanted in wanted_disjuncts {
        let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
            continue;
        };
        if wanted.operator != "<" {
            continue;
        }
        if repair_operands_equivalent(wanted.left, subject, equivalences)
            && repair_numeric_order_literal(wanted.right) == Some(excluded)
        {
            has_lower_side = true;
        }
        if repair_numeric_order_literal(wanted.left) == Some(excluded)
            && repair_operands_equivalent(wanted.right, subject, equivalences)
        {
            has_upper_side = true;
        }
    }
    has_lower_side && has_upper_side
}

fn inclusive_bound_implies_order_or_equality_disjunction(
    required: &str,
    wanted_disjuncts: &[String],
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return false;
    };
    if required.operator != "<=" {
        return false;
    }
    let mut has_strict_side = false;
    let mut has_equality_side = false;
    for wanted in wanted_disjuncts {
        let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
            continue;
        };
        if repair_operands_equivalent_ordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        ) && wanted.operator == "<"
        {
            has_strict_side = true;
        }
        if repair_operands_equivalent_unordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        ) && wanted.operator == "=="
        {
            has_equality_side = true;
        }
    }
    has_strict_side && has_equality_side
}

fn numeric_literal_comparison_side<'a>(
    comparison: &'a ParsedRepairComparison<'a>,
) -> Option<(&'a str, RepairRational)> {
    repair_numeric_order_literal(comparison.left)
        .map(|literal| (comparison.right, literal))
        .or_else(|| {
            repair_numeric_order_literal(comparison.right).map(|literal| (comparison.left, literal))
        })
}

fn boolean_literal_comparison_implies_atom(
    required: &str,
    wanted_atom: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return false;
    };
    let Some((required_atom, required_truth)) = boolean_literal_comparison_truth(&required) else {
        return false;
    };
    let Some((wanted_atom, wanted_truth)) = boolean_atom_truth(wanted_atom) else {
        return false;
    };
    required_truth == wanted_truth
        && repair_operands_equivalent(required_atom, wanted_atom, equivalences)
}

fn boolean_atom_implies_literal_comparison(
    required_atom: &str,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((required_atom, required_truth)) = boolean_atom_truth(required_atom) else {
        return false;
    };
    let Some((wanted_atom, wanted_truth)) = boolean_literal_comparison_truth(wanted) else {
        return false;
    };
    required_truth == wanted_truth
        && repair_operands_equivalent(required_atom, wanted_atom, equivalences)
}

fn boolean_literal_comparison_implies_comparison(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((required_atom, required_truth)) = boolean_literal_comparison_truth(required) else {
        return false;
    };
    let Some((wanted_atom, wanted_truth)) = boolean_literal_comparison_truth(wanted) else {
        return false;
    };
    required_truth == wanted_truth
        && repair_operands_equivalent(required_atom, wanted_atom, equivalences)
}

fn boolean_disequality_alias_implies_comparison(
    required_clauses: &[String],
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((wanted_atom, wanted_truth)) = boolean_literal_comparison_truth(wanted) else {
        return false;
    };
    required_clauses.iter().any(|required| {
        let Some(required) = ParsedRepairComparison::parse(required) else {
            return false;
        };
        if required.operator != "!=" {
            return false;
        }
        if repair_operands_equivalent(required.left, wanted_atom, equivalences) {
            return boolean_literal_value_for_operand(
                required_clauses,
                required.right,
                equivalences,
            ) == Some(!wanted_truth);
        }
        if repair_operands_equivalent(required.right, wanted_atom, equivalences) {
            return boolean_literal_value_for_operand(
                required_clauses,
                required.left,
                equivalences,
            ) == Some(!wanted_truth);
        }
        false
    })
}

fn boolean_literal_value_for_operand(
    required_clauses: &[String],
    operand: &str,
    equivalences: &RepairEquivalences,
) -> Option<bool> {
    required_clauses.iter().find_map(|required| {
        let required = ParsedRepairComparison::parse(required)?;
        let (atom, truth) = boolean_literal_comparison_truth(&required)?;
        repair_operands_equivalent(atom, operand, equivalences).then_some(truth)
    })
}

fn boolean_literal_comparison_truth<'a>(
    comparison: &'a ParsedRepairComparison<'a>,
) -> Option<(&'a str, bool)> {
    let left_literal = RepairLiteral::parse(comparison.left);
    let right_literal = RepairLiteral::parse(comparison.right);
    let (atom, literal) = match (left_literal, right_literal) {
        (None, Some(RepairLiteral::Bool(value))) => (comparison.left, value),
        (Some(RepairLiteral::Bool(value)), None) => (comparison.right, value),
        _ => return None,
    };
    let atom_truth = match comparison.operator {
        "==" => literal,
        "!=" => !literal,
        _ => return None,
    };
    Some((atom, atom_truth))
}

fn boolean_atom_truth(atom: &str) -> Option<(&str, bool)> {
    let atom = strip_balanced_outer_parens(atom);
    if atom.is_empty()
        || atom == "true"
        || atom == "false"
        || split_top_level_keyword(atom, "and").len() > 1
        || split_top_level_keyword(atom, "or").len() > 1
        || ParsedRepairComparison::parse(atom).is_some()
    {
        return None;
    }
    if let Some(negated) = stripped_not_operand(atom) {
        let negated = strip_balanced_outer_parens(negated);
        if negated.is_empty()
            || split_top_level_keyword(negated, "and").len() > 1
            || split_top_level_keyword(negated, "or").len() > 1
            || ParsedRepairComparison::parse(negated).is_some()
        {
            return None;
        }
        return Some((negated, false));
    }
    Some((atom, true))
}

fn equality_with_distinct_literal_implies_disequality(
    required_left: &str,
    required_operator: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_operator: &str,
    wanted_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    if required_operator != "==" || wanted_operator != "!=" {
        return false;
    }
    equality_side_excludes_wanted_literal(
        required_left,
        required_right,
        wanted_left,
        wanted_right,
        equivalences,
    ) || equality_side_excludes_wanted_literal(
        required_right,
        required_left,
        wanted_left,
        wanted_right,
        equivalences,
    )
}

fn equality_side_excludes_wanted_literal(
    required_subject: &str,
    required_value: &str,
    wanted_left: &str,
    wanted_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    if repair_operands_equivalent(required_subject, wanted_left, equivalences) {
        return repair_literals_are_distinct(required_value, wanted_right);
    }
    if repair_operands_equivalent(required_subject, wanted_right, equivalences) {
        return repair_literals_are_distinct(required_value, wanted_left);
    }
    false
}

fn repair_literals_are_distinct(left: &str, right: &str) -> bool {
    if let (Some(left), Some(right)) = (repair_numeric_literal(left), repair_numeric_literal(right))
    {
        return left != right;
    }
    let Some(left) = RepairLiteral::parse(left.trim()) else {
        return false;
    };
    let Some(right) = RepairLiteral::parse(right.trim()) else {
        return false;
    };
    left != right
}

fn repair_numeric_literal(text: &str) -> Option<RepairNumber> {
    if let Some(value) = repair_numeric_expression(text) {
        return Some(value);
    }
    match RepairLiteral::parse(text.trim())? {
        RepairLiteral::Number(value) => Some(value),
        RepairLiteral::Bool(_) | RepairLiteral::String(_) => None,
    }
}

fn repair_numeric_order_literal(text: &str) -> Option<RepairRational> {
    if let Some(value) = repair_numeric_rational_expression(text) {
        return Some(value);
    }
    repair_numeric_literal(text).and_then(RepairRational::from_number)
}

fn repair_numeric_expression(predicate: &str) -> Option<RepairNumber> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if let Some(number) = parse_repair_number_literal(predicate) {
        return Some(number);
    }
    for operator in ["+", "-"] {
        if let Some((left, right)) = split_repair_numeric_operator(predicate, operator) {
            let left = repair_numeric_expression(left)?;
            let right = repair_numeric_expression(right)?;
            return match operator {
                "+" => left.add(right),
                "-" => left.sub(right),
                _ => None,
            };
        }
    }
    for operator in ["*", "/"] {
        if let Some((left, right)) = split_repair_numeric_operator(predicate, operator) {
            let left = repair_numeric_expression(left)?;
            let right = repair_numeric_expression(right)?;
            return match operator {
                "*" => left.mul(right),
                "/" => left.div(right),
                _ => None,
            };
        }
    }
    if let Some(rest) = predicate.strip_prefix('-') {
        return repair_numeric_expression(rest)?.negate();
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct RepairRational {
    numerator: i128,
    denominator: i128,
}

impl Ord for RepairRational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.numerator
            .checked_mul(other.denominator)
            .expect("repair rational comparison overflow")
            .cmp(
                &other
                    .numerator
                    .checked_mul(self.denominator)
                    .expect("repair rational comparison overflow"),
            )
    }
}

impl PartialOrd for RepairRational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl RepairRational {
    fn from_number(number: RepairNumber) -> Option<Self> {
        Self::from_raw(number.mantissa, 10_i128.checked_pow(number.scale)?)
    }

    fn from_raw(mut numerator: i128, mut denominator: i128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        if denominator < 0 {
            numerator = numerator.checked_neg()?;
            denominator = denominator.checked_neg()?;
        }
        let divisor = repair_gcd_i128(numerator, denominator)?;
        Some(Self {
            numerator: numerator.checked_div(divisor)?,
            denominator: denominator.checked_div(divisor)?,
        })
    }

    fn negate(self) -> Option<Self> {
        Some(Self {
            numerator: self.numerator.checked_neg()?,
            denominator: self.denominator,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator
                .checked_mul(other.denominator)?
                .checked_add(other.numerator.checked_mul(self.denominator)?)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn add_int(self, integer: i128) -> Option<Self> {
        self.add(Self::from_raw(integer, 1)?)
    }

    fn is_integer(&self) -> bool {
        self.denominator == 1
    }

    fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    fn mul(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn div(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }
}

fn repair_numeric_rational_expression(predicate: &str) -> Option<RepairRational> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if let Some(number) = parse_repair_number_literal(predicate) {
        return RepairRational::from_number(number);
    }
    for operator in ["+", "-"] {
        if let Some((left, right)) = split_repair_numeric_operator(predicate, operator) {
            let left = repair_numeric_rational_expression(left)?;
            let right = repair_numeric_rational_expression(right)?;
            return match operator {
                "+" => left.add(right),
                "-" => left.sub(right),
                _ => None,
            };
        }
    }
    for operator in ["*", "/"] {
        if let Some((left, right)) = split_repair_numeric_operator(predicate, operator) {
            let left = repair_numeric_rational_expression(left)?;
            let right = repair_numeric_rational_expression(right)?;
            return match operator {
                "*" => left.mul(right),
                "/" => left.div(right),
                _ => None,
            };
        }
    }
    if let Some(rest) = predicate.strip_prefix('-') {
        return repair_numeric_rational_expression(rest)?.negate();
    }
    None
}

fn split_repair_numeric_operator<'a>(
    predicate: &'a str,
    operator: &str,
) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in predicate.char_indices().rev() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            ')' => depth += 1,
            '(' => depth = depth.saturating_sub(1),
            _ if depth == 0 && predicate[index..].starts_with(operator) => {
                let left = predicate[..index].trim();
                let right = predicate[index + operator.len()..].trim();
                if !left.is_empty() && !right.is_empty() && operator_is_binary(left, operator) {
                    return Some((left, right));
                }
            }
            _ => {}
        }
    }
    None
}

fn operator_is_binary(left: &str, operator: &str) -> bool {
    operator != "-"
        || left
            .chars()
            .next_back()
            .is_some_and(|ch| ch.is_ascii_digit() || ch == ')' || ch == '"')
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct RepairNumber {
    mantissa: i128,
    scale: u32,
}

impl Ord for RepairNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.mantissa.is_negative(), other.mantissa.is_negative()) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        let ordering = self.abs_cmp(other);
        if self.mantissa.is_negative() {
            ordering.reverse()
        } else {
            ordering
        }
    }
}

impl PartialOrd for RepairNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl RepairNumber {
    fn abs_cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (left_integer, left_fraction) = self.abs_parts();
        let (right_integer, right_fraction) = other.abs_parts();
        left_integer
            .len()
            .cmp(&right_integer.len())
            .then_with(|| left_integer.cmp(&right_integer))
            .then_with(|| {
                let scale = left_fraction.len().max(right_fraction.len());
                let mut left_fraction = left_fraction;
                let mut right_fraction = right_fraction;
                left_fraction.extend(std::iter::repeat_n('0', scale - left_fraction.len()));
                right_fraction.extend(std::iter::repeat_n('0', scale - right_fraction.len()));
                left_fraction.cmp(&right_fraction)
            })
    }

    fn abs_parts(&self) -> (String, String) {
        let mut digits = self.mantissa.unsigned_abs().to_string();
        if self.scale == 0 {
            return (digits, String::new());
        }
        let scale = self.scale as usize;
        if digits.len() <= scale {
            let padding = "0".repeat(scale + 1 - digits.len());
            digits = format!("{padding}{digits}");
        }
        let split = digits.len() - scale;
        let integer = digits[..split].trim_start_matches('0');
        let integer = if integer.is_empty() { "0" } else { integer };
        (integer.to_string(), digits[split..].to_string())
    }

    fn from_raw(mut mantissa: i128, mut scale: u32) -> Self {
        while scale > 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        Self { mantissa, scale }
    }

    fn negate(self) -> Option<Self> {
        Some(Self {
            mantissa: self.mantissa.checked_neg()?,
            scale: self.scale,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        let scale = self.scale.max(other.scale);
        let left = self.scaled_mantissa(scale)?;
        let right = other.scaled_mantissa(scale)?;
        Some(Self::from_raw(left.checked_add(right)?, scale))
    }

    fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    fn mul(self, other: Self) -> Option<Self> {
        Some(Self::from_raw(
            self.mantissa.checked_mul(other.mantissa)?,
            self.scale.checked_add(other.scale)?,
        ))
    }

    fn div(self, other: Self) -> Option<Self> {
        if other.mantissa == 0 {
            return None;
        }

        let mut numerator = self
            .mantissa
            .checked_mul(10_i128.checked_pow(other.scale)?)?;
        let mut denominator = other
            .mantissa
            .checked_mul(10_i128.checked_pow(self.scale)?)?;
        if denominator < 0 {
            numerator = numerator.checked_neg()?;
            denominator = denominator.checked_neg()?;
        }

        let divisor = repair_gcd_i128(numerator, denominator)?;
        numerator /= divisor;
        denominator /= divisor;

        let mut twos = 0u32;
        while denominator % 2 == 0 {
            denominator /= 2;
            twos += 1;
        }
        let mut fives = 0u32;
        while denominator % 5 == 0 {
            denominator /= 5;
            fives += 1;
        }
        if denominator != 1 {
            return None;
        }

        let scale = twos.max(fives);
        let scale_up = 10_i128.checked_pow(scale)?;
        let mantissa = numerator
            .checked_mul(scale_up)?
            .checked_div(repair_divisor_scale(twos, fives)?)?;
        Some(Self::from_raw(mantissa, scale))
    }

    fn scaled_mantissa(self, scale: u32) -> Option<i128> {
        let extra_scale = scale.checked_sub(self.scale)?;
        self.mantissa.checked_mul(10_i128.checked_pow(extra_scale)?)
    }
}

fn repair_divisor_scale(twos: u32, fives: u32) -> Option<i128> {
    let twos = 2_i128.checked_pow(twos)?;
    let fives = 5_i128.checked_pow(fives)?;
    twos.checked_mul(fives)
}

fn repair_gcd_i128(left: i128, right: i128) -> Option<i128> {
    let mut left = left.checked_abs()?;
    let mut right = right.checked_abs()?;
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    (left != 0).then_some(left)
}

#[derive(PartialEq, Eq)]
enum RepairLiteral {
    Bool(bool),
    Number(RepairNumber),
    String(String),
}

impl RepairLiteral {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "true" => return Some(Self::Bool(true)),
            "false" => return Some(Self::Bool(false)),
            _ => {}
        }
        if let Some(number) = parse_repair_number_literal(text) {
            return Some(Self::Number(number));
        }
        parse_repair_string_literal(text).map(Self::String)
    }
}

fn parse_repair_number_literal(text: &str) -> Option<RepairNumber> {
    let (negative, digits) = text
        .strip_prefix('-')
        .map_or((false, text), |digits| (true, digits.trim_start()));
    if digits.is_empty() {
        return None;
    }
    let (integer, fraction) = digits.split_once('.').map_or((digits, ""), |parts| parts);
    if integer.is_empty()
        || !integer.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
        || (digits.contains('.') && fraction.is_empty())
    {
        return None;
    }
    let mut scale = fraction.len() as u32;
    let signed_digits = if negative {
        format!("-{integer}{fraction}")
    } else {
        format!("{integer}{fraction}")
    };
    let mut mantissa = signed_digits.parse::<i128>().ok()?;
    while scale > 0 && mantissa % 10 == 0 {
        mantissa /= 10;
        scale -= 1;
    }
    Some(RepairNumber { mantissa, scale })
}

fn parse_repair_string_literal(text: &str) -> Option<String> {
    if !text.starts_with('"') || !text.ends_with('"') {
        return None;
    }
    let mut value = String::new();
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let escaped = chars.next()?;
            value.push(escaped);
        } else if ch == '"' {
            return None;
        } else {
            value.push(ch);
        }
    }
    Some(value)
}

fn ordering_path_implies_clause(
    required_clauses: &[String],
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    match wanted.operator {
        "==" => {
            ordering_path_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                false,
                equivalences,
            ) && ordering_path_exists(
                required_clauses,
                wanted.right,
                wanted.left,
                false,
                equivalences,
            )
        }
        "<" => {
            ordering_path_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                true,
                equivalences,
            ) || (ordering_path_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                false,
                equivalences,
            ) && (disequality_clause_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                equivalences,
            ) || ordering_path_contains_disequality(
                required_clauses,
                wanted.left,
                wanted.right,
                equivalences,
            )))
        }
        "<=" => ordering_path_exists(
            required_clauses,
            wanted.left,
            wanted.right,
            false,
            equivalences,
        ),
        "!=" => {
            ordering_path_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                true,
                equivalences,
            ) || ordering_path_exists(
                required_clauses,
                wanted.right,
                wanted.left,
                true,
                equivalences,
            )
        }
        _ => false,
    }
}

fn ordering_path_contains_disequality(
    required_clauses: &[String],
    from: &str,
    to: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    required_clauses.iter().any(|clause| {
        let Some(parsed) = ParsedRepairComparison::parse(clause) else {
            return false;
        };
        if parsed.operator != "!=" {
            return false;
        }
        disequality_lies_on_ordering_path(
            required_clauses,
            from,
            to,
            parsed.left,
            parsed.right,
            equivalences,
        ) || disequality_lies_on_ordering_path(
            required_clauses,
            from,
            to,
            parsed.right,
            parsed.left,
            equivalences,
        )
    })
}

fn disequality_lies_on_ordering_path(
    required_clauses: &[String],
    from: &str,
    to: &str,
    disequal_left: &str,
    disequal_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    ordering_path_exists(required_clauses, from, disequal_left, false, equivalences)
        && ordering_path_exists(
            required_clauses,
            disequal_left,
            disequal_right,
            false,
            equivalences,
        )
        && ordering_path_exists(required_clauses, disequal_right, to, false, equivalences)
}

fn disequality_clause_exists(
    required_clauses: &[String],
    left: &str,
    right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    required_clauses.iter().any(|clause| {
        let Some(parsed) = ParsedRepairComparison::parse(clause) else {
            return false;
        };
        parsed.operator == "!="
            && repair_operands_equivalent_unordered(
                parsed.left,
                parsed.right,
                left,
                right,
                equivalences,
            )
    })
}

fn ordering_path_exists(
    required_clauses: &[String],
    from: &str,
    to: &str,
    needs_strict: bool,
    equivalences: &RepairEquivalences,
) -> bool {
    let edges = required_clauses
        .iter()
        .filter_map(|clause| {
            let parsed = ParsedRepairComparison::parse(clause)?;
            matches!(parsed.operator, "<" | "<=").then_some((
                parsed.left,
                parsed.right,
                parsed.operator == "<",
            ))
        })
        .collect::<Vec<_>>();
    let mut pending = vec![(from, false)];
    let mut visited = Vec::<(String, bool)>::new();
    while let Some((current, has_strict)) = pending.pop() {
        if repair_operands_equivalent(current, to, equivalences) && (!needs_strict || has_strict) {
            return true;
        }
        if visited
            .iter()
            .any(|(operand, strict)| operand == current && *strict == has_strict)
        {
            continue;
        }
        visited.push((current.to_string(), has_strict));
        for (left, right, edge_strict) in &edges {
            if repair_operands_equivalent(current, left, equivalences) {
                pending.push((right, has_strict || *edge_strict));
            }
        }
    }
    false
}

fn repair_equivalences(clauses: &[String]) -> RepairEquivalences {
    let mut equivalences = RepairEquivalences::default();
    for clause in clauses {
        let Some(parsed) = ParsedRepairComparison::parse(clause) else {
            continue;
        };
        if parsed.operator == "==" {
            equivalences.union(parsed.left, parsed.right);
        }
    }
    equivalences
}

#[derive(Default)]
struct RepairEquivalences {
    groups: Vec<Vec<String>>,
}

impl RepairEquivalences {
    fn union(&mut self, left: &str, right: &str) {
        let left = normalized_repair_operand_text(left);
        let right = normalized_repair_operand_text(right);
        if left == right {
            return;
        }
        let left_index = self.group_index(&left);
        let right_index = self.group_index(&right);
        match (left_index, right_index) {
            (Some(left_index), Some(right_index)) if left_index != right_index => {
                let right_group = self.groups.remove(right_index);
                let destination = if right_index < left_index {
                    left_index - 1
                } else {
                    left_index
                };
                self.groups[destination].extend(right_group);
            }
            (Some(index), None) => self.groups[index].push(right),
            (None, Some(index)) => self.groups[index].push(left),
            (None, None) => self.groups.push(vec![left, right]),
            _ => {}
        }
    }

    fn equivalent(&self, left: &str, right: &str) -> bool {
        let left = normalized_repair_operand_text(left);
        let right = normalized_repair_operand_text(right);
        left == right
            || self.groups.iter().any(|group| {
                group.iter().any(|item| item == &left) && group.iter().any(|item| item == &right)
            })
    }

    fn canonical_expression(&self, expression: &str) -> String {
        let mut output = String::with_capacity(expression.len());
        let mut chars = expression.char_indices().peekable();
        while let Some((start, ch)) = chars.next() {
            if ch == '"' {
                output.push(ch);
                let mut escaped = false;
                for (_, string_ch) in chars.by_ref() {
                    output.push(string_ch);
                    if escaped {
                        escaped = false;
                    } else if string_ch == '\\' {
                        escaped = true;
                    } else if string_ch == '"' {
                        break;
                    }
                }
            } else if is_ident_start(ch) {
                let mut end = start + ch.len_utf8();
                while let Some((next, next_ch)) = chars.peek().copied() {
                    if !is_ident_continue(next_ch) {
                        break;
                    }
                    chars.next();
                    end = next + next_ch.len_utf8();
                }
                let ident = &expression[start..end];
                if is_value_identifier_position(expression, start, end) {
                    output.push_str(self.canonical_operand(ident));
                } else {
                    output.push_str(ident);
                }
            } else if !ch.is_whitespace() {
                output.push(ch);
            }
        }
        output
    }

    fn canonical_operand<'a>(&'a self, operand: &'a str) -> &'a str {
        self.groups
            .iter()
            .find(|group| group.iter().any(|item| item == operand))
            .and_then(|group| group.iter().min().map(String::as_str))
            .unwrap_or(operand)
    }

    fn group_index(&self, operand: &str) -> Option<usize> {
        self.groups
            .iter()
            .position(|group| group.iter().any(|item| item == operand))
    }
}

fn normalized_repair_operand_text(operand: &str) -> String {
    compact_direct_repair_expression_text(strip_balanced_outer_parens(operand))
}

fn same_repair_operands_unordered(
    required_left: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_right: &str,
) -> bool {
    (required_left == wanted_left && required_right == wanted_right)
        || (required_left == wanted_right && required_right == wanted_left)
}

fn repair_operands_equivalent_ordered(
    required_left: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required_left, wanted_left, equivalences)
        && repair_operands_equivalent(required_right, wanted_right, equivalences)
}

fn repair_operands_equivalent_unordered(
    required_left: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent_ordered(
        required_left,
        required_right,
        wanted_left,
        wanted_right,
        equivalences,
    ) || repair_operands_equivalent_ordered(
        required_left,
        required_right,
        wanted_right,
        wanted_left,
        equivalences,
    )
}

fn repair_operands_equivalent(
    required: &str,
    wanted: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    equivalences.equivalent(required, wanted)
        || compact_predicate_text(required) == compact_predicate_text(wanted)
        || equivalences.canonical_expression(required) == equivalences.canonical_expression(wanted)
}

fn repair_atoms_equivalent(
    required: &str,
    wanted: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    ParsedRepairComparison::parse(required).is_none()
        && (compact_predicate_text(required) == compact_predicate_text(wanted)
            || equivalences.canonical_expression(required)
                == equivalences.canonical_expression(wanted))
}

struct ParsedRepairComparison<'a> {
    clause: &'a str,
    left: &'a str,
    operator: &'static str,
    right: &'a str,
}

impl<'a> ParsedRepairComparison<'a> {
    fn parse(clause: &'a str) -> Option<Self> {
        for operator in ["==", "!=", "<=", ">=", "<", ">"] {
            let Some((left, right)) = clause.split_once(operator) else {
                continue;
            };
            let left = left.trim();
            let right = right.trim();
            if left.is_empty() || right.is_empty() {
                return None;
            }
            return Some(Self {
                clause,
                left,
                operator,
                right,
            });
        }
        None
    }
}

struct NormalizedRepairComparison<'a> {
    left: &'a str,
    operator: &'static str,
    right: &'a str,
}

impl<'a> NormalizedRepairComparison<'a> {
    fn parse(clause: &'a str) -> Option<Self> {
        let parsed = ParsedRepairComparison::parse(strip_balanced_outer_parens(clause))?;
        Some(match parsed.operator {
            ">" => Self {
                left: parsed.right,
                operator: "<",
                right: parsed.left,
            },
            ">=" => Self {
                left: parsed.right,
                operator: "<=",
                right: parsed.left,
            },
            _ => Self {
                left: parsed.left,
                operator: parsed.operator,
                right: parsed.right,
            },
        })
    }

    fn same_operands_unordered(&self, other: &Self) -> bool {
        (compact_predicate_text(self.left) == compact_predicate_text(other.left)
            && compact_predicate_text(self.right) == compact_predicate_text(other.right))
            || self.same_operands_reversed(other)
    }

    fn same_operands_reversed(&self, other: &Self) -> bool {
        compact_predicate_text(self.left) == compact_predicate_text(other.right)
            && compact_predicate_text(self.right) == compact_predicate_text(other.left)
    }
}

fn split_top_level_keyword<'a>(predicate: &'a str, keyword: &str) -> Vec<&'a str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut cursor = 0;

    while cursor < predicate.len() {
        let ch = predicate[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        let end = cursor + ch.len_utf8();
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            cursor = end;
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && predicate[cursor..].starts_with(keyword) => {
                let keyword_end = cursor + keyword.len();
                if is_word_boundary(predicate, cursor, keyword_end) {
                    clauses.push(&predicate[start..cursor]);
                    start = keyword_end;
                    cursor = keyword_end;
                    continue;
                }
            }
            _ => {}
        }
        cursor = end;
    }

    clauses.push(&predicate[start..]);
    clauses
}

fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|ch| !is_ident_continue(ch)) && after.is_none_or(|ch| !is_ident_continue(ch))
}

fn normalized_predicate_clause(predicate: &str) -> String {
    let predicate = strip_balanced_outer_parens(predicate);
    if let Some(negated) = stripped_not_operand(predicate) {
        return match normalized_predicate_clause(negated).as_str() {
            "true" => "false".to_string(),
            "false" => "true".to_string(),
            _ => predicate.split_whitespace().collect::<Vec<_>>().join(" "),
        };
    }
    predicate.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn stripped_not_operand(predicate: &str) -> Option<&str> {
    if let Some(negated) = predicate.strip_prefix("not ") {
        return Some(negated);
    }
    predicate
        .strip_prefix("not(")
        .map(|negated| negated.strip_suffix(')').unwrap_or(negated).trim())
}

fn strip_balanced_outer_parens(mut predicate: &str) -> &str {
    loop {
        let trimmed = predicate.trim();
        if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
            return trimmed;
        }
        let mut depth = 0;
        let mut wraps_whole_clause = true;
        for (index, ch) in trimmed.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && index + ch.len_utf8() != trimmed.len() {
                        wraps_whole_clause = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if wraps_whole_clause && depth == 0 {
            predicate = &trimmed[1..trimmed.len() - 1];
        } else {
            return trimmed;
        }
    }
}

fn canonical_repair_clause(clause: impl AsRef<str>) -> String {
    let clause = strip_balanced_outer_parens(clause.as_ref());
    if let Some(negated) = canonical_negated_repair_clause(clause) {
        return negated;
    }
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        let Some((left, right)) = clause.split_once(operator) else {
            continue;
        };
        let left = left.trim();
        let right = right.trim();
        if left.is_empty() || right.is_empty() {
            return clause.to_string();
        }
        return match operator {
            "==" | "!=" if right < left => format!("{right} {operator} {left}"),
            ">" => format!("{right} < {left}"),
            ">=" => format!("{right} <= {left}"),
            _ => format!("{left} {operator} {right}"),
        };
    }
    clause.to_string()
}

fn canonical_negated_repair_clause(clause: &str) -> Option<String> {
    let trimmed = clause.trim();
    let negated = if let Some(negated) = trimmed.strip_prefix("not ") {
        negated
    } else {
        trimmed
            .strip_prefix("not(")
            .map(|negated| negated.strip_suffix(')').unwrap_or(negated).trim())?
    };
    let negated = strip_balanced_outer_parens(negated);
    if let Some(double_negated) = stripped_not_operand(negated) {
        return Some(canonical_repair_clause(double_negated));
    }
    match normalized_predicate_clause(negated).as_str() {
        "true" => return Some("false".to_string()),
        "false" => return Some("true".to_string()),
        _ => {}
    }
    for (operator, inverse) in [
        ("==", "!="),
        ("!=", "=="),
        ("<=", ">"),
        ("<", ">="),
        (">=", "<"),
        (">", "<="),
    ] {
        let Some((left, right)) = negated.split_once(operator) else {
            continue;
        };
        let left = left.trim();
        let right = right.trim();
        if left.is_empty() || right.is_empty() {
            return None;
        }
        return Some(canonical_repair_clause(format!("{left} {inverse} {right}")));
    }
    None
}

fn canonical_negated_repair_or_atom_clause(clause: &str) -> Option<String> {
    canonical_negated_repair_clause(clause).or_else(|| {
        let negated = stripped_not_operand(clause.trim())?;
        let negated = strip_balanced_outer_parens(negated);
        if negated.is_empty()
            || split_top_level_keyword(negated, "and").len() > 1
            || split_top_level_keyword(negated, "or").len() > 1
            || ParsedRepairComparison::parse(negated).is_some()
        {
            return None;
        }
        Some(format!("not {negated}"))
    })
}

fn replace_identifier(predicate: &str, target: &str, replacement: &str) -> String {
    let mut output = String::with_capacity(predicate.len());
    let mut chars = predicate.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch == '"' {
            output.push(ch);
            let mut escaped = false;
            for (_, string_ch) in chars.by_ref() {
                output.push(string_ch);
                if escaped {
                    escaped = false;
                } else if string_ch == '\\' {
                    escaped = true;
                } else if string_ch == '"' {
                    break;
                }
            }
        } else if is_ident_start(ch) {
            let mut end = start + ch.len_utf8();
            while let Some((next, next_ch)) = chars.peek().copied() {
                if !is_ident_continue(next_ch) {
                    break;
                }
                chars.next();
                end = next + next_ch.len_utf8();
            }
            let ident = &predicate[start..end];
            if ident == target && is_value_identifier_position(predicate, start, end) {
                output.push_str(replacement);
            } else {
                output.push_str(ident);
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn is_value_identifier_position(predicate: &str, start: usize, end: usize) -> bool {
    !predicate[..start].ends_with('.')
        && !predicate[..start].ends_with("::")
        && !predicate[end..].starts_with("::")
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn callee_name_path_and_type_args(callee: &Expr) -> Option<(&[String], Option<&[String]>)> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some((segments, None)),
        ExprKind::TypeApply { callee, type_args } => type_applied_name_path(callee)
            .map(|(segments, _)| (segments, Some(type_args.as_slice()))),
        _ => None,
    }
}

fn type_applied_name_path(callee: &Expr) -> Option<(&[String], &[String])> {
    match &callee.kind {
        ExprKind::TypeApply { callee, type_args } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return None;
            };
            Some((segments, type_args))
        }
        _ => None,
    }
}

fn function_returns_result(ty: &Type) -> Option<(&Type, &Type)> {
    let (_, return_type) = ty.function_parts()?;
    adt::result_parts(return_type)
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

fn contract_call_result_feeds_boolean_predicate(predicate: &str, start: usize, end: usize) -> bool {
    let Some(call_depth) = paren_depth_before(predicate, start) else {
        return false;
    };
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in predicate.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if (index < start || index >= end)
                && depth <= call_depth
                && predicate[index..].starts_with_comparison_operator() =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn contract_call_result_has_field_access(predicate: &str, end: usize) -> bool {
    predicate[end..].trim_start().starts_with('.')
}

fn paren_depth_before(text: &str, offset: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if index >= offset {
            return Some(depth);
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Some(depth)
}

trait StartsWithComparisonOperator {
    fn starts_with_comparison_operator(&self) -> bool;
}

impl StartsWithComparisonOperator for str {
    fn starts_with_comparison_operator(&self) -> bool {
        self.starts_with("==")
            || self.starts_with("!=")
            || self.starts_with("<=")
            || self.starts_with(">=")
            || self.starts_with('<')
            || self.starts_with('>')
    }
}

fn contract_call_is_argument(calls: &[ContractCall], call_index: usize) -> bool {
    let call = &calls[call_index];
    calls.iter().enumerate().any(|(index, outer)| {
        index != call_index && outer.start < call.start && call.end < outer.end
    })
}
