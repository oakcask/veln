use super::*;

#[test]
fn schema_alias_target_import_resolution_is_indexed_once() {
    for count in [100usize, 200, 400] {
        let mut facade_text = String::from("mod facade\n");
        for index in 0..count.saturating_sub(1) {
            facade_text.push_str(&format!("use unused{index}\n"));
        }
        facade_text.push_str("use core\n\n");
        let mut core_text = String::from("mod core\n\n");
        for index in 0..count {
            facade_text.push_str(&format!("pub schema Alias{index} = core::Packet{index}\n"));
            core_text.push_str(&format!("pub schema Packet{index}\n  value: Int\nend\n\n"));
        }
        let facade_source = SourceFile::new("facade.veln", &facade_text);
        let facade = parse(&facade_source);
        assert!(facade.diagnostics.is_empty(), "{:#?}", facade.diagnostics);
        let core_source = SourceFile::new("core.veln", &core_text);
        let core = parse(&core_source);
        assert!(core.diagnostics.is_empty(), "{:#?}", core.diagnostics);
        let mut module = lower_surface_ast(&facade.tree);
        module.schemas.extend(lower_surface_ast(&core.tree).schemas);

        reset_schema_alias_target_import_work();
        assert_eq!(resolved_schema_aliases(&module).len(), count);
        assert_eq!(
            schema_alias_target_import_work(),
            (count, count),
            "imports must be indexed once and each qualified alias must use one route lookup"
        );
    }
}

#[test]
fn schema_alias_chain_resolution_work_is_linear() {
    for count in [64usize, 128, 256] {
        let mut source = String::from("mod facade\n\npub schema Packet\n  value: Int\nend\n\n");
        source.push_str("pub schema Alias0 = Packet\n");
        for index in 1..count {
            source.push_str(&format!("pub schema Alias{index} = Alias{}\n", index - 1));
        }
        let parsed = parse(&SourceFile::new("facade.veln", &source));
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        reset_schema_alias_chain_resolution_work();
        assert_eq!(resolved_schema_alias_chains(&module).len(), count);
        assert!(
            schema_alias_chain_resolution_work() <= count * 2,
            "chain resolution must visit each alias a bounded number of times"
        );
    }
}

#[test]
fn schema_alias_chain_shared_suffix_and_disconnected_cycle_stay_bounded() {
    for count in [64usize, 128, 256] {
        let mut source = String::from("mod facade\n\npub schema Packet\n  value: Int\nend\n\n");
        source.push_str("pub schema Shared0 = Packet\n");
        for index in 1..count {
            source.push_str(&format!("pub schema Shared{index} = Shared{}\n", index - 1));
        }
        for index in 0..count {
            source.push_str(&format!("pub schema Left{index} = Shared{}\n", count - 1));
            source.push_str(&format!("pub schema Right{index} = Shared{}\n", count - 1));
        }
        source.push_str("pub schema CycleA = CycleB\npub schema CycleB = CycleA\n");

        let parsed = parse(&SourceFile::new("facade.veln", &source));
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        reset_schema_alias_chain_resolution_work();
        let resolved = resolved_schema_alias_chains(&module);
        assert_eq!(resolved.len(), count * 3);
        let work = schema_alias_chain_resolution_work();
        assert!(
            work <= count * 8 + 8,
            "shared suffixes and disconnected cycles must not cause repeated walks: work={work} count={count}"
        );
    }
}

#[test]
fn format_neutral_schema_missing_qualifier_import_work_is_linear() {
    fn work(count: usize) -> usize {
        let mut source = String::from("mod app\n");
        for index in 0..count {
            source.push_str(&format!("use dependency{index} as alias{index}\n"));
        }
        source.push_str("\nschema Packet\n  payload: missing::Payload\nend\n");
        let parsed = parse(&SourceFile::new("app.veln", &source));
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        let schema = &module.schemas[0];

        crate::name_recovery::reset_import_classification_work();
        assert!(crate::types::schema_encode_value_type(&module, schema).is_none());
        crate::name_recovery::import_classification_work()
    }

    let small = work(128);
    let large = work(256);
    eprintln!("format-neutral missing-qualifier import work: 128={small} 256={large}");
    assert!(
        large <= small * 2 + 16,
        "format-neutral schema qualifier misses must remain linear: small={small} large={large}"
    );
}
