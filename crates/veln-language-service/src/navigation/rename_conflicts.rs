impl SymbolIndex {
    fn rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        match result.selected_symbol.kind.rename_name_class() {
            RenameNameClass::Type => self.type_rename_conflict(result, requested_name),
            RenameNameClass::Constructor => self.constructor_rename_conflict(result, requested_name),
            RenameNameClass::Function => self.function_rename_conflict(result, requested_name),
            RenameNameClass::ValueBinding => self.local_rename_conflict(result, requested_name),
        }
    }

    fn type_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let selected = self.selected_type(result)?;
        self.local_type_namespace_conflict(&selected.module, requested_name)
            .filter(|candidate| !candidate.is_selected_type(&selected))
            .map(|candidate| {
                (
                    candidate.declaration(),
                    RenameAffectedScope::Module {
                        name: selected.module.clone(),
                    },
                )
            })
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
                        Some((
                            conflict.declaration(),
                            RenameAffectedScope::Module {
                                name: file.module.clone(),
                            },
                        ))
                    })
            })
    }

    fn constructor_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let selected = self.selected_constructor(result)?;
        self.constructors
            .iter()
            .find(|candidate| {
                candidate.package.is_none()
                    && candidate.module == selected.module
                    && candidate.type_name == selected.type_name
                    && candidate.name == requested_name
                    && !same_constructor(candidate, &selected)
            })
            .map(|candidate| {
                (
                    candidate.declaration.clone(),
                    RenameAffectedScope::Module {
                        name: selected.module.clone(),
                    },
                )
            })
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
                        Some((
                            conflict.declaration,
                            RenameAffectedScope::Module {
                                name: file.module.clone(),
                            },
                        ))
                    })
            })
    }

    fn function_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let selected = self.selected_function(result)?;
        self.function_module_conflict(&selected, requested_name)
            .or_else(|| self.function_reference_conflict(result, requested_name, &selected))
    }

    fn function_module_conflict(
        &self,
        selected: &FunctionSymbol,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        self.functions
            .iter()
            .find(|candidate| {
                candidate.package.is_none()
                    && candidate.module == selected.module
                    && candidate.name == requested_name
                    && !same_function(candidate, selected)
            })
            .map(|candidate| {
                (
                    candidate.declaration.clone(),
                    RenameAffectedScope::Module {
                        name: selected.module.clone(),
                    },
                )
            })
    }

    fn function_reference_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
        selected: &FunctionSymbol,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        self.affected_spans(result)
            .into_iter()
            .find_map(|span| self.function_span_conflict(span, requested_name, selected))
    }

    fn function_span_conflict(
        &self,
        span: &SourceSpan,
        requested_name: &str,
        selected: &FunctionSymbol,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let (file, token_index) = self.file_token_for_span(span)?;
        if local_binding_shadows_call_target(&file.tokens, token_index, requested_name) {
            return self.local_conflict_in_file(file, requested_name, span);
        }
        if let Some(conflict) =
            self.constructor_conflict_for_call(file, &file.tokens, token_index, requested_name)
        {
            return Some(module_rename_conflict(conflict.declaration, &file.module));
        }
        let conflict = self
            .function_conflict_for_call(file, token_index, requested_name)
            .filter(|candidate| !same_function(candidate, selected))?;
        Some(module_rename_conflict(conflict.declaration, &file.module))
    }

    fn function_conflict_for_call(
        &self,
        file: &IndexedFile,
        token_index: usize,
        requested_name: &str,
    ) -> Option<FunctionSymbol> {
        match qualifier_for_token(&file.tokens, token_index) {
            Some(qualifier) => self.function_for_qualified_call(file, &qualifier, requested_name),
            None => self
                .symbol_for_bare_call(file, &file.tokens, token_index, requested_name)
                .and_then(|symbol| match symbol {
                    Symbol::Function(symbol) => Some(symbol),
                    _ => None,
                }),
        }
    }

    fn local_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let selected = self.selected_local(result)?;
        let file = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == selected.scope_file)?;
        let scope = RenameAffectedScope::Lexical {
            file: selected.scope_file.clone(),
            start_offset: selected.scope_start,
            end_offset: selected.scope_end,
        };
        let affected_spans = self.affected_spans(result);
        let clause_bindings = handler_operation_clause_bindings(file, &file.tokens);
        self.clause_binding_scope_conflict(&clause_bindings, &selected, requested_name, &scope)
            .or_else(|| {
                self.clause_binding_affected_conflict(
                    clause_bindings,
                    &selected,
                    requested_name,
                    &affected_spans,
                    &scope,
                )
            })
            .or_else(|| {
                self.local_binding_affected_conflict(
                    file,
                    &selected,
                    requested_name,
                    &affected_spans,
                    scope,
                )
            })
    }

    fn clause_binding_scope_conflict(
        &self,
        clause_bindings: &[ClauseBinding],
        selected: &LocalSymbol,
        requested_name: &str,
        scope: &RenameAffectedScope,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        clause_bindings
            .iter()
            .find(|binding| same_scope_binding_conflicts(binding, selected, requested_name))
            .map(|binding| (workspace_location(binding.declaration.clone()), scope.clone()))
    }

    fn clause_binding_affected_conflict(
        &self,
        clause_bindings: Vec<ClauseBinding>,
        selected: &LocalSymbol,
        requested_name: &str,
        affected_spans: &[&SourceSpan],
        scope: &RenameAffectedScope,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        clause_bindings
            .into_iter()
            .find(|binding| {
                binding.name == requested_name
                    && !same_span(&binding.declaration, &selected.declaration)
                    && span_list_intersects_range(affected_spans, binding.start, binding.end)
            })
            .map(|binding| (workspace_location(binding.declaration), scope.clone()))
    }

    fn local_binding_affected_conflict(
        &self,
        file: &IndexedFile,
        selected: &LocalSymbol,
        requested_name: &str,
        affected_spans: &[&SourceSpan],
        scope: RenameAffectedScope,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        local_bindings(&file.tokens, selected.scope_start, selected.scope_end)
            .into_iter()
            .find(|binding| {
                binding.name == requested_name
                    && !same_span(&local_binding_declaration(file, binding), &selected.declaration)
                    && span_list_intersects_range(affected_spans, binding.start, binding.end)
            })
            .map(|binding| {
                (
                    workspace_location(local_binding_declaration(file, &binding)),
                    scope,
                )
            })
    }
}
