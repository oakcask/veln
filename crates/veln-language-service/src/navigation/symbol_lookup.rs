impl SymbolIndex {
    fn has_visible_schema_alias_declaration(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> bool {
        let Some(qualifier) = qualifier_for_token(tokens, token_index) else {
            return self.schema_alias_declarations.iter().any(|symbol| {
                symbol.package.is_none() && symbol.module == file.module && symbol.name == name
            });
        };
        match qualified_workspace_module(file, &qualifier) {
            QualifiedWorkspaceModule::Workspace(module) => {
                self.schema_alias_declarations.iter().any(|symbol| {
                    symbol.package.is_none() && symbol.module == module && symbol.name == name
                })
            }
            QualifiedWorkspaceModule::External => {
                let qualified_modules = self.qualified_module_candidates(file, &qualifier);
                self.schema_alias_declarations.iter().any(|symbol| {
                    symbol.name == name
                        && qualified_modules.iter().any(|module| module == &symbol.module)
                        && symbol.package.as_ref().is_some_and(|package| {
                            file.external_uses
                                .contains(&(symbol.module.clone(), package.clone()))
                        })
                })
            }
            QualifiedWorkspaceModule::Ambiguous | QualifiedWorkspaceModule::Unresolved => false,
        }
    }

    fn visible_schema_alias_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<NeutralSymbol> {
        self.schema_aliases
            .iter()
            .find(|symbol| {
                symbol.name == name
                    && symbol.module == file.module
                    && symbol.package.is_none()
            })
            .cloned()
    }

    fn visible_schema_alias_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<NeutralSymbol> {
        let qualification = qualified_workspace_module(file, qualifier);
        if matches!(&qualification, QualifiedWorkspaceModule::Ambiguous) {
            return None;
        }
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        let mut candidates = self.schema_aliases.iter().filter(|symbol| {
            symbol.name == name
                && qualified_modules.iter().any(|module| module == &symbol.module)
                && match &symbol.package {
                    Some(package) => {
                        !matches!(&qualification, QualifiedWorkspaceModule::Workspace(_))
                            && symbol.package_origin == Some(PackageOrigin::DirectDependency)
                            && file
                                .external_uses
                                .contains(&(symbol.module.clone(), package.clone()))
                    }
                    None => {
                        !matches!(&qualification, QualifiedWorkspaceModule::External)
                            && (symbol.module == file.module
                                || file.uses.contains(&symbol.module))
                    }
                }
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn visible_schema_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<NeutralSymbol> {
        self.schemas.iter().find(|symbol| {
            symbol.name == name && symbol.module == file.module && symbol.package.is_none()
        }).cloned()
    }

    fn visible_schema_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<NeutralSymbol> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        let mut candidates = self.schemas.iter().filter(|symbol| {
            symbol.name == name
                && qualified_modules.iter().any(|module| module == &symbol.module)
                && match &symbol.package {
                    Some(package) => file
                        .external_uses
                        .contains(&(symbol.module.clone(), package.clone())),
                    None => {
                        (symbol.module == file.module || file.uses.contains(&symbol.module))
                            && visible_schema_from_workspace_module(file, symbol)
                    }
                }
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

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

    fn visible_type_alias_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeAliasSymbol> {
        if is_field_name(tokens, token_index) {
            return None;
        }
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            self.visible_type_alias_for_qualified_reference(file, &qualifier, name)
        } else {
            self.visible_type_alias_for_bare_reference(file, name)
        }
    }

    fn type_namespace_symbol_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<Symbol> {
        if is_field_name(tokens, token_index) {
            return None;
        }
        let mut types = self.visible_types_for_reference(file, tokens, token_index, name);
        match types.len() {
            1 => types.pop().map(Symbol::Type),
            0 => self
                .visible_type_alias_for_reference(file, tokens, token_index, name)
                .filter(|symbol| {
                    matches!(
                        symbol.package_origin,
                        Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                    )
                })
                .map(Symbol::TypeAlias),
            _ => None,
        }
    }

    fn visible_types_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Vec<TypeSymbol> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return self.visible_types_for_qualified_reference(file, &qualifier, name);
        }
        self.visible_types_for_bare_reference(file, name)
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
        let mut candidates = self.visible_types_for_bare_reference(file, name);
        let candidate = candidates.pop()?;
        candidates.is_empty().then_some(candidate)
    }

    fn visible_types_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Vec<TypeSymbol> {
        if let Some(symbol) = self.first_local_type_for_bare_reference(file, name) {
            return vec![symbol.clone()];
        }

        self.types
            .iter()
            .filter(|symbol| visible_imported_type_for_bare_reference(file, symbol, name))
            .cloned()
            .collect()
    }

    fn visible_type_alias_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<TypeAliasSymbol> {
        let mut candidates = self.type_aliases.iter().filter(|symbol| {
            visible_imported_type_alias_for_bare_reference(file, symbol, name)
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
        let mut candidates = self.visible_types_for_qualified_reference(file, qualifier, name);
        let candidate = candidates.pop()?;
        candidates.is_empty().then_some(candidate)
    }

    fn visible_types_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Vec<TypeSymbol> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        self.types
            .iter()
            .filter(|symbol| {
                visible_type_for_qualified_reference(file, symbol, &qualified_modules, name)
            })
            .cloned()
            .collect()
    }

    fn visible_type_alias_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<TypeAliasSymbol> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        let mut candidates = self.type_aliases.iter().filter(|symbol| {
            visible_type_alias_for_qualified_reference(file, symbol, &qualified_modules, name)
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
            valid_function_navigation_symbol(symbol)
                && symbol.name == name
                && symbol.module == file.module
                && symbol.package.is_none()
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
            .find(|symbol| {
                valid_function_navigation_symbol(symbol)
                    && symbol.name == name
                    && symbol.standard_prelude
            })
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
                    valid_function_navigation_symbol(symbol)
                        && symbol.name == name
                        && qualified_modules.iter().any(|module| module == &symbol.module)
                        && (symbol.standard_prelude
                            || file
                                .external_uses
                                .contains(&(symbol.module.clone(), package.clone())))
                }
                None => {
                    valid_function_navigation_symbol(symbol)
                        && symbol.name == name
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

    fn function_conflict_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<FunctionSymbol> {
        self.functions
            .iter()
            .find(|symbol| {
                valid_function_navigation_symbol(symbol)
                    && symbol.name == name
                    && symbol.module == file.module
                    && symbol.package.is_none()
            })
            .cloned()
            .or_else(|| self.first_visible_imported_function_for_bare_call(file, name))
    }

    fn first_visible_imported_function_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<FunctionSymbol> {
        self.functions
            .iter()
            .find(|symbol| visible_imported_function_for_bare_call(file, symbol, name))
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
                && symbol.public
                && symbol.package.as_ref().is_some_and(|package| {
                    symbol.standard_prelude
                        || file
                            .external_uses
                            .contains(&(symbol.module.clone(), package.clone()))
                })
        })
        .or_else(|| {
            self.unique_constructor_matching(|symbol| {
                symbol.name == name
                    && symbol.public
                    && !symbol.standard_prelude
                    && symbol.package.as_ref().is_some_and(|package| {
                        self.constructor_reexport_visible_from(file, symbol, Some(package))
                    })
            })
            .map(constructor_selected_through_public_alias)
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
        self.first_constructor_matching(|symbol| {
            symbol.package.is_none()
                && symbol.name == name
                && (qualified_modules
                    .iter()
                    .any(|module| constructor_qualifier_matches(symbol, module))
                    || (qualifier == symbol.type_name && symbol.module == file.module)
                    || self.workspace_constructor_reexport_qualifier_matches(
                        file, symbol, qualifier,
                    ))
                && (symbol.module == file.module
                    || ((file.uses.contains(&symbol.module)
                        || self.constructor_reexport_visible_from(file, symbol, None))
                        && visible_workspace_constructor_from(file, symbol)))
        })
        .or_else(|| {
            self.unique_constructor_matching(|symbol| {
                symbol.name == name
                    && (qualified_modules
                        .iter()
                        .any(|module| constructor_qualifier_matches(symbol, module))
                        || qualified_modules
                            .iter()
                            .any(|module| module == &format!("{}::{}", symbol.module, symbol.type_name)))
                    && match &symbol.package {
                        Some(package) => {
                            symbol.standard_prelude
                                || file
                                    .external_uses
                                    .contains(&(symbol.module.clone(), package.clone()))
                        }
                        None => false,
                    }
            })
        })
        .or_else(|| {
            self.unique_constructor_matching(|symbol| {
                symbol.name == name
                    && symbol.public
                    && symbol.package.is_some()
                    && self.package_constructor_reexport_qualifier_matches(file, symbol, qualifier)
            })
            .map(constructor_selected_through_public_alias)
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

    fn workspace_constructor_reexport_qualifier_matches(
        &self,
        file: &IndexedFile,
        symbol: &ConstructorSymbol,
        qualifier: &str,
    ) -> bool {
        if symbol.package.is_some() {
            return false;
        }
        self.type_aliases.iter().any(|alias| {
            alias.package.is_none()
                && type_alias_targets_constructor(alias, symbol)
                && (qualifier == alias.module
                    || qualifier == format!("{}::{}", alias.module, alias.name))
                && (file.uses.contains(&alias.module) || file.module == alias.module)
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

    fn package_constructor_reexport_qualifier_matches(
        &self,
        file: &IndexedFile,
        symbol: &ConstructorSymbol,
        qualifier: &str,
    ) -> bool {
        let Some(symbol_package) = symbol.package.as_ref() else {
            return false;
        };
        self.type_aliases.iter().any(|alias| {
            if !type_alias_targets_constructor(alias, symbol) {
                return false;
            }
            if alias.package.as_ref() != Some(symbol_package) {
                return false;
            }
            if qualifier != alias.module && qualifier != format!("{}::{}", alias.module, alias.name)
            {
                return false;
            }
            if alias.standard_prelude {
                return true;
            }
            file.external_uses
                .contains(&(alias.module.clone(), symbol_package.clone()))
        })
    }

    fn has_visible_non_prelude_imported_function(&self, file: &IndexedFile, name: &str) -> bool {
        self.functions
            .iter()
            .any(|symbol| visible_imported_function_for_bare_call(file, symbol, name))
    }
}

enum QualifiedWorkspaceModule {
    Workspace(String),
    External,
    Ambiguous,
    Unresolved,
}

fn qualified_workspace_module(file: &IndexedFile, qualifier: &str) -> QualifiedWorkspaceModule {
    if file.uses.contains(qualifier) || file.module == qualifier {
        return QualifiedWorkspaceModule::Workspace(qualifier.to_string());
    }
    let exact_external_count = file
        .external_uses
        .iter()
        .filter(|(module, _)| module == qualifier)
        .count();
    if exact_external_count == 1 {
        return QualifiedWorkspaceModule::External;
    }
    if exact_external_count > 1 {
        return QualifiedWorkspaceModule::Ambiguous;
    }

    let workspace_modules = file
        .uses
        .iter()
        .filter(|module| module.rsplit("::").next() == Some(qualifier))
        .cloned()
        .collect::<Vec<_>>();
    let external_module_count = file
        .external_uses
        .iter()
        .filter(|(module, _)| module.rsplit("::").next() == Some(qualifier))
        .count();
    match (workspace_modules.as_slice(), external_module_count) {
        ([module], 0) => QualifiedWorkspaceModule::Workspace(module.clone()),
        ([], 1) => QualifiedWorkspaceModule::External,
        ([], 0) => QualifiedWorkspaceModule::Unresolved,
        _ => QualifiedWorkspaceModule::Ambiguous,
    }
}

fn constructor_selected_through_public_alias(mut symbol: ConstructorSymbol) -> ConstructorSymbol {
    symbol.declaration_kind = SymbolDeclarationKind::PublicAlias;
    symbol
}

fn visible_imported_function_for_bare_call(
    file: &IndexedFile,
    symbol: &FunctionSymbol,
    name: &str,
) -> bool {
    if !valid_function_navigation_symbol(symbol) || symbol.name != name || symbol.standard_prelude {
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
}

fn valid_function_navigation_symbol(symbol: &FunctionSymbol) -> bool {
    !symbol.invalid_declaration_name
}
