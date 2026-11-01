use super::*;

fn module_value_publication_chain_work(
    count: usize,
) -> crate::pipeline::ModuleValuePublicationWork {
    let source = (0..count)
        .map(|index| {
            let visibility = if index + 1 == count { "pub " } else { "" };
            let previous = if index == 0 {
                "0".to_string()
            } else {
                format!("value_{}", index - 1)
            };
            format!(
                "{visibility}let value_{index}: {{ previous: Int, bad: Int }} = {{ previous: {previous}, bad: \"wrong-{index}\" }}\n"
            )
        })
        .collect::<String>();
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    let identities = vec![(None, None, format!("value_{}", count - 1))];
    let (matched_roots, indices, work) =
        lowered.module_value_publication_diagnostic_indices_for_identities(&identities);
    assert_eq!(matched_roots, 1);
    assert_eq!(indices.len(), count, "{:#?}", lowered.diagnostics);
    work
}

#[test]
fn provenance_and_requested_traversal_are_linear() {
    let original = module_value_publication_chain_work(16);
    let adjacent = module_value_publication_chain_work(32);

    assert_eq!(original.retained_values, 16);
    assert_eq!(adjacent.retained_values, 32);
    assert_eq!(original.retained_edges, 15);
    assert_eq!(adjacent.retained_edges, 31);
    assert_eq!(original.retained_direct_causes, 16);
    assert_eq!(adjacent.retained_direct_causes, 32);
    assert_eq!(original.traversal_values, 16);
    assert_eq!(adjacent.traversal_values, 32);
    assert_eq!(original.traversal_edges, 15);
    assert_eq!(adjacent.traversal_edges, 31);
    assert_eq!(original.cause_insertions, 16);
    assert_eq!(adjacent.cause_insertions, 32);
}

fn duplicate_identity_publication_work(
    count: usize,
) -> crate::pipeline::ModuleValuePublicationWork {
    let source = (0..count)
        .map(|index| format!("pub let shared: Int = {index}\n"))
        .collect::<String>();
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    let identities = vec![(None, None, "shared".to_string()); count];
    let (matched_roots, _, work) =
        lowered.module_value_publication_diagnostic_indices_for_identities(&identities);
    assert_eq!(matched_roots, count);
    work
}

#[test]
fn duplicate_identity_root_expansion_is_linear() {
    let original = duplicate_identity_publication_work(16);
    let adjacent = duplicate_identity_publication_work(32);

    assert_eq!(original.exported_root_lookups, 16);
    assert_eq!(adjacent.exported_root_lookups, 32);
    assert_eq!(original.identity_root_expansions, 16);
    assert_eq!(adjacent.identity_root_expansions, 32);
    assert_eq!(original.traversal_values, 16);
    assert_eq!(adjacent.traversal_values, 32);
}
