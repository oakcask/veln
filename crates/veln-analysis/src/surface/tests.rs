use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use veln_ast::{
    CodecDecl, CodecDirection, CodecImplementationClause, CodecImplementationKind, FunctionKind,
    SurfaceModule, UseOrigin, Visibility, lower_surface_ast,
};
use veln_project::{
    ManifestExport, ManifestField, ManifestLib, ManifestTool, ManifestUnsupportedSection, Project,
    ProjectManifest, parse_manifest_text,
};
use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan};
use veln_syntax::parse;

use super::{
    Diagnostic, EmbeddedStandardModuleEntry, EmbeddedStandardPackage, ReachabilityCache,
    SurfaceParts, embedded_standard_counters, load_embedded_standard_package_from,
    load_project_sources, load_surface_module, reachability_counters, reachable_entry_module,
    reachable_entry_module_with_cache, reachable_entry_module_with_standard_cache,
    validate_manifest_exports,
};

fn lower(text: &str) -> SurfaceModule {
    let source = SourceFile::new("main_test.veln", text);
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    lower_surface_ast(&parsed.tree)
}

fn reachable_function_names(module: &SurfaceModule) -> Vec<(&str, &str)> {
    let mut functions = module
        .functions
        .iter()
        .filter_map(|function| Some((function.module_name.as_deref()?, function.name.as_deref()?)))
        .collect::<Vec<_>>();
    functions.sort_unstable();
    functions
}

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "veln-analysis-surface-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temporary project root should be created");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
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

#[test]
fn external_path_dependency_loads_direct_manifest_package_root() {
    let temp = TempProject::new("external-path-dependency-root");
    temp.write(
        "veln.toml",
        "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
    );
    temp.write(
        "main.veln",
        "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  add_one(1)\nend\n",
    );
    temp.write(
        "vendor/foo/veln.toml",
        "[package]\nname = \"github.com/oakcask/foo\"\n\n[lib]\nexports = [\"foo.veln\"]\n",
    );
    temp.write(
        "vendor/foo/foo.veln",
        "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
    );

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let (_, diagnostics) = load_surface_module(&project);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn external_git_dependency_loads_materialized_subdir_package_root() {
    let temp = TempProject::new("external-git-dependency-subdir-root");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/foo\"]\n",
            "git = \"materialized/mono\"\n",
            "rev = \"abc123\"\n",
            "subdir = \"packages/foo\"\n",
        ),
    );
    temp.write(
        "main.veln",
        "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  add_one(1)\nend\n",
    );
    temp.write(
        "materialized/mono/packages/foo/veln.toml",
        "[package]\nname = \"github.com/oakcask/foo\"\n\n[lib]\nexports = [\"foo.veln\"]\n",
    );
    temp.write(
        "materialized/mono/packages/foo/foo.veln",
        "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
    );

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let (_, diagnostics) = load_surface_module(&project);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[cfg(unix)]
#[test]
fn external_path_dependency_without_direct_manifest_does_not_read_sources() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempProject::new("external-path-dependency-missing-manifest");
    temp.write(
        "veln.toml",
        "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
    );
    temp.write(
        "main.veln",
        "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  0\nend\n",
    );
    temp.write("vendor/foo/foo.veln", "unreadable source");
    let source = temp.path("vendor/foo/foo.veln");
    let original = fs::metadata(&source).unwrap().permissions();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o000)).unwrap();

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let (_, diagnostics) = load_surface_module(&project);

    fs::set_permissions(&source, original).unwrap();
    if !nix_like_effective_root() {
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert_eq!(diagnostics[0].id, "manifest.package_name_mismatch");
        assert!(
            diagnostics[0]
                .message
                .contains("dependency package name `<missing>`"),
            "{diagnostics:#?}"
        );
    }
}

#[cfg(unix)]
fn nix_like_effective_root() -> bool {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .and_then(|line| line.split_whitespace().nth(2))
                .and_then(|uid| uid.parse::<u32>().ok())
        })
        == Some(0)
}

#[test]
fn reachable_resolution_skips_unrelated_annotated_functions() {
    fn resolution_scans(unrelated_count: usize) -> (usize, usize, usize, usize) {
        let mut source =
            String::from("pub fn main() -> Int\n  helper()\nend\n\nfn helper() -> Int\n  1\nend\n");
        for index in 0..unrelated_count {
            source.push_str(&format!(
                "\nfn unrelated_{index}(value: Int) -> Int\n  value\nend\n"
            ));
        }
        let module = lower(&source);
        reachability_counters::reset();
        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        assert_eq!(reachable.functions.len(), 2);
        reachability_counters::snapshot()
    }

    let base = resolution_scans(0);
    let expanded = resolution_scans(128);

    assert_eq!(
        expanded, base,
        "unrelated annotated functions must not add repeated resolution scans"
    );
}

#[test]
fn reachability_cache_keeps_entry_results_independent() {
    let module = lower(concat!(
        "mod app\n",
        "pub fn main() -> Int\n",
        "  main_helper()\n",
        "end\n",
        "fn main_helper() -> Int\n",
        "  1\n",
        "end\n",
        "pub fn alternate() -> Int\n",
        "  alternate_helper()\n",
        "end\n",
        "fn alternate_helper() -> Int\n",
        "  2\n",
        "end\n",
    ));
    let cache = ReachabilityCache::default();

    let main = reachable_entry_module_with_cache(&module, "main", FunctionKind::Function, &cache);
    let alternate =
        reachable_entry_module_with_cache(&module, "alternate", FunctionKind::Function, &cache);

    assert_eq!(
        reachable_function_names(&main),
        [("app", "main"), ("app", "main_helper")]
    );
    assert_eq!(
        reachable_function_names(&alternate),
        [("app", "alternate"), ("app", "alternate_helper")]
    );
}

