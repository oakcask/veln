use std::collections::BTreeMap;

use veln_ast::{BodyLine, BodyLineKind, Expr, ExprKind, SurfaceModule, Visibility};
use veln_source::SourceSpan;

use crate::types::TypeEnvironment;

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

pub fn resolved_module_value_navigation(
    module: &SurfaceModule,
) -> Vec<ResolvedModuleValueNavigation> {
    let mut declarations = BTreeMap::<ModuleValueNavigationIdentity, Vec<_>>::new();
    for value in &module.module_values {
        let (Some(name), Some(declaration)) = (&value.name, &value.name_span) else {
            continue;
        };
        declarations
            .entry(ModuleValueNavigationIdentity {
                package: value.package_name.clone(),
                module: value.module_name.clone(),
                name: name.clone(),
            })
            .or_default()
            .push((declaration.clone(), value.visibility));
    }
    let mut resolved = declarations
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
        .collect::<BTreeMap<_, _>>();
    let environment = TypeEnvironment::from_module(module);

    for value in &module.module_values {
        collect_expr_references(
            &value.initializer,
            value.module_name.as_deref(),
            value.package_name.as_deref(),
            &environment,
            &mut Vec::new(),
            &mut resolved,
        );
    }
    for function in &module.functions {
        let mut bindings = function
            .params
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<Vec<_>>();
        if let Some(binding) = &function.return_binding {
            bindings.push(binding.name.clone());
        }
        collect_body_references(
            &function.body,
            function.module_name.as_deref(),
            function.package_name.as_deref(),
            &environment,
            &mut bindings,
            &mut resolved,
        );
    }
    for handler in &module.handlers {
        let handler_bindings = handler
            .params
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<Vec<_>>();
        for clause in &handler.operation_clauses {
            let mut bindings = handler_bindings.clone();
            bindings.extend(clause.params.iter().map(|parameter| parameter.name.clone()));
            collect_expr_references(
                &clause.body,
                handler.module_name.as_deref(),
                handler.package_name.as_deref(),
                &environment,
                &mut bindings,
                &mut resolved,
            );
        }
    }

    resolved.into_values().collect()
}

fn collect_expr_references(
    expr: &Expr,
    owner_module: Option<&str>,
    owner_package: Option<&str>,
    environment: &TypeEnvironment,
    bindings: &mut Vec<String>,
    resolved: &mut BTreeMap<ModuleValueNavigationIdentity, ResolvedModuleValueNavigation>,
) {
    if let ExprKind::NamePath {
        segments,
        segment_spans,
    } = &expr.kind
        && !matches!(segments.as_slice(), [name] if bindings.iter().rev().any(|binding| binding == name))
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
    bindings: &mut Vec<String>,
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
