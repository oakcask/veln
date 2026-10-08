use super::*;
use veln_diagnostics::JsonValue;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

fn detail<'a>(diagnostic: &'a Diagnostic, name: &str) -> &'a JsonValue {
    let JsonValue::Object(entries) = &diagnostic.details else {
        panic!("diagnostic details must be an object")
    };
    entries
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
        .unwrap_or_else(|| panic!("missing diagnostic detail `{name}`"))
}

fn spanned_text<'a>(source: &'a str, diagnostic: &Diagnostic) -> &'a str {
    let span = diagnostic.span.as_ref().expect("diagnostic span");
    &source[span.start.offset..span.end.offset]
}

const PRELUDE: &str = concat!(
    "type State\n",
    "  Ready\n",
    "  Closed\n",
    "  Failed\n",
    "end\n",
    "fn accept_ready(value: State::Ready) -> ()\n",
    "  ()\n",
    "end\n",
    "fn accept_closed(value: State::Closed) -> ()\n",
    "  ()\n",
    "end\n",
    "fn accept_failed(value: State::Failed) -> ()\n",
    "  ()\n",
    "end\n",
    "fn accept_residual(value: State::Closed | State::Failed) -> ()\n",
    "  ()\n",
    "end\n",
);

#[test]
fn direct_nested_and_root_alias_paths_refine_and_restore_by_arm() {
    let source = format!(
        "{PRELUDE}{}",
        concat!(
            "fn inspect(holder: {session: {state: State}}) -> ()\n",
            "  let alias = holder\n",
            "  match holder.session.state\n",
            "    Ready => begin\n",
            "      accept_ready(holder.session.state)\n",
            "      accept_ready(alias.session.state)\n",
            "    end\n",
            "    remaining => begin\n",
            "      let remaining_alias = remaining\n",
            "      accept_residual(holder.session.state)\n",
            "      accept_residual(alias.session.state)\n",
            "      accept_residual(remaining)\n",
            "      accept_residual(remaining_alias)\n",
            "      match remaining_alias\n",
            "        Closed => begin\n",
            "          accept_closed(holder.session.state)\n",
            "          accept_closed(alias.session.state)\n",
            "          accept_closed(remaining)\n",
            "          accept_closed(remaining_alias)\n",
            "        end\n",
            "        Failed => begin\n",
            "          accept_failed(holder.session.state)\n",
            "          accept_failed(alias.session.state)\n",
            "          accept_failed(remaining)\n",
            "          accept_failed(remaining_alias)\n",
            "        end\n",
            "      end\n",
            "      accept_residual(holder.session.state)\n",
            "      accept_residual(alias.session.state)\n",
            "      accept_residual(remaining)\n",
            "      accept_residual(remaining_alias)\n",
            "      match alias.session.state\n",
            "        Closed => begin\n",
            "          accept_closed(holder.session.state)\n",
            "          accept_closed(alias.session.state)\n",
            "          accept_closed(remaining)\n",
            "          accept_closed(remaining_alias)\n",
            "        end\n",
            "        Failed => begin\n",
            "          accept_failed(holder.session.state)\n",
            "          accept_failed(alias.session.state)\n",
            "          accept_failed(remaining)\n",
            "          accept_failed(remaining_alias)\n",
            "        end\n",
            "      end\n",
            "      accept_residual(holder.session.state)\n",
            "      accept_residual(alias.session.state)\n",
            "      accept_residual(remaining)\n",
            "      accept_residual(remaining_alias)\n",
            "    end\n",
            "  end\n",
            "  accept_ready(holder.session.state)\n",
            "  accept_ready(alias.session.state)\n",
            "end\n",
        )
    );
    let diagnostics = diagnostics_for(&source);

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 2, "{diagnostics:#?}");
    assert!(mismatches.iter().all(|diagnostic| {
        detail(diagnostic, "actual_type").as_text() == Some("State")
            && detail(diagnostic, "expected_type").as_text() == Some("State::Ready")
    }));
    assert_eq!(
        mismatches
            .iter()
            .map(|diagnostic| spanned_text(&source, diagnostic))
            .collect::<Vec<_>>(),
        ["holder.session.state", "alias.session.state"]
    );
}

