use super::*;

#[test]
fn modules_manifest_section_is_rejected() {
    let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
    let project = Project {
        root: ".".into(),
        files: vec![source],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: Vec::new(),
            },
            dependencies: Vec::new(),
            unsupported_sections: vec![ManifestUnsupportedSection {
                name: "modules".to_string(),
                span: span("veln.toml", 1, 2, 9),
            }],
            tools: Vec::new(),
        }),
    };

    let (module, diagnostics) = load_surface_module(&project);

    assert_eq!(module.module.as_ref().unwrap().name, "src::main");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "manifest.unsupported_section");
    assert_eq!(
        diagnostics[0].message,
        "`[modules]` is not supported; use `[lib].exports` for public source files"
    );
}

#[test]
fn source_mod_declaration_reports_module_diagnostic() {
    let source = SourceFile::new(
        "src/main.veln",
        "mod app.main\nfn main() -> ()\n  ()\nend\n",
    );
    let project = Project {
        root: ".".into(),
        files: vec![source],
        manifest: None,
    };

    let (_, diagnostics) = load_surface_module(&project);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "module.source_mod");
    assert_eq!(
        diagnostics[0].message,
        "source `mod` declarations are not supported"
    );
}

#[test]
fn invalid_source_path_casing_reports_all_segments_without_registering_module() {
    let source = SourceFile::new("app.veln", "use App::_net\nfn main() -> ()\n  ()\nend\n");
    let invalid = SourceFile::new("App/_net.veln", "pub fn value() -> Int\n  1\nend\n");
    let project = Project {
        root: ".".into(),
        files: vec![source, invalid],
        manifest: None,
    };

    let (_, diagnostics) = load_surface_module(&project);

    let invalid_cases = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();
    assert_eq!(invalid_cases.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        invalid_cases
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        [
            "module name `App` must start with an ASCII lowercase letter",
            "module name `_net` must start with an ASCII lowercase letter",
        ]
    );
    assert_eq!(
        detail_string(invalid_cases[0], "origin"),
        Some("source_path")
    );
    assert_eq!(
        detail_string(invalid_cases[0], "source_kind"),
        Some("regular")
    );
    assert_eq!(
        detail_string(invalid_cases[0], "source_path"),
        Some("App/_net.veln")
    );
    assert_eq!(detail_string(invalid_cases[0], "segment"), Some("App"));
    assert_eq!(
        detail_string(invalid_cases[0], "observed_initial"),
        Some("ascii_uppercase")
    );
    assert_eq!(detail_number(invalid_cases[0], "segment_index"), Some(0));
    assert_eq!(detail_string(invalid_cases[1], "segment"), Some("_net"));
    assert_eq!(
        detail_string(invalid_cases[1], "observed_initial"),
        Some("underscore")
    );
    assert_eq!(detail_number(invalid_cases[1], "segment_index"), Some(1));
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "module.unresolved_import"
                && diagnostic.message
                    == "local import `App::_net` has no matching selected source file"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "module.duplicate_source_path"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_source_path_identity_does_not_satisfy_import_but_valid_modules_still_analyze() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "use App\n",
                    "\n",
                    "fn main() -> Int\n",
                    "  App::entry()\n",
                    "end\n",
                ),
            ),
            SourceFile::new("App.veln", "pub fn entry() -> Int\n  1\nend\n"),
            SourceFile::new("probe.veln", "fn probe() -> Int\n  \"kept\"\nend\n"),
        ],
        manifest: None,
    };

    let (_, source_diagnostics) = load_surface_module(&project);
    let diagnostics = analyze_project(project, DoctestMode::Exclude).checked_diagnostics();

    assert!(
        source_diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "module.unresolved_import"
                && diagnostic.message == "local import `App` has no matching selected source file"
        }),
        "{source_diagnostics:#?}"
    );
    assert!(
        source_diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                && diagnostic.message
                    == "module name `App` must start with an ASCII lowercase letter"
                && detail_string(diagnostic, "origin") == Some("source_path")
        }),
        "{source_diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.file.as_str() == "probe.veln")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_source_path_identity_does_not_collide_with_duplicate_modules() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new("app.veln", "fn main() -> Int\n  1\nend\n"),
            SourceFile::new("App.veln", "pub fn upper() -> Int\n  1\nend\n"),
            SourceFile::new("app.test.veln", "test companion() -> ()\n  ()\nend\n"),
            SourceFile::new("probe.veln", "fn probe() -> Int\n  \"kept\"\nend\n"),
        ],
        manifest: None,
    };

    let (_, source_diagnostics) = load_surface_module(&project);
    let diagnostics = analyze_project(project, DoctestMode::Exclude).checked_diagnostics();

    assert!(
        source_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"
                && detail_string(diagnostic, "source_path") == Some("App.veln")),
        "{source_diagnostics:#?}"
    );
    assert!(
        source_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "module.duplicate_source_path"),
        "{source_diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.file.as_str() == "probe.veln")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_source_path_identity_is_absent_from_registration_boundary() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new("App.veln", "pub fn first() -> Int\n  1\nend\n"),
            SourceFile::new("App.veln", "pub fn second() -> Int\n  2\nend\n"),
            SourceFile::new("probe.veln", "fn probe() -> Int\n  \"kept\"\nend\n"),
        ],
        manifest: None,
    };
    let mut diagnostics = Vec::new();
    let mut parts = SurfaceParts::new();

    load_project_sources(&project, &mut diagnostics, &mut parts, None, None, None);

    assert_eq!(
        parts
            .derived_modules
            .iter()
            .map(|(module, _)| module.as_str())
            .collect::<Vec<_>>(),
        ["probe"],
        "{:#?}",
        parts.derived_modules
    );
    assert!(
        parts.rejected_derived_modules.contains("App"),
        "{:#?}",
        parts.rejected_derived_modules
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "module.duplicate_source_path"),
        "{diagnostics:#?}"
    );
}

