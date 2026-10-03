use super::*;
use veln_core::CoreCallbackTarget;
use veln_ir::IrCallbackTarget;

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
    let CoreExprKind::Call { target, args } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(
        target,
        CoreCallTarget::CallbackBoundary {
            target: CoreCallbackTarget::PreludeBuiltin(name),
            callsite,
        } if name == "vec_map" && matches!(callsite.kind, CoreExprKind::Record(_))
    ));
    assert!(matches!(
        &args[1].kind,
        CoreExprKind::FunctionValue { name, callsite: false } if name == "stringify"
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
    let IrExprKind::Call { target, args } = &value.kind else {
        panic!("tail expression should lower as IR call");
    };
    assert!(matches!(
        target,
        IrCallTarget::CallbackBoundary {
            target: IrCallbackTarget::PreludeBuiltin(name),
            callsite,
        } if name == "vec_map" && matches!(callsite.kind, IrExprKind::Record(_))
    ));
    assert!(matches!(
        &args[1].kind,
        IrExprKind::FunctionValue { name, callsite: false } if name == "stringify"
    ));
}

#[test]
fn runtime_callback_boundaries_forward_callsite_wrapper_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn add(total: Int, item: Int) -> Int callsite\n",
            "  total + item + callsite.start_line\n",
            "end\n",
            "pub fn fold(items: Vec<Int>) -> Int callsite\n",
            "  vec_fold(items, 0, add)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_project_reachable_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let fold = core
        .functions
        .iter()
        .find(|function| function.name == "fold")
        .expect("wrapper should be lowered");
    let CoreStmtKind::Return { expr } = &fold.body[0].kind else {
        panic!("runtime callback call should lower as a return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::CallbackBoundary {
                target: CoreCallbackTarget::PreludeBuiltin(name),
                callsite,
            },
            args,
        } if name == "vec_fold"
            && matches!(callsite.kind, CoreExprKind::Local(ref name) if name == "callsite")
            && matches!(&args[2].kind, CoreExprKind::FunctionValue { name, callsite: true } if name == "add")
    ));

    let ir = lowered.ir.expect("complete core should lower to IR");
    let fold = ir
        .functions
        .iter()
        .find(|function| function.name == "fold")
        .expect("wrapper should reach IR");
    let IrStmtKind::Return { value } = &fold.body[0].kind else {
        panic!("runtime callback call should reach IR");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::CallbackBoundary {
                target: IrCallbackTarget::PreludeBuiltin(name),
                callsite,
            },
            args,
        } if name == "vec_fold"
            && matches!(callsite.kind, IrExprKind::Local(ref name) if name == "callsite")
            && matches!(&args[2].kind, IrExprKind::FunctionValue { name, callsite: true } if name == "add")
    ));
}

#[test]
fn callsite_wrapper_forwards_context_after_explicit_function_value_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn located(value: Int) -> {value: Int, location: SourceLocation} callsite\n",
            "  {value: value, location: callsite}\n",
            "end\n",
            "fn forward(value: Int) -> {value: Int, location: SourceLocation} callsite\n",
            "  let stored: fn(Int) -> {value: Int, location: SourceLocation} = located\n",
            "  stored(value)\n",
            "end\n",
            "pub fn main() -> {value: Int, location: SourceLocation}\n",
            "  forward(37)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_project_reachable_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let forward = core
        .functions
        .iter()
        .find(|function| function.name == "forward")
        .expect("wrapper should be lowered");
    assert!(forward.callsite);
    assert_eq!(forward.params.len(), 2);
    assert_eq!(forward.params[0].name, "value");
    assert_eq!(forward.params[1].name, "callsite");
    let CoreStmtKind::Let { expr: stored, .. } = &forward.body[0].kind else {
        panic!("stored function should lower as a let binding");
    };
    assert!(matches!(
        stored.kind,
        CoreExprKind::FunctionValue {
            ref name,
            callsite: true,
        } if name == "located"
    ));
    let CoreType::Function { params, .. } = &stored.ty else {
        panic!("stored value should retain its callable type");
    };
    assert_eq!(params, &[CoreType::int()]);
    let CoreStmtKind::Return { expr: invoked } = &forward.body[1].kind else {
        panic!("indirect invocation should lower as a return");
    };
    assert!(matches!(
        &invoked.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::CallsiteValue { name, callsite },
            args,
        } if name == "stored"
            && matches!(callsite.kind, CoreExprKind::Local(ref name) if name == "callsite")
            && matches!(args.as_slice(), [arg] if matches!(arg.kind, CoreExprKind::Local(ref name) if name == "value"))
    ));

    let ir = lowered.ir.expect("complete core should lower to IR");
    let forward = ir
        .functions
        .iter()
        .find(|function| function.name == "forward")
        .expect("wrapper should reach IR");
    assert!(forward.callsite);
    assert_eq!(forward.params.len(), 2);
    let IrStmtKind::Return { value: invoked } = &forward.body[1].kind else {
        panic!("indirect invocation should reach IR");
    };
    assert!(matches!(
        &invoked.kind,
        IrExprKind::Call {
            target: IrCallTarget::CallsiteValue { name, callsite },
            args,
        } if name == "stored"
            && matches!(callsite.kind, IrExprKind::Local(ref name) if name == "callsite")
            && matches!(args.as_slice(), [arg] if matches!(arg.kind, IrExprKind::Local(ref name) if name == "value"))
    ));
}

