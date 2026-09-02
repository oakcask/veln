use super::*;

use crate::effects::{is_stdio_call, prelude_effects, standard_library_effects};

pub(super) struct ExprEffectContext<'a> {
    pub(super) uses: &'a [UseDecl],
    pub(super) current_module: Option<&'a str>,
    pub(super) bindings: &'a [Binding],
    pub(super) functions: &'a [FunctionSignature],
    pub(super) effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
    pub(super) effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
    pub(super) companion_access_targets: &'a BTreeMap<String, String>,
    pub(super) companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
    pub(super) user_effects: &'a [EffectSignature],
    pub(super) handlers: &'a [HandlerSignature],
}

pub(super) fn handler_for_path<'a>(
    segments: &[String],
    context: &ExprEffectContext<'a>,
) -> Option<&'a HandlerSignature> {
    match segments {
        [name] => context.handlers.iter().find(|handler| {
            handler.name == *name && handler.module_name.as_deref() == context.current_module
        }),
        [_, .., name] => {
            let use_decl = imported_use_for_path(
                context.uses,
                &segments[..segments.len() - 1],
                context.current_module,
            )?;
            context.handlers.iter().find(|handler| {
                handler.name == *name
                    && handler.module_name.as_deref() == Some(use_decl.name.as_str())
                    && imported_handler_is_visible(
                        handler,
                        use_decl,
                        context.current_module,
                        context.companion_effect_access_targets,
                    )
            })
        }
        _ => None,
    }
}

pub(super) fn collect_expr_effect_dependencies(
    expr: &Expr,
    context: &ExprEffectContext<'_>,
    dependencies: &mut BTreeSet<EffectDependencyNode>,
) {
    ExprEffectDependencyCollector {
        context,
        dependencies,
    }
    .collect(expr);
}

trait ExprEffectTraversal {
    fn collect(&mut self, expr: &Expr);

    fn collect_pair(&mut self, first: &Expr, second: &Expr) {
        self.collect(first);
        self.collect(second);
    }

    fn collect_all(&mut self, expressions: &[Expr]) {
        for expression in expressions {
            self.collect(expression);
        }
    }
}

struct ExprEffectDependencyCollector<'context, 'data, 'output> {
    context: &'context ExprEffectContext<'data>,
    dependencies: &'output mut BTreeSet<EffectDependencyNode>,
}

impl ExprEffectDependencyCollector<'_, '_, '_> {
    fn collect_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call { callee, args } => self.collect_call(callee, args),
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => self.collect_handle(body, handler, args),
            ExprKind::SchemaDecode { input, base, .. } => self.collect_pair(input, base),
            ExprKind::Perform { args, .. } => self.collect_all(args),
            ExprKind::SchemaEncode { value, .. } => self.collect(value),
            ExprKind::FieldAccess { base, .. }
            | ExprKind::Try(base)
            | ExprKind::TypeApply { callee: base, .. }
            | ExprKind::Prefix { expr: base, .. } => self.collect(base),
            ExprKind::Record(fields) => {
                for field in fields {
                    self.collect(&field.expr);
                }
            }
            ExprKind::Dict(entries) => {
                for entry in entries {
                    self.collect_pair(&entry.key, &entry.value);
                }
            }
            ExprKind::List(items) => self.collect_all(items),
            ExprKind::Match { scrutinee, arms } => {
                self.collect(scrutinee);
                for arm in arms {
                    self.collect(&arm.expr);
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.collect_pair(condition, then_branch);
                for branch in else_if_branches {
                    self.collect_pair(&branch.condition, &branch.expr);
                }
                self.collect(else_branch);
            }
            ExprKind::Binary { left, right, .. } => self.collect_pair(left, right),
            ExprKind::NamePath { segments, .. } => self.collect_name_path(segments),
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    fn collect_call(&mut self, callee: &Expr, args: &[Expr]) {
        if let Some(segments) = callee.callee_name_path() {
            self.collect_name_path(segments);
        } else {
            self.collect(callee);
        }
        self.collect_all(args);
    }

    fn collect_handle(&mut self, body: &Expr, handler: &[String], args: &[Expr]) {
        self.collect_all(args);
        if let Some(handler) = handler_for_path(handler, self.context)
            && handler.visibility != Visibility::Public
        {
            self.dependencies
                .insert(EffectDependencyNode::PrivateHandler(
                    handler.qualified_name.clone(),
                ));
        }
        self.collect(body);
    }

    fn collect_name_path(&mut self, segments: &[String]) {
        if let [name] = segments
            && let Some(target) = self
                .context
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.name == *name)
                .and_then(|binding| binding.private_function_value.clone())
        {
            self.dependencies
                .insert(EffectDependencyNode::Function(target));
            return;
        }
        if let Some(signature) = function_signature_path(
            segments,
            self.context.uses,
            self.context.functions,
            self.context.current_module,
            self.context.companion_access_targets,
        ) {
            self.dependencies.insert(EffectDependencyNode::Function((
                signature.module_name.clone(),
                signature.name.clone(),
            )));
        }
    }
}

impl ExprEffectTraversal for ExprEffectDependencyCollector<'_, '_, '_> {
    fn collect(&mut self, expr: &Expr) {
        self.collect_expr(expr);
    }
}

