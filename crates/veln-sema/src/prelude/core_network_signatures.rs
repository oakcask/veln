use veln_core::CoreType;

pub(super) fn core_net_system_signature(
    name: &str,
    expected: &CoreType,
) -> Option<(Vec<CoreType>, CoreType)> {
    let arity = match name {
        "net_system_resolve"
        | "net_system_listen"
        | "net_system_listener_address"
        | "net_system_close_listener"
        | "net_system_local_address"
        | "net_system_peer_address"
        | "net_system_shutdown_read"
        | "net_system_shutdown_write"
        | "net_system_close_stream" => 1,
        "net_system_connect" | "net_system_accept" | "net_system_read" => 3,
        "net_system_write" => 4,
        _ => return None,
    };
    Some((vec![CoreType::Unknown; arity], expected.clone()))
}
