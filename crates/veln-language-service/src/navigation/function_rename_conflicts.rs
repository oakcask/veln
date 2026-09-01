impl SymbolIndex {
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
        let rename_index = self.function_rename_index();
        self.affected_spans(result).into_iter().find_map(|span| {
            self.function_span_conflict(
                span,
                requested_name,
                selected,
                &rename_index.scopes_by_file,
                &rename_index.handler_files,
            )
        })
    }

    fn function_span_conflict(
        &self,
        span: &SourceSpan,
        requested_name: &str,
        selected: &FunctionSymbol,
        scope_cache: &BTreeMap<String, Vec<FunctionScope>>,
        handler_files: &BTreeSet<String>,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let (file, token_index) = self.file_token_for_span(span)?;
        let file_path = file.source.path().as_str();
        if local_binding_shadows_call_target_in_scopes(
            scope_cache
                .get(file_path)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            &file.tokens,
            token_index,
            requested_name,
        ) {
            return self.local_conflict_in_file(file, requested_name, span);
        }
        if handler_files.contains(file_path)
            && let Some(conflict) = handler_binding_conflict_for_function_reference(
                file,
                &file.tokens,
                token_index,
                requested_name,
            )
        {
            return Some(conflict);
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

    fn function_post_rename_visibility_conflict(
        &self,
        selected: &FunctionSymbol,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let rename_index = self.function_rename_index();
        self.files
            .iter()
            .filter(|file| function_visible_after_rename(file, selected))
            .find_map(|file| {
                let file_path = file.source.path().as_str();
                let file_has_handler_references = rename_index.handler_files.contains(file_path);
                let file_scopes = rename_index
                    .scopes_by_file
                    .get(file_path)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                file.tokens
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| {
                        function_reference_candidate_after_rename(
                            file,
                            file_scopes,
                            *index,
                            requested_name,
                            file_has_handler_references,
                        )
                    })
                    .find_map(|(token_index, _)| {
                        self.function_reference_resolution_conflict(
                            file,
                            file_scopes,
                            token_index,
                            requested_name,
                            selected,
                            file_has_handler_references,
                        )
                    })
            })
    }

    fn function_reference_resolution_conflict(
        &self,
        file: &IndexedFile,
        file_scopes: &[FunctionScope],
        token_index: usize,
        requested_name: &str,
        selected: &FunctionSymbol,
        file_has_handler_references: bool,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        if function_reference_resolution_unchanged(
            self,
            file,
            file_scopes,
            token_index,
            requested_name,
            selected,
            file_has_handler_references,
        ) {
            return None;
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

    fn function_rename_index(&self) -> &FunctionRenameIndex {
        self.function_rename_index.get_or_init(|| {
            let mut scopes_by_file = BTreeMap::new();
            let mut handler_files = BTreeSet::new();
            for file in self
                .files
                .iter()
                .filter(|file| matches!(file.origin, IndexedOrigin::Workspace))
            {
                #[cfg(test)]
                record_function_scope_collection();
                if file
                    .tokens
                    .iter()
                    .any(|token| token.kind == TokenKind::Handler)
                {
                    handler_files.insert(file.source.path().as_str().to_string());
                }
                scopes_by_file.insert(
                    file.source.path().as_str().to_string(),
                    function_scopes(&file.tokens),
                );
            }
            FunctionRenameIndex {
                scopes_by_file,
                handler_files,
            }
        })
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
}

fn handler_function_reference_is_unshadowed(
    file: &IndexedFile,
    tokens: &[Token],
    index: usize,
    name: &str,
) -> bool {
    let file_end = tokens.last().map_or(tokens[index].range.end, |token| token.range.end);
    let offset = tokens[index].range.start;
    handler_function_reference_token(tokens, index, name)
        && !local_binding_shadows_name(tokens, name, offset, 0, file_end)
        && !handler_binding_shadows_function_reference(file, tokens, index, name)
}

fn function_reference_candidate_after_rename(
    file: &IndexedFile,
    file_scopes: &[FunctionScope],
    index: usize,
    requested_name: &str,
    file_has_handler_references: bool,
) -> bool {
    let token = &file.tokens[index];
    if token.kind != TokenKind::Ident
        || token.text != requested_name
        || qualifier_for_token(&file.tokens, index).is_some()
    {
        return false;
    }
    is_bare_function_reference_token(&file.tokens, file_scopes, index, requested_name)
        || file_has_handler_references
            && handler_function_reference_is_unshadowed(file, &file.tokens, index, requested_name)
}

fn function_reference_resolution_unchanged(
    index: &SymbolIndex,
    file: &IndexedFile,
    file_scopes: &[FunctionScope],
    token_index: usize,
    requested_name: &str,
    selected: &FunctionSymbol,
    file_has_handler_references: bool,
) -> bool {
    local_binding_shadows_call_target_in_scopes(
        file_scopes,
        &file.tokens,
        token_index,
        requested_name,
    ) || file_has_handler_references
        && handler_binding_shadows_function_reference(file, &file.tokens, token_index, requested_name)
        || index.function_local_resolution_unchanged(file, selected, requested_name)
}

fn handler_function_reference_token(tokens: &[Token], index: usize, name: &str) -> bool {
    tokens[index].text == name
        && tokens[index].kind == TokenKind::Ident
        && is_identifier(&tokens[index].text)
        && inside_handler_operation_clause_body(tokens, tokens[index].range.start)
        && previous_non_layout_token(tokens, index)
            .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
        && !is_field_name(tokens, index)
        && !is_function_declaration_name(tokens, index)
        && !is_parameter_name(tokens, index)
        && !is_local_binding_name(tokens, index)
        && !is_handler_operation_clause_operation_name(tokens, index)
        && !is_type_position_token(tokens, index)
        && (is_call_target_token(tokens, index)
            || !matches!(
                next_non_layout_token(tokens, index).map(|token| token.kind),
                Some(TokenKind::LParen | TokenKind::Colon | TokenKind::Dot | TokenKind::DoubleColon)
            ))
}

fn handler_binding_conflict_for_function_reference(
    file: &IndexedFile,
    tokens: &[Token],
    index: usize,
    name: &str,
) -> Option<(NavigationLocation, RenameAffectedScope)> {
    let binding = handler_shadowing_binding(file, tokens, index, name)?;
    Some((
        workspace_location(binding.declaration),
        RenameAffectedScope::Lexical {
            file: file.source.path().as_str().to_string(),
            start_offset: binding.start,
            end_offset: binding.end,
        },
    ))
}

fn handler_binding_shadows_function_reference(
    file: &IndexedFile,
    tokens: &[Token],
    index: usize,
    name: &str,
) -> bool {
    handler_shadowing_binding(file, tokens, index, name).is_some()
}

fn handler_shadowing_binding(
    file: &IndexedFile,
    tokens: &[Token],
    index: usize,
    name: &str,
) -> Option<ClauseBinding> {
    let offset = tokens[index].range.start;
    handler_operation_clause_bindings(file, tokens)
        .into_iter()
        .find(|binding| {
            binding.name == name
                && offset >= binding.start
                && offset < binding.end
                && (binding.kind != LocalSymbolKind::HandlerContextParameter
                    || inside_handler_operation_clause_body(tokens, offset))
                && !local_binding_shadows_name(tokens, name, offset, binding.start, binding.end)
        })
}
