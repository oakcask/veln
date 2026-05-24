use std::collections::BTreeMap;

use veln_ast::{
    BodyLineKind, Expr, ExprKind, FunctionKind, NodeId, Pattern, PatternKind, SurfaceModule,
    UseDecl,
};
use veln_core::CoreType;
use veln_source::SourceSpan;

pub(crate) struct TypeEnvironment {
    functions: Vec<FunctionSignature>,
    uses: Vec<UseDecl>,
}

#[derive(Clone)]
pub(crate) struct FunctionSignature {
    pub(crate) name: String,
    pub(crate) module_name: Option<String>,
    pub(crate) params: Vec<Type>,
    pub(crate) return_type: Type,
    pub(crate) effects: Vec<String>,
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
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

    pub(crate) fn result(value: Type, error: Type) -> Self {
        Self::named("Result", vec![value, error])
    }

    pub(crate) fn list(item: Type) -> Self {
        Self::named("List", vec![item])
    }

    pub(crate) fn dict(key: Type, value: Type) -> Self {
        Self::named("Dict", vec![key, value])
    }

    pub(crate) fn function(params: Vec<Type>, return_type: Type, effects: Vec<String>) -> Self {
        Self::Function {
            params,
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

    pub(crate) fn result_parts(&self) -> Option<(&Type, &Type)> {
        match self {
            Self::Named { name, args } if name == "Result" && args.len() == 2 => {
                Some((&args[0], &args[1]))
            }
            _ => None,
        }
    }

    pub(crate) fn option_part(&self) -> Option<&Type> {
        match self {
            Self::Named { name, args } if name == "Option" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    pub(crate) fn list_part(&self) -> Option<&Type> {
        match self {
            Self::Named { name, args } if name == "List" && args.len() == 1 => Some(&args[0]),
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
                return_type,
                ..
            } => Some((params, return_type)),
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
        let mut functions = module
            .functions
            .iter()
            .filter(|function| function.kind == FunctionKind::Function)
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
                    module_name: function.module_name.clone(),
                    params,
                    return_type,
                    effects: function.effects.clone().unwrap_or_default(),
                    node_id: function.node_id,
                    span: function.span.clone(),
                })
            })
            .collect::<Vec<_>>();
        infer_function_body_effects(module, &mut functions);
        Self {
            functions,
            uses: module.uses.clone(),
        }
    }

    pub(crate) fn function(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.iter().find(|function| function.name == name)
    }

    pub(crate) fn function_path(&self, segments: &[String]) -> Option<&FunctionSignature> {
        match segments {
            [name] => self.function(name),
            [alias, name] => {
                let module_name = self
                    .uses
                    .iter()
                    .find(|use_decl| use_decl.alias == *alias)
                    .map(|use_decl| use_decl.name.as_str())?;
                self.functions.iter().find(|function| {
                    function.name == *name && function.module_name.as_deref() == Some(module_name)
                })
            }
            _ => None,
        }
    }
}

impl FunctionSignature {
    pub(crate) fn ty(&self) -> Type {
        Type::function(
            self.params.clone(),
            self.return_type.clone(),
            self.effects.clone(),
        )
    }
}

