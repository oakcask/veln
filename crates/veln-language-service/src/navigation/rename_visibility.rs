fn type_visible_after_rename(file: &IndexedFile, selected: &TypeSymbol) -> bool {
    matches!(file.origin, IndexedOrigin::Workspace)
        && selected.package.is_none()
        && (file.module == selected.module || (selected.public && file.uses.contains(&selected.module)))
}

fn function_visible_after_rename(file: &IndexedFile, selected: &FunctionSymbol) -> bool {
    matches!(file.origin, IndexedOrigin::Workspace)
        && selected.package.is_none()
        && (file.module == selected.module
            || (selected.public
                && (file.uses.contains(&selected.module)
                    || file
                        .companion_target_module
                        .as_ref()
                        .is_some_and(|target| target == &selected.module))))
}

fn workspace_module_is_barely_imported(file: &IndexedFile, module: &str) -> bool {
    file.workspace_imports.iter().any(|import| {
        import.syntax_valid
            && import.bare_exports
            && import.module == module
    })
}

fn module_value_visible_after_rename(file: &IndexedFile, selected: &ModuleValueSymbol) -> bool {
    matches!(file.origin, IndexedOrigin::Workspace)
        && selected.package.is_none()
        && (file.module == selected.module
            || (selected.public
                && (workspace_module_is_barely_imported(file, &selected.module)
                    || file
                        .companion_target_module
                        .as_ref()
                        .is_some_and(|target| target == &selected.module))))
}

fn workspace_module_value_visible_in_file(
    file: &IndexedFile,
    candidate: &ModuleValueSymbol,
) -> bool {
    candidate.package.is_none()
        && (candidate.module == file.module
            || (candidate.public
                && workspace_module_is_barely_imported(file, &candidate.module)))
}
