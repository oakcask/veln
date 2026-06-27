use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{
    BinaryOp, BodyLineKind, CodecDecl, CodecDirection, CodecImplementationKind, Expr, ExprKind,
    Function, FunctionKind, IfBranch, MatchArm, NodeId, Pattern, PatternKind, PublicAliasKind,
    SchemaDecl, SchemaField, SurfaceModule, UseDecl, Visibility,
};
use veln_core::CoreType;
use veln_source::SourceSpan;

use crate::adt::{self, AdtRegistry};
use crate::effects::{concurrency_effects, is_stdio_call, standard_library_effects};
use crate::schema::mapping::{
    SchemaDecodeMappingExpr, SchemaMappingSelectorComparison, SchemaMappingTyper,
    schema_decode_mapping_record_fields, schema_mapping_selector_predicate,
    schema_mapping_target_record_fields,
};

pub(crate) struct TypeEnvironment {
    functions: Vec<FunctionSignature>,
    codec_calls: Vec<CodecCallSignature>,
    pub(crate) uses: Vec<UseDecl>,
    pub(crate) adts: AdtRegistry,
}

#[derive(Clone)]
pub(crate) struct FunctionSignature {
    pub(crate) name: String,
    pub(crate) target_name: String,
    pub(crate) module_name: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) params: Vec<Type>,
    pub(crate) variadic: Option<Type>,
    pub(crate) return_type: Type,
    pub(crate) effects: Vec<String>,
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
}

#[derive(Clone)]
pub(crate) struct CodecCallSignature {
    pub(crate) name: String,
    pub(crate) target_name: String,
    pub(crate) boundary: CodecCallBoundary,
    pub(crate) module_name: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) params: Vec<Type>,
    pub(crate) return_type: Type,
    pub(crate) effects: Vec<String>,
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodecCallBoundary {
    Direct,
    HandWrittenDecode,
}

pub(crate) const SCHEMA_DECODE_TARGET_PREFIX: &str = "schema-decode:";
pub(crate) const SCHEMA_DECODE_STEP_TARGET_PREFIX: &str = "schema-decode-step:";
pub(crate) const SCHEMA_ENCODE_TARGET_PREFIX: &str = "schema-encode:";
pub(crate) const SCHEMA_ENCODE_STEP_TARGET_PREFIX: &str = "schema-encode-step:";
pub(crate) const SCHEMA_VALIDATE_TARGET_PREFIX: &str = "schema-validate:";

pub(crate) enum FunctionLookup<'a> {
    Found(&'a FunctionSignature),
    Ambiguous,
    Missing,
}

pub(crate) enum MatchScrutineePatternInference {
    Uninferred,
    Inferred(Type),
    Ambiguous(Vec<String>),
}

impl<'a> FunctionLookup<'a> {
    pub(crate) fn found(self) -> Option<&'a FunctionSignature> {
        match self {
            Self::Found(function) => Some(function),
            Self::Ambiguous | Self::Missing => None,
        }
    }
}

pub(crate) struct CallOrigin {
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
    pub(crate) symbol: String,
    pub(crate) effects: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct EffectUse {
    pub(crate) effect: String,
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
    pub(crate) kind: &'static str,
    pub(crate) symbol: String,
}

#[derive(Clone)]
pub(crate) struct Binding {
    pub(crate) name: String,
    pub(crate) ty: Type,
}

type FunctionKey = (Option<String>, String);
type FunctionAstMap<'a> = BTreeMap<FunctionKey, &'a Function>;
type FunctionSignatureMap = BTreeMap<FunctionKey, FunctionSignature>;
type FunctionReturnMap = BTreeMap<FunctionKey, Type>;
type PrivateSlotOmissions = (Vec<bool>, bool);
type PrivateSlotMap = BTreeMap<FunctionKey, PrivateSlotOmissions>;

struct PrivateInferenceExprContext<'a> {
    expected: Option<&'a Type>,
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    returns_by_path: &'a FunctionReturnMap,
    adts: &'a AdtRegistry,
}

#[derive(Clone)]
pub(crate) struct ExpectedType {
    pub(crate) ty: Type,
    pub(crate) source: ExpectedTypeSource,
    pub(crate) origin_node_id: NodeId,
    pub(crate) origin_span: Option<SourceSpan>,
    pub(crate) origin_message: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) enum ExpectedTypeSource {
    DeclaredReturn,
    DeclaredParameter,
    LocalAnnotation,
    Inferred,
    Unknown,
}

impl ExpectedTypeSource {
    pub(crate) fn as_type_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn => "declared_return",
            Self::DeclaredParameter => "declared_parameter",
            Self::LocalAnnotation => "local_annotation",
            Self::Inferred => "inferred_expression",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn as_hole_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn | Self::DeclaredParameter | Self::LocalAnnotation => "declared",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    Unknown,
    Named {
        name: String,
        args: Vec<Type>,
    },
    Record(Vec<(String, Type)>),
    Function {
        params: Vec<Type>,
        variadic: Option<Box<Type>>,
        return_type: Box<Type>,
        effects: Vec<String>,
    },
}

impl Type {
    pub(crate) fn named(name: impl Into<String>, args: Vec<Type>) -> Self {
        Self::Named {
            name: name.into(),
            args,
        }
    }

    pub(crate) fn bool() -> Self {
        Self::named("Bool", Vec::new())
    }

    pub(crate) fn int() -> Self {
        Self::named("Int", Vec::new())
    }

    pub(crate) fn float() -> Self {
        Self::named("Float", Vec::new())
    }

    pub(crate) fn string() -> Self {
        Self::named("String", Vec::new())
    }

    pub(crate) fn unit() -> Self {
        Self::named("Unit", Vec::new())
    }

    #[cfg(test)]
    pub(crate) fn result(value: Type, error: Type) -> Self {
        Self::named("Result", vec![value, error])
    }

    pub(crate) fn vec(item: Type) -> Self {
        Self::named("Vec", vec![item])
    }

    pub(crate) fn dict(key: Type, value: Type) -> Self {
        Self::named("Dict", vec![key, value])
    }

    pub(crate) fn function(params: Vec<Type>, return_type: Type, effects: Vec<String>) -> Self {
        Self::Function {
            params,
            variadic: None,
            return_type: Box::new(return_type),
            effects,
        }
    }

    pub(crate) fn variadic_function(
        params: Vec<Type>,
        variadic: Type,
        return_type: Type,
        effects: Vec<String>,
    ) -> Self {
        Self::Function {
            params,
            variadic: Some(Box::new(variadic)),
            return_type: Box::new(return_type),
            effects,
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Unknown => "unknown".to_string(),
            Self::Named { name, args } if name == "Unit" && args.is_empty() => "()".to_string(),
            Self::Named { name, args } if args.is_empty() => name.clone(),
            Self::Named { name, args } => {
                let args = args.iter().map(Type::render).collect::<Vec<_>>().join(", ");
                format!("{name}<{args}>")
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
                variadic,
                return_type,
                effects,
            } => {
                let mut rendered_params = params.iter().map(Type::render).collect::<Vec<_>>();
                if let Some(variadic) = variadic {
                    rendered_params.push(format!("...{}", variadic.render()));
                }
                let params = rendered_params.join(", ");
                let effects = if effects.is_empty() {
                    String::new()
                } else {
                    format!(" effects [{}]", effects.join(", "))
                };
                format!("fn({params}) -> {}{effects}", return_type.render())
            }
        }
    }

    pub(crate) fn vec_part(&self) -> Option<&Type> {
        match self {
            Self::Named { name, args } if name == "Vec" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    pub(crate) fn dict_parts(&self) -> Option<(&Type, &Type)> {
        match self {
            Self::Named { name, args } if name == "Dict" && args.len() == 2 => {
                Some((&args[0], &args[1]))
            }
            _ => None,
        }
    }

    pub(crate) fn record_field(&self, field_name: &str) -> Option<&Type> {
        match self {
            Self::Record(fields) => fields
                .iter()
                .find_map(|(name, ty)| (name == field_name).then_some(ty)),
            _ => None,
        }
    }

    pub(crate) fn function_parts(&self) -> Option<(&[Type], &Type)> {
        match self {
            Self::Function {
                params,
                variadic,
                return_type,
                ..
            } if variadic.is_none() => Some((params, return_type)),
            _ => None,
        }
    }

    pub(crate) fn callable_parts(&self) -> Option<(&[Type], Option<&Type>, &Type)> {
        match self {
            Self::Function {
                params,
                variadic,
                return_type,
                ..
            } => Some((params, variadic.as_deref(), return_type)),
            _ => None,
        }
    }

    pub(crate) fn function_effects(&self) -> Option<&[String]> {
        match self {
            Self::Function { effects, .. } => Some(effects),
            _ => None,
        }
    }
}

impl TypeEnvironment {
    pub(crate) fn from_module(module: &SurfaceModule) -> Self {
        let mut functions = ordinary_function_signatures(module);
        let adts = AdtRegistry::from_module(module);
        infer_private_function_body_return_types(module, &mut functions, &adts);
        infer_private_function_call_site_signature_types(module, &mut functions, &adts);
        infer_private_function_body_return_types(module, &mut functions, &adts);
        infer_private_prelude_callback_return_types(module, &mut functions, &adts);
        functions.extend(schema_decode_function_signatures(module));
        functions.extend(schema_encode_function_signatures(module));
        functions.extend(schema_validate_function_signatures(module));
        infer_function_body_effects(module, &mut functions);
        let codec_calls = codec_call_signatures(module, &functions);
        let aliases = function_alias_signatures(module, &functions);
        functions.extend(aliases);
        Self {
            functions,
            codec_calls,
            uses: module.uses.clone(),
            adts,
        }
    }

    pub(crate) fn function(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.iter().find(|function| function.name == name)
    }

    pub(crate) fn function_by_node_id(&self, node_id: NodeId) -> Option<&FunctionSignature> {
        self.functions
            .iter()
            .find(|function| function.node_id == node_id)
    }

    pub(crate) fn unqualified_function(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> FunctionLookup<'_> {
        if let Some(function) = self.functions.iter().find(|function| {
            function.name == name && function.module_name.as_deref() == current_module
        }) {
            return FunctionLookup::Found(function);
        }

        let mut matches = self.functions.iter().filter(|function| {
            function.name == name
                && function.visibility == Visibility::Public
                && function.module_name.as_deref().is_some_and(|module_name| {
                    self.uses.iter().any(|use_decl| {
                        use_decl.module_name.as_deref() == current_module
                            && use_decl.name.as_str() == module_name
                    })
                })
        });
        let Some(first) = matches.next() else {
            return FunctionLookup::Missing;
        };
        if matches.next().is_some() {
            FunctionLookup::Ambiguous
        } else {
            FunctionLookup::Found(first)
        }
    }

    pub(crate) fn unqualified_function_import_candidates(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> Vec<&FunctionSignature> {
        self.functions
            .iter()
            .filter(|function| {
                function.name == name
                    && function.visibility == Visibility::Public
                    && function.module_name.as_deref().is_some_and(|module_name| {
                        self.uses.iter().any(|use_decl| {
                            use_decl.module_name.as_deref() == current_module
                                && use_decl.name.as_str() == module_name
                        })
                    })
            })
            .collect()
    }

    pub(crate) fn function_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        match segments {
            [name] => self.function(name),
            [_, .., name] => {
                let use_decl = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                )?;
                let module_name = use_decl.name.as_str();
                self.functions.iter().find(|function| {
                    function.name == *name
                        && function.module_name.as_deref() == Some(module_name)
                        && imported_function_is_visible(function, use_decl)
                })
            }
            _ => None,
        }
    }

    pub(crate) fn unqualified_codec_calls(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> Vec<&CodecCallSignature> {
        self.codec_calls
            .iter()
            .filter(|codec| codec.name == name && codec.module_name.as_deref() == current_module)
            .collect()
    }

    pub(crate) fn codec_call_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<&CodecCallSignature> {
        match segments {
            [name] => self.unqualified_codec_calls(name, current_module),
            [_, .., name] => {
                let Some(use_decl) = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                ) else {
                    return Vec::new();
                };
                let module_name = use_decl.name.as_str();
                self.codec_calls
                    .iter()
                    .filter(|codec| {
                        codec.name == *name
                            && codec.module_name.as_deref() == Some(module_name)
                            && codec.visibility == Visibility::Public
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

pub(crate) fn ordinary_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            let name = function.name.clone()?;
            let (params, variadic) = function_signature_params(function);
            let return_type = parse_type_or_unknown(function.return_type.as_deref());
            Some(FunctionSignature {
                target_name: name.clone(),
                name,
                module_name: function.module_name.clone(),
                visibility: function.visibility,
                params,
                variadic,
                return_type,
                effects: function.effects.clone().unwrap_or_default(),
                node_id: function.node_id,
                span: function.span.clone(),
            })
        })
        .collect()
}

fn function_signature_params(function: &veln_ast::Function) -> (Vec<Type>, Option<Type>) {
    let mut params = Vec::new();
    let mut variadic = None;
    for param in &function.params {
        let ty = parse_type_or_unknown(param.ty.as_deref());
        if param.is_variadic {
            variadic = Some(ty);
        } else {
            params.push(ty);
        }
    }
    (params, variadic)
}

fn infer_private_function_body_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let mut changed = true;
    while changed {
        changed = false;
        let signatures_by_path = signatures_by_path(functions);
        let returns_by_path = returns_by_path(functions);
        for function in module.functions.iter().filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        }) {
            let Some(name) = &function.name else {
                continue;
            };
            let key = (function.module_name.clone(), name.clone());
            let inferred = infer_private_function_tail_type(
                function,
                &module.uses,
                &signatures_by_path,
                &returns_by_path,
                adts,
            );
            if inferred == Type::Unknown {
                continue;
            }
            let Some(signature) = functions
                .iter_mut()
                .find(|signature| signature.module_name == key.0 && signature.name == key.1)
            else {
                continue;
            };
            if signature.return_type == inferred {
                continue;
            }
            if !type_has_unknown(&signature.return_type) {
                continue;
            }
            signature.return_type = inferred;
            changed = true;
        }
    }
}

fn infer_private_function_call_site_signature_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let function_by_path = module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let omitted_private_slots = module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.name.is_some()
        })
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                (
                    function
                        .params
                        .iter()
                        .map(|param| param.ty.is_none())
                        .collect::<Vec<_>>(),
                    function.return_type.is_none(),
                ),
            ))
        })
        .collect::<BTreeMap<_, _>>();

    let mut changed = true;
    while changed {
        changed = false;
        let signatures_by_path = signatures_by_path(functions);
        let returns_by_path = returns_by_path(functions);
        for function in &module.functions {
            collect_private_call_site_constraints(
                function,
                &mut PrivateCallSiteConstraintContext {
                    uses: &module.uses,
                    function_by_path: &function_by_path,
                    omitted_private_slots: &omitted_private_slots,
                    signatures_by_path: &signatures_by_path,
                    returns_by_path: &returns_by_path,
                    functions,
                    adts,
                    changed: &mut changed,
                },
            );
        }
    }
}

fn signatures_by_path(functions: &[FunctionSignature]) -> FunctionSignatureMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.clone(),
            )
        })
        .collect()
}

fn returns_by_path(functions: &[FunctionSignature]) -> FunctionReturnMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect()
}

struct PrivateCallSiteConstraintContext<'a> {
    uses: &'a [UseDecl],
    function_by_path: &'a FunctionAstMap<'a>,
    omitted_private_slots: &'a PrivateSlotMap,
    signatures_by_path: &'a FunctionSignatureMap,
    returns_by_path: &'a FunctionReturnMap,
    functions: &'a mut [FunctionSignature],
    adts: &'a AdtRegistry,
    changed: &'a mut bool,
}

struct PrivateCallSiteExprContext<'a, 'b> {
    current_module: Option<&'b str>,
    caller_key: Option<&'b FunctionKey>,
    bindings: &'b [Binding],
    constraints: &'b mut PrivateCallSiteConstraintContext<'a>,
}

fn collect_private_call_site_constraints(
    function: &Function,
    context: &mut PrivateCallSiteConstraintContext<'_>,
) {
    let current_module = function.module_name.as_deref();
    let caller_key = function
        .name
        .as_ref()
        .map(|name| (function.module_name.clone(), name.clone()));
    let mut bindings = private_function_body_bindings(function, context.signatures_by_path);
    let declared_return = function.return_type.as_deref().map_or_else(
        || {
            caller_key
                .as_ref()
                .and_then(|key| context.signatures_by_path.get(key))
                .map(|signature| signature.return_type.clone())
                .filter(|ty| !type_has_unknown(ty))
        },
        |return_type| Some(parse_type_or_unknown(Some(return_type))),
    );

    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                collect_private_call_site_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        caller_key: caller_key.as_ref(),
                        bindings: &bindings,
                        constraints: context,
                    },
                );
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        current_module,
                        context.uses,
                        &bindings,
                        context.returns_by_path,
                        context.adts,
                    )
                });
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                collect_private_call_site_expr_constraints(
                    expr,
                    expected,
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        caller_key: caller_key.as_ref(),
                        bindings: &bindings,
                        constraints: context,
                    },
                );
            }
        }
    }
}

