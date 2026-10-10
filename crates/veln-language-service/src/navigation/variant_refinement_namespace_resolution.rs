impl SymbolIndex {
    fn unique_variant_refinement_type_namespace_for_reference(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        if let Some(qualifier) = qualifier_for_token(tokens, token_index) {
            return self.unique_variant_refinement_type_namespace_for_qualified_reference(
                file, &qualifier, name,
            );
        }
        self.unique_variant_refinement_type_namespace_for_bare_reference(file, name)
    }

    fn unique_variant_refinement_type_namespace_for_bare_reference(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        let local = self.variant_refinement_local_type_namespace_candidates(file, name);
        if !local.is_empty() {
            return unique_variant_refinement_type_namespace(local);
        }

        let imported = self.variant_refinement_imported_type_namespace_candidates(file, name);
        if !imported.is_empty() {
            return unique_variant_refinement_type_namespace(imported);
        }

        unique_variant_refinement_type_namespace(
            self.variant_refinement_prelude_type_namespace_candidates(file, name),
        )
    }

    fn variant_refinement_local_type_namespace_candidates(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Vec<TypeConflictCandidate> {
        self
            .workspace_types_in_module(&file.module, name)
            .cloned()
            .map(TypeConflictCandidate::Type)
            .chain(
                self.workspace_type_aliases_in_module(&file.module, name)
                    .cloned()
                    .map(TypeConflictCandidate::Alias),
            )
            .collect()
    }

    fn variant_refinement_imported_type_namespace_candidates(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Vec<TypeConflictCandidate> {
        let workspace_imports = file.uses.iter().flat_map(|module| {
            self.workspace_types_in_module(module, name)
                .filter(|symbol| visible_imported_type_for_bare_reference(file, symbol, name))
                .cloned()
                .map(TypeConflictCandidate::Type)
                .chain(
                    self.workspace_type_aliases_in_module(module, name)
                        .filter(|symbol| {
                            visible_imported_type_alias_for_bare_reference(file, symbol, name)
                        })
                        .cloned()
                        .map(TypeConflictCandidate::Alias),
                )
        });
        let package_imports = (!file.external_uses.is_empty())
            .then(|| {
                self.types_named(name)
                    .filter(|symbol| {
                        !symbol.standard_prelude
                            && visible_imported_type_for_bare_reference(file, symbol, name)
                    })
                    .cloned()
                    .map(TypeConflictCandidate::Type)
            })
            .into_iter()
            .flatten();
        workspace_imports.chain(package_imports).collect()
    }

    fn variant_refinement_prelude_type_namespace_candidates(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Vec<TypeConflictCandidate> {
        self
            .types_named(name)
            .filter(|symbol| {
                symbol.standard_prelude
                    && visible_imported_type_for_bare_reference(file, symbol, name)
            })
            .cloned()
            .map(TypeConflictCandidate::Type)
            .chain(
                self.type_aliases_named(name)
                    .filter(|symbol| {
                        symbol.standard_prelude
                            && visible_imported_type_alias_for_bare_reference(file, symbol, name)
                    })
                    .cloned()
                    .map(TypeConflictCandidate::Alias),
            )
            .collect()
    }

    fn unique_variant_refinement_type_namespace_for_qualified_reference(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<TypeConflictCandidate> {
        let (external_routes, workspace_routes) =
            variant_refinement_qualified_routes(file, qualifier);
        let mut candidates = self.variant_refinement_qualified_workspace_candidates(
            file,
            name,
            &workspace_routes,
        );
        candidates.extend(self.variant_refinement_qualified_package_candidates(
            name,
            &external_routes,
        ));
        unique_variant_refinement_type_namespace(candidates)
    }

    fn variant_refinement_qualified_workspace_candidates(
        &self,
        file: &IndexedFile,
        name: &str,
        workspace_routes: &BTreeSet<String>,
    ) -> Vec<TypeConflictCandidate> {
        workspace_routes
            .iter()
            .flat_map(|module| {
                self.workspace_types_in_module(module, name)
                    .filter(|symbol| visible_qualified_workspace_type(file, symbol))
                    .cloned()
                    .map(TypeConflictCandidate::Type)
                    .chain(
                        self.workspace_type_aliases_in_module(module, name)
                            .cloned()
                            .map(TypeConflictCandidate::Alias),
                    )
            })
            .collect()
    }

    fn variant_refinement_qualified_package_candidates(
        &self,
        name: &str,
        external_routes: &BTreeSet<(String, String)>,
    ) -> Vec<TypeConflictCandidate> {
        let package_types = (!external_routes.is_empty())
            .then(|| {
                self.types_named(name)
                    .filter(|symbol| visible_qualified_package_type(symbol, external_routes))
                    .cloned()
                    .map(TypeConflictCandidate::Type)
            })
            .into_iter()
            .flatten();
        let package_aliases = external_routes.iter().flat_map(|(module, package)| {
            self.package_type_aliases_in_module(module, name)
                .filter(|symbol| symbol.package.as_deref() == Some(package.as_str()))
                .cloned()
                .map(TypeConflictCandidate::Alias)
        });
        package_types.chain(package_aliases).collect()
    }

}

fn variant_refinement_qualified_routes(
    file: &IndexedFile,
    qualifier: &str,
) -> (BTreeSet<(String, String)>, BTreeSet<String>) {
    let (head, suffix) = qualifier
        .split_once("::")
        .map_or((qualifier, None), |(head, suffix)| (head, Some(suffix)));
    let external_routes = file
        .external_uses
        .iter()
        .filter_map(|(module, package)| {
            variant_refinement_qualified_route(module, qualifier, head, suffix)
                .map(|module| (module, package.clone()))
        })
        .collect();
    let workspace_routes = file
        .uses
        .iter()
        .filter_map(|module| {
            variant_refinement_qualified_route(module, qualifier, head, suffix)
        })
        .chain((qualifier == file.module).then(|| file.module.clone()))
        .collect();
    (external_routes, workspace_routes)
}

fn variant_refinement_qualified_route(
    module: &str,
    qualifier: &str,
    head: &str,
    suffix: Option<&str>,
) -> Option<String> {
    if qualifier == module {
        return Some(module.to_string());
    }
    (module.rsplit("::").next() == Some(head)).then(|| {
        suffix.map_or_else(
            || module.to_string(),
            |suffix| format!("{module}::{suffix}"),
        )
    })
}

fn visible_qualified_workspace_type(file: &IndexedFile, symbol: &TypeSymbol) -> bool {
    symbol.public
        || symbol.module == file.module
        || file
            .companion_target_module
            .as_ref()
            .is_some_and(|target| target == &symbol.module)
}

fn visible_qualified_package_type(
    symbol: &TypeSymbol,
    external_routes: &BTreeSet<(String, String)>,
) -> bool {
    symbol.public
        && symbol.package.as_deref().is_some_and(|package| {
            external_routes.contains(&(symbol.module.clone(), package.to_string()))
        })
}
