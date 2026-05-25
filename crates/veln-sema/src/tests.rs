use crate::*;
use veln_ast::{SurfaceModule, lower_surface_ast};
use veln_core::{
    CoreBlocker, CoreCallTarget, CoreExprKind, CorePatternKind, CoreReadiness, CoreStmtKind,
    CoreType,
};
use veln_diagnostics::DiagnosticKind;
use veln_ir::{IrCallTarget, IrExprKind, IrPatternKind, IrStmtKind};
use veln_source::SourceFile;
use veln_syntax::parse;

#[test]
fn public_function_requires_explicit_boundary() {
    let source = SourceFile::new("main.veln", "pub fn main(value)\n  value\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public parameter `value` has no type annotation"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public function has no return type annotation"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "effect.missing_public"
            && diagnostic.kind == DiagnosticKind::Effect
            && diagnostic.message == "public function has no effects annotation"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn private_function_may_omit_boundary_annotations_when_inference_is_complete() {
    let source = SourceFile::new("main.veln", "fn answer()\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn private_function_reports_incomplete_annotation_inference() {
    let source = SourceFile::new("main.veln", "fn helper(value)\n  value\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private parameter `value` has no inferred type"
            && diagnostic
                .details
                .to_json()
                .contains("\"missing_fact\":\"parameter_type\"")
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private function has no inferred return type"
            && diagnostic
                .details
                .to_json()
                .contains("\"missing_fact\":\"return_type\"")
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn test_declaration_requires_explicit_test_shape() {
    let source = SourceFile::new(
        "main_test.veln",
        "test bad(value: Int) -> Int\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.parameters"
            && diagnostic.message == "test declaration has parameters"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.return_type"
            && diagnostic.message == "test declaration returns `Int`"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "effect.missing_test"
            && diagnostic.kind == DiagnosticKind::Effect
            && diagnostic.message == "test declaration has no effects annotation"
            && diagnostic
                .details
                .to_json()
                .contains("\"boundary\":\"test_declaration\"")
    }));
}

#[test]
fn test_declaration_checks_declared_effect_boundary() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test prints() -> () effects []\n",
            "  stdio::println(\"hello\")\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_test");
    assert_eq!(
        diagnostics[0].message,
        "test declaration uses undeclared effect `stdio`"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"node_id\":\"test-1\"")
    );
}

#[test]
fn test_declarations_are_not_callable_functions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "test helper() -> () effects []\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  helper()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.unresolved");
    assert_eq!(diagnostics[0].message, "unresolved call_target `helper`");
}

#[test]
fn duplicate_function_like_declaration_names_are_static_errors() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test same() -> () effects []\n",
            "  ()\n",
            "end\n",
            "fn same() -> () effects []\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(
        diagnostics[0].message,
        "duplicate function declaration name `same`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"function\"")
    );
}

#[test]
fn duplicate_use_aliases_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use platform.io\n",
            "use local.io\n",
            "fn main() -> () effects []\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate import alias name `io`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"module\"")
    );
}

#[test]
fn use_declarations_require_module_identity() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use platform.io\n",
            "fn main() -> () effects []\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "module.missing_identity");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Module);
    assert_eq!(
        diagnostics[0].message,
        "module import requires a module identity"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"field\":\"module_identity\"")
    );
}

#[test]
fn duplicate_parameter_names_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Int, value: Int) -> Int effects []\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate parameter name `value`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"value\"")
    );
}

#[test]
fn let_names_cannot_duplicate_the_function_value_scope() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Int) -> Int effects []\n  let value = 1\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(
        diagnostics[0].message,
        "duplicate local binding name `value`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"value\"")
    );
}

#[test]
fn duplicate_record_field_names_are_static_errors() {
    let source = SourceFile::new("main.veln", "fn bad() -> {a: Int}\n  {a: 1, a: 2}\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate record field name `a`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"record_field\"")
    );
}

#[test]
fn duplicate_pattern_bindings_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(input: {left: Int, right: Int}) -> Int\n",
            "  match input\n",
            "    {left: value, right: value} => value\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate pattern binding name `value`"
                && diagnostic.related.len() == 1
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn duplicate_record_pattern_field_names_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(input: {value: Int}) -> Int\n",
            "  match input\n",
            "    {value: first, value: second} => first\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate record pattern field name `value`"
                && diagnostic.related.len() == 1
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn reports_hole_with_declared_return_expected_type() {
    let source = SourceFile::new("main.veln", "fn todo() -> Result((), AppError)\n  _\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Hole);
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"hole\",\"node_id\":\"hole-3\",\"label\":null,",
            "\"expected_type\":\"Result((), AppError)\",",
            "\"expected_type_source\":\"declared\",",
            "\"constraints\":[],\"local_bindings\":[],",
            "\"candidate_queries\":[{\"kind\":\"symbol\",",
            "\"candidate_status\":\"query_only\",",
            "\"application_policy\":\"manual_review_required\",",
            "\"query\":\"fn() -> Result((), AppError)\"}]}"
        )
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn ranks_visible_symbol_candidates_for_hole_expected_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"candidates\":["));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\",",
        "\"edits\":[{\"kind\":\"replace\","
    )));
    assert!(details.contains(concat!(
        "\"span\":{\"file\":\"main.veln\",",
        "\"start\":{\"line\":3,\"column\":3,\"offset\":48},",
        "\"end\":{\"line\":3,\"column\":4,\"offset\":49}}"
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\",",
        "\"edits\":[{\"kind\":\"replace\","
    )));
    assert!(details.contains("\"replacement\":\"limit\""));
}

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
fn does_not_mark_disjunctive_direct_satisfy_candidates_with_different_bindings() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
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
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(!details.contains("\"application_policy\":\"safe_repair_candidate\""));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
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

