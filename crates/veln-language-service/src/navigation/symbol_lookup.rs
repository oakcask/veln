impl SymbolIndex {
    fn visible_type_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeSymbol> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return self.visible_type_for_qualified_reference(file, &qualifier, name);
        }
        self.visible_type_for_bare_reference(file, name)
    }

    fn visible_type_conflict_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return self.first_visible_type_namespace_for_qualified_reference(file, &qualifier, name);
        }
        self.first_visible_type_namespace_for_bare_reference(file, name)
    }

    fn local_type_namespace_conflict(
        &self,
        module: &str,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        self.types
            .iter()
            .find(|symbol| symbol.name == name && symbol.module == module && symbol.package.is_none())
            .cloned()
            .map(TypeConflictCandidate::Type)
            .or_else(|| {
                self.type_aliases
                    .iter()
                    .find(|symbol| {
                        symbol.name == name && symbol.module == module && symbol.package.is_none()
                    })
                    .cloned()
                    .map(TypeConflictCandidate::Alias)
            })
    }

    fn first_visible_type_namespace_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        self.first_local_type_namespace_for_bare_reference(file, name)
            .or_else(|| {
                self.types
                    .iter()
                    .find(|symbol| visible_imported_type_for_bare_reference(file, symbol, name))
                    .cloned()
                    .map(TypeConflictCandidate::Type)
            })
            .or_else(|| {
                self.type_aliases
                    .iter()
                    .find(|symbol| visible_imported_type_alias_for_bare_reference(file, symbol, name))
                    .cloned()
                    .map(TypeConflictCandidate::Alias)
            })
    }

    fn first_local_type_namespace_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        self.types
            .iter()
            .find(|symbol| {
                symbol.name == name && symbol.module == file.module && symbol.package.is_none()
            })
            .cloned()
            .map(TypeConflictCandidate::Type)
            .or_else(|| {
                self.type_aliases
                    .iter()
                    .find(|symbol| {
                        symbol.name == name
                            && symbol.module == file.module
                            && symbol.package.is_none()
                    })
                    .cloned()
                    .map(TypeConflictCandidate::Alias)
            })
    }

    fn first_visible_type_namespace_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        self.types
            .iter()
            .find(|symbol| {
                visible_type_for_qualified_reference(file, symbol, &qualified_modules, name)
            })
            .cloned()
            .map(TypeConflictCandidate::Type)
            .or_else(|| {
                self.type_aliases
                    .iter()
                    .find(|symbol| {
                        visible_type_alias_for_qualified_reference(
                            file,
                            symbol,
                            &qualified_modules,
                            name,
                        )
                    })
                    .cloned()
                    .map(TypeConflictCandidate::Alias)
            })
    }

    fn visible_type_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<TypeSymbol> {
        if let Some(symbol) = self.first_local_type_for_bare_reference(file, name) {
            return Some(symbol.clone());
        }

        let mut candidates = self.types.iter().filter(|symbol| {
            visible_imported_type_for_bare_reference(file, symbol, name)
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn first_local_type_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<&TypeSymbol> {
        self.types.iter().find(|symbol| {
            symbol.name == name && symbol.module == file.module && symbol.package.is_none()
        })
    }

    fn visible_type_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<TypeSymbol> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        let mut candidates = self.types.iter().filter(|symbol| {
            symbol.name == name
                && qualified_modules.iter().any(|module| module == &symbol.module)
                && match &symbol.package {
                    Some(package) => file
                        .external_uses
                        .contains(&(symbol.module.clone(), package.clone()))
                        || symbol.standard_prelude,
                    None => symbol.module == file.module || file.uses.contains(&symbol.module),
                }
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn type_for_constructor_qualifier_token(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeSymbol> {
        let constructor_index = next_path_segment_index(tokens, token_index)?;
        let qualifier = qualifier_for_token(tokens, token_index)
            .map(|prefix| format!("{prefix}::{name}"))
            .unwrap_or_else(|| name.to_string());
        let constructor =
            self.constructor_for_qualified_call(file, &qualifier, &tokens[constructor_index].text)?;
        self.types
            .iter()
            .find(|symbol| {
                symbol.module == constructor.module
                    && symbol.name == constructor.type_name
                    && symbol.package == constructor.package
            })
            .cloned()
    }

    fn symbol_for_bare_call(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<Symbol> {
        if let Some(symbol) = self.constructor_for_bare_call(file, name) {
            return Some(Symbol::Constructor(symbol));
        }
        if let Some(symbol) = self.functions.iter().find(|symbol| {
            symbol.name == name && symbol.module == file.module && symbol.package.is_none()
        }) {
            return Some(Symbol::Function(symbol.clone()));
        }
        if local_binding_shadows_call_target(tokens, token_index, name)
            || self.has_visible_non_prelude_imported_function(file, name)
            || self.has_visible_non_prelude_imported_constructor(file, name)
        {
            return None;
        }
        self.functions
            .iter()
            .find(|symbol| symbol.name == name && symbol.standard_prelude)
            .cloned()
            .map(Symbol::Function)
    }

    fn symbol_for_qualified_call(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<Symbol> {
        if let Some(symbol) = self.constructor_for_qualified_call(file, qualifier, name) {
            return Some(Symbol::Constructor(symbol));
        }
        self.function_for_qualified_call(file, qualifier, name)
            .map(Symbol::Function)
    }

    fn function_for_qualified_call(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<FunctionSymbol> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        self.functions
            .iter()
            .find(|symbol| match &symbol.package {
                Some(package) => {
                    symbol.name == name
                        && qualified_modules.iter().any(|module| module == &symbol.module)
                        && (symbol.standard_prelude
                            || file
                                .external_uses
                                .contains(&(symbol.module.clone(), package.clone())))
                }
                None => {
                    symbol.name == name
                        && qualified_modules.iter().any(|module| module == &symbol.module)
                        && file.uses.contains(&symbol.module)
                        && (symbol.public
                            || file
                                .companion_target_module
                                .as_ref()
                                .is_some_and(|target| target == &symbol.module))
                }
            })
            .cloned()
    }

    fn constructor_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.local_constructor_for_bare_call(file, name)
            .or_else(|| self.imported_workspace_constructor_for_bare_call(file, name))
            .or_else(|| self.imported_package_constructor_for_bare_call(file, name))
    }

    fn local_constructor_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.constructors
            .iter()
            .find(|symbol| {
                symbol.name == name
                    && symbol.package.is_none()
                    && symbol.module == file.module
                    && visible_workspace_constructor_from(file, symbol)
            })
            .cloned()
    }

    fn imported_workspace_constructor_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.unique_constructor_matching(|symbol| {
            symbol.name == name
                && !symbol.standard_prelude
                && symbol.package.is_none()
                && symbol.module != file.module
                && (file.uses.contains(&symbol.module)
                    || self.constructor_reexport_visible_from(file, symbol, None))
                && visible_workspace_constructor_from(file, symbol)
        })
    }

    fn imported_package_constructor_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.unique_constructor_matching(|symbol| {
            symbol.name == name
                && !symbol.standard_prelude
                && symbol.public
                && symbol.package.as_ref().is_some_and(|package| {
                    file.external_uses
                        .contains(&(symbol.module.clone(), package.clone()))
                        || self.constructor_reexport_visible_from(file, symbol, Some(package))
                })
        })
    }

    fn unique_constructor_matching(
        &self,
        predicate: impl Fn(&ConstructorSymbol) -> bool,
    ) -> Option<ConstructorSymbol> {
        let mut candidates = self.constructors.iter().filter(|symbol| predicate(symbol));
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn first_constructor_matching(
        &self,
        predicate: impl Fn(&ConstructorSymbol) -> bool,
    ) -> Option<ConstructorSymbol> {
        self.constructors
            .iter()
            .find(|symbol| predicate(symbol))
            .cloned()
    }

    fn constructor_for_qualified_call(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        self.constructors
            .iter()
            .find(|symbol| {
                symbol.name == name
                    && (qualified_modules
                        .iter()
                        .any(|module| constructor_qualifier_matches(symbol, module))
                        || qualified_modules.iter().any(|module| {
                            module == &format!("{}::{}", symbol.module, symbol.type_name)
                        })
                        || (qualifier == symbol.type_name && symbol.module == file.module)
                        || self.constructor_reexport_qualifier_matches(file, symbol, qualifier))
                    && match &symbol.package {
                        Some(package) => {
                            symbol.standard_prelude
                                || file
                                    .external_uses
                                    .contains(&(symbol.module.clone(), package.clone()))
                                || self.constructor_reexport_visible_from(
                                    file,
                                    symbol,
                                    Some(package),
                                )
                        }
                        None => {
                            symbol.module == file.module
                                || ((file.uses.contains(&symbol.module)
                                    || self.constructor_reexport_visible_from(file, symbol, None))
                                    && visible_workspace_constructor_from(file, symbol))
                        }
                    }
            })
            .cloned()
    }

    fn constructor_reexport_qualifier_matches(
        &self,
        file: &IndexedFile,
        symbol: &ConstructorSymbol,
        qualifier: &str,
    ) -> bool {
        self.type_aliases.iter().any(|alias| {
            type_alias_targets_constructor(alias, symbol)
                && (qualifier == alias.module
                    || qualifier == format!("{}::{}", alias.module, alias.name))
                && match &alias.package {
                    Some(alias_package) => file
                        .external_uses
                        .contains(&(alias.module.clone(), alias_package.clone())),
                    None => file.uses.contains(&alias.module) || file.module == alias.module,
                }
        })
    }

    fn has_visible_non_prelude_imported_constructor(&self, file: &IndexedFile, name: &str) -> bool {
        self.constructors.iter().any(|symbol| {
            if symbol.name != name || symbol.standard_prelude {
                return false;
            }
            if symbol.package.is_none() && symbol.module == file.module {
                return false;
            }
            match &symbol.package {
                Some(package) => {
                    symbol.public
                        && file
                            .external_uses
                            .contains(&(symbol.module.clone(), package.clone()))
                }
                None => {
                    (file.uses.contains(&symbol.module)
                        || self.constructor_reexport_visible_from(file, symbol, None))
                        && visible_workspace_constructor_from(file, symbol)
                }
            }
        })
    }

    fn constructor_reexport_visible_from(
        &self,
        file: &IndexedFile,
        symbol: &ConstructorSymbol,
        package: Option<&String>,
    ) -> bool {
        self.type_aliases.iter().any(|alias| {
            if !type_alias_targets_constructor(alias, symbol) {
                return false;
            }
            if alias.package.as_ref() != package {
                return false;
            }
            match &alias.package {
                Some(alias_package) => file
                    .external_uses
                    .contains(&(alias.module.clone(), alias_package.clone())),
                None => file.uses.contains(&alias.module),
            }
        })
    }

    fn has_visible_non_prelude_imported_function(&self, file: &IndexedFile, name: &str) -> bool {
        self.functions.iter().any(|symbol| {
            if symbol.name != name || symbol.standard_prelude {
                return false;
            }
            if symbol.package.is_none() && symbol.module == file.module {
                return false;
            }
            if symbol.package.is_none() && !symbol.public {
                return false;
            }
            match &symbol.package {
                Some(package) => file
                    .external_uses
                    .contains(&(symbol.module.clone(), package.clone())),
                None => file.uses.contains(&symbol.module),
            }
        })
    }
}
