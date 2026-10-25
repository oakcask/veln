use super::*;

#[test]
fn explicit_import_alias_casing_reports_exact_token_and_quarantines_every_route() {
    for (alias, observed) in [
        ("Bad", "ascii_uppercase"),
        ("_bad", "underscore"),
        ("_", "underscore"),
    ] {
        let source = SourceFile::new(
            "app.veln",
            format!("mod app\nuse target as {alias}\nfn retained() -> ()\n  ()\nend\n"),
        );
        let module = lower_surface_ast(&parse(&source).tree);
        let diagnostics = analyze_surface_module(&module);
        let invalid = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "name.invalid_case")
            .expect("invalid explicit alias diagnostic");

        assert_diagnostic_span(invalid, 2, 15, 2, 15 + alias.len());
        let details = invalid.details.to_json();
        for fragment in [
            format!("\"name\":\"{alias}\""),
            "\"name_class\":\"module\"".to_string(),
            "\"occurrence\":\"declaration\"".to_string(),
            "\"required_initial\":\"ascii_lowercase\"".to_string(),
            format!("\"observed_initial\":\"{observed}\""),
        ] {
            assert!(details.contains(&fragment), "{details}");
        }
        for route in [alias, "target"] {
            assert!(
                crate::name_recovery::normal_imported_use_for_path(
                    &module,
                    &[route.to_string()],
                    Some("app")
                )
                .is_none(),
                "invalid alias must quarantine route {route:?}"
            );
        }
    }
}

