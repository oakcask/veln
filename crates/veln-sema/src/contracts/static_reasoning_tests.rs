use super::*;

fn exhaustive_case_split_predicate(subject: &str, fields: &[&str]) -> String {
    let assignment_count = 1usize << fields.len();
    (0..assignment_count)
        .map(|assignment| {
            let conjuncts = fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let bit = 1usize << (fields.len() - index - 1);
                    if assignment & bit != 0 {
                        format!("{subject}.{field}")
                    } else {
                        format!("not {subject}.{field}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" and ");
            format!("({conjuncts})")
        })
        .collect::<Vec<_>>()
        .join(" or ")
}

#[test]
fn negation_prefix_does_not_capture_following_conjunction() {
    let predicate = "(not value.ready and value.paid) or (value.ready and value.paid)";

    assert!(!has_complementary_top_level_clauses(predicate, "or"));
    assert!(!predicate_is_statically_true(predicate));
}

#[test]
fn small_boolean_truth_table_proves_nested_tautology() {
    let predicate = "not (value.ready and not extra) or not (not value.ready and not extra)";
    let mut atoms = Vec::new();
    collect_boolean_formula_atoms(predicate, &mut atoms);

    assert_eq!(atoms, vec!["value.ready".to_string(), "extra".to_string()]);
    assert_eq!(eval_boolean_formula(predicate, &atoms, 0), Some(true));
    assert_eq!(eval_boolean_formula(predicate, &atoms, 1), Some(true));
    assert_eq!(eval_boolean_formula(predicate, &atoms, 2), Some(true));
    assert_eq!(eval_boolean_formula(predicate, &atoms, 3), Some(true));
    assert_eq!(
        static_boolean_truth_table_value(predicate),
        Some(StaticBooleanValue::True)
    );
    assert!(predicate_is_statically_true(predicate));
}

#[test]
fn static_contract_reasoning_evaluates_integer_bitwise_expressions() {
    for predicate in [
        "(~0 & 255) == 255",
        "(1 << 63) == -9223372036854775808",
        "(-8 >> 2) == -2",
        "(-1 >>> 1) == 9223372036854775807",
        "(6 | 3) == 7 and (6 ^ 3) == 5",
    ] {
        assert!(predicate_is_statically_true(predicate), "{predicate}");
    }
    assert!(!predicate_is_statically_true("1 << 64 == 1"));
}

#[test]
fn static_numeric_expression_preserves_operator_precedence_and_associativity() {
    for predicate in [
        "1 + 2 * 3 == 7",
        "(1 + 2) * 3 == 9",
        "8 / 2 / 2 == 2",
        "8 >> 1 + 1 == 2",
        "1 | 6 ^ 3 & 5 == 7",
        "~1 + 3 == 1",
    ] {
        assert!(predicate_is_statically_true(predicate), "{predicate}");
    }
}

#[test]
fn high_arity_exhaustive_case_splits_are_statically_true() {
    for fields in [
        &["a", "b", "c", "d", "e", "f", "g", "h", "i"][..],
        &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"][..],
        &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"][..],
    ] {
        let predicate = exhaustive_case_split_predicate("value", fields);

        assert!(predicate_is_statically_true(&predicate));
    }
}

#[test]
fn boolean_formula_comparison_proves_commutative_conjunction() {
    assert_eq!(
        static_boolean_formula_comparison(
            "(value.ready and value.paid)",
            "==",
            "(value.paid and value.ready)",
        ),
        Some(true)
    );
    assert_eq!(
        split_top_level_operator(
            "(value.ready and value.paid) ==(value.paid and value.ready)",
            "==",
        ),
        Some((
            "(value.ready and value.paid)",
            "(value.paid and value.ready)"
        ))
    );
    assert!(predicate_is_statically_true(
        "(value.ready and value.paid) ==(value.paid and value.ready)"
    ));
}

#[test]
fn contract_static_truth_classifies_disjoint_literal_bounds() {
    for predicate in [
        "not (value > 10 and value < 5)",
        "not (10 < value and value < 10)",
        "not (1 + 1 <= value and value < 2)",
        "not (2 > value and value >= 2)",
    ] {
        assert!(
            contract_predicate_is_statically_true(predicate),
            "{predicate}"
        );
    }
}

#[test]
fn contract_static_truth_classifies_covering_literal_bounds() {
    for predicate in [
        "value <= 10 or value >= 5",
        "value < 10 or 5 <= value",
        "1 + 1 >= value or value >= 2",
        "value > 2 or value <= 2",
    ] {
        assert!(
            contract_predicate_is_statically_true(predicate),
            "{predicate}"
        );
    }
}

#[test]
fn general_static_truth_leaves_literal_bound_shapes_unknown() {
    assert!(!predicate_is_statically_true(
        "not (value > 10 and value < 5)"
    ));
    assert!(!predicate_is_statically_false("value > 10 and value < 5"));
    assert!(!predicate_is_statically_true("value <= 10 or value >= 5"));
}

#[test]
fn contract_static_truth_classifies_exclusive_literal_equalities() {
    for predicate in [
        "not (value == \"ready\" and value == \"done\")",
        "not (1 == value and value == 2)",
        "not ((value.ready) == true and false == value.ready)",
    ] {
        assert!(
            contract_predicate_is_statically_true(predicate),
            "{predicate}"
        );
    }
}

#[test]
fn repair_static_truth_classifies_exclusive_literal_equalities() {
    for predicate in [
        "not (value == \"ready\" and value == \"done\")",
        "not (1 == value and value == 2)",
        "not ((value.ready) == true and false == value.ready)",
    ] {
        assert!(
            predicate_is_statically_true_with_literal_bounds(predicate),
            "{predicate}"
        );
    }
}

#[test]
fn general_static_truth_leaves_literal_equality_shapes_unknown() {
    assert!(!predicate_is_statically_true(
        "not (value == \"ready\" and value == \"done\")"
    ));
    assert!(!predicate_is_statically_false(
        "value == \"ready\" and value == \"done\""
    ));
}

#[test]
fn contract_static_truth_keeps_compatible_literal_equalities_runtime_checked() {
    for predicate in [
        "value == \"ready\" and value == \"ready\"",
        "value == \"ready\" and other == \"done\"",
    ] {
        assert!(
            !has_exclusive_literal_equalities_top_level_and(predicate),
            "{predicate}"
        );
        assert!(
            !contract_predicate_is_statically_true(&format!("not ({predicate})")),
            "{predicate}"
        );
    }
    assert!(!has_exclusive_literal_equalities_top_level_and(
        "\"ready\" == \"done\" and value == \"ready\""
    ));
}

#[test]
fn contract_static_truth_keeps_overlapping_literal_bounds_runtime_checked() {
    assert!(!has_exclusive_numeric_literal_bounds_top_level_and(
        "value >= 5 and value <= 5"
    ));
    assert!(!has_exclusive_inclusive_order_top_level_and(
        "value >= 5 and value <= 5"
    ));
    assert_eq!(
        static_boolean_truth_table_value("value >= 5 and value <= 5"),
        Some(StaticBooleanValue::Unknown)
    );
    assert_eq!(static_comparison_value("value >= 5"), None);
    assert_eq!(static_comparison_value("value <= 5"), None);
    assert!(!complementary_predicates("value >= 5", "value <= 5"));
    assert_eq!(
        static_boolean_value_inner("value >= 5 and value <= 5", true, true),
        StaticBooleanValue::Unknown
    );
    for predicate in [
        "not (value > 5 and value < 10)",
        "not (value >= 5 and value <= 5)",
        "not (value > 5 and other < 5)",
        "value < 5 or value > 5",
        "value <= 5 or other >= 5",
    ] {
        assert!(
            !contract_predicate_is_statically_true(predicate),
            "{predicate}"
        );
    }
}

#[test]
fn contract_static_truth_classifies_literal_bounds_excluding_disequality_values() {
    for predicate in [
        "not (value > 10) or (value != 10)",
        "not (value <= 1 / 2) or (value != 0.75)",
        "not (value == alias and alias < 20) or (value != 20)",
    ] {
        assert!(
            contract_predicate_is_statically_true(predicate),
            "{predicate}"
        );
    }
}

#[test]
fn contract_static_truth_keeps_possible_bound_endpoint_disequality_runtime_checked() {
    for predicate in [
        "not (value >= 10) or (value != 10)",
        "not (value <= 20) or (value != 20)",
        "not (value == alias and alias > 5) or (value != 6)",
    ] {
        assert!(
            !contract_predicate_is_statically_true(predicate),
            "{predicate}"
        );
    }
}
