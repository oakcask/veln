use super::*;

use crate::name_recovery::normal_use_decls;
use crate::type_syntax::parse_type_or_unknown;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum EffectDependencyNode {
    Function(FunctionKey),
    PrivateHandler(String),
}

struct FunctionEffectContext<'a> {
    uses: &'a [UseDecl],
    functions: &'a [FunctionSignature],
    user_effects: &'a [EffectSignature],
    handlers: &'a [HandlerSignature],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
    companion_access_targets: &'a BTreeMap<String, String>,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
}

impl FunctionEffectContext<'_> {
    fn expression_context<'a>(
        &'a self,
        current_module: Option<&'a str>,
        bindings: &'a [Binding],
    ) -> ExprEffectContext<'a> {
        ExprEffectContext {
            uses: self.uses,
            current_module,
            bindings,
            functions: self.functions,
            effects_by_function: self.effects_by_function,
            effects_by_module_path: self.effects_by_module_path,
            companion_access_targets: self.companion_access_targets,
            companion_effect_access_targets: self.companion_effect_access_targets,
            user_effects: self.user_effects,
            handlers: self.handlers,
        }
    }
}

struct HandlerEffectContext<'a> {
    module: &'a SurfaceModule,
    uses: &'a [UseDecl],
    user_effects: &'a [EffectSignature],
    functions: &'a [FunctionSignature],
    effects_by_function: &'a EffectsByFunction,
    effects_by_module_path: &'a EffectsByModulePath,
    handlers: &'a [HandlerSignature],
    companion_access_targets: &'a BTreeMap<String, String>,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
}

pub(super) fn infer_function_and_private_handler_effects(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    user_effects: &[EffectSignature],
    handlers: &mut [HandlerSignature],
) {
    EffectInference::new(module, functions, user_effects, handlers).run();
}

pub(super) type EffectsByFunction = BTreeMap<(Option<String>, String), Vec<String>>;
pub(super) type EffectsByModulePath = BTreeMap<(String, String), (Vec<String>, Visibility)>;

struct EffectInference<'a> {
    module: &'a SurfaceModule,
    uses: Vec<UseDecl>,
    functions: &'a mut [FunctionSignature],
    user_effects: &'a [EffectSignature],
    handlers: &'a mut [HandlerSignature],
    graph: EffectDependencyGraph,
    companion_access_targets: BTreeMap<String, String>,
    companion_effect_access_targets: BTreeMap<String, CompanionAccessTarget>,
    clause_companion_access_targets: BTreeMap<String, String>,
    effects_by_function: EffectsByFunction,
    effects_by_module_path: EffectsByModulePath,
    handler_index: BTreeMap<String, usize>,
    function_index: BTreeMap<FunctionKey, usize>,
    function_ast_by_key: BTreeMap<FunctionKey, &'a Function>,
    queue: VecDeque<EffectDependencyNode>,
    queued: BTreeSet<EffectDependencyNode>,
    evaluated: BTreeSet<EffectDependencyNode>,
}

impl<'a> EffectInference<'a> {
    fn new(
        module: &'a SurfaceModule,
        functions: &'a mut [FunctionSignature],
        user_effects: &'a [EffectSignature],
        handlers: &'a mut [HandlerSignature],
    ) -> Self {
        let graph = effect_dependency_graph(module, functions, user_effects, handlers);
        let (effects_by_function, effects_by_module_path) = effect_lookup_maps(functions);
        let queue = graph.ordered_nodes.iter().cloned().collect();
        let queued = graph.nodes.clone();
        Self {
            module,
            uses: normal_use_decls(module),
            companion_access_targets: companion_function_access_targets(module),
            companion_effect_access_targets: companion_access_target_infos(module),
            clause_companion_access_targets: companion_access_targets_for_signatures(functions),
            effects_by_function,
            effects_by_module_path,
            handler_index: handler_signature_index(handlers),
            function_index: function_signature_index(functions),
            function_ast_by_key: function_ast_index(module),
            queue,
            queued,
            evaluated: BTreeSet::new(),
            graph,
            functions,
            user_effects,
            handlers,
        }
    }

    fn run(mut self) {
        while let Some(node) = self.queue.pop_front() {
            self.queued.remove(&node);
            let is_reevaluation = self.evaluated.contains(&node);
            let Some(changed) = self.evaluate_node(&node, is_reevaluation) else {
                continue;
            };
            self.evaluated.insert(node.clone());
            if changed {
                self.enqueue_dependents(&node);
            }
        }
    }

