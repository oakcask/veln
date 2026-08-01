use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use veln_ast::{FunctionKind, SurfaceModule, UseOrigin};
use veln_diagnostics::JsonValue;
use veln_project::Project;

pub struct TestTargetExpansion {
    pub targets: Vec<PathBuf>,
    pub source_to_test_added_count: usize,
}

pub struct TestSelectionPlan {
    pub analysis_targets: Vec<PathBuf>,
    pub selected_roots: Option<BTreeSet<String>>,
    pub metadata: TestSelectionMetadata,
}

#[derive(Default)]
pub struct TestSelectionMetadata {
    pub confidence: Option<TestSelectionConfidence>,
    pub reason: Option<TestSelectionReason>,
    pub notes: Vec<String>,
}

impl TestSelectionPlan {
    pub fn discovered() -> Self {
        Self {
            analysis_targets: Vec::new(),
            selected_roots: None,
            metadata: TestSelectionMetadata::default(),
        }
    }

    pub fn explicit(selected_roots: BTreeSet<String>, source_to_test_added_count: usize) -> Self {
        Self {
            analysis_targets: selected_roots.iter().map(PathBuf::from).collect(),
            selected_roots: Some(selected_roots),
            metadata: source_to_test_metadata(source_to_test_added_count),
        }
    }
}

pub fn expand_test_targets(root: &Path, targets: &[PathBuf]) -> TestTargetExpansion {
    if targets.is_empty() {
        return TestTargetExpansion {
            targets: Vec::new(),
            source_to_test_added_count: 0,
        };
    }

    let mut original_targets = targets.to_vec();
    original_targets.sort();
    original_targets.dedup();
    let original_count = original_targets.len();
    let mut expanded = targets.to_vec();
    for target in targets {
        if let Some(test_target) = paired_test_target(root, target) {
            expanded.push(test_target);
        }
    }
    expanded.sort();
    expanded.dedup();
    let source_to_test_added_count = expanded.len().saturating_sub(original_count);
    TestTargetExpansion {
        targets: expanded,
        source_to_test_added_count,
    }
}

fn paired_test_target(root: &Path, target: &Path) -> Option<PathBuf> {
    let absolute = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    if absolute.is_dir()
        || absolute
            .extension()
            .is_none_or(|extension| extension != "veln")
    {
        return None;
    }
    let file_name = absolute.file_name()?.to_str()?;
    if file_name.ends_with("_test.veln") {
        return None;
    }
    let stem = absolute.file_stem()?.to_str()?;
    let candidate = absolute.with_file_name(format!("{stem}_test.veln"));
    if !candidate.is_file() {
        return None;
    }
    if target.is_absolute() {
        Some(candidate)
    } else {
        candidate.strip_prefix(root).map_or_else(
            |_| Some(candidate.clone()),
            |relative| Some(relative.to_path_buf()),
        )
    }
}

pub fn selected_test_files(
    project: &Project,
    module: &SurfaceModule,
    selected_roots: Option<&BTreeSet<String>>,
) -> BTreeSet<String> {
    if let Some(selected_roots) = selected_roots {
        return project
            .files
            .iter()
            .filter(|source| selected_roots.contains(selection_target_path(source.path().as_str())))
            .map(|source| source.path().as_str().to_string())
            .collect();
    }

    project
        .files
        .iter()
        .filter(|source| source.path().as_str().ends_with("_test.veln"))
        .map(|source| source.path().as_str().to_string())
        .chain(same_file_test_files(module))
        .collect()
}

fn same_file_test_files(module: &SurfaceModule) -> BTreeSet<String> {
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Test)
        .map(|function| function.span.file.as_str().to_string())
        .collect()
}

pub fn selection_targets(project: &Project, test_files: &BTreeSet<String>) -> Vec<String> {
    let mut targets = BTreeSet::new();
    project
        .files
        .iter()
        .filter(|source| {
            let path = source.path().as_str();
            test_files.contains(path)
        })
        .for_each(|source| {
            targets.insert(selection_target_path(source.path().as_str()).to_string());
        });
    targets.into_iter().collect()
}

fn selection_target_path(path: &str) -> &str {
    path.split_once("#doctest-")
        .map_or(path, |(origin, _)| origin)
}

