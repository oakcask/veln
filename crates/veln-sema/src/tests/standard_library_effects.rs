use super::*;

#[test]
fn source_backed_prelude_helpers_report_direct_argument_diagnostics() {
    for (helper, source_text, expected_message) in [
        (
            "vec_is_empty",
            concat!(
                "pub fn main(value: Int) -> Bool\n",
                "  vec_is_empty(value)\n",
                "end\n",
            ),
            "expected `Vec<unknown>`, but found `Int`",
        ),
        (
            "vec_push",
            concat!(
                "pub fn main(value: Int) -> Vec<Int>\n",
                "  vec_push(value, 1)\n",
                "end\n",
            ),
            "expected `Vec<Int>`, but found `Int`",
        ),
        (
            "vec_concat",
            concat!(
                "pub fn main(value: Int, other: Vec<Int>) -> Vec<Int>\n",
                "  vec_concat(value, other)\n",
                "end\n",
            ),
            "expected `Vec<Int>`, but found `Int`",
        ),
        (
            "vec_map",
            concat!(
                "fn stringify(value: Int) -> String\n",
                "  \"ok\"\n",
                "end\n",
                "pub fn main(value: Int) -> Vec<String>\n",
                "  vec_map(value, stringify)\n",
                "end\n",
            ),
            "expected `Vec<unknown>`, but found `Int`",
        ),
        (
            "vec_try_map",
            concat!(
                "fn stringify(value: Int) -> Result<String, String>\n",
                "  Ok(\"ok\")\n",
                "end\n",
                "pub fn main(value: Int) -> Result<Vec<String>, String>\n",
                "  vec_try_map(value, stringify)\n",
                "end\n",
            ),
            "expected `Vec<unknown>`, but found `Int`",
        ),
        (
            "vec_try_map_with",
            concat!(
                "fn stringify(context: String, value: Int) -> Result<String, String>\n",
                "  Ok(context)\n",
                "end\n",
                "pub fn main(value: Int) -> Result<Vec<String>, String>\n",
                "  vec_try_map_with(\"prefix\", value, stringify)\n",
                "end\n",
            ),
            "expected `Vec<unknown>`, but found `Int`",
        ),
        (
            "list_is_empty",
            concat!(
                "type List<A>\n",
                "  Nil\n",
                "  Cons(head: A, tail: List<A>)\n",
                "end\n",
                "pub fn main(value: Int) -> Bool\n",
                "  list_is_empty(value)\n",
                "end\n",
            ),
            "expected `List<unknown>`, but found `Int`",
        ),
        (
            "list_map",
            concat!(
                "type List<A>\n",
                "  Nil\n",
                "  Cons(head: A, tail: List<A>)\n",
                "end\n",
                "fn stringify(value: Int) -> String\n",
                "  \"ok\"\n",
                "end\n",
                "pub fn main(value: Int) -> List<String>\n",
                "  list_map(value, stringify)\n",
                "end\n",
            ),
            "expected `List<unknown>`, but found `Int`",
        ),
        (
            "list_try_map",
            concat!(
                "type List<A>\n",
                "  Nil\n",
                "  Cons(head: A, tail: List<A>)\n",
                "end\n",
                "fn stringify(value: Int) -> Result<String, String>\n",
                "  Ok(\"ok\")\n",
                "end\n",
                "pub fn main(value: Int) -> Result<List<String>, String>\n",
                "  list_try_map(value, stringify)\n",
                "end\n",
            ),
            "expected `List<unknown>`, but found `Int`",
        ),
        (
            "dict_get",
            concat!(
                "pub fn main(value: Int) -> Option<String>\n",
                "  dict_get(value, \"key\")\n",
                "end\n",
            ),
            "expected `Dict<unknown, String>`, but found `Int`",
        ),
        (
            "dict_contains",
            concat!(
                "pub fn main(value: Int) -> Bool\n",
                "  dict_contains(value, \"key\")\n",
                "end\n",
            ),
            "expected `Dict<unknown, unknown>`, but found `Int`",
        ),
        (
            "dict_insert",
            concat!(
                "pub fn main(value: Int) -> Dict<String, Int>\n",
                "  dict_insert(value, \"key\", 1)\n",
                "end\n",
            ),
            "expected `Dict<String, Int>`, but found `Int`",
        ),
        (
            "dict_remove",
            concat!(
                "pub fn main(value: Int) -> Dict<String, Int>\n",
                "  dict_remove(value, \"key\")\n",
                "end\n",
            ),
            "expected `Dict<String, Int>`, but found `Int`",
        ),
        (
            "int_to_string",
            concat!(
                "pub fn main(value: String) -> String\n",
                "  int_to_string(value)\n",
                "end\n",
            ),
            "expected `Int`, but found `String`",
        ),
        (
            "string_parse_int",
            concat!(
                "pub fn main(value: Int) -> Result<Int, String>\n",
                "  string_parse_int(value)\n",
                "end\n",
            ),
            "expected `String`, but found `Int`",
        ),
        (
            "string_split_once",
            concat!(
                "pub fn main(value: Int) -> Option<{left: String, right: String}>\n",
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
            "pub fn main() -> ()\n",
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
            "pub fn main() -> ()\n",
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
            "pub fn main(tx: Sender<String>) -> Result<(), SendError>\n",
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
fn cancellable_channel_select_many_timeout_requires_time_and_concurrency_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>, token: CancelToken) -> Result<Option<{index: Int, value: String}>, SelectError>\n",
            "  channel::select_many_timeout_cancellable(receivers, 10, token)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `time`"
    );
    let time_details = diagnostics[0].details.to_json();
    assert!(time_details.contains("\"effect\":\"time\""));
    assert!(time_details.contains("\"inferred_effects\":[\"time\",\"concurrency\"]"));
    assert!(time_details.contains("\"symbol\":\"channel::select_many_timeout_cancellable\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `concurrency`"
    );
    let concurrency_details = diagnostics[1].details.to_json();
    assert!(concurrency_details.contains("\"effect\":\"concurrency\""));
    assert!(concurrency_details.contains("\"inferred_effects\":[\"time\",\"concurrency\"]"));
    assert!(
        concurrency_details.contains("\"symbol\":\"channel::select_many_timeout_cancellable\"")
    );
}

#[test]
fn cancellable_channel_select_timeout_requires_time_and_concurrency_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>, token: CancelToken) -> Result<Option<{index: Int, value: String}>, SelectError>\n",
            "  channel::select_timeout_cancellable(left, right, 10, token)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `time`"
    );
    let time_details = diagnostics[0].details.to_json();
    assert!(time_details.contains("\"effect\":\"time\""));
    assert!(time_details.contains("\"inferred_effects\":[\"time\",\"concurrency\"]"));
    assert!(time_details.contains("\"symbol\":\"channel::select_timeout_cancellable\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `concurrency`"
    );
    let concurrency_details = diagnostics[1].details.to_json();
    assert!(concurrency_details.contains("\"effect\":\"concurrency\""));
    assert!(concurrency_details.contains("\"inferred_effects\":[\"time\",\"concurrency\"]"));
    assert!(concurrency_details.contains("\"symbol\":\"channel::select_timeout_cancellable\""));
}

