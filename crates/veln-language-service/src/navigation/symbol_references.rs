impl SymbolIndex {
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
                        && !local_binding_shadows_name(
                            &tokens,
                            &symbol.name,
                            token.range.start,
                            symbol.scope_start,
                            symbol.scope_end,
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
        if symbol.package.is_some() {
            return Vec::new();
        }
        if !matches!(file.origin, IndexedOrigin::Workspace) {
            return Vec::new();
        }
        if file.module == symbol.module {
            return call_references(file, &symbol.name);
        }
        if file.uses.contains(&symbol.module)
            && (symbol.public
                || file
                    .companion_target_module
                    .as_ref()
                    .is_some_and(|target| target == &symbol.module))
        {
            return self
                .qualifiers_for_module(file, &symbol.module, symbol.package.as_deref())
                .into_iter()
                .flat_map(|qualifier| {
                    self.qualified_function_references(file, &qualifier, symbol)
                })
                .collect();
        }
        Vec::new()
    }

    fn function_references(&self, symbol: &FunctionSymbol) -> Vec<SourceSpan> {
        self.files
            .iter()
            .flat_map(|file| self.references_in_file(file, symbol))
            .collect()
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
                        || file.classified_path_segments.iter().any(|segment| {
                            segment.role == NameClass::ValueBinding
                                && same_span(&segment.span, &file.source.span(token.range))
                        }))
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
            .filter(|file| matches!(file.origin, IndexedOrigin::Workspace))
            .flat_map(|file| {
                let tokens = lex(&file.source).tokens;
                let mut spans = type_reference_spans(&file.source, &tokens, &symbol.name)
                    .into_iter()
                    .filter_map(|(token_index, span)| {
                        self.visible_type_for_reference(file, &tokens, token_index, &symbol.name)
                            .is_some_and(|candidate| same_type(&candidate, symbol))
                            .then_some(span)
                    })
                    .collect::<Vec<_>>();
                spans.extend(self.constructor_type_qualifier_references(file, &tokens, symbol));
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
        self.files
            .iter()
            .filter(|file| matches!(file.origin, IndexedOrigin::Workspace))
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
                                .is_some_and(|candidate| same_constructor(&candidate, symbol))
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

fn same_recovery_symbol(left: &RecoverySymbol, right: &RecoverySymbol) -> bool {
    left.kind == right.kind
        && left.source_file == right.source_file
        && same_span(&left.declaration, &right.declaration)
}
