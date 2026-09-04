use super::*;

#[test]
fn lowers_function_declarations_as_callable_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn main(items: Vec<Int>) -> Vec<String>\n",
            "  vec_map(items, stringify)\n",
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
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(
        &args[1].kind,
        CoreExprKind::FunctionValue(name) if name == "stringify"
    ));
    assert_eq!(
        args[1].ty,
        CoreType::Function {
            params: vec![CoreType::int()],
            variadic: None,
            return_type: Box::new(CoreType::string()),
            effects: Vec::new()
        }
    );

    let ir = lowered.ir.expect("complete core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Call { args, .. } = &value.kind else {
        panic!("tail expression should lower as IR call");
    };
    assert!(matches!(
        &args[1].kind,
        IrExprKind::FunctionValue(name) if name == "stringify"
    ));
}

#[test]
fn lowers_function_return_types_with_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn printer(text: String) -> () effects [stdio]\n",
            "  stdio::println(text)\n",
            "  ()\n",
            "end\n",
            "pub fn callback_factory() -> fn(String) -> () effects [stdio]\n",
            "  printer\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let factory = core
        .functions
        .iter()
        .find(|function| function.name == "callback_factory")
        .expect("factory should be lowered");
    assert_eq!(
        factory.return_type,
        CoreType::Function {
            params: vec![CoreType::string()],
            variadic: None,
            return_type: Box::new(CoreType::unit()),
            effects: vec!["stdio".to_string()],
        }
    );
    assert_eq!(factory.effects, Vec::<String>::new());
}

#[test]
fn function_return_effects_must_cover_actual_callable_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn printer(text: String) -> () effects [stdio]\n",
            "  stdio::println(text)\n",
            "  ()\n",
            "end\n",
            "pub fn callback_factory() -> fn(String) -> ()\n",
            "  printer\n",
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
        "expected `fn(String) -> ()`, but found `fn(String) -> () effects [stdio]`"
    );
}

#[test]
fn call_resolution_prefers_local_callable_over_function_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String\n",
            "  \"function\"\n",
            "end\n",
            "pub fn main(stringify: fn(Int) -> String effects []) -> String\n",
            "  stringify(1)\n",
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
    let CoreExprKind::Call { target, args } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert_eq!(target, &CoreCallTarget::Value("stringify".to_string()));
    assert!(matches!(&args[0].kind, CoreExprKind::IntLiteral(value) if value == "1"));
}

#[test]
fn non_callable_local_shadow_blocks_function_call_resolution() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String\n",
            "  \"function\"\n",
            "end\n",
            "pub fn main(stringify: Int) -> String\n",
            "  stringify(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `stringify`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn lowers_record_field_access_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String\n",
            "  let payload: {name: String, count: Int} = {name: \"veln\", count: 1}\n",
            "  payload.name\n",
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
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::FieldAccess { field, .. } if field == "name"
    ));
    assert_eq!(expr.ty, CoreType::string());

    let ir = lowered.ir.expect("complete core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[1].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::FieldAccess { field, .. } if field == "name"
    ));
}

#[test]
fn reports_missing_record_field_access() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Int\n",
            "  let payload: {count: Int} = {count: 1}\n",
            "  payload.name\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.field_missing");
    assert_eq!(
        diagnostics[0].message,
        "type `{count: Int}` has no field `name`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn prelude_helpers_check_direct_expected_return_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Option<Int>) -> Int\n",
            "  option_unwrap_or(value, \"bad\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
}

#[test]
fn prelude_helper_result_context_refines_empty_callback_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn empty_vec_callback(value: Int)\n",
            "  []\n",
            "end\n",
            "pub fn main() -> Vec<Vec<Int>>\n",
            "  vec_map([1], empty_vec_callback)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let callback = core
        .functions
        .iter()
        .find(|function| function.name == "empty_vec_callback")
        .expect("callback should be lowered");
    assert_eq!(callback.return_type, CoreType::vec(CoreType::int()));
    let CoreStmtKind::Return { expr } = &callback.body[0].kind else {
        panic!("callback tail should lower as return");
    };
    assert_eq!(expr.ty, CoreType::vec(CoreType::int()));
}

#[test]
fn prelude_helper_result_context_reaches_callback_control_flow_branches() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose_empty_items(value: Int)\n",
            "  if value == 0\n",
            "    []\n",
            "  else if value == 1\n",
            "    []\n",
            "  else\n",
            "    []\n",
            "  end\n",
            "end\n",
            "pub fn main() -> Vec<Vec<String>>\n",
            "  vec_map([0, 1, 2], choose_empty_items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let callback = core
        .functions
        .iter()
        .find(|function| function.name == "choose_empty_items")
        .expect("callback should be lowered");
    assert_eq!(callback.return_type, CoreType::vec(CoreType::string()));
}