fn collect_private_call_site_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_call_site_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_call_site_expr_constraints(&entry.key, key_expected, context);
                collect_private_call_site_expr_constraints(&entry.value, value_expected, context);
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_call_site_expr_constraints(&field.expr, field_expected, context);
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_call_site_call_constraints(callee, args, expected, context);
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_call_site_expr_constraints(base, None, context);
        }
        ExprKind::Match { scrutinee, arms } => {
            let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
                arms,
                context.current_module,
                context.constraints.uses,
                context.constraints.adts,
            ) {
                MatchScrutineePatternInference::Inferred(ty) => Some(ty),
                MatchScrutineePatternInference::Uninferred
                | MatchScrutineePatternInference::Ambiguous(_) => None,
            };
            collect_private_call_site_expr_constraints(
                scrutinee,
                scrutinee_expected.as_ref(),
                context,
            );
            for arm in arms {
                collect_private_call_site_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_call_site_expr_constraints(condition, Some(&Type::bool()), context);
            collect_private_call_site_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_call_site_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_call_site_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_call_site_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_call_site_expr_constraints(left, expected, context);
            collect_private_call_site_expr_constraints(right, expected, context);
        }
        ExprKind::NamePath(segments) => {
            collect_private_parameter_constraints(segments, expected, context);
            collect_private_function_value_constraints(segments, expected, context);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn collect_private_parameter_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(expected) = expected.filter(|ty| !type_has_unknown(ty)) else {
        return;
    };
    let [name] = segments else {
        return;
    };
    let Some(caller_key) = context.caller_key else {
        return;
    };
    let Some((omitted_params, _)) = context.constraints.omitted_private_slots.get(caller_key)
    else {
        return;
    };
    let Some(function) = context.constraints.function_by_path.get(caller_key) else {
        return;
    };
    let Some(index) = function
        .params
        .iter()
        .position(|param| param.name == *name && param.ty.is_none())
    else {
        return;
    };
    if !omitted_params.get(index).copied().unwrap_or(false) {
        return;
    }
    update_private_signature_param(
        context.constraints.functions,
        caller_key,
        index,
        expected.clone(),
        context.constraints.changed,
    );
}

fn collect_private_call_site_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(target_key) = private_same_module_call_target(
        callee,
        context.current_module,
        context.constraints.function_by_path,
    ) else {
        collect_private_call_site_non_target_call_args(callee, args, expected, context);
        return;
    };

    let is_recursive_edge = context.caller_key == Some(&target_key);
    if !is_recursive_edge
        && let Some((omitted_params, omitted_return)) =
            context.constraints.omitted_private_slots.get(&target_key)
    {
        if let Some(target_params) = context
            .constraints
            .signatures_by_path
            .get(&target_key)
            .map(|signature| signature.params.clone())
        {
            for (index, arg) in args.iter().enumerate() {
                if omitted_params.get(index).copied().unwrap_or(false) {
                    let actual = infer_private_signature_expr_type(
                        arg,
                        None,
                        context.current_module,
                        context.constraints.uses,
                        context.bindings,
                        context.constraints.returns_by_path,
                        context.constraints.adts,
                    );
                    if !type_has_unknown(&actual) {
                        update_private_signature_param(
                            context.constraints.functions,
                            &target_key,
                            index,
                            actual,
                            context.constraints.changed,
                        );
                    }
                }
                let arg_expected = target_params
                    .get(index)
                    .filter(|ty| private_expected_can_constrain(ty));
                collect_private_call_site_expr_constraints(arg, arg_expected, context);
            }
        }

        if *omitted_return
            && let Some(expected) = expected
            && !type_has_unknown(expected)
        {
            update_private_signature_return(
                context.constraints.functions,
                &target_key,
                expected.clone(),
                context.constraints.changed,
            );
        }
    }
}

fn collect_private_call_site_non_target_call_args(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let ExprKind::NamePath(segments) = &callee.kind else {
        for arg in args {
            collect_private_call_site_expr_constraints(arg, None, context);
        }
        return;
    };
    let params = private_call_site_non_target_params(segments, args, expected, context);
    for (index, arg) in args.iter().enumerate() {
        let arg_expected = params
            .get(index)
            .filter(|ty| private_expected_can_constrain(ty));
        collect_private_call_site_expr_constraints(arg, arg_expected, context);
    }
}

fn private_expected_can_constrain(ty: &Type) -> bool {
    if !type_has_unknown(ty) {
        return true;
    }
    matches!(
        ty,
        Type::Function {
            params,
            return_type,
            ..
        } if params.iter().any(|param| !type_has_unknown(param))
            || !type_has_unknown(return_type)
    )
}

fn private_call_site_non_target_params(
    segments: &[String],
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateCallSiteExprContext<'_, '_>,
) -> Vec<Type> {
    if let crate::adt::ConstructorLookup::Found(constructor) = context.constraints.adts.constructor(
        segments,
        context.current_module,
        context.constraints.uses,
    ) {
        return expected
            .and_then(|expected| adt::adt_args(expected, constructor.descriptor))
            .map(|_| {
                constructor
                    .variant
                    .payload_fields
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        expected
                            .and_then(|expected| adt::payload_type(expected, constructor, index))
                            .unwrap_or(Type::Unknown)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }

    if let Some(signature) = private_call_site_declared_signature(
        segments,
        context.current_module,
        context.constraints.uses,
        context.constraints.signatures_by_path,
    ) {
        return signature.params.clone();
    }

    private_prelude_constraint_name(
        segments,
        context.current_module,
        context.constraints.function_by_path,
    )
    .and_then(|name| {
        let input_type = private_prelude_input_arg(args, name).map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.constraints.uses,
                context.bindings,
                context.constraints.returns_by_path,
                context.constraints.adts,
            )
        });
        let mut params =
            crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
                .map(|(params, _)| params)?;
        if name == "vec_try_map_with" {
            let context_type = args.first().map(|arg| {
                infer_private_signature_expr_type(
                    arg,
                    None,
                    context.current_module,
                    context.constraints.uses,
                    context.bindings,
                    context.constraints.returns_by_path,
                    context.constraints.adts,
                )
            });
            apply_vec_try_map_with_context_param(&mut params, context_type);
        }
        Some(params)
    })
    .unwrap_or_default()
}

fn private_prelude_input_arg<'a>(args: &'a [Expr], helper_name: &str) -> Option<&'a Expr> {
    match helper_name {
        "vec_try_map_with" | "dict_map_with" | "dict_filter_with" | "dict_fold_with"
        | "dict_try_map_with" => args.get(1),
        _ => args.first(),
    }
}

fn collect_private_function_value_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(Type::Function {
        params,
        return_type,
        ..
    }) = expected
    else {
        return;
    };
    let [name] = segments else {
        return;
    };
    let target_key = (context.current_module.map(str::to_string), name.clone());
    if context.caller_key == Some(&target_key) {
        return;
    }
    let Some((omitted_params, omitted_return)) =
        context.constraints.omitted_private_slots.get(&target_key)
    else {
        return;
    };
    for (index, param) in params.iter().enumerate() {
        if omitted_params.get(index).copied().unwrap_or(false) && !type_has_unknown(param) {
            update_private_signature_param(
                context.constraints.functions,
                &target_key,
                index,
                param.clone(),
                context.constraints.changed,
            );
        }
    }
    if *omitted_return && !type_has_unknown(return_type) {
        update_private_signature_return(
            context.constraints.functions,
            &target_key,
            return_type.as_ref().clone(),
            context.constraints.changed,
        );
    }
}

fn private_call_site_declared_signature<'a>(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
    signatures_by_path: &'a FunctionSignatureMap,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => signatures_by_path.get(&(current_module.map(str::to_string), name.clone())),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    signatures_by_path.get(&(Some(use_decl.name.clone()), name.clone()))
                })
                .filter(|signature| signature.visibility == Visibility::Public)
        }
        _ => None,
    }
}

fn private_same_module_call_target(
    callee: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
) -> Option<FunctionKey> {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return None;
    };
    let [name] = segments.as_slice() else {
        return None;
    };
    let key = (current_module.map(str::to_string), name.clone());
    let function = function_by_path.get(&key)?;
    (function.kind == FunctionKind::Function && function.visibility == Visibility::Private)
        .then_some(key)
}

fn update_private_signature_param(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    index: usize,
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    let Some(current) = signature.params.get_mut(index) else {
        return;
    };
    if type_has_unknown(current) {
        *current = inferred;
        *changed = true;
    }
}

fn update_private_signature_return(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    if type_has_unknown(&signature.return_type) {
        signature.return_type = inferred;
        *changed = true;
    }
}

fn infer_private_prelude_callback_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let function_by_path = module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let omitted_private_returns = module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        })
        .filter_map(|function| Some((function.module_name.clone(), function.name.clone()?)))
        .collect::<BTreeSet<_>>();
    let mut returns_by_path = functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut changed = true;
    while changed {
        changed = false;
        for function in module
            .functions
            .iter()
            .filter(|function| function.kind == FunctionKind::Function)
        {
            collect_private_prelude_callback_return_constraints(
                function,
                &module.uses,
                &function_by_path,
                &omitted_private_returns,
                &mut returns_by_path,
                adts,
                &mut changed,
            );
        }
    }

    for function in functions {
        let key = (function.module_name.clone(), function.name.clone());
        if !omitted_private_returns.contains(&key) {
            continue;
        }
        if let Some(inferred) = returns_by_path.get(&key)
            && inferred != &function.return_type
        {
            function.return_type = inferred.clone();
        }
    }
}

fn collect_private_prelude_callback_return_constraints(
    function: &Function,
    uses: &[UseDecl],
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
    omitted_private_returns: &BTreeSet<(Option<String>, String)>,
    returns_by_path: &mut BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
    changed: &mut bool,
) {
    let mut bindings = function
        .params
        .iter()
        .map(|param| Binding {
            name: param.name.clone(),
            ty: function_body_param_type(param),
        })
        .collect::<Vec<_>>();
    let declared_return = function
        .return_type
        .as_deref()
        .map(|return_type| parse_type_or_unknown(Some(return_type)));
    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    expected,
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
            }
        }
    }
}

struct PrivatePreludeCallbackConstraintContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    function_by_path: &'a BTreeMap<(Option<String>, String), &'a Function>,
    omitted_private_returns: &'a BTreeSet<(Option<String>, String)>,
    returns_by_path: &'a mut BTreeMap<(Option<String>, String), Type>,
    adts: &'a AdtRegistry,
    changed: &'a mut bool,
}

fn collect_private_prelude_callback_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_prelude_callback_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_prelude_callback_expr_constraints(
                    &entry.key,
                    key_expected,
                    context,
                );
                collect_private_prelude_callback_expr_constraints(
                    &entry.value,
                    value_expected,
                    context,
                );
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_prelude_callback_expr_constraints(
                    &field.expr,
                    field_expected,
                    context,
                );
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_prelude_callback_call_constraints(callee, args, expected, context);
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_prelude_callback_expr_constraints(base, None, context);
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_private_prelude_callback_expr_constraints(scrutinee, None, context);
            for arm in arms {
                collect_private_prelude_callback_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_prelude_callback_expr_constraints(
                condition,
                Some(&Type::bool()),
                context,
            );
            collect_private_prelude_callback_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_prelude_callback_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_prelude_callback_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_prelude_callback_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_prelude_callback_expr_constraints(left, expected, context);
            collect_private_prelude_callback_expr_constraints(right, expected, context);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn collect_private_prelude_callback_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return;
    };
    let Some(name) =
        private_prelude_constraint_name(segments, context.current_module, context.function_by_path)
    else {
        return;
    };
    let input_type = private_prelude_input_arg(args, name).map(|arg| {
        infer_private_signature_expr_type(
            arg,
            None,
            context.current_module,
            context.uses,
            context.bindings,
            context.returns_by_path,
            context.adts,
        )
    });
    let Some((mut params, _)) =
        crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
    else {
        return;
    };
    if name == "vec_try_map_with" {
        let context_type = args.first().map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.uses,
                context.bindings,
                context.returns_by_path,
                context.adts,
            )
        });
        apply_vec_try_map_with_context_param(&mut params, context_type);
    }
    for (arg, param) in args.iter().zip(params.iter()) {
        collect_private_callback_return_constraint(arg, param, context);
        collect_private_prelude_callback_expr_constraints(arg, Some(param), context);
    }
}

fn apply_vec_try_map_with_context_param(params: &mut [Type], context_type: Option<Type>) {
    let Some(context_type) = context_type else {
        return;
    };
    if let Some(param) = params.first_mut() {
        *param = context_type.clone();
    }
    let Some(Type::Function {
        params: callback_params,
        ..
    }) = params.get_mut(2)
    else {
        return;
    };
    if let Some(callback_context) = callback_params.first_mut() {
        *callback_context = context_type;
    }
}

fn private_prelude_constraint_name<'a>(
    segments: &'a [String],
    current_module: Option<&str>,
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
) -> Option<&'a str> {
    match segments {
        [name]
            if !function_by_path
                .contains_key(&(current_module.map(str::to_string), name.clone())) =>
        {
            Some(name)
        }
        [module, name] if module == "prelude" || module == "prelude_builtin" => Some(name),
        _ => None,
    }
}

fn collect_private_callback_return_constraint(
    arg: &Expr,
    expected_callback: &Type,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Type::Function { return_type, .. } = expected_callback else {
        return;
    };
    if type_has_unknown(return_type) {
        return;
    }
    let ExprKind::NamePath(segments) = &arg.kind else {
        return;
    };
    let [name] = segments.as_slice() else {
        return;
    };
    let key = (context.current_module.map(str::to_string), name.clone());
    if !context.omitted_private_returns.contains(&key) {
        return;
    }
    let Some(function) = context.function_by_path.get(&key) else {
        return;
    };
    if !private_tail_can_use_expected(function, return_type, context.uses, context.adts) {
        return;
    }
    if context.returns_by_path.get(&key) == Some(return_type) {
        return;
    }
    context
        .returns_by_path
        .insert(key, return_type.as_ref().clone());
    *context.changed = true;
}

fn private_tail_can_use_expected(
    function: &Function,
    expected: &Type,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    let Some(BodyLineKind::Expr { expr }) = function.body.last().map(|line| &line.kind) else {
        return false;
    };
    tail_expr_can_use_expected(expr, expected, function.module_name.as_deref(), uses, adts)
}

fn tail_expr_can_use_expected(
    expr: &Expr,
    expected: &Type,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    match &expr.kind {
        ExprKind::List(_) => expected.vec_part().is_some(),
        ExprKind::Dict(_) => expected.dict_parts().is_some(),
        ExprKind::Record(fields) => {
            if fields.is_empty() && expected.dict_parts().is_some() {
                return true;
            }
            !fields.is_empty()
                && fields
                    .iter()
                    .all(|field| expected.record_field(&field.name).is_some())
        }
        ExprKind::NamePath(segments) => {
            matches!(
                adts.nullary_constructor(segments, current_module, uses),
                crate::adt::ConstructorLookup::Found(constructor)
                    if adt::adt_args(expected, constructor.descriptor).is_some()
            )
        }
        ExprKind::Call { callee, .. } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return false;
            };
            matches!(
                adts.constructor(segments, current_module, uses),
                crate::adt::ConstructorLookup::Found(constructor)
                    if adt::adt_args(expected, constructor.descriptor).is_some()
            )
        }
        ExprKind::Match { arms, .. } => arms
            .iter()
            .all(|arm| tail_expr_can_use_expected(&arm.expr, expected, current_module, uses, adts)),
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => std::iter::once(then_branch.as_ref())
            .chain(else_if_branches.iter().map(|branch| &branch.expr))
            .chain(std::iter::once(else_branch.as_ref()))
            .all(|branch| tail_expr_can_use_expected(branch, expected, current_module, uses, adts)),
        _ => false,
    }
}

