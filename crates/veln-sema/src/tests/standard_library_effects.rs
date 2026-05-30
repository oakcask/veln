use super::*;

#[test]
fn source_backed_prelude_helpers_report_direct_argument_diagnostics() {
    for (helper, source_text, expected_message) in [
        (
            "vec_is_empty",
            concat!(
                "pub fn main(value: Int) -> Bool effects []\n",
                "  vec_is_empty(value)\n",
                "end\n",
            ),
            "expected `Vec(unknown)`, but found `Int`",
        ),
        (
            "vec_push",
            concat!(
                "pub fn main(value: Int) -> Vec(Int) effects []\n",
                "  vec_push(value, 1)\n",
                "end\n",
            ),
            "expected `Vec(Int)`, but found `Int`",
        ),
        (
            "vec_concat",
            concat!(
                "pub fn main(value: Int, other: Vec(Int)) -> Vec(Int) effects []\n",
                "  vec_concat(value, other)\n",
                "end\n",
            ),
            "expected `Vec(Int)`, but found `Int`",
        ),
        (
            "vec_map",
            concat!(
                "fn stringify(value: Int) -> String effects []\n",
                "  \"ok\"\n",
                "end\n",
                "pub fn main(value: Int) -> Vec(String) effects []\n",
                "  vec_map(value, stringify)\n",
                "end\n",
            ),
            "expected `Vec(unknown)`, but found `Int`",
        ),
        (
            "vec_try_map",
            concat!(
                "fn stringify(value: Int) -> Result(String, String) effects []\n",
                "  Ok(\"ok\")\n",
                "end\n",
                "pub fn main(value: Int) -> Result(Vec(String), String) effects []\n",
                "  vec_try_map(value, stringify)\n",
                "end\n",
            ),
            "expected `Vec(unknown)`, but found `Int`",
        ),
        (
            "vec_try_map_with",
            concat!(
                "fn stringify(context: String, value: Int) -> Result(String, String) effects []\n",
                "  Ok(context)\n",
                "end\n",
                "pub fn main(value: Int) -> Result(Vec(String), String) effects []\n",
                "  vec_try_map_with(\"prefix\", value, stringify)\n",
                "end\n",
            ),
            "expected `Vec(unknown)`, but found `Int`",
        ),
        (
            "list_is_empty",
            concat!(
                "type List(A)\n",
                "  Nil\n",
                "  Cons(head: A, tail: List(A))\n",
                "end\n",
                "pub fn main(value: Int) -> Bool effects []\n",
                "  list_is_empty(value)\n",
                "end\n",
            ),
            "expected `List(unknown)`, but found `Int`",
        ),
        (
            "list_map",
            concat!(
                "type List(A)\n",
                "  Nil\n",
                "  Cons(head: A, tail: List(A))\n",
                "end\n",
                "fn stringify(value: Int) -> String effects []\n",
                "  \"ok\"\n",
                "end\n",
                "pub fn main(value: Int) -> List(String) effects []\n",
                "  list_map(value, stringify)\n",
                "end\n",
            ),
            "expected `List(unknown)`, but found `Int`",
        ),
        (
            "list_try_map",
            concat!(
                "type List(A)\n",
                "  Nil\n",
                "  Cons(head: A, tail: List(A))\n",
                "end\n",
                "fn stringify(value: Int) -> Result(String, String) effects []\n",
                "  Ok(\"ok\")\n",
                "end\n",
                "pub fn main(value: Int) -> Result(List(String), String) effects []\n",
                "  list_try_map(value, stringify)\n",
                "end\n",
            ),
            "expected `List(unknown)`, but found `Int`",
        ),
        (
            "dict_get",
            concat!(
                "pub fn main(value: Int) -> Option(String) effects []\n",
                "  dict_get(value, \"key\")\n",
                "end\n",
            ),
            "expected `Dict(unknown, String)`, but found `Int`",
        ),
        (
            "dict_contains",
            concat!(
                "pub fn main(value: Int) -> Bool effects []\n",
                "  dict_contains(value, \"key\")\n",
                "end\n",
            ),
            "expected `Dict(unknown, unknown)`, but found `Int`",
        ),
        (
            "dict_insert",
            concat!(
                "pub fn main(value: Int) -> Dict(String, Int) effects []\n",
                "  dict_insert(value, \"key\", 1)\n",
                "end\n",
            ),
            "expected `Dict(String, Int)`, but found `Int`",
        ),
        (
            "dict_remove",
            concat!(
                "pub fn main(value: Int) -> Dict(String, Int) effects []\n",
                "  dict_remove(value, \"key\")\n",
                "end\n",
            ),
            "expected `Dict(String, Int)`, but found `Int`",
        ),
        (
            "int_to_string",
            concat!(
                "pub fn main(value: String) -> String effects []\n",
                "  int_to_string(value)\n",
                "end\n",
            ),
            "expected `Int`, but found `String`",
        ),
        (
            "string_parse_int",
            concat!(
                "pub fn main(value: Int) -> Result(Int, String) effects []\n",
                "  string_parse_int(value)\n",
                "end\n",
            ),
            "expected `String`, but found `Int`",
        ),
        (
            "string_split_once",
            concat!(
                "pub fn main(value: Int) -> Option({left: String, right: String}) effects []\n",
                "  string_split_once(value, \",\")\n",
                "end\n",
            ),
            "expected `String`, but found `Int`",
        ),
    ] {
        assert_helper_user_call_site_type_mismatch(helper, source_text, expected_message);
    }
}

