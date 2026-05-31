use super::*;

#[test]
fn match_exhaustiveness_reports_missing_result_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Err(error) => error\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing result case should be diagnosed");
    assert_eq!(diagnostic.message, "match is missing case Ok(_)");
}

#[test]
fn accepts_float_numeric_operators() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float, right: Float) -> {sum: Float, negated: Float, ordered: Bool}\n",
            "  {sum: left + right, negated: -left, ordered: left < right}\n",
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
        panic!("tail expression should lower as record");
    };
    assert_eq!(fields[0].expr.ty, CoreType::float());
    assert_eq!(fields[1].expr.ty, CoreType::float());
    assert_eq!(fields[2].expr.ty, CoreType::bool());
    assert!(matches!(
        &fields[0].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
    assert!(matches!(
        &fields[1].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_negate"
    ));
    assert!(matches!(
        &fields[2].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_less"
    ));
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Record(fields) = &value.kind else {
        panic!("tail expression should lower as IR record");
    };
    assert!(matches!(
        &fields[0].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
    assert!(matches!(
        &fields[1].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_negate"
    ));
    assert!(matches!(
        &fields[2].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_less"
    ));
}

#[test]
fn lowers_boolean_literals_through_core_and_ir() {
    let source = SourceFile::new("main.veln", "fn main() -> Bool\n  true\nend\n");
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
    assert_eq!(expr.ty, CoreType::bool());
    assert!(matches!(expr.kind, CoreExprKind::BoolLiteral(true)));

    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert_eq!(value.ty, CoreType::bool());
    assert!(matches!(value.kind, IrExprKind::BoolLiteral(true)));
}

#[test]
fn infers_float_numeric_operators_from_call_results() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn value() -> Float\n",
            "  1.0\n",
            "end\n",
            "fn main()\n",
            "  value() + value()\n",
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
    assert_eq!(expr.ty, CoreType::float());
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
}

#[test]
fn accepts_int_operands_in_float_operator_contexts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float, count: Int) -> {sum: Float, ordered: Bool, expected: Float}\n",
            "  {sum: left + count, ordered: count < left, expected: 1 + 2}\n",
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
        panic!("tail expression should lower as record");
    };
    assert_eq!(fields[0].expr.ty, CoreType::float());
    assert_eq!(fields[1].expr.ty, CoreType::bool());
    assert_eq!(fields[2].expr.ty, CoreType::float());
    assert!(matches!(
        &fields[0].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
    assert!(matches!(
        &fields[1].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_less"
    ));
    assert!(matches!(
        &fields[2].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
}

#[test]
fn rejects_int_values_in_float_assignment_contexts() {
    let source = SourceFile::new(
        "main.veln",
        concat!("pub fn main() -> Float\n", "  1\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Float`, but found `Int`");
}

#[test]
fn reports_float_operator_operand_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float) -> Float\n",
            "  left + \"bad\"\n",
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
        "expected `Float`, but found `String`"
    );
}

#[test]
fn comparison_does_not_select_float_from_expected_result() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Int, right: Int) -> Float\n",
            "  left < right\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Float`, but found `Bool`");
}

#[test]
fn reports_invalid_type_annotations() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Result<Int>) -> Option<>\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.invalid_annotation")
    );
}

#[test]
fn infers_non_constructor_calls_from_local_function_signatures() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main() -> Result<Int, AppError>\n",
            "  parse(\"1\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn resolves_qualified_calls_through_import_aliases() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.math\n",
            "pub fn main() -> Int\n",
            "  math::double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod app.math\n",
            "fn double(value: Int) -> Int\n",
            "  value + value\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let math = lower_surface_ast(&parse(&math_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        types: main.types.into_iter().chain(math.types).collect(),
        functions: main.functions.into_iter().chain(math.functions).collect(),
    };

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
    let CoreExprKind::Call { target, .. } = &expr.kind else {
        panic!("qualified call should lower as a call");
    };
    assert_eq!(target, &CoreCallTarget::Function("double".to_string()));
}

#[test]
fn resolves_qualified_function_values_through_import_aliases() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.text\n",
            "pub fn main() -> Vec<String>\n",
            "  vec_map([1], text::stringify)\n",
            "end\n",
        ),
    );
    let text_source = SourceFile::new(
        "text.veln",
        concat!(
            "mod app.text\n",
            "fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let text = lower_surface_ast(&parse(&text_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        types: main.types.into_iter().chain(text.types).collect(),
        functions: main.functions.into_iter().chain(text.functions).collect(),
    };

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
}

#[test]
fn resolves_unqualified_public_function_imports() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.math\n",
            "pub fn main() -> Int\n",
            "  double(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod app.math\n",
            "pub fn double(value: Int) -> Int\n",
            "  value + value\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let math = lower_surface_ast(&parse(&math_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        types: main.types.into_iter().chain(math.types).collect(),
        functions: main.functions.into_iter().chain(math.functions).collect(),
    };

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
    let CoreExprKind::Call { target, .. } = &expr.kind else {
        panic!("unqualified import should lower as a call");
    };
    assert_eq!(target, &CoreCallTarget::Function("double".to_string()));
}

