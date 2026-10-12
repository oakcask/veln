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
fn invalid_contract_and_satisfy_remainder_operands_report_at_each_operand() {
    let invalid = [("1.0", "Float"), ("true", "Bool"), ("\"bad\"", "String")];
    for (clause, line, left_column, right_column) in [
        ("require", 2, 9, 17),
        ("ensure", 2, 8, 16),
        ("invariant", 2, 11, 19),
    ] {
        for (literal, actual_type) in invalid {
            for (left, expected_column) in [(true, left_column), (false, right_column)] {
                let predicate = if left {
                    format!("{literal} % value == 0")
                } else {
                    format!("value % {literal} == 0")
                };
                let source = format!(
                    "fn checked(value: Int) -> output: Int\n{clause} {predicate}\n  value\nend\n"
                );
                let diagnostics = diagnostics_for(&source);
                let mismatches = diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.id == "type.mismatch")
                    .collect::<Vec<_>>();
                assert_eq!(mismatches.len(), 1, "{source}\n{diagnostics:#?}");
                let mismatch = mismatches[0];
                assert_eq!(mismatch.kind, DiagnosticKind::Type);
                assert_eq!(
                    mismatch.message,
                    format!("expected `Int`, but found `{actual_type}`")
                );
                let span = mismatch.span.as_ref().expect("operand span");
                assert_eq!(
                    (span.start.line, span.start.column, span.end.column),
                    (line, expected_column, expected_column + literal.len())
                );
                let details = mismatch.details.to_json();
                for expected in [
                    "\"phase\":\"type\"",
                    "\"expected_type\":\"Int\"",
                    &format!("\"actual_type\":\"{actual_type}\""),
                    "\"expected_type_source\":\"inferred_expression\"",
                    "\"actual_type_source\":\"inferred_expression\"",
                    "\"constraint\":\"operator_operand\"",
                ] {
                    assert!(details.contains(expected), "{details}");
                }
                assert!(diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "contract.type_mismatch"
                        && diagnostic.message != "expected `Bool`, but found `unknown`"
                }));
            }
        }
    }

    for (literal, actual_type) in invalid {
        for left in [true, false] {
            let predicate = if left {
                format!("{literal} % candidate == 0")
            } else {
                format!("candidate % {literal} == 0")
            };
            let source = format!(
                "fn checked(value: Int) -> Int\n  let result: Int = _value satisfy candidate => {predicate}\n  result\nend\n"
            );
            let diagnostics = diagnostics_for(&source);
            let mismatch = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.id == "type.mismatch")
                .unwrap_or_else(|| panic!("missing operand mismatch: {diagnostics:#?}"));
            assert_eq!(
                mismatch.message,
                format!("expected `Int`, but found `{actual_type}`")
            );
            let span = mismatch.span.as_ref().expect("satisfy operand span");
            let expected_offset = source.find(literal).expect("literal in source");
            assert_eq!(
                (span.start.line, span.start.offset, span.end.offset),
                (2, expected_offset, expected_offset + literal.len())
            );
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.id != "hole.satisfy_type_mismatch"),
                "{diagnostics:#?}"
            );
        }
    }

    let diagnostics = diagnostics_for(concat!(
        "fn checked() -> Int\n",
        "require true % \"bad\" == 0\n",
        "  0\n",
        "end\n",
    ));
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        2,
        "{diagnostics:#?}"
    );
}

