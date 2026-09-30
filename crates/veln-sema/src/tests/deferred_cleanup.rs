use super::*;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn defer_sees_only_bindings_visible_at_the_statement() {
    let diagnostics = diagnostics_for(concat!(
        "fn main() -> ()\n",
        "  let earlier: Int = 1\n",
        "  defer\n",
        "    let copy: Int = earlier\n",
        "    ()\n",
        "  end\n",
        "  ()\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let diagnostics = diagnostics_for(concat!(
        "fn main() -> ()\n",
        "  defer\n",
        "    let copy: Int = later\n",
        "    ()\n",
        "  end\n",
        "  let later: Int = 1\n",
        "  ()\n",
        "end\n",
    ));

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message.contains("later")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn begin_returns_its_tail_value_without_leaking_local_bindings() {
    let diagnostics = diagnostics_for(concat!(
        "fn main() -> Int\n",
        "  let value: Int = begin\n",
        "    let local: Int = 41\n",
        "    local + 1\n",
        "  end\n",
        "  value\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let diagnostics = diagnostics_for(concat!(
        "fn main() -> Int\n",
        "  let value: Int = begin\n",
        "    let local: Int = 41\n",
        "    local + 1\n",
        "  end\n",
        "  local\n",
        "end\n",
    ));

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message.contains("local")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn begin_without_tail_expression_has_unit_type() {
    let diagnostics = diagnostics_for(concat!(
        "fn main() -> ()\n",
        "  let value: () = begin\n",
        "    let local: Int = 41\n",
        "  end\n",
        "  value\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn defer_rejects_non_unit_result_with_repair_note() {
    let diagnostics = diagnostics_for(concat!(
        "fn main() -> ()\n",
        "  defer\n",
        "    1\n",
        "  end\n",
        "  ()\n",
        "end\n",
    ));

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "defer.non_unit")
        .expect("non-unit deferred block should be rejected");
    assert_eq!(
        diagnostic.message,
        "deferred block must have type `()`, but found `Int`"
    );
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    let span = diagnostic.span.as_ref().expect("non-unit result span");
    assert_eq!((span.start.line, span.start.column), (3, 5));
    assert_eq!(diagnostic.related.len(), 1);
    assert!(diagnostic.related[0].to_json().contains("repair_hint"));
}

#[test]
fn defer_rejects_result_propagation_with_repair_note() {
    let diagnostics = diagnostics_for(concat!(
        "fn parse() -> Result<(), String>\n",
        "  Ok(())\n",
        "end\n",
        "fn main() -> Result<(), String>\n",
        "  defer\n",
        "    parse()?\n",
        "  end\n",
        "  Ok(())\n",
        "end\n",
    ));

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "defer.propagation")
        .expect("result propagation in deferred block should be rejected");
    assert_eq!(diagnostic.message, "deferred block cannot use `?`");
    let span = diagnostic.span.as_ref().expect("question-mark span");
    assert_eq!(
        (
            span.start.line,
            span.start.column,
            span.end.line,
            span.end.column
        ),
        (6, 12, 6, 13)
    );
    assert_eq!(diagnostic.related.len(), 1);
    assert!(diagnostic.related[0].to_json().contains("repair_hint"));
}

#[test]
fn defer_rejects_nested_registration_with_repair_note() {
    let diagnostics = diagnostics_for(concat!(
        "fn main() -> ()\n",
        "  defer\n",
        "    defer\n",
        "      ()\n",
        "    end\n",
        "    ()\n",
        "  end\n",
        "  ()\n",
        "end\n",
    ));

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "defer.nested")
        .expect("nested deferred block should be rejected");
    assert_eq!(
        diagnostic.message,
        "deferred block cannot register another deferred block"
    );
    let span = diagnostic.span.as_ref().expect("nested defer keyword span");
    assert_eq!(
        (
            span.start.line,
            span.start.column,
            span.end.line,
            span.end.column
        ),
        (3, 5, 3, 10)
    );
    assert_eq!(diagnostic.related.len(), 1);
    let related = diagnostic.related[0].to_json();
    assert!(related.contains("repair_hint"));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":1"));
}

#[test]
fn cleanup_region_effects_contribute_to_the_enclosing_function() {
    for body in [
        concat!(
            "  defer\n",
            "    perform Log::write()\n",
            "  end\n",
            "  ()\n"
        ),
        concat!(
            "  let value: Int = begin\n",
            "    perform Log::write()\n",
            "    1\n",
            "  end\n",
            "  ()\n",
        ),
    ] {
        let diagnostics = diagnostics_for(&format!(
            "effect Log\n  write() -> ()\nend\npub fn main() -> ()\n{body}end\n"
        ));

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "effect.missing_public"),
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn handler_begin_cleanup_region_collects_direct_and_dependent_effects() {
    let diagnostics = diagnostics_for(concat!(
        "effect Ask\n",
        "  value() -> ()\n",
        "end\n",
        "effect Log\n",
        "  write() -> ()\n",
        "end\n",
        "effect Audit\n",
        "  record() -> ()\n",
        "end\n",
        "fn audit() -> ()\n",
        "  perform Audit::record()\n",
        "end\n",
        "pub handler ask() handles Ask\n",
        "  value() => begin\n",
        "    perform Log::write()\n",
        "    audit()\n",
        "  end\n",
        "end\n",
    ));

    let missing_effects = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "handler.missing_public_effect")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert!(
        missing_effects
            .iter()
            .any(|message| message.contains("`Log`")),
        "{diagnostics:#?}"
    );
    assert!(
        missing_effects
            .iter()
            .any(|message| message.contains("`Audit`")),
        "{diagnostics:#?}"
    );
}

