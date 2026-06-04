use std::collections::BTreeMap;

use veln_ast::{FunctionKind, SurfaceModule};
use veln_diagnostics::Diagnostic;
use veln_project::Project;
use veln_sema::{
    LoweredSurfaceModule, analyze_surface_module, lower_analyzed_surface_module,
    lower_checked_surface_module,
};
use veln_source::SourceSpan;
use veln_test::{DoctestExpectation, doctest_sources, reconcile_expected_doctest_failures};

use crate::surface::{load_surface_module, reachable_entry_module};

pub enum DoctestMode {
    Include,
    Exclude,
}

pub struct ProjectAnalysis {
    pub project: Project,
    pub module: SurfaceModule,
    pub doctest_expectations: BTreeMap<String, DoctestExpectation>,
    source_diagnostics: Vec<Diagnostic>,
    semantic_diagnostics: Vec<Diagnostic>,
    checked: LoweredSurfaceModule,
    expected_doctest_failures: BTreeMap<String, SourceSpan>,
}

pub struct ReachableEntryAnalysis {
    pub module: SurfaceModule,
    pub lowered: LoweredSurfaceModule,
}

pub fn analyze_project(mut project: Project, doctest_mode: DoctestMode) -> ProjectAnalysis {
    let doctests = match doctest_mode {
        DoctestMode::Include => Some(doctest_sources(&project.files)),
        DoctestMode::Exclude => None,
    };
    let mut source_diagnostics = Vec::new();
    let mut doctest_expectations = BTreeMap::new();
    let mut expected_doctest_failures = BTreeMap::new();

    if let Some(doctests) = doctests {
        source_diagnostics.extend(doctests.diagnostics);
        project.files.extend(doctests.sources);
        doctest_expectations = doctests.expectations;
        expected_doctest_failures = doctests.expected_failures;
    }

    let (module, parse_diagnostics) = load_surface_module(&project);
    source_diagnostics.extend(parse_diagnostics);
    let semantic_diagnostics = analyze_surface_module(&module);
    let checked = lower_analyzed_surface_module(&module, semantic_diagnostics.clone());

    ProjectAnalysis {
        project,
        module,
        doctest_expectations,
        source_diagnostics,
        semantic_diagnostics,
        checked,
        expected_doctest_failures,
    }
}

pub fn checked_project_diagnostics(project: Project, doctest_mode: DoctestMode) -> Vec<Diagnostic> {
    analyze_project(project, doctest_mode).checked_diagnostics()
}

impl ProjectAnalysis {
    pub fn source_diagnostics(&self) -> Vec<Diagnostic> {
        self.reconcile_doctest_failures(self.source_diagnostics.clone())
    }

    pub fn semantic_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.source_diagnostics.clone();
        diagnostics.extend(self.semantic_diagnostics.clone());
        self.reconcile_doctest_failures(diagnostics)
    }

    pub fn checked_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.source_diagnostics.clone();
        diagnostics.extend(self.checked.diagnostics.clone());
        self.reconcile_doctest_failures(diagnostics)
    }

    pub fn lower_reachable_entry(
        &self,
        entry: &str,
        entry_kind: FunctionKind,
    ) -> ReachableEntryAnalysis {
        let module = reachable_entry_module(&self.module, entry, entry_kind);
        let lowered = lower_checked_surface_module(&module);
        ReachableEntryAnalysis { module, lowered }
    }

    fn reconcile_doctest_failures(&self, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        reconcile_expected_doctest_failures(diagnostics, &self.expected_doctest_failures)
    }
}
