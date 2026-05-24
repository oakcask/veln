//! Name, type, effect, contract, and hole analysis.

use veln_ast::{
    BinaryOp, BodyLineKind, ContractKind, Expr, ExprKind, Function, NodeId, RecordField,
    SatisfyClause, SurfaceModule, Visibility,
};
use veln_core::{
    CheckedProgram, CoreBlocker, CoreCallTarget, CoreContract, CoreExpr, CoreExprKind,
    CoreFunction, CoreParam, CoreReadiness, CoreRecordField, CoreStmt, CoreStmtKind, CoreType,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_ir::{TypedProgram, lower_checked_core};
use veln_source::SourceSpan;

#[derive(Clone, Debug)]
pub struct LoweredSurfaceModule {
    pub diagnostics: Vec<Diagnostic>,
    pub core: Option<CheckedProgram>,
    pub ir: Option<TypedProgram>,
}

pub fn analyze_surface_module(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let environment = TypeEnvironment::from_module(module);

    for function in &module.functions {
        if function.visibility == Visibility::Public {
            diagnostics.extend(check_public_function_boundary(function));
        }
        diagnostics.extend(check_function_body(function, &environment));
    }

    diagnostics
}

pub fn lower_checked_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    let diagnostics = analyze_surface_module(module);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return LoweredSurfaceModule {
            diagnostics,
            core: None,
            ir: None,
        };
    }

    let environment = TypeEnvironment::from_module(module);
    let core = lower_surface_module_to_core(module, &environment);
    let ir = lower_checked_core(&core).ok();

    LoweredSurfaceModule {
        diagnostics,
        core: Some(core),
        ir,
    }
}

fn check_public_function_boundary(function: &Function) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for param in &function.params {
        if param.ty.is_none() {
            diagnostics.push(Diagnostic::new(
                "type.public_signature_missing",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "public function parameter `{}` must declare a type",
                    param.name
                ),
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
            "public function must declare a return type",
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
        diagnostics.push(Diagnostic::new(
            "effect.missing_public",
            Severity::Error,
            DiagnosticKind::Effect,
            "public function must declare effects, use `effects []` for pure functions",
            Some(function.span.clone()),
            effect_details(function.node_id.display("fn")),
        ));
    }

    diagnostics
}

fn check_function_body(function: &Function, environment: &TypeEnvironment) -> Vec<Diagnostic> {
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

struct TypeEnvironment {
    functions: Vec<FunctionSignature>,
}

#[derive(Clone)]
struct FunctionSignature {
    name: String,
    params: Vec<Type>,
    return_type: Type,
    effects: Vec<String>,
    node_id: NodeId,
    span: SourceSpan,
}

struct CallOrigin {
    node_id: NodeId,
    span: SourceSpan,
    symbol: String,
    effects: Vec<String>,
}

#[derive(Clone)]
struct EffectUse {
    effect: String,
    node_id: NodeId,
    span: SourceSpan,
    kind: &'static str,
    symbol: String,
}

#[derive(Clone)]
struct Binding {
    name: String,
    ty: Type,
}

#[derive(Clone)]
struct CoreBinding {
    name: String,
    ty: CoreType,
}

struct CoreLowerer<'a> {
    function: &'a Function,
    environment: &'a TypeEnvironment,
    bindings: Vec<CoreBinding>,
    blockers: Vec<CoreBlocker>,
}

#[derive(Clone)]
struct ExpectedType {
    ty: Type,
    source: ExpectedTypeSource,
    origin_node_id: NodeId,
    origin_span: Option<SourceSpan>,
    origin_message: &'static str,
}

#[derive(Clone, Copy)]
enum ExpectedTypeSource {
    DeclaredReturn,
    DeclaredParameter,
    LocalAnnotation,
    Inferred,
    Unknown,
}

impl ExpectedTypeSource {
    fn as_type_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn => "declared_return",
            Self::DeclaredParameter => "declared_parameter",
            Self::LocalAnnotation => "local_annotation",
            Self::Inferred => "inferred_expression",
            Self::Unknown => "unknown",
        }
    }

    fn as_hole_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn | Self::DeclaredParameter | Self::LocalAnnotation => "declared",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Type {
    Unknown,
    Named {
        name: String,
        args: Vec<Type>,
    },
    Record(Vec<(String, Type)>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        effects: Vec<String>,
    },
}

impl Type {
    fn named(name: impl Into<String>, args: Vec<Type>) -> Self {
        Self::Named {
            name: name.into(),
            args,
        }
    }

    fn bool() -> Self {
        Self::named("Bool", Vec::new())
    }

    fn int() -> Self {
        Self::named("Int", Vec::new())
    }

    fn float() -> Self {
        Self::named("Float", Vec::new())
    }

    fn string() -> Self {
        Self::named("String", Vec::new())
    }

    fn unit() -> Self {
        Self::named("Unit", Vec::new())
    }

    fn result(value: Type, error: Type) -> Self {
        Self::named("Result", vec![value, error])
    }

    fn list(item: Type) -> Self {
        Self::named("List", vec![item])
    }

    fn dict(key: Type, value: Type) -> Self {
        Self::named("Dict", vec![key, value])
    }

    fn render(&self) -> String {
        match self {
            Self::Unknown => "unknown".to_string(),
            Self::Named { name, args } if args.is_empty() => name.clone(),
            Self::Named { name, args } => {
                let args = args.iter().map(Type::render).collect::<Vec<_>>().join(", ");
                format!("{name}({args})")
            }
            Self::Record(fields) => {
                let fields = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {}", ty.render()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{fields}}}")
            }
            Self::Function {
                params,
                return_type,
                effects,
            } => {
                let params = params
                    .iter()
                    .map(Type::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                let effects = if effects.is_empty() {
                    String::new()
                } else {
                    format!(" effects [{}]", effects.join(", "))
                };
                format!("fn({params}) -> {}{effects}", return_type.render())
            }
        }
    }

