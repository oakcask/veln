use super::*;

#[test]
fn declared_helpers_infer_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn apply_int(value: Int, callback: fn(Int) -> String) -> String\n",
            "  callback(value)\n",
            "end\n",
            "fn apply_pair(label: String, value: Int, callback: fn(String, Int) -> Bool) -> Bool\n",
            "  callback(label, value)\n",
            "end\n",
            "fn apply_effect(value: String, callback: fn(String) -> () effects [stdio]) -> () effects [stdio]\n",
            "  callback(value)\n",
            "end\n",
            "fn stringify(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn keep_pair(label, value) -> Bool\n",
            "  true\n",
            "end\n",
            "fn emit(value) effects [stdio]\n",
            "  ()\n",
            "end\n",
            "fn ignore(value)\n",
            "  ()\n",
            "end\n",
            "pub fn main() -> {text: String, kept: Bool} effects [stdio]\n",
            "  apply_effect(\"ready\", emit)\n",
            "  apply_effect(\"ready\", ignore)\n",
            "  {text: apply_int(1, stringify), kept: apply_pair(\"one\", 1, keep_pair)}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let stringify = core
        .functions
        .iter()
        .find(|function| function.name == "stringify")
        .expect("callback should be lowered");
    assert_eq!(stringify.params[0].ty, CoreType::int());
    let keep_pair = core
        .functions
        .iter()
        .find(|function| function.name == "keep_pair")
        .expect("pair callback should be lowered");
    assert_eq!(keep_pair.params[0].ty, CoreType::string());
    assert_eq!(keep_pair.params[1].ty, CoreType::int());
    let emit = core
        .functions
        .iter()
        .find(|function| function.name == "emit")
        .expect("effectful callback should be lowered");
    assert_eq!(emit.params[0].ty, CoreType::string());
    assert_eq!(emit.return_type, CoreType::unit());
    let ignore = core
        .functions
        .iter()
        .find(|function| function.name == "ignore")
        .expect("pure callback should be lowered");
    assert_eq!(ignore.params[0].ty, CoreType::string());
    assert_eq!(ignore.return_type, CoreType::unit());
}

#[test]
fn record_field_expected_type_infers_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn use_options(options: {map: fn(Int) -> String}) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn stringify(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn main() -> String\n",
            "  use_options({map: stringify})\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let stringify = core
        .functions
        .iter()
        .find(|function| function.name == "stringify")
        .expect("callback should be lowered");
    assert_eq!(stringify.params[0].ty, CoreType::int());
}

#[test]
fn constructor_payload_expected_type_infers_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type CallbackCarrier\n",
            "  Processor(run: fn(Int) -> String)\n",
            "  OptionalProcessor(run: fn(Int) -> Option<Vec<String>>)\n",
            "end\n",
            "fn stringify(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn some_string(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn ok_string(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn err_string(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn optional_items(value)\n",
            "  Some([])\n",
            "end\n",
            "pub fn main() -> {processor: CallbackCarrier, optional: CallbackCarrier, some: Option<fn(Int) -> String>, ok: Result<fn(Int) -> String, String>, err: Result<String, fn(Int) -> String>}\n",
            "  {\n",
            "    processor: Processor(stringify),\n",
            "    optional: OptionalProcessor(optional_items),\n",
            "    some: Some(some_string),\n",
            "    ok: Ok(ok_string),\n",
            "    err: Err(err_string)\n",
            "  }\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let stringify = core
        .functions
        .iter()
        .find(|function| function.name == "stringify")
        .expect("string callback should be lowered");
    assert_eq!(stringify.params[0].ty, CoreType::int());
    for name in ["some_string", "ok_string", "err_string"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .expect("compiler-owned constructor callback should be lowered");
        assert_eq!(function.params[0].ty, CoreType::int(), "{name}");
    }
    let optional = core
        .functions
        .iter()
        .find(|function| function.name == "optional_items")
        .expect("optional callback should be lowered");
    assert_eq!(optional.params[0].ty, CoreType::int());
    assert_eq!(
        optional.return_type,
        CoreType::option(CoreType::vec(CoreType::string()))
    );
}