#[test]
fn task_calls_require_concurrency_effect() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn produce() -> String\n",
            "  \"hello\"\n",
            "end\n",
            "pub fn main() -> Task<String>\n",
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
            "pub fn main(path: Path) -> Result<String, FsError>\n",
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
fn fs_calls_reject_string_for_every_path_parameter() {
    for (name, source_text) in [
        (
            "read_to_string",
            concat!(
                "pub fn main(path: String) -> Result<String, FsError> effects [fs]\n",
                "  fs::read_to_string(path)\n",
                "end\n",
            ),
        ),
        (
            "write_string",
            concat!(
                "pub fn main(path: String) -> Result<(), FsError> effects [fs]\n",
                "  fs::write_string(path, \"text\")\n",
                "end\n",
            ),
        ),
        (
            "exists",
            concat!(
                "pub fn main(path: String) -> Result<Bool, FsError> effects [fs]\n",
                "  fs::exists(path)\n",
                "end\n",
            ),
        ),
        (
            "read_dir",
            concat!(
                "pub fn main(path: String) -> Result<Vec<Path>, FsError> effects [fs]\n",
                "  fs::read_dir(path)\n",
                "end\n",
            ),
        ),
    ] {
        let source = SourceFile::new("main.veln", source_text);
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1, "{name}");
        assert_eq!(diagnostics[0].id, "type.mismatch", "{name}");
        assert_eq!(
            diagnostics[0].message, "expected `Path`, but found `String`",
            "{name}"
        );
    }
}

