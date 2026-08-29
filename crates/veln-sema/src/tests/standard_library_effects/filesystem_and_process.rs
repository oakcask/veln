use super::*;

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
            "  net::close_listener(listener)\n",
            "  let listener_local: String = net::listener_local_addr(listener)\n",
            "  let local: String = net::stream_local_addr(stream)\n",
            "  let peer: String = net::stream_peer_addr(stream)\n",
            "  let can_read: Bool = net::stream_can_read(stream)\n",
            "  let can_write: Bool = net::stream_can_write(stream)\n",
            "  let closed: Bool = net::stream_is_closed(stream)\n",
            "  let _ = net::read_chunk(stream)\n",
            "  net::shutdown_write(stream)\n",
            "  net::shutdown_read(stream)\n",
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