    fn result_parts(&self) -> Option<(&Type, &Type)> {
        match self {
            Self::Named { name, args } if name == "Result" && args.len() == 2 => {
                Some((&args[0], &args[1]))
            }
            _ => None,
        }
    }

    fn option_part(&self) -> Option<&Type> {
        match self {
            Self::Named { name, args } if name == "Option" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    fn list_part(&self) -> Option<&Type> {
        match self {
            Self::Named { name, args } if name == "List" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    fn record_field(&self, field_name: &str) -> Option<&Type> {
        match self {
            Self::Record(fields) => fields
                .iter()
                .find_map(|(name, ty)| (name == field_name).then_some(ty)),
            _ => None,
        }
    }

    fn function_parts(&self) -> Option<(&[Type], &Type)> {
        match self {
            Self::Function {
                params,
                return_type,
                ..
            } => Some((params, return_type)),
            _ => None,
        }
    }
}

impl TypeEnvironment {
    fn from_module(module: &SurfaceModule) -> Self {
        let functions = module
            .functions
            .iter()
            .filter_map(|function| {
                let name = function.name.clone()?;
                let params = function
                    .params
                    .iter()
                    .map(|param| parse_type_or_unknown(param.ty.as_deref()))
                    .collect();
                let return_type = parse_type_or_unknown(function.return_type.as_deref());
                Some(FunctionSignature {
                    name,
                    params,
                    return_type,
                    effects: function.effects.clone().unwrap_or_default(),
                    node_id: function.node_id,
                    span: function.span.clone(),
                })
            })
            .collect();
        Self { functions }
    }

    fn function(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.iter().find(|function| function.name == name)
    }
}

fn lower_surface_module_to_core(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> CheckedProgram {
    let mut blockers = Vec::new();
    let functions = module
        .functions
        .iter()
        .map(|function| {
            let mut lowerer = CoreLowerer::new(function, environment);
            let lowered = lowerer.lower_function();
            blockers.extend(lowerer.blockers);
            lowered
        })
        .collect();
    CheckedProgram {
        functions,
        readiness: if blockers.is_empty() {
            CoreReadiness::Complete
        } else {
            CoreReadiness::Blocked(blockers)
        },
    }
}

impl<'a> CoreLowerer<'a> {
    fn new(function: &'a Function, environment: &'a TypeEnvironment) -> Self {
        Self {
            function,
            environment,
            bindings: Vec::new(),
            blockers: Vec::new(),
        }
    }

    fn lower_function(&mut self) -> CoreFunction {
        let params = self
            .function
            .params
            .iter()
            .map(|param| {
                let ty = core_type(&parse_type_or_unknown(param.ty.as_deref()));
                self.bindings.push(CoreBinding {
                    name: param.name.clone(),
                    ty: ty.clone(),
                });
                CoreParam {
                    node_id: param.node_id,
                    name: param.name.clone(),
                    ty,
                    span: param.span.clone(),
                }
            })
            .collect();
        let return_type = core_type(&parse_type_or_unknown(self.function.return_type.as_deref()));
        let contracts = self
            .function
            .contracts
            .iter()
            .map(|contract| CoreContract {
                node_id: contract.node_id,
                kind: contract.kind,
                predicate: contract.text.clone(),
                span: contract.span.clone(),
            })
            .collect();
        let body = self.lower_body(&return_type);

        CoreFunction {
            node_id: self.function.node_id,
            name: self
                .function
                .name
                .clone()
                .unwrap_or_else(|| "<missing>".to_string()),
            visibility: self.function.visibility,
            params,
            return_type,
            effects: self.function.effects.clone().unwrap_or_default(),
            contracts,
            body,
            span: self.function.span.clone(),
        }
    }

    fn lower_body(&mut self, return_type: &CoreType) -> Vec<CoreStmt> {
        let mut body = Vec::new();
        for (index, line) in self.function.body.iter().enumerate() {
            match &line.kind {
                BodyLineKind::Let {
                    name,
                    annotation,
                    expr,
                } => {
                    let expected = annotation
                        .as_deref()
                        .map(|annotation| core_type(&parse_type_or_unknown(Some(annotation))));
                    let lowered = self.lower_expr(expr, expected.as_ref());
                    let ty = expected.unwrap_or_else(|| lowered.ty.clone());
                    let name = name.clone().unwrap_or_else(|| "<missing>".to_string());
                    self.bindings.push(CoreBinding {
                        name: name.clone(),
                        ty: ty.clone(),
                    });
                    body.push(CoreStmt {
                        node_id: line.node_id,
                        kind: CoreStmtKind::Let {
                            name,
                            ty,
                            expr: lowered,
                        },
                        span: line.span.clone(),
                    });
                }
                BodyLineKind::Expr { expr } => {
                    let is_tail = index + 1 == self.function.body.len();
                    let expected = is_tail.then_some(return_type);
                    let lowered = self.lower_expr(expr, expected);
                    body.push(CoreStmt {
                        node_id: line.node_id,
                        kind: if is_tail {
                            CoreStmtKind::Return { expr: lowered }
                        } else {
                            CoreStmtKind::Expr { expr: lowered }
                        },
                        span: line.span.clone(),
                    });
                }
            }
        }
        body
    }

    fn lower_expr(&mut self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
            ExprKind::Missing => {
                self.blockers.push(CoreBlocker::MissingExpression {
                    node_id: expr.node_id,
                });
                self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
            }
            ExprKind::Hole { name, .. } => {
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
            ExprKind::NamePath(segments) => self.lower_name_path(expr, segments),
            ExprKind::StringLiteral(value) => self.core_expr(
                expr,
                CoreType::string(),
                CoreExprKind::StringLiteral(value.clone()),
            ),
            ExprKind::IntLiteral(value) => self.core_expr(
                expr,
                CoreType::int(),
                CoreExprKind::IntLiteral(value.clone()),
            ),
            ExprKind::FloatLiteral(value) => self.core_expr(
                expr,
                CoreType::float(),
                CoreExprKind::FloatLiteral(value.clone()),
            ),
            ExprKind::Unit => self.core_expr(expr, CoreType::unit(), CoreExprKind::Unit),
            ExprKind::Call { callee, args } => self.lower_call(expr, callee, args, expected),
            ExprKind::Try(inner) => self.lower_try(expr, inner, expected),
            ExprKind::Record(fields) => self.lower_record(expr, fields, expected),
            ExprKind::List(items) => self.lower_list(expr, items, expected),
            ExprKind::Prefix { op, expr: inner } => {
                let expected_operand = match op {
                    veln_ast::PrefixOp::Not => CoreType::bool(),
                    veln_ast::PrefixOp::Negate => CoreType::int(),
                };
                let lowered = self.lower_expr(inner, Some(&expected_operand));
                self.core_expr(
                    expr,
                    expected_operand,
                    CoreExprKind::Prefix {
                        op: *op,
                        expr: Box::new(lowered),
                    },
                )
            }
            ExprKind::Binary { op, left, right } => {
                let (operand, result) = match op {
                    BinaryOp::Or | BinaryOp::And => (CoreType::bool(), CoreType::bool()),
                    BinaryOp::Equal | BinaryOp::NotEqual => (CoreType::Unknown, CoreType::bool()),
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => (CoreType::int(), CoreType::bool()),
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                        (CoreType::int(), CoreType::int())
                    }
                    BinaryOp::PipeGreater => (CoreType::Unknown, CoreType::Unknown),
                };
                let left = self.lower_expr(left, Some(&operand));
                let right = self.lower_expr(right, Some(&operand));
                self.core_expr(
                    expr,
                    result,
                    CoreExprKind::Binary {
                        op: *op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                )
            }
        }
    }

    fn lower_name_path(&self, expr: &Expr, segments: &[String]) -> CoreExpr {
        match segments {
            [name] if name == "true" => {
                self.core_expr(expr, CoreType::bool(), CoreExprKind::BoolLiteral(true))
            }
            [name] if name == "false" => {
                self.core_expr(expr, CoreType::bool(), CoreExprKind::BoolLiteral(false))
            }
            [name] => {
                let ty = self
                    .bindings
                    .iter()
                    .rev()
                    .find(|binding| binding.name == *name)
                    .map_or(CoreType::Unknown, |binding| binding.ty.clone());
                self.core_expr(expr, ty, CoreExprKind::Local(name.clone()))
            }
            _ => self.core_expr(
                expr,
                CoreType::Unknown,
                CoreExprKind::Local(segments.join("::")),
            ),
        }
    }

    fn lower_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if let ExprKind::NamePath(segments) = &callee.kind {
            if matches!(segments.as_slice(), [name] if name == "Ok") {
                return self.lower_result_constructor(expr, args, expected, true);
            }
            if matches!(segments.as_slice(), [name] if name == "Err") {
                return self.lower_result_constructor(expr, args, expected, false);
            }
            if matches!(segments.as_slice(), [name] if name == "Some") {
                return self.lower_option_constructor(expr, args, expected);
            }
        }

        let signature = self.core_call_signature(callee);
        if let Some(signature) = &signature {
            if args.len() != signature.params.len() {
                self.blockers.push(CoreBlocker::UnsupportedExpression {
                    node_id: expr.node_id,
                    reason: "call_arity_mismatch".to_string(),
                });
            }
        }
        let lowered_args = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                let expected = signature
                    .as_ref()
                    .and_then(|signature| signature.params.get(index));
                self.lower_expr(arg, expected)
            })
            .collect();
        let (target, return_type) = signature.map_or_else(
            || {
                let symbol = callee_symbol(callee).unwrap_or_else(|| "<unknown>".to_string());
                (CoreCallTarget::Unresolved(symbol), CoreType::Unknown)
            },
            |signature| (signature.target, signature.return_type),
        );

        self.core_expr(
            expr,
            return_type,
            CoreExprKind::Call {
                target,
                args: lowered_args,
            },
        )
    }

    fn lower_result_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
        is_ok: bool,
    ) -> CoreExpr {
        if args.len() != 1 {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: expr.node_id,
                reason: "result_constructor_arity_mismatch".to_string(),
            });
        }
        let (value_type, error_type) = expected
            .and_then(CoreType::result_parts)
            .map_or((CoreType::Unknown, CoreType::Unknown), |(value, error)| {
                (value.clone(), error.clone())
            });
        let arg_expected = if is_ok { &value_type } else { &error_type };
        let first = args
            .first()
            .map(|arg| self.lower_expr(arg, Some(arg_expected)))
            .unwrap_or_else(|| {
                self.blockers.push(CoreBlocker::MissingExpression {
                    node_id: expr.node_id,
                });
                self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
            });
        let ty = if expected.and_then(CoreType::result_parts).is_some() {
            CoreType::result(value_type, error_type)
        } else if is_ok {
            CoreType::result(first.ty.clone(), CoreType::Unknown)
        } else {
            CoreType::result(CoreType::Unknown, first.ty.clone())
        };
        for arg in args.iter().skip(1) {
            self.lower_expr(arg, None);
        }
        self.core_expr(
            expr,
            ty,
            if is_ok {
                CoreExprKind::ResultOk(Box::new(first))
            } else {
                CoreExprKind::ResultErr(Box::new(first))
            },
        )
    }

    fn lower_option_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if args.len() != 1 {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: expr.node_id,
                reason: "option_constructor_arity_mismatch".to_string(),
            });
        }
        let value_type = expected
            .and_then(CoreType::option_part)
            .cloned()
            .unwrap_or(CoreType::Unknown);
        let first = args
            .first()
            .map(|arg| self.lower_expr(arg, Some(&value_type)))
            .unwrap_or_else(|| {
                self.blockers.push(CoreBlocker::MissingExpression {
                    node_id: expr.node_id,
                });
                self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
            });
        for arg in args.iter().skip(1) {
            self.lower_expr(arg, None);
        }
        let ty = if value_type == CoreType::Unknown {
            CoreType::option(first.ty.clone())
        } else {
            CoreType::option(value_type)
        };
        self.core_expr(expr, ty, CoreExprKind::OptionSome(Box::new(first)))
    }

    fn lower_try(&mut self, expr: &Expr, inner: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        let return_result = self
            .function
            .return_type
            .as_deref()
            .and_then(|return_type| parse_type_annotation(return_type).ok())
            .map(|ty| core_type(&ty))
            .and_then(|ty| {
                ty.result_parts()
                    .map(|(value, error)| (value.clone(), error.clone()))
            });
        let (value_type, error_type) = match (expected, return_result) {
            (Some(expected), Some((_, error))) => (expected.clone(), error),
            (Some(expected), None) => (expected.clone(), CoreType::Unknown),
            (None, Some((value, error))) => (value, error),
            (None, None) => (CoreType::Unknown, CoreType::Unknown),
        };
        let inner_expected = CoreType::result(value_type.clone(), error_type);
        let inner = self.lower_expr(inner, Some(&inner_expected));
        self.core_expr(expr, value_type, CoreExprKind::Try(Box::new(inner)))
    }

    fn lower_record(
        &mut self,
        expr: &Expr,
        fields: &[RecordField],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
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

    fn lower_list(&mut self, expr: &Expr, items: &[Expr], expected: Option<&CoreType>) -> CoreExpr {
        let item_expected = expected.and_then(CoreType::list_part).cloned();
        let items = items
            .iter()
            .map(|item| self.lower_expr(item, item_expected.as_ref()))
            .collect::<Vec<_>>();
        let item_type = item_expected.unwrap_or_else(|| {
            items
                .first()
                .map_or(CoreType::Unknown, |item| item.ty.clone())
        });
        self.core_expr(expr, CoreType::list(item_type), CoreExprKind::List(items))
    }

    fn core_call_signature(&self, callee: &Expr) -> Option<CoreCallSignature> {
        let ExprKind::NamePath(segments) = &callee.kind else {
            return None;
        };
        if stdio_signature(segments, callee).is_some() {
            return Some(CoreCallSignature {
                target: CoreCallTarget::StdioBuiltin(segments.join("::")),
                params: vec![CoreType::string()],
                return_type: CoreType::unit(),
            });
        }
        let name = segments.last()?;
        if let Some(function) = self.environment.function(name) {
            return Some(CoreCallSignature {
                target: CoreCallTarget::Function(function.name.clone()),
                params: function.params.iter().map(core_type).collect(),
                return_type: core_type(&function.return_type),
            });
        }
        let binding = self
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)?;
        let CoreType::Function {
            params,
            return_type,
            ..
        } = &binding.ty
        else {
            return None;
        };
        Some(CoreCallSignature {
            target: CoreCallTarget::Value(name.clone()),
            params: params.clone(),
            return_type: return_type.as_ref().clone(),
        })
    }

    fn core_expr(&self, expr: &Expr, ty: CoreType, kind: CoreExprKind) -> CoreExpr {
        CoreExpr {
            node_id: expr.node_id,
            ty,
            kind,
            span: expr.span.clone(),
        }
    }
}

