#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};

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
    static VARIANT_CANONICALIZATION_LOOKUPS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_type_canonicalization_visits() {
    TYPE_CANONICALIZATION_VISITS.with(|visits| visits.set(0));
}

#[cfg(test)]
pub(crate) fn take_type_canonicalization_visits() -> usize {
    TYPE_CANONICALIZATION_VISITS.with(|visits| visits.replace(0))
}

#[cfg(test)]
pub(crate) fn reset_variant_canonicalization_lookups() {
    VARIANT_CANONICALIZATION_LOOKUPS.with(|lookups| lookups.set(0));
}

#[cfg(test)]
pub(crate) fn take_variant_canonicalization_lookups() -> usize {
    VARIANT_CANONICALIZATION_LOOKUPS.with(|lookups| lookups.replace(0))
}

fn record_type_canonicalization_visit() {
    #[cfg(test)]
    TYPE_CANONICALIZATION_VISITS.with(|visits| visits.set(visits.get() + 1));
}

fn record_variant_canonicalization_lookup() {
    #[cfg(test)]
    VARIANT_CANONICALIZATION_LOOKUPS.with(|lookups| lookups.set(lookups.get() + 1));
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

pub(crate) fn canonicalize_type_effects(
    ty: Type,
    uses: &[UseDecl],
    quarantined_uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Type {
    TypeCanonicalizer {
        uses,
        quarantined_uses,
        current_module,
        effects,
        adts,
        companion_effect_access_targets,
    }
    .canonicalize(ty)
}

struct TypeCanonicalizer<'a> {
    uses: &'a [UseDecl],
    quarantined_uses: &'a [UseDecl],
    current_module: Option<&'a str>,
    effects: &'a [EffectSignature],
    adts: &'a AdtRegistry,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
}

impl TypeCanonicalizer<'_> {
    fn canonicalize(&self, ty: Type) -> Type {
        record_type_canonicalization_visit();
        match ty {
            Type::Named { name, args, .. } => self.canonicalize_named(name, args),
            Type::VariantRefinement {
                name,
                args,
                variants,
                unresolved_alternatives,
                ..
            } => {
                self.canonicalize_refinement(name, args, variants.to_vec(), unresolved_alternatives)
            }
            Type::Record(fields) => Type::Record(
                fields
                    .into_iter()
                    .map(|(name, ty)| (name, self.canonicalize(ty)))
                    .collect(),
            ),
            Type::Function {
                params,
                variadic,
                return_type,
                effects,
            } => self.canonicalize_function(params, variadic, *return_type, effects),
            Type::Unknown => Type::Unknown,
        }
    }

    fn canonicalize_named(&self, name: String, args: Vec<Type>) -> Type {
        if let Some(refinement) = self.canonicalize_named_variant(&name, &args) {
            return refinement;
        }
        let descriptor =
            self.adts
                .descriptor_for_type_path(&name, args.len(), self.current_module, self.uses);
        if args.is_empty()
            && let Some(annotation_type) = descriptor
                .and_then(|descriptor| self.adts.annotation_type_for_descriptor(descriptor))
        {
            return annotation_type.clone();
        }
        if args.is_empty() && descriptor.is_none() {
            if name == "WallTime" {
                return Type::wall_time();
            }
            if name == "SourceLocation" {
                return Type::source_location();
            }
        }
        let Some(canonical_name) = descriptor
            .map(|descriptor| descriptor.type_name.clone())
            .or_else(|| {
                canonical_type_name_without_descriptor(
                    &name,
                    self.current_module,
                    self.uses,
                    self.quarantined_uses,
                    args.len(),
                    self.adts,
                )
            })
        else {
            return Type::Unknown;
        };
        let args = self.canonicalize_args(args);
        if let Some(descriptor) = descriptor {
            Type::resolved_named(canonical_name, descriptor.identity(), args)
        } else {
            Type::named(canonical_name, args)
        }
    }

    fn canonicalize_named_variant(&self, name: &str, args: &[Type]) -> Option<Type> {
        let (base_name, variant) = name.rsplit_once("::")?;
        let descriptor = self.adts.descriptor_for_type_path(
            base_name,
            args.len(),
            self.current_module,
            self.uses,
        )?;
        let declaration_order = self
            .adts
            .variant_declaration_order_for_descriptor(descriptor)?;
        record_variant_canonicalization_lookup();
        declaration_order.rank(variant)?;
        let canonical_args = self.canonicalize_args(args.to_vec());
        if declaration_order.len() == 1 {
            return Some(Type::resolved_named(
                &descriptor.type_name,
                descriptor.identity(),
                canonical_args,
            ));
        }
        Some(Type::resolved_variant_refinement(
            &descriptor.type_name,
            descriptor.identity(),
            canonical_args,
            vec![variant.to_string()],
        ))
    }

    fn canonicalize_refinement(
        &self,
        name: String,
        args: Vec<Type>,
        variants: Vec<String>,
        unresolved_alternatives: Vec<(String, Vec<Type>, String)>,
    ) -> Type {
        let Some(descriptor) =
            self.adts
                .descriptor_for_type_path(&name, args.len(), self.current_module, self.uses)
        else {
            return Type::Unknown;
        };
        let Some(declaration_order) = self
            .adts
            .variant_declaration_order_for_descriptor(descriptor)
        else {
            return Type::Unknown;
        };
        if variants.iter().any(|variant| {
            crate::type_relations::record_variant_set_lookup();
            record_variant_canonicalization_lookup();
            declaration_order.rank(variant).is_none()
        }) {
            return Type::Unknown;
        }
        let canonical_args = self.canonicalize_args(args);
        let Some(variants) = self.resolve_refinement_alternatives(
            descriptor,
            canonical_args.as_slice(),
            variants,
            unresolved_alternatives,
            &declaration_order,
        ) else {
            return Type::Unknown;
        };
        canonical_variant_refinement(descriptor, &declaration_order, canonical_args, variants)
    }

    fn resolve_refinement_alternatives(
        &self,
        descriptor: &crate::adt::descriptors::AdtDescriptor,
        canonical_args: &[Type],
        mut variants: Vec<String>,
        unresolved_alternatives: Vec<(String, Vec<Type>, String)>,
        declaration_order: &crate::adt::registry::VariantDeclarationOrder,
    ) -> Option<Vec<String>> {
        let mut selected_variants = variants.iter().cloned().collect::<HashSet<_>>();
        for (name, args, variant) in unresolved_alternatives {
            let alternative_descriptor = self.adts.descriptor_for_type_path(
                &name,
                args.len(),
                self.current_module,
                self.uses,
            )?;
            crate::type_relations::record_variant_set_lookup();
            record_variant_canonicalization_lookup();
            if alternative_descriptor.identity() != descriptor.identity()
                || self.canonicalize_args(args) != canonical_args
                || declaration_order.rank(&variant).is_none()
            {
                return None;
            }
            crate::type_relations::record_variant_set_lookup();
            if selected_variants.insert(variant.clone()) {
                variants.push(variant);
            }
        }
        Some(variants)
    }

    fn canonicalize_function(
        &self,
        params: Vec<Type>,
        variadic: Option<Box<Type>>,
        return_type: Type,
        effects: Vec<String>,
    ) -> Type {
        Type::Function {
            params: self.canonicalize_args(params),
            variadic: variadic.map(|ty| Box::new(self.canonicalize(*ty))),
            return_type: Box::new(self.canonicalize(return_type)),
            effects: canonical_declared_effects(
                effects,
                self.uses,
                self.quarantined_uses,
                self.current_module,
                self.effects,
                self.companion_effect_access_targets,
            ),
        }
    }

    fn canonicalize_args(&self, args: Vec<Type>) -> Vec<Type> {
        args.into_iter().map(|arg| self.canonicalize(arg)).collect()
    }
}

#[allow(clippy::too_many_arguments)]
fn canonical_variant_refinement(
    descriptor: &crate::adt::descriptors::AdtDescriptor,
    declaration_order: &crate::adt::registry::VariantDeclarationOrder,
    args: Vec<Type>,
    variants: Vec<String>,
) -> Type {
    let mut requested = HashSet::with_capacity(variants.len());
    let mut ranked_variants = variants
        .into_iter()
        .filter_map(|variant| {
            if !requested.insert(variant.clone()) {
                return None;
            }
            crate::type_relations::record_variant_set_lookup();
            record_variant_canonicalization_lookup();
            declaration_order.rank(&variant).map(|rank| (rank, variant))
        })
        .collect::<Vec<_>>();
    ranked_variants.sort_unstable_by_key(|(rank, _)| *rank);
    if ranked_variants.len() == declaration_order.len() {
        Type::resolved_named(&descriptor.type_name, descriptor.identity(), args)
    } else {
        Type::resolved_variant_refinement(
            &descriptor.type_name,
            descriptor.identity(),
            args,
            ranked_variants
                .into_iter()
                .map(|(_, variant)| variant)
                .collect(),
        )
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