#[test]
fn resolves_unqualified_imported_function_values() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.text\n",
            "pub fn main() -> Vec<String>\n",
            "  vec_map([1], stringify)\n",
            "end\n",
        ),
    );
    let text_source = SourceFile::new(
        "text.veln",
        concat!(
            "mod app.text\n",
            "pub fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let text = lower_surface_ast(&parse(&text_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        types: main.types.into_iter().chain(text.types).collect(),
        functions: main.functions.into_iter().chain(text.functions).collect(),
    };

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
}

#[test]
fn local_functions_shadow_unqualified_function_imports() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.math\n",
            "fn double(value: String) -> String\n",
            "  value\n",
            "end\n",
            "pub fn main() -> String\n",
            "  double(\"ok\")\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod app.math\n",
            "pub fn double(value: Int) -> Int\n",
            "  value + value\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let math = lower_surface_ast(&parse(&math_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        types: main.types.into_iter().chain(math.types).collect(),
        functions: main.functions.into_iter().chain(math.functions).collect(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn ambiguous_unqualified_public_function_imports_are_rejected() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.left\n",
            "use app.right\n",
            "pub fn main() -> Int\n",
            "  size()\n",
            "end\n",
        ),
    );
    let left_source = SourceFile::new(
        "left.veln",
        concat!("mod app.left\n", "pub fn size() -> Int\n", "  1\n", "end\n",),
    );
    let right_source = SourceFile::new(
        "right.veln",
        concat!(
            "mod app.right\n",
            "pub fn size() -> Int\n",
            "  2\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let left = lower_surface_ast(&parse(&left_source).tree);
    let right = lower_surface_ast(&parse(&right_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        types: Vec::new(),
        functions: main
            .functions
            .into_iter()
            .chain(left.functions)
            .chain(right.functions)
            .collect(),
    };

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "name.ambiguous"
                && diagnostic.message == "ambiguous call_target `size`"
        })
        .expect("ambiguous imported call should be diagnosed");
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>();
    assert!(
        related
            .iter()
            .any(|note| note.contains("use `left::size` to select it"))
    );
    assert!(
        related
            .iter()
            .any(|note| note.contains("use `right::size` to select it"))
    );
}

#[test]
fn private_functions_are_hidden_from_unqualified_imports() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.math\n",
            "pub fn main() -> Int\n",
            "  hidden(2)\n",
            "end\n",
        ),
    );
    let math_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod app.math\n",
            "fn hidden(value: Int) -> Int\n",
            "  value\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let math = lower_surface_ast(&parse(&math_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        types: Vec::new(),
        functions: main.functions.into_iter().chain(math.functions).collect(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `hidden`"
    }));
}

#[test]
fn public_function_alias_reexports_imported_target() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod spec.app\n",
            "use spec.api\n",
            "pub fn main() -> Int\n",
            "  api::twice(21)\n",
            "end\n",
        ),
    );
    let api_source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "use spec.impl\n",
            "pub fn twice = impl::double\n",
        ),
    );
    let impl_source = SourceFile::new(
        "impl.veln",
        concat!(
            "mod spec.impl\n",
            "fn double(value: Int) -> Int\n",
            "  value + value\n",
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
        types: Vec::new(),
        functions: app
            .functions
            .into_iter()
            .chain(implementation.functions)
            .collect(),
    };

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
    let CoreExprKind::Call { target, .. } = &expr.kind else {
        panic!("alias call should lower as a call");
    };
    assert_eq!(target, &CoreCallTarget::Function("double".to_string()));
}

#[test]
fn unresolved_qualified_calls_do_not_fall_back_to_bare_functions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "fn helper(value: String) -> String\n",
            "  value\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  math::helper(2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `math::helper`"
    }));
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic.message == "expected `String`, but found `Int`"
    }));
}

#[test]
fn pipeline_inserts_left_value_as_first_call_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn add(left: Int, right: Int) -> Int\n",
            "  left + right\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  1 |> add(2)\n",
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
        panic!("pipeline should lower as a call");
    };
    assert_eq!(target, &CoreCallTarget::Function("add".to_string()));
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0].kind, CoreExprKind::IntLiteral(value) if value == "1"));
    assert!(matches!(&args[1].kind, CoreExprKind::IntLiteral(value) if value == "2"));
}

#[test]
fn pipeline_requires_call_target() {
    let source = SourceFile::new("main.veln", "pub fn main() -> Int\n  1 |> 2\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.pipeline_target"
            && diagnostic.message == "pipeline target is not a call"
    }));
}

#[test]
fn pipeline_requires_named_call_target() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn make(value: Int, callback: fn(Int) -> Int) -> fn(Int) -> Int\n",
            "  callback\n",
            "end\n",
            "pub fn main(callback: fn(Int) -> Int) -> Int\n",
            "  1 |> make(0, callback)(2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.pipeline_target"
            && diagnostic.message == "pipeline target is not a named call"
    }));
}

#[test]
fn method_call_shape_reports_targeted_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: String) -> Int\n",
            "  value.len()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.method_call");
    assert_eq!(
        diagnostics[0].message,
        "method call syntax is not supported"
    );
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-4\",",
            "\"expected\":\"function_call\",\"actual\":\"method_call\",",
            "\"constraint\":\"call_target\",\"method\":\"len\"}"
        )
    );
    assert!(diagnostics[0].related.iter().any(|related| {
        related
            .to_json()
            .contains("\"Use a named function call with the receiver as an explicit argument.\"")
    }));
}
