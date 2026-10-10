#[derive(Clone)]
enum VariantRefinementAliasTarget {
    Type(TypeSymbol),
    Alias(TypeAliasSymbol),
}

struct VariantRefinementAliasDeclarations {
    by_identity: BTreeMap<TypeIdentity, Vec<VariantRefinementAliasTarget>>,
    standard_prelude: BTreeMap<String, Vec<VariantRefinementAliasTarget>>,
}
type VariantRefinementAliases<'a> = BTreeMap<TypeIdentity, Vec<&'a TypeAliasSymbol>>;

impl SymbolIndex {
    fn variant_refinement_identity<'a>(
        &self,
        file: &'a IndexedFile,
        token: &Token,
    ) -> Option<&'a VariantRefinementNavigationIdentity> {
        self.variant_refinement_identity_for_final_range(
            file,
            &(token.range.start, token.range.end),
        )
    }

    fn variant_refinement_identity_for_final_range<'a>(
        &self,
        file: &'a IndexedFile,
        final_range: &(usize, usize),
    ) -> Option<&'a VariantRefinementNavigationIdentity> {
        #[cfg(test)]
        record_classified_role_lookup();
        file.classified_paths
            .variant_refinement_identities
            .get_or_init(|| self.variant_refinement_identities(file))
            .get(final_range)
    }

    fn variant_refinement_final_range_for_base(
        &self,
        file: &IndexedFile,
        token: &Token,
    ) -> Option<(usize, usize)> {
        record_variant_refinement_base_final_lookup();
        file.variant_refinement_final_range_by_base_range
            .get(&(token.range.start, token.range.end))
            .copied()
    }

    fn variant_refinement_identities(
        &self,
        file: &IndexedFile,
    ) -> BTreeMap<(usize, usize), VariantRefinementNavigationIdentity> {
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
        let mut candidates = BTreeMap::new();
        for final_range in &file.variant_refinement_final_ranges {
            if let Some((range, identity)) =
                self.variant_refinement_identity_for_range(file, &tokens_by_range, final_range)
            {
                candidates.insert(range, identity);
            }
        }
        let mut identities = BTreeMap::new();
        let mut visited = BTreeSet::new();
        for final_range in &file.variant_refinement_final_ranges {
            if visited.contains(final_range) {
                continue;
            }
            let union_ranges = file
                .variant_refinement_union_final_ranges_by_final_range
                .get(final_range)
                .map(Vec::as_slice)
                .unwrap_or(std::slice::from_ref(final_range));
            visited.extend(union_ranges.iter().copied());
            if self.variant_refinement_union_is_valid(file, &candidates, union_ranges)
            {
                for range in union_ranges {
                    if let Some(identity) = candidates.get(range) {
                        identities.insert(*range, identity.clone());
                    }
                }
            }
        }
        identities
    }

    fn variant_refinement_union_is_valid(
        &self,
        file: &IndexedFile,
        candidates: &BTreeMap<(usize, usize), VariantRefinementNavigationIdentity>,
        union_ranges: &[(usize, usize)],
    ) -> bool {
        let Some(first_range) = union_ranges.first() else {
            return false;
        };
        let Some(first) = candidates.get(first_range) else {
            return false;
        };
        if union_ranges.len() == 1 {
            return true;
        }
        if !union_ranges.iter().all(|range| {
            candidates.get(range).is_some_and(|candidate| {
                candidate.constructor.package == first.constructor.package
                    && candidate.constructor.package_origin == first.constructor.package_origin
                    && candidate.constructor.module == first.constructor.module
                    && candidate.constructor.type_name == first.constructor.type_name
            })
        }) {
            return false;
        }
        if union_ranges.iter().all(|range| {
            candidates
                .get(range)
                .is_some_and(|candidate| candidate.semantically_valid)
        }) {
            return true;
        }
        file.variant_refinement_type_argument_fingerprints_by_final_range
            .get(first_range)
            .is_some_and(|first| {
                union_ranges.iter().all(|range| {
                    file.variant_refinement_type_argument_fingerprints_by_final_range
                        .get(range)
                        == Some(first)
                })
            })
    }

    fn variant_refinement_identity_for_range(
        &self,
        file: &IndexedFile,
        tokens_by_range: &BTreeMap<(usize, usize), (usize, &Token)>,
        final_range: &(usize, usize),
    ) -> Option<((usize, usize), VariantRefinementNavigationIdentity)> {
        let (variant_index, variant_token) = tokens_by_range.get(final_range).copied()?;
        let semantically_valid =
            file.token_has_classified_role(variant_token, NameClass::Constructor);
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
        if !semantically_valid
            && !(terminal.package.is_some()
                && file
                    .fully_resolved_variant_refinement_type_arguments
                    .contains(final_range))
        {
            return None;
        }
        let written_generic_arity = file
            .variant_refinement_type_argument_count_by_final_range
            .get(final_range)?;
        if *written_generic_arity != terminal.generic_arity {
            return None;
        }
        let constructor = self.unique_variant_refinement_constructor(file, &terminal, variant_token)?;
        Some((
            (variant_token.range.start, variant_token.range.end),
            VariantRefinementNavigationIdentity {
                base,
                constructor,
                semantically_valid,
            },
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
        match self.unique_variant_refinement_type_namespace_for_reference(
            file,
            tokens,
            token_index,
            name,
        )? {
            TypeConflictCandidate::Type(symbol) => {
                Some(VariantRefinementBaseSymbol::Type(symbol))
            }
            TypeConflictCandidate::Alias(symbol) => {
                Some(VariantRefinementBaseSymbol::Alias(symbol))
            }
        }
    }

    fn unique_variant_refinement_type_namespace_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return self.unique_variant_refinement_type_namespace_for_qualified_reference(
                file, &qualifier, name,
            );
        }
        self.unique_variant_refinement_type_namespace_for_bare_reference(file, name)
    }

    fn unique_variant_refinement_type_namespace_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        let local = self
            .workspace_types_in_module(&file.module, name)
            .cloned()
            .map(TypeConflictCandidate::Type)
            .chain(
                self.workspace_type_aliases_in_module(&file.module, name)
                    .cloned()
                    .map(TypeConflictCandidate::Alias),
            )
            .collect::<Vec<_>>();
        if !local.is_empty() {
            return unique_variant_refinement_type_namespace(local);
        }

        let workspace_imports = file.uses.iter().flat_map(|module| {
            self.workspace_types_in_module(module, name)
                .filter(|symbol| visible_imported_type_for_bare_reference(file, symbol, name))
                .cloned()
                .map(TypeConflictCandidate::Type)
                .chain(
                    self.workspace_type_aliases_in_module(module, name)
                        .filter(|symbol| {
                            visible_imported_type_alias_for_bare_reference(file, symbol, name)
                        })
                        .cloned()
                        .map(TypeConflictCandidate::Alias),
                )
        });
        let package_imports = (!file.external_uses.is_empty())
            .then(|| {
                self.types_named(name)
                    .filter(|symbol| {
                        !symbol.standard_prelude
                            && visible_imported_type_for_bare_reference(file, symbol, name)
                    })
                    .cloned()
                    .map(TypeConflictCandidate::Type)
            })
            .into_iter()
            .flatten();
        let imported = workspace_imports.chain(package_imports).collect::<Vec<_>>();
        if !imported.is_empty() {
            return unique_variant_refinement_type_namespace(imported);
        }

        let prelude = self
            .types_named(name)
            .filter(|symbol| {
                symbol.standard_prelude
                    && visible_imported_type_for_bare_reference(file, symbol, name)
            })
            .cloned()
            .map(TypeConflictCandidate::Type)
            .chain(
                self.type_aliases_named(name)
                    .filter(|symbol| {
                        symbol.standard_prelude
                            && visible_imported_type_alias_for_bare_reference(file, symbol, name)
                    })
                    .cloned()
                    .map(TypeConflictCandidate::Alias),
            )
            .collect::<Vec<_>>();
        unique_variant_refinement_type_namespace(prelude)
    }

    fn unique_variant_refinement_type_namespace_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        let (head, suffix) = qualifier
            .split_once("::")
            .map_or((qualifier, None), |(head, suffix)| (head, Some(suffix)));
        let resolve_route = |module: &str| {
            if qualifier == module {
                Some(module.to_string())
            } else if module.rsplit("::").next() == Some(head) {
                Some(suffix.map_or_else(
                    || module.to_string(),
                    |suffix| format!("{module}::{suffix}"),
                ))
            } else {
                None
            }
        };
        let external_routes = file
            .external_uses
            .iter()
            .filter_map(|(module, package)| {
                resolve_route(module).map(|module| (module, package.clone()))
            })
            .collect::<BTreeSet<_>>();
        let workspace_routes = file
            .uses
            .iter()
            .filter_map(|module| resolve_route(module))
            .chain((qualifier == file.module).then(|| file.module.clone()))
            .collect::<BTreeSet<_>>();
        let workspace_candidates = workspace_routes.iter().flat_map(|module| {
            self.workspace_types_in_module(module, name)
                .filter(|symbol| symbol.public || symbol.module == file.module)
                .cloned()
                .map(TypeConflictCandidate::Type)
                .chain(
                    self.workspace_type_aliases_in_module(module, name)
                        .cloned()
                        .map(TypeConflictCandidate::Alias),
                )
        });
        let package_types = (!external_routes.is_empty())
            .then(|| {
                self.types_named(name)
                    .filter(|symbol| {
                        symbol.public
                            && symbol.package.as_deref().is_some_and(|package| {
                                external_routes
                                    .contains(&(symbol.module.clone(), package.to_string()))
                            })
                    })
                    .cloned()
                    .map(TypeConflictCandidate::Type)
            })
            .into_iter()
            .flatten();
        let package_aliases = external_routes.iter().flat_map(|(module, package)| {
            self.package_type_aliases_in_module(module, name)
                .filter(|symbol| symbol.package.as_deref() == Some(package.as_str()))
                .cloned()
                .map(TypeConflictCandidate::Alias)
        });
        let candidates = workspace_candidates
            .chain(package_types)
            .chain(package_aliases)
            .collect::<Vec<_>>();
        unique_variant_refinement_type_namespace(candidates)
    }

    fn variant_refinement_base_alias_definition_supported(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        alias: &TypeAliasSymbol,
    ) -> bool {
        let Some(final_range) =
            self.variant_refinement_final_range_for_base(file, &tokens[token_index])
        else {
            return false;
        };
        self.variant_refinement_identity_for_final_range(file, &final_range)
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
        _name: &str,
    ) -> Option<Symbol> {
        let final_range =
            self.variant_refinement_final_range_for_base(file, &tokens[token_index])?;
        let identity = self.variant_refinement_identity_for_final_range(file, &final_range)?;
        match &identity.base {
            VariantRefinementBaseSymbol::Type(symbol) => Some(Symbol::Type(symbol.clone())),
            VariantRefinementBaseSymbol::Alias(symbol) => Some(Symbol::TypeAlias(symbol.clone())),
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
        let mut declarations = VariantRefinementAliasDeclarations {
            by_identity: BTreeMap::new(),
            standard_prelude: BTreeMap::new(),
        };
        for symbol in &self.types {
            record_variant_refinement_alias_index_entry();
            declarations
                .by_identity
                .entry(type_symbol_identity(symbol))
                .or_default()
                .push(VariantRefinementAliasTarget::Type(symbol.clone()));
            if symbol.standard_prelude {
                declarations
                    .standard_prelude
                    .entry(symbol.name.clone())
                    .or_default()
                    .push(VariantRefinementAliasTarget::Type(symbol.clone()));
            }
        }
        let mut aliases = BTreeMap::<TypeIdentity, Vec<&TypeAliasSymbol>>::new();
        for symbol in &self.type_aliases {
            record_variant_refinement_alias_index_entry();
            declarations
                .by_identity
                .entry(type_alias_identity(symbol))
                .or_default()
                .push(VariantRefinementAliasTarget::Alias(symbol.clone()));
            if symbol.standard_prelude {
                declarations
                    .standard_prelude
                    .entry(symbol.name.clone())
                    .or_default()
                    .push(VariantRefinementAliasTarget::Alias(symbol.clone()));
            }
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
            Some(qualifier) => self
                .indexed_workspace_type_namespace_for_qualified_alias_target(
                    declaring_file,
                    qualifier,
                    &alias.target_name,
                ),
            None => self.unique_variant_refinement_type_namespace_for_bare_reference(
                declaring_file,
                &alias.target_name,
            ),
        }?;
        Some(match target {
            TypeConflictCandidate::Type(symbol) => VariantRefinementAliasTarget::Type(symbol),
            TypeConflictCandidate::Alias(symbol) => VariantRefinementAliasTarget::Alias(symbol),
        })
    }

    fn indexed_workspace_type_namespace_for_qualified_alias_target(
        &self,
        declaring_file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        self.unique_variant_refinement_type_namespace_for_qualified_reference(
            declaring_file,
            qualifier,
            name,
        )
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
                .by_identity
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
        match (candidates.next(), candidates.next()) {
            (Some(candidate), None) => Some(candidate),
            (Some(_), Some(_)) => None,
            (None, _) if alias.target_module.is_none() => {
                let candidates = declarations.standard_prelude.get(&alias.target_name)?;
                let [candidate] = candidates.as_slice() else {
                    return None;
                };
                Some(candidate.clone())
            }
            (None, _) => None,
        }
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

fn unique_variant_refinement_type_namespace(
    mut candidates: Vec<TypeConflictCandidate>,
) -> Option<TypeConflictCandidate> {
    let candidate = candidates.pop()?;
    candidates.is_empty().then_some(candidate)
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