fn infer_private_function_tail_type(
    function: &veln_ast::Function,
    uses: &[UseDecl],
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    let mut bindings = private_function_body_bindings(function, signatures_by_path);
    let mut tail = Type::unit();
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                tail = infer_private_signature_expr_type(
                    expr,
                    None,
                    function.module_name.as_deref(),
                    uses,
                    &bindings,
                    returns_by_path,
                    adts,
                );
            }
        }
    }
    tail
}

fn private_function_body_bindings(
    function: &veln_ast::Function,
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
) -> Vec<Binding> {
    let signature = function
        .name
        .as_ref()
        .and_then(|name| signatures_by_path.get(&(function.module_name.clone(), name.clone())));
    function
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let ty = if param.is_variadic {
                signature
                    .and_then(|signature| signature.variadic.clone())
                    .map(|ty| Type::named("List", vec![ty]))
                    .unwrap_or_else(|| function_body_param_type(param))
            } else {
                signature
                    .and_then(|signature| signature.params.get(index).cloned())
                    .unwrap_or_else(|| function_body_param_type(param))
            };
            Binding {
                name: param.name.clone(),
                ty,
            }
        })
        .collect()
}

fn infer_private_signature_expr_type(
    expr: &Expr,
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    match &expr.kind {
        ExprKind::Missing | ExprKind::Hole { .. } | ExprKind::TypeApply { .. } => Type::Unknown,
        ExprKind::StringLiteral(_) => Type::string(),
        ExprKind::IntLiteral(_) => Type::int(),
        ExprKind::FloatLiteral(_) => Type::float(),
        ExprKind::BoolLiteral(_) => Type::bool(),
        ExprKind::Unit => Type::unit(),
        ExprKind::NamePath(segments) => infer_private_signature_name_type(
            segments,
            expected,
            current_module,
            uses,
            bindings,
            returns_by_path,
            adts,
        ),
        ExprKind::List(items) => {
            let expected_item = expected
                .and_then(Type::vec_part)
                .cloned()
                .unwrap_or(Type::Unknown);
            let mut item_type = expected_item;
            for item in items {
                let actual = infer_private_signature_expr_type(
                    item,
                    item_type_unknown_as_none(&item_type),
                    current_module,
                    uses,
                    bindings,
                    returns_by_path,
                    adts,
                );
                if item_type == Type::Unknown {
                    item_type = actual;
                }
            }
            Type::vec(item_type)
        }
        ExprKind::Dict(entries) => {
            let (mut key_type, mut value_type) = expected
                .and_then(Type::dict_parts)
                .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
                    (key.clone(), value.clone())
                });
            for entry in entries {
                let key_actual = infer_private_signature_expr_type(
                    &entry.key,
                    item_type_unknown_as_none(&key_type),
                    current_module,
                    uses,
                    bindings,
                    returns_by_path,
                    adts,
                );
                if key_type == Type::Unknown {
                    key_type = key_actual;
                }
                let value_actual = infer_private_signature_expr_type(
                    &entry.value,
                    item_type_unknown_as_none(&value_type),
                    current_module,
                    uses,
                    bindings,
                    returns_by_path,
                    adts,
                );
                if value_type == Type::Unknown {
                    value_type = value_actual;
                }
            }
            Type::dict(key_type, value_type)
        }
        ExprKind::Record(fields) => {
            if fields.is_empty()
                && let Some(expected) = expected
                && expected.dict_parts().is_some()
            {
                return expected.clone();
            }
            Type::Record(
                fields
                    .iter()
                    .map(|field| {
                        let field_expected =
                            expected.and_then(|expected| expected.record_field(&field.name));
                        (
                            field.name.clone(),
                            infer_private_signature_expr_type(
                                &field.expr,
                                field_expected,
                                current_module,
                                uses,
                                bindings,
                                returns_by_path,
                                adts,
                            ),
                        )
                    })
                    .collect(),
            )
        }
        ExprKind::Call { callee, args } => infer_private_signature_call_type(
            callee,
            args,
            expected,
            &PrivateSignatureInferContext {
                current_module,
                uses,
                bindings,
                returns_by_path,
                adts,
            },
        ),
        ExprKind::FieldAccess { base, field, .. } => infer_private_signature_expr_type(
            base,
            None,
            current_module,
            uses,
            bindings,
            returns_by_path,
            adts,
        )
        .record_field(field)
        .cloned()
        .unwrap_or(Type::Unknown),
        ExprKind::Try(inner) => expected.cloned().unwrap_or_else(|| {
            let inner_type = infer_private_signature_expr_type(
                inner,
                None,
                current_module,
                uses,
                bindings,
                returns_by_path,
                adts,
            );
            adt::result_parts(&inner_type).map_or(Type::Unknown, |(value, _)| value.clone())
        }),
        ExprKind::Match { scrutinee, arms } => {
            let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
                arms,
                current_module,
                uses,
                adts,
            ) {
                MatchScrutineePatternInference::Inferred(ty) => Some(ty),
                MatchScrutineePatternInference::Uninferred
                | MatchScrutineePatternInference::Ambiguous(_) => None,
            };
            infer_private_signature_expr_type(
                scrutinee,
                scrutinee_expected.as_ref(),
                current_module,
                uses,
                bindings,
                returns_by_path,
                adts,
            );
            let mut result = expected.cloned().unwrap_or(Type::Unknown);
            for arm in arms {
                let actual = infer_private_signature_expr_type(
                    &arm.expr,
                    item_type_unknown_as_none(&result),
                    current_module,
                    uses,
                    bindings,
                    returns_by_path,
                    adts,
                );
                if result == Type::Unknown {
                    result = actual;
                }
            }
            result
        }
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => infer_private_if_result_type(
            then_branch,
            else_if_branches,
            else_branch,
            &PrivateInferenceExprContext {
                expected,
                current_module,
                uses,
                bindings,
                returns_by_path,
                adts,
            },
        ),
        ExprKind::Prefix { expr, .. } => {
            infer_private_signature_expr_type(
                expr,
                expected,
                current_module,
                uses,
                bindings,
                returns_by_path,
                adts,
            );
            Type::Unknown
        }
        ExprKind::Binary { op, left, right } => match op {
            veln_ast::BinaryOp::Equal
            | veln_ast::BinaryOp::NotEqual
            | veln_ast::BinaryOp::Less
            | veln_ast::BinaryOp::LessEqual
            | veln_ast::BinaryOp::Greater
            | veln_ast::BinaryOp::GreaterEqual
            | veln_ast::BinaryOp::Or
            | veln_ast::BinaryOp::And => Type::bool(),
            veln_ast::BinaryOp::Add
            | veln_ast::BinaryOp::Subtract
            | veln_ast::BinaryOp::Multiply
            | veln_ast::BinaryOp::Divide => {
                let left = infer_private_signature_expr_type(
                    left,
                    expected,
                    current_module,
                    uses,
                    bindings,
                    returns_by_path,
                    adts,
                );
                let right = infer_private_signature_expr_type(
                    right,
                    expected,
                    current_module,
                    uses,
                    bindings,
                    returns_by_path,
                    adts,
                );
                if left == Type::float() || right == Type::float() {
                    Type::float()
                } else {
                    Type::int()
                }
            }
            veln_ast::BinaryOp::PipeGreater => Type::Unknown,
        },
    }
}

fn infer_private_if_result_type(
    then_branch: &Expr,
    else_if_branches: &[IfBranch],
    else_branch: &Expr,
    context: &PrivateInferenceExprContext<'_>,
) -> Type {
    let mut result = context.expected.cloned().unwrap_or(Type::Unknown);
    for branch_expr in std::iter::once(then_branch)
        .chain(else_if_branches.iter().map(|branch| &branch.expr))
        .chain(std::iter::once(else_branch))
    {
        let actual = infer_private_signature_expr_type(
            branch_expr,
            item_type_unknown_as_none(&result),
            context.current_module,
            context.uses,
            context.bindings,
            context.returns_by_path,
            context.adts,
        );
        if result == Type::Unknown {
            result = actual;
        }
    }
    result
}

fn item_type_unknown_as_none(ty: &Type) -> Option<&Type> {
    (ty != &Type::Unknown).then_some(ty)
}

pub(crate) fn infer_match_scrutinee_type_from_constructor_patterns(
    arms: &[MatchArm],
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> MatchScrutineePatternInference {
    let mut inferred: Option<(crate::adt::AdtConstructor<'_>, Vec<Type>)> = None;

    for arm in arms {
        let PatternKind::Constructor { name, args } = &arm.pattern.kind else {
            continue;
        };
        let candidates = adts.constructor_candidates(name, current_module, uses);
        if candidates.is_empty() {
            continue;
        }
        let descriptor_names = unique_constructor_descriptor_names(&candidates);
        if descriptor_names.len() != 1 {
            return MatchScrutineePatternInference::Ambiguous(descriptor_names);
        }
        let constructor = candidates[0];
        if let Some((previous, _)) = &inferred {
            if !same_constructor_descriptor(previous, &constructor) {
                let mut names = unique_constructor_descriptor_names(&[*previous, constructor]);
                names.sort();
                return MatchScrutineePatternInference::Ambiguous(names);
            }
        } else {
            inferred = Some((
                constructor,
                vec![Type::Unknown; constructor.descriptor.type_parameters.len()],
            ));
        }
        let Some((_, type_args)) = &mut inferred else {
            continue;
        };
        for (index, pattern) in args.iter().enumerate() {
            let Some(pattern_type) =
                infer_pattern_type_from_constructor_patterns(pattern, current_module, uses, adts)
            else {
                continue;
            };
            adt::merge_type_args_from_payload(type_args, constructor, index, &pattern_type);
        }
    }

    match inferred {
        Some((constructor, type_args)) => MatchScrutineePatternInference::Inferred(
            adt::constructed_type_from_args(constructor, &type_args),
        ),
        None => MatchScrutineePatternInference::Uninferred,
    }
}

fn infer_pattern_type_from_constructor_patterns(
    pattern: &Pattern,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> Option<Type> {
    match &pattern.kind {
        PatternKind::StringLiteral(_) => Some(Type::string()),
        PatternKind::IntLiteral(_) => Some(Type::int()),
        PatternKind::FloatLiteral(_) => Some(Type::float()),
        PatternKind::BoolLiteral(_) => Some(Type::bool()),
        PatternKind::Unit => Some(Type::unit()),
        PatternKind::Record(fields) => Some(Type::Record(
            fields
                .iter()
                .map(|field| {
                    (
                        field.name.clone(),
                        infer_pattern_type_from_constructor_patterns(
                            &field.pattern,
                            current_module,
                            uses,
                            adts,
                        )
                        .unwrap_or(Type::Unknown),
                    )
                })
                .collect(),
        )),
        PatternKind::Constructor { name, args } => {
            let candidates = adts.constructor_candidates(name, current_module, uses);
            let [constructor] = candidates.as_slice() else {
                return None;
            };
            let mut type_args = vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
            for (index, pattern) in args.iter().enumerate() {
                let Some(pattern_type) = infer_pattern_type_from_constructor_patterns(
                    pattern,
                    current_module,
                    uses,
                    adts,
                ) else {
                    continue;
                };
                adt::merge_type_args_from_payload(
                    &mut type_args,
                    *constructor,
                    index,
                    &pattern_type,
                );
            }
            Some(adt::constructed_type_from_args(*constructor, &type_args))
        }
        PatternKind::Wildcard | PatternKind::Binding(_) => None,
    }
}

fn unique_constructor_descriptor_names(
    constructors: &[crate::adt::AdtConstructor<'_>],
) -> Vec<String> {
    let mut names = Vec::new();
    for constructor in constructors {
        let name = constructor.descriptor.diagnostic_name.clone();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn same_constructor_descriptor(
    left: &crate::adt::AdtConstructor<'_>,
    right: &crate::adt::AdtConstructor<'_>,
) -> bool {
    left.descriptor.type_name == right.descriptor.type_name
        && left.descriptor.module_name == right.descriptor.module_name
        && left.descriptor.type_parameters.len() == right.descriptor.type_parameters.len()
}

fn type_has_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } => args.iter().any(type_has_unknown),
        Type::Record(fields) => fields.iter().any(|(_, ty)| type_has_unknown(ty)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(type_has_unknown)
                || variadic.as_deref().is_some_and(type_has_unknown)
                || type_has_unknown(return_type)
        }
    }
}

fn infer_private_signature_name_type(
    segments: &[String],
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    if let crate::adt::ConstructorLookup::Found(constructor) =
        adts.nullary_constructor(segments, current_module, uses)
    {
        return expected
            .and_then(|expected| {
                adt::adt_args(expected, constructor.descriptor).map(|_| expected.clone())
            })
            .unwrap_or_else(|| adt::constructed_type(constructor, &[]));
    }
    match segments {
        [name] => bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                returns_by_path
                    .get(&(current_module.map(str::to_string), name.clone()))
                    .cloned()
            })
            .unwrap_or(Type::Unknown),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    returns_by_path
                        .get(&(Some(use_decl.name.clone()), name.clone()))
                        .cloned()
                })
                .unwrap_or(Type::Unknown)
        }
        _ => Type::Unknown,
    }
}

struct PrivateSignatureInferContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    returns_by_path: &'a BTreeMap<(Option<String>, String), Type>,
    adts: &'a AdtRegistry,
}

fn infer_private_signature_call_type(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    if let ExprKind::NamePath(segments) = &callee.kind {
        if let crate::adt::ConstructorLookup::Found(constructor) =
            context
                .adts
                .constructor(segments, context.current_module, context.uses)
        {
            let actual_args = args
                .iter()
                .map(|arg| {
                    infer_private_signature_expr_type(
                        arg,
                        None,
                        context.current_module,
                        context.uses,
                        context.bindings,
                        context.returns_by_path,
                        context.adts,
                    )
                })
                .collect::<Vec<_>>();
            if expected
                .and_then(|expected| adt::adt_args(expected, constructor.descriptor))
                .is_some()
            {
                return expected.cloned().unwrap_or(Type::Unknown);
            }
            return adt::constructed_type(constructor, &actual_args);
        }
        if let Some(name) = segments.last() {
            if let Some(return_type) = match segments.as_slice() {
                [name] => context
                    .returns_by_path
                    .get(&(context.current_module.map(str::to_string), name.clone())),
                [_, .., name] => imported_use_for_path(
                    context.uses,
                    &segments[..segments.len() - 1],
                    context.current_module,
                )
                .and_then(|use_decl| {
                    context
                        .returns_by_path
                        .get(&(Some(use_decl.name.clone()), name.clone()))
                }),
                _ => None,
            } {
                return return_type.clone();
            }
            if let Some((params, return_type)) = crate::prelude::prelude_signature(name, expected) {
                for (arg, param) in args.iter().zip(params.iter()) {
                    infer_private_signature_expr_type(
                        arg,
                        Some(param),
                        context.current_module,
                        context.uses,
                        context.bindings,
                        context.returns_by_path,
                        context.adts,
                    );
                }
                return return_type;
            }
        }
    }
    Type::Unknown
}

pub(crate) fn function_body_param_type(param: &veln_ast::Param) -> Type {
    let ty = parse_type_or_unknown(param.ty.as_deref());
    if param.is_variadic {
        Type::named("List", vec![ty])
    } else {
        ty
    }
}

fn codec_call_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<CodecCallSignature> {
    module
        .codecs
        .iter()
        .flat_map(|codec| {
            let name = codec.name.clone()?;
            Some(
                codec
                    .implementations
                    .iter()
                    .flat_map(move |implementation| {
                        match (&implementation.direction, &implementation.kind) {
                            (
                                CodecDirection::Decode,
                                CodecImplementationKind::With {
                                    function: Some(function_name),
                                },
                            ) => codec_with_signature(
                                codec,
                                functions,
                                name.clone(),
                                function_name,
                                CodecCallBoundary::HandWrittenDecode,
                            )
                            .into_iter()
                            .collect(),
                            (
                                CodecDirection::Encode,
                                CodecImplementationKind::With {
                                    function: Some(function_name),
                                },
                            ) => codec_with_signature(
                                codec,
                                functions,
                                name.clone(),
                                function_name,
                                CodecCallBoundary::Direct,
                            )
                            .into_iter()
                            .collect(),
                            (CodecDirection::Decode, CodecImplementationKind::Derive) => {
                                codec_derive_decode_signature(
                                    module,
                                    functions,
                                    codec,
                                    name.clone(),
                                )
                                .into_iter()
                                .collect()
                            }
                            (CodecDirection::Encode, CodecImplementationKind::Derive) => {
                                codec_derive_encode_signatures(
                                    module,
                                    functions,
                                    codec,
                                    name.clone(),
                                )
                            }
                            (_, CodecImplementationKind::With { function: None }) => Vec::new(),
                        }
                    }),
            )
        })
        .flatten()
        .collect()
}

