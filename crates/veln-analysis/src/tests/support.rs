use super::*;

pub(super) fn checked_diagnostic_json(project: Project) -> Vec<String> {
    analyze_project(project, DoctestMode::Exclude)
        .checked_diagnostics()
        .iter()
        .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
        .collect()
}

pub(super) fn checked_diagnostic_json_with_cache(
    project: Project,
    cache: &crate::analysis::TestStandardEnvironmentCache,
) -> Vec<String> {
    checked_diagnostic_json_with_cache_and_mode(project, DoctestMode::Exclude, cache)
}

pub(super) fn checked_diagnostic_json_with_cache_and_mode(
    project: Project,
    doctest_mode: DoctestMode,
    cache: &crate::analysis::TestStandardEnvironmentCache,
) -> Vec<String> {
    crate::analysis::analyze_project_with_test_standard_cache(project, doctest_mode, cache)
        .checked_diagnostics()
        .iter()
        .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
        .collect()
}

pub(super) fn lowered_function_names(analysis: &ReachableEntryAnalysis) -> Vec<&str> {
    analysis
        .lowered
        .core
        .as_ref()
        .expect("reachable entry should lower to core")
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect()
}

pub(super) fn standard_declaration_count(module: &SurfaceModule) -> usize {
    module
        .uses
        .iter()
        .filter(|decl| is_standard(&decl.module_name))
        .count()
        + module
            .aliases
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
        + module
            .effects
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
        + module
            .handlers
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
        + module
            .types
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
        + module
            .schemas
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
        + module
            .functions
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
}

pub(super) fn is_standard(module_name: &Option<String>) -> bool {
    module_name
        .as_deref()
        .is_some_and(|module_name| module_name.starts_with("std::"))
}

pub(super) fn checked_discovered_diagnostic_json_with_cache(
    temp: &TempProject,
    inputs: &[PathBuf],
    cache: &crate::analysis::TestStandardEnvironmentCache,
) -> Vec<String> {
    checked_diagnostic_json_with_cache(
        Project::discover(temp.root().to_path_buf(), inputs)
            .expect("project discovery should succeed"),
        cache,
    )
}

pub(super) fn assert_project_evidence(
    diagnostics: &[String],
    source_path: &str,
    module_path: &str,
    type_message: &str,
) {
    assert_eq!(
        diagnostic_ids(diagnostics),
        ["module.source_mod", "type.mismatch"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains(source_path)),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains(module_path)),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains(type_message)),
        "{diagnostics:#?}"
    );
}

pub(super) fn assert_no_project_leak(
    diagnostics: &[String],
    source_path: &str,
    module_path: &str,
    type_message: &str,
) {
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.contains(source_path)),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.contains(module_path)),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.contains(type_message)),
        "{diagnostics:#?}"
    );
}

pub(super) fn assert_diagnostics_contain(
    diagnostics: &[String],
    source_path: &str,
    type_message: &str,
) {
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains(source_path)),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains(type_message)),
        "{diagnostics:#?}"
    );
}

pub(super) fn diagnostic_ids(diagnostics: &[String]) -> Vec<&'static str> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            if diagnostic.contains("\"id\":\"module.source_mod\"") {
                "module.source_mod"
            } else if diagnostic.contains("\"id\":\"manifest.missing_export\"") {
                "manifest.missing_export"
            } else if diagnostic.contains("\"id\":\"type.mismatch\"") {
                "type.mismatch"
            } else {
                "unexpected"
            }
        })
        .collect()
}

pub(super) fn project(path: &str, text: &str) -> Project {
    Project {
        root: ".".into(),
        files: vec![SourceFile::new(path, text)],
        manifest: None,
    }
}

pub(super) struct TempProject {
    root: PathBuf,
}

impl TempProject {
    pub(super) fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-analysis-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temporary project root should be created");
        Self { root }
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("temporary project parent should be created");
        }
        fs::write(path, contents).expect("temporary project file should be written");
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(super) fn diagnostic(
    parser_context: &'static str,
    expected: Vec<&'static str>,
) -> ParseDiagnostic {
    ParseDiagnostic {
        id: "parse.invalid",
        message: "invalid syntax".to_string(),
        span: None,
        parser_context,
        unexpected: UnexpectedToken {
            kind: "Invalid".to_string(),
            text: "?".to_string(),
        },
        expected,
        recovery: Recovery {
            strategy: RecoveryStrategy::None,
            anchor: None,
            dropped_token_count: 0,
        },
        repair_candidates: Vec::new(),
    }
}

pub(super) fn span(
    file: &str,
    start_line: usize,
    start_column: usize,
    start_offset: usize,
    end_line: usize,
    end_column: usize,
    end_offset: usize,
) -> SourceSpan {
    SourceSpan {
        file: SourcePath::new(file),
        start: LineCol {
            line: start_line,
            column: start_column,
            offset: start_offset,
        },
        end: LineCol {
            line: end_line,
            column: end_column,
            offset: end_offset,
        },
    }
}
