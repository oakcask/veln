use std::collections::BTreeMap;

use veln_ast::{FunctionKind, SurfaceModule};
use veln_diagnostics::Diagnostic;
use veln_project::Project;
use veln_sema::{LoweredSurfaceModule, analyze_surface_module, lower_checked_surface_module};
use veln_source::SourceSpan;
use veln_test::{DoctestExpectation, doctest_sources, reconcile_expected_doctest_failures};

use crate::surface::{load_surface_module, reachable_entry_module};

pub(crate) enum DoctestMode {
    Include,
    Exclude,
}

pub(crate) struct ProjectAnalysis {
    pub(crate) project: Project,
    pub(crate) module: SurfaceModule,
    pub(crate) doctest_expectations: BTreeMap<String, DoctestExpectation>,
    source_diagnostics: Vec<Diagnostic>,
    expected_doctest_failures: BTreeMap<String, SourceSpan>,
}

pub(crate) struct ReachableEntryAnalysis {
    pub(crate) module: SurfaceModule,
    pub(crate) lowered: LoweredSurfaceModule,
}

pub(crate) fn analyze_project(mut project: Project, doctest_mode: DoctestMode) -> ProjectAnalysis {
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

    ProjectAnalysis {
        project,
        module,
        doctest_expectations,
        source_diagnostics,
        expected_doctest_failures,
    }
}

impl ProjectAnalysis {
    pub(crate) fn source_diagnostics(&self) -> Vec<Diagnostic> {
        self.reconcile_doctest_failures(self.source_diagnostics.clone())
    }

    pub(crate) fn semantic_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.source_diagnostics.clone();
        diagnostics.extend(analyze_surface_module(&self.module));
        self.reconcile_doctest_failures(diagnostics)
    }

    pub(crate) fn checked_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.source_diagnostics.clone();
        diagnostics.extend(lower_checked_surface_module(&self.module).diagnostics);
        self.reconcile_doctest_failures(diagnostics)
    }

    pub(crate) fn lower_reachable_entry(
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
