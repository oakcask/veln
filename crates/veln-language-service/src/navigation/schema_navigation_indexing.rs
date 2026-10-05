struct WorkspaceSchemaCompositionDeclarations<'a> {
    schemas: &'a [NeutralSymbol],
    schema_aliases: &'a [NeutralSymbol],
    types: &'a [TypeSymbol],
    type_aliases: &'a [TypeAliasSymbol],
}

fn package_schema_composition_references(
    files: &[IndexedFile],
    workspace: WorkspaceSchemaCompositionDeclarations<'_>,
    schema_index: &BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol>,
    alias_index: &BTreeMap<(String, String, String), Vec<NeutralSymbol>>,
    schema_aliases: &[NeutralSymbol],
    module_imports: &BTreeMap<String, SchemaAliasModuleImports>,
) -> Vec<SchemaCompositionReference> {
    let prelude_alias_index = standard_prelude_schema_alias_index(schema_aliases);
    let workspace_schema_blockers = workspace_schema_blocker_index(workspace.schemas);
    let workspace_schema_alias_blockers =
        workspace_schema_alias_blocker_index(workspace.schema_aliases);
    let workspace_type_blockers =
        workspace_type_blocker_index(workspace.types, workspace.type_aliases);
    let context = SchemaCompositionNavigationContext {
        schema_index,
        alias_index,
        prelude_alias_index: &prelude_alias_index,
        workspace_schema_blockers: &workspace_schema_blockers,
        workspace_schema_alias_blockers: &workspace_schema_alias_blockers,
        workspace_type_blockers: &workspace_type_blockers,
        module_imports,
    };
    files
        .iter()
        .filter(|file| workspace_navigation_file(file))
        .flat_map(|file| {
            let mut token_cursor = 0usize;
            file.schema_composition_leaf_spans
                .iter()
                .filter_map(|span| {
                    context.direct_dependency_schema_composition_reference(
                        file,
                        span,
                        &mut token_cursor,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn workspace_schema_blocker_index(
    schemas: &[NeutralSymbol],
) -> BTreeSet<(String, String)> {
    schemas
        .iter()
        .filter(|schema| schema.package.is_none())
        .map(|schema| (schema.module.clone(), schema.name.clone()))
        .collect()
}

fn workspace_schema_alias_blocker_index(
    aliases: &[NeutralSymbol],
) -> BTreeSet<(String, String)> {
    aliases
        .iter()
        .filter(|alias| alias.package.is_none())
        .map(|alias| (alias.module.clone(), alias.name.clone()))
        .collect()
}

fn workspace_type_blocker_index(
    types: &[TypeSymbol],
    type_aliases: &[TypeAliasSymbol],
) -> BTreeSet<(String, String)> {
    types
        .iter()
        .filter(|symbol| symbol.package.is_none())
        .map(|symbol| (symbol.module.clone(), symbol.name.clone()))
        .chain(
            type_aliases
                .iter()
                .filter(|symbol| symbol.package.is_none())
                .map(|symbol| (symbol.module.clone(), symbol.name.clone())),
        )
        .collect()
}

fn standard_prelude_schema_alias_index(
    schema_aliases: &[NeutralSymbol],
) -> BTreeMap<String, Vec<NeutralSymbol>> {
    let mut aliases = BTreeMap::new();
    for alias in schema_aliases.iter().filter(|alias| alias.standard_prelude) {
        aliases
            .entry(alias.name.clone())
            .or_insert_with(Vec::new)
            .push(alias.clone());
    }
    aliases
}

fn bare_schema_alias_index(
    schemas: &[NeutralSymbol],
    eligible_aliases: &[NeutralSymbol],
    alias_declarations: &[NeutralSymbol],
) -> BareSchemaAliasIndex {
    let mut workspace_aliases = BTreeMap::new();
    for alias in eligible_aliases
        .iter()
        .filter(|alias| alias.package.is_none())
    {
        workspace_aliases
            .entry((alias.module.clone(), alias.name.clone()))
            .or_insert_with(|| alias.clone());
    }
    BareSchemaAliasIndex {
        workspace_aliases,
        workspace_schemas: workspace_schema_blocker_index(schemas),
        workspace_alias_declarations: workspace_schema_alias_blocker_index(alias_declarations),
        standard_prelude_aliases: standard_prelude_schema_alias_index(eligible_aliases),
        standard_prelude_alias_declarations: alias_declarations
            .iter()
            .filter(|alias| alias.standard_prelude)
            .map(|alias| alias.name.clone())
            .collect(),
    }
}

fn schema_operation_lookup_index(
    schemas: &[NeutralSymbol],
    schema_aliases: &[NeutralSymbol],
    alias_declarations: &[PackageSchemaAliasDeclaration],
) -> SchemaOperationLookupIndex {
    let mut index = SchemaOperationLookupIndex::default();
    for schema in schemas {
        if let Some(package) = &schema.package {
            if matches!(
                schema.package_origin,
                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
            ) {
                index
                    .package_schemas
                    .entry((package.clone(), schema.module.clone(), schema.name.clone()))
                    .or_default()
                    .push(schema.clone());
            }
        } else {
            index
                .workspace_schemas
                .entry((schema.module.clone(), schema.name.clone()))
                .or_default()
                .push(schema.clone());
        }
    }
    for alias in schema_aliases {
        if let Some(package) = &alias.package {
            if matches!(
                alias.package_origin,
                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
            ) {
                index
                    .package_aliases
                    .entry((package.clone(), alias.module.clone(), alias.name.clone()))
                    .or_default()
                    .push(alias.clone());
            }
        } else {
            index
                .workspace_aliases
                .entry((alias.module.clone(), alias.name.clone()))
                .or_default()
                .push(alias.clone());
        }
    }
    index.package_alias_declarations = alias_declarations
        .iter()
        .filter(|alias| {
            matches!(
                alias.package_origin,
                PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary
            )
        })
        .map(|alias| (alias.package.clone(), alias.module.clone(), alias.name.clone()))
        .collect();
    index
}

struct SchemaCompositionNavigationContext<'a> {
    schema_index: &'a BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol>,
    alias_index: &'a BTreeMap<(String, String, String), Vec<NeutralSymbol>>,
    prelude_alias_index: &'a BTreeMap<String, Vec<NeutralSymbol>>,
    workspace_schema_blockers: &'a BTreeSet<(String, String)>,
    workspace_schema_alias_blockers: &'a BTreeSet<(String, String)>,
    workspace_type_blockers: &'a BTreeSet<(String, String)>,
    module_imports: &'a BTreeMap<String, SchemaAliasModuleImports>,
}

impl SchemaCompositionNavigationContext<'_> {
    fn direct_dependency_schema_composition_reference(
        &self,
        file: &IndexedFile,
        span: &SourceSpan,
        token_cursor: &mut usize,
    ) -> Option<SchemaCompositionReference> {
        let token = schema_composition_token(file, span, token_cursor)?;
        let Some(qualifier) = qualifier_for_token(&file.tokens, *token_cursor) else {
            return self.bare_prelude_schema_composition_reference(file, span, &token.text);
        };
        match schema_qualified_workspace_module(file, &qualifier, self.module_imports) {
            QualifiedWorkspaceModule::Unresolved if qualifier == "prelude" => {
                self.prelude_schema_composition_reference(span, &token.text)
            }
            QualifiedWorkspaceModule::External => {
                self.imported_schema_composition_reference(file, span, &qualifier, &token.text)
            }
            QualifiedWorkspaceModule::Workspace(_)
            | QualifiedWorkspaceModule::Ambiguous
            | QualifiedWorkspaceModule::Unresolved => None,
        }
    }

    fn bare_prelude_schema_composition_reference(
        &self,
        file: &IndexedFile,
        span: &SourceSpan,
        name: &str,
    ) -> Option<SchemaCompositionReference> {
        let blocker = (file.module.clone(), name.to_string());
        #[cfg(test)]
        record_schema_composition_blocker_lookup();
        if self.workspace_schema_blockers.contains(&blocker)
            || self.workspace_schema_alias_blockers.contains(&blocker)
            || self.workspace_type_blockers.contains(&blocker)
        {
            return None;
        }
        self.prelude_schema_composition_reference(span, name)
    }

    fn prelude_schema_composition_reference(
        &self,
        span: &SourceSpan,
        name: &str,
    ) -> Option<SchemaCompositionReference> {
        #[cfg(test)]
        record_schema_composition_prelude_lookup();
        let [alias] = self.prelude_alias_index.get(name)?.as_slice() else {
            return None;
        };
        Some(SchemaCompositionReference {
            span: span.clone(),
            target: SchemaReferenceTarget::Alias(alias.clone()),
        })
    }

    fn imported_schema_composition_reference(
        &self,
        file: &IndexedFile,
        span: &SourceSpan,
        qualifier: &str,
        name: &str,
    ) -> Option<SchemaCompositionReference> {
        let (module, package) = self
            .module_imports
            .get(&file.module)?
            .valid_external_route(qualifier)?;
        let package_origin = if package == "std" {
            PackageOrigin::StandardLibrary
        } else {
            PackageOrigin::DirectDependency
        };
        let alias_key = (package.clone(), module.clone(), name.to_string());
        let key = (package_origin, package, module, name.to_string());
        let target = match self.alias_index.get(&alias_key) {
            Some(candidates) if candidates.len() == 1 => {
                SchemaReferenceTarget::Alias(candidates[0].clone())
            }
            Some(_) => return None,
            None => SchemaReferenceTarget::Schema(
                package_schema_target(self.schema_index, &key)?.clone(),
            ),
        };
        Some(SchemaCompositionReference {
            span: span.clone(),
            target,
        })
    }
}

fn schema_composition_token<'a>(
    file: &'a IndexedFile,
    span: &SourceSpan,
    token_cursor: &mut usize,
) -> Option<&'a Token> {
    while *token_cursor < file.tokens.len()
        && file.tokens[*token_cursor].range.end <= span.start.offset
    {
        *token_cursor += 1;
    }
    let token = file.tokens.get(*token_cursor)?;
    (token.range.start == span.start.offset
        && token.range.end == span.end.offset
        && token
            .text
            .chars()
            .next()
            .is_some_and(|initial| initial.is_ascii_uppercase()))
    .then_some(token)
}

fn package_schema_target<'a>(
    schema_index: &'a BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol>,
    key: &(PackageOrigin, String, String, String),
) -> Option<&'a NeutralSymbol> {
    #[cfg(test)]
    {
        SCHEMA_COMPOSITION_TARGET_LOOKUPS.set(SCHEMA_COMPOSITION_TARGET_LOOKUPS.get() + 1);
    }
    schema_index.get(key)
}