#[test]
fn collection_element_expected_type_infers_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type CallbackGroup\n",
            "  Handlers(callbacks: Vec<fn(Int) -> Option<Vec<String>>>)\n",
            "end\n",
            "fn vec_item(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"vec\"])\n",
            "end\n",
            "fn returned_item(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"return\"])\n",
            "end\n",
            "fn local_item(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"local\"])\n",
            "end\n",
            "fn alias_item(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"alias\"])\n",
            "end\n",
            "fn list_item(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"list\"])\n",
            "end\n",
            "fn nested_item(value)\n",
            "  let checked: Int = value\n",
            "  Some([\"nested\"])\n",
            "end\n",
            "fn returned_callbacks() -> Vec<fn(Int) -> Option<Vec<String>>>\n",
            "  [returned_item]\n",
            "end\n",
            "fn local_callbacks() -> Vec<fn(Int) -> Option<Vec<String>>>\n",
            "  let callbacks: Vec<fn(Int) -> Option<Vec<String>>> = [local_item]\n",
            "  callbacks\n",
            "end\n",
            "fn alias_callbacks() -> Vec<fn(Int) -> Option<Vec<String>>>\n",
            "  let callback = alias_item\n",
            "  [callback]\n",
            "end\n",
            "fn list_callbacks() -> List<fn(Int) -> Option<Vec<String>>>\n",
            "  Cons(list_item, Nil)\n",
            "end\n",
            "fn nested_callbacks() -> CallbackGroup\n",
            "  Handlers([nested_item])\n",
            "end\n",
            "pub fn main() -> {direct: Vec<fn(Int) -> Option<Vec<String>>>, returned: Vec<fn(Int) -> Option<Vec<String>>>, local: Vec<fn(Int) -> Option<Vec<String>>>, alias: Vec<fn(Int) -> Option<Vec<String>>>, list: List<fn(Int) -> Option<Vec<String>>>, nested: CallbackGroup}\n",
            "  {direct: [vec_item], returned: returned_callbacks(), local: local_callbacks(), alias: alias_callbacks(), list: list_callbacks(), nested: nested_callbacks()}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for name in [
        "vec_item",
        "returned_item",
        "local_item",
        "alias_item",
        "list_item",
        "nested_item",
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
fn collection_element_callback_expected_type_reports_body_conflict() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bad_vec_item(value)\n",
            "  let checked: Int = value\n",
            "  Some([1])\n",
            "end\n",
            "fn missing_context(value)\n",
            "  \"ok\"\n",
            "end\n",
            "type GenericGroup<A>\n",
            "  GenericHandlers(callbacks: Vec<fn(A, Int) -> String>)\n",
            "end\n",
            "fn missing_generic_context(value, fixed) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn body_conflict() -> Vec<fn(Int) -> Option<Vec<String>>>\n",
            "  [bad_vec_item]\n",
            "end\n",
            "pub fn unconstrained_element() -> String\n",
            "  let callbacks = [missing_context]\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn non_concrete_element() -> String\n",
            "  let group = GenericHandlers([missing_generic_context])\n",
            "  \"ok\"\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 6, "{diagnostics:#?}");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic.message == "expected `String`, but found `Int`"
    }));
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
fn non_concrete_constructor_payload_does_not_infer_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type GenericCarrier<A>\n",
            "  GenericProcessor(run: fn(A, Int) -> String)\n",
            "end\n",
            "fn stringify(value, fixed) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn main() -> String\n",
            "  let processor = GenericProcessor(stringify)\n",
            "  \"ok\"\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.private_inference_incomplete"
                && diagnostic.message == "private parameter `value` has no inferred type"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.private_inference_incomplete"
                && diagnostic.message == "private parameter `fixed` has no inferred type"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.inference_ambiguous"
                && diagnostic.message == "constructor `GenericProcessor` needs type context"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn local_function_binding_infers_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn emit(value) -> () effects [stdio]\n",
            "  ()\n",
            "end\n",
            "fn apply_int(value: Int, callback: fn(Int) -> String) -> String\n",
            "  callback(value)\n",
            "end\n",
            "fn apply_effect(value: String, callback: fn(String) -> () effects [stdio]) -> () effects [stdio]\n",
            "  callback(value)\n",
            "end\n",
            "fn callback_factory() -> fn(Int) -> String\n",
            "  let callback: fn(Int) -> String = stringify\n",
            "  callback\n",
            "end\n",
            "pub fn main() -> {called: String, returned: String} effects [stdio]\n",
            "  let callback: fn(Int) -> String = stringify\n",
            "  let effectful: fn(String) -> () effects [stdio] = emit\n",
            "  apply_effect(\"ready\", effectful)\n",
            "  let returned: fn(Int) -> String = callback_factory()\n",
            "  {called: apply_int(1, callback), returned: returned(2)}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let stringify = core
        .functions
        .iter()
        .find(|function| function.name == "stringify")
        .expect("callback should be lowered");
    assert_eq!(stringify.params[0].ty, CoreType::int());
    let emit = core
        .functions
        .iter()
        .find(|function| function.name == "emit")
        .expect("effectful callback should be lowered");
    assert_eq!(emit.params[0].ty, CoreType::string());
}

