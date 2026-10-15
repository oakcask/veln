use super::*;

impl TypeEnvironment {
    pub(crate) fn canonicalize_type_annotation(
        &self,
        ty: Type,
        current_module: Option<&str>,
    ) -> Type {
        canonicalize_type_effects(
            ty,
            &self.uses,
            &self.quarantined_uses,
            current_module,
            &self.effects,
            &self.adts,
            &self.companion_effect_access_targets,
        )
    }

    pub(crate) fn variant_refinement_annotation_error(
        &self,
        ty: &Type,
        current_module: Option<&str>,
    ) -> Option<String> {
        let canonical = self.canonicalize_type_annotation(ty.clone(), current_module);
        self.variant_refinement_annotation_error_with_canonical(ty, &canonical, current_module)
    }

    pub(crate) fn variant_refinement_base_failure(
        &self,
        base: &str,
        args_len: usize,
        current_module: Option<&str>,
    ) -> Option<VariantRefinementBaseFailure> {
        if let Some(descriptor) =
            self.variant_refinement_descriptor_identity(base, args_len, current_module)
        {
            if descriptor.type_parameters.len() != args_len {
                return None;
            }
            return self.variant_refinement_descriptor_failure(descriptor);
        }

        let resolved = self
            .resolved_source_less_non_adt_base(base, args_len, current_module)
            .or_else(|| {
                recover_type_case(base).as_deref().and_then(|candidate| {
                    self.resolved_source_less_non_adt_base(candidate, args_len, current_module)
                })
            })?;
        let Type::Named { name, identity, .. } = resolved else {
            return None;
        };
        Some(VariantRefinementBaseFailure {
            reason: "not_adt",
            resolved_identity: identity,
            declaration_span: None,
            related_kind: "base_type_origin",
            provider: Some("type_syntax"),
            related_message: format!(
                "Type `{name}` resolves through the `type_syntax` provider without finite ADT variants."
            ),
        })
    }

    pub(crate) fn variant_refinement_descriptor_identity(
        &self,
        base: &str,
        args_len: usize,
        current_module: Option<&str>,
    ) -> Option<&crate::adt::descriptors::AdtDescriptor> {
        if let Some(descriptor) =
            self.adts
                .unique_descriptor_for_type_path(base, args_len, current_module, &self.uses)
        {
            return Some(descriptor);
        }
        if self
            .adts
            .descriptor_for_type_path(base, args_len, current_module, &self.uses)
            .is_some()
        {
            return None;
        }
        if self
            .adts
            .descriptor_for_type_path_any_arity(base, current_module, &self.uses)
            .is_some()
        {
            return self.adts.unique_descriptor_for_type_path_any_arity(
                base,
                current_module,
                &self.uses,
            );
        }

        let recovered = recover_type_case(base)?;
        self.adts
            .unique_descriptor_for_type_path_any_arity(&recovered, current_module, &self.uses)
    }

