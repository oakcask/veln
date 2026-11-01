use super::*;

#[derive(Clone, Copy)]
enum ClassificationWorkload {
    ExportedChain,
    IndependentInvalid,
    InvalidCasing,
    DuplicateCallable,
}

fn classification_work(
    count: usize,
    workload: ClassificationWorkload,
) -> PackageDocSemanticClassificationWork {
    let source = classification_source(count, workload);
    reset_package_doc_semantic_classification_work();
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-classification-scaling\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[("main.veln", source.as_str())],
    );
    assert_classification_result(&result, count, workload);
    take_package_doc_semantic_classification_work()
}

fn classification_source(count: usize, workload: ClassificationWorkload) -> String {
    match workload {
        ClassificationWorkload::ExportedChain => (0..count)
            .map(|index| {
                if index == 0 {
                    "pub let value_0: Int = \"wrong\"\n".to_string()
                } else {
                    format!("pub let value_{index}: Int = value_{}\n", index - 1)
                }
            })
            .collect::<String>(),
        ClassificationWorkload::IndependentInvalid => (0..count)
            .map(|index| format!("pub let value_{index}: Int = \"wrong-{index}\"\n"))
            .collect::<String>(),
        ClassificationWorkload::InvalidCasing => (0..count)
            .map(|index| format!("pub let BadValue{index}: Int = {index}\n"))
            .collect::<String>(),
        ClassificationWorkload::DuplicateCallable => {
            let functions = (0..count)
                .map(|index| format!("fn value_{index}() -> Int\n  {index}\nend\n"))
                .collect::<String>();
            let values = (0..count)
                .map(|index| format!("pub let value_{index}: Int = {index}\n"))
                .collect::<String>();
            format!("{functions}{values}")
        }
    }
}

fn assert_classification_result(
    result: &PackageDocResult,
    count: usize,
    workload: ClassificationWorkload,
) {
    assert!(result.catalog().is_none());
    let first_name = match workload {
        ClassificationWorkload::InvalidCasing => "BadValue0",
        ClassificationWorkload::ExportedChain
        | ClassificationWorkload::IndependentInvalid
        | ClassificationWorkload::DuplicateCallable => "value_0",
    };
    assert!(
        result
            .declaration_uri_for("main", "module_value", first_name)
            .is_none()
    );
    let diagnostics = &result.status().diagnostics;
    let expected_code = match workload {
        ClassificationWorkload::ExportedChain | ClassificationWorkload::IndependentInvalid => {
            "type.mismatch"
        }
        ClassificationWorkload::InvalidCasing => "name.invalid_case",
        ClassificationWorkload::DuplicateCallable => "name.duplicate",
    };
    let expected_count = match workload {
        ClassificationWorkload::ExportedChain => 1,
        ClassificationWorkload::IndependentInvalid | ClassificationWorkload::InvalidCasing => count,
        ClassificationWorkload::DuplicateCallable => count,
    };
    assert_eq!(diagnostics.len(), expected_count, "{diagnostics:#?}");
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.gate == "module_value" && diagnostic.code == expected_code
    }));
    let causal_offsets = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.span.as_ref().expect("causal source span").offset)
        .collect::<HashSet<_>>();
    assert_eq!(causal_offsets.len(), expected_count, "{diagnostics:#?}");
}

fn assert_linear_adjacent_work(
    original: PackageDocSemanticClassificationWork,
    adjacent: PackageDocSemanticClassificationWork,
) {
    assert!(original.exported_module_value_lookups > 0);
    assert!(original.diagnostic_lookups > 0);

    assert_eq!(
        adjacent.exported_module_value_lookups,
        original.exported_module_value_lookups * 2
    );
    assert!(adjacent.identity_root_expansions <= original.identity_root_expansions * 2);
    assert!(adjacent.retained_values <= original.retained_values * 2);
    assert!(adjacent.retained_edges <= original.retained_edges * 2 + 1);
    assert!(adjacent.retained_direct_causes <= original.retained_direct_causes * 2);
    assert!(adjacent.visited_initializations <= original.visited_initializations * 2);
    assert!(adjacent.traversal_values <= original.traversal_values * 2);
    assert!(adjacent.traversal_edges <= original.traversal_edges * 2 + 1);
    assert!(adjacent.cause_processing <= original.cause_processing * 2);
    assert!(adjacent.cause_index_insertions <= original.cause_index_insertions * 2);
    assert!(adjacent.diagnostic_lookups <= original.diagnostic_lookups * 2);
    assert!(adjacent.collision_identity_insertions <= original.collision_identity_insertions * 2);
    assert!(adjacent.collision_diagnostic_lookups <= original.collision_diagnostic_lookups * 2);
    assert!(
        adjacent.collision_association_iterations <= original.collision_association_iterations * 2
    );
}

#[test]
fn exported_invalid_chain_classification_is_linear_for_all_public_roots() {
    let original = classification_work(32, ClassificationWorkload::ExportedChain);
    let adjacent = classification_work(64, ClassificationWorkload::ExportedChain);
    eprintln!("exported chain classification work: 32={original:?}, 64={adjacent:?}");

    assert_linear_adjacent_work(original, adjacent);
    assert!(original.retained_values > 0);
    assert!(original.visited_initializations > 0);
    assert!(original.traversal_values > 0);
    assert!(original.cause_processing > 0);
    assert!(original.cause_index_insertions > 0);
    assert_eq!(original.retained_edges, 31);
    assert_eq!(adjacent.retained_edges, 63);
    assert_eq!(original.traversal_edges, 31);
    assert_eq!(adjacent.traversal_edges, 63);
    assert_eq!(original.casing_range_comparisons, 0);
    assert_eq!(adjacent.casing_range_comparisons, 0);
}

