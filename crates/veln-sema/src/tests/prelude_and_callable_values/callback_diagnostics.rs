use super::*;

#[test]
fn imported_effectful_declared_helpers_report_callback_mismatches() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod spec.app\n",
            "use spec.helpers\n",
            "fn wrong_return(value) -> Int effects [stdio]\n",
            "  1\n",
            "end\n",
            "fn extra_effect(value) -> String effects [stdio, net]\n",
            "  value\n",
            "end\n",
            "pub fn main() -> {wrong_return: String, extra_effect: String} effects [stdio, net]\n",
            "  {wrong_return: helpers::apply_effect(wrong_return), extra_effect: helpers::apply_effect(extra_effect)}\n",
            "end\n",
        ),
    );
    let helpers_source = SourceFile::new(
        "helpers.veln",
        concat!(
            "mod spec.helpers\n",
            "pub fn apply_effect(callback: fn(String) -> String effects [stdio]) -> String effects [stdio]\n",
            "  callback(\"ready\")\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let helpers = lower_surface_ast(&parse(&helpers_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: [app.types, helpers.types].concat(),
        functions: [app.functions, helpers.functions].concat(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    for (expected_message, expected_column) in [
        (
            "expected `fn(String) -> String effects [stdio]`, but found `fn(String) -> Int effects [stdio]`",
            40,
        ),
        (
            "expected `fn(String) -> String effects [stdio]`, but found `fn(String) -> String effects [stdio, net]`",
            91,
        ),
    ] {
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == expected_message)
            .unwrap_or_else(|| panic!("missing diagnostic: {expected_message}"));
        assert_eq!(diagnostic.id, "type.mismatch");
        let span = diagnostic
            .span
            .as_ref()
            .expect("diagnostic should point at the imported helper call");
        assert_eq!(span.file.as_str(), "app.veln");
        assert_eq!(span.start.line, 10);
        assert_eq!(span.start.column, expected_column);
    }
}

#[test]
fn unconstrained_helpers_do_not_infer_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn apply_unknown(callback) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn stringify(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn main() -> String\n",
            "  apply_unknown(stringify)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.private_inference_incomplete"
            && diagnostic.message == "private parameter `value` has no inferred type"
    }));
}

#[test]
fn dictionary_prelude_callbacks_infer_key_and_value_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn dict_string(key, value) -> String\n",
            "  key\n",
            "end\n",
            "fn qualified_dict_string(key, value) -> String\n",
            "  key\n",
            "end\n",
            "fn dict_keep(key, value) -> Bool\n",
            "  true\n",
            "end\n",
            "fn dict_folder(acc: String, key, value) -> String\n",
            "  acc\n",
            "end\n",
            "fn dict_try(key, value) -> Result<String, String>\n",
            "  Ok(key)\n",
            "end\n",
            "pub fn main(table: Dict<String, Int>) -> {mapped: Dict<String, String>, qualified_mapped: Dict<String, String>, filtered: Dict<String, Int>, folded: String, tried: Result<Dict<String, String>, String>}\n",
            "  {\n",
            "    mapped: dict_map(table, dict_string),\n",
            "    qualified_mapped: prelude::dict_map(table, qualified_dict_string),\n",
            "    filtered: dict_filter(table, dict_keep),\n",
            "    folded: dict_fold(table, \"\", dict_folder),\n",
            "    tried: dict_try_map(table, dict_try)\n",
            "  }\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for name in [
        "dict_string",
        "qualified_dict_string",
        "dict_keep",
        "dict_try",
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .expect("dictionary callback should be lowered");
        assert_eq!(function.params[0].ty, CoreType::string(), "{name}");
        assert_eq!(function.params[1].ty, CoreType::int(), "{name}");
    }
    let folder = core
        .functions
        .iter()
        .find(|function| function.name == "dict_folder")
        .expect("fold callback should be lowered");
    assert_eq!(folder.params[0].ty, CoreType::string());
    assert_eq!(folder.params[1].ty, CoreType::string());
    assert_eq!(folder.params[2].ty, CoreType::int());
}

