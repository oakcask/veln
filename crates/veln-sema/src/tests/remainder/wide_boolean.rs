use super::*;

fn repeated_boolean_clauses(clause: &str, count: usize, operator: &str) -> String {
    std::iter::repeat_n(clause, count)
        .collect::<Vec<_>>()
        .join(&format!(" {operator} "))
}

fn lower_remainder_boolean_predicate(predicate: &str) -> ContractObligationStatus {
    let source = SourceFile::new(
        "main.veln",
        format!("fn checked(value: Int) -> Int\nrequire {predicate}\n  value\nend\n"),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    lowered.core.expect("core program").functions[0].contracts[0].obligation_status
}

fn assert_proven(predicate: &str) {
    assert_eq!(
        lower_remainder_boolean_predicate(predicate),
        ContractObligationStatus::StaticallyProven
    );
}

#[test]
fn wide_literal_remainder_boolean_composition_is_stack_safe() {
    for operator in ["and", "or"] {
        for (clause, expected) in [
            ("7 % 3 == 1", ContractObligationStatus::StaticallyProven),
            ("7 % 3 == 0", ContractObligationStatus::RuntimeRequired),
        ] {
            let predicate = repeated_boolean_clauses(clause, 4_097, operator);
            assert_eq!(
                lower_remainder_boolean_predicate(&predicate),
                expected,
                "{operator}: {clause}"
            );
        }
    }
}

#[test]
fn wide_remainder_boolean_tails_preserve_final_obligation_status() {
    let tail = repeated_boolean_clauses("true", 4_096, "and");
    for predicate in [
        format!("7 % 3 == 1 and {tail}"),
        format!("{tail} and 7 % 3 == 1"),
    ] {
        assert_proven(&predicate);
    }

    let literal_tail = repeated_boolean_clauses("7 % 3 == 1", 4_096, "or");
    for predicate in [
        format!("value % 3 == 1 or {literal_tail}"),
        format!("{literal_tail} or value % 3 == 1"),
    ] {
        assert_eq!(
            lower_remainder_boolean_predicate(&predicate),
            ContractObligationStatus::RuntimeRequired
        );
    }

    for predicate in ["7 % 3 == 0", "value % 3 == 1"] {
        assert_eq!(
            lower_remainder_boolean_predicate(predicate),
            ContractObligationStatus::RuntimeRequired,
            "{predicate}"
        );
    }
}

fn assert_simple_wide_boolean_identities() {
    let conjunction = repeated_boolean_clauses("false", 513, "and");
    assert_proven(&format!("7 % 3 == 1 or {conjunction}"));

    let false_remainders = repeated_boolean_clauses("7 % 3 == 0", 511, "or");
    assert_proven(&format!("value >= 0 or value < 0 or {false_remainders}"));

    let false_remainders = repeated_boolean_clauses("7 % 3 == 0", 511, "or");
    assert_proven(&format!(
        concat!(
            "not (value >= 0 and not value < 0) or ",
            "not (not value >= 0 and not value < 0) or {}"
        ),
        false_remainders
    ));
}

fn assert_wide_partial_case_split() {
    let parameters = std::iter::once("value: Int".to_string())
        .chain(["b: Bool".to_string(), "c: Bool".to_string()])
        .chain((0..12).map(|index| format!("d{index}: Bool")))
        .collect::<Vec<_>>()
        .join(", ");
    let independent_tail = (0..12)
        .map(|index| format!("d{index}"))
        .collect::<Vec<_>>()
        .join(" or ");
    let false_remainders = repeated_boolean_clauses("7 % 3 == 0", 497, "or");
    let predicate = format!(
        concat!(
            "value < 0 or (value >= 0 and b) or ",
            "(value >= 0 and not b and c) or ",
            "(value >= 0 and not b and not c) or {} or {}"
        ),
        independent_tail, false_remainders
    );
    let source = SourceFile::new(
        "main.veln",
        format!("fn checked({parameters}) -> Int\nrequire {predicate}\n  value\nend\n"),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert_eq!(
        lowered.core.expect("core program").functions[0].contracts[0].obligation_status,
        ContractObligationStatus::StaticallyProven
    );
}

#[test]
fn wide_remainder_boolean_composition_preserves_precedence_and_identities() {
    assert_simple_wide_boolean_identities();
    assert_wide_partial_case_split();
}

fn classification_work(clause_count: usize) -> usize {
    let predicate = repeated_boolean_clauses("7 % 3 == 1", clause_count, "and");
    let source = SourceFile::new(
        "main.veln",
        format!("fn checked() -> Int\nrequire {predicate}\n  0\nend\n"),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    crate::contracts::reset_static_boolean_work();
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert_eq!(
        lowered.core.expect("core program").functions[0].contracts[0].obligation_status,
        ContractObligationStatus::StaticallyProven
    );
    crate::contracts::take_static_boolean_work()
}

#[test]
fn wide_literal_remainder_classification_work_is_proportional() {
    let at_2_048 = classification_work(2_048);
    let at_4_096 = classification_work(4_096);
    assert_eq!(at_2_048, 2_048);
    assert_eq!(at_4_096, 4_096);
}
