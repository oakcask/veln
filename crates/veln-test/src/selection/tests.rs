use std::fs;

use veln_ast::{lower_surface_ast, lower_surface_ast_with_module_identity};
use veln_source::{SourceFile, TextRange};
use veln_syntax::parse;

use super::*;

#[test]
fn discovered_selection_uses_test_file_pattern() {
    let module = empty_surface_module();
    let project = main_and_test_project();

    let test_files = selected_test_files(&project, &module, None);
    let selection = TestSelection::new(&project, &test_files, false);

    assert_eq!(selection.mode, TestSelectionMode::Discovered);
    assert_eq!(selection.targets, vec!["main_test.veln"]);
    assert_eq!(selection.reason, TestSelectionReason::PatternDiscovery);
}

#[test]
fn discovered_selection_includes_same_file_test_declarations() {
    let source = SourceFile::new("main.veln", "test same_file() -> ()\n  ()\nend\n");
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = lower_surface_ast(&parsed.tree);
    let project = Project {
        root: PathBuf::new(),
        manifest: None,
        files: vec![source],
    };

    let test_files = selected_test_files(&project, &module, None);
    let selection = TestSelection::new(&project, &test_files, false);

    assert_eq!(selection.targets, vec!["main.veln"]);
    assert_eq!(selection.reason, TestSelectionReason::PatternDiscovery);
}

#[test]
fn explicit_selection_includes_non_test_files() {
    let module = empty_surface_module();
    let project = main_and_test_project();

    let selected_roots = BTreeSet::from(["main.veln".to_string(), "main_test.veln".to_string()]);
    let test_files = selected_test_files(&project, &module, Some(&selected_roots));
    let selection = TestSelection::new(&project, &test_files, true);

    assert_eq!(
        test_files,
        BTreeSet::from(["main.veln".to_string(), "main_test.veln".to_string()])
    );
    assert_eq!(selection.mode, TestSelectionMode::Explicit);
    assert_eq!(selection.targets, vec!["main.veln", "main_test.veln"]);
    assert_eq!(selection.reason, TestSelectionReason::UserSelected);
}

#[test]
fn dependency_graph_selects_tests_that_import_selected_source() {
    let (project, module) = project_module(vec![
        SourceFile::new(
            "math.veln",
            concat!(
                "pub fn double(value: Int) -> Int\n",
                "  value * 2\n",
                "end\n",
            ),
        ),
        SourceFile::new(
            "app_test.veln",
            concat!(
                "use math\n",
                "\n",
                "test doubles() -> Int\n",
                "  math::double(2)\n",
                "end\n",
            ),
        ),
    ]);
    let plan = selected_math_source_plan(&project, &module);

    assert_eq!(
        plan.analysis_targets,
        vec![PathBuf::from("app_test.veln"), PathBuf::from("math.veln")]
    );
    assert_eq!(
        plan.selected_roots,
        Some(BTreeSet::from([
            "app_test.veln".to_string(),
            "math.veln".to_string(),
        ]))
    );
    assert_complete_dependency_graph(&plan.metadata, "added 1 test source by dependency graph");
}

#[test]
fn dependency_graph_widens_when_selected_source_has_no_module_identity() {
    let (project, module) = project_module_without_derived_identity(vec![
        SourceFile::new("math.veln", "fn value() -> Int\n  1\nend\n"),
        SourceFile::new("alpha_test.veln", "test alpha() -> ()\n  ()\nend\n"),
        SourceFile::new("beta_test.veln", "test beta() -> ()\n  ()\nend\n"),
    ]);
    let plan = selected_math_source_plan(&project, &module);

    assert_eq!(
        plan.analysis_targets,
        vec![
            PathBuf::from("alpha_test.veln"),
            PathBuf::from("beta_test.veln"),
            PathBuf::from("math.veln"),
        ]
    );
    assert_eq!(
        plan.selected_roots,
        Some(BTreeSet::from([
            "alpha_test.veln".to_string(),
            "beta_test.veln".to_string(),
            "math.veln".to_string(),
        ]))
    );
    assert_eq!(
        plan.metadata.confidence,
        Some(TestSelectionConfidence::Unknown)
    );
    assert_eq!(
        plan.metadata.reason,
        Some(TestSelectionReason::WidenedDependencyGraph)
    );
    assert!(plan.metadata.notes.contains(
        &"dependency graph is missing module identity for selected source `math.veln`".to_string()
    ));
}

