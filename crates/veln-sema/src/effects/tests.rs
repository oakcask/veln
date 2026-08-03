use super::*;

fn path(module: &str, name: &str) -> Vec<String> {
    vec![module.to_string(), name.to_string()]
}

fn path_type() -> Type {
    Type::named("Path", Vec::new())
}

fn byte_chunk_type() -> Type {
    Type::named("ByteChunk", Vec::new())
}

fn net_listener_type() -> Type {
    Type::named("NetListener", Vec::new())
}

fn net_stream_type() -> Type {
    Type::named("NetStream", Vec::new())
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
fn net_listener_signatures_come_from_standard_descriptors() {
    assert_standard_signature("net", "listen", vec![Type::string()], net_listener_type());
    assert_standard_signature(
        "net",
        "accept",
        vec![net_listener_type()],
        net_stream_type(),
    );
    assert_standard_signature(
        "net",
        "accept_or_end",
        vec![net_listener_type()],
        adt::option_type(net_stream_type()),
    );
    assert_standard_signature(
        "net",
        "accept_until",
        vec![net_listener_type(), Type::named("Deadline", Vec::new())],
        adt::option_type(net_stream_type()),
    );
    assert_standard_signature(
        "net",
        "accept_until_cancellable",
        vec![
            net_listener_type(),
            Type::named("Deadline", Vec::new()),
            cancel_token_type(),
        ],
        Type::named("AcceptOutcome", Vec::new()),
    );
    assert_standard_signature(
        "net",
        "listener_local_addr",
        vec![net_listener_type()],
        Type::string(),
    );
    assert_standard_signature(
        "net",
        "close_listener",
        vec![net_listener_type()],
        Type::unit(),
    );
}

#[test]
fn net_connection_and_stream_state_signatures_come_from_standard_descriptors() {
    assert_standard_signature("net", "receive_chunk", Vec::new(), byte_chunk_type());
    assert_standard_signature("net", "send_chunk", vec![byte_chunk_type()], Type::unit());
    assert_standard_signature("net", "connect", vec![Type::string()], net_stream_type());
    for name in ["stream_local_addr", "stream_peer_addr"] {
        assert_standard_signature("net", name, vec![net_stream_type()], Type::string());
    }
    for name in ["stream_can_read", "stream_can_write", "stream_is_closed"] {
        assert_standard_signature("net", name, vec![net_stream_type()], Type::bool());
    }
    for name in ["close_stream", "shutdown_write", "shutdown_read"] {
        assert_standard_signature("net", name, vec![net_stream_type()], Type::unit());
    }
}

#[test]
fn net_read_signatures_come_from_standard_descriptors() {
    assert_standard_signature(
        "net",
        "read_chunk",
        vec![net_stream_type()],
        byte_chunk_type(),
    );
    assert_standard_signature(
        "net",
        "read_chunk_or_end",
        vec![net_stream_type()],
        adt::option_type(byte_chunk_type()),
    );
    assert_standard_signature(
        "net",
        "read_chunk_until",
        vec![net_stream_type(), Type::named("Deadline", Vec::new())],
        adt::option_type(byte_chunk_type()),
    );
    assert_standard_signature(
        "net",
        "read_chunk_until_cancellable",
        vec![
            net_stream_type(),
            Type::named("Deadline", Vec::new()),
            cancel_token_type(),
        ],
        Type::named("StreamReadOutcome", Vec::new()),
    );
}

#[test]
fn net_write_signatures_come_from_standard_descriptors() {
    let deadline = || Type::named("Deadline", Vec::new());
    let outcome = || Type::named("StreamWriteOutcome", Vec::new());
    assert_standard_signature(
        "net",
        "write_chunk",
        vec![net_stream_type(), byte_chunk_type()],
        Type::unit(),
    );
    assert_standard_signature(
        "net",
        "write_chunk_until",
        vec![net_stream_type(), byte_chunk_type(), deadline()],
        outcome(),
    );
    assert_standard_signature(
        "net",
        "write_chunk_until_cancellable",
        vec![
            net_stream_type(),
            byte_chunk_type(),
            deadline(),
            cancel_token_type(),
        ],
        outcome(),
    );
    assert_standard_signature(
        "net",
        "write_chunks",
        vec![net_stream_type(), adt::list_type(byte_chunk_type())],
        Type::unit(),
    );
    assert_standard_signature(
        "net",
        "write_chunks_until",
        vec![
            net_stream_type(),
            adt::list_type(byte_chunk_type()),
            deadline(),
        ],
        outcome(),
    );
    assert_standard_signature(
        "net",
        "write_chunks_until_cancellable",
        vec![
            net_stream_type(),
            adt::list_type(byte_chunk_type()),
            deadline(),
            cancel_token_type(),
        ],
        outcome(),
    );
}

#[test]
fn time_deadline_signatures_come_from_standard_descriptors() {
    assert_standard_signature("time", "monotonic_ms", Vec::new(), Type::int());
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
