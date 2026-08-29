use super::*;

#[test]
fn project_loading_injects_origin_tagged_standard_prelude_imports() {
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
    assert!(module.functions.iter().any(|function| {
        function.module_name.as_deref() == Some("std::prelude")
            && function.name.as_deref() == Some("vec_len")
    }));
    assert!(module.uses.iter().any(|use_decl| {
        use_decl.module_name.as_deref() == Some("main")
            && use_decl.name == "std::prelude"
            && use_decl.origin == UseOrigin::ImplicitStandardPrelude
    }));
    assert!(!module.uses.iter().any(|use_decl| {
        use_decl.module_name.as_deref() == Some("std::prelude")
            && use_decl.origin == UseOrigin::ImplicitStandardPrelude
    }));
    assert!(!module.uses.iter().any(|use_decl| {
        use_decl
            .module_name
            .as_deref()
            .is_some_and(|module_name| module_name.starts_with("std::"))
            && use_decl.origin == UseOrigin::ImplicitStandardPrelude
    }));
}

#[test]
fn ordinary_project_does_not_load_http2_modules() {
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
    assert!(module.functions.iter().all(|function| {
        !function
            .module_name
            .as_deref()
            .is_some_and(|module_name| module_name.starts_with("std::http2::"))
    }));
}

#[derive(Debug, PartialEq, Eq)]
struct StandardInitializationWork {
    loaded_modules: Vec<String>,
    materialized_modules: usize,
    materialized_lowered_bytes: usize,
    prepared_declarations: usize,
}

fn load_synthetic_standard(unrelated_count: usize) -> StandardInitializationWork {
    let standard = synthetic_standard_package(unrelated_count);
    let mut diagnostics = Vec::new();
    let mut parts = SurfaceParts::new();
    load_project_sources(
        &single_file_project("pub fn main() -> Int\n  1\nend\n"),
        &mut diagnostics,
        &mut parts,
        None,
        None,
        None,
    );
    let ((), standard_work) = embedded_standard_counters::observe(|| {
        load_embedded_standard_package_from(&standard, &mut diagnostics, &mut parts, true);
    });
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    check_standard_surface_module(&parts.module);
    standard_initialization_work(
        &standard,
        &parts.module,
        standard_work.materialized_lowered_bytes,
    )
}

fn single_file_project(text: &str) -> Project {
    Project {
        root: ".".into(),
        files: vec![SourceFile::new("main.veln", text)],
        manifest: None,
    }
}

fn synthetic_standard_package(unrelated_count: usize) -> EmbeddedStandardPackage {
    let modules = synthetic_standard_sources(unrelated_count)
        .into_iter()
        .map(|(path, text)| {
            (
                standard_module_name(&path),
                embedded_standard_entry(path, text),
            )
        })
        .collect();
    EmbeddedStandardPackage { modules }
}

fn synthetic_standard_sources(unrelated_count: usize) -> [(String, String); 3] {
    [
        (
            "prelude.veln".to_string(),
            concat!(
                "pub type PreludePayload\n",
                "  PreludePayload(Int)\n",
                "end\n",
                "\n",
                "pub fn prelude_answer(value: Int) -> Int\n",
                "  value\n",
                "end\n",
            )
            .to_string(),
        ),
        (
            "extra.veln".to_string(),
            concat!(
                "pub fn extra_answer(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
            )
            .to_string(),
        ),
        (
            "unrelated.veln".to_string(),
            unrelated_annotated_standard_module(unrelated_count),
        ),
    ]
}

fn embedded_standard_entry(path: String, text: String) -> EmbeddedStandardModuleEntry {
    EmbeddedStandardModuleEntry {
        lowered: std::borrow::Cow::Owned(lowered_standard_module_bytes(&path, &text)),
        path,
        module: std::sync::OnceLock::new(),
    }
}

