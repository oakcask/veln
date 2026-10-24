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
        if package.is_some_and(|package| package != veln_stdlib::PACKAGE_NAME)
            && classify_companion_source(source.path().as_str()).is_some()
        {
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
        if parsed
            .diagnostics
            .iter()
            .any(|diagnostic| !diagnostic.allows_semantic_analysis())
        {
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
        Some(module_name) => match package {
            Some(package_name) => lower_surface_ast_with_package_module_identity(
                tree,
                package_name.to_string(),
                module_name,
                source.span(TextRange::new(0, 0)),
            ),
            None => lower_surface_ast_with_module_identity(
                tree,
                module_name,
                source.span(TextRange::new(0, 0)),
            ),
        },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercase_final_parse_failure_retains_the_source_for_semantic_loading() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "pub type State\n",
                    "  Hidden\n",
                    "end\n",
                    "pub fn exposed(value: State::hidden) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };
        let mut diagnostics = Vec::new();
        let mut parts = SurfaceParts::new();

        load_project_sources(&project, &mut diagnostics, &mut parts, None, None, None);

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "parse.variant_refinement_type"),
            "{diagnostics:#?}"
        );
        assert_eq!(parts.module.types.len(), 1, "{:#?}", parts.module.types);
        assert_eq!(
            parts.module.functions.len(),
            1,
            "{:#?}",
            parts.module.functions
        );
        assert_eq!(parts.module.functions[0].params[0].ty_refinements.len(), 1);
        assert_eq!(
            parts.module.functions[0].params[0].ty_refinements[0].alternatives[0].variant,
            "hidden"
        );
    }

    #[test]
    fn combined_lowercase_candidate_retains_dual_source_metadata() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "pub type State\n",
                    "  Ready\n",
                    "end\n",
                    "pub fn exposed(value: state::ready) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };
        let mut diagnostics = Vec::new();
        let mut parts = SurfaceParts::new();

        load_project_sources(&project, &mut diagnostics, &mut parts, None, None, None);

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let param = &parts.module.functions[0].params[0];
        assert_eq!(param.ty_paths[0].segments, ["state", "ready"]);
        assert_eq!(param.ty_refinements.len(), 1);
        assert_eq!(param.ty_refinements[0].alternatives[0].variant, "ready");
    }

    #[test]
    fn mixed_union_and_generic_base_failure_candidates_reach_semantic_loading() {
        let text = concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "type GenericEmpty<A>\n",
            "end\n",
            "fn mixed(value: state::ready | State::closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn generic(value: genericEmpty<Int>::missing) -> ()\n",
            "  ()\n",
            "end\n",
        );
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new("main.veln", text)],
            manifest: None,
        };
        let mut diagnostics = Vec::new();
        let mut parts = SurfaceParts::new();

        load_project_sources(&project, &mut diagnostics, &mut parts, None, None, None);

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "parse.variant_refinement_type")
                .count(),
            2,
            "{diagnostics:#?}"
        );
        assert_eq!(
            parts.module.functions.len(),
            2,
            "{:#?}",
            parts.module.functions
        );
        let mixed = &parts.module.functions[0].params[0].ty_refinements[0];
        assert_eq!(mixed.alternatives.len(), 2);
        assert_eq!(mixed.alternatives[0].base.segments, ["state"]);
        assert_eq!(mixed.alternatives[0].variant, "ready");
        assert_eq!(mixed.alternatives[1].base.segments, ["State"]);
        assert_eq!(mixed.alternatives[1].variant, "closed");
        let generic = &parts.module.functions[1].params[0].ty_refinements[0].alternatives[0];
        assert_eq!(generic.base.segments, ["genericEmpty"]);
        assert_eq!(generic.type_arguments.len(), 1);
        assert_eq!(generic.type_arguments[0].ty_fragments, ["Int"]);
        assert_eq!(generic.variant, "missing");
        assert_eq!(
            &text[generic.base.segment_spans[0].start.offset
                ..generic.base.segment_spans[0].end.offset],
            "genericEmpty"
        );
        assert_eq!(
            &text[generic.type_arguments[0].span.start.offset
                ..generic.type_arguments[0].span.end.offset],
            "Int"
        );
        assert_eq!(
            &text[generic.variant_span.start.offset..generic.variant_span.end.offset],
            "missing"
        );
        assert_eq!(
            &text[generic.span.start.offset..generic.span.end.offset],
            "genericEmpty<Int>::missing"
        );
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.variant_refinement_type"
                && diagnostic.span.as_ref() == Some(&generic.variant_span)
        }));
    }

    #[test]
    fn unrelated_variant_refinement_parse_failure_still_excludes_semantic_loading() {
        let project = Project {
            root: ".".into(),
            files: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "pub type State\n",
                    "  Hidden\n",
                    "end\n",
                    "pub fn recoverable(value: State::hidden) -> ()\n",
                    "  ()\n",
                    "end\n",
                    "pub fn exposed(value: State::) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            )],
            manifest: None,
        };
        let mut diagnostics = Vec::new();
        let mut parts = SurfaceParts::new();

        load_project_sources(&project, &mut diagnostics, &mut parts, None, None, None);

        assert!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "parse.variant_refinement_type")
                .count()
                >= 2,
            "{diagnostics:#?}"
        );
        assert!(parts.module.types.is_empty(), "{:#?}", parts.module.types);
        assert!(
            parts.module.functions.is_empty(),
            "{:#?}",
            parts.module.functions
        );
    }
}
