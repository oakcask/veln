use super::*;

#[test]
fn marks_static_sept_case_split_satisfy_predicate_as_tautology_repair() {
    let predicate =
        exhaustive_case_split_predicate("candidate", &["a", "b", "c", "d", "e", "f", "g"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn main(primary: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}, fallback: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}) -> {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}\n  _value satisfy candidate => {predicate}\nend\n"
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
fn marks_static_oct_case_split_satisfy_predicate_as_tautology_repair() {
    let predicate =
        exhaustive_case_split_predicate("candidate", &["a", "b", "c", "d", "e", "f", "g", "h"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn main(primary: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}, fallback: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}) -> {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}\n  _value satisfy candidate => {predicate}\nend\n"
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
fn marks_distinct_literal_equality_contradiction_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: String, fallback: String) -> String\n",
            "  _value satisfy candidate => not (candidate == \"ready\" and candidate == \"done\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",\
         \"type\":\"String\",\"rank\":1,\"reason\":\"satisfy_tautology\",\
         \"application_policy\":\"safe_repair_candidate\","
    ));
    assert!(details.contains(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",\
         \"type\":\"String\",\"rank\":2,\"reason\":\"satisfy_tautology\",\
         \"application_policy\":\"safe_repair_candidate\","
    ));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn does_not_mark_invalid_static_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: Int, fallback: Int) -> Int\n",
            "  _value satisfy candidate => candidate.ready or (not candidate.ready and true)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_field_missing"
            && diagnostic.details.to_json().contains("\"field\":\"ready\"")
    }));
    let hole = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "hole.unfilled")
        .expect("unfilled hole diagnostic");
    let details = hole.details.to_json();
    assert!(!details.contains("\"reason\":\"satisfy_tautology\""));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
    assert!(!details.contains("\"application_policy\":\"safe_repair_candidate\""));
}
