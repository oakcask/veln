use super::*;

#[test]
fn marks_intersecting_disjunctive_satisfy_comparisons_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, spare: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => ",
            "(candidate >= fallback or candidate == max) and ",
            "(candidate <= fallback or candidate == spare)\n",
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
fn marks_shared_nested_disjunctive_satisfy_candidates_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(fallback: Int, spare: Int) -> Int\n",
            "  _value satisfy candidate => ",
            "(candidate == fallback or candidate == spare) and ",
            "(candidate >= fallback or candidate >= spare)\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_reflexive_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_reflexive_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn preserves_reasons_for_shared_disjunctive_satisfy_candidates() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(fallback: Int, spare: Int) -> Int\n",
            "  _value satisfy candidate => ",
            "(candidate >= fallback or candidate == spare) and ",
            "(candidate <= fallback or candidate == spare)\n",
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
fn deduplicates_shared_top_level_disjunctive_satisfy_candidate() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => ",
            "(candidate == fallback and candidate >= fallback) or ",
            "(candidate == fallback and candidate <= fallback)\n",
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
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        1
    );
}

#[test]
fn rejects_empty_intersection_for_disjunctive_satisfy_conjuncts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, spare: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => ",
            "(candidate == fallback or candidate == max) and ",
            "candidate == spare\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"spare\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-3\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":3,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert_eq!(
        details
            .matches("\"application_policy\":\"safe_repair_candidate\"")
            .count(),
        0
    );
}

#[test]
fn ignores_false_disjuncts_in_direct_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => false or candidate == fallback\n",
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
fn ignores_static_false_disjuncts_in_direct_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(flag: Bool, fallback: Int) -> Int\n",
            "  _value satisfy candidate => (flag and not flag) or candidate == fallback\n",
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
fn ignores_trailing_false_disjuncts_in_direct_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate == fallback or false\n",
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
fn ignores_nested_false_disjuncts_in_direct_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate == fallback and (false or candidate >= fallback)\n",
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
fn marks_reflexive_direct_satisfy_disjuncts_for_different_bindings() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int, spare: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate == fallback or candidate == limit\n",
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
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"spare\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-3\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":3,\"reason\":\"satisfy_equality_match\",",
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
fn marks_parenthesized_satisfy_reflexive_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => (candidate == fallback)\n",
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
fn ignores_true_conjuncts_in_satisfy_reflexive_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate == fallback and true\n",
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
fn ignores_static_true_conjuncts_in_satisfy_reflexive_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(flag: Bool, fallback: Int) -> Int\n",
            "  _value satisfy candidate => candidate == fallback and (flag or not flag)\n",
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
fn ignores_tautological_nested_disjuncts_in_satisfy_reflexive_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate == fallback and (candidate > fallback or true)\n",
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
fn ignores_complementary_nested_disjuncts_in_satisfy_reflexive_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: {ready: Bool}, fallback: {ready: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => ",
            "candidate.ready == fallback.ready and (candidate.ready or not candidate.ready)\n",
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
        "\"type\":\"{ready: Bool}\",\"rank\":1,\"reason\":\"satisfy_equality_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"{ready: Bool}\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn marks_field_access_satisfy_reflexive_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: {count: Int}) -> {count: Int}\n",
            "  let fallback = {count: 1}\n",
            "  _value satisfy candidate => candidate.count == fallback.count\n",
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
        "\"type\":\"{count: Int}\",\"rank\":1,\"reason\":\"satisfy_equality_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"{count: Int}\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
}

#[test]
fn marks_field_access_satisfy_reflexive_comparison_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: {count: Int}) -> {count: Int}\n",
            "  let fallback = {count: 1}\n",
            "  _value satisfy candidate => fallback.count <= candidate.count and candidate.count <= fallback.count\n",
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
        "\"type\":\"{count: Int}\",\"rank\":1,\"reason\":\"satisfy_reflexive_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_satisfy_tautology_candidates_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => candidate == candidate\n",
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
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
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
