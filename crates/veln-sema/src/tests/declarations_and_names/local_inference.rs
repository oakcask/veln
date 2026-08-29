use super::*;

#[test]
fn omitted_local_binding_type_infers_from_later_call_use() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn consume(items: Vec<Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  let items = []\n",
            "  consume(items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn omitted_local_binding_type_infers_from_return_compatible_use() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Vec<Int>\n",
            "  let items = []\n",
            "  items\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn omitted_local_list_nil_infers_from_later_call_use() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn consume(items: List<Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  let items = Nil\n",
            "  consume(items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn omitted_local_empty_dictionary_infers_from_later_call_use() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn consume(items: Dict<String, Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  let items = {}\n",
            "  consume(items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn omitted_local_non_empty_collections_infer_from_initializer() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> {items: Vec<Int>, table: Dict<String, Int>}\n",
            "  let items = [1, 2, 3]\n",
            "  let table = {\"one\": 1, \"two\": 2}\n",
            "  {items: items, table: table}\n",
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
    let CoreStmtKind::Let {
        ty: items_ty,
        expr: items_expr,
        ..
    } = &main.body[0].kind
    else {
        panic!("first statement should bind items");
    };
    assert_eq!(*items_ty, CoreType::vec(CoreType::int()));
    assert_eq!(items_expr.ty, CoreType::vec(CoreType::int()));
    let CoreStmtKind::Let {
        ty: table_ty,
        expr: table_expr,
        ..
    } = &main.body[1].kind
    else {
        panic!("second statement should bind table");
    };
    assert_eq!(
        *table_ty,
        CoreType::dict(CoreType::string(), CoreType::int())
    );
    assert_eq!(
        table_expr.ty,
        CoreType::dict(CoreType::string(), CoreType::int())
    );
    assert!(lowered.ir.is_some(), "checked core should lower to IR");
}

#[test]
fn omitted_local_non_empty_collection_conflicts_report_focused_mismatches() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bad_vec() -> Int\n",
            "  let items = [1, \"bad\"]\n",
            "  1\n",
            "end\n",
            "\n",
            "fn bad_dict_key() -> Int\n",
            "  let table = {\"one\": 1, 2: 2}\n",
            "  1\n",
            "end\n",
            "\n",
            "fn bad_dict_value() -> Int\n",
            "  let table = {\"one\": 1, \"two\": \"bad\"}\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id == "type.mismatch" && !diagnostic.message.starts_with("omitted local binding")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message == "expected `Int`, but found `String`"
            && diagnostic
                .details
                .to_json()
                .contains("\"constraint\":\"list_element\"")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message == "expected `String`, but found `Int`"
            && diagnostic
                .details
                .to_json()
                .contains("\"constraint\":\"dict_key\"")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.message == "expected `Int`, but found `String`"
            && diagnostic
                .details
                .to_json()
                .contains("\"constraint\":\"dict_value\"")
    }));
}

#[test]
fn omitted_local_binding_type_reports_unconstrained_unknown() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> Int\n", "  let items = []\n", "  1\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.local_inference_incomplete");
    assert_eq!(
        diagnostics[0].message,
        "omitted local binding `items` has no concrete inferred type"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"inferred_type\":\"Vec<unknown>\"")
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn omitted_local_list_nil_reports_unconstrained_unknown() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  let items = Nil\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.local_inference_incomplete");
    assert_eq!(
        diagnostics[0].message,
        "omitted local binding `items` has no concrete inferred type"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"inferred_type\":\"List<unknown>\"")
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn omitted_local_binding_type_reports_conflicting_later_uses() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn consume_int(items: Vec<Int>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn consume_string(items: Vec<String>) -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  let items = []\n",
            "  let first: Int = consume_int(items)\n",
            "  consume_string(items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `Vec<String>`, but found `Vec<Int>`"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"constraint\":\"call_argument\"")
    );
}