fn package_schema_index(
    schemas: &[NeutralSymbol],
    declarations: &PackageSchemaDeclarations<'_>,
) -> BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol> {
    let mut candidates = BTreeMap::new();
    for schema in schemas
        .iter()
        .filter(|schema| {
            matches!(
                schema.package_origin,
                Some(PackageOrigin::DirectDependency | PackageOrigin::StandardLibrary)
            )
        })
    {
        #[cfg(test)]
        record_schema_composition_declaration_visit();
        let Some(package) = schema.package.as_ref() else {
            continue;
        };
        candidates
            .entry((
                schema.package_origin.expect("package schema has an origin"),
                package.clone(),
                schema.module.clone(),
                schema.name.clone(),
            ))
            .or_insert_with(Vec::new)
            .push(schema);
    }

    candidates
        .into_iter()
        .filter_map(|(identity, candidates)| {
            (candidates.len() == 1
                && declarations.contains_schema(&(
                    identity.0,
                    &identity.1,
                    &identity.2,
                    &identity.3,
                )))
            .then(|| (identity, candidates[0].clone()))
        })
        .collect()
}

fn eligible_schema_aliases(
    aliases: Vec<NeutralSymbol>,
    declarations: &PackageSchemaDeclarations<'_>,
    resolved_package_aliases: &[ResolvedPackageSchemaAlias],
    resolved: Vec<veln_sema::ResolvedSchemaAlias>,
) -> Vec<NeutralSymbol> {
    let package_eligibility =
        PackageSchemaAliasEligibility::new(declarations, resolved_package_aliases);
    let workspace_aliases = workspace_resolved_schema_alias_index(&resolved);
    aliases
        .iter()
        .filter(|alias| match alias.package_origin {
            None => {
                #[cfg(test)]
                record_workspace_schema_alias_resolution_lookup();
                let identity = (
                    alias.module.as_str(),
                    alias.name.as_str(),
                    &alias.declaration.span.file,
                );
                workspace_aliases
                    .get(&identity)
                    .is_some_and(|candidates| candidates.iter().any(|candidate| {
                        #[cfg(test)]
                        record_workspace_schema_alias_resolution_candidate_visit();
                        alias.declaration.span.start.offset >= candidate.alias_span.start.offset
                            && alias.declaration.span.end.offset <= candidate.alias_span.end.offset
                    }))
            }
            Some(PackageOrigin::DirectDependency) => package_eligibility.contains(alias),
            Some(PackageOrigin::StandardLibrary) => package_eligibility.contains(alias),
        })
        .cloned()
        .collect()
}

fn workspace_resolved_schema_alias_index(
    aliases: &[veln_sema::ResolvedSchemaAlias],
) -> BTreeMap<(&str, &str, &SourcePath), Vec<&veln_sema::ResolvedSchemaAlias>> {
    let mut index = BTreeMap::new();
    for alias in aliases {
        #[cfg(test)]
        record_workspace_schema_alias_resolution_index_visit();
        let Some(module) = alias.alias_module.as_deref() else {
            continue;
        };
        index
            .entry((
                module,
                alias.alias_name.as_str(),
                &alias.alias_span.file,
            ))
            .or_insert_with(Vec::new)
            .push(alias);
    }
    index
}