pub(super) fn collect_expr_effects(
    expr: &Expr,
    context: &ExprEffectContext<'_>,
    inferred: &mut Vec<String>,
) {
    ExprEffectCollector { context, inferred }.collect(expr);
}

struct ExprEffectCollector<'context, 'data, 'output> {
    context: &'context ExprEffectContext<'data>,
    inferred: &'output mut Vec<String>,
}

impl ExprEffectCollector<'_, '_, '_> {
    fn collect_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call { callee, args } => self.collect_call(callee, args),
            ExprKind::SchemaDecode { input, base, .. } => self.collect_pair(input, base),
            ExprKind::Perform { effect, args, .. } => self.collect_perform(effect, args),
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => self.collect_handle(body, handler, args),
            ExprKind::SchemaEncode { value, .. } => self.collect(value),
            ExprKind::FieldAccess { base, .. }
            | ExprKind::Try(base)
            | ExprKind::TypeApply { callee: base, .. }
            | ExprKind::Prefix { expr: base, .. } => self.collect(base),
            ExprKind::Record(fields) => self.collect_record_fields(fields),
            ExprKind::Dict(entries) => self.collect_dict_entries(entries),
            ExprKind::List(items) => self.collect_all(items),
            ExprKind::Match { scrutinee, arms } => self.collect_match(scrutinee, arms),
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => self.collect_if(condition, then_branch, else_if_branches, else_branch),
            ExprKind::Binary { left, right, .. } => self.collect_pair(left, right),
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::NamePath { .. }
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    fn collect_call(&mut self, callee: &Expr, args: &[Expr]) {
        let Some(segments) = callee.callee_name_path() else {
            self.collect(callee);
            self.collect_all(args);
            return;
        };
        if is_stdio_call(segments) {
            push_unique_effect(self.inferred, "stdio");
        } else if let Some(effects) = concurrency_effects_for_call(segments, args, self.context) {
            self.push_all(&effects);
        } else if let Some(effects) = standard_library_effects(segments) {
            for effect in effects {
                push_unique_effect(self.inferred, effect);
            }
        } else if let [name] = segments
            && let Some(effects) = lexical_effects_for_bare_callee(
                name,
                self.context.bindings,
                self.context.effects_by_function,
            )
        {
            self.push_all(effects);
        } else if let Some(effects) = prelude_effects(segments) {
            for effect in effects {
                push_unique_effect(self.inferred, effect);
            }
        } else if let Some(signature) = function_signature_path(
            segments,
            self.context.uses,
            self.context.functions,
            self.context.current_module,
            self.context.companion_access_targets,
        ) {
            self.push_all(&instantiate_call_effect_rows(signature, args, self.context));
        } else {
            if let Some(effects) = effects_for_callee_path(
                segments,
                self.context.uses,
                self.context.current_module,
                self.context.bindings,
                self.context.effects_by_function,
                self.context.effects_by_module_path,
                self.context.companion_access_targets,
            ) {
                self.push_all(effects);
            }
        }
        self.collect_all(args);
    }

    fn collect_perform(&mut self, effect: &[String], args: &[Expr]) {
        if let Some(label) = canonical_user_effect_label(
            effect,
            self.context.uses,
            self.context.current_module,
            self.context.user_effects,
            self.context.companion_effect_access_targets,
        ) {
            push_unique_effect(self.inferred, &label);
        }
        self.collect_all(args);
    }

    fn collect_handle(&mut self, body: &Expr, handler: &[String], args: &[Expr]) {
        self.collect_all(args);
        let Some((handled_effect, handler_effects)) = handler_for_path(handler, self.context)
            .map(|handler| (handler.effect.clone(), handler.effects.clone()))
        else {
            self.collect(body);
            return;
        };
        let before_body = self.inferred.len();
        self.collect(body);
        let retained_body_effects = self
            .inferred
            .drain(before_body..)
            .filter(|effect| effect != &handled_effect)
            .collect::<Vec<_>>();
        self.inferred.extend(retained_body_effects);
        self.push_all(&handler_effects);
    }

    fn collect_record_fields(&mut self, fields: &[RecordField]) {
        for field in fields {
            self.collect(&field.expr);
        }
    }

    fn collect_dict_entries(&mut self, entries: &[DictEntry]) {
        for entry in entries {
            self.collect_pair(&entry.key, &entry.value);
        }
    }

    fn collect_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) {
        self.collect(scrutinee);
        for arm in arms {
            self.collect(&arm.expr);
        }
    }

    fn collect_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_if_branches: &[IfBranch],
        else_branch: &Expr,
    ) {
        self.collect_pair(condition, then_branch);
        for branch in else_if_branches {
            self.collect_pair(&branch.condition, &branch.expr);
        }
        self.collect(else_branch);
    }

    fn push_all(&mut self, effects: &[String]) {
        for effect in effects {
            push_unique_effect(self.inferred, effect);
        }
    }
}

impl ExprEffectTraversal for ExprEffectCollector<'_, '_, '_> {
    fn collect(&mut self, expr: &Expr) {
        self.collect_expr(expr);
    }
}
