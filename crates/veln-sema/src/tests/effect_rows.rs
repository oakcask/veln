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
        "handler transport() handles Transport effects [net]\n",
        "\tread = provide\n",
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
        "handler transport() handles Transport effects [net]\n",
        "\tread = provide\n",
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
