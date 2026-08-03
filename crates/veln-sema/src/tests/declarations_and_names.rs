use super::*;
use crate::types::TypeEnvironment;

#[test]
fn public_function_requires_explicit_type_boundary() {
    let source = SourceFile::new("main.veln", "pub fn main(value)\n  value\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public parameter `value` has no type annotation"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public function has no return type annotation"
    }));
}

#[test]
fn public_function_accepts_omitted_empty_effect_boundary() {
    let source = SourceFile::new("main.veln", "pub fn main() -> Int\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn nominal_effect_perform_checks_and_lowers_operation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user: String, count: Int) -> String\n",
            "end\n",
            "\n",
            "pub fn main() -> String effects [Audit]\n",
            "  perform Audit::record(\"user\", 1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be present");
    assert_eq!(core.effects.len(), 1);
    assert_eq!(core.functions[0].effects, ["Audit"]);
    let veln_core::CoreStmtKind::Return { expr } = &core.functions[0].body[0].kind else {
        panic!("expected return statement");
    };
    assert!(matches!(
        &expr.kind,
        veln_core::CoreExprKind::Perform { effect, operation, args }
            if effect == "Audit" && operation == "record" && args.len() == 2
    ));
}

#[test]
fn nominal_effect_unknown_operation_reports_operation_span() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user: String) -> String\n",
            "end\n",
            "\n",
            "pub fn main() -> String effects [Audit]\n",
            "  perform Audit::missing(\"user\")\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.unknown_operation");
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.line, 6);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.column, 18);
}

#[test]
fn nominal_effect_unknown_perform_reports_effect_span() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String effects [MissingAudit]\n",
            "  perform MissingAudit::record(\"user\")\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "effect.unknown"
                && diagnostic.message == "performed effect `MissingAudit` is not known"
        })
        .unwrap_or_else(|| panic!("expected performed unknown effect: {diagnostics:#?}"));
    assert_eq!(diagnostic.span.as_ref().unwrap().start.line, 2);
    assert_eq!(diagnostic.span.as_ref().unwrap().start.column, 11);
    assert_eq!(diagnostic.span.as_ref().unwrap().end.column, 23);
}

#[test]
fn nominal_effect_missing_public_reports_perform_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user: String) -> String\n",
            "end\n",
            "\n",
            "pub fn main() -> String\n",
            "  perform Audit::record(\"user\")\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.missing_public")
        .unwrap_or_else(|| panic!("expected missing public effect: {diagnostics:#?}"));
    assert_eq!(
        diagnostic.message,
        "public function uses undeclared effect `Audit`"
    );
    assert_eq!(diagnostic.related.len(), 1);
    let related = diagnostic.related[0].to_json();
    assert!(related.contains("\"kind\":\"effect_provenance\""));
    assert!(related.contains("Call to `Audit::record` requires this effect."));
    assert!(related.contains("\"start\":{\"line\":6,\"column\":3,"));
}

#[test]
fn public_handler_requires_and_canonicalizes_declared_provider_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn traced(offset: Int) -> Int effects [stdio]\n",
            "  stdio::println(\"provider\")\n",
            "  offset + 1\n",
            "end\n",
            "\n",
            "pub handler missing(offset: Int) handles Ask\n",
            "  value = traced\n",
            "end\n",
            "\n",
            "pub handler declared(offset: Int) handles Ask effects [stdio, stdio]\n",
            "  value = traced\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let environment = TypeEnvironment::from_module(&module);
    let declared = environment
        .handler_path(&["declared".to_string()], None)
        .expect("declared handler should be present");
    assert_eq!(declared.effects, ["stdio"]);

    let diagnostics = analyze_surface_module(&module);

    let missing = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "handler.missing_public_effect")
        .unwrap_or_else(|| panic!("expected missing handler effect: {diagnostics:#?}"));
    assert_eq!(
        missing.message,
        "public handler `missing` uses undeclared effect `stdio`"
    );
    assert_eq!(missing.span.as_ref().unwrap().start.line, 10);
}

#[test]
fn unknown_declared_effect_reports_effect_label_span() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> () effects [stdio, telepathy]\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.unknown");
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.line, 1);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.column, 37);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().end.column, 46);
}

#[test]
fn unknown_function_type_effect_reports_effect_label_span() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(callback: fn() -> () effects [MissingAudit]) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "effect.unknown"
                && diagnostic.message == "function type effect `MissingAudit` is not known"
        })
        .unwrap_or_else(|| panic!("expected function type unknown effect: {diagnostics:#?}"));
    assert_eq!(diagnostic.span.as_ref().unwrap().start.line, 1);
    assert_eq!(diagnostic.span.as_ref().unwrap().start.column, 43);
    assert_eq!(diagnostic.span.as_ref().unwrap().end.column, 55);
}

