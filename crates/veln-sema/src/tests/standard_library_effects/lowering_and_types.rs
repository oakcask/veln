use super::*;
use crate::lowering::{reset_core_call_signature_work, take_core_call_signature_work};
use crate::semantic_model::Type;
use crate::types::TypeEnvironment;
use veln_ir::IrCallbackTarget;

mod io_provider;

#[test]
fn pure_source_less_calls_lower_with_string_and_path_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: String, right: String) -> Path\n",
            "  let joined: String = string::concat(left, right)\n",
            "  fs::path(joined)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("pure calls should build checked core");
    let main = &core.functions[0];
    assert!(main.effects.is_empty());
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("expected concatenated string binding");
    };
    assert_eq!(expr.ty, CoreType::string());
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("expected path return");
    };
    assert_eq!(expr.ty, CoreType::named("Path", Vec::new()));
    let ir = lowered
        .ir
        .expect("pure calls should lower to executable IR");
    assert!(ir.functions[0].effects.is_empty());
    assert_eq!(
        standard_library_builtin_calls(&ir.functions[0]),
        [("let", "string::concat"), ("return", "fs::path")]
    );
}

#[test]
fn pure_source_less_calls_reject_wrong_argument_and_result_types() {
    for (expression, return_type, expected) in [
        ("fs::path(1)", "Path", "expected `String`, but found `Int`"),
        (
            "string::concat(1, \"right\")",
            "String",
            "expected `String`, but found `Int`",
        ),
        (
            "string::concat(\"left\", 1)",
            "String",
            "expected `String`, but found `Int`",
        ),
        (
            "fs::path(\"relative\")",
            "String",
            "expected `String`, but found `Path`",
        ),
        (
            "string::concat(\"left\", \"right\")",
            "Path",
            "expected `Path`, but found `String`",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("pub fn main() -> {return_type}\n  {expression}\nend\n"),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);
        let diagnostics = analyze_surface_module(&module);
        assert_eq!(diagnostics.len(), 1, "{expression}: {diagnostics:#?}");
        assert_eq!(diagnostics[0].id, "type.mismatch", "{expression}");
        assert_eq!(diagnostics[0].message, expected, "{expression}");
    }
}

#[test]
fn task_spawn_and_join_preserve_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn produce() -> String\n",
            "  \"hello\"\n",
            "end\n",
            "pub fn main() -> Result<String, JoinError> effects [concurrency]\n",
            "  let task = task::spawn(produce)\n",
            "  task::join(task)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("expected task binding");
    };
    assert_eq!(expr.ty, CoreType::named("Task", vec![CoreType::string()]));
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("expected joined return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(CoreType::string(), CoreType::named("JoinError", Vec::new()))
    );
    let ir = lowered.ir.expect("task calls should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should lower to IR");
    assert!(matches!(
        &main.body[0].kind,
        IrStmtKind::Let { value, .. }
            if matches!(
                &value.kind,
                IrExprKind::Call {
                    target: IrCallTarget::CallbackBoundary {
                        target: IrCallbackTarget::ConcurrencyBuiltin(name),
                        ..
                    },
                    ..
                } if name == "task::spawn"
            )
    ));
}

#[test]
fn task_spawn_with_preserves_argument_and_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn produce(input: String) -> String effects [net, db]\n",
            "  input\n",
            "end\n",
            "pub fn main(input: String) -> Result<String, JoinError> effects [concurrency, net, db]\n",
            "  let task = task::spawn_with(produce, input)\n",
            "  task::join(task)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("expected task binding");
    };
    assert_eq!(expr.ty, CoreType::named("Task", vec![CoreType::string()]));
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("expected task call");
    };
    assert_eq!(args[1].ty, CoreType::string());
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("expected joined return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(CoreType::string(), CoreType::named("JoinError", Vec::new()))
    );
    let ir = lowered.ir.expect("task calls should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should lower to IR");
    assert!(matches!(
        &main.body[0].kind,
        IrStmtKind::Let { value, .. }
            if matches!(
                &value.kind,
                IrExprKind::Call {
                    target: IrCallTarget::CallbackBoundary {
                        target: IrCallbackTarget::ConcurrencyBuiltin(name),
                        ..
                    },
                    args,
                } if name == "task::spawn_with" && args.len() == 2
            )
    ));
}

