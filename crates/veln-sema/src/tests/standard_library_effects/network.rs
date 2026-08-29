use super::*;

#[test]
fn connect_requires_net_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> NetStream\n",
            "  net::connect(\"127.0.0.1:0\")\n",
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
    assert!(details.contains("\"symbol\":\"net::connect\""));
}

#[test]
fn listener_address_call_requires_net_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(listener: NetListener) -> String\n",
            "  net::listener_local_addr(listener)\n",
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
    assert!(details.contains("\"symbol\":\"net::listener_local_addr\""));
}

#[test]
fn stream_address_calls_require_net_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(stream: NetStream) -> String\n",
            "  net::stream_local_addr(stream)\n",
            "end\n",
            "\n",
            "pub fn peer(stream: NetStream) -> String\n",
            "  net::stream_peer_addr(stream)\n",
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
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\"]"));
    assert!(details.contains("\"symbol\":\"net::stream_local_addr\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\"]"));
    assert!(details.contains("\"symbol\":\"net::stream_peer_addr\""));
}

#[test]
fn stream_state_calls_require_net_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn can_read(stream: NetStream) -> Bool\n",
            "  net::stream_can_read(stream)\n",
            "end\n",
            "\n",
            "pub fn can_write(stream: NetStream) -> Bool\n",
            "  net::stream_can_write(stream)\n",
            "end\n",
            "\n",
            "pub fn closed(stream: NetStream) -> Bool\n",
            "  net::stream_is_closed(stream)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\"]"));
    assert!(details.contains("\"symbol\":\"net::stream_can_read\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"symbol\":\"net::stream_can_write\""));

    assert_eq!(diagnostics[2].id, "effect.missing_public");
    let details = diagnostics[2].details.to_json();
    assert!(details.contains("\"symbol\":\"net::stream_is_closed\""));
}

#[test]
fn write_chunks_requires_net_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(stream: NetStream, chunks: List<ByteChunk>) -> ()\n",
            "  net::write_chunks(stream, chunks)\n",
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
    assert!(details.contains("\"symbol\":\"net::write_chunks\""));
}

#[test]
fn close_listener_requires_net_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(listener: NetListener) -> ()\n",
            "  net::close_listener(listener)\n",
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
    assert!(details.contains("\"symbol\":\"net::close_listener\""));
}

#[test]
fn shutdown_read_requires_net_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(stream: NetStream) -> ()\n",
            "  net::shutdown_read(stream)\n",
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
    assert!(details.contains("\"symbol\":\"net::shutdown_read\""));
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
fn accept_until_cancellable_requires_net_and_time_effects_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn missing_time(listener: NetListener, deadline: Deadline, token: CancelToken) -> AcceptOutcome effects [net]\n",
            "  net::accept_until_cancellable(listener, deadline, token)\n",
            "end\n",
            "\n",
            "pub fn missing_net(listener: NetListener, deadline: Deadline, token: CancelToken) -> AcceptOutcome effects [time]\n",
            "  net::accept_until_cancellable(listener, deadline, token)\n",
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
    assert!(details.contains("\"symbol\":\"net::accept_until_cancellable\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::accept_until_cancellable\""));
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
fn read_chunk_until_cancellable_requires_net_and_time_effects_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn missing_time(stream: NetStream, deadline: Deadline, token: CancelToken) -> StreamReadOutcome effects [net]\n",
            "  net::read_chunk_until_cancellable(stream, deadline, token)\n",
            "end\n",
            "\n",
            "pub fn missing_net(stream: NetStream, deadline: Deadline, token: CancelToken) -> StreamReadOutcome effects [time]\n",
            "  net::read_chunk_until_cancellable(stream, deadline, token)\n",
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
    assert!(details.contains("\"symbol\":\"net::read_chunk_until_cancellable\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::read_chunk_until_cancellable\""));
}

#[test]
fn write_chunk_until_cancellable_requires_net_and_time_effects_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn missing_time(stream: NetStream, chunk: ByteChunk, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [net]\n",
            "  net::write_chunk_until_cancellable(stream, chunk, deadline, token)\n",
            "end\n",
            "\n",
            "pub fn missing_net(stream: NetStream, chunk: ByteChunk, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [time]\n",
            "  net::write_chunk_until_cancellable(stream, chunk, deadline, token)\n",
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
    assert!(details.contains("\"symbol\":\"net::write_chunk_until_cancellable\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::write_chunk_until_cancellable\""));
}

