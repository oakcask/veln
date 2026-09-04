use super::*;

#[test]
fn compares_prefixed_integer_bounds_by_value_for_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int, fallback: Int) -> Int\n",
            "  require limit <= 0x0A\n",
            "  _value satisfy candidate => candidate < 0b10100\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\""
    )));
}

#[test]
fn marks_aliased_strict_integer_lower_bound_as_adjacent_inclusive_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int, other: Int) -> Int\n",
            "  require max == fallback\n",
            "  require fallback > 0\n",
            "  _value satisfy candidate => candidate >= 1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"other\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-3\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":3,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn marks_stronger_literal_upper_bound_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(min: Int, fallback: Int) -> Int\n",
            "  require min <= 10\n",
            "  _value satisfy candidate => candidate < 20\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"min\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_strict_integer_upper_bound_as_adjacent_inclusive_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(min: Int, fallback: Int) -> Int\n",
            "  require min < 10\n",
            "  _value satisfy candidate => candidate <= 9\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"min\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_stronger_float_literal_lower_bound_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(ratio: Float, fallback: Float) -> Float\n",
            "  require ratio >= 10.5\n",
            "  _value satisfy candidate => candidate > 0.5\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",",
        "\"type\":\"Float\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"ratio\",",
        "\"type\":\"Float\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn leaves_equal_inclusive_float_literal_bound_as_manual_for_strict_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(ratio: Float, fallback: Float) -> Float\n",
            "  require ratio >= 10.5\n",
            "  _value satisfy candidate => candidate > 10.5\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"ratio\",",
        "\"type\":\"Float\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_stronger_negative_float_literal_lower_bound_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(ratio: Float, fallback: Float) -> Float\n",
            "  require ratio >= -0.5\n",
            "  _value satisfy candidate => candidate > -1.5\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"ratio\",",
        "\"type\":\"Float\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn leaves_equal_inclusive_literal_bound_as_manual_for_strict_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max >= 10\n",
            "  _value satisfy candidate => candidate > 10\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_literal_equality_as_lower_bound_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max == 10\n",
            "  _value satisfy candidate => candidate > 0\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_literal_equality_as_upper_bound_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(min: Int, fallback: Int) -> Int\n",
            "  require min == 10\n",
            "  _value satisfy candidate => candidate < 20\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"min\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_lower_literal_bound_as_disequality_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 10\n",
            "  _value satisfy candidate => candidate != 0\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_literal_arithmetic_bound_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 1 + 1\n",
            "  _value satisfy candidate => candidate > 2\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_hexadecimal_literal_subtraction_bound_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0xA - 1\n",
            "  _value satisfy candidate => candidate > 9\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_nested_literal_arithmetic_bound_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Float, fallback: Float) -> Float\n",
            "  require max > (1 + 3) * 2 / 4\n",
            "  _value satisfy candidate => candidate > 1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Float\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_subtracted_literal_arithmetic_bound_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(min: Int, fallback: Int) -> Int\n",
            "  require min < -(3 - 1)\n",
            "  _value satisfy candidate => candidate < -2\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"min\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_inexact_literal_arithmetic_bound_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 1 / 3\n",
            "  _value satisfy candidate => candidate > 0.3\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_inclusive_integer_lower_bound_as_adjacent_strict_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max >= 10\n",
            "  _value satisfy candidate => candidate > 9\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_inclusive_integer_upper_bound_as_adjacent_strict_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(min: Int, fallback: Int) -> Int\n",
            "  require min <= 10\n",
            "  _value satisfy candidate => candidate < 11\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"min\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn leaves_non_adjacent_inclusive_integer_lower_bound_as_manual_for_strict_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max >= 10\n",
            "  _value satisfy candidate => candidate > 10\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn leaves_non_adjacent_inclusive_integer_upper_bound_as_manual_for_strict_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(min: Int, fallback: Int) -> Int\n",
            "  require min <= 10\n",
            "  _value satisfy candidate => candidate < 10\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"min\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn keeps_safe_satisfy_candidate_beyond_top_five_ranked_bindings() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(target: Int, a: Int, b: Int, c: Int, d: Int, e: Int) -> Int\n",
            "  require target > 0\n",
            "  _value satisfy candidate => candidate > 0\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-6\",\"name\":\"target\",",
        "\"type\":\"Int\",\"rank\":6,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn keeps_require_proven_candidate_with_many_unrelated_bindings() {
    let unrelated = (0..64)
        .map(|index| format!("unrelated_{index}: Int"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn main(target: Int, {unrelated}) -> Int\n\
             require target > 0\n\
             _value satisfy candidate => candidate > 0\n\
             end\n"
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"name\":\"target\""));
    assert!(details.contains("\"reason\":\"satisfy_require_match\""));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        1
    );
}

#[test]
fn leaves_equal_inclusive_literal_bound_as_manual_for_disequality_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max >= 10\n",
            "  _value satisfy candidate => candidate != 10\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_upper_float_literal_bound_as_disequality_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(ratio: Float, fallback: Float) -> Float\n",
            "  require ratio <= -0.5\n",
            "  _value satisfy candidate => candidate != 0.5\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"ratio\",",
        "\"type\":\"Float\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}
