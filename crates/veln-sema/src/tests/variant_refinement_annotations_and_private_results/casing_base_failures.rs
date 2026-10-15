use super::*;

#[test]
fn nested_invalid_cased_bases_keep_independent_eligibility_failures() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "type Box<A>\n",
        "  Boxed(A)\n",
        "end\n",
        "fn main(value: Box<empty::Missing>::Boxed) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let relevant = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "name.invalid_case" | "type.variant_refinement_base"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(relevant.len(), 2, "{diagnostics:#?}");
    assert_eq!(relevant[0].id, "name.invalid_case");
    assert_eq!(relevant[1].id, "type.variant_refinement_base");
    let base_json = veln_diagnostics::diagnostic_to_json(relevant[1]).to_json();
    assert!(base_json.contains("\"written_type\":\"empty\""));
    assert!(base_json.contains("\"reason\":\"not_adt\""));
    assert!(base_json.contains("\"resolved_identity\":\"Empty\""));
}

#[test]
fn nested_casing_precedes_outer_base_eligibility_in_one_alternative() {
    let source = SourceFile::new(
        "review_ordering.veln",
        concat!(
            "type Empty<A>\n",
            "end\n",
            "type State\n",
            "  Ready\n",
            "end\n",
            "fn f(value: Empty<review_ordering::state>::Missing) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "type.variant_refinement_base"],
        "{diagnostics:#?}"
    );
}

#[test]
fn nested_casing_and_base_failures_follow_written_alternative_order() {
    let source = SourceFile::new(
        "review_ordering.veln",
        concat!(
            "type Empty<A>\n",
            "end\n",
            "type State\n",
            "  Ready\n",
            "end\n",
            "fn base_then_casing(value: Empty<Int>::Missing | Empty<review_ordering::state>::Missing) -> ()\n",
            "  ()\n",
            "end\n",
            "fn casing_then_base(value: Empty<review_ordering::state>::Missing | Empty<Int>::Missing) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variant_refinement_base",
            "name.invalid_case",
            "type.variant_refinement_base",
            "name.invalid_case",
            "type.variant_refinement_base",
            "type.variant_refinement_base",
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn unrelated_diagnostics_do_not_block_written_alternative_order() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Empty<A>\n",
            "end\n",
            "type State\n",
            "  Ready\n",
            "end\n",
            "pub fn before()\n",
            "  ()\n",
            "end\n",
            "fn f(value: Empty<Int>::Missing | Empty<main::state>::Missing) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variant_refinement_base",
            "name.invalid_case",
            "type.public_signature_missing",
            "type.variant_refinement_base",
        ],
        "{diagnostics:#?}"
    );
    assert!(diagnostics[0].message.contains("Empty<Int>"));
    assert_eq!(
        diagnostics[0].span.as_ref().map(|span| (
            span.start.line,
            span.start.column,
            span.end.column
        )),
        Some((9, 13, 18))
    );
    assert_eq!(
        diagnostics[1].span.as_ref().map(|span| (
            span.start.line,
            span.start.column,
            span.end.column
        )),
        Some((9, 47, 52))
    );
    assert_eq!(
        diagnostics[3].span.as_ref().map(|span| (
            span.start.line,
            span.start.column,
            span.end.column
        )),
        Some((9, 35, 40))
    );
}

#[test]
fn invalid_cased_bases_keep_independent_generic_arity_failures() {
    let diagnostics = diagnostics_for(concat!(
        "type GenericEmpty<A>\n",
        "end\n",
        "fn missing(value: genericEmpty::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn extra(value: genericEmpty<Int, String>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let casing = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .count();
    assert_eq!(casing, 2, "{diagnostics:#?}");
    let arity_messages = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.invalid_annotation")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(arity_messages.len(), 2, "{diagnostics:#?}");
    assert!(
        arity_messages
            .iter()
            .any(|message| message.contains("`GenericEmpty` expects 1 type argument(s), found 0")),
        "{diagnostics:#?}"
    );
    assert!(
        arity_messages
            .iter()
            .any(|message| message.contains("`genericEmpty` expects 1 type argument(s), found 2")),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.variant_refinement_base"),
        "{diagnostics:#?}"
    );
}

#[test]
fn source_less_arity_survives_exact_and_casing_recovered_refinements() {
    let diagnostics = diagnostics_for(concat!(
        "fn recovered(value: int<Int>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn exact(value: Int<Int>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "type.invalid_annotation",
            "type.invalid_annotation",
        ],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics[1]
            .message
            .contains("`int` expects 0 type argument(s), found 1"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics[2]
            .message
            .contains("`Int` expects 0 type argument(s), found 1"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[0].span.as_ref().map(|span| (
            span.start.line,
            span.start.column,
            span.end.column
        )),
        Some((1, 21, 24))
    );
    assert_eq!(
        diagnostics[1].span.as_ref().map(|span| (
            span.start.line,
            span.start.column,
            span.end.column
        )),
        Some((1, 14, 38))
    );
    assert_eq!(
        diagnostics[2].span.as_ref().map(|span| (
            span.start.line,
            span.start.column,
            span.end.column
        )),
        Some((4, 10, 34))
    );
}

#[test]
fn generic_invalid_cased_base_with_correct_arity_keeps_identity_and_eligibility_failure() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type GenericEmpty<A>\n",
            "end\n",
            "fn bad(invalid: genericEmpty<Int>::Missing, sibling: String) -> Int\n",
            "  0\n",
            "end\n",
            "fn caller() -> ()\n",
            "  let result: String = bad(1, 2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let signature = environment.function("bad").unwrap();
    assert_eq!(signature.params, [Type::Unknown, Type::Unknown]);
    assert_eq!(signature.return_type, Type::Unknown);

    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "type.variant_refinement_base"],
        "{diagnostics:#?}"
    );
    let base = veln_diagnostics::diagnostic_to_json(&diagnostics[1]).to_json();
    assert!(
        base.contains(
            "\"details\":{\"written_type\":\"genericEmpty<Int>\",\"reason\":\"not_adt\"}"
        ),
        "{base}"
    );
    assert!(
        base.contains("\"resolved_identity\":\"GenericEmpty\""),
        "{base}"
    );
    assert!(!base.contains("type.invalid_annotation"), "{base}");
}
