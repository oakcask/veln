use super::*;

#[test]
fn selected_manifest_export_is_accepted() {
    let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
    let project = Project {
        root: ".".into(),
        files: vec![source],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: vec![ManifestExport {
                    path: "src/main.veln".to_string(),
                    path_span: span("veln.toml", 2, 13, 26),
                }],
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };

    let (module, diagnostics) = load_surface_module(&project);

    assert_eq!(module.module.as_ref().unwrap().name, "src::main");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "manifest.invalid_export"),
        "{diagnostics:#?}"
    );
}

#[test]
fn selected_manifest_export_reports_source_path_casing_diagnostic() {
    let source = SourceFile::new("App/_net.veln", "pub fn value() -> Int\n  1\nend\n");
    let project = Project {
        root: ".".into(),
        files: vec![source],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: vec![ManifestExport {
                    path: "App/_net.veln".to_string(),
                    path_span: span("veln.toml", 2, 13, 26),
                }],
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };

    let diagnostics = validate_manifest_exports(&project);

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "name.invalid_case"]
    );
    assert_eq!(
        diagnostics[0].span.as_ref().unwrap().file.as_str(),
        "App/_net.veln"
    );
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.offset, 0);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().end.offset, 0);
    assert_eq!(
        detail_string(&diagnostics[0], "source_kind"),
        Some("export")
    );
    assert_eq!(detail_string(&diagnostics[0], "segment"), Some("App"));
    assert_eq!(
        detail_string(&diagnostics[0], "observed_initial"),
        Some("ascii_uppercase")
    );
    assert_eq!(detail_number(&diagnostics[0], "segment_index"), Some(0));
    assert_eq!(
        detail_string(&diagnostics[1], "source_kind"),
        Some("export")
    );
    assert_eq!(detail_string(&diagnostics[1], "segment"), Some("_net"));
    assert_eq!(
        detail_string(&diagnostics[1], "observed_initial"),
        Some("underscore")
    );
    assert_eq!(detail_number(&diagnostics[1], "segment_index"), Some(1));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "manifest.invalid_export"),
        "{diagnostics:#?}"
    );
}

#[test]
fn generated_sources_use_origin_metadata_for_visible_modules() {
    let generated = SourceFile::generated(
        "target/bookkeeping.veln",
        "pub fn value() -> Int\n  1\nend\n",
        Some(SourcePath::new("src/generated_api.veln")),
    );
    let source_less = SourceFile::generated(
        "target/source_less.veln",
        "pub fn hidden() -> Int\n  1\nend\n",
        None::<SourcePath>,
    );
    let project = Project {
        root: ".".into(),
        files: vec![generated, source_less],
        manifest: None,
    };

    let (module, diagnostics) = load_surface_module(&project);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(
        module.functions.iter().any(|function| {
            function.module_name.as_deref() == Some("src::generated_api")
                && function.name.as_deref() == Some("value")
        }),
        "{module:#?}"
    );
    assert!(
        module
            .functions
            .iter()
            .all(|function| function.name.as_deref() != Some("hidden")
                || function.module_name.is_none()),
        "{module:#?}"
    );
}

#[test]
fn generated_manifest_export_uses_origin_metadata_before_synthetic_path() {
    let generated = SourceFile::generated(
        "Target/_bookkeeping.veln",
        "pub fn value() -> Int\n  1\nend\n",
        Some(SourcePath::new("src/generated_api.veln")),
    );
    let project = Project {
        root: ".".into(),
        files: vec![generated],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: vec![ManifestExport {
                    path: "Target/_bookkeeping.veln".to_string(),
                    path_span: span("veln.toml", 2, 13, 37),
                }],
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };

    let (module, diagnostics) = load_surface_module(&project);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(
        module.functions.iter().any(|function| {
            function.module_name.as_deref() == Some("src::generated_api")
                && function.name.as_deref() == Some("value")
        }),
        "{module:#?}"
    );
}

#[test]
fn generated_manifest_export_rejects_invalid_origin_before_valid_synthetic_path() {
    let generated = SourceFile::generated(
        "target/bookkeeping.veln",
        "pub fn value() -> Int\n  1\nend\n",
        Some(SourcePath::new("App/generated_api.veln")),
    );
    let project = Project {
        root: ".".into(),
        files: vec![generated],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: vec![ManifestExport {
                    path: "target/bookkeeping.veln".to_string(),
                    path_span: span("veln.toml", 2, 13, 36),
                }],
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };

    let (module, diagnostics) = load_surface_module(&project);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.invalid_case");
    assert_eq!(
        detail_string(&diagnostics[0], "source_kind"),
        Some("generated")
    );
    assert_eq!(detail_string(&diagnostics[0], "segment"), Some("App"));
    assert_eq!(detail_number(&diagnostics[0], "segment_index"), Some(0));
    assert!(
        !module.functions.iter().any(|function| {
            function.module_name.as_deref() == Some("target::bookkeeping")
                || function.module_name.as_deref() == Some("App::generated_api")
        }),
        "{module:#?}"
    );
}