#[test]
fn marks_negated_satisfy_tautology_candidates_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => not (candidate != candidate)\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
}

#[test]
fn marks_parenthesized_satisfy_tautology_candidates_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _value satisfy candidate => (candidate <= candidate)\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_tautology\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
}

#[test]
fn ignores_false_disjuncts_in_satisfy_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int, fallback: Int) -> Int\n",
            "  _value satisfy candidate => candidate == candidate or false\n",
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

#[test]
fn marks_true_satisfy_disjunct_as_tautological_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int, fallback: Int) -> Int\n",
            "  _value satisfy candidate => true or candidate == fallback\n",
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

#[test]
fn marks_require_discharged_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0\n",
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
fn marks_commuted_require_discharged_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require 0 < max\n",
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
fn marks_strict_require_as_inclusive_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0\n",
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
fn marks_strict_require_as_disequality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max > 0\n",
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
fn marks_commuted_strict_require_as_disequality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require 0 < max\n",
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
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_less_than_require_as_disequality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max < 10\n",
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
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_equality_require_as_inclusive_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max == 0\n",
            "  _value satisfy candidate => candidate <= 0 and candidate >= 0\n",
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
fn marks_string_require_discharged_satisfy_candidate_as_safe_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(name: String, fallback: String) -> String\n",
            "  require name != \"\"\n",
            "  _value satisfy candidate => candidate != \"\"\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"name\",",
        "\"type\":\"String\",\"rank\":2,\"reason\":\"satisfy_require_match\",",
        "\"application_policy\":\"safe_repair_candidate\","
    )));
    assert!(details.contains("\"replacement\":\"name\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_negated_equality_require_as_disequality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require not (max == 0)\n",
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
    assert!(details.contains("\"replacement\":\"max\""));
    assert!(details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn marks_disequality_require_as_negated_equality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(max: Int, fallback: Int) -> Int\n",
            "  require max != 0\n",
            "  _value satisfy candidate => not (candidate == 0)\n",
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
fn marks_transitive_strict_order_as_disequality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(low: Int, mid: Int, max: Int, fallback: Int) -> Int\n",
            "  require low <= mid\n",
            "  require mid < max\n",
            "  _value satisfy candidate => candidate != low\n",
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
fn marks_transitive_inclusive_bounds_as_equality_satisfy_repair_evidence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(low: Int, mid: Int, max: Int, fallback: Int) -> Int\n",
            "  require low <= mid\n",
            "  require mid <= max\n",
            "  require max <= low\n",
            "  _value satisfy candidate => candidate == low\n",
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
fn leaves_inclusive_transitive_order_as_manual_for_strict_satisfy_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(low: Int, mid: Int, max: Int, fallback: Int) -> Int\n",
            "  require low <= mid\n",
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
        "{\"candidate_id\":\"symbol-2\",\"name\":\"max\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\","
    )));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
}

#[test]
fn reports_return_type_mismatch() {
    let source = SourceFile::new("main.veln", "fn bad() -> Int\n  \"no\"\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-3\",\"expected_type\":\"Int\",",
            "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"inferred_expression\",",
            "\"constraint\":\"return_value\",",
            "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-3\"]}"
        )
    );
}

#[test]
fn omitted_tail_expression_returns_unit() {
    let source = SourceFile::new("main.veln", "fn main() -> () effects []\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn omitted_tail_expression_checks_declared_return_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Int effects []\n  let value = 1\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `()`");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"fn-1\",\"expected_type\":\"Int\",",
            "\"actual_type\":\"()\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"implicit_unit\",",
            "\"constraint\":\"return_value\",",
            "\"origin_node_ids\":[\"fn-1\",\"fn-1\"]}"
        )
    );
}

#[test]
fn omitted_tail_expression_lowers_to_unit_return() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> () effects []\n", "  let value = 1\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    assert!(matches!(main.body[0].kind, CoreStmtKind::Let { .. }));
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("omitted tail should lower as unit return");
    };
    assert!(matches!(expr.kind, CoreExprKind::Unit));

    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[1].kind else {
        panic!("omitted tail should lower as IR unit return");
    };
    assert!(matches!(value.kind, IrExprKind::Unit));
}

#[test]
fn ok_constructor_accepts_declared_result_return() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result((), AppError)\n  Ok(())\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn result_constructor_checks_expected_value_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result((), AppError)\n  Ok(\"no\")\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-5\",\"expected_type\":\"()\",",
            "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"inferred_expression\",",
            "\"constraint\":\"call_argument\",",
            "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-5\"]}"
        )
    );
}

#[test]
fn accepts_first_slice_type_forms_and_record_expected_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> {score: Float, names: List(String), table: Dict(String, Int), ",
            "callback: fn(Int) -> String}\n",
            "  {score: _, names: [], table: _, callback: _}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.details.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("\"expected_type\":\"Float\""));
    assert!(rendered.contains("\"expected_type\":\"Dict(String, Int)\""));
    assert!(rendered.contains("\"expected_type\":\"fn(Int) -> String\""));
    assert!(rendered.contains("\"candidate_queries\":[{\"kind\":\"symbol\""));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.related.is_empty())
    );
}

