use super::*;

fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn flat_remainder_validation_types_each_leaf_once() {
    fn type_checks(operator_count: usize) -> usize {
        let predicate = std::iter::once("number()".to_string())
            .chain((0..operator_count).map(|_| "% number()".to_string()))
            .collect::<Vec<_>>()
            .join(" ");
        let source = SourceFile::new(
            "main.veln",
            format!(
                "fn number() -> Int\n  1\nend\nfn checked() -> Int\nrequire {predicate} == 0\n  0\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        let contract = &module.functions[1].contracts[0];
        let calls = std::cell::Cell::new(0usize);
        assert!(!crate::contracts::predicate_has_invalid_remainder_operands(
            &contract.remainder_spans,
            &[],
            &|_| {
                calls.set(calls.get() + 1);
                Some(crate::semantic_model::Type::int())
            },
        ));
        calls.get()
    }

    let at_500 = type_checks(500);
    let at_501 = type_checks(501);
    assert_eq!(at_500, 501);
    assert_eq!(at_501 - at_500, 1);
}

#[test]
fn flat_remainder_complete_pipeline_is_stack_safe_beyond_prior_failure() {
    fn lower_chain(operator_count: usize, literal: bool) -> ContractObligationStatus {
        let mut predicate = if literal {
            "7".to_string()
        } else {
            "value".to_string()
        };
        for _ in 0..operator_count {
            predicate.push_str(" % 3");
        }
        let expected = if literal { 1 } else { 0 };
        let source = SourceFile::new(
            "main.veln",
            format!(
                "fn checked(value: Int) -> Int\nrequire {predicate} == {expected}\n  value\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        lowered.core.expect("core program").functions[0].contracts[0].obligation_status
    }

    for operator_count in [4_096, 4_097] {
        assert_eq!(
            lower_chain(operator_count, false),
            ContractObligationStatus::RuntimeRequired
        );
        assert_eq!(
            lower_chain(operator_count, true),
            ContractObligationStatus::StaticallyProven
        );
    }
}

fn two_remainder_chains(operator_count: usize, literal: bool, operator: &str) -> String {
    let mut left = if literal { "7" } else { "left" }.to_string();
    let mut right = if literal { "8" } else { "right" }.to_string();
    for _ in 0..operator_count {
        left.push_str(" % 3");
        right.push_str(" % 3");
    }
    let (left_expected, right_expected) = if literal { (1, 2) } else { (0, 0) };
    format!(
        "fn checked(left: Int, right: Int) -> Int\nrequire ({left} == {left_expected}) {operator} ({right} == {right_expected})\n  left\nend\n"
    )
}

#[test]
fn independent_long_remainder_chains_complete_the_semantic_pipeline_iteratively() {
    for operator in ["and", "or"] {
        for literal in [false, true] {
            let source =
                SourceFile::new("main.veln", two_remainder_chains(4_097, literal, operator));
            let parsed = parse(&source);
            assert!(
                parsed.diagnostics.is_empty(),
                "{operator}: {:#?}",
                parsed.diagnostics
            );
            let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
            assert!(
                lowered.diagnostics.is_empty(),
                "{operator}: {:#?}",
                lowered.diagnostics
            );
            assert_eq!(
                lowered.core.expect("core program").functions[0].contracts[0].obligation_status,
                if literal {
                    ContractObligationStatus::StaticallyProven
                } else {
                    ContractObligationStatus::RuntimeRequired
                },
                "{operator} with literal={literal}"
            );
        }
    }
}

#[test]
fn independent_long_remainder_chains_support_comparison_sides_and_call_wrappers() {
    let mut left = String::from("left");
    let mut right = String::from("right");
    for _ in 0..4_097 {
        left.push_str(" % 3");
        right.push_str(" % 3");
    }
    for predicate in [
        format!("({left}) == ({right})"),
        format!("identity({left}) == identity({right})"),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "fn identity(value: Int) -> Int\n  value\nend\nfn checked(left: Int, right: Int) -> Int\nrequire {predicate}\n  left\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        assert_eq!(
            lowered.core.expect("core program").functions[1].contracts[0].obligation_status,
            ContractObligationStatus::RuntimeRequired
        );
    }
}

fn wrapped_remainder_contract(operator_count: usize, call_wrapper: bool) -> String {
    let mut chain = String::from("value");
    for _ in 0..operator_count {
        chain.push_str(" % 3");
    }
    let predicate = if call_wrapper {
        format!("identity(-({chain})) <= 0")
    } else {
        format!("-({chain}) <= 0")
    };
    format!(
        "fn identity(value: Int) -> Int\n  value\nend\nfn checked(value: Int) -> Int\nrequire {predicate}\n  value\nend\n"
    )
}

#[test]
fn wrapped_remainder_complete_pipeline_work_is_bounded() {
    fn typing_work(operator_count: usize) -> usize {
        let source = SourceFile::new(
            "main.veln",
            wrapped_remainder_contract(operator_count, false),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::contracts::reset_predicate_typing_work();
        let diagnostics = analyze_surface_module(&module);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        crate::contracts::take_predicate_typing_work()
    }

    let at_500 = typing_work(500);
    let at_501 = typing_work(501);
    eprintln!("wrapped remainder typing work at 500 and 501 operators: {at_500}, {at_501}");
    assert!(at_500 > 0, "the metric must observe predicate typing work");
    assert!(
        at_501 <= at_500 + 64,
        "one additional remainder must add bounded typing work: {at_500} -> {at_501}"
    );
}

#[test]
fn wrapped_remainder_complete_pipeline_is_stack_safe_beyond_review_failure() {
    for call_wrapper in [false, true] {
        let source = SourceFile::new("main.veln", wrapped_remainder_contract(4_097, call_wrapper));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        assert_eq!(
            lowered.core.expect("core program").functions[1].contracts[0].obligation_status,
            ContractObligationStatus::RuntimeRequired
        );
    }
}

#[test]
fn flat_remainder_satisfy_pipeline_is_stack_safe() {
    let operator_count = 4_096;
    let mut predicate = String::from("candidate");
    for _ in 0..operator_count {
        predicate.push_str(" % 3");
    }
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn checked(value: Int) -> Int\n  _value satisfy candidate => {predicate} == 1\nend\n"
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "hole.unfilled"),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "hole.satisfy_type_mismatch" && diagnostic.id != "type.mismatch"
    }));
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
                let mismatch =
                    super::remainder_assertions::assert_operand_mismatch(&diagnostics, actual_type);
                assert_eq!(mismatch.kind, DiagnosticKind::Type);
                let span = mismatch.span.as_ref().expect("operand span");
                assert_eq!(
                    (span.start.line, span.start.column, span.end.column),
                    (line, expected_column, expected_column + literal.len())
                );
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
            let mismatch =
                super::remainder_assertions::assert_operand_mismatch(&diagnostics, actual_type);
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
fn composite_remainder_operands_report_the_invalid_leaf_and_block_core() {
    let source = concat!(
        "fn is_zero(value: Int) -> Bool\n",
        "  value == 0\n",
        "end\n",
        "fn checked() -> output: Int\n",
        "require (true * 2) % 3 == 0\n",
        "ensure 9 % (2 * \"bad\") == 0\n",
        "invariant 9 % 2 % (1.5 * 2) == 0\n",
        "require is_zero(9 % (false * 2))\n",
        "  0\n",
        "end\n",
    );
    let parsed = parse(&SourceFile::new("main.veln", source));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    let mismatches = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();

    assert_eq!(mismatches.len(), 4, "{:#?}", lowered.diagnostics);
    for (diagnostic, (leaf, actual_type)) in mismatches.iter().zip([
        ("true", "Bool"),
        ("\"bad\"", "String"),
        ("1.5", "Float"),
        ("false", "Bool"),
    ]) {
        assert_eq!(
            diagnostic.message,
            format!("expected `Int`, but found `{actual_type}`")
        );
        let span = diagnostic.span.as_ref().expect("invalid leaf span");
        let start = source.find(leaf).expect("invalid leaf in source");
        assert_eq!(
            (span.start.offset, span.end.offset),
            (start, start + leaf.len())
        );
    }
    assert!(
        lowered.core.is_none(),
        "invalid remainder must not produce runnable core"
    );
    assert!(lowered.diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "contract.type_mismatch"
            && diagnostic.id != "contract.unsupported_construct"
    }));
}

