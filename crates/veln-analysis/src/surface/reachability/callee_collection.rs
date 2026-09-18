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

pub(super) struct CalleeCollectionInputs<'a> {
    uses: Vec<&'a UseDecl>,
    handlers: Vec<&'a veln_ast::HandlerDecl>,
    types: Vec<&'a veln_ast::TypeDecl>,
}

impl<'a> CalleeCollectionInputs<'a> {
    pub(super) fn new(inputs: &ReachabilityInputs<'a>) -> Self {
        #[cfg(test)]
        reachability_counters::record_callee_context_preparation();
        Self {
            uses: inputs.uses(),
            handlers: inputs.handlers(),
            types: inputs.types().collect(),
        }
    }
}

pub(super) fn direct_function_callees(
    function: &Function,
    inputs: &CalleeCollectionInputs<'_>,
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
) -> Vec<ReachableFunction> {
    let mut callees = Vec::new();
    let context = FunctionCalleeContext {
        current_module: function.module_name.as_deref(),
        uses: &inputs.uses,
        function_targets,
        companion_access_targets,
        handlers: &inputs.handlers,
        types: &inputs.types,
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
                ..
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
    match &expr.kind {
        ExprKind::NamePath { segments, .. } => {
            collect_function_name_reference(segments, context, local_bindings, None, callees);
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
        ExprKind::Match { scrutinee, arms } => {
            collect_function_callees(scrutinee, context, local_bindings, callees);
            for arm in arms {
                let mut arm_bindings = local_bindings.to_vec();
                collect_pattern_bindings(&arm.pattern, None, &mut arm_bindings);
                collect_function_callees(&arm.expr, context, &arm_bindings, callees);
            }
        }
        _ => {
            if matches!(expr.kind, ExprKind::Handle { .. }) {
                collect_handler_operation_clause_callees(
                    expr,
                    context.current_module,
                    context.uses,
                    context.function_targets,
                    context.companion_access_targets,
                    context.handlers,
                    callees,
                );
            }
            expr.for_each_child(&mut |child| {
                collect_function_callees(child, context, local_bindings, callees);
            });
        }
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
                name: target.target_name.clone(),
                module_name: target.target_module_name.clone(),
                node_id: Some(target.target_node_id),
                alias: target.alias.clone(),
            },
        );
    }
}
