use super::qualified_use_segments::merged_modules_with_names;
use super::*;

#[test]
fn nested_expression_paths_keep_call_value_and_pattern_roles() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use helper\n",
            "fn main(input: helper::Item) -> Int\n",
            "  let callbacks = [helper::make]\n",
            "  let record = {value: helper::make()}\n",
            "  match input\n",
            "    helper::Item::Ready(value) => -helper::make() + value\n",
            "  end\n",
            "end\n",
        ),
    );
    let helper = SourceFile::new(
        "helper.veln",
        "pub type Item\n  pub Ready(Int)\nend\npub fn make() -> Int\n  1\nend\n",
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = merged_modules_with_names([("main", source), ("helper", helper)]);
    let observed = classified_project_qualified_path_segments(&module)
        .into_iter()
        .filter(|segment| {
            segment.span.file.as_str() == "main.veln"
                && segment.span.start.line >= 3
                && (segment.name == "make" || segment.name == "Ready")
        })
        .map(|segment| (segment.name, segment.role.as_str(), segment.span.start.line))
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        [
            ("make".to_string(), "value_binding", 3),
            ("make".to_string(), "function", 4),
            ("Ready".to_string(), "constructor", 6),
            ("make".to_string(), "function", 6),
        ]
    );
}

#[test]
fn nested_recovered_type_qualifiers_distinguish_calls_from_constructor_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Sample\n",
            "  Made(Int)\n",
            "  Empty\n",
            "end\n",
            "fn main() -> Int\n",
            "  let calls = [sample::Made(1)]\n",
            "  let values = {empty: sample::Empty}\n",
            "  let unapplied = [sample::Made]\n",
            "  0\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let recovered = classified_project_qualified_path_segments(&module)
        .into_iter()
        .filter(|segment| segment.name == "sample")
        .map(|segment| (segment.span.start.line, segment.role, segment.evidence))
        .collect::<Vec<_>>();
    assert_eq!(
        recovered,
        [
            (
                6,
                veln_ast::NameClass::Type,
                veln_ast::QualifiedPathSegmentEvidence::UniqueRecovery
            ),
            (
                7,
                veln_ast::NameClass::Type,
                veln_ast::QualifiedPathSegmentEvidence::UniqueRecovery
            ),
        ]
    );
}

#[test]
fn type_applied_unresolved_calls_do_not_gain_function_casing_roles() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use helper\n",
            "fn main() -> Int\n",
            "  let values = [helper::Missing<Int>()]\n",
            "  helper::Missing<Int>()\n",
            "end\n",
        ),
    );
    let helper = SourceFile::new("helper.veln", "pub fn make() -> Int\n  1\nend\n");
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = merged_modules_with_names([("main", source), ("helper", helper)]);
    let classified = classified_project_qualified_path_segments(&module);
    assert!(
        classified.iter().all(|segment| segment.name != "Missing"),
        "{classified:#?}"
    );
    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count(),
        2,
        "{diagnostics:#?}"
    );
}