#[test]
fn imported_qualified_effect_is_known_in_function_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "use logging\n",
            "\n",
            "pub fn main(callback: fn() -> () effects [logging::Audit]) -> ()\n",
            "  ()\n",
            "end\n",
            "\n",
            "pub effect Audit\n",
            "  record() -> ()\n",
            "end\n",
        ),
    );
    let mut module = lower_surface_ast(&parse(&source).tree);
    module.effects[0].module_name = Some("logging".to_string());

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn duplicate_effect_operation_reports_operation_name_span() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user: String) -> String\n",
            "  record(user: String) -> String\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.line, 3);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.column, 3);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().end.column, 9);
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0].related[0]
            .to_json()
            .contains("\"kind\":\"duplicate_origin\"")
    );
}

#[test]
fn private_function_may_omit_boundary_annotations_when_inference_is_complete() {
    let source = SourceFile::new("main.veln", "fn answer()\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn private_function_reports_incomplete_annotation_inference() {
    let source = SourceFile::new("main.veln", "fn helper(value)\n  value\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private parameter `value` has no inferred type"
            && diagnostic
                .details
                .to_json()
                .contains("\"missing_fact\":\"parameter_type\"")
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private function has no inferred return type"
            && diagnostic
                .details
                .to_json()
                .contains("\"missing_fact\":\"return_type\"")
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn private_function_reports_partial_unknown_return_inference() {
    let source = SourceFile::new("main.veln", "fn helper()\n  []\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.private_inference_incomplete");
    assert_eq!(
        diagnostics[0].message,
        "private function has no inferred return type"
    );
    assert!(diagnostics[0].details.to_json().contains(
        "\"slot_kind\":\"private_return\",\"missing_fact\":\"return_type\",\"inferred_type\":\"Vec<unknown>\""
    ));
}

#[test]
fn private_helper_signature_infers_from_same_module_call_site() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn identity(value)\n",
            "  value\n",
            "end\n",
            "\n",
            "fn consume(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  consume(identity(1))\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_helper_return_infers_from_same_module_expected_call_result() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn empty_items()\n",
            "  []\n",
            "end\n",
            "\n",
            "fn main() -> Vec<Int>\n",
            "  empty_items()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_helper_return_infers_nested_collection_control_flow() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn selected_items(flag: Bool)\n",
            "  if flag\n",
            "    {items: [1]}\n",
            "  else\n",
            "    {items: [2]}\n",
            "  end\n",
            "end\n",
            "\n",
            "fn main() -> {items: Vec<Int>}\n",
            "  selected_items(true)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_helper_signature_infers_from_same_module_test_call_site() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "fn identity(value)\n",
            "  value\n",
            "end\n",
            "\n",
            "test uses_helper() -> ()\n",
            "  let value: Int = identity(1)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_helper_parameter_reports_conflicting_call_site_constraints() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn identity(value)\n",
            "  value\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  let first: Int = identity(1)\n",
            "  identity(\"bad\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"constraint\":\"call_argument\"")
    );
}

#[test]
fn private_helper_parameter_remains_incomplete_for_non_concrete_call_site() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn length(items)\n",
            "  vec_len(items)\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  length([])\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private parameter `items` has no inferred type"
    }));
}

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

#[test]
fn test_declaration_requires_explicit_test_shape() {
    let source = SourceFile::new(
        "main_test.veln",
        "test bad(value: Int) -> Int\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.parameters"
            && diagnostic.message == "test declaration has parameters"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.return_type"
            && diagnostic.message == "test declaration returns `Int`"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn test_declaration_checks_omitted_effect_boundary() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test prints() -> ()\n",
            "  stdio::println(\"hello\")\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_test");
    assert_eq!(
        diagnostics[0].message,
        "test declaration uses undeclared effect `stdio`"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"node_id\":\"test-1\"")
    );
}

#[test]
fn function_declaration_rejects_empty_effects_list() {
    let source = SourceFile::new("main.veln", "fn helper() -> Int effects []\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.empty_declaration");
    assert_eq!(
        diagnostics[0].message,
        "empty effects list is not allowed on a function declaration"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"boundary\":\"private_function\""));
    assert!(details.contains("\"declared_effects\":[]"));
    assert_eq!(diagnostics[0].related.len(), 2);
}

