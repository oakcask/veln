use super::*;

#[test]
fn marks_literal_bound_contradiction_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: Int, fallback: Int) -> Int\n",
            "  _value satisfy candidate => not (candidate > 10 and candidate < 5)\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",",
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
fn marks_inclusive_bound_as_order_or_equality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max >= 10\n",
            "  _value satisfy candidate => candidate > 10 or candidate == 10\n",
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
fn marks_numeric_disequality_as_ordering_disjunction_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max != 0\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_rational_disequality_as_ordering_disjunction_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Float, fallback: Float) -> Float\n",
            "  require max != 1 / 3\n",
            "  _value satisfy candidate => candidate < 1 / 3 or candidate > 1 / 3\n",
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
fn marks_aliased_numeric_disequality_as_ordering_disjunction_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max == fallback\n",
            "  require fallback != 0\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
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
fn marks_disjunctive_alias_branches_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int, backup: Int, other: Int) -> Int\n",
            "  require max == fallback or max == backup\n",
            "  require fallback > 0\n",
            "  require backup > 0\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"backup\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-3\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":3,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-4\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":4,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        3
    );
}

#[test]
fn marks_boolean_disequality_alias_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(flag: Bool, ready: Bool, fallback: Bool) -> Bool\n",
            "  require flag != ready\n",
            "  require ready == false\n",
            "  _value satisfy candidate => candidate == true\n",
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
        "{\"candidate_id\":\"symbol-3\",\"name\":\"flag\",",
        "\"type\":\"Bool\",\"rank\":3,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        1
    );
}

#[test]
fn marks_reversed_boolean_disequality_alias_as_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(flag: Bool, ready: Bool, fallback: Bool) -> Bool\n",
            "  require ready != flag\n",
            "  require ready == false\n",
            "  _value satisfy candidate => candidate == true\n",
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
        "{\"candidate_id\":\"symbol-3\",\"name\":\"flag\",",
        "\"type\":\"Bool\",\"rank\":3,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        1
    );
}

