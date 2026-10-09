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
            "System.getenv(\"VELN_CLEANUP_ERRORS\")",
            "System.getenv(\"VELN_RUNTIME_ERRORS\")",
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
        (
            "diagnostics.java.inc",
            "System.getenv(\"VELN_CLEANUP_ERRORS\")",
        ),
        (
            "diagnostics.java.inc",
            "System.getenv(\"VELN_RUNTIME_ERRORS\")",
        ),
    ] {
        assert!(ambient_effect_inputs(source, owner).is_empty());
        assert!(!ambient_effect_inputs(source, "new_adapter.java.inc").is_empty());
    }
}

#[test]
fn task_cancellation_state_is_task_local_and_not_a_retained_thread_registry() {
    let source = include_str!("../runtime/concurrency.java.inc");

    assert!(source.contains("ThreadLocal<TaskCancellation>"));
    assert!(!source.contains("CANCELLED_TASK_THREADS"));
    assert!(!source.contains("Set<Thread>"));
    assert!(!source.contains("Set<java.io.Closeable>"));
    assert!(!source.contains("blocker.close()"));
}

#[test]
fn concurrent_task_cancellation_uses_one_atomic_initiation_claim() {
    let source = include_str!("../runtime/concurrency.java.inc");

    assert!(source.contains("AtomicBoolean cancellationRequested"));
    assert!(source.contains("cancellationRequested.compareAndSet(false, true)"));
    assert!(!source.contains("!handle.cancellationRequested"));
}

#[test]
fn network_system_cleanup_uses_one_identity_ledger_without_activation_races() {
    let values = include_str!("../runtime/values.java.inc");
    let effects = include_str!("../runtime/effects.java.inc");

    assert!(values.contains("new java.util.IdentityHashMap<Object, Boolean>()"));
    assert!(values.contains("cleanupNetworkSystem(frame);"));
    assert!(!values.contains("if (frame.networkSystem)"));
    assert!(!values.contains("networkListeners"));
    assert!(!values.contains("networkStreams"));
    assert!(
        effects.contains("resources = new java.util.ArrayList<Object>(owner.networkResources);")
    );
    assert!(effects.contains("netSystemPublishResource(HandlerFrame owner, Object resource)"));
    assert!(effects.contains("netSystemCleanupResource(owner, resource);"));
    assert!(!effects.contains("owner.networkResources.clear();"));
}

#[test]
fn network_system_resolution_has_bounded_detached_workers() {
    let effects = include_str!("../runtime/effects.java.inc");

    assert!(effects.contains("NET_SYSTEM_RESOLVER_WORKERS = 4"));
    assert!(effects.contains("new java.util.concurrent.SynchronousQueue<Runnable>()"));
    assert!(effects.contains("new java.util.concurrent.ThreadPoolExecutor.AbortPolicy()"));
    assert!(effects.contains("HANDLERS.remove();"));
    assert!(!effects.contains("new Thread(resolution"));
}