#[test]
fn direct_return_infers_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value) -> String\n",
            "  int_to_string(value)\n",
            "end\n",
            "fn emit(value) -> () effects [stdio]\n",
            "  ()\n",
            "end\n",
            "fn callback_factory() -> fn(Int) -> String\n",
            "  stringify\n",
            "end\n",
            "fn effect_callback_factory() -> fn(String) -> () effects [stdio]\n",
            "  emit\n",
            "end\n",
            "pub fn main() -> {text: String, effect: ()} effects [stdio]\n",
            "  let callback: fn(Int) -> String = callback_factory()\n",
            "  let effectful: fn(String) -> () effects [stdio] = effect_callback_factory()\n",
            "  let ignored: () = effectful(\"ready\")\n",
            "  {text: callback(7), effect: ignored}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let stringify = core
        .functions
        .iter()
        .find(|function| function.name == "stringify")
        .expect("callback should be lowered");
    assert_eq!(stringify.params[0].ty, CoreType::int());
    let emit = core
        .functions
        .iter()
        .find(|function| function.name == "emit")
        .expect("effectful callback should be lowered");
    assert_eq!(emit.params[0].ty, CoreType::string());
}

#[test]
fn if_branch_expected_function_type_infers_private_callback_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn then_items(value)\n",
            "  Some([value])\n",
            "end\n",
            "fn else_if_items(value)\n",
            "  Some([value + 1])\n",
            "end\n",
            "fn else_items(value)\n",
            "  Some([value + 2])\n",
            "end\n",
            "fn choose_items(flag: Bool, backup: Bool) -> fn(Int) -> Option<Vec<Int>>\n",
            "  if flag\n",
            "    then_items\n",
            "  else if backup\n",
            "    else_if_items\n",
            "  else\n",
            "    else_items\n",
            "  end\n",
            "end\n",
            "pub fn main() -> Option<Vec<Int>>\n",
            "  let callback: fn(Int) -> Option<Vec<Int>> = choose_items(false, true)\n",
            "  callback(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for name in ["then_items", "else_if_items", "else_items"] {
        let callback = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("{name} should be lowered"));
        assert_eq!(callback.params[0].ty, CoreType::int());
        assert_eq!(
            callback.return_type,
            CoreType::option(CoreType::vec(CoreType::int()))
        );
    }
}

#[test]
fn imported_declared_helpers_infer_private_callback_parameters() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod spec.app\n",
            "use spec.helpers\n",
            "fn stringify(value) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "fn emit(value) effects [stdio]\n",
            "  \"sent\"\n",
            "end\n",
            "fn pure_emit(value)\n",
            "  \"pure\"\n",
            "end\n",
            "pub fn main() -> {plain: String, effectful: String, pure: String} effects [stdio]\n",
            "  {plain: helpers::apply_int(1, stringify), effectful: helpers::apply_effect(emit), pure: helpers::apply_effect(pure_emit)}\n",
            "end\n",
        ),
    );
    let helpers_source = SourceFile::new(
        "helpers.veln",
        concat!(
            "mod spec.helpers\n",
            "pub fn apply_int(value: Int, callback: fn(Int) -> String) -> String\n",
            "  callback(value)\n",
            "end\n",
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
        types: [app.types, helpers.types].concat(),
        functions: [app.functions, helpers.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let stringify = core
        .functions
        .iter()
        .find(|function| function.name == "stringify")
        .expect("callback should be lowered");
    assert_eq!(stringify.params[0].ty, CoreType::int());
    let emit = core
        .functions
        .iter()
        .find(|function| function.name == "emit")
        .expect("effectful callback should be lowered");
    assert_eq!(emit.params[0].ty, CoreType::string());
    assert_eq!(emit.return_type, CoreType::string());
    let pure_emit = core
        .functions
        .iter()
        .find(|function| function.name == "pure_emit")
        .expect("pure callback should be lowered");
    assert_eq!(pure_emit.params[0].ty, CoreType::string());
    assert_eq!(pure_emit.return_type, CoreType::string());
}

#[test]
fn public_alias_effectful_declared_helpers_infer_private_callback_parameters() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod spec.app\n",
            "use spec.api\n",
            "fn emit(value) effects [stdio]\n",
            "  \"sent\"\n",
            "end\n",
            "fn pure_emit(value)\n",
            "  \"pure\"\n",
            "end\n",
            "pub fn main() -> {effectful: String, pure: String} effects [stdio]\n",
            "  {effectful: api::apply_effect(emit), pure: api::apply_effect(pure_emit)}\n",
            "end\n",
        ),
    );
    let api_source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "use spec.impl\n",
            "pub fn apply_effect = impl::apply_effect\n",
        ),
    );
    let impl_source = SourceFile::new(
        "impl.veln",
        concat!(
            "mod spec.impl\n",
            "pub fn apply_effect(callback: fn(String) -> String effects [stdio]) -> String effects [stdio]\n",
            "  callback(\"ready\")\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let api = lower_surface_ast(&parse(&api_source).tree);
    let implementation = lower_surface_ast(&parse(&impl_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses.into_iter().chain(api.uses).collect(),
        aliases: api.aliases,
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        functions: app
            .functions
            .into_iter()
            .chain(implementation.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for name in ["emit", "pure_emit"] {
        let callback = core
            .functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("{name} callback should be lowered"));
        assert_eq!(callback.params[0].ty, CoreType::string(), "{name}");
        assert_eq!(callback.return_type, CoreType::string(), "{name}");
    }
}
