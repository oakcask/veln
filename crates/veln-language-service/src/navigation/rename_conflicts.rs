impl SymbolIndex {
    fn rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        if result.is_recovery {
            return self.recovery_rename_conflict(result, requested_name);
        }
        match result.selected_symbol.kind.rename_name_class() {
            RenameNameClass::CasingNeutral => None,
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

    fn type_post_rename_visibility_conflict(
        &self,
        selected: &TypeSymbol,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        self.files
            .iter()
            .filter(|file| type_visible_after_rename(file, selected))
            .find_map(|file| {
                file.type_reference_spans(requested_name)
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
            .filter(|file| self.constructor_visible_after_rename(file, selected))
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

    fn constructor_visible_after_rename(
        &self,
        file: &IndexedFile,
        selected: &ConstructorSymbol,
    ) -> bool {
        matches!(file.origin, IndexedOrigin::Workspace)
            && selected.package.is_none()
            && (file.module == selected.module
                || (selected.public
                    && (file.uses.contains(&selected.module)
                        || file
                            .companion_target_module
                            .as_ref()
                            .is_some_and(|target| target == &selected.module)
                        || self.constructor_reexport_visible_from(file, selected, None))))
    }

    fn local_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let selected = self.selected_local(result)?;
        self.local_rename_conflict_for_symbol(&selected, result, requested_name)
    }

    fn local_rename_conflict_for_symbol(
        &self,
        selected: &LocalSymbol,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
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
        self.lexical_declaration_conflict(file, selected, requested_name, &scope)
            .or_else(|| {
                self.clause_binding_scope_conflict(&clause_bindings, selected, requested_name, &scope)
            })
            .or_else(|| {
                self.clause_binding_affected_conflict(
                    clause_bindings,
                    selected,
                    requested_name,
                    &affected_spans,
                    &scope,
                )
            })
            .or_else(|| {
                self.local_binding_affected_conflict(
                    file,
                    selected,
                    requested_name,
                    &affected_spans,
                    scope,
                )
            })
    }

    fn lexical_declaration_conflict(
        &self,
        file: &IndexedFile,
        selected: &LocalSymbol,
        requested_name: &str,
        scope: &RenameAffectedScope,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        function_scopes(&file.tokens)
            .into_iter()
            .find(|function_scope| {
                function_scope.body_start == selected.declaration_scope_start
                    && function_scope.end == selected.declaration_scope_end
            })
            .and_then(|function_scope| {
                function_scope
                    .params
                    .iter()
                    .find(|binding| {
                        binding.name == requested_name
                            && !same_span(
                                &scoped_binding_declaration(file, binding),
                                &selected.declaration,
                            )
                    })
                    .map(|binding| scoped_binding_declaration(file, binding))
                    .or_else(|| {
                        function_scope
                            .result_binding
                            .as_ref()
                            .filter(|binding| {
                                binding.name == requested_name
                                    && !same_span(
                                        &scoped_binding_declaration(file, binding),
                                        &selected.declaration,
                                    )
                            })
                            .map(|binding| scoped_binding_declaration(file, binding))
                    })
                    .or_else(|| {
                        function_scope
                            .local_bindings
                            .iter()
                            .find(|binding| {
                                binding.name == requested_name
                                    && !same_span(
                                        &local_binding_declaration(file, binding),
                                        &selected.declaration,
                                    )
                            })
                            .map(|binding| local_binding_declaration(file, binding))
                    })
            })
            .map(|declaration| (workspace_location(declaration), scope.clone()))
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
