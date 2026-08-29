use super::*;

#[test]
fn reachable_resolution_skips_unrelated_annotated_functions() {
    fn resolution_scans(unrelated_count: usize) -> (usize, usize, usize, usize) {
        let mut source =
            String::from("pub fn main() -> Int\n  helper()\nend\n\nfn helper() -> Int\n  1\nend\n");
        for index in 0..unrelated_count {
            source.push_str(&format!(
                "\nfn unrelated_{index}(value: Int) -> Int\n  value\nend\n"
            ));
        }
        let module = lower(&source);
        reachability_counters::reset();
        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        assert_eq!(reachable.functions.len(), 2);
        reachability_counters::snapshot()
    }

    let base = resolution_scans(0);
    let expanded = resolution_scans(128);

    assert_eq!(
        expanded, base,
        "unrelated annotated functions must not add repeated resolution scans"
    );
}

#[test]
fn reachability_cache_keeps_entry_results_independent() {
    let module = lower(concat!(
        "mod app\n",
        "pub fn main() -> Int\n",
        "  main_helper()\n",
        "end\n",
        "fn main_helper() -> Int\n",
        "  1\n",
        "end\n",
        "pub fn alternate() -> Int\n",
        "  alternate_helper()\n",
        "end\n",
        "fn alternate_helper() -> Int\n",
        "  2\n",
        "end\n",
    ));
    let cache = ReachabilityCache::default();

    let main = reachable_entry_module_with_cache(&module, "main", FunctionKind::Function, &cache);
    let alternate =
        reachable_entry_module_with_cache(&module, "alternate", FunctionKind::Function, &cache);

    assert_eq!(
        reachable_function_names(&main),
        [("app", "main"), ("app", "main_helper")]
    );
    assert_eq!(
        reachable_function_names(&alternate),
        [("app", "alternate"), ("app", "alternate_helper")]
    );
}

#[test]
fn reachable_entry_keeps_invalid_import_segments_with_alias_proof_only() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "use HTTP\n",
                    "\n",
                    "fn main() -> Int\n",
                    "  HTTP::entry()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "HTTP.veln",
                concat!("pub fn entry() -> Bool\n", "  1\n", "end\n"),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "module.unresolved_import"
                && diagnostic.message == "local import `HTTP` has no matching selected source file"
        }),
        "{diagnostics:#?}"
    );

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(reachable_function_names(&reachable), vec![("app", "main")]);
    assert!(
        reachable.invalid_names.iter().any(|invalid| {
            invalid.name == "HTTP"
                && invalid.class == veln_ast::NameClass::Module
                && invalid.occurrence == veln_ast::NameOccurrence::PathSegment
        }),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn reachable_entry_skips_invalid_import_in_unselected_module() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new("main.veln", "fn main() -> Int\n  1\nend\n"),
            SourceFile::new(
                "unused.veln",
                concat!(
                    "use HTTP\n",
                    "\n",
                    "fn dead() -> Int\n",
                    "  HTTP::entry()\n",
                    "end\n",
                ),
            ),
            SourceFile::new("HTTP.veln", "pub fn entry() -> Int\n  1\nend\n"),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "module.unresolved_import"
                && diagnostic.message == "local import `HTTP` has no matching selected source file"
        }),
        "{diagnostics:#?}"
    );

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(reachable_function_names(&reachable), vec![("main", "main")]);
    assert!(
        reachable.invalid_names.iter().all(|invalid| {
            !(invalid.name == "HTTP"
                && invalid.class == veln_ast::NameClass::Module
                && invalid.occurrence == veln_ast::NameOccurrence::PathSegment)
        }),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn reachable_entry_skips_unused_invalid_import_in_entry_module() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new("app.veln", "use HTTP\n\nfn main() -> Int\n  1\nend\n"),
            SourceFile::new("HTTP.veln", "pub fn entry() -> Int\n  1\nend\n"),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "module.unresolved_import"
                && diagnostic.message == "local import `HTTP` has no matching selected source file"
        }),
        "{diagnostics:#?}"
    );

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(reachable_function_names(&reachable), vec![("app", "main")]);
    assert!(
        reachable.invalid_names.iter().all(|invalid| {
            !(invalid.name == "HTTP"
                && invalid.class == veln_ast::NameClass::Module
                && invalid.occurrence == veln_ast::NameOccurrence::PathSegment)
        }),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn reachable_entry_keeps_valid_import_alias_target_reachable() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "use helper\n",
                    "\n",
                    "fn main() -> Int\n",
                    "  helper::entry()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "helper.veln",
                concat!("pub fn entry() -> Int\n", "  1\n", "end\n"),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(
        reachable_function_names(&reachable),
        vec![("app", "main"), ("helper", "entry")]
    );
}

