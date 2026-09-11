impl SymbolIndex {
    fn schema_references(&self, symbol: &NeutralSymbol) -> Vec<SourceSpan> {
        if symbol.package.is_some() {
            return Vec::new();
        }
        self.files
            .iter()
            .filter(|file| workspace_navigation_file(file))
            .flat_map(|file| {
                file.tokens
                    .iter()
                    .enumerate()
                    .filter(|(index, token)| {
                        token.kind == TokenKind::Ident
                            && token.text == symbol.name
                            && is_schema_path_leaf_token(&file.tokens, *index)
                            && self
                                .schema_for_reference(file, &file.tokens, *index, &token.text)
                                .is_some_and(|candidate| same_schema(&candidate, symbol))
                    })
                    .map(|(_, token)| file.source.span(token.range))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn local_references(&self, symbol: &LocalSymbol, include_declaration: bool) -> Vec<SourceSpan> {
        let Some(file) = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == symbol.scope_file)
        else {
            return Vec::new();
        };
        let tokens = lex(&file.source).tokens;
        let mut spans = Vec::new();
        if include_declaration {
            spans.push(symbol.declaration.clone());
        }
        spans.extend(
            tokens
                .iter()
                .enumerate()
                .filter(|(index, token)| {
                    token.text == symbol.name
                        && token.kind == TokenKind::Ident
                        && token.range.start >= symbol.scope_start
                        && token.range.start < symbol.scope_end
                        && !is_field_name(&tokens, *index)
                        && !is_local_binding_name(&tokens, *index)
                        && (symbol.kind != LocalSymbolKind::HandlerContextParameter
                            || inside_handler_operation_clause_body(&tokens, token.range.start))
                        && !local_binding_shadows_other_name(
                            &tokens,
                            &symbol.name,
                            token.range.start,
                            symbol.scope_start,
                            symbol.scope_end,
                            symbol.declaration.start.offset,
                        )
                        && (symbol.kind != LocalSymbolKind::HandlerContextParameter
                            || !handler_operation_clause_parameter_shadows_name(
                                &tokens,
                                &symbol.name,
                                token.range.start,
                                symbol.scope_start,
                                symbol.scope_end,
                            ))
                })
                .map(|(_, token)| file.source.span(token.range)),
        );
        spans.sort_by_key(|span| span.start.offset);
        spans.dedup_by_key(|span| (span.start.offset, span.end.offset));
        spans
    }

    fn references_in_file(&self, file: &IndexedFile, symbol: &FunctionSymbol) -> Vec<SourceSpan> {
        if !matches!(file.origin, IndexedOrigin::Workspace) {
            return Vec::new();
        }
        if symbol.package.is_none() && file.module == symbol.module {
            return call_references(file, &symbol.name);
        }
        let mut references = Vec::new();
        if symbol.standard_prelude {
            references.extend(self.bare_prelude_function_references(file, symbol));
        }
        if self.function_references_visible_from(file, symbol) {
            references.extend(
                self
                .qualifiers_for_module(file, &symbol.module, symbol.package.as_deref())
                .into_iter()
                .flat_map(|qualifier| {
                    self.qualified_function_references(file, &qualifier, symbol)
                }),
            );
        }
        references
    }

    fn function_references(&self, symbol: &FunctionSymbol) -> Vec<SourceSpan> {
        if !self.supports_function_reference_symbol(symbol) {
            return Vec::new();
        }
        self.files
            .iter()
            .filter(|file| workspace_navigation_file(file))
            .flat_map(|file| self.references_in_file(file, symbol))
            .collect()
    }

    fn supports_function_reference_symbol(&self, symbol: &FunctionSymbol) -> bool {
        match symbol.declaration_kind {
            SymbolDeclarationKind::Declaration => true,
            SymbolDeclarationKind::Recovery => false,
            SymbolDeclarationKind::PublicAlias => {
                symbol.package.is_some() && !self.function_alias_targets_alias(symbol)
            }
        }
    }

    fn function_alias_targets_alias(&self, symbol: &FunctionSymbol) -> bool {
        let alias = self
            .function_aliases
            .iter()
            .find(|alias| {
                alias.package == symbol.package
                    && alias.module == symbol.module
                    && alias.name == symbol.name
            });
        let Some(target_name) = alias
            .and_then(|alias| alias.target_name.as_deref())
            .or(symbol.alias_target_name.as_deref())
        else {
            return false;
        };
        let target_modules = alias
            .map(|alias| function_alias_target_modules(alias))
            .unwrap_or_else(|| {
                vec![
                    symbol
                        .alias_target_module
                        .clone()
                        .unwrap_or_else(|| symbol.module.clone()),
                ]
            });
        self.function_aliases.iter().any(|candidate| {
            candidate.package == symbol.package
                && target_modules.iter().any(|module| module == &candidate.module)
                && candidate.name == target_name
        })
    }

    fn bare_prelude_function_references(
        &self,
        file: &IndexedFile,
        symbol: &FunctionSymbol,
    ) -> Vec<SourceSpan> {
        let tokens = lex(&file.source).tokens;
        tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                token.text == symbol.name
                    && previous_non_layout_token(&tokens, *index)
                        .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
                    && is_call_target_token(&tokens, *index)
                    && self
                        .symbol_for_bare_call(file, &tokens, *index, &token.text)
                        .is_some_and(|candidate| {
                            matches!(candidate, Symbol::Function(candidate) if same_function(&candidate, symbol))
                        })
            })
            .map(|(_, token)| file.source.span(token.range))
            .collect()
    }

    fn function_references_visible_from(
        &self,
        file: &IndexedFile,
        symbol: &FunctionSymbol,
    ) -> bool {
        match &symbol.package {
            Some(package) => {
                symbol.public
                    && (symbol.standard_prelude
                        || file
                            .external_uses
                            .contains(&(symbol.module.clone(), package.clone())))
            }
            None => {
                file.uses.contains(&symbol.module)
                    && (symbol.public
                        || file
                            .companion_target_module
                            .as_ref()
                            .is_some_and(|target| target == &symbol.module))
            }
        }
    }

    fn qualified_function_references(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        symbol: &FunctionSymbol,
    ) -> Vec<SourceSpan> {
        let tokens = lex(&file.source).tokens;
        let module_segments = qualifier.split("::").collect::<Vec<_>>();
        tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                token.text == symbol.name
                    && qualified_reference_matches(&tokens, *index, &module_segments)
                    && (is_call_target_token(&tokens, *index)
                        || ((symbol.package.is_some() || symbol.public)
                            && (file.classified_path_segments.iter().any(|segment| {
                                segment.role == NameClass::ValueBinding
                                    && same_span(&segment.span, &file.source.span(token.range))
                            }) || is_qualified_function_value_token(&tokens, *index))))
                    && self
                        .function_for_qualified_call(file, qualifier, &token.text)
                        .is_some_and(|candidate| same_function(&candidate, symbol))
            })
            .map(|(_, token)| file.source.span(token.range))
            .collect()
    }

    fn type_references(&self, symbol: &TypeSymbol) -> Vec<SourceSpan> {
        self.files
            .iter()
            .filter(|file| workspace_navigation_file(file))
            .flat_map(|file| {
                let tokens = &file.tokens;
                let mut spans = file
                    .type_reference_spans(&symbol.name)
                    .into_iter()
                    .filter_map(|(token_index, span)| {
                        self.visible_type_for_reference(file, tokens, token_index, &symbol.name)
                            .is_some_and(|candidate| same_type(&candidate, symbol))
                            .then_some(span)
                    })
                    .collect::<Vec<_>>();
                spans.extend(self.constructor_type_qualifier_references(file, tokens, symbol));
                spans
            })
            .collect()
    }

    fn constructor_type_qualifier_references(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        symbol: &TypeSymbol,
    ) -> Vec<SourceSpan> {
        tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| token.kind == TokenKind::Ident && token.text == symbol.name)
            .filter(|(index, token)| {
                self.type_for_constructor_qualifier_token(file, tokens, *index, &token.text)
                    .is_some_and(|candidate| same_type(&candidate, symbol))
            })
            .map(|(_, token)| file.source.span(token.range))
            .collect()
    }

    fn constructor_references(&self, symbol: &ConstructorSymbol) -> Vec<SourceSpan> {
        if symbol.declaration_kind != SymbolDeclarationKind::Declaration {
            return Vec::new();
        }
        #[cfg(test)]
        record_constructor_reference_collection();
        self.files
            .iter()
            .filter(|file| workspace_navigation_file(file))
            .flat_map(|file| {
                let tokens = lex(&file.source).tokens;
                tokens
                    .iter()
                    .enumerate()
                    .filter(|(index, token)| {
                        token.kind == TokenKind::Ident
                            && token.text == symbol.name
                            && !same_span(&file.source.span(token.range), &symbol.declaration.span)
                            && is_constructor_reference_token(&tokens, *index)
                            && self
                                .constructor_symbol_for_call(file, &tokens, *index, &token.text)
                                .is_some_and(|candidate| {
                                    candidate.declaration_kind == SymbolDeclarationKind::Declaration
                                        && same_constructor(&candidate, symbol)
                                })
                    })
                    .map(|(_, token)| file.source.span(token.range))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn constructor_symbol_for_call(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        match qualifier_for_token(tokens, token_index) {
            Some(qualifier) => self.constructor_for_qualified_call(file, &qualifier, name),
            None => self.constructor_for_bare_call(file, name),
        }
    }

    fn recovery_references(&self, symbol: &RecoverySymbol) -> Vec<SourceSpan> {
        let Some(file) = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == symbol.source_file)
        else {
            return Vec::new();
        };
        #[cfg(test)]
        record_function_scope_collection();
        let scopes = function_scopes(&file.tokens);
        file.tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                token.kind == TokenKind::Ident
                    && token.text == symbol.name
                    && !same_span(&file.source.span(token.range), &symbol.declaration)
                    && self
                        .symbol_for_selection(
                            file,
                            &file.tokens,
                            *index,
                            &token.text,
                            &file.source.span(token.range),
                            Some(&scopes),
                        )
                        .is_some_and(|selected| {
                            matches!(
                                selected.symbol,
                                Symbol::Recovery(ref candidate)
                                    if same_recovery_symbol(candidate, symbol)
                            )
                        })
            })
            .map(|(_, token)| file.source.span(token.range))
            .collect()
    }

    fn constructor_conflict_for_call(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        match qualifier_for_token(tokens, token_index) {
            Some(qualifier) => self.constructor_for_qualified_call(file, &qualifier, name),
            None => self.constructor_conflict_for_bare_call(file, name),
        }
    }

    fn constructor_conflict_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.local_constructor_for_bare_call(file, name)
            .or_else(|| {
                self.first_constructor_matching(|symbol| {
                    symbol.name == name
                        && !symbol.standard_prelude
                        && symbol.package.is_none()
                        && symbol.module != file.module
                        && (file.uses.contains(&symbol.module)
                            || self.constructor_reexport_visible_from(file, symbol, None))
                        && visible_workspace_constructor_from(file, symbol)
                })
            })
            .or_else(|| {
                self.first_constructor_matching(|symbol| {
                    symbol.name == name
                        && !symbol.standard_prelude
                        && symbol.public
                        && symbol.package.as_ref().is_some_and(|package| {
                            file.external_uses
                                .contains(&(symbol.module.clone(), package.clone()))
                                || self.constructor_reexport_visible_from(
                                    file,
                                    symbol,
                                    Some(package),
                                )
                        })
                })
            })
    }

    fn qualified_module_candidates(&self, file: &IndexedFile, qualifier: &str) -> Vec<String> {
        let mut modules = vec![qualifier.to_string()];
        if let Some(module) = resolve_qualified_alias(&file.import_aliases, qualifier) {
            modules.push(module);
        }
        if let Some((module, _package)) =
            resolve_external_qualified_alias(&file.external_import_aliases, qualifier)
        {
            modules.push(module);
        }
        modules
    }

    fn qualifiers_for_module(
        &self,
        file: &IndexedFile,
        module: &str,
        package: Option<&str>,
    ) -> Vec<String> {
        let mut qualifiers = vec![module.to_string()];
        match package {
            Some(package) => qualifiers.extend(
                file.external_import_aliases
                    .iter()
                    .filter(|(_, (target_module, target_package))| {
                        target_module == module && target_package == package
                    })
                    .map(|(alias, _)| alias.clone()),
            ),
            None => qualifiers.extend(
                file.import_aliases
                    .iter()
                    .filter(|(_, target)| *target == module)
                    .map(|(alias, _)| alias.clone()),
            ),
        }
        qualifiers
    }
}

fn function_alias_target_modules(alias: &FunctionAliasSymbol) -> Vec<String> {
    match alias.target_module.as_deref() {
        Some(qualifier) => {
            let mut modules = vec![qualifier.to_string()];
            if let Some(module) = resolve_qualified_alias(&alias.import_aliases, qualifier) {
                modules.push(module);
            }
            modules
        }
        None => vec![alias.module.clone()],
    }
}

fn is_qualified_function_value_token(tokens: &[Token], token_index: usize) -> bool {
    previous_non_layout_token(tokens, token_index)
        .is_some_and(|previous| previous.kind == TokenKind::DoubleColon)
        && next_non_layout_token(tokens, token_index)
            .is_none_or(|next| next.kind != TokenKind::DoubleColon && next.kind != TokenKind::LParen)
}

fn same_recovery_symbol(left: &RecoverySymbol, right: &RecoverySymbol) -> bool {
    left.kind == right.kind
        && left.source_file == right.source_file
        && same_span(&left.declaration, &right.declaration)
}

fn workspace_navigation_file(file: &IndexedFile) -> bool {
    matches!(file.origin, IndexedOrigin::Workspace) && !file.navigation_isolated
}
