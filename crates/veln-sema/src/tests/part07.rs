use super::*;

#[test]
fn marks_negated_conjunctive_require_as_disjunctive_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require not (max <= 0 and max > 10)\n",
            "  _value satisfy candidate => candidate > 0 or candidate <= 10\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
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
fn ignores_false_branch_from_negated_true_conjunctive_require_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require not (true and max <= 0)\n",
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
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_paired_inclusive_require_bounds_as_equality_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max <= 10\n",
            "  require max >= 10\n",
            "  _value satisfy candidate => candidate == 10\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_conjoined_inclusive_require_bounds_as_equality_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max <= 10 and max >= 10\n",
            "  _value satisfy candidate => candidate == 10\n",
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
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_equal_require_alias_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int, other: Int) -> Int\n",
            "  require max == fallback\n",
            "  require fallback > 0\n",
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
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_aliased_strict_require_as_disequality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int, other: Int) -> Int\n",
            "  require max == fallback\n",
            "  require fallback > 0\n",
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
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_aliased_inclusive_bounds_as_equality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int, other: Int) -> Int\n",
            "  require max == fallback\n",
            "  require fallback <= 10\n",
            "  require max >= 10\n",
            "  _value satisfy candidate => candidate == 10\n",
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
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_transitive_order_require_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(low: Int, mid: Int, max: Int, fallback: Int) -> Int\n",
            "  require low < mid\n",
            "  require mid <= max\n",
            "  _value satisfy candidate => candidate > low\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_disjunctive_common_order_require_as_transitive_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(low: Int, mid: Int, max: Int, fallback: Int) -> Int\n",
            "  require low < mid or low == mid\n",
            "  require mid < max\n",
            "  _value satisfy candidate => candidate > low\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_disjunctive_require_as_disjunctive_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0 or max == 0\n",
            "  _value satisfy candidate => candidate > 0 or candidate == 0\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_disjunctive_require_with_weaker_satisfy_branch_as_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0 or max == 0\n",
            "  _value satisfy candidate => candidate >= 0 or candidate == 0\n",
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
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_branch_local_aliases_as_disjunctive_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Int, low: Int, high: Int, fallback: Int) -> Int\n",
            "  require (value == low and low < 0) or (value == high and high > 0)\n",
            "  _value satisfy candidate => candidate < 0 or candidate > 0\n",
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
    assert!(
        details.contains(concat!(
            "{\"candidate_id\":\"symbol-4\",\"name\":\"value\",",
            "\"type\":\"Int\",\"rank\":4,\"reason\":\"satisfy_require_match\",",
            "\"application_policy\":\"safe_repair_candidate\","
        )),
        "{details}"
    );
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        1
    );
}

#[test]
fn marks_disjunctive_equality_require_as_disequality_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max == 1 or max == 2\n",
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
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_numeric_literal_expression_equality_as_disequality_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max == 1 + 1\n",
            "  _value satisfy candidate => candidate != 3\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_boolean_literal_equality_require_as_disequality_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(flag: Bool, fallback: Bool) -> Bool\n",
            "  require flag == true\n",
            "  _value satisfy candidate => candidate != false\n",
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
        "\"type\":\"Bool\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"flag\",",
        "\"type\":\"Bool\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"flag\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_string_literal_equality_require_as_disequality_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(name: String, fallback: String) -> String\n",
            "  require name == \"ready\"\n",
            "  _value satisfy candidate => candidate != \"pending\"\n",
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
        "\"type\":\"String\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"name\",",
        "\"type\":\"String\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"name\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_aliased_literal_equality_require_as_disequality_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int, other: Int) -> Int\n",
            "  require max == fallback\n",
            "  require fallback == 1\n",
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