#[test]
fn remainder_only_overrides_non_boolean_whole_call_diagnostic_precedence() {
    fn lower(predicate: &str) -> LoweredSurfaceModule {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "fn identity(value: Int) -> Int\n  value\nend\nfn checked(box: {{value: Int}}) -> Int\nrequire {predicate}\n  0\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        lower_checked_surface_module(&lower_surface_ast(&parsed.tree))
    }

    let invalid_remainder = lower("identity(true % 2)");
    assert!(invalid_remainder.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch" && diagnostic.message == "expected `Int`, but found `Bool`"
    }));
    assert!(invalid_remainder.diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "contract.type_mismatch"
            && diagnostic.id != "contract.unsupported_construct"
    }));
    assert!(invalid_remainder.core.is_none());

    let missing_remainder_field = lower("identity(box.missing % 2)");
    assert!(
        missing_remainder_field
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "contract.field_missing")
    );
    assert!(
        missing_remainder_field
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "contract.type_mismatch")
    );
    assert!(missing_remainder_field.core.is_none());

    for predicate in ["identity(true)", "identity(box.missing)"] {
        let no_remainder = lower(predicate);
        assert!(no_remainder.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "contract.type_mismatch"
                && diagnostic.message == "contract predicate is not `Bool`"
        }));
        assert!(no_remainder.diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "contract.field_missing"
                && diagnostic.id != "contract.unsupported_construct"
        }));
        assert!(no_remainder.core.is_none());
    }
}

