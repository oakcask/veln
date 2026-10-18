use super::*;

fn path(module: &str, name: &str) -> Vec<String> {
    vec![module.to_string(), name.to_string()]
}

fn path_type() -> Type {
    Type::named("Path", Vec::new())
}

fn cancel_owner_type() -> Type {
    Type::named("CancelOwner", Vec::new())
}

fn assert_standard_signature(module: &str, name: &str, params: Vec<Type>, return_type: Type) {
    assert_eq!(
        standard_library_signature(&path(module, name)),
        Some((params, return_type)),
        "{module}::{name} signature"
    );
}

#[test]
fn stdio_detection_comes_from_descriptor_effect_metadata() {
    assert!(is_stdio_call(&path("stdio", "println")));
    assert!(!is_stdio_call(&path("channel", "send")));
    assert!(!is_stdio_call(&path("stdio", "flush")));
}

#[test]
fn concurrency_detection_comes_from_descriptor_effect_metadata() {
    assert!(is_concurrency_call(&path("task", "spawn")));
    assert!(is_concurrency_call(&path("channel", "send")));
    assert!(!is_concurrency_call(&path("stdio", "println")));
    assert!(!is_concurrency_call(&path("task", "sleep")));
}

#[test]
fn fs_and_process_signatures_come_from_standard_descriptors() {
    let (params, return_type) =
        standard_library_signature(&path("fs", "read_to_string")).expect("fs signature");
    assert_eq!(params, vec![path_type()]);
    assert_eq!(
        return_type,
        Type::result(Type::string(), Type::named("FsError", Vec::new()))
    );

    let (_, return_type) =
        standard_library_signature(&path("process", "args")).expect("process signature");
    assert_eq!(return_type, Type::vec(Type::string()));
}

#[test]
fn pure_standard_calls_have_signatures_and_empty_effect_rows() {
    assert_standard_signature("fs", "path", vec![Type::string()], path_type());
    assert_standard_signature(
        "string",
        "concat",
        vec![Type::string(), Type::string()],
        Type::string(),
    );
    for (module, name) in [("fs", "path"), ("string", "concat")] {
        assert_eq!(standard_library_effects(&path(module, name)), Some(&[][..]));
        assert!(!is_stdio_call(&path(module, name)));
        assert!(!is_concurrency_call(&path(module, name)));
    }
}

#[test]
fn time_deadline_signatures_come_from_standard_descriptors() {
    assert_standard_signature("time", "monotonic_ms", Vec::new(), Type::int());
    assert_standard_signature("time", "wall_time", Vec::new(), Type::wall_time());
    assert_standard_signature("time", "timeout_ms", vec![Type::int()], Type::unit());
    for name in ["deadline_after_ms", "deadline_at_ms"] {
        assert_standard_signature(
            "time",
            name,
            vec![Type::int()],
            Type::named("Deadline", Vec::new()),
        );
    }
    assert_standard_signature(
        "time",
        "wait_until",
        vec![Type::named("Deadline", Vec::new())],
        Type::unit(),
    );
}

#[test]
fn time_cancellation_signatures_come_from_standard_descriptors() {
    assert_standard_signature("time", "cancel_token", Vec::new(), cancel_token_type());
    assert_standard_signature("time", "cancel_owner", Vec::new(), cancel_owner_type());
    assert_standard_signature(
        "time",
        "cancel_token_from",
        vec![cancel_owner_type()],
        cancel_token_type(),
    );
    assert_standard_signature(
        "time",
        "cancel_owned",
        vec![cancel_owner_type()],
        Type::unit(),
    );
    assert_standard_signature("time", "cancel", vec![cancel_token_type()], Type::unit());
    assert_standard_signature(
        "time",
        "is_cancelled",
        vec![cancel_token_type()],
        Type::bool(),
    );
    assert_standard_signature(
        "time",
        "is_cancelled_owner",
        vec![cancel_owner_type()],
        Type::bool(),
    );
    assert_standard_signature(
        "time",
        "wait_until_cancellable",
        vec![Type::named("Deadline", Vec::new()), cancel_token_type()],
        Type::unit(),
    );
    assert_standard_signature(
        "time",
        "wait_until_cancellable_outcome",
        vec![Type::named("Deadline", Vec::new()), cancel_token_type()],
        Type::named("CancellableWaitOutcome", Vec::new()),
    );
}

#[test]
fn standard_type_lowering_preserves_named_types_inside_containers() {
    let nested = StandardType::Result(
        &StandardType::Vec(&StandardType::Named("Path")),
        &StandardType::Named("ProcessError"),
    );

    assert_eq!(
        standard_type(&nested),
        Some(adt::result_type(
            Type::vec(Type::named("Path", Vec::new())),
            Type::named("ProcessError", Vec::new()),
        ))
    );
}