struct CoreCallSignature {
    target: CoreCallTarget,
    params: Vec<CoreType>,
    return_type: CoreType,
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
                        "contract predicate must produce `Bool`",
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
                format!("public function must declare `{effect}` in its effects list"),
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
                        JsonValue::string(format!("Effect required by `{}`.", effect_use.symbol)),
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

fn is_assignable(expected: &Type, actual: &Type) -> bool {
    if expected == &Type::Unknown || actual == &Type::Unknown || expected == actual {
        return true;
    }
    match (expected, actual) {
        (Type::Record(expected_fields), Type::Record(actual_fields)) => {
            expected_fields.iter().all(|(expected_name, expected_ty)| {
                actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == expected_name)
                    .is_some_and(|(_, actual_ty)| is_assignable(expected_ty, actual_ty))
            })
        }
        (
            Type::Function {
                params: expected_params,
                return_type: expected_return,
                ..
            },
            Type::Function {
                params: actual_params,
                return_type: actual_return,
                ..
            },
        ) => {
            expected_params.len() == actual_params.len()
                && expected_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| is_assignable(expected, actual))
                && is_assignable(expected_return, actual_return)
        }
        _ => false,
    }
}

fn parse_type_or_unknown(text: Option<&str>) -> Type {
    text.and_then(|text| parse_type_annotation(text).ok())
        .unwrap_or(Type::Unknown)
}

