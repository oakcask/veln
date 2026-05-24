use crate::*;
use veln_ast::lower_surface_ast;
use veln_core::{CoreBlocker, CoreCallTarget, CoreExprKind, CoreReadiness, CoreStmtKind, CoreType};
use veln_diagnostics::DiagnosticKind;
use veln_ir::{IrCallTarget, IrExprKind, IrStmtKind};
use veln_source::SourceFile;
use veln_syntax::parse;

#[test]
fn public_function_requires_explicit_boundary() {
    let source = SourceFile::new("main.veln", "pub fn main(value)\n  value\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public parameter `value` has no type annotation"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public function has no return type annotation"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "effect.missing_public"
            && diagnostic.kind == DiagnosticKind::Effect
            && diagnostic.message == "public function has no effects annotation"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn private_function_may_omit_boundary_annotations() {
    let source = SourceFile::new("main.veln", "fn helper(value)\n  value\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn test_declaration_requires_explicit_test_shape() {
    let source = SourceFile::new(
        "main_test.veln",
        "test bad(value: Int) -> Int\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.parameters"
            && diagnostic.message == "test declaration has parameters"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "test.return_type"
            && diagnostic.message == "test declaration returns `Int`"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "effect.missing_test"
            && diagnostic.kind == DiagnosticKind::Effect
            && diagnostic.message == "test declaration has no effects annotation"
            && diagnostic
                .details
                .to_json()
                .contains("\"boundary\":\"test_declaration\"")
    }));
}

#[test]
fn test_declaration_checks_declared_effect_boundary() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test prints() -> () effects []\n",
            "  stdio::println(\"hello\")\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_test");
    assert_eq!(
        diagnostics[0].message,
        "test declaration uses undeclared effect `stdio`"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"node_id\":\"test-1\"")
    );
}

#[test]
fn test_declarations_are_not_callable_functions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "test helper() -> () effects []\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  helper()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.unresolved");
    assert_eq!(diagnostics[0].message, "unresolved call_target `helper`");
}

#[test]
fn duplicate_function_like_declaration_names_are_static_errors() {
    let source = SourceFile::new(
        "main_test.veln",
        concat!(
            "test same() -> () effects []\n",
            "  ()\n",
            "end\n",
            "fn same() -> () effects []\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(
        diagnostics[0].message,
        "duplicate function declaration name `same`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"function\"")
    );
}

#[test]
fn duplicate_use_aliases_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use platform.io\n",
            "use local.io\n",
            "fn main() -> () effects []\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate import alias name `io`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"module\"")
    );
}

#[test]
fn use_declarations_require_module_identity() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use platform.io\n",
            "fn main() -> () effects []\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "module.missing_identity");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Module);
    assert_eq!(
        diagnostics[0].message,
        "module import requires a module identity"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"field\":\"module_identity\"")
    );
}

#[test]
fn duplicate_parameter_names_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Int, value: Int) -> Int effects []\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate parameter name `value`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"value\"")
    );
}

#[test]
fn let_names_cannot_duplicate_the_function_value_scope() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Int) -> Int effects []\n  let value = 1\n  value\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(
        diagnostics[0].message,
        "duplicate local binding name `value`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"value\"")
    );
}

#[test]
fn duplicate_record_field_names_are_static_errors() {
    let source = SourceFile::new("main.veln", "fn bad() -> {a: Int}\n  {a: 1, a: 2}\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate record field name `a`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"record_field\"")
    );
}

#[test]
fn reports_hole_with_declared_return_expected_type() {
    let source = SourceFile::new("main.veln", "fn todo() -> Result((), AppError)\n  _\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Hole);
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"hole\",\"node_id\":\"hole-3\",\"label\":null,",
            "\"expected_type\":\"Result((), AppError)\",",
            "\"expected_type_source\":\"declared\",",
            "\"constraints\":[],\"local_bindings\":[],",
            "\"candidate_queries\":[{\"kind\":\"symbol\",",
            "\"query\":\"fn() -> Result((), AppError)\"}]}"
        )
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn reports_return_type_mismatch() {
    let source = SourceFile::new("main.veln", "fn bad() -> Int\n  \"no\"\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-3\",\"expected_type\":\"Int\",",
            "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"inferred_expression\",",
            "\"constraint\":\"return_value\",",
            "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-3\"]}"
        )
    );
}