pub fn dependency_aware_selection_plan(
    project: &Project,
    module: &SurfaceModule,
    explicit_roots: &BTreeSet<String>,
    source_roots: &BTreeSet<String>,
    source_to_test_added_count: usize,
) -> TestSelectionPlan {
    let graph = SourceDependencyGraph::new(project, module);
    let source_roots = source_roots
        .iter()
        .filter(|path| !graph.test_roots.contains(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    if source_roots.is_empty() {
        return TestSelectionPlan::explicit(explicit_roots.clone(), source_to_test_added_count);
    }

    let mut selected_roots = explicit_roots.clone();
    let mut metadata = source_to_test_metadata(source_to_test_added_count);
    let mut missing_evidence = graph.missing_evidence_for(&source_roots);

    if !missing_evidence.is_empty() {
        selected_roots.extend(graph.test_roots.iter().cloned());
        metadata.confidence = Some(TestSelectionConfidence::Unknown);
        metadata.reason = Some(TestSelectionReason::WidenedDependencyGraph);
        metadata.notes.append(&mut missing_evidence);
        metadata.notes.push(
            "selected all discovered tests because dependency graph evidence is incomplete"
                .to_string(),
        );
        return TestSelectionPlan {
            analysis_targets: graph.all_paths(),
            selected_roots: Some(selected_roots),
            metadata,
        };
    }

    let affected_tests = graph.affected_tests(&source_roots);
    let graph_added_count = affected_tests
        .iter()
        .filter(|path| !selected_roots.contains(*path))
        .count();
    let graph_selected_count = affected_tests.len();
    selected_roots.extend(affected_tests);

    if graph_selected_count > 0 {
        metadata.confidence = Some(TestSelectionConfidence::Complete);
        metadata.reason = Some(TestSelectionReason::DependencyGraph);
    }

    if graph_added_count > 0 {
        let noun = if graph_added_count == 1 {
            "test source"
        } else {
            "test sources"
        };
        metadata.notes.push(format!(
            "added {graph_added_count} {noun} by dependency graph"
        ));
    }

    let analysis_targets = graph.analysis_closure(&selected_roots, &source_roots);
    TestSelectionPlan {
        analysis_targets,
        selected_roots: Some(selected_roots),
        metadata,
    }
}

fn source_to_test_metadata(added_count: usize) -> TestSelectionMetadata {
    let mut metadata = TestSelectionMetadata::default();
    if added_count > 0 {
        metadata.confidence = Some(TestSelectionConfidence::Partial);
        metadata.reason = Some(TestSelectionReason::SourceToTestConvention);
        let noun = if added_count == 1 { "file" } else { "files" };
        metadata.notes.push(format!(
            "added {added_count} test {noun} by source-to-test convention"
        ));
    }
    metadata
}

struct SourceDependencyGraph {
    paths: BTreeSet<String>,
    test_roots: BTreeSet<String>,
    module_by_path: BTreeMap<String, String>,
    module_paths: BTreeMap<String, BTreeSet<String>>,
    imports_by_path: BTreeMap<String, Vec<String>>,
    edges: BTreeMap<String, BTreeSet<String>>,
}

impl SourceDependencyGraph {
    fn new(project: &Project, module: &SurfaceModule) -> Self {
        let paths = Self::collect_source_paths(project);
        let (module_by_path, module_paths) = Self::index_modules(&paths, module);
        let imports_by_path = Self::index_imports(&paths, module);
        let test_roots = Self::detect_test_roots(&paths, module);
        let edges = Self::build_edges(&imports_by_path, &module_paths);

        Self {
            paths,
            test_roots,
            module_by_path,
            module_paths,
            imports_by_path,
            edges,
        }
    }

    fn collect_source_paths(project: &Project) -> BTreeSet<String> {
        project
            .files
            .iter()
            .filter_map(|source| {
                let path = source.path().as_str();
                (!path.contains("#doctest-")).then(|| path.to_string())
            })
            .collect()
    }

    fn index_modules(
        paths: &BTreeSet<String>,
        module: &SurfaceModule,
    ) -> (BTreeMap<String, String>, BTreeMap<String, BTreeSet<String>>) {
        let mut module_by_path = BTreeMap::<String, String>::new();
        let mut module_paths = BTreeMap::<String, BTreeSet<String>>::new();
        for function in &module.functions {
            let Some(module_name) = &function.module_name else {
                continue;
            };
            let path = selection_target_path(function.span.file.as_str()).to_string();
            if !paths.contains(&path) {
                continue;
            }
            module_by_path
                .entry(path.clone())
                .or_insert_with(|| module_name.clone());
            module_paths
                .entry(module_name.clone())
                .or_default()
                .insert(path);
        }
        (module_by_path, module_paths)
    }

    fn index_imports(
        paths: &BTreeSet<String>,
        module: &SurfaceModule,
    ) -> BTreeMap<String, Vec<String>> {
        let mut imports_by_path = BTreeMap::<String, Vec<String>>::new();
        for use_decl in &module.uses {
            if use_decl.origin != UseOrigin::Source {
                continue;
            }
            let path = selection_target_path(use_decl.span.file.as_str()).to_string();
            if paths.contains(&path) {
                imports_by_path
                    .entry(path)
                    .or_default()
                    .push(use_decl.name.clone());
            }
        }
        imports_by_path
    }

    fn detect_test_roots(paths: &BTreeSet<String>, module: &SurfaceModule) -> BTreeSet<String> {
        let same_file_tests = same_file_test_files(module)
            .into_iter()
            .map(|path| selection_target_path(&path).to_string())
            .collect::<BTreeSet<_>>();
        paths
            .iter()
            .filter(|path| path.ends_with("_test.veln") || same_file_tests.contains(*path))
            .cloned()
            .collect()
    }

    fn build_edges(
        imports_by_path: &BTreeMap<String, Vec<String>>,
        module_paths: &BTreeMap<String, BTreeSet<String>>,
    ) -> BTreeMap<String, BTreeSet<String>> {
        imports_by_path
            .iter()
            .map(|(path, imports)| {
                let dependencies = imports
                    .iter()
                    .filter_map(|module_name| module_paths.get(module_name))
                    .flatten()
                    .filter(|dependency| *dependency != path)
                    .cloned()
                    .collect::<BTreeSet<_>>();
                (path.clone(), dependencies)
            })
            .collect()
    }

    fn all_paths(&self) -> Vec<PathBuf> {
        self.paths.iter().map(PathBuf::from).collect()
    }

    fn missing_evidence_for(&self, source_roots: &BTreeSet<String>) -> Vec<String> {
        let mut missing = Vec::new();
        for path in source_roots {
            if self.paths.contains(path) && !self.module_by_path.contains_key(path) {
                missing.push(format!(
                    "dependency graph is missing module identity for selected source `{path}`"
                ));
            }
        }
        for (path, imports) in &self.imports_by_path {
            for module_name in imports {
                if !self.module_paths.contains_key(module_name) {
                    missing.push(format!(
                        "dependency graph is missing source for import `{module_name}` in `{path}`"
                    ));
                }
            }
        }
        missing.sort();
        missing.dedup();
        missing
    }

    fn affected_tests(&self, source_roots: &BTreeSet<String>) -> BTreeSet<String> {
        self.test_roots
            .iter()
            .filter(|test_root| {
                let dependencies = self.dependency_closure(test_root);
                source_roots
                    .iter()
                    .any(|source_root| dependencies.contains(source_root))
            })
            .cloned()
            .collect()
    }

    fn analysis_closure(
        &self,
        selected_roots: &BTreeSet<String>,
        source_roots: &BTreeSet<String>,
    ) -> Vec<PathBuf> {
        let mut closure = source_roots.clone();
        for root in selected_roots {
            closure.extend(self.dependency_closure(root));
        }
        closure.into_iter().map(PathBuf::from).collect()
    }

    fn dependency_closure(&self, root: &str) -> BTreeSet<String> {
        let mut visited = BTreeSet::new();
        let mut stack = vec![root.to_string()];
        while let Some(path) = stack.pop() {
            if !visited.insert(path.clone()) {
                continue;
            }
            if let Some(dependencies) = self.edges.get(&path) {
                stack.extend(dependencies.iter().cloned());
            }
        }
        visited
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestSelectionMode {
    Discovered,
    Explicit,
}

impl TestSelectionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Explicit => "explicit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestSelectionConfidence {
    Complete,
    Partial,
    Unknown,
}

impl TestSelectionConfidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestSelectionReason {
    PatternDiscovery,
    UserSelected,
    SourceToTestConvention,
    DependencyGraph,
    WidenedDependencyGraph,
}

impl TestSelectionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PatternDiscovery => "pattern_discovery",
            Self::UserSelected => "user_selected",
            Self::SourceToTestConvention => "source_to_test_convention",
            Self::DependencyGraph => "dependency_graph",
            Self::WidenedDependencyGraph => "widened_dependency_graph",
        }
    }
}

