use super::*;

fn lower_with_publication_work(
    source: String,
) -> (
    LoweredSurfaceModule,
    crate::pipeline::PublicationPropagationWork,
) {
    let source = SourceFile::new("api.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    reset_publication_propagation_work();
    let lowered = lower_checked_surface_module(&module);
    (lowered, publication_propagation_work())
}

fn adt_chain_source(length: usize, dependent_first: bool) -> String {
    let state = concat!(
        "mod api\n",
        "pub type State\n",
        "  HiddenOne\n",
        "  HiddenTwo\n",
        "end\n",
    );
    let root = concat!(
        "pub type Root\n",
        "  pub Rooted(one: State::HiddenOne, two: State::HiddenTwo)\n",
        "end\n",
    );
    let mut layers = Vec::new();
    for index in 0..length {
        let previous = if index == 0 {
            "Root".to_string()
        } else {
            format!("Layer{}", index - 1)
        };
        layers.push(format!(
            "pub type Layer{index}\n  pub Wrapped(previous: {previous}, shared: Root)\nend\n"
        ));
    }
    let cycle = concat!(
        "pub type CycleA\n",
        "  pub Next(value: CycleB)\n",
        "end\n",
        "pub type CycleB\n",
        "  pub Next(value: CycleA, cause: Root)\n",
        "end\n",
    );

    let mut source = state.to_string();
    if dependent_first {
        source.push_str(cycle);
        for layer in layers.iter().rev() {
            source.push_str(layer);
        }
        source.push_str(root);
    } else {
        source.push_str(root);
        for layer in &layers {
            source.push_str(layer);
        }
        source.push_str(cycle);
    }
    source
}

fn function_alias_chain_source(length: usize, dependent_first: bool) -> String {
    let prefix = concat!(
        "mod api\n",
        "pub type State\n",
        "  HiddenOne\n",
        "  HiddenTwo\n",
        "end\n",
    );
    let target = concat!(
        "pub fn exposed(one: State::HiddenOne, two: State::HiddenTwo) -> ()\n",
        "  ()\n",
        "end\n",
    );
    let mut aliases = Vec::new();
    for index in 0..length {
        let target_name = if index + 1 == length {
            "exposed".to_string()
        } else {
            format!("alias{}", index + 1)
        };
        aliases.push(format!("pub fn alias{index} = {target_name}\n"));
    }

    let mut source = prefix.to_string();
    if dependent_first {
        for alias in &aliases {
            source.push_str(alias);
        }
        source.push_str(target);
    } else {
        source.push_str(target);
        for alias in aliases.iter().rev() {
            source.push_str(alias);
        }
    }
    source
}

fn qualified_function_alias_source(count: usize) -> String {
    let mut source = String::from("mod api\n");
    for index in 0..count {
        source.push_str(&format!("use dependency{index} as alias{index}\n"));
    }
    for index in 0..count {
        source.push_str(&format!("pub fn exported{index} = alias{index}::target\n"));
    }
    source
}

fn qualified_function_alias_import_work(
    count: usize,
) -> crate::name_recovery::NormalImportSelectionWork {
    let source = SourceFile::new("api.veln", qualified_function_alias_source(count));
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    crate::name_recovery::reset_normal_import_selection_work();
    let _lowered = lower_checked_surface_module(&module);
    crate::name_recovery::normal_import_selection_work()
}

fn cause_columns(declaration: &UnpublishedDeclaration) -> Vec<(usize, usize)> {
    declaration
        .causal_diagnostics
        .iter()
        .map(|cause| (cause.span.start.column, cause.span.end.column))
        .collect()
}

#[test]
fn checked_lowering_propagates_adt_causes_through_shared_edges_and_cycles_in_both_orders() {
    for dependent_first in [false, true] {
        let (lowered, _) = lower_with_publication_work(adt_chain_source(6, dependent_first));
        for name in ["Layer5", "CycleA", "CycleB"] {
            let declaration = lowered
                .unpublished_declarations
                .iter()
                .find(|declaration| declaration.name == name)
                .unwrap_or_else(|| panic!("missing unpublished declaration {name}"));
            assert_eq!(declaration.causal_diagnostics.len(), 2, "{declaration:#?}");
            assert!(
                declaration
                    .causal_diagnostics
                    .iter()
                    .all(|cause| cause.diagnostic_id == "type.variant_refinement_private")
            );
            assert_eq!(cause_columns(declaration), [(26, 35), (49, 58)]);
        }
    }
}

#[test]
fn checked_lowering_propagates_forward_function_alias_causes_in_both_orders() {
    for dependent_first in [false, true] {
        let (lowered, _) =
            lower_with_publication_work(function_alias_chain_source(8, dependent_first));
        let alias = lowered
            .unpublished_declarations
            .iter()
            .find(|declaration| declaration.name == "alias0")
            .expect("forward alias must remain unpublished");
        assert_eq!(alias.causal_diagnostics.len(), 2, "{alias:#?}");
        assert!(
            alias
                .causal_diagnostics
                .iter()
                .all(|cause| cause.diagnostic_id == "type.variant_refinement_private")
        );
    }
}

#[test]
fn checked_lowering_publication_work_is_affine_for_both_chain_orders() {
    fn measurements(
        source: impl Fn(usize, bool) -> String,
        dependent_first: bool,
    ) -> Vec<crate::pipeline::PublicationPropagationWork> {
        [8, 16, 24]
            .into_iter()
            .map(|length| lower_with_publication_work(source(length, dependent_first)).1)
            .collect()
    }

    for source in [
        adt_chain_source as fn(usize, bool) -> String,
        function_alias_chain_source,
    ] {
        for dependent_first in [false, true] {
            let work = measurements(source, dependent_first);
            assert_eq!(
                work[1].reverse_edges - work[0].reverse_edges,
                work[2].reverse_edges - work[1].reverse_edges,
                "{work:#?}"
            );
            assert_eq!(
                work[1].admitted_cause_pairs - work[0].admitted_cause_pairs,
                work[2].admitted_cause_pairs - work[1].admitted_cause_pairs,
                "{work:#?}"
            );
            assert_eq!(
                work[1].propagation_attempts - work[0].propagation_attempts,
                work[2].propagation_attempts - work[1].propagation_attempts,
                "{work:#?}"
            );
        }
    }
}

#[test]
fn checked_lowering_qualified_function_alias_import_work_scales_linearly() {
    let small = qualified_function_alias_import_work(64);
    let large = qualified_function_alias_import_work(128);
    assert!(
        large.operations <= small.operations * 2 + 16,
        "doubling imports and qualified public aliases must keep aggregate selection work linear: small={small:?} large={large:?}"
    );
}