#[test]
fn ordinary_nested_calls_have_linear_core_signature_work() {
    fn work_at_depth(depth: usize) -> usize {
        let mut expression = "value".to_string();
        for _ in 0..depth {
            expression = format!("identity({expression})");
        }
        let source = SourceFile::new(
            "main.veln",
            format!(
                "fn identity(value: Int) -> Int\n  value\nend\npub fn main(value: Int) -> Int\n  {expression}\nend\n"
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);
        reset_core_call_signature_work();
        let lowered = lower_checked_surface_module(&module);
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        assert!(lowered.ir.is_some());
        take_core_call_signature_work()
    }

    std::thread::Builder::new()
        .name("nested-call-scaling".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let at_200 = work_at_depth(200);
            let at_400 = work_at_depth(400);
            let at_800 = work_at_depth(800);
            assert_eq!(at_200, 400);
            assert_eq!(at_400, 800);
            assert_eq!(at_800, 1600);
        })
        .expect("nested-call scaling worker should start")
        .join()
        .expect("nested-call scaling worker should finish");
}

#[test]
fn nested_concurrency_calls_have_linear_shallow_type_work() {
    fn work_at_depth(depth: usize) -> usize {
        let mut expression = "sender".to_string();
        for _ in 0..depth {
            expression = format!("channel::clone({expression})");
        }
        let source = SourceFile::new(
            "main.veln",
            format!(
                "pub fn main(sender: Sender<Int>) -> Sender<Int> effects [concurrency]\n  {expression}\nend\n"
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);
        reset_core_call_signature_work();
        let lowered = lower_checked_surface_module(&module);
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        assert!(lowered.ir.is_some());
        take_core_call_signature_work()
    }

    let at_50 = work_at_depth(50);
    let at_100 = work_at_depth(100);
    assert_eq!(at_50, 49);
    assert_eq!(at_100, 99);
    assert!(
        at_100 <= at_50 * 3,
        "nested concurrency shallow-type work grew faster than linear: depth 50={at_50}, depth 100={at_100}"
    );
}

#[test]
fn io_provider_atomic_state_rejects_mismatched_generic_evidence() {
    let source = SourceFile::new(
        "io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type State<T>\n",
            "end\n",
            "pub fn state(initial: T) -> State<T> effects [concurrency]\n",
            "  prelude_builtin::io_provider_state(initial)\n",
            "end\n",
            "pub fn update(cell: State<T>, context: C, action: fn(T, C) -> {state: T, result: R}) -> R effects [concurrency]\n",
            "  prelude_builtin::io_provider_state_update(cell, context, action)\n",
            "end\n",
            "fn wrong_step(value: Bool, context: Int) -> {state: Bool, result: Bool}\n",
            "  {state: value, result: true}\n",
            "end\n",
            "fn effectful_step(value: String, context: String) -> {state: String, result: Bool} effects [stdio]\n",
            "  {state: value, result: true}\n",
            "end\n",
            "fn width_step(value: {name: String}, context: String) -> {state: {name: String}, result: ()}\n",
            "  {state: {name: context}, result: ()}\n",
            "end\n",
            "fn exact_step(value: {name: String, retained: String}, context: ()) -> {state: {name: String, retained: String}, result: String}\n",
            "  {state: value, result: value.retained}\n",
            "end\n",
            "pub fn evidence() -> () effects [concurrency]\n",
            "  let cell = state(\"value\")\n",
            "  let _ = update(cell, \"context\", wrong_step)\n",
            "  let _ = update(cell, \"context\", effectful_step)\n",
            "  let precise = state({name: \"first\", retained: \"kept\"})\n",
            "  let widened: State<{name: String}> = precise\n",
            "  let _ = update(widened, \"second\", width_step)\n",
            "  let _ = update(precise, (), exact_step)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let mut module = lower_standard_package_callables(&source);
    for function in &mut module.functions {
        function.module_name = Some("std::io_provider".to_string());
    }
    for declaration in &mut module.types {
        declaration.module_name = Some("std::io_provider".to_string());
        declaration.package_name = Some(veln_stdlib::PACKAGE_NAME.to_string());
    }
    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("fn(String, String)")),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("effects [stdio]")),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("State<{name: String}>")
                && diagnostic
                    .message
                    .contains("State<{name: String, retained: String}>")),
        "{diagnostics:#?}"
    );
}