    fn evaluate_node(
        &mut self,
        node: &EffectDependencyNode,
        is_reevaluation: bool,
    ) -> Option<bool> {
        if is_reevaluation {
            #[cfg(test)]
            effect_inference_counters::record_changed_reevaluation();
        }
        match node {
            EffectDependencyNode::Function(function_key) => self.evaluate_function(function_key),
            EffectDependencyNode::PrivateHandler(qualified_name) => {
                self.evaluate_handler(qualified_name)
            }
        }
    }

    fn evaluate_function(&mut self, function_key: &FunctionKey) -> Option<bool> {
        let function = self.function_ast_by_key.get(function_key).copied()?;
        let inferred = collect_function_body_effects(function, &self.function_context());
        let changed = self.effects_by_function.get(function_key) != Some(&inferred);
        if changed {
            self.update_function_effects(function_key, inferred);
        }
        Some(changed)
    }

    fn evaluate_handler(&mut self, qualified_name: &str) -> Option<bool> {
        let index = self.handler_index.get(qualified_name).copied()?;
        let inferred = collect_private_handler_effects(
            &self.handlers[index],
            &HandlerEffectContext {
                module: self.module,
                uses: &self.uses,
                user_effects: self.user_effects,
                functions: self.functions,
                effects_by_function: &self.effects_by_function,
                effects_by_module_path: &self.effects_by_module_path,
                handlers: self.handlers,
                companion_access_targets: &self.clause_companion_access_targets,
                companion_effect_access_targets: &self.companion_effect_access_targets,
            },
        );
        let changed = self.handlers[index].effects != inferred;
        if changed {
            self.handlers[index].effects = inferred;
        }
        Some(changed)
    }

    fn function_context(&self) -> FunctionEffectContext<'_> {
        FunctionEffectContext {
            uses: &self.uses,
            functions: self.functions,
            user_effects: self.user_effects,
            handlers: self.handlers,
            effects_by_function: &self.effects_by_function,
            effects_by_module_path: &self.effects_by_module_path,
            companion_access_targets: &self.companion_access_targets,
            companion_effect_access_targets: &self.companion_effect_access_targets,
        }
    }

    fn update_function_effects(&mut self, function_key: &FunctionKey, inferred: Vec<String>) {
        self.effects_by_function
            .insert(function_key.clone(), inferred.clone());
        if let Some(module_name) = &function_key.0 {
            let visibility = self
                .function_index
                .get(function_key)
                .map(|index| self.functions[*index].visibility)
                .unwrap_or(Visibility::Private);
            self.effects_by_module_path.insert(
                (module_name.clone(), function_key.1.clone()),
                (inferred.clone(), visibility),
            );
        }
        if let Some(index) = self.function_index.get(function_key).copied() {
            self.functions[index].effects = inferred;
        }
    }

    fn enqueue_dependents(&mut self, node: &EffectDependencyNode) {
        let Some(dependents) = self.graph.dependents.get(node) else {
            return;
        };
        for dependent in dependents {
            if self.queued.insert(dependent.clone()) {
                self.queue.push_back(dependent.clone());
            }
        }
    }
}

fn effect_lookup_maps(functions: &[FunctionSignature]) -> (EffectsByFunction, EffectsByModulePath) {
    let effects_by_function = functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.effects.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let effects_by_module_path = functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone()?, function.name.clone()),
                (function.effects.clone(), function.visibility),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    (effects_by_function, effects_by_module_path)
}

fn handler_signature_index(handlers: &[HandlerSignature]) -> BTreeMap<String, usize> {
    handlers
        .iter()
        .enumerate()
        .map(|(index, handler)| (handler.qualified_name.clone(), index))
        .collect()
}

fn function_signature_index(functions: &[FunctionSignature]) -> BTreeMap<FunctionKey, usize> {
    functions
        .iter()
        .enumerate()
        .map(|(index, function)| ((function.module_name.clone(), function.name.clone()), index))
        .collect()
}

fn function_ast_index(module: &SurfaceModule) -> BTreeMap<FunctionKey, &Function> {
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect()
}

struct EffectDependencyGraph {
    nodes: BTreeSet<EffectDependencyNode>,
    ordered_nodes: Vec<EffectDependencyNode>,
    dependents: BTreeMap<EffectDependencyNode, BTreeSet<EffectDependencyNode>>,
}

impl EffectDependencyGraph {
    fn new() -> Self {
        Self {
            nodes: BTreeSet::new(),
            ordered_nodes: Vec::new(),
            dependents: BTreeMap::new(),
        }
    }

    fn insert_node(&mut self, node: EffectDependencyNode) {
        if self.nodes.insert(node.clone()) {
            self.ordered_nodes.push(node);
        }
    }

    fn insert_dependency(
        &mut self,
        dependency: EffectDependencyNode,
        dependent: EffectDependencyNode,
    ) {
        self.dependents
            .entry(dependency)
            .or_default()
            .insert(dependent);
    }
}

