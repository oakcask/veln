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
