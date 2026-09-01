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
            .or_else(|| self.function_post_rename_visibility_conflict(&selected, requested_name))
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
        let scope_cache = self.function_scope_cache();
        self.affected_spans(result)
            .into_iter()
            .find_map(|span| {
                self.function_span_conflict(span, requested_name, selected, &scope_cache)
            })
    }

    fn function_span_conflict(
        &self,
        span: &SourceSpan,
        requested_name: &str,
        selected: &FunctionSymbol,
        scope_cache: &BTreeMap<String, Vec<FunctionScope>>,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let (file, token_index) = self.file_token_for_span(span)?;
        if local_binding_shadows_call_target_in_scopes(
            scope_cache
                .get(file.source.path().as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            &file.tokens,
            token_index,
            requested_name,
        ) {
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

    fn type_post_rename_visibility_conflict(
        &self,
        selected: &TypeSymbol,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        self.files
            .iter()
            .filter(|file| type_visible_after_rename(file, selected))
            .find_map(|file| {
                type_reference_spans(&file.source, &file.tokens, requested_name)
                    .into_iter()
                    .filter(|(token_index, _)| {
                        qualifier_for_token(&file.tokens, *token_index).is_none()
                    })
                    .find_map(|(token_index, _)| {
                        if type_local_resolution_unchanged(file, selected, requested_name) {
                            return None;
                        }
                        let conflict = self
                            .visible_type_conflict_for_reference(
                                file,
                                &file.tokens,
                                token_index,
                                requested_name,
                            )
                            .filter(|candidate| !candidate.is_selected_type(selected))?;
                        Some(module_rename_conflict(conflict.declaration(), &file.module))
                    })
            })
    }

    fn constructor_post_rename_visibility_conflict(
        &self,
        selected: &ConstructorSymbol,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        self.files
            .iter()
            .filter(|file| constructor_visible_after_rename(file, selected))
            .find_map(|file| {
                file.tokens
                    .iter()
                    .enumerate()
                    .filter(|(index, token)| {
                        token.kind == TokenKind::Ident
                            && token.text == requested_name
                            && qualifier_for_token(&file.tokens, *index).is_none()
                            && is_constructor_reference_token(&file.tokens, *index)
                    })
                    .find_map(|(token_index, _)| {
                        if self.constructor_local_resolution_unchanged(
                            file,
                            selected,
                            requested_name,
                        ) {
                            return None;
                        }
                        let conflict = self
                            .constructor_conflict_for_call(
                                file,
                                &file.tokens,
                                token_index,
                                requested_name,
                            )
                            .filter(|candidate| !same_constructor(candidate, selected))?;
                        Some(module_rename_conflict(conflict.declaration, &file.module))
                    })
            })
    }

    fn function_post_rename_visibility_conflict(
        &self,
        selected: &FunctionSymbol,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let scope_cache = self.function_scope_cache();
        self.files
            .iter()
            .filter(|file| function_visible_after_rename(file, selected))
            .find_map(|file| {
                let file_scopes = scope_cache
                    .get(file.source.path().as_str())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                file.tokens
                    .iter()
                    .enumerate()
                    .filter(|(index, token)| {
                        token.kind == TokenKind::Ident
                            && token.text == requested_name
                            && qualifier_for_token(&file.tokens, *index).is_none()
                            && is_bare_function_reference_token(
                                &file.tokens,
                                file_scopes,
                                *index,
                                requested_name,
                            )
                    })
                    .find_map(|(token_index, _)| {
                        if local_binding_shadows_call_target_in_scopes(
                            file_scopes,
                            &file.tokens,
                            token_index,
                            requested_name,
                        ) || self.function_local_resolution_unchanged(
                            file,
                            selected,
                            requested_name,
                        ) {
                            return None;
                        }
                        if let Some(conflict) = self.constructor_conflict_for_call(
                            file,
                            &file.tokens,
                            token_index,
                            requested_name,
                        ) {
                            return Some(module_rename_conflict(conflict.declaration, &file.module));
                        }
                        let conflict = self
                            .function_conflict_for_call(file, token_index, requested_name)
                            .filter(|candidate| !same_function(candidate, selected))?;
                        Some(module_rename_conflict(conflict.declaration, &file.module))
                    })
            })
    }

    fn function_scope_cache(&self) -> BTreeMap<String, Vec<FunctionScope>> {
        self.files
            .iter()
            .map(|file| {
                (
                    file.source.path().as_str().to_string(),
                    function_scopes(&file.tokens),
                )
            })
            .collect()
    }

    fn constructor_local_resolution_unchanged(
        &self,
        file: &IndexedFile,
        selected: &ConstructorSymbol,
        requested_name: &str,
    ) -> bool {
        file.module != selected.module
            && self
                .local_constructor_for_bare_call(file, requested_name)
                .is_some()
    }

    fn function_local_resolution_unchanged(
        &self,
        file: &IndexedFile,
        selected: &FunctionSymbol,
        requested_name: &str,
    ) -> bool {
        file.module != selected.module
            && self.functions.iter().any(|symbol| {
                symbol.name == requested_name
                    && symbol.module == file.module
                    && symbol.package.is_none()
            })
    }

    fn function_conflict_for_call(
        &self,
        file: &IndexedFile,
        token_index: usize,
        requested_name: &str,
    ) -> Option<FunctionSymbol> {
        match qualifier_for_token(&file.tokens, token_index) {
            Some(qualifier) => self.function_for_qualified_call(file, &qualifier, requested_name),
            None => self.function_conflict_for_bare_call(file, requested_name),
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
            .find(|binding| clause_binding_conflicts_after_rename(binding, selected, requested_name, affected_spans))
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
                local_binding_conflicts_after_rename(file, binding, selected, requested_name, affected_spans)
            })
            .map(|binding| {
                (
                    workspace_location(local_binding_declaration(file, &binding)),
                    scope,
                )
            })
    }
}

fn clause_binding_conflicts_after_rename(
    binding: &ClauseBinding,
    selected: &LocalSymbol,
    requested_name: &str,
    affected_spans: &[&SourceSpan],
) -> bool {
    binding.name == requested_name
        && !same_span(&binding.declaration, &selected.declaration)
        && affected_spans.iter().any(|span| {
            span.start.offset >= binding.start
                && span.start.offset < binding.end
                && !selected_binding_wins_at_span(selected, binding.start, binding.end, span)
        })
}

fn local_binding_conflicts_after_rename(
    file: &IndexedFile,
    binding: &LocalBinding,
    selected: &LocalSymbol,
    requested_name: &str,
    affected_spans: &[&SourceSpan],
) -> bool {
    binding.name == requested_name
        && !same_span(&local_binding_declaration(file, binding), &selected.declaration)
        && affected_spans.iter().any(|span| {
            span.start.offset >= binding.start
                && span.start.offset < binding.end
                && !selected_binding_wins_at_span(selected, binding.start, binding.end, span)
        })
}

fn selected_binding_wins_at_span(
    selected: &LocalSymbol,
    candidate_start: usize,
    candidate_end: usize,
    span: &SourceSpan,
) -> bool {
    same_span(span, &selected.declaration)
        || (span.start.offset >= selected.scope_start
            && span.start.offset < selected.scope_end
            && selected.scope_start >= candidate_start
            && selected.scope_end <= candidate_end)
}

fn type_local_resolution_unchanged(
    file: &IndexedFile,
    selected: &TypeSymbol,
    requested_name: &str,
) -> bool {
    file.module != selected.module
        && file.tokens.iter().enumerate().any(|(index, token)| {
            token.kind == TokenKind::Ident
                && token.text == requested_name
                && is_type_declaration_name(&file.tokens, index)
        })
}

fn type_visible_after_rename(file: &IndexedFile, selected: &TypeSymbol) -> bool {
    matches!(file.origin, IndexedOrigin::Workspace)
        && selected.package.is_none()
        && (file.module == selected.module || (selected.public && file.uses.contains(&selected.module)))
}

fn constructor_visible_after_rename(file: &IndexedFile, selected: &ConstructorSymbol) -> bool {
    matches!(file.origin, IndexedOrigin::Workspace)
        && selected.package.is_none()
        && (file.module == selected.module
            || (selected.public
                && (file.uses.contains(&selected.module)
                    || file
                        .companion_target_module
                        .as_ref()
                        .is_some_and(|target| target == &selected.module))))
}

fn function_visible_after_rename(file: &IndexedFile, selected: &FunctionSymbol) -> bool {
    matches!(file.origin, IndexedOrigin::Workspace)
        && selected.package.is_none()
        && (file.module == selected.module
            || (selected.public
                && (file.uses.contains(&selected.module)
                    || file
                        .companion_target_module
                        .as_ref()
                        .is_some_and(|target| target == &selected.module))))
}