#[test]
fn float_zero_remainder_divisors_only_report_operand_mismatches() {
    for (clause, line, column) in [("require", 2, 17), ("ensure", 2, 16), ("invariant", 2, 19)] {
        let source = format!(
            "fn checked(value: Int) -> output: Int\n{clause} value % 0.0 == 0\n  value\nend\n"
        );
        let diagnostics = diagnostics_for(&source);
        assert_eq!(diagnostics.len(), 1, "{source}\n{diagnostics:#?}");
        let mismatch = &diagnostics[0];
        assert_eq!(mismatch.id, "type.mismatch");
        let span = mismatch.span.as_ref().expect("operand span");
        assert_eq!(
            (span.start.line, span.start.column, span.end.column),
            (line, column, column + 3)
        );
        let details = mismatch.details.to_json();
        for expected in [
            "\"phase\":\"type\"",
            "\"expected_type\":\"Int\"",
            "\"actual_type\":\"Float\"",
            "\"expected_type_source\":\"inferred_expression\"",
            "\"actual_type_source\":\"inferred_expression\"",
            "\"constraint\":\"operator_operand\"",
        ] {
            assert!(details.contains(expected), "{details}");
        }
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "type.remainder_by_zero"),
            "{diagnostics:#?}"
        );
    }

    let source = concat!(
        "fn checked(value: Int) -> Int\n",
        "  let result: Int = _value satisfy candidate => candidate % 0.0 == 0\n",
        "  result\n",
        "end\n",
    );
    let diagnostics = diagnostics_for(source);
    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    let mismatch = mismatches[0];
    let span = mismatch.span.as_ref().expect("satisfy operand span");
    let expected_offset = source.find("0.0").expect("float zero in source");
    assert_eq!(
        (span.start.line, span.start.offset, span.end.offset),
        (2, expected_offset, expected_offset + 3)
    );
    let details = mismatch.details.to_json();
    for expected in [
        "\"phase\":\"type\"",
        "\"expected_type\":\"Int\"",
        "\"actual_type\":\"Float\"",
        "\"expected_type_source\":\"inferred_expression\"",
        "\"actual_type_source\":\"inferred_expression\"",
        "\"constraint\":\"operator_operand\"",
    ] {
        assert!(details.contains(expected), "{details}");
    }
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.remainder_by_zero"),
        "{diagnostics:#?}"
    );
}