#[test]
fn dictionary_prelude_callback_aliases_infer_context_key_and_value_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn map_entry(context, key, value) -> String\n",
            "  let label: String = context\n",
            "  label\n",
            "end\n",
            "fn keep_entry(context, key, value) -> Bool\n",
            "  let minimum: Int = context\n",
            "  let current: Int = value\n",
            "  true\n",
            "end\n",
            "fn fold_entry(context, acc, key, value) -> String\n",
            "  let label: String = context\n",
            "  acc\n",
            "end\n",
            "fn try_entry(context, key, value) -> Result<String, String>\n",
            "  let label: String = context\n",
            "  Ok(label)\n",
            "end\n",
            "pub fn main(table: Dict<String, Int>) -> {mapped: Dict<String, String>, filtered: Dict<String, Int>, folded: String, tried: Result<Dict<String, String>, String>}\n",
            "  {\n",
            "    mapped: dict_map_with(\"ctx\", table, map_entry),\n",
            "    filtered: dict_filter_with(3, table, keep_entry),\n",
            "    folded: dict_fold_with(\"ctx\", table, \"\", fold_entry),\n",
            "    tried: dict_try_map_with(\"ctx\", table, try_entry)\n",
            "  }\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for name in ["map_entry", "try_entry"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .expect("dictionary alias callback should be lowered");
        assert_eq!(function.params[0].ty, CoreType::string(), "{name}");
        assert_eq!(function.params[1].ty, CoreType::string(), "{name}");
        assert_eq!(function.params[2].ty, CoreType::int(), "{name}");
    }
    let keep = core
        .functions
        .iter()
        .find(|function| function.name == "keep_entry")
        .expect("filter callback should be lowered");
    assert_eq!(keep.params[0].ty, CoreType::int());
    assert_eq!(keep.params[1].ty, CoreType::string());
    assert_eq!(keep.params[2].ty, CoreType::int());
    let fold = core
        .functions
        .iter()
        .find(|function| function.name == "fold_entry")
        .expect("fold callback should be lowered");
    assert_eq!(fold.params[0].ty, CoreType::string());
    assert_eq!(fold.params[1].ty, CoreType::string());
    assert_eq!(fold.params[2].ty, CoreType::string());
    assert_eq!(fold.params[3].ty, CoreType::int());
}

#[test]
fn compiler_adapter_helpers_report_user_call_site_diagnostics() {
    for (helper, value_type, return_type, expected_callback) in [
        ("vec_map", "Vec<Int>", "Vec<String>", "fn(Int) -> String"),
        ("vec_filter", "Vec<Int>", "Vec<Int>", "fn(Int) -> Bool"),
        (
            "option_map",
            "Option<Int>",
            "Option<String>",
            "fn(Int) -> String",
        ),
        (
            "option_and_then",
            "Option<Int>",
            "Option<String>",
            "fn(Int) -> Option<String>",
        ),
        (
            "result_map",
            "Result<Int, String>",
            "Result<String, String>",
            "fn(Int) -> String",
        ),
        (
            "result_map_err",
            "Result<String, Int>",
            "Result<String, String>",
            "fn(Int) -> String",
        ),
        (
            "result_and_then",
            "Result<Int, String>",
            "Result<String, String>",
            "fn(Int) -> Result<String, String>",
        ),
        (
            "vec_try_map",
            "Vec<Int>",
            "Result<Vec<String>, String>",
            "fn(Int) -> Result<String, String>",
        ),
        ("list_map", "List<Int>", "List<String>", "fn(Int) -> String"),
        ("list_filter", "List<Int>", "List<Int>", "fn(Int) -> Bool"),
        (
            "list_try_map",
            "List<Int>",
            "Result<List<String>, String>",
            "fn(Int) -> Result<String, String>",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                concat!(
                    "type List<A>\n",
                    "  Nil\n",
                    "  Cons(head: A, tail: List<A>)\n",
                    "end\n",
                    "fn to_int(value: Int) -> Int\n",
                    "  value\n",
                    "end\n",
                    "pub fn main(value: {}) -> {}\n",
                    "  {}(value, to_int)\n",
                    "end\n",
                ),
                value_type, return_type, helper
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1, "{helper}");
        assert_eq!(diagnostics[0].id, "type.mismatch");
        assert_eq!(
            diagnostics[0].message,
            format!("expected `{expected_callback}`, but found `fn(Int) -> Int`")
        );
        let span = diagnostics[0]
            .span
            .as_ref()
            .expect("diagnostic should point at user source");
        assert_eq!(span.file.as_str(), "main.veln");
    }
}