#[test]
fn public_function_declaration_rejects_empty_effects_list() {
    let source = SourceFile::new("main.veln", "pub fn helper() -> Int effects []\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.empty_declaration");
    assert_eq!(
        diagnostics[0].message,
        "empty effects list is not allowed on a function declaration"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"boundary\":\"public_function\"")
    );
}

#[test]
fn test_declaration_rejects_empty_effects_list() {
    let source = SourceFile::new(
        "main_test.veln",
        "test helper() -> () effects []\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.empty_declaration");
    assert_eq!(
        diagnostics[0].message,
        "empty effects list is not allowed on a test declaration"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"boundary\":\"test_declaration\"")
    );
}

#[test]
fn test_declaration_accepts_result_unit_return() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test returns_result() -> Result<(), String>\n",
            "  Ok(())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn test_declaration_accepts_unit_return() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!("test returns_unit() -> ()\n", "  ()\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn binary_schema_accepts_reserved_bits_literal_primitive() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Http2FrameHeader\n",
            "  format binary\n",
            "\n",
            "  priority: UInt16be\n",
            "  length: UInt24be\n",
            "  kind: UInt8\n",
            "  stream_reserved: ReservedBits(1, 0)\n",
            "  stream_id: UInt31be\n",
            "  checksum: UInt32be\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn exact_width_binary_schema_primitives_require_binary_schema_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BadHeader\n",
            "  format text\n",
            "\n",
            "  priority: UInt16be\n",
            "  little_priority: UInt16le\n",
            "  length: UInt24be\n",
            "  little_length: UInt24le\n",
            "  tiny: UInt5\n",
            "  kind: UInt8\n",
            "  stream_id: UInt31be\n",
            "  little_stream_id: UInt31le\n",
            "  checksum: UInt32be\n",
            "  little_checksum: UInt32le\n",
            "  trace_id: UInt40be\n",
            "  little_trace_id: UInt40le\n",
            "  extended_checksum: UInt48be\n",
            "  little_extended_checksum: UInt48le\n",
            "  seven_byte_checksum: UInt56be\n",
            "  little_seven_byte_checksum: UInt56le\n",
            "  massive_checksum: UInt64be\n",
            "  little_massive_checksum: UInt64le\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 18);
    for primitive in [
        "UInt16be", "UInt16le", "UInt24be", "UInt24le", "UInt5", "UInt8", "UInt31be", "UInt31le",
        "UInt32be", "UInt32le", "UInt40be", "UInt40le", "UInt48be", "UInt48le", "UInt56be",
        "UInt56le", "UInt64be", "UInt64le",
    ] {
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.exact_width_primitive"
                && diagnostic.message
                    == format!(
                        "binary schema primitive `{primitive}` can only be used in a `format binary` schema field"
                    )
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"reason\":\"non_binary_format\"")
        }));
    }
}

#[test]
fn binary_schema_primitives_without_format_clause_report_schema_wrong_kind() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingFormatUInt\n",
            "  length: UInt16be\n",
            "end\n",
            "\n",
            "schema MissingFormatReserved\n",
            "  padding: ReservedBits(8, 0)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.exact_width_primitive"
            && diagnostic.message
                == "binary schema primitive `UInt16be` can only be used in a `format binary` schema field"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_binary_format\"")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.reserved_bits_primitive"
            && diagnostic.message
                == "`ReservedBits` can only be used in a `format binary` schema field"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_binary_format\"")
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn exact_width_binary_schema_primitives_are_not_ordinary_types_or_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn ordinary_types(value: UInt16be, little: UInt16le, little_length: UInt24le, little_stream: UInt31le, trace: UInt40be, extended: UInt48be, seven_byte: UInt56be, massive: UInt64be, tiny: UInt5, another: UInt8) -> {short: UInt24be, wide: UInt32be, little_wide: UInt32le, little_trace: UInt40le, little_extended: UInt48le, little_seven_byte: UInt56le, little_massive: UInt64le}\n",
            "  UInt31be\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 18);
    for (primitive, reason) in [
        ("UInt16be", "parameter_type"),
        ("UInt16le", "parameter_type"),
        ("UInt24le", "parameter_type"),
        ("UInt31le", "parameter_type"),
        ("UInt40be", "parameter_type"),
        ("UInt48be", "parameter_type"),
        ("UInt56be", "parameter_type"),
        ("UInt64be", "parameter_type"),
        ("UInt5", "parameter_type"),
        ("UInt8", "parameter_type"),
        ("UInt24be", "return_type"),
        ("UInt32be", "return_type"),
        ("UInt32le", "return_type"),
        ("UInt40le", "return_type"),
        ("UInt48le", "return_type"),
        ("UInt56le", "return_type"),
        ("UInt64le", "return_type"),
        ("UInt31be", "value_position"),
    ] {
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.exact_width_primitive"
                && diagnostic
                    .details
                    .to_json()
                    .contains(&format!("\"primitive\":\"{primitive}\""))
                && diagnostic
                    .details
                    .to_json()
                    .contains(&format!("\"reason\":\"{reason}\""))
        }));
    }
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn binary_schema_rejects_malformed_reserved_bits_primitive() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BadHeader\n",
            "  format binary\n",
            "\n",
            "  missing: ReservedBits()\n",
            "  bare: ReservedBits\n",
            "  named: ReservedBits(width, 0)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "schema.reserved_bits_primitive"
                    && diagnostic.message
                        == "`ReservedBits` requires width and value integer arguments"
                    && diagnostic
                        .details
                        .to_json()
                        .contains("\"reason\":\"argument_count\"")
            })
            .count(),
        2
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.reserved_bits_primitive"
            && diagnostic.message
                == "`ReservedBits` arguments must be literal non-negative integers"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_literal_argument\"")
    }));
}

