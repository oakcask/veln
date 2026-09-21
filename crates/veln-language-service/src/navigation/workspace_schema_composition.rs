struct WorkspaceSchemaCompositionIndex<'a> {
    files: BTreeMap<&'a str, &'a IndexedFile>,
    schemas: BTreeMap<(&'a str, &'a str, &'a str), &'a NeutralSymbol>,
    aliases: BTreeMap<(&'a str, &'a str, &'a str), Vec<&'a NeutralSymbol>>,
}

impl<'a> WorkspaceSchemaCompositionIndex<'a> {
    fn new(
        files: &'a [IndexedFile],
        schemas: &'a [NeutralSymbol],
        aliases: &'a [NeutralSymbol],
    ) -> Self {
        let mut index = Self {
            files: BTreeMap::new(),
            schemas: BTreeMap::new(),
            aliases: BTreeMap::new(),
        };
        for file in files {
            index
                .files
                .entry(file.source.path().as_str())
                .or_insert(file);
        }
        for schema in schemas.iter().filter(|schema| schema.package.is_none()) {
            #[cfg(test)]
            record_workspace_schema_composition_schema_visit();
            index
                .schemas
                .entry((
                    schema.module.as_str(),
                    schema.name.as_str(),
                    schema.declaration.span.file.as_str(),
                ))
                .or_insert(schema);
        }
        for alias in aliases.iter().filter(|alias| alias.package.is_none()) {
            #[cfg(test)]
            record_workspace_schema_composition_alias_visit();
            index
                .aliases
                .entry((
                    alias.module.as_str(),
                    alias.name.as_str(),
                    alias.declaration.span.file.as_str(),
                ))
                .or_default()
                .push(alias);
        }
        index
    }

    fn schema_target(
        &self,
        reference: &veln_sema::ResolvedSchemaCompositionReference,
    ) -> Option<&'a NeutralSymbol> {
        self.schemas
            .get(&(
                reference.target_module.as_deref()?,
                reference.target_name.as_str(),
                reference.target_span.file.as_str(),
            ))
            .copied()
    }

    fn alias_target(
        &self,
        reference: &veln_sema::ResolvedSchemaCompositionReference,
    ) -> Option<&'a NeutralSymbol> {
        let span = reference.alias_span.as_ref()?;
        self.aliases
            .get(&(
                reference.alias_module.as_deref()?,
                reference.alias_name.as_deref()?,
                span.file.as_str(),
            ))?
            .iter()
            .copied()
            .find(|alias| {
                alias.declaration.span.start.offset >= span.start.offset
                    && alias.declaration.span.end.offset <= span.end.offset
            })
    }

    fn file(&self, span: &SourceSpan) -> Option<&'a IndexedFile> {
        self.files.get(span.file.as_str()).copied()
    }
}

fn workspace_schema_composition_references(
    files: &[IndexedFile],
    schemas: &[NeutralSymbol],
    schema_aliases: &[NeutralSymbol],
    module_imports: &BTreeMap<String, SchemaAliasModuleImports>,
    references: Vec<veln_sema::ResolvedSchemaCompositionReference>,
) -> Vec<SchemaCompositionReference> {
    let index = WorkspaceSchemaCompositionIndex::new(files, schemas, schema_aliases);
    references
        .into_iter()
        .filter_map(|reference| {
            workspace_schema_composition_reference(&index, module_imports, &reference)
        })
        .collect()
}

fn workspace_schema_composition_reference(
    index: &WorkspaceSchemaCompositionIndex<'_>,
    module_imports: &BTreeMap<String, SchemaAliasModuleImports>,
    reference: &veln_sema::ResolvedSchemaCompositionReference,
) -> Option<SchemaCompositionReference> {
    let schema_target = index.schema_target(reference)?;
    let alias_target = index.alias_target(reference);
    let leaf = reference.path.last()?;
    let file = index.file(&reference.field_span)?;
    let (token_index, token) = workspace_schema_composition_token(file, &reference.field_span, leaf)?;
    let target = alias_target.map_or(schema_target, |alias| alias);
    if qualifier_for_token(&file.tokens, token_index).is_some_and(|qualifier| {
        !matches!(
            schema_qualified_workspace_module(file, &qualifier, module_imports),
            QualifiedWorkspaceModule::Workspace(module) if module == target.module
        )
    }) {
        return None;
    }
    Some(SchemaCompositionReference {
        span: file.source.span(token.range),
        target: alias_target.map_or_else(
            || SchemaReferenceTarget::Schema(schema_target.clone()),
            |alias| SchemaReferenceTarget::Alias(alias.clone()),
        ),
    })
}

fn workspace_schema_composition_token<'a>(
    file: &'a IndexedFile,
    span: &SourceSpan,
    leaf: &str,
) -> Option<(usize, &'a Token)> {
    let first = file
        .tokens
        .partition_point(|token| token.range.end <= span.start.offset);
    file.tokens
        .iter()
        .enumerate()
        .skip(first)
        .take_while(|(_, token)| token.range.start < span.end.offset)
        .find(|(index, token)| {
            #[cfg(test)]
            record_workspace_schema_composition_token_visit();
            token.range.end <= span.end.offset
                && token.text == leaf
                && is_schema_composition_path_leaf_token(&file.tokens, *index)
        })
}