#[test]
fn reachable_entry_keeps_invalid_import_segments_with_alias_proof_only() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "use HTTP\n",
                    "\n",
                    "fn main() -> Int\n",
                    "  HTTP::entry()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "HTTP.veln",
                concat!("pub fn entry() -> Bool\n", "  1\n", "end\n"),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(
        reachable_function_names(&reachable),
        vec![("HTTP", "entry"), ("app", "main")]
    );
    let imported_entry = reachable
        .functions
        .iter()
        .find(|function| {
            function.module_name.as_deref() == Some("HTTP")
                && function.name.as_deref() == Some("entry")
        })
        .expect("quarantined import proof signature should be retained");
    assert!(
        matches!(
            imported_entry.body.as_slice(),
            [veln_ast::BodyLine {
                kind: veln_ast::BodyLineKind::Expr {
                    expr: veln_ast::Expr {
                        kind: veln_ast::ExprKind::Unit,
                        ..
                    },
                },
                ..
            }]
        ),
        "{:#?}",
        imported_entry.body
    );
    assert!(
        reachable.invalid_names.iter().any(|invalid| {
            invalid.name == "HTTP"
                && invalid.class == veln_ast::NameClass::Module
                && invalid.occurrence == veln_ast::NameOccurrence::PathSegment
        }),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn reachable_entry_skips_invalid_import_in_unselected_module() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new("main.veln", "fn main() -> Int\n  1\nend\n"),
            SourceFile::new(
                "unused.veln",
                concat!(
                    "use HTTP\n",
                    "\n",
                    "fn dead() -> Int\n",
                    "  HTTP::entry()\n",
                    "end\n",
                ),
            ),
            SourceFile::new("HTTP.veln", "pub fn entry() -> Int\n  1\nend\n"),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(reachable_function_names(&reachable), vec![("main", "main")]);
    assert!(
        reachable.invalid_names.iter().all(|invalid| {
            !(invalid.name == "HTTP"
                && invalid.class == veln_ast::NameClass::Module
                && invalid.occurrence == veln_ast::NameOccurrence::PathSegment)
        }),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn reachable_entry_skips_unused_invalid_import_in_entry_module() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new("app.veln", "use HTTP\n\nfn main() -> Int\n  1\nend\n"),
            SourceFile::new("HTTP.veln", "pub fn entry() -> Int\n  1\nend\n"),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(reachable_function_names(&reachable), vec![("app", "main")]);
    assert!(
        reachable.invalid_names.iter().all(|invalid| {
            !(invalid.name == "HTTP"
                && invalid.class == veln_ast::NameClass::Module
                && invalid.occurrence == veln_ast::NameOccurrence::PathSegment)
        }),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn reachable_entry_keeps_valid_import_alias_target_reachable() {
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
                concat!("pub fn entry() -> Int\n", "  1\n", "end\n"),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert_eq!(
        reachable_function_names(&reachable),
        vec![("app", "main"), ("helper", "entry")]
    );
}

#[test]
fn reachable_recovery_selection_skips_unrelated_invalid_declarations() {
    fn recovery_candidate_scans(unrelated_count: usize) -> usize {
        let mut source = String::from(concat!(
            "pub fn main() -> Int\n",
            "  Value\n",
            "end\n",
            "\n",
            "type item\n",
            "  Value\n",
            "end\n",
        ));
        for index in 0..unrelated_count {
            source.push_str(&format!("\ntype unrelated_{index}\n  Other_{index}\nend\n"));
        }
        let module = lower(&source);
        reachability_counters::reset();
        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        assert_eq!(
            reachable.invalid_names.len(),
            1,
            "{:#?}",
            reachable.invalid_names
        );
        reachability_counters::snapshot().3
    }

    assert_eq!(
        recovery_candidate_scans(128),
        recovery_candidate_scans(0),
        "unrelated invalid declarations must not add repeated recovery selector scans"
    );
}

#[test]
fn reachable_materialization_skips_unrelated_annotated_function_bodies() {
    fn materialized_body_count(unrelated_count: usize) -> usize {
        let mut source =
            String::from("pub fn main() -> Int\n  helper()\nend\n\nfn helper() -> Int\n  1\nend\n");
        for index in 0..unrelated_count {
            source.push_str(&format!(
                "\nfn unrelated_{index}(value: Int) -> Int\n  value\nend\n"
            ));
        }
        let module = lower(&source);
        reachability_counters::reset();
        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        assert_eq!(reachable.functions.len(), 2);
        reachability_counters::snapshot().2
    }

    assert_eq!(
        materialized_body_count(128),
        materialized_body_count(0),
        "unreachable annotated functions must not be materialized for lowering"
    );
}

#[test]
fn separated_reachable_materialization_skips_unrelated_annotated_function_bodies() {
    fn materialized_body_count(unrelated_count: usize) -> usize {
        let standard = lower(concat!(
            "mod std::prelude\n",
            "pub fn standard_value() -> Int\n",
            "  1\n",
            "end\n",
        ));
        let mut source = String::from(concat!(
            "mod app\n",
            "use std::prelude\n",
            "\n",
            "pub fn main() -> Int\n",
            "  helper() + standard_value()\n",
            "end\n",
            "\n",
            "fn helper() -> Int\n",
            "  1\n",
            "end\n",
        ));
        for index in 0..unrelated_count {
            source.push_str(&format!(
                "\nfn unrelated_{index}(value: Int) -> Int\n  value\nend\n"
            ));
        }
        let application = lower(&source);
        reachability_counters::reset();
        let reachable = reachable_entry_module_with_standard_cache(
            &standard,
            &application,
            "main",
            FunctionKind::Function,
            &ReachabilityCache::default(),
        );
        assert_eq!(reachable.functions.len(), 3);
        reachability_counters::snapshot().2
    }

    assert_eq!(
        materialized_body_count(128),
        materialized_body_count(0),
        "separated reachable inputs must not materialize unreachable annotated functions"
    );
}

#[test]
fn separated_reachable_inputs_match_combined_resolution_results() {
    let mut standard = lower(concat!(
        "mod std::prelude\n",
        "pub type StandardValue\n",
        "  Present\n",
        "end\n",
        "pub schema Packet\n",
        "  value: Int\n",
        "end\n",
        "pub fn standard_value() -> Int\n",
        "  1\n",
        "end\n",
    ));
    add_payload_codec_for_test(&mut standard);
    let mut application = lower(concat!(
        "mod app\n",
        "use std::prelude\n",
        "\n",
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "\n",
        "fn answer() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "\n",
        "handler ask(seed: Int) handles Ask\n",
        "  value() => seed\n",
        "end\n",
        "\n",
        "type ApplicationValue\n",
        "  Present\n",
        "end\n",
        "\n",
        "schema Packet\n",
        "  value: Int\n",
        "end\n",
        "\n",
        "pub fn exposed = answer\n",
        "\n",
        "pub fn main() -> Int\n",
        "  let handled = handle exposed() with ask(2)\n",
        "  handled + standard_value()\n",
        "end\n",
    ));
    add_payload_codec_for_test(&mut application);
    let mut combined = standard.clone();
    combined.uses.extend(application.uses.clone());
    combined.aliases.extend(application.aliases.clone());
    combined.effects.extend(application.effects.clone());
    combined.handlers.extend(application.handlers.clone());
    combined.types.extend(application.types.clone());
    combined.schemas.extend(application.schemas.clone());
    combined.codecs.extend(application.codecs.clone());
    combined.functions.extend(application.functions.clone());
    combined
        .invalid_names
        .extend(application.invalid_names.clone());
    combined.module = application.module.clone();

    let combined_reachable = reachable_entry_module(&combined, "main", FunctionKind::Function);
    let separated_reachable = reachable_entry_module_with_standard_cache(
        &standard,
        &application,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );

    let combined_functions = reachable_function_names(&combined_reachable);
    let separated_functions = reachable_function_names(&separated_reachable);
    assert_eq!(separated_functions, combined_functions);
    assert_eq!(
        separated_functions,
        vec![
            ("app", "answer"),
            ("app", "main"),
            ("std::prelude", "standard_value"),
        ]
    );
    assert_eq!(
        separated_reachable
            .module
            .as_ref()
            .map(|module| module.name.as_str()),
        Some("app")
    );
    assert_eq!(
        separated_reachable.uses.len(),
        combined_reachable.uses.len()
    );
    assert_eq!(
        separated_reachable.aliases.len(),
        combined_reachable.aliases.len()
    );
    assert_eq!(
        separated_reachable.effects.len(),
        combined_reachable.effects.len()
    );
    assert_eq!(
        separated_reachable.handlers.len(),
        combined_reachable.handlers.len()
    );
    assert_eq!(
        separated_reachable.types.len(),
        combined_reachable.types.len()
    );
    assert_eq!(
        separated_reachable.schemas.len(),
        combined_reachable.schemas.len()
    );
    assert_eq!(
        separated_reachable.codecs.len(),
        combined_reachable.codecs.len()
    );
}

#[test]
fn separated_reachable_inputs_resolve_codec_with_targets() {
    let mut standard = lower(concat!(
        "mod std::prelude\n",
        "pub schema Packet\n",
        "  value: Int\n",
        "end\n",
        "\n",
        "fn decode_payload_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{value: Int}>\n",
        "  NeedMore(NeedEnd)\n",
        "end\n",
    ));
    add_payload_codec_for_test(&mut standard);
    let application = lower(concat!(
        "mod app\n",
        "use std::prelude\n",
        "\n",
        "pub fn main(source: ByteView, base: ByteOffset) -> DecodeStep<{value: Int}>\n",
        "  std::prelude::PayloadCodec(source, base)\n",
        "end\n",
    ));
    let mut combined = standard.clone();
    combined.uses.extend(application.uses.clone());
    combined.aliases.extend(application.aliases.clone());
    combined.effects.extend(application.effects.clone());
    combined.handlers.extend(application.handlers.clone());
    combined.types.extend(application.types.clone());
    combined.schemas.extend(application.schemas.clone());
    combined.codecs.extend(application.codecs.clone());
    combined.functions.extend(application.functions.clone());
    combined
        .invalid_names
        .extend(application.invalid_names.clone());
    combined.module = application.module.clone();

    let combined_reachable = reachable_entry_module(&combined, "main", FunctionKind::Function);
    let separated_reachable = reachable_entry_module_with_standard_cache(
        &standard,
        &application,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );

    let combined_functions = reachable_function_names(&combined_reachable);
    let separated_functions = reachable_function_names(&separated_reachable);
    assert_eq!(separated_functions, combined_functions);
    assert_eq!(
        separated_functions,
        vec![("app", "main"), ("std::prelude", "decode_payload_packet")]
    );
}

fn add_payload_codec_for_test(module: &mut SurfaceModule) {
    let schema = module
        .schemas
        .iter()
        .find(|schema| schema.name.as_deref() == Some("Packet"))
        .expect("test standard module should define Packet schema");
    module.codecs.push(CodecDecl {
        node_id: schema.node_id,
        module_name: Some("std::prelude".to_string()),
        visibility: Visibility::Public,
        name: Some("PayloadCodec".to_string()),
        schema: Some("Packet".to_string()),
        directions: vec![CodecDirection::Decode],
        implementations: vec![CodecImplementationClause {
            node_id: schema.node_id,
            direction: CodecDirection::Decode,
            kind: CodecImplementationKind::With {
                function: Some("decode_payload_packet".to_string()),
            },
            span: schema.span.clone(),
        }],
        span: schema.span.clone(),
    });
}

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

#[test]
fn test_entry_can_reach_function_callee() {
    let module = lower(concat!(
        "test foo() -> ()\n",
        "  helper()\n",
        "end\n",
        "fn helper() -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("helper")),
        ]
    );
}

#[test]
fn test_entry_can_reach_function_value_reference() {
    let module = lower(concat!(
        "test foo() -> ()\n",
        "  vec_map([1], stringify)\n",
        "  ()\n",
        "end\n",
        "fn stringify(value: Int) -> String\n",
        "  \"ok\"\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("stringify")),
        ]
    );
}

#[test]
fn test_entry_reaches_same_shape_variadic_function_value_targets() {
    let module = lower(concat!(
        "test foo(callback: fn(String, ...String) -> String) -> ()\n",
        "  callback(\"prefix\", \"a\", \"b\")\n",
        "  ()\n",
        "end\n",
        "fn join(prefix: String, values: ...String) -> String\n",
        "  prefix\n",
        "end\n",
        "fn fixed(prefix: String, value: String) -> String\n",
        "  prefix\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("join")),
        ]
    );
}

#[test]
fn test_entry_does_not_reach_variadic_function_value_targets_for_too_few_args() {
    let module = lower(concat!(
        "test foo(callback: fn(String, ...String) -> String) -> ()\n",
        "  callback()\n",
        "  ()\n",
        "end\n",
        "fn join(prefix: String, values: ...String) -> String\n",
        "  prefix\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Test, Some("foo"))]);
}

#[test]
fn test_entry_conservatively_reaches_opaque_function_value_call_targets() {
    let module = lower(concat!(
        "test foo() -> Bool\n",
        "  invoke(ready)\n",
        "end\n",
        "fn invoke(job: fn() -> Bool) -> Bool\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "fn risky() -> Bool\n",
        "  _\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("invoke")),
            (FunctionKind::Function, Some("ready")),
            (FunctionKind::Function, Some("risky")),
        ]
    );
}

#[test]
fn test_entry_reaches_opaque_function_value_call_targets_with_spaced_type() {
    let module = lower(concat!(
        "test foo() -> Bool\n",
        "  invoke(ready)\n",
        "end\n",
        "fn invoke(job: fn () -> Bool) -> Bool\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "fn risky() -> Bool\n",
        "  _\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("invoke")),
            (FunctionKind::Function, Some("ready")),
            (FunctionKind::Function, Some("risky")),
        ]
    );
}

#[test]
fn test_entry_conservatively_reaches_opaque_local_function_value_call_targets() {
    let module = lower(concat!(
        "test foo() -> Bool\n",
        "  invoke()\n",
        "end\n",
        "fn invoke() -> Bool\n",
        "  let job: fn() -> Bool = ready\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "fn risky() -> Bool\n",
        "  _\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("invoke")),
            (FunctionKind::Function, Some("ready")),
            (FunctionKind::Function, Some("risky")),
        ]
    );
}

#[test]
fn run_entry_conservatively_reaches_opaque_function_value_call_targets() {
    let standard = lower("mod std::prelude\n");
    let application = lower(concat!(
        "mod app\n",
        "fn invoke(job: fn() -> Bool) -> Bool\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "fn risky() -> Bool\n",
        "  _\n",
        "end\n",
        "pub fn main() -> Bool\n",
        "  invoke(ready)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module_with_standard_cache(
        &standard,
        &application,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );
    let functions = reachable_function_names(&reachable);

    assert_eq!(
        functions,
        vec![
            ("app", "invoke"),
            ("app", "main"),
            ("app", "ready"),
            ("app", "risky"),
        ]
    );
}

#[test]
fn test_entry_can_reach_qualified_function_value_reference() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main_test.veln",
                concat!(
                    "use app::text\n",
                    "test foo() -> ()\n",
                    "  vec_map([1], app::text::stringify)\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/text.veln",
                concat!(
                    "pub fn stringify(value: Int) -> String\n",
                    "  \"ok\"\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (Some("app::main_test"), FunctionKind::Test, Some("foo")),
            (Some("app::text"), FunctionKind::Function, Some("stringify")),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_push")
            ),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_concat")
            ),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_append")
            ),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_map")
            ),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_map_step")
            ),
        ]
    );
}