#[test]
fn generated_dependency_export_is_visible_by_origin_metadata() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "use src::generated_api from \"github.com/oakcask/generated\"\n",
                "\n",
                "pub fn main() -> Int\n",
                "  generated_api::value()\n",
                "end\n",
            ),
        )],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: Vec::new(),
            },
            dependencies: vec![veln_project::ManifestDependency {
                package: "github.com/oakcask/generated".to_string(),
                package_span: span("veln.toml", 1, 15, 43),
                git: None,
                path: Some(ManifestField {
                    key: "path".to_string(),
                    value: "vendor/generated".to_string(),
                    key_span: span("veln.toml", 2, 1, 5),
                    value_span: span("veln.toml", 2, 8, 24),
                }),
                vendor: None,
                mirror: None,
                subdir: None,
                selectors: Vec::new(),
            }],
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };
    let dependency_project = Project {
        root: "vendor/generated".into(),
        files: vec![SourceFile::generated(
            "Target/_bookkeeping.veln",
            "pub fn value() -> Int\n  1\nend\n",
            Some(SourcePath::new("src/generated_api.veln")),
        )],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: veln_project::ManifestPackage {
                fields: vec![ManifestField {
                    key: "name".to_string(),
                    value: "github.com/oakcask/generated".to_string(),
                    key_span: span("veln.toml", 2, 1, 5),
                    value_span: span("veln.toml", 2, 8, 36),
                }],
            },
            lib: ManifestLib {
                exports: vec![ManifestExport {
                    path: "Target/_bookkeeping.veln".to_string(),
                    path_span: span("veln.toml", 5, 13, 37),
                }],
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };
    let dependencies = [CapturedDependencyProject {
        package: "github.com/oakcask/generated".to_string(),
        source: "vendor/generated".to_string(),
        project: Some(dependency_project),
    }];

    let (modules, diagnostics) =
        load_surface_modules_with_captured_dependencies(&project, &dependencies);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(
        modules.application.functions.iter().any(|function| {
            function.module_name.as_deref()
                == Some("github.com/oakcask/generated::src::generated_api")
                && function.name.as_deref() == Some("value")
        }),
        "{:#?}",
        modules.application
    );
}

#[test]
fn selected_manifest_export_with_parse_errors_is_still_selected() {
    let source = SourceFile::new("main.veln", "fn main() -> ()\n");
    let project = Project {
        root: ".".into(),
        files: vec![source],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: vec![ManifestExport {
                    path: "main.veln".to_string(),
                    path_span: span("veln.toml", 2, 13, 22),
                }],
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };

    let (_, diagnostics) = load_surface_module(&project);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "manifest.unselected_export"),
        "{diagnostics:#?}"
    );
}

#[test]
fn manifest_export_validation_preserves_manifest_order_and_first_duplicate_origin() {
    let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
    let project = Project {
        root: ".".into(),
        files: vec![source],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: vec![
                    ManifestExport {
                        path: "../outside.veln".to_string(),
                        path_span: span("veln.toml", 2, 4, 21),
                    },
                    ManifestExport {
                        path: "missing.veln".to_string(),
                        path_span: span("veln.toml", 3, 4, 18),
                    },
                    ManifestExport {
                        path: "src/main.veln".to_string(),
                        path_span: span("veln.toml", 4, 4, 19),
                    },
                    ManifestExport {
                        path: "./src/main.veln".to_string(),
                        path_span: span("veln.toml", 5, 4, 21),
                    },
                ],
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };

    let diagnostics = validate_manifest_exports(&project);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "manifest.invalid_export",
            "manifest.missing_export",
            "manifest.duplicate_export",
        ]
    );
    assert_eq!(
        diagnostics[2].message,
        "manifest export `./src/main.veln` duplicates module export `src::main`"
    );
    assert_eq!(diagnostics[2].related.len(), 1);
}

#[test]
fn companion_manifest_export_reports_boundary_before_selection_checks() {
    let root = env::temp_dir().join(format!(
        "veln-surface-companion-export-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("test root should be created");
    fs::write(root.join("math.test.veln"), "test companion() -> ()\nend\n")
        .expect("companion source should be written");
    let source = SourceFile::new("math.veln", "pub fn value() -> Int\n  1\nend\n");
    let project = Project {
        root: root.clone(),
        files: vec![source],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: vec![
                    ManifestExport {
                        path: "math.test.veln".to_string(),
                        path_span: span("veln.toml", 3, 4, 20),
                    },
                    ManifestExport {
                        path: "missing.test.veln".to_string(),
                        path_span: span("veln.toml", 4, 4, 23),
                    },
                ],
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };

    let (_, diagnostics) = load_surface_module(&project);
    let _ = fs::remove_dir_all(&root);

    let invalid_exports = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "manifest.invalid_export")
        .collect::<Vec<_>>();
    assert_eq!(invalid_exports.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "manifest.unselected_export"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "manifest.missing_export"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        invalid_exports[0].message,
        "manifest export `math.test.veln` is invalid: export names a test companion"
    );
    assert_eq!(
        detail_string(invalid_exports[0], "field"),
        Some("lib.exports")
    );
    assert_eq!(
        detail_string(invalid_exports[0], "source_path"),
        Some("math.test.veln")
    );
    assert_eq!(
        detail_string(invalid_exports[0], "companion_path"),
        Some("math.test.veln")
    );
    assert_eq!(
        detail_string(invalid_exports[0], "reason"),
        Some("test_companion")
    );
}

#[test]
fn unselected_manifest_export_reports_diagnostic() {
    let root = env::temp_dir().join(format!(
        "veln-surface-unselected-export-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("test root should be created");
    fs::write(root.join("src/other.veln"), "fn other() -> ()\n  ()\nend\n")
        .expect("unselected source should be written");
    let source = SourceFile::new("src/main.veln", "fn main() -> ()\n  ()\nend\n");
    let project = Project {
        root: root.clone(),
        files: vec![source],
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: Default::default(),
            lib: ManifestLib {
                exports: vec![ManifestExport {
                    path: "src/other.veln".to_string(),
                    path_span: span("veln.toml", 2, 13, 27),
                }],
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };

    let (_, diagnostics) = load_surface_module(&project);
    let _ = fs::remove_dir_all(&root);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "manifest.unselected_export");
    assert_eq!(
        diagnostics[0].message,
        "manifest export `src/other.veln` has no matching selected source file"
    );
}