fn core_type(ty: &Type) -> CoreType {
    match ty {
        Type::Unknown => CoreType::Unknown,
        Type::Named { name, args } => {
            CoreType::named(name.clone(), args.iter().map(core_type).collect())
        }
        Type::Record(fields) => CoreType::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), core_type(ty)))
                .collect(),
        ),
        Type::Function {
            params,
            return_type,
            effects,
        } => CoreType::Function {
            params: params.iter().map(core_type).collect(),
            return_type: Box::new(core_type(return_type)),
            effects: effects.clone(),
        },
    }
}

fn callee_symbol(callee: &Expr) -> Option<String> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments.join("::")),
        _ => None,
    }
}

fn parse_type_annotation(text: &str) -> Result<Type, String> {
    let mut parser = TypeParser::new(text);
    let ty = parser.parse_type()?;
    parser.skip_ws();
    if parser.at_end() {
        Ok(ty)
    } else {
        Err(format!("unexpected `{}`", &parser.text[parser.cursor..]))
    }
}

struct TypeParser<'a> {
    text: &'a str,
    cursor: usize,
}

impl<'a> TypeParser<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, cursor: 0 }
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        self.skip_ws();
        if self.eat('{') {
            return self.parse_record_type();
        }
        if self.eat_keyword("fn") {
            return self.parse_function_type();
        }

        let Some(name) = self.parse_ident() else {
            return Err("expected type".to_string());
        };
        self.skip_ws();
        let args = if self.eat('(') {
            let args = self.parse_type_list(')')?;
            self.expect(')')?;
            args
        } else {
            Vec::new()
        };
        self.validate_named_type(name, args)
    }

    fn parse_record_type(&mut self) -> Result<Type, String> {
        let mut fields = Vec::new();
        while !self.at_end() && !self.at('}') {
            let Some(name) = self.parse_ident() else {
                return Err("expected record field name".to_string());
            };
            self.expect(':')?;
            let ty = self.parse_type()?;
            fields.push((name, ty));
            self.skip_ws();
            if !self.eat(',') {
                break;
            }
        }
        self.expect('}')?;
        Ok(Type::Record(fields))
    }

    fn parse_function_type(&mut self) -> Result<Type, String> {
        self.expect('(')?;
        let params = self.parse_type_list(')')?;
        self.expect(')')?;
        self.skip_ws();
        if !self.eat_str("->") {
            return Err("expected `->` in function type".to_string());
        }
        let return_type = self.parse_type()?;
        let effects = if self.eat_keyword("effects") {
            self.expect('[')?;
            let mut effects = Vec::new();
            while !self.at_end() && !self.at(']') {
                let Some(effect) = self.parse_ident() else {
                    return Err("expected effect name".to_string());
                };
                effects.push(effect);
                self.skip_ws();
                if !self.eat(',') {
                    break;
                }
            }
            self.expect(']')?;
            effects
        } else {
            Vec::new()
        };
        Ok(Type::Function {
            params,
            return_type: Box::new(return_type),
            effects,
        })
    }

    fn parse_type_list(&mut self, end: char) -> Result<Vec<Type>, String> {
        let mut args = Vec::new();
        self.skip_ws();
        while !self.at_end() && !self.at(end) {
            args.push(self.parse_type()?);
            self.skip_ws();
            if !self.eat(',') {
                break;
            }
            self.skip_ws();
            if self.at(end) {
                break;
            }
        }
        Ok(args)
    }

    fn validate_named_type(&self, name: String, args: Vec<Type>) -> Result<Type, String> {
        let expected_arity = match name.as_str() {
            "Bool" | "Int" | "Float" | "String" | "Unit" => Some(0),
            "Option" | "List" => Some(1),
            "Result" | "Dict" => Some(2),
            _ => None,
        };
        if let Some(expected) = expected_arity {
            if args.len() != expected {
                return Err(format!(
                    "`{name}` expects {expected} type argument(s), found {}",
                    args.len()
                ));
            }
        }
        if name == "Dict" && args.len() == 2 {
            Ok(Type::dict(args[0].clone(), args[1].clone()))
        } else {
            Ok(Type::named(name, args))
        }
    }

    fn parse_ident(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.cursor;
        while let Some(ch) = self.current() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.cursor += ch.len_utf8();
            } else {
                break;
            }
        }
        while self.text[self.cursor..].starts_with("::") {
            self.cursor += 2;
            let segment_start = self.cursor;
            while let Some(ch) = self.current() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    self.cursor += ch.len_utf8();
                } else {
                    break;
                }
            }
            if self.cursor == segment_start {
                self.cursor = start;
                return None;
            }
        }
        (self.cursor > start).then(|| self.text[start..self.cursor].to_string())
    }

    fn skip_ws(&mut self) {
        while self.current().is_some_and(char::is_whitespace) {
            self.cursor += 1;
        }
    }

    fn eat(&mut self, expected: char) -> bool {
        self.skip_ws();
        if self.at(expected) {
            self.cursor += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn at(&self, expected: char) -> bool {
        self.current() == Some(expected)
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        if self.eat(expected) {
            Ok(())
        } else {
            Err(format!("expected `{expected}`"))
        }
    }

    fn eat_keyword(&mut self, keyword: &str) -> bool {
        self.skip_ws();
        if self.text[self.cursor..].starts_with(keyword)
            && self.text[self.cursor + keyword.len()..]
                .chars()
                .next()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
        {
            self.cursor += keyword.len();
            true
        } else {
            false
        }
    }

    fn eat_str(&mut self, expected: &str) -> bool {
        self.skip_ws();
        if self.text[self.cursor..].starts_with(expected) {
            self.cursor += expected.len();
            true
        } else {
            false
        }
    }

    fn at_end(&self) -> bool {
        self.cursor >= self.text.len()
    }

    fn current(&self) -> Option<char> {
        self.text[self.cursor..].chars().next()
    }
}

enum ContractValidation {
    Valid,
    NonBoolean { actual_type: String },
    UnsupportedConstruct { reason: &'static str },
    UnresolvedName { name: String },
}

fn stdio_signature(segments: &[String], callee: &Expr) -> Option<CallOrigin> {
    let [module, name] = segments else {
        return None;
    };
    if module != "stdio" || !matches!(name.as_str(), "print" | "println" | "eprint" | "eprintln") {
        return None;
    }
    Some(CallOrigin {
        node_id: callee.node_id,
        span: callee.span.clone(),
        symbol: format!("{module}::{name}"),
        effects: vec!["stdio".to_string()],
    })
}

fn contract_kind_text(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::Require => "require",
        ContractKind::Ensure => "ensure",
    }
}

fn contains_call_like_construct(predicate: &str) -> bool {
    let bytes = predicate.as_bytes();
    bytes.windows(1).enumerate().any(|(index, window)| {
        window == b"(" && index > 0 && predicate[..index].trim_end().ends_with_identifier()
    })
}

trait EndsWithIdentifier {
    fn ends_with_identifier(&self) -> bool;
}

impl EndsWithIdentifier for str {
    fn ends_with_identifier(&self) -> bool {
        self.chars()
            .rev()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    }
}

fn referenced_names(predicate: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = predicate.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < bytes.len() {
                let ch = bytes[index] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    index += 1;
                } else {
                    break;
                }
            }
            if start >= 2 && &predicate[start - 2..start] == "::" {
                continue;
            }
            if index + 2 <= bytes.len() && &predicate[index..index + 2] == "::" {
                continue;
            }
            let name = predicate[start..index].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        } else {
            index += 1;
        }
    }
    names
}

