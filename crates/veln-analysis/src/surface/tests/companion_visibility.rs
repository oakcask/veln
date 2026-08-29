use super::*;

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
