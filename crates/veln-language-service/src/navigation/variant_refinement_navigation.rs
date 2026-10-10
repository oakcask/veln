#[derive(Clone)]
enum VariantRefinementAliasTarget {
    Type(TypeSymbol),
    Alias(TypeAliasSymbol),
}

type VariantRefinementAliasDeclarations =
    BTreeMap<TypeIdentity, Vec<VariantRefinementAliasTarget>>;
type VariantRefinementAliases<'a> = BTreeMap<TypeIdentity, Vec<&'a TypeAliasSymbol>>;

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
            if let Some((range, identity)) =
                self.variant_refinement_identity_for_range(file, &tokens_by_range, final_range)
            {
                identities.insert(range, identity);
            }
        }
        identities
    }

    fn variant_refinement_identity_for_range(
        &self,
        file: &IndexedFile,
        tokens_by_range: &BTreeMap<(usize, usize), (usize, &Token)>,
        final_range: &(usize, usize),
    ) -> Option<((usize, usize), VariantRefinementNavigationIdentity)> {
        let (variant_index, variant_token) = tokens_by_range.get(final_range).copied()?;
        let base_index = variant_refinement_base_index(&file.tokens, variant_index)?;
        let base_token = &file.tokens[base_index];
        let base = self.variant_refinement_base_symbol(
            file,
            &file.tokens,
            base_index,
            &base_token.text,
        )?;
        let terminal = match &base {
            VariantRefinementBaseSymbol::Type(symbol) => symbol.clone(),
            VariantRefinementBaseSymbol::Alias(symbol) => self.terminal_type_for_alias(symbol)?,
        };
        let constructor = self.unique_variant_refinement_constructor(file, &terminal, variant_token)?;
        Some((
            (variant_token.range.start, variant_token.range.end),
            VariantRefinementNavigationIdentity { base, constructor },
        ))
    }

    fn unique_variant_refinement_constructor(
        &self,
        file: &IndexedFile,
        terminal: &TypeSymbol,
        variant_token: &Token,
    ) -> Option<ConstructorSymbol> {
        let key = (
            terminal.package.clone(),
            terminal.package_origin,
            terminal.module.clone(),
            terminal.name.clone(),
            variant_token.text.clone(),
        );
        let indices = self.constructor_indices_by_identity.get(&key)?;
        let mut constructors = indices
            .iter()
            .map(|index| &self.constructors[*index])
            .filter(|constructor| {
                #[cfg(test)]
                record_variant_refinement_constructor_candidate_visit();
                constructor.declaration_kind == SymbolDeclarationKind::Declaration
                    && self.variant_refinement_constructor_visible(file, constructor)
            });
        let constructor = constructors.next().cloned()?;
        constructors.next().is_none().then_some(constructor)
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

    fn variant_refinement_base_alias_definition_supported(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        alias: &TypeAliasSymbol,
    ) -> bool {
        let Some(variant_index) = variant_refinement_variant_index(tokens, token_index) else {
            return false;
        };
        self.variant_refinement_identity(file, &tokens[variant_index])
            .is_some_and(|identity| {
                matches!(
                    &identity.base,
                    VariantRefinementBaseSymbol::Alias(base) if same_type_alias(base, alias)
                )
            })
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
    ) -> Option<TypeSymbol> {
        let identity = type_alias_identity(alias);
        let terminal = self
            .variant_refinement_alias_index
            .get_or_init(|| self.build_variant_refinement_alias_index())
            .terminal_types
            .get(&identity)
            .cloned()
            .flatten();
        record_variant_refinement_alias_cache_reuse();
        terminal
    }

    fn build_variant_refinement_alias_index(&self) -> VariantRefinementAliasIndex {
        let (declarations, aliases) = self.variant_refinement_alias_declarations();
        let targets = self.variant_refinement_alias_targets(&aliases, &declarations);
        let terminal_types = terminal_variant_refinement_alias_types(&targets);
        VariantRefinementAliasIndex { terminal_types }
    }

    fn variant_refinement_alias_declarations(
        &self,
    ) -> (VariantRefinementAliasDeclarations, VariantRefinementAliases<'_>) {
        let mut declarations = BTreeMap::<TypeIdentity, Vec<VariantRefinementAliasTarget>>::new();
        for symbol in &self.types {
            record_variant_refinement_alias_index_entry();
            declarations
                .entry(type_symbol_identity(symbol))
                .or_default()
                .push(VariantRefinementAliasTarget::Type(symbol.clone()));
        }
        let mut aliases = BTreeMap::<TypeIdentity, Vec<&TypeAliasSymbol>>::new();
        for symbol in &self.type_aliases {
            record_variant_refinement_alias_index_entry();
            declarations
                .entry(type_alias_identity(symbol))
                .or_default()
                .push(VariantRefinementAliasTarget::Alias(symbol.clone()));
            aliases
                .entry(type_alias_identity(symbol))
                .or_default()
                .push(symbol);
        }
        (declarations, aliases)
    }

    fn variant_refinement_alias_targets(
        &self,
        aliases: &VariantRefinementAliases<'_>,
        declarations: &VariantRefinementAliasDeclarations,
    ) -> BTreeMap<TypeIdentity, Option<VariantRefinementAliasTarget>> {
        let mut targets = BTreeMap::<TypeIdentity, Option<VariantRefinementAliasTarget>>::new();
        for (identity, candidates) in aliases {
            let target = match candidates.as_slice() {
                [alias] => self.indexed_type_alias_target(alias, declarations),
                _ => None,
            };
            targets.insert(identity.clone(), target);
        }
        targets
    }

    fn indexed_type_alias_target(
        &self,
        alias: &TypeAliasSymbol,
        declarations: &VariantRefinementAliasDeclarations,
    ) -> Option<VariantRefinementAliasTarget> {
        let declaring_file = self.type_alias_declaring_file(alias)?;
        if alias.package.is_none() {
            return self.indexed_workspace_type_alias_target(alias, declaring_file);
        }
        self.indexed_package_type_alias_target(alias, declaring_file, declarations)
    }

    fn indexed_workspace_type_alias_target(
        &self,
        alias: &TypeAliasSymbol,
        declaring_file: &IndexedFile,
    ) -> Option<VariantRefinementAliasTarget> {
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
        Some(match target {
            TypeConflictCandidate::Type(symbol) => VariantRefinementAliasTarget::Type(symbol),
            TypeConflictCandidate::Alias(symbol) => VariantRefinementAliasTarget::Alias(symbol),
        })
    }

    fn indexed_package_type_alias_target(
        &self,
        alias: &TypeAliasSymbol,
        declaring_file: &IndexedFile,
        declarations: &VariantRefinementAliasDeclarations,
    ) -> Option<VariantRefinementAliasTarget> {
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
        let mut candidates = modules.into_iter().flat_map(|module| {
            declarations
                .get(&(
                    alias.package.clone(),
                    alias.package_origin,
                    module,
                    alias.target_name.clone(),
                ))
                .into_iter()
                .flatten()
                .cloned()
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then_some(candidate)
    }

    fn type_alias_declaring_file(&self, alias: &TypeAliasSymbol) -> Option<&IndexedFile> {
        let key = (
            alias.package.clone(),
            alias.package_origin,
            alias.declaration.span.file.as_str().to_string(),
        );
        let [index] = self.file_indices_by_identity.get(&key)?.as_slice() else {
            return None;
        };
        self.files.get(*index)
    }
}

fn terminal_variant_refinement_alias_types(
    targets: &BTreeMap<TypeIdentity, Option<VariantRefinementAliasTarget>>,
) -> BTreeMap<TypeIdentity, Option<TypeSymbol>> {
    let mut terminal_types = BTreeMap::new();
    for start in targets.keys() {
        if terminal_types.contains_key(start) {
            record_variant_refinement_alias_cache_reuse();
            continue;
        }
        resolve_variant_refinement_alias_trace(start, targets, &mut terminal_types);
    }
    terminal_types
}

fn resolve_variant_refinement_alias_trace(
    start: &TypeIdentity,
    targets: &BTreeMap<TypeIdentity, Option<VariantRefinementAliasTarget>>,
    terminal_types: &mut BTreeMap<TypeIdentity, Option<TypeSymbol>>,
) {
    let mut trace = Vec::new();
    let mut positions = BTreeMap::new();
    let mut current = start.clone();
    let terminal = loop {
        if let Some(terminal) = terminal_types.get(&current) {
            record_variant_refinement_alias_cache_reuse();
            break terminal.clone();
        }
        if positions.insert(current.clone(), trace.len()).is_some() {
            break None;
        }
        trace.push(current.clone());
        record_variant_refinement_alias_target_lookup();
        match targets.get(&current).cloned().flatten() {
            Some(VariantRefinementAliasTarget::Type(symbol)) => break Some(symbol),
            Some(VariantRefinementAliasTarget::Alias(symbol)) => {
                current = type_alias_identity(&symbol);
            }
            None => break None,
        }
    };
    for identity in trace {
        terminal_types.insert(identity, terminal.clone());
    }
}

fn type_alias_identity(alias: &TypeAliasSymbol) -> TypeIdentity {
    (
        alias.package.clone(),
        alias.package_origin,
        alias.module.clone(),
        alias.name.clone(),
    )
}

fn type_symbol_identity(symbol: &TypeSymbol) -> TypeIdentity {
    (
        symbol.package.clone(),
        symbol.package_origin,
        symbol.module.clone(),
        symbol.name.clone(),
    )
}