#[test]
fn prelude_helper_result_context_refines_non_empty_callback_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn optional_items(value: Int)\n",
            "  Some([])\n",
            "end\n",
            "fn tried_items(value: Int)\n",
            "  Ok({items: []})\n",
            "end\n",
            "fn error_items(value: Int)\n",
            "  Err([])\n",
            "end\n",
            "fn dict_items(value: Int)\n",
            "  {\"one\": 1}\n",
            "end\n",
            "pub fn main() -> {optional: Vec<Option<Vec<String>>>, tried: Result<Vec<{items: Vec<String>}>, String>, error: Vec<Result<String, Vec<String>>>, dict: Vec<Dict<String, Int>>}\n",
            "  {\n",
            "    optional: vec_map([1], optional_items),\n",
            "    tried: vec_try_map([1], tried_items),\n",
            "    error: vec_map([1], error_items),\n",
            "    dict: vec_map([1], dict_items)\n",
            "  }\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let optional = core
        .functions
        .iter()
        .find(|function| function.name == "optional_items")
        .expect("optional callback should be lowered");
    assert_eq!(
        optional.return_type,
        CoreType::option(CoreType::vec(CoreType::string()))
    );
    let tried = core
        .functions
        .iter()
        .find(|function| function.name == "tried_items")
        .expect("try callback should be lowered");
    assert_eq!(
        tried.return_type,
        CoreType::result(
            CoreType::Record(vec![(
                "items".to_string(),
                CoreType::vec(CoreType::string())
            )]),
            CoreType::string(),
        )
    );
    let error = core
        .functions
        .iter()
        .find(|function| function.name == "error_items")
        .expect("error callback should be lowered");
    assert_eq!(
        error.return_type,
        CoreType::result(CoreType::string(), CoreType::vec(CoreType::string()))
    );
    let dict = core
        .functions
        .iter()
        .find(|function| function.name == "dict_items")
        .expect("dict callback should be lowered");
    assert_eq!(
        dict.return_type,
        CoreType::dict(CoreType::string(), CoreType::int())
    );
}

#[test]
fn prelude_helper_result_context_reports_conflicting_callback_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bad_optional(value: Int)\n",
            "  Some([1])\n",
            "end\n",
            "pub fn main() -> Vec<Option<Vec<String>>>\n",
            "  vec_map([1], bad_optional)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `String`, but found `Int`");
}

#[test]
fn prelude_helper_input_types_infer_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "\n",
            "fn vec_string(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn qualified_vec_string(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn vec_keep(value) -> Bool\n",
            "  true\n",
            "end\n",
            "fn vec_folder(acc: String, value) -> String\n",
            "  acc\n",
            "end\n",
            "fn vec_try(value) -> Result<String, String>\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "fn vec_try_with(context, value) -> Result<String, String>\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "fn qualified_vec_try_with(context, value) -> Result<String, String>\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "fn list_string(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn list_keep(value) -> Bool\n",
            "  true\n",
            "end\n",
            "fn list_folder(acc: String, value) -> String\n",
            "  acc\n",
            "end\n",
            "fn list_try(value) -> Result<String, String>\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "fn option_string(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn option_next(value) -> Option<String>\n",
            "  Some(\"ok\")\n",
            "end\n",
            "fn qualified_option_next(value) -> Option<String>\n",
            "  Some(\"ok\")\n",
            "end\n",
            "fn result_string(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn result_error_string(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn result_next(value) -> Result<String, String>\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "fn qualified_result_next(value) -> Result<String, String>\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "pub fn main(vec: Vec<Int>, list: List<Int>, opt: Option<Int>, res: Result<Int, String>, err: Result<String, Int>) -> {vec_mapped: Vec<String>, qualified_vec_mapped: Vec<String>, vec_filtered: Vec<Int>, vec_folded: String, vec_tried: Result<Vec<String>, String>, vec_tried_with: Result<Vec<String>, String>, qualified_vec_tried_with: Result<Vec<String>, String>, list_mapped: List<String>, list_filtered: List<Int>, list_folded: String, list_tried: Result<List<String>, String>, option_mapped: Option<String>, option_nexted: Option<String>, qualified_option_nexted: Option<String>, result_mapped: Result<String, String>, result_error_mapped: Result<String, String>, result_nexted: Result<String, String>, qualified_result_nexted: Result<String, String>}\n",
            "  {\n",
            "    vec_mapped: vec_map(vec, vec_string),\n",
            "    qualified_vec_mapped: prelude::vec_map(vec, qualified_vec_string),\n",
            "    vec_filtered: vec_filter(vec, vec_keep),\n",
            "    vec_folded: vec_fold(vec, \"\", vec_folder),\n",
            "    vec_tried: vec_try_map(vec, vec_try),\n",
            "    vec_tried_with: vec_try_map_with(\"ctx\", vec, vec_try_with),\n",
            "    qualified_vec_tried_with: prelude::vec_try_map_with(\"ctx\", vec, qualified_vec_try_with),\n",
            "    list_mapped: list_map(list, list_string),\n",
            "    list_filtered: list_filter(list, list_keep),\n",
            "    list_folded: list_fold(list, \"\", list_folder),\n",
            "    list_tried: list_try_map(list, list_try),\n",
            "    option_mapped: option_map(opt, option_string),\n",
            "    option_nexted: option_and_then(opt, option_next),\n",
            "    qualified_option_nexted: prelude::option_and_then(opt, qualified_option_next),\n",
            "    result_mapped: result_map(res, result_string),\n",
            "    result_error_mapped: result_map_err(err, result_error_string),\n",
            "    result_nexted: result_and_then(res, result_next),\n",
            "    qualified_result_nexted: prelude::result_and_then(res, qualified_result_next)\n",
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
        "vec_string",
        "qualified_vec_string",
        "vec_keep",
        "vec_try",
        "list_string",
        "list_keep",
        "list_try",
        "option_string",
        "option_next",
        "qualified_option_next",
        "result_string",
        "result_error_string",
        "result_next",
        "qualified_result_next",
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .expect("callback should be lowered");
        assert_eq!(function.params[0].ty, CoreType::int(), "{name}");
    }
    for name in ["vec_folder", "list_folder"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .expect("fold callback should be lowered");
        assert_eq!(function.params[0].ty, CoreType::string(), "{name}");
        assert_eq!(function.params[1].ty, CoreType::int(), "{name}");
    }
    for name in ["vec_try_with", "qualified_vec_try_with"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .expect("try-map-with callback should be lowered");
        assert_eq!(function.params[0].ty, CoreType::string(), "{name}");
        assert_eq!(function.params[1].ty, CoreType::int(), "{name}");
    }
}
