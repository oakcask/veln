use super::*;

#[test]
fn parses_module_use_nested_types_and_multiple_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.core\n",
            "use platform.io\n",
            "fn collect(items: Vec<Result<Int, Error>>) -> Result<Vec<Int>, Error> effects [fs, net]\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.module.as_ref().unwrap().name, "app.core");
    assert_eq!(output.tree.uses[0].name, "platform.io");
    let function = first_function(&output);
    assert_eq!(
        function.params[0].ty.as_deref(),
        Some("Vec<Result<Int, Error>>")
    );
    assert_eq!(
        function.return_type.as_deref(),
        Some("Result<Vec<Int>, Error>")
    );
    assert_eq!(
        function.effects.as_ref().unwrap(),
        &vec!["fs".to_string(), "net".to_string()]
    );
}

#[test]
fn parses_underscore_led_module_header_for_casing_recovery() {
    let source = SourceFile::new(
        "main.veln",
        concat!("mod _net\r\n", "fn main() -> ()\r\n", "  ()\r\n", "end\r\n"),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let module = output.tree.module.as_ref().expect("module header");
    assert_eq!(module.name, "_net");
    assert_eq!(module.name_spans[0].start.line, 1);
    assert_eq!(module.name_spans[0].start.column, 5);
    assert_eq!(module.name_spans[0].end.column, 9);
}

#[test]
fn parses_external_package_use_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use sub::module from \"github.com/oakcask/foo\"\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.uses[0].name, "sub::module");
    let package = output.tree.uses[0]
        .package
        .as_ref()
        .expect("use declaration should keep package source");
    assert_eq!(package.name, "github.com/oakcask/foo");
    assert_eq!(package.span.start.line, 1);
    assert_eq!(package.span.start.column, 22);
}

#[test]
fn parses_function_return_type_effects_before_declaration_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn callback_factory() -> fn(String) -> () effects [stdio]\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let function = first_function(&output);
    assert_eq!(
        function.return_type.as_deref(),
        Some("fn(String) -> () effects [stdio]")
    );
    assert_eq!(function.effects, None);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "pub fn callback_factory() -> fn(String) -> () effects [stdio]\n",
            "end\n",
        )
    );
}

#[test]
fn parses_and_formats_result_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn clamp(value: Int) -> output: Int\n",
            "ensure output >= value\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let function = first_function(&output);
    assert_eq!(
        function
            .return_binding
            .as_ref()
            .map(|binding| binding.name.as_str()),
        Some("output")
    );
    assert_eq!(function.return_type.as_deref(), Some("Int"));
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn clamp(value: Int) -> output: Int\n",
            "\tensure output >= value\n",
            "\tvalue\n",
            "end\n",
        )
    );
}

#[test]
fn parses_contract_predicate_subset() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn clamp(value: Int, limit: Int) -> output: Int\n",
            "require value >= 0 and value <= limit\n",
            "ensure output.total == value + limit\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let function = first_function(&output);
    assert_eq!(function.contracts.len(), 2);
    assert_eq!(function.contracts[0].text, "value >= 0 and value <= limit");
    assert_eq!(function.contracts[1].text, "output.total == value + limit");
}

#[test]
fn parses_remainder_in_contract_predicates_at_multiply_precedence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn checked(value: Int) -> output: Int\n",
            "require value % 3 * 2 == 2\n",
            "ensure output * 2 % 3 == 2\n",
            "invariant value % 4 % 3 == 1\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    assert_eq!(function.contracts.len(), 3);
}

#[test]
fn flat_remainder_chain_retains_linear_operand_provenance() {
    fn retained_structure(operator_count: usize) -> (usize, usize) {
        let predicate = std::iter::once("value".to_string())
            .chain((0..operator_count).map(|_| "% 2".to_string()))
            .collect::<Vec<_>>()
            .join(" ");
        let source = SourceFile::new(
            "main.veln",
            format!("fn checked(value: Int) -> Int\nrequire {predicate} == 0\n  value\nend\n"),
        );
        let output = parse(&source);
        assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
        let function = first_function(&output);
        let remainders = &function.contracts[0].remainder_spans;
        assert_eq!(remainders.len(), operator_count);
        assert_eq!(
            &function.contracts[0].text[remainders[0].left_range.clone()],
            "value"
        );
        assert_eq!(remainders[0].left_remainder, None);
        assert!(
            remainders[1..]
                .iter()
                .enumerate()
                .all(|(index, remainder)| remainder.left_remainder == Some(index)
                    && &function.contracts[0].text[remainder.right_range.clone()] == "2")
        );
        (
            remainders.len(),
            function.contracts[0].predicate_tokens.len(),
        )
    }

    let at_500 = retained_structure(500);
    let at_501 = retained_structure(501);
    assert_eq!(at_501.0 - at_500.0, 1);
    assert_eq!(at_501.1 - at_500.1, 2);
}

#[test]
fn right_nested_remainder_retains_linear_source_backed_structure() {
    fn retained_structure(operator_count: usize) -> (usize, usize) {
        let mut predicate = "value".to_string();
        for _ in 0..operator_count {
            predicate = format!("value % ({predicate})");
        }
        let source = SourceFile::new(
            "main.veln",
            format!("fn checked(value: Int) -> Int\nrequire {predicate} == 0\n  value\nend\n"),
        );
        let output = parse(&source);
        assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
        let contract = &first_function(&output).contracts[0];
        assert_eq!(contract.remainder_spans.len(), operator_count);
        assert!(
            contract
                .remainder_spans
                .iter()
                .all(|remainder| remainder.left_remainder.is_none())
        );
        (
            contract.remainder_spans.len(),
            contract.predicate_tokens.len(),
        )
    }

    let at_120 = retained_structure(120);
    let at_121 = retained_structure(121);
    assert_eq!(at_121.0 - at_120.0, 1);
    assert_eq!(at_121.1 - at_120.1, 4);
}

