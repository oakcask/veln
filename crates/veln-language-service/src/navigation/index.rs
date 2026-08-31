impl SymbolIndex {
    pub(crate) fn new(
        sources: Vec<SourceFile>,
        dependencies: Vec<DirectDependencySnapshot>,
        standard_library: Option<DirectDependencySnapshot>,
    ) -> Self {
        let mut files = Vec::new();
        let mut declarations = FileDeclarations::default();
        for source in sources {
            let (file, file_declarations) = index_workspace_source(source);
            declarations.extend(file_declarations);
            files.push(file);
        }
        for dependency in dependencies.into_iter().chain(standard_library) {
            index_dependency_sources(&mut files, &mut declarations, dependency);
        }
        attach_classified_path_segments(&mut files);
        Self {
            functions: declarations.functions,
            types: declarations.types,
            constructors: declarations.constructors,
            type_aliases: declarations.type_aliases,
            files,
        }
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
        let offset = offset_for_position(file.source.text(), position)?;
        let tokens = &file.tokens;
        let (token_index, token) = identifier_token_at(tokens, offset)?;
        let selection = file.source.span(token.range);
        let name = file
            .source
            .text()
            .get(selection.start.offset..selection.end.offset)?
            .to_string();
        let selected = self.symbol_for_selection(file, tokens, token_index, &name, &selection)?;
        Some(SymbolRequest {
            index: self,
            symbol: selected.symbol,
            selection,
            classified_path_segment: selected.classified_path_segment,
        })
    }

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
        self.types
            .iter()
            .find(|candidate| {
                candidate.package.is_none()
                    && candidate.module == selected.module
                    && candidate.name == requested_name
                    && !same_type(candidate, &selected)
            })
            .map(|candidate| {
                (
                    candidate.declaration.clone(),
                    RenameAffectedScope::Module {
                        name: selected.module.clone(),
                    },
                )
            })
            .or_else(|| {
                self.affected_spans(result)
                    .into_iter()
                    .find_map(|span| {
                        let (file, token_index) = self.file_token_for_span(span)?;
                        let conflict = self
                            .visible_type_for_reference(file, &file.tokens, token_index, requested_name)
                            .filter(|candidate| !same_type(candidate, &selected))?;
                        Some((
                            conflict.declaration,
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
            .or_else(|| {
                self.affected_spans(result)
                    .into_iter()
                    .find_map(|span| {
                        let (file, token_index) = self.file_token_for_span(span)?;
                        let conflict = self
                            .constructor_symbol_for_call(
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
        self.functions
            .iter()
            .find(|candidate| {
                candidate.package.is_none()
                    && candidate.module == selected.module
                    && candidate.name == requested_name
                    && !same_function(candidate, &selected)
            })
            .map(|candidate| {
                (
                    candidate.declaration.clone(),
                    RenameAffectedScope::Module {
                        name: selected.module.clone(),
                    },
                )
            })
            .or_else(|| {
                self.affected_spans(result)
                    .into_iter()
                    .find_map(|span| {
                        let (file, token_index) = self.file_token_for_span(span)?;
                        if local_binding_shadows_call_target(&file.tokens, token_index, requested_name)
                        {
                            return self.local_conflict_in_file(file, requested_name, span);
                        }
                        if let Some(conflict) = self
                            .constructor_symbol_for_call(
                                file,
                                &file.tokens,
                                token_index,
                                requested_name,
                            )
                        {
                            return Some((
                                conflict.declaration,
                                RenameAffectedScope::Module {
                                    name: file.module.clone(),
                                },
                            ));
                        }
                        let conflict = match qualifier_for_token(&file.tokens, token_index) {
                            Some(qualifier) => {
                                self.function_for_qualified_call(file, &qualifier, requested_name)
                            }
                            None => self.symbol_for_bare_call(
                                file,
                                &file.tokens,
                                token_index,
                                requested_name,
                            )
                            .and_then(|symbol| match symbol {
                                Symbol::Function(symbol) => Some(symbol),
                                _ => None,
                            }),
                        }
                        .filter(|candidate| !same_function(candidate, &selected))?;
                        Some((
                            conflict.declaration,
                            RenameAffectedScope::Module {
                                name: file.module.clone(),
                            },
                        ))
                    })
            })
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
        handler_operation_clause_bindings(file, &file.tokens)
            .into_iter()
            .find(|binding| {
                binding.name == requested_name
                    && !same_span(&binding.declaration, &selected.declaration)
                    && binding.start < selected.scope_end
                    && selected.scope_start < binding.end
            })
            .map(|binding| (workspace_location(binding.declaration), scope.clone()))
            .or_else(|| {
                local_bindings(&file.tokens, selected.scope_start, selected.scope_end)
                    .into_iter()
                    .find(|binding| binding.name == requested_name)
                    .map(|binding| {
                        let span = span_for_name_at_or_before(file, requested_name, binding.start)
                            .unwrap_or_else(|| selected.declaration.clone());
                        (workspace_location(span), scope)
                    })
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
                .find(|binding| same_span(&binding.declaration, &result.selected_symbol.declaration.span))
                .map(|binding| LocalSymbol {
                    name: binding.name,
                    declaration: binding.declaration,
                    scope_file: file.source.path().as_str().to_string(),
                    scope_start: binding.start,
                    scope_end: binding.end,
                    kind: binding.kind,
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
                let conflict = span_for_name_before_offset(file, requested_name, span.start.offset)
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

    fn symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
        if let Some(symbol) =
            handler_operation_clause_symbol(file, tokens, token_index, name, selection)
        {
            return Some(SelectedNavigationSymbol::bare(Symbol::Local(symbol)));
        }

        if is_handler_operation_clause_operation_name(tokens, token_index) {
            return None;
        }

        if let Some(symbol) = self.function_declared_at(name, selection) {
            return Some(SelectedNavigationSymbol::bare(Symbol::Function(symbol)));
        }

        if let Some(symbol) = self.type_declared_at(name, selection) {
            return Some(SelectedNavigationSymbol::bare(Symbol::Type(symbol)));
        }

        if let Some(symbol) = self.constructor_declared_at(name, selection) {
            return Some(SelectedNavigationSymbol::bare(Symbol::Constructor(symbol)));
        }

        if let Some(segment) =
            self.classified_qualified_segment(file, tokens, token_index, name, selection)
            && let Some(selected) = segment.into_selected_symbol()
        {
            return Some(selected);
        }

        if is_qualified_path_token(tokens, token_index)
            && !is_call_target_token(tokens, token_index)
        {
            return None;
        }

        if !is_call_target_token(tokens, token_index) {
            if is_type_reference_token(&file.source, name, selection) {
                return self
                    .visible_type_for_reference(file, tokens, token_index, name)
                    .map(Symbol::Type)
                    .map(SelectedNavigationSymbol::bare);
            }
            return None;
        }
        let Some(qualifier) = qualifier_for_token(tokens, token_index) else {
            return self
                .symbol_for_bare_call(file, tokens, token_index, name)
                .map(SelectedNavigationSymbol::bare);
        };
        self.symbol_for_qualified_call(file, &qualifier, name)
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
            let symbol = match segment.role {
                NameClass::Type => self
                    .visible_type_for_reference(file, tokens, token_index, name)
                    .map(Symbol::Type),
                NameClass::Constructor => {
                    qualifier_for_token(tokens, token_index)
                        .and_then(|qualifier| {
                            self.constructor_for_qualified_call(file, &qualifier, name)
                        })
                        .map(Symbol::Constructor)
                }
                NameClass::Function => {
                    qualifier_for_token(tokens, token_index)
                        .and_then(|qualifier| self.function_for_qualified_call(file, &qualifier, name))
                        .map(Symbol::Function)
                }
                NameClass::ValueBinding => {
                    qualifier_for_token(tokens, token_index)
                        .and_then(|qualifier| {
                            self.function_for_qualified_call(file, &qualifier, name)
                        })
                        .map(Symbol::Function)
                }
                _ => None,
            };
            return Some(ClassifiedNavigationSegment {
                segment: segment.clone(),
                symbol,
            });
        }
        None
    }

    fn function_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<FunctionSymbol> {
        self.functions
            .iter()
            .find(|symbol| declaration_matches(name, selection, &symbol.name, symbol.package.as_deref(), &symbol.declaration.span))
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

    fn visible_type_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeSymbol> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return self.visible_type_for_qualified_reference(file, &qualifier, name);
        }
        self.visible_type_for_bare_reference(file, name)
    }

    fn visible_type_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<TypeSymbol> {
        if let Some(symbol) = self.types.iter().find(|symbol| {
            symbol.name == name && symbol.module == file.module && symbol.package.is_none()
        }) {
            return Some(symbol.clone());
        }

        let mut candidates = self.types.iter().filter(|symbol| {
            symbol.name == name
                && symbol.public
                && match &symbol.package {
                    Some(package) => file
                        .external_uses
                        .contains(&(symbol.module.clone(), package.clone()))
                        || symbol.standard_prelude,
                    None => symbol.module != file.module && file.uses.contains(&symbol.module),
                }
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn visible_type_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<TypeSymbol> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        let mut candidates = self.types.iter().filter(|symbol| {
            symbol.name == name
                && qualified_modules.iter().any(|module| module == &symbol.module)
                && match &symbol.package {
                    Some(package) => file
                        .external_uses
                        .contains(&(symbol.module.clone(), package.clone()))
                        || symbol.standard_prelude,
                    None => symbol.module == file.module || file.uses.contains(&symbol.module),
                }
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn type_for_constructor_qualifier_token(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeSymbol> {
        let constructor_index = next_path_segment_index(tokens, token_index)?;
        let qualifier = qualifier_for_token(tokens, token_index)
            .map(|prefix| format!("{prefix}::{name}"))
            .unwrap_or_else(|| name.to_string());
        let constructor =
            self.constructor_for_qualified_call(file, &qualifier, &tokens[constructor_index].text)?;
        self.types
            .iter()
            .find(|symbol| {
                symbol.module == constructor.module
                    && symbol.name == constructor.type_name
                    && symbol.package == constructor.package
            })
            .cloned()
    }

    fn symbol_for_bare_call(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<Symbol> {
        if let Some(symbol) = self.constructor_for_bare_call(file, name) {
            return Some(Symbol::Constructor(symbol));
        }
        if let Some(symbol) = self.functions.iter().find(|symbol| {
            symbol.name == name && symbol.module == file.module && symbol.package.is_none()
        }) {
            return Some(Symbol::Function(symbol.clone()));
        }
        if local_binding_shadows_call_target(tokens, token_index, name)
            || self.has_visible_non_prelude_imported_function(file, name)
            || self.has_visible_non_prelude_imported_constructor(file, name)
        {
            return None;
        }
        self.functions
            .iter()
            .find(|symbol| symbol.name == name && symbol.standard_prelude)
            .cloned()
            .map(Symbol::Function)
    }

    fn symbol_for_qualified_call(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<Symbol> {
        if let Some(symbol) = self.constructor_for_qualified_call(file, qualifier, name) {
            return Some(Symbol::Constructor(symbol));
        }
        self.function_for_qualified_call(file, qualifier, name)
            .map(Symbol::Function)
    }

    fn function_for_qualified_call(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<FunctionSymbol> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        self.functions
            .iter()
            .find(|symbol| match &symbol.package {
                Some(package) => {
                    symbol.name == name
                        && qualified_modules.iter().any(|module| module == &symbol.module)
                        && (symbol.standard_prelude
                            || file
                                .external_uses
                                .contains(&(symbol.module.clone(), package.clone())))
                }
                None => {
                    symbol.name == name
                        && qualified_modules.iter().any(|module| module == &symbol.module)
                        && file.uses.contains(&symbol.module)
                        && (symbol.public
                            || file
                                .companion_target_module
                                .as_ref()
                                .is_some_and(|target| target == &symbol.module))
                }
            })
            .cloned()
    }

    fn constructor_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.local_constructor_for_bare_call(file, name)
            .or_else(|| self.imported_workspace_constructor_for_bare_call(file, name))
            .or_else(|| self.imported_package_constructor_for_bare_call(file, name))
    }

    fn local_constructor_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.constructors
            .iter()
            .find(|symbol| {
                symbol.name == name
                    && symbol.package.is_none()
                    && symbol.module == file.module
                    && visible_workspace_constructor_from(file, symbol)
            })
            .cloned()
    }

    fn imported_workspace_constructor_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.unique_constructor_matching(|symbol| {
            symbol.name == name
                && !symbol.standard_prelude
                && symbol.package.is_none()
                && symbol.module != file.module
                && (file.uses.contains(&symbol.module)
                    || self.constructor_reexport_visible_from(file, symbol, None))
                && visible_workspace_constructor_from(file, symbol)
        })
    }

    fn imported_package_constructor_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        self.unique_constructor_matching(|symbol| {
            symbol.name == name
                && !symbol.standard_prelude
                && symbol.public
                && symbol.package.as_ref().is_some_and(|package| {
                    file.external_uses
                        .contains(&(symbol.module.clone(), package.clone()))
                        || self.constructor_reexport_visible_from(file, symbol, Some(package))
                })
        })
    }

    fn unique_constructor_matching(
        &self,
        predicate: impl Fn(&ConstructorSymbol) -> bool,
    ) -> Option<ConstructorSymbol> {
        let mut candidates = self.constructors.iter().filter(|symbol| predicate(symbol));
        let candidate = candidates.next()?;
        candidates.next().is_none().then(|| candidate.clone())
    }

    fn constructor_for_qualified_call(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        let qualified_modules = self.qualified_module_candidates(file, qualifier);
        self.constructors
            .iter()
            .find(|symbol| {
                symbol.name == name
                    && (qualified_modules
                        .iter()
                        .any(|module| constructor_qualifier_matches(symbol, module))
                        || qualified_modules.iter().any(|module| {
                            module == &format!("{}::{}", symbol.module, symbol.type_name)
                        })
                        || (qualifier == symbol.type_name && symbol.module == file.module)
                        || self.constructor_reexport_qualifier_matches(file, symbol, qualifier))
                    && match &symbol.package {
                        Some(package) => {
                            symbol.standard_prelude
                                || file
                                    .external_uses
                                    .contains(&(symbol.module.clone(), package.clone()))
                                || self.constructor_reexport_visible_from(
                                    file,
                                    symbol,
                                    Some(package),
                                )
                        }
                        None => {
                            symbol.module == file.module
                                || ((file.uses.contains(&symbol.module)
                                    || self.constructor_reexport_visible_from(file, symbol, None))
                                    && visible_workspace_constructor_from(file, symbol))
                        }
                    }
            })
            .cloned()
    }

    fn constructor_reexport_qualifier_matches(
        &self,
        file: &IndexedFile,
        symbol: &ConstructorSymbol,
        qualifier: &str,
    ) -> bool {
        self.type_aliases.iter().any(|alias| {
            type_alias_targets_constructor(alias, symbol)
                && (qualifier == alias.module
                    || qualifier == format!("{}::{}", alias.module, alias.name))
                && match &alias.package {
                    Some(alias_package) => file
                        .external_uses
                        .contains(&(alias.module.clone(), alias_package.clone())),
                    None => file.uses.contains(&alias.module) || file.module == alias.module,
                }
        })
    }

    fn has_visible_non_prelude_imported_constructor(&self, file: &IndexedFile, name: &str) -> bool {
        self.constructors.iter().any(|symbol| {
            if symbol.name != name || symbol.standard_prelude {
                return false;
            }
            if symbol.package.is_none() && symbol.module == file.module {
                return false;
            }
            match &symbol.package {
                Some(package) => {
                    symbol.public
                        && file
                            .external_uses
                            .contains(&(symbol.module.clone(), package.clone()))
                }
                None => {
                    (file.uses.contains(&symbol.module)
                        || self.constructor_reexport_visible_from(file, symbol, None))
                        && visible_workspace_constructor_from(file, symbol)
                }
            }
        })
    }

    fn constructor_reexport_visible_from(
        &self,
        file: &IndexedFile,
        symbol: &ConstructorSymbol,
        package: Option<&String>,
    ) -> bool {
        self.type_aliases.iter().any(|alias| {
            if !type_alias_targets_constructor(alias, symbol) {
                return false;
            }
            if alias.package.as_ref() != package {
                return false;
            }
            match &alias.package {
                Some(alias_package) => file
                    .external_uses
                    .contains(&(alias.module.clone(), alias_package.clone())),
                None => file.uses.contains(&alias.module),
            }
        })
    }

    fn has_visible_non_prelude_imported_function(&self, file: &IndexedFile, name: &str) -> bool {
        self.functions.iter().any(|symbol| {
            if symbol.name != name || symbol.standard_prelude {
                return false;
            }
            if symbol.package.is_none() && symbol.module == file.module {
                return false;
            }
            if symbol.package.is_none() && !symbol.public {
                return false;
            }
            match &symbol.package {
                Some(package) => file
                    .external_uses
                    .contains(&(symbol.module.clone(), package.clone())),
                None => file.uses.contains(&symbol.module),
            }
        })
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
            return call_references(&file.source, &symbol.name);
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

fn resolve_qualified_alias(aliases: &BTreeMap<String, String>, qualifier: &str) -> Option<String> {
    let mut parts = qualifier.split("::");
    let alias = parts.next()?;
    let module = aliases.get(alias)?;
    let rest = parts.collect::<Vec<_>>();
    if rest.is_empty() {
        Some(module.clone())
    } else {
        Some(format!("{}::{}", module, rest.join("::")))
    }
}

fn resolve_external_qualified_alias(
    aliases: &BTreeMap<String, (String, String)>,
    qualifier: &str,
) -> Option<(String, String)> {
    let mut parts = qualifier.split("::");
    let alias = parts.next()?;
    let (module, package) = aliases.get(alias)?;
    let rest = parts.collect::<Vec<_>>();
    if rest.is_empty() {
        Some((module.clone(), package.clone()))
    } else {
        Some((format!("{}::{}", module, rest.join("::")), package.clone()))
    }
}

fn span_for_name_before_offset(
    file: &IndexedFile,
    name: &str,
    offset: usize,
) -> Option<SourceSpan> {
    file.tokens
        .iter()
        .rev()
        .find(|token| {
            token.kind == TokenKind::Ident && token.text == name && token.range.start < offset
        })
        .map(|token| file.source.span(token.range))
}

fn span_for_name_at_or_before(
    file: &IndexedFile,
    name: &str,
    offset: usize,
) -> Option<SourceSpan> {
    file.tokens
        .iter()
        .rev()
        .find(|token| {
            token.kind == TokenKind::Ident && token.text == name && token.range.start <= offset
        })
        .map(|token| file.source.span(token.range))
}

fn declaration_matches(
    expected_name: &str,
    selection: &SourceSpan,
    actual_name: &str,
    package: Option<&str>,
    declaration: &SourceSpan,
) -> bool {
    actual_name == expected_name
        && package.is_none()
        && declaration.file == selection.file
        && declaration.start.offset == selection.start.offset
        && declaration.end.offset == selection.end.offset
}

fn is_qualified_path_token(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|token| token.kind == TokenKind::DoubleColon)
        || next_non_layout_token(tokens, index)
            .is_some_and(|token| token.kind == TokenKind::DoubleColon)
}