#[test]
fn lowercase_parse_failure_does_not_add_single_segment_unresolved_import() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!("use helper\n", "\n", "fn main() -> ()\n", "  ()\n", "end\n"),
            ),
            SourceFile::new("helper.veln", "pub fn value() -> Int\n  1\n"),
        ],
        manifest: None,
    };

    let (_, diagnostics) = load_surface_module(&project);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.expected_end"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "module.unresolved_import"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_source_path_identity_does_not_add_reachability_cycle_edge() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "use helper\n",
                    "\n",
                    "fn main() -> Int\n",
                    "  helper::entry()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "helper.veln",
                concat!(
                    "use App\n",
                    "\n",
                    "pub fn entry() -> Int\n",
                    "  App::back()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "App.veln",
                "use app\n\npub fn back() -> Int\n  app::main()\nend\n",
            ),
            SourceFile::new("probe.veln", "fn probe() -> Int\n  \"kept\"\nend\n"),
        ],
        manifest: None,
    };
    let (module, source_diagnostics) = load_surface_module(&project);
    let diagnostics = analyze_project(project, DoctestMode::Exclude).checked_diagnostics();

    assert!(
        source_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"
                && detail_string(diagnostic, "source_path") == Some("App.veln")),
        "{source_diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.file.as_str() == "probe.veln")
        }),
        "{diagnostics:#?}"
    );

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(
        reachable_function_names(&reachable),
        vec![("app", "main"), ("helper", "entry")]
    );
    assert!(
        reachable
            .uses
            .iter()
            .all(|use_decl| use_decl.module_name.as_deref() != Some("App")),
        "{:#?}",
        reachable.uses
    );
}

#[test]
fn source_path_casing_is_reported_when_source_parsing_fails() {
    let regular = SourceFile::new("App/broken.veln", "fn main() -> ()\n");
    let companion = SourceFile::new("Net/math.test.veln", "test broken() -> ()\n");
    let project = Project {
        root: ".".into(),
        files: vec![regular, companion],
        manifest: None,
    };

    let (module, diagnostics) = load_surface_module(&project);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id.starts_with("parse.")),
        "{diagnostics:#?}"
    );
    let invalid_cases = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();
    assert_eq!(invalid_cases.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        invalid_cases
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        [
            "module name `App` must start with an ASCII lowercase letter",
            "module name `Net` must start with an ASCII lowercase letter",
        ]
    );
    assert_eq!(
        detail_string(invalid_cases[0], "source_kind"),
        Some("regular")
    );
    assert_eq!(
        detail_string(invalid_cases[0], "source_path"),
        Some("App/broken.veln")
    );
    assert_eq!(
        detail_string(invalid_cases[1], "source_kind"),
        Some("companion")
    );
    assert_eq!(
        detail_string(invalid_cases[1], "source_path"),
        Some("Net/math.test.veln")
    );
    assert!(
        module
            .functions
            .iter()
            .all(
                |function| function.module_name.as_deref() != Some("App::broken")
                    && function.module_name.as_deref() != Some("Net::math__test_companion")
            ),
        "{module:#?}"
    );
}

#[test]
fn chained_companion_reports_structure_without_module_identity() {
    let source = SourceFile::new("App/_math.test.test.veln", "fn helper() -> Int\n  1\nend\n");
    let project = Project {
        root: ".".into(),
        files: vec![source],
        manifest: None,
    };

    let (module, diagnostics) = load_surface_module(&project);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["module.chained_companion"],
        "{diagnostics:#?}"
    );
    assert!(
        module
            .functions
            .iter()
            .filter(|function| function.span.file.as_str() == "App/_math.test.test.veln")
            .all(|function| function.module_name.is_none()),
        "{module:#?}"
    );
}

#[test]
fn source_path_structural_error_after_valid_initial_is_not_casing() {
    let source = SourceFile::new("appé.veln", "fn main() -> ()\n  ()\nend\n");
    let project = Project {
        root: ".".into(),
        files: vec![source],
        manifest: None,
    };

    let (_, diagnostics) = load_surface_module(&project);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["module.invalid_source_path"],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[0].message,
        "source path segment cannot be used as a module identifier: `appé`"
    );
    assert_eq!(detail_string(&diagnostics[0], "segment"), Some("appé"));
}
