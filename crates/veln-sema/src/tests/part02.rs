use super::*;

#[test]
fn marks_satisfy_equality_hole_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate == fallback\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"satisfy_candidate_binding\":\"candidate\""));
    assert!(details.contains("\"satisfy_predicate\":\"candidate == fallback\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_equality_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains("\"satisfy_status\":\"blocked_until_discharged\""));
}

#[test]
fn marks_negated_satisfy_equality_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => not (candidate != fallback)\n",
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
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn marks_negated_disjunctive_direct_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => not (candidate != fallback or candidate < fallback)\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_reflexive_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn marks_negated_conjunctive_direct_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => not (candidate != fallback and candidate < fallback)\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_reflexive_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn ignores_false_branch_from_negated_true_conjunct_in_direct_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => not (true and candidate != fallback)\n",
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
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn marks_negated_false_conjunctive_satisfy_as_tautological_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => not (false and candidate != fallback)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert_eq!(
        details
            .matches("\"application_policy\":\"safe_repair_candidate\"")
            .count(),
        2
    );
    assert_eq!(
        details
            .matches(concat!(
                "\"reason\":\"satisfy_tautology\",",
                "\"application_policy\":\"safe_repair_candidate\""
            ))
            .count(),
        2
    );
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn rejects_negated_disjunctive_direct_satisfy_candidates_with_different_bindings() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => not (candidate != fallback or candidate < limit)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(!details.contains("\"application_policy\":\"safe_repair_candidate\""));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_satisfy_reflexive_comparison_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate >= fallback and candidate <= fallback\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_reflexive_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn marks_same_shape_expression_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate + 1 == fallback + 1\n",
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
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn marks_same_shape_expression_satisfy_comparison_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate + 1 <= fallback + 1\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_reflexive_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_parenthesized_same_shape_expression_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => (candidate) + 1 == fallback + 1\n",
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
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn marks_binding_substituted_static_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}, fallback: {ready: Bool, paid: Bool}) -> {ready: Bool, paid: Bool}\n",
            "  _value satisfy candidate => not ((candidate.ready and order.paid) and not (order.ready and order.paid))\n",
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
        "\"type\":\"{ready: Bool, paid: Bool}\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"order\",",
        "\"type\":\"{ready: Bool, paid: Bool}\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"order\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn safe_assignable_satisfy_candidates_report_discharge_reason() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => candidate.ready == order.ready\n",
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
        "{\"candidate_id\":\"symbol-1\",\"name\":\"order\",",
        "\"type\":\"{ready: Bool, paid: Bool}\",\"rank\":1,",
        "\"reason\":\"satisfy_equality_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_negated_strict_satisfy_comparison_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => not (candidate < fallback)\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_reflexive_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_disjunctive_direct_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate == fallback or candidate >= fallback\n",
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
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn marks_each_top_level_disjunctive_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int, fallback: Int, spare: Int) -> Int\n",
            "  _value satisfy candidate => candidate == fallback or candidate == spare\n",
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
        "{\"candidate_id\":\"symbol-1\",\"name\":\"spare\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_equality_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_equality_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-3\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":3,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn preserves_reasons_for_mixed_top_level_disjunctive_satisfy_candidates() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(fallback: Int, spare: Int) -> Int\n",
            "  _value satisfy candidate => candidate >= fallback or candidate == spare\n",
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
        "{\"candidate_id\":\"symbol-1\",\"name\":\"spare\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_equality_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_reflexive_match\",",
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
fn marks_intersecting_disjunctive_satisfy_conjuncts_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, spare: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => ",
            "(candidate == fallback or candidate == max) and ",
            "(candidate == fallback or candidate == spare)\n",
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
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"spare\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-3\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":3,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}
