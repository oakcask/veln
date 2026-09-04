use super::*;

pub(super) fn load_project_sources(
    project: &Project,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    package: Option<&str>,
    exported_source_paths: Option<&BTreeSet<String>>,
    checked_export_source_paths: Option<&mut BTreeSet<String>>,
) {
    let mut checked_export_source_paths = checked_export_source_paths;
    for source in &project.files {
        if package.is_some() && classify_companion_source(source.path().as_str()).is_some() {
            continue;
        }
        #[cfg(test)]
        if package == Some(veln_stdlib::PACKAGE_NAME) {
            embedded_standard_counters::record_runtime_standard_parse_lower();
        }
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        let is_exported_source =
            exported_source_paths.is_some_and(|paths| paths.contains(source.path().as_str()));
        if is_exported_source
            && let Some(checked_export_source_paths) = checked_export_source_paths.as_deref_mut()
        {
            checked_export_source_paths.insert(source.path().as_str().to_string());
        }
        if !parsed.diagnostics.is_empty() {
            derive_source_module(source, diagnostics, is_exported_source);
            record_rejected_source_module(source, parts, package);
            continue;
        }
        let derived_module = derive_and_record_source_module(
            source,
            diagnostics,
            parts,
            package,
            is_exported_source,
        );
        process_parsed_source(
            source,
            &parsed.tree,
            diagnostics,
            parts,
            package,
            derived_module,
        );
    }
}

fn process_parsed_source(
    source: &SourceFile,
    tree: &veln_syntax::SyntaxTree,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    package: Option<&str>,
    derived_module: Option<String>,
) {
    push_source_parse_semantic_diagnostics(tree, diagnostics);
    let mut lowered = lower_source_tree(source, tree, derived_module, package);
    rewrite_import_targets(&mut lowered.uses, package);
    if parts.module.module.is_none() {
        parts.module.module = lowered.module;
    }
    parts.module.uses.extend(lowered.uses);
    parts.module.aliases.extend(lowered.aliases);
    parts.module.effects.extend(lowered.effects);
    parts.module.handlers.extend(lowered.handlers);
    parts.module.types.extend(lowered.types);
    parts.module.schemas.extend(lowered.schemas);
    parts.module.functions.extend(lowered.functions);
    parts.module.invalid_names.extend(lowered.invalid_names);
}

fn push_source_parse_semantic_diagnostics(
    tree: &veln_syntax::SyntaxTree,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(module) = &tree.module {
        diagnostics.push(source_mod_decl_diagnostic(module));
    }
    for use_decl in &tree.uses {
        if use_decl.name.contains('.') {
            diagnostics.push(dotted_use_decl_diagnostic(use_decl));
        }
    }
}

fn derive_and_record_source_module(
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    package: Option<&str>,
    is_exported_source: bool,
) -> Option<String> {
    let source_kind = source_module_kind(is_exported_source);
    match derive_visible_source_module_path_with_source_kind(source, source_kind) {
        Ok(Some(module_name)) => {
            record_derived_source_module(source, &module_name, diagnostics, parts, package);
            Some(module_name)
        }
        Ok(None) => None,
        Err(source_diagnostics) => {
            record_rejected_source_module(source, parts, package);
            diagnostics.extend(source_diagnostics);
            None
        }
    }
}

fn derive_source_module(
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    is_exported_source: bool,
) -> Option<String> {
    match derive_visible_source_module_path_with_source_kind(
        source,
        source_module_kind(is_exported_source),
    ) {
        Ok(module_name) => module_name,
        Err(source_diagnostics) => {
            diagnostics.extend(source_diagnostics);
            None
        }
    }
}

fn source_module_kind(is_exported_source: bool) -> &'static str {
    if is_exported_source {
        "export"
    } else {
        "regular"
    }
}

fn record_derived_source_module(
    source: &SourceFile,
    module_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    package: Option<&str>,
) {
    let internal_module_name = internal_module_name(package, module_name);
    if module_name == "prelude" && package != Some(veln_stdlib::PACKAGE_NAME) {
        diagnostics.push(reserved_source_module_diagnostic(source, module_name));
    }
    if is_doctest_source(source) {
        return;
    }
    if let Some((_, first_source)) = parts
        .derived_modules
        .iter()
        .find(|(known_module, _)| known_module == &internal_module_name)
    {
        diagnostics.push(duplicate_derived_module_diagnostic(
            module_name,
            source,
            first_source,
        ));
    } else {
        parts
            .derived_modules
            .push((internal_module_name, source.clone()));
    }
}

fn record_rejected_source_module(
    source: &SourceFile,
    parts: &mut SurfaceParts,
    package: Option<&str>,
) {
    if let Some(module_name) = invalid_case_rejected_visible_module_path(source) {
        let internal_module_name = internal_module_name(package, &module_name);
        parts.rejected_derived_modules.insert(internal_module_name);
    }
}

pub(super) fn lower_source_tree(
    source: &SourceFile,
    tree: &veln_syntax::SyntaxTree,
    derived_module: Option<String>,
    package: Option<&str>,
) -> SurfaceModule {
    match derived_module {
        Some(module_name) => {
            let internal_module_name = internal_module_name(package, &module_name);
            lower_surface_ast_with_module_identity(
                tree,
                internal_module_name,
                source.span(TextRange::new(0, 0)),
            )
        }
        None => lower_surface_ast(tree),
    }
}

fn rewrite_import_targets(uses: &mut [UseDecl], package: Option<&str>) {
    for use_decl in uses {
        if let Some(package) = &use_decl.package {
            use_decl.name = external_module_key(package, &use_decl.name);
        } else if let Some(package) = package {
            use_decl.name = external_module_key(package, &use_decl.name);
        }
    }
}

pub(super) fn rewrite_standard_import_targets(uses: &mut [UseDecl]) {
    for use_decl in uses {
        if use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
            && !use_decl.name.starts_with("std::")
        {
            use_decl.name = external_module_key(veln_stdlib::PACKAGE_NAME, &use_decl.name);
        }
    }
}

pub(super) fn internal_module_name(package: Option<&str>, module_name: &str) -> String {
    package.map_or_else(
        || module_name.to_string(),
        |package| external_module_key(package, module_name),
    )
}

pub(super) fn external_module_key(package: &str, module_name: &str) -> String {
    format!("{package}::{module_name}")
}