#[test]
fn dependency_graph_upgrades_convention_selection_to_complete() {
    let (project, module) = project_module(vec![
        SourceFile::new(
            "math.veln",
            concat!(
                "pub fn double(value: Int) -> Int\n",
                "  value * 2\n",
                "end\n",
            ),
        ),
        SourceFile::new(
            "math_test.veln",
            concat!(
                "use math\n",
                "\n",
                "test doubles() -> Int\n",
                "  math::double(2)\n",
                "end\n",
            ),
        ),
    ]);
    let explicit_roots = BTreeSet::from(["math.veln".to_string(), "math_test.veln".to_string()]);
    let source_roots = BTreeSet::from(["math.veln".to_string()]);

    let plan =
        dependency_aware_selection_plan(&project, &module, &explicit_roots, &source_roots, 1);

    assert_complete_dependency_graph(
        &plan.metadata,
        "added 1 test file by source-to-test convention",
    );
}

#[test]
fn empty_explicit_targets_do_not_expand() {
    let expansion = expand_test_targets(&PathBuf::new(), &[]);

    assert!(expansion.targets.is_empty());
    assert_eq!(expansion.source_to_test_added_count, 0);
}

#[test]
fn expands_explicit_source_target_to_paired_test_file() {
    let root = test_root("paired-source");
    fs::create_dir_all(&root).expect("create test root");
    fs::write(root.join("app.veln"), "").expect("write source file");
    fs::write(root.join("app_test.veln"), "").expect("write test file");

    let expansion = expand_test_targets(&root, &[PathBuf::from("app.veln")]);

    assert_eq!(
        expansion.targets,
        vec![PathBuf::from("app.veln"), PathBuf::from("app_test.veln")]
    );
    assert_eq!(expansion.source_to_test_added_count, 1);
    fs::remove_dir_all(root).expect("remove test root");
}

#[test]
fn expands_nested_relative_source_target_to_paired_test_file() {
    let root = test_root("paired-nested-source");
    fs::create_dir_all(root.join("src/cases")).expect("create test root");
    fs::write(root.join("src/cases/app.veln"), "").expect("write source file");
    fs::write(root.join("src/cases/app_test.veln"), "").expect("write test file");

    let expansion = expand_test_targets(&root, &[PathBuf::from("src/cases/app.veln")]);

    assert_eq!(
        expansion.targets,
        vec![
            PathBuf::from("src/cases/app.veln"),
            PathBuf::from("src/cases/app_test.veln"),
        ]
    );
    assert_eq!(expansion.source_to_test_added_count, 1);
    fs::remove_dir_all(root).expect("remove test root");
}

#[test]
fn expands_multiple_source_targets_to_paired_test_files() {
    let root = test_root("paired-multiple-source");
    fs::create_dir_all(&root).expect("create test root");
    fs::write(root.join("app.veln"), "").expect("write app source file");
    fs::write(root.join("app_test.veln"), "").expect("write app test file");
    fs::write(root.join("lib.veln"), "").expect("write lib source file");
    fs::write(root.join("lib_test.veln"), "").expect("write lib test file");

    let expansion = expand_test_targets(
        &root,
        &[PathBuf::from("lib.veln"), PathBuf::from("app.veln")],
    );

    assert_eq!(
        expansion.targets,
        vec![
            PathBuf::from("app.veln"),
            PathBuf::from("app_test.veln"),
            PathBuf::from("lib.veln"),
            PathBuf::from("lib_test.veln"),
        ]
    );
    assert_eq!(expansion.source_to_test_added_count, 2);
    fs::remove_dir_all(root).expect("remove test root");
}

#[test]
fn source_to_test_expansion_deduplicates_explicit_paired_target() {
    let root = test_root("paired-source-dedupe");
    fs::create_dir_all(&root).expect("create test root");
    fs::write(root.join("app.veln"), "").expect("write source file");
    fs::write(root.join("app_test.veln"), "").expect("write test file");

    let expansion = expand_test_targets(
        &root,
        &[PathBuf::from("app.veln"), PathBuf::from("app_test.veln")],
    );

    assert_eq!(
        expansion.targets,
        vec![PathBuf::from("app.veln"), PathBuf::from("app_test.veln")]
    );
    assert_eq!(expansion.source_to_test_added_count, 0);
    fs::remove_dir_all(root).expect("remove test root");
}

#[test]
fn does_not_expand_directory_or_test_file_targets() {
    let root = test_root("direct-target");
    fs::create_dir_all(root.join("cases")).expect("create test root");
    fs::write(root.join("app_test.veln"), "").expect("write test file");

    let expansion = expand_test_targets(
        &root,
        &[PathBuf::from("cases"), PathBuf::from("app_test.veln")],
    );

    assert_eq!(
        expansion.targets,
        vec![PathBuf::from("app_test.veln"), PathBuf::from("cases")]
    );
    assert_eq!(expansion.source_to_test_added_count, 0);
    fs::remove_dir_all(root).expect("remove test root");
}

