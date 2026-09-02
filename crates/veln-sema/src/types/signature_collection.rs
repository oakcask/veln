use std::collections::BTreeMap;

use veln_ast::{FunctionKind, SurfaceModule, UseDecl, Visibility};

use crate::adt::registry::AdtRegistry;
use crate::name_recovery::normal_use_decls;
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_or_unknown;

use super::effect_call_resolution::push_unique_effect;
use super::effect_inference::{canonical_user_effect_label, quarantined_public_user_effect_label};
use super::private_inference::function_signature_params;
use super::signatures::{
    CompanionAccessTarget, EffectOperationSignature, EffectSignature, FunctionSignature,
    HandlerOperationClauseSignature, HandlerSignature, synthetic_handler_clause_function_name,
};
use super::symbols::imported_use_for_path;

pub(super) fn ordinary_function_signatures(
    module: &SurfaceModule,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<FunctionSignature> {
    let uses = normal_use_decls(module);
    let quarantined_uses = module
        .uses
        .iter()
        .filter(|use_decl| {
            crate::name_recovery::use_decl_has_invalid_module_segment(module, use_decl)
        })
        .cloned()
        .collect::<Vec<_>>();
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            let name = function.name.clone()?;
            if !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase) {
                return None;
            }
            let (params, variadic) = function_signature_params(function);
            let params = params
                .into_iter()
                .map(|ty| {
                    canonicalize_type_effects(
                        ty,
                        &uses,
                        &quarantined_uses,
                        function.module_name.as_deref(),
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .collect();
            let variadic = variadic.map(|ty| {
                canonicalize_type_effects(
                    ty,
                    &uses,
                    &quarantined_uses,
                    function.module_name.as_deref(),
                    effects,
                    adts,
                    companion_effect_access_targets,
                )
            });
            let return_type = canonicalize_type_effects(
                parse_type_or_unknown(function.return_type.as_deref()),
                &uses,
                &quarantined_uses,
                function.module_name.as_deref(),
                effects,
                adts,
                companion_effect_access_targets,
            );
            Some(FunctionSignature {
                target_name: crate::standard_symbols::standard_function_link_name(
                    function.module_name.as_deref(),
                    &name,
                ),
                name,
                module_name: function.module_name.clone(),
                visibility: function.visibility,
                params,
                variadic,
                return_type,
                effects: canonical_declared_effects(
                    function.effects.clone().unwrap_or_default(),
                    &uses,
                    &quarantined_uses,
                    function.module_name.as_deref(),
                    effects,
                    companion_effect_access_targets,
                ),
                node_id: function.node_id,
                span: function.span.clone(),
            })
        })
        .collect()
}

pub(super) fn canonicalize_type_effects(
    ty: Type,
    uses: &[UseDecl],
    quarantined_uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Type {
    match ty {
        Type::Named { name, args } => {
            let Some(canonical_name) = adts
                .descriptor_for_type_path(&name, args.len(), current_module, uses)
                .map(|descriptor| descriptor.type_name.clone())
                .or_else(|| {
                    canonical_type_name_without_descriptor(
                        &name,
                        current_module,
                        uses,
                        quarantined_uses,
                        args.len(),
                        adts,
                    )
                })
            else {
                return Type::Unknown;
            };
            Type::Named {
                name: canonical_name,
                args: args
                    .into_iter()
                    .map(|arg| {
                        canonicalize_type_effects(
                            arg,
                            uses,
                            quarantined_uses,
                            current_module,
                            effects,
                            adts,
                            companion_effect_access_targets,
                        )
                    })
                    .collect(),
            }
        }
        Type::Record(fields) => Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| {
                    (
                        name,
                        canonicalize_type_effects(
                            ty,
                            uses,
                            quarantined_uses,
                            current_module,
                            effects,
                            adts,
                            companion_effect_access_targets,
                        ),
                    )
                })
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects: declared,
        } => Type::Function {
            params: params
                .into_iter()
                .map(|param| {
                    canonicalize_type_effects(
                        param,
                        uses,
                        quarantined_uses,
                        current_module,
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .collect(),
            variadic: variadic
                .map(|ty| {
                    canonicalize_type_effects(
                        *ty,
                        uses,
                        quarantined_uses,
                        current_module,
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .map(Box::new),
            return_type: Box::new(canonicalize_type_effects(
                *return_type,
                uses,
                quarantined_uses,
                current_module,
                effects,
                adts,
                companion_effect_access_targets,
            )),
            effects: canonical_declared_effects(
                declared,
                uses,
                quarantined_uses,
                current_module,
                effects,
                companion_effect_access_targets,
            ),
        },
        Type::Unknown => Type::Unknown,
    }
}

fn canonical_type_name_without_descriptor(
    name: &str,
    current_module: Option<&str>,
    uses: &[UseDecl],
    quarantined_uses: &[UseDecl],
    args_len: usize,
    adts: &AdtRegistry,
) -> Option<String> {
    if !name.contains("::") {
        return Some(name.to_string());
    }
    let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
    match segments.as_slice() {
        [_, .., _] => {
            if imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .is_some()
            {
                return Some(name.to_string());
            }
            let use_decl = imported_use_for_path(
                quarantined_uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            adts.descriptor_for_type_path(name, args_len, current_module, quarantined_uses)
                .filter(|descriptor| {
                    descriptor.module_name.as_deref() == Some(use_decl.name.as_str())
                        && descriptor.visibility == Visibility::Public
                })
                .map_or_else(
                    || (!use_decl.name.contains("::")).then(|| name.to_string()),
                    |_| None,
                )
        }
        _ => Some(name.to_string()),
    }
}

fn canonical_declared_effects(
    declared: Vec<String>,
    uses: &[UseDecl],
    quarantined_uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<String> {
    let mut canonical = Vec::new();
    for effect in declared {
        if effect.starts_with("...") {
            push_unique_effect(&mut canonical, &effect);
            continue;
        }
        let segments = effect.split("::").map(str::to_string).collect::<Vec<_>>();
        let label = canonical_user_effect_label(
            &segments,
            uses,
            current_module,
            effects,
            companion_effect_access_targets,
        )
        .unwrap_or(effect);
        if quarantined_public_user_effect_label(
            &segments,
            quarantined_uses,
            current_module,
            effects,
        )
        .is_some()
        {
            continue;
        }
        push_unique_effect(&mut canonical, &label);
    }
    canonical
}

pub(super) fn effect_signatures(module: &SurfaceModule) -> Vec<EffectSignature> {
    module
        .effects
        .iter()
        .filter_map(|effect| {
            let name = effect.name.clone()?;
            let qualified_name = if let Some(module_name) = &effect.module_name {
                format!("{module_name}::{name}")
            } else {
                name.clone()
            };
            Some(EffectSignature {
                name,
                qualified_name,
                module_name: effect.module_name.clone(),
                visibility: effect.visibility,
                span: effect.span.clone(),
                operations: effect
                    .operations
                    .iter()
                    .filter_map(|operation| {
                        Some(EffectOperationSignature {
                            name: operation.name.clone()?,
                            params: operation
                                .params
                                .iter()
                                .map(|param| parse_type_or_unknown(param.ty.as_deref()))
                                .collect(),
                            return_type: parse_type_or_unknown(operation.return_type.as_deref()),
                            node_id: operation.node_id,
                            name_span: operation.name_span.clone(),
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}

pub(super) fn handler_signatures(
    module: &SurfaceModule,
    effects: &[EffectSignature],
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<HandlerSignature> {
    let uses = normal_use_decls(module);
    let quarantined_uses = module
        .uses
        .iter()
        .filter(|use_decl| {
            crate::name_recovery::use_decl_has_invalid_module_segment(module, use_decl)
        })
        .cloned()
        .collect::<Vec<_>>();
    module
        .handlers
        .iter()
        .filter_map(|handler| {
            let name = handler.name.clone()?;
            let qualified_name = if let Some(module_name) = &handler.module_name {
                format!("{module_name}::{name}")
            } else {
                name.clone()
            };
            let effect = canonical_user_effect_label(
                &handler.effect,
                &uses,
                handler.module_name.as_deref(),
                effects,
                companion_effect_access_targets,
            )
            .unwrap_or_else(|| handler.effect.join("::"));
            Some(HandlerSignature {
                name,
                qualified_name,
                module_name: handler.module_name.clone(),
                visibility: handler.visibility,
                params: handler
                    .params
                    .iter()
                    .map(|param| parse_type_or_unknown(param.ty.as_deref()))
                    .collect(),
                effect,
                effects: canonical_declared_effects(
                    handler.effects.clone().unwrap_or_default(),
                    &uses,
                    &quarantined_uses,
                    handler.module_name.as_deref(),
                    effects,
                    companion_effect_access_targets,
                ),
                operation_clauses: handler
                    .operation_clauses
                    .iter()
                    .filter_map(|clause| {
                        Some(HandlerOperationClauseSignature {
                            operation: clause.operation.clone()?,
                            function: synthetic_handler_clause_function_name(
                                handler.name.as_deref().unwrap_or("missing"),
                                clause.operation.as_deref().unwrap_or("missing"),
                            ),
                            module_name: handler.module_name.clone(),
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}
