use super::*;

#[test]
fn dictionary_value_expected_type_infers_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type CallbackGroup\n",
            "  Handlers(callbacks: Dict<String, fn(Int) -> Option<Vec<String>>>)\n",
            "end\n",
            "fn direct_value(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"direct\"])\n",
            "end\n",
            "fn returned_value(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"return\"])\n",
            "end\n",
            "fn local_value(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"local\"])\n",
            "end\n",
            "fn alias_value(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"alias\"])\n",
            "end\n",
            "fn nested_value(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"nested\"])\n",
            "end\n",
            "fn returned_callbacks() -> Dict<String, fn(Int) -> Option<Vec<String>>>\n",
            "  {\"returned\": returned_value}\n",
            "end\n",
            "fn local_callbacks() -> Dict<String, fn(Int) -> Option<Vec<String>>>\n",
            "  let callbacks: Dict<String, fn(Int) -> Option<Vec<String>>> = {\"local\": local_value}\n",
            "  callbacks\n",
            "end\n",
            "fn alias_callbacks() -> Dict<String, fn(Int) -> Option<Vec<String>>>\n",
            "  let callback = alias_value\n",
            "  {\"alias\": callback}\n",
            "end\n",
            "fn nested_callbacks() -> CallbackGroup\n",
            "  Handlers({\"nested\": nested_value})\n",
            "end\n",
            "pub fn main() -> {direct: Dict<String, fn(Int) -> Option<Vec<String>>>, returned: Dict<String, fn(Int) -> Option<Vec<String>>>, local: Dict<String, fn(Int) -> Option<Vec<String>>>, alias: Dict<String, fn(Int) -> Option<Vec<String>>>, nested: CallbackGroup}\n",
            "  {direct: {\"direct\": direct_value}, returned: returned_callbacks(), local: local_callbacks(), alias: alias_callbacks(), nested: nested_callbacks()}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for name in [
        "direct_value",
        "returned_value",
        "local_value",
        "alias_value",
        "nested_value",
    ] {
        let callback = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("{name} should be lowered"));
        assert_eq!(callback.params[0].ty, CoreType::int(), "{name}");
        assert_eq!(
            callback.return_type,
            CoreType::option(CoreType::vec(CoreType::string())),
            "{name}"
        );
    }
}

#[test]
fn dictionary_value_callback_expected_type_reports_missing_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn missing_context(value)\n",
            "  \"ok\"\n",
            "end\n",
            "type GenericGroup<A>\n",
            "  GenericHandlers(callbacks: Dict<String, fn(A, Int) -> String>)\n",
            "end\n",
            "fn missing_generic_context(value, fixed) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn unconstrained_value() -> String\n",
            "  let callbacks = {\"missing\": missing_context}\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn non_concrete_value() -> String\n",
            "  let group = GenericHandlers({\"missing\": missing_generic_context})\n",
            "  \"ok\"\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 5, "{diagnostics:#?}");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private parameter `value` has no inferred type"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.local_inference_incomplete"
            && diagnostic.message
                == "omitted local binding `callbacks` has no concrete inferred type"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private parameter `fixed` has no inferred type"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.inference_ambiguous"
            && diagnostic.message == "constructor `GenericHandlers` needs type context"
    }));
}

#[test]
fn match_arm_expected_function_type_infers_private_callback_returns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn first_item(value: Int)\n",
            "  Some([value])\n",
            "end\n",
            "fn next_item(value: Int)\n",
            "  Some([value + 1])\n",
            "end\n",
            "fn choose_item(flag: Bool) -> fn(Int) -> Option<Vec<Int>>\n",
            "  match flag\n",
            "    true => first_item\n",
            "    false => next_item\n",
            "  end\n",
            "end\n",
            "pub fn main() -> Option<Vec<Int>>\n",
            "  let callback: fn(Int) -> Option<Vec<Int>> = choose_item(false)\n",
            "  callback(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for name in ["first_item", "next_item"] {
        let callback = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("{name} should be lowered"));
        assert_eq!(
            callback.return_type,
            CoreType::option(CoreType::vec(CoreType::int())),
            "{name}"
        );
    }
}
