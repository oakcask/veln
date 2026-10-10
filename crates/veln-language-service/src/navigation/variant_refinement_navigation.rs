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
        let tokens_by_range = file
            .tokens
            .iter()
            .enumerate()
            .map(|(index, token)| {
                #[cfg(test)]
                record_variant_refinement_token_index_entry();
                ((token.range.start, token.range.end), (index, token))
            })
            .collect::<BTreeMap<_, _>>();
        for final_range in &file.variant_refinement_final_ranges {
            let Some((variant_index, variant_token)) = tokens_by_range
                .get(final_range)
                .copied()
            else {
                continue;
            };
            let Some(base_index) = variant_refinement_base_index(&file.tokens, variant_index)
            else {
                continue;
            };
            let base_token = &file.tokens[base_index];
            let Some(base) = self.variant_refinement_base_symbol(
                file,
                &file.tokens,
                base_index,
                &base_token.text,
            ) else {
                continue;
            };
            let terminal = match &base {
                VariantRefinementBaseSymbol::Type(symbol) => Some(symbol.clone()),
                VariantRefinementBaseSymbol::Alias(symbol) => {
                    self.terminal_type_for_alias(symbol, &mut BTreeSet::new())
                }
            };
            let Some(terminal) = terminal else {
                continue;
            };
            let key = (
                terminal.package.clone(),
                terminal.package_origin,
                terminal.module.clone(),
                terminal.name.clone(),
                variant_token.text.clone(),
            );
            let Some(indices) = self.constructor_indices_by_identity.get(&key) else {
                continue;
            };
            let mut constructors = indices
                .iter()
                .map(|index| &self.constructors[*index])
                .filter(|constructor| {
                    #[cfg(test)]
                    record_variant_refinement_constructor_candidate_visit();
                    constructor.declaration_kind == SymbolDeclarationKind::Declaration
                        && self.variant_refinement_constructor_visible(file, constructor)
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

    fn variant_refinement_base_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<Symbol> {
        let variant_index = variant_refinement_variant_index(tokens, token_index)?;
        let variant = &tokens[variant_index];
        if !file
            .variant_refinement_final_ranges
            .contains(&(variant.range.start, variant.range.end))
        {
            return None;
        }
        match self.variant_refinement_base_symbol(file, tokens, token_index, name)? {
            VariantRefinementBaseSymbol::Type(symbol) => Some(Symbol::Type(symbol)),
            VariantRefinementBaseSymbol::Alias(symbol) => Some(Symbol::TypeAlias(symbol)),
        }
    }

    fn variant_refinement_constructor_for_selection(
        &self,
        file: &IndexedFile,
        token: &Token,
    ) -> Option<ConstructorSymbol> {
        self.variant_refinement_identity(file, token)
            .map(|identity| identity.constructor.clone())
    }

    fn is_variant_refinement_final_token(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
    ) -> bool {
        let token = &tokens[token_index];
        file.variant_refinement_final_ranges
            .contains(&(token.range.start, token.range.end))
    }

    fn variant_refinement_constructor_visible(
        &self,
        file: &IndexedFile,
        constructor: &ConstructorSymbol,
    ) -> bool {
        match constructor.package_origin {
            Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary) => {
                constructor.public
            }
            None => visible_workspace_constructor_from(file, constructor),
        }
    }

    fn terminal_type_for_alias(
        &self,
        alias: &TypeAliasSymbol,
        visited: &mut BTreeSet<(Option<String>, Option<PackageOrigin>, String, String)>,
    ) -> Option<TypeSymbol> {
        let identity = (
            alias.package.clone(),
            alias.package_origin,
            alias.module.clone(),
            alias.name.clone(),
        );
        if !visited.insert(identity) {
            return None;
        }
        let declaring_file = self.type_alias_declaring_file(alias)?;
        match self.unique_type_alias_target(alias, declaring_file)? {
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
                    (
                        IndexedOrigin::Package {
                            identity,
                            standard_library,
                            ..
                        },
                        Some(package),
                    ) => {
                        identity == package
                            && alias.package_origin
                                == Some(if *standard_library {
                                    PackageOrigin::StandardLibrary
                                } else {
                                    PackageOrigin::DirectDependency
                                })
                    }
                    _ => false,
                }
        })
    }

    fn unique_type_alias_target(
        &self,
        alias: &TypeAliasSymbol,
        declaring_file: &IndexedFile,
    ) -> Option<VariantRefinementAliasTarget> {
        if alias.package.is_none() {
            let target = match alias.target_module.as_deref() {
                Some(qualifier) => self.first_visible_type_namespace_for_qualified_reference(
                    declaring_file,
                    qualifier,
                    &alias.target_name,
                ),
                None => self.first_visible_type_namespace_for_bare_reference(
                    declaring_file,
                    &alias.target_name,
                ),
            }?;
            return Some(match target {
                TypeConflictCandidate::Type(symbol) => VariantRefinementAliasTarget::Type(symbol),
                TypeConflictCandidate::Alias(symbol) => VariantRefinementAliasTarget::Alias(symbol),
            });
        }
        let modules = alias.target_module.as_deref().map_or_else(
            || vec![alias.module.clone()],
            |qualifier| {
                if declaring_file.uses.contains(qualifier) {
                    vec![qualifier.to_string()]
                } else {
                    resolve_qualified_alias(&declaring_file.import_aliases, qualifier)
                        .into_iter()
                        .collect()
                }
            },
        );
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
