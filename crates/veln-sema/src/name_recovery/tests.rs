use super::*;

#[test]
fn invalid_explicit_aliases_are_atomically_quarantined() {
    for alias in ["_wire", "_"] {
        let source = veln_source::SourceFile::new(
            "app.veln",
            format!("mod app\nuse transport as {alias}\n"),
        );
        let parsed = veln_syntax::parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = veln_ast::lower_surface_ast(&parsed.tree);
        assert_eq!(module.uses.len(), 1, "alias={alias}");
        assert!(use_decl_is_quarantined(&module, &module.uses[0]));
        assert!(normal_use_decls(&module).is_empty(), "alias={alias}");
    }
}

#[test]
fn recovered_target_identity_preserves_the_rewritten_package_boundary() {
    let app_source = veln_source::SourceFile::new(
        "app.veln",
        "use Http as wire from \"pkg\"\nfn main() -> Int\n  wire::entry()\nend\n",
    );
    let app_parsed = veln_syntax::parse(&app_source);
    let mut module = veln_ast::lower_surface_ast_with_module_identity(
        &app_parsed.tree,
        "app".to_string(),
        app_source.span(veln_source::TextRange::new(0, 0)),
    );
    module.uses[0].name = "pkg::Http".to_string();

    let target_source =
        veln_source::SourceFile::new("http.veln", "pub fn entry() -> Int\n  1\nend\n");
    let target_parsed = veln_syntax::parse(&target_source);
    let target = veln_ast::lower_surface_ast_with_package_module_identity(
        &target_parsed.tree,
        "pkg".to_string(),
        "http".to_string(),
        target_source.span(veln_source::TextRange::new(0, 0)),
    );
    module.functions.extend(target.functions);

    let recovered = quarantined_use_decls(&module);
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].name, "pkg::http");
    assert_eq!(recovered[0].package.as_deref(), Some("pkg"));
    assert_eq!(module.uses[0].name, "pkg::Http");
}

#[test]
fn normal_import_classification_work_scales_linearly() {
    fn work(count: usize) -> usize {
        let mut text = String::from("mod app\n");
        for index in 0..count {
            text.push_str(&format!("use dependency{index} as alias{index}\n"));
        }
        let source = veln_source::SourceFile::new("app.veln", text);
        let parsed = veln_syntax::parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = veln_ast::lower_surface_ast(&parsed.tree);
        reset_import_classification_work();
        assert_eq!(normal_use_decls(&module).len(), count);
        import_classification_work()
    }

    let small = work(128);
    let large = work(256);
    assert!(
        large <= small * 2 + 8,
        "normal import classification must remain linear: small={small} large={large}"
    );
}

#[test]
fn recovery_target_materialization_scales_with_qualified_identity_bytes() {
    fn measure(depth: usize, segment_width: usize, recovery_candidate: bool) -> RecoveryTargetWork {
        let segment = "a".repeat(segment_width);
        let identity = std::iter::repeat_n(segment.as_str(), depth)
            .collect::<Vec<_>>()
            .join("::");
        let mut written = identity.clone();
        if recovery_candidate {
            let final_segment = written.rfind("::").map_or(0, |offset| offset + 2);
            written.replace_range(final_segment..final_segment + 1, "A");
        }
        let source = veln_source::SourceFile::new(
            "app.veln",
            format!("mod {identity}\nuse {written} as wire\n"),
        );
        let parsed = veln_syntax::parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = veln_ast::lower_surface_ast(&parsed.tree);

        reset_import_classification_work();
        let recovered = quarantined_use_decls(&module);
        if recovery_candidate {
            assert_eq!(recovered.len(), 1);
            assert_eq!(recovered[0].name, identity);
        } else {
            assert!(recovered.is_empty(), "{recovered:#?}");
        }
        recovery_target_work()
    }

    for (depth, width) in [(64, 1), (128, 1), (1, 128), (1, 256)] {
        assert_eq!(
            measure(depth, width, false),
            RecoveryTargetWork::default(),
            "clean imports must not build recovery target data"
        );
    }

    let by_depth = [64, 128].map(|depth| measure(depth, 1, true));
    let by_bytes = [128, 256].map(|width| measure(1, width, true));
    for measurements in [by_depth, by_bytes] {
        let [small, large] = measurements;
        assert!(small.declared_identity_bytes > 0, "{measurements:?}");
        assert!(small.candidate_bytes > 0, "{measurements:?}");
        assert!(small.corrected_bytes > 0, "{measurements:?}");
        assert!(
            large.declared_identity_bytes <= small.declared_identity_bytes * 2 + 2,
            "declared identity materialization must remain linear: {measurements:?}"
        );
        assert!(
            large.candidate_bytes <= small.candidate_bytes * 2 + 4,
            "candidate processing must remain linear: {measurements:?}"
        );
        assert!(
            large.corrected_bytes <= small.corrected_bytes * 2 + 2,
            "corrected path materialization must remain linear: {measurements:?}"
        );
    }
}

fn run_recovery_target_depth_cross_check(depth: usize) {
    let identity = std::iter::repeat_n("a", depth)
        .collect::<Vec<_>>()
        .join("::");
    let mut written = identity.clone();
    let final_segment = written.rfind("::").map_or(0, |offset| offset + 2);
    written.replace_range(final_segment..final_segment + 1, "A");
    let source = veln_source::SourceFile::new(
        "deep.veln",
        format!(
            "mod {identity}\nuse {written} as wire\npub fn entry() -> Int\n  1\nend\npub fn exposed = wire::entry\n"
        ),
    );
    let parsed = veln_syntax::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = veln_ast::lower_surface_ast(&parsed.tree);
    let diagnostics = crate::analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
#[ignore = "procedural wall-clock and RSS cross-check for recovery depth"]
fn recovery_target_depth_500_cross_check() {
    run_recovery_target_depth_cross_check(500);
}

#[test]
#[ignore = "procedural wall-clock and RSS cross-check for recovery depth"]
fn recovery_target_depth_1000_cross_check() {
    run_recovery_target_depth_cross_check(1000);
}

#[test]
#[ignore = "procedural wall-clock and RSS cross-check for recovery depth"]
fn recovery_target_depth_1500_cross_check() {
    run_recovery_target_depth_cross_check(1500);
}

#[test]
fn missing_qualifier_lookup_work_scales_linearly() {
    fn work(count: usize, schema_fallback: bool) -> usize {
        let mut text = String::from("mod app\n");
        for index in 0..count {
            text.push_str(&format!("use dependency{index} as alias{index}\n"));
        }
        let source = veln_source::SourceFile::new("app.veln", text);
        let parsed = veln_syntax::parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = veln_ast::lower_surface_ast(&parsed.tree);
        let missing = vec!["missing".to_string()];

        reset_import_classification_work();
        let resolved = if schema_fallback {
            schema_composition_imported_use_for_path(&module, &missing, Some("app"))
        } else {
            normal_imported_use_for_path(&module, &missing, Some("app"))
        };
        assert!(resolved.is_none());
        import_classification_work()
    }

    for schema_fallback in [false, true] {
        let small = work(128, schema_fallback);
        let large = work(256, schema_fallback);
        eprintln!(
            "missing-qualifier import work: fallback={schema_fallback} 128={small} 256={large}"
        );
        assert!(
            large <= small * 2 + 8,
            "missing qualifier lookup must remain linear: fallback={schema_fallback} small={small} large={large}"
        );
    }
}
