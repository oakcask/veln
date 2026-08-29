use super::*;

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
fn private_body_lines_share_local_binding_and_return_inference_state() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn helper(value: Int)\n",
            "  let alias = value\n",
            "  alias\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    let helper = environment
        .function("helper")
        .expect("private helper should be present");
    assert_eq!(helper.params, [crate::semantic_model::Type::int()]);
    assert_eq!(helper.return_type, crate::semantic_model::Type::int());
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
fn private_helper_call_site_types_are_preserved_in_core() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn identity(value)\n",
            "  value\n",
            "end\n",
            "\n",
            "pub fn main() -> Int\n",
            "  identity(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let identity = core
        .functions
        .iter()
        .find(|function| function.name == "identity")
        .expect("private helper should be lowered");
    assert_eq!(identity.params.len(), 1);
    assert_eq!(identity.params[0].ty, CoreType::int());
    assert_eq!(identity.return_type, CoreType::int());
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
fn private_helper_return_infers_through_record_field_and_if_branch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn empty_items()\n",
            "  []\n",
            "end\n",
            "\n",
            "fn main(flag: Bool) -> {items: Vec<Int>}\n",
            "  {items: if flag\n",
            "    empty_items()\n",
            "  else\n",
            "    []\n",
            "  end}\n",
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