fn is_contract_keyword(name: &str) -> bool {
    matches!(name, "and" | "or" | "not")
}

fn predicate_is_boolean(predicate: &str, bindings: &[Binding]) -> bool {
    let trimmed = predicate.trim();
    if matches!(trimmed, "true" | "false") {
        return true;
    }
    if trimmed.contains(" and ")
        || trimmed.contains(" or ")
        || trimmed.starts_with("not ")
        || ["==", "!=", "<=", ">=", "<", ">"]
            .iter()
            .any(|operator| trimmed.contains(operator))
    {
        return true;
    }
    bindings.iter().any(|binding| {
        binding.name == trimmed && matches!(binding.ty, Type::Named { ref name, ref args } if name == "Bool" && args.is_empty())
    })
}

fn predicate_rendered_type(predicate: &str, bindings: &[Binding]) -> String {
    let trimmed = predicate.trim();
    if trimmed.starts_with('"') {
        return Type::string().render();
    }
    if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return Type::int().render();
    }
    bindings
        .iter()
        .find(|binding| binding.name == trimmed)
        .map_or_else(|| "unknown".to_string(), |binding| binding.ty.render())
}

fn span_json(span: &SourceSpan) -> JsonValue {
    JsonValue::object([
        ("file", JsonValue::string(span.file.as_str())),
        (
            "start",
            JsonValue::object([
                ("line", JsonValue::Number(span.start.line as i64)),
                ("column", JsonValue::Number(span.start.column as i64)),
                ("offset", JsonValue::Number(span.start.offset as i64)),
            ]),
        ),
        (
            "end",
            JsonValue::object([
                ("line", JsonValue::Number(span.end.line as i64)),
                ("column", JsonValue::Number(span.end.column as i64)),
                ("offset", JsonValue::Number(span.end.offset as i64)),
            ]),
        ),
    ])
}

