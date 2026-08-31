use super::*;

pub(crate) fn private_tail_can_use_expected(
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

pub(crate) fn tail_expr_can_use_expected(
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
        ExprKind::NamePath { segments, .. } => {
            matches!(
                adts.nullary_constructor(segments, current_module, uses),
                ConstructorLookup::Found(constructor)
                    if unification::adt_args(expected, constructor.descriptor).is_some()
            )
        }
        ExprKind::Call { callee, .. } => {
            let ExprKind::NamePath { segments, .. } = &callee.kind else {
                return false;
            };
            matches!(
                adts.constructor(segments, current_module, uses),
                ConstructorLookup::Found(constructor)
                    if unification::adt_args(expected, constructor.descriptor).is_some()
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

pub(crate) fn infer_private_function_tail_type(
    function: &veln_ast::Function,
    uses: &[UseDecl],
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    #[cfg(test)]
    private_inference_counters::record_body_return_scan();

    let mut bindings = private_function_body_bindings(function, signatures_by_path);
    let mut tail = Type::unit();
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
                ..
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

pub(crate) fn private_function_body_bindings(
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
        .filter(|(_, param)| valid_value_binding_name(&param.name))
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
            Binding::new(param.name.clone(), ty)
        })
        .collect()
}

pub(crate) fn infer_private_signature_expr_type(
    expr: &Expr,
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    let context = PrivateSignatureInferContext {
        current_module,
        uses,
        bindings,
        returns_by_path,
        adts,
    };
    match &expr.kind {
        ExprKind::Missing | ExprKind::Hole { .. } | ExprKind::TypeApply { .. } => Type::Unknown,
        ExprKind::StringLiteral(_) => Type::string(),
        ExprKind::IntLiteral(_) => Type::int(),
        ExprKind::FloatLiteral(_) => Type::float(),
        ExprKind::BoolLiteral(_) => Type::bool(),
        ExprKind::Unit => Type::unit(),
        ExprKind::NamePath { segments, .. } => infer_private_signature_name_type(
            segments,
            expected,
            current_module,
            uses,
            bindings,
            returns_by_path,
            adts,
        ),
        ExprKind::List(items) => infer_private_list_type(items, expected, &context),
        ExprKind::Dict(entries) => infer_private_dict_type(entries, expected, &context),
        ExprKind::Record(fields) => infer_private_record_type(fields, expected, &context),
        ExprKind::Call { callee, args } => {
            infer_private_signature_call_type(callee, args, expected, &context)
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            Type::Unknown
        }
        ExprKind::Handle { body, args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            context.infer(body, expected)
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            context.infer(input, Some(&Type::named("ByteView", Vec::new())));
            context.infer(base, Some(&Type::named("ByteOffset", Vec::new())));
            Type::Unknown
        }
        ExprKind::SchemaEncode { value, .. } => {
            context.infer(value, None);
            Type::Unknown
        }
        ExprKind::FieldAccess { base, field, .. } => context
            .infer(base, None)
            .record_field(field)
            .cloned()
            .unwrap_or(Type::Unknown),
        ExprKind::Try(inner) => expected.cloned().unwrap_or_else(|| {
            let inner_type = context.infer(inner, None);
            adt::result_parts(&inner_type).map_or(Type::Unknown, |(value, _)| value.clone())
        }),
        ExprKind::Match { scrutinee, arms } => {
            infer_private_match_type(scrutinee, arms, expected, &context)
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
            expected,
            &context,
        ),
        ExprKind::Prefix { expr, .. } => {
            context.infer(expr, expected);
            Type::Unknown
        }
        ExprKind::Binary { op, left, right } => {
            infer_private_binary_type(*op, left, right, expected, &context)
        }
    }
}

pub(crate) fn infer_private_list_type(
    items: &[Expr],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let mut item_type = expected
        .and_then(Type::vec_part)
        .cloned()
        .unwrap_or(Type::Unknown);
    for item in items {
        let actual = context.infer(item, item_type_unknown_as_none(&item_type));
        if item_type == Type::Unknown {
            item_type = actual;
        }
    }
    Type::vec(item_type)
}

pub(crate) fn infer_private_dict_type(
    entries: &[DictEntry],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let (mut key_type, mut value_type) = expected
        .and_then(Type::dict_parts)
        .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });
    for entry in entries {
        let key_actual = context.infer(&entry.key, item_type_unknown_as_none(&key_type));
        if key_type == Type::Unknown {
            key_type = key_actual;
        }
        let value_actual = context.infer(&entry.value, item_type_unknown_as_none(&value_type));
        if value_type == Type::Unknown {
            value_type = value_actual;
        }
    }
    Type::dict(key_type, value_type)
}

pub(crate) fn infer_private_record_type(
    fields: &[RecordField],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
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
                    context.infer(&field.expr, field_expected),
                )
            })
            .collect(),
    )
}

pub(crate) fn infer_private_match_type(
    scrutinee: &Expr,
    arms: &[MatchArm],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
        arms,
        context.current_module,
        context.uses,
        context.adts,
    ) {
        MatchScrutineePatternInference::Inferred(ty) => Some(ty),
        MatchScrutineePatternInference::Uninferred
        | MatchScrutineePatternInference::Ambiguous(_) => None,
    };
    context.infer(scrutinee, scrutinee_expected.as_ref());
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    for arm in arms {
        let actual = context.infer(&arm.expr, item_type_unknown_as_none(&result));
        if result == Type::Unknown {
            result = actual;
        }
    }
    result
}

