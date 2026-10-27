use super::*;

impl AdtRegistry {
    pub(crate) fn constructor_for_core_type(
        &self,
        segments: &[String],
        ty: &CoreType,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<AdtConstructor<'_>> {
        let descriptor = self.descriptor_for_core_type(ty)?;
        self.constructor_for_descriptor(segments, descriptor, current_module, uses)
    }

    pub(crate) fn constructor_for_expected_type(
        &self,
        segments: &[String],
        expected: Option<&CoreType>,
        current_module: Option<&str>,
        uses: &[UseDecl],
        shape: ConstructorShape,
    ) -> ConstructorLookup<'_> {
        let expected_constructor = || {
            expected
                .and_then(|ty| self.constructor_for_core_type(segments, ty, current_module, uses))
                .filter(|constructor| shape.matches(*constructor))
        };
        if segments.len() == 1
            && let Some(constructor) = expected_constructor()
        {
            return ConstructorLookup::Found(constructor);
        }

        match self.constructor(segments, current_module, uses) {
            ConstructorLookup::Found(constructor) if shape.matches(constructor) => {
                ConstructorLookup::Found(constructor)
            }
            ConstructorLookup::Ambiguous => expected_constructor()
                .map(ConstructorLookup::Found)
                .unwrap_or(ConstructorLookup::Ambiguous),
            ConstructorLookup::Found(_) | ConstructorLookup::Missing => ConstructorLookup::Missing,
        }
    }

    pub(crate) fn constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> ConstructorLookup<'_> {
        self.lookup_constructor(segments, current_module, uses, true)
    }

    pub(crate) fn constructor_candidates(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Vec<AdtConstructor<'_>> {
        self.lookup_constructor_candidates(segments, current_module, uses, true)
    }

    pub(crate) fn public_constructor_in_module(
        &self,
        package_name: Option<&str>,
        module_name: &str,
        segments: &[String],
    ) -> ConstructorLookup<'_> {
        let (type_name, variant_name) = match segments {
            [variant_name] => (None, variant_name),
            [type_name, variant_name] => (Some(type_name), variant_name),
            _ => return ConstructorLookup::Missing,
        };
        let mut matches = self
            .variants_named(variant_name)
            .filter(|(descriptor, variant)| {
                descriptor.module_name.as_deref() == Some(module_name)
                    && descriptor.package_name.as_deref() == package_name
                    && descriptor.visibility == Visibility::Public
                    && variant.visibility == Visibility::Public
                    && type_name.is_none_or(|type_name| type_name == &descriptor.type_name)
            })
            .map(|(descriptor, variant)| AdtConstructor {
                descriptor,
                variant,
            })
            .collect::<Vec<_>>();
        deduplicate_nominal_constructors(&mut matches);
        match matches.as_slice() {
            [constructor] => ConstructorLookup::Found(*constructor),
            [] => ConstructorLookup::Missing,
            _ => ConstructorLookup::Ambiguous,
        }
    }

    pub(crate) fn nullary_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> ConstructorLookup<'_> {
        match self.constructor(segments, current_module, uses) {
            ConstructorLookup::Found(constructor)
                if constructor.variant.payload_fields.is_empty() =>
            {
                ConstructorLookup::Found(constructor)
            }
            ConstructorLookup::Found(_) => ConstructorLookup::Missing,
            other => other,
        }
    }

    pub(crate) fn constructor_for_descriptor(
        &self,
        segments: &[String],
        descriptor: &AdtDescriptor,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<AdtConstructor<'_>> {
        match self.constructor(segments, current_module, uses) {
            ConstructorLookup::Found(constructor)
                if same_descriptor(constructor.descriptor, descriptor) =>
            {
                return Some(constructor);
            }
            ConstructorLookup::Found(_) if segments.len() == 1 => {}
            ConstructorLookup::Ambiguous => {}
            _ => return None,
        }

        let mut matches = Vec::new();
        let name = segments.last()?;
        for (candidate, variant) in self.variants_named(name) {
            if !same_descriptor(candidate, descriptor)
                || !self.descriptor_visible(candidate, segments, current_module, uses, true)
            {
                continue;
            }
            if constructor_matches_visible_path(candidate, variant, segments, uses, current_module)
                && self.variant_visible(candidate, variant, current_module, uses, segments)
            {
                matches.push(AdtConstructor {
                    descriptor: candidate,
                    variant,
                });
            }
        }
        match matches.as_slice() {
            [constructor] => Some(*constructor),
            _ => None,
        }
    }

    fn lookup_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        include_imports: bool,
    ) -> ConstructorLookup<'_> {
        if include_imports
            && segments.len() >= 3
            && matches!(
                self.qualified_source_identity(
                    &segments[segments.len() - 2],
                    &segments[..segments.len() - 1],
                    current_module,
                    uses,
                ),
                QualifiedSourceIdentity::Ambiguous
            )
        {
            return ConstructorLookup::Ambiguous;
        }
        let mut matches =
            self.lookup_constructor_candidates(segments, current_module, uses, include_imports);
        prefer_current_module_constructors(&mut matches, segments, current_module, uses);
        match matches.as_slice() {
            [] => ConstructorLookup::Missing,
            [constructor] => ConstructorLookup::Found(*constructor),
            _ => ConstructorLookup::Ambiguous,
        }
    }

    fn lookup_constructor_candidates(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        include_imports: bool,
    ) -> Vec<AdtConstructor<'_>> {
        let mut matches = Vec::new();
        let Some(name) = segments.last() else {
            return matches;
        };
        for (descriptor, variant) in self.variants_named(name) {
            #[cfg(test)]
            constructor_lookup_counters::record_candidate_scan();
            if !self.descriptor_visible(descriptor, segments, current_module, uses, include_imports)
            {
                continue;
            }
            if constructor_matches_visible_path(descriptor, variant, segments, uses, current_module)
                && self.variant_visible(descriptor, variant, current_module, uses, segments)
            {
                matches.push(AdtConstructor {
                    descriptor,
                    variant,
                });
            }
        }
        deduplicate_nominal_constructors(&mut matches);
        matches
    }
}

fn deduplicate_nominal_constructors(constructors: &mut Vec<AdtConstructor<'_>>) {
    let mut unique = Vec::<AdtConstructor<'_>>::with_capacity(constructors.len());
    for constructor in constructors.drain(..) {
        let duplicate = unique.iter_mut().find(|candidate| {
            candidate.descriptor.identity() == constructor.descriptor.identity()
                && candidate.variant.name == constructor.variant.name
        });
        match duplicate {
            Some(candidate)
                if candidate.descriptor.nominal_identity.is_some()
                    && constructor.descriptor.nominal_identity.is_none() =>
            {
                *candidate = constructor;
            }
            Some(_) => {}
            None => unique.push(constructor),
        }
    }
    *constructors = unique;
}

fn prefer_current_module_constructors<'a>(
    matches: &mut Vec<AdtConstructor<'a>>,
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
) {
    if segments.len() != 1 {
        return;
    }
    let Some(current_module) = current_module else {
        return;
    };
    let current_package =
        import_owner_for_module(uses, Some(current_module)).and_then(|owner| owner.package);
    if !matches.iter().any(|constructor| {
        constructor.descriptor.module_name.as_deref() == Some(current_module)
            && constructor.descriptor.package_name == current_package
    }) {
        return;
    }
    matches.retain(|constructor| {
        constructor.descriptor.module_name.as_deref() == Some(current_module)
            && constructor.descriptor.package_name == current_package
    });
}
