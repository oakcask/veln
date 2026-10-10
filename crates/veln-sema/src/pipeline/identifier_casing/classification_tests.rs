use super::*;
use crate::types::effect_inference_counters;
use crate::types::private_inference::private_inference_counters;
use veln_source::SourceFile;

#[test]
fn path_classification_preserves_inferred_function_roles_without_running_inference() {
    for count in [16, 32] {
        let mut text = String::from("mod main\nuse main\n");
        for index in 0..count {
            text.push_str(&format!(
                "fn identity_{index}(value)\n  value\nend\n\
                 pub fn exported_{index} = identity_{index}\n\
                 fn caller_{index}() -> Int\n  identity_{index}(1)\n  main::exported_{index}(1)\nend\n"
            ));
        }
        let module = named_module("main", &text);
        private_inference_counters::reset();
        effect_inference_counters::reset();
        let environment = TypeEnvironment::from_module(&module);
        assert!(effect_inference_counters::snapshot().dependency_discovery_scans > 0);
        let expected = classified_qualified_path_segments(&module, &environment);
        assert_eq!(expected.len(), count * 2);

        private_inference_counters::reset();
        effect_inference_counters::reset();
        let actual = classified_project_qualified_path_segments(&module);

        assert_eq!(actual, expected);
        assert_eq!(private_inference_counters::snapshot(), Default::default());
        assert_eq!(effect_inference_counters::snapshot(), Default::default());
    }
}

#[test]
fn path_classification_preserves_dependency_alias_constructor_and_recovery_roles() {
    let workspace = named_module(
        "main",
        concat!(
            "use helper from \"example/pkg\"\n",
            "fn main() -> Int\n",
            "  helper::exported(1)\n",
            "  helper::Item::Ready\n",
            "  Helper::exported(1)\n",
            "  helper::Exported(1)\n",
            "end\n",
        ),
    );
    let dependency = named_module(
        "helper",
        concat!(
            "pub type Item\n  pub Ready\nend\n",
            "fn identity(value)\n  value\nend\n",
            "pub fn exported = identity\n",
            "fn constrain() -> Int\n  identity(1)\nend\n",
        ),
    );
    let mut project = workspace.clone();
    project.types.extend(dependency.types);
    project.functions.extend(dependency.functions);
    project.aliases.extend(dependency.aliases);
    let environment = TypeEnvironment::from_module(&project);
    let expected = classified_qualified_path_segments(&workspace, &environment);
    assert!(
        expected
            .iter()
            .any(|segment| segment.role == NameClass::Constructor)
    );
    assert!(
        expected
            .iter()
            .any(|segment| { segment.evidence == QualifiedPathSegmentEvidence::UniqueRecovery })
    );

    assert_eq!(
        classified_project_qualified_path_segments_with_context(&workspace, &project),
        expected,
    );
}

#[test]
fn navigation_classification_excludes_invalid_refinement_unions_without_losing_casing_roles() {
    let module = named_module(
        "main",
        concat!(
            "pub type Left\n  pub LeftReady\nend\n",
            "pub type Right\n  pub RightReady\nend\n",
            "fn inspect(value: Left::LeftReady | Right::RightReady) -> Int\n  0\nend\n",
        ),
    );
    let environment = TypeEnvironment::from_module(&module);

    let casing_segments = classified_qualified_path_segments(&module, &environment);
    assert_eq!(
        casing_segments
            .iter()
            .filter(|segment| segment.role == NameClass::Constructor)
            .count(),
        2
    );

    let navigation_segments = classified_project_qualified_path_segments(&module);
    assert!(
        navigation_segments
            .iter()
            .filter(|segment| segment.name == "LeftReady" || segment.name == "RightReady")
            .all(|segment| segment.role != NameClass::Constructor),
        "{navigation_segments:#?}"
    );
}

#[test]
fn expression_type_argument_refinements_are_navigation_only() {
    let module = named_module(
        "main",
        concat!(
            "type State\n  Ready\nend\n",
            "fn inspect(value: State) -> State\n  keep<State::Ready>(value)\nend\n",
        ),
    );
    let environment = TypeEnvironment::from_module(&module);

    let casing_segments = classified_qualified_path_segments(&module, &environment);
    assert!(
        casing_segments
            .iter()
            .all(|segment| segment.span.start.line != 5),
        "{casing_segments:#?}"
    );

    let navigation_segments = classified_project_qualified_path_segments(&module);
    assert_eq!(
        navigation_segments
            .iter()
            .filter(|segment| segment.span.start.line == 5)
            .map(|segment| (segment.name.as_str(), segment.role))
            .collect::<Vec<_>>(),
        [
            ("State", NameClass::Type),
            ("Ready", NameClass::Constructor)
        ]
    );
}

#[test]
fn path_classification_builds_one_adt_registry_for_schema_helpers() {
    let mut text = String::from("mod model\n");
    for index in 0..16 {
        text.push_str(&format!("pub schema Packet{index}\n  value: Int\nend\n"));
    }
    let module = named_module("model", &text);

    crate::adt_source_less::reset_adt_registry_from_module_builds();
    let _ = classified_project_qualified_path_segments(&module);

    assert_eq!(crate::adt_source_less::adt_registry_from_module_builds(), 1);
}

#[test]
fn invalid_path_classification_work_grows_with_invalid_segments() {
    fn index_lookups(count: usize) -> usize {
        let mut text = String::from("mod main\n");
        for index in 0..count {
            text.push_str(&format!(
                "fn caller_{index}() -> Int\n  Missing{index}::value\nend\n"
            ));
        }
        let module = named_module("main", &text);
        let environment = TypeEnvironment::from_module(&module);
        invalid_path_classification_counters::reset();
        let _ = classified_qualified_path_segments(&module, &environment);
        invalid_path_classification_counters::index_lookups()
    }

    let smaller = index_lookups(32);
    let larger = index_lookups(64);
    assert!(
        smaller > 0,
        "generated invalid paths must exercise the index"
    );
    assert_eq!(
        larger,
        smaller * 2,
        "invalid-path index work must grow linearly across adjacent input sizes"
    );
}

fn named_module(name: &str, text: &str) -> SurfaceModule {
    let source = SourceFile::new(format!("{name}.veln"), text);
    let parsed = veln_syntax::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    veln_ast::lower_surface_ast_with_module_identity(
        &parsed.tree,
        name.to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    )
}
