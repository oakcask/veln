use super::*;

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
fn run_entry_reachability_preserves_explicit_import_alias_identity() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::util as selected\n",
                    "use app::alternate as util\n",
                    "pub fn main() -> Int\n",
                    "  selected::value() + app::util::value()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/util.veln",
                concat!("pub fn value() -> Int\n", "  1\n", "end\n",),
            ),
            SourceFile::new(
                "app/alternate.veln",
                concat!("pub fn value() -> Int\n", "  40\n", "end\n",),
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
        ],
        "the colliding local qualifier must not redirect the explicit alias or exact path"
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
            (Some("app::impl"), FunctionKind::Function, Some("double")),
            (Some("app::main"), FunctionKind::Function, Some("main")),
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
fn malformed_import_aliases_never_retain_targets_through_reachability() {
    let malformed_rows = [
        "use pkg::target as\n",
        "use pkg::target as from \"pkg\"\n",
        "use pkg::target as \"local\"\n",
        "use pkg::target as kept::tail\n",
        "use pkg::target from \"pkg\" as misplaced\n",
        "use pkg::target as kept as repeated\n",
        "use pkg::target from\n",
    ];

    for malformed in malformed_rows {
        let app_source = SourceFile::new(
            "app.veln",
            format!(
                concat!(
                    "{malformed}",
                    "fn retained() -> Int\n",
                    "  1\n",
                    "end\n",
                    "pub fn main() -> Int\n",
                    "  let exact = pkg::target::member()\n",
                    "  let leaf = target::member()\n",
                    "  let alias = kept::member()\n",
                    "  exact + leaf + alias + member() + retained()\n",
                    "end\n",
                ),
                malformed = malformed,
            ),
        );
        let parsed_app = parse(&app_source);
        assert!(!parsed_app.diagnostics.is_empty(), "{malformed:?}");
        let mut module = veln_ast::lower_surface_ast_with_module_identity(
            &parsed_app.tree,
            "app".to_string(),
            app_source.span(veln_source::TextRange::new(0, 0)),
        );
        assert_eq!(module.uses.len(), 1, "{malformed:?}");
        assert!(module.uses[0].syntax_recovered, "{malformed:?}");

        let target_source =
            SourceFile::new("pkg/target.veln", "pub fn member() -> Int\n  40\nend\n");
        let parsed_target = parse(&target_source);
        assert!(parsed_target.diagnostics.is_empty());
        let target = veln_ast::lower_surface_ast_with_module_identity(
            &parsed_target.tree,
            "pkg::target".to_string(),
            target_source.span(veln_source::TextRange::new(0, 0)),
        );
        module.functions.extend(target.functions);

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        assert_eq!(
            reachable
                .functions
                .iter()
                .filter_map(|function| Some((
                    function.module_name.as_deref()?,
                    function.name.as_deref()?,
                )))
                .collect::<Vec<_>>(),
            [("app", "retained"), ("app", "main")],
            "malformed import retained a target through a normal route: {malformed:?}"
        );
    }
}

#[test]
fn duplicate_target_aliases_do_not_reach_calls_or_function_values_in_either_order() {
    for reverse_imports in [false, true] {
        for reverse_sources in [false, true] {
            let module = duplicate_alias_reachability_module(reverse_imports, reverse_sources);

            let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
            assert_eq!(
                reachable_function_names(&reachable),
                [("app", "accepts"), ("app", "main")],
                "duplicate target aliases became reachable with reverse_imports={reverse_imports} reverse_sources={reverse_sources}"
            );
        }
    }
}