#[test]
fn task_spawn_with_preserves_explicit_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn produce(input: String) -> String effects [db]\n",
            "  input\n",
            "end\n",
            "pub fn main(input: String) -> Result<String, JoinError> effects [concurrency, db]\n",
            "  let task = task::spawn_with<String>(produce, input)\n",
            "  task::join(task)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("expected task binding");
    };
    assert_eq!(expr.ty, CoreType::named("Task", vec![CoreType::string()]));
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("expected task call");
    };
    assert_eq!(args[1].ty, CoreType::string());
}

#[test]
fn task_spawn_with_explicit_context_type_overrides_handler_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn route(context: {payload: String, marker: Int}) -> String effects [net, db]\n",
            "  context.payload\n",
            "end\n",
            "pub fn main(payload: String, marker: Int) -> Result<String, JoinError> effects [concurrency, net, db]\n",
            "  let context = {payload: payload, marker: marker}\n",
            "  let task = task::spawn_with<String, {payload: String, marker: Int}>(route, context)\n",
            "  task::join(task)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Let { expr, .. } = &main.body[1].kind else {
        panic!("expected task binding");
    };
    assert_eq!(expr.ty, CoreType::named("Task", vec![CoreType::string()]));
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("expected task call");
    };
    assert_eq!(
        args[1].ty,
        CoreType::Record(vec![
            ("payload".to_string(), CoreType::string()),
            ("marker".to_string(), CoreType::int()),
        ])
    );
}

#[test]
fn task_spawn_with2_is_unresolved_after_numbered_api_removal() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn combine(left: String, right: Int) -> String effects [concurrency]\n",
            "  left\n",
            "end\n",
            "pub fn main(input: String, count: Int) -> Result<String, JoinError> effects [concurrency]\n",
            "  let task = task::spawn_with2<String>(combine, input, count)\n",
            "  task::join(task)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `task::spawn_with2`"),
        "{:#?}",
        diagnostics
    );
}

#[test]
fn task_spawn_with_rejects_extra_explicit_type_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn produce(input: String) -> String effects [concurrency]\n",
            "  input\n",
            "end\n",
            "pub fn main(input: String) -> Task<String> effects [concurrency]\n",
            "  task::spawn_with<String, String, Int>(produce, input)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "core.type_argument_count_mismatch"),
        "{:#?}",
        lowered.diagnostics
    );
}

#[test]
fn declared_concurrency_calls_lower_to_executable_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String effects [concurrency]\n",
            "  let pair: {tx: Sender<String>, rx: Receiver<String>} = channel::bounded(1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    assert!(matches!(
        lowered
            .core
            .expect("checked core should be built")
            .readiness,
        CoreReadiness::Complete
    ));
    let ir = lowered.ir.expect("concurrency calls should lower to IR");
    let main = &ir.functions[0];
    assert!(matches!(
        &main.body[0].kind,
        IrStmtKind::Let { value, .. }
            if matches!(
                &value.kind,
                IrExprKind::Call {
                    target: IrCallTarget::ConcurrencyBuiltin(name),
                    ..
                } if name == "channel::bounded"
            )
    ));
}

#[test]
fn channel_bounded_accepts_explicit_item_type_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String effects [concurrency]\n",
            "  let pair = channel::bounded<String>(1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("expected channel binding");
    };
    assert_eq!(
        expr.ty,
        CoreType::Record(vec![
            (
                "tx".to_string(),
                CoreType::named("Sender", vec![CoreType::string()])
            ),
            (
                "rx".to_string(),
                CoreType::named("Receiver", vec![CoreType::string()])
            ),
        ])
    );
}

#[test]
fn channel_clone_preserves_sender_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(tx: Sender<String>) -> Result<(), SendError> effects [concurrency]\n",
            "  let clone = channel::clone(tx)\n",
            "  channel::send(clone, \"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("expected cloned sender binding");
    };
    assert_eq!(expr.ty, CoreType::named("Sender", vec![CoreType::string()]));
}

#[test]
fn channel_send_checks_value_against_sender_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(tx: Sender<String>) -> Result<(), SendError> effects [concurrency]\n",
            "  channel::send(tx, 1)\n",
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
