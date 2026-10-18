use super::*;

#[test]
fn every_removed_legacy_network_call_has_the_exact_migration_diagnostic() {
    const REMOVED_CALLS: &[&str] = &[
        "receive_chunk",
        "send_chunk",
        "listen",
        "connect",
        "accept",
        "accept_or_end",
        "accept_until",
        "accept_until_cancellable",
        "listener_local_addr",
        "read_chunk",
        "stream_local_addr",
        "stream_peer_addr",
        "stream_can_read",
        "stream_can_write",
        "stream_is_closed",
        "read_chunk_until",
        "read_chunk_until_cancellable",
        "read_chunk_or_end",
        "write_chunk",
        "write_chunk_until",
        "write_chunk_until_cancellable",
        "write_chunks",
        "write_chunks_until",
        "write_chunks_until_cancellable",
        "shutdown_write",
        "shutdown_read",
        "close_stream",
        "close_listener",
    ];
    const EXPECTED_RELATED: &[&str] = &[
        r#"{"kind":"network_import_migration","message":"Import the exported network module with `use net from \"std\"`."}"#,
        r#"{"kind":"network_effect_migration","message":"Declare the nominal `net::IO` effect for functions that use network operations."}"#,
        r#"{"kind":"network_handler_migration","message":"At the application boundary, handle `net::IO` with `net::system()`."}"#,
    ];

    for name in REMOVED_CALLS {
        let symbol = format!("net::{name}");
        let source = SourceFile::new(
            "main.veln",
            format!("fn main() -> ()\n  {symbol}()\n  ()\nend\n"),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1, "{symbol}: {diagnostics:#?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.id, "name.unresolved", "{symbol}");
        assert_eq!(
            diagnostic.message,
            format!("unresolved call_target `{symbol}`"),
            "{symbol}"
        );
        let related = diagnostic
            .related
            .iter()
            .map(|value| value.to_json())
            .collect::<Vec<_>>();
        assert_eq!(related, EXPECTED_RELATED, "{symbol}");
    }
}

#[test]
fn arbitrary_unknown_net_call_has_no_legacy_migration_notes() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> ()\n  net::missing()\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].message,
        "unresolved call_target `net::missing`"
    );
    assert!(diagnostics[0].related.is_empty());
}

#[test]
fn removed_legacy_network_name_in_value_position_has_no_migration_notes() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> ()\n  let operation = net::connect\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message == "unresolved value `net::connect`")
        .expect("the qualified value should be unresolved");

    assert_eq!(diagnostic.id, "name.unresolved");
    assert!(diagnostic.related.is_empty(), "{diagnostic:#?}");
}

#[test]
fn explicit_net_import_suppresses_legacy_migration_notes() {
    let source = SourceFile::new(
        "main.veln",
        "mod main\n\nuse net from \"std\"\n\nfn main() -> ()\n  net::connect()\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message == "unresolved call_target `net::connect`")
        .expect("the imported qualified call should be unresolved without package facts");

    assert_eq!(diagnostic.id, "name.unresolved");
    assert!(diagnostic.related.is_empty(), "{diagnostic:#?}");
}