pub(crate) fn infer_private_if_result_type(
    then_branch: &Expr,
    else_if_branches: &[IfBranch],
    else_branch: &Expr,
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    for branch_expr in std::iter::once(then_branch)
        .chain(else_if_branches.iter().map(|branch| &branch.expr))
        .chain(std::iter::once(else_branch))
    {
        let actual = context.infer(branch_expr, item_type_unknown_as_none(&result));
        if result == Type::Unknown {
            result = actual;
        }
    }
    result
}

pub(crate) fn infer_private_binary_type(
    op: veln_ast::BinaryOp,
    left: &Expr,
    right: &Expr,
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    match op {
        veln_ast::BinaryOp::Equal
        | veln_ast::BinaryOp::NotEqual
        | veln_ast::BinaryOp::Less
        | veln_ast::BinaryOp::LessEqual
        | veln_ast::BinaryOp::Greater
        | veln_ast::BinaryOp::GreaterEqual
        | veln_ast::BinaryOp::Or
        | veln_ast::BinaryOp::And => Type::bool(),
        veln_ast::BinaryOp::BitwiseOr
        | veln_ast::BinaryOp::BitwiseXor
        | veln_ast::BinaryOp::BitwiseAnd
        | veln_ast::BinaryOp::ShiftLeft
        | veln_ast::BinaryOp::ShiftRight
        | veln_ast::BinaryOp::ShiftRightLogical => Type::int(),
        veln_ast::BinaryOp::Add
        | veln_ast::BinaryOp::Subtract
        | veln_ast::BinaryOp::Multiply
        | veln_ast::BinaryOp::Divide => {
            let left = context.infer(left, expected);
            let right = context.infer(right, expected);
            if left == Type::float() || right == Type::float() {
                Type::float()
            } else {
                Type::int()
            }
        }
        veln_ast::BinaryOp::PipeGreater => Type::Unknown,
    }
}

pub(crate) fn item_type_unknown_as_none(ty: &Type) -> Option<&Type> {
    (ty != &Type::Unknown).then_some(ty)
}

pub(crate) fn infer_match_scrutinee_type_from_constructor_patterns(
    arms: &[MatchArm],
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> MatchScrutineePatternInference {
    let mut inferred: Option<(AdtConstructor<'_>, Vec<Type>)> = None;

    for arm in arms {
        let PatternKind::Constructor { name, args, .. } = &arm.pattern.kind else {
            continue;
        };
        if invalid_qualified_constructor_pattern(name) {
            continue;
        }
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

pub(crate) fn invalid_qualified_constructor_pattern(name: &[String]) -> bool {
    name.len() > 1
        && name
            .last()
            .and_then(|name| name.as_bytes().first())
            .is_some_and(u8::is_ascii_lowercase)
}

pub(crate) fn infer_pattern_type_from_constructor_patterns(
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
        PatternKind::Constructor { name, args, .. } => {
            if invalid_qualified_constructor_pattern(name) {
                return None;
            }
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

pub(crate) fn unique_constructor_descriptor_names(
    constructors: &[AdtConstructor<'_>],
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

pub(crate) fn same_constructor_descriptor(
    left: &AdtConstructor<'_>,
    right: &AdtConstructor<'_>,
) -> bool {
    left.descriptor.type_name == right.descriptor.type_name
        && left.descriptor.module_name == right.descriptor.module_name
        && left.descriptor.type_parameters.len() == right.descriptor.type_parameters.len()
}

pub(crate) fn type_has_unknown(ty: &Type) -> bool {
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

pub(crate) fn infer_private_signature_name_type(
    segments: &[String],
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    if let ConstructorLookup::Found(constructor) =
        adts.nullary_constructor(segments, current_module, uses)
    {
        return expected
            .and_then(|expected| {
                unification::adt_args(expected, constructor.descriptor).map(|_| expected.clone())
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

pub(crate) struct PrivateSignatureInferContext<'a> {
    pub(crate) current_module: Option<&'a str>,
    pub(crate) uses: &'a [UseDecl],
    pub(crate) bindings: &'a [Binding],
    pub(crate) returns_by_path: &'a BTreeMap<(Option<String>, String), Type>,
    pub(crate) adts: &'a AdtRegistry,
}

impl PrivateSignatureInferContext<'_> {
    pub(crate) fn infer(&self, expr: &Expr, expected: Option<&Type>) -> Type {
        infer_private_signature_expr_type(
            expr,
            expected,
            self.current_module,
            self.uses,
            self.bindings,
            self.returns_by_path,
            self.adts,
        )
    }
}

pub(crate) fn infer_private_signature_call_type(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    if let ExprKind::NamePath { segments, .. } = &callee.kind {
        if let ConstructorLookup::Found(constructor) =
            context
                .adts
                .constructor(segments, context.current_module, context.uses)
        {
            let actual_args = args
                .iter()
                .map(|arg| context.infer(arg, None))
                .collect::<Vec<_>>();
            if expected
                .and_then(|expected| unification::adt_args(expected, constructor.descriptor))
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
                    context.infer(arg, Some(param));
                }
                return return_type;
            }
        }
    }
    Type::Unknown
}