fn type_details(
    node_id: String,
    expected_type: impl Into<String>,
    actual_type: impl Into<String>,
    expected_type_source: &'static str,
    actual_type_source: &'static str,
    constraint: &'static str,
    origin_node_ids: impl IntoIterator<Item = String>,
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("type")),
        ("node_id", JsonValue::string(node_id)),
        ("expected_type", JsonValue::string(expected_type)),
        ("actual_type", JsonValue::string(actual_type)),
        (
            "expected_type_source",
            JsonValue::string(expected_type_source),
        ),
        ("actual_type_source", JsonValue::string(actual_type_source)),
        ("constraint", JsonValue::string(constraint)),
        (
            "origin_node_ids",
            JsonValue::array(origin_node_ids.into_iter().map(JsonValue::string)),
        ),
    ])
}

fn effect_details(node_id: String) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("effect")),
        ("node_id", JsonValue::string(node_id)),
        ("effect", JsonValue::string("unknown")),
        ("boundary", JsonValue::string("public_function")),
        ("declared_effects", JsonValue::array([])),
        ("inferred_effects", JsonValue::array([])),
        ("provenance", JsonValue::array([])),
        ("provenance_truncated", JsonValue::Bool(false)),
    ])
}

fn effect_missing_public_details(
    node_id: String,
    effect: &str,
    declared_effects: &[String],
    inferred_effects: &[String],
    provenance: &[EffectUse],
    provenance_truncated: bool,
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("effect")),
        ("node_id", JsonValue::string(node_id)),
        ("effect", JsonValue::string(effect)),
        ("boundary", JsonValue::string("public_function")),
        (
            "declared_effects",
            JsonValue::array(declared_effects.iter().cloned().map(JsonValue::string)),
        ),
        (
            "inferred_effects",
            JsonValue::array(inferred_effects.iter().cloned().map(JsonValue::string)),
        ),
        (
            "provenance",
            JsonValue::array(provenance.iter().map(|effect_use| {
                JsonValue::object([
                    (
                        "node_id",
                        JsonValue::string(effect_use.node_id.display("call")),
                    ),
                    ("kind", JsonValue::string(effect_use.kind)),
                    ("symbol", JsonValue::string(effect_use.symbol.clone())),
                ])
            })),
        ),
        (
            "provenance_truncated",
            JsonValue::Bool(provenance_truncated),
        ),
    ])
}

