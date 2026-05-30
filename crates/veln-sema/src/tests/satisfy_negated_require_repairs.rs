use super::*;

#[test]
fn normalizes_double_negated_satisfy_comparison_for_direct_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  _value satisfy candidate => not (not (candidate == max))\n",
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
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_equality_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn normalizes_double_negated_require_comparison_for_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require not (not (max > 0))\n",
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
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_negated_less_than_satisfy_as_inclusive_require_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max >= 0\n",
            "  _value satisfy candidate => not (candidate < 0)\n",
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
fn marks_negated_inclusive_satisfy_as_strict_require_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0\n",
            "  _value satisfy candidate => not (candidate <= 0)\n",
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
fn marks_negated_less_than_require_as_inclusive_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require not (max < 0)\n",
            "  _value satisfy candidate => candidate >= 0\n",
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
fn marks_negated_inclusive_require_as_strict_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require not (max <= 0)\n",
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
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_parenthesized_require_discharged_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require (0 < max)\n",
            "  _value satisfy candidate => (candidate > 0)\n",
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
fn marks_parenthesized_equality_alias_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int, fallback: Int) -> Int\n",
            "  require ((fallback)) == ((limit))\n",
            "  _value satisfy candidate => candidate == limit\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_parenthesized_conjunction_require_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require (max > 0 and max <= 10)\n",
            "  _value satisfy candidate => (candidate > 0 and candidate <= 10)\n",
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
fn ignores_true_conjuncts_in_require_discharged_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0\n",
            "  _value satisfy candidate => candidate > 0 and true\n",
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
fn ignores_static_true_conjuncts_in_require_discharged_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0\n",
            "  _value satisfy candidate => candidate > 0 and candidate == candidate\n",
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
fn marks_disjunctive_require_discharged_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0\n",
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
fn ignores_false_disjuncts_in_require_discharged_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require false or max > 0\n",
            "  _value satisfy candidate => false or candidate > 0\n",
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
fn marks_disjunctive_require_as_satisfy_repair_evidence_when_all_branches_discharge() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0 or max == 0\n",
            "  _value satisfy candidate => candidate >= 0\n",
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
fn marks_nested_disjunctive_require_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require (max > 0 or max == 0) and max <= 10\n",
            "  _value satisfy candidate => candidate >= 0 and candidate <= 10\n",
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
fn marks_nested_disjunctive_satisfy_as_require_discharged_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0 and max <= 10\n",
            "  _value satisfy candidate => (candidate > 0 or candidate == 0) and candidate <= 10\n",
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
fn marks_negated_disjunctive_require_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require not (max < 0 or max > 10)\n",
            "  _value satisfy candidate => candidate >= 0 and candidate <= 10\n",
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
fn ignores_false_branches_in_negated_disjunctive_direct_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(fallback: Int) -> Int\n",
            "  _value satisfy candidate => not (false or candidate != fallback)\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_equality_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn ignores_false_branches_in_negated_disjunctive_require_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require not (false or max <= 0)\n",
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
fn marks_negated_conjunctive_satisfy_as_require_discharged_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0\n",
            "  _value satisfy candidate => not (candidate <= 0 and candidate > -10)\n",
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