#[test]
fn reserved_bits_prefix_does_not_capture_type_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Header\n",
            "  format binary\n",
            "\n",
            "  field: ReservedBits::Visible\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "schema.reserved_bits_primitive"),
        "{diagnostics:#?}"
    );
}

#[test]
fn reserved_bits_primitive_reports_non_binary_schema_format() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BadHeader\n",
            "  format text\n",
            "\n",
            "  stream_reserved: ReservedBits(1, 0)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.reserved_bits_primitive"
            && diagnostic.message
                == "`ReservedBits` can only be used in a `format binary` schema field"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_binary_format\"")
    }));
}

#[test]
fn test_declaration_requires_return_annotation() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!("test missing_return()\n", "  ()\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "test.return_type");
    assert_eq!(
        diagnostics[0].message,
        "test declaration has no return type annotation"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"expected_type\":\"() or Result<(), E>\",\"actual_type\":\"missing\"")
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn public_function_rejects_unknown_declared_effect_label() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> () effects [telepathy]\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.unknown");
    assert_eq!(
        diagnostics[0].message,
        "declared effect `telepathy` is not known"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"boundary\":\"public_function\""));
    assert!(details.contains("\"effect\":\"telepathy\""));
    assert!(details.contains("\"known_effects\":[\"stdio\",\"fs\",\"net\",\"db\",\"time\",\"random\",\"process\",\"concurrency\"]"));
}

#[test]
fn accepts_coarse_declared_effect_labels() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [stdio, fs, net, db, time, random, process, concurrency]\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn test_declarations_are_not_callable_functions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "test helper() -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  helper()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.unresolved");
    assert_eq!(diagnostics[0].message, "unresolved call_target `helper`");
}

#[test]
fn duplicate_function_like_declaration_names_are_static_errors() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test same() -> ()\n",
            "  ()\n",
            "end\n",
            "fn same() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(
        diagnostics[0].message,
        "duplicate function declaration name `same`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"function\"")
    );
}

