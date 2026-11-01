#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use veln_ast::{BodyLine, BodyLineKind, Expr, ExprKind, SurfaceModule, Visibility};
use veln_source::SourceSpan;

use crate::analysis::{VariantDiagnosticInterner, check_module_values};
use crate::types::TypeEnvironment;

#[cfg(test)]
thread_local! {
    static MODULE_VALUE_BINDING_PUSHES: Cell<usize> = const { Cell::new(0) };
    static MODULE_VALUE_BINDING_POPS: Cell<usize> = const { Cell::new(0) };
    static MODULE_VALUE_BINDING_LOOKUPS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct ModuleValueNavigationWork {
    pub(crate) binding_pushes: usize,
    pub(crate) binding_pops: usize,
    pub(crate) binding_lookups: usize,
}

#[cfg(test)]
pub(crate) fn reset_module_value_navigation_work() {
    MODULE_VALUE_BINDING_PUSHES.set(0);
    MODULE_VALUE_BINDING_POPS.set(0);
    MODULE_VALUE_BINDING_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn module_value_navigation_work() -> ModuleValueNavigationWork {
    ModuleValueNavigationWork {
        binding_pushes: MODULE_VALUE_BINDING_PUSHES.get(),
        binding_pops: MODULE_VALUE_BINDING_POPS.get(),
        binding_lookups: MODULE_VALUE_BINDING_LOOKUPS.get(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleValueNavigationIdentity {
    pub package: Option<String>,
    pub module: Option<String>,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct ResolvedModuleValueNavigation {
    pub identity: ModuleValueNavigationIdentity,
    pub declaration: SourceSpan,
    pub visibility: Visibility,
    pub references: Vec<SourceSpan>,
}

#[derive(Default)]
struct ScopedBindings {
    names: Vec<String>,
    counts: HashMap<String, usize>,
}

impl ScopedBindings {
    fn push(&mut self, name: String) {
        #[cfg(test)]
        MODULE_VALUE_BINDING_PUSHES.set(MODULE_VALUE_BINDING_PUSHES.get() + 1);
        *self.counts.entry(name.clone()).or_default() += 1;
        self.names.push(name);
    }

    fn contains(&self, name: &str) -> bool {
        #[cfg(test)]
        MODULE_VALUE_BINDING_LOOKUPS.set(MODULE_VALUE_BINDING_LOOKUPS.get() + 1);
        self.counts.contains_key(name)
    }

    fn len(&self) -> usize {
        self.names.len()
    }

    fn truncate(&mut self, len: usize) {
        while self.names.len() > len {
            let name = self.names.pop().expect("binding length was checked");
            #[cfg(test)]
            MODULE_VALUE_BINDING_POPS.set(MODULE_VALUE_BINDING_POPS.get() + 1);
            let count = self
                .counts
                .get_mut(&name)
                .expect("stacked binding must have a count");
            *count -= 1;
            if *count == 0 {
                self.counts.remove(&name);
            }
        }
    }
}

pub fn resolved_module_value_navigation(
    module: &SurfaceModule,
) -> Vec<ResolvedModuleValueNavigation> {
    if module.module_values.is_empty() {
        return Vec::new();
    }
    let mut environment = TypeEnvironment::from_module(module);
    let analysis = check_module_values(
        module,
        &mut environment,
        &mut VariantDiagnosticInterner::default(),
    );
    let mut resolved = admitted_module_value_declarations(module, &analysis.invalid);
    if resolved.is_empty() {
        return Vec::new();
    }
    collect_initializer_references(module, &environment, &mut resolved);
    collect_function_references(module, &environment, &mut resolved);
    collect_handler_references(module, &environment, &mut resolved);

    resolved.into_values().collect()
}

fn admitted_module_value_declarations(
    module: &SurfaceModule,
    invalid: &BTreeSet<(Option<String>, Option<String>, String)>,
) -> BTreeMap<ModuleValueNavigationIdentity, ResolvedModuleValueNavigation> {
    let mut declarations = BTreeMap::<ModuleValueNavigationIdentity, Vec<_>>::new();
    for value in &module.module_values {
        let (Some(name), Some(declaration)) = (&value.name, &value.name_span) else {
            continue;
        };
        if invalid.contains(&(
            value.package_name.clone(),
            value.module_name.clone(),
            name.clone(),
        )) {
            continue;
        }
        declarations
            .entry(ModuleValueNavigationIdentity {
                package: value.package_name.clone(),
                module: value.module_name.clone(),
                name: name.clone(),
            })
            .or_default()
            .push((declaration.clone(), value.visibility));
    }
    declarations
        .into_iter()
        .filter_map(|(identity, candidates)| {
            let [(declaration, visibility)] = candidates.as_slice() else {
                return None;
            };
            Some((
                identity.clone(),
                ResolvedModuleValueNavigation {
                    identity,
                    declaration: declaration.clone(),
                    visibility: *visibility,
                    references: Vec::new(),
                },
            ))
        })
        .collect()
}

fn collect_initializer_references(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    resolved: &mut BTreeMap<ModuleValueNavigationIdentity, ResolvedModuleValueNavigation>,
) {
    for value in &module.module_values {
        collect_expr_references(
            &value.initializer,
            value.module_name.as_deref(),
            value.package_name.as_deref(),
            environment,
            &mut ScopedBindings::default(),
            resolved,
        );
    }
}

fn collect_function_references(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    resolved: &mut BTreeMap<ModuleValueNavigationIdentity, ResolvedModuleValueNavigation>,
) {
    for function in &module.functions {
        let mut bindings = ScopedBindings::default();
        if function.callsite.is_some() {
            bindings.push("callsite".to_string());
        }
        for parameter in &function.params {
            bindings.push(parameter.name.clone());
        }
        if let Some(binding) = &function.return_binding {
            bindings.push(binding.name.clone());
        }
        collect_body_references(
            &function.body,
            function.module_name.as_deref(),
            function.package_name.as_deref(),
            environment,
            &mut bindings,
            resolved,
        );
    }
}

fn collect_handler_references(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    resolved: &mut BTreeMap<ModuleValueNavigationIdentity, ResolvedModuleValueNavigation>,
) {
    for handler in &module.handlers {
        for clause in &handler.operation_clauses {
            let mut bindings = ScopedBindings::default();
            for parameter in &handler.params {
                bindings.push(parameter.name.clone());
            }
            for parameter in &clause.params {
                bindings.push(parameter.name.clone());
            }
            collect_expr_references(
                &clause.body,
                handler.module_name.as_deref(),
                handler.package_name.as_deref(),
                environment,
                &mut bindings,
                resolved,
            );
        }
    }
}

fn collect_expr_references(
    expr: &Expr,
    owner_module: Option<&str>,
    owner_package: Option<&str>,
    environment: &TypeEnvironment,
    bindings: &mut ScopedBindings,
    resolved: &mut BTreeMap<ModuleValueNavigationIdentity, ResolvedModuleValueNavigation>,
) {
    if let ExprKind::NamePath {
        segments,
        segment_spans,
    } = &expr.kind
        && !matches!(segments.as_slice(), [name] if bindings.contains(name))
        && let Some((package, module, name)) = environment
            .module_value_dependency_identity_in_owner(segments, owner_module, owner_package)
    {
        let identity = ModuleValueNavigationIdentity {
            package,
            module,
            name,
        };
        if let (Some(target), Some(reference)) = (resolved.get_mut(&identity), segment_spans.last())
        {
            target.references.push(reference.clone());
        }
    }
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_references(
                scrutinee,
                owner_module,
                owner_package,
                environment,
                bindings,
                resolved,
            );
            for arm in arms {
                let binding_count = bindings.len();
                arm.pattern
                    .for_each_binding(&mut |name| bindings.push(name.to_string()));
                collect_expr_references(
                    &arm.expr,
                    owner_module,
                    owner_package,
                    environment,
                    bindings,
                    resolved,
                );
                bindings.truncate(binding_count);
            }
        }
        ExprKind::Begin { body, .. } => {
            let binding_count = bindings.len();
            collect_body_references(
                body,
                owner_module,
                owner_package,
                environment,
                bindings,
                resolved,
            );
            bindings.truncate(binding_count);
        }
        _ => expr.for_each_child(&mut |child| {
            collect_expr_references(
                child,
                owner_module,
                owner_package,
                environment,
                bindings,
                resolved,
            );
        }),
    }
}

fn collect_body_references(
    body: &[BodyLine],
    owner_module: Option<&str>,
    owner_package: Option<&str>,
    environment: &TypeEnvironment,
    bindings: &mut ScopedBindings,
    resolved: &mut BTreeMap<ModuleValueNavigationIdentity, ResolvedModuleValueNavigation>,
) {
    for line in body {
        match &line.kind {
            BodyLineKind::Let { pattern, expr, .. } => {
                collect_expr_references(
                    expr,
                    owner_module,
                    owner_package,
                    environment,
                    bindings,
                    resolved,
                );
                pattern.for_each_binding(&mut |name| bindings.push(name.to_string()));
            }
            BodyLineKind::Expr { expr } => collect_expr_references(
                expr,
                owner_module,
                owner_package,
                environment,
                bindings,
                resolved,
            ),
            BodyLineKind::Defer { body, .. } => {
                let binding_count = bindings.len();
                collect_body_references(
                    body,
                    owner_module,
                    owner_package,
                    environment,
                    bindings,
                    resolved,
                );
                bindings.truncate(binding_count);
            }
        }
    }
}