#[test]
fn companion_test_entry_reaches_qualified_private_target_function() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "test increment_test() -> Int\n",
                    "  math::increment(1)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "math.veln",
                concat!(
                    "fn increment(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "increment_test", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        functions.contains(&(
            Some("math__test_companion"),
            FunctionKind::Test,
            Some("increment_test")
        )),
        "{functions:#?}"
    );
    assert!(
        functions.contains(&(Some("math"), FunctionKind::Function, Some("increment"))),
        "{functions:#?}"
    );
}

#[test]
fn companion_test_entry_keeps_qualified_private_target_handler() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "test handler_test() -> Int\n",
                    "  handle math::compute() with math::ask(41)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "math.veln",
                concat!(
                    "effect Ask\n",
                    "  value() -> Int\n",
                    "end\n",
                    "fn provide(offset: Int) -> Int\n",
                    "  offset + 1\n",
                    "end\n",
                    "handler ask(offset: Int) handles Ask\n",
                    "  value() => provide(offset)\n",
                    "end\n",
                    "pub fn compute() -> Int effects [Ask]\n",
                    "  perform Ask::value()\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "handler_test", FunctionKind::Test);
    let handlers = reachable
        .handlers
        .iter()
        .map(|handler| (handler.module_name.as_deref(), handler.name.as_deref()))
        .collect::<Vec<_>>();
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        handlers.contains(&(Some("math"), Some("ask"))),
        "{handlers:#?}"
    );
    assert!(
        functions.contains(&(Some("math"), FunctionKind::Function, Some("provide"))),
        "{functions:#?}"
    );
}

