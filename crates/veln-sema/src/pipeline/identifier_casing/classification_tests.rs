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
