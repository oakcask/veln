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
            &contract.text,
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
            "(5 / 2)".to_string()
        } else {
            "value".to_string()
        };
        for _ in 0..operator_count {
            predicate.push_str(" % 3");
        }
        let expected = if literal { 2 } else { 0 };
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

fn independent_remainder_predicate(count: usize, call_wrapped: bool) -> String {
    (0..count)
        .map(|index| {
            let remainder = format!("value % {}", 1_000 + index);
            if call_wrapped {
                format!("is_zero({remainder})")
            } else {
                format!("{remainder} == 0")
            }
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

fn independent_remainder_typing_work(
    count: usize,
    satisfy: bool,
    call_wrapped: bool,
) -> crate::contracts::RemainderTypingIndexWork {
    let mut predicate = independent_remainder_predicate(count, call_wrapped);
    let helper = if call_wrapped {
        "fn is_zero(value: Int) -> Bool\n  value == 0\nend\n"
    } else {
        ""
    };
    let checked = if satisfy {
        predicate = predicate.replace("value", "candidate");
        format!("fn checked(value: Int) -> Int\n  _value satisfy candidate => {predicate}\nend\n")
    } else {
        format!("fn checked(value: Int) -> Int\nrequire {predicate}\n  value\nend\n")
    };
    let source = SourceFile::new("main.veln", format!("{helper}{checked}"));
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    crate::contracts::reset_remainder_typing_index_work();
    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics.is_empty()
            || satisfy
                && diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.id == "hole.unfilled"),
        "{diagnostics:#?}"
    );
    crate::contracts::take_remainder_typing_index_work()
}

#[test]
fn independent_remainder_typing_uses_one_linear_index_per_root() {
    for (satisfy, call_wrapped) in [(false, false), (true, false), (false, true)] {
        let at_128 = independent_remainder_typing_work(128, satisfy, call_wrapped);
        let at_256 = independent_remainder_typing_work(256, satisfy, call_wrapped);
        assert_eq!(
            at_128.index_builds, 1,
            "satisfy={satisfy}, call_wrapped={call_wrapped}"
        );
        assert_eq!(
            at_256.index_builds, 1,
            "satisfy={satisfy}, call_wrapped={call_wrapped}"
        );
        assert_eq!(
            at_128.indexed_chains, 128,
            "satisfy={satisfy}, call_wrapped={call_wrapped}"
        );
        assert_eq!(
            at_256.indexed_chains, 256,
            "satisfy={satisfy}, call_wrapped={call_wrapped}"
        );
        assert!(
            at_256.range_lookups <= at_128.range_lookups * 2 + 8,
            "range lookup work must grow linearly: {at_128:?} -> {at_256:?}; satisfy={satisfy}, call_wrapped={call_wrapped}"
        );
    }
}

#[test]
fn remainder_call_arguments_keep_distinct_source_ranges() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn is_zero(value: Int) -> Bool\n",
            "  value == 0\n",
            "end\n",
            "fn checked(value: Int) -> Int\n",
            "require is_zero( value% 1001 ) and is_zero(value % 1001)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
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
fn composite_remainder_leaf_spans_preserve_original_source_spacing() {
    let operands = ["(2*true)", "(2    *    true)", "((2*true))", "(2 * true)"];
    for clause in ["require", "ensure", "invariant"] {
        for operand in operands {
            for left in [true, false] {
                let predicate = if left {
                    format!("{operand} % 3 == 0")
                } else {
                    format!("3 % {operand} == 0")
                };
                let source =
                    format!("fn checked() -> output: Int\n{clause} {predicate}\n  0\nend\n");
                let diagnostics = diagnostics_for(&source);
                let mismatch =
                    super::remainder_assertions::assert_operand_mismatch(&diagnostics, "Bool");
                let span = mismatch.span.as_ref().expect("invalid leaf span");
                let start = source.find("true").expect("invalid leaf in source");
                assert_eq!((span.start.offset, span.end.offset), (start, start + 4));
            }
        }
    }

    for operand in operands {
        for left in [true, false] {
            let predicate = if left {
                format!("{operand} % candidate == 0")
            } else {
                format!("candidate % {operand} == 0")
            };
            let source = format!(
                "fn checked() -> Int\n  let result: Int = _value satisfy candidate => {predicate}\n  result\nend\n"
            );
            let diagnostics = diagnostics_for(&source);
            let mismatch =
                super::remainder_assertions::assert_operand_mismatch(&diagnostics, "Bool");
            let span = mismatch.span.as_ref().expect("invalid satisfy leaf span");
            let start = source.find("true").expect("invalid leaf in source");
            assert_eq!((span.start.offset, span.end.offset), (start, start + 4));
        }
    }
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

mod diagnostics;