#[test]
fn callsite_function_values_preserve_metadata_and_lower_indirect_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn located() -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "fn invoke(callback: fn() -> SourceLocation) -> SourceLocation\n",
            "  callback()\n",
            "end\n",
            "pub fn main() -> SourceLocation\n",
            "  let stored: fn() -> SourceLocation = located\n",
            "  invoke(stored)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_project_reachable_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let located = core
        .functions
        .iter()
        .find(|function| function.name == "located")
        .expect("callsite function should be lowered");
    assert!(located.callsite);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("stored function should lower as a let binding");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::FunctionValue { name, callsite: true } if name == "located"
    ));
    let invoke = core
        .functions
        .iter()
        .find(|function| function.name == "invoke")
        .expect("invoke should be lowered");
    let CoreStmtKind::Return { expr } = &invoke.body[0].kind else {
        panic!("indirect invocation should lower as a return");
    };
    let CoreExprKind::Call { target, args } = &expr.kind else {
        panic!("indirect invocation should lower as a call");
    };
    assert!(
        args.is_empty(),
        "hidden context must not enter source arguments"
    );
    assert!(matches!(
        target,
        CoreCallTarget::CallsiteValue { name, callsite }
            if name == "callback" && matches!(callsite.kind, CoreExprKind::Record(_))
    ));

    let ir = lowered.ir.expect("complete core should lower to IR");
    let located = ir
        .functions
        .iter()
        .find(|function| function.name == "located")
        .expect("callsite function should reach IR");
    assert!(located.callsite);
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should reach IR");
    let IrStmtKind::Let { value, .. } = &main.body[0].kind else {
        panic!("stored function should reach IR");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::FunctionValue { name, callsite: true } if name == "located"
    ));
    let invoke = ir
        .functions
        .iter()
        .find(|function| function.name == "invoke")
        .expect("invoke should reach IR");
    let IrStmtKind::Return { value } = &invoke.body[0].kind else {
        panic!("indirect invocation should reach IR");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::CallsiteValue { name, callsite },
            args,
        } if name == "callback"
            && args.is_empty()
            && matches!(callsite.kind, IrExprKind::Record(_))
    ));
}

#[test]
fn callsite_function_value_hidden_context_does_not_change_arity_diagnostics() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn located() -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "pub fn main() -> SourceLocation\n",
            "  let stored: fn() -> SourceLocation = located\n",
            "  stored(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "core.call_arity_mismatch")
        .expect("source arity mismatch should be reported");
    assert_eq!(diagnostic.message, "call expects 0 argument(s), but got 1");
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
    let CoreExprKind::Call { target, args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(
        target,
        CoreCallTarget::CallsiteValue { name, callsite }
            if name == "stringify" && matches!(callsite.kind, CoreExprKind::Record(_))
    ));
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
fn begin_body_conflicting_callback_return_constraints_converge() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn empty_items(value: Int)\n",
            "  []\n",
            "end\n",
            "pub fn main() -> ()\n",
            "  begin\n",
            "    let ints: Vec<Vec<Int>> = vec_map([1], empty_items)\n",
            "    let strings: Vec<Vec<String>> = vec_map([1], empty_items)\n",
            "    ()\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `fn(Int) -> Vec<String>`, but found `fn(Int) -> Vec<Int>`"
    );
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
