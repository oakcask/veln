impl IndexedDependencies {
    pub(crate) fn empty() -> Self {
        Self {
            files: Vec::new(),
            declarations: FileDeclarations::default(),
            module: empty_surface_module(),
        }
    }

    pub(crate) fn new_direct(dependencies: Vec<DirectDependencySnapshot>) -> Self {
        let mut indexed = Self::index(dependencies);
        attach_classified_path_segments(&mut indexed.files, &indexed.module, &indexed.module);
        indexed
    }

    pub(crate) fn new_standard_library(
        standard_library: Option<DirectDependencySnapshot>,
    ) -> Self {
        let mut indexed = Self::index(standard_library);
        attach_classified_path_segments(&mut indexed.files, &indexed.module, &indexed.module);
        indexed
    }

    fn index(dependencies: impl IntoIterator<Item = DirectDependencySnapshot>) -> Self {
        let mut files = Vec::new();
        let mut declarations = FileDeclarations::default();
        let mut module = empty_surface_module();
        for dependency in dependencies {
            index_dependency_sources(&mut files, &mut declarations, &mut module, dependency);
        }
        Self {
            files,
            declarations,
            module,
        }
    }
}

struct WorkspaceIndexInput {
    files: Vec<IndexedFile>,
    declarations: FileDeclarations,
    module: veln_ast::SurfaceModule,
}

struct SchemaNavigationIndex {
    aliases: Vec<NeutralSymbol>,
    package_schemas: BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol>,
    composition_references: Vec<SchemaCompositionReference>,
    alias_module_imports: BTreeMap<String, SchemaAliasModuleImports>,
    bare_aliases: BareSchemaAliasIndex,
    operation_lookup: SchemaOperationLookupIndex,
}

impl SymbolIndex {
    pub(crate) fn new(
        sources: Vec<SourceFile>,
        direct_dependencies: &IndexedDependencies,
        standard_library: &IndexedDependencies,
    ) -> Self {
        Self::new_with_path_classification(
            sources,
            direct_dependencies,
            standard_library,
            true,
        )
    }

    pub(crate) fn new_for_schema_navigation(
        sources: Vec<SourceFile>,
        direct_dependencies: &IndexedDependencies,
        standard_library: &IndexedDependencies,
    ) -> Self {
        Self::new_with_path_classification(
            sources,
            direct_dependencies,
            standard_library,
            false,
        )
    }

    fn new_with_path_classification(
        sources: Vec<SourceFile>,
        direct_dependencies: &IndexedDependencies,
        standard_library: &IndexedDependencies,
        classify_workspace_paths: bool,
    ) -> Self {
        let WorkspaceIndexInput {
            mut files,
            mut declarations,
            module: workspace_module,
        } = index_workspace_input(
            sources,
            direct_dependencies,
            standard_library,
            classify_workspace_paths,
        );
        let SchemaNavigationIndex {
            aliases: schema_aliases,
            package_schemas,
            composition_references: schema_composition_references,
            alias_module_imports: schema_alias_module_imports,
            bare_aliases: bare_schema_alias_index,
            operation_lookup: schema_operation_lookup_index,
        } = index_schema_navigation(&files, &workspace_module, &mut declarations);
        let type_indices_by_name = symbol_indices_by_name(&declarations.types);
        let function_indices_by_identity = function_indices_by_identity(&declarations.functions);
        let type_alias_indices_by_name = symbol_indices_by_name(&declarations.type_aliases);
        let package_type_alias_indices_by_name =
            package_symbol_indices_by_name(&declarations.type_aliases);
        let workspace_type_indices_by_module_and_name =
            workspace_symbol_indices_by_module_and_name(&declarations.types);
        let workspace_type_alias_indices_by_module_and_name =
            workspace_symbol_indices_by_module_and_name(&declarations.type_aliases);
        let package_type_alias_indices_by_module_and_name =
            package_symbol_indices_by_module_and_name(&declarations.type_aliases);
        let eligible_workspace_effect_indices =
            eligible_workspace_effect_indices(&declarations.effects, &files);
        let eligible_workspace_effect_operation_indices = eligible_workspace_effect_operation_indices(
            &declarations.operations,
            &eligible_workspace_effect_indices,
            &files,
        );
        files.extend(direct_dependencies.files.clone());
        files.extend(standard_library.files.clone());
        let file_indices_by_identity = file_indices_by_identity(&files);
        Self {
            schemas: declarations.schemas,
            schema_aliases,
            package_schemas,
            effects: declarations.effects,
            handlers: declarations.handlers,
            operations: declarations.operations,
            functions: declarations.functions,
            function_indices_by_identity,
            file_indices_by_identity,
            package_function_targets: declarations.package_function_targets,
            package_type_targets: declarations.package_type_targets,
            package_constructor_targets: declarations.package_constructor_targets,
            types: declarations.types,
            constructors: declarations.constructors,
            type_aliases: declarations.type_aliases,
            type_indices_by_name,
            type_alias_indices_by_name,
            package_type_alias_indices_by_name,
            workspace_type_indices_by_module_and_name,
            workspace_type_alias_indices_by_module_and_name,
            package_type_alias_indices_by_module_and_name,
            eligible_workspace_effect_indices,
            eligible_workspace_effect_operation_indices,
            schema_composition_references,
            schema_alias_module_imports,
            bare_schema_alias_index,
            schema_operation_lookup_index,
            files,
            function_rename_index: OnceLock::new(),
        }
    }
}