#[test]
fn public_schema_aliases_reject_unresolved_private_and_wrong_kind_targets() {
    let facade_source = SourceFile::new(
        "facade.veln",
        concat!(
            "mod facade\n",
            "use wire\n",
            "pub schema MissingPacket = wire::MissingPacket\n",
            "pub schema PrivatePacket = wire::PrivatePacket\n",
            "pub schema FunctionPacket = wire::make_packet\n",
            "pub schema TypePacket = wire::PacketShape\n",
            "pub schema MissingCodecPacket = wire::PacketCodec\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "pub schema PublicPacket\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "schema PrivatePacket\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub fn make_packet() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub type PacketShape\n",
            "  pub Packet(Int)\n",
            "end\n",
        ),
    );
    let facade = lower_surface_ast(&parse(&facade_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: facade.module,
        uses: facade.uses,
        aliases: facade.aliases,
        effects: Vec::new(),
        handlers: Vec::new(),
        types: wire.types,
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: wire.functions,
    };

    let diagnostics = analyze_surface_module(&module);

    for (id, message) in [
        (
            "name.unresolved",
            "unresolved schema alias target `wire::MissingPacket`",
        ),
        (
            "name.visibility",
            "schema alias target `wire::PrivatePacket` is private",
        ),
        (
            "name.kind_mismatch",
            "public alias target `wire::make_packet` is a function, not a schema",
        ),
        (
            "name.kind_mismatch",
            "public alias target `wire::PacketShape` is a type, not a schema",
        ),
        (
            "name.unresolved",
            "unresolved schema alias target `wire::PacketCodec`",
        ),
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == id && diagnostic.message == message),
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn dispatch_payload_schema_references_report_resolution_diagnostics() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "type Shape\n",
            "  Shape(Int)\n",
            "end\n",
            "\n",
            "schema MissingPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => MissingPayload)\n",
            "end\n",
            "\n",
            "schema NonSchemaPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => Shape)\n",
            "end\n",
            "\n",
            "schema ImportedPrivatePacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::PrivatePayload)\n",
            "end\n",
            "\n",
            "schema ImportedMissingPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::MissingPayload)\n",
            "end\n",
            "\n",
            "schema ImportedWrongKindPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::WireShape)\n",
            "end\n",
            "\n",
            "schema ImportedPublicPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::PublicPayload)\n",
            "end\n",
            "\n",
            "schema ImportedTextPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::TextPayload)\n",
            "end\n",
            "\n",
            "schema SelfPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => SelfPacket)\n",
            "end\n",
            "\n",
            "schema ForwardPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => LaterPayload)\n",
            "end\n",
            "\n",
            "schema PriorPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "  value: UInt8\n",
            "end\n",
            "\n",
            "schema MixedPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => UInt8, 2 => PriorPayload)\n",
            "end\n",
            "\n",
            "schema LaterPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "schema PrivatePayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "pub type WireShape\n",
            "  WireShape(Int)\n",
            "end\n",
            "\n",
            "pub schema PublicPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "pub schema TextPayload\n",
            "  code: Int\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let mut schemas = app.schemas;
    schemas.extend(wire.schemas);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: [app.types, wire.types].concat(),
        schemas,
        codecs: Vec::new(),
        functions: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_payload_schema",
            "dispatch payload schema `MissingPayload` is not declared",
        ),
        (
            "non_schema_payload",
            "dispatch payload `Shape` resolves to a type, not a schema",
        ),
        (
            "private_imported_payload_schema",
            "imported dispatch payload schema `wire::PrivatePayload` is private",
        ),
        (
            "unknown_payload_schema",
            "dispatch payload schema `wire::MissingPayload` is not declared",
        ),
        (
            "non_schema_payload",
            "dispatch payload `wire::WireShape` resolves to a type, not a schema",
        ),
        (
            "non_binary_payload_schema",
            "dispatch payload schema `wire::TextPayload` must use `format binary`",
        ),
        (
            "recursive_payload_missing_length_bound",
            "dispatch payload schema `SelfPacket` requires parent dispatch field `payload` to include a length field",
        ),
        (
            "forward_payload_schema",
            "dispatch payload schema `LaterPayload` must be declared before schema `ForwardPacket`",
        ),
        (
            "incompatible_payload_type",
            "dispatch payload case `2` decodes as `{code: Int, value: Int}`, but earlier cases decode as `Int`",
        ),
    ] {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.dispatch_payload"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{diagnostics:#?}"
        );
    }
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "schema.dispatch_payload"
                || !diagnostic.message.contains("wire::PublicPayload")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn dispatch_payload_schema_incompatible_helper_reports_helper_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ForwardByteViewPayload\n",
            "  format binary\n",
            "  payload: ByteView(later_length)\n",
            "  later_length: UInt8\n",
            "end\n",
            "\n",
            "schema ClosedPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => ForwardByteViewPayload)\n",
            "end\n",
            "\n",
            "schema ExtensionPacket\n",
            "  format binary\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => ForwardByteViewPayload)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let payload_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.dispatch_payload")
        .collect::<Vec<_>>();
    assert_eq!(payload_diagnostics.len(), 2, "{diagnostics:#?}");
    for diagnostic in payload_diagnostics {
        assert_eq!(
            diagnostic.message,
            "dispatch payload schema `ForwardByteViewPayload` is outside the generated binary schema helper slice"
        );
        let details = diagnostic.details.to_json();
        assert!(details.contains("\"reason\":\"incompatible_payload_schema\""));
        assert!(
            details.contains(
                "\"field_path\":[{\"kind\":\"schema\",\"name\":\"ClosedPacket\"},{\"kind\":\"field\",\"name\":\"payload\"}]"
            ) || details.contains(
                "\"field_path\":[{\"kind\":\"schema\",\"name\":\"ExtensionPacket\"},{\"kind\":\"field\",\"name\":\"payload\"}]"
            )
        );
        assert!(
            details.contains(
                "\"expected_decode_helper\":\"byte_decode_step_forward_byte_view_payload\""
            )
        );
        assert!(
            details.contains("\"decode_helper_boundary\":\"generated_binary_schema_decode_step\"")
        );
        assert!(
            details
                .contains("\"expected_encode_helper\":\"byte_encode_forward_byte_view_payload\"")
        );
        assert!(details.contains("\"encode_helper_boundary\":\"generated_binary_schema_encode\""));
        assert!(details.contains("\"unsupported_nested_schema\":\"ForwardByteViewPayload\""));
        assert!(details.contains("\"unsupported_nested_field\":\"payload\""));
        assert!(details.contains(
            "\"unsupported_nested_layout_reason\":\"ineligible_byte_view_length_reference\""
        ));
        assert!(details.contains("\"unavailable_helper_directions\":[\"decode\",\"encode\"]"));
        assert_eq!(diagnostic.related.len(), 3);
        assert!(diagnostic.related[0].to_json().contains(
            "does not expose the generated `byte_decode_step_forward_byte_view_payload` helper"
        ));
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("Nested dispatch payload field `ForwardByteViewPayload.payload` prevents generated decode and encode helpers")
        );
        assert!(
            diagnostic.related[1]
                .to_json()
                .contains("length reference `later_length` to be declared before field `payload`")
        );
        assert!(
            diagnostic.related[2]
                .to_json()
                .contains("expected `byte_decode_step_forward_byte_view_payload` and `byte_encode_forward_byte_view_payload`")
        );
    }
}

