use super::*;

#[derive(Clone, Debug)]
pub(super) struct LocalBinding {
    pub(super) name: String,
    pub(super) function_shape: Option<FunctionShape>,
}

pub(super) struct FunctionCalleeContext<'a> {
    pub(super) current_module: Option<&'a str>,
    pub(super) uses: &'a [&'a UseDecl],
    pub(super) function_targets: &'a FunctionTargetIndex,
    pub(super) companion_access_targets: &'a HashMap<String, String>,
    pub(super) handlers: &'a [&'a veln_ast::HandlerDecl],
    pub(super) types: &'a [&'a veln_ast::TypeDecl],
}

pub(super) fn direct_function_callees(
    function: &Function,
    inputs: &ReachabilityInputs<'_>,
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
) -> Vec<ReachableFunction> {
    let mut callees = Vec::new();
    let uses = inputs.uses();
    let handlers = inputs.handlers();
    let types = inputs.types().collect::<Vec<_>>();
    let context = FunctionCalleeContext {
        current_module: function.module_name.as_deref(),
        uses: &uses,
        function_targets,
        companion_access_targets,
        handlers: &handlers,
        types: &types,
    };
    let mut local_bindings = function
        .params
        .iter()
        .map(|param| LocalBinding {
            name: param.name.clone(),
            function_shape: param.ty.as_deref().and_then(function_type_shape),
        })
        .collect::<Vec<_>>();
    for contract in &function.contracts {
        collect_contract_callees(
            &contract.text,
            context.current_module,
            context.uses,
            function_targets,
            companion_access_targets,
            &mut callees,
        );
    }
    for line in &function.body {
        match &line.kind {
            veln_ast::BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                collect_function_callees(expr, &context, &local_bindings, &mut callees);
                collect_pattern_bindings(
                    pattern,
                    annotation.as_deref().and_then(function_type_shape),
                    &mut local_bindings,
                );
            }
            veln_ast::BodyLineKind::Expr { expr } => {
                collect_function_callees(expr, &context, &local_bindings, &mut callees);
            }
        }
    }
    callees
}

pub(super) fn collect_contract_callees(
    predicate: &str,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
    callees: &mut Vec<ReachableFunction>,
) {
    let source = SourceFile::new("<contract>", predicate);
    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .collect::<Vec<_>>();
    let mut index = 0usize;
    while index < tokens.len() {
        let name = &tokens[index];
        if name.kind != TokenKind::Ident {
            index += 1;
            continue;
        }
        let mut segments = vec![name.text.clone()];
        let mut next_index = index + 1;
        while next_index + 1 < tokens.len()
            && tokens[next_index].kind == TokenKind::DoubleColon
            && tokens[next_index + 1].kind == TokenKind::Ident
        {
            segments.push(tokens[next_index + 1].text.clone());
            next_index += 2;
        }
        let Some(next) = tokens.get(next_index) else {
            break;
        };
        if next.kind != TokenKind::LParen {
            index += 1;
            continue;
        }
        for callee in resolve_function_reference(
            &segments,
            current_module,
            uses,
            function_targets,
            companion_access_targets,
            None,
        ) {
            push_reachable(callees, callee);
        }
        index = next_index + 1;
    }
    collect_contract_function_value_references(
        &tokens,
        current_module,
        uses,
        function_targets,
        companion_access_targets,
        callees,
    );
}

pub(super) fn collect_contract_function_value_references(
    tokens: &[veln_syntax::Token],
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    _companion_access_targets: &HashMap<String, String>,
    callees: &mut Vec<ReachableFunction>,
) {
    let mut index = 0usize;
    while index < tokens.len() {
        if tokens[index].kind != TokenKind::Ident {
            index += 1;
            continue;
        }
        if index > 0
            && matches!(
                tokens[index - 1].kind,
                TokenKind::Dot | TokenKind::DoubleColon
            )
        {
            index += 1;
            continue;
        }
        if tokens
            .get(index + 1)
            .is_some_and(|token| matches!(token.kind, TokenKind::Dot | TokenKind::LParen))
        {
            index += 1;
            continue;
        }
        let segments = if tokens
            .get(index + 1)
            .is_some_and(|token| token.kind == TokenKind::DoubleColon)
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.kind == TokenKind::Ident)
        {
            let mut segments = vec![tokens[index].text.clone()];
            index += 1;
            while tokens
                .get(index)
                .is_some_and(|token| token.kind == TokenKind::DoubleColon)
                && tokens
                    .get(index + 1)
                    .is_some_and(|token| token.kind == TokenKind::Ident)
            {
                segments.push(tokens[index + 1].text.clone());
                index += 2;
            }
            segments
        } else {
            let segments = vec![tokens[index].text.clone()];
            index += 1;
            segments
        };
        let public_or_same_module_access = HashMap::new();
        for callee in resolve_function_reference(
            &segments,
            current_module,
            uses,
            function_targets,
            &public_or_same_module_access,
            None,
        ) {
            push_reachable(callees, callee);
        }
    }
}