fn effect_dependency_graph(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    user_effects: &[EffectSignature],
    handlers: &[HandlerSignature],
) -> EffectDependencyGraph {
    let uses = normal_use_decls(module);
    let companion_access_targets = companion_function_access_targets(module);
    let companion_effect_access_targets = companion_access_target_infos(module);
    let (effects_by_function, effects_by_module_path) = effect_lookup_maps(functions);
    let module_private_handlers = module
        .handlers
        .iter()
        .filter(|handler| handler.visibility != Visibility::Public)
        .filter_map(|handler| {
            let name = handler.name.as_deref()?;
            Some(qualified_name(handler.module_name.as_deref(), name))
        })
        .collect::<BTreeSet<_>>();
    let context = FunctionEffectContext {
        uses: &uses,
        functions,
        user_effects,
        handlers,
        effects_by_function: &effects_by_function,
        effects_by_module_path: &effects_by_module_path,
        companion_access_targets: &companion_access_targets,
        companion_effect_access_targets: &companion_effect_access_targets,
    };
    let mut graph = EffectDependencyGraph::new();
    for function in module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
    {
        insert_function_effect_dependencies(&mut graph, function, &context);
    }
    for handler in handlers
        .iter()
        .filter(|handler| handler.visibility != Visibility::Public)
        .filter(|handler| module_private_handlers.contains(&handler.qualified_name))
    {
        insert_handler_effect_dependencies(&mut graph, handler, module, &context);
    }
    graph
}

fn qualified_name(module_name: Option<&str>, name: &str) -> String {
    module_name.map_or_else(
        || name.to_string(),
        |module_name| format!("{module_name}::{name}"),
    )
}

fn insert_function_effect_dependencies(
    graph: &mut EffectDependencyGraph,
    function: &Function,
    context: &FunctionEffectContext<'_>,
) {
    let Some(name) = &function.name else {
        return;
    };
    #[cfg(test)]
    effect_inference_counters::record_dependency_discovery_scan();
    let node = EffectDependencyNode::Function((function.module_name.clone(), name.clone()));
    graph.insert_node(node.clone());
    for dependency in function_effect_dependencies(function, context) {
        graph.insert_dependency(dependency, node.clone());
    }
}

fn insert_handler_effect_dependencies(
    graph: &mut EffectDependencyGraph,
    handler: &HandlerSignature,
    module: &SurfaceModule,
    context: &FunctionEffectContext<'_>,
) {
    #[cfg(test)]
    effect_inference_counters::record_dependency_discovery_scan();
    let node = EffectDependencyNode::PrivateHandler(handler.qualified_name.clone());
    graph.insert_node(node.clone());
    let Some(decl) = module.handlers.iter().find(|decl| {
        decl.name.as_deref() == Some(handler.name.as_str())
            && decl.module_name == handler.module_name
    }) else {
        return;
    };
    let mut bindings = handler_parameter_bindings(decl, handler);
    for clause in &decl.operation_clauses {
        let binding_count = bindings.len();
        bindings.extend(
            clause
                .params
                .iter()
                .filter(|param| valid_value_binding_name(&param.name))
                .map(|param| Binding::new(param.name.clone(), Type::Unknown)),
        );
        let expr_context = context.expression_context(handler.module_name.as_deref(), &bindings);
        let mut dependencies = BTreeSet::new();
        collect_expr_effect_dependencies(&clause.body, &expr_context, &mut dependencies);
        for dependency in dependencies {
            graph.insert_dependency(dependency, node.clone());
        }
        bindings.truncate(binding_count);
    }
}

fn collect_private_handler_effects(
    handler: &HandlerSignature,
    context: &HandlerEffectContext<'_>,
) -> Vec<String> {
    #[cfg(test)]
    effect_inference_counters::record_handler_operation_clause_evaluation();
    let Some(decl) = context.module.handlers.iter().find(|decl| {
        decl.name.as_deref() == Some(handler.name.as_str())
            && decl.module_name == handler.module_name
    }) else {
        return Vec::new();
    };
    let Some(effect) = context
        .user_effects
        .iter()
        .find(|effect| effect.qualified_name == handler.effect)
    else {
        return Vec::new();
    };
    let mut inferred = Vec::new();
    for clause in &decl.operation_clauses {
        let Some(operation_name) = clause.operation.as_deref() else {
            continue;
        };
        let Some(operation) = effect
            .operations
            .iter()
            .find(|operation| operation.name == operation_name)
        else {
            continue;
        };
        let mut bindings = handler_parameter_bindings(decl, handler);
        bindings.extend(
            clause
                .params
                .iter()
                .enumerate()
                .filter_map(|(index, param)| {
                    if valid_value_binding_name(&param.name) {
                        Some(Binding::new(
                            param.name.clone(),
                            operation
                                .params
                                .get(index)
                                .cloned()
                                .unwrap_or(Type::Unknown),
                        ))
                    } else {
                        None
                    }
                }),
        );
        let expr_context = ExprEffectContext {
            uses: context.uses,
            current_module: handler.module_name.as_deref(),
            bindings: &bindings,
            functions: context.functions,
            effects_by_function: context.effects_by_function,
            effects_by_module_path: context.effects_by_module_path,
            companion_access_targets: context.companion_access_targets,
            companion_effect_access_targets: context.companion_effect_access_targets,
            user_effects: context.user_effects,
            handlers: context.handlers,
        };
        collect_expr_effects(&clause.body, &expr_context, &mut inferred);
    }
    inferred
}