#[test]
fn reachable_recovery_selection_skips_unrelated_invalid_declarations() {
    fn recovery_candidate_scans(unrelated_count: usize) -> usize {
        let mut source = String::from(concat!(
            "pub fn main() -> Int\n",
            "  Value\n",
            "end\n",
            "\n",
            "type item\n",
            "  Value\n",
            "end\n",
        ));
        for index in 0..unrelated_count {
            source.push_str(&format!("\ntype unrelated_{index}\n  Other_{index}\nend\n"));
        }
        let module = lower(&source);
        reachability_counters::reset();
        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        assert_eq!(
            reachable.invalid_names.len(),
            1,
            "{:#?}",
            reachable.invalid_names
        );
        reachability_counters::snapshot().3
    }

    assert_eq!(
        recovery_candidate_scans(128),
        recovery_candidate_scans(0),
        "unrelated invalid declarations must not add repeated recovery selector scans"
    );
}

#[test]
fn reachable_materialization_skips_unrelated_annotated_function_bodies() {
    fn materialized_body_count(unrelated_count: usize) -> usize {
        let mut source =
            String::from("pub fn main() -> Int\n  helper()\nend\n\nfn helper() -> Int\n  1\nend\n");
        for index in 0..unrelated_count {
            source.push_str(&format!(
                "\nfn unrelated_{index}(value: Int) -> Int\n  value\nend\n"
            ));
        }
        let module = lower(&source);
        reachability_counters::reset();
        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        assert_eq!(reachable.functions.len(), 2);
        reachability_counters::snapshot().2
    }

    assert_eq!(
        materialized_body_count(128),
        materialized_body_count(0),
        "unreachable annotated functions must not be materialized for lowering"
    );
}

#[test]
fn separated_reachable_materialization_skips_unrelated_annotated_function_bodies() {
    fn materialized_body_count(unrelated_count: usize) -> usize {
        let standard = lower(concat!(
            "mod std::prelude\n",
            "pub fn standard_value() -> Int\n",
            "  1\n",
            "end\n",
        ));
        let mut source = String::from(concat!(
            "mod app\n",
            "use std::prelude\n",
            "\n",
            "pub fn main() -> Int\n",
            "  helper() + standard_value()\n",
            "end\n",
            "\n",
            "fn helper() -> Int\n",
            "  1\n",
            "end\n",
        ));
        for index in 0..unrelated_count {
            source.push_str(&format!(
                "\nfn unrelated_{index}(value: Int) -> Int\n  value\nend\n"
            ));
        }
        let application = lower(&source);
        reachability_counters::reset();
        let reachable = reachable_entry_module_with_standard_cache(
            &standard,
            &application,
            "main",
            FunctionKind::Function,
            &ReachabilityCache::default(),
        );
        assert_eq!(reachable.functions.len(), 3);
        reachability_counters::snapshot().2
    }

    assert_eq!(
        materialized_body_count(128),
        materialized_body_count(0),
        "separated reachable inputs must not materialize unreachable annotated functions"
    );
}