#[test]
fn write_chunk_until_requires_net_and_time_effects_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn missing_time(stream: NetStream, chunk: ByteChunk, deadline: Deadline) -> StreamWriteOutcome effects [net]\n",
            "  net::write_chunk_until(stream, chunk, deadline)\n",
            "end\n",
            "\n",
            "pub fn missing_net(stream: NetStream, chunk: ByteChunk, deadline: Deadline) -> StreamWriteOutcome effects [time]\n",
            "  net::write_chunk_until(stream, chunk, deadline)\n",
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
    assert!(details.contains("\"symbol\":\"net::write_chunk_until\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::write_chunk_until\""));
}

#[test]
fn write_chunks_until_cancellable_requires_net_and_time_effects_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn missing_time(stream: NetStream, chunks: List<ByteChunk>, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [net]\n",
            "  net::write_chunks_until_cancellable(stream, chunks, deadline, token)\n",
            "end\n",
            "\n",
            "pub fn missing_net(stream: NetStream, chunks: List<ByteChunk>, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [time]\n",
            "  net::write_chunks_until_cancellable(stream, chunks, deadline, token)\n",
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
    assert!(details.contains("\"symbol\":\"net::write_chunks_until_cancellable\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::write_chunks_until_cancellable\""));
}

#[test]
fn stream_adapter_cancellable_write_drain_requires_net_time_and_concurrency_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn handle_input(input: StreamInput) -> List<StreamAdapterAction>\n",
            "  match input\n",
            "    Chunk(bytes) => list_cons(SendBytes(bytes), list_nil())\n",
            "    End => list_cons(EndStream, list_nil())\n",
            "  end\n",
            "end\n",
            "\n",
            "pub fn missing_time(stream: NetStream, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [net, concurrency]\n",
            "  stream_adapter_drain_actions_until_cancellable(stream, handle_input, deadline, token)\n",
            "end\n",
            "\n",
            "pub fn missing_concurrency(stream: NetStream, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [net, time]\n",
            "  stream_adapter_drain_actions_until_cancellable(stream, handle_input, deadline, token)\n",
            "end\n",
            "\n",
            "pub fn missing_net(stream: NetStream, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [time, concurrency]\n",
            "  stream_adapter_drain_actions_until_cancellable(stream, handle_input, deadline, token)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\",\"concurrency\"]"));
    assert!(details.contains("\"symbol\":\"stream_adapter_drain_actions_until_cancellable\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `concurrency`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"concurrency\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\",\"concurrency\"]"));
    assert!(details.contains("\"symbol\":\"stream_adapter_drain_actions_until_cancellable\""));

    assert_eq!(diagnostics[2].id, "effect.missing_public");
    assert_eq!(
        diagnostics[2].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[2].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\",\"concurrency\"]"));
    assert!(details.contains("\"symbol\":\"stream_adapter_drain_actions_until_cancellable\""));
}

#[test]
fn stream_adapter_accept_loop_requires_net_and_concurrency_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn handle_input(input: StreamInput) -> List<StreamAdapterAction>\n",
            "  match input\n",
            "    Chunk(bytes) => list_cons(SendBytes(bytes), list_nil())\n",
            "    End => list_cons(EndStream, list_nil())\n",
            "  end\n",
            "end\n",
            "\n",
            "pub fn missing_concurrency(listener: NetListener) -> () effects [net]\n",
            "  stream_adapter_accept_loop(listener, handle_input)\n",
            "end\n",
            "\n",
            "pub fn missing_net(listener: NetListener) -> () effects [concurrency]\n",
            "  stream_adapter_accept_loop(listener, handle_input)\n",
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
        "public function uses undeclared effect `concurrency`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"concurrency\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"concurrency\"]"));
    assert!(details.contains("\"symbol\":\"stream_adapter_accept_loop\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"concurrency\"]"));
    assert!(details.contains("\"symbol\":\"stream_adapter_accept_loop\""));
}

#[test]
fn write_chunks_until_requires_net_and_time_effects_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn missing_time(stream: NetStream, chunks: List<ByteChunk>, deadline: Deadline) -> StreamWriteOutcome effects [net]\n",
            "  net::write_chunks_until(stream, chunks, deadline)\n",
            "end\n",
            "\n",
            "pub fn missing_net(stream: NetStream, chunks: List<ByteChunk>, deadline: Deadline) -> StreamWriteOutcome effects [time]\n",
            "  net::write_chunks_until(stream, chunks, deadline)\n",
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
    assert!(details.contains("\"symbol\":\"net::write_chunks_until\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `net`"
    );
    let details = diagnostics[1].details.to_json();
    assert!(details.contains("\"effect\":\"net\""));
    assert!(details.contains("\"inferred_effects\":[\"net\",\"time\"]"));
    assert!(details.contains("\"symbol\":\"net::write_chunks_until\""));
}
