impl SymbolIndex {
    fn rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        if result.is_recovery {
            return self.recovery_rename_conflict(result, requested_name);
        }
        if result.selected_symbol.kind == SymbolKind::ModuleValue {
            return self.module_value_rename_conflict(result, requested_name);
        }
        match result.selected_symbol.kind.rename_name_class() {
            RenameNameClass::CasingNeutral => None,
            RenameNameClass::ImportAlias => {
                self.import_alias_rename_conflict(result, requested_name)
            }
            RenameNameClass::Type => self.type_rename_conflict(result, requested_name),
            RenameNameClass::Constructor => self.constructor_rename_conflict(result, requested_name),
            RenameNameClass::Function => self.function_rename_conflict(result, requested_name),
            RenameNameClass::ValueBinding => self.local_rename_conflict(result, requested_name),
        }
    }

    fn module_value_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let selected = self.selected_module_value(result)?;
        let rename_index = self.function_rename_index();
        self.module_value_owner_conflict(&selected, requested_name)
            .or_else(|| {
                self.module_value_post_rename_visibility_conflict(
                    result,
                    &selected,
                    requested_name,
                )
            })
            .or_else(|| {
                self.module_value_lexical_conflict(result, requested_name, rename_index)
            })
    }

    fn module_value_owner_conflict(
        &self,
        selected: &ModuleValueSymbol,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        self.module_value_candidates(None, None, &selected.module, requested_name)
            .into_iter()
            .find(|candidate| {
                candidate.declaration != selected.declaration
            })
            .map(|candidate| {
                module_rename_conflict(candidate.declaration.clone(), &selected.module)
            })
            .or_else(|| {
                self.function_candidates(None, None, &selected.module, requested_name)
                    .into_iter()
                    .next()
                    .map(|candidate| {
                        module_rename_conflict(candidate.declaration.clone(), &selected.module)
                    })
            })
    }

    fn module_value_lexical_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
        rename_index: &FunctionRenameIndex,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        self.affected_spans(result).into_iter().find_map(|span| {
            let (file, token_index) = self.file_token_for_span(span)?;
            if qualifier_for_token(&file.tokens, token_index).is_some() {
                return None;
            }
            self.local_conflict_in_file(file, requested_name, token_index)
                .or_else(|| {
                    rename_index
                        .handler_files
                        .contains(file.source.path().as_str())
                        .then(|| {
                            handler_binding_conflict_for_function_reference(
                                file,
                                &file.tokens,
                                rename_index
                                    .scopes_by_file
                                    .get(file.source.path().as_str())
                                    .map(Vec::as_slice)
                                    .unwrap_or(&[]),
                                token_index,
                                requested_name,
                            )
                        })
                        .flatten()
                })
            })
    }

    fn module_value_post_rename_visibility_conflict(
        &self,
        result: &NavigationResult,
        selected: &ModuleValueSymbol,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let rename_index = self.function_rename_index();
        let affected_bare_files = self.affected_bare_module_value_files(result);
        self.files
            .iter()
            .filter(|file| module_value_visible_after_rename(file, selected))
            .filter(|file| file.module != selected.module)
            .find_map(|file| {
                let affected_bare_reference =
                    affected_bare_files.contains(file.source.path().as_str());
                let scopes = rename_index
                    .scopes_by_file
                    .get(file.source.path().as_str())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                self.module_value_visibility_conflict_in_file(
                    file,
                    selected,
                    requested_name,
                    affected_bare_reference,
                    scopes,
                )
            })
    }

    fn affected_bare_module_value_files(&self, result: &NavigationResult) -> BTreeSet<String> {
        self.affected_spans(result)
            .into_iter()
            .filter_map(|span| {
                let (file, token_index) = self.file_token_for_span(span)?;
                qualifier_for_token(&file.tokens, token_index)
                    .is_none()
                    .then(|| file.source.path().as_str().to_string())
            })
            .collect()
    }

    fn module_value_visibility_conflict_in_file(
        &self,
        file: &IndexedFile,
        selected: &ModuleValueSymbol,
        requested_name: &str,
        affected_bare_reference: bool,
        scopes: &[FunctionScope],
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        self.local_module_value_visibility_conflict(
            file,
            selected,
            requested_name,
            affected_bare_reference,
        )
        .or_else(|| {
            self.imported_module_value_visibility_conflict(
                file,
                selected,
                requested_name,
                affected_bare_reference,
            )
        })
        .or_else(|| {
            self.imported_function_visibility_conflict(
                file,
                requested_name,
                affected_bare_reference,
                scopes,
            )
        })
        .map(|conflict| module_rename_conflict(conflict, &file.module))
    }

    fn local_module_value_visibility_conflict(
        &self,
        file: &IndexedFile,
        selected: &ModuleValueSymbol,
        requested_name: &str,
        affected_bare_reference: bool,
    ) -> Option<NavigationLocation> {
        affected_bare_reference.then_some(())?;
        self.module_value_candidates(None, None, &file.module, requested_name)
            .into_iter()
            .find(|candidate| {
                candidate.declaration != selected.declaration
            })
            .map(|candidate| candidate.declaration.clone())
            .or_else(|| {
                self.function_candidates(None, None, &file.module, requested_name)
                    .into_iter()
                    .next()
                    .map(|candidate| candidate.declaration.clone())
            })
    }

    fn imported_module_value_visibility_conflict(
        &self,
        file: &IndexedFile,
        selected: &ModuleValueSymbol,
        requested_name: &str,
        affected_bare_reference: bool,
    ) -> Option<NavigationLocation> {
        self.imported_module_value_candidates(file, requested_name)
            .into_iter()
            .find(|candidate| {
                candidate.declaration != selected.declaration
                    && candidate.public
                    && (affected_bare_reference
                        || candidate.references.iter().any(|span| {
                            span.file.as_str() == file.source.path().as_str()
                                && self.file_token_for_span(span).is_some_and(
                                    |(_, token_index)| {
                                        qualifier_for_token(&file.tokens, token_index).is_none()
                                    },
                                )
                        }))
            })
            .map(|candidate| candidate.declaration.clone())
    }

    fn imported_function_visibility_conflict(
        &self,
        file: &IndexedFile,
        requested_name: &str,
        affected_bare_reference: bool,
        scopes: &[FunctionScope],
    ) -> Option<NavigationLocation> {
        let candidate = self
            .imported_function_candidates(file, requested_name)
            .into_iter()
            .find(|candidate| candidate.public)?;
        if affected_bare_reference
            || file.tokens.iter().enumerate().any(|(token_index, token)| {
                token.text == requested_name
                    && is_bare_function_reference_token(
                        &file.tokens,
                        scopes,
                        token_index,
                        requested_name,
                    )
            })
        {
            return Some(candidate.declaration.clone());
        }
        None
    }

    fn module_value_candidates(
        &self,
        package: Option<&str>,
        package_origin: Option<PackageOrigin>,
        module: &str,
        name: &str,
    ) -> Vec<&ModuleValueSymbol> {
        record_module_value_rename_candidate_lookup();
        self.module_value_indices_by_identity
            .get(&(
                package.map(str::to_string),
                package_origin,
                module.to_string(),
                name.to_string(),
            ))
            .into_iter()
            .flatten()
            .map(|index| &self.module_values[*index])
            .collect()
    }

    fn function_candidates(
        &self,
        package: Option<&str>,
        package_origin: Option<PackageOrigin>,
        module: &str,
        name: &str,
    ) -> Vec<&FunctionSymbol> {
        record_module_value_rename_candidate_lookup();
        self.function_indices_by_identity
            .get(&(
                package.map(str::to_string),
                package_origin,
                module.to_string(),
                name.to_string(),
            ))
            .into_iter()
            .flatten()
            .map(|index| &self.functions[*index])
            .collect()
    }

    fn imported_module_value_candidates(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Vec<&ModuleValueSymbol> {
        file
            .bare_workspace_uses
            .iter()
            .flat_map(|module| self.module_value_candidates(None, None, module, name))
            .chain(
                file.bare_external_uses
                    .iter()
                    .flat_map(|(module, package)| {
                        self.module_value_candidates(
                            Some(package),
                            Some(PackageOrigin::DirectDependency),
                            module,
                            name,
                        )
                    }),
            )
            .collect()
    }

    fn imported_function_candidates(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Vec<&FunctionSymbol> {
        file
            .bare_workspace_uses
            .iter()
            .flat_map(|module| self.function_candidates(None, None, module, name))
            .chain(
                file.bare_external_uses
                    .iter()
                    .flat_map(|(module, package)| {
                        self.function_candidates(
                            Some(package),
                            Some(PackageOrigin::DirectDependency),
                            module,
                            name,
                        )
                    }),
            )
            .collect()
    }

    fn import_alias_rename_conflict(
        &self,
        result: &NavigationResult,
        requested_name: &str,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        let selected = self.import_aliases.iter().find(|alias| {
            alias.declaration == result.selected_symbol.declaration
        })?;
        self.import_namespace_bindings
            .iter()
            .find(|binding| {
                binding.module == selected.module
                    && binding.qualifier == requested_name
                    && binding.selected_alias_declaration.as_ref()
                        != Some(&selected.declaration.span)
            })
            .map(|binding| {
                (
                    binding.declaration.clone(),
                    RenameAffectedScope::Module {
                        name: selected.module.clone(),
                    },
                )
            })
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
                            && file.token_is_constructor_reference(*index)
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
                                    && binding.end == selected.scope_end
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