#[test]
fn companion_public_alias_cannot_reexport_private_target_function() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "pub fn expose = math::increment\n",
                    "test expose_test() -> Int\n",
                    "  expose(1)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "math.veln",
                concat!(
                    "fn increment(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "module.companion_public_declaration")
        .unwrap_or_else(|| {
            panic!("expected companion public declaration diagnostic for alias: {diagnostics:#?}")
        });
    assert_eq!(
        detail_string(diagnostic, "reason"),
        Some("public_function_alias")
    );

    let reachable = reachable_entry_module(&module, "expose_test", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        functions.contains(&(
            Some("math__test_companion"),
            FunctionKind::Test,
            Some("expose_test")
        )),
        "{functions:#?}"
    );
    assert!(
        !functions.contains(&(Some("math"), FunctionKind::Function, Some("increment"))),
        "{functions:#?}"
    );
}

#[test]
fn companion_test_entry_does_not_reach_private_target_function_value() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "test increment_value_test() -> Int\n",
                    "  let mapper: fn(Int) -> Int = math::increment\n",
                    "  mapper(1)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "math.veln",
                concat!(
                    "fn increment(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "increment_value_test", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        functions.contains(&(
            Some("math__test_companion"),
            FunctionKind::Test,
            Some("increment_value_test")
        )),
        "{functions:#?}"
    );
    assert!(
        !functions.contains(&(Some("math"), FunctionKind::Function, Some("increment"))),
        "{functions:#?}"
    );
}