fn nested_contract_value(depth: usize) -> String {
    format!("{}value{}", "(".repeat(depth), ")".repeat(depth))
}

fn right_nested_remainder(depth: usize) -> String {
    let mut predicate = "value".to_string();
    for _ in 0..depth {
        predicate = format!("value % ({predicate})");
    }
    predicate
}

fn nested_call_remainder(depth: usize) -> String {
    format!(
        "{}value % 2{}",
        "identity(".repeat(depth),
        ")".repeat(depth)
    )
}

#[test]
fn remainder_predicate_parenthesis_nesting_has_an_exact_safe_boundary() {
    for depth in [127, 128] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "fn checked(value: Int) -> Int\nrequire {} == value % 2\n  value\nend\n",
                right_nested_remainder(depth)
            ),
        );
        let output = parse(&source);
        assert!(
            output.diagnostics.is_empty(),
            "depth {depth}: {:#?}",
            output.diagnostics
        );
    }

    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn checked(value: Int) -> Int\nrequire {} == value % 2\n  value\nend\n",
            right_nested_remainder(129)
        ),
    );
    let output = parse(&source);
    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    assert_eq!(output.diagnostics[0].id, "parse.contract_predicate");
    assert_eq!(
        output.diagnostics[0].message,
        "contract predicate nesting exceeds the supported limit of 128 parenthesized expressions"
    );
}

#[test]
fn remainder_call_parenthesis_nesting_has_an_exact_safe_boundary() {
    for (depth, expected_diagnostics) in [(128, 0), (129, 1)] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "fn checked(value: Int) -> Int\nrequire {} == value % 2\n  value\nend\n",
                nested_call_remainder(depth)
            ),
        );
        let output = parse(&source);
        assert_eq!(
            output.diagnostics.len(),
            expected_diagnostics,
            "depth {depth}: {:#?}",
            output.diagnostics
        );
    }
}

#[test]
fn remainder_nesting_limit_is_scoped_to_remainder_predicates() {
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn checked(value: Int) -> Int\nrequire {} == value\n  value\nend\n",
            nested_contract_value(129)
        ),
    );
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn checked() -> Int\n  let result: Int = _value satisfy candidate => {} == candidate\n  result\nend\n",
            nested_contract_value(129)
        ),
    );
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn checked() -> Int\n  let result: Int = _value satisfy candidate => {} == candidate % 2\n  result\nend\n",
            right_nested_remainder(129)
        ),
    );
    let output = parse(&source);
    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    assert_eq!(output.diagnostics[0].id, "parse.satisfy_predicate");
}

#[test]
fn rejects_non_predicate_contract_syntax() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bad(value: Int) -> Int\n",
            "require _missing\n",
            "ensure [value]\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.contract_predicate"
            && diagnostic.message == "hole syntax is not allowed in a contract predicate"
            && diagnostic.parser_context == "contract_predicate"
    }));
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.contract_predicate"
            && diagnostic.message == "list syntax is not allowed in a contract predicate"
            && diagnostic.parser_context == "contract_predicate"
    }));
}

#[test]
fn records_only_structurally_complete_perform_qualifiers_in_contracts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn guarded() -> Int\n",
            "require perform Choose::pick(1 2) + perform Choose::pick(\n",
            "  1\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    let function = first_function(&output);
    assert_eq!(function.contracts[0].perform_effect_spans.len(), 1);
    assert_eq!(function.contracts[0].perform_effect_spans[0].start.line, 2);
    assert_eq!(
        function.contracts[0].perform_effect_spans[0].start.column,
        17
    );
}

#[test]
fn formats_unit_type_with_empty_tuple_spelling() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Unit) -> Result<Unit, AppError>\n",
            "\tlet ready: Unit = ()\n",
            "\tOk(ready)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn main(value: ()) -> Result<(), AppError>\n",
            "\tlet ready: () = ()\n",
            "\tOk(ready)\n",
            "end\n",
        )
    );
}

#[test]
fn parses_hole_satisfy_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> ()\n",
            "\t_value satisfy candidate => candidate > 0 and candidate < 10\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Hole {
        name,
        satisfy: Some(satisfy),
    } = &expr.kind
    else {
        panic!("expected hole with satisfy clause");
    };
    assert_eq!(name.as_deref(), Some("value"));
    assert_eq!(satisfy.candidate.as_deref(), Some("candidate"));
    assert_eq!(satisfy.predicate, "candidate > 0 and candidate < 10");
    assert_eq!(format_tree(&output.tree), source.text());
}

#[test]
fn reports_malformed_hole_satisfy_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> ()\n",
            "  _first satisfy => candidate > 0\n",
            "  _second satisfy candidate candidate > 0\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.satisfy_candidate"
            && diagnostic.message == "satisfy clause is missing a candidate binding"
            && diagnostic.expected == vec!["candidate binding"]
            && diagnostic.recovery.anchor.as_deref() == Some("=>")
    }));
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.satisfy_arrow"
            && diagnostic.message == "satisfy clause is missing `=>`"
            && diagnostic.expected == vec!["=>"]
    }));
}

#[test]
fn rejects_non_predicate_satisfy_syntax() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> Int\n",
            "  _value satisfy candidate => candidate |> valid\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.satisfy_predicate"
            && diagnostic.message == "pipeline syntax is not allowed in a contract predicate"
            && diagnostic.parser_context == "satisfy_predicate"
    }));
}