fn eligible_workspace_effect_operation_indices(
    operations: &[EffectOperationSymbol],
    eligible_effects: &BTreeMap<(String, String), usize>,
    files: &[IndexedFile],
) -> BTreeMap<(String, String, String), usize> {
    let recovery_by_file = files
        .iter()
        .filter(|file| workspace_navigation_file(file))
        .map(|file| {
            (
                file.source.path().as_str(),
                (
                    file.invalid_declaration_names
                        .iter()
                        .map(|span| (span.start.offset, span.end.offset))
                        .collect::<BTreeSet<_>>(),
                    file.recovered_effect_declarations
                        .iter()
                        .map(|span| (span.start.offset, span.end.offset))
                        .collect::<Vec<_>>(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut candidates = BTreeMap::<(String, String, String), Vec<usize>>::new();
    for (index, operation) in operations.iter().enumerate() {
        if operation.package.is_none() {
            candidates
                .entry((
                    operation.module.clone(),
                    operation.effect_name.clone(),
                    operation.name.clone(),
                ))
                .or_default()
                .push(index);
        }
    }
    let mut indices = BTreeMap::new();
    for (key, candidates) in candidates {
        let [index] = candidates.as_slice() else {
            continue;
        };
        let operation = &operations[*index];
        let declaration_range = (
            operation.declaration.span.start.offset,
            operation.declaration.span.end.offset,
        );
        let declaration_is_unrecovered = recovery_by_file
            .get(operation.declaration.span.file.as_str())
            .is_some_and(|(invalid_names, recovered_effects)| {
                !invalid_names.contains(&declaration_range)
                    && !recovered_effects.iter().any(|(start, end)| {
                        *start <= declaration_range.0 && declaration_range.1 <= *end
                    })
            });
        if eligible_effects
            .contains_key(&(operation.module.clone(), operation.effect_name.clone()))
            && declaration_is_unrecovered
        {
            indices.insert(key, *index);
        }
    }
    indices
}

fn index_workspace_input(
    sources: Vec<SourceFile>,
    direct_dependencies: &IndexedDependencies,
    standard_library: &IndexedDependencies,
    classify_workspace_paths: bool,
) -> WorkspaceIndexInput {
    let mut files = Vec::new();
    let mut declarations = FileDeclarations::default();
    let mut module = empty_surface_module();
    for source in sources {
        let (file, file_declarations, parsed) = index_workspace_source(source);
        declarations.extend(file_declarations);
        append_parsed_surface_module(&mut module, &file, &parsed);
        files.push(file);
    }
    declarations.extend(direct_dependencies.declarations.clone());
    declarations.extend(standard_library.declarations.clone());
    if classify_workspace_paths && workspace_needs_path_classification(&files, &module) {
        let mut complete_module = module.clone();
        append_surface_module(&mut complete_module, direct_dependencies.module.clone());
        append_surface_module(&mut complete_module, standard_library.module.clone());
        attach_classified_path_segments(&mut files, &module, &complete_module);
    }
    WorkspaceIndexInput {
        files,
        declarations,
        module,
    }
}

fn file_indices_by_identity(
    files: &[IndexedFile],
) -> HashMap<IndexedFileIdentity, usize> {
    let mut indices = HashMap::new();
    for (index, file) in files.iter().enumerate() {
        let (package, origin) = match &file.origin {
            IndexedOrigin::Workspace => (None, None),
            IndexedOrigin::Package {
                identity,
                standard_library,
                ..
            } => (
                Some(identity.clone()),
                Some(if *standard_library {
                    PackageOrigin::StandardLibrary
                } else {
                    PackageOrigin::DirectDependency
                }),
            ),
        };
        indices
            .entry((package, origin, file.source.path().as_str().to_string()))
            .or_insert(index);
    }
    indices
}

fn index_schema_navigation(
    files: &[IndexedFile],
    workspace_module: &veln_ast::SurfaceModule,
    declarations: &mut FileDeclarations,
) -> SchemaNavigationIndex {
    let alias_module_imports = index_schema_alias_module_imports(files);
    let alias_declarations = declarations
        .schema_aliases
        .iter()
        .chain(&declarations.schema_alias_blockers)
        .cloned()
        .collect::<Vec<_>>();
    let package_declarations = PackageSchemaDeclarations::new(
        &declarations.package_schema_alias_declarations,
        &declarations.package_schema_targets,
        &declarations.recovered_package_schema_targets,
    );
    let aliases = eligible_schema_aliases(
        std::mem::take(&mut declarations.schema_aliases),
        &package_declarations,
        &declarations.resolved_package_schema_aliases,
        veln_sema::resolved_schema_aliases(workspace_module),
    );
    let bare_aliases =
        bare_schema_alias_index(&declarations.schemas, &aliases, &alias_declarations);
    let operation_lookup = schema_operation_lookup_index(
        &declarations.schemas,
        &aliases,
        &declarations.package_schema_alias_declarations,
    );
    let mut composition_references = workspace_schema_composition_references(
        files,
        &declarations.schemas,
        &aliases,
        &alias_module_imports,
        veln_sema::resolved_schema_composition_references(workspace_module),
    );
    let package_schemas = package_schema_index(&declarations.schemas, &package_declarations);
    composition_references.extend(package_schema_composition_references(
        files,
        WorkspaceSchemaCompositionDeclarations {
            schemas: &declarations.schemas,
            schema_aliases: &alias_declarations,
            types: &declarations.types,
            type_aliases: &declarations.type_aliases,
        },
        &package_schemas,
        &operation_lookup.package_aliases,
        &aliases,
        &alias_module_imports,
    ));
    SchemaNavigationIndex {
        aliases,
        package_schemas,
        composition_references,
        alias_module_imports,
        bare_aliases,
        operation_lookup,
    }
}

trait NamedTypeSymbol {
    fn name(&self) -> &str;
    fn module(&self) -> &str;
    fn is_workspace_symbol(&self) -> bool;
}

impl NamedTypeSymbol for TypeSymbol {
    fn name(&self) -> &str {
        &self.name
    }

    fn module(&self) -> &str {
        &self.module
    }

    fn is_workspace_symbol(&self) -> bool {
        self.package.is_none()
    }
}

impl NamedTypeSymbol for TypeAliasSymbol {
    fn name(&self) -> &str {
        &self.name
    }

    fn module(&self) -> &str {
        &self.module
    }

    fn is_workspace_symbol(&self) -> bool {
        self.package.is_none()
    }
}

fn symbol_indices_by_name<T: NamedTypeSymbol>(symbols: &[T]) -> BTreeMap<String, Vec<usize>> {
    let mut by_name = BTreeMap::<String, Vec<usize>>::new();
    for (index, symbol) in symbols.iter().enumerate() {
        by_name
            .entry(symbol.name().to_string())
            .or_default()
            .push(index);
    }
    by_name
}

fn function_indices_by_identity(
    functions: &[FunctionSymbol],
) -> HashMap<FunctionIdentity, Vec<usize>> {
    let mut by_identity = HashMap::<_, Vec<usize>>::new();
    for (index, function) in functions.iter().enumerate() {
        by_identity
            .entry((
                function.package.clone(),
                function.package_origin,
                function.module.clone(),
                function.name.clone(),
            ))
            .or_default()
            .push(index);
    }
    by_identity
}

fn package_symbol_indices_by_name<T: NamedTypeSymbol>(
    symbols: &[T],
) -> BTreeMap<String, Vec<usize>> {
    let mut by_name = BTreeMap::<String, Vec<usize>>::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if !symbol.is_workspace_symbol() {
            by_name
                .entry(symbol.name().to_string())
                .or_default()
                .push(index);
        }
    }
    by_name
}

fn workspace_symbol_indices_by_module_and_name<T: NamedTypeSymbol>(
    symbols: &[T],
) -> BTreeMap<(String, String), Vec<usize>> {
    let mut by_module_and_name = BTreeMap::<(String, String), Vec<usize>>::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if symbol.is_workspace_symbol() {
            by_module_and_name
                .entry((symbol.module().to_string(), symbol.name().to_string()))
                .or_default()
                .push(index);
        }
    }
    by_module_and_name
}

fn eligible_workspace_effect_indices(
    effects: &[NeutralSymbol],
    files: &[IndexedFile],
) -> BTreeMap<(String, String), usize> {
    let recovered_by_file = files
        .iter()
        .filter(|file| workspace_navigation_file(file))
        .map(|file| {
            let mut ranges = file
                .recovered_effect_declarations
                .iter()
                .map(|span| (span.start.offset, span.end.offset))
                .collect::<Vec<_>>();
            ranges.sort_unstable();
            (file.source.path().as_str(), ranges)
        })
        .collect::<BTreeMap<_, _>>();
    let mut candidates = BTreeMap::<(String, String), Vec<usize>>::new();
    for (index, symbol) in effects.iter().enumerate() {
        #[cfg(test)]
        record_effect_declaration_index_visit();
        if symbol.package.is_none() {
            candidates
                .entry((symbol.module.clone(), symbol.name.clone()))
                .or_default()
                .push(index);
        }
    }
    candidates
        .into_iter()
        .filter_map(|(identity, candidates)| {
            let [index] = candidates.as_slice() else {
                return None;
            };
            let symbol = &effects[*index];
            let ranges = recovered_by_file.get(symbol.declaration.span.file.as_str())?;
            let insertion = ranges.partition_point(|(start, _)| {
                *start <= symbol.declaration.span.start.offset
            });
            let recovered = insertion.checked_sub(1).is_some_and(|range_index| {
                let (start, end) = ranges[range_index];
                start <= symbol.declaration.span.start.offset
                    && symbol.declaration.span.end.offset <= end
            });
            (!recovered).then_some((identity, *index))
        })
        .collect()
}

fn package_symbol_indices_by_module_and_name<T: NamedTypeSymbol>(
    symbols: &[T],
) -> BTreeMap<(String, String), Vec<usize>> {
    let mut by_module_and_name = BTreeMap::<(String, String), Vec<usize>>::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if !symbol.is_workspace_symbol() {
            by_module_and_name
                .entry((symbol.module().to_string(), symbol.name().to_string()))
                .or_default()
                .push(index);
        }
    }
    by_module_and_name
}