#[test]
fn ok_constructor_accepts_declared_result_return() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result((), AppError)\n  Ok(())\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn result_constructor_checks_expected_value_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result((), AppError)\n  Ok(\"no\")\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-5\",\"expected_type\":\"()\",",
            "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"inferred_expression\",",
            "\"constraint\":\"call_argument\",",
            "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-5\"]}"
        )
    );
}

#[test]
fn accepts_first_slice_type_forms_and_record_expected_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> {score: Float, names: List(String), table: Dict(String, Int), ",
            "callback: fn(Int) -> String}\n",
            "  {score: _, names: [], table: _, callback: _}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.details.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("\"expected_type\":\"Float\""));
    assert!(rendered.contains("\"expected_type\":\"Dict(String, Int)\""));
    assert!(rendered.contains("\"expected_type\":\"fn(Int) -> String\""));
    assert!(rendered.contains("\"candidate_queries\":[{\"kind\":\"symbol\""));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.related.is_empty())
    );
}

#[test]
fn accepts_float_numeric_operators() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float, right: Float) -> {sum: Float, negated: Float, ordered: Bool} effects []\n",
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
fn reports_float_operator_operand_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float) -> Float effects []\n",
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
            "pub fn main(left: Int, right: Int) -> Float effects []\n",
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
        "fn bad(value: Result(Int)) -> Option()\n  ()\nend\n",
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
            "fn parse(raw: String) -> Result(Int, AppError)\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main() -> Result(Int, AppError) effects []\n",
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
fn infers_prelude_helper_calls_from_expected_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(items: List(Int), other: List(Int), table: Dict(String, Int), ",
            "mapper: fn(Int) -> String, keep: fn(Int) -> Bool, folder: fn(String, Int) -> String, ",
            "fallible: fn(Int) -> Result(String, AppError), opt: Option(Int), ",
            "opt_map: fn(Int) -> String, opt_next: fn(Int) -> Option(String), ",
            "res: Result(Int, AppError), err_map: fn(AppError) -> String, ",
            "res_next: fn(Int) -> Result(String, AppError)) -> {",
            "count: Int, empty: Bool, pushed: List(Int), joined: List(Int), mapped: List(String), ",
            "filtered: List(Int), folded: String, tried: Result(List(String), AppError), ",
            "found: Option(Int), has_key: Bool, inserted: Dict(String, Int), removed: Dict(String, Int), ",
            "opt_mapped: Option(String), opt_nexted: Option(String), opt_value: Int, ",
            "res_mapped: Result(String, AppError), res_err: Result(Int, String), ",
            "res_nexted: Result(String, AppError)} effects []\n",
            "  {count: list_len(items), empty: list_is_empty(items), ",
            "pushed: list_push(items, 1), joined: list_concat(items, other), ",
            "mapped: list_map(items, mapper), filtered: list_filter(items, keep), ",
            "folded: list_fold(items, \"\", folder), tried: list_try_map(items, fallible), ",
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
        } if name == "list_len"
    ));
    assert!(matches!(first.expr.ty, CoreType::Named { ref name, .. } if name == "Int"));
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
        } if name == "list_len"
    ));
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
fn flows_call_argument_expected_type_into_holes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn consume(value: Float) -> ()\n",
            "  ()\n",
            "end\n",
            "pub fn main() -> () effects []\n",
            "  consume(_)\n",
            "end\n",
        ),
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
            .contains("\"expected_type\":\"Float\"")
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn reports_missing_public_effect_with_call_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> () effects []\n",
            "  stdio::println(\"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Effect);
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"stdio\""));
    assert!(details.contains("\"declared_effects\":[]"));
    assert!(details.contains("\"inferred_effects\":[\"stdio\"]"));
    assert!(details.contains("\"symbol\":\"stdio::println\""));
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn reports_non_boolean_contract_predicate() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Int) -> () effects []\n",
            "require value\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.type_mismatch"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract predicate is not `Bool`"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_boolean_predicate\"")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic.kind == DiagnosticKind::Type
            && diagnostic.message == "expected `Bool`, but found `Int`"
    }));
}

