use super::*;

#[test]
fn private_standard_byte_module_cannot_be_imported() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "use bytes from \"std\"\n",
                "pub fn main() -> Int\n",
                "  0\n",
                "end\n",
            ),
        )],
        manifest: None,
    };

    let (_, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "module.unexported_import" && diagnostic.message.contains("bytes")
    }));
}

#[test]
fn private_standard_diagnostic_module_cannot_be_imported() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "use diagnostic from \"std\"\n",
                "pub fn main() -> Int\n",
                "  0\n",
                "end\n",
            ),
        )],
        manifest: None,
    };

    let (_, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "module.unexported_import" && diagnostic.message.contains("diagnostic")
    }));
}

#[test]
fn toolchain_standard_project_is_not_loaded_twice() {
    let (module, diagnostics, runtime_standard_parse_lowers, expected_runtime_sources) =
        loaded_toolchain_standard_fixture();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert_eq!(runtime_standard_parse_lowers, expected_runtime_sources);
    assert!(
        module.functions.iter().any(|function| {
            function.module_name.as_deref() == Some("std::http2::core")
                && function.name.as_deref() == Some("client_stream_id")
        }),
        "functions: {:#?}",
        module
            .functions
            .iter()
            .map(|function| (function.module_name.as_deref(), function.name.as_deref()))
            .collect::<Vec<_>>()
    );
    assert!(
        module.uses.iter().any(|use_decl| {
            use_decl.module_name.as_deref() == Some("std::http2::core_test")
                && use_decl.name == "std::http2::core"
        }),
        "uses: {:#?}",
        module.uses
    );
    assert_eq!(
        module
            .functions
            .iter()
            .filter(|function| {
                function.module_name.as_deref() == Some("std::prelude")
                    && function.name.as_deref() == Some("vec_len")
            })
            .count(),
        1
    );
}

#[test]
fn toolchain_standard_project_allows_extra_companion_source() {
    let bundle = veln_stdlib::package_bundle();
    let mut files = bundle
        .files
        .iter()
        .map(|file| SourceFile::new(file.path, file.text))
        .collect::<Vec<_>>();
    files.push(SourceFile::new(
        "prelude.test.veln",
        "test companion() -> ()\nend\n",
    ));
    let project = Project {
        root: ".".into(),
        files,
        manifest: Some(ProjectManifest {
            path: SourcePath::new("veln.toml"),
            source_bytes: Vec::new(),
            package: veln_project::ManifestPackage {
                fields: vec![ManifestField {
                    key: "name".to_string(),
                    value: veln_stdlib::PACKAGE_NAME.to_string(),
                    key_span: span("veln.toml", 2, 1, 5),
                    value_span: span("veln.toml", 2, 8, 13),
                }],
            },
            lib: ManifestLib {
                exports: bundle
                    .exports
                    .iter()
                    .map(|export| ManifestExport {
                        path: (*export).to_string(),
                        path_span: span("veln.toml", 4, 1, 1 + export.len()),
                    })
                    .collect(),
            },
            dependencies: Vec::new(),
            unsupported_sections: Vec::new(),
            tools: Vec::new(),
        }),
    };

    assert!(super::is_toolchain_standard_project(&project));
}

#[test]
fn standard_http2_tests_load_with_private_imports() {
    let (module, diagnostics, _, _) = loaded_toolchain_standard_fixture();

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    for entry in [
        "receive_frame_dispatch_decodes_headers_with_production_hpack",
        "outbound_request_headers_send_emits_hpack_bytes_and_creates_stream",
        "output_buffer_preserves_successful_send_order",
        "goaway_send_emits_exact_bytes_and_updates_shutdown_immutably",
    ] {
        assert!(
            module.functions.iter().any(|function| {
                function.module_name.as_deref() == Some("std::http2::core_test")
                    && function.name.as_deref() == Some(entry)
                    && function.kind == FunctionKind::Test
            }),
            "{entry} should load from the standard HTTP/2 core test module"
        );
    }
}

#[test]
fn standard_project_with_manifest_additions_is_reserved_user_package() {
    let mut project = toolchain_standard_project(Vec::new());
    project
        .manifest
        .as_mut()
        .expect("standard project manifest")
        .tools
        .push(ManifestTool {
            name: "extra".to_string(),
            fields: Vec::new(),
        });

    let toolchain_std = super::is_toolchain_standard_project(&project);
    assert!(!toolchain_std);
    let diagnostics = super::validate_reserved_standard_package(&project, toolchain_std);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "manifest.reserved_standard_package"
            && diagnostic.message == "package name `std` is reserved by the Veln toolchain"
    }));
}

fn loaded_toolchain_standard_fixture() -> &'static (SurfaceModule, Vec<Diagnostic>, usize, usize) {
    static FIXTURE: OnceLock<(SurfaceModule, Vec<Diagnostic>, usize, usize)> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../veln-stdlib/veln");
        let core_test = fs::read_to_string(root.join("http2/core_test.veln"))
            .expect("standard HTTP/2 core test source should load");
        let project =
            toolchain_standard_project(vec![SourceFile::new("http2/core_test.veln", core_test)]);
        let expected_runtime_sources = project
            .files
            .iter()
            .filter(|source| source.path().as_str().ends_with("_test.veln"))
            .count();
        let ((module, diagnostics), work) =
            embedded_standard_counters::observe(|| load_surface_module(&project));
        (
            module,
            diagnostics,
            work.runtime_standard_parse_lowers,
            expected_runtime_sources,
        )
    })
}

fn toolchain_standard_project(additional_files: Vec<SourceFile>) -> Project {
    let bundle = veln_stdlib::package_bundle();
    let mut files = bundle
        .files
        .iter()
        .map(|file| SourceFile::new(file.path, file.text))
        .collect::<Vec<_>>();
    files.extend(additional_files);
    Project {
        root: ".".into(),
        files,
        manifest: Some(parse_manifest_text("veln.toml", bundle.manifest)),
    }
}

#[test]
fn project_standard_calls_lower_through_mangled_veln_functions() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            "pub fn main() -> Int\n  vec_len([1])\nend\n",
        )],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let lowered = veln_sema::lower_project_reachable_surface_module(&reachable);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("project should lower to core");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main function");
    assert!(matches!(
        &main.body[0].kind,
        veln_core::CoreStmtKind::Return { expr }
            if matches!(
                &expr.kind,
                veln_core::CoreExprKind::Call {
                    target: veln_core::CoreCallTarget::Function(name),
                    ..
                } if name == "__veln_std$prelude$vec_len"
            )
    ));
    let std_vec_len = core
        .functions
        .iter()
        .find(|function| function.name == "__veln_std$prelude$vec_len")
        .expect("reachable std vec_len body");
    assert!(matches!(
        &std_vec_len.body[0].kind,
        veln_core::CoreStmtKind::Return { expr }
            if matches!(
                &expr.kind,
                veln_core::CoreExprKind::Call {
                    target: veln_core::CoreCallTarget::PreludeBuiltin(name),
                    ..
                } if name == "vec_len"
            )
    ));
}