#[test]
fn untypable_remainder_operands_preserve_specific_diagnostics() {
    for clause in ["require", "ensure", "invariant"] {
        let unary = diagnostics_for(&format!(
            "fn checked(flag: Bool) -> output: Int\n{clause} (-flag) % 2 == 0\n  0\nend\n"
        ));
        assert_eq!(
            unary
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.mismatch")
                .count(),
            1,
            "{clause}: {unary:#?}"
        );
        assert!(unary.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message == "expected `Int`, but found `Bool`"
        }));
        assert!(unary.iter().all(|diagnostic| {
            diagnostic.id != "contract.type_mismatch"
                && diagnostic.id != "contract.unsupported_construct"
        }));

        let missing = diagnostics_for(&format!(
            "fn checked(box: {{value: Int}}) -> output: Int\n{clause} box.missing % 2 == 0\n  0\nend\n"
        ));
        assert!(
            missing
                .iter()
                .any(|diagnostic| diagnostic.id == "contract.field_missing"),
            "{clause}: {missing:#?}"
        );
        assert!(missing.iter().all(|diagnostic| {
            diagnostic.id != "contract.type_mismatch"
                && diagnostic.id != "contract.unsupported_construct"
        }));
    }

    let satisfy = diagnostics_for(concat!(
        "fn checked(box: {value: Int}) -> Int\n",
        "  _value satisfy candidate => candidate % box.missing == 0\n",
        "end\n",
    ));
    assert!(
        satisfy
            .iter()
            .any(|diagnostic| diagnostic.id == "hole.satisfy_field_missing"),
        "{satisfy:#?}"
    );
    assert!(
        satisfy
            .iter()
            .all(|diagnostic| diagnostic.id != "hole.satisfy_type_mismatch"),
        "{satisfy:#?}"
    );

    let satisfy_unary = diagnostics_for(concat!(
        "fn checked(flag: Bool) -> Int\n",
        "  _value satisfy candidate => candidate % (-flag) == 0\n",
        "end\n",
    ));
    assert!(
        satisfy_unary.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message == "expected `Int`, but found `Bool`"
        }),
        "{satisfy_unary:#?}"
    );
    assert!(
        satisfy_unary
            .iter()
            .all(|diagnostic| diagnostic.id != "hole.satisfy_type_mismatch"),
        "{satisfy_unary:#?}"
    );
}