fn lowered_standard_module_bytes(path: &str, text: &str) -> Vec<u8> {
    let source = SourceFile::new(path, text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut lowered = veln_ast::lower_surface_ast_with_module_identity(
        &parsed.tree,
        standard_module_name(path),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    for use_decl in &mut lowered.uses {
        let imported = use_decl.name.clone();
        use_decl.name = format!("std::{imported}");
    }
    veln_ast::encode_surface_module(&lowered)
}

fn standard_module_name(path: &str) -> String {
    format!("std::{}", path.trim_end_matches(".veln").replace('/', "::"))
}

fn unrelated_annotated_standard_module(function_count: usize) -> String {
    let mut text = String::new();
    for index in 0..function_count {
        text.push_str(&format!(
            "pub fn unrelated_{index}(value: Int) -> Int\n  value + {index}\nend\n\n"
        ));
    }
    text
}

fn check_standard_surface_module(module: &SurfaceModule) {
    let reusable = veln_sema::prepare_current_reusable_standard_surface_module_environment(module);
    let (semantic_diagnostics, checked) =
        veln_sema::check_project_surface_module_with_standard_environment(module, &reusable);
    assert!(semantic_diagnostics.is_empty(), "{semantic_diagnostics:#?}");
    assert!(checked.diagnostics.is_empty(), "{:#?}", checked.diagnostics);
}

fn standard_initialization_work(
    standard: &EmbeddedStandardPackage,
    module: &SurfaceModule,
    materialized_lowered_bytes: usize,
) -> StandardInitializationWork {
    StandardInitializationWork {
        loaded_modules: loaded_standard_modules(module),
        materialized_modules: materialized_standard_modules(standard),
        materialized_lowered_bytes,
        prepared_declarations: standard_declaration_count(module),
    }
}

fn loaded_standard_modules(module: &SurfaceModule) -> Vec<String> {
    let mut modules = module
        .functions
        .iter()
        .filter_map(|function| function.module_name.as_deref())
        .filter(|module_name| module_name.starts_with("std::"))
        .chain(
            module
                .types
                .iter()
                .filter_map(|decl| decl.module_name.as_deref())
                .filter(|module_name| module_name.starts_with("std::")),
        )
        .map(str::to_string)
        .collect::<Vec<_>>();
    modules.sort_unstable();
    modules.dedup();
    modules
}

fn materialized_standard_modules(standard: &EmbeddedStandardPackage) -> usize {
    standard
        .modules
        .values()
        .filter(|entry| entry.module.get().is_some())
        .count()
}

fn standard_declaration_count(module: &SurfaceModule) -> usize {
    module
        .functions
        .iter()
        .filter(|decl| is_standard(&decl.module_name))
        .count()
        + module
            .types
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
        + module
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
            .schemas
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
        + module
            .codecs
            .iter()
            .filter(|decl| is_standard(&decl.module_name))
            .count()
}

fn is_standard(module_name: &Option<String>) -> bool {
    module_name
        .as_deref()
        .is_some_and(|module_name| module_name.starts_with("std::"))
}

#[test]
fn standard_package_loading_keeps_initial_analysis_work_constant_for_unrelated_modules() {
    let base = load_synthetic_standard(0);
    let expanded = load_synthetic_standard(128);

    assert_eq!(base.loaded_modules, vec!["std::prelude".to_string()]);
    assert_eq!(expanded, base);
}

#[test]
fn ordinary_project_loads_private_byte_dependency_through_prelude() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "pub fn main() -> Int\n",
                "  let byte: Byte = Byte(42)\n",
                "  let chunk: ByteChunk = ByteChunk([byte])\n",
                "  let offset: ByteOffset = ByteOffset(3)\n",
                "  let count: ByteCount = byte_chunk_count(chunk)\n",
                "  let view: ByteView = ByteView(chunk, offset, count)\n",
                "  match view\n",
                "    ByteView(ByteChunk(_), ByteOffset(start), ByteCount(length)) => start + length\n",
                "  end\n",
                "end\n",
            ),
        )],
        manifest: None,
    };

    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    for name in ["Byte", "ByteChunk", "ByteOffset", "ByteCount", "ByteView"] {
        let owners = module
            .types
            .iter()
            .filter(|type_decl| type_decl.name.as_deref() == Some(name))
            .map(|type_decl| type_decl.module_name.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(owners, [Some("std::bytes")]);

        let aliases = module
            .aliases
            .iter()
            .filter(|alias| {
                alias.module_name.as_deref() == Some("std::prelude")
                    && alias.name.as_deref() == Some(name)
            })
            .collect::<Vec<_>>();
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].target, ["bytes", name]);
    }

    let lowered = veln_sema::lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.is_some(), "byte alias usage should lower");
}

