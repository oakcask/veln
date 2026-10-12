use super::*;

fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn remainder_is_int_only_even_in_float_contexts() {
    let diagnostics = diagnostics_for(concat!(
        "fn main() -> Float\n",
        "  let a: Int = 7 % 3\n",
        "  let b = 7.0 % 3\n",
        "  let c = 7 % 3.0\n",
        "  let d = true % 2\n",
        "  let e = 7 % \"3\"\n",
        "  7 % 3\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        5,
        "{diagnostics:#?}"
    );
}

#[test]
fn literal_zero_remainder_reports_divisor_and_operation() {
    for divisor in ["0", "(-0)", "0x0", "0b0"] {
        let diagnostics = diagnostics_for(&format!("fn main() -> Int\n  7 % {divisor}\nend\n"));
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "type.remainder_by_zero")
            .expect("literal zero should be rejected");
        assert_eq!(diagnostic.message.as_str(), "remainder divisor is zero");
        assert_eq!(diagnostic.related.len(), 1);
    }
}

#[test]
fn computed_zero_remainder_remains_runtime_checked() {
    let diagnostics = diagnostics_for("fn main() -> Int\n  7 % (1 - 1)\nend\n");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.remainder_by_zero"),
        "{diagnostics:#?}"
    );
}

#[test]
fn contract_and_satisfy_remainder_are_int_only() {
    let diagnostics = diagnostics_for(concat!(
        "fn checked(value: Int, decimal: Float, flag: Bool, text: String) -> output: Int\n",
        "require value % 2 == 0\n",
        "require decimal % 2 == 0\n",
        "require value % 2.0 == 0\n",
        "ensure flag % 2 == 0\n",
        "invariant value % text == 0\n",
        "  _value satisfy candidate => candidate % 3 == 1\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        4,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "hole.satisfy_type_mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn nonliteral_remainder_contracts_remain_runtime_required() {
    let source = SourceFile::new(
        "main.veln",
        "fn even(value: Int) -> Int\nrequire value % 2 == 0\n  value\nend\n",
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let contract = &lowered.core.expect("core program").functions[0].contracts[0];
    assert_eq!(
        contract.obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}

#[test]
fn nonliteral_remainder_does_not_create_modular_repairs() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(fallback: Int) -> Int\n",
            "require fallback >= 0\n",
            "  let unresolved: Int = _value satisfy candidate => candidate % 3 == 1\n",
            "  _other satisfy candidate => candidate >= 0\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    let holes = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "hole.unfilled")
        .collect::<Vec<_>>();
    assert_eq!(holes.len(), 2, "{diagnostics:#?}");
    let unresolved = holes[0].details.to_json();
    let independent = holes[1].details.to_json();

    assert!(
        !unresolved.contains("safe_repair_candidate"),
        "{unresolved}"
    );
    assert!(!unresolved.contains("statically_satisfied"), "{unresolved}");
    assert!(
        independent.contains("safe_repair_candidate"),
        "{independent}"
    );
}

#[test]
fn literal_zero_remainder_in_contract_and_satisfy_reports_type_diagnostic() {
    let diagnostics = diagnostics_for(concat!(
        "fn checked(value: Int) -> Int\n",
        "require value % 0 == 0\n",
        "  _value satisfy candidate => candidate % (-0) == 0\n",
        "end\n",
    ));
    let remainder_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.remainder_by_zero")
        .collect::<Vec<_>>();

    assert_eq!(remainder_diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(remainder_diagnostics.iter().all(|diagnostic| {
        diagnostic.message == "remainder divisor is zero" && diagnostic.related.len() == 1
    }));
    let require_span = remainder_diagnostics[0]
        .span
        .as_ref()
        .expect("require diagnostic span");
    assert_eq!(
        (
            require_span.start.line,
            require_span.start.column,
            require_span.end.column,
        ),
        (2, 17, 18),
    );
}

#[test]
fn literal_remainder_contracts_are_statically_proven_in_each_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn checked() -> output: Int\n",
            "require -7 % 3 == -1\n",
            "ensure 7 % -3 == 1\n",
            "invariant (-9223372036854775807 - 1) % -1 == 0\n",
            "  0\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("core program");
    assert!(core.functions[0].contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}