#[test]
fn repeat_payload_schema_references_report_resolution_diagnostics() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "type Shape\n",
            "  Shape(Int)\n",
            "end\n",
            "\n",
            "schema MissingCountPacket\n",
            "  format binary\n",
            "  items: Repeat(count, UInt8)\n",
            "end\n",
            "\n",
            "schema ForwardCountPacket\n",
            "  format binary\n",
            "  items: Repeat(count, UInt8)\n",
            "  count: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindCountPacket\n",
            "  format binary\n",
            "  length: UInt8\n",
            "  flags: ByteView(length)\n",
            "  items: Repeat(flags, UInt8)\n",
            "end\n",
            "\n",
            "schema MissingPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, MissingPayload)\n",
            "end\n",
            "\n",
            "schema NonSchemaPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, Shape)\n",
            "end\n",
            "\n",
            "schema ImportedPrivatePacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::PrivatePayload)\n",
            "end\n",
            "\n",
            "schema ImportedMissingPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::MissingPayload)\n",
            "end\n",
            "\n",
            "schema ImportedWrongKindPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::WireShape)\n",
            "end\n",
            "\n",
            "schema ImportedPublicPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::PublicPayload)\n",
            "end\n",
            "\n",
            "schema ImportedTextPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::TextPayload)\n",
            "end\n",
            "\n",
            "schema SelfPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, SelfPacket)\n",
            "end\n",
            "\n",
            "schema ForwardPacket\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  items: Repeat(count, LaterPayload)\n",
            "end\n",
            "\n",
            "schema LaterPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "schema PrivatePayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "pub type WireShape\n",
            "  WireShape(Int)\n",
            "end\n",
            "\n",
            "pub schema PublicPayload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "pub schema TextPayload\n",
            "  code: Int\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let mut schemas = app.schemas;
    schemas.extend(wire.schemas);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: [app.types, wire.types].concat(),
        schemas,
        codecs: Vec::new(),
        functions: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat count field `count` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat count field `count` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat count field `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.repeat_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{diagnostics:#?}"
        );
    }

    for (reason, message) in [
        (
            "unknown_payload_schema",
            "repeat payload schema `MissingPayload` is not declared",
        ),
        (
            "non_schema_payload",
            "repeat payload `Shape` resolves to a type, not a schema",
        ),
        (
            "private_imported_payload_schema",
            "imported repeat payload schema `wire::PrivatePayload` is private",
        ),
        (
            "unknown_payload_schema",
            "repeat payload schema `wire::MissingPayload` is not declared",
        ),
        (
            "non_schema_payload",
            "repeat payload `wire::WireShape` resolves to a type, not a schema",
        ),
        (
            "non_binary_payload_schema",
            "repeat payload schema `wire::TextPayload` must use `format binary`",
        ),
        (
            "self_payload_schema",
            "repeat payload schema `SelfPacket` cannot reference itself",
        ),
        (
            "forward_payload_schema",
            "repeat payload schema `LaterPayload` must be declared before schema `ForwardPacket`",
        ),
    ] {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.repeat_payload"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{diagnostics:#?}"
        );
    }
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "schema.repeat_payload"
                || !diagnostic.message.contains("wire::PublicPayload")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn duplicate_use_aliases_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use platform.io\n",
            "use local.io\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate import alias name `io`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"module\"")
    );
}

#[test]
fn duplicate_use_aliases_are_scoped_to_declaring_module() {
    let first_source = SourceFile::new(
        "first.veln",
        concat!(
            "mod first\n",
            "use shared\n",
            "fn first_value() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let second_source = SourceFile::new(
        "second.veln",
        concat!(
            "mod second\n",
            "use shared\n",
            "fn second_value() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let first = lower_surface_ast(&parse(&first_source).tree);
    let second = lower_surface_ast(&parse(&second_source).tree);
    let module = SurfaceModule {
        module: first.module,
        uses: [first.uses, second.uses].concat(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        functions: [first.functions, second.functions].concat(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );
}

#[test]
fn public_function_alias_rejects_type_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "pub fn parse = Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message == "public alias target `Document` is a type, not a function"
    }));
}

#[test]
fn public_type_alias_rejects_function_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub type Document = parse\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message == "public alias target `parse` is a function, not a type"
    }));
}

#[test]
fn public_alias_rejects_unresolved_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "pub fn parse = impl::parse\n",
            "pub type Document = impl::Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved function alias target `impl::parse`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved type alias target `impl::Document`"
    }));
}

#[test]
fn public_alias_names_share_member_namespaces() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn parse = parse\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "pub type Document = Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate function alias name `parse`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate type alias name `Document`"
    }));
}

#[test]
fn public_schema_alias_names_share_schema_namespace() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "pub schema Packet\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "pub schema Packet = Packet\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate schema alias name `Packet`"
    }));
}

#[test]
fn use_declarations_require_module_identity() {
    let source = SourceFile::new(
        "main.veln",
        concat!("use platform.io\n", "fn main() -> ()\n", "  ()\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "module.missing_identity");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Module);
    assert_eq!(
        diagnostics[0].message,
        "module import requires a module identity"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"field\":\"module_identity\"")
    );
}

#[test]
fn duplicate_parameter_names_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Int, value: Int) -> Int\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate parameter name `value`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"value\"")
    );
}

#[test]
fn duplicate_variadic_parameter_keeps_shape_diagnostics() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(values: ...String, values: ...String) -> String\n  \"\"\nend\n",
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 4, "{diagnostics:#?}");
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variadic_parameter_position",
            "type.variadic_parameter_duplicate",
            "type.variadic_parameter_duplicate",
            "name.duplicate",
        ]
    );
}

#[test]
fn let_names_cannot_duplicate_the_function_value_scope() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Int) -> Int\n  let value = 1\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(
        diagnostics[0].message,
        "duplicate local binding name `value`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"value\"")
    );
}

#[test]
fn wildcard_let_pattern_does_not_bind_or_shadow_names() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Int) -> Int\n",
            "  let _: Int = value\n",
            "  value\n",
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
    let CoreStmtKind::Expr { expr } = &main.body[0].kind else {
        panic!("wildcard let should lower as expression statement");
    };
    assert!(matches!(&expr.kind, CoreExprKind::Local(name) if name == "value"));
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(&expr.kind, CoreExprKind::Local(name) if name == "value"));
    assert!(lowered.ir.is_some());
}

#[test]
fn lexical_handler_lowers_through_checked_core_and_typed_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn provide(ctx: Int) -> Int\n",
            "  ctx\n",
            "end\n",
            "\n",
            "handler ask(ctx: Int) handles Ask\n",
            "  value = provide\n",
            "end\n",
            "\n",
            "pub fn main() -> Int\n",
            "  handle perform Ask::value() with ask(41)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should lower to checked core");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Handle { effect, providers, context_args, body }
            if effect == "Ask"
                && providers.len() == 1
                && providers[0].operation == "value"
                && context_args.len() == 1
                && matches!(&body.kind, CoreExprKind::Perform { operation, .. } if operation == "value")
    ));
    let ir = lowered.ir.as_ref().expect("typed IR should be built");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should lower to typed IR");
    let IrStmtKind::Return { value: expr } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &expr.kind,
        IrExprKind::Handle { effect, providers, context_args, body }
            if effect == "Ask"
                && providers.len() == 1
                && providers[0].operation == "value"
                && context_args.len() == 1
                && matches!(&body.kind, IrExprKind::Perform { operation, .. } if operation == "value")
    ));
}

#[test]
fn duplicate_record_field_names_are_static_errors() {
    let source = SourceFile::new("main.veln", "fn bad() -> {a: Int}\n  {a: 1, a: 2}\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate record field name `a`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"record_field\"")
    );
}

#[test]
fn duplicate_pattern_bindings_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(input: {left: Int, right: Int}) -> Int\n",
            "  match input\n",
            "    {left: value, right: value} => value\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate pattern binding name `value`"
                && diagnostic.related.len() == 1
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn duplicate_record_pattern_field_names_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(input: {value: Int}) -> Int\n",
            "  match input\n",
            "    {value: first, value: second} => first\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate record pattern field name `value`"
                && diagnostic.related.len() == 1
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn reports_hole_with_declared_return_expected_type() {
    let source = SourceFile::new("main.veln", "fn todo() -> Result<(), AppError>\n  _\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Hole);
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"hole\",\"node_id\":\"hole-3\",\"label\":null,",
            "\"expected_type\":\"Result<(), AppError>\",",
            "\"expected_type_source\":\"declared\",",
            "\"constraints\":[],\"local_bindings\":[],",
            "\"candidate_queries\":[{\"kind\":\"symbol\",",
            "\"candidate_status\":\"query_only\",",
            "\"application_policy\":\"manual_review_required\",",
            "\"query\":\"fn() -> Result<(), AppError>\"}]}"
        )
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn ranks_visible_symbol_candidates_for_hole_expected_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"candidates\":["));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\",",
        "\"edits\":[{\"kind\":\"replace\","
    )));
    assert!(details.contains(concat!(
        "\"span\":{\"file\":\"main.veln\",",
        "\"start\":{\"line\":3,\"column\":3,\"offset\":48},",
        "\"end\":{\"line\":3,\"column\":4,\"offset\":49}}"
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\",",
        "\"edits\":[{\"kind\":\"replace\","
    )));
    assert!(details.contains("\"replacement\":\"limit\""));
    assert!(details.contains("\"target\":{\"node_id\":\"hole-"));
    assert!(details.contains("\"edit_summary\":\"Replace hole with `fallback`\""));
    assert!(details.contains(concat!(
        "\"evidence\":[{\"kind\":\"type\",\"status\":\"passed\",",
        "\"expected_type\":\"Int\",\"candidate_type\":\"Int\"},",
        "{\"kind\":\"ranking\",\"status\":\"ranked\",\"rank\":1,"
    )));
    assert!(details.contains(concat!(
        "\"known_limits\":[\"edit is advisory and unapplied\",",
        "\"tests and examples have not been run\"]"
    )));
    assert!(details.contains(concat!(
        "\"blocking_obligations\":[\"manual_review_required\",",
        "\"verification.not_run\"]"
    )));
    assert!(details.contains(concat!(
        "\"verification_hint\":{\"command\":\"veln check --json main.veln\",",
        "\"scope\":\"after_applying_candidate_edit\"}"
    )));
    assert!(details.contains("\"application_status\":\"unapplied\""));
}

#[test]
fn holes_receive_expected_types_from_expression_contexts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Box<A>\n",
            "  Box(value: A)\n",
            "end\n",
            "\n",
            "fn accept(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "fn return_context(candidate: Int) -> Int\n",
            "  _\n",
            "end\n",
            "\n",
            "fn call_context(candidate: Int) -> Int\n",
            "  accept(_)\n",
            "end\n",
            "\n",
            "fn record_context(candidate: Int) -> {value: Int}\n",
            "  {value: _}\n",
            "end\n",
            "\n",
            "fn if_context(flag: Bool, candidate: Int) -> Int\n",
            "  if flag\n",
            "    _\n",
            "  else\n",
            "    candidate\n",
            "  end\n",
            "end\n",
            "\n",
            "fn match_context(flag: Bool, candidate: Int) -> Int\n",
            "  match flag\n",
            "    true => _\n",
            "    false => candidate\n",
            "  end\n",
            "end\n",
            "\n",
            "fn constructor_context(candidate: Int) -> Box<Int>\n",
            "  Box(_)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let holes = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "hole.unfilled")
        .collect::<Vec<_>>();
    assert_eq!(holes.len(), 6, "{diagnostics:#?}");
    assert_eq!(diagnostics.len(), holes.len(), "{diagnostics:#?}");
    for hole in holes {
        assert_eq!(hole.message, "hole requires a `Int` value");
        let details = hole.details.to_json();
        assert!(details.contains("\"expected_type\":\"Int\""), "{details}");
        assert!(
            details.contains("\"expected_type_source\":\"declared\""),
            "{details}"
        );
        assert!(
            details.contains("\"candidate_queries\":[{\"kind\":\"symbol\""),
            "{details}"
        );
        assert!(
            details.contains("\"name\":\"candidate\",\"type\":\"Int\""),
            "{details}"
        );
    }
}