fn codec_with_signature(
    codec: &CodecDecl,
    functions: &[FunctionSignature],
    name: String,
    function_name: &str,
    boundary: CodecCallBoundary,
) -> Option<CodecCallSignature> {
    let function = functions.iter().find(|function| {
        function.name == function_name && function.module_name == codec.module_name
    })?;
    Some(CodecCallSignature {
        name,
        target_name: function.target_name.clone(),
        boundary,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    })
}

fn codec_derive_decode_signature(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    codec: &CodecDecl,
    name: String,
) -> Option<CodecCallSignature> {
    let schema = codec_referenced_schema(module, codec)?;
    let schema_name = schema.name.as_ref()?;
    let step_name = schema_decode_step_function_name(schema_name);
    let function = functions.iter().find(|function| {
        function.name == step_name && function.module_name == schema.module_name
    })?;
    Some(CodecCallSignature {
        name,
        target_name: function.target_name.clone(),
        boundary: CodecCallBoundary::Direct,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    })
}

fn codec_derive_encode_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    codec: &CodecDecl,
    name: String,
) -> Vec<CodecCallSignature> {
    let Some(schema) = codec_referenced_schema(module, codec) else {
        return Vec::new();
    };
    let Some(schema_name) = schema.name.as_ref() else {
        return Vec::new();
    };
    let encode_name = schema_encode_function_name(schema_name);
    let Some(function) = functions.iter().find(|function| {
        function.name == encode_name && function.module_name == schema.module_name
    }) else {
        return Vec::new();
    };
    let unbounded = CodecCallSignature {
        name,
        target_name: format!("{SCHEMA_ENCODE_STEP_TARGET_PREFIX}{schema_name}"),
        boundary: CodecCallBoundary::Direct,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: Type::named("EncodeStep", vec![Type::unit()]),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    };
    let Some(value_type) = function.params.first().cloned() else {
        return vec![unbounded];
    };
    let mut state_fields = match &value_type {
        Type::Record(fields) => fields.clone(),
        _ => Vec::new(),
    };
    state_fields.push((
        "encoded_offset".to_string(),
        Type::named("ByteCount", Vec::new()),
    ));
    let budgeted = CodecCallSignature {
        name: unbounded.name.clone(),
        target_name: unbounded.target_name.clone(),
        boundary: unbounded.boundary,
        module_name: unbounded.module_name.clone(),
        visibility: unbounded.visibility,
        params: vec![value_type, Type::named("ByteCount", Vec::new())],
        return_type: Type::named("EncodeStep", vec![Type::Record(state_fields)]),
        effects: unbounded.effects.clone(),
        node_id: unbounded.node_id,
        span: unbounded.span.clone(),
    };
    vec![unbounded, budgeted]
}

fn codec_referenced_schema<'a>(
    module: &'a SurfaceModule,
    codec: &CodecDecl,
) -> Option<&'a SchemaDecl> {
    let schema_name = codec.schema.as_ref()?;
    let segments = schema_name
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    schema_reference(
        module,
        &segments,
        codec.module_name.as_deref(),
        true,
        &mut Vec::new(),
    )
}

fn schema_reference<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> Option<&'a SchemaDecl> {
    match segments {
        [name] => schema_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
        ),
        [_, .., name] => {
            let use_decl = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            schema_in_module(module, Some(&use_decl.name), name, false, visited_aliases)
        }
        _ => None,
    }
}

fn schema_in_module<'a>(
    module: &'a SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> Option<&'a SchemaDecl> {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return (allow_private_schema || schema.visibility == Visibility::Public).then_some(schema);
    }
    let alias = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    })?;
    let alias_name = alias.name.as_ref()?;
    let key = (alias.module_name.clone(), alias_name.clone());
    if visited_aliases.contains(&key) {
        return None;
    }
    visited_aliases.push(key);
    let schema = schema_reference(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
    );
    visited_aliases.pop();
    schema
}

fn schema_decode_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .flat_map(|schema| schema_decode_function_signatures_for_schema(module, schema))
        .collect()
}

fn schema_decode_function_signatures_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Vec<FunctionSignature> {
    let Some(schema_name) = schema.name.as_ref() else {
        return Vec::new();
    };
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return Vec::new();
    }
    let Some(fields) = schema_decode_record_fields(module, schema) else {
        return Vec::new();
    };
    let byte_view = Type::named("ByteView", Vec::new());
    let byte_offset = Type::named("ByteOffset", Vec::new());
    let mapped_fields = schema_decode_mapping_record_fields(module, schema, &fields)
        .unwrap_or_else(|| fields.into_iter().map(|(name, ty, _)| (name, ty)).collect());
    let decoded_type = Type::Record(mapped_fields);
    let result = Type::named("Result", vec![decoded_type.clone(), Type::string()]);
    let step = Type::named("DecodeStep", vec![decoded_type]);
    vec![
        FunctionSignature {
            name: schema_decode_function_name(schema_name),
            target_name: format!("{SCHEMA_DECODE_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![byte_view.clone()],
            variadic: None,
            return_type: result,
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        },
        FunctionSignature {
            name: schema_decode_step_function_name(schema_name),
            target_name: format!("{SCHEMA_DECODE_STEP_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![byte_view, byte_offset],
            variadic: None,
            return_type: step,
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        },
    ]
}

pub(crate) fn schema_decode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type, u8)>> {
    schema_decode_record_fields_inner(module, schema, &mut Vec::new())
}

pub(crate) fn schema_decode_record_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    if schema.format.as_ref()?.name != "binary" {
        return None;
    }
    Some(Type::Record(
        schema_decode_record_fields(module, schema)?
            .into_iter()
            .map(|(name, ty, _)| (name, ty))
            .collect(),
    ))
}

fn schema_decode_record_fields_inner(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<Vec<(String, Type, u8)>> {
    let schema_name = schema.name.as_ref()?;
    if stack.iter().any(|name| name == schema_name) {
        return None;
    }
    stack.push(schema_name.clone());
    let fields = schema_decode_record_fields_inner_after_push(module, schema, stack);
    stack.pop();
    fields
}

fn schema_decode_record_fields_inner_after_push(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<Vec<(String, Type, u8)>> {
    let mut decoded_fields = BTreeMap::<String, Type>::new();
    let mut fields = Vec::new();
    for (index, field) in schema.fields.iter().enumerate() {
        if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
            supported_encode_reserved_bits(&schema.fields, index, reserved)?;
            continue;
        }
        let (width, ty) = if let Some(width) = exact_width_schema_primitive(&field.ty) {
            let ty = if let Some(flag_type) = flag_schema_primitive(&field.ty) {
                Type::named(flag_type, Vec::new())
            } else {
                Type::int()
            };
            (width, ty)
        } else if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
            if length_expr
                .references()
                .into_iter()
                .any(|reference| decoded_fields.get(reference) != Some(&Type::int()))
            {
                return None;
            }
            (0, Type::named("ByteView", Vec::new()))
        } else if let Some(repeat) = repeat_schema_primitive(&field.ty) {
            if schema_length_expression_references(&repeat.count_field)?
                .into_iter()
                .any(|reference| decoded_fields.get(reference) != Some(&Type::int()))
            {
                return None;
            }
            if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload
                && decoded_fields.get(length_field) != Some(&Type::int())
            {
                return None;
            }
            let element_ty = schema_repeat_payload_type(module, schema, &repeat, stack)?;
            (0, Type::named("List", vec![element_ty]))
        } else {
            let dispatch = closed_dispatch_schema_primitive(&field.ty)
                .or_else(|| extension_dispatch_schema_primitive(&field.ty))?;
            if decoded_fields.get(&dispatch.tag_field) != Some(&Type::int())
                || dispatch.length_field.as_ref().is_some_and(|length_field| {
                    decoded_fields.get(length_field) != Some(&Type::int())
                })
            {
                return None;
            }
            let payload_types = schema_dispatch_case_types(module, schema, &dispatch, stack)?;
            let payload_ty =
                schema_dispatch_payload_type(module, schema, field, &dispatch, &payload_types)?;
            let field_ty = if dispatch.preserves_unknown {
                Type::named("SchemaDispatchPayload", vec![payload_ty])
            } else {
                payload_ty
            };
            (0, field_ty)
        };
        decoded_fields.insert(field.name.clone(), ty.clone());
        fields.push((field.name.clone(), ty, width));
    }
    Some(fields)
}

fn schema_dispatch_case_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    stack: &mut Vec<String>,
) -> Option<Vec<(i64, Type)>> {
    dispatch
        .cases
        .iter()
        .map(|case| {
            let ty = schema_dispatch_case_type(module, schema, case, stack)?;
            Some((case.tag, ty))
        })
        .collect()
}

fn schema_dispatch_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    payload_types: &[(i64, Type)],
) -> Option<Type> {
    let first = payload_types.first()?.1.clone();
    if payload_types.iter().all(|(_, ty)| ty == &first) {
        Some(first)
    } else if selected_mappings_cover_closed_dispatch(schema, dispatch)
        || (dispatch.length_field.is_some()
            && dispatch.cases.iter().any(|case| {
                matches!(
                    &case.payload,
                    SchemaDispatchCasePayload::Schema { schema_name }
                        if recursive_dispatch_payload_case_is_eligible(
                            module,
                            schema,
                            field,
                            dispatch,
                            schema_name,
                        )
                )
            })
            && selected_mappings_cover_dispatch_cases(schema, dispatch))
    {
        schema_recursive_dispatch_payload_type(module, schema)
    } else {
        None
    }
}

pub(crate) fn schema_dispatch_case_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    case: &SchemaDispatchCase,
    stack: &mut Vec<String>,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::Schema { schema_name } => {
            if schema.name.as_deref() == Some(schema_name.as_str()) {
                return schema_recursive_dispatch_payload_type(module, schema);
            }
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
            schema_decode_value_type_inner(module, nested, stack)
        }
    }
}

fn schema_repeat_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    repeat: &SchemaRepeatSpec,
    stack: &mut Vec<String>,
) -> Option<Type> {
    match &repeat.payload {
        SchemaRepeatPayload::Primitive { .. } => Some(Type::int()),
        SchemaRepeatPayload::ByteView { .. } => Some(Type::named("ByteView", Vec::new())),
        SchemaRepeatPayload::Schema { schema_name } => {
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
            schema_decode_value_type_inner(module, nested, stack)
        }
    }
}

fn schema_decode_value_type_inner(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<Type> {
    let fields = schema_decode_record_fields_inner(module, schema, stack)?;
    let mapped_fields = schema_decode_mapping_record_fields(module, schema, &fields)
        .unwrap_or_else(|| fields.into_iter().map(|(name, ty, _)| (name, ty)).collect());
    Some(Type::Record(mapped_fields))
}

pub(crate) fn schema_recursive_dispatch_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    let [first_mapping, rest @ ..] = schema.mappings.as_slice() else {
        return None;
    };
    let target_fields = schema_mapping_target_record_fields(module, schema, first_mapping)?;
    for mapping in rest {
        mapping.selector.as_ref()?;
        if schema_mapping_target_record_fields(module, schema, mapping)? != target_fields {
            return None;
        }
    }
    Some(Type::Record(target_fields))
}

pub(crate) fn recursive_dispatch_payload_is_eligible(
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    schema.name.as_deref() == Some(schema_name)
        && dispatch.length_field.is_some()
        && schema.mappings.len() == dispatch.cases.len()
        && selected_mappings_cover_dispatch_cases(schema, dispatch)
        && schema
            .fields
            .iter()
            .position(|candidate| candidate.node_id == field.node_id)
            .is_some_and(|index| index > 0)
        && dispatch.cases.iter().any(|case| {
            !matches!(
                &case.payload,
                SchemaDispatchCasePayload::Schema { schema_name }
                    if schema.name.as_deref() == Some(schema_name.as_str())
            )
        })
}

pub(crate) fn recursive_dispatch_payload_case_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    if recursive_dispatch_payload_is_eligible(schema, field, dispatch, schema_name) {
        return true;
    }
    dispatch.length_field.is_some()
        && selected_mappings_cover_dispatch_cases(schema, dispatch)
        && dispatch_has_non_recursive_payload_case(module, schema, dispatch)
        && recursive_dispatch_payload_target_is_eligible(module, schema, schema_name)
}

pub(crate) fn schema_has_eligible_recursive_dispatch_payload(schema: &SchemaDecl) -> bool {
    let Some(schema_name) = schema.name.as_deref() else {
        return false;
    };
    schema.fields.iter().any(|field| {
        closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
            .is_some_and(|dispatch| {
                recursive_dispatch_payload_is_eligible(schema, field, &dispatch, schema_name)
            })
    })
}

pub(crate) fn schema_has_recursive_dispatch_payload(schema: &SchemaDecl) -> bool {
    let Some(schema_name) = schema.name.as_deref() else {
        return false;
    };
    schema.fields.iter().any(|field| {
        closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
            .is_some_and(|dispatch| {
                dispatch.cases.iter().any(|case| {
                    matches!(
                        &case.payload,
                        SchemaDispatchCasePayload::Schema { schema_name: payload_name }
                            if payload_name == schema_name
                    )
                })
            })
    })
}

fn recursive_dispatch_payload_target_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> bool {
    schema_dispatch_payload_schema(module, schema, schema_name)
        .is_some_and(schema_has_eligible_recursive_dispatch_payload)
}

fn dispatch_has_non_recursive_payload_case(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    dispatch.cases.iter().any(|case| match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => true,
        SchemaDispatchCasePayload::Schema { schema_name } => {
            !recursive_dispatch_payload_target_is_eligible(module, schema, schema_name)
        }
    })
}

pub(crate) fn same_module_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> Option<&'a SchemaDecl> {
    if schema_name.contains("::") {
        return None;
    }
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| candidate.node_id == schema.node_id)?;
    module
        .schemas
        .iter()
        .enumerate()
        .find_map(|(index, candidate)| {
            (candidate.name.as_deref() == Some(schema_name)
                && candidate.module_name.as_deref() == schema.module_name.as_deref()
                && candidate.format.as_ref().map(|format| format.name.as_str()) == Some("binary")
                && index < current_index)
                .then_some(candidate)
        })
}

pub(crate) fn schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> Option<&'a SchemaDecl> {
    let segments = schema_payload_name_path(schema_name)?;
    match segments.as_slice() {
        [name] => same_module_schema(module, schema, name),
        [_, .., name] => {
            let use_decl = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            )?;
            let target_module = Some(use_decl.name.as_str());
            module.schemas.iter().find(|candidate| {
                candidate.name.as_deref() == Some(name)
                    && candidate.module_name.as_deref() == target_module
                    && candidate.visibility == Visibility::Public
                    && candidate.format.as_ref().map(|format| format.name.as_str())
                        == Some("binary")
            })
        }
        _ => None,
    }
}

pub(crate) fn schema_decode_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    schema_decode_value_type_inner(module, schema, &mut Vec::new())
}

pub(crate) fn selected_mappings_cover_closed_dispatch(
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    !dispatch.preserves_unknown && selected_mappings_cover_dispatch_cases(schema, dispatch)
}

pub(crate) fn selected_mappings_cover_dispatch_cases(
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    if schema.mappings.len() != dispatch.cases.len() || schema.mappings.is_empty() {
        return false;
    }
    let case_tags = dispatch
        .cases
        .iter()
        .map(|case| case.tag)
        .collect::<BTreeSet<_>>();
    let mut selector_tags = BTreeSet::<i64>::new();
    for mapping in &schema.mappings {
        let Some(selector) = &mapping.selector else {
            return false;
        };
        let Some((field, SchemaMappingSelectorComparison::Equal, value)) =
            schema_mapping_selector_predicate(selector)
                .ok()
                .and_then(|predicate| {
                    predicate
                        .as_simple_comparison()
                        .map(|(field, op, value)| (field.to_string(), op, value))
                })
        else {
            return false;
        };
        if field != dispatch.tag_field
            || !case_tags.contains(&value)
            || !selector_tags.insert(value)
        {
            return false;
        }
    }
    selector_tags == case_tags
}

pub(crate) fn schema_payload_name_path(text: &str) -> Option<Vec<String>> {
    let segments = text.split("::").map(str::trim).collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| !is_schema_identifier(segment))
    {
        return None;
    }
    Some(segments.into_iter().map(str::to_string).collect())
}

pub(crate) fn schema_payload_name_is_path(text: &str) -> bool {
    schema_payload_name_path(text).is_some()
}

pub(crate) fn schema_payload_name_last_segment(text: &str) -> &str {
    text.rsplit("::").next().unwrap_or(text)
}

