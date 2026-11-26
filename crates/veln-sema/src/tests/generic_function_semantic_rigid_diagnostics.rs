use super::*;

fn analyze(source_text: &str) -> (SurfaceModule, Vec<Diagnostic>) {
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    (module, diagnostics)
}

#[test]
fn distinct_rigid_call_argument_mismatch_identifies_both_binder_owners() {
    let (_, diagnostics) = analyze(concat!(
        "fn second<T>(value: T) -> T\n",
        "  value\n",
        "end\n",
        "fn passes_to_second<T>(value: T) -> ()\n",
        "  second(value)\n",
        "  ()\n",
        "end\n",
    ));

    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "type.mismatch" && diagnostic.message == "expected `T`, but found `T`"
        })
        .expect("distinct rigid argument mismatch");
    assert_eq!(
        mismatch
            .span
            .as_ref()
            .map(|span| (span.start.line, span.start.column)),
        Some((5, 10))
    );
    assert_eq!(mismatch.related.len(), 2, "{mismatch:#?}");
    let expected = mismatch.related[0].to_json();
    assert!(
        expected.contains("\"kind\":\"expected_rigid_type_parameter\"")
            && expected.contains(
                "\"message\":\"Expected type parameter `T` is declared by function `second` here.\""
            )
            && expected.contains("\"line\":1")
            && expected.contains("\"column\":11"),
        "{mismatch:#?}"
    );
    let actual = mismatch.related[1].to_json();
    assert!(
        actual.contains("\"kind\":\"actual_rigid_type_parameter\"")
            && actual.contains(
                "\"message\":\"Actual type parameter `T` is declared by function `passes_to_second` here.\""
            )
            && actual.contains("\"line\":4")
            && actual.contains("\"column\":21"),
        "{mismatch:#?}"
    );
}

#[test]
fn distinct_rigid_provenance_work_is_bounded_by_binders_and_identity_probes() {
    fn source_with_mismatched_pairs(pair_count: usize) -> String {
        let mut source = String::new();
        for index in 0..pair_count {
            source.push_str(&format!(
                "fn expected{index}<T>(value: T) -> T\n  value\nend\n"
            ));
            source.push_str(&format!(
                "fn actual{index}<T>(value: T) -> ()\n  expected{index}(value)\n  ()\nend\n"
            ));
        }
        source
    }

    fn provenance_work(pair_count: usize) -> ((usize, usize), usize) {
        let source = SourceFile::new("main.veln", source_with_mismatched_pairs(pair_count));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::types::reset_function_type_parameter_provenance_work();
        let diagnostics = analyze_surface_module(&module);
        let mismatch_count = diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "type.mismatch"
                    && diagnostic.message == "expected `T`, but found `T`"
            })
            .count();
        (
            crate::types::take_function_type_parameter_provenance_work(),
            mismatch_count,
        )
    }

    let (smaller_work, smaller_mismatches) = provenance_work(16);
    let (larger_work, larger_mismatches) = provenance_work(32);
    assert_eq!(smaller_mismatches, 16);
    assert_eq!(larger_mismatches, 32);
    assert!(smaller_work.0 >= 32, "{smaller_work:?}");
    assert!(larger_work.0 >= 64, "{larger_work:?}");
    assert!(smaller_work.0 <= 32 * 8, "{smaller_work:?}");
    assert!(larger_work.0 <= 64 * 8, "{larger_work:?}");
    assert!(
        larger_work.0 <= smaller_work.0 * 2,
        "{smaller_work:?} {larger_work:?}"
    );
    assert_eq!(smaller_work.1, smaller_mismatches * 2);
    assert_eq!(larger_work.1, larger_mismatches * 2);
}

#[test]
fn distinct_rigid_mismatch_through_alias_uses_original_binder_owner() {
    let (_, diagnostics) = analyze(concat!(
        "pub fn second<T>(value: T) -> T\n",
        "  value\n",
        "end\n",
        "pub fn second_alias = second\n",
        "fn passes_to_alias<T>(value: T) -> ()\n",
        "  second_alias(value)\n",
        "  ()\n",
        "end\n",
    ));

    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "type.mismatch" && diagnostic.message == "expected `T`, but found `T`"
        })
        .expect("distinct rigid argument mismatch through alias");
    assert_eq!(mismatch.related.len(), 2, "{mismatch:#?}");
    let expected = mismatch.related[0].to_json();
    assert!(
        expected.contains(
            "\"message\":\"Expected type parameter `T` is declared by function `second` here.\""
        ) && expected.contains("\"line\":1")
            && expected.contains("\"column\":15"),
        "{mismatch:#?}"
    );
    let actual = mismatch.related[1].to_json();
    assert!(
        actual.contains(
            "\"message\":\"Actual type parameter `T` is declared by function `passes_to_alias` here.\""
        ) && actual.contains("\"line\":5")
            && actual.contains("\"column\":20"),
        "{mismatch:#?}"
    );
}
