use super::*;

#[test]
fn fs_process_net_and_time_calls_lower_to_standard_library_builtins() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(path: Path, key: String) -> Result<String, FsError> effects [fs, process, net, time]\n",
            "  let cwd: Result<Path, ProcessError> = process::cwd()\n",
            "  let present: Option<String> = process::env(key)\n",
            "  let chunk: ByteChunk = net::receive_chunk()\n",
            "  net::send_chunk(chunk)\n",
            "  let listener: NetListener = net::listen(\"127.0.0.1:0\")\n",
            "  let stream: NetStream = net::accept(listener)\n",
            "  let optional_stream: Option<NetStream> = net::accept_or_end(listener)\n",
            "  let accept_deadline: Deadline = time::deadline_after_ms(1)\n",
            "  let timed_stream: Option<NetStream> = net::accept_until(listener, accept_deadline)\n",
            "  let accept_token: CancelToken = time::cancel_token()\n",
            "  let cancellable_accept: AcceptOutcome = net::accept_until_cancellable(listener, accept_deadline, accept_token)\n",
            "  net::close_listener(listener)\n",
            "  let listener_addr: String = net::listener_local_addr(listener)\n",
            "  let local_addr: String = net::stream_local_addr(stream)\n",
            "  let peer_addr: String = net::stream_peer_addr(stream)\n",
            "  let can_read: Bool = net::stream_can_read(stream)\n",
            "  let can_write: Bool = net::stream_can_write(stream)\n",
            "  let stream_closed: Bool = net::stream_is_closed(stream)\n",
            "  let socket_chunk: ByteChunk = net::read_chunk(stream)\n",
            "  let socket_chunk_or_end: Option<ByteChunk> = net::read_chunk_or_end(stream)\n",
            "  let read_deadline: Deadline = time::deadline_after_ms(1)\n",
            "  let socket_chunk_until: Option<ByteChunk> = net::read_chunk_until(stream, read_deadline)\n",
            "  let read_token: CancelToken = time::cancel_token()\n",
            "  let cancellable_socket_read: StreamReadOutcome = net::read_chunk_until_cancellable(stream, read_deadline, read_token)\n",
            "  net::write_chunk(stream, socket_chunk)\n",
            "  let write_deadline: Deadline = time::deadline_after_ms(1)\n",
            "  let timed_socket_write: StreamWriteOutcome = net::write_chunk_until(stream, socket_chunk, write_deadline)\n",
            "  let cancellable_socket_write: StreamWriteOutcome = net::write_chunk_until_cancellable(stream, socket_chunk, write_deadline, read_token)\n",
            "  net::write_chunks(stream, byte_chunks_one(socket_chunk))\n",
            "  let timed_socket_writes: StreamWriteOutcome = net::write_chunks_until(stream, byte_chunks_one(socket_chunk), write_deadline)\n",
            "  let cancellable_socket_writes: StreamWriteOutcome = net::write_chunks_until_cancellable(stream, byte_chunks_one(socket_chunk), write_deadline, read_token)\n",
            "  net::shutdown_write(stream)\n",
            "  net::shutdown_read(stream)\n",
            "  net::close_stream(stream)\n",
            "  time::timeout_ms(1)\n",
            "  let deadline: Deadline = time::deadline_after_ms(1)\n",
            "  time::wait_until(deadline)\n",
            "  let token: CancelToken = time::cancel_token()\n",
            "  time::wait_until_cancellable(deadline, token)\n",
            "  let outcome: CancellableWaitOutcome = time::wait_until_cancellable_outcome(deadline, token)\n",
            "  let owner: CancelOwner = time::cancel_owner()\n",
            "  let observer_token: CancelToken = time::cancel_token_from(owner)\n",
            "  time::cancel_owned(owner)\n",
            "  let owner_cancelled: Bool = time::is_cancelled_owner(owner)\n",
            "  let observer_cancelled: Bool = time::is_cancelled(observer_token)\n",
            "  time::cancel(token)\n",
            "  let cancelled: Bool = time::is_cancelled(token)\n",
            "  let elapsed: Int = time::monotonic_ms()\n",
            "  let connected_stream: NetStream = net::connect(\"127.0.0.1:0\")\n",
            "  fs::read_to_string(path)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("fs and process calls should lower");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    assert_eq!(
        standard_library_builtin_calls(main),
        [
            ("let", "process::cwd"),
            ("let", "process::env"),
            ("let", "net::receive_chunk"),
            ("expr", "net::send_chunk"),
            ("let", "net::listen"),
            ("let", "net::accept"),
            ("let", "net::accept_or_end"),
            ("let", "time::deadline_after_ms"),
            ("let", "net::accept_until"),
            ("let", "time::cancel_token"),
            ("let", "net::accept_until_cancellable"),
            ("expr", "net::close_listener"),
            ("let", "net::listener_local_addr"),
            ("let", "net::stream_local_addr"),
            ("let", "net::stream_peer_addr"),
            ("let", "net::stream_can_read"),
            ("let", "net::stream_can_write"),
            ("let", "net::stream_is_closed"),
            ("let", "net::read_chunk"),
            ("let", "net::read_chunk_or_end"),
            ("let", "time::deadline_after_ms"),
            ("let", "net::read_chunk_until"),
            ("let", "time::cancel_token"),
            ("let", "net::read_chunk_until_cancellable"),
            ("expr", "net::write_chunk"),
            ("let", "time::deadline_after_ms"),
            ("let", "net::write_chunk_until"),
            ("let", "net::write_chunk_until_cancellable"),
            ("expr", "net::write_chunks"),
            ("let", "net::write_chunks_until"),
            ("let", "net::write_chunks_until_cancellable"),
            ("expr", "net::shutdown_write"),
            ("expr", "net::shutdown_read"),
            ("expr", "net::close_stream"),
            ("expr", "time::timeout_ms"),
            ("let", "time::deadline_after_ms"),
            ("expr", "time::wait_until"),
            ("let", "time::cancel_token"),
            ("expr", "time::wait_until_cancellable"),
            ("let", "time::wait_until_cancellable_outcome"),
            ("let", "time::cancel_owner"),
            ("let", "time::cancel_token_from"),
            ("expr", "time::cancel_owned"),
            ("let", "time::is_cancelled_owner"),
            ("let", "time::is_cancelled"),
            ("expr", "time::cancel"),
            ("let", "time::is_cancelled"),
            ("let", "time::monotonic_ms"),
            ("let", "net::connect"),
            ("return", "fs::read_to_string"),
        ]
    );
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
                    target: IrCallTarget::ConcurrencyBuiltin(name),
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
                    target: IrCallTarget::ConcurrencyBuiltin(name),
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