#[test]
fn invalid_target_path_quarantines_a_valid_explicit_alias_atomically() {
    let source = SourceFile::new(
        "app.veln",
        "mod app\nuse Bad as good\nfn retained() -> ()\n  ()\nend\n",
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    for route in ["Bad", "good"] {
        assert!(
            crate::name_recovery::normal_imported_use_for_path(
                &module,
                &[route.to_string()],
                Some("app")
            )
            .is_none(),
            "invalid target path must quarantine route {route:?}"
        );
    }
}

#[test]
fn invalid_explicit_import_alias_recovers_all_public_carriers_without_readiness() {
    let module = merged_modules_with_identities(vec![
        (
            "http",
            SourceFile::new(
                "http.veln",
                concat!(
                    "pub fn answer() -> Int\n",
                    "  1\n",
                    "end\n",
                    "pub type Payload\n",
                    "  pub Data(Int)\n",
                    "end\n",
                    "pub effect Audit\n",
                    "  record() -> Int\n",
                    "end\n",
                    "pub handler audit() for Audit\n",
                    "  record() => 1\n",
                    "end\n",
                    "pub fn compute() -> Int effects [Audit]\n",
                    "  perform Audit::record()\n",
                    "end\n",
                    "pub schema Packet\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n",
                ),
            ),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use http as HTTP\n",
                    "schema Envelope\n",
                    "  format binary\n",
                    "  packet: HTTP::Packet\n",
                    "end\n",
                    "fn call() -> Int\n",
                    "  HTTP::answer()\n",
                    "end\n",
                    "fn construct() -> HTTP::Payload\n",
                    "  HTTP::Payload::Data(1)\n",
                    "end\n",
                    "fn inspect(value: HTTP::Payload) -> Int\n",
                    "  match value\n",
                    "    HTTP::Payload::Data(payload) => payload\n",
                    "  end\n",
                    "end\n",
                    "fn invoke() -> Int effects [HTTP::Audit]\n",
                    "  perform HTTP::Audit::record()\n",
                    "end\n",
                    "fn handled() -> Int\n",
                    "  handle HTTP::compute() with HTTP::audit()\n",
                    "end\n",
                    "fn decode_packet(view: ByteView, base: ByteOffset) -> DecodeStep<{value: Int}>\n",
                    "  decode HTTP::Packet from view at base\n",
                    "end\n",
                    "fn encode_packet(value: {value: Int}) -> Result<ByteChunk, EncodeError>\n",
                    "  encode HTTP::Packet from value\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "name.invalid_case"),
        "only independent casing diagnostics may remain: {diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic
                .details
                .to_json()
                .contains("\"occurrence\":\"declaration\""))
            .count(),
        1,
        "{diagnostics:#?}"
    );
    for route in ["HTTP", "http"] {
        assert!(
            crate::name_recovery::normal_imported_use_for_path(
                &module,
                &[route.to_string()],
                Some("app")
            )
            .is_none(),
            "invalid alias imports must not install normal route {route:?}"
        );
    }
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn invalid_implicit_import_alias_suppresses_only_quarantine_cascade() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main() -> Int\n",
            "  HTTP::entry()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!("mod http\n", "pub fn entry() -> Int\n", "  1\n", "end\n"),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "module.missing_identity"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_suppresses_public_effect_quarantine_cascade() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main() -> () effects [HTTP::Audit]\n",
            "  ()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub effect Audit\n",
            "  record() -> ()\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "module.missing_identity"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "effect.unknown"),
        "{diagnostics:#?}"
    );
    let environment = TypeEnvironment::from_module(&module);
    let main = environment
        .function("main")
        .expect("valid function signature remains available");
    assert!(main.effects.is_empty());
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn invalid_implicit_import_alias_preserves_missing_private_and_wrong_kind_effects() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn missing() -> () effects [HTTP::Missing]\n",
            "  ()\n",
            "end\n",
            "\n",
            "fn private() -> () effects [HTTP::Audit]\n",
            "  ()\n",
            "end\n",
            "\n",
            "fn wrong_kind() -> () effects [HTTP::entry]\n",
            "  ()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "effect Audit\n",
            "  record() -> ()\n",
            "end\n",
            "pub fn entry() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "effect.unknown",
            "effect.unknown",
            "effect.unknown",
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_suppresses_public_handler_quarantine_cascade() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn body() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn main() -> Int\n",
            "  handle body() with HTTP::audit()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub effect Audit\n",
            "  record() -> Int\n",
            "end\n",
            "pub handler audit() for Audit\n",
            "  record() => 1\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "module.missing_identity"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "handler.unknown"),
        "{diagnostics:#?}"
    );
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn invalid_implicit_import_alias_preserves_missing_private_and_wrong_kind_handlers() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn body() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn missing() -> Int\n",
            "  handle body() with HTTP::missing()\n",
            "end\n",
            "\n",
            "fn private() -> Int\n",
            "  handle body() with HTTP::private()\n",
            "end\n",
            "\n",
            "fn wrong_kind() -> Int\n",
            "  handle body() with HTTP::Audit()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub effect Audit\n",
            "  record() -> Int\n",
            "end\n",
            "handler private() for Audit\n",
            "  record() => 1\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "handler.unknown",
            "handler.unknown",
            "handler.unknown",
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_preserves_private_call_target() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main() -> Int\n",
            "  HTTP::entry()\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!("mod http\n", "fn entry() -> Int\n", "  1\n", "end\n"),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "name.unresolved"
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_suppresses_constructor_quarantine_cascade() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main() -> HTTP::Payload\n",
            "  HTTP::Payload::Data(1)\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub type Payload\n",
            "  pub Data(Int)\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    assert_eq!(module.uses[0].alias, "HTTP");
    assert_eq!(module.uses[0].module_name.as_deref(), Some("app"));
    assert_eq!(module.types[0].module_name.as_deref(), Some("HTTP"));
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment.quarantined_import_constructor_recovery_candidate_count(
            &[
                "HTTP".to_string(),
                "Payload".to_string(),
                "Data".to_string()
            ],
            Some("app"),
            Some(1),
        ),
        1
    );
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved"
                && diagnostic.id != "type.mismatch"
                && diagnostic.id != "core.constructor_arity_mismatch"
        }),
        "{diagnostics:#?}"
    );
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn invalid_implicit_import_alias_does_not_infer_private_signature_type() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main(value: HTTP::Payload) -> Int\n",
            "  value\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub type Payload\n",
            "  pub Data(Int)\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
    assert!(
        TypeEnvironment::from_module(&module)
            .uses
            .iter()
            .all(|use_decl| use_decl.alias != "HTTP")
    );
}

