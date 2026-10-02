use super::*;
use crate::semantic_model::Type;

fn diagnostics(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn callsite_modifier_introduces_source_location_binding() {
    let diagnostics = diagnostics(concat!(
        "pub fn start_line() -> Int callsite\n",
        "  callsite.start_line\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_return_inference_sees_the_callsite_binding() {
    let diagnostics = diagnostics("fn start_line() callsite\n  callsite.start_line\nend\n");

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn callsite_contract_references_lower_for_execution() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn location() -> SourceLocation callsite\n",
            "require callsite.start_line > 0\n",
            "invariant callsite.start_column > 0\n",
            "ensure callsite.end_line > 0\n",
            "  callsite\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);

    let lowered = lower_project_reachable_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(matches!(
        lowered.core.as_ref().expect("checked core").readiness,
        CoreReadiness::Complete
    ));
    assert!(lowered.ir.is_some());
}

#[test]
fn callsite_aware_contract_calls_lower_when_the_enclosing_function_has_callsite_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn located() -> Bool callsite\n",
            "  callsite.start_line > 0\n",
            "end\n",
            "pub fn guarded() -> () callsite\n",
            "require located()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);

    let lowered = lower_project_reachable_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(matches!(
        lowered.core.as_ref().expect("checked core").readiness,
        CoreReadiness::Complete
    ));
    let guarded = lowered
        .ir
        .as_ref()
        .expect("typed IR")
        .functions
        .iter()
        .find(|function| function.name == "guarded")
        .expect("guarded function");
    assert_eq!(guarded.contracts[0].callsite_callees, ["located"]);
}

#[test]
fn ordinary_function_contract_calls_that_need_callsite_context_remain_blocked() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn located() -> Bool callsite\n",
            "  callsite.start_line > 0\n",
            "end\n",
            "pub fn main() -> ()\n",
            "require located()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);

    let lowered = lower_project_reachable_surface_module(&lower_surface_ast(&parsed.tree));
    let blockers = match &lowered.core.as_ref().expect("checked core").readiness {
        CoreReadiness::Blocked(blockers) => blockers,
        CoreReadiness::Complete => panic!("ordinary contracts cannot construct call-site context"),
    };
    assert!(blockers.iter().any(|blocker| matches!(
        blocker,
        CoreBlocker::UnsupportedExpression { reason, .. }
            if reason == "callsite_contract_call_unsupported"
    )));

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "core.callsite_contract_call_unsupported")
        .expect("call-site contract call diagnostic");
    assert_diagnostic_span(diagnostic, 5, 9, 5, 16);
    assert!(diagnostic.message.contains("`located`"));
    assert!(diagnostic.related.iter().any(|related| {
        related
            .to_json()
            .contains("Only a call-site-aware enclosing function has hidden context")
    }));
    assert!(lowered.ir.is_none());
}

#[test]
fn ordinary_callsite_parameter_lowers_in_contract_and_body() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn identity(callsite: Int) -> Int\n",
            "require callsite > 0\n",
            "  callsite\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  identity(42)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);

    let lowered = lower_project_reachable_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(matches!(
        lowered.core.as_ref().expect("checked core").readiness,
        CoreReadiness::Complete
    ));
    assert!(lowered.ir.is_some());
}