fn schema_encode_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_encode_function_signature_for_schema(module, schema))
        .collect()
}

fn schema_validate_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_validate_function_signature_for_schema(module, schema))
        .collect()
}

fn schema_validate_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let fields = schema_decode_record_fields(module, schema)?
        .into_iter()
        .map(|(name, ty, _)| (name, ty))
        .collect::<Vec<_>>();
    let decoded_type = Type::Record(fields);
    Some(FunctionSignature {
        name: schema_validate_function_name(schema_name),
        target_name: format!("{SCHEMA_VALIDATE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![decoded_type.clone()],
        variadic: None,
        return_type: Type::named("Result", vec![decoded_type, Type::string()]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

fn schema_encode_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let mut fields = Vec::new();
    let mut exact_width_field_names = Vec::new();
    for (index, field) in schema.fields.iter().enumerate() {
        if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
            supported_encode_reserved_bits(&schema.fields, index, reserved)?;
            continue;
        }
        if exact_width_schema_primitive(&field.ty).is_some() {
            exact_width_field_names.push(field.name.clone());
            let ty = if let Some(flag_type) = flag_schema_primitive(&field.ty) {
                Type::named(flag_type, Vec::new())
            } else {
                Type::int()
            };
            fields.push((field.name.clone(), ty));
            continue;
        }
        if let Some(repeat) = repeat_schema_primitive(&field.ty) {
            if schema_length_expression_references(&repeat.count_field)?
                .into_iter()
                .any(|reference| {
                    !exact_width_field_names
                        .iter()
                        .any(|field| field == reference)
                })
            {
                return None;
            }
            if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload
                && !exact_width_field_names
                    .iter()
                    .any(|field| field == length_field)
            {
                return None;
            }
            let element_ty = schema_repeat_payload_type(module, schema, &repeat, &mut Vec::new())?;
            fields.push((field.name.clone(), Type::named("List", vec![element_ty])));
            continue;
        }
        if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
            if length_expr.references().into_iter().any(|reference| {
                !exact_width_field_names
                    .iter()
                    .any(|field| field == reference)
            }) {
                return None;
            }
            fields.push((field.name.clone(), Type::named("ByteView", Vec::new())));
            continue;
        }
        let dispatch = closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))?;
        let recursive_dispatch_payload =
            recursive_dispatch_encode_payload_field(module, schema, field, &dispatch);
        if !exact_width_field_names.contains(&dispatch.tag_field)
            || dispatch
                .length_field
                .as_ref()
                .is_some_and(|length_field| !exact_width_field_names.contains(length_field))
            || (dispatch.length_field.is_some()
                && !dispatch.preserves_unknown
                && !recursive_dispatch_payload)
        {
            return None;
        }
        let mut payload_types = dispatch
            .cases
            .iter()
            .map(|case| schema_dispatch_case_type(module, schema, case, &mut Vec::new()))
            .collect::<Option<Vec<_>>>()?;
        let selected_mapping_closed_dispatch =
            selected_mappings_cover_closed_dispatch(schema, &dispatch);
        let payload_ty = if recursive_dispatch_payload
            && selected_mappings_cover_dispatch_cases(schema, &dispatch)
        {
            schema_recursive_dispatch_payload_type(module, schema)?
        } else if selected_mapping_closed_dispatch {
            payload_types.pop()?
        } else {
            let payload_ty = payload_types.pop()?;
            if payload_types.iter().any(|ty| ty != &payload_ty) {
                return None;
            }
            payload_ty
        };
        if !recursive_dispatch_payload
            && !selected_mapping_closed_dispatch
            && payload_types.iter().any(|ty| ty != &payload_ty)
        {
            return None;
        }
        if dispatch.preserves_unknown {
            fields.push((
                field.name.clone(),
                Type::named("SchemaDispatchPayload", vec![payload_ty]),
            ));
        } else {
            fields.push((field.name.clone(), payload_ty));
        }
    }
    let value_fields =
        schema_encode_value_fields(module, schema, &fields, &exact_width_field_names)?;
    let byte_chunk = Type::named("ByteChunk", Vec::new());
    let encode_error = Type::named("EncodeError", Vec::new());
    Some(FunctionSignature {
        name: schema_encode_function_name(schema_name),
        target_name: format!("{SCHEMA_ENCODE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![Type::Record(value_fields)],
        variadic: None,
        return_type: Type::named("Result", vec![byte_chunk, encode_error]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

fn recursive_dispatch_encode_payload_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    dispatch.cases.iter().any(|case| {
        matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name }
                if recursive_dispatch_payload_case_is_eligible(
                    module,
                    schema,
                    field,
                    dispatch,
                    schema_name,
                )
        )
    })
}

pub(crate) fn schema_encode_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    schema_encode_function_signature_for_schema(module, schema)
        .and_then(|signature| signature.params.into_iter().next())
}

fn schema_encode_value_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    exact_width_field_names: &[String],
) -> Option<Vec<(String, Type)>> {
    let [] = schema.mappings.as_slice() else {
        return schema_encode_mapping_value_fields(
            module,
            schema,
            schema_fields,
            exact_width_field_names,
        );
    };
    Some(schema_fields.to_vec())
}

fn schema_encode_mapping_value_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    exact_width_field_names: &[String],
) -> Option<Vec<(String, Type)>> {
    let [mapping] = schema.mappings.as_slice() else {
        return schema_encode_selected_mapping_value_fields(
            module,
            schema,
            schema_fields,
            exact_width_field_names,
        );
    };
    let target_fields = schema_mapping_target_record_fields(module, schema, mapping)?;
    let schema_field_types = schema_fields.iter().cloned().collect::<BTreeMap<_, _>>();
    let typer = SchemaMappingTyper::new(module, schema);
    let source_context = SchemaEncodeMappingSourceContext {
        module,
        typer: &typer,
        schema_field_types: &schema_field_types,
        exact_width_field_names,
        allow_single_payload_variant: false,
    };
    if mapping.selector.is_some()
        || schema_encode_mapping_source_targets(
            schema_fields,
            &source_context,
            mapping,
            &target_fields,
        )
        .is_none()
    {
        return None;
    }
    Some(target_fields)
}

fn schema_encode_selected_mapping_value_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    exact_width_field_names: &[String],
) -> Option<Vec<(String, Type)>> {
    let [first, rest @ ..] = schema.mappings.as_slice() else {
        return None;
    };
    first.selector.as_ref()?;
    let target_fields = schema_mapping_target_record_fields(module, schema, first)?;
    let (schema_field_types, supported_int_field_names) = schema_encode_mapping_field_types(
        module,
        schema,
        schema_fields,
        exact_width_field_names,
        first,
    )?;
    let typer = SchemaMappingTyper::new(module, schema);
    let source_context = SchemaEncodeMappingSourceContext {
        module,
        typer: &typer,
        schema_field_types: &schema_field_types,
        exact_width_field_names: &supported_int_field_names,
        allow_single_payload_variant: true,
    };
    schema_encode_mapping_source_targets(schema_fields, &source_context, first, &target_fields)?;
    for mapping in rest {
        mapping.selector.as_ref()?;
        let candidate_target_fields = schema_mapping_target_record_fields(module, schema, mapping)?;
        if candidate_target_fields != target_fields {
            return None;
        }
        let (schema_field_types, supported_int_field_names) = schema_encode_mapping_field_types(
            module,
            schema,
            schema_fields,
            exact_width_field_names,
            mapping,
        )?;
        let source_context = SchemaEncodeMappingSourceContext {
            module,
            typer: &typer,
            schema_field_types: &schema_field_types,
            exact_width_field_names: &supported_int_field_names,
            allow_single_payload_variant: true,
        };
        schema_encode_mapping_source_targets(
            schema_fields,
            &source_context,
            mapping,
            &target_fields,
        )?;
    }
    Some(target_fields)
}

fn schema_encode_mapping_field_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    exact_width_field_names: &[String],
    mapping: &veln_ast::SchemaMappingClause,
) -> Option<(BTreeMap<String, Type>, Vec<String>)> {
    let mut schema_field_types = schema_fields.iter().cloned().collect::<BTreeMap<_, _>>();
    let mut supported_int_field_names = exact_width_field_names.to_vec();
    let selector = mapping.selector.as_ref()?;
    for field in &schema.fields {
        let Some(dispatch) = closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
        else {
            continue;
        };
        let selector_case =
            schema_mapping_selector_predicate(selector)
                .ok()
                .and_then(|predicate| {
                    predicate
                        .as_simple_comparison()
                        .map(|(field, op, value)| (field.to_string(), op, value))
                });
        let Some((selector_field, SchemaMappingSelectorComparison::Equal, selector_value)) =
            selector_case
        else {
            continue;
        };
        let recursive_payload = dispatch.cases.iter().any(|case| {
            matches!(
                &case.payload,
                SchemaDispatchCasePayload::Schema { schema_name }
                    if recursive_dispatch_payload_case_is_eligible(
                        module,
                        schema,
                        field,
                        &dispatch,
                        schema_name,
                    )
            )
        });
        if selector_field != dispatch.tag_field
            || (dispatch.preserves_unknown && !recursive_payload)
        {
            continue;
        }
        let case = dispatch
            .cases
            .iter()
            .find(|case| case.tag == selector_value)?;
        let case_ty = schema_dispatch_case_type(module, schema, case, &mut Vec::new())?;
        if case_ty == Type::int() {
            supported_int_field_names.push(field.name.clone());
        }
        schema_field_types.insert(field.name.clone(), case_ty);
    }
    Some((schema_field_types, supported_int_field_names))
}

struct SchemaEncodeMappingSourceContext<'a> {
    module: &'a SurfaceModule,
    typer: &'a SchemaMappingTyper<'a>,
    schema_field_types: &'a BTreeMap<String, Type>,
    exact_width_field_names: &'a [String],
    allow_single_payload_variant: bool,
}

fn schema_encode_mapping_source_targets(
    schema_fields: &[(String, Type)],
    context: &SchemaEncodeMappingSourceContext<'_>,
    mapping: &veln_ast::SchemaMappingClause,
    target_fields: &[(String, Type)],
) -> Option<BTreeMap<String, String>> {
    let target_field_types = target_fields.iter().cloned().collect::<BTreeMap<_, _>>();
    let mut source_to_target = BTreeMap::<String, String>::new();
    for assignment in &mapping.assignments {
        let target_ty = target_field_types.get(&assignment.target)?;
        let sources = schema_encode_mapping_assignment_sources(
            context,
            context.schema_field_types,
            context.exact_width_field_names,
            assignment,
            target_ty,
            context.allow_single_payload_variant,
        )?;
        for source in sources {
            if source_to_target
                .insert(source.clone(), assignment.target.clone())
                .is_some()
            {
                return None;
            }
        }
    }
    if schema_fields
        .iter()
        .any(|(source, _)| !source_to_target.contains_key(source))
    {
        return None;
    }
    Some(source_to_target)
}

fn schema_encode_mapping_assignment_sources(
    context: &SchemaEncodeMappingSourceContext<'_>,
    schema_field_types: &BTreeMap<String, Type>,
    exact_width_field_names: &[String],
    assignment: &veln_ast::SchemaMappingAssignment,
    target_ty: &Type,
    allow_single_payload_variant: bool,
) -> Option<Vec<String>> {
    if let ExprKind::NamePath(segments) = &assignment.expr.kind {
        let [source] = segments.as_slice() else {
            return None;
        };
        let source_ty = schema_field_types.get(source)?;
        return is_assignable(target_ty, source_ty).then(|| vec![source.clone()]);
    }

    let typed = context
        .typer
        .assignment_expr_typed_for_codegen(schema_field_types, assignment, target_ty)
        .ok()?;
    match typed.expr {
        SchemaDecodeMappingExpr::Constructor { name, args } => {
            let registry = AdtRegistry::from_module(context.module);
            let descriptor = registry.descriptor_for_type(target_ty)?;
            if name.len() != 2 || name[0] != descriptor.type_name {
                return None;
            }
            if !descriptor.variants.iter().any(|variant| {
                variant.name == name[1] && variant.payload_fields.len() == args.len()
            }) {
                return None;
            }
            if !allow_single_payload_variant
                && args.len() == 1
                && descriptor.variants.len() != 1
                && !matches!(
                    args.first(),
                    Some(
                        SchemaDecodeMappingExpr::Record(_)
                            | SchemaDecodeMappingExpr::Constructor { .. }
                    )
                )
            {
                return None;
            }
            let mut sources = Vec::with_capacity(args.len());
            for arg in args {
                let arg_sources = schema_encode_mapping_expr_sources(
                    &arg,
                    schema_field_types,
                    exact_width_field_names,
                )?;
                if arg_sources.is_empty() {
                    return None;
                }
                sources.extend(arg_sources);
            }
            Some(sources)
        }
        expr => {
            let sources = schema_encode_mapping_expr_sources(
                &expr,
                schema_field_types,
                exact_width_field_names,
            )?;
            (!sources.is_empty()).then_some(sources)
        }
    }
}

fn schema_encode_mapping_expr_sources(
    expr: &SchemaDecodeMappingExpr,
    schema_field_types: &BTreeMap<String, Type>,
    exact_width_field_names: &[String],
) -> Option<Vec<String>> {
    match expr {
        SchemaDecodeMappingExpr::Field(source) => {
            let source_ty = schema_field_types.get(source)?;
            schema_encode_mapping_source_supported(source, source_ty, exact_width_field_names)
                .then(|| vec![source.clone()])
        }
        SchemaDecodeMappingExpr::Record(fields) => {
            let mut sources = Vec::with_capacity(fields.len());
            for field in fields {
                let SchemaDecodeMappingExpr::Field(source) = &field.expr else {
                    return None;
                };
                let source_ty = schema_field_types.get(source)?;
                if !schema_encode_mapping_source_supported(
                    source,
                    source_ty,
                    exact_width_field_names,
                ) {
                    return None;
                }
                sources.push(source.clone());
            }
            Some(sources)
        }
        SchemaDecodeMappingExpr::FieldAccess { base, field } => {
            schema_encode_mapping_selected_record_source(
                base,
                field,
                schema_field_types,
                exact_width_field_names,
            )
            .map(|source| vec![source])
        }
        SchemaDecodeMappingExpr::Constructor { args, .. } => {
            let mut sources = Vec::new();
            for arg in args {
                let arg_sources = schema_encode_mapping_expr_sources(
                    arg,
                    schema_field_types,
                    exact_width_field_names,
                )?;
                sources.extend(arg_sources);
            }
            Some(sources)
        }
        SchemaDecodeMappingExpr::Converter {
            inverse_function,
            args,
            ..
        } => {
            inverse_function.as_ref()?;
            let [arg] = args.as_slice() else {
                return None;
            };
            schema_encode_mapping_expr_sources(
                &arg.expr,
                schema_field_types,
                exact_width_field_names,
            )
        }
        SchemaDecodeMappingExpr::Binary { op, left, right } => {
            schema_encode_mapping_arithmetic_sources(
                *op,
                left,
                right,
                schema_field_types,
                exact_width_field_names,
            )
        }
        SchemaDecodeMappingExpr::Literal(_) | SchemaDecodeMappingExpr::Prefix { .. } => None,
    }
}

fn schema_encode_mapping_arithmetic_sources(
    op: BinaryOp,
    left: &SchemaDecodeMappingExpr,
    right: &SchemaDecodeMappingExpr,
    schema_field_types: &BTreeMap<String, Type>,
    exact_width_field_names: &[String],
) -> Option<Vec<String>> {
    match op {
        BinaryOp::Add => schema_encode_mapping_field_literal_source(
            left,
            right,
            schema_field_types,
            exact_width_field_names,
        )
        .or_else(|| {
            schema_encode_mapping_field_literal_source(
                right,
                left,
                schema_field_types,
                exact_width_field_names,
            )
        }),
        BinaryOp::Subtract => schema_encode_mapping_field_literal_source(
            left,
            right,
            schema_field_types,
            exact_width_field_names,
        ),
        _ => None,
    }
}

fn schema_encode_mapping_field_literal_source(
    field_expr: &SchemaDecodeMappingExpr,
    literal_expr: &SchemaDecodeMappingExpr,
    schema_field_types: &BTreeMap<String, Type>,
    exact_width_field_names: &[String],
) -> Option<Vec<String>> {
    let SchemaDecodeMappingExpr::Field(source) = field_expr else {
        return None;
    };
    let SchemaDecodeMappingExpr::Literal(_) = literal_expr else {
        return None;
    };
    let source_ty = schema_field_types.get(source)?;
    schema_encode_mapping_source_supported(source, source_ty, exact_width_field_names)
        .then(|| vec![source.clone()])
}

