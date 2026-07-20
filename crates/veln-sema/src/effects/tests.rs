use super::*;

fn path(module: &str, name: &str) -> Vec<String> {
    vec![module.to_string(), name.to_string()]
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
fn net_and_time_signatures_come_from_standard_descriptors() {
    let (params, return_type) =
        standard_library_signature(&path("net", "receive_chunk")).expect("net signature");
    assert!(params.is_empty());
    assert_eq!(return_type, byte_chunk_type());

    let (params, return_type) =
        standard_library_signature(&path("net", "send_chunk")).expect("net signature");
    assert_eq!(params, vec![byte_chunk_type()]);
    assert_eq!(return_type, Type::unit());

    let (params, return_type) =
        standard_library_signature(&path("net", "connect")).expect("net signature");
    assert_eq!(params, vec![Type::string()]);
    assert_eq!(return_type, net_stream_type());

    let (params, return_type) =
        standard_library_signature(&path("net", "accept_until")).expect("net signature");
    assert_eq!(
        params,
        vec![net_listener_type(), Type::named("Deadline", Vec::new())]
    );
    assert_eq!(return_type, adt::option_type(net_stream_type()));

    let (params, return_type) =
        standard_library_signature(&path("net", "accept_until_cancellable"))
            .expect("net signature");
    assert_eq!(
        params,
        vec![
            net_listener_type(),
            Type::named("Deadline", Vec::new()),
            cancel_token_type()
        ]
    );
    assert_eq!(return_type, Type::named("AcceptOutcome", Vec::new()));

    let (params, return_type) =
        standard_library_signature(&path("net", "read_chunk_or_end")).expect("net signature");
    assert_eq!(params, vec![net_stream_type()]);
    assert_eq!(return_type, adt::option_type(byte_chunk_type()));

    let (params, return_type) =
        standard_library_signature(&path("net", "write_chunk_until")).expect("net signature");
    assert_eq!(
        params,
        vec![
            net_stream_type(),
            byte_chunk_type(),
            Type::named("Deadline", Vec::new())
        ]
    );
    assert_eq!(return_type, Type::named("StreamWriteOutcome", Vec::new()));

    let (params, return_type) =
        standard_library_signature(&path("net", "close_stream")).expect("net signature");
    assert_eq!(params, vec![net_stream_type()]);
    assert_eq!(return_type, Type::unit());

    let (params, return_type) =
        standard_library_signature(&path("net", "shutdown_write")).expect("net signature");
    assert_eq!(params, vec![net_stream_type()]);
    assert_eq!(return_type, Type::unit());

    let (params, return_type) =
        standard_library_signature(&path("net", "shutdown_read")).expect("net signature");
    assert_eq!(params, vec![net_stream_type()]);
    assert_eq!(return_type, Type::unit());

    let (params, return_type) =
        standard_library_signature(&path("time", "monotonic_ms")).expect("time signature");
    assert!(params.is_empty());
    assert_eq!(return_type, Type::int());

    let (params, return_type) =
        standard_library_signature(&path("time", "timeout_ms")).expect("time signature");
    assert_eq!(params, vec![Type::int()]);
    assert_eq!(return_type, Type::unit());

    let (params, return_type) =
        standard_library_signature(&path("time", "deadline_after_ms")).expect("time signature");
    assert_eq!(params, vec![Type::int()]);
    assert_eq!(return_type, Type::named("Deadline", Vec::new()));

    let (params, return_type) =
        standard_library_signature(&path("time", "deadline_at_ms")).expect("time signature");
    assert_eq!(params, vec![Type::int()]);
    assert_eq!(return_type, Type::named("Deadline", Vec::new()));

    let (params, return_type) =
        standard_library_signature(&path("time", "wait_until")).expect("time signature");
    assert_eq!(params, vec![Type::named("Deadline", Vec::new())]);
    assert_eq!(return_type, Type::unit());

    let (params, return_type) =
        standard_library_signature(&path("time", "cancel_token")).expect("time signature");
    assert!(params.is_empty());
    assert_eq!(return_type, cancel_token_type());

    let (params, return_type) =
        standard_library_signature(&path("time", "cancel_owner")).expect("time signature");
    assert!(params.is_empty());
    assert_eq!(return_type, cancel_owner_type());

    let (params, return_type) =
        standard_library_signature(&path("time", "cancel_token_from")).expect("time signature");
    assert_eq!(params, vec![cancel_owner_type()]);
    assert_eq!(return_type, cancel_token_type());

    let (params, return_type) =
        standard_library_signature(&path("time", "cancel_owned")).expect("time signature");
    assert_eq!(params, vec![cancel_owner_type()]);
    assert_eq!(return_type, Type::unit());

    let (params, return_type) =
        standard_library_signature(&path("time", "cancel")).expect("time signature");
    assert_eq!(params, vec![cancel_token_type()]);
    assert_eq!(return_type, Type::unit());

    let (params, return_type) =
        standard_library_signature(&path("time", "is_cancelled")).expect("time signature");
    assert_eq!(params, vec![cancel_token_type()]);
    assert_eq!(return_type, Type::bool());

    let (params, return_type) =
        standard_library_signature(&path("time", "is_cancelled_owner")).expect("time signature");
    assert_eq!(params, vec![cancel_owner_type()]);
    assert_eq!(return_type, Type::bool());

    let (params, return_type) = standard_library_signature(&path("time", "wait_until_cancellable"))
        .expect("time signature");
    assert_eq!(
        params,
        vec![Type::named("Deadline", Vec::new()), cancel_token_type()]
    );
    assert_eq!(return_type, Type::unit());

    let (params, return_type) =
        standard_library_signature(&path("time", "wait_until_cancellable_outcome"))
            .expect("time signature");
    assert_eq!(
        params,
        vec![Type::named("Deadline", Vec::new()), cancel_token_type()]
    );
    assert_eq!(
        return_type,
        Type::named("CancellableWaitOutcome", Vec::new())
    );
}
