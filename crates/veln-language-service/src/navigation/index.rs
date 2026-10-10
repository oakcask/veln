impl SymbolIndex {
    pub(crate) fn workspace_schema_alias_is_eligible(
        &self,
        module: &str,
        name: &str,
        declaration: &NavigationLocation,
    ) -> bool {
        self.schema_aliases.iter().any(|candidate| {
            candidate.package.is_none()
                && candidate.module == module
                && candidate.name == name
                && candidate.declaration == *declaration
        })
    }

    fn schema_composition_symbol_at(
        &self,
        file: &IndexedFile,
        token: &Token,
    ) -> Option<Symbol> {
        self.schema_composition_references
            .iter()
            .find(|reference| {
                reference.span.file == *file.source.path()
                    && reference.span.start.offset == token.range.start
                    && reference.span.end.offset == token.range.end
            })
            .map(|reference| match &reference.target {
                SchemaReferenceTarget::Schema(symbol) => Symbol::Schema(symbol.clone()),
                SchemaReferenceTarget::Alias(symbol) => Symbol::SchemaAlias(symbol.clone()),
            })
    }

    fn symbol_at_position(
        self: Arc<Self>,
        source_path: &str,
        position: &SourcePosition,
    ) -> Option<SymbolRequest> {
        let file = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == source_path)?;
        if file.navigation_isolated {
            return None;
        }
        let offset = offset_for_position(file.source.text(), position)?;
        let tokens = &file.tokens;
        let (token_index, token) = identifier_token_at(tokens, offset)?;
        let selection = file.source.span(token.range);
        let name = file
            .source
            .text()
            .get(selection.start.offset..selection.end.offset)?
            .to_string();
        let selected =
            self.symbol_for_selection(file, tokens, token_index, &name, &selection, None)?;
        let references_supported = !matches!(
            &selected.symbol,
            Symbol::Schema(symbol)
                if matches!(
                    symbol.package_origin,
                    Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                )
                    && is_schema_operation_path_leaf_candidate_token(tokens, token_index)
                    && !is_schema_operation_path_leaf_token(file, token_index)
        ) && !matches!(
            &selected.symbol,
            Symbol::SchemaAlias(symbol)
                if matches!(
                    symbol.package_origin,
                    Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                )
                    && is_schema_operation_path_leaf_candidate_token(tokens, token_index)
                    && !is_schema_operation_path_leaf_token(file, token_index)
        );
        Some(SymbolRequest {
            index: self,
            symbol: selected.symbol,
            selection,
            classified_path_segment: selected.classified_path_segment,
            references_supported,
        })
    }

    fn selected_type(&self, result: &NavigationResult) -> Option<TypeSymbol> {
        self.types
            .iter()
            .find(|symbol| {
                symbol.package.is_none()
                    && symbol.declaration == result.selected_symbol.declaration
            })
            .cloned()
            .or_else(|| {
                self.type_aliases
                    .iter()
                    .find(|symbol| {
                        symbol.package.is_none()
                            && symbol.declaration == result.selected_symbol.declaration
                    })
                    .map(|symbol| TypeSymbol {
                        module: symbol.module.clone(),
                        name: symbol.name.clone(),
                        declaration: symbol.declaration.clone(),
                        package: None,
                        package_origin: None,
                        public: true,
                        standard_prelude: symbol.standard_prelude,
                    })
            })
    }

    fn selected_type_alias(&self, result: &NavigationResult) -> Option<TypeAliasSymbol> {
        self.type_aliases
            .iter()
            .find(|symbol| {
                symbol.package.is_none()
                    && symbol.declaration == result.selected_symbol.declaration
            })
            .cloned()
    }

    fn selected_constructor(&self, result: &NavigationResult) -> Option<ConstructorSymbol> {
        self.constructors
            .iter()
            .find(|symbol| {
                symbol.package.is_none()
                    && symbol.declaration == result.selected_symbol.declaration
            })
            .cloned()
    }

    fn selected_function(&self, result: &NavigationResult) -> Option<FunctionSymbol> {
        self.functions
            .iter()
            .find(|symbol| {
                symbol.package.is_none()
                    && symbol.declaration == result.selected_symbol.declaration
            })
            .cloned()
    }

    fn selected_local(&self, result: &NavigationResult) -> Option<LocalSymbol> {
        let NavigationSource::Workspace = result.selected_symbol.declaration.source else {
            return None;
        };
        self.files.iter().find_map(|file| {
            handler_operation_clause_bindings(file, &file.tokens)
                .into_iter()
                .find(|binding| {
                    same_span(
                        &binding.declaration,
                        &result.selected_symbol.declaration.span,
                    )
                })
                .map(|binding| LocalSymbol {
                    name: binding.name,
                    declaration: binding.declaration,
                    scope_file: file.source.path().as_str().to_string(),
                    scope_start: binding.start,
                    scope_end: binding.end,
                    declaration_scope_start: binding.start,
                    declaration_scope_end: binding.end,
                    kind: binding.kind,
                })
                .or_else(|| {
                    function_scopes(&file.tokens)
                        .into_iter()
                        .find_map(|scope| {
                            scope
                                .params
                                .iter()
                                .chain(scope.result_binding.iter())
                                .find_map(|binding| {
                                    let declaration = scoped_binding_declaration(file, binding);
                                    same_span(
                                        &declaration,
                                        &result.selected_symbol.declaration.span,
                                    )
                                    .then(|| {
                                        (
                                            binding.name.clone(),
                                            declaration,
                                            scope.body_start,
                                            scope.end,
                                        )
                                    })
                                })
                                .or_else(|| {
                                    scope.local_bindings.iter().find_map(|binding| {
                                        let declaration =
                                            local_binding_declaration(file, binding);
                                        same_span(
                                            &declaration,
                                            &result.selected_symbol.declaration.span,
                                        )
                                        .then(|| {
                                            (
                                                binding.name.clone(),
                                                declaration,
                                                binding.start,
                                                binding.end,
                                            )
                                        })
                                    })
                                })
                                .map(|(name, declaration, scope_start, scope_end)| LocalSymbol {
                                    name,
                                    declaration,
                                    scope_file: file.source.path().as_str().to_string(),
                                    scope_start,
                                    scope_end,
                                    declaration_scope_start: scope.body_start,
                                    declaration_scope_end: scope.end,
                                    kind: LocalSymbolKind::ValueBinding,
                                })
                        })
                })
        })
    }

    fn affected_spans<'a>(&self, result: &'a NavigationResult) -> Vec<&'a SourceSpan> {
        std::iter::once(&result.definition.span)
            .chain(&result.references)
            .collect()
    }

    fn file_token_for_span<'a>(&'a self, span: &SourceSpan) -> Option<(&'a IndexedFile, usize)> {
        let file = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == span.file.as_str())?;
        let token_index = file.tokens.iter().position(|token| {
            token.range.start == span.start.offset && token.range.end == span.end.offset
        })?;
        Some((file, token_index))
    }

    fn local_conflict_in_file(
        &self,
        file: &IndexedFile,
        requested_name: &str,
        span: &SourceSpan,
    ) -> Option<(NavigationLocation, RenameAffectedScope)> {
        function_scopes(&file.tokens)
            .into_iter()
            .find(|scope| {
                span.start.offset >= scope.body_start
                    && span.start.offset < scope.end
                    && scope.shadows(requested_name, &file.tokens, self.token_index_for_span(file, span).unwrap_or(0))
            })
            .map(|scope| {
                let conflict = scope
                    .shadowing_binding(
                        requested_name,
                        &file.tokens,
                        self.token_index_for_span(file, span).unwrap_or(0),
                    )
                    .map(|binding| scope_shadow_declaration(file, binding))
                    .unwrap_or_else(|| span.clone());
                (
                    workspace_location(conflict),
                    RenameAffectedScope::Lexical {
                        file: file.source.path().as_str().to_string(),
                        start_offset: scope.body_start,
                        end_offset: scope.end,
                    },
                )
            })
    }

    fn token_index_for_span(&self, file: &IndexedFile, span: &SourceSpan) -> Option<usize> {
        file.tokens.iter().position(|token| {
            token.range.start == span.start.offset && token.range.end == span.end.offset
        })
    }

    fn function_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<FunctionSymbol> {
        self.functions
            .iter()
            .find(|symbol| {
                (!symbol.invalid_declaration_name || symbol.package.is_some())
                    && declaration_matches(
                        name,
                        selection,
                        &symbol.name,
                        symbol.package.as_deref(),
                        &symbol.declaration.span,
                    )
            })
            .cloned()
    }

    fn type_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<TypeSymbol> {
        self.types
            .iter()
            .find(|symbol| declaration_matches(name, selection, &symbol.name, symbol.package.as_deref(), &symbol.declaration.span))
            .cloned()
    }

    fn constructor_declared_at(
        &self,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<ConstructorSymbol> {
        self.constructors
            .iter()
            .find(|symbol| declaration_matches(name, selection, &symbol.name, symbol.package.as_deref(), &symbol.declaration.span))
            .cloned()
    }

    fn schema_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<NeutralSymbol> {
        self.schemas
            .iter()
            .find(|symbol| {
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn schema_alias_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<NeutralSymbol> {
        self.schema_aliases
            .iter()
            .find(|symbol| {
                // Package declarations are retained for canonical adapter
                // declarations, but package-source occurrences are not
                // navigable selections or reference-set members.
                symbol.package.is_none()
                    &&
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn type_alias_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<TypeAliasSymbol> {
        self.type_aliases
            .iter()
            .find(|symbol| {
                symbol.package.is_none()
                    && declaration_matches(
                        name,
                        selection,
                        &symbol.name,
                        symbol.package.as_deref(),
                        &symbol.declaration.span,
                    )
            })
            .cloned()
    }

    fn effect_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<NeutralSymbol> {
        self.effects
            .iter()
            .find(|symbol| {
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn handler_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<NeutralSymbol> {
        self.handlers
            .iter()
            .find(|symbol| {
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn operation_declared_at(
        &self,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<EffectOperationSymbol> {
        self.operations
            .iter()
            .find(|symbol| {
                declaration_matches(
                    name,
                    selection,
                    &symbol.name,
                    symbol.package.as_deref(),
                    &symbol.declaration.span,
                )
            })
            .cloned()
    }

    fn schema_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<NeutralSymbol> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return match self.schema_alias_qualified_workspace_module(file, &qualifier) {
                QualifiedWorkspaceModule::Workspace(module) => self
                    .schema_operation_lookup_index
                    .workspace_schemas
                    .get(&(module, name.to_string()))?
                    .iter()
                    .find(|symbol| visible_schema_from_workspace_module(file, symbol))
                    .cloned(),
                QualifiedWorkspaceModule::Ambiguous => None,
                QualifiedWorkspaceModule::External => {
                    self.visible_external_schema_for_qualified_reference(file, &qualifier, name)
                }
                QualifiedWorkspaceModule::Unresolved => None,
            };
        }
        self.visible_schema_for_bare_reference(file, name)
    }

    fn schema_alias_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<NeutralSymbol> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return self.visible_schema_alias_for_qualified_reference(file, &qualifier, name);
        }
        self.visible_schema_alias_for_bare_reference(file, name)
    }

    fn effect_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<NeutralSymbol> {
        let qualifier = qualifier_for_token(tokens, token_index);
        let module = match &qualifier {
            Some(qualifier) => match self.effect_qualified_workspace_module(file, qualifier) {
                QualifiedWorkspaceModule::Workspace(module) => module,
                QualifiedWorkspaceModule::Ambiguous
                | QualifiedWorkspaceModule::External
                | QualifiedWorkspaceModule::Unresolved => return None,
            },
            None => file.module.clone(),
        };
        #[cfg(test)]
        record_effect_identity_lookup();
        let candidate = self
            .eligible_workspace_effect_indices
            .get(&(module, name.to_string()))
            .and_then(|index| self.effects.get(*index))?;
        (qualifier.is_none() || candidate.public).then(|| candidate.clone())
    }

    fn effect_qualified_workspace_module(
        &self,
        file: &IndexedFile,
        qualifier: &str,
    ) -> QualifiedWorkspaceModule {
        let Some(imports) = self.schema_alias_module_imports.get(&file.module) else {
            return QualifiedWorkspaceModule::Unresolved;
        };
        let external_exact = imports.external_imports_by_module.get(qualifier);
        if imports.workspace_imports.contains(qualifier) {
            if external_exact.is_some_and(|routes| !routes.is_empty()) {
                return QualifiedWorkspaceModule::Ambiguous;
            }
            return if imports.valid_workspace_imports.contains(qualifier) {
                QualifiedWorkspaceModule::Workspace(qualifier.to_string())
            } else {
                QualifiedWorkspaceModule::Ambiguous
            };
        }
        if external_exact.is_some_and(|routes| !routes.is_empty()) {
            return QualifiedWorkspaceModule::External;
        }
        let workspace_routes = imports.workspace_imports_by_alias.get(qualifier);
        let external_route_count = imports
            .external_imports_by_alias
            .get(qualifier)
            .map_or(0, BTreeSet::len);
        match (
            workspace_routes.map_or(0, BTreeSet::len),
            external_route_count,
        ) {
            (1, 0) => {
                let valid_routes = imports.valid_workspace_imports_by_alias.get(qualifier);
                if valid_routes.map_or(0, BTreeSet::len) != 1 {
                    return QualifiedWorkspaceModule::Ambiguous;
                }
                QualifiedWorkspaceModule::Workspace(
                    valid_routes
                        .and_then(|routes| routes.iter().next())
                        .cloned()
                        .expect("one valid workspace import is present"),
                )
            }
            (0, 1) => QualifiedWorkspaceModule::External,
            (0, 0) => QualifiedWorkspaceModule::Unresolved,
            _ => QualifiedWorkspaceModule::Ambiguous,
        }
    }

    fn handler_for_reference(&self, file: &IndexedFile, name: &str) -> Option<NeutralSymbol> {
        let mut candidates = self
            .handlers
            .iter()
            .filter(|symbol| {
                symbol.name == name && symbol.module == file.module && symbol.package.is_none()
            });
        let candidate = candidates.next()?.clone();
        (candidates.next().is_none() && self.handler_declaration_is_unrecovered(&candidate))
            .then_some(candidate)
    }

    fn handler_declaration_is_unrecovered(&self, symbol: &NeutralSymbol) -> bool {
        self.files.iter().any(|file| {
            workspace_navigation_file(file)
                && file.source.path() == &symbol.declaration.span.file
                && !file.recovered_handler_declarations.iter().any(|span| {
                    span.start.offset <= symbol.declaration.span.start.offset
                        && symbol.declaration.span.end.offset <= span.end.offset
                })
        })
    }

    fn operation_for_qualified_perform(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<EffectOperationSymbol> {
        let effect_index = previous_path_segment_index(tokens, token_index)?;
        let effect = &tokens[effect_index];
        if !file
            .effect_reference_ranges
            .contains(&(effect.range.start, effect.range.end))
            || file.generic_effect_binder_shadows(&effect.text, effect.range.start)
        {
            return None;
        }
        let owning_effect = self.effect_for_reference(
            file,
            tokens,
            effect_index,
            &effect.text,
        )?;
        self.unique_workspace_effect_operation(&owning_effect.module, &owning_effect.name, name)
            .cloned()
    }

    fn operation_for_handler_clause(
        &self,
        file: &IndexedFile,
        token: &Token,
    ) -> Option<EffectOperationSymbol> {
        let clause = file
            .handler_operation_clause_references
            .iter()
            .find(|clause| {
                clause.span.start.offset == token.range.start
                    && clause.span.end.offset == token.range.end
            })?;
        self.handler_for_reference(file, &clause.handler_name)?;
        self.unique_workspace_effect_operation(
            &file.module,
            &clause.effect_name,
            &clause.operation_name,
        )
        .cloned()
    }
}
