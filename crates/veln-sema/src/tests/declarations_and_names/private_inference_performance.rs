use super::*;

#[test]
fn private_function_may_omit_boundary_annotations_when_inference_is_complete() {
    let source = SourceFile::new("main.veln", "fn answer()\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn fully_annotated_private_modules_do_not_scan_private_inference_bodies() {
    private_inference_counters::reset();
    let module = merged_modules(
        (0..8)
            .map(|module_index| {
                let mut source = format!("mod annotated_{module_index}\n");
                for function_index in 0..6 {
                    source.push_str(&format!(
                        "fn helper_{function_index}(value: Int) -> Int\n  value\nend\n"
                    ));
                }
                SourceFile::new(format!("annotated_{module_index}.veln"), source)
            })
            .collect(),
    );

    let diagnostics = analyze_surface_module(&module);
    let counters = private_inference_counters::snapshot();

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(counters.body_return_scans, 0, "{counters:#?}");
    assert_eq!(counters.call_site_discovery_scans, 0, "{counters:#?}");
    assert_eq!(counters.call_site_scans, 0, "{counters:#?}");
    assert_eq!(
        counters.private_reference_candidate_scans, 0,
        "{counters:#?}"
    );
    assert_eq!(counters.private_reference_index_scans, 0, "{counters:#?}");
    assert_eq!(
        counters.prelude_callback_discovery_scans, 0,
        "{counters:#?}"
    );
    assert_eq!(counters.prelude_callback_scans, 0, "{counters:#?}");
}

#[test]
fn omitted_private_signature_chain_skips_unrelated_annotated_modules() {
    private_inference_counters::reset();
    let mut sources = vec![SourceFile::new(
        "target.veln",
        concat!(
            "mod target\n",
            "fn identity(value)\n",
            "  value\n",
            "end\n",
            "\n",
            "fn pass(value)\n",
            "  identity(value)\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  pass(1)\n",
            "end\n",
        ),
    )];
    for module_index in 0..10 {
        sources.push(SourceFile::new(
            format!("unrelated_{module_index}.veln"),
            format!(
                "mod unrelated_{module_index}\n\
                 fn helper(value: Int) -> Int\n  value\nend\n"
            ),
        ));
    }
    let module = merged_modules(sources);

    let diagnostics = analyze_surface_module(&module);
    let counters = private_inference_counters::snapshot();

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(counters.body_return_scans > 0, "{counters:#?}");
    assert_eq!(counters.call_site_discovery_scans, 3, "{counters:#?}");
    assert!(counters.call_site_scans > 0, "{counters:#?}");
    assert_eq!(
        counters.private_reference_candidate_scans, 1,
        "{counters:#?}"
    );
    assert_eq!(counters.private_reference_index_scans, 3, "{counters:#?}");
    assert_eq!(
        counters.prelude_callback_discovery_scans, 0,
        "{counters:#?}"
    );
    assert_eq!(counters.prelude_callback_scans, 0, "{counters:#?}");
    assert!(
        counters.call_site_scans < 10,
        "call-site inference should not scan unrelated modules: {counters:#?}"
    );

    let environment = TypeEnvironment::from_module(&module);
    for name in ["identity", "pass"] {
        let function = environment
            .function(name)
            .unwrap_or_else(|| panic!("{name} should be present"));
        assert_eq!(
            function.params[0],
            crate::semantic_model::Type::int(),
            "{name}"
        );
        assert_eq!(
            function.return_type,
            crate::semantic_model::Type::int(),
            "{name}"
        );
    }
}

#[test]
fn omitted_private_reference_index_finds_calls_nested_in_collections() {
    private_inference_counters::reset();
    let source = SourceFile::new(
        "target.veln",
        concat!(
            "mod target\n",
            "fn identity(value)\n",
            "  value\n",
            "end\n",
            "\n",
            "fn main() -> {items: Vec<Int>}\n",
            "  {items: [identity(1)]}\n",
            "end\n",
        ),
    );
    let module = merged_modules(vec![source]);

    let diagnostics = analyze_surface_module(&module);
    let counters = private_inference_counters::snapshot();

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(
        counters.private_reference_candidate_scans, 1,
        "{counters:#?}"
    );
    assert_eq!(counters.private_reference_index_scans, 2, "{counters:#?}");
}

#[test]
fn omitted_private_signature_chain_skips_unrelated_annotated_functions_in_same_module() {
    private_inference_counters::reset();
    let mut source = String::from(
        "mod target\n\
         fn identity(value)\n  value\nend\n\
         \n\
         fn pass(value)\n  identity(value)\nend\n\
         \n\
         fn main() -> Int\n  pass(1)\nend\n",
    );
    for function_index in 0..12 {
        source.push_str(&format!(
            "\nfn annotated_{function_index}(value: Int) -> Int\n  value\nend\n"
        ));
    }
    let module = merged_modules(vec![SourceFile::new("target.veln", source)]);

    let diagnostics = analyze_surface_module(&module);
    let counters = private_inference_counters::snapshot();

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(counters.body_return_scans, 2, "{counters:#?}");
    assert_eq!(counters.call_site_discovery_scans, 15, "{counters:#?}");
    assert_eq!(counters.call_site_scans, 6, "{counters:#?}");
    assert_eq!(
        counters.private_reference_candidate_scans, 13,
        "{counters:#?}"
    );
    assert_eq!(counters.private_reference_index_scans, 3, "{counters:#?}");
    assert_eq!(
        counters.prelude_callback_discovery_scans, 0,
        "{counters:#?}"
    );
    assert_eq!(counters.prelude_callback_scans, 0, "{counters:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for name in ["identity", "pass"] {
        let function = environment
            .function(name)
            .unwrap_or_else(|| panic!("{name} should be present"));
        assert_eq!(
            function.params[0],
            crate::semantic_model::Type::int(),
            "{name}"
        );
        assert_eq!(
            function.return_type,
            crate::semantic_model::Type::int(),
            "{name}"
        );
    }
}

#[test]
fn omitted_private_signature_index_ignores_local_candidate_name_shadows() {
    private_inference_counters::reset();
    let source = SourceFile::new(
        "target.veln",
        concat!(
            "mod target\n",
            "fn identity(value)\n",
            "  value\n",
            "end\n",
            "\n",
            "fn pass(value)\n",
            "  identity(value)\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  pass(1)\n",
            "end\n",
            "\n",
            "fn parameter_shadow(identity: Int) -> Int\n",
            "  identity\n",
            "end\n",
            "\n",
            "fn local_shadow(value: Int) -> Int\n",
            "  let identity = value\n",
            "  identity\n",
            "end\n",
            "\n",
            "fn match_shadow(input: Option<Int>) -> Int\n",
            "  match input\n",
            "    Some(identity) => identity\n",
            "    None => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let module = merged_modules(vec![source]);

    let diagnostics = analyze_surface_module(&module);
    let counters = private_inference_counters::snapshot();

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(counters.body_return_scans, 2, "{counters:#?}");
    assert_eq!(counters.call_site_discovery_scans, 6, "{counters:#?}");
    assert_eq!(counters.call_site_scans, 6, "{counters:#?}");
    assert_eq!(
        counters.private_reference_candidate_scans, 4,
        "{counters:#?}"
    );
    assert_eq!(counters.private_reference_index_scans, 3, "{counters:#?}");
    assert_eq!(
        counters.prelude_callback_discovery_scans, 0,
        "{counters:#?}"
    );
    assert_eq!(counters.prelude_callback_scans, 0, "{counters:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for name in ["identity", "pass"] {
        let function = environment
            .function(name)
            .unwrap_or_else(|| panic!("{name} should be present"));
        assert_eq!(
            function.params[0],
            crate::semantic_model::Type::int(),
            "{name}"
        );
        assert_eq!(
            function.return_type,
            crate::semantic_model::Type::int(),
            "{name}"
        );
    }
}

#[test]
fn prelude_callback_return_inference_skips_unrelated_annotated_helpers() {
    private_inference_counters::reset();
    let mut source = String::from(
        "mod target\n\
         fn nested(value)\n  Some([])\nend\n\
         \n\
         fn fixed(value)\n  \"ok\"\nend\n\
         \n\
         fn main() -> {nested: Vec<Option<Vec<String>>>, fixed: Vec<String>}\n\
           {\n\
             nested: vec_map([1], nested),\n\
             fixed: vec_map([1], fixed)\n\
           }\n\
         end\n",
    );
    for function_index in 0..12 {
        source.push_str(&format!(
            "\nfn annotated_{function_index}(value: Int) -> Int\n  value\nend\n"
        ));
    }
    let module = merged_modules(vec![SourceFile::new("target.veln", source)]);

    let diagnostics = analyze_surface_module(&module);
    let counters = private_inference_counters::snapshot();

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(counters.body_return_scans, 3, "{counters:#?}");
    assert_eq!(counters.call_site_discovery_scans, 15, "{counters:#?}");
    assert_eq!(counters.call_site_scans, 3, "{counters:#?}");
    assert_eq!(
        counters.private_reference_candidate_scans, 27,
        "{counters:#?}"
    );
    assert_eq!(counters.private_reference_index_scans, 5, "{counters:#?}");
    assert_eq!(
        counters.prelude_callback_discovery_scans, 1,
        "{counters:#?}"
    );
    assert_eq!(counters.prelude_callback_scans, 1, "{counters:#?}");

    let environment = TypeEnvironment::from_module(&module);
    let nested = environment
        .function("nested")
        .expect("nested callback should be present");
    assert_eq!(nested.params[0], crate::semantic_model::Type::int());
    assert_eq!(
        nested.return_type,
        crate::semantic_model::Type::named(
            "Option",
            vec![crate::semantic_model::Type::named(
                "Vec",
                vec![crate::semantic_model::Type::string()]
            )]
        )
    );
    let fixed = environment
        .function("fixed")
        .expect("fixed callback should be present");
    assert_eq!(fixed.params[0], crate::semantic_model::Type::int());
    assert_eq!(fixed.return_type, crate::semantic_model::Type::string());
}

#[test]
fn prelude_callback_return_inference_has_zero_scan_when_helper_return_is_fixed() {
    private_inference_counters::reset();
    let mut source = String::from(
        "mod target\n\
         fn fixed(value)\n  \"ok\"\nend\n\
         \n\
         fn main() -> Vec<String>\n\
           vec_map([1], fixed)\n\
         end\n",
    );
    for function_index in 0..12 {
        source.push_str(&format!(
            "\nfn annotated_{function_index}(value: Int) -> Int\n  value\nend\n"
        ));
    }
    let module = merged_modules(vec![SourceFile::new("target.veln", source)]);

    let diagnostics = analyze_surface_module(&module);
    let counters = private_inference_counters::snapshot();

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(counters.body_return_scans, 1, "{counters:#?}");
    assert_eq!(counters.call_site_discovery_scans, 14, "{counters:#?}");
    assert_eq!(counters.call_site_scans, 2, "{counters:#?}");
    assert_eq!(
        counters.private_reference_candidate_scans, 13,
        "{counters:#?}"
    );
    assert_eq!(counters.private_reference_index_scans, 2, "{counters:#?}");
    assert_eq!(
        counters.prelude_callback_discovery_scans, 0,
        "{counters:#?}"
    );
    assert_eq!(counters.prelude_callback_scans, 0, "{counters:#?}");

    let environment = TypeEnvironment::from_module(&module);
    let fixed = environment
        .function("fixed")
        .expect("fixed callback should be present");
    assert_eq!(fixed.params[0], crate::semantic_model::Type::int());
    assert_eq!(fixed.return_type, crate::semantic_model::Type::string());
}