fn duplicate_alias_reachability_module(
    reverse_imports: bool,
    reverse_sources: bool,
) -> veln_ast::SurfaceModule {
    let (first_import, second_import) = if reverse_imports {
        ("use shared as second\n", "use shared as first\n")
    } else {
        ("use shared as first\n", "use shared as second\n")
    };
    let first_app_source = SourceFile::new(
        "app-first.veln",
        format!(
            concat!(
                "mod app\n",
                "{import}",
                "fn accepts(job: fn() -> Int) -> Int\n",
                "  job()\n",
                "end\n",
                "pub fn main() -> Int\n",
                "  first::answer() + accepts(second::answer)\n",
                "end\n",
            ),
            import = first_import,
        ),
    );
    let second_app_source = SourceFile::new("app-second.veln", format!("mod app\n{second_import}"));
    let mut module = lower_module_source(&first_app_source, "app");
    let second = lower_module_source(&second_app_source, "app");
    if reverse_sources {
        let mut uses = second.uses;
        uses.extend(module.uses);
        module.uses = uses;
    } else {
        module.uses.extend(second.uses);
    }
    let shared_source = SourceFile::new(
        "shared.veln",
        "mod shared\npub fn answer() -> Int\n  40\nend\n",
    );
    module
        .functions
        .extend(lower_module_source(&shared_source, "shared").functions);
    module
}

fn lower_module_source(source: &SourceFile, identity: &str) -> veln_ast::SurfaceModule {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty());
    veln_ast::lower_surface_ast_with_module_identity(
        &parsed.tree,
        identity.to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    )
}

#[test]
fn duplicate_target_aliases_do_not_select_constructor_over_function_recovery() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "mod app\n",
                    "use shared as first\n",
                    "use shared as second\n",
                    "pub fn main() -> Int\n",
                    "  first::Build()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "shared.veln",
                concat!(
                    "mod shared\n",
                    "pub type Item\n",
                    "  pub Build\n",
                    "end\n",
                    "pub fn Build() -> Int\n",
                    "  40\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, _) = load_surface_module(&project);

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    assert_eq!(reachable_function_names(&reachable), [("app", "main")]);
    assert!(
        reachable.invalid_names.is_empty(),
        "a colliding constructor route retained function recovery: {:#?}",
        reachable.invalid_names
    );
}

#[test]
fn duplicate_target_aliases_do_not_reach_invalid_function_recovery() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "mod app\n",
                    "use shared as first\n",
                    "use shared as second\n",
                    "pub fn main() -> Int\n",
                    "  first::Bad()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "shared.veln",
                "mod shared\npub fn Bad() -> Int\n  40\nend\n",
            ),
        ],
        manifest: None,
    };
    let (module, _) = load_surface_module(&project);

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    assert_eq!(reachable_function_names(&reachable), [("app", "main")]);
    assert!(
        reachable.invalid_names.is_empty(),
        "a colliding import retained invalid function recovery: {:#?}",
        reachable.invalid_names
    );
}

#[test]
fn duplicate_unaliased_targets_and_qualifier_collisions_do_not_reach_functions() {
    let cases = [
        ("use shared\nuse shared\n", "answer() + shared::answer()"),
        (
            "use first::shared as route\nuse second::shared as route\n",
            "route::answer()",
        ),
    ];
    for (imports, body) in cases {
        for reverse in [false, true] {
            let imports = if reverse {
                imports.lines().rev().collect::<Vec<_>>().join("\n") + "\n"
            } else {
                imports.to_string()
            };
            let app = SourceFile::new(
                "app.veln",
                format!("mod app\n{imports}pub fn main() -> Int\n  {body}\nend\n"),
            );
            let first = SourceFile::new(
                "first/shared.veln",
                "mod first::shared\npub fn answer() -> Int\n  20\nend\n",
            );
            let second = SourceFile::new(
                "second/shared.veln",
                "mod second::shared\npub fn answer() -> Int\n  20\nend\n",
            );
            let shared = SourceFile::new(
                "shared.veln",
                "mod shared\npub fn answer() -> Int\n  40\nend\n",
            );
            let (module, _) = load_surface_module(&Project {
                root: ".".into(),
                files: vec![app, first, second, shared],
                manifest: None,
            });

            let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
            assert_eq!(reachable_function_names(&reachable), [("app", "main")]);
        }
    }
}

