impl SymbolIndex {
    fn schema_alias_selection_blocks_schema_fallback(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> bool {
        let Some(qualifier) = qualifier_for_token(tokens, token_index) else {
            if self.schemas.iter().any(|symbol| {
                symbol.name == name && symbol.module == file.module && symbol.package.is_none()
            }) {
                return false;
            }
            return self.schema_alias_declarations.iter().any(|symbol| {
                symbol.name == name
                    && ((symbol.package.is_none() && symbol.module == file.module)
                        || symbol.standard_prelude)
            });
        };
        match self.schema_alias_qualified_workspace_module(file, &qualifier) {
            QualifiedWorkspaceModule::Workspace(module) => {
                self.schema_alias_declarations.iter().any(|symbol| {
                    symbol.package.is_none() && symbol.module == module && symbol.name == name
                })
            }
            QualifiedWorkspaceModule::External | QualifiedWorkspaceModule::Unresolved => {
                self.package_schema_alias_declarations.iter().any(|alias| {
                    matches!(
                        alias.package_origin,
                        PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary
                    )
                        && alias.name == name
                        && self.schema_alias_external_import_blocks_fallback(
                            file,
                            &qualifier,
                            &alias.module,
                            &alias.package,
                        )
                })
            }
            QualifiedWorkspaceModule::Ambiguous => true,
        }
    }

    fn schema_alias_external_import_blocks_fallback(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        module: &str,
        package: &str,
    ) -> bool {
        self.schema_alias_module_imports
            .get(&file.module)
            .is_some_and(|imports| imports.raw_external_route_matches(qualifier, module, package))
    }

    fn visible_schema_alias_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<NeutralSymbol> {
        if let Some(alias) = self.schema_aliases.iter().find(|symbol| {
            symbol.name == name && symbol.module == file.module && symbol.package.is_none()
        }) {
            return Some(alias.clone());
        }
        // The standard-library prelude is a fallback. A same-named workspace
        // schema must remain visible instead of being replaced by the prelude alias.
        if self.schemas.iter().any(|symbol| {
            symbol.name == name && symbol.module == file.module && symbol.package.is_none()
        }) {
            return None;
        }
        self.schema_aliases
            .iter()
            .find(|symbol| symbol.name == name && symbol.standard_prelude)
            .cloned()
    }

    fn visible_schema_alias_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<NeutralSymbol> {
        let qualification = self.schema_alias_qualified_workspace_module(file, qualifier);
        let mut candidates = self.schema_aliases.iter().filter(|symbol| {
            symbol.name == name
                && match (&qualification, &symbol.package) {
                    (
                        QualifiedWorkspaceModule::External
                        | QualifiedWorkspaceModule::Unresolved,
                        Some(package),
                    ) => {
                        matches!(
                            symbol.package_origin,
                            Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                        )
                            && self.valid_schema_alias_external_import(
                                file,
                                qualifier,
                                &symbol.module,
                                package,
                            )
                    }
                    (QualifiedWorkspaceModule::Workspace(module), None) => {
                        symbol.module == *module
                    }
                    (QualifiedWorkspaceModule::Ambiguous, _)
                    | (QualifiedWorkspaceModule::Workspace(_), Some(_))
                    | (QualifiedWorkspaceModule::External, None)
                    | (QualifiedWorkspaceModule::Unresolved, None) => false,
                }
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn valid_schema_alias_external_import(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        module: &str,
        package: &str,
    ) -> bool {
        self.schema_alias_module_imports
            .get(&file.module)
            .and_then(|imports| imports.valid_external_route(qualifier))
            .is_some_and(|(resolved, candidate_package)| {
                resolved == module && candidate_package == package
            })
    }

    fn schema_alias_qualified_workspace_module(
        &self,
        file: &IndexedFile,
        qualifier: &str,
    ) -> QualifiedWorkspaceModule {
        schema_qualified_workspace_module(file, qualifier, &self.schema_alias_module_imports)
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
        let qualification = self.schema_alias_qualified_workspace_module(file, qualifier);
        if matches!(qualification, QualifiedWorkspaceModule::External) {
            let (module, package) = self
                .schema_alias_module_imports
                .get(&file.module)?
                .valid_external_route(qualifier)?;
            let mut candidates = self.schemas.iter().filter(|symbol| {
                matches!(
                        symbol.package_origin,
                        Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                    )
                    && symbol.package.as_deref() == Some(package.as_str())
                    && symbol.module == module
                    && symbol.name == name
            });
            if let Some(candidate) = candidates.next() {
                return candidates.next().is_none().then(|| candidate.clone());
            }
            return self
                .schemas
                .iter()
                .find(|symbol| {
                    symbol.package_origin == Some(PackageOrigin::StandardLibrary)
                        && symbol.package.as_deref() == Some(package.as_str())
                        && symbol.module == module
                        && symbol.name == name
                })
                .cloned();
        }
        let QualifiedWorkspaceModule::Workspace(module) = qualification else {
            return None;
        };
        let mut candidates = self.schemas.iter().filter(|symbol| {
            symbol.package.is_none()
                && symbol.name == name
                && symbol.module == module
                && visible_schema_from_workspace_module(file, symbol)
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

fn index_schema_alias_module_imports(
    files: &[IndexedFile],
) -> BTreeMap<String, SchemaAliasModuleImports> {
    let mut collected = BTreeMap::<String, (BTreeSet<String>, Vec<ExternalImport>)>::new();
    for file in files.iter().filter(|file| workspace_navigation_file(file)) {
        let (workspace_imports, external_imports) =
            collected.entry(file.module.clone()).or_default();
        workspace_imports.extend(file.uses.iter().cloned());
        #[cfg(test)]
        record_schema_alias_import_index_entries(file.schema_alias_external_imports.len());
        external_imports.extend(file.schema_alias_external_imports.iter().cloned());
    }
    collected
        .into_iter()
        .map(|(module, (workspace_imports, external_imports))| {
            (
                module,
                SchemaAliasModuleImports::new(workspace_imports, external_imports),
            )
        })
        .collect()
}

fn schema_qualified_workspace_module(
    file: &IndexedFile,
    qualifier: &str,
    module_imports: &BTreeMap<String, SchemaAliasModuleImports>,
) -> QualifiedWorkspaceModule {
    if file.module == qualifier {
        return QualifiedWorkspaceModule::Workspace(qualifier.to_string());
    }

    let Some(imports) = module_imports.get(&file.module) else {
        return QualifiedWorkspaceModule::Unresolved;
    };
    let exact_external_imports = imports.external_imports_by_module.get(qualifier);
    let has_exact_workspace_module = imports.workspace_imports.contains(qualifier);
    if has_exact_workspace_module && exact_external_imports.is_some_and(|set| !set.is_empty()) {
        return QualifiedWorkspaceModule::Ambiguous;
    }
    if has_exact_workspace_module {
        return QualifiedWorkspaceModule::Workspace(qualifier.to_string());
    }
    if exact_external_imports.is_some_and(|set| set.len() == 1) {
        return QualifiedWorkspaceModule::External;
    }
    if exact_external_imports.is_some_and(|set| set.len() > 1) {
        return QualifiedWorkspaceModule::Ambiguous;
    }

    let workspace_modules = imports.workspace_imports_by_alias.get(qualifier);
    let external_module_count = imports
        .external_imports_by_alias
        .get(qualifier)
        .map_or(0, BTreeSet::len);
    match (
        workspace_modules.map(BTreeSet::len).unwrap_or(0),
        external_module_count,
    ) {
        (1, 0) => QualifiedWorkspaceModule::Workspace(
            workspace_modules
                .and_then(|modules| modules.iter().next())
                .cloned()
                .expect("one workspace module is present"),
        ),
        (0, 1) => QualifiedWorkspaceModule::External,
        (0, 0) => QualifiedWorkspaceModule::Unresolved,
        _ => QualifiedWorkspaceModule::Ambiguous,
    }
}

impl SchemaAliasModuleImports {
    fn new(workspace_imports: BTreeSet<String>, external_imports: Vec<ExternalImport>) -> Self {
        let workspace_imports_by_alias = workspace_imports_by_alias(&workspace_imports);
        let duplicate_counts = schema_alias_import_duplicate_counts(&external_imports);
        let mut indexed = Self {
            workspace_imports,
            workspace_imports_by_alias,
            ..Self::default()
        };
        for import in external_imports {
            indexed.index_external_import(import, &duplicate_counts);
        }
        indexed
    }

    fn index_external_import(
        &mut self,
        import: ExternalImport,
        duplicate_counts: &BTreeMap<(String, String, String), usize>,
    ) {
        #[cfg(test)]
        record_schema_alias_import_index_entries(1);
        let identity = (import.module.clone(), import.package.clone());
        self.external_imports_by_module
            .entry(import.module.clone())
            .or_default()
            .insert(identity.clone());
        self.external_imports_by_alias
            .entry(import.alias.clone())
            .or_default()
            .insert(identity.clone());
        let duplicate_key = (
            import.module.clone(),
            import.package.clone(),
            import.alias.clone(),
        );
        if import.syntax_valid && duplicate_counts.get(&duplicate_key) == Some(&1) {
            self.valid_external_imports_by_module
                .entry(import.module)
                .or_default()
                .insert(identity.clone());
            self.valid_external_imports_by_alias
                .entry(import.alias)
                .or_default()
                .insert(identity);
        }
    }

    fn valid_external_route(&self, qualifier: &str) -> Option<(String, String)> {
        #[cfg(test)]
        record_schema_alias_import_route_lookup();
        if self.external_imports_by_module.contains_key(qualifier) {
            return unique_external_route(
                self.valid_external_imports_by_module.get(qualifier)?,
                None,
            );
        }
        let (alias, suffix) = split_import_qualifier(qualifier);
        unique_external_route(self.valid_external_imports_by_alias.get(alias)?, suffix)
    }

    fn raw_external_route_matches(
        &self,
        qualifier: &str,
        expected_module: &str,
        expected_package: &str,
    ) -> bool {
        if let Some(routes) = self.external_imports_by_module.get(qualifier) {
            return routes
                .iter()
                .any(|(module, package)| module == expected_module && package == expected_package);
        }
        let (alias, suffix) = split_import_qualifier(qualifier);
        self.external_imports_by_alias
            .get(alias)
            .is_some_and(|routes| {
                routes.iter().any(|(module, package)| {
                    append_module_suffix(module, suffix) == expected_module
                        && package == expected_package
                })
            })
    }
}

fn workspace_imports_by_alias(
    workspace_imports: &BTreeSet<String>,
) -> BTreeMap<String, BTreeSet<String>> {
    workspace_imports.iter().fold(
        BTreeMap::<String, BTreeSet<String>>::new(),
        |mut by_alias, module| {
            let alias = module.rsplit("::").next().unwrap_or(module).to_string();
            by_alias.entry(alias).or_default().insert(module.clone());
            by_alias
        },
    )
}

fn schema_alias_import_duplicate_counts(
    external_imports: &[ExternalImport],
) -> BTreeMap<(String, String, String), usize> {
    external_imports.iter().fold(
        BTreeMap::<(String, String, String), usize>::new(),
        |mut counts, import| {
            #[cfg(test)]
            record_schema_alias_import_index_entries(1);
            *counts
                .entry((
                    import.module.clone(),
                    import.package.clone(),
                    import.alias.clone(),
                ))
                .or_default() += 1;
            counts
        },
    )
}

fn split_import_qualifier(qualifier: &str) -> (&str, Option<&str>) {
    qualifier
        .split_once("::")
        .map_or((qualifier, None), |(alias, suffix)| (alias, Some(suffix)))
}

fn unique_external_route(
    routes: &BTreeSet<(String, String)>,
    suffix: Option<&str>,
) -> Option<(String, String)> {
    let (module, package) = routes.iter().next()?;
    (routes.len() == 1).then(|| (append_module_suffix(module, suffix), package.clone()))
}

fn append_module_suffix(module: &str, suffix: Option<&str>) -> String {
    suffix.map_or_else(|| module.to_string(), |suffix| format!("{module}::{suffix}"))
}

enum QualifiedWorkspaceModule {
    Workspace(String),
    External,
    Ambiguous,
    Unresolved,
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