#[test]
fn accepts_dictionary_literals_with_expected_key_and_value_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Dict(String, Int)\n",
            "  {\"one\": 1, \"two\": 2}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::dict(CoreType::string(), CoreType::int()));
    let CoreExprKind::Dict(entries) = &expr.kind else {
        panic!("tail expression should lower as dictionary");
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].key.ty, CoreType::string());
    assert_eq!(entries[0].value.ty, CoreType::int());
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Dict(entries) = &value.kind else {
        panic!("tail expression should lower as IR dictionary");
    };
    assert_eq!(entries.len(), 2);
}

#[test]
fn accepts_dictionary_literals_with_identifier_led_expression_keys() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(seed: Int) -> Dict(Int, String)\n",
            "  {seed + 1: \"next\"}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::dict(CoreType::int(), CoreType::string()));
    let CoreExprKind::Dict(entries) = &expr.kind else {
        panic!("tail expression should lower as dictionary");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key.ty, CoreType::int());
    assert_eq!(entries[0].value.ty, CoreType::string());
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Dict(entries) = &value.kind else {
        panic!("tail expression should lower as IR dictionary");
    };
    assert_eq!(entries.len(), 1);
}

#[test]
fn record_patterns_bind_field_types_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: {count: Int, label: String}) -> String effects []\n",
            "  match value\n",
            "    {count: 0, label: name} => name\n",
            "    {count: count, label: _} => \"many\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Match { arms, .. } = &expr.kind else {
        panic!("tail expression should lower as match");
    };
    let CorePatternKind::Record(fields) = &arms[0].pattern.kind else {
        panic!("first arm should lower as record pattern");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "count");
    assert_eq!(fields[1].name, "label");

    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Match { arms, .. } = &value.kind else {
        panic!("tail expression should lower as IR match");
    };
    assert!(matches!(
        &arms[1].pattern.kind,
        IrPatternKind::Record(fields)
            if fields.iter().any(|field| field.name == "count")
    ));
}

#[test]
fn match_expression_type_checks_inside_call_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn wrap(value: String) -> String effects []\n",
            "  value\n",
            "end\n",
            "fn describe(value: Option(Int)) -> String effects []\n",
            "  wrap(match value\n",
            "    Some(count) => \"some\"\n",
            "    None => \"none\"\n",
            "  end)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let describe = core
        .functions
        .iter()
        .find(|function| function.name == "describe")
        .expect("describe should be lowered");
    let CoreStmtKind::Return { expr } = &describe.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(args[0].kind, CoreExprKind::Match { .. }));
}

#[test]
fn accepts_float_numeric_operators() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float, right: Float) -> {sum: Float, negated: Float, ordered: Bool} effects []\n",
            "  {sum: left + right, negated: -left, ordered: left < right}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Record(fields) = &expr.kind else {
        panic!("tail expression should lower as record");
    };
    assert_eq!(fields[0].expr.ty, CoreType::float());
    assert_eq!(fields[1].expr.ty, CoreType::float());
    assert_eq!(fields[2].expr.ty, CoreType::bool());
    assert!(matches!(
        &fields[0].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
    assert!(matches!(
        &fields[1].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_negate"
    ));
    assert!(matches!(
        &fields[2].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_less"
    ));
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Record(fields) = &value.kind else {
        panic!("tail expression should lower as IR record");
    };
    assert!(matches!(
        &fields[0].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
    assert!(matches!(
        &fields[1].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_negate"
    ));
    assert!(matches!(
        &fields[2].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_less"
    ));
}

#[test]
fn lowers_boolean_literals_through_core_and_ir() {
    let source = SourceFile::new("main.veln", "fn main() -> Bool\n  true\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::bool());
    assert!(matches!(expr.kind, CoreExprKind::BoolLiteral(true)));

    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert_eq!(value.ty, CoreType::bool());
    assert!(matches!(value.kind, IrExprKind::BoolLiteral(true)));
}

#[test]
fn infers_float_numeric_operators_from_call_results() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn value() -> Float\n",
            "  1.0\n",
            "end\n",
            "fn main()\n",
            "  value() + value()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::float());
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
}

#[test]
fn accepts_int_operands_in_float_operator_contexts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float, count: Int) -> {sum: Float, ordered: Bool, expected: Float} effects []\n",
            "  {sum: left + count, ordered: count < left, expected: 1 + 2}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Record(fields) = &expr.kind else {
        panic!("tail expression should lower as record");
    };
    assert_eq!(fields[0].expr.ty, CoreType::float());
    assert_eq!(fields[1].expr.ty, CoreType::bool());
    assert_eq!(fields[2].expr.ty, CoreType::float());
    assert!(matches!(
        &fields[0].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
    assert!(matches!(
        &fields[1].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_less"
    ));
    assert!(matches!(
        &fields[2].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
}

#[test]
fn rejects_int_values_in_float_assignment_contexts() {
    let source = SourceFile::new(
        "main.veln",
        concat!("pub fn main() -> Float effects []\n", "  1\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Float`, but found `Int`");
}

#[test]
fn reports_float_operator_operand_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float) -> Float effects []\n",
            "  left + \"bad\"\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `Float`, but found `String`"
    );
}

#[test]
fn comparison_does_not_select_float_from_expected_result() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Int, right: Int) -> Float effects []\n",
            "  left < right\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Float`, but found `Bool`");
}

#[test]
fn reports_invalid_type_annotations() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Result(Int)) -> Option()\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.invalid_annotation")
    );
}