#[test]
fn does_not_mark_boolean_disequality_alias_when_literal_conflicts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(flag: Bool, ready: Bool, fallback: Bool) -> Bool\n",
            "  require flag != ready\n",
            "  require ready == true\n",
            "  _value satisfy candidate => candidate == true\n",
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
    assert!(!details.contains(concat!(
        "{\"candidate_id\":\"symbol-3\",\"name\":\"flag\",",
        "\"type\":\"Bool\",\"rank\":3,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
}

#[test]
fn marks_static_case_split_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: {ready: Bool}, fallback: {ready: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => candidate.ready or (not candidate.ready and true)\n",
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
        "\"type\":\"{ready: Bool}\",\"rank\":1,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",",
        "\"type\":\"{ready: Bool}\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
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
fn marks_static_negated_disjunction_covered_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: {ready: Bool, paid: Bool}, fallback: {ready: Bool, paid: Bool}) -> {ready: Bool, paid: Bool}\n",
            "  _value satisfy candidate => not ((candidate.ready or candidate.paid) and not candidate.ready and not candidate.paid)\n",
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
        "\"type\":\"{ready: Bool, paid: Bool}\",\"rank\":1,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",",
        "\"type\":\"{ready: Bool, paid: Bool}\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
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
fn marks_static_negated_disjunction_or_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: {ready: Bool, paid: Bool}, fallback: {ready: Bool, paid: Bool}) -> {ready: Bool, paid: Bool}\n",
            "  _value satisfy candidate => not (candidate.ready or candidate.paid) or candidate.ready or candidate.paid\n",
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
        "\"type\":\"{ready: Bool, paid: Bool}\",\"rank\":1,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",",
        "\"type\":\"{ready: Bool, paid: Bool}\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
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
fn marks_static_negated_disjunction_repeat_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: {ready: Bool}, fallback: {ready: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => not (candidate.ready and not (candidate.ready or primary.ready))\n",
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
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn marks_static_triple_case_split_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: {ready: Bool, paid: Bool, shipped: Bool}, fallback: {ready: Bool, paid: Bool, shipped: Bool}) -> {ready: Bool, paid: Bool, shipped: Bool}\n",
            "  _value satisfy candidate => ",
            "(candidate.ready and candidate.paid and candidate.shipped) or ",
            "(candidate.ready and candidate.paid and not candidate.shipped) or ",
            "(candidate.ready and not candidate.paid and candidate.shipped) or ",
            "(candidate.ready and not candidate.paid and not candidate.shipped) or ",
            "(not candidate.ready and candidate.paid and candidate.shipped) or ",
            "(not candidate.ready and candidate.paid and not candidate.shipped) or ",
            "(not candidate.ready and not candidate.paid and candidate.shipped) or ",
            "(not candidate.ready and not candidate.paid and not candidate.shipped)\n",
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
        "\"type\":\"{ready: Bool, paid: Bool, shipped: Bool}\",\"rank\":1,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",",
        "\"type\":\"{ready: Bool, paid: Bool, shipped: Bool}\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
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
fn marks_static_quad_case_split_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: {a: Bool, b: Bool, c: Bool, d: Bool}, fallback: {a: Bool, b: Bool, c: Bool, d: Bool}) -> {a: Bool, b: Bool, c: Bool, d: Bool}\n",
            "  _value satisfy candidate => ",
            "(candidate.a and candidate.b and candidate.c and candidate.d) or ",
            "(candidate.a and candidate.b and candidate.c and not candidate.d) or ",
            "(candidate.a and candidate.b and not candidate.c and candidate.d) or ",
            "(candidate.a and candidate.b and not candidate.c and not candidate.d) or ",
            "(candidate.a and not candidate.b and candidate.c and candidate.d) or ",
            "(candidate.a and not candidate.b and candidate.c and not candidate.d) or ",
            "(candidate.a and not candidate.b and not candidate.c and candidate.d) or ",
            "(candidate.a and not candidate.b and not candidate.c and not candidate.d) or ",
            "(not candidate.a and candidate.b and candidate.c and candidate.d) or ",
            "(not candidate.a and candidate.b and candidate.c and not candidate.d) or ",
            "(not candidate.a and candidate.b and not candidate.c and candidate.d) or ",
            "(not candidate.a and candidate.b and not candidate.c and not candidate.d) or ",
            "(not candidate.a and not candidate.b and candidate.c and candidate.d) or ",
            "(not candidate.a and not candidate.b and candidate.c and not candidate.d) or ",
            "(not candidate.a and not candidate.b and not candidate.c and candidate.d) or ",
            "(not candidate.a and not candidate.b and not candidate.c and not candidate.d)\n",
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
        "\"type\":\"{a: Bool, b: Bool, c: Bool, d: Bool}\",\"rank\":1,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",",
        "\"type\":\"{a: Bool, b: Bool, c: Bool, d: Bool}\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
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
fn marks_static_quint_case_split_satisfy_predicate_as_tautology_repair() {
    let predicate = exhaustive_case_split_predicate("candidate", &["a", "b", "c", "d", "e"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn main(primary: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool}}, fallback: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool}}) -> {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool}}\n  _value satisfy candidate => {predicate}\nend\n"
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
        "\"type\":\"{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool}\",\"rank\":1,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",",
        "\"type\":\"{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool}\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
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
fn marks_static_sext_case_split_satisfy_predicate_as_tautology_repair() {
    let predicate = exhaustive_case_split_predicate("candidate", &["a", "b", "c", "d", "e", "f"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn main(primary: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool}}, fallback: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool}}) -> {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool}}\n  _value satisfy candidate => {predicate}\nend\n"
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
        "\"type\":\"{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool}\",\"rank\":1,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",",
        "\"type\":\"{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool}\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}