#[test]
fn separated_reachable_inputs_match_combined_resolution_results() {
    let mut standard = lower(concat!(
        "mod std::prelude\n",
        "pub type StandardValue\n",
        "  Present\n",
        "end\n",
        "pub schema Packet\n",
        "  value: Int\n",
        "end\n",
        "pub fn standard_value() -> Int\n",
        "  1\n",
        "end\n",
    ));
    add_payload_codec_for_test(&mut standard);
    let mut application = lower(concat!(
        "mod app\n",
        "use std::prelude\n",
        "\n",
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "\n",
        "fn answer() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "\n",
        "handler ask(seed: Int) handles Ask\n",
        "  value() => seed\n",
        "end\n",
        "\n",
        "type ApplicationValue\n",
        "  Present\n",
        "end\n",
        "\n",
        "schema Packet\n",
        "  value: Int\n",
        "end\n",
        "\n",
        "pub fn exposed = answer\n",
        "\n",
        "pub fn main() -> Int\n",
        "  let handled = handle exposed() with ask(2)\n",
        "  handled + standard_value()\n",
        "end\n",
    ));
    add_payload_codec_for_test(&mut application);
    let mut combined = standard.clone();
    combined.uses.extend(application.uses.clone());
    combined.aliases.extend(application.aliases.clone());
    combined.effects.extend(application.effects.clone());
    combined.handlers.extend(application.handlers.clone());
    combined.types.extend(application.types.clone());
    combined.schemas.extend(application.schemas.clone());
    combined.codecs.extend(application.codecs.clone());
    combined.functions.extend(application.functions.clone());
    combined
        .invalid_names
        .extend(application.invalid_names.clone());
    combined.module = application.module.clone();

    let combined_reachable = reachable_entry_module(&combined, "main", FunctionKind::Function);
    let separated_reachable = reachable_entry_module_with_standard_cache(
        &standard,
        &application,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );

    let combined_functions = reachable_function_names(&combined_reachable);
    let separated_functions = reachable_function_names(&separated_reachable);
    assert_eq!(separated_functions, combined_functions);
    assert_eq!(
        separated_functions,
        vec![
            ("app", "answer"),
            ("app", "main"),
            ("std::prelude", "standard_value"),
        ]
    );
    assert_eq!(
        separated_reachable
            .module
            .as_ref()
            .map(|module| module.name.as_str()),
        Some("app")
    );
    assert_eq!(
        separated_reachable.uses.len(),
        combined_reachable.uses.len()
    );
    assert_eq!(
        separated_reachable.aliases.len(),
        combined_reachable.aliases.len()
    );
    assert_eq!(
        separated_reachable.effects.len(),
        combined_reachable.effects.len()
    );
    assert_eq!(
        separated_reachable.handlers.len(),
        combined_reachable.handlers.len()
    );
    assert_eq!(
        separated_reachable.types.len(),
        combined_reachable.types.len()
    );
    assert_eq!(
        separated_reachable.schemas.len(),
        combined_reachable.schemas.len()
    );
    assert_eq!(
        separated_reachable.codecs.len(),
        combined_reachable.codecs.len()
    );
}

#[test]
fn separated_reachable_inputs_resolve_codec_with_targets() {
    let mut standard = lower(concat!(
        "mod std::prelude\n",
        "pub schema Packet\n",
        "  value: Int\n",
        "end\n",
        "\n",
        "fn decode_payload_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{value: Int}>\n",
        "  NeedMore(NeedEnd)\n",
        "end\n",
    ));
    add_payload_codec_for_test(&mut standard);
    let application = lower(concat!(
        "mod app\n",
        "use std::prelude\n",
        "\n",
        "pub fn main(source: ByteView, base: ByteOffset) -> DecodeStep<{value: Int}>\n",
        "  std::prelude::PayloadCodec(source, base)\n",
        "end\n",
    ));
    let mut combined = standard.clone();
    combined.uses.extend(application.uses.clone());
    combined.aliases.extend(application.aliases.clone());
    combined.effects.extend(application.effects.clone());
    combined.handlers.extend(application.handlers.clone());
    combined.types.extend(application.types.clone());
    combined.schemas.extend(application.schemas.clone());
    combined.codecs.extend(application.codecs.clone());
    combined.functions.extend(application.functions.clone());
    combined
        .invalid_names
        .extend(application.invalid_names.clone());
    combined.module = application.module.clone();

    let combined_reachable = reachable_entry_module(&combined, "main", FunctionKind::Function);
    let separated_reachable = reachable_entry_module_with_standard_cache(
        &standard,
        &application,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );

    let combined_functions = reachable_function_names(&combined_reachable);
    let separated_functions = reachable_function_names(&separated_reachable);
    assert_eq!(separated_functions, combined_functions);
    assert_eq!(
        separated_functions,
        vec![("app", "main"), ("std::prelude", "decode_payload_packet")]
    );
}

fn add_payload_codec_for_test(module: &mut SurfaceModule) {
    let schema = module
        .schemas
        .iter()
        .find(|schema| schema.name.as_deref() == Some("Packet"))
        .expect("test standard module should define Packet schema");
    module.codecs.push(CodecDecl {
        node_id: schema.node_id,
        module_name: Some("std::prelude".to_string()),
        visibility: Visibility::Public,
        name: Some("PayloadCodec".to_string()),
        schema: Some("Packet".to_string()),
        directions: vec![CodecDirection::Decode],
        implementations: vec![CodecImplementationClause {
            node_id: schema.node_id,
            direction: CodecDirection::Decode,
            kind: CodecImplementationKind::With {
                function: Some("decode_payload_packet".to_string()),
            },
            span: schema.span.clone(),
        }],
        span: schema.span.clone(),
    });
}
