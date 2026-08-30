use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use veln_ast::{FunctionKind, SurfaceModule, UseOrigin};
use veln_diagnostics::JsonValue;
use veln_project::{Project, classify_companion_source};

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
        if let Some(test_target) = paired_companion_test_target(root, target) {
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
    let absolute = absolute_veln_target(root, target)?;
    let file_name = absolute.file_name()?.to_str()?;
    if file_name.ends_with("_test.veln") {
        return None;
    }
    let stem = absolute.file_stem()?.to_str()?;
    let candidate = absolute.with_file_name(format!("{stem}_test.veln"));
    existing_paired_target(root, target, candidate)
}

fn paired_companion_test_target(root: &Path, target: &Path) -> Option<PathBuf> {
    let absolute = absolute_veln_target(root, target)?;
    let relative = if target.is_absolute() {
        absolute.strip_prefix(root).ok()?.to_path_buf()
    } else {
        target.to_path_buf()
    };
    let relative_text = relative.to_string_lossy().replace('\\', "/");
    if classify_companion_source(&relative_text).is_some() || relative_text.ends_with("_test.veln")
    {
        return None;
    }
    let candidate =
        absolute.with_file_name(format!("{}.test.veln", absolute.file_stem()?.to_str()?));
    existing_paired_target(root, target, candidate)
}

fn absolute_veln_target(root: &Path, target: &Path) -> Option<PathBuf> {
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
    Some(absolute)
}

fn existing_paired_target(root: &Path, target: &Path, candidate: PathBuf) -> Option<PathBuf> {
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
        .filter(|source| {
            let path = source.path().as_str();
            path.ends_with("_test.veln") || classify_companion_source(path).is_some()
        })
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
            .filter(|path| {
                path.ends_with("_test.veln")
                    || classify_companion_source(path).is_some()
                    || same_file_tests.contains(*path)
            })
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
#[path = "selection/tests.rs"]
mod tests;