#[test]
fn invalid_implicit_import_alias_preserves_private_schema_composition() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "schema Packet\n",
            "  payload: HTTP::Wire\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!("mod http\n", "schema Wire\n", "  payload: Int\n", "end\n"),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "schema.composition_reference"
        ],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics[2]
            .details
            .to_json()
            .contains("\"reason\":\"private_schema\""),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_preserves_missing_schema_composition() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "schema Packet\n",
            "  payload: HTTP::Missing\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&app_source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "schema.composition_reference"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics[1]
            .details
            .to_json()
            .contains("\"reason\":\"missing_schema\""),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_implicit_import_alias_preserves_missing_type_export() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "\n",
            "fn main(value: HTTP::Payload) -> Int\n",
            "  value\n",
            "end\n",
        ),
    );
    let http_source = SourceFile::new(
        "http.veln",
        concat!(
            "mod http\n",
            "pub type Other\n",
            "  pub Data(Int)\n",
            "end\n",
        ),
    );
    let module = merged_app_and_http_modules(app_source, http_source);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.missing_identity",
            "type.mismatch"
        ],
        "{diagnostics:#?}"
    );
    assert_eq!(
        TypeEnvironment::from_module(&module).quarantined_import_type_recovery_candidate_count(
            "HTTP::Payload",
            Some("app"),
            0,
        ),
        0
    );
}

fn merged_app_and_http_modules(app_source: SourceFile, http_source: SourceFile) -> SurfaceModule {
    let app_module = lower_surface_ast(&parse(&app_source).tree);
    let mut http_module = lower_surface_ast(&parse(&http_source).tree);
    assign_module_name(&mut http_module, "HTTP");
    merge_surface_modules(vec![app_module, http_module])
}

fn assign_module_name(module: &mut SurfaceModule, name: &str) {
    for use_decl in &mut module.uses {
        use_decl.module_name = Some(name.to_string());
    }
    for alias in &mut module.aliases {
        alias.module_name = Some(name.to_string());
    }
    for effect in &mut module.effects {
        effect.module_name = Some(name.to_string());
    }
    for handler in &mut module.handlers {
        handler.module_name = Some(name.to_string());
    }
    for schema in &mut module.schemas {
        schema.module_name = Some(name.to_string());
    }
    for type_decl in &mut module.types {
        type_decl.module_name = Some(name.to_string());
    }
    for function in &mut module.functions {
        function.module_name = Some(name.to_string());
    }
}

fn merge_surface_modules(modules: Vec<SurfaceModule>) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for module in modules {
        merged.uses.extend(module.uses);
        merged.aliases.extend(module.aliases);
        merged.effects.extend(module.effects);
        merged.handlers.extend(module.handlers);
        merged.schemas.extend(module.schemas);
        merged.types.extend(module.types);
        merged.functions.extend(module.functions);
        merged.invalid_names.extend(module.invalid_names);
    }
    merged
}

#[test]
fn duplicate_invalid_implicit_import_aliases_stay_in_duplicate_analysis() {
    let source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use HTTP\n",
            "use HTTP\n",
            "\n",
            "fn main() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "name.invalid_case", "name.duplicate"],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[2].message, "duplicate import path name `HTTP`",
        "{diagnostics:#?}"
    );
}

#[test]
fn quarantined_import_alias_use_reports_unresolved_when_target_is_missing() {
    let source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use missing::HTTP\n",
            "\n",
            "fn main() -> Int\n",
            "  HTTP::entry()\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "name.unresolved"],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[1].message, "unresolved call_target `HTTP::entry`",
        "{diagnostics:#?}"
    );
}
