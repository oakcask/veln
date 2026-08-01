use super::*;

#[test]
fn contract_predicate_equality_path_does_not_imply_strict_bound() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int\n",
            "require not (low == mid and mid <= high) or low < high\n",
            "ensure not (output == mid and mid == high) or output < high\n",
            "  low\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::RuntimeRequired
    }));
}

#[test]
fn contract_predicate_reflexive_equality_does_not_create_order_path() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, high: Int) -> output: Int\n",
            "require not (low == low and high == high) or low <= high\n",
            "ensure not (output == output and high == high) or output <= high\n",
            "  low\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::RuntimeRequired
    }));
}

#[test]
fn contract_predicate_negated_complementary_and_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}) -> output: {ready: Bool}\n",
            "require not (value.ready and not value.ready)\n",
            "ensure not((output.ready) and not(output.ready))\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_negated_complementary_comparison_and_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, limit: Int) -> output: Int\n",
            "require not (value == limit and limit != value)\n",
            "ensure not(output < limit and output >= limit)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_nested_negated_complementary_and_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require not (value.ready and (extra and not value.ready))\n",
            "ensure not((output.ready) and (extra and not(output.ready)))\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_negated_multi_branch_complementary_and_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require not (value.ready and extra and not value.ready)\n",
            "ensure not((output.ready) and extra and not(output.ready))\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn satisfy_predicate_ignores_names_inside_string_literals() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: String) -> String\n",
            "  _value satisfy candidate => candidate == \"missing_call(value)\"\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.unfilled" && diagnostic.kind == DiagnosticKind::Hole
    }));
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_unresolved_name"
            && diagnostic.message.contains("missing_call")
    }));
}

#[test]
fn contract_predicate_rejects_non_numeric_call_in_arithmetic_comparison() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn label(value: Int) -> String\n",
            "  \"item\"\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
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
            "pub fn identity(value: Int) -> Int\n",
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
            "fn next(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
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
            "pub fn identity(value: Int) -> Int\n",
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
            "pub fn identity(value: Int) -> Int\n",
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
fn contract_predicate_rejects_perform_as_effectful_operation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user: String) -> Bool\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
            "require perform Audit::record(\"contract\")\n",
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
            "fn same(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
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
            "fn same(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
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
            "pub fn identity(value: {total: Int}) -> output: {total: Int}\n",
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
            "fn summary(value: Int) -> {total: Int}\n",
            "  {total: value}\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
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
            "pub fn main() -> output: Int\n",
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
fn invariant_cannot_reference_result_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> output: Int\n",
            "invariant output > 0\n",
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
            "pub fn identity(value: Int) -> Int\n",
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
        concat!("pub fn main() -> output: Int\n", "  output\n", "end\n",),
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
            "pub fn main(output: Int) -> output: Int\n",
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
            "pub fn default_port(max: Int) -> Int\n",
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
    assert!(details.contains("\"repair_status\":\"statically_satisfied\""));
    assert_eq!(diagnostics[0].related.len(), 3);
}

#[test]
fn hole_diagnostic_keeps_undischarged_satisfy_constraint_blocked() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn default_port(max: Int, fallback: Int) -> Int\n",
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
    assert!(details.contains("\"repair_status\":\"blocked_until_discharged\""));
    assert!(!details.contains("\"repair_status\":\"statically_satisfied\""));
}
