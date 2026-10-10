enum VariantRefinementAliasTarget {
    Type(TypeSymbol),
    Alias(TypeAliasSymbol),
}

impl SymbolIndex {
    fn variant_refinement_identity<'a>(
        &self,
        file: &'a IndexedFile,
        token: &Token,
    ) -> Option<&'a VariantRefinementNavigationIdentity> {
        #[cfg(test)]
        record_classified_role_lookup();
        file.classified_paths
            .variant_refinement_identities
            .get_or_init(|| self.variant_refinement_identities(file))
            .get(&(token.range.start, token.range.end))
    }

    fn variant_refinement_identities(
        &self,
        file: &IndexedFile,
    ) -> BTreeMap<(usize, usize), VariantRefinementNavigationIdentity> {
        let mut identities = BTreeMap::new();
        for segment in file
            .classified_paths
            .segments
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
                .classified_paths
                .by_range
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

    fn variant_refinement_base_alias_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeAliasSymbol> {
        let variant_index = variant_refinement_variant_index(tokens, token_index)?;
        file.token_has_classified_role(&tokens[variant_index], NameClass::Constructor)
            .then(|| {
                match self.visible_type_conflict_for_reference(file, tokens, token_index, name) {
                    Some(TypeConflictCandidate::Alias(alias)) => Some(alias),
                    _ => None,
                }
            })
            .flatten()
    }

    fn variant_refinement_constructor_for_selection(
        &self,
        file: &IndexedFile,
        token: &Token,
    ) -> Option<ConstructorSymbol> {
        self.variant_refinement_identity(file, token)
            .map(|identity| identity.constructor.clone())
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
        let declaring_file = self.type_alias_declaring_file(alias)?;
        let modules = alias.target_module.as_deref().map_or_else(
            || vec![alias.module.clone()],
            |qualifier| self.qualified_module_candidates(declaring_file, qualifier),
        );
        match self.unique_type_alias_target(alias, &modules)? {
            VariantRefinementAliasTarget::Type(symbol) => Some(symbol),
            VariantRefinementAliasTarget::Alias(symbol) => {
                self.terminal_type_for_alias(&symbol, visited)
            }
        }
    }

    fn type_alias_declaring_file(&self, alias: &TypeAliasSymbol) -> Option<&IndexedFile> {
        self.files.iter().find(|file| {
            file.source.path() == &alias.declaration.span.file
                && match (&file.origin, alias.package.as_deref()) {
                    (IndexedOrigin::Workspace, None) => true,
                    (IndexedOrigin::Package { identity, .. }, Some(package)) => identity == package,
                    _ => false,
                }
        })
    }

    fn unique_type_alias_target(
        &self,
        alias: &TypeAliasSymbol,
        modules: &[String],
    ) -> Option<VariantRefinementAliasTarget> {
        let package_matches = |package: &Option<String>, origin: Option<PackageOrigin>| {
            package == &alias.package && origin == alias.package_origin
        };
        let matching_module = |module: &str| modules.iter().any(|candidate| candidate == module);
        let types = self.types.iter().filter(|candidate| {
            candidate.name == alias.target_name
                && matching_module(&candidate.module)
                && package_matches(&candidate.package, candidate.package_origin)
        });
        let aliases = self.type_aliases.iter().filter(|candidate| {
            candidate.name == alias.target_name
                && matching_module(&candidate.module)
                && package_matches(&candidate.package, candidate.package_origin)
        });
        let mut candidates = types
            .cloned()
            .map(VariantRefinementAliasTarget::Type)
            .chain(
                aliases
                    .cloned()
                    .map(VariantRefinementAliasTarget::Alias),
            );
        let candidate = candidates.next()?;
        candidates.next().is_none().then_some(candidate)
    }
}
