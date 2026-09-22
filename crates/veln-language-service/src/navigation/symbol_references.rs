impl SymbolIndex {
    fn effect_operation_references(&self, symbol: &EffectOperationSymbol) -> Vec<SourceSpan> {
        let eligible_handlers = self.effect_operation_reference_handlers();
        self.files
            .iter()
            .filter(|file| workspace_navigation_file(file) && file.module == symbol.module)
            .flat_map(|file| effect_operation_references_in_file(file, symbol, &eligible_handlers))
            .collect()
    }

    fn effect_operation_reference_handlers(&self) -> BTreeSet<(String, String)> {
        let mut handler_counts = BTreeMap::<(String, String), usize>::new();
        for handler in self.handlers.iter().filter(|handler| handler.package.is_none()) {
            *handler_counts
                .entry((handler.module.clone(), handler.name.clone()))
                .or_default() += 1;
        }
        self.handlers
            .iter()
            .filter(|handler| {
                handler.package.is_none()
                    && handler_counts
                        .get(&(handler.module.clone(), handler.name.clone()))
                        == Some(&1)
                    && self.handler_declaration_is_unrecovered(handler)
            })
            .map(|handler| (handler.module.clone(), handler.name.clone()))
            .collect()
    }

    fn effect_operation_references_supported(&self, symbol: &EffectOperationSymbol) -> bool {
        if symbol.package.is_some()
            || !symbol
                .effect_name
                .chars()
                .next()
                .is_some_and(|initial| initial.is_ascii_uppercase())
            || !symbol
                .name
                .chars()
                .next()
                .is_some_and(|initial| initial.is_ascii_lowercase())
        {
            return false;
        }
        self.effect_operation_identity_is_unambiguous(symbol)
    }

    fn effect_operation_identity_is_unambiguous(&self, symbol: &EffectOperationSymbol) -> bool {
        if symbol.package.is_some() {
            return false;
        }
        let mut effects = self.effects.iter().filter(|candidate| {
            candidate.package.is_none()
                && candidate.module == symbol.module
                && candidate.name == symbol.effect_name
        });
        let Some(effect) = effects.next() else {
            return false;
        };
        if effects.next().is_some() || !self.effect_declaration_is_unrecovered(effect) {
            return false;
        }
        let mut operations = self.operations.iter().filter(|candidate| {
            candidate.package.is_none()
                && candidate.module == symbol.module
                && candidate.effect_name == symbol.effect_name
                && candidate.name == symbol.name
        });
        let Some(candidate) = operations.next() else {
            return false;
        };
        candidate.declaration == symbol.declaration
            && operations.next().is_none()
            && self.effect_operation_declaration_is_unrecovered(symbol)
    }

    fn effect_operation_declaration_is_unrecovered(
        &self,
        symbol: &EffectOperationSymbol,
    ) -> bool {
        self.files.iter().any(|file| {
            workspace_navigation_file(file)
                && file.source.path() == &symbol.declaration.span.file
                && !file.invalid_declaration_names.iter().any(|span| {
                    span.start.offset == symbol.declaration.span.start.offset
                        && span.end.offset == symbol.declaration.span.end.offset
                })
                && !file.recovered_effect_declarations.iter().any(|span| {
                    span.start.offset <= symbol.declaration.span.start.offset
                        && symbol.declaration.span.end.offset <= span.end.offset
                })
        })
    }

    fn handler_references(&self, symbol: &NeutralSymbol) -> Vec<SourceSpan> {
        self.files
            .iter()
            .filter(|file| workspace_navigation_file(file) && file.module == symbol.module)
            .flat_map(|file| {
                file.tokens
                    .iter()
                    .filter(|token| {
                        token.text == symbol.name
                            && file
                                .handler_reference_ranges
                                .contains(&(token.range.start, token.range.end))
                    })
                    .map(|token| file.source.span(token.range))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn handler_references_supported(&self, symbol: &NeutralSymbol) -> bool {
        if symbol.package.is_some() {
            return false;
        }
        let mut declarations = self.handlers.iter().filter(|candidate| {
            candidate.package.is_none()
                && candidate.module == symbol.module
                && candidate.name == symbol.name
        });
        let Some(candidate) = declarations.next() else {
            return false;
        };
        candidate.declaration == symbol.declaration
            && declarations.next().is_none()
            && self.handler_declaration_is_unrecovered(symbol)
    }

    fn effect_references(&self, symbol: &NeutralSymbol) -> Vec<SourceSpan> {
        self.files
            .iter()
            .filter(|file| {
                workspace_navigation_file(file)
                    && file.module == symbol.module
            })
            .flat_map(|file| {
                let ranges = file
                    .tokens
                    .iter()
                    .enumerate()
                    .filter(|(index, token)| {
                        token.kind == TokenKind::Ident
                            && token.text == symbol.name
                            && is_effect_reference_token(file, *index)
                    })
                    .map(|(_, token)| token.range)
                    .collect::<Vec<_>>();
                source_spans_for_sorted_ranges(&file.source, &ranges)
            })
            .collect()
    }

    fn effect_references_supported(&self, symbol: &NeutralSymbol) -> bool {
        if symbol.package.is_some()
            || !symbol
                .name
                .chars()
                .next()
                .is_some_and(|initial| initial.is_ascii_uppercase())
        {
            return false;
        }
        let mut declarations = self.effects.iter().filter(|candidate| {
            candidate.package.is_none()
                && candidate.module == symbol.module
                && candidate.name == symbol.name
        });
        let Some(candidate) = declarations.next() else {
            return false;
        };
        candidate.declaration == symbol.declaration
            && declarations.next().is_none()
            && self.effect_declaration_is_unrecovered(symbol)
    }

    fn workspace_type_alias_references(&self, symbol: &TypeAliasSymbol) -> Vec<SourceSpan> {
        self.files
            .iter()
            .filter(|file| workspace_navigation_file(file))
            .flat_map(|file| {
                let tokens = &file.tokens;
                let mut spans = file
                    .type_reference_spans(&symbol.name)
                    .into_iter()
                    .filter_map(|(token_index, span)| {
                        self.workspace_type_alias_for_reference(
                            file,
                            tokens,
                            token_index,
                            &symbol.name,
                        )
                        .is_some_and(|candidate| same_type_alias(&candidate, symbol))
                        .then_some(span)
                    })
                    .collect::<Vec<_>>();
                spans.extend(
                    tokens
                        .iter()
                        .enumerate()
                        .filter(|(_, token)| {
                            token.kind == TokenKind::Ident && token.text == symbol.name
                        })
                        .filter(|(token_index, _)| {
                            self.workspace_type_alias_for_constructor_qualifier_token(
                                file,
                                tokens,
                                *token_index,
                                &symbol.name,
                            )
                            .is_some_and(|candidate| same_type_alias(&candidate, symbol))
                        })
                        .map(|(_, token)| file.source.span(token.range)),
                );
                spans
            })
            .collect()
    }

    fn workspace_type_alias_for_constructor_qualifier_token(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeAliasSymbol> {
        let constructor_index = next_path_segment_index(tokens, token_index)?;
        let alias = self.workspace_type_alias_for_reference(file, tokens, token_index, name)?;
        let qualifier = qualifier_for_token(tokens, token_index)
            .map(|prefix| format!("{prefix}::{name}"))
            .unwrap_or_else(|| name.to_string());
        let constructor = self.constructor_for_qualified_call(
            file,
            &qualifier,
            &tokens[constructor_index].text,
        )?;
        self.workspace_type_alias_targets_constructor(&alias, &constructor)
            .then_some(alias)
    }

    fn workspace_type_alias_targets_constructor(
        &self,
        alias: &TypeAliasSymbol,
        constructor: &ConstructorSymbol,
    ) -> bool {
        if alias.package.is_some()
            || constructor.package.is_some()
            || alias.target_name != constructor.type_name
        {
            return false;
        }
        let Some(target_module) = alias.target_module.as_deref() else {
            return alias.module == constructor.module;
        };
        self.files
            .iter()
            .find(|file| file.source.path() == &alias.declaration.span.file)
            .is_some_and(|declaring_file| {
                self.visible_type_for_qualified_reference(
                    declaring_file,
                    target_module,
                    &alias.target_name,
                )
                .is_some_and(|target| {
                    target.package.is_none()
                        && target.module == constructor.module
                        && target.name == constructor.type_name
                })
            })
    }

    fn schema_alias_references(&self, symbol: &NeutralSymbol) -> Vec<SourceSpan> {
        let mut references = self
            .files
            .iter()
            .filter(|file| workspace_navigation_file(file))
            .flat_map(|file| {
                file.tokens
                    .iter()
                    .enumerate()
                    .filter(|(index, token)| {
                        token.kind == TokenKind::Ident
                            && token.text == symbol.name
                            && if matches!(
                                symbol.package_origin,
                                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                            ) {
                                is_schema_operation_path_leaf_token(file, *index)
                            } else {
                                is_schema_operation_path_leaf_candidate_token(&file.tokens, *index)
                            }
                            && self
                                .schema_alias_for_reference(
                                    file,
                                    &file.tokens,
                                    *index,
                                    &token.text,
                                )
                                .is_some_and(|candidate| same_schema(&candidate, symbol))
                    })
                    .map(|(_, token)| file.source.span(token.range))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if symbol.package.is_none()
            || matches!(
                symbol.package_origin,
                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
            )
        {
            references.extend(
                self.schema_composition_references
                    .iter()
                    .filter_map(|reference| match &reference.target {
                        SchemaReferenceTarget::Alias(candidate)
                            if same_schema(candidate, symbol) =>
                        {
                            Some(reference.span.clone())
                        }
                        _ => None,
                    }),
            );
        }
        references
    }

    fn schema_references(&self, symbol: &NeutralSymbol) -> Vec<SourceSpan> {
        if !self.schema_references_supported(symbol) {
            return Vec::new();
        }
        let mut references = self.files
            .iter()
            .filter(|file| workspace_navigation_file(file))
            .flat_map(|file| {
                file.tokens
                    .iter()
                    .enumerate()
                    .filter(|(index, token)| {
                        token.kind == TokenKind::Ident
                            && token.text == symbol.name
                            && if matches!(
                                symbol.package_origin,
                                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
                            ) {
                                is_schema_operation_path_leaf_token(file, *index)
                            } else {
                                is_schema_operation_path_leaf_candidate_token(&file.tokens, *index)
                            }
                            && self
                                .schema_for_reference(file, &file.tokens, *index, &token.text)
                                .is_some_and(|candidate| same_schema(&candidate, symbol))
                    })
                    .map(|(_, token)| file.source.span(token.range))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if symbol.package.is_none()
            || matches!(
                symbol.package_origin,
                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
            )
        {
            references.extend(
                self.schema_composition_references
                    .iter()
                    .filter(|reference| {
                        matches!(
                            &reference.target,
                            SchemaReferenceTarget::Schema(candidate)
                                if same_schema(candidate, symbol)
                        )
                    })
                    .map(|reference| reference.span.clone()),
            );
        }
        references
    }

    fn schema_references_supported(&self, symbol: &NeutralSymbol) -> bool {
        symbol.package.is_none()
            || (matches!(
                symbol.package_origin,
                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
            )
                && symbol
                    .name
                    .chars()
                    .next()
                    .is_some_and(|initial| initial.is_ascii_uppercase())
                && symbol.package.as_ref().is_some_and(|package| {
                    self.package_schemas.contains_key(&(
                        symbol.package_origin.expect("package schema has an origin"),
                        package.clone(),
                        symbol.module.clone(),
                        symbol.name.clone(),
                    ))
                }))
    }

    fn local_references(&self, symbol: &LocalSymbol, include_declaration: bool) -> Vec<SourceSpan> {
        let Some(file) = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == symbol.scope_file)
        else {
            return Vec::new();
        };
        let tokens = &file.tokens;
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
                        && !is_field_name(tokens, *index)
                        && !is_local_binding_name(tokens, *index)
                        && (symbol.kind != LocalSymbolKind::HandlerContextParameter
                            || inside_handler_operation_clause_body(tokens, token.range.start))
                        && !local_binding_shadows_other_name(
                            tokens,
                            &symbol.name,
                            token.range.start,
                            symbol.scope_start,
                            symbol.scope_end,
                            symbol.declaration.start.offset,
                        )
                        && (symbol.kind != LocalSymbolKind::HandlerContextParameter
                            || !handler_operation_clause_parameter_shadows_name(
                                tokens,
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
        if !self.function_references_supported(symbol) {
            return Vec::new();
        }
        self.function_reference_locations(symbol)
    }

    fn workspace_function_alias_references(
        &self,
        symbol: &FunctionSymbol,
    ) -> Vec<SourceSpan> {
        if symbol.package.is_some()
            || symbol.declaration_kind != SymbolDeclarationKind::PublicAlias
        {
            return Vec::new();
        }
        self.function_reference_locations(symbol)
    }

    fn function_reference_locations(&self, symbol: &FunctionSymbol) -> Vec<SourceSpan> {
        self.files
            .iter()
            .filter(|file| workspace_navigation_file(file))
            .flat_map(|file| self.references_in_file(file, symbol))
            .collect()
    }

    fn function_references_supported(&self, symbol: &FunctionSymbol) -> bool {
        if symbol.invalid_declaration_name {
            return false;
        }
        if symbol.declaration_kind == SymbolDeclarationKind::Declaration {
            return true;
        }
        matches!(
            symbol.package_origin,
            Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
        ) && self.function_alias_target_resolves_to_function(symbol)
    }

    fn function_alias_target_resolves_to_function(&self, symbol: &FunctionSymbol) -> bool {
        let Some(target_name) = symbol.alias_target_name.as_deref() else {
            return false;
        };
        let Some(target_module) = self.function_alias_target_module(symbol) else {
            return false;
        };
        self.package_function_targets.iter().any(|candidate| {
            candidate.name == target_name
                && candidate.module == target_module
                && Some(candidate.package.as_str()) == symbol.package.as_deref()
                && Some(candidate.package_origin) == symbol.package_origin
        })
    }

    fn function_alias_target_module(&self, symbol: &FunctionSymbol) -> Option<String> {
        let Some(target_module) = symbol.alias_target_module.as_deref() else {
            return Some(symbol.module.clone());
        };
        let declaring_file = self.files.iter().find(|file| {
            file.source.path() == &symbol.declaration.span.file
                && matches!(
                    (&file.origin, symbol.package.as_deref(), symbol.package_origin),
                    (
                        IndexedOrigin::Package {
                            identity,
                            standard_library,
                            ..
                        },
                        Some(package),
                        Some(origin),
                    ) if identity == package
                        && if *standard_library {
                            origin == PackageOrigin::StandardLibrary
                        } else {
                            origin == PackageOrigin::DirectDependency
                        }
                )
        });
        match declaring_file {
            None => None,
            Some(file) => {
                if file.uses.contains(target_module) {
                    Some(target_module.to_string())
                } else {
                    resolve_qualified_alias(&file.import_aliases, target_module)
                }
            }
        }
    }

    fn bare_prelude_function_references(
        &self,
        file: &IndexedFile,
        symbol: &FunctionSymbol,
    ) -> Vec<SourceSpan> {
        let tokens = &file.tokens;
        tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                token.text == symbol.name
                    && previous_non_layout_token(tokens, *index)
                        .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
                    && (is_call_target_token(tokens, *index)
                        || ((symbol.package.is_some() || symbol.public)
                            && (file.classified_path_segments.iter().any(|segment| {
                                segment.role == NameClass::ValueBinding
                                    && same_span(&segment.span, &file.source.span(token.range))
                            }) || is_bare_function_value_token(tokens, *index))))
                    && self
                        .symbol_for_bare_call(file, tokens, *index, &token.text)
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
        let tokens = &file.tokens;
        let module_segments = qualifier.split("::").collect::<Vec<_>>();
        tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                token.text == symbol.name
                    && qualified_reference_matches(tokens, *index, &module_segments)
                    && (is_call_target_token(tokens, *index)
                        || ((symbol.package.is_some() || symbol.public)
                            && (file.classified_path_segments.iter().any(|segment| {
                                segment.role == NameClass::ValueBinding
                                    && same_span(&segment.span, &file.source.span(token.range))
                            }) || is_qualified_function_value_token(tokens, *index))))
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
                        if is_field_name(tokens, token_index) {
                            return None;
                        }
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

    fn type_alias_references(&self, symbol: &TypeAliasSymbol) -> Vec<SourceSpan> {
        if !self.type_alias_references_supported(symbol) {
            return Vec::new();
        }
        self.files
            .iter()
            .filter(|file| workspace_navigation_file(file))
            .flat_map(|file| {
                let tokens = &file.tokens;
                let mut spans = file
                    .type_reference_spans(&symbol.name)
                    .into_iter()
                    .filter_map(|(token_index, span)| {
                        self.type_namespace_symbol_for_reference(
                            file,
                            tokens,
                            token_index,
                            &symbol.name,
                        )
                        .is_some_and(|candidate| {
                            matches!(
                                candidate,
                                Symbol::TypeAlias(ref candidate)
                                    if same_type_alias(candidate, symbol)
                            )
                        })
                        .then_some(span)
                    })
                    .collect::<Vec<_>>();
                spans.extend(self.constructor_type_alias_qualifier_references(file, tokens, symbol));
                spans
            })
            .collect()
    }

    fn type_alias_references_supported(&self, symbol: &TypeAliasSymbol) -> bool {
        matches!(
            symbol.package_origin,
            Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
        ) && self.type_alias_target_resolves_to_type(symbol)
    }

    fn type_alias_definition_supported(&self, symbol: &TypeAliasSymbol) -> bool {
        match symbol.package_origin {
            Some(PackageOrigin::DirectDependency) => self.type_alias_target_resolves_to_type(symbol),
            Some(PackageOrigin::StandardLibrary) => false,
            None => true,
        }
    }

    fn type_alias_target_resolves_to_type(&self, symbol: &TypeAliasSymbol) -> bool {
        let Some(target_module) = self.type_alias_target_module(symbol) else {
            return false;
        };
        let mut candidates = self.package_type_targets.iter().filter(|candidate| {
            candidate.name == symbol.target_name
                && candidate.module == target_module
                && Some(candidate.package.as_str()) == symbol.package.as_deref()
                && Some(candidate.package_origin) == symbol.package_origin
        });
        candidates.next().is_some() && candidates.next().is_none()
    }

    fn type_alias_target_module(&self, symbol: &TypeAliasSymbol) -> Option<String> {
        let Some(target_module) = symbol.target_module.as_deref() else {
            return Some(symbol.module.clone());
        };
        let declaring_file = self.files.iter().find(|file| {
            file.source.path() == &symbol.declaration.span.file
                && matches!(
                    (&file.origin, symbol.package.as_deref(), symbol.package_origin),
                    (
                        IndexedOrigin::Package {
                            identity,
                            standard_library,
                            ..
                        },
                        Some(package),
                        Some(origin),
                    ) if identity == package
                        && if *standard_library {
                            origin == PackageOrigin::StandardLibrary
                        } else {
                            origin == PackageOrigin::DirectDependency
                        }
                )
        })?;
        if declaring_file.uses.contains(target_module) {
            Some(target_module.to_string())
        } else {
            resolve_qualified_alias(&declaring_file.import_aliases, target_module)
        }
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
                if self
                    .visible_type_alias_for_reference(file, tokens, *index, &token.text)
                    .is_some()
                {
                    return false;
                }
                self.type_for_constructor_qualifier_token(file, tokens, *index, &token.text)
                    .is_some_and(|candidate| same_type(&candidate, symbol))
            })
            .map(|(_, token)| file.source.span(token.range))
            .collect()
    }

    fn constructor_type_alias_qualifier_references(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        symbol: &TypeAliasSymbol,
    ) -> Vec<SourceSpan> {
        tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| token.kind == TokenKind::Ident && token.text == symbol.name)
            .filter(|(index, token)| {
                self.type_alias_for_constructor_qualifier_token(file, tokens, *index, &token.text)
                    .is_some_and(|candidate| same_type_alias(&candidate, symbol))
            })
            .map(|(_, token)| file.source.span(token.range))
            .collect()
    }

    fn type_alias_for_constructor_qualifier_token(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeAliasSymbol> {
        let constructor_index = next_path_segment_index(tokens, token_index)?;
        let Symbol::TypeAlias(alias) =
            self.type_namespace_symbol_for_reference(file, tokens, token_index, name)?
        else {
            return None;
        };
        if !self.type_alias_references_supported(&alias) {
            return None;
        }
        let target_module = self.type_alias_target_module(&alias)?;
        self.package_constructor_targets
            .iter()
            .any(|constructor| {
                constructor.module == target_module
                    && constructor.type_name == alias.target_name
                    && constructor.name == tokens[constructor_index].text
                    && Some(constructor.package.as_str()) == alias.package.as_deref()
                    && Some(constructor.package_origin) == alias.package_origin
            })
            .then_some(alias)
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
                let tokens = &file.tokens;
                tokens
                    .iter()
                    .enumerate()
                    .filter(|(index, token)| {
                        token.kind == TokenKind::Ident
                            && token.text == symbol.name
                            && !same_span(&file.source.span(token.range), &symbol.declaration.span)
                            && is_constructor_reference_token(tokens, *index)
                            && self
                                .constructor_symbol_for_call(file, tokens, *index, &token.text)
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

fn effect_operation_references_in_file(
    file: &IndexedFile,
    symbol: &EffectOperationSymbol,
    eligible_handlers: &BTreeSet<(String, String)>,
) -> Vec<SourceSpan> {
    let mut references = file
        .tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| effect_operation_token_matches(file, *index, token, symbol))
        .map(|(_, token)| file.source.span(token.range))
        .collect::<Vec<_>>();
    references.extend(
        file.handler_operation_clause_references
            .iter()
            .filter(|clause| {
                clause.effect_name == symbol.effect_name
                    && clause.operation_name == symbol.name
                    && eligible_handlers
                        .contains(&(file.module.clone(), clause.handler_name.clone()))
            })
            .map(|clause| clause.span.clone()),
    );
    references.sort_by_key(|span| (span.start.offset, span.end.offset));
    references
}

fn effect_operation_token_matches(
    file: &IndexedFile,
    index: usize,
    token: &Token,
    symbol: &EffectOperationSymbol,
) -> bool {
    if token.kind != TokenKind::Ident
        || token.text != symbol.name
        || !file
            .effect_operation_ranges
            .contains(&(token.range.start, token.range.end))
    {
        return false;
    }
    let Some(qualifier_index) = previous_path_segment_index(&file.tokens, index) else {
        return false;
    };
    let qualifier = &file.tokens[qualifier_index];
    qualifier.text == symbol.effect_name
        && !file.generic_effect_binder_shadows(&symbol.effect_name, qualifier.range.start)
        && file
            .effect_reference_ranges
            .contains(&(qualifier.range.start, qualifier.range.end))
        && qualifier_for_token(&file.tokens, index).is_some_and(|name| name == symbol.effect_name)
}

fn source_spans_for_sorted_ranges(source: &SourceFile, ranges: &[TextRange]) -> Vec<SourceSpan> {
    let mut cursor = 0usize;
    let mut line = 1usize;
    let mut column = 1usize;
    ranges
        .iter()
        .map(|range| {
            let start = advance_source_position(
                source.text(),
                &mut cursor,
                &mut line,
                &mut column,
                range.start,
            );
            let end = advance_source_position(
                source.text(),
                &mut cursor,
                &mut line,
                &mut column,
                range.end,
            );
            SourceSpan {
                file: source.path().clone(),
                start,
                end,
            }
        })
        .collect()
}

fn advance_source_position(
    text: &str,
    cursor: &mut usize,
    line: &mut usize,
    column: &mut usize,
    target: usize,
) -> LineCol {
    for ch in text[*cursor..target].chars() {
        #[cfg(test)]
        record_effect_reference_source_scalar_visit();
        *cursor += ch.len_utf8();
        if ch == '\n' {
            *line += 1;
            *column = 1;
        } else {
            *column += 1;
        }
    }
    LineCol {
        line: *line,
        column: *column,
        offset: target,
    }
}

fn is_qualified_function_value_token(tokens: &[Token], token_index: usize) -> bool {
    previous_non_layout_token(tokens, token_index)
        .is_some_and(|previous| previous.kind == TokenKind::DoubleColon)
        && next_non_layout_token(tokens, token_index)
            .is_none_or(|next| next.kind != TokenKind::DoubleColon && next.kind != TokenKind::LParen)
}

fn is_bare_function_value_token(tokens: &[Token], token_index: usize) -> bool {
    previous_non_layout_token(tokens, token_index)
        .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
        && next_non_layout_token(tokens, token_index)
            .is_none_or(|next| next.kind != TokenKind::DoubleColon && next.kind != TokenKind::LParen)
        && !is_field_name(tokens, token_index)
        && !is_function_declaration_name(tokens, token_index)
        && !is_parameter_name(tokens, token_index)
        && !is_local_binding_name(tokens, token_index)
        && !is_handler_operation_clause_operation_name(tokens, token_index)
}

fn same_recovery_symbol(left: &RecoverySymbol, right: &RecoverySymbol) -> bool {
    left.kind == right.kind
        && left.source_file == right.source_file
        && same_span(&left.declaration, &right.declaration)
}

fn workspace_navigation_file(file: &IndexedFile) -> bool {
    matches!(file.origin, IndexedOrigin::Workspace) && !file.navigation_isolated
}