#[test]
fn path_identity_separates_shadowed_roots_unrelated_fields_and_computed_bases() {
    let source = format!(
        "{PRELUDE}{}",
        concat!(
            "fn identity(holder: {state: State}) -> {state: State}\n",
            "  holder\n",
            "end\n",
            "fn identity_pair(holder: {left: State, right: State}) -> {left: State, right: State}\n",
            "  holder\n",
            "end\n",
            "fn boundaries(holder: {left: State, right: State}, other: {left: State, right: State}) -> ()\n",
            "  match holder.left\n",
            "    Ready => begin\n",
            "      accept_ready(holder.left)\n",
            "      accept_ready(holder.right)\n",
            "      let unrelated = other\n",
            "      accept_ready(unrelated.left)\n",
            "    end\n",
            "    Closed => ()\n",
            "    Failed => ()\n",
            "  end\n",
            "  match identity({state: holder.left}).state\n",
            "    Ready => accept_ready(identity({state: holder.left}).state)\n",
            "    Closed => ()\n",
            "    Failed => ()\n",
            "  end\n",
            "  match {state: holder.left}.state\n",
            "    Ready => accept_ready({state: holder.left}.state)\n",
            "    Closed => ()\n",
            "    Failed => ()\n",
            "  end\n",
            "  match (holder |> identity_pair()).left\n",
            "    Ready => accept_ready((holder |> identity_pair()).left)\n",
            "    Closed => ()\n",
            "    Failed => ()\n",
            "  end\n",
            "end\n",
        )
    );
    let diagnostics = diagnostics_for(&source);

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 5, "{diagnostics:#?}");
    assert!(mismatches.iter().all(|diagnostic| {
        detail(diagnostic, "actual_type").as_text() == Some("State")
            && detail(diagnostic, "expected_type").as_text() == Some("State::Ready")
    }));
    assert_eq!(
        mismatches
            .iter()
            .map(|diagnostic| spanned_text(&source, diagnostic))
            .collect::<Vec<_>>(),
        [
            "holder.right",
            "unrelated.left",
            "identity({state: holder.left}).state",
            "{state: holder.left}.state",
            "(holder |> identity_pair()).left",
        ]
    );
}

#[test]
fn same_spelled_roots_in_distinct_arm_scopes_keep_distinct_path_facts() {
    let source = format!(
        "{PRELUDE}{}",
        concat!(
            "fn inspect(select_first: Bool, first: {state: State}, second: {state: State}) -> ()\n",
            "  match select_first\n",
            "    true => begin\n",
            "      let holder = first\n",
            "      match holder.state\n",
            "        Ready => accept_ready(holder.state)\n",
            "        Closed => ()\n",
            "        Failed => ()\n",
            "      end\n",
            "    end\n",
            "    false => begin\n",
            "      let holder = second\n",
            "      accept_ready(holder.state)\n",
            "    end\n",
            "  end\n",
            "end\n",
        )
    );
    let diagnostics = diagnostics_for(&source);

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        detail(mismatches[0], "actual_type").as_text(),
        Some("State")
    );
    assert_eq!(
        spanned_text(&source, mismatches[0]),
        "holder.state",
        "the second arm's root must not inherit the first arm's path fact"
    );
}

#[test]
fn refined_field_domain_preserves_impossible_and_redundant_arm_classification() {
    let diagnostics = diagnostics_for(&format!(
        "{PRELUDE}{}",
        concat!(
            "fn classify(holder: {state: State::Ready | State::Closed}) -> ()\n",
            "  match holder.state\n",
            "    Failed => ()\n",
            "    Ready => accept_ready(holder.state)\n",
            "    Ready => ()\n",
            "    Closed => ()\n",
            "    _ => ()\n",
            "  end\n",
            "end\n",
        )
    ));

    let classified = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "type.match_impossible_variant" | "type.match_redundant_arm"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(classified.len(), 3, "{diagnostics:#?}");

    assert_eq!(classified[0].id, "type.match_impossible_variant");
    assert_eq!(
        detail(classified[0], "scrutinee_type").as_text(),
        Some("State::Ready | State::Closed")
    );
    assert_eq!(
        detail(classified[0], "arm_variant").as_text(),
        Some("Failed")
    );

    assert_eq!(classified[1].id, "type.match_redundant_arm");
    assert_eq!(
        detail(classified[1], "reason").as_text(),
        Some("duplicate_variant")
    );
    assert_eq!(classified[1].related.len(), 1);

    assert_eq!(classified[2].id, "type.match_redundant_arm");
    assert_eq!(
        detail(classified[2], "reason").as_text(),
        Some("complete_prior_coverage")
    );
    assert_eq!(classified[2].related.len(), 2);
}