fn infer_function_body_effects(module: &SurfaceModule, functions: &mut [FunctionSignature]) {
    let mut effects_by_name = functions
        .iter()
        .map(|function| (function.name.clone(), function.effects.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut effects_by_module_path = functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone()?, function.name.clone()),
                function.effects.clone(),
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
                    ty: parse_type_or_unknown(param.ty.as_deref()),
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
                        effects_by_name[name].clone(),
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
    bindings: &[Binding],
    effects_by_name: &BTreeMap<String, Vec<String>>,
    effects_by_module_path: &BTreeMap<(String, String), Vec<String>>,
    inferred: &mut Vec<String>,
) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let ExprKind::NamePath(segments) = &callee.kind {
                if segments.as_slice().is_stdio_call() {
                    push_unique_effect(inferred, "stdio");
                } else {
                    for effect in effects_for_callee_path(
                        segments,
                        uses,
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
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_expr_effects(
                base,
                uses,
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
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
                collect_expr_effects(
                    &entry.value,
                    uses,
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
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
            for arm in arms {
                collect_expr_effects(
                    &arm.expr,
                    uses,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_effects(
                left,
                uses,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
            collect_expr_effects(
                right,
                uses,
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

fn effects_for_callee_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    bindings: &'a [Binding],
    effects_by_name: &'a BTreeMap<String, Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), Vec<String>>,
) -> &'a [String] {
    match segments {
        [name] => effects_for_bare_callee(name, bindings, effects_by_name),
        [alias, name] => {
            let Some(module_name) = uses
                .iter()
                .find(|use_decl| use_decl.alias == *alias)
                .map(|use_decl| use_decl.name.as_str())
            else {
                return &[];
            };
            effects_by_module_path
                .get(&(module_name.to_string(), name.clone()))
                .map_or(&[], Vec::as_slice)
        }
        _ => &[],
    }
}

fn effects_for_bare_callee<'a>(
    name: &str,
    bindings: &'a [Binding],
    effects_by_name: &'a BTreeMap<String, Vec<String>>,
) -> &'a [String] {
    if let Some(binding) = bindings.iter().rev().find(|binding| binding.name == name) {
        if let Some(effects) = binding.ty.function_effects() {
            return effects;
        }
    }
    effects_by_name.get(name).map_or(&[], Vec::as_slice)
}

fn push_unique_effect(effects: &mut Vec<String>, effect: &str) {
    if !effects.iter().any(|existing| existing == effect) {
        effects.push(effect.to_string());
    }
}

trait StdioSegments {
    fn is_stdio_call(&self) -> bool;
}

impl StdioSegments for [String] {
    fn is_stdio_call(&self) -> bool {
        matches!(
            self,
            [module, name]
                if module == "stdio"
                    && matches!(name.as_str(), "print" | "println" | "eprint" | "eprintln")
        )
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
            return_type,
            effects,
        } => CoreType::Function {
            params: params.iter().map(core_type).collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_tuple_spelling_as_unit_type() {
        assert_eq!(parse_type_annotation("()"), Ok(Type::unit()));
        assert_eq!(
            parse_type_annotation("Result((), AppError)"),
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
            "Result((), AppError)"
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
            ("scores".to_string(), Type::list(Type::int())),
        ]);
        let pure_function = Type::Function {
            params: vec![Type::int(), Type::float()],
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };
        let effectful_function = Type::Function {
            params: vec![record.clone()],
            return_type: Box::new(Type::result(
                Type::unit(),
                Type::named("AppError", Vec::new()),
            )),
            effects: vec!["stdio".to_string(), "net".to_string()],
        };

        assert_eq!(record.render(), "{name: String, scores: List(Int)}");
        assert_eq!(pure_function.render(), "fn(Int, Float) -> Bool");
        assert_eq!(
            effectful_function.render(),
            "fn({name: String, scores: List(Int)}) -> Result((), AppError) effects [stdio, net]"
        );
    }

    #[test]
    fn exposes_type_parts_and_core_type_shape() {
        let function = Type::Function {
            params: vec![Type::list(Type::int())],
            return_type: Box::new(Type::Record(vec![("ok".to_string(), Type::bool())])),
            effects: vec!["stdio".to_string()],
        };

        let (params, return_type) = function
            .function_parts()
            .expect("function type should expose parts");
        assert_eq!(params, &[Type::list(Type::int())]);
        assert_eq!(
            return_type,
            &Type::Record(vec![("ok".to_string(), Type::bool())])
        );
        assert!(Type::string().function_parts().is_none());
        assert_eq!(
            core_type(&function),
            CoreType::Function {
                params: vec![CoreType::list(CoreType::int())],
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
        let expected_function = Type::Function {
            params: vec![Type::int()],
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };
        let actual_function = Type::Function {
            params: vec![Type::int()],
            return_type: Box::new(Type::bool()),
            effects: vec!["stdio".to_string()],
        };
        let wrong_function = Type::Function {
            params: vec![Type::int(), Type::int()],
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };

        assert!(is_assignable(&Type::Unknown, &Type::string()));
        assert!(is_assignable(&Type::string(), &Type::Unknown));
        assert!(is_assignable(&expected_record, &actual_record));
        assert!(!is_assignable(&expected_record, &wrong_record));
        assert!(is_assignable(&expected_function, &actual_function));
        assert!(!is_assignable(&expected_function, &wrong_function));
        assert!(!is_assignable(&Type::int(), &Type::float()));
    }

    #[test]
    fn parses_nested_type_annotations_with_whitespace() {
        assert_eq!(
            parse_type_annotation(
                " fn ( List ( Int ) , platform::Request ) -> Result ( Dict ( String , Int ) , AppError ) effects [ stdio , net ] "
            ),
            Ok(Type::Function {
                params: vec![
                    Type::list(Type::int()),
                    Type::named("platform::Request", Vec::new()),
                ],
                return_type: Box::new(Type::result(
                    Type::dict(Type::string(), Type::int()),
                    Type::named("AppError", Vec::new())
                )),
                effects: vec!["stdio".to_string(), "net".to_string()],
            })
        );
        assert_eq!(
            parse_type_annotation("{ name: String, scores: List(Int) }"),
            Ok(Type::Record(vec![
                ("name".to_string(), Type::string()),
                ("scores".to_string(), Type::list(Type::int())),
            ]))
        );
        assert_eq!(
            parse_type_annotation("{ name: String, scores: List(Int), }"),
            Ok(Type::Record(vec![
                ("name".to_string(), Type::string()),
                ("scores".to_string(), Type::list(Type::int())),
            ]))
        );
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
            ("List", "`List` expects 1 type argument(s), found 0"),
            ("Dict(String)", "`Dict` expects 2 type argument(s), found 1"),
            ("std::", "expected type"),
        ];

        for (text, message) in cases {
            assert_eq!(parse_type_annotation(text), Err(message.to_string()));
        }
        assert_eq!(parse_type_or_unknown(Some("List")), Type::Unknown);
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