#[test]
fn ordinary_project_loads_private_diagnostic_dependency_through_prelude() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "pub fn main() -> RuntimeDiagnostic\n",
                "  let detail: RuntimeDiagnosticDetail = RuntimeValueDiagnostic(list_nil(), \"reason\")\n",
                "  RuntimeDiagnostic(\"example\", \"message\", detail)\n",
                "end\n",
            ),
        )],
        manifest: None,
    };

    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    for name in [
        "RuntimeDiagnostic",
        "RuntimeDiagnosticDetail",
        "RuntimeDiagnosticFieldPathSegment",
        "RuntimeByteDiagnosticFacts",
        "RuntimeBytePreview",
        "Http2DiagnosticDetail",
        "HpackDiagnosticDetail",
    ] {
        let owners = module
            .types
            .iter()
            .filter(|type_decl| type_decl.name.as_deref() == Some(name))
            .map(|type_decl| type_decl.module_name.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(owners, [Some("std::diagnostic")]);

        let aliases = module
            .aliases
            .iter()
            .filter(|alias| {
                alias.module_name.as_deref() == Some("std::prelude")
                    && alias.name.as_deref() == Some(name)
            })
            .collect::<Vec<_>>();
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].target, ["diagnostic", name]);
    }

    let lowered = veln_sema::lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(
        lowered.core.is_some(),
        "diagnostic alias usage should lower"
    );
}

#[test]
fn explicit_standard_http2_import_loads_only_its_dependency_closure() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "use http2::frame from \"std\"\n",
                "pub fn main(view: ByteView) -> Result<{ length : Int, kind : Int, flags : Int, stream_id : Int, payload : ByteView }, String>\n",
                "  http2::frame::decode(view)\n",
                "end\n",
            ),
        )],
        manifest: None,
    };

    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(module.functions.iter().any(|function| {
        function.module_name.as_deref() == Some("std::http2::frame")
            && function.name.as_deref() == Some("decode")
    }));
    assert!(module.functions.iter().all(|function| {
        function.module_name.as_deref() != Some("std::http2::hpack")
            && function.module_name.as_deref() != Some("std::http2::core")
    }));
}

#[test]
fn explicit_standard_hpack_import_loads_encoder_dependency_closure() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "use http2::hpack from \"std\"\n",
                "pub fn main() -> Result<DynamicTable, String>\n",
                "  http2::hpack::empty_dynamic_table(64)\n",
                "end\n",
            ),
        )],
        manifest: None,
    };

    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    for dependency in [
        "std::http2::hpack",
        "std::http2::hpack::header_encoder",
        "std::http2::hpack::header_list_encoder",
        "std::http2::hpack::string_encoder",
    ] {
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.module_name.as_deref() == Some(dependency)),
            "missing HPACK dependency {dependency}"
        );
    }
    assert!(module.functions.iter().all(|function| {
        !matches!(
            function.module_name.as_deref(),
            Some("std::http2::frame")
                | Some("std::http2::core")
                | Some("std::http2::diagnostic")
                | Some("std::http2::hpack::diagnostic")
        )
    }));
}

#[test]
fn private_standard_http2_modules_cannot_be_imported() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "use http2::hpack::integer from \"std\"\n",
                "use http2::core::pending_header_block from \"std\"\n",
                "pub fn main() -> Int\n",
                "  0\n",
                "end\n",
            ),
        )],
        manifest: None,
    };

    let (_, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "module.unexported_import"
            && diagnostic.message.contains("http2::hpack::integer")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "module.unexported_import"
            && diagnostic
                .message
                .contains("http2::core::pending_header_block")
    }));
}