fn handler_parameter_bindings(
    declaration: &HandlerDecl,
    signature: &HandlerSignature,
) -> Vec<Binding> {
    declaration
        .params
        .iter()
        .enumerate()
        .filter(|(_, param)| valid_value_binding_name(&param.name))
        .map(|(index, param)| {
            Binding::new(
                param.name.clone(),
                signature
                    .params
                    .get(index)
                    .cloned()
                    .unwrap_or(Type::Unknown),
            )
        })
        .collect()
}

fn collect_function_body_effects(
    function: &Function,
    context: &FunctionEffectContext<'_>,
) -> Vec<String> {
    #[cfg(test)]
    effect_inference_counters::record_function_body_collection();
    let function_key = (
        function.module_name.clone(),
        function.name.clone().unwrap_or_default(),
    );
    let mut inferred = context
        .effects_by_function
        .get(&function_key)
        .cloned()
        .unwrap_or_default();
    visit_function_body_expressions(function, context, |expr, expr_context| {
        collect_expr_effects(expr, expr_context, &mut inferred);
    });
    inferred
}

fn function_effect_dependencies(
    function: &Function,
    context: &FunctionEffectContext<'_>,
) -> BTreeSet<EffectDependencyNode> {
    let mut dependencies = BTreeSet::new();
    visit_function_body_expressions(function, context, |expr, expr_context| {
        collect_expr_effect_dependencies(expr, expr_context, &mut dependencies);
    });
    dependencies
}

fn visit_function_body_expressions(
    function: &Function,
    context: &FunctionEffectContext<'_>,
    mut visit: impl FnMut(&Expr, &ExprEffectContext<'_>),
) {
    let mut bindings = function_parameter_bindings(function);
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
                ..
            } => {
                let expr_context =
                    context.expression_context(function.module_name.as_deref(), &bindings);
                visit(expr, &expr_context);
                let ty = parse_type_or_unknown(annotation.as_deref());
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                let expr_context =
                    context.expression_context(function.module_name.as_deref(), &bindings);
                visit(expr, &expr_context);
            }
        }
    }
}

pub(crate) fn canonical_user_effect_label(
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Option<String> {
    match segments {
        [name] => effects
            .iter()
            .find(|effect| effect.name == *name && effect.module_name.as_deref() == current_module)
            .map(|effect| effect.qualified_name.clone()),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            effects
                .iter()
                .find(|effect| {
                    effect.name == *name
                        && effect.module_name.as_deref() == Some(use_decl.name.as_str())
                        && imported_effect_is_visible(
                            use_decl,
                            current_module,
                            use_decl.name.as_str(),
                            effect.visibility,
                            companion_effect_access_targets,
                        )
                })
                .map(|effect| effect.qualified_name.clone())
        }
        _ => None,
    }
}

pub(super) fn quarantined_public_user_effect_label(
    segments: &[String],
    quarantined_uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
) -> Option<String> {
    let [_, .., name] = segments else {
        return None;
    };
    let use_decl = imported_use_for_path(
        quarantined_uses,
        &segments[..segments.len() - 1],
        current_module,
    )?;
    let mut matches = effects.iter().filter(|effect| {
        effect.name == *name
            && effect.module_name.as_deref() == Some(use_decl.name.as_str())
            && effect.visibility == Visibility::Public
    });
    let first = matches.next()?;
    matches
        .next()
        .is_none()
        .then(|| first.qualified_name.clone())
}

pub(crate) fn imported_effect_is_visible(
    use_decl: &UseDecl,
    current_module: Option<&str>,
    target_module: &str,
    visibility: Visibility,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> bool {
    visibility == Visibility::Public
        || (use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                (current_module.starts_with("std::") && target_module.starts_with("std::"))
                    || companion_effect_access_targets
                        .get(current_module)
                        .is_some_and(|access| access.target_module == target_module)
            }))
}
