impl SymbolIndex {
    fn variant_refinement_identity<'a>(
        &self,
        file: &'a IndexedFile,
        token: &Token,
    ) -> Option<&'a VariantRefinementNavigationIdentity> {
        #[cfg(test)]
        record_classified_role_lookup();
        file.variant_refinement_identities
            .get_or_init(|| self.variant_refinement_identities(file))
            .get(&(token.range.start, token.range.end))
    }

    fn variant_refinement_identities(
        &self,
        file: &IndexedFile,
    ) -> BTreeMap<(usize, usize), VariantRefinementNavigationIdentity> {
        let mut identities = BTreeMap::new();
        for segment in file
            .classified_path_segments
            .iter()
            .filter(|segment| segment.role == NameClass::Constructor)
        {
            let Some((variant_index, variant_token)) = file
                .tokens
                .iter()
                .enumerate()
                .find(|(_, token)| {
                    token.range.start == segment.span.start.offset
                        && token.range.end == segment.span.end.offset
                })
            else {
                continue;
            };
            let Some(base_index) = variant_refinement_base_index(&file.tokens, variant_index)
            else {
                continue;
            };
            let base_token = &file.tokens[base_index];
            let base_span = file.source.span(base_token.range);
            let Some(base_segment) = file
                .classified_path_segments_by_range
                .get(&(base_token.range.start, base_token.range.end))
                .filter(|segment| segment.role == NameClass::Type)
            else {
                continue;
            };
            debug_assert!(same_span(&base_segment.span, &base_span));
            let Some(base) = self.variant_refinement_base_symbol(
                file,
                &file.tokens,
                base_index,
                &base_token.text,
            ) else {
                continue;
            };
            if matches!(
                &base,
                VariantRefinementBaseSymbol::Alias(alias) if alias.package.is_some()
            ) {
                continue;
            }
            let terminal = match &base {
                VariantRefinementBaseSymbol::Type(symbol) => Some(symbol.clone()),
                VariantRefinementBaseSymbol::Alias(symbol) => {
                    self.terminal_type_for_alias(symbol, &mut BTreeSet::new())
                }
            };
            let Some(terminal) = terminal else {
                continue;
            };
            let mut constructors = self.constructors.iter().filter(|constructor| {
                constructor.declaration_kind == SymbolDeclarationKind::Declaration
                    && constructor.name == variant_token.text
                    && constructor.module == terminal.module
                    && constructor.type_name == terminal.name
                    && constructor.package == terminal.package
                    && constructor.package_origin == terminal.package_origin
            });
            let Some(constructor) = constructors.next().cloned() else {
                continue;
            };
            if constructors.next().is_some() {
                continue;
            }
            identities.insert(
                (variant_token.range.start, variant_token.range.end),
                VariantRefinementNavigationIdentity { base, constructor },
            );
        }
        identities
    }

    fn variant_refinement_base_symbol(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<VariantRefinementBaseSymbol> {
        match self.visible_type_conflict_for_reference(file, tokens, token_index, name)? {
            TypeConflictCandidate::Type(symbol) => {
                Some(VariantRefinementBaseSymbol::Type(symbol))
            }
            TypeConflictCandidate::Alias(symbol) => {
                Some(VariantRefinementBaseSymbol::Alias(symbol))
            }
        }
    }

    fn terminal_type_for_alias(
        &self,
        alias: &TypeAliasSymbol,
        visited: &mut BTreeSet<(Option<String>, String, String)>,
    ) -> Option<TypeSymbol> {
        let identity = (alias.package.clone(), alias.module.clone(), alias.name.clone());
        if !visited.insert(identity) {
            return None;
        }
        let declaring_file = self.files.iter().find(|file| {
            file.source.path() == &alias.declaration.span.file
                && match (&file.origin, alias.package.as_deref()) {
                    (IndexedOrigin::Workspace, None) => true,
                    (IndexedOrigin::Package { identity, .. }, Some(package)) => identity == package,
                    _ => false,
                }
        })?;
        let modules = alias.target_module.as_deref().map_or_else(
            || vec![alias.module.clone()],
            |qualifier| self.qualified_module_candidates(declaring_file, qualifier),
        );
        let package_matches = |package: &Option<String>, origin: Option<PackageOrigin>| {
            package == &alias.package && origin == alias.package_origin
        };
        let mut types = self.types.iter().filter(|candidate| {
            candidate.name == alias.target_name
                && modules.iter().any(|module| module == &candidate.module)
                && package_matches(&candidate.package, candidate.package_origin)
        });
        let first_type = types.next().cloned();
        if types.next().is_some() {
            return None;
        }
        let mut aliases = self.type_aliases.iter().filter(|candidate| {
            candidate.name == alias.target_name
                && modules.iter().any(|module| module == &candidate.module)
                && package_matches(&candidate.package, candidate.package_origin)
        });
        let next_alias = aliases.next().cloned();
        if aliases.next().is_some() || (first_type.is_some() && next_alias.is_some()) {
            return None;
        }
        first_type.or_else(|| {
            self.terminal_type_for_alias(next_alias.as_ref()?, visited)
        })
    }
}
