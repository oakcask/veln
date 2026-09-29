use super::*;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn defer_captures_only_bindings_available_at_registration() {
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
    assert_eq!(diagnostic.related.len(), 1);
    assert!(diagnostic.related[0].to_json().contains("repair_hint"));
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
