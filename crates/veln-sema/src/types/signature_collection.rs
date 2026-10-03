#[cfg(test)]
use std::cell::Cell;
use std::collections::BTreeMap;

use veln_ast::{FunctionKind, HandlerDecl, SurfaceModule, UseDecl, Visibility};

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

#[cfg(test)]
thread_local! {
    static TYPE_CANONICALIZATION_VISITS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_type_canonicalization_visits() {
    TYPE_CANONICALIZATION_VISITS.with(|visits| visits.set(0));
}

#[cfg(test)]
pub(crate) fn take_type_canonicalization_visits() -> usize {
    TYPE_CANONICALIZATION_VISITS.with(|visits| visits.replace(0))
}

fn record_type_canonicalization_visit() {
    #[cfg(test)]
    TYPE_CANONICALIZATION_VISITS.with(|visits| visits.set(visits.get() + 1));
}

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
                callsite: function.callsite.is_some(),
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
    record_type_canonicalization_visit();
    match ty {
        Type::Named { name, args, .. } => {
            if let Some((base_name, variant)) = name.rsplit_once("::")
                && let Some(descriptor) =
                    adts.descriptor_for_type_path(base_name, args.len(), current_module, uses)
                && descriptor
                    .variants
                    .iter()
                    .any(|candidate| candidate.name == variant)
            {
                let args = canonical_variant_arguments(
                    args,
                    uses,
                    quarantined_uses,
                    current_module,
                    effects,
                    adts,
                    companion_effect_access_targets,
                );
                return canonical_variant_refinement(descriptor, args, vec![variant.to_string()]);
            }
            let descriptor = adts.descriptor_for_type_path(&name, args.len(), current_module, uses);
            if args.is_empty() {
                if let Some(annotation_type) = descriptor
                    .and_then(|descriptor| adts.annotation_type_for_descriptor(descriptor))
                {
                    return annotation_type.clone();
                }
                if name == "WallTime" && descriptor.is_none() {
                    return Type::wall_time();
                }
                if name == "SourceLocation" && descriptor.is_none() {
                    return Type::source_location();
                }
            }
            let Some(canonical_name) = descriptor
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
            let args = args
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
                .collect();
            if let Some(descriptor) = descriptor {
                Type::resolved_named(canonical_name, descriptor.identity(), args)
            } else {
                Type::named(canonical_name, args)
            }
        }
        Type::VariantRefinement {
            name,
            args,
            variants,
            unresolved_alternatives,
            ..
        } => {
            let Some(descriptor) =
                adts.descriptor_for_type_path(&name, args.len(), current_module, uses)
            else {
                return Type::Unknown;
            };
            if variants.iter().any(|variant| {
                !descriptor
                    .variants
                    .iter()
                    .any(|candidate| candidate.name == *variant)
            }) {
                return Type::Unknown;
            }
            let canonical_args = canonical_variant_arguments(
                args,
                uses,
                quarantined_uses,
                current_module,
                effects,
                adts,
                companion_effect_access_targets,
            );
            let mut variants = variants;
            for (alternative_name, alternative_args, alternative_variant) in unresolved_alternatives
            {
                let Some(alternative_descriptor) = adts.descriptor_for_type_path(
                    &alternative_name,
                    alternative_args.len(),
                    current_module,
                    uses,
                ) else {
                    return Type::Unknown;
                };
                if alternative_descriptor.identity() != descriptor.identity()
                    || canonical_variant_arguments(
                        alternative_args,
                        uses,
                        quarantined_uses,
                        current_module,
                        effects,
                        adts,
                        companion_effect_access_targets,
                    ) != canonical_args
                    || !alternative_descriptor
                        .variants
                        .iter()
                        .any(|candidate| candidate.name == alternative_variant)
                {
                    return Type::Unknown;
                }
                if !variants.contains(&alternative_variant) {
                    variants.push(alternative_variant);
                }
            }
            canonical_variant_refinement(descriptor, canonical_args, variants)
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

#[allow(clippy::too_many_arguments)]
fn canonical_variant_refinement(
    descriptor: &crate::adt::descriptors::AdtDescriptor,
    args: Vec<Type>,
    variants: Vec<String>,
) -> Type {
    let variants = descriptor
        .variants
        .iter()
        .filter(|candidate| variants.contains(&candidate.name))
        .map(|candidate| candidate.name.clone())
        .collect::<Vec<_>>();
    if variants.len() == descriptor.variants.len() {
        Type::resolved_named(&descriptor.type_name, descriptor.identity(), args)
    } else {
        Type::resolved_variant_refinement(
            &descriptor.type_name,
            descriptor.identity(),
            args,
            variants,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn canonical_variant_arguments(
    args: Vec<Type>,
    uses: &[UseDecl],
    quarantined_uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<Type> {
    args.into_iter()
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
        .collect()
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

pub(super) fn canonicalize_effect_signature_types(
    module: &SurfaceModule,
    effects: &mut [EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) {
    let uses = normal_use_decls(module);
    let quarantined_uses = module
        .uses
        .iter()
        .filter(|use_decl| {
            crate::name_recovery::use_decl_has_invalid_module_segment(module, use_decl)
        })
        .cloned()
        .collect::<Vec<_>>();
    let effect_catalog = effects.to_vec();
    for effect in effects {
        for operation in &mut effect.operations {
            operation.params = std::mem::take(&mut operation.params)
                .into_iter()
                .map(|ty| {
                    canonicalize_type_effects(
                        ty,
                        &uses,
                        &quarantined_uses,
                        effect.module_name.as_deref(),
                        &effect_catalog,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .collect();
            operation.return_type = canonicalize_type_effects(
                std::mem::replace(&mut operation.return_type, Type::Unknown),
                &uses,
                &quarantined_uses,
                effect.module_name.as_deref(),
                &effect_catalog,
                adts,
                companion_effect_access_targets,
            );
        }
    }
}

pub(super) fn handler_signatures(
    module: &SurfaceModule,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
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
    let context = HandlerSignatureContext {
        uses: &uses,
        quarantined_uses: &quarantined_uses,
        effects,
        adts,
        companion_effect_access_targets,
    };
    module
        .handlers
        .iter()
        .filter_map(|handler| context.signature(handler))
        .collect()
}

struct HandlerSignatureContext<'a> {
    uses: &'a [UseDecl],
    quarantined_uses: &'a [UseDecl],
    effects: &'a [EffectSignature],
    adts: &'a AdtRegistry,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
}

impl HandlerSignatureContext<'_> {
    fn signature(&self, handler: &HandlerDecl) -> Option<HandlerSignature> {
        let name = handler.name.clone()?;
        Some(HandlerSignature {
            qualified_name: qualified_handler_name(handler.module_name.as_deref(), &name),
            name,
            module_name: handler.module_name.clone(),
            visibility: handler.visibility,
            params: handler
                .params
                .iter()
                .map(|param| self.canonical_type(param.ty.as_deref(), handler))
                .collect(),
            effect: self.handled_effect(handler),
            effects: self.declared_effects(handler),
            operation_clauses: operation_clause_signatures(handler),
        })
    }

    fn canonical_type(&self, ty: Option<&str>, handler: &HandlerDecl) -> Type {
        canonicalize_type_effects(
            parse_type_or_unknown(ty),
            self.uses,
            self.quarantined_uses,
            handler.module_name.as_deref(),
            self.effects,
            self.adts,
            self.companion_effect_access_targets,
        )
    }

    fn handled_effect(&self, handler: &HandlerDecl) -> String {
        canonical_user_effect_label(
            &handler.effect,
            self.uses,
            handler.module_name.as_deref(),
            self.effects,
            self.companion_effect_access_targets,
        )
        .unwrap_or_else(|| handler.effect.join("::"))
    }

    fn declared_effects(&self, handler: &HandlerDecl) -> Vec<String> {
        canonical_declared_effects(
            handler.effects.clone().unwrap_or_default(),
            self.uses,
            self.quarantined_uses,
            handler.module_name.as_deref(),
            self.effects,
            self.companion_effect_access_targets,
        )
    }
}

fn qualified_handler_name(module_name: Option<&str>, name: &str) -> String {
    module_name.map_or_else(|| name.to_string(), |module| format!("{module}::{name}"))
}

fn operation_clause_signatures(handler: &HandlerDecl) -> Vec<HandlerOperationClauseSignature> {
    handler
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
        .collect()
}