#[test]
fn schema_annotation_boundaries_cover_nested_cleanup_regions_and_handler_clauses() {
    let diagnostics = diagnostics_for(concat!(
        "schema Packet\n",
        "  value: Int\n",
        "end\n",
        "effect Ask\n",
        "  value() -> ()\n",
        "end\n",
        "fn main() -> ()\n",
        "  let direct: Packet = ()\n",
        "  let begun: () = begin\n",
        "    let in_begin: Packet = ()\n",
        "    ()\n",
        "  end\n",
        "  defer\n",
        "    let in_defer: Packet = ()\n",
        "    let nested: () = begin\n",
        "      let deeply_nested: Packet = ()\n",
        "      ()\n",
        "    end\n",
        "    ()\n",
        "  end\n",
        "  ()\n",
        "end\n",
        "handler ask() handles Ask\n",
        "  value() => begin\n",
        "    let in_handler: Packet = ()\n",
        "    ()\n",
        "  end\n",
        "end\n",
    ));

    let boundary_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.schema_reference")
        .collect::<Vec<_>>();
    assert_eq!(boundary_diagnostics.len(), 5, "{diagnostics:#?}");
    assert!(boundary_diagnostics.iter().all(|diagnostic| {
        let details = diagnostic.details.to_json();
        details.contains("\"schema\":\"Packet\"")
            && details.contains("\"use_kind\":\"local_annotation\"")
    }));
}

#[test]
fn schema_primitive_annotation_boundaries_match_across_cleanup_regions() {
    let diagnostics = diagnostics_for(concat!(
        "fn main() -> ()\n",
        "  let direct_exact: UInt16be = ()\n",
        "  let direct_lower: uint24be = ()\n",
        "  let begun: () = begin\n",
        "    let begin_exact: UInt16be = ()\n",
        "    let begin_lower: uint24be = ()\n",
        "    ()\n",
        "  end\n",
        "  defer\n",
        "    let defer_exact: UInt16be = ()\n",
        "    let defer_lower: uint24be = ()\n",
        "    ()\n",
        "  end\n",
        "  ()\n",
        "end\n",
    ));

    let exact_boundaries = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.exact_width_primitive")
        .collect::<Vec<_>>();
    assert_eq!(exact_boundaries.len(), 3, "{diagnostics:#?}");
    assert!(exact_boundaries.iter().all(|diagnostic| {
        diagnostic
            .details
            .to_json()
            .contains("\"reason\":\"local_annotation\"")
    }));

    let lowercase_boundaries = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.lowercase_primitive")
        .collect::<Vec<_>>();
    assert_eq!(lowercase_boundaries.len(), 3, "{diagnostics:#?}");
    assert!(lowercase_boundaries.iter().all(|diagnostic| {
        diagnostic
            .details
            .to_json()
            .contains("\"reason\":\"local_annotation\"")
    }));
}

#[test]
fn cleanup_regions_block_executable_lowering_until_runtime_support_exists() {
    for body in [
        concat!("  defer\n", "    ()\n", "  end\n", "  ()\n"),
        concat!("  begin\n", "    ()\n", "  end\n"),
    ] {
        let source = SourceFile::new("main.veln", format!("pub fn main() -> ()\n{body}end\n"));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);

        let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
        assert!(matches!(
            lowered.core.expect("checked core should be available").readiness,
            CoreReadiness::Blocked(ref blockers)
                if blockers.iter().any(|blocker| matches!(
                    blocker,
                    CoreBlocker::UnsupportedExpression { reason, .. }
                        if reason == "deferred_cleanup_runtime"
                ))
        ));
        assert!(lowered.ir.is_none());
    }
}

#[test]
fn nested_cleanup_binding_environment_retention_grows_linearly() {
    fn source_with_depth(depth: usize) -> String {
        let mut source = String::from("fn identity(value)\n");
        for level in 0..depth {
            source.push_str(&"  ".repeat(level + 1));
            source.push_str("begin\n");
            source.push_str(&"  ".repeat(level + 2));
            source.push_str(&format!("let value_{level}: Int = value\n"));
        }
        source.push_str(&"  ".repeat(depth + 1));
        source.push_str("value\n");
        for level in (0..depth).rev() {
            source.push_str(&"  ".repeat(level + 1));
            source.push_str("end\n");
        }
        source.push_str("end\n\nfn main() -> Int\n  identity(1)\nend\n");
        source
    }

    fn binding_work(depth: usize) -> (usize, usize) {
        crate::semantic_model::reset_binding_clone_count();
        let diagnostics = diagnostics_for(&source_with_depth(depth));
        assert!(diagnostics.is_empty(), "depth {depth}: {diagnostics:#?}");
        (
            crate::semantic_model::binding_clone_count(),
            crate::semantic_model::max_scoped_binding_count(),
        )
    }

    let (shallow_clones, shallow) = binding_work(12);
    let (medium_clones, medium) = binding_work(24);
    let (deep_clones, deep) = binding_work(48);
    assert_eq!(
        (shallow_clones, medium_clones, deep_clones),
        (0, 0, 0),
        "nested cleanup traversal must not clone binding environments"
    );
    assert!(
        shallow < medium && medium < deep,
        "generated nesting must exercise scoped binding retention: \
         shallow={shallow}, medium={medium}, deep={deep}"
    );
    let first_growth = medium - shallow;
    let second_growth = deep - medium;

    assert!(
        second_growth <= first_growth * 3,
        "retained binding slots must remain linear across nested cleanup regions: \
         shallow={shallow}, medium={medium}, deep={deep}"
    );
}