#[test]
fn ensure_can_reference_explicit_result_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> output: Int effects []\n",
            "ensure output == value\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_field_access_resolves_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {total: Int}) -> output: {total: Int} effects []\n",
            "ensure output.total == value.total\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_boolean_field_access_is_a_boolean_predicate() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}) -> output: {ready: Bool} effects []\n",
            "require value.ready\n",
            "ensure output.ready\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_missing_record_field_reports_contract_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {total: Int}) -> output: {total: Int} effects []\n",
            "ensure output.missing == value.total\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.field_missing"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract field `missing` is not present on `{total: Int}`"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"missing_field\"")
    }));
}

#[test]
fn require_cannot_reference_result_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> output: Int effects []\n",
            "require output > 0\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved contract_predicate `output`"
    }));
}

#[test]
fn bare_result_has_no_ensure_special_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> Int effects []\n",
            "ensure result == value\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved contract_predicate `result`"
    }));
}

#[test]
fn result_binding_is_not_in_function_body_scope() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> output: Int effects []\n",
            "  output\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `output`"
    }));
}

#[test]
fn result_binding_cannot_duplicate_parameter_name() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(output: Int) -> output: Int effects []\n",
            "ensure output == 0\n",
            "  output\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate result binding name `output`"
    }));
}

#[test]
fn hole_diagnostic_includes_contract_and_satisfy_constraints() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn default_port(max: Int) -> Int effects []\n",
            "require max > 0\n",
            "  _port satisfy candidate => candidate > 0 and candidate <= max\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type\":\"Int\""));
    assert!(details.contains("\"kind\":\"contract\""));
    assert!(details.contains("\"clause\":\"require\""));
    assert!(details.contains("\"text\":\"max > 0\""));
    assert!(details.contains("\"kind\":\"satisfy\""));
    assert!(details.contains(
        "\"text\":\"candidate > 0 and candidate <= max\",\"candidate_binding\":\"candidate\""
    ));
    assert!(details.contains("\"repair_status\":\"blocked_until_discharged\""));
    assert_eq!(diagnostics[0].related.len(), 3);
}

#[test]
fn satisfy_candidate_reports_shadowing_and_unused_predicates() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn default_port(max: Int) -> Int\n",
            "  _port satisfy max => true\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_candidate_shadow"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy candidate `max` shadows a visible binding"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_candidate_unused"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy predicate does not reference candidate `max`"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn propagates_try_expected_type_from_result_return() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result(Int, AppError)\n  Ok(_?)\nend\n",
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
            .contains("\"expected_type\":\"Result(Int, AppError)\"")
    );
}

#[test]
fn lowers_option_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option(String) effects []\n",
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
        concat!(
            "pub fn main() -> Option(String) effects []\n",
            "  None\n",
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
    assert!(matches!(expr.kind, CoreExprKind::OptionNone));
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_runnable_checked_program_to_core_and_typed_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main(raw: String) -> Result((), AppError) effects [stdio]\n",
            "  let value: Int = parse(raw)?\n",
            "  stdio::println(\"ok\")\n",
            "  Ok(())\n",
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
    assert!(matches!(main.body[0].kind, CoreStmtKind::Let { .. }));
    let CoreStmtKind::Expr { expr } = &main.body[1].kind else {
        panic!("stdio call should lower as an expression statement");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::StdioBuiltin(symbol),
            ..
        } if symbol == "stdio::println"
    ));
    assert!(matches!(main.body[2].kind, CoreStmtKind::Return { .. }));

    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    assert!(matches!(main.body[0].kind, IrStmtKind::Let { .. }));
    let IrStmtKind::Expr { value } = &main.body[1].kind else {
        panic!("stdio call should stay an expression statement in IR");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StdioBuiltin(symbol),
            ..
        } if symbol == "stdio::println"
    ));
    let IrStmtKind::Return { value } = &main.body[2].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(value.kind, IrExprKind::ResultOk(_)));
}

#[test]
fn holes_build_blocked_core_but_not_executable_ir() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> Result((), AppError) effects []\n  _\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 1);
    assert_eq!(lowered.diagnostics[0].id, "hole.unfilled");
    let core = lowered.core.expect("partial checked core should be built");
    assert!(matches!(
        core.readiness,
        CoreReadiness::Blocked(ref blockers) if matches!(blockers.as_slice(), [CoreBlocker::Hole { .. }])
    ));
    assert!(lowered.ir.is_none());
}

#[test]
fn semantic_errors_block_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> Int effects []\n  \"no\"\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch")
    );
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}