#[test]
fn dictionary_prelude_callbacks_report_user_call_site_diagnostics() {
    for (helper, return_type, callback_source, expected_callback) in [
        (
            "dict_map",
            "Dict<String, String>",
            concat!(
                "fn to_int(key: String, value: Int) -> Int\n",
                "  value\n",
                "end\n",
            ),
            "fn(String, Int) -> String",
        ),
        (
            "dict_filter",
            "Dict<String, Int>",
            concat!(
                "fn to_int(key: String, value: Int) -> Int\n",
                "  value\n",
                "end\n",
            ),
            "fn(String, Int) -> Bool",
        ),
        (
            "dict_try_map",
            "Result<Dict<String, String>, String>",
            concat!(
                "fn to_int(key: String, value: Int) -> Int\n",
                "  value\n",
                "end\n",
            ),
            "fn(String, Int) -> Result<String, String>",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                concat!(
                    "{}",
                    "pub fn main(value: Dict<String, Int>) -> {}\n",
                    "  {}(value, to_int)\n",
                    "end\n",
                ),
                callback_source, return_type, helper
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1, "{helper}");
        assert_eq!(diagnostics[0].id, "type.mismatch");
        assert_eq!(
            diagnostics[0].message,
            format!("expected `{expected_callback}`, but found `fn(String, Int) -> Int`")
        );
        let span = diagnostics[0]
            .span
            .as_ref()
            .expect("diagnostic should point at user source");
        assert_eq!(span.file.as_str(), "main.veln");
    }
}

#[test]
fn dictionary_prelude_helpers_check_keys_and_values_from_input_dict() {
    for (helper, source_text, expected_message) in [
        (
            "dict_contains",
            concat!(
                "pub fn main(table: Dict<Int, String>) -> Bool\n",
                "  dict_contains(table, \"key\")\n",
                "end\n",
            ),
            "expected `Int`, but found `String`",
        ),
        (
            "dict_get",
            concat!(
                "pub fn main(table: Dict<Int, String>) -> Option<String>\n",
                "  dict_get(table, \"key\")\n",
                "end\n",
            ),
            "expected `Int`, but found `String`",
        ),
        (
            "dict_insert",
            concat!(
                "pub fn main(table: Dict<String, Int>) -> Dict<String, Int>\n",
                "  dict_insert(table, \"key\", \"bad\")\n",
                "end\n",
            ),
            "expected `Int`, but found `String`",
        ),
        (
            "dict_remove",
            concat!(
                "pub fn main(table: Dict<Int, String>) -> Dict<Int, String>\n",
                "  dict_remove(table, \"key\")\n",
                "end\n",
            ),
            "expected `Int`, but found `String`",
        ),
    ] {
        let source = SourceFile::new("main.veln", source_text);
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1, "{helper}");
        assert_eq!(diagnostics[0].id, "type.mismatch", "{helper}");
        assert_eq!(diagnostics[0].message, expected_message, "{helper}");
        let span = diagnostics[0]
            .span
            .as_ref()
            .expect("diagnostic should point at user source");
        assert_eq!(span.file.as_str(), "main.veln");
    }
}