#[test]
fn infers_non_constructor_calls_from_local_function_signatures() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result(Int, AppError)\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main() -> Result(Int, AppError) effects []\n",
            "  parse(\"1\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn resolves_qualified_calls_through_import_aliases() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.math\n",
            "pub fn main() -> Int effects []\n",
            "  math::double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod app.math\n",
            "fn double(value: Int) -> Int\n",
            "  value + value\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let math = lower_surface_ast(&parse(&math_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        functions: main.functions.into_iter().chain(math.functions).collect(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { target, .. } = &expr.kind else {
        panic!("qualified call should lower as a call");
    };
    assert_eq!(target, &CoreCallTarget::Function("double".to_string()));
}

#[test]
fn unresolved_qualified_calls_do_not_fall_back_to_bare_functions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "fn helper(value: String) -> String\n",
            "  value\n",
            "end\n",
            "pub fn main() -> Int effects []\n",
            "  math::helper(2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `math::helper`"
    }));
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic.message == "expected `String`, but found `Int`"
    }));
}

#[test]
fn pipeline_inserts_left_value_as_first_call_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn add(left: Int, right: Int) -> Int\n",
            "  left + right\n",
            "end\n",
            "pub fn main() -> Int effects []\n",
            "  1 |> add(2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { target, args } = &expr.kind else {
        panic!("pipeline should lower as a call");
    };
    assert_eq!(target, &CoreCallTarget::Function("add".to_string()));
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0].kind, CoreExprKind::IntLiteral(value) if value == "1"));
    assert!(matches!(&args[1].kind, CoreExprKind::IntLiteral(value) if value == "2"));
}

#[test]
fn pipeline_requires_call_target() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> Int effects []\n  1 |> 2\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.pipeline_target"
            && diagnostic.message == "pipeline target is not a call"
    }));
}

#[test]
fn pipeline_requires_named_call_target() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn make(value: Int, callback: fn(Int) -> Int) -> fn(Int) -> Int\n",
            "  callback\n",
            "end\n",
            "pub fn main(callback: fn(Int) -> Int) -> Int effects []\n",
            "  1 |> make(0, callback)(2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.pipeline_target"
            && diagnostic.message == "pipeline target is not a named call"
    }));
}

#[test]
fn infers_prelude_helper_calls_from_expected_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(items: List(Int), other: List(Int), table: Dict(String, Int), ",
            "mapper: fn(Int) -> String, keep: fn(Int) -> Bool, folder: fn(String, Int) -> String, ",
            "fallible: fn(Int) -> Result(String, AppError), opt: Option(Int), ",
            "fallible_with: fn(String, Int) -> Result(String, AppError), ",
            "opt_map: fn(Int) -> String, opt_next: fn(Int) -> Option(String), ",
            "res: Result(Int, AppError), err_map: fn(AppError) -> String, ",
            "res_next: fn(Int) -> Result(String, AppError)) -> {",
            "count: Int, empty: Bool, pushed: List(Int), joined: List(Int), mapped: List(String), ",
            "filtered: List(Int), folded: String, tried: Result(List(String), AppError), ",
            "tried_with: Result(List(String), AppError), split: Option({left: String, right: String}), ",
            "parsed: Result(Int, String), rendered: String, ",
            "found: Option(Int), has_key: Bool, inserted: Dict(String, Int), removed: Dict(String, Int), ",
            "opt_mapped: Option(String), opt_nexted: Option(String), opt_value: Int, ",
            "res_mapped: Result(String, AppError), res_err: Result(Int, String), ",
            "res_nexted: Result(String, AppError)} effects []\n",
            "  {count: list_len(items), empty: list_is_empty(items), ",
            "pushed: list_push(items, 1), joined: list_concat(items, other), ",
            "mapped: list_map(items, mapper), filtered: list_filter(items, keep), ",
            "folded: list_fold(items, \"\", folder), tried: list_try_map(items, fallible), ",
            "tried_with: list_try_map_with(\"prefix\", items, fallible_with), ",
            "split: string_split_once(\"sku,2\", \",\"), parsed: string_parse_int(\"2\"), ",
            "rendered: int_to_string(2), ",
            "found: dict_get(table, \"a\"), has_key: dict_contains(table, \"a\"), ",
            "inserted: dict_insert(table, \"b\", 2), removed: dict_remove(table, \"b\"), ",
            "opt_mapped: option_map(opt, opt_map), opt_nexted: option_and_then(opt, opt_next), ",
            "opt_value: option_unwrap_or(opt, 0), res_mapped: result_map(res, opt_map), ",
            "res_err: result_map_err(res, err_map), res_nexted: result_and_then(res, res_next)}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Record(fields) = &expr.kind else {
        panic!("prelude results should be returned in a record");
    };
    let first = fields
        .first()
        .expect("record should contain prelude result fields");
    assert!(matches!(
        &first.expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "list_len"
    ));
    assert!(matches!(first.expr.ty, CoreType::Named { ref name, .. } if name == "Int"));
    let ir = lowered
        .ir
        .expect("complete prelude core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Record(fields) = &value.kind else {
        panic!("prelude record should lower to IR");
    };
    assert!(matches!(
        &fields[0].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "list_len"
    ));
}

#[test]
fn suggests_list_try_map_for_result_returning_map_callback() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(value: Int) -> Result(String, AppError) effects []\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "pub fn main(items: List(Int)) -> List(String) effects []\n",
            "  list_map(items, parse)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("callback type mismatch should be reported");
    assert_eq!(
        diagnostic.message,
        "expected `fn(unknown) -> String`, but found `fn(Int) -> Result(String, AppError)`"
    );
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| { related.to_json().contains("Use `list_try_map`") })
    );
}