#[test]
fn process_cwd_path_return_is_not_assignable_to_string() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Result<String, ProcessError> effects [process]\n",
            "  let cwd: Result<String, ProcessError> = process::cwd()\n",
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
        "expected `Result<String, ProcessError>`, but found `Result<Path, ProcessError>`"
    );
}

#[test]
fn process_cwd_path_value_is_not_usable_as_string_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option<String> effects [process]\n",
            "  match process::cwd()\n",
            "    Ok(cwd) => process::env(cwd)\n",
            "    Err(_) => None\n",
            "  end\n",
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
        "expected `String`, but found `Path`"
    );
}

#[test]
fn process_calls_require_process_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Vec<String>\n",
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
fn net_calls_require_net_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option<ByteChunk>\n",
            "  let listener: NetListener = net::listen(\"127.0.0.1:0\")\n",
            "  let stream: NetStream = net::accept(listener)\n",
            "  let optional_stream: Option<NetStream> = net::accept_or_end(listener)\n",
            "  let _ = net::read_chunk(stream)\n",
            "  net::close_stream(stream)\n",
            "  net::read_chunk_or_end(stream)\n",
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
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\"]"));
    assert!(details.contains("\"symbol\":\"net::listen\""));
}

#[test]
fn accept_until_requires_net_and_time_effects_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn missing_time(listener: NetListener, deadline: Deadline) -> Option<NetStream> effects [net]\n",
            "  net::accept_until(listener, deadline)\n",
            "end\n",
            "\n",
            "pub fn missing_net(listener: NetListener, deadline: Deadline) -> Option<NetStream> effects [time]\n",
            "  net::accept_until(listener, deadline)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::accept_until\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::accept_until\""));
}

#[test]
fn read_chunk_until_requires_net_and_time_effects_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn missing_time(stream: NetStream, deadline: Deadline) -> Option<ByteChunk> effects [net]\n",
            "  net::read_chunk_until(stream, deadline)\n",
            "end\n",
            "\n",
            "pub fn missing_net(stream: NetStream, deadline: Deadline) -> Option<ByteChunk> effects [time]\n",
            "  net::read_chunk_until(stream, deadline)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::read_chunk_until\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::read_chunk_until\""));
}

#[test]
fn time_calls_require_time_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "  let deadline: Deadline = time::deadline_after_ms(10)\n",
            "  let token: CancelToken = time::cancel_token()\n",
            "  time::wait_until_cancellable(deadline, token)\n",
            "  time::wait_until(deadline)\n",
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
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"time\"]"));
    assert!(details.contains("\"symbol\":\"time::deadline_after_ms\""));
}