#[test]
fn companion_call_does_not_change_production_private_inference_reachability() {
    let target = SourceFile::new(
        "math.veln",
        concat!(
            "fn identity(value)\n",
            "  value\n",
            "end\n",
            "pub fn production() -> Int\n",
            "  identity(_)\n",
            "end\n",
        ),
    );
    let project_without_companion = Project {
        root: ".".into(),
        files: vec![target.clone()],
        manifest: None,
    };
    let project_with_companion = Project {
        root: ".".into(),
        files: vec![
            target,
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "test identity_test() -> Int\n",
                    "  math::identity(1)\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };

    let (without_companion, without_diagnostics) = load_surface_module(&project_without_companion);
    let (with_companion, with_diagnostics) = load_surface_module(&project_with_companion);
    assert!(without_diagnostics.is_empty(), "{without_diagnostics:#?}");
    assert!(with_diagnostics.is_empty(), "{with_diagnostics:#?}");

    let production_without =
        reachable_entry_module(&without_companion, "production", FunctionKind::Function);
    let production_with =
        reachable_entry_module(&with_companion, "production", FunctionKind::Function);
    let without_functions = production_without
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
                function
                    .params
                    .iter()
                    .map(|param| param.ty.as_deref())
                    .collect::<Vec<_>>(),
                function.return_type.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    let with_functions = production_with
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
                function
                    .params
                    .iter()
                    .map(|param| param.ty.as_deref())
                    .collect::<Vec<_>>(),
                function.return_type.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(with_functions, without_functions);
}

#[test]
fn run_entry_filters_unreachable_invalid_non_function_names() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  1\n",
        "end\n",
        "fn Bad() -> Int\n",
        "  2\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "pub fn Exported = Bad\n",
        "pub type exported = item\n",
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "handler ask(Context: Int) handles Ask\n",
        "  value() => Context\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert!(reachable.aliases.is_empty(), "{:#?}", reachable.aliases);
    assert!(reachable.handlers.is_empty(), "{:#?}", reachable.handlers);
}

#[test]
fn run_entry_keeps_invalid_type_names_referenced_by_reachable_signature() {
    let module = lower(concat!(
        "type item\n",
        "  value\n",
        "end\n",
        "fn main() -> item\n",
        "  1\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item", "value"]);
}

#[test]
fn run_entry_does_not_reach_invalid_type_from_local_value_spelling() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  let item = 1\n",
        "  item\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_reach_invalid_type_from_record_field_spelling() {
    let module = lower(concat!(
        "fn main() -> {item: Int}\n",
        "  {item: 1}\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_reach_invalid_alias_from_return_type_spelling() {
    let module = lower(concat!(
        "type Item\n",
        "  Value\n",
        "end\n",
        "fn main() -> Item\n",
        "  Value\n",
        "end\n",
        "fn good() -> Item\n",
        "  Value\n",
        "end\n",
        "pub fn Item = good\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert!(reachable.aliases.is_empty(), "{:#?}", reachable.aliases);
}

#[test]
fn run_entry_keeps_reachable_invalid_function_alias_name() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Exported()\n",
        "end\n",
        "fn good() -> Int\n",
        "  1\n",
        "end\n",
        "pub fn Exported = good\n",
        "pub fn Unreachable = good\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["Exported"]);
    assert!(
        reachable
            .aliases
            .iter()
            .any(|alias| alias.name.as_deref() == Some("Exported"))
    );
    assert!(
        reachable
            .aliases
            .iter()
            .all(|alias| alias.name.as_deref() != Some("Unreachable")),
        "unreachable invalid aliases must not materialize: {:#?}",
        reachable.aliases
    );
}

#[test]
fn run_entry_keeps_invalid_constructor_referenced_by_reachable_expression_path() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  value\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "type other\n",
        "  other_value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item", "value"]);
}

#[test]
fn run_entry_keeps_unique_invalid_constructor_call_by_arity() {
    let module = lower(concat!(
        "fn main() -> item\n",
        "  value(1)\n",
        "end\n",
        "type item\n",
        "  value(Int)\n",
        "end\n",
        "type other\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item", "value"]);
}

#[test]
fn run_entry_keeps_only_selected_invalid_constructor_in_valid_type() {
    let module = lower(concat!(
        "fn main() -> Item\n",
        "  value(1)\n",
        "end\n",
        "type Item\n",
        "  value(Int)\n",
        "  other(Int)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["value"]);
}

#[test]
fn run_entry_keeps_invalid_type_for_reachable_valid_nullary_constructor() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Value\n",
        "end\n",
        "type item\n",
        "  Value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item"]);
}

#[test]
fn run_entry_keeps_invalid_type_for_reachable_valid_payload_constructor() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Payload(1)\n",
        "end\n",
        "type item\n",
        "  Payload(Int)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item"]);
}