#[test]
fn independent_invalid_export_classification_is_linear() {
    let original = classification_work(32, ClassificationWorkload::IndependentInvalid);
    let adjacent = classification_work(64, ClassificationWorkload::IndependentInvalid);
    eprintln!("independent invalid classification work: 32={original:?}, 64={adjacent:?}");

    assert_linear_adjacent_work(original, adjacent);
    assert!(original.retained_values > 0);
    assert!(original.visited_initializations > 0);
    assert!(original.traversal_values > 0);
    assert!(original.cause_processing > 0);
    assert!(original.cause_index_insertions > 0);
    assert_eq!(original.retained_edges, 0);
    assert_eq!(adjacent.retained_edges, 0);
    assert_eq!(original.traversal_edges, 0);
    assert_eq!(adjacent.traversal_edges, 0);
    assert_eq!(original.casing_range_comparisons, 0);
    assert_eq!(adjacent.casing_range_comparisons, 0);
}

#[test]
fn invalid_casing_export_classification_uses_indexed_provenance_linearly() {
    let original = classification_work(32, ClassificationWorkload::InvalidCasing);
    let adjacent = classification_work(64, ClassificationWorkload::InvalidCasing);
    eprintln!("invalid casing classification work: 32={original:?}, 64={adjacent:?}");

    assert_linear_adjacent_work(original, adjacent);
    assert_eq!(original.casing_range_comparisons, 32);
    assert_eq!(adjacent.casing_range_comparisons, 64);
}

#[test]
fn duplicate_callable_classification_uses_keyed_span_lookups_linearly() {
    let original = classification_work(32, ClassificationWorkload::DuplicateCallable);
    let adjacent = classification_work(64, ClassificationWorkload::DuplicateCallable);
    let larger = classification_work(128, ClassificationWorkload::DuplicateCallable);
    eprintln!(
        "duplicate callable classification work: 32={original:?}, 64={adjacent:?}, 128={larger:?}"
    );

    assert_linear_adjacent_work(original, adjacent);
    assert_linear_adjacent_work(adjacent, larger);
    assert_eq!(original.collision_identity_insertions, 64);
    assert_eq!(adjacent.collision_identity_insertions, 128);
    assert_eq!(larger.collision_identity_insertions, 256);
    assert_eq!(original.collision_diagnostic_lookups, 32);
    assert_eq!(adjacent.collision_diagnostic_lookups, 64);
    assert_eq!(larger.collision_diagnostic_lookups, 128);
    assert_eq!(original.collision_association_iterations, 32);
    assert_eq!(adjacent.collision_association_iterations, 64);
    assert_eq!(larger.collision_association_iterations, 128);
}

fn captured_collision_dependency(
    package: &str,
    source_root: &str,
    source: &str,
) -> CapturedDependencyProject {
    CapturedDependencyProject {
        package: package.to_string(),
        source: source_root.to_string(),
        project: Some(Project {
            root: PathBuf::new(),
            files: vec![SourceFile::new("main.veln", source)],
            manifest: Some(parse_manifest_text(
                "veln.toml",
                &format!("[package]\nname = \"{package}\"\n[lib]\nexports = [\"main.veln\"]\n"),
            )),
        }),
    }
}

fn captured_collision_package_work(count: usize) -> PackageDocSemanticClassificationWork {
    let collision_source = concat!(
        "fn shared() -> Int\n",
        "  1\n",
        "end\n",
        "pub let shared: Int = 1\n",
    );
    let mut manifest = String::from(
        "[package]\nname = \"captured-collision-scaling\"\n[lib]\nexports = [\"main.veln\"]\n",
    );
    let mut imports = String::new();
    let mut values = String::new();
    let dependencies = (0..count)
        .map(|index| {
            let package = format!("p{index}");
            manifest.push_str(&format!(
                "[dependencies.\"{package}\"]\npath = \"vendor/{package}\"\n"
            ));
            imports.push_str(&format!("use main as dep_{index} from \"{package}\"\n"));
            values.push_str(&format!(
                "pub let value_{index}: Int = dep_{index}::shared\n"
            ));
            captured_collision_dependency(&package, &format!("vendor/{package}"), collision_source)
        })
        .collect::<Vec<_>>();
    let source = format!("{imports}{values}");
    reset_package_doc_semantic_classification_work();
    let result =
        generate_with_dependencies(&manifest, &[("main.veln", source.as_str())], &dependencies);
    assert!(result.catalog().is_none(), "{:#?}", result.status());
    assert_eq!(
        result
            .status()
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "name.duplicate")
            .count(),
        count,
        "{:#?}",
        result.status().diagnostics
    );
    assert!(
        result
            .status()
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.gate == "module_value")
    );
    take_package_doc_semantic_classification_work()
}

#[test]
fn captured_identical_span_collision_association_is_linear() {
    let original = captured_collision_package_work(8);
    let adjacent = captured_collision_package_work(16);
    eprintln!("captured collision association work: 8={original:?}, 16={adjacent:?}");

    assert_eq!(original.collision_association_iterations, 8);
    assert_eq!(adjacent.collision_association_iterations, 16);
    assert!(
        adjacent.collision_association_iterations <= original.collision_association_iterations * 2
    );
}
