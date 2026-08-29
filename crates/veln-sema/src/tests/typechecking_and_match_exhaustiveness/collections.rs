use super::*;

#[test]
fn accepts_supported_type_forms_and_record_expected_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> {score: Float, names: Vec<String>, table: Dict<String, Int>, ",
            "callback: fn(Int) -> String}\n",
            "  {score: _, names: [], table: _, callback: _}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.details.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("\"expected_type\":\"Float\""));
    assert!(rendered.contains("\"expected_type\":\"Dict<String, Int>\""));
    assert!(rendered.contains("\"expected_type\":\"fn(Int) -> String\""));
    assert!(rendered.contains("\"candidate_queries\":[{\"kind\":\"symbol\""));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.related.is_empty())
    );
}

#[test]
fn accepts_dictionary_literals_with_expected_key_and_value_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Dict<String, Int>\n",
            "  {\"one\": 1, \"two\": 2}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::dict(CoreType::string(), CoreType::int()));
    let CoreExprKind::Dict(entries) = &expr.kind else {
        panic!("tail expression should lower as dictionary");
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].key.ty, CoreType::string());
    assert_eq!(entries[0].value.ty, CoreType::int());
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Dict(entries) = &value.kind else {
        panic!("tail expression should lower as IR dictionary");
    };
    assert_eq!(entries.len(), 2);
}

#[test]
fn accepts_empty_dictionary_literal_with_expected_key_and_value_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> Dict<String, Int>\n", "  {}\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::dict(CoreType::string(), CoreType::int()));
    let CoreExprKind::Dict(entries) = &expr.kind else {
        panic!("tail expression should lower as empty dictionary");
    };
    assert!(entries.is_empty());
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Dict(entries) = &value.kind else {
        panic!("tail expression should lower as IR dictionary");
    };
    assert!(entries.is_empty());
}

#[test]
fn accepts_dictionary_literals_with_identifier_led_expression_keys() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(seed: Int) -> Dict<Int, String>\n",
            "  {seed + 1: \"next\"}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::dict(CoreType::int(), CoreType::string()));
    let CoreExprKind::Dict(entries) = &expr.kind else {
        panic!("tail expression should lower as dictionary");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key.ty, CoreType::int());
    assert_eq!(entries[0].value.ty, CoreType::string());
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Dict(entries) = &value.kind else {
        panic!("tail expression should lower as IR dictionary");
    };
    assert_eq!(entries.len(), 1);
}

#[test]
fn accepts_empty_collections_from_expected_type_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Box<A>\n",
            "  Box(value: A)\n",
            "end\n",
            "\n",
            "fn consume_vec(items: Vec<Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn consume_list(items: List<Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn consume_dict(items: Dict<String, Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn annotated_vec() -> Vec<Int>\n",
            "  let items: Vec<Int> = []\n",
            "  items\n",
            "end\n",
            "\n",
            "fn annotated_list() -> List<Int>\n",
            "  let items: List<Int> = Nil\n",
            "  items\n",
            "end\n",
            "\n",
            "fn annotated_dict() -> Dict<String, Int>\n",
            "  let items: Dict<String, Int> = {}\n",
            "  items\n",
            "end\n",
            "\n",
            "fn inferred_dict() -> Int\n",
            "  let items = {}\n",
            "  consume_dict(items)\n",
            "end\n",
            "\n",
            "fn return_vec() -> Vec<Int>\n",
            "  []\n",
            "end\n",
            "\n",
            "fn return_list() -> List<Int>\n",
            "  Nil\n",
            "end\n",
            "\n",
            "fn return_dict() -> Dict<String, Int>\n",
            "  {}\n",
            "end\n",
            "\n",
            "fn call_vec() -> Int\n",
            "  consume_vec([])\n",
            "end\n",
            "\n",
            "fn call_list() -> Int\n",
            "  consume_list(Nil)\n",
            "end\n",
            "\n",
            "fn call_dict() -> Int\n",
            "  consume_dict({})\n",
            "end\n",
            "\n",
            "fn record_fields() -> {vec: Vec<Int>, list: List<Int>, dict: Dict<String, Int>}\n",
            "  {vec: [], list: Nil, dict: {}}\n",
            "end\n",
            "\n",
            "fn match_vec(flag: Bool) -> Vec<Int>\n",
            "  match flag\n",
            "    true => []\n",
            "    false => [1]\n",
            "  end\n",
            "end\n",
            "\n",
            "fn match_list(flag: Bool) -> List<Int>\n",
            "  match flag\n",
            "    true => Nil\n",
            "    false => Cons(1, Nil)\n",
            "  end\n",
            "end\n",
            "\n",
            "fn match_dict(flag: Bool) -> Dict<String, Int>\n",
            "  match flag\n",
            "    true => {}\n",
            "    false => {\"one\": 1}\n",
            "  end\n",
            "end\n",
            "\n",
            "fn constructor_vec() -> Box<Vec<Int>>\n",
            "  Box([])\n",
            "end\n",
            "\n",
            "fn constructor_list() -> Box<List<Int>>\n",
            "  Box(Nil)\n",
            "end\n",
            "\n",
            "fn constructor_dict() -> Box<Dict<String, Int>>\n",
            "  Box({})\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn omitted_local_empty_collections_lower_with_concrete_later_use_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn consume_vec(items: Vec<Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn consume_list(items: List<Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn consume_dict(items: Dict<String, Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  let vec_items = []\n",
            "  let list_items = Nil\n",
            "  let dict_items = {}\n",
            "  consume_vec(vec_items) + consume_list(list_items) + consume_dict(dict_items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[3].kind else {
        panic!("tail expression should lower as return");
    };
    let actual = call_argument_types(expr);
    assert_eq!(
        actual,
        vec![
            CoreType::vec(CoreType::int()),
            CoreType::named("List", vec![CoreType::int()]),
            CoreType::dict(CoreType::string(), CoreType::int()),
        ]
    );
    assert!(lowered.ir.is_some(), "checked core should lower to IR");
}

#[test]
fn rejects_empty_collections_with_ambiguous_expected_type_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  let vec_count: Int = vec_len([])\n",
            "  let list_empty: Bool = list_is_empty(Nil)\n",
            "  let has_key: Bool = dict_contains({}, \"key\")\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| { diagnostic.id == "type.inference_ambiguous" })
    );
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.details.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("\"inferred_type\":\"Vec<unknown>\""));
    assert!(rendered.contains("\"inferred_type\":\"List<unknown>\""));
    assert!(rendered.contains("\"inferred_type\":\"Dict<unknown, unknown>\""));
}