#[test]
fn duplicate_exact_tombstones_block_competing_qualifiers_but_not_independent_exact_routes() {
    for reverse in [false, true] {
        let imports = if reverse {
            concat!(
                "use clean\n",
                "use other as shared\n",
                "use shared as second\n",
                "use shared as first\n",
            )
        } else {
            concat!(
                "use shared as first\n",
                "use shared as second\n",
                "use other as shared\n",
                "use clean\n",
            )
        };
        let project = Project {
            root: ".".into(),
            files: vec![
                SourceFile::new(
                    "app.veln",
                    format!(
                        "mod app\n{imports}pub fn main() -> Int\n  shared::answer() + clean::answer()\nend\n"
                    ),
                ),
                SourceFile::new(
                    "shared.veln",
                    "mod shared\npub fn answer() -> Int\n  10\nend\n",
                ),
                SourceFile::new(
                    "other.veln",
                    "mod other\npub fn answer() -> Int\n  20\nend\n",
                ),
                SourceFile::new(
                    "clean.veln",
                    "mod clean\npub fn answer() -> Int\n  30\nend\n",
                ),
            ],
            manifest: None,
        };
        let (module, _) = load_surface_module(&project);

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        assert_eq!(
            reachable_function_names(&reachable),
            [("app", "main"), ("clean", "answer")]
        );
    }
}

#[test]
fn duplicate_targets_do_not_reach_opaque_callback_candidates() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "mod app\n",
                    "use shared as first\n",
                    "use shared as second\n",
                    "fn invoke(job: fn() -> Int) -> Int\n",
                    "  job()\n",
                    "end\n",
                    "fn local_job() -> Int\n",
                    "  1\n",
                    "end\n",
                    "pub fn main() -> Int\n",
                    "  invoke(local_job)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "shared.veln",
                "mod shared\npub fn answer() -> Int\n  40\nend\n",
            ),
        ],
        manifest: None,
    };
    let (module, _) = load_surface_module(&project);

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    assert!(
        reachable_function_names(&reachable)
            .iter()
            .all(|(module, _)| *module != "shared"),
        "duplicate target leaked through opaque callback reachability"
    );
}

#[test]
fn duplicate_target_aliases_do_not_reach_handler_clause_callees() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "mod app\n",
                    "use shared as first\n",
                    "use shared as second\n",
                    "pub fn main() -> Int\n",
                    "  handle first::compute() with first::ask()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "shared.veln",
                concat!(
                    "mod shared\n",
                    "pub effect Ask\n",
                    "  value() -> Int\n",
                    "end\n",
                    "fn helper() -> Int\n",
                    "  40\n",
                    "end\n",
                    "pub handler ask() for Ask\n",
                    "  value() => helper()\n",
                    "end\n",
                    "pub fn compute() -> Int effects [Ask]\n",
                    "  perform Ask::value()\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, _) = load_surface_module(&project);

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    assert_eq!(reachable_function_names(&reachable), [("app", "main")]);
}

#[test]
fn invalid_and_reserved_import_aliases_never_retain_targets_through_reachability() {
    for import in ["use pkg::target as Bad\n", "use pkg::target as prelude\n"] {
        let app_source = SourceFile::new(
            "app.veln",
            format!(
                concat!(
                    "{import}",
                    "fn retained() -> Int\n",
                    "  1\n",
                    "end\n",
                    "pub fn main() -> Int\n",
                    "  retained() + Bad::member() + prelude::member() + pkg::target::member()\n",
                    "end\n",
                ),
                import = import,
            ),
        );
        let parsed_app = parse(&app_source);
        assert!(parsed_app.diagnostics.is_empty(), "{import:?}");
        let mut module = veln_ast::lower_surface_ast_with_module_identity(
            &parsed_app.tree,
            "app".to_string(),
            app_source.span(veln_source::TextRange::new(0, 0)),
        );

        let target_source =
            SourceFile::new("pkg/target.veln", "pub fn member() -> Int\n  40\nend\n");
        let parsed_target = parse(&target_source);
        assert!(parsed_target.diagnostics.is_empty());
        let target = veln_ast::lower_surface_ast_with_module_identity(
            &parsed_target.tree,
            "pkg::target".to_string(),
            target_source.span(veln_source::TextRange::new(0, 0)),
        );
        module.functions.extend(target.functions);

        let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
        assert_eq!(
            reachable
                .functions
                .iter()
                .filter_map(|function| Some((
                    function.module_name.as_deref()?,
                    function.name.as_deref()?,
                )))
                .collect::<Vec<_>>(),
            [("app", "retained"), ("app", "main")],
            "invalid import retained a target through a normal route: {import:?}"
        );
    }
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
