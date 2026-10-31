use super::*;

fn module(source: &str) -> SurfaceModule {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

#[test]
fn effect_row_substitution_reaches_concrete_call_boundaries() {
    let module = module(concat!(
        "effect Transport\n",
        "\tread() -> Int\n",
        "end\n",
        "\n",
        "fn pure_callback() -> Int\n",
        "\t1\n",
        "end\n",
        "\n",
        "fn db_callback() -> Int effects [db]\n",
        "\t2\n",
        "end\n",
        "\n",
        "fn connection<effect E>(callback: fn() -> Int effects [...E]) -> Int effects [Transport, ...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "pub fn pure_missing_transport() -> Int\n",
        "\tconnection(pure_callback)\n",
        "end\n",
        "\n",
        "pub fn db_missing_db() -> Int effects [Transport]\n",
        "\tconnection(db_callback)\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `Transport`"
    );
    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `db`"
    );
}

#[test]
fn duplicate_concrete_effect_is_reported_once_after_row_substitution() {
    let module = module(concat!(
        "fn stdio_callback() -> Int effects [stdio]\n",
        "\t1\n",
        "end\n",
        "\n",
        "fn boundary<effect E>(callback: fn() -> Int effects [...E]) -> Int effects [stdio, ...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "pub fn missing_stdio() -> Int\n",
        "\tboundary(stdio_callback)\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
}

#[test]
fn effect_boundary_policy_distinguishes_public_test_and_private_functions() {
    let module = module(concat!(
        "pub fn public_boundary() -> ()\n",
        "\tstdio::println(\"public\")\n",
        "end\n",
        "\n",
        "test test_boundary() -> ()\n",
        "\tstdio::println(\"test\")\n",
        "end\n",
        "\n",
        "fn private_helper() -> ()\n",
        "\tstdio::println(\"private\")\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    assert_eq!(diagnostics[1].id, "effect.missing_test");
    assert_eq!(
        diagnostics[1].message,
        "test declaration uses undeclared effect `stdio`"
    );
}

#[test]
fn function_type_compatibility_accepts_pure_and_effectful_callbacks() {
    let module = module(concat!(
        "fn pure_callback() -> Int\n",
        "\t1\n",
        "end\n",
        "\n",
        "fn stdio_callback() -> Int effects [stdio]\n",
        "\t2\n",
        "end\n",
        "\n",
        "fn call<effect E>(callback: fn() -> Int effects [...E]) -> Int effects [...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "pub fn pure_ok() -> Int\n",
        "\tcall(pure_callback)\n",
        "end\n",
        "\n",
        "pub fn stdio_ok() -> Int effects [stdio]\n",
        "\tcall(stdio_callback)\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn concrete_handler_replacement_preserves_row_effects() {
    let module = module(concat!(
        "effect Transport\n",
        "\tread() -> Int\n",
        "end\n",
        "\n",
        "fn provide() -> Int\n",
        "\t1\n",
        "end\n",
        "\n",
        "handler transport() for Transport effects [net]\n",
        "\tread() => provide()\n",
        "end\n",
        "\n",
        "fn db_callback() -> Int effects [db]\n",
        "\t2\n",
        "end\n",
        "\n",
        "fn connection<effect E>(callback: fn() -> Int effects [...E]) -> Int effects [Transport, ...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "pub fn handled_missing_db() -> Int effects [net]\n",
        "\thandle connection(db_callback) with transport()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `db`"
    );
}

#[test]
fn handler_arguments_keep_effects_outside_the_handled_body() {
    let module = module(concat!(
        "effect Ask\n",
        "\tvalue() -> Int\n",
        "end\n",
        "\n",
        "effect Audit\n",
        "\toffset() -> Int\n",
        "end\n",
        "\n",
        "fn provide(offset: Int) -> Int\n",
        "\toffset\n",
        "end\n",
        "\n",
        "handler ask(offset: Int) for Ask\n",
        "\tvalue() => provide(offset)\n",
        "end\n",
        "\n",
        "pub fn main() -> Int effects [Audit]\n",
        "\thandle perform Ask::value() with ask(perform Audit::offset())\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_helper_instantiates_effect_row_before_public_caller() {
    let module = module(concat!(
        "effect Transport\n",
        "\tread() -> Int\n",
        "end\n",
        "\n",
        "fn provide() -> Int\n",
        "\t1\n",
        "end\n",
        "\n",
        "handler transport() for Transport effects [net]\n",
        "\tread() => provide()\n",
        "end\n",
        "\n",
        "fn pure_callback() -> Int\n",
        "\t1\n",
        "end\n",
        "\n",
        "fn db_callback() -> Int effects [db]\n",
        "\t2\n",
        "end\n",
        "\n",
        "fn connection<effect E>(callback: fn() -> Int effects [...E]) -> Int effects [Transport, ...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "fn pure_helper() -> Int effects [net]\n",
        "\thandle connection(pure_callback) with transport()\n",
        "end\n",
        "\n",
        "fn db_helper() -> Int effects [net]\n",
        "\thandle connection(db_callback) with transport()\n",
        "end\n",
        "\n",
        "pub fn main() -> Int effects [net]\n",
        "\tpure_helper() + db_helper()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `db`"
    );
    assert!(!diagnostics[0].message.contains("...E"), "{diagnostics:#?}");
}

#[test]
fn invalid_row_syntax_is_rejected() {
    let module = module(concat!(
        "fn unbound() -> Int effects [...E]\n",
        "\t1\n",
        "end\n",
        "\n",
        "fn non_final<effect E>() -> Int effects [...E, stdio]\n",
        "\t1\n",
        "end\n",
        "\n",
        "fn multiple<effect E>() -> Int effects [...E, ...E]\n",
        "\t1\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);
    let ids = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&"effect.row_unbound"), "{diagnostics:#?}");
    assert!(ids.contains(&"effect.row_non_final"), "{diagnostics:#?}");
    assert!(ids.contains(&"effect.row_multiple"), "{diagnostics:#?}");
}

#[test]
fn checked_core_and_typed_ir_preserve_effect_row_representation() {
    let module = module(concat!(
        "effect Transport\n",
        "\tread() -> Int\n",
        "end\n",
        "\n",
        "fn connection<effect E>(callback: fn() -> Int effects [...E]) -> Int effects [Transport, ...E]\n",
        "\tcallback()\n",
        "end\n",
    ));

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let function = core
        .functions
        .iter()
        .find(|function| function.name == "connection")
        .expect("connection core function");
    assert_eq!(
        function.effects,
        vec!["Transport".to_string(), "...E".to_string()]
    );
    let CoreType::Function { effects, .. } = &function.params[0].ty else {
        panic!("expected callback parameter to lower as function type");
    };
    assert_eq!(effects, &vec!["...E".to_string()]);

    let ir = lowered.ir.expect("typed ir should be built");
    let function = ir
        .functions
        .iter()
        .find(|function| function.name == "connection")
        .expect("connection ir function");
    assert_eq!(
        function.effects,
        vec!["Transport".to_string(), "...E".to_string()]
    );
}

#[test]
fn call_bound_effect_rows_solve_shared_lower_and_upper_bounds() {
    let module = module(concat!(
        "fn stdio_callback() -> () effects [stdio]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn accepts_stdio_or_db(callback: fn() -> () effects [stdio, db]) -> ()\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn lower_then_upper<effect E>(produce: fn() -> () effects [...E], consume: fn(fn() -> () effects [...E]) -> ()) -> () effects [fs, ...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn upper_then_lower<effect E>(consume: fn(fn() -> () effects [...E]) -> (), produce: fn() -> () effects [...E]) -> () effects [fs, ...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "pub fn first_order() -> () effects [fs, stdio]\n",
        "\tlower_then_upper(stdio_callback, accepts_stdio_or_db)\n",
        "end\n",
        "\n",
        "pub fn reverse_order() -> () effects [fs, stdio]\n",
        "\tupper_then_lower(accepts_stdio_or_db, stdio_callback)\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn unconstrained_call_bound_effect_row_infers_empty_set() {
    let module = module(concat!(
        "fn unconstrained<effect E>() -> () effects [...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\tunconstrained()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn unsatisfied_effect_row_upper_bound_rejects_without_leaking_effects() {
    let module = module(concat!(
        "fn network_callback() -> () effects [net]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn accepts_stdio(callback: fn() -> () effects [stdio]) -> ()\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn constrained<effect E>(produce: fn() -> () effects [...E], consume: fn(fn() -> () effects [...E]) -> ()) -> () effects [...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "pub fn rejected_call_is_transactional() -> ()\n",
        "\tconstrained(network_callback, accepts_stdio)\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert!(
        !diagnostics[0].message.contains("missing"),
        "{diagnostics:#?}"
    );
}

#[test]
fn rejected_non_row_payload_does_not_leak_a_partial_row_substitution() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn invoke<effect E>(callback: fn() -> () effects [...E], count: Int) -> () effects [...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "fn rejected_helper() -> ()\n",
        "\tinvoke(db_callback, \"wrong\")\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\trejected_helper()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch");
}

#[test]
fn rejected_call_shape_does_not_leak_a_partial_row_substitution() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn invoke<effect E>(callback: fn() -> () effects [...E], count: Int) -> () effects [...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "fn missing_argument() -> ()\n",
        "\tinvoke(db_callback)\n",
        "end\n",
        "\n",
        "fn unresolved_non_row_argument() -> ()\n",
        "\tinvoke(db_callback, { value: 1 })\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\tmissing_argument()\n",
        "\tunresolved_non_row_argument()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "effect.missing_public"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn variadic_call_bound_effect_rows_contribute_every_argument() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn stdio_callback() -> () effects [stdio]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn invoke_all<effect E>(callbacks: ...fn() -> () effects [...E]) -> () effects [...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn helper() -> ()\n",
        "\tinvoke_all(db_callback, stdio_callback)\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\thelper()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);
    let messages = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "effect.missing_public")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        messages,
        vec![
            "public function uses undeclared effect `db`",
            "public function uses undeclared effect `stdio`",
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn enclosing_effect_rows_remain_rigid_during_body_checking() {
    let module = module(concat!(
        "fn call_once<effect E>(callback: fn() -> () effects [...E]) -> () effects [...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "fn rigid<effect R>(callback: fn() -> () effects [...R]) -> () effects [...R]\n",
        "\tcall_once(callback)\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn accepted_record_argument_preserves_instantiated_call_effects() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn invoke<effect E>(callback: fn() -> () effects [...E], config: {count: Int}) -> () effects [...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "fn through_record() -> ()\n",
        "\tinvoke(db_callback, {count: 1})\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\tthrough_record()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `db`"
    );
}

#[test]
fn accepted_call_expression_argument_preserves_instantiated_private_call_effects() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn make_int() -> Int\n",
        "\t1\n",
        "end\n",
        "\n",
        "fn invoke<effect E>(callback: fn() -> () effects [...E], count: Int) -> () effects [...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "fn through_call() -> ()\n",
        "\tinvoke(db_callback, make_int())\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\tthrough_call()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `db`"
    );
}

#[test]
fn solved_effect_row_instantiates_the_call_result_type() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn effect_identity<effect E>(callback: fn() -> () effects [...E]) -> fn() -> () effects [...E]\n",
        "\tcallback\n",
        "end\n",
        "\n",
        "fn accept_db(callback: fn() -> () effects [db]) -> ()\n",
        "\t()\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\taccept_db(effect_identity(db_callback))\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn return_only_call_bound_row_instantiates_empty_for_declared_and_local_calls() {
    let module = module(concat!(
        "fn pure_callback() -> ()\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn make<effect E>() -> fn() -> () effects [...E]\n",
        "\tpure_callback\n",
        "end\n",
        "\n",
        "fn accept_pure(callback: fn() -> ()) -> ()\n",
        "\t()\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\taccept_pure(make())\n",
        "\tlet local = make\n",
        "\taccept_pure(local())\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn nested_option_payload_contributes_to_the_shared_effect_row() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn invoke_optional<effect E>(callback: Option<fn() -> () effects [...E]>) -> () effects [...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "pub fn main() -> () effects [db]\n",
        "\tinvoke_optional(Some(db_callback))\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn nested_result_and_source_adt_payloads_contribute_to_private_effect_inference() {
    let module = module(concat!(
        "type CallbackBox<A>\n",
        "\tWrapped(value: A)\n",
        "end\n",
        "\n",
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn invoke_result<effect E>(callback: Result<fn() -> () effects [...E], String>) -> () effects [...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn invoke_box<effect E>(callback: CallbackBox<fn() -> () effects [...E]>) -> () effects [...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn private_helper() -> ()\n",
        "\tinvoke_result(Result::Ok(db_callback))\n",
        "\tinvoke_box(CallbackBox::Wrapped(db_callback))\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\tprivate_helper()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);
    let missing_effects = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "effect.missing_public")
        .collect::<Vec<_>>();

    assert_eq!(missing_effects.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        missing_effects[0].message,
        "public function uses undeclared effect `db`"
    );
}

#[test]
fn enclosing_effect_row_cannot_satisfy_a_concrete_upper_bound() {
    let module = module(concat!(
        "fn accepts_pure(callback: fn() -> ()) -> ()\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn constrained<effect E>(produce: fn() -> () effects [...E], consume: fn(fn() -> () effects [...E]) -> ()) -> () effects [...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn rigid<effect R>(callback: fn() -> () effects [...R]) -> () effects [...R]\n",
        "\tconstrained(callback, accepts_pure)\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch");
}

#[test]
fn local_alias_preserves_declared_call_bound_row_provenance() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn invoke<effect E>(callback: fn() -> () effects [...E]) -> () effects [...E]\n",
        "\tcallback()\n",
        "end\n",
        "\n",
        "fn through_alias() -> ()\n",
        "\tlet local = invoke\n",
        "\tlocal(db_callback)\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\tthrough_alias()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `db`"
    );
}

#[test]
fn rejected_nested_payload_does_not_publish_a_partial_row_solution() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn invoke_optional<effect E>(callback: Option<fn() -> () effects [...E]>, count: Int) -> () effects [...E]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn rejected_helper() -> ()\n",
        "\tinvoke_optional(Some(db_callback), {count: 1})\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\trejected_helper()\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "effect.missing_public"),
        "{diagnostics:#?}"
    );
}

#[test]
fn rejected_row_bearing_result_recovers_without_publishing_type_or_effects() {
    let module = module(concat!(
        "fn db_callback() -> () effects [db]\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn produce<effect E>(callback: fn() -> () effects [...E], count: Int) -> fn() -> () effects [...E] effects [...E]\n",
        "\tcallback\n",
        "end\n",
        "\n",
        "fn accepts_pure(callback: fn() -> ()) -> ()\n",
        "\t()\n",
        "end\n",
        "\n",
        "pub fn main() -> ()\n",
        "\tlet recovered = produce(db_callback, \"wrong\")\n",
        "\taccepts_pure(recovered)\n",
        "end\n",
    ));

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "effect.missing_public"),
        "{diagnostics:#?}"
    );
}