#[test]
fn run_entry_does_not_choose_ambiguous_owned_constructor_recovery_with_same_owner_span() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Value\n",
        "end\n",
        "type item\n",
        "  Value\n",
        "  Value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_reach_unreachable_invalid_type_with_valid_constructor() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  1\n",
        "end\n",
        "type item\n",
        "  Value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_choose_ambiguous_constructor_recovery() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  value\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "type other\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_choose_cross_class_recovery_ambiguity() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Bad(1)\n",
        "end\n",
        "type item\n",
        "  Bad(Int)\n",
        "end\n",
        "fn Bad(value: Int) -> Int\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert_eq!(
        reachable
            .functions
            .iter()
            .filter(|function| {
                function.kind == FunctionKind::Function && function.name.as_deref() == Some("Bad")
            })
            .count(),
        0
    );
}

#[test]
fn run_entry_filters_same_name_recovery_peers_by_call_arity() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Bad(1)\n",
        "end\n",
        "fn Bad(value: Int) -> Int\n",
        "  value\n",
        "end\n",
        "fn Bad(left: Int, right: Int) -> Int\n",
        "  left + right\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["Bad"]);
    assert_eq!(
        reachable
            .functions
            .iter()
            .filter(|function| {
                function.kind == FunctionKind::Function && function.name.as_deref() == Some("Bad")
            })
            .count(),
        1
    );
}

#[test]
fn run_entry_uses_valid_constructor_before_same_spelled_function_recovery() {
    let module = lower(concat!(
        "type Item\n",
        "  Bad\n",
        "end\n",
        "fn main() -> Item\n",
        "  Bad\n",
        "end\n",
        "fn Bad() -> Item\n",
        "  Bad\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_uses_valid_function_value_before_constructor_recovery() {
    let module = lower(concat!(
        "type Item\n",
        "  bad\n",
        "end\n",
        "fn bad() -> Int\n",
        "  1\n",
        "end\n",
        "fn main() -> Int\n",
        "  let callable: fn() -> Int = bad\n",
        "  callable()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_uses_valid_function_arity_error_before_constructor_recovery() {
    let module = lower(concat!(
        "type Item\n",
        "  good(Int)\n",
        "end\n",
        "fn good() -> Int\n",
        "  7\n",
        "end\n",
        "fn main() -> Int\n",
        "  good(1)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_uses_valid_constructor_arity_error_before_function_recovery() {
    let module = lower(concat!(
        "type Item\n",
        "  Bad(Int)\n",
        "end\n",
        "fn Bad() -> Item\n",
        "  Bad(1)\n",
        "end\n",
        "fn main() -> Item\n",
        "  Bad()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_keeps_invalid_bindings_in_reachable_handler() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "fn body() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "handler ask(Context: Int) handles Ask\n",
        "  value(Result) => Context + Result\n",
        "end\n",
        "fn main() -> Int\n",
        "  handle body() with ask(1)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["Context", "Result"]);
    assert_eq!(reachable.handlers.len(), 1, "{:#?}", reachable.handlers);
}

#[test]
fn run_entry_ignores_invalid_bindings_in_unreachable_handler() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "handler ask(Context: Int) handles Ask\n",
        "  value(Result) => Context + Result\n",
        "end\n",
        "fn main() -> Int\n",
        "  1\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert!(reachable.handlers.is_empty(), "{:#?}", reachable.handlers);
}

#[test]
fn run_entry_keeps_invalid_type_from_reachable_handler_parameter_annotation() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "fn body() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "handler ask(seed: item) handles Ask\n",
        "  value() => 1\n",
        "end\n",
        "handler unreachable(seed: other) handles Ask\n",
        "  value() => 2\n",
        "end\n",
        "type other\n",
        "  other_value\n",
        "end\n",
        "fn main() -> Int\n",
        "  handle body() with ask(value)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item", "value"]);
}

#[test]
fn run_entry_keeps_invalid_constructor_from_reachable_handler_clause_expression() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "fn body() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "handler ask() handles Ask\n",
        "  value() => value\n",
        "end\n",
        "handler unreachable() handles Ask\n",
        "  value() => other_value\n",
        "end\n",
        "type other\n",
        "  other_value\n",
        "end\n",
        "fn main() -> Int\n",
        "  handle body() with ask()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item", "value"]);
}

#[test]
fn run_entry_keeps_invalid_constructor_from_reachable_handler_match_scrutinee() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "fn body() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "handler ask() handles Ask\n",
        "  value() => match value\n",
        "    value => 1\n",
        "  end\n",
        "end\n",
        "handler unreachable() handles Ask\n",
        "  value() => other_value\n",
        "end\n",
        "type other\n",
        "  other_value\n",
        "end\n",
        "fn main() -> Int\n",
        "  handle body() with ask()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item", "value"]);
}