#[test]
fn lowers_function_declarations_as_callable_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String effects []\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn main(items: List(Int)) -> List(String) effects []\n",
            "  list_map(items, stringify)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(
        &args[1].kind,
        CoreExprKind::FunctionValue(name) if name == "stringify"
    ));
    assert_eq!(
        args[1].ty,
        CoreType::Function {
            params: vec![CoreType::int()],
            return_type: Box::new(CoreType::string()),
            effects: Vec::new()
        }
    );

    let ir = lowered.ir.expect("complete core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Call { args, .. } = &value.kind else {
        panic!("tail expression should lower as IR call");
    };
    assert!(matches!(
        &args[1].kind,
        IrExprKind::FunctionValue(name) if name == "stringify"
    ));
}

#[test]
fn lowers_function_return_types_with_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn printer(text: String) -> () effects [stdio]\n",
            "  stdio::println(text)\n",
            "  ()\n",
            "end\n",
            "pub fn callback_factory() -> fn(String) -> () effects [stdio] effects []\n",
            "  printer\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let factory = core
        .functions
        .iter()
        .find(|function| function.name == "callback_factory")
        .expect("factory should be lowered");
    assert_eq!(
        factory.return_type,
        CoreType::Function {
            params: vec![CoreType::string()],
            return_type: Box::new(CoreType::unit()),
            effects: vec!["stdio".to_string()],
        }
    );
    assert_eq!(factory.effects, Vec::<String>::new());
}

#[test]
fn function_return_effects_must_cover_actual_callable_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn printer(text: String) -> () effects [stdio]\n",
            "  stdio::println(text)\n",
            "  ()\n",
            "end\n",
            "pub fn callback_factory() -> fn(String) -> () effects [] effects []\n",
            "  printer\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `fn(String) -> ()`, but found `fn(String) -> () effects [stdio]`"
    );
}

#[test]
fn call_resolution_prefers_local_callable_over_function_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String effects []\n",
            "  \"function\"\n",
            "end\n",
            "pub fn main(stringify: fn(Int) -> String effects []) -> String effects []\n",
            "  stringify(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { target, args } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert_eq!(target, &CoreCallTarget::Value("stringify".to_string()));
    assert!(matches!(&args[0].kind, CoreExprKind::IntLiteral(value) if value == "1"));
}

#[test]
fn non_callable_local_shadow_blocks_function_call_resolution() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String effects []\n",
            "  \"function\"\n",
            "end\n",
            "pub fn main(stringify: Int) -> String effects []\n",
            "  stringify(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `stringify`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn lowers_record_field_access_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String effects []\n",
            "  let payload: {name: String, count: Int} = {name: \"veln\", count: 1}\n",
            "  payload.name\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::FieldAccess { field, .. } if field == "name"
    ));
    assert_eq!(expr.ty, CoreType::string());

    let ir = lowered.ir.expect("complete core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[1].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::FieldAccess { field, .. } if field == "name"
    ));
}

#[test]
fn reports_missing_record_field_access() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Int effects []\n",
            "  let payload: {count: Int} = {count: 1}\n",
            "  payload.name\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.field_missing");
    assert_eq!(
        diagnostics[0].message,
        "type `{count: Int}` has no field `name`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn prelude_helpers_check_direct_expected_return_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Option(Int)) -> Int effects []\n",
            "  option_unwrap_or(value, \"bad\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
}

#[test]
fn flows_call_argument_expected_type_into_holes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn consume(value: Float) -> ()\n",
            "  ()\n",
            "end\n",
            "pub fn main() -> () effects []\n",
            "  consume(_)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"expected_type\":\"Float\"")
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn reports_missing_public_effect_with_call_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> () effects []\n",
            "  stdio::println(\"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Effect);
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"stdio\""));
    assert!(details.contains("\"declared_effects\":[]"));
    assert!(details.contains("\"inferred_effects\":[\"stdio\"]"));
    assert!(details.contains("\"symbol\":\"stdio::println\""));
    assert!(details.contains("\"provenance_paths\":[{\"effect\":\"stdio\""));
    assert!(details.contains("\"kind\":\"public_boundary\""));
    assert!(details.contains("\"hidden_frame_count\":0"));
    assert!(details.contains("\"omitted_path_count\":0"));
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn channel_calls_require_concurrency_effect() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(tx: Sender(String)) -> Result((), SendError) effects []\n",
            "  channel::send(tx, \"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `concurrency`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"concurrency\""));
    assert!(details.contains("\"inferred_effects\":[\"concurrency\"]"));
    assert!(details.contains("\"symbol\":\"channel::send\""));
}

#[test]
fn declared_concurrency_calls_lower_to_executable_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String effects [concurrency]\n",
            "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    assert!(matches!(
        lowered
            .core
            .expect("checked core should be built")
            .readiness,
        CoreReadiness::Complete
    ));
    let ir = lowered.ir.expect("concurrency calls should lower to IR");
    let main = &ir.functions[0];
    assert!(matches!(
        &main.body[0].kind,
        IrStmtKind::Let { value, .. }
            if matches!(
                &value.kind,
                IrExprKind::Call {
                    target: IrCallTarget::ConcurrencyBuiltin(name),
                    ..
                } if name == "channel::bounded"
            )
    ));
}

