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
fn reports_hole_with_declared_return_expected_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn todo() -> Result(Unit, AppError)\n  _\nend\n",
    );
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
            "\"expected_type\":\"Result(Unit, AppError)\",",
            "\"expected_type_source\":\"declared\",",
            "\"constraints\":[],\"local_bindings\":[],",
            "\"candidate_queries\":[{\"kind\":\"symbol\",",
            "\"query\":\"fn() -> Result(Unit, AppError)\"}]}"
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
        "fn main() -> Result(Unit, AppError)\n  Ok(())\nend\n",
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
        "fn main() -> Result(Unit, AppError)\n  Ok(\"no\")\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-5\",\"expected_type\":\"Unit\",",
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
fn flows_call_argument_expected_type_into_holes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn consume(value: Float) -> Unit\n",
            "  ()\n",
            "end\n",
            "pub fn main() -> Unit effects []\n",
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
            "pub fn main() -> Unit effects []\n",
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
            "pub fn main(value: Int) -> Unit effects []\n",
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
fn lowers_runnable_checked_program_to_core_and_typed_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main(raw: String) -> Result(Unit, AppError) effects [stdio]\n",
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
        "pub fn main() -> Result(Unit, AppError) effects []\n  _\nend\n",
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