#[test]
fn run_entry_does_not_select_imported_function_recovery() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "mod app\n",
                    "use helper\n",
                    "fn main() -> Int\n",
                    "  helper::Bad()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "helper.veln",
                concat!("mod helper\n", "pub fn Bad() -> Int\n", "  1\n", "end\n"),
            ),
        ],
        manifest: None,
    };
    let (module, _) = load_surface_module(&project);

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_preserves_qualified_type_references_for_recovery_selection() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "mod app\n",
                    "use helper\n",
                    "fn main(input: helper::item) -> Int\n",
                    "  1\n",
                    "end\n",
                    "type item\n",
                    "  Value\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "helper.veln",
                concat!("mod helper\n", "pub type item\n", "  Value\n", "end\n"),
            ),
        ],
        manifest: None,
    };
    let (module, _) = load_surface_module(&project);

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn companion_public_declarations_report_stable_reasons() {
    let cases = [
        (
            "public_function",
            concat!("pub fn exposed() -> ()\n", "  ()\n", "end\n"),
        ),
        (
            "public_effect",
            concat!("pub effect Visible\n", "  call() -> ()\n", "end\n"),
        ),
        (
            "public_handler",
            concat!(
                "effect Ask\n",
                "  call() -> ()\n",
                "end\n",
                "fn provide() -> ()\n",
                "  ()\n",
                "end\n",
                "pub handler visible() handles Ask\n",
                "  call() => provide()\n",
                "end\n",
            ),
        ),
        (
            "public_type",
            concat!("pub type Visible\n", "  Case\n", "end\n"),
        ),
        (
            "public_type_variant",
            concat!("type Local\n", "  pub Visible\n", "end\n"),
        ),
        (
            "public_schema",
            concat!(
                "pub schema Visible\n",
                "  format binary\n",
                "  value: UInt8\n",
                "end\n",
            ),
        ),
        ("public_function_alias", "pub fn visible = math::target\n"),
        ("public_type_alias", "pub type Visible = math::Target\n"),
        ("public_schema_alias", "pub schema Visible = math::Target\n"),
    ];

    for (reason, companion_text) in cases {
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new("math.test.veln", companion_text),
                SourceFile::new("math.veln", "fn target() -> ()\n  ()\nend\n"),
            ],
            manifest: None,
        };

        let (_, diagnostics) = load_surface_module(&project);
        let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.id == "module.companion_public_declaration")
                .unwrap_or_else(|| {
                    panic!(
                        "expected companion public declaration diagnostic for {reason}: {diagnostics:#?}"
                    )
                });

        assert_eq!(
            detail_string(diagnostic, "companion_path"),
            Some("math.test.veln")
        );
        assert_eq!(detail_string(diagnostic, "reason"), Some(reason));
    }
}

#[test]
fn companion_private_declarations_remain_valid() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "fn helper() -> ()\n",
                    "  ()\n",
                    "end\n",
                    "effect Ask\n",
                    "  call() -> ()\n",
                    "end\n",
                    "handler local() handles Ask\n",
                    "  call=helper\n",
                    "end\n",
                    "type Local\n",
                    "  Case\n",
                    "end\n",
                    "schema Packet\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n",
                ),
            ),
            SourceFile::new("math.veln", "fn target() -> ()\n  ()\nend\n"),
        ],
        manifest: None,
    };

    let (_, diagnostics) = load_surface_module(&project);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "module.companion_public_declaration"),
        "{diagnostics:#?}"
    );
}

#[test]
fn ordinary_public_declarations_remain_valid() {
    let declaration = concat!(
        "pub fn exposed() -> ()\n",
        "  ()\n",
        "end\n",
        "pub effect Ask\n",
        "  call() -> ()\n",
        "end\n",
        "pub handler visible() handles Ask\n",
        "  call=exposed\n",
        "end\n",
        "pub type Visible\n",
        "  pub Case\n",
        "end\n",
        "pub schema Packet\n",
        "  format binary\n",
        "  value: UInt8\n",
        "end\n",
        "pub fn alias = math::exposed\n",
        "pub type Alias = math::Visible\n",
        "pub schema PacketAlias = math::Packet\n",
    );
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new("math.veln", declaration),
            SourceFile::new("math_test.veln", declaration),
        ],
        manifest: None,
    };

    let (_, diagnostics) = load_surface_module(&project);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "module.companion_public_declaration"),
        "{diagnostics:#?}"
    );
}

#[test]
fn run_entry_keeps_schema_decode_expression_in_entry_function() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "schema PacketWire\n",
                "  format binary\n",
                "  length: UInt8\n",
                "end\n",
                "\n",
                "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
                "  decode PacketWire from view at base\n",
                "end\n",
            ),
        )],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![(Some("main"), FunctionKind::Function, Some("main"))]
    );
}

#[test]
fn run_entry_keeps_schema_encode_expression_in_entry_function() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "schema PacketWire\n",
                "  format binary\n",
                "  length: UInt8\n",
                "end\n",
                "\n",
                "pub fn main(packet: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
                "  encode PacketWire from packet\n",
                "end\n",
            ),
        )],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![(Some("main"), FunctionKind::Function, Some("main"))]
    );
}

#[test]
fn run_entry_can_reach_contract_helper() {
    let module = lower(concat!(
        "fn positive(value: Int) -> Bool\n",
        "  value > 0\n",
        "end\n",
        "pub fn main(value: Int) -> output: Int\n",
        "  ensure positive(output)\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Function, Some("positive")),
            (FunctionKind::Function, Some("main")),
        ]
    );
}