#[test]
fn integer_zero_remainder_is_reported_when_the_left_operand_is_invalid() {
    let diagnostics = diagnostics_for(concat!(
        "fn checked() -> Int\n",
        "require true % 0 == 0\n",
        "  0\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.remainder_by_zero")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn nonliteral_remainder_contracts_remain_runtime_required() {
    for predicate in [
        "value % 2 == 0",
        "value % divisor == value % divisor",
        "value % divisor == 0 or value % divisor != 0",
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "fn checked(value: Int, divisor: Int) -> Int\nrequire {predicate}\n  value\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        let lowered = lower_checked_surface_module(&module);

        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        let contract = &lowered.core.expect("core program").functions[0].contracts[0];
        assert_eq!(
            contract.obligation_status,
            ContractObligationStatus::RuntimeRequired,
            "{predicate}"
        );
    }
}

#[test]
fn nonliteral_remainder_does_not_create_modular_repairs() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(fallback: Int, divisor: Int) -> Int\n",
            "require fallback >= 0\n",
            "  let unresolved: Int = _value satisfy candidate => candidate % 3 == 1\n",
            "  let reflexive: Int = _reflexive satisfy candidate => candidate % divisor == candidate % divisor\n",
            "  let exhaustive: Int = _exhaustive satisfy candidate => candidate % divisor == 0 or candidate % divisor != 0\n",
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
    assert_eq!(holes.len(), 4, "{diagnostics:#?}");

    for hole in &holes[..3] {
        let unresolved = hole.details.to_json();
        assert!(
            !unresolved.contains("safe_repair_candidate"),
            "{unresolved}"
        );
        assert!(!unresolved.contains("statically_satisfied"), "{unresolved}");
    }
    let independent = holes[3].details.to_json();
    assert!(
        independent.contains("safe_repair_candidate"),
        "{independent}"
    );
}

#[test]
fn every_literal_zero_remainder_in_one_predicate_reports_its_operation() {
    let source = concat!(
        "fn checked(value: Int, other: Int) -> Int\n",
        "require value % 0x0 == 0 and other % -0 == 0\n",
        "  _value satisfy candidate => candidate % 0b0 == 0 and candidate % 000 == 0\n",
        "end\n",
    );
    let diagnostics = diagnostics_for(source);
    let remainder_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.remainder_by_zero")
        .collect::<Vec<_>>();
    assert_eq!(remainder_diagnostics.len(), 4, "{diagnostics:#?}");

    for (diagnostic, (literal, operation)) in remainder_diagnostics.iter().zip([
        ("0x0", "value % 0x0"),
        ("-0", "other % -0"),
        ("0b0", "candidate % 0b0"),
        ("000", "candidate % 000"),
    ]) {
        let span = diagnostic.span.as_ref().expect("divisor span");
        let literal_start = source.find(literal).expect("literal in source");
        assert_eq!(
            (span.start.offset, span.end.offset),
            (literal_start, literal_start + literal.len())
        );
        assert_eq!(diagnostic.related.len(), 1);
        let operation_start = source.find(operation).expect("operation in source");
        let related = diagnostic.related[0].to_json();
        assert!(
            related.contains(&format!("\"offset\":{operation_start}")),
            "{related}"
        );
        assert!(
            related.contains(&format!("\"offset\":{}", operation_start + operation.len())),
            "{related}"
        );
    }
}

#[test]
fn literal_zero_remainder_in_contract_and_satisfy_reports_type_diagnostic() {
    let source = concat!(
        "fn checked(value: Int) -> Int\n",
        "require value %     0    == 0\n",
        "  _value satisfy candidate => candidate %     (-0)    == 0\n",
        "end\n",
    );
    let diagnostics = diagnostics_for(source);
    let remainder_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.remainder_by_zero")
        .collect::<Vec<_>>();

    assert_eq!(remainder_diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(remainder_diagnostics.iter().all(|diagnostic| {
        diagnostic.message == "remainder divisor is zero" && diagnostic.related.len() == 1
    }));
    let require = remainder_diagnostics[0];
    let require_span = require.span.as_ref().expect("require diagnostic span");
    assert_eq!(
        (
            require_span.start.line,
            require_span.start.column,
            require_span.end.column,
        ),
        (2, 21, 22),
    );
    let require_related = require.related[0].to_json();
    assert!(require_related.contains("\"start\":{\"line\":2,\"column\":9"));
    assert!(require_related.contains("\"end\":{\"line\":2,\"column\":22"));

    let satisfy = remainder_diagnostics[1];
    let satisfy_span = satisfy.span.as_ref().expect("satisfy diagnostic span");
    assert_eq!(
        (
            satisfy_span.start.line,
            satisfy_span.start.column,
            satisfy_span.end.column,
        ),
        (3, 47, 51),
    );
    let satisfy_related = satisfy.related[0].to_json();
    assert!(satisfy_related.contains("\"start\":{\"line\":3,\"column\":31"));
    assert!(satisfy_related.contains("\"end\":{\"line\":3,\"column\":51"));
}

#[test]
fn mixed_literal_remainder_contracts_remain_runtime_required() {
    let predicates = [
        "5 % 3 * 2 == 5",
        "5 % (3 * 2) == 4",
        "1 % 2 / 2 == 0.5",
        "5 % (3 / 2) == 1",
        "20 % 6 % 4 == 0",
        "20 % (6 % 4) == 1",
    ];
    let source = format!(
        "fn checked() -> Int\n{}  0\nend\n",
        predicates
            .iter()
            .map(|predicate| format!("require {predicate}\n"))
            .collect::<String>()
    );
    let parsed = parse(&SourceFile::new("main.veln", &source));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.expect("core").functions[0]
        .contracts
        .iter()
        .all(|contract| contract.obligation_status == ContractObligationStatus::RuntimeRequired));
}

#[test]
fn literal_remainder_contracts_are_statically_proven_in_each_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn checked() -> output: Int\n",
            "require -7 % 3 == -1\n",
            "require 1 % 9223372036854775807 == 1\n",
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
