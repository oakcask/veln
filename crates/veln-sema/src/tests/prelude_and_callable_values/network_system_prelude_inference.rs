use super::*;

const NETWORK_SYSTEM_ADAPTERS: &[&str] = &[
    "net_system_resolve",
    "net_system_listen",
    "net_system_connect",
    "net_system_accept",
    "net_system_listener_address",
    "net_system_close_listener",
    "net_system_read",
    "net_system_write",
    "net_system_local_address",
    "net_system_peer_address",
    "net_system_shutdown_read",
    "net_system_shutdown_write",
    "net_system_close_stream",
];

#[test]
fn network_system_adapters_lower_as_prelude_builtins() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> {resolved: Int, listener: Int, connected: Int, accepted: Int, ",
            "listener_address: Int, listener_closed: Int, read: Int, written: Int, ",
            "local_address: Int, peer_address: Int, read_shutdown: Int, write_shutdown: Int, ",
            "stream_closed: Int}\n",
            "  {resolved: prelude_builtin::net_system_resolve(0), ",
            "listener: prelude_builtin::net_system_listen(0), ",
            "connected: prelude_builtin::net_system_connect(0, 0, 0), ",
            "accepted: prelude_builtin::net_system_accept(0, 0, 0), ",
            "listener_address: prelude_builtin::net_system_listener_address(0), ",
            "listener_closed: prelude_builtin::net_system_close_listener(0), ",
            "read: prelude_builtin::net_system_read(0, 0, 0), ",
            "written: prelude_builtin::net_system_write(0, 0, 0, 0), ",
            "local_address: prelude_builtin::net_system_local_address(0), ",
            "peer_address: prelude_builtin::net_system_peer_address(0), ",
            "read_shutdown: prelude_builtin::net_system_shutdown_read(0), ",
            "write_shutdown: prelude_builtin::net_system_shutdown_write(0), ",
            "stream_closed: prelude_builtin::net_system_close_stream(0)}\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let CoreStmtKind::Return { expr } = &core.functions[0].body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Record(core_fields) = &expr.kind else {
        panic!("network adapter results should form a core record");
    };
    let core_names = core_fields
        .iter()
        .map(|field| match &field.expr.kind {
            CoreExprKind::Call {
                target: CoreCallTarget::PreludeBuiltin(name),
                ..
            } => name.as_str(),
            other => panic!("network adapter should lower to a core prelude call: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(core_names, NETWORK_SYSTEM_ADAPTERS);

    let ir = lowered.ir.expect("network adapter core should lower to IR");
    let IrStmtKind::Return { value } = &ir.functions[0].body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Record(ir_fields) = &value.kind else {
        panic!("network adapter results should form an IR record");
    };
    let ir_names = ir_fields
        .iter()
        .map(|field| match &field.value.kind {
            IrExprKind::Call {
                target: IrCallTarget::PreludeBuiltin(name),
                ..
            } => name.as_str(),
            other => panic!("network adapter should lower to an IR prelude call: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(ir_names, NETWORK_SYSTEM_ADAPTERS);
}
