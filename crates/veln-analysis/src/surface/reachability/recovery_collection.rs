use super::*;

impl<'a> ReachableInvalidNameSelector<'a> {
    pub(super) fn new(inputs: &'a ReachabilityInputs<'_>) -> Self {
        let companion_access_targets = companion_function_access_targets(inputs);
        let aliases = inputs.aliases().collect::<Vec<_>>();
        let handlers = inputs.handlers();
        let types = inputs.types().collect::<Vec<_>>();
        let functions = inputs.functions().collect::<Vec<_>>();
        let functions_by_name = index_functions_by_name(&functions);
        let aliases_by_name = index_aliases_by_name(&aliases);
        let types_by_name = index_types_by_name(&types);
        let constructors_by_name = index_constructors_by_name(&types);
        let invalid_names = inputs.invalid_names().collect::<Vec<_>>();
        Self {
            uses: inputs.uses(),
            invalid_uses: inputs
                .all_uses()
                .into_iter()
                .filter(|use_decl| use_decl_has_invalid_module_segment(use_decl, &invalid_names))
                .collect(),
            handlers,
            functions_by_name,
            aliases_by_name,
            types_by_name,
            constructors_by_name,
            invalid_names,
            companion_access_targets,
        }
    }

    pub(super) fn collect_function(
        &mut self,
        function: &Function,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let mut local_bindings = function
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>();
        for param in &function.params {
            self.collect_type_annotation(
                param.ty.as_deref(),
                function.module_name.as_deref(),
                spans,
            );
        }
        self.collect_type_annotation(
            function.return_type.as_deref(),
            function.module_name.as_deref(),
            spans,
        );
        for line in &function.body {
            match &line.kind {
                veln_ast::BodyLineKind::Let {
                    pattern,
                    annotation,
                    expr,
                    ..
                } => {
                    self.collect_pattern(pattern, function.module_name.as_deref(), spans);
                    self.collect_type_annotation(
                        annotation.as_deref(),
                        function.module_name.as_deref(),
                        spans,
                    );
                    self.collect_expr(
                        expr,
                        function.module_name.as_deref(),
                        &local_bindings,
                        spans,
                    );
                    collect_pattern_binding_names(pattern, &mut local_bindings);
                }
                veln_ast::BodyLineKind::Expr { expr } => {
                    self.collect_expr(
                        expr,
                        function.module_name.as_deref(),
                        &local_bindings,
                        spans,
                    );
                }
            }
        }
    }

    pub(super) fn collect_type_annotation(
        &mut self,
        annotation: Option<&str>,
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let Some(annotation) = annotation else {
            return;
        };
        let Ok(type_names) = veln_sema::type_annotation_reference_paths(annotation) else {
            return;
        };
        for path in type_names {
            self.select_type_name(&path, current_module, spans);
        }
    }