#[test]
fn cancellation_status_query_requires_time_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn token_status(token: CancelToken) -> Bool\n",
            "  time::is_cancelled(token)\n",
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
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"time\"]"));
    assert!(details.contains("\"symbol\":\"time::is_cancelled\""));
}

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
            "  let socket_chunk: ByteChunk = net::read_chunk(stream)\n",
            "  let socket_chunk_or_end: Option<ByteChunk> = net::read_chunk_or_end(stream)\n",
            "  let read_deadline: Deadline = time::deadline_after_ms(1)\n",
            "  let socket_chunk_until: Option<ByteChunk> = net::read_chunk_until(stream, read_deadline)\n",
            "  net::write_chunk(stream, socket_chunk)\n",
            "  net::close_stream(stream)\n",
            "  time::timeout_ms(1)\n",
            "  let deadline: Deadline = time::deadline_after_ms(1)\n",
            "  time::wait_until(deadline)\n",
            "  let token: CancelToken = time::cancel_token()\n",
            "  time::wait_until_cancellable(deadline, token)\n",
            "  let outcome: CancellableWaitOutcome = time::wait_until_cancellable_outcome(deadline, token)\n",
            "  time::cancel(token)\n",
            "  let cancelled: Bool = time::is_cancelled(token)\n",
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
    let IrStmtKind::Let { value, .. } = &main.body[2].kind else {
        panic!("net receive call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::receive_chunk"
    ));
    let IrStmtKind::Expr { value } = &main.body[3].kind else {
        panic!("net send call should lower as an expression");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::send_chunk"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[4].kind else {
        panic!("net listen call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::listen"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[5].kind else {
        panic!("net accept call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::accept"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[6].kind else {
        panic!("net optional accept call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::accept_or_end"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[7].kind else {
        panic!("accept deadline call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::deadline_after_ms"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[8].kind else {
        panic!("net deadline accept call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::accept_until"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[9].kind else {
        panic!("net read call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::read_chunk"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[10].kind else {
        panic!("net optional read call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::read_chunk_or_end"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[11].kind else {
        panic!("read deadline call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::deadline_after_ms"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[12].kind else {
        panic!("net deadline read call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::read_chunk_until"
    ));
    let IrStmtKind::Expr { value } = &main.body[13].kind else {
        panic!("net write call should lower as an expression");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::write_chunk"
    ));
    let IrStmtKind::Expr { value } = &main.body[14].kind else {
        panic!("net close call should lower as an expression");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "net::close_stream"
    ));
    let IrStmtKind::Expr { value } = &main.body[15].kind else {
        panic!("time call should lower as an expression");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::timeout_ms"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[16].kind else {
        panic!("deadline call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::deadline_after_ms"
    ));
    let IrStmtKind::Expr { value } = &main.body[17].kind else {
        panic!("wait call should lower as an expression");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::wait_until"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[18].kind else {
        panic!("cancel token call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::cancel_token"
    ));
    let IrStmtKind::Expr { value } = &main.body[19].kind else {
        panic!("cancellable wait call should lower as an expression");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::wait_until_cancellable"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[20].kind else {
        panic!("cancellable wait outcome call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::wait_until_cancellable_outcome"
    ));
    let IrStmtKind::Expr { value } = &main.body[21].kind else {
        panic!("cancel call should lower as an expression");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::cancel"
    ));
    let IrStmtKind::Let { value, .. } = &main.body[22].kind else {
        panic!("cancel status call should lower as a let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StandardLibraryBuiltin(symbol),
            ..
        } if symbol == "time::is_cancelled"
    ));
    let IrStmtKind::Return { value } = &main.body[23].kind else {
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
            "fn produce(input: String) -> String effects [concurrency]\n",
            "  input\n",
            "end\n",
            "pub fn main(input: String) -> Result<String, JoinError> effects [concurrency]\n",
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
            "fn produce(input: String) -> String effects [concurrency]\n",
            "  input\n",
            "end\n",
            "pub fn main(input: String) -> Result<String, JoinError> effects [concurrency]\n",
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
            "fn route(context: {payload: String, marker: Int}) -> String effects [concurrency]\n",
            "  context.payload\n",
            "end\n",
            "pub fn main(payload: String, marker: Int) -> Result<String, JoinError> effects [concurrency]\n",
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
