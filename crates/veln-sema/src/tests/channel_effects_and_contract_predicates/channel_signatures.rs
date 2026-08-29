use super::*;

#[test]
fn channel_recv_checks_receiver_against_expected_option_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(rx: Receiver<Int>) -> Option<String> effects [concurrency]\n",
            "  channel::recv(rx)\n",
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
        "expected `Receiver<String>`, but found `Receiver<Int>`"
    );
}

#[test]
fn channel_select_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select(left, right)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_checks_both_receivers_against_same_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<Int>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select(left, right)\n",
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
        "expected `Receiver<String>`, but found `Receiver<Int>`"
    );
}

#[test]
fn channel_select_priority_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_priority(left, right)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select priority return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_many_priority_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_many_priority(receivers)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select many priority return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_many_priority_checks_receiver_list_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<Int>>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_many_priority(receivers)\n",
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
        "expected `List<Receiver<String>>`, but found `List<Receiver<Int>>`"
    );
}

#[test]
fn channel_select_many_timeout_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_many_timeout(receivers, 10)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select many timeout return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_many_timeout_requires_integer_timeout() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_many_timeout(receivers, \"soon\")\n",
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
fn channel_select_many_timeout_result_reports_interrupts_with_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [concurrency]\n",
            "  channel::select_many_timeout_result(receivers, 10)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select many timeout result return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::Record(vec![
                ("index".to_string(), CoreType::int()),
                ("value".to_string(), CoreType::string()),
            ])),
            CoreType::named("SelectError", Vec::new())
        )
    );
}

#[test]
fn channel_select_many_timeout_result_requires_integer_timeout() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [concurrency]\n",
            "  channel::select_many_timeout_result(receivers, \"soon\")\n",
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
fn channel_select_many_timeout_cancellable_reports_cancellation_with_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>, token: CancelToken) -> Result<Option<{index: Int, value: String}>, SelectError> effects [time, concurrency]\n",
            "  channel::select_many_timeout_cancellable(receivers, 10, token)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected cancellable select many timeout return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::Record(vec![
                ("index".to_string(), CoreType::int()),
                ("value".to_string(), CoreType::string()),
            ])),
            CoreType::named("SelectError", Vec::new())
        )
    );
}

#[test]
fn channel_select_many_timeout_cancellable_requires_cancel_token() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [time, concurrency]\n",
            "  channel::select_many_timeout_cancellable(receivers, 10, \"stop\")\n",
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
        "expected `CancelToken`, but found `String`"
    );
}

#[test]
fn channel_select_timeout_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_timeout(left, right, 10)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select timeout return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_result_reports_interrupts_with_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [concurrency]\n",
            "  channel::select_result(left, right)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select result return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::Record(vec![
                ("index".to_string(), CoreType::int()),
                ("value".to_string(), CoreType::string()),
            ])),
            CoreType::named("SelectError", Vec::new())
        )
    );
}

#[test]
fn channel_select_timeout_cancellable_reports_interrupts_with_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>, token: CancelToken) -> Result<Option<{index: Int, value: String}>, SelectError> effects [time, concurrency]\n",
            "  channel::select_timeout_cancellable(left, right, 10, token)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected cancellable select timeout return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::Record(vec![
                ("index".to_string(), CoreType::int()),
                ("value".to_string(), CoreType::string()),
            ])),
            CoreType::named("SelectError", Vec::new())
        )
    );
}

#[test]
fn channel_select_timeout_cancellable_requires_cancel_token() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [time, concurrency]\n",
            "  channel::select_timeout_cancellable(left, right, 10, \"stop\")\n",
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
        "expected `CancelToken`, but found `String`"
    );
}

#[test]
fn channel_select_timeout_result_requires_integer_timeout() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [concurrency]\n",
            "  channel::select_timeout_result(left, right, \"soon\")\n",
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
fn channel_select_timeout_requires_integer_timeout() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_timeout(left, right, \"soon\")\n",
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
fn channel_close_requires_sender_handle() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(rx: Receiver<String>) -> () effects [concurrency]\n",
            "  channel::close(rx)\n",
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
        "expected `Sender<unknown>`, but found `Receiver<String>`"
    );
}