#[test]
fn channel_bounded_accepts_explicit_item_type_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String effects [concurrency]\n",
            "  let pair = channel::bounded[String](1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("expected channel binding");
    };
    assert_eq!(
        expr.ty,
        CoreType::Record(vec![
            (
                "tx".to_string(),
                CoreType::named("Sender", vec![CoreType::string()])
            ),
            (
                "rx".to_string(),
                CoreType::named("Receiver", vec![CoreType::string()])
            ),
        ])
    );
}

#[test]
fn channel_clone_preserves_sender_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(tx: Sender(String)) -> Result((), SendError) effects [concurrency]\n",
            "  let clone = channel::clone(tx)\n",
            "  channel::send(clone, \"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("expected cloned sender binding");
    };
    assert_eq!(expr.ty, CoreType::named("Sender", vec![CoreType::string()]));
}

#[test]
fn channel_send_checks_value_against_sender_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(tx: Sender(String)) -> Result((), SendError) effects [concurrency]\n",
            "  channel::send(tx, 1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `String`, but found `Int`");
}

#[test]
fn channel_recv_checks_receiver_against_expected_option_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(rx: Receiver(Int)) -> Option(String) effects [concurrency]\n",
            "  channel::recv(rx)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `Receiver(String)`, but found `Receiver(Int)`"
    );
}

#[test]
fn channel_close_requires_sender_handle() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(rx: Receiver(String)) -> () effects [concurrency]\n",
            "  channel::close(rx)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `Sender(unknown)`, but found `Receiver(String)`"
    );
}

#[test]
fn infers_transitive_private_helper_effects_from_body() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn say(text: String) -> ()\n",
            "  stdio::println(text)\n",
            "end\n",
            "fn greet(text: String) -> ()\n",
            "  say(text)\n",
            "end\n",
            "pub fn main() -> () effects []\n",
            "  greet(\"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"inferred_effects\":[\"stdio\"]"));
    assert!(details.contains("\"symbol\":\"greet\""));
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn infers_import_alias_call_effects_from_function_body() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.console\n",
            "pub fn main() -> () effects []\n",
            "  console::say(\"hello\")\n",
            "end\n",
        ),
    );
    let console_source = SourceFile::new(
        "console.veln",
        concat!(
            "mod app.console\n",
            "fn say(text: String) -> ()\n",
            "  stdio::println(text)\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let console = lower_surface_ast(&parse(&console_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        functions: main
            .functions
            .into_iter()
            .chain(console.functions)
            .collect(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"inferred_effects\":[\"stdio\"]"));
    assert!(details.contains("\"symbol\":\"console::say\""));
}

#[test]
fn function_typed_value_calls_infer_declared_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(callback: fn(String) -> () effects [stdio]) -> () effects []\n",
            "  callback(\"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"symbol\":\"callback\""));
}

#[test]
fn effect_provenance_reports_omitted_equivalent_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> () effects []\n",
            "  stdio::print(\"one\")\n",
            "  stdio::println(\"two\")\n",
            "  stdio::eprint(\"three\")\n",
            "  stdio::eprintln(\"four\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"provenance_truncated\":true"));
    assert!(details.contains("\"omitted_path_count\":1"));
}

#[test]
fn reports_non_boolean_contract_predicate() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Int) -> () effects []\n",
            "require value\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.type_mismatch"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract predicate is not `Bool`"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_boolean_predicate\"")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic.kind == DiagnosticKind::Type
            && diagnostic.message == "expected `Bool`, but found `Int`"
    }));
}

#[test]
fn ensure_can_reference_explicit_result_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> output: Int effects []\n",
            "ensure output == value\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_field_access_resolves_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {total: Int}) -> output: {total: Int} effects []\n",
            "ensure output.total == value.total\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_boolean_field_access_is_a_boolean_predicate() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}) -> output: {ready: Bool} effects []\n",
            "require value.ready\n",
            "ensure output.ready\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_pure_call_result_field_access() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn summary(value: Int) -> {total: Int, ready: Bool} effects []\n",
            "  {total: value, ready: true}\n",
            "end\n",
            "pub fn identity(value: Int) -> output: Int effects []\n",
            "require summary(value).ready\n",
            "ensure summary(output).total >= value\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_pure_boolean_function_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn positive(value: Int) -> Bool effects []\n",
            "  value > 0\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require positive(value)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_qualified_pure_function_calls() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.rules\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require rules::positive(value)\n",
            "  value\n",
            "end\n",
        ),
    );
    let rules_source = SourceFile::new(
        "rules.veln",
        concat!(
            "mod app.rules\n",
            "fn positive(value: Int) -> Bool effects []\n",
            "  value > 0\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let rules = lower_surface_ast(&parse(&rules_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        functions: main.functions.into_iter().chain(rules.functions).collect(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_pure_function_calls_inside_comparisons() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn same(value: Int) -> Int effects []\n",
            "  value\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require same(value) > 0\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_nested_pure_function_call_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn same(value: Int) -> Int effects []\n",
            "  value\n",
            "end\n",
            "fn positive(value: Int) -> Bool effects []\n",
            "  value > 0\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require positive(same(value))\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_arithmetic_function_call_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn positive(value: Int) -> Bool effects []\n",
            "  value > 0\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require positive(value + 1)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_arithmetic_function_call_comparisons() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn next(value: Int) -> Int effects []\n",
            "  value + 1\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require next(value) + 1 > 0\n",
            "require (next(value) * 2) > 0\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_arithmetic_comparisons() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, ratio: Float) -> Int effects []\n",
            "require value + 1 > 0\n",
            "require ratio + 1.5 > 0.0\n",
            "require not (value * 2 < 0)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_rejects_non_numeric_call_in_arithmetic_comparison() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn label(value: Int) -> String effects []\n",
            "  \"item\"\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require label(value) + 1 > 0\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.type_mismatch"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract predicate is not `Bool`"
    }));
}

