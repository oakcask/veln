use super::*;

#[test]
fn infers_prelude_helper_calls_from_expected_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(items: Vec(Int), other: Vec(Int), table: Dict(String, Int), ",
            "mapper: fn(Int) -> String, keep: fn(Int) -> Bool, folder: fn(String, Int) -> String, ",
            "fallible: fn(Int) -> Result(String, AppError), opt: Option(Int), ",
            "fallible_with: fn(String, Int) -> Result(String, AppError), ",
            "opt_map: fn(Int) -> String, opt_next: fn(Int) -> Option(String), ",
            "res: Result(Int, AppError), err_map: fn(AppError) -> String, ",
            "res_next: fn(Int) -> Result(String, AppError)) -> {",
            "count: Int, empty: Bool, pushed: Vec(Int), joined: Vec(Int), mapped: Vec(String), ",
            "filtered: Vec(Int), folded: String, tried: Result(Vec(String), AppError), ",
            "tried_with: Result(Vec(String), AppError), split: Option({left: String, right: String}), ",
            "parsed: Result(Int, String), rendered: String, ",
            "found: Option(Int), has_key: Bool, inserted: Dict(String, Int), removed: Dict(String, Int), ",
            "opt_mapped: Option(String), opt_nexted: Option(String), opt_value: Int, ",
            "res_mapped: Result(String, AppError), res_err: Result(Int, String), ",
            "res_nexted: Result(String, AppError)} effects []\n",
            "  {count: vec_len(items), empty: vec_is_empty(items), ",
            "pushed: vec_push(items, 1), joined: vec_concat(items, other), ",
            "mapped: vec_map(items, mapper), filtered: vec_filter(items, keep), ",
            "folded: vec_fold(items, \"\", folder), tried: vec_try_map(items, fallible), ",
            "tried_with: vec_try_map_with(\"prefix\", items, fallible_with), ",
            "split: string_split_once(\"sku,2\", \",\"), parsed: string_parse_int(\"2\"), ",
            "rendered: int_to_string(2), ",
            "found: dict_get(table, \"a\"), has_key: dict_contains(table, \"a\"), ",
            "inserted: dict_insert(table, \"b\", 2), removed: dict_remove(table, \"b\"), ",
            "opt_mapped: option_map(opt, opt_map), opt_nexted: option_and_then(opt, opt_next), ",
            "opt_value: option_unwrap_or(opt, 0), res_mapped: result_map(res, opt_map), ",
            "res_err: result_map_err(res, err_map), res_nexted: result_and_then(res, res_next)}\n",
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
    let CoreExprKind::Record(fields) = &expr.kind else {
        panic!("prelude results should be returned in a record");
    };
    let first = fields
        .first()
        .expect("record should contain prelude result fields");
    assert!(matches!(
        &first.expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
    assert!(matches!(first.expr.ty, CoreType::Named { ref name, .. } if name == "Int"));
    let source_backed_prelude_names =
        crate::standard_symbols::source_backed_prelude_names().collect::<Vec<_>>();
    let core_prelude_calls = fields
        .iter()
        .filter_map(|field| match &field.expr.kind {
            CoreExprKind::Call {
                target: CoreCallTarget::PreludeBuiltin(name),
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for name in &source_backed_prelude_names {
        assert!(
            core_prelude_calls.contains(name),
            "{name} should keep prelude core lowering"
        );
    }
    let ir = lowered
        .ir
        .expect("complete prelude core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Record(fields) = &value.kind else {
        panic!("prelude record should lower to IR");
    };
    assert!(matches!(
        &fields[0].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
    let ir_prelude_calls = fields
        .iter()
        .filter_map(|field| match &field.value.kind {
            IrExprKind::Call {
                target: IrCallTarget::PreludeBuiltin(name),
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for name in &source_backed_prelude_names {
        assert!(
            ir_prelude_calls.contains(name),
            "{name} should keep prelude IR lowering"
        );
    }
}

#[test]
fn source_backed_prelude_helper_source_is_embedded_and_checkable() {
    let mut entries = Vec::new();

    for symbol in crate::standard_symbols::source_backed_symbols() {
        let source = symbol.source.expect("source metadata");
        assert_eq!(symbol.name, source.entry);
        entries.push(source.entry);
        let file = SourceFile::new(source.path, source.text);
        let parsed = parse(&file);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics for {}: {:#?}",
            source.path,
            parsed.diagnostics
        );

        let module = lower_surface_ast(&parsed.tree);
        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.is_empty(),
            "unexpected source helper diagnostics for {}: {diagnostics:#?}",
            source.path
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.name.as_deref() == Some(source.entry)),
            "embedded source should define {}",
            source.entry
        );
    }

    let mut expected_entries =
        crate::standard_symbols::source_backed_prelude_names().collect::<Vec<_>>();
    entries.sort_unstable();
    expected_entries.sort_unstable();
    assert_eq!(entries, expected_entries);
}

#[test]
fn compiler_support_source_loads_text_through_standard_fs_subset() {
    let source = crate::standard_symbols::compiler_support_sources()
        .find(|source| source.entry == "load_source_text")
        .expect("compiler support source should be embedded");
    let file = SourceFile::new(source.path, source.text);
    let parsed = parse(&file);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics for {}: {:#?}",
        source.path,
        parsed.diagnostics
    );

    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "unexpected compiler support diagnostics for {}: {:#?}",
        source.path,
        lowered.diagnostics
    );
    let core = lowered.core.expect("compiler support should lower to core");
    let function = core
        .functions
        .iter()
        .find(|function| function.name == source.entry)
        .expect("compiler support entry should lower");
    let CoreStmtKind::Let { expr, .. } = &function.body[0].kind else {
        panic!("first statement should call fs before wrapping the result");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Try(value) if matches!(
            &value.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::StandardLibraryBuiltin(name),
                ..
            } if name == "fs::read_to_string"
        )
    ));
}

#[test]
fn suggests_vec_try_map_for_result_returning_map_callback() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(value: Int) -> Result(String, AppError) effects []\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "pub fn main(items: Vec(Int)) -> Vec(String) effects []\n",
            "  vec_map(items, parse)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("callback type mismatch should be reported");
    assert_eq!(
        diagnostic.message,
        "expected `fn(unknown) -> String`, but found `fn(Int) -> Result(String, AppError)`"
    );
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| { related.to_json().contains("Use `vec_try_map`") })
    );
}

#[test]
fn lowers_function_declarations_as_callable_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String effects []\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn main(items: Vec(Int)) -> Vec(String) effects []\n",
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
            "pub fn callback_factory() -> fn(String) -> () effects [stdio] effects []\n",
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
            "pub fn callback_factory() -> fn(String) -> () effects [] effects []\n",
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
            "fn stringify(value: Int) -> String effects []\n",
            "  \"function\"\n",
            "end\n",
            "pub fn main(stringify: fn(Int) -> String effects []) -> String effects []\n",
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
            "fn stringify(value: Int) -> String effects []\n",
            "  \"function\"\n",
            "end\n",
            "pub fn main(stringify: Int) -> String effects []\n",
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
            "pub fn main() -> String effects []\n",
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
            "pub fn main() -> Int effects []\n",
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
            "pub fn main(value: Option(Int)) -> Int effects []\n",
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
fn source_backed_prelude_helpers_report_user_call_site_diagnostics() {
    for (helper, value_type, return_type, expected_callback) in [
        (
            "vec_map",
            "Vec(Int)",
            "Vec(String)",
            "fn(unknown) -> String",
        ),
        ("vec_filter", "Vec(Int)", "Vec(Int)", "fn(Int) -> Bool"),
        (
            "option_map",
            "Option(Int)",
            "Option(String)",
            "fn(unknown) -> String",
        ),
        (
            "option_and_then",
            "Option(Int)",
            "Option(String)",
            "fn(unknown) -> Option(String)",
        ),
        (
            "result_map",
            "Result(Int, String)",
            "Result(String, String)",
            "fn(unknown) -> String",
        ),
        (
            "result_map_err",
            "Result(String, Int)",
            "Result(String, String)",
            "fn(unknown) -> String",
        ),
        (
            "result_and_then",
            "Result(Int, String)",
            "Result(String, String)",
            "fn(unknown) -> Result(String, String)",
        ),
        (
            "vec_try_map",
            "Vec(Int)",
            "Result(Vec(String), String)",
            "fn(unknown) -> Result(String, String)",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                concat!(
                    "fn to_int(value: Int) -> Int effects []\n",
                    "  value\n",
                    "end\n",
                    "pub fn main(value: {}) -> {} effects []\n",
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
