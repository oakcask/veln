use super::*;

#[test]
fn qualified_use_path_segment_matrix_reports_each_fixed_role() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use helper\n",
            "use foo::bar\n",
            "\n",
            "type State\n",
            "  Ready(Int)\n",
            "end\n",
            "\n",
            "fn main(flag: helper::Item) -> prelude::Option<Int>\n",
            "  let value: Helper::Item = helper::Item::ready(1)\n",
            "  let fallback: helper::item = helper::Item::Ready(1)\n",
            "  let built: prelude::option<Int> = prelude::Some(1)\n",
            "  let local = Helper::make()\n",
            "  helper::Make()\n",
            "  let nested_ok = foo::bar::double(3)\n",
            "  let nested_first = Foo::bar::double(3)\n",
            "  let nested_middle = foo::Bar::double(3)\n",
            "  let nested_leaf = foo::bar::Double(3)\n",
            "  let nested_value = foo::bar::Double\n",
            "end\n",
        ),
    );
    let nested = SourceFile::new(
        "foo/bar.veln",
        concat!(
            "pub fn double(value: Int) -> Int\n",
            "  value + value\n",
            "end\n"
        ),
    );
    let helper = SourceFile::new(
        "helper.veln",
        concat!(
            "pub type Item\n",
            "  pub Ready(Int)\n",
            "end\n",
            "\n",
            "pub fn make() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let module = merged_modules_with_names([
        ("main", source.clone()),
        ("helper", helper),
        ("foo::bar", nested),
    ]);
    let diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();

    let observed = diagnostics
        .iter()
        .map(|diagnostic| {
            let span = diagnostic.span.as_ref().expect("name diagnostic span");
            (
                &source.text()[span.start.offset..span.end.offset],
                class_detail(diagnostic),
                segment_index_detail(diagnostic),
                span.start.line,
                span.start.column,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        [
            ("Helper", "module", Some(0), 9, 14),
            ("ready", "constructor", Some(2), 9, 43),
            ("item", "type", Some(1), 10, 25),
            ("option", "type", Some(1), 11, 23),
            ("Helper", "module", Some(0), 12, 15),
            ("Make", "function", Some(1), 13, 11),
            ("Foo", "module", Some(0), 15, 22),
            ("Bar", "module", Some(1), 16, 28),
            ("Double", "function", Some(2), 17, 31),
            ("Double", "value_binding", Some(2), 18, 32),
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn lowering_result_exposes_classified_qualified_use_segments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Sample\n",
            "  Made(Int)\n",
            "end\n",
            "fn main() -> Sample\n",
            "  sample::Made(1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let lowered = lower_checked_surface_module(&module);

    let segment = lowered
        .qualified_path_segments
        .iter()
        .find(|segment| segment.name == "sample")
        .expect("lowered result exposes recovered type segment");
    assert_eq!(segment.role, veln_ast::NameClass::Type);
    assert_eq!(segment.occurrence, veln_ast::NameOccurrence::PathSegment);
    assert_eq!(
        segment.evidence,
        veln_ast::QualifiedPathSegmentEvidence::UniqueRecovery
    );
    assert_eq!(segment.segment_index, 0);
    assert_eq!((segment.span.start.line, segment.span.start.column), (5, 3));
}

#[test]
fn semantic_classifier_exposes_valid_qualified_use_segments() {
    let main = SourceFile::new(
        "main.veln",
        concat!(
            "use helper\n",
            "\n",
            "fn main(input: helper::Item) -> prelude::Option<Int>\n",
            "  let made: helper::Item = helper::Item::Ready(1)\n",
            "  let maybe = prelude::byte(1)\n",
            "  let number = helper::make()\n",
            "  match made\n",
            "    helper::Item::Ready(value) -> value\n",
            "  end\n",
            "end\n",
        ),
    );
    let helper = SourceFile::new(
        "helper.veln",
        concat!(
            "pub type Item\n",
            "  pub Ready(Int)\n",
            "end\n",
            "\n",
            "pub fn make() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let module = merged_modules_with_names([("main", main), ("helper", helper)]);

    let observed = classified_project_qualified_path_segments(&module)
        .into_iter()
        .filter(|segment| segment.span.file.as_str() == "main.veln")
        .map(|segment| {
            (
                segment.name,
                segment.role.as_str(),
                segment.evidence,
                segment.segment_index,
                segment.span.start.line,
                segment.span.start.column,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        [
            (
                "helper".to_string(),
                "module",
                veln_ast::QualifiedPathSegmentEvidence::Syntax,
                0,
                3,
                16,
            ),
            (
                "Item".to_string(),
                "type",
                veln_ast::QualifiedPathSegmentEvidence::Syntax,
                1,
                3,
                24,
            ),
            (
                "prelude".to_string(),
                "module",
                veln_ast::QualifiedPathSegmentEvidence::Syntax,
                0,
                3,
                33,
            ),
            (
                "Option".to_string(),
                "type",
                veln_ast::QualifiedPathSegmentEvidence::Syntax,
                1,
                3,
                42,
            ),
            (
                "helper".to_string(),
                "module",
                veln_ast::QualifiedPathSegmentEvidence::Syntax,
                0,
                4,
                13,
            ),
            (
                "Item".to_string(),
                "type",
                veln_ast::QualifiedPathSegmentEvidence::Syntax,
                1,
                4,
                21,
            ),
            (
                "helper".to_string(),
                "module",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                0,
                4,
                28,
            ),
            (
                "Item".to_string(),
                "type",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                1,
                4,
                36,
            ),
            (
                "Ready".to_string(),
                "constructor",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                2,
                4,
                42,
            ),
            (
                "prelude".to_string(),
                "module",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                0,
                5,
                15,
            ),
            (
                "byte".to_string(),
                "function",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                1,
                5,
                24,
            ),
            (
                "helper".to_string(),
                "module",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                0,
                6,
                16,
            ),
            (
                "make".to_string(),
                "function",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                1,
                6,
                24,
            ),
            (
                "helper".to_string(),
                "module",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                0,
                8,
                5,
            ),
            (
                "Item".to_string(),
                "type",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                1,
                8,
                13,
            ),
            (
                "Ready".to_string(),
                "constructor",
                veln_ast::QualifiedPathSegmentEvidence::Resolved,
                2,
                8,
                19,
            ),
        ],
    );
}

#[test]
fn declaration_type_path_carriers_report_qualified_segments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use helper\n",
            "\n",
            "type Box\n",
            "  Wrap(Helper::Item)\n",
            "  Record { item: helper::item }\n",
            "end\n",
            "\n",
            "effect Store\n",
            "  fetch(input: Helper::Item) -> helper::item\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  format binary\n",
            "  payload: Helper::Item\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  0\n",
            "end\n",
        ),
    );
    let helper = SourceFile::new(
        "helper.veln",
        concat!("pub type Item\n", "  pub Ready(Int)\n", "end\n"),
    );
    let module = merged_modules_with_names([("main", source.clone()), ("helper", helper)]);
    let diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();

    let observed = diagnostics
        .iter()
        .map(|diagnostic| {
            let span = diagnostic.span.as_ref().expect("name diagnostic span");
            (
                &source.text()[span.start.offset..span.end.offset],
                class_detail(diagnostic),
                segment_index_detail(diagnostic),
                span.start.line,
                span.start.column,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        observed,
        [
            ("Helper", "module", Some(0), 4, 8),
            ("item", "type", Some(1), 5, 26),
            ("Helper", "module", Some(0), 9, 16),
            ("item", "type", Some(1), 9, 41),
            ("Helper", "module", Some(0), 14, 12),
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn unresolved_qualified_call_does_not_guess_module_segment_role() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> Int\n", "  Foo::bar()\n", "end\n"),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `Foo::bar`"
    }));
}

#[test]
fn unique_recovered_type_qualified_constructor_reports_type_segment() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Sample\n",
            "  Made(Int)\n",
            "end\n",
            "fn main() -> Sample\n",
            "  sample::Made(1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    let invalid = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                && diagnostic.message
                    == "type name `sample` must start with an ASCII uppercase letter"
        })
        .expect("recovered type segment casing diagnostic");
    assert_diagnostic_span(invalid, 5, 3, 5, 9);
    let details = invalid.details.to_json();
    assert!(
        details.contains("\"occurrence\":\"path_segment\""),
        "{details}"
    );
    assert!(details.contains("\"name_class\":\"type\""), "{details}");
    assert!(details.contains("\"segment_index\":0"), "{details}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn generated_unresolved_qualified_calls_do_not_create_segment_casing_work() {
    for count in [64, 128] {
        let source = SourceFile::new("main.veln", generated_unresolved_qualified_calls(count));
        let module = lower_surface_ast(&parse(&source).tree);
        let diagnostics = analyze_surface_module(&module);

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.invalid_case")
                .count(),
            0,
            "{diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.unresolved")
                .count(),
            count,
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn generated_declaration_type_carriers_classify_invalid_segments() {
    for count in [400, 800] {
        let source = SourceFile::new("main.veln", generated_declaration_type_carriers(count));
        let helper = SourceFile::new("helper.veln", concat!("pub type Item\n", "end\n"));
        let module = merged_modules_with_names([("main", source), ("helper", helper)]);
        let diagnostics = analyze_surface_module(&module);

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.invalid_case")
                .count(),
            count,
            "{diagnostics:#?}"
        );
        assert_eq!(
            classified_project_qualified_path_segments(&module)
                .into_iter()
                .filter(|segment| segment.role == veln_ast::NameClass::Module)
                .count(),
            count,
        );
    }
}

fn generated_unresolved_qualified_calls(count: usize) -> String {
    let mut source = String::new();
    for index in 0..count {
        source.push_str(&format!(
            "fn case_{index}() -> Int\n  Missing::bad::Value()\nend\n"
        ));
    }
    source
}

fn generated_declaration_type_carriers(count: usize) -> String {
    let mut source = String::from("use helper\n\ntype Many\n");
    for index in 0..count {
        source.push_str(&format!("  Case{index}(Helper::Item)\n"));
    }
    source.push_str("end\n\nfn main() -> Int\n  0\nend\n");
    source
}

fn merged_modules_with_names<const N: usize>(sources: [(&str, SourceFile); N]) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for (name, source) in sources {
        let mut module = lower_surface_ast(&parse(&source).tree);
        for use_decl in &mut module.uses {
            use_decl.module_name = Some(name.to_string());
        }
        for alias in &mut module.aliases {
            alias.module_name = Some(name.to_string());
        }
        for effect in &mut module.effects {
            effect.module_name = Some(name.to_string());
        }
        for handler in &mut module.handlers {
            handler.module_name = Some(name.to_string());
        }
        for type_decl in &mut module.types {
            type_decl.module_name = Some(name.to_string());
        }
        for schema in &mut module.schemas {
            schema.module_name = Some(name.to_string());
        }
        for codec in &mut module.codecs {
            codec.module_name = Some(name.to_string());
        }
        for function in &mut module.functions {
            function.module_name = Some(name.to_string());
        }
        merged.uses.extend(module.uses);
        merged.aliases.extend(module.aliases);
        merged.effects.extend(module.effects);
        merged.handlers.extend(module.handlers);
        merged.schemas.extend(module.schemas);
        merged.codecs.extend(module.codecs);
        merged.types.extend(module.types);
        merged.functions.extend(module.functions);
        merged.invalid_names.extend(module.invalid_names);
    }
    merged
}

fn class_detail(diagnostic: &Diagnostic) -> &'static str {
    let details = diagnostic.details.to_json();
    for class in ["module", "type", "constructor", "function", "value_binding"] {
        if details.contains(&format!("\"name_class\":\"{class}\"")) {
            return class;
        }
    }
    panic!("diagnostic did not include name_class: {details}");
}

fn segment_index_detail(diagnostic: &Diagnostic) -> Option<usize> {
    let details = diagnostic.details.to_json();
    (0..4).find(|index| details.contains(&format!("\"segment_index\":{index}")))
}