#[test]
fn contract_predicate_rejects_arithmetic_as_non_boolean_predicate() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> Int effects []\n",
            "require value + 1\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.type_mismatch"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract predicate is not `Bool`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch" && diagnostic.message == "expected `Bool`, but found `Int`"
    }));
}

#[test]
fn contract_predicate_rejects_arithmetic_function_call_as_non_boolean_predicate() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn next(value: Int) -> Int effects []\n",
            "  value + 1\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require next(value) + 1\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.type_mismatch"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract predicate is not `Bool`"
    }));
}

#[test]
fn contract_predicate_rejects_not_on_non_boolean_arithmetic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> Int effects []\n",
            "require not value + 1\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.type_mismatch"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract predicate is not `Bool`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic.message == "expected `Bool`, but found `unknown`"
    }));
}

#[test]
fn contract_predicate_rejects_effectful_function_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn noisy(value: Int) -> Bool effects [stdio]\n",
            "  stdio::println(\"checking\")\n",
            "  value > 0\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require noisy(value)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.unsupported_construct"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"effectful_operation\"")
    }));
}

#[test]
fn contract_predicate_rejects_non_boolean_function_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn same(value: Int) -> Int effects []\n",
            "  value\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require same(value)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.type_mismatch"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract predicate is not `Bool`"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_boolean_predicate\"")
    }));
}

#[test]
fn contract_predicate_rejects_non_boolean_function_calls_in_boolean_position() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn same(value: Int) -> Int effects []\n",
            "  value\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require same(value) and true\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.type_mismatch"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract predicate is not `Bool`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic
                .message
                .contains("expected `Bool`, but found `Int`")
    }));
}

#[test]
fn contract_missing_record_field_reports_contract_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {total: Int}) -> output: {total: Int} effects []\n",
            "ensure output.missing == value.total\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.field_missing"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract field `missing` is not present on `{total: Int}`"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"missing_field\"")
    }));
}

#[test]
fn contract_missing_call_result_field_reports_contract_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn summary(value: Int) -> {total: Int} effects []\n",
            "  {total: value}\n",
            "end\n",
            "pub fn identity(value: Int) -> Int effects []\n",
            "require summary(value).missing == 1\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.field_missing"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract field `missing` is not present on `{total: Int}`"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"missing_field\"")
    }));
}

#[test]
fn require_cannot_reference_result_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> output: Int effects []\n",
            "require output > 0\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved contract_predicate `output`"
    }));
}

#[test]
fn bare_result_has_no_ensure_special_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> Int effects []\n",
            "ensure result == value\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved contract_predicate `result`"
    }));
}

#[test]
fn result_binding_is_not_in_function_body_scope() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> output: Int effects []\n",
            "  output\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `output`"
    }));
}

#[test]
fn result_binding_cannot_duplicate_parameter_name() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(output: Int) -> output: Int effects []\n",
            "ensure output == 0\n",
            "  output\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate result binding name `output`"
    }));
}

#[test]
fn hole_diagnostic_includes_contract_and_satisfy_constraints() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn default_port(max: Int) -> Int effects []\n",
            "require max > 0\n",
            "  _port satisfy candidate => candidate > 0 and candidate <= max\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type\":\"Int\""));
    assert!(details.contains("\"candidate_status\":\"query_only\""));
    assert!(details.contains("\"application_policy\":\"manual_review_required\""));
    assert!(details.contains("\"kind\":\"contract\""));
    assert!(details.contains("\"clause\":\"require\""));
    assert!(details.contains("\"text\":\"max > 0\""));
    assert!(details.contains("\"kind\":\"satisfy\""));
    assert!(details.contains(
        "\"text\":\"candidate > 0 and candidate <= max\",\"candidate_binding\":\"candidate\""
    ));
    assert!(details.contains("\"repair_status\":\"blocked_until_discharged\""));
    assert_eq!(diagnostics[0].related.len(), 3);
}

#[test]
fn satisfy_candidate_reports_shadowing_and_unused_predicates() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn default_port(max: Int) -> Int\n",
            "  _port satisfy max => true\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_candidate_shadow"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy candidate `max` shadows a visible binding"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_candidate_unused"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy predicate does not reference candidate `max`"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn satisfy_predicate_is_checked_with_candidate_expected_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(limit: Int) -> Int\n",
            "  _value satisfy candidate => candidate > 0 and candidate <= limit\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
}

#[test]
fn satisfy_predicate_reports_non_boolean_candidate_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> Int\n",
            "  _value satisfy candidate => candidate\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_type_mismatch"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy predicate is not `Bool`"
            && diagnostic
                .details
                .to_json()
                .contains("\"actual_type\":\"Int\"")
    }));
}

#[test]
fn satisfy_predicate_reports_unresolved_names() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> Int\n",
            "  _value satisfy candidate => candidate == missing\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.kind == DiagnosticKind::Name
            && diagnostic.message == "unresolved satisfy_predicate `missing`"
    }));
}