fn contract_details(
    node_id: String,
    kind: ContractKind,
    predicate_text: &str,
    validation_status: &'static str,
    obligation_status: &'static str,
    reason: &'static str,
    runtime_required: bool,
    referenced_bindings: Vec<JsonValue>,
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("contract")),
        ("node_id", JsonValue::string(node_id)),
        ("clause", JsonValue::string(contract_kind_text(kind))),
        ("predicate_text", JsonValue::string(predicate_text)),
        ("validation_status", JsonValue::string(validation_status)),
        ("obligation_status", JsonValue::string(obligation_status)),
        ("reason", JsonValue::string(reason)),
        (
            "blame",
            JsonValue::string(match kind {
                ContractKind::Require => "caller",
                ContractKind::Ensure => "implementation",
            }),
        ),
        ("runtime_required", JsonValue::Bool(runtime_required)),
        ("referenced_bindings", JsonValue::array(referenced_bindings)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use veln_ast::lower_surface_ast;
    use veln_core::{
        CoreBlocker, CoreCallTarget, CoreExprKind, CoreReadiness, CoreStmtKind, CoreType,
    };
    use veln_ir::{IrCallTarget, IrExprKind, IrStmtKind};
    use veln_source::SourceFile;
    use veln_syntax::parse;

    #[test]
    fn public_function_requires_explicit_boundary() {
        let source = SourceFile::new("main.veln", "pub fn main(value)\n  value\nend\n");
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 3);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.public_signature_missing"
                && diagnostic.message == "public function parameter `value` must declare a type"
        }));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.public_signature_missing"
                && diagnostic.message == "public function must declare a return type"
        }));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "effect.missing_public" && diagnostic.kind == DiagnosticKind::Effect
        }));
    }

    #[test]
    fn private_function_may_omit_boundary_annotations() {
        let source = SourceFile::new("main.veln", "fn helper(value)\n  value\nend\n");
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_hole_with_declared_return_expected_type() {
        let source = SourceFile::new(
            "main.veln",
            "fn todo() -> Result(Unit, AppError)\n  _\nend\n",
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "hole.unfilled");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Hole);
        assert_eq!(
            diagnostics[0].details.to_json(),
            concat!(
                "{\"phase\":\"hole\",\"node_id\":\"hole-3\",\"label\":null,",
                "\"expected_type\":\"Result(Unit, AppError)\",",
                "\"expected_type_source\":\"declared\",",
                "\"constraints\":[],\"local_bindings\":[],",
                "\"candidate_queries\":[{\"kind\":\"symbol\",",
                "\"query\":\"fn() -> Result(Unit, AppError)\"}]}"
            )
        );
        assert_eq!(diagnostics[0].related.len(), 1);
    }

    #[test]
    fn reports_return_type_mismatch() {
        let source = SourceFile::new("main.veln", "fn bad() -> Int\n  \"no\"\nend\n");
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "type.mismatch");
        assert_eq!(
            diagnostics[0].details.to_json(),
            concat!(
                "{\"phase\":\"type\",\"node_id\":\"expr-3\",\"expected_type\":\"Int\",",
                "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
                "\"actual_type_source\":\"inferred_expression\",",
                "\"constraint\":\"return_value\",",
                "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-3\"]}"
            )
        );
    }

    #[test]
    fn ok_constructor_accepts_declared_result_return() {
        let source = SourceFile::new(
            "main.veln",
            "fn main() -> Result(Unit, AppError)\n  Ok(())\nend\n",
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn result_constructor_checks_expected_value_type() {
        let source = SourceFile::new(
            "main.veln",
            "fn main() -> Result(Unit, AppError)\n  Ok(\"no\")\nend\n",
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "type.mismatch");
        assert_eq!(
            diagnostics[0].details.to_json(),
            concat!(
                "{\"phase\":\"type\",\"node_id\":\"expr-5\",\"expected_type\":\"Unit\",",
                "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
                "\"actual_type_source\":\"inferred_expression\",",
                "\"constraint\":\"call_argument\",",
                "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-5\"]}"
            )
        );
    }

    #[test]
    fn accepts_first_slice_type_forms_and_record_expected_fields() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn main() -> {score: Float, names: List(String), table: Dict(String, Int), ",
                "callback: fn(Int) -> String}\n",
                "  {score: _, names: [], table: _, callback: _}\n",
                "end\n",
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 3);
        let rendered = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.details.to_json())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("\"expected_type\":\"Float\""));
        assert!(rendered.contains("\"expected_type\":\"Dict(String, Int)\""));
        assert!(rendered.contains("\"expected_type\":\"fn(Int) -> String\""));
        assert!(rendered.contains("\"candidate_queries\":[{\"kind\":\"symbol\""));
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.related.is_empty())
        );
    }

    #[test]
    fn reports_invalid_type_annotations() {
        let source = SourceFile::new(
            "main.veln",
            "fn bad(value: Result(Int)) -> Option()\n  ()\nend\n",
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id == "type.invalid_annotation")
        );
    }

    #[test]
    fn infers_non_constructor_calls_from_local_function_signatures() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn parse(raw: String) -> Result(Int, AppError)\n",
                "  Ok(1)\n",
                "end\n",
                "pub fn main() -> Result(Int, AppError) effects []\n",
                "  parse(\"1\")\n",
                "end\n",
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn flows_call_argument_expected_type_into_holes() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn consume(value: Float) -> Unit\n",
                "  ()\n",
                "end\n",
                "pub fn main() -> Unit effects []\n",
                "  consume(_)\n",
                "end\n",
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "hole.unfilled");
        assert!(
            diagnostics[0]
                .details
                .to_json()
                .contains("\"expected_type\":\"Float\"")
        );
        assert_eq!(diagnostics[0].related.len(), 1);
    }

    #[test]
    fn reports_missing_public_effect_with_call_provenance() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "pub fn main() -> Unit effects []\n",
                "  stdio::println(\"hello\")\n",
                "end\n",
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "effect.missing_public");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Effect);
        assert_eq!(
            diagnostics[0].message,
            "public function must declare `stdio` in its effects list"
        );
        let details = diagnostics[0].details.to_json();
        assert!(details.contains("\"effect\":\"stdio\""));
        assert!(details.contains("\"declared_effects\":[]"));
        assert!(details.contains("\"inferred_effects\":[\"stdio\"]"));
        assert!(details.contains("\"symbol\":\"stdio::println\""));
        assert_eq!(diagnostics[0].related.len(), 1);
    }

    #[test]
    fn reports_non_boolean_contract_predicate() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "pub fn main(value: Int) -> Unit effects []\n",
                "require value\n",
                "  ()\n",
                "end\n",
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "contract.type_mismatch"
                && diagnostic.kind == DiagnosticKind::Contract
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"reason\":\"non_boolean_predicate\"")
        }));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.kind == DiagnosticKind::Type
                && diagnostic.message == "expected `Bool`, but found `Int`"
        }));
    }

    #[test]
    fn hole_diagnostic_includes_contract_and_satisfy_constraints() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "pub fn default_port(max: Int) -> Int effects []\n",
                "require max > 0\n",
                "  _port satisfy candidate => candidate > 0 and candidate <= max\n",
                "end\n",
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "hole.unfilled");
        let details = diagnostics[0].details.to_json();
        assert!(details.contains("\"expected_type\":\"Int\""));
        assert!(details.contains("\"kind\":\"contract\""));
        assert!(details.contains("\"clause\":\"require\""));
        assert!(details.contains("\"text\":\"max > 0\""));
        assert!(details.contains("\"kind\":\"satisfy\""));
        assert!(details.contains(
            "\"text\":\"candidate > 0 and candidate <= max\",\"candidate_binding\":\"candidate\""
        ));
        assert!(details.contains("\"repair_status\":\"blocked_until_discharged\""));
        assert_eq!(diagnostics[0].related.len(), 3);
    }

    #[test]
    fn propagates_try_expected_type_from_result_return() {
        let source = SourceFile::new(
            "main.veln",
            "fn main() -> Result(Int, AppError)\n  Ok(_?)\nend\n",
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "hole.unfilled");
        assert!(
            diagnostics[0]
                .details
                .to_json()
                .contains("\"expected_type\":\"Result(Int, AppError)\"")
        );
    }

    #[test]
    fn lowers_option_constructor_with_expected_return_type() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "pub fn main() -> Option(String) effects []\n",
                "  Some(\"ok\")\n",
                "end\n",
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let lowered = lower_checked_surface_module(&module);

        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        let core = lowered.core.expect("checked core should be built");
        assert_eq!(core.readiness, CoreReadiness::Complete);
        let main = core
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main should be lowered");
        let CoreStmtKind::Return { expr } = &main.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert_eq!(expr.ty, CoreType::option(CoreType::string()));
        let CoreExprKind::OptionSome(value) = &expr.kind else {
            panic!("Some call should lower to an option constructor");
        };
        assert_eq!(value.ty, CoreType::string());
        assert!(lowered.ir.is_some());
    }

    #[test]
    fn lowers_runnable_checked_program_to_core_and_typed_ir() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
                "  Ok(1)\n",
                "end\n",
                "pub fn main(raw: String) -> Result(Unit, AppError) effects [stdio]\n",
                "  let value: Int = parse(raw)?\n",
                "  stdio::println(\"ok\")\n",
                "  Ok(())\n",
                "end\n",
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let lowered = lower_checked_surface_module(&module);

        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        let core = lowered.core.expect("checked core should be built");
        assert_eq!(core.readiness, CoreReadiness::Complete);
        let main = core
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main should be lowered");
        assert!(matches!(main.body[0].kind, CoreStmtKind::Let { .. }));
        let CoreStmtKind::Expr { expr } = &main.body[1].kind else {
            panic!("stdio call should lower as an expression statement");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::StdioBuiltin(symbol),
                ..
            } if symbol == "stdio::println"
        ));
        assert!(matches!(main.body[2].kind, CoreStmtKind::Return { .. }));

        let ir = lowered.ir.expect("complete core should lower to typed IR");
        let main = ir
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main should be in IR");
        assert!(matches!(main.body[0].kind, IrStmtKind::Let { .. }));
        let IrStmtKind::Expr { value } = &main.body[1].kind else {
            panic!("stdio call should stay an expression statement in IR");
        };
        assert!(matches!(
            &value.kind,
            IrExprKind::Call {
                target: IrCallTarget::StdioBuiltin(symbol),
                ..
            } if symbol == "stdio::println"
        ));
        let IrStmtKind::Return { value } = &main.body[2].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(value.kind, IrExprKind::ResultOk(_)));
    }

    #[test]
    fn holes_build_blocked_core_but_not_executable_ir() {
        let source = SourceFile::new(
            "main.veln",
            "pub fn main() -> Result(Unit, AppError) effects []\n  _\nend\n",
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let lowered = lower_checked_surface_module(&module);

        assert_eq!(lowered.diagnostics.len(), 1);
        assert_eq!(lowered.diagnostics[0].id, "hole.unfilled");
        let core = lowered.core.expect("partial checked core should be built");
        assert!(matches!(
            core.readiness,
            CoreReadiness::Blocked(ref blockers) if matches!(blockers.as_slice(), [CoreBlocker::Hole { .. }])
        ));
        assert!(lowered.ir.is_none());
    }

    #[test]
    fn semantic_errors_block_core_and_ir() {
        let source = SourceFile::new(
            "main.veln",
            "pub fn main() -> Int effects []\n  \"no\"\nend\n",
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let lowered = lower_checked_surface_module(&module);

        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "type.mismatch")
        );
        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
    }
}