pub(super) fn collect_function_callees(
    expr: &Expr,
    context: &FunctionCalleeContext<'_>,
    local_bindings: &[LocalBinding],
    callees: &mut Vec<ReachableFunction>,
) {
    let current_module = context.current_module;
    let uses = context.uses;
    let function_targets = context.function_targets;
    let companion_access_targets = context.companion_access_targets;
    let handlers = context.handlers;

    match &expr.kind {
        ExprKind::NamePath { segments, .. } => {
            collect_function_name_reference(segments, context, local_bindings, None, callees);
        }
        ExprKind::TypeApply { callee, .. } => {
            collect_function_callees(callee, context, local_bindings, callees);
        }
        ExprKind::Call { callee, args } => {
            if let Some(segments) = callee.callee_name_path() {
                collect_function_name_reference(
                    segments,
                    context,
                    local_bindings,
                    Some(args.len()),
                    callees,
                );
            } else {
                collect_function_callees(callee, context, local_bindings, callees);
            }
            for arg in args {
                collect_function_callees(arg, context, local_bindings, callees);
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_function_callees(arg, context, local_bindings, callees);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_handler_operation_clause_callees(
                expr,
                current_module,
                uses,
                function_targets,
                companion_access_targets,
                handlers,
                callees,
            );
            collect_function_callees(body, context, local_bindings, callees);
            for arg in args {
                collect_function_callees(arg, context, local_bindings, callees);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_function_callees(input, context, local_bindings, callees);
            collect_function_callees(base, context, local_bindings, callees);
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_function_callees(value, context, local_bindings, callees);
        }
        ExprKind::FieldAccess { base, .. } => {
            collect_function_callees(base, context, local_bindings, callees);
        }
        ExprKind::Try(inner) => collect_function_callees(inner, context, local_bindings, callees),
        ExprKind::Record(fields) => {
            for field in fields {
                collect_function_callees(&field.expr, context, local_bindings, callees);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_function_callees(&entry.key, context, local_bindings, callees);
                collect_function_callees(&entry.value, context, local_bindings, callees);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_function_callees(item, context, local_bindings, callees);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_function_callees(scrutinee, context, local_bindings, callees);
            for arm in arms {
                let mut arm_bindings = local_bindings.to_vec();
                collect_pattern_bindings(&arm.pattern, None, &mut arm_bindings);
                collect_function_callees(&arm.expr, context, &arm_bindings, callees);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_function_callees(condition, context, local_bindings, callees);
            collect_function_callees(then_branch, context, local_bindings, callees);
            for branch in else_if_branches {
                collect_function_callees(&branch.condition, context, local_bindings, callees);
                collect_function_callees(&branch.expr, context, local_bindings, callees);
            }
            collect_function_callees(else_branch, context, local_bindings, callees);
        }
        ExprKind::Prefix { expr, .. } => {
            collect_function_callees(expr, context, local_bindings, callees);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_function_callees(left, context, local_bindings, callees);
            collect_function_callees(right, context, local_bindings, callees);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

pub(super) fn collect_pattern_bindings(
    pattern: &Pattern,
    function_shape: Option<FunctionShape>,
    bindings: &mut Vec<LocalBinding>,
) {
    if let PatternKind::Binding(name) = &pattern.kind {
        bindings.push(LocalBinding {
            name: name.clone(),
            function_shape,
        });
        return;
    }
    pattern.for_each_binding(&mut |name| {
        bindings.push(LocalBinding {
            name: name.to_string(),
            function_shape: None,
        });
    });
}

pub(super) fn collect_opaque_function_value_callees(
    shape: &FunctionShape,
    arg_count: Option<usize>,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    _companion_access_targets: &HashMap<String, String>,
    callees: &mut Vec<ReachableFunction>,
) {
    if current_module.is_some_and(|module| module.starts_with("std::")) {
        return;
    }
    if shape.variadic.is_some() && arg_count.is_some_and(|arg_count| arg_count < shape.fixed_arity)
    {
        return;
    }
    let public_or_same_module_access = HashMap::new();
    for target in function_targets.shaped(shape).filter(|target| {
        target_visible_from_current_module(
            target,
            current_module,
            uses,
            &public_or_same_module_access,
        )
    }) {
        push_reachable(
            callees,
            ReachableFunction {
                kind: FunctionKind::Function,
                name: target.name.clone(),
                module_name: target.module_name.clone(),
                node_id: None,
            },
        );
    }
}
