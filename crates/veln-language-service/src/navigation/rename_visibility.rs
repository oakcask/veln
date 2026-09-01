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