fn schema_encode_mapping_selected_record_source(
    base: &SchemaDecodeMappingExpr,
    field: &str,
    schema_field_types: &BTreeMap<String, Type>,
    exact_width_field_names: &[String],
) -> Option<String> {
    let SchemaDecodeMappingExpr::Record(fields) = base else {
        return None;
    };
    let selected = fields.iter().find(|candidate| candidate.name == field)?;
    let SchemaDecodeMappingExpr::Field(source) = &selected.expr else {
        return None;
    };
    let source_ty = schema_field_types.get(source)?;
    schema_encode_mapping_source_supported(source, source_ty, exact_width_field_names)
        .then(|| source.clone())
}

fn schema_encode_mapping_source_supported(
    source: &str,
    ty: &Type,
    exact_width_field_names: &[String],
) -> bool {
    is_type_flag_bitset(ty)
        || ty != &Type::int()
        || exact_width_field_names.iter().any(|field| field == source)
}

fn is_type_flag_bitset(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { name, args }
            if matches!(
                name.as_str(),
                "Flag8" | "Flag16be" | "Flag16le" | "Flag24be" | "Flag24le"
                    | "Flag32be" | "Flag32le" | "Flag40be" | "Flag40le"
                    | "Flag48be" | "Flag48le" | "Flag56be" | "Flag56le"
                    | "Flag64be" | "Flag64le"
            )
                && args.is_empty()
    )
}

pub(crate) fn schema_decode_function_name(schema_name: &str) -> String {
    format!("byte_decode_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_decode_step_function_name(schema_name: &str) -> String {
    format!("byte_decode_step_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_encode_function_name(schema_name: &str) -> String {
    format!("byte_encode_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_validate_function_name(schema_name: &str) -> String {
    format!("validate_{}", snake_case_identifier(schema_name))
}

pub(crate) fn exact_width_schema_primitive(ty: &str) -> Option<u8> {
    match ty.trim() {
        "UInt1" | "UInt2" | "UInt3" | "UInt4" | "UInt5" | "UInt6" | "UInt7" => Some(1),
        "UInt8" | "Flag8" => Some(1),
        "Flag16be" | "Flag16le" => Some(2),
        "Flag24be" | "Flag24le" => Some(3),
        "Flag32be" | "Flag32le" => Some(4),
        "Flag40be" | "Flag40le" => Some(5),
        "Flag48be" | "Flag48le" => Some(6),
        "Flag56be" | "Flag56le" => Some(7),
        "Flag64be" | "Flag64le" => Some(8),
        "UInt16be" | "UInt16le" => Some(2),
        "UInt24be" | "UInt24le" => Some(3),
        "UInt31be" | "UInt31le" | "UInt32be" | "UInt32le" => Some(4),
        "UInt40be" | "UInt40le" => Some(5),
        "UInt48be" | "UInt48le" => Some(6),
        "UInt56be" | "UInt56le" => Some(7),
        "UInt64be" | "UInt64le" => Some(8),
        _ => None,
    }
}

pub(crate) fn exact_width_schema_primitive_little_endian(ty: &str) -> bool {
    matches!(
        ty.trim(),
        "UInt16le"
            | "Flag16le"
            | "Flag24le"
            | "UInt24le"
            | "UInt31le"
            | "UInt32le"
            | "UInt40le"
            | "Flag40le"
            | "Flag48le"
            | "UInt48le"
            | "UInt56le"
            | "Flag56le"
            | "Flag32le"
            | "UInt64le"
            | "Flag64le"
    )
}

pub(crate) fn flag_schema_primitive(ty: &str) -> Option<&'static str> {
    match ty.trim() {
        "Flag8" => Some("Flag8"),
        "Flag16be" => Some("Flag16be"),
        "Flag16le" => Some("Flag16le"),
        "Flag24be" => Some("Flag24be"),
        "Flag24le" => Some("Flag24le"),
        "Flag32be" => Some("Flag32be"),
        "Flag32le" => Some("Flag32le"),
        "Flag40be" => Some("Flag40be"),
        "Flag40le" => Some("Flag40le"),
        "Flag48be" => Some("Flag48be"),
        "Flag48le" => Some("Flag48le"),
        "Flag56be" => Some("Flag56be"),
        "Flag56le" => Some("Flag56le"),
        "Flag64be" => Some("Flag64be"),
        "Flag64le" => Some("Flag64le"),
        _ => None,
    }
}

pub(crate) fn exact_width_schema_primitive_bit_width(ty: &str) -> Option<u8> {
    match ty.trim() {
        "UInt1" => Some(1),
        "UInt2" => Some(2),
        "UInt3" => Some(3),
        "UInt4" => Some(4),
        "UInt5" => Some(5),
        "UInt6" => Some(6),
        "UInt7" => Some(7),
        "UInt8" | "Flag8" => Some(8),
        "Flag16be" | "Flag16le" => Some(16),
        "Flag24be" | "Flag24le" => Some(24),
        "Flag32be" | "Flag32le" => Some(32),
        "Flag40be" | "Flag40le" => Some(40),
        "Flag48be" | "Flag48le" => Some(48),
        "Flag56be" | "Flag56le" => Some(56),
        "Flag64be" | "Flag64le" => Some(64),
        "UInt16be" | "UInt16le" => Some(16),
        "UInt24be" | "UInt24le" => Some(24),
        "UInt31be" | "UInt31le" => Some(31),
        "UInt32be" | "UInt32le" => Some(32),
        "UInt40be" | "UInt40le" => Some(40),
        "UInt48be" | "UInt48le" => Some(48),
        "UInt56be" | "UInt56le" => Some(56),
        "UInt64be" | "UInt64le" => Some(64),
        _ => None,
    }
}

pub(crate) fn exact_width_schema_primitive_max_value(ty: &str) -> Option<i64> {
    match ty.trim() {
        "UInt1" => Some(0x1),
        "UInt2" => Some(0x3),
        "UInt3" => Some(0x7),
        "UInt4" => Some(0xf),
        "UInt5" => Some(0x1f),
        "UInt6" => Some(0x3f),
        "UInt7" => Some(0x7f),
        "UInt8" | "Flag8" => Some(0xff),
        "Flag16be" | "Flag16le" => Some(0xffff),
        "Flag24be" | "Flag24le" => Some(0xffffff),
        "Flag32be" | "Flag32le" => Some(0xffffffff),
        "Flag40be" | "Flag40le" => Some(0xffffffffff),
        "Flag48be" | "Flag48le" => Some(0xffffffffffff),
        "Flag56be" | "Flag56le" => Some(0xffffffffffffff),
        "Flag64be" | "Flag64le" => Some(i64::MAX),
        "UInt16be" | "UInt16le" => Some(0xffff),
        "UInt24be" | "UInt24le" => Some(0xffffff),
        "UInt31be" | "UInt31le" => Some(0x7fffffff),
        "UInt32be" | "UInt32le" => Some(0xffffffff),
        "UInt40be" | "UInt40le" => Some(0xffffffffff),
        "UInt48be" | "UInt48le" => Some(0xffffffffffff),
        "UInt56be" | "UInt56le" => Some(0xffffffffffffff),
        "UInt64be" | "UInt64le" => Some(i64::MAX),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ByteViewLengthExpr {
    Field(String),
    Sum { left: String, right: String },
    Difference { left: String, right: String },
    Product { left: String, right: String },
    Quotient { left: String, right: String },
}

impl ByteViewLengthExpr {
    pub(crate) fn references(&self) -> Vec<&str> {
        match self {
            Self::Field(field) => vec![field.as_str()],
            Self::Sum { left, right } => vec![left.as_str(), right.as_str()],
            Self::Difference { left, right } => vec![left.as_str(), right.as_str()],
            Self::Product { left, right } => vec![left.as_str(), right.as_str()],
            Self::Quotient { left, right } => vec![left.as_str(), right.as_str()],
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Field(field) => field.clone(),
            Self::Sum { left, right } => format!("{left} + {right}"),
            Self::Difference { left, right } => format!("{left} - {right}"),
            Self::Product { left, right } => format!("{left} * {right}"),
            Self::Quotient { left, right } => format!("{left} / {right}"),
        }
    }
}

pub(crate) fn schema_length_expression(text: &str) -> Option<ByteViewLengthExpr> {
    schema_length_expression_with_product(text, true)
}

fn schema_length_expression_with_product(
    text: &str,
    allow_product: bool,
) -> Option<ByteViewLengthExpr> {
    let text = text.trim();
    if is_simple_schema_field_reference(text) {
        return Some(ByteViewLengthExpr::Field(text.to_string()));
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '+') {
        return Some(ByteViewLengthExpr::Sum {
            left: left.to_string(),
            right: right.to_string(),
        });
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '-') {
        return Some(ByteViewLengthExpr::Difference {
            left: left.to_string(),
            right: right.to_string(),
        });
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '/') {
        return Some(ByteViewLengthExpr::Quotient {
            left: left.to_string(),
            right: right.to_string(),
        });
    }
    if !allow_product {
        return None;
    }
    let (left, right) = schema_length_binary_expression_operands(text, '*')?;
    Some(ByteViewLengthExpr::Product {
        left: left.to_string(),
        right: right.to_string(),
    })
}

pub(crate) fn schema_length_expression_references(text: &str) -> Option<Vec<&str>> {
    let text = text.trim();
    if is_simple_schema_field_reference(text) {
        return Some(vec![text]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '+') {
        return Some(vec![left, right]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '-') {
        return Some(vec![left, right]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '*') {
        return Some(vec![left, right]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '/') {
        return Some(vec![left, right]);
    }
    None
}

fn schema_length_binary_expression_operands(text: &str, op: char) -> Option<(&str, &str)> {
    for other_op in ['+', '-', '*', '/'] {
        if other_op != op && text.contains(other_op) {
            return None;
        }
    }
    let (left, right) = text.split_once(op)?;
    if right.contains(op) {
        return None;
    }
    let left = left.trim();
    let right = right.trim();
    if is_simple_schema_field_reference(left) && is_simple_schema_field_reference(right) {
        Some((left, right))
    } else {
        None
    }
}

pub(crate) fn byte_view_schema_primitive(ty: &str) -> Option<ByteViewLengthExpr> {
    let text = ty.trim();
    let inner = text.strip_prefix("ByteView(")?.strip_suffix(')')?.trim();
    schema_length_expression_with_product(inner, true)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaRepeatSpec {
    pub(crate) count_field: String,
    pub(crate) payload: SchemaRepeatPayload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaRepeatPayload {
    Primitive {
        width: u8,
        max_value: i64,
        little_endian: bool,
    },
    ByteView {
        length_field: String,
    },
    Schema {
        schema_name: String,
    },
}

pub(crate) fn repeat_schema_primitive(ty: &str) -> Option<SchemaRepeatSpec> {
    let inner = schema_call_inner(ty, "Repeat")?;
    let args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    let [count_field, primitive] = args.as_slice() else {
        return None;
    };
    let count_expr = schema_length_expression(count_field)?;
    let payload = if let Some(width) = exact_width_schema_primitive(primitive) {
        if exact_width_schema_primitive_bit_width(primitive)? < 8
            || flag_schema_primitive(primitive).is_some()
        {
            return None;
        }
        SchemaRepeatPayload::Primitive {
            width,
            max_value: exact_width_schema_primitive_max_value(primitive)?,
            little_endian: exact_width_schema_primitive_little_endian(primitive),
        }
    } else if let Some(length_expr) = byte_view_schema_primitive(primitive) {
        match length_expr {
            ByteViewLengthExpr::Field(length_field) => {
                SchemaRepeatPayload::ByteView { length_field }
            }
            ByteViewLengthExpr::Sum { .. }
            | ByteViewLengthExpr::Difference { .. }
            | ByteViewLengthExpr::Product { .. }
            | ByteViewLengthExpr::Quotient { .. } => return None,
        }
    } else if schema_payload_name_path(primitive).is_some() {
        SchemaRepeatPayload::Schema {
            schema_name: (*primitive).to_string(),
        }
    } else {
        return None;
    };
    Some(SchemaRepeatSpec {
        count_field: count_expr.render(),
        payload,
    })
}

fn is_simple_schema_field_reference(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(crate) fn reserved_bits_schema_primitive(ty: &str) -> Option<(i64, i64)> {
    let rest = ty.strip_prefix("ReservedBits")?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let rest = rest.trim();
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }
    let inner = rest[1..rest.len() - 1].trim();
    let args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    let [width, value] = args.as_slice() else {
        return None;
    };
    let width = parse_reserved_bits_integer(width)?;
    let value = parse_reserved_bits_integer(value)?;
    Some((width, value))
}

pub(crate) fn supported_encode_reserved_bits(
    fields: &[veln_ast::SchemaField],
    index: usize,
    reserved: (i64, i64),
) -> Option<(u8, i64)> {
    let (bit_width, expected_value) = reserved;
    if supported_bit_packed_reserved_group(fields, index) {
        return Some((bit_width as u8, expected_value));
    }
    if supported_byte_interleaved_reserved_group(fields, index, bit_width, expected_value) {
        return Some((bit_width as u8, expected_value));
    }
    let previous_previous_field = index
        .checked_sub(2)
        .and_then(|previous| fields.get(previous));
    let previous_field = index
        .checked_sub(1)
        .and_then(|previous| fields.get(previous));
    let next_field = fields.get(index + 1);
    let next_next_field = fields.get(index + 2);
    if bit_width == 1
        && expected_value == 0
        && next_field.is_some_and(|field| field.ty.trim() == "UInt31be")
    {
        return Some((1, 0));
    }
    if supported_reserved_byte_prefix(bit_width, expected_value, next_field) {
        return Some((bit_width as u8, expected_value));
    }
    let packed_storage_bit_width = if (1..=7).contains(&bit_width) {
        Some(8)
    } else if (9..=15).contains(&bit_width) {
        Some(16)
    } else if (17..=23).contains(&bit_width) {
        Some(24)
    } else if (25..=31).contains(&bit_width) {
        Some(32)
    } else {
        None
    };
    if packed_storage_bit_width.is_some_and(|storage_bit_width| {
        next_field
            .and_then(|field| exact_width_schema_primitive_bit_width(&field.ty))
            .is_some_and(|next_bit_width| {
                i64::from(next_bit_width) + bit_width == storage_bit_width
            })
    }) {
        let max_value = (1_i64 << bit_width) - 1;
        if expected_value <= max_value {
            return Some((bit_width as u8, expected_value));
        }
    }
    if let (Some(next_field), Some(next_next_field)) = (next_field, next_next_field)
        && supported_prefix_reserved_group(next_field, next_next_field, bit_width, expected_value)
    {
        return Some((bit_width as u8, expected_value));
    }
    if let Some(packed_storage_bit_width) = suffix_packed_reserved_storage_bit_width(bit_width)
        && !previous_previous_field.is_some_and(|field| {
            previous_field.is_some_and(|visible| supported_packed_reserved_prefix(field, visible))
        })
        && previous_field
            .and_then(|field| exact_width_schema_primitive_bit_width(&field.ty))
            .is_some_and(|previous_bit_width| {
                i64::from(previous_bit_width) + bit_width == packed_storage_bit_width
            })
    {
        let max_value = (1_i64 << bit_width) - 1;
        if expected_value <= max_value {
            return Some((bit_width as u8, expected_value));
        }
    }
    if let (Some(previous_previous_field), Some(previous_field)) =
        (previous_previous_field, previous_field)
        && supported_suffix_reserved_group(
            previous_previous_field,
            previous_field,
            bit_width,
            expected_value,
        )
    {
        return Some((bit_width as u8, expected_value));
    }
    if let (Some(previous_field), Some(next_field)) = (previous_field, next_field)
        && supported_middle_reserved_bits(previous_field, next_field, bit_width, expected_value)
    {
        return Some((bit_width as u8, expected_value));
    }
    if bit_width <= 0 || bit_width > 32 || bit_width % 8 != 0 {
        return None;
    }
    let max_value = if bit_width == 32 {
        0xffffffff
    } else {
        (1_i64 << bit_width) - 1
    };
    if expected_value <= max_value {
        return Some((bit_width as u8, expected_value));
    }
    None
}

fn supported_bit_packed_reserved_group(fields: &[veln_ast::SchemaField], index: usize) -> bool {
    for start in 0..=index {
        let mut total_bit_width = 0_i64;
        let mut has_reserved = false;
        let mut has_visible = false;
        for (offset, field) in fields[start..].iter().enumerate() {
            let Some(bit_width) = bit_packed_group_field_width(field) else {
                break;
            };
            total_bit_width += bit_width;
            has_reserved |= reserved_bits_schema_primitive(&field.ty).is_some();
            has_visible |= reserved_bits_schema_primitive(&field.ty).is_none();
            if matches!(total_bit_width, 8 | 16 | 24 | 32 | 40 | 48 | 56 | 64) {
                let end = start + offset;
                if has_reserved && has_visible && start <= index && index <= end {
                    return true;
                }
                break;
            }
            if total_bit_width > 64 {
                break;
            }
        }
    }
    false
}

fn bit_packed_group_field_width(field: &veln_ast::SchemaField) -> Option<i64> {
    if let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(&field.ty) {
        if bit_width <= 0 || bit_width >= 64 || bit_width % 8 == 0 {
            return None;
        }
        let max_value = reserved_bits_max_value(bit_width)?;
        return (expected_value <= max_value).then_some(bit_width);
    }
    if exact_width_schema_primitive_little_endian(&field.ty)
        || flag_schema_primitive(&field.ty).is_some()
    {
        return None;
    }
    let bit_width = i64::from(exact_width_schema_primitive_bit_width(&field.ty)?);
    (bit_width % 8 != 0).then_some(bit_width)
}

fn reserved_bits_max_value(bit_width: i64) -> Option<i64> {
    if !(1..=63).contains(&bit_width) {
        return None;
    }
    if bit_width == 63 {
        return Some(i64::MAX);
    }
    Some((1_i64 << bit_width) - 1)
}

fn supported_prefix_reserved_group(
    first_visible_field: &veln_ast::SchemaField,
    second_visible_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 57 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&first_visible_field.ty)
        || exact_width_schema_primitive_little_endian(&second_visible_field.ty)
        || flag_schema_primitive(&first_visible_field.ty).is_some()
        || flag_schema_primitive(&second_visible_field.ty).is_some()
    {
        return false;
    }
    let Some(first_bit_width) = exact_width_schema_primitive_bit_width(&first_visible_field.ty)
    else {
        return false;
    };
    let Some(second_bit_width) = exact_width_schema_primitive_bit_width(&second_visible_field.ty)
    else {
        return false;
    };
    if first_bit_width > 8 || second_bit_width > 8 {
        return false;
    }
    let total_bit_width = bit_width + i64::from(first_bit_width) + i64::from(second_bit_width);
    let supported_one_byte_group = bit_width % 8 != 0
        && (bit_width + i64::from(first_bit_width)) % 8 != 0
        && total_bit_width == 8;
    let supported_two_byte_group = total_bit_width == 16;
    let supported_three_byte_group = (17..=23).contains(&bit_width) && total_bit_width == 24;
    let supported_four_byte_group = (25..=31).contains(&bit_width) && total_bit_width == 32;
    let supported_five_byte_group = bit_width == 33 && total_bit_width == 40;
    let supported_six_byte_group = bit_width == 41 && total_bit_width == 48;
    let supported_seven_byte_group = bit_width == 49 && total_bit_width == 56;
    let supported_eight_byte_group = bit_width == 57 && total_bit_width == 64;
    (supported_one_byte_group
        || supported_two_byte_group
        || supported_three_byte_group
        || supported_four_byte_group
        || supported_five_byte_group
        || supported_six_byte_group
        || supported_seven_byte_group
        || supported_eight_byte_group)
        && expected_value < (1_i64 << bit_width)
}

fn supported_suffix_reserved_group(
    first_visible_field: &veln_ast::SchemaField,
    second_visible_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 7 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&first_visible_field.ty)
        || exact_width_schema_primitive_little_endian(&second_visible_field.ty)
        || flag_schema_primitive(&first_visible_field.ty).is_some()
        || flag_schema_primitive(&second_visible_field.ty).is_some()
    {
        return false;
    }
    let Some(first_bit_width) = exact_width_schema_primitive_bit_width(&first_visible_field.ty)
    else {
        return false;
    };
    let Some(second_bit_width) = exact_width_schema_primitive_bit_width(&second_visible_field.ty)
    else {
        return false;
    };
    first_bit_width <= 8
        && second_bit_width == 8
        && i64::from(first_bit_width) + i64::from(second_bit_width) + bit_width == 16
        && expected_value < (1_i64 << bit_width)
}

fn supported_packed_reserved_prefix(
    reserved_field: &veln_ast::SchemaField,
    visible_field: &veln_ast::SchemaField,
) -> bool {
    let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(&reserved_field.ty)
    else {
        return false;
    };
    packed_reserved_storage_bit_width(bit_width).is_some_and(|storage_bit_width| {
        exact_width_schema_primitive_bit_width(&visible_field.ty).is_some_and(|visible_bit_width| {
            i64::from(visible_bit_width) + bit_width == storage_bit_width
        }) && expected_value < (1_i64 << bit_width)
    })
}

fn supported_reserved_byte_prefix(
    bit_width: i64,
    expected_value: i64,
    visible_field: Option<&veln_ast::SchemaField>,
) -> bool {
    matches!(bit_width, 2 | 9)
        && expected_value == 0
        && visible_field.is_some_and(|field| field.ty.trim() == "UInt8")
}

fn supported_middle_reserved_bits(
    previous_field: &veln_ast::SchemaField,
    next_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 32 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&previous_field.ty)
        || exact_width_schema_primitive_little_endian(&next_field.ty)
        || flag_schema_primitive(&previous_field.ty).is_some()
        || flag_schema_primitive(&next_field.ty).is_some()
    {
        return false;
    }
    let Some(previous_bit_width) = exact_width_schema_primitive_bit_width(&previous_field.ty)
    else {
        return false;
    };
    let Some(next_bit_width) = exact_width_schema_primitive_bit_width(&next_field.ty) else {
        return false;
    };
    let total_bit_width = i64::from(previous_bit_width) + bit_width + i64::from(next_bit_width);
    previous_bit_width % 8 != 0
        && (i64::from(previous_bit_width) + bit_width) % 8 != 0
        && matches!(total_bit_width, 8 | 16 | 24 | 32)
        && expected_value < (1_i64 << bit_width)
}

fn supported_byte_interleaved_reserved_group(
    fields: &[veln_ast::SchemaField],
    index: usize,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 7 {
        return false;
    }
    let Some(first_field) = index
        .checked_sub(1)
        .and_then(|previous| fields.get(previous))
    else {
        return false;
    };
    let (Some(byte_field), Some(last_field)) = (fields.get(index + 1), fields.get(index + 2))
    else {
        return false;
    };
    if [first_field, byte_field, last_field].iter().any(|field| {
        exact_width_schema_primitive_little_endian(&field.ty)
            || flag_schema_primitive(&field.ty).is_some()
    }) {
        return false;
    }
    let Some(first_bit_width) = exact_width_schema_primitive_bit_width(&first_field.ty) else {
        return false;
    };
    let Some(byte_bit_width) = exact_width_schema_primitive_bit_width(&byte_field.ty) else {
        return false;
    };
    let Some(last_bit_width) = exact_width_schema_primitive_bit_width(&last_field.ty) else {
        return false;
    };
    first_bit_width < 8
        && byte_bit_width == 8
        && last_bit_width < 8
        && i64::from(first_bit_width) + bit_width + 8 + i64::from(last_bit_width) == 16
        && (i64::from(first_bit_width) + bit_width) % 8 != 0
        && expected_value < (1_i64 << bit_width)
}

fn packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    if (1..=7).contains(&bit_width) {
        Some(8)
    } else if (9..=15).contains(&bit_width) {
        Some(16)
    } else if (17..=23).contains(&bit_width) {
        Some(24)
    } else if (25..=31).contains(&bit_width) {
        Some(32)
    } else {
        None
    }
}

fn suffix_packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    packed_reserved_storage_bit_width(bit_width).or_else(|| {
        if (33..=39).contains(&bit_width) {
            Some(40)
        } else if (41..=47).contains(&bit_width) {
            Some(48)
        } else {
            None
        }
    })
}

fn parse_reserved_bits_integer(text: &str) -> Option<i64> {
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDispatchSpec {
    pub(crate) tag_field: String,
    pub(crate) length_field: Option<String>,
    pub(crate) preserves_unknown: bool,
    pub(crate) cases: Vec<SchemaDispatchCase>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDispatchCase {
    pub(crate) tag: i64,
    pub(crate) payload: SchemaDispatchCasePayload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaDispatchCasePayload {
    Primitive { width: u8, little_endian: bool },
    Schema { schema_name: String },
}

pub(crate) fn closed_dispatch_schema_primitive(ty: &str) -> Option<SchemaDispatchSpec> {
    let inner = schema_call_inner(ty, "Dispatch")?;
    let mut args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .peekable();
    let tag_field = args.next()?.to_string();
    if !is_schema_identifier(&tag_field) {
        return None;
    }
    let length_field = args
        .peek()
        .filter(|arg| !arg.contains("=>"))
        .map(|arg| (*arg).to_string());
    if length_field
        .as_deref()
        .is_some_and(|length_field| !is_schema_identifier(length_field))
    {
        return None;
    }
    if length_field.is_some() {
        args.next();
    }
    let cases = args
        .map(|arg| {
            let (tag, primitive) = arg.split_once("=>")?;
            let tag = parse_schema_tag(tag.trim())?;
            let payload = schema_dispatch_case_payload(primitive.trim())?;
            Some(SchemaDispatchCase { tag, payload })
        })
        .collect::<Option<Vec<_>>>()?;
    if cases.is_empty() {
        return None;
    }
    Some(SchemaDispatchSpec {
        tag_field,
        length_field,
        preserves_unknown: false,
        cases,
    })
}

pub(crate) fn extension_dispatch_schema_primitive(ty: &str) -> Option<SchemaDispatchSpec> {
    let inner = schema_call_inner(ty, "ExtensionDispatch")?;
    let mut args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty());
    let tag_field = args.next()?.to_string();
    let length_field = args.next()?.to_string();
    if !is_schema_identifier(&tag_field) || !is_schema_identifier(&length_field) {
        return None;
    }
    let cases = args
        .map(|arg| {
            let (tag, primitive) = arg.split_once("=>")?;
            let tag = parse_schema_tag(tag.trim())?;
            let payload = schema_dispatch_case_payload(primitive.trim())?;
            Some(SchemaDispatchCase { tag, payload })
        })
        .collect::<Option<Vec<_>>>()?;
    if cases.is_empty() {
        return None;
    }
    Some(SchemaDispatchSpec {
        tag_field,
        length_field: Some(length_field),
        preserves_unknown: true,
        cases,
    })
}

fn schema_dispatch_case_payload(text: &str) -> Option<SchemaDispatchCasePayload> {
    if let Some(width) = exact_width_schema_primitive(text) {
        if exact_width_schema_primitive_bit_width(text)? < 8 {
            return None;
        }
        return Some(SchemaDispatchCasePayload::Primitive {
            width,
            little_endian: exact_width_schema_primitive_little_endian(text),
        });
    }
    schema_payload_name_is_path(text).then(|| SchemaDispatchCasePayload::Schema {
        schema_name: text.to_string(),
    })
}

fn schema_call_inner<'a>(ty: &'a str, name: &str) -> Option<&'a str> {
    let rest = ty.trim().strip_prefix(name)?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let rest = rest.trim();
    rest.strip_prefix('(')?.strip_suffix(')')
}

fn parse_schema_tag(text: &str) -> Option<i64> {
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

fn is_schema_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn snake_case_identifier(name: &str) -> String {
    let mut out = String::new();
    let mut previous_was_lower_or_digit = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if previous_was_lower_or_digit && !out.ends_with('_') {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
                previous_was_lower_or_digit = false;
            } else {
                out.push(ch);
                previous_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
            }
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
            previous_was_lower_or_digit = false;
        }
    }
    out.trim_matches('_').to_string()
}

impl FunctionSignature {
    pub(crate) fn ty(&self) -> Type {
        match &self.variadic {
            Some(variadic) => Type::variadic_function(
                self.params.clone(),
                variadic.clone(),
                self.return_type.clone(),
                self.effects.clone(),
            ),
            None => Type::function(
                self.params.clone(),
                self.return_type.clone(),
                self.effects.clone(),
            ),
        }
    }
}

fn function_alias_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<FunctionSignature> {
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            let target = function_signature_path(
                &alias.target,
                &module.uses,
                functions,
                alias.module_name.as_deref(),
            )?;
            Some(FunctionSignature {
                name,
                target_name: target.target_name.clone(),
                module_name: alias.module_name.clone(),
                visibility: Visibility::Public,
                params: target.params.clone(),
                variadic: target.variadic.clone(),
                return_type: target.return_type.clone(),
                effects: target.effects.clone(),
                node_id: alias.node_id,
                span: alias.span.clone(),
            })
        })
        .collect()
}

fn function_signature_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    functions: &'a [FunctionSignature],
    current_module: Option<&str>,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => functions.iter().find(|function| function.name == *name),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            let module_name = use_decl.name.as_str();
            functions.iter().find(|function| {
                function.name == *name
                    && function.module_name.as_deref() == Some(module_name)
                    && imported_function_is_visible(function, use_decl)
            })
        }
        _ => None,
    }
}