#[test]
fn propagates_try_expected_type_from_result_return() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result(Int, AppError)\n  Ok(_?)\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"expected_type\":\"Result(Int, AppError)\"")
    );
}

#[test]
fn lowers_option_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option(String) effects []\n",
            "  Some(\"ok\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::option(CoreType::string()));
    let CoreExprKind::OptionSome(value) = &expr.kind else {
        panic!("Some call should lower to an option constructor");
    };
    assert_eq!(value.ty, CoreType::string());
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_none_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option(String) effects []\n",
            "  None\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::option(CoreType::string()));
    assert!(matches!(expr.kind, CoreExprKind::OptionNone));
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_qualified_none_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option(String) effects []\n",
            "  Option::None\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::option(CoreType::string()));
    assert!(matches!(expr.kind, CoreExprKind::OptionNone));
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_qualified_builtin_constructors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(use_result: Bool) -> Result(Option(String), AppError) effects []\n",
            "  if_missing(use_result)\n",
            "end\n",
            "fn if_missing(use_result: Bool) -> Result(Option(String), AppError) effects []\n",
            "  Result::Ok(Option::Some(\"ok\"))\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let helper = core
        .functions
        .iter()
        .find(|function| function.name == "if_missing")
        .expect("helper should be lowered");
    let CoreStmtKind::Return { expr } = &helper.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::string()),
            CoreType::named("AppError", Vec::new())
        )
    );
    let CoreExprKind::ResultOk(value) = &expr.kind else {
        panic!("Result::Ok call should lower to a result constructor");
    };
    assert!(matches!(value.kind, CoreExprKind::OptionSome(_)));
}

#[test]
fn lowers_runnable_checked_program_to_core_and_typed_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main(raw: String) -> Result((), AppError) effects [stdio]\n",
            "  let value: Int = parse(raw)?\n",
            "  stdio::println(\"ok\")\n",
            "  Ok(())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    assert!(matches!(main.body[0].kind, CoreStmtKind::Let { .. }));
    let CoreStmtKind::Expr { expr } = &main.body[1].kind else {
        panic!("stdio call should lower as an expression statement");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::StdioBuiltin(symbol),
            ..
        } if symbol == "stdio::println"
    ));
    assert!(matches!(main.body[2].kind, CoreStmtKind::Return { .. }));

    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    assert!(matches!(main.body[0].kind, IrStmtKind::Let { .. }));
    let IrStmtKind::Expr { value } = &main.body[1].kind else {
        panic!("stdio call should stay an expression statement in IR");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StdioBuiltin(symbol),
            ..
        } if symbol == "stdio::println"
    ));
    let IrStmtKind::Return { value } = &main.body[2].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(value.kind, IrExprKind::ResultOk(_)));
}

#[test]
fn wildcard_let_lowers_to_discarding_expression_statement() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Int) -> () effects []\n",
            "  let _: Int = value\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Expr { expr } = &main.body[0].kind else {
        panic!("wildcard let should lower as a discarded expression");
    };
    assert_eq!(expr.ty, CoreType::int());
    assert!(matches!(expr.kind, CoreExprKind::Local(ref name) if name == "value"));
    assert!(matches!(main.body[1].kind, CoreStmtKind::Return { .. }));

    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    assert!(matches!(main.body[0].kind, IrStmtKind::Expr { .. }));
}

#[test]
fn record_let_pattern_binds_field_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: {count: Int, label: String}) -> Int effects []\n",
            "  let {count: amount}: {count: Int, label: String} = value\n",
            "  amount\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    assert_eq!(main.body.len(), 3);
    let CoreStmtKind::Let { name, expr, .. } = &main.body[1].kind else {
        panic!("record field binding should lower as a let statement");
    };
    assert_eq!(name, "amount");
    assert!(matches!(
        &expr.kind,
        CoreExprKind::FieldAccess { field, .. } if field == "count"
    ));
    assert_eq!(expr.ty, CoreType::int());
    let CoreStmtKind::Return { expr } = &main.body[2].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(&expr.kind, CoreExprKind::Local(name) if name == "amount"));
}

#[test]
fn refutable_let_pattern_reports_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Option(Int)) -> () effects []\n",
            "  let Some(amount) = value\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "pattern.refutable_let"
            && diagnostic.message == "refutable let pattern is not supported"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn match_expression_binds_constructor_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Option(Int)) -> Int effects []\n",
            "  match value\n",
            "    Some(count) => count + 1\n",
            "    None => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::int());
    assert!(matches!(expr.kind, CoreExprKind::Match { .. }));
    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(value.kind, IrExprKind::Match { .. }));
}

#[test]
fn match_expression_binds_qualified_constructor_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Result(Int, String)) -> Int effects []\n",
            "  match value\n",
            "    Result::Ok(count) => count + 1\n",
            "    Result::Err(_) => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::int());
    assert!(matches!(expr.kind, CoreExprKind::Match { .. }));
    assert!(lowered.ir.is_some());
}

#[test]
fn holes_build_blocked_core_but_not_executable_ir() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> Result((), AppError) effects []\n  _\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 1);
    assert_eq!(lowered.diagnostics[0].id, "hole.unfilled");
    let core = lowered.core.expect("partial checked core should be built");
    assert!(matches!(
        core.readiness,
        CoreReadiness::Blocked(ref blockers) if matches!(blockers.as_slice(), [CoreBlocker::Hole { .. }])
    ));
    assert!(lowered.ir.is_none());
}

#[test]
fn semantic_errors_block_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> Int effects []\n  \"no\"\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch")
    );
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}