fn assert_helper_user_call_site_type_mismatch(
    helper: &str,
    source_text: &'static str,
    expected_message: &'static str,
) {
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{helper}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{helper}");
    assert_eq!(diagnostics[0].message, expected_message, "{helper}");
    let span = diagnostics[0]
        .span
        .as_ref()
        .expect("diagnostic should point at user source");
    assert_eq!(span.file.as_str(), "main.veln");
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
    assert!(details.contains("\"provenance_paths\":[{\"effect\":\"stdio\""));
    assert!(details.contains("\"kind\":\"public_boundary\""));
    assert!(details.contains("\"hidden_frame_count\":0"));
    assert!(details.contains("\"omitted_path_count\":0"));
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn channel_calls_require_concurrency_effect() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(tx: Sender(String)) -> Result((), SendError) effects []\n",
            "  channel::send(tx, \"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `concurrency`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"concurrency\""));
    assert!(details.contains("\"inferred_effects\":[\"concurrency\"]"));
    assert!(details.contains("\"symbol\":\"channel::send\""));
}

#[test]
fn task_calls_require_concurrency_effect() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn produce() -> String effects []\n",
            "  \"hello\"\n",
            "end\n",
            "pub fn main() -> Task(String) effects []\n",
            "  task::spawn(produce)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `concurrency`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"concurrency\""));
    assert!(details.contains("\"symbol\":\"task::spawn\""));
}

#[test]
fn fs_calls_require_fs_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(path: Path) -> Result(String, FsError) effects []\n",
            "  fs::read_to_string(path)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `fs`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"fs\""));
    assert!(details.contains("\"inferred_effects\":[\"fs\"]"));
    assert!(details.contains("\"symbol\":\"fs::read_to_string\""));
}

#[test]
fn fs_calls_require_path_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(path: String) -> Result(String, FsError) effects [fs]\n",
            "  fs::read_to_string(path)\n",
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
        "expected `Path`, but found `String`"
    );
}

#[test]
fn process_cwd_path_return_is_not_assignable_to_string() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Result(String, ProcessError) effects [process]\n",
            "  let cwd: Result(String, ProcessError) = process::cwd()\n",
            "  cwd\n",
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
        "expected `Result(String, ProcessError)`, but found `Result(Path, ProcessError)`"
    );
}

#[test]
fn process_calls_require_process_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Vec(String) effects []\n",
            "  process::args()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `process`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"process\""));
    assert!(details.contains("\"symbol\":\"process::args\""));
}

#[test]
fn fs_and_process_calls_lower_to_standard_library_builtins() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(path: Path, key: String) -> Result(String, FsError) effects [fs, process]\n",
            "  let cwd: Result(Path, ProcessError) = process::cwd()\n",
            "  let present: Option(String) = process::env(key)\n",
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
    let IrStmtKind::Let { value, .. } = &main.body[0].kind else {
        panic!("cwd call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "process::cwd"
    ));
    let IrStmtKind::Return { value } = &main.body[2].kind else {
        panic!("fs call should lower as tail return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "fs::read_to_string"
    ));
}

#[test]
fn task_spawn_and_join_preserve_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn produce() -> String effects []\n",
            "  \"hello\"\n",
            "end\n",
            "pub fn main() -> Result(String, JoinError) effects [concurrency]\n",
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
fn declared_concurrency_calls_lower_to_executable_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String effects [concurrency]\n",
            "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
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
            "  let pair = channel::bounded[String](1)\n",
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
            "pub fn main(tx: Sender(String)) -> Result((), SendError) effects [concurrency]\n",
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
            "pub fn main(tx: Sender(String)) -> Result((), SendError) effects [concurrency]\n",
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