#[test]
fn unresolved_callsite_reference_suggests_the_modifier() {
    let diagnostics = diagnostics(concat!(
        "pub fn location() -> SourceLocation\n",
        "  callsite\n",
        "end\n",
        "pub fn guarded() -> ()\n",
        "require callsite\n",
        "  ()\n",
        "end\n",
    ));

    let missing_modifiers = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.callsite_requires_modifier")
        .collect::<Vec<_>>();
    assert_eq!(missing_modifiers.len(), 2, "{diagnostics:#?}");
    assert_diagnostic_span(missing_modifiers[0], 2, 3, 2, 11);
    assert_diagnostic_span(missing_modifiers[1], 5, 9, 5, 17);
    assert!(missing_modifiers.iter().all(|diagnostic| {
        diagnostic.related.iter().any(|related| {
            related
                .to_json()
                .contains("Add `callsite` after the function's optional effects clause.")
        })
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn contract_missing_modifier_points_to_the_unresolved_value_reference() {
    let diagnostics = diagnostics(concat!(
        "fn guarded(record: { callsite: Bool }) -> ()\n",
        "require record.callsite and callsite.start_line > 0\n",
        "  ()\n",
        "end\n",
    ));

    let missing_modifier = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.callsite_requires_modifier")
        .expect("missing callsite modifier diagnostic");
    assert_diagnostic_span(missing_modifier, 2, 29, 2, 37);
}

#[test]
fn missing_modifier_is_the_only_actionable_private_inference_diagnostic() {
    let diagnostics = diagnostics(concat!(
        "fn direct()\n",
        "  callsite\n",
        "end\n",
        "fn through_local()\n",
        "  let location = callsite\n",
        "  location\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.callsite_requires_modifier")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| !matches!(
            diagnostic.id.as_str(),
            "type.private_inference_incomplete" | "type.local_inference_incomplete"
        )),
        "{diagnostics:#?}"
    );
}

#[test]
fn missing_modifier_does_not_hide_independent_private_inference_failure() {
    let diagnostics = diagnostics(concat!(
        "fn independent()\n",
        "  let ignored = callsite\n",
        "  []\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.callsite_requires_modifier")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.private_inference_incomplete")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn unavailable_callsite_modifiers_keep_the_ordinary_unresolved_name_diagnostic() {
    let diagnostics = diagnostics(concat!(
        "test missing_modifier() -> ()\n",
        "  callsite\n",
        "end\n",
        "effect Locate\n",
        "  current() -> SourceLocation\n",
        "end\n",
        "handler locate() handles Locate\n",
        "  current() => callsite\n",
        "end\n",
    ));

    let mut unresolved = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .collect::<Vec<_>>();
    unresolved.sort_by_key(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line));
    assert_eq!(unresolved.len(), 2, "{diagnostics:#?}");
    assert_diagnostic_span(unresolved[0], 2, 3, 2, 11);
    assert_diagnostic_span(unresolved[1], 8, 16, 8, 24);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.callsite_requires_modifier"),
        "{diagnostics:#?}"
    );
}

#[test]
fn callsite_bindings_cannot_shadow_the_builtin() {
    let diagnostics = diagnostics(concat!(
        "fn parameter(callsite: Int) -> Int callsite\n",
        "  0\n",
        "end\n",
        "fn result() -> callsite: Int callsite\n",
        "  0\n",
        "end\n",
        "fn local() -> Int callsite\n",
        "  let callsite: Int = 1\n",
        "  0\n",
        "end\n",
        "fn pattern() -> Int callsite\n",
        "  match true\n",
        "    callsite => 0\n",
        "  end\n",
        "end\n",
    ));

    let shadows = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.callsite_shadow")
        .collect::<Vec<_>>();
    assert_eq!(shadows.len(), 4, "{diagnostics:#?}");
    assert_diagnostic_span(shadows[0], 1, 14, 1, 22);
    assert_diagnostic_span(shadows[1], 4, 16, 4, 24);
    assert_diagnostic_span(shadows[2], 8, 7, 8, 15);
    assert_diagnostic_span(shadows[3], 13, 5, 13, 13);
    assert!(shadows.iter().all(|diagnostic| {
        diagnostic
            .related
            .iter()
            .any(|related| related.to_json().contains("Rename this binding"))
    }));
}

#[test]
fn rejected_result_binding_does_not_override_builtin_in_contracts() {
    let diagnostics = diagnostics(concat!(
        "fn result() -> callsite: Int callsite\n",
        "ensure callsite\n",
        "  0\n",
        "end\n",
    ));

    let contract = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "contract.type_mismatch")
        .expect("contract type mismatch");
    let contract_json = contract.details.to_json();
    assert!(
        contract_json
            .contains("\"referenced_bindings\":[{\"name\":\"callsite\",\"kind\":\"local\"}]")
    );
    assert!(!contract_json.contains("\"kind\":\"result\""));

    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("predicate type mismatch");
    assert_eq!(
        mismatch.message,
        format!(
            "expected `Bool`, but found `{}`",
            Type::source_location().render()
        )
    );
}

#[test]
fn rejected_body_bindings_do_not_override_builtin_during_private_return_inference() {
    let diagnostics = diagnostics(concat!(
        "fn local() callsite\n",
        "  let callsite: Int = 1\n",
        "  callsite\n",
        "end\n",
        "fn pattern() callsite\n",
        "  match true\n",
        "    callsite => callsite\n",
        "  end\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.callsite_shadow")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn rejected_satisfy_candidate_does_not_override_builtin_in_predicate() {
    let diagnostics = diagnostics(concat!(
        "fn choose() -> SourceLocation callsite\n",
        "  _value satisfy callsite => callsite\n",
        "end\n",
    ));

    let shadow = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.callsite_shadow")
        .expect("callsite shadow diagnostic");
    assert_eq!(
        shadow.message,
        "satisfy candidate `callsite` shadows the built-in call-site location"
    );
    assert_diagnostic_span(shadow, 2, 18, 2, 26);
    assert!(shadow.related.iter().any(|related| {
        related
            .to_json()
            .contains("The `callsite` modifier introduces the built-in binding here.")
    }));
    assert!(shadow.related.iter().any(|related| {
        related
            .to_json()
            .contains("Rename this binding so the built-in `callsite` value remains visible.")
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "hole.satisfy_candidate_shadow"),
        "{diagnostics:#?}"
    );

    let predicate_type = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "hole.satisfy_type_mismatch")
        .expect("predicate type diagnostic");
    assert!(
        predicate_type
            .details
            .to_json()
            .contains(&Type::source_location().render()),
        "{predicate_type:#?}"
    );
}

#[test]
fn source_location_is_the_exact_standard_record_shape() {
    let diagnostics = diagnostics(concat!(
        "pub fn fields(value: SourceLocation) -> Int\n",
        "  value.start_line + value.start_column + value.start_offset + value.end_line + value.end_column + value.end_offset\n",
        "end\n",
        "pub fn package_name(value: SourceLocation) -> String\n",
        "  value.package\n",
        "end\n",
        "pub fn module_name(value: SourceLocation) -> String\n",
        "  value.module\n",
        "end\n",
        "pub fn file_name(value: SourceLocation) -> String\n",
        "  value.file\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}
