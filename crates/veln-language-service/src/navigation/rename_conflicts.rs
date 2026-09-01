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
            RenameNameClass::Type => self.type_rename_conflict(result, requested_name),
            RenameNameClass::Constructor => self.constructor_rename_conflict(result, requested_name),
            RenameNameClass::Function => self.function_rename_conflict(result, requested_name),
            RenameNameClass::ValueBinding => self.local_rename_conflict(result, requested_name),
        }
    }

    fn recovery_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        match result.selected_symbol.kind.rename_name_class() {
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
        let rename_index = self.function_rename_index();
        self.affected_spans(result)
            .into_iter()
            .find_map(|span| self.function_span_conflict(
                span,
                requested_name,
                selected,
                &rename_index.scopes_by_file,
                &rename_index.handler_files,
            ))
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
        ) {
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
            && handler_function_reference_is_unshadowed(
                file,
                &file.tokens,
                index,
                requested_name,
            )
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