    pub(crate) fn variant_refinement_variant_identity<'a>(
        &'a self,
        descriptor: &'a crate::adt::descriptors::AdtDescriptor,
        variant: &str,
    ) -> Option<&'a crate::adt::descriptors::AdtVariantDescriptor> {
        match self.adts.variant_for_descriptor(descriptor, variant) {
            crate::adt::registry::VariantLookup::Found(variant) => Some(variant),
            crate::adt::registry::VariantLookup::Ambiguous => None,
            crate::adt::registry::VariantLookup::Missing => {
                let recovered = recover_type_case(variant)?;
                let mut variants = descriptor
                    .variants
                    .iter()
                    .filter(|candidate| candidate.name == recovered);
                let variant = variants.next()?;
                variants.next().is_none().then_some(variant)
            }
        }
    }

    fn variant_refinement_descriptor_failure(
        &self,
        descriptor: &crate::adt::descriptors::AdtDescriptor,
    ) -> Option<VariantRefinementBaseFailure> {
        match descriptor.refinement_availability {
            crate::adt::descriptors::VariantRefinementAvailability::Finite => None,
            crate::adt::descriptors::VariantRefinementAvailability::Unavailable => {
                let target = self.adts.canonical_descriptor_for(descriptor);
                let declaration_span = self.adts.declaration_span(target).cloned()?;
                Some(VariantRefinementBaseFailure {
                    reason: "not_adt",
                    resolved_identity: target.identity(),
                    declaration_span: Some(declaration_span),
                    related_kind: "base_type_declaration",
                    provider: None,
                    related_message: format!(
                        "Type `{}` is declared here without finite ADT variants.",
                        target.type_name
                    ),
                })
            }
            crate::adt::descriptors::VariantRefinementAvailability::Opaque => {
                Some(VariantRefinementBaseFailure {
                    reason: "opaque",
                    resolved_identity: descriptor.identity(),
                    declaration_span: None,
                    related_kind: "base_type_origin",
                    provider: Some("adt"),
                    related_message: format!(
                        "Type `{}` is provided as opaque and does not expose variant identities.",
                        descriptor.type_name
                    ),
                })
            }
        }
    }

    fn resolved_source_less_non_adt_base(
        &self,
        base: &str,
        args_len: usize,
        current_module: Option<&str>,
    ) -> Option<Type> {
        if self
            .adts
            .descriptor_for_type_path_any_arity(base, current_module, &self.uses)
            .is_some()
        {
            return None;
        }
        let published = crate::source_less_lookup::with_builtin_type_syntax_registry(|registry| {
            registry.arity(base) == Some(args_len)
        })
        .unwrap_or(false);
        if !published {
            return None;
        }
        let resolved = self.canonicalize_type_annotation(
            Type::named(base, vec![Type::Unknown; args_len]),
            current_module,
        );
        matches!(resolved, Type::Named { .. }).then_some(resolved)
    }

    pub(crate) fn non_adt_refinement_base_resolves(
        &self,
        base: &str,
        args_len: usize,
        current_module: Option<&str>,
    ) -> bool {
        self.variant_refinement_base_failure(base, args_len, current_module)
            .is_some()
    }

    pub(crate) fn type_annotation_is_fully_resolved(
        &self,
        ty: &Type,
        current_module: Option<&str>,
        type_parameters: &[String],
    ) -> bool {
        let type_parameters = TypeParameterIdentityContext::new(type_parameters);
        self.type_annotation_is_fully_resolved_with_context(ty, current_module, &type_parameters)
    }

    pub(crate) fn type_annotation_is_fully_resolved_with_context(
        &self,
        ty: &Type,
        current_module: Option<&str>,
        type_parameters: &TypeParameterIdentityContext<'_>,
    ) -> bool {
        match ty {
            Type::Unknown => false,
            Type::Named { name, args, .. } => self.named_type_annotation_is_fully_resolved(
                ty,
                name,
                args,
                current_module,
                type_parameters,
            ),
            Type::VariantRefinement {
                name,
                args,
                unresolved_alternatives,
                ..
            } => self.variant_refinement_is_fully_resolved(
                ty,
                name,
                args,
                unresolved_alternatives,
                current_module,
                type_parameters,
            ),
            Type::Record(fields) => self.type_annotations_are_fully_resolved(
                fields.iter().map(|(_, field)| field),
                current_module,
                type_parameters,
            ),
            Type::Function {
                params,
                variadic,
                return_type,
                ..
            } => self.function_type_annotation_is_fully_resolved(
                params,
                variadic.as_deref(),
                return_type,
                current_module,
                type_parameters,
            ),
        }
    }

    fn named_type_annotation_is_fully_resolved(
        &self,
        ty: &Type,
        name: &str,
        args: &[Type],
        current_module: Option<&str>,
        type_parameters: &TypeParameterIdentityContext<'_>,
    ) -> bool {
        self.type_annotations_are_fully_resolved(args, current_module, type_parameters)
            && (args.is_empty() && type_parameters.ordinal(name).is_some()
                || crate::type_syntax::builtin_type_syntax_arity(name)
                    .is_some_and(|arity| arity == args.len())
                || self
                    .adts
                    .descriptor_for_type_path(name, args.len(), current_module, &self.uses)
                    .is_some()
                || self.is_variant_refinement_candidate(ty, current_module)
                    && self.canonicalize_type_annotation(ty.clone(), current_module)
                        != Type::Unknown)
    }

    fn variant_refinement_is_fully_resolved(
        &self,
        ty: &Type,
        name: &str,
        args: &[Type],
        unresolved_alternatives: &[(String, Vec<Type>, String)],
        current_module: Option<&str>,
        type_parameters: &TypeParameterIdentityContext<'_>,
    ) -> bool {
        self.variant_refinement_annotation_error(ty, current_module)
            .is_none()
            && self
                .adts
                .descriptor_for_type_path(name, args.len(), current_module, &self.uses)
                .is_some()
            && self.type_annotations_are_fully_resolved(args, current_module, type_parameters)
            && unresolved_alternatives
                .iter()
                .all(|(alternative_name, alternative_args, _)| {
                    self.variant_refinement_alternative_is_fully_resolved(
                        alternative_name,
                        alternative_args,
                        current_module,
                        type_parameters,
                    )
                })
    }

    fn variant_refinement_alternative_is_fully_resolved(
        &self,
        name: &str,
        args: &[Type],
        current_module: Option<&str>,
        type_parameters: &TypeParameterIdentityContext<'_>,
    ) -> bool {
        self.adts
            .descriptor_for_type_path(name, args.len(), current_module, &self.uses)
            .is_some()
            && self.type_annotations_are_fully_resolved(args, current_module, type_parameters)
    }

    fn function_type_annotation_is_fully_resolved(
        &self,
        params: &[Type],
        variadic: Option<&Type>,
        return_type: &Type,
        current_module: Option<&str>,
        type_parameters: &TypeParameterIdentityContext<'_>,
    ) -> bool {
        self.type_annotations_are_fully_resolved(params, current_module, type_parameters)
            && variadic.is_none_or(|variadic| {
                self.type_annotation_is_fully_resolved_with_context(
                    variadic,
                    current_module,
                    type_parameters,
                )
            })
            && self.type_annotation_is_fully_resolved_with_context(
                return_type,
                current_module,
                type_parameters,
            )
    }

    fn type_annotations_are_fully_resolved<'a>(
        &self,
        types: impl IntoIterator<Item = &'a Type>,
        current_module: Option<&str>,
        type_parameters: &TypeParameterIdentityContext<'_>,
    ) -> bool {
        types.into_iter().all(|ty| {
            self.type_annotation_is_fully_resolved_with_context(ty, current_module, type_parameters)
        })
    }

    pub(crate) fn recovered_variant_refinement_candidate(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> bool {
        if segments.len() < 2 {
            return false;
        }
        let base = segments[..segments.len() - 1].join("::");
        if self
            .adts
            .descriptor_for_type_path_any_arity(&base, current_module, &self.uses)
            .is_some()
        {
            return false;
        }
        let Some(recovered) = recover_type_case(&base) else {
            return false;
        };
        self.variant_refinement_base_name_resolves(&recovered, current_module)
    }

    pub(crate) fn recovered_variant_refinement_path_resolves(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> bool {
        if segments.len() < 2 {
            return false;
        }
        if matches!(
            self.adts.constructor(segments, current_module, &self.uses),
            crate::adt::registry::ConstructorLookup::Found(_)
        ) {
            return false;
        }
        let variant_index = segments.len() - 1;
        let base = segments[..variant_index].join("::");
        let Some(descriptor) =
            self.variant_refinement_descriptor_identity(&base, 0, current_module)
        else {
            return false;
        };
        self.variant_refinement_variant_identity(descriptor, &segments[variant_index])
            .is_some()
    }

    pub(crate) fn recovered_variant_refinement_base_resolves(
        &self,
        base: &str,
        current_module: Option<&str>,
    ) -> bool {
        recover_type_case(base).is_some_and(|recovered| {
            self.variant_refinement_base_name_resolves(&recovered, current_module)
        })
    }

    fn variant_refinement_base_name_resolves(
        &self,
        base: &str,
        current_module: Option<&str>,
    ) -> bool {
        if self
            .adts
            .descriptor_for_type_path_any_arity(base, current_module, &self.uses)
            .is_some()
        {
            return self
                .adts
                .unique_descriptor_for_type_path_any_arity(base, current_module, &self.uses)
                .is_some();
        }
        crate::source_less_lookup::with_builtin_type_syntax_registry(|registry| {
            registry.arity(base).is_some()
        })
        .unwrap_or(false)
            || matches!(base, "WallTime" | "SourceLocation")
    }

    pub(crate) fn recovered_variant_refinement_base_failure(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<VariantRefinementBaseFailure> {
        self.recovered_variant_refinement_candidate(segments, current_module)
            .then(|| {
                let base = segments[..segments.len() - 1].join("::");
                self.variant_refinement_base_failure(&base, 0, current_module)
            })
            .flatten()
    }

    pub(crate) fn recovered_variant_refinement_annotation_error(
        &self,
        paths: &[veln_ast::TypePathSegments],
        current_module: Option<&str>,
    ) -> Option<String> {
        paths.iter().find_map(|path| {
            if !self.recovered_variant_refinement_candidate(&path.segments, current_module) {
                return None;
            }
            let base = path.segments[..path.segments.len() - 1].join("::");
            let recovered = recover_type_case(&base)?;
            let descriptor = self.adts.unique_descriptor_for_type_path_any_arity(
                &recovered,
                current_module,
                &self.uses,
            )?;
            if !descriptor.type_parameters.is_empty() {
                return Some(format!(
                    "`{recovered}` expects {} type argument(s), found 0",
                    descriptor.type_parameters.len()
                ));
            }
            if descriptor.refinement_availability
                != crate::adt::descriptors::VariantRefinementAvailability::Finite
            {
                return None;
            }
            let variant = path.segments.last()?.clone();
            self.variant_refinement_annotation_error(
                &Type::variant_refinement(recovered, Vec::new(), vec![variant]),
                current_module,
            )
        })
    }

    fn variant_refinement_annotation_error_with_canonical(
        &self,
        ty: &Type,
        canonical: &Type,
        current_module: Option<&str>,
    ) -> Option<String> {
        if let Some(error) = self.variant_refinement_arity_error(ty, current_module) {
            return Some(error);
        }
        if self.is_variant_refinement_candidate(ty, current_module) && canonical == &Type::Unknown {
            return Some(
                "variant refinement alternatives must resolve to declared variants of one ADT"
                    .to_string(),
            );
        }
        if canonical == &Type::Unknown {
            return None;
        }
        type_children(ty)
            .into_iter()
            .zip(type_children(canonical))
            .find_map(|(child, canonical_child)| {
                self.variant_refinement_annotation_error_with_canonical(
                    child,
                    canonical_child,
                    current_module,
                )
            })
    }

    pub(crate) fn variant_refinement_arity_error(
        &self,
        ty: &Type,
        current_module: Option<&str>,
    ) -> Option<String> {
        let (name, args) = match ty {
            Type::Named { name, args, .. } | Type::VariantRefinement { name, args, .. } => {
                (name, args)
            }
            _ => return None,
        };
        let base = name
            .rsplit_once("::")
            .map(|(base, _)| base)
            .or_else(|| matches!(ty, Type::VariantRefinement { .. }).then_some(name.as_str()))?;
        self.type_path_arity_error(base, args.len(), current_module)
    }

    pub(crate) fn type_path_arity_error(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
    ) -> Option<String> {
        let expected = self.resolved_type_path_arity(name, current_module)?;
        if expected == args_len {
            return None;
        }
        Some(format!(
            "`{name}` expects {expected} type argument(s), found {args_len}"
        ))
    }

    fn resolved_type_path_arity(&self, name: &str, current_module: Option<&str>) -> Option<usize> {
        if let Some(descriptor) =
            self.adts
                .unique_descriptor_for_type_path_any_arity(name, current_module, &self.uses)
        {
            return Some(descriptor.type_parameters.len());
        }
        if self
            .adts
            .descriptor_for_type_path_any_arity(name, current_module, &self.uses)
            .is_some()
        {
            return None;
        }
        if let Some(arity) = crate::type_syntax::builtin_type_syntax_arity(name) {
            return Some(arity);
        }

        let recovered = recover_type_case(name)?;
        if let Some(descriptor) = self.adts.unique_descriptor_for_type_path_any_arity(
            &recovered,
            current_module,
            &self.uses,
        ) {
            return Some(descriptor.type_parameters.len());
        }
        if self
            .adts
            .descriptor_for_type_path_any_arity(&recovered, current_module, &self.uses)
            .is_some()
        {
            return None;
        }
        crate::type_syntax::builtin_type_syntax_arity(&recovered)
    }

    fn is_variant_refinement_candidate(&self, ty: &Type, current_module: Option<&str>) -> bool {
        match ty {
            Type::VariantRefinement { .. } => true,
            Type::Named { name, args, .. } => name.rsplit_once("::").is_some_and(|(base, _)| {
                self.adts
                    .descriptor_for_type_path(base, args.len(), current_module, &self.uses)
                    .is_some()
            }),
            _ => false,
        }
    }
}

fn recover_type_case(name: &str) -> Option<String> {
    let (prefix, leaf) = name.rsplit_once("::").unwrap_or(("", name));
    let mut chars = leaf.chars();
    let first = chars.next()?;
    if !first.is_ascii_lowercase() {
        return None;
    }
    let recovered_leaf = format!("{}{}", first.to_ascii_uppercase(), chars.as_str());
    Some(if prefix.is_empty() {
        recovered_leaf
    } else {
        format!("{prefix}::{recovered_leaf}")
    })
}

fn type_children(ty: &Type) -> Vec<&Type> {
    match ty {
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => args.iter().collect(),
        Type::Record(fields) => fields.iter().map(|(_, ty)| ty).collect(),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => params
            .iter()
            .chain(variadic.iter().map(Box::as_ref))
            .chain(std::iter::once(return_type.as_ref()))
            .collect(),
        Type::Unknown => Vec::new(),
    }
}
