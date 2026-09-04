impl SymbolIndex {
    fn symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> Option<SelectedNavigationSymbol> {
        if let Some(symbol) =
            handler_operation_clause_symbol(file, tokens, token_index, name, selection)
        {
            return Some(SelectedNavigationSymbol::bare(Symbol::Local(symbol)));
        }
        if is_handler_operation_clause_operation_name(tokens, token_index) {
            return None;
        }
        self.declaration_symbol_for_selection(file, tokens, token_index, name, selection)
            .or_else(|| self.bare_nullary_constructor_pattern_symbol(file, tokens, token_index, name))
            .or_else(|| {
                self.local_or_recovery_declaration_symbol(
                    file,
                    tokens,
                    token_index,
                    name,
                    selection,
                    prepared_scopes,
                )
            })
            .or_else(|| {
                self.non_declaration_symbol_for_selection(
                    file,
                    tokens,
                    token_index,
                    name,
                    selection,
                    prepared_scopes,
                )
            })
    }

    fn bare_nullary_constructor_pattern_symbol(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<SelectedNavigationSymbol> {
        is_bare_nullary_constructor_pattern(tokens, token_index)
            .then(|| self.constructor_symbol_for_call(file, tokens, token_index, name))
            .flatten()
            .map(Symbol::Constructor)
            .map(SelectedNavigationSymbol::bare)
    }

    fn declaration_symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
        self.declared_item_symbol(name, selection)
            .or_else(|| {
                self.neutral_role_symbol_for_selection(file, tokens, token_index, name, selection)
            })
            .map(SelectedNavigationSymbol::bare)
    }

    fn declared_item_symbol(&self, name: &str, selection: &SourceSpan) -> Option<Symbol> {
        self.function_declared_at(name, selection)
            .map(Symbol::Function)
            .or_else(|| self.type_declared_at(name, selection).map(Symbol::Type))
            .or_else(|| {
                self.constructor_declared_at(name, selection)
                    .map(Symbol::Constructor)
            })
    }

    fn local_or_recovery_declaration_symbol(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> Option<SelectedNavigationSymbol> {
        self.local_binding_symbol_for_selection(file, tokens, token_index, name, prepared_scopes)
            .map(|symbol| SelectedNavigationSymbol::bare(Symbol::Local(symbol)))
            .or_else(|| {
                self.recovery_declared_at(file, tokens, token_index, name, selection)
                    .map(|symbol| SelectedNavigationSymbol::bare(Symbol::Recovery(symbol)))
            })
    }

    fn local_binding_symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> Option<LocalSymbol> {
        if is_field_name(tokens, token_index) || is_local_binding_name(tokens, token_index) {
            return None;
        }
        let owned_scopes;
        let scopes = match prepared_scopes {
            Some(scopes) => scopes,
            None => {
                owned_scopes = function_scopes(tokens);
                &owned_scopes
            }
        };
        let scope = scopes
            .iter()
            .find(|scope| {
                let offset = tokens[token_index].range.start;
                offset >= scope.body_start && offset < scope.end
            })?;
        let shadow = scope.shadowing_binding(name, tokens, token_index)?;
        let (declaration_start, declaration_end) = shadow.declaration_range();
        let declaration = file
            .source
            .span(TextRange::new(declaration_start, declaration_end));
        if is_invalid_declaration_name(file, &declaration) {
            return None;
        }
        Some(LocalSymbol {
            name: name.to_string(),
            declaration,
            scope_file: file.source.path().as_str().to_string(),
            scope_start: scope.body_start,
            scope_end: scope.end,
            declaration_scope_start: scope.body_start,
            declaration_scope_end: scope.end,
            kind: LocalSymbolKind::ValueBinding,
        })
    }

    fn neutral_role_symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<Symbol> {
        self.neutral_declaration_symbol(name, selection)
            .or_else(|| self.neutral_reference_symbol(file, tokens, token_index, name))
    }

    fn neutral_declaration_symbol(&self, name: &str, selection: &SourceSpan) -> Option<Symbol> {
        self.schema_declared_at(name, selection)
            .map(Symbol::Schema)
            .or_else(|| self.effect_declared_at(name, selection).map(Symbol::Effect))
            .or_else(|| self.handler_declared_at(name, selection).map(Symbol::Handler))
            .or_else(|| {
                self.operation_declared_at(name, selection)
                    .map(Symbol::EffectOperation)
            })
    }

    fn neutral_reference_symbol(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<Symbol> {
        if is_schema_path_leaf_token(tokens, token_index) {
            return self
                .schema_for_reference(file, tokens, token_index, name)
                .map(Symbol::Schema);
        }
        if is_effect_reference_token(tokens, token_index)
            || is_perform_effect_qualifier_token(tokens, token_index)
        {
            return self.effect_for_reference(file, name).map(Symbol::Effect);
        }
        if is_perform_operation_token(tokens, token_index) {
            let qualifier = qualifier_for_token(tokens, token_index)?;
            return self
                .operation_for_qualified_perform(file, &qualifier, name)
                .map(Symbol::EffectOperation);
        }
        if is_handler_reference_token(tokens, token_index) {
            return self.handler_for_reference(file, name).map(Symbol::Handler);
        }
        None
    }

    fn non_declaration_symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> Option<SelectedNavigationSymbol> {
        if let Some(symbol) =
            self.qualified_segment_selection(file, tokens, token_index, name, selection)
        {
            return Some(symbol);
        }
        if is_qualified_path_token(tokens, token_index)
            && !is_call_target_token(tokens, token_index)
            && !is_constructor_reference_token(tokens, token_index)
        {
            return None;
        }
        if is_call_target_token(tokens, token_index) {
            self.call_target_selection(file, tokens, token_index, name, prepared_scopes)
        } else {
            self.non_call_target_selection(file, tokens, token_index, name, selection)
        }
    }

    fn qualified_segment_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
        let segment = self.classified_qualified_segment(file, tokens, token_index, name, selection)?;
        segment.clone().into_selected_symbol().or_else(|| {
            self.unique_recovery_for_role(file, tokens, token_index, name, segment.segment.role)
                .map(|symbol| SelectedNavigationSymbol {
                    symbol: Symbol::Recovery(symbol),
                    classified_path_segment: Some(segment.segment),
                })
        })
    }

    fn non_call_target_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
        self.type_reference_selection(file, tokens, token_index, name, selection)
            .or_else(|| {
                self.recovery_type_reference_selection(file, tokens, token_index, name, selection)
            })
            .or_else(|| self.bare_nullary_constructor_selection(file, tokens, token_index, name))
            .or_else(|| self.recovery_value_binding_selection(file, tokens, token_index, name))
    }

    fn call_target_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> Option<SelectedNavigationSymbol> {
        let Some(qualifier) = qualifier_for_token(tokens, token_index) else {
            return self.bare_call_selection(file, tokens, token_index, name, prepared_scopes);
        };
        self.symbol_for_qualified_call(file, &qualifier, name)
            .map(SelectedNavigationSymbol::bare)
    }

    fn bare_call_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> Option<SelectedNavigationSymbol> {
        self.local_binding_symbol_for_selection(file, tokens, token_index, name, prepared_scopes)
            .map(|symbol| SelectedNavigationSymbol::bare(Symbol::Local(symbol)))
            .or_else(|| self.symbol_for_bare_call(file, tokens, token_index, name)
            .map(SelectedNavigationSymbol::bare)
            .or_else(|| {
                (!self.bare_call_recovery_blocked(file, tokens, token_index, name, prepared_scopes))
                    .then(|| self.recovery_bare_call_selection(file, tokens, token_index, name))
                    .flatten()
            }))
    }

    fn type_reference_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
        is_type_reference_token(file, name, selection)
            .then(|| self.visible_type_for_reference(file, tokens, token_index, name))
            .flatten()
            .map(Symbol::Type)
            .map(SelectedNavigationSymbol::bare)
    }

    fn recovery_type_reference_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
        is_type_reference_token_named(file, name, selection)
            .then(|| self.unique_recovery_for_role(file, tokens, token_index, name, NameClass::Type))
            .flatten()
            .map(Symbol::Recovery)
            .map(SelectedNavigationSymbol::bare)
    }

    fn recovery_bare_call_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<SelectedNavigationSymbol> {
        self.unique_recovery_for_roles(
            file,
            tokens,
            token_index,
            name,
            &[
                NameClass::Constructor,
                NameClass::Function,
                NameClass::ValueBinding,
            ],
        )
        .map(Symbol::Recovery)
        .map(SelectedNavigationSymbol::bare)
    }

    fn bare_nullary_constructor_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<SelectedNavigationSymbol> {
        is_constructor_reference_token(tokens, token_index)
            .then(|| self.constructor_symbol_for_call(file, tokens, token_index, name))
            .flatten()
            .map(Symbol::Constructor)
            .map(SelectedNavigationSymbol::bare)
    }

    fn recovery_value_binding_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<SelectedNavigationSymbol> {
        if is_field_name(tokens, token_index)
            || is_local_binding_name(tokens, token_index)
            || is_parameter_name(tokens, token_index)
        {
            return None;
        }
        self.unique_recovery_for_role(file, tokens, token_index, name, NameClass::ValueBinding)
            .map(Symbol::Recovery)
            .map(SelectedNavigationSymbol::bare)
    }

    fn classified_qualified_segment(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<ClassifiedNavigationSegment> {
        if previous_non_layout_token(tokens, token_index)
            .is_none_or(|token| token.kind != TokenKind::DoubleColon)
            && next_non_layout_token(tokens, token_index)
                .is_none_or(|token| token.kind != TokenKind::DoubleColon)
        {
            return None;
        }

        if let Some(segment) = file
            .classified_path_segments
            .iter()
            .find(|segment| same_span(&segment.span, selection))
        {
            let symbol = self.symbol_for_classified_segment(file, tokens, token_index, name, segment);
            return Some(ClassifiedNavigationSegment {
                segment: segment.clone(),
                symbol,
            });
        }
        None
    }

    fn symbol_for_classified_segment(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        segment: &QualifiedPathSegment,
    ) -> Option<Symbol> {
        match segment.role {
            NameClass::Type => self
                .visible_type_for_reference(file, tokens, token_index, name)
                .map(Symbol::Type),
            NameClass::Constructor => self
                .qualified_call_symbol(file, tokens, token_index, name, SymbolIndex::constructor_symbol)
                .map(Symbol::Constructor),
            NameClass::Function | NameClass::ValueBinding => self
                .qualified_call_symbol(file, tokens, token_index, name, SymbolIndex::function_symbol)
                .map(Symbol::Function),
            _ => None,
        }
    }

    fn qualified_call_symbol<T>(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        lookup: fn(&Self, &IndexedFile, &str, &str) -> Option<T>,
    ) -> Option<T> {
        let qualifier = qualifier_for_token(tokens, token_index)?;
        lookup(self, file, &qualifier, name)
    }

    fn constructor_symbol(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.constructor_for_qualified_call(file, qualifier, name)
    }

    fn function_symbol(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<FunctionSymbol> {
        self.function_for_qualified_call(file, qualifier, name)
    }

    fn recovery_declared_at(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<RecoverySymbol> {
        let roles = recovery_roles_for_declaration_token(tokens, token_index)?;
        let selected = dedup_recovery_symbols(
            file.recovery_symbols
                .iter()
                .filter(|symbol| recovery_matches_declaration(symbol, file, name, &roles, selection))
                .cloned()
                .collect(),
        );
        let selected_symbol = selected.first()?;
        let visible = dedup_recovery_symbols(
            file.recovery_symbols
                .iter()
                .filter(|symbol| recovery_visible_to_selected(symbol, file, name, &roles, &selected))
                .cloned()
                .collect(),
        );
        (visible.len() == 1).then(|| selected_symbol.clone())
    }

    fn unique_recovery_for_role(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        role: NameClass,
    ) -> Option<RecoverySymbol> {
        self.unique_recovery_for_roles(file, tokens, token_index, name, &[role])
    }

    fn unique_recovery_for_roles(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        roles: &[NameClass],
    ) -> Option<RecoverySymbol> {
        let offset = tokens[token_index].range.start;
        let mut candidates = file.recovery_symbols.iter().filter(|symbol| {
            symbol.name == name
                && symbol.source_file == file.source.path().as_str()
                && offset >= symbol.scope_start
                && offset < symbol.scope_end
                && symbol.name_class().is_some_and(|class| {
                    roles
                        .iter()
                        .any(|role| recovery_roles_compatible(class, *role))
                })
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn bare_call_recovery_blocked(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> bool {
        self.bare_call_shadowed_by_distinct_binding(
            file,
            tokens,
            token_index,
            name,
            prepared_scopes,
        )
            || self.has_visible_non_prelude_imported_function(file, name)
            || self.has_visible_non_prelude_imported_constructor(file, name)
    }

    fn bare_call_shadowed_by_distinct_binding(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        prepared_scopes: Option<&[FunctionScope]>,
    ) -> bool {
        let owned_scopes;
        let scopes = match prepared_scopes {
            Some(scopes) => scopes,
            None => {
                owned_scopes = function_scopes(tokens);
                &owned_scopes
            }
        };
        let Some(shadow) =
            local_binding_shadowing_call_target_in_scopes(scopes, tokens, token_index, name)
        else {
            return false;
        };
        !self.shadow_matches_visible_recovery_binding(file, tokens, token_index, name, &shadow)
    }

    fn shadow_matches_visible_recovery_binding(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        shadow: &ScopeShadow<'_>,
    ) -> bool {
        let offset = tokens[token_index].range.start;
        let (declaration_start, declaration_end) = shadow.declaration_range();
        file.recovery_symbols.iter().any(|symbol| {
            symbol.name == name
                && symbol.source_file == file.source.path().as_str()
                && symbol.kind == SymbolKind::ValueBinding
                && symbol.declaration.start.offset == declaration_start
                && symbol.declaration.end.offset == declaration_end
                && offset >= symbol.scope_start
                && offset < symbol.scope_end
        })
    }
}