#[test]
fn run_entry_can_reach_contract_function_value() {
    let module = lower(concat!(
        "fn accepts(job: fn() -> Bool) -> Bool\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "pub fn main() -> ()\n",
        "  require accepts(ready)\n",
        "  ()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Function, Some("accepts")),
            (FunctionKind::Function, Some("ready")),
            (FunctionKind::Function, Some("main")),
        ]
    );
}

#[test]
fn run_entry_can_reach_qualified_contract_helper() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::rules\n",
                    "pub fn main(value: Int) -> output: Int\n",
                    "  ensure app::rules::positive(output)\n",
                    "  value\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/rules.veln",
                concat!(
                    "pub fn positive(value: Int) -> Bool\n",
                    "  value > 0\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::rules"), FunctionKind::Function, Some("positive")),
        ]
    );
}

#[test]
fn run_entry_can_reach_imported_qualified_call() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::util\n",
                    "pub fn main() -> Int\n",
                    "  app::util::value()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/util.veln",
                concat!("pub fn value() -> Int\n", "  1\n", "end\n",),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::util"), FunctionKind::Function, Some("value")),
        ]
    );
}

#[test]
fn run_entry_can_reach_imported_alias_target() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::api\n",
                    "pub fn main() -> Int\n",
                    "  app::api::twice(21)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/api.veln",
                concat!("use app::impl\n", "pub fn twice = app::impl::double\n",),
            ),
            SourceFile::new(
                "app/impl.veln",
                concat!(
                    "fn double(value: Int) -> Int\n",
                    "  value + value\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::impl"), FunctionKind::Function, Some("double")),
        ]
    );
}

#[test]
fn run_entry_can_reach_qualified_contract_function_value() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::rules\n",
                    "fn accepts(job: fn() -> Bool) -> Bool\n",
                    "  job()\n",
                    "end\n",
                    "pub fn main() -> ()\n",
                    "  require accepts(app::rules::ready)\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/rules.veln",
                concat!("pub fn ready() -> Bool\n", "  true\n", "end\n",),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (Some("app::main"), FunctionKind::Function, Some("accepts")),
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::rules"), FunctionKind::Function, Some("ready")),
        ]
    );
}

#[test]
fn imported_reachability_keeps_module_specific_function_names() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::util\n",
                    "fn value() -> Int\n",
                    "  _\n",
                    "end\n",
                    "pub fn main() -> Int\n",
                    "  app::util::value()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/util.veln",
                concat!("pub fn value() -> Int\n", "  1\n", "end\n",),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::util"), FunctionKind::Function, Some("value")),
        ]
    );
}

#[test]
fn bare_reachability_keeps_current_module_function_names() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "fn value() -> Int\n",
                    "  1\n",
                    "end\n",
                    "pub fn main() -> Int\n",
                    "  value()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/other.veln",
                concat!("fn value() -> Int\n", "  _\n", "end\n",),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (Some("app::main"), FunctionKind::Function, Some("value")),
            (Some("app::main"), FunctionKind::Function, Some("main")),
        ]
    );
}

#[test]
fn local_binding_shadowing_function_name_does_not_reach_function() {
    let module = lower(concat!(
        "fn helper() -> Int\n",
        "  _\n",
        "end\n",
        "pub fn main() -> Int\n",
        "  let helper = 1\n",
        "  helper\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
}

#[test]
fn match_binding_shadowing_function_name_does_not_reach_function() {
    let module = lower(concat!(
        "fn helper() -> Int\n",
        "  _\n",
        "end\n",
        "pub fn main(value: Option<Int>) -> Int\n",
        "  match value\n",
        "    Some(helper) => helper\n",
        "    None => 0\n",
        "  end\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
}

#[test]
fn run_entry_does_not_reach_qualified_call_without_import_alias() {
    let module = lower(concat!(
        "pub fn main() -> Int\n",
        "  util::value()\n",
        "end\n",
        "fn value() -> Int\n",
        "  _\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
}

#[test]
fn contract_reachability_ignores_function_names_inside_strings() {
    let module = lower(concat!(
        "fn positive(value: Int) -> Bool\n",
        "  value > 0\n",
        "end\n",
        "pub fn main() -> output: String\n",
        "  ensure \"positive(\" == output\n",
        "  \"positive(\"\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
}

#[test]
fn run_entry_does_not_include_tests() {
    let module = lower(concat!(
        "test helper() -> ()\n",
        "  ()\n",
        "end\n",
        "fn foo() -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("foo"))]);
}

#[test]
fn run_entry_reaches_spawn_with_context_function_value() {
    let module = lower(concat!(
        "fn combine(context: {payload: String, suffix: String}) -> String effects [concurrency]\n",
        "  suffix(context.suffix)\n",
        "end\n",
        "fn suffix(value: String) -> String\n",
        "  value\n",
        "end\n",
        "pub fn main() -> Task<String> effects [concurrency]\n",
        "  task::spawn_with(combine, {payload: \"body\", suffix: \"tail\"})\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Function, Some("combine")),
            (FunctionKind::Function, Some("suffix")),
            (FunctionKind::Function, Some("main")),
        ]
    );
}

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

fn span(file: &str, line: usize, start_column: usize, end_column: usize) -> SourceSpan {
    SourceSpan {
        file: SourcePath::new(file),
        start: LineCol {
            line,
            column: start_column,
            offset: 0,
        },
        end: LineCol {
            line,
            column: end_column,
            offset: 0,
        },
    }
}

fn detail_string<'a>(diagnostic: &'a veln_diagnostics::Diagnostic, key: &str) -> Option<&'a str> {
    let veln_diagnostics::JsonValue::Object(entries) = &diagnostic.details else {
        return None;
    };
    entries.iter().find_map(|(entry_key, value)| {
        if entry_key == key
            && let veln_diagnostics::JsonValue::String(value) = value
        {
            Some(value.as_str())
        } else {
            None
        }
    })
}
