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

        self.non_declaration_symbol_for_selection(file, tokens, token_index, name, selection)
    }

    fn non_declaration_symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
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
            return self.type_reference_selection(file, tokens, token_index, name, selection);
        }
        let Some(qualifier) = qualifier_for_token(tokens, token_index) else {
            return self
                .symbol_for_bare_call(file, tokens, token_index, name)
                .map(SelectedNavigationSymbol::bare);
        };
        self.symbol_for_qualified_call(file, &qualifier, name)
            .map(SelectedNavigationSymbol::bare)
    }

    fn type_reference_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<SelectedNavigationSymbol> {
        is_type_reference_token(&file.source, name, selection)
            .then(|| self.visible_type_for_reference(file, tokens, token_index, name))
            .flatten()
            .map(Symbol::Type)
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

fn visible_imported_type_for_bare_reference(
    file: &IndexedFile,
    symbol: &TypeSymbol,
    name: &str,
) -> bool {
    symbol.name == name
        && symbol.public
        && match &symbol.package {
            Some(package) => {
                file.external_uses
                    .contains(&(symbol.module.clone(), package.clone()))
                    || symbol.standard_prelude
            }
            None => symbol.module != file.module && file.uses.contains(&symbol.module),
        }
}

fn visible_imported_type_alias_for_bare_reference(
    file: &IndexedFile,
    symbol: &TypeAliasSymbol,
    name: &str,
) -> bool {
    symbol.name == name
        && match &symbol.package {
            Some(package) => {
                file.external_uses
                    .contains(&(symbol.module.clone(), package.clone()))
                    || symbol.standard_prelude
            }
            None => symbol.module != file.module && file.uses.contains(&symbol.module),
        }
}

fn visible_type_for_qualified_reference(
    file: &IndexedFile,
    symbol: &TypeSymbol,
    qualified_modules: &[String],
    name: &str,
) -> bool {
    symbol.name == name
        && qualified_modules
            .iter()
            .any(|module| module == &symbol.module)
        && match &symbol.package {
            Some(package) => {
                file.external_uses
                    .contains(&(symbol.module.clone(), package.clone()))
                    || symbol.standard_prelude
            }
            None => symbol.module == file.module || file.uses.contains(&symbol.module),
        }
}

fn visible_type_alias_for_qualified_reference(
    file: &IndexedFile,
    symbol: &TypeAliasSymbol,
    qualified_modules: &[String],
    name: &str,
) -> bool {
    symbol.name == name
        && qualified_modules
            .iter()
            .any(|module| module == &symbol.module)
        && match &symbol.package {
            Some(package) => {
                file.external_uses
                    .contains(&(symbol.module.clone(), package.clone()))
                    || symbol.standard_prelude
            }
            None => symbol.module == file.module || file.uses.contains(&symbol.module),
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

fn local_binding_declaration(file: &IndexedFile, binding: &LocalBinding) -> SourceSpan {
    file.source.span(TextRange::new(
        binding.declaration_start,
        binding.declaration_end,
    ))
}

fn scoped_binding_declaration(file: &IndexedFile, binding: &ScopedBinding) -> SourceSpan {
    file.source.span(TextRange::new(
        binding.declaration_start,
        binding.declaration_end,
    ))
}

fn scope_shadow_declaration(file: &IndexedFile, binding: ScopeShadow<'_>) -> SourceSpan {
    match binding {
        ScopeShadow::FunctionBinding(binding) => scoped_binding_declaration(file, binding),
        ScopeShadow::LocalBinding(binding) => local_binding_declaration(file, binding),
    }
}

fn same_scope_binding_conflicts(
    binding: &ClauseBinding,
    selected: &LocalSymbol,
    requested_name: &str,
) -> bool {
    binding.name == requested_name
        && binding.start == selected.scope_start
        && binding.end == selected.scope_end
        && !same_span(&binding.declaration, &selected.declaration)
}

fn module_rename_conflict(
    declaration: NavigationLocation,
    module: &str,
) -> (NavigationLocation, RenameAffectedScope) {
    (
        declaration,
        RenameAffectedScope::Module {
            name: module.to_string(),
        },
    )
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
