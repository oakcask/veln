use super::*;

#[test]
fn removed_legacy_network_call_is_one_unresolved_diagnostic_with_migration_notes() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> ()\n  net::connect(\"127.0.0.1:0\")\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.id, "name.unresolved");
    assert_eq!(diagnostic.message, "unresolved call_target `net::connect`");
    assert_eq!(diagnostic.related.len(), 3);
    let related = diagnostic
        .related
        .iter()
        .map(|value| value.to_json())
        .collect::<Vec<_>>();
    assert!(
        related[0].contains("use net from \\\"std\\\""),
        "{related:#?}"
    );
    assert!(related[1].contains("net::IO"), "{related:#?}");
    assert!(related[2].contains("net::system()"), "{related:#?}");
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
