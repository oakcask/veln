impl SymbolIndex {
    fn recovery_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        match result.selected_symbol.kind.rename_name_class() {
            RenameNameClass::CasingNeutral => None,
            RenameNameClass::Type => self.recovery_type_rename_conflict(result, requested_name),
            RenameNameClass::Constructor => {
                self.recovery_constructor_rename_conflict(result, requested_name)
            }
            RenameNameClass::Function => {
                self.recovery_function_rename_conflict(result, requested_name)
            }
            RenameNameClass::ValueBinding => self.recovery_local_rename_conflict(result, requested_name),
        }
    }

    fn selected_recovery(
        &self,
        result: &NavigationResult,
    ) -> Option<(&IndexedFile, RecoverySymbol)> {
        let file = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == result.definition.span.file.as_str())?;
        let symbol = file
            .recovery_symbols
            .iter()
            .find(|symbol| {
                symbol.kind == result.selected_symbol.kind
                    && same_span(&symbol.declaration, &result.definition.span)
            })?
            .clone();
        Some((file, symbol))
    }

    fn recovery_type_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let (file, symbol) = self.selected_recovery(result)?;
        let selected = TypeSymbol {
            module: file.module.clone(),
            name: symbol.name,
            declaration: workspace_location(symbol.declaration),
            package: None,
            public: symbol.public,
            standard_prelude: false,
        };
        self.local_type_namespace_conflict(&selected.module, requested_name)
            .filter(|candidate| !candidate.is_selected_type(&selected))
            .map(|candidate| module_rename_conflict(candidate.declaration(), &selected.module))
            .or_else(|| self.type_post_rename_visibility_conflict(&selected, requested_name))
            .or_else(|| {
                self.affected_spans(result)
                    .into_iter()
                    .find_map(|span| {
                        let (file, token_index) = self.file_token_for_span(span)?;
                        let conflict = self
                            .visible_type_conflict_for_reference(
                                file,
                                &file.tokens,
                                token_index,
                                requested_name,
                            )
                            .filter(|candidate| !candidate.is_selected_type(&selected))?;
                        Some(module_rename_conflict(conflict.declaration(), &file.module))
                    })
            })
    }

    fn recovery_constructor_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let (file, symbol) = self.selected_recovery(result)?;
        let type_name = recovery_constructor_type_name(file, &symbol)?;
        let selected = ConstructorSymbol {
            module: file.module.clone(),
            type_name,
            name: symbol.name,
            declaration: workspace_location(symbol.declaration),
            package: None,
            public: symbol.public,
            standard_prelude: false,
        };
        self.constructors
            .iter()
            .find(|candidate| {
                candidate.package.is_none()
                    && candidate.module == selected.module
                    && candidate.type_name == selected.type_name
                    && candidate.name == requested_name
                    && !same_constructor(candidate, &selected)
            })
            .map(|candidate| module_rename_conflict(candidate.declaration.clone(), &selected.module))
            .or_else(|| self.constructor_post_rename_visibility_conflict(&selected, requested_name))
            .or_else(|| {
                self.affected_spans(result)
                    .into_iter()
                    .find_map(|span| {
                        let (file, token_index) = self.file_token_for_span(span)?;
                        let conflict = self
                            .constructor_conflict_for_call(
                                file,
                                &file.tokens,
                                token_index,
                                requested_name,
                            )
                            .filter(|candidate| !same_constructor(candidate, &selected))?;
                        Some(module_rename_conflict(conflict.declaration, &file.module))
                    })
            })
    }

    fn recovery_function_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let (file, symbol) = self.selected_recovery(result)?;
        let selected = FunctionSymbol {
            module: file.module.clone(),
            name: symbol.name,
            declaration: workspace_location(symbol.declaration),
            package: None,
            public: symbol.public,
            standard_prelude: false,
        };
        self.function_module_conflict(&selected, requested_name)
            .or_else(|| self.function_post_rename_visibility_conflict(&selected, requested_name))
            .or_else(|| self.function_reference_conflict(result, requested_name, &selected))
    }

    fn recovery_local_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let (_, symbol) = self.selected_recovery(result)?;
        let selected = LocalSymbol {
            name: symbol.name,
            declaration: symbol.declaration.clone(),
            scope_file: symbol.source_file,
            scope_start: symbol.scope_start,
            scope_end: symbol.scope_end,
            declaration_scope_start: symbol.declaration_scope_start,
            declaration_scope_end: symbol.declaration_scope_end,
            kind: match symbol.kind {
                SymbolKind::HandlerContextParameter => LocalSymbolKind::HandlerContextParameter,
                SymbolKind::HandlerOperationClauseParameter => {
                    LocalSymbolKind::HandlerOperationClauseParameter
                }
                _ => LocalSymbolKind::ValueBinding,
            },
        };
        self.local_rename_conflict_for_symbol(&selected, result, requested_name)
    }
}

fn recovery_constructor_type_name(file: &IndexedFile, symbol: &RecoverySymbol) -> Option<String> {
    let parsed = parse(&file.source);
    parsed.tree.items.iter().find_map(|item| {
        let SyntaxItem::Type(type_decl) = item else {
            return None;
        };
        type_decl
            .variants
            .iter()
            .any(|variant| {
                same_span(
                    &constructor_variant_name_span(
                        file,
                        &file.tokens,
                        variant,
                        symbol.name.as_str(),
                    ),
                    &symbol.declaration,
                )
            })
            .then(|| type_decl.name.clone())
            .flatten()
    })
}
