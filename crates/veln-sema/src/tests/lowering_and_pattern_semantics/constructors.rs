use super::*;

#[test]
fn propagates_try_expected_type_from_result_return() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result<Int, AppError>\n  Ok(_?)\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"expected_type\":\"Result<Int, AppError>\"")
    );
}

#[test]
fn lowers_option_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option<String>\n",
            "  Some(\"ok\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::option(CoreType::string()));
    let CoreExprKind::OptionSome(value) = &expr.kind else {
        panic!("Some call should lower to an option constructor");
    };
    assert_eq!(value.ty, CoreType::string());
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_none_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!("pub fn main() -> Option<String>\n", "  None\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::option(CoreType::string()));
    assert!(matches!(expr.kind, CoreExprKind::OptionNone));
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_qualified_none_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option<String>\n",
            "  Option::None\n",
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
    assert_eq!(expr.ty, CoreType::option(CoreType::string()));
    assert!(matches!(expr.kind, CoreExprKind::OptionNone));
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_qualified_builtin_constructors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(use_result: Bool) -> Result<Option<String>, AppError>\n",
            "  if_missing(use_result)\n",
            "end\n",
            "fn if_missing(use_result: Bool) -> Result<Option<String>, AppError>\n",
            "  Result::Ok(Option::Some(\"ok\"))\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let helper = core
        .functions
        .iter()
        .find(|function| function.name == "if_missing")
        .expect("helper should be lowered");
    let CoreStmtKind::Return { expr } = &helper.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::string()),
            CoreType::named("AppError", Vec::new())
        )
    );
    let CoreExprKind::ResultOk(value) = &expr.kind else {
        panic!("Result::Ok call should lower to a result constructor");
    };
    assert!(matches!(value.kind, CoreExprKind::OptionSome(_)));
}

#[test]
fn lowers_name_paths_by_resolution_category() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn main(value: Int) -> {local: Int, constructor: Option<String>, callback: fn(Int) -> String}\n",
            "  {local: value, constructor: None, callback: stringify}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
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
        panic!("tail expression should lower as a record");
    };
    assert!(matches!(
        &fields[0].expr.kind,
        CoreExprKind::Local(name) if name == "value"
    ));
    assert!(matches!(fields[1].expr.kind, CoreExprKind::OptionNone));
    assert_eq!(fields[1].expr.ty, CoreType::option(CoreType::string()));
    assert!(matches!(
        &fields[2].expr.kind,
        CoreExprKind::FunctionValue(name) if name == "stringify"
    ));
}

#[test]
fn infers_payload_constructor_type_arguments_without_expected_adt_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Box<A>\n",
            "  Box(value: A)\n",
            "end\n",
            "fn main() -> {option: Option<Int>, list: List<Int>, boxed: Box<String>}\n",
            "  let option = Some(1)\n",
            "  let list = Cons(1, Nil)\n",
            "  let boxed = Box(\"ok\")\n",
            "  {option: option, list: list, boxed: boxed}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("option binding should lower as let");
    };
    assert_eq!(expr.ty, CoreType::option(CoreType::int()));
    let CoreStmtKind::Let { expr, .. } = &main.body[1].kind else {
        panic!("list binding should lower as let");
    };
    assert_eq!(expr.ty, CoreType::named("List", vec![CoreType::int()]));
    let CoreStmtKind::Let { expr, .. } = &main.body[2].kind else {
        panic!("box binding should lower as let");
    };
    assert_eq!(expr.ty, CoreType::named("Box", vec![CoreType::string()]));
}

#[test]
fn unresolved_payload_constructor_type_arguments_are_ambiguous() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  let value = Ok(1)\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.inference_ambiguous");
    assert_eq!(
        diagnostics[0].message,
        "constructor `Ok` needs type context"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"inferred_type\":\"Result<Int, unknown>\"")
    );
}

#[test]
fn conflicting_payload_constructor_type_arguments_report_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Both<A>\n",
            "  Both(left: A, right: A)\n",
            "end\n",
            "fn main() -> Int\n",
            "  let value = Both(1, \"bad\")\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
    assert_diagnostic_span(&diagnostics[0], 5, 23, 5, 28);
}

#[test]
fn non_constructor_expected_type_still_reports_outer_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> Int\n", "  Some(1)\n", "end\n",),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `Int`, but found `Option<Int>`"
    );
}