#[test]
fn nested_call_does_not_mask_remainder_operand_mismatch() {
    let diagnostics = diagnostics_for(concat!(
        "fn accepts_bool(value: Bool) -> Bool\n",
        "  value\n",
        "end\n",
        "fn checked(flag: Bool) -> Int\n",
        "require accepts_bool(flag % 2)\n",
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
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch" && diagnostic.message == "expected `Int`, but found `Bool`"
    }));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "contract.unsupported_construct"
            && diagnostic.id != "contract.type_mismatch"
    }));

    let satisfy = diagnostics_for(concat!(
        "fn accepts_bool(value: Bool) -> Bool\n",
        "  value\n",
        "end\n",
        "fn checked(flag: Bool) -> Int\n",
        "  _value satisfy candidate => accepts_bool(candidate % flag)\n",
        "end\n",
    ));
    assert!(
        satisfy.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message == "expected `Int`, but found `Bool`"
        }),
        "{satisfy:#?}"
    );
    assert!(
        satisfy.iter().all(|diagnostic| {
            diagnostic.id != "hole.satisfy_unsupported_construct"
                && diagnostic.id != "hole.satisfy_type_mismatch"
        }),
        "{satisfy:#?}"
    );
}

#[test]
fn whole_call_result_type_does_not_mask_remainder_operand_type() {
    fn call_diagnostics(helper: &str, argument: &str) -> Vec<Diagnostic> {
        diagnostics_for(&format!(
            "fn {helper}(value: Int) -> {}\n  {}\nend\nfn checked(value: Int, flag: Bool) -> Int\nrequire {helper}({argument} % 2)\n  value\nend\n",
            if helper == "is_even" { "Bool" } else { "Int" },
            if helper == "is_even" {
                "value == 0"
            } else {
                "value"
            },
        ))
    }

    for helper in ["identity", "is_even"] {
        let invalid = call_diagnostics(helper, "flag");
        let mismatch = invalid
            .iter()
            .find(|diagnostic| {
                diagnostic.id == "type.mismatch"
                    && diagnostic.message == "expected `Int`, but found `Bool`"
            })
            .unwrap_or_else(|| panic!("missing operand mismatch: {invalid:#?}"));
        let span = mismatch.span.as_ref().expect("operand mismatch span");
        let expected_column = if helper == "identity" { 18 } else { 17 };
        assert_eq!(span.start.line, 5, "{invalid:#?}");
        assert_eq!(
            (span.start.column, span.end.column),
            (expected_column, expected_column + 4),
            "{invalid:#?}"
        );
        assert!(invalid.iter().all(|diagnostic| {
            diagnostic.id != "contract.type_mismatch"
                && diagnostic.id != "contract.unsupported_construct"
        }));
    }

    let valid_bool = call_diagnostics("is_even", "value");
    assert!(valid_bool.is_empty(), "{valid_bool:#?}");

    let valid_int = call_diagnostics("identity", "value");
    assert!(
        valid_int.iter().any(|diagnostic| {
            diagnostic.id == "contract.type_mismatch"
                && diagnostic.message == "contract predicate is not `Bool`"
        }),
        "{valid_int:#?}"
    );
    assert!(valid_int.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch" && diagnostic.message == "expected `Bool`, but found `Int`"
    }));

    let satisfy = diagnostics_for(concat!(
        "fn identity(value: Int) -> Int\n",
        "  value\n",
        "end\n",
        "fn checked(flag: Bool) -> Int\n",
        "  _value satisfy candidate => identity(candidate % flag)\n",
        "end\n",
    ));
    assert!(satisfy.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch" && diagnostic.message == "expected `Int`, but found `Bool`"
    }));
    assert!(satisfy.iter().all(|diagnostic| {
        diagnostic.id != "hole.satisfy_type_mismatch"
            && diagnostic.id != "hole.satisfy_unsupported_construct"
    }));
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
    assert!(
        core.functions[0].contracts.iter().all(|contract| {
            contract.obligation_status == ContractObligationStatus::StaticallyProven
        }),
        "{:#?}",
        core.functions[0].contracts
    );
}

#[test]
fn literal_remainder_contracts_are_statically_proven_through_boolean_composition() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn checked() -> output: Int\n",
            "require 7 % 3 == 1 and true\n",
            "ensure false or 9 % 3 == 0\n",
            "invariant (20 % 6 % 4 == 2) and (8 % 3 == 2)\n",
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
    assert!(
        core.functions[0].contracts.iter().all(|contract| {
            contract.obligation_status == ContractObligationStatus::StaticallyProven
        }),
        "{:#?}",
        core.functions[0].contracts
    );
}