pub(crate) fn infer_function_body_effects(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
) {
    let mut effects_by_name = functions
        .iter()
        .map(|function| (function.name.clone(), function.effects.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut effects_by_module_path = functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone()?, function.name.clone()),
                (function.effects.clone(), function.visibility),
            ))
        })
        .collect::<BTreeMap<_, _>>();

    let mut changed = true;
    while changed {
        changed = false;
        for function in module
            .functions
            .iter()
            .filter(|function| function.kind == FunctionKind::Function)
        {
            let Some(name) = &function.name else {
                continue;
            };
            let mut bindings = function
                .params
                .iter()
                .map(|param| Binding {
                    name: param.name.clone(),
                    ty: function_body_param_type(param),
                })
                .collect::<Vec<_>>();
            let mut inferred = effects_by_name.get(name).cloned().unwrap_or_default();
            for line in &function.body {
                match &line.kind {
                    BodyLineKind::Let {
                        pattern,
                        annotation,
                        expr,
                    } => {
                        collect_expr_effects(
                            expr,
                            &module.uses,
                            function.module_name.as_deref(),
                            &bindings,
                            &effects_by_name,
                            &effects_by_module_path,
                            &mut inferred,
                        );
                        let ty = parse_type_or_unknown(annotation.as_deref());
                        collect_pattern_bindings(pattern, &ty, &mut bindings);
                    }
                    BodyLineKind::Expr { expr } => {
                        collect_expr_effects(
                            expr,
                            &module.uses,
                            function.module_name.as_deref(),
                            &bindings,
                            &effects_by_name,
                            &effects_by_module_path,
                            &mut inferred,
                        );
                    }
                }
            }
            if effects_by_name.get(name) != Some(&inferred) {
                effects_by_name.insert(name.clone(), inferred);
                if let Some(module_name) = &function.module_name {
                    effects_by_module_path.insert(
                        (module_name.clone(), name.clone()),
                        (effects_by_name[name].clone(), function.visibility),
                    );
                }
                changed = true;
            }
        }
    }

    for function in functions {
        if let Some(effects) = effects_by_name.remove(&function.name) {
            function.effects = effects;
        }
    }
}

