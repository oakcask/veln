use super::*;
use veln_ir::IrCallbackTarget;

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
