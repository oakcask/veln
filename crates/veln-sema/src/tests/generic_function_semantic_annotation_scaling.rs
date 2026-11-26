use super::*;
use std::collections::HashSet;

fn wide_generic_annotation_source(
    binder_count: usize,
    annotation_count: usize,
    annotation: &str,
) -> String {
    let binders = (0..binder_count)
        .map(|index| format!("T{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut source = format!("fn wide<{binders}>(value: T0) -> T0\n");
    for index in 0..annotation_count {
        source.push_str(&format!("  let value{index}: {annotation} = value\n"));
    }
    source.push_str("  value\nend\n");
    source
}

#[test]
fn body_annotations_reuse_one_declaration_type_parameter_context() {
    fn context_work(
        binder_count: usize,
        annotation_count: usize,
        annotation: &str,
    ) -> ((usize, usize), Vec<Diagnostic>) {
        let source = SourceFile::new(
            "main.veln",
            wide_generic_annotation_source(binder_count, annotation_count, annotation),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::types::reset_type_parameter_context_work();
        let diagnostics = analyze_surface_module(&module);
        (
            crate::types::take_type_parameter_context_work(),
            diagnostics,
        )
    }

    let (one_annotation, one_diagnostics) = context_work(64, 1, "T0");
    let (many_annotations, many_diagnostics) = context_work(64, 128, "T0");
    assert!(one_diagnostics.is_empty(), "{one_diagnostics:#?}");
    assert!(many_diagnostics.is_empty(), "{many_diagnostics:#?}");
    assert!(
        one_annotation.0 > 0,
        "the metric must observe context builds"
    );
    assert!(
        one_annotation.1 >= 64,
        "the metric must observe declaration binder indexing"
    );
    assert_eq!(
        many_annotations.1, one_annotation.1,
        "additional annotations must not re-index declaration binders"
    );

    let (invalid_annotations, invalid_diagnostics) = context_work(64, 128, "Int<Int>::Some");
    assert!(
        invalid_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.invalid_annotation"),
        "expected invalid annotation diagnostics, found ids: {:?}",
        invalid_diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        invalid_annotations.1, one_annotation.1,
        "failed validation must not re-index declaration binders"
    );
}

#[test]
fn annotation_named_symbol_lookup_work_is_linear_in_symbols_and_occurrences() {
    fn measurement(size: usize) -> ((usize, usize), Vec<Diagnostic>) {
        let mut source = String::new();
        for index in 0..size {
            source.push_str(&format!("pub type Visible{index} = Int\n"));
        }
        source.push_str("fn inspect(value: Int) -> ()\n");
        for index in 0..size {
            source.push_str(&format!(
                "  let resolved{index}: Visible{} = value\n",
                size - 1
            ));
            source.push_str(&format!("  let missing{index}: Missing = value\n"));
        }
        source.push_str("  ()\nend\n");

        let source = SourceFile::new("lookup.veln", source);
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::types::reset_annotation_named_symbol_index_work();
        let diagnostics = analyze_surface_module(&module);
        (
            crate::types::take_annotation_named_symbol_index_work(),
            diagnostics,
        )
    }

    let mut totals = Vec::new();
    for size in [32, 64, 128] {
        let ((index_entries, lookups), diagnostics) = measurement(size);
        let missing = diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved type `Missing`"
            })
            .collect::<Vec<_>>();
        assert_eq!(missing.len(), size, "{diagnostics:#?}");
        assert_eq!(
            missing
                .iter()
                .filter_map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
                .collect::<HashSet<_>>()
                .len(),
            size,
            "each missing occurrence must retain its own span"
        );
        assert!(index_entries >= size, "index build was not measured");
        assert!(
            lookups >= size * 2,
            "resolved and missing lookups were not measured"
        );
        let total = index_entries + lookups;
        assert!(
            total <= size * 8 + 32,
            "size={size} index_entries={index_entries} lookups={lookups}"
        );
        totals.push(total);
    }
    assert!(
        totals[2] <= totals[1] * 2 + 32,
        "doubling visible symbols and occurrences must remain linear: {totals:?}"
    );
}

fn schema_lookup_measurement(
    size: usize,
) -> (crate::types::SchemaSymbolIndexWork, Vec<Diagnostic>) {
    let mut source = String::new();
    for index in 0..size {
        source.push_str(&format!(
            "pub schema Packet{index}\n  value: Int\nend\npub schema Alias{index} = Packet{index}\n"
        ));
    }
    source.push_str("fn inspect(value: Int) -> ()\n");
    for index in 0..size {
        source.push_str(&format!("  let missing{index}: Missing = value\n"));
    }
    source.push_str("  ()\nend\n");

    let source = SourceFile::new("schema_lookup.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    crate::types::reset_schema_symbol_index_work();
    let diagnostics = analyze_surface_module(&module);
    (crate::types::take_schema_symbol_index_work(), diagnostics)
}

fn assert_schema_lookup_measurement(
    size: usize,
    work: &crate::types::SchemaSymbolIndexWork,
    diagnostics: &[Diagnostic],
) {
    let missing = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `Missing`"
        })
        .collect::<Vec<_>>();
    assert_eq!(missing.len(), size, "{diagnostics:#?}");
    let missing_lines = missing
        .iter()
        .filter_map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
        .collect::<HashSet<_>>();
    assert_eq!(
        missing_lines.len(),
        size,
        "each missing annotation must retain its own span"
    );
    assert!(
        work.schema_entries >= size,
        "schema index build was not measured"
    );
    assert!(
        work.alias_entries >= size,
        "alias index build was not measured"
    );
    assert!(
        work.schema_lookups >= size,
        "schema probes were not measured"
    );
    assert!(work.alias_lookups >= size, "alias probes were not measured");
}

fn measured_schema_lookup_total(size: usize) -> usize {
    let (work, diagnostics) = schema_lookup_measurement(size);
    assert_schema_lookup_measurement(size, &work, &diagnostics);
    work.total()
}

#[test]
fn annotation_schema_symbol_lookup_work_is_linear_in_symbols_and_occurrences() {
    let totals = [32, 64, 128].map(measured_schema_lookup_total);
    assert!(
        totals[1] <= totals[0] * 2 + 32,
        "doubling schemas, aliases, and annotations must remain linear: {totals:?}"
    );
    assert!(
        totals[2] <= totals[1] * 2 + 32,
        "doubling schemas, aliases, and annotations must remain linear: {totals:?}"
    );
    eprintln!("schema annotation index work for sizes 32/64/128: {totals:?}");
}
