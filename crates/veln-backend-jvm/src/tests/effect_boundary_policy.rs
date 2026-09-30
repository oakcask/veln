use std::path::Path;

fn ambient_effect_inputs(source: &str, owner: &str) -> Vec<&'static str> {
    let mut compact: String = source.chars().filter(|ch| !ch.is_whitespace()).collect();
    let allowed: &[&str] = match owner {
        "effects.java.inc" => &[
            "System.getenv(asString(name))",
            "System.getenv(\"VELN_TRANSPORT_ERRORS\")",
        ],
        "stdio.java.inc" => &["System.getenv(\"VELN_STDIO_EVENTS\")"],
        "diagnostics.java.inc" => &[
            "System.getenv(\"VELN_CONTRACT_ERRORS\")",
            "System.getenv(\"VELN_RESULT_ERRORS\")",
        ],
        _ => &[],
    };
    for call in allowed {
        compact = compact.replace(call, "");
    }
    [
        "System.getenv(",
        "System.getProperty(",
        "System.getProperties(",
    ]
    .into_iter()
    .filter(|call| compact.contains(*call))
    .collect()
}

fn check_runtime_directory(root: &Path) {
    for entry in std::fs::read_dir(root).expect("runtime directory should be readable") {
        let path = entry.expect("runtime entry should be readable").path();
        if path.is_dir() {
            check_runtime_directory(&path);
            continue;
        }
        let owner = path.file_name().unwrap().to_str().unwrap();
        if !owner.ends_with(".java") && !owner.ends_with(".java.inc") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("runtime source should be readable");
        assert!(
            ambient_effect_inputs(&source, owner).is_empty(),
            "{owner}: inject scoped Veln effect handlers for deterministic tests; ambient \
             configuration changes production behavior. Environment access belongs only \
             to process::env and diagnostic output destinations."
        );
    }
}

#[test]
fn runtime_effects_do_not_select_test_behavior_from_ambient_configuration() {
    check_runtime_directory(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"));
}

#[test]
fn ambient_policy_rejects_fake_inputs_and_property_selectors() {
    for source in [
        "return System.getenv(\"EXAMPLE_FAKE_CLOCK\");",
        "return System . getProperty (\"example.fake.adapter\");",
        "return System.getProperties();",
    ] {
        assert!(!ambient_effect_inputs(source, "clock.java.inc").is_empty());
    }
}

#[test]
fn ambient_policy_permits_only_existing_process_and_diagnostic_owners() {
    for (owner, source) in [
        ("effects.java.inc", "System.getenv(asString(name))"),
        (
            "effects.java.inc",
            "System.getenv(\"VELN_TRANSPORT_ERRORS\")",
        ),
        ("stdio.java.inc", "System.getenv(\"VELN_STDIO_EVENTS\")"),
        (
            "diagnostics.java.inc",
            "System.getenv(\"VELN_RESULT_ERRORS\")",
        ),
        (
            "diagnostics.java.inc",
            "System.getenv(\"VELN_CONTRACT_ERRORS\")",
        ),
    ] {
        assert!(ambient_effect_inputs(source, owner).is_empty());
        assert!(!ambient_effect_inputs(source, "new_adapter.java.inc").is_empty());
    }
}