fn collect_pattern_bindings(pattern: &Pattern, ty: &Type, bindings: &mut Vec<Binding>) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(Binding {
            name: name.clone(),
            ty: ty.clone(),
        }),
        PatternKind::Record(fields) => {
            for field in fields {
                let field_ty = ty.record_field(&field.name).unwrap_or(&Type::Unknown);
                collect_pattern_bindings(&field.pattern, field_ty, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit
        | PatternKind::Constructor { .. } => {}
    }
}

fn collect_expr_effects(
    expr: &Expr,
    uses: &[UseDecl],
    current_module: Option<&str>,
    bindings: &[Binding],
    effects_by_name: &BTreeMap<String, Vec<String>>,
    effects_by_module_path: &BTreeMap<(String, String), (Vec<String>, Visibility)>,
    inferred: &mut Vec<String>,
) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let Some(segments) = callee_name_path(callee) {
                if is_stdio_call(segments) {
                    push_unique_effect(inferred, "stdio");
                } else if let Some(effects) = concurrency_effects(segments) {
                    for effect in effects {
                        push_unique_effect(inferred, effect);
                    }
                } else if let Some(effects) = standard_library_effects(segments) {
                    for effect in effects {
                        push_unique_effect(inferred, effect);
                    }
                } else {
                    for effect in effects_for_callee_path(
                        segments,
                        uses,
                        current_module,
                        bindings,
                        effects_by_name,
                        effects_by_module_path,
                    ) {
                        push_unique_effect(inferred, effect);
                    }
                }
            } else {
                collect_expr_effects(
                    callee,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
            for arg in args {
                collect_expr_effects(
                    arg,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::TypeApply { callee: base, .. }
        | ExprKind::Prefix { expr: base, .. } => {
            collect_expr_effects(
                base,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_expr_effects(
                    &field.expr,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_expr_effects(
                    &entry.key,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
                collect_expr_effects(
                    &entry.value,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_expr_effects(
                    item,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_effects(
                scrutinee,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
            for arm in arms {
                collect_expr_effects(
                    &arm.expr,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_expr_effects(
                condition,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
            collect_expr_effects(
                then_branch,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
            for branch in else_if_branches {
                collect_expr_effects(
                    &branch.condition,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
                collect_expr_effects(
                    &branch.expr,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
            collect_expr_effects(
                else_branch,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_effects(
                left,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
            collect_expr_effects(
                right,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn callee_name_path(callee: &Expr) -> Option<&Vec<String>> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments),
        ExprKind::TypeApply { callee, .. } => callee_name_path(callee),
        _ => None,
    }
}

fn effects_for_callee_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
    bindings: &'a [Binding],
    effects_by_name: &'a BTreeMap<String, Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
) -> &'a [String] {
    match segments {
        [name] => effects_for_bare_callee(name, bindings, effects_by_name),
        [_, .., name] => {
            let Some(use_decl) =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
            else {
                return &[];
            };
            effects_by_module_path
                .get(&(use_decl.name.clone(), name.clone()))
                .filter(|(_, visibility)| {
                    use_decl.package.is_none() || *visibility == Visibility::Public
                })
                .map_or(&[], |(effects, _)| effects.as_slice())
        }
        _ => &[],
    }
}

pub(crate) fn imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

fn imported_function_is_visible(function: &FunctionSignature, use_decl: &UseDecl) -> bool {
    use_decl.package.is_none() || function.visibility == Visibility::Public
}

pub(crate) fn imported_module_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a str> {
    imported_use_for_path(uses, segments, current_module).map(|use_decl| use_decl.name.as_str())
}

fn effects_for_bare_callee<'a>(
    name: &str,
    bindings: &'a [Binding],
    effects_by_name: &'a BTreeMap<String, Vec<String>>,
) -> &'a [String] {
    if let Some(binding) = bindings.iter().rev().find(|binding| binding.name == name)
        && let Some(effects) = binding.ty.function_effects()
    {
        return effects;
    }
    effects_by_name.get(name).map_or(&[], Vec::as_slice)
}

fn push_unique_effect(effects: &mut Vec<String>, effect: &str) {
    if !effects.iter().any(|existing| existing == effect) {
        effects.push(effect.to_string());
    }
}

pub(crate) fn is_assignable(expected: &Type, actual: &Type) -> bool {
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
            Type::Named {
                name: expected_name,
                args: expected_args,
            },
            Type::Named {
                name: actual_name,
                args: actual_args,
            },
        ) => {
            expected_name == actual_name
                && expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| is_assignable(expected, actual))
        }
        (
            Type::Function {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                effects: expected_effects,
            },
            Type::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: actual_effects,
            },
        ) => {
            expected_params.len() == actual_params.len()
                && expected_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| is_assignable(expected, actual))
                && match (expected_variadic, actual_variadic) {
                    (Some(expected), Some(actual)) => is_assignable(expected, actual),
                    (None, None) => true,
                    _ => false,
                }
                && is_assignable(expected_return, actual_return)
                && effects_are_assignable(expected_effects, actual_effects)
        }
        _ => false,
    }
}

fn effects_are_assignable(expected: &[String], actual: &[String]) -> bool {
    actual
        .iter()
        .all(|effect| expected.iter().any(|expected| expected == effect))
}

pub(crate) fn parse_type_or_unknown(text: Option<&str>) -> Type {
    text.and_then(|text| parse_type_annotation(text).ok())
        .unwrap_or(Type::Unknown)
}

pub(crate) fn core_type(ty: &Type) -> CoreType {
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
            variadic,
            return_type,
            effects,
        } => CoreType::Function {
            params: params.iter().map(core_type).collect(),
            variadic: variadic.as_deref().map(core_type).map(Box::new),
            return_type: Box::new(core_type(return_type)),
            effects: effects.clone(),
        },
    }
}

pub(crate) fn parse_type_annotation(text: &str) -> Result<Type, String> {
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
        if self.eat('(') {
            self.skip_ws();
            if self.eat(')') {
                return Ok(Type::unit());
            }
            return Err("expected `)` for unit type `()`".to_string());
        }
        if self.eat_keyword("fn") {
            return self.parse_function_type();
        }

        let Some(name) = self.parse_ident() else {
            return Err("expected type".to_string());
        };
        self.skip_ws();
        let args = if self.eat('<') {
            let args = self.parse_type_list('>')?;
            self.expect('>')?;
            args
        } else if self.at('(') {
            return Err(format!("unexpected `{}`", &self.text[self.cursor..]));
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
            if fields
                .iter()
                .any(|(field_name, _): &(String, Type)| field_name == &name)
            {
                return Err(format!("duplicate record field `{name}`"));
            }
            self.expect(':')?;
            let ty = self.parse_type()?;
            fields.push((name, ty));
            self.skip_ws();
            if !self.eat(',') {
                break;
            }
            self.skip_ws();
            if self.at('}') {
                break;
            }
        }
        self.expect('}')?;
        Ok(Type::Record(fields))
    }

    fn parse_function_type(&mut self) -> Result<Type, String> {
        self.expect('(')?;
        let (params, variadic) = self.parse_function_param_type_list()?;
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
            variadic: variadic.map(Box::new),
            return_type: Box::new(return_type),
            effects,
        })
    }

    fn parse_function_param_type_list(&mut self) -> Result<(Vec<Type>, Option<Type>), String> {
        let mut params = Vec::new();
        let mut variadic = None;
        self.skip_ws();
        while !self.at_end() && !self.at(')') {
            let is_variadic = self.eat_str("...");
            let ty = if is_variadic {
                self.parse_type()
                    .map_err(|_| "expected type after variadic marker".to_string())?
            } else {
                self.parse_type()?
            };
            self.skip_ws();
            let has_more = self.eat(',');
            if is_variadic {
                if variadic.is_some() {
                    return Err("function type has more than one variadic parameter".to_string());
                }
                if has_more {
                    return Err(
                        "variadic function type parameter must be the final parameter".to_string(),
                    );
                }
                variadic = Some(ty);
            } else {
                params.push(ty);
            }
            self.skip_ws();
            if !has_more {
                break;
            }
            if self.at(')') {
                break;
            }
        }
        Ok((params, variadic))
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
            "Option" | "Vec" => Some(1),
            "Result" | "Dict" => Some(2),
            _ => None,
        };
        if let Some(expected) = expected_arity
            && args.len() != expected
        {
            return Err(format!(
                "`{name}` expects {expected} type argument(s), found {}",
                args.len()
            ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_tuple_spelling_as_unit_type() {
        assert_eq!(parse_type_annotation("()"), Ok(Type::unit()));
        assert_eq!(
            parse_type_annotation("Result<(), AppError>"),
            Ok(Type::result(
                Type::unit(),
                Type::named("AppError", Vec::new())
            ))
        );
    }

    #[test]
    fn renders_unit_type_with_empty_tuple_spelling() {
        assert_eq!(Type::unit().render(), "()");
        assert_eq!(
            Type::result(Type::unit(), Type::named("AppError", Vec::new())).render(),
            "Result<(), AppError>"
        );
    }

    #[test]
    fn keeps_unit_name_as_compatibility_alias() {
        assert_eq!(parse_type_annotation("Unit"), Ok(Type::unit()));
    }

    #[test]
    fn renders_record_and_function_types() {
        let record = Type::Record(vec![
            ("name".to_string(), Type::string()),
            ("scores".to_string(), Type::vec(Type::int())),
        ]);
        let pure_function = Type::Function {
            params: vec![Type::int(), Type::float()],
            variadic: None,
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };
        let effectful_function = Type::Function {
            params: vec![record.clone()],
            variadic: None,
            return_type: Box::new(Type::result(
                Type::unit(),
                Type::named("AppError", Vec::new()),
            )),
            effects: vec!["stdio".to_string(), "net".to_string()],
        };

        assert_eq!(record.render(), "{name: String, scores: Vec<Int>}");
        assert_eq!(pure_function.render(), "fn(Int, Float) -> Bool");
        assert_eq!(
            effectful_function.render(),
            "fn({name: String, scores: Vec<Int>}) -> Result<(), AppError> effects [stdio, net]"
        );
    }

    #[test]
    fn exposes_type_parts_and_core_type_shape() {
        let function = Type::Function {
            params: vec![Type::vec(Type::int())],
            variadic: None,
            return_type: Box::new(Type::Record(vec![("ok".to_string(), Type::bool())])),
            effects: vec!["stdio".to_string()],
        };

        let (params, return_type) = function
            .function_parts()
            .expect("function type should expose parts");
        assert_eq!(params, &[Type::vec(Type::int())]);
        assert_eq!(
            return_type,
            &Type::Record(vec![("ok".to_string(), Type::bool())])
        );
        assert!(Type::string().function_parts().is_none());
        assert_eq!(
            core_type(&function),
            CoreType::Function {
                params: vec![CoreType::vec(CoreType::int())],
                variadic: None,
                return_type: Box::new(CoreType::Record(vec![("ok".to_string(), CoreType::bool())])),
                effects: vec!["stdio".to_string()],
            }
        );
    }

    #[test]
    fn assignability_allows_unknowns_record_width_and_function_shapes() {
        let expected_record = Type::Record(vec![
            ("name".to_string(), Type::string()),
            (
                "meta".to_string(),
                Type::Record(vec![("count".to_string(), Type::int())]),
            ),
        ]);
        let actual_record = Type::Record(vec![
            ("name".to_string(), Type::string()),
            ("extra".to_string(), Type::bool()),
            (
                "meta".to_string(),
                Type::Record(vec![("count".to_string(), Type::int())]),
            ),
        ]);
        let wrong_record = Type::Record(vec![("name".to_string(), Type::int())]);
        let expected_pure_function = Type::Function {
            params: vec![Type::int()],
            variadic: None,
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };
        let actual_effectful_function = Type::Function {
            params: vec![Type::int()],
            variadic: None,
            return_type: Box::new(Type::bool()),
            effects: vec!["stdio".to_string()],
        };
        let expected_effectful_function = Type::Function {
            params: vec![Type::int()],
            variadic: None,
            return_type: Box::new(Type::bool()),
            effects: vec!["stdio".to_string()],
        };
        let actual_pure_function = Type::Function {
            params: vec![Type::int()],
            variadic: None,
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };
        let wrong_function = Type::Function {
            params: vec![Type::int(), Type::int()],
            variadic: None,
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };

        assert!(is_assignable(&Type::Unknown, &Type::string()));
        assert!(is_assignable(&Type::string(), &Type::Unknown));
        assert!(is_assignable(&expected_record, &actual_record));
        assert!(!is_assignable(&expected_record, &wrong_record));
        assert!(!is_assignable(
            &Type::named("Path", Vec::new()),
            &Type::string()
        ));
        assert!(!is_assignable(
            &Type::string(),
            &Type::named("Path", Vec::new())
        ));
        assert!(is_assignable(
            &expected_effectful_function,
            &actual_pure_function
        ));
        assert!(!is_assignable(
            &expected_pure_function,
            &actual_effectful_function
        ));
        assert!(!is_assignable(&expected_pure_function, &wrong_function));
        assert!(!is_assignable(&Type::int(), &Type::float()));
    }

    #[test]
    fn parses_nested_type_annotations_with_whitespace() {
        assert_eq!(
            parse_type_annotation(
                " fn ( Vec< Int > , platform::Request ) -> Result < Dict < String , Int > , AppError > effects [ stdio , net ] "
            ),
            Ok(Type::Function {
                params: vec![
                    Type::vec(Type::int()),
                    Type::named("platform::Request", Vec::new()),
                ],
                variadic: None,
                return_type: Box::new(Type::result(
                    Type::dict(Type::string(), Type::int()),
                    Type::named("AppError", Vec::new())
                )),
                effects: vec!["stdio".to_string(), "net".to_string()],
            })
        );
        assert_eq!(
            parse_type_annotation("{ name: String, scores: Vec<Int> }"),
            Ok(Type::Record(vec![
                ("name".to_string(), Type::string()),
                ("scores".to_string(), Type::vec(Type::int())),
            ]))
        );
        assert_eq!(
            parse_type_annotation("{ name: String, scores: Vec<Int>, }"),
            Ok(Type::Record(vec![
                ("name".to_string(), Type::string()),
                ("scores".to_string(), Type::vec(Type::int())),
            ]))
        );
    }

    #[test]
    fn parses_angle_bracket_type_annotations() {
        assert_eq!(
            parse_type_annotation(
                "fn(Vec<Int>, domain::Envelope<String, Result<(), AppError>>) -> Dict<String, Int>"
            ),
            Ok(Type::Function {
                params: vec![
                    Type::vec(Type::int()),
                    Type::named(
                        "domain::Envelope",
                        vec![
                            Type::string(),
                            Type::result(Type::unit(), Type::named("AppError", Vec::new())),
                        ],
                    ),
                ],
                variadic: None,
                return_type: Box::new(Type::dict(Type::string(), Type::int())),
                effects: Vec::new(),
            })
        );
    }

    #[test]
    fn parses_variadic_function_type_annotations() {
        assert_eq!(
            parse_type_annotation("fn(String, ...String) -> ()"),
            Ok(Type::Function {
                params: vec![Type::string()],
                variadic: Some(Box::new(Type::string())),
                return_type: Box::new(Type::unit()),
                effects: Vec::new(),
            })
        );
        assert_eq!(
            parse_type_annotation("...String"),
            Err("expected type".to_string())
        );
        assert_eq!(
            parse_type_annotation("fn(...String, String) -> ()"),
            Err("variadic function type parameter must be the final parameter".to_string())
        );
    }

    #[test]
    fn variadic_and_fixed_function_types_are_not_assignable() {
        let variadic = Type::Function {
            params: vec![Type::string()],
            variadic: Some(Box::new(Type::string())),
            return_type: Box::new(Type::unit()),
            effects: Vec::new(),
        };
        let fixed = Type::Function {
            params: vec![Type::string(), Type::string()],
            variadic: None,
            return_type: Box::new(Type::unit()),
            effects: Vec::new(),
        };

        assert!(!is_assignable(&variadic, &fixed));
        assert!(!is_assignable(&fixed, &variadic));
    }

    #[test]
    fn rejects_malformed_type_annotations_with_specific_errors() {
        let cases = [
            ("", "expected type"),
            ("(Int)", "expected `)` for unit type `()`"),
            ("Int trailing", "unexpected `trailing`"),
            ("{ : Int }", "expected record field name"),
            ("{ name: String,, }", "expected record field name"),
            (
                "{ value: Int, value: String }",
                "duplicate record field `value`",
            ),
            ("{ value Int }", "expected `:`"),
            ("fn(Int) Int", "expected `->` in function type"),
            ("fn(Int -> Int", "expected `)`"),
            ("fn() -> () effects [,]", "expected effect name"),
            ("fn() -> () effects [stdio", "expected `]`"),
            ("Result(Int, String)", "unexpected `(Int, String)`"),
            ("Vec", "`Vec` expects 1 type argument(s), found 0"),
            ("Dict<String>", "`Dict` expects 2 type argument(s), found 1"),
            ("std::", "expected type"),
        ];

        for (text, message) in cases {
            assert_eq!(parse_type_annotation(text), Err(message.to_string()));
        }
        assert_eq!(parse_type_or_unknown(Some("Vec")), Type::Unknown);
        assert_eq!(parse_type_or_unknown(None), Type::Unknown);
    }

    #[test]
    fn expected_type_sources_render_for_diagnostics_and_holes() {
        let cases = [
            (
                ExpectedTypeSource::DeclaredReturn,
                "declared_return",
                "declared",
            ),
            (
                ExpectedTypeSource::DeclaredParameter,
                "declared_parameter",
                "declared",
            ),
            (
                ExpectedTypeSource::LocalAnnotation,
                "local_annotation",
                "declared",
            ),
            (
                ExpectedTypeSource::Inferred,
                "inferred_expression",
                "inferred",
            ),
            (ExpectedTypeSource::Unknown, "unknown", "unknown"),
        ];

        for (source, type_source, hole_source) in cases {
            assert_eq!(source.as_type_source(), type_source);
            assert_eq!(source.as_hole_source(), hole_source);
        }
    }
}
