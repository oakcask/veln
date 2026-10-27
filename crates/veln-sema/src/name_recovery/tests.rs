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