pub struct TestSelection {
    pub mode: TestSelectionMode,
    pub targets: Vec<String>,
    pub confidence: TestSelectionConfidence,
    pub reason: TestSelectionReason,
    pub notes: Vec<String>,
}

impl TestSelection {
    pub fn new(project: &Project, test_files: &BTreeSet<String>, explicit: bool) -> Self {
        Self {
            mode: if explicit {
                TestSelectionMode::Explicit
            } else {
                TestSelectionMode::Discovered
            },
            targets: selection_targets(project, test_files),
            confidence: TestSelectionConfidence::Complete,
            reason: if explicit {
                TestSelectionReason::UserSelected
            } else {
                TestSelectionReason::PatternDiscovery
            },
            notes: Vec::new(),
        }
    }

    pub fn source_to_test_convention(self, added_count: usize) -> Self {
        self.apply_metadata(source_to_test_metadata(added_count))
    }

    pub fn apply_metadata(mut self, metadata: TestSelectionMetadata) -> Self {
        if let Some(confidence) = metadata.confidence {
            self.confidence = confidence;
        }
        if let Some(reason) = metadata.reason {
            self.reason = reason;
        }
        self.notes.extend(metadata.notes);
        self
    }

    pub(crate) fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("mode", JsonValue::string(self.mode.as_str())),
            (
                "targets",
                JsonValue::array(self.targets.iter().map(JsonValue::string)),
            ),
            ("confidence", JsonValue::string(self.confidence.as_str())),
            ("reason", JsonValue::string(self.reason.as_str())),
        ];
        if !self.notes.is_empty() {
            fields.push((
                "notes",
                JsonValue::array(self.notes.iter().map(JsonValue::string)),
            ));
        }
        JsonValue::object(fields)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use veln_ast::{lower_surface_ast, lower_surface_ast_with_module_identity};
    use veln_source::{SourceFile, TextRange};
    use veln_syntax::parse;

    use super::*;

    #[test]
    fn discovered_selection_uses_test_file_pattern() {
        let module = SurfaceModule {
            module: None,
            uses: Vec::new(),
            aliases: Vec::new(),
            effects: Vec::new(),
            handlers: Vec::new(),
            schemas: Vec::new(),
            codecs: Vec::new(),
            types: Vec::new(),
            functions: Vec::new(),
        };
        let project = Project {
            root: PathBuf::new(),
            manifest: None,
            files: vec![
                SourceFile::new("main.veln", ""),
                SourceFile::new("main_test.veln", ""),
            ],
        };

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
        let module = SurfaceModule {
            module: None,
            uses: Vec::new(),
            aliases: Vec::new(),
            effects: Vec::new(),
            handlers: Vec::new(),
            schemas: Vec::new(),
            codecs: Vec::new(),
            types: Vec::new(),
            functions: Vec::new(),
        };
        let project = Project {
            root: PathBuf::new(),
            manifest: None,
            files: vec![
                SourceFile::new("main.veln", ""),
                SourceFile::new("main_test.veln", ""),
            ],
        };

        let selected_roots =
            BTreeSet::from(["main.veln".to_string(), "main_test.veln".to_string()]);
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
        let explicit_roots = BTreeSet::from(["math.veln".to_string()]);
        let source_roots = BTreeSet::from(["math.veln".to_string()]);

        let plan =
            dependency_aware_selection_plan(&project, &module, &explicit_roots, &source_roots, 0);

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
        assert_eq!(
            plan.metadata.confidence,
            Some(TestSelectionConfidence::Complete)
        );
        assert_eq!(
            plan.metadata.reason,
            Some(TestSelectionReason::DependencyGraph)
        );
        assert_eq!(
            plan.metadata.notes,
            vec!["added 1 test source by dependency graph"]
        );
    }

    #[test]
    fn dependency_graph_widens_when_selected_source_has_no_module_identity() {
        let (project, module) = project_module_without_derived_identity(vec![
            SourceFile::new("math.veln", "fn value() -> Int\n  1\nend\n"),
            SourceFile::new("alpha_test.veln", "test alpha() -> ()\n  ()\nend\n"),
            SourceFile::new("beta_test.veln", "test beta() -> ()\n  ()\nend\n"),
        ]);
        let explicit_roots = BTreeSet::from(["math.veln".to_string()]);
        let source_roots = BTreeSet::from(["math.veln".to_string()]);

        let plan =
            dependency_aware_selection_plan(&project, &module, &explicit_roots, &source_roots, 0);

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
        assert!(
            plan.metadata.notes.contains(
                &"dependency graph is missing module identity for selected source `math.veln`"
                    .to_string()
            )
        );
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
        let explicit_roots =
            BTreeSet::from(["math.veln".to_string(), "math_test.veln".to_string()]);
        let source_roots = BTreeSet::from(["math.veln".to_string()]);

        let plan =
            dependency_aware_selection_plan(&project, &module, &explicit_roots, &source_roots, 1);

        assert_eq!(
            plan.metadata.confidence,
            Some(TestSelectionConfidence::Complete)
        );
        assert_eq!(
            plan.metadata.reason,
            Some(TestSelectionReason::DependencyGraph)
        );
        assert_eq!(
            plan.metadata.notes,
            vec!["added 1 test file by source-to-test convention"]
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

    fn project_module_without_derived_identity(
        sources: Vec<SourceFile>,
    ) -> (Project, SurfaceModule) {
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
        let mut codecs = Vec::new();
        let mut functions = Vec::new();
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
            codecs.extend(lowered.codecs);
            functions.extend(lowered.functions);
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
                codecs,
                functions,
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
}