    pub(super) fn collect_expr(
        &mut self,
        expr: &Expr,
        current_module: Option<&str>,
        local_bindings: &[String],
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        match &expr.kind {
            ExprKind::NamePath { segments, .. } => {
                if !matches!(segments.as_slice(), [name] if local_bindings.iter().rev().any(|binding| binding == name))
                {
                    self.select_value_name(segments, current_module, spans);
                }
            }
            ExprKind::Hole { .. } => {}
            ExprKind::TypeApply { callee, type_args } => {
                self.collect_expr(callee, current_module, local_bindings, spans);
                for type_arg in type_args {
                    self.collect_type_annotation(Some(type_arg), current_module, spans);
                }
            }
            ExprKind::Call { callee, args } => {
                if let Some(segments) = callee.callee_name_path() {
                    if !matches!(segments, [name] if local_bindings.iter().rev().any(|binding| binding == name))
                    {
                        self.select_call_name(segments, current_module, args.len(), spans);
                    }
                } else {
                    self.collect_expr(callee, current_module, local_bindings, spans);
                }
                for arg in args {
                    self.collect_expr(arg, current_module, local_bindings, spans);
                }
            }
            ExprKind::Perform { args, .. } => {
                for arg in args {
                    self.collect_expr(arg, current_module, local_bindings, spans);
                }
            }
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => {
                self.select_handler(handler, current_module, spans);
                self.collect_expr(body, current_module, local_bindings, spans);
                for arg in args {
                    self.collect_expr(arg, current_module, local_bindings, spans);
                }
            }
            ExprKind::SchemaDecode {
                schema: _,
                input,
                base,
            } => {
                self.collect_expr(input, current_module, local_bindings, spans);
                self.collect_expr(base, current_module, local_bindings, spans);
            }
            ExprKind::SchemaEncode { schema: _, value } => {
                self.collect_expr(value, current_module, local_bindings, spans);
            }
            ExprKind::FieldAccess { base, .. }
            | ExprKind::Try(base)
            | ExprKind::Prefix { expr: base, .. } => {
                self.collect_expr(base, current_module, local_bindings, spans);
            }
            ExprKind::Record(fields) => {
                for field in fields {
                    self.collect_expr(&field.expr, current_module, local_bindings, spans);
                }
            }
            ExprKind::Dict(entries) => {
                for entry in entries {
                    self.collect_expr(&entry.key, current_module, local_bindings, spans);
                    self.collect_expr(&entry.value, current_module, local_bindings, spans);
                }
            }
            ExprKind::List(items) => {
                for item in items {
                    self.collect_expr(item, current_module, local_bindings, spans);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.collect_expr(scrutinee, current_module, local_bindings, spans);
                for arm in arms {
                    self.collect_pattern(&arm.pattern, current_module, spans);
                    let mut arm_bindings = local_bindings.to_vec();
                    collect_pattern_binding_names(&arm.pattern, &mut arm_bindings);
                    self.collect_expr(&arm.expr, current_module, &arm_bindings, spans);
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.collect_expr(condition, current_module, local_bindings, spans);
                self.collect_expr(then_branch, current_module, local_bindings, spans);
                for branch in else_if_branches {
                    self.collect_expr(&branch.condition, current_module, local_bindings, spans);
                    self.collect_expr(&branch.expr, current_module, local_bindings, spans);
                }
                self.collect_expr(else_branch, current_module, local_bindings, spans);
            }
            ExprKind::Binary { left, right, .. } => {
                self.collect_expr(left, current_module, local_bindings, spans);
                self.collect_expr(right, current_module, local_bindings, spans);
            }
            ExprKind::Missing
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    pub(super) fn collect_pattern(
        &mut self,
        pattern: &Pattern,
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        match &pattern.kind {
            PatternKind::Binding(_) => {}
            PatternKind::Constructor { name, args, .. } => {
                self.select_constructor_name(name, current_module, None, spans);
                for arg in args {
                    self.collect_pattern(arg, current_module, spans);
                }
            }
            PatternKind::Record(fields) => {
                for field in fields {
                    self.collect_pattern(&field.pattern, current_module, spans);
                }
            }
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => {}
        }
    }

    pub(super) fn select_value_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if self.has_valid_constructor(segments, current_module, None) {
            return;
        }
        if self.select_invalid_import_path(segments, current_module, spans) {
            return;
        }
        if self.has_valid_function(segments, current_module, None) {
            return;
        }
        if self.has_valid_function_alias(segments, current_module) {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_value_recovery(segments, current_module, spans);
        }
    }

    pub(super) fn select_call_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: usize,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if self.has_valid_function(segments, current_module, None)
            || self.has_valid_function_alias(segments, current_module)
            || self.has_valid_constructor(segments, current_module, None)
        {
            return;
        }
        if self.select_invalid_import_path(segments, current_module, spans) {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_call_recovery(segments, current_module, arg_count, spans);
        }
    }

    pub(super) fn select_type_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if self.has_valid_type(segments, current_module)
            || self.has_valid_type_alias(segments, current_module)
        {
            return;
        }
        if self.select_invalid_import_path(segments, current_module, spans) {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_type_recovery(segments, current_module, spans);
        }
    }

    pub(super) fn select_constructor_name(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if self.has_valid_constructor(segments, current_module, arg_count) {
            return;
        }
        if self.select_invalid_import_path(segments, current_module, spans) {
            return;
        }
        if same_module_recovery_path(segments) {
            self.select_unique_constructor_recovery(segments, current_module, arg_count, spans);
        }
    }

    pub(super) fn select_handler(
        &mut self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        if let Some(handler) = self.visible_handler(segments, current_module) {
            if spans.iter().any(|span| span.is_declaration(&handler.span)) {
                return;
            }
            spans.push(ReachableInvalidNameSpan::Declaration(handler.span.clone()));
            self.collect_handler(handler, spans);
        } else {
            self.select_invalid_import_path(segments, current_module, spans);
        }
    }

    pub(super) fn select_invalid_import_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) -> bool {
        let Some(use_decl) = imported_use_for_path(
            &self.invalid_uses,
            &segments[..segments.len().saturating_sub(1)],
            current_module,
        ) else {
            return false;
        };
        spans.extend(invalid_import_path_segment_spans(
            use_decl,
            &self.invalid_names,
        ));
        true
    }

    pub(super) fn collect_handler(
        &mut self,
        handler: &veln_ast::HandlerDecl,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let current_module = handler.module_name.as_deref();
        let mut local_bindings = handler
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>();
        for param in &handler.params {
            self.collect_type_annotation(param.ty.as_deref(), current_module, spans);
        }
        for clause in &handler.operation_clauses {
            let binding_count = local_bindings.len();
            local_bindings.extend(clause.params.iter().map(|param| param.name.clone()));
            self.collect_expr(&clause.body, current_module, &local_bindings, spans);
            local_bindings.truncate(binding_count);
        }
    }
}