#[test]
fn selection_targets_report_doctest_origin_source() {
    let project = Project {
        root: PathBuf::new(),
        manifest: None,
        files: vec![
            SourceFile::new("main.veln", ""),
            SourceFile::new("main.veln#doctest-1_test.veln", ""),
        ],
    };
    let test_files = BTreeSet::from(["main.veln#doctest-1_test.veln".to_string()]);

    assert_eq!(selection_targets(&project, &test_files), vec!["main.veln"]);
}

#[test]
fn source_to_test_convention_records_plural_note() {
    let selection = TestSelection {
        mode: TestSelectionMode::Explicit,
        targets: vec!["app.veln".to_string(), "app_test.veln".to_string()],
        confidence: TestSelectionConfidence::Complete,
        reason: TestSelectionReason::UserSelected,
        notes: Vec::new(),
    }
    .source_to_test_convention(2);

    assert_eq!(selection.confidence, TestSelectionConfidence::Partial);
    assert_eq!(
        selection.reason,
        TestSelectionReason::SourceToTestConvention
    );
    assert_eq!(
        selection.notes,
        vec!["added 2 test files by source-to-test convention"]
    );
    assert!(
        selection
            .to_json()
            .to_json()
            .contains("\"notes\":[\"added 2 test files by source-to-test convention\"]")
    );
}

#[test]
fn source_to_test_convention_zero_count_keeps_original_selection() {
    let selection = TestSelection {
        mode: TestSelectionMode::Explicit,
        targets: vec!["app.veln".to_string()],
        confidence: TestSelectionConfidence::Complete,
        reason: TestSelectionReason::UserSelected,
        notes: Vec::new(),
    }
    .source_to_test_convention(0);

    assert_eq!(selection.confidence, TestSelectionConfidence::Complete);
    assert_eq!(selection.reason, TestSelectionReason::UserSelected);
    assert!(selection.notes.is_empty());
    assert!(!selection.to_json().to_json().contains("\"notes\""));
}

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("veln-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

fn project_module(sources: Vec<SourceFile>) -> (Project, SurfaceModule) {
    project_module_with_lowering(sources, true)
}

fn project_module_without_derived_identity(sources: Vec<SourceFile>) -> (Project, SurfaceModule) {
    project_module_with_lowering(sources, false)
}

fn project_module_with_lowering(
    sources: Vec<SourceFile>,
    derive_identity: bool,
) -> (Project, SurfaceModule) {
    let mut module = None;
    let mut uses = Vec::new();
    let mut aliases = Vec::new();
    let mut types = Vec::new();
    let mut schemas = Vec::new();
    let mut functions = Vec::new();
    let mut invalid_names = Vec::new();
    for source in &sources {
        let parsed = parse(source);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}",
            parsed.diagnostics
        );
        let lowered = if derive_identity {
            lower_surface_ast_with_module_identity(
                &parsed.tree,
                derived_module_name(source),
                source.span(TextRange::new(0, 0)),
            )
        } else {
            lower_surface_ast(&parsed.tree)
        };
        module = module.or(lowered.module);
        uses.extend(lowered.uses);
        aliases.extend(lowered.aliases);
        types.extend(lowered.types);
        schemas.extend(lowered.schemas);
        functions.extend(lowered.functions);
        invalid_names.extend(lowered.invalid_names);
    }
    (
        Project {
            root: PathBuf::new(),
            files: sources,
            manifest: None,
        },
        SurfaceModule {
            module,
            uses,
            aliases,
            effects: Vec::new(),
            handlers: Vec::new(),
            types,
            schemas,
            functions,
            invalid_names,
        },
    )
}

fn derived_module_name(source: &SourceFile) -> String {
    source
        .path()
        .as_str()
        .strip_suffix(".veln")
        .expect("selection tests use .veln source paths")
        .replace('/', "::")
}

fn empty_surface_module() -> SurfaceModule {
    SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    }
}

fn main_and_test_project() -> Project {
    Project {
        root: PathBuf::new(),
        manifest: None,
        files: vec![
            SourceFile::new("main.veln", ""),
            SourceFile::new("main_test.veln", ""),
        ],
    }
}

fn assert_complete_dependency_graph(metadata: &TestSelectionMetadata, note: &str) {
    assert_eq!(metadata.confidence, Some(TestSelectionConfidence::Complete));
    assert_eq!(metadata.reason, Some(TestSelectionReason::DependencyGraph));
    assert_eq!(metadata.notes, vec![note.to_string()]);
}

fn selected_math_source_plan(project: &Project, module: &SurfaceModule) -> TestSelectionPlan {
    let roots = BTreeSet::from(["math.veln".to_string()]);
    dependency_aware_selection_plan(project, module, &roots, &roots, 0)
}
