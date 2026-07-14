#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StandardSymbolKind {
    Runtime,
    Prelude,
    Veln,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StandardSymbolDescriptor {
    pub(crate) module: Option<&'static str>,
    pub(crate) name: &'static str,
    pub(crate) kind: StandardSymbolKind,
    pub(crate) effects: &'static [&'static str],
    pub(crate) lowering: Option<&'static str>,
    pub(crate) signature: Option<StandardSignature>,
    pub(crate) source: Option<veln_stdlib::StdlibSource>,
    pub(crate) stability: StandardSymbolStability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StandardSignature {
    pub(crate) params: &'static [StandardType],
    pub(crate) return_type: StandardType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StandardType {
    Bool,
    Int,
    String,
    Unit,
    Path,
    FsError,
    ProcessError,
    ByteChunk,
    NetListener,
    NetStream,
    Deadline,
    CancelOwner,
    CancelToken,
    AcceptOutcome,
    StreamReadOutcome,
    StreamWriteOutcome,
    CancellableWaitOutcome,
    Vec(&'static StandardType),
    List(&'static StandardType),
    Option(&'static StandardType),
    Result(&'static StandardType, &'static StandardType),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StandardSymbolStability {
    RequiredForSelfHosting,
    CompatibilityOnly,
}

const STDIO_EFFECTS: &[&str] = &["stdio"];
const CONCURRENCY_EFFECTS: &[&str] = &["concurrency"];
const TIME_CONCURRENCY_EFFECTS: &[&str] = &["time", "concurrency"];
const FS_EFFECTS: &[&str] = &["fs"];
const NET_EFFECTS: &[&str] = &["net"];
const NET_TIME_EFFECTS: &[&str] = &["net", "time"];
const PROCESS_EFFECTS: &[&str] = &["process"];
const TIME_EFFECTS: &[&str] = &["time"];
const PURE_EFFECTS: &[&str] = &[];
const STRING_TYPE: StandardType = StandardType::String;
const UNIT_TYPE: StandardType = StandardType::Unit;
const BOOL_TYPE: StandardType = StandardType::Bool;
const PATH_TYPE: StandardType = StandardType::Path;
const FS_ERROR_TYPE: StandardType = StandardType::FsError;
const PROCESS_ERROR_TYPE: StandardType = StandardType::ProcessError;
const BYTE_CHUNK_TYPE: StandardType = StandardType::ByteChunk;
const NET_STREAM_TYPE: StandardType = StandardType::NetStream;
const ACCEPT_OUTCOME_TYPE: StandardType = StandardType::AcceptOutcome;
const STREAM_READ_OUTCOME_TYPE: StandardType = StandardType::StreamReadOutcome;
const STREAM_WRITE_OUTCOME_TYPE: StandardType = StandardType::StreamWriteOutcome;
const RESULT_STRING_FS_ERROR_TYPE: StandardType =
    StandardType::Result(&STRING_TYPE, &FS_ERROR_TYPE);
const RESULT_UNIT_FS_ERROR_TYPE: StandardType = StandardType::Result(&UNIT_TYPE, &FS_ERROR_TYPE);
const RESULT_BOOL_FS_ERROR_TYPE: StandardType = StandardType::Result(&BOOL_TYPE, &FS_ERROR_TYPE);
const VEC_PATH_TYPE: StandardType = StandardType::Vec(&PATH_TYPE);
const RESULT_VEC_PATH_FS_ERROR_TYPE: StandardType =
    StandardType::Result(&VEC_PATH_TYPE, &FS_ERROR_TYPE);
const OPTION_NET_STREAM_TYPE: StandardType = StandardType::Option(&NET_STREAM_TYPE);
const OPTION_BYTE_CHUNK_TYPE: StandardType = StandardType::Option(&BYTE_CHUNK_TYPE);
const LIST_BYTE_CHUNK_TYPE: StandardType = StandardType::List(&BYTE_CHUNK_TYPE);
const VEC_STRING_TYPE: StandardType = StandardType::Vec(&STRING_TYPE);
const OPTION_STRING_TYPE: StandardType = StandardType::Option(&STRING_TYPE);
const RESULT_PATH_PROCESS_ERROR_TYPE: StandardType =
    StandardType::Result(&PATH_TYPE, &PROCESS_ERROR_TYPE);
const PARAM_PATH: &[StandardType] = &[StandardType::Path];
const PARAM_PATH_STRING: &[StandardType] = &[StandardType::Path, StandardType::String];
const PARAM_BYTE_CHUNK: &[StandardType] = &[StandardType::ByteChunk];
const PARAM_STRING: &[StandardType] = &[StandardType::String];
const PARAM_NET_LISTENER: &[StandardType] = &[StandardType::NetListener];
const PARAM_NET_LISTENER_DEADLINE: &[StandardType] =
    &[StandardType::NetListener, StandardType::Deadline];
const PARAM_NET_LISTENER_DEADLINE_CANCEL_TOKEN: &[StandardType] = &[
    StandardType::NetListener,
    StandardType::Deadline,
    StandardType::CancelToken,
];
const PARAM_NET_STREAM: &[StandardType] = &[StandardType::NetStream];
const PARAM_NET_STREAM_DEADLINE: &[StandardType] =
    &[StandardType::NetStream, StandardType::Deadline];
const PARAM_NET_STREAM_DEADLINE_CANCEL_TOKEN: &[StandardType] = &[
    StandardType::NetStream,
    StandardType::Deadline,
    StandardType::CancelToken,
];
const PARAM_NET_STREAM_BYTE_CHUNK: &[StandardType] =
    &[StandardType::NetStream, StandardType::ByteChunk];
const PARAM_NET_STREAM_BYTE_CHUNK_DEADLINE: &[StandardType] = &[
    StandardType::NetStream,
    StandardType::ByteChunk,
    StandardType::Deadline,
];
const PARAM_NET_STREAM_BYTE_CHUNK_DEADLINE_CANCEL_TOKEN: &[StandardType] = &[
    StandardType::NetStream,
    StandardType::ByteChunk,
    StandardType::Deadline,
    StandardType::CancelToken,
];
const PARAM_NET_STREAM_BYTE_CHUNKS: &[StandardType] =
    &[StandardType::NetStream, LIST_BYTE_CHUNK_TYPE];
const PARAM_NET_STREAM_BYTE_CHUNKS_DEADLINE: &[StandardType] = &[
    StandardType::NetStream,
    LIST_BYTE_CHUNK_TYPE,
    StandardType::Deadline,
];
const PARAM_NET_STREAM_BYTE_CHUNKS_DEADLINE_CANCEL_TOKEN: &[StandardType] = &[
    StandardType::NetStream,
    LIST_BYTE_CHUNK_TYPE,
    StandardType::Deadline,
    StandardType::CancelToken,
];
const PARAM_INT: &[StandardType] = &[StandardType::Int];
const PARAM_DEADLINE: &[StandardType] = &[StandardType::Deadline];
const PARAM_CANCEL_OWNER: &[StandardType] = &[StandardType::CancelOwner];
const PARAM_CANCEL_TOKEN: &[StandardType] = &[StandardType::CancelToken];
const PARAM_DEADLINE_CANCEL_TOKEN: &[StandardType] =
    &[StandardType::Deadline, StandardType::CancelToken];
#[cfg(test)]
const SOURCE_BACKED_PRIVATE_HELPERS: &[&str] = &[
    "vec_map_step",
    "vec_try_map_step",
    "vec_try_map_with_step",
    "list_reverse_step",
    "list_map_step",
    "list_filter_step",
    "list_try_map_step",
    "dict_map_with_step",
    "dict_filter_with_step",
    "dict_fold_with_step",
    "dict_try_map_with_step",
];

macro_rules! source_prelude_symbol_set {
    ($($name:literal => $source:expr),+ $(,)?) => {
        #[cfg(test)]
        const SOURCE_PRELUDE_NAMES: &[&str] = &[$($name),+];
        const SOURCE_PRELUDE_SYMBOLS: &[StandardSymbolDescriptor] = &[
            $(source_prelude_symbol_descriptor($name, $source)),+
        ];
    };
}

const QUALIFIED_SYMBOLS: &[StandardSymbolDescriptor] = &[
    runtime_symbol("stdio", "print", STDIO_EFFECTS, "runtime.stdio.print"),
    runtime_symbol("stdio", "println", STDIO_EFFECTS, "runtime.stdio.println"),
    runtime_symbol("stdio", "eprint", STDIO_EFFECTS, "runtime.stdio.eprint"),
    runtime_symbol("stdio", "eprintln", STDIO_EFFECTS, "runtime.stdio.eprintln"),
    runtime_symbol(
        "channel",
        "bounded",
        CONCURRENCY_EFFECTS,
        "runtime.channel.bounded",
    ),
    runtime_symbol(
        "channel",
        "clone",
        CONCURRENCY_EFFECTS,
        "runtime.channel.clone",
    ),
    runtime_symbol(
        "channel",
        "send",
        CONCURRENCY_EFFECTS,
        "runtime.channel.send",
    ),
    runtime_symbol(
        "channel",
        "recv",
        CONCURRENCY_EFFECTS,
        "runtime.channel.recv",
    ),
    runtime_symbol(
        "channel",
        "select",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select",
    ),
    runtime_symbol(
        "channel",
        "select_priority",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select_priority",
    ),
    runtime_symbol(
        "channel",
        "select_many_priority",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select_many_priority",
    ),
    runtime_symbol(
        "channel",
        "select_many_timeout",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select_many_timeout",
    ),
    runtime_symbol(
        "channel",
        "select_many_timeout_result",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select_many_timeout_result",
    ),
    runtime_symbol(
        "channel",
        "select_many_timeout_cancellable",
        TIME_CONCURRENCY_EFFECTS,
        "runtime.channel.select_many_timeout_cancellable",
    ),
    runtime_symbol(
        "channel",
        "select_timeout",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select_timeout",
    ),
    runtime_symbol(
        "channel",
        "select_timeout_cancellable",
        TIME_CONCURRENCY_EFFECTS,
        "runtime.channel.select_timeout_cancellable",
    ),
    runtime_symbol(
        "channel",
        "select_result",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select_result",
    ),
    runtime_symbol(
        "channel",
        "select_priority_result",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select_priority_result",
    ),
    runtime_symbol(
        "channel",
        "select_timeout_result",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select_timeout_result",
    ),
    runtime_symbol(
        "channel",
        "close",
        CONCURRENCY_EFFECTS,
        "runtime.channel.close",
    ),
    runtime_symbol("task", "spawn", CONCURRENCY_EFFECTS, "runtime.task.spawn"),
    runtime_symbol(
        "task",
        "spawn_with",
        CONCURRENCY_EFFECTS,
        "runtime.task.spawn_with",
    ),
    runtime_symbol("task", "join", CONCURRENCY_EFFECTS, "runtime.task.join"),
    runtime_symbol("task", "cancel", CONCURRENCY_EFFECTS, "runtime.task.cancel"),
    runtime_symbol_with_signature(
        "fs",
        "read_to_string",
        FS_EFFECTS,
        "runtime.fs.read_to_string",
        StandardSignature {
            params: PARAM_PATH,
            return_type: RESULT_STRING_FS_ERROR_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "fs",
        "write_string",
        FS_EFFECTS,
        "runtime.fs.write_string",
        StandardSignature {
            params: PARAM_PATH_STRING,
            return_type: RESULT_UNIT_FS_ERROR_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "fs",
        "exists",
        FS_EFFECTS,
        "runtime.fs.exists",
        StandardSignature {
            params: PARAM_PATH,
            return_type: RESULT_BOOL_FS_ERROR_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "fs",
        "read_dir",
        FS_EFFECTS,
        "runtime.fs.read_dir",
        StandardSignature {
            params: PARAM_PATH,
            return_type: RESULT_VEC_PATH_FS_ERROR_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "receive_chunk",
        NET_EFFECTS,
        "runtime.net.receive_chunk",
        StandardSignature {
            params: &[],
            return_type: StandardType::ByteChunk,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "send_chunk",
        NET_EFFECTS,
        "runtime.net.send_chunk",
        StandardSignature {
            params: PARAM_BYTE_CHUNK,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "listen",
        NET_EFFECTS,
        "runtime.net.listen",
        StandardSignature {
            params: PARAM_STRING,
            return_type: StandardType::NetListener,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "connect",
        NET_EFFECTS,
        "runtime.net.connect",
        StandardSignature {
            params: PARAM_STRING,
            return_type: StandardType::NetStream,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "accept",
        NET_EFFECTS,
        "runtime.net.accept",
        StandardSignature {
            params: PARAM_NET_LISTENER,
            return_type: StandardType::NetStream,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "accept_or_end",
        NET_EFFECTS,
        "runtime.net.accept_or_end",
        StandardSignature {
            params: PARAM_NET_LISTENER,
            return_type: OPTION_NET_STREAM_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "accept_until",
        NET_TIME_EFFECTS,
        "runtime.net.accept_until",
        StandardSignature {
            params: PARAM_NET_LISTENER_DEADLINE,
            return_type: OPTION_NET_STREAM_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "accept_until_cancellable",
        NET_TIME_EFFECTS,
        "runtime.net.accept_until_cancellable",
        StandardSignature {
            params: PARAM_NET_LISTENER_DEADLINE_CANCEL_TOKEN,
            return_type: ACCEPT_OUTCOME_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "listener_local_addr",
        NET_EFFECTS,
        "runtime.net.listener_local_addr",
        StandardSignature {
            params: PARAM_NET_LISTENER,
            return_type: StandardType::String,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "read_chunk",
        NET_EFFECTS,
        "runtime.net.read_chunk",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: StandardType::ByteChunk,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "stream_local_addr",
        NET_EFFECTS,
        "runtime.net.stream_local_addr",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: StandardType::String,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "stream_peer_addr",
        NET_EFFECTS,
        "runtime.net.stream_peer_addr",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: StandardType::String,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "stream_can_read",
        NET_EFFECTS,
        "runtime.net.stream_can_read",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: StandardType::Bool,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "stream_can_write",
        NET_EFFECTS,
        "runtime.net.stream_can_write",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: StandardType::Bool,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "stream_is_closed",
        NET_EFFECTS,
        "runtime.net.stream_is_closed",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: StandardType::Bool,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "read_chunk_until",
        NET_TIME_EFFECTS,
        "runtime.net.read_chunk_until",
        StandardSignature {
            params: PARAM_NET_STREAM_DEADLINE,
            return_type: OPTION_BYTE_CHUNK_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "read_chunk_until_cancellable",
        NET_TIME_EFFECTS,
        "runtime.net.read_chunk_until_cancellable",
        StandardSignature {
            params: PARAM_NET_STREAM_DEADLINE_CANCEL_TOKEN,
            return_type: STREAM_READ_OUTCOME_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "read_chunk_or_end",
        NET_EFFECTS,
        "runtime.net.read_chunk_or_end",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: OPTION_BYTE_CHUNK_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "write_chunk",
        NET_EFFECTS,
        "runtime.net.write_chunk",
        StandardSignature {
            params: PARAM_NET_STREAM_BYTE_CHUNK,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "write_chunk_until",
        NET_TIME_EFFECTS,
        "runtime.net.write_chunk_until",
        StandardSignature {
            params: PARAM_NET_STREAM_BYTE_CHUNK_DEADLINE,
            return_type: STREAM_WRITE_OUTCOME_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "write_chunk_until_cancellable",
        NET_TIME_EFFECTS,
        "runtime.net.write_chunk_until_cancellable",
        StandardSignature {
            params: PARAM_NET_STREAM_BYTE_CHUNK_DEADLINE_CANCEL_TOKEN,
            return_type: STREAM_WRITE_OUTCOME_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "write_chunks",
        NET_EFFECTS,
        "runtime.net.write_chunks",
        StandardSignature {
            params: PARAM_NET_STREAM_BYTE_CHUNKS,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "write_chunks_until",
        NET_TIME_EFFECTS,
        "runtime.net.write_chunks_until",
        StandardSignature {
            params: PARAM_NET_STREAM_BYTE_CHUNKS_DEADLINE,
            return_type: STREAM_WRITE_OUTCOME_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "write_chunks_until_cancellable",
        NET_TIME_EFFECTS,
        "runtime.net.write_chunks_until_cancellable",
        StandardSignature {
            params: PARAM_NET_STREAM_BYTE_CHUNKS_DEADLINE_CANCEL_TOKEN,
            return_type: STREAM_WRITE_OUTCOME_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "shutdown_write",
        NET_EFFECTS,
        "runtime.net.shutdown_write",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "shutdown_read",
        NET_EFFECTS,
        "runtime.net.shutdown_read",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "close_stream",
        NET_EFFECTS,
        "runtime.net.close_stream",
        StandardSignature {
            params: PARAM_NET_STREAM,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "close_listener",
        NET_EFFECTS,
        "runtime.net.close_listener",
        StandardSignature {
            params: PARAM_NET_LISTENER,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "process",
        "args",
        PROCESS_EFFECTS,
        "runtime.process.args",
        StandardSignature {
            params: &[],
            return_type: VEC_STRING_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "process",
        "env",
        PROCESS_EFFECTS,
        "runtime.process.env",
        StandardSignature {
            params: PARAM_STRING,
            return_type: OPTION_STRING_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "process",
        "cwd",
        PROCESS_EFFECTS,
        "runtime.process.cwd",
        StandardSignature {
            params: &[],
            return_type: RESULT_PATH_PROCESS_ERROR_TYPE,
        },
    ),
    runtime_symbol_with_signature(
        "process",
        "exit",
        PROCESS_EFFECTS,
        "runtime.process.exit",
        StandardSignature {
            params: PARAM_INT,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "monotonic_ms",
        TIME_EFFECTS,
        "runtime.time.monotonic_ms",
        StandardSignature {
            params: &[],
            return_type: StandardType::Int,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "timeout_ms",
        TIME_EFFECTS,
        "runtime.time.timeout_ms",
        StandardSignature {
            params: PARAM_INT,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "deadline_after_ms",
        TIME_EFFECTS,
        "runtime.time.deadline_after_ms",
        StandardSignature {
            params: PARAM_INT,
            return_type: StandardType::Deadline,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "deadline_at_ms",
        TIME_EFFECTS,
        "runtime.time.deadline_at_ms",
        StandardSignature {
            params: PARAM_INT,
            return_type: StandardType::Deadline,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "wait_until",
        TIME_EFFECTS,
        "runtime.time.wait_until",
        StandardSignature {
            params: PARAM_DEADLINE,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "cancel_token",
        TIME_EFFECTS,
        "runtime.time.cancel_token",
        StandardSignature {
            params: &[],
            return_type: StandardType::CancelToken,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "cancel_owner",
        TIME_EFFECTS,
        "runtime.time.cancel_owner",
        StandardSignature {
            params: &[],
            return_type: StandardType::CancelOwner,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "cancel_token_from",
        TIME_EFFECTS,
        "runtime.time.cancel_token_from",
        StandardSignature {
            params: PARAM_CANCEL_OWNER,
            return_type: StandardType::CancelToken,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "cancel_owned",
        TIME_EFFECTS,
        "runtime.time.cancel_owned",
        StandardSignature {
            params: PARAM_CANCEL_OWNER,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "cancel",
        TIME_EFFECTS,
        "runtime.time.cancel",
        StandardSignature {
            params: PARAM_CANCEL_TOKEN,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "is_cancelled",
        TIME_EFFECTS,
        "runtime.time.is_cancelled",
        StandardSignature {
            params: PARAM_CANCEL_TOKEN,
            return_type: StandardType::Bool,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "is_cancelled_owner",
        TIME_EFFECTS,
        "runtime.time.is_cancelled_owner",
        StandardSignature {
            params: PARAM_CANCEL_OWNER,
            return_type: StandardType::Bool,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "wait_until_cancellable",
        TIME_EFFECTS,
        "runtime.time.wait_until_cancellable",
        StandardSignature {
            params: PARAM_DEADLINE_CANCEL_TOKEN,
            return_type: StandardType::Unit,
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "wait_until_cancellable_outcome",
        TIME_EFFECTS,
        "runtime.time.wait_until_cancellable_outcome",
        StandardSignature {
            params: PARAM_DEADLINE_CANCEL_TOKEN,
            return_type: StandardType::CancellableWaitOutcome,
        },
    ),
];

const FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS: &[StandardSymbolDescriptor] = &[
    prelude_symbol_descriptor("float_negate"),
    prelude_symbol_descriptor("float_add"),
    prelude_symbol_descriptor("float_subtract"),
    prelude_symbol_descriptor("float_multiply"),
    prelude_symbol_descriptor("float_divide"),
    prelude_symbol_descriptor("float_less"),
    prelude_symbol_descriptor("float_less_equal"),
    prelude_symbol_descriptor("float_greater"),
    prelude_symbol_descriptor("float_greater_equal"),
];

const SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS: &[StandardSymbolDescriptor] = &[];

source_prelude_symbol_set! {
    "byte" => veln_stdlib::prelude_source("byte"),
    "byte_to_int" => veln_stdlib::prelude_source("byte_to_int"),
    "byte_chunk" => veln_stdlib::prelude_source("byte_chunk"),
    "byte_chunk_count" => veln_stdlib::prelude_source("byte_chunk_count"),
    "byte_append" => veln_stdlib::prelude_source("byte_append"),
    "byte_chunk_from_hex" => veln_stdlib::prelude_source("byte_chunk_from_hex"),
    "byte_chunk_to_visible_ascii_string" => veln_stdlib::prelude_source("byte_chunk_to_visible_ascii_string"),
    "hpack_fixture_huffman_bytes_label" => veln_stdlib::prelude_source("hpack_fixture_huffman_bytes_label"),
    "hpack_fixture_huffman_label_bytes" => veln_stdlib::prelude_source("hpack_fixture_huffman_label_bytes"),
    "byte_chunk_from_visible_ascii_string" => veln_stdlib::prelude_source("byte_chunk_from_visible_ascii_string"),
    "byte_take" => veln_stdlib::prelude_source("byte_take"),
    "byte_drop" => veln_stdlib::prelude_source("byte_drop"),
    "byte_view" => veln_stdlib::prelude_source("byte_view"),
    "byte_view_to_chunk" => veln_stdlib::prelude_source("byte_view_to_chunk"),
    "byte_view_count" => veln_stdlib::prelude_source("byte_view_count"),
    "byte_view_take" => veln_stdlib::prelude_source("byte_view_take"),
    "byte_view_drop" => veln_stdlib::prelude_source("byte_view_drop"),
    "byte_view_slice" => veln_stdlib::prelude_source("byte_view_slice"),
    "byte_chunks_empty" => veln_stdlib::prelude_source("byte_chunks_empty"),
    "byte_chunks_one" => veln_stdlib::prelude_source("byte_chunks_one"),
    "byte_chunks_append" => veln_stdlib::prelude_source("byte_chunks_append"),
    "byte_chunks_produce" => veln_stdlib::prelude_source("byte_chunks_produce"),
    "byte_read_u8_be" => veln_stdlib::prelude_source("byte_read_u8_be"),
    "byte_expect_fixed_u8_be" => veln_stdlib::prelude_source("byte_expect_fixed_u8_be"),
    "byte_decode_http2_frame" => veln_stdlib::prelude_source("byte_decode_http2_frame"),
    "byte_decode_schema_width_sample" => veln_stdlib::prelude_source("byte_decode_schema_width_sample"),
    "byte_decode_schema_validation_sample" => veln_stdlib::prelude_source("byte_decode_schema_validation_sample"),
    "http2_protocol_closed_with_pending" => veln_stdlib::prelude_source("http2_protocol_closed_with_pending"),
    "http2_protocol_partial_preface" => veln_stdlib::prelude_source("http2_protocol_partial_preface"),
    "http2_protocol_invalid_preface" => veln_stdlib::prelude_source("http2_protocol_invalid_preface"),
    "http2_protocol_initial_peer_settings_required" => veln_stdlib::prelude_source("http2_protocol_initial_peer_settings_required"),
    "http2_protocol_continuation_expected" => veln_stdlib::prelude_source("http2_protocol_continuation_expected"),
    "http2_protocol_invalid_frame_kind" => veln_stdlib::prelude_source("http2_protocol_invalid_frame_kind"),
    "http2_protocol_invalid_stream_id" => veln_stdlib::prelude_source("http2_protocol_invalid_stream_id"),
    "http2_protocol_invalid_payload_length" => veln_stdlib::prelude_source("http2_protocol_invalid_payload_length"),
    "http2_protocol_invalid_window_update_increment" => veln_stdlib::prelude_source("http2_protocol_invalid_window_update_increment"),
    "http2_protocol_invalid_data_padding" => veln_stdlib::prelude_source("http2_protocol_invalid_data_padding"),
    "http2_protocol_content_length_mismatch" => veln_stdlib::prelude_source("http2_protocol_content_length_mismatch"),
    "http2_protocol_invalid_request_header_list" => veln_stdlib::prelude_source("http2_protocol_invalid_request_header_list"),
    "http2_protocol_invalid_response_header_list" => veln_stdlib::prelude_source("http2_protocol_invalid_response_header_list"),
    "http2_protocol_unexpected_settings_ack" => veln_stdlib::prelude_source("http2_protocol_unexpected_settings_ack"),
    "http2_protocol_settings_not_allowed_for_endpoint" => veln_stdlib::prelude_source("http2_protocol_settings_not_allowed_for_endpoint"),
    "http2_protocol_invalid_priority_dependency" => veln_stdlib::prelude_source("http2_protocol_invalid_priority_dependency"),
    "http2_protocol_stream_after_goaway" => veln_stdlib::prelude_source("http2_protocol_stream_after_goaway"),
    "http2_peer_limit_frame_size_exceeded" => veln_stdlib::prelude_source("http2_peer_limit_frame_size_exceeded"),
    "http2_peer_limit_header_list_size_exceeded" => veln_stdlib::prelude_source("http2_peer_limit_header_list_size_exceeded"),
    "http2_peer_limit_header_table_size_exceeded" => veln_stdlib::prelude_source("http2_peer_limit_header_table_size_exceeded"),
    "http2_peer_limit_flow_control_window_exceeded" => veln_stdlib::prelude_source("http2_peer_limit_flow_control_window_exceeded"),
    "http2_peer_limit_concurrent_streams_exceeded" => veln_stdlib::prelude_source("http2_peer_limit_concurrent_streams_exceeded"),
    "http2_peer_limit_settings_value_out_of_range" => veln_stdlib::prelude_source("http2_peer_limit_settings_value_out_of_range"),
    "hpack_fixture_unsupported_header_block" => veln_stdlib::prelude_source("hpack_fixture_unsupported_header_block"),
    "hpack_fixture_unsupported_static_index" => veln_stdlib::prelude_source("hpack_fixture_unsupported_static_index"),
    "hpack_fixture_malformed_string_length" => veln_stdlib::prelude_source("hpack_fixture_malformed_string_length"),
    "hpack_fixture_malformed_raw_string_value" => veln_stdlib::prelude_source("hpack_fixture_malformed_raw_string_value"),
    "hpack_fixture_malformed_huffman_padding" => veln_stdlib::prelude_source("hpack_fixture_malformed_huffman_padding"),
    "hpack_fixture_huffman_eos_symbol" => veln_stdlib::prelude_source("hpack_fixture_huffman_eos_symbol"),
    "hpack_fixture_huffman_non_visible_value" => veln_stdlib::prelude_source("hpack_fixture_huffman_non_visible_value"),
    "hpack_fixture_table_size_update_malformed" => veln_stdlib::prelude_source("hpack_fixture_table_size_update_malformed"),
    "hpack_fixture_dynamic_index_out_of_range" => veln_stdlib::prelude_source("hpack_fixture_dynamic_index_out_of_range"),
    "hpack_fixture_dynamic_name_continuation_missing" => veln_stdlib::prelude_source("hpack_fixture_dynamic_name_continuation_missing"),
    "hpack_fixture_dynamic_name_continuation_malformed" => veln_stdlib::prelude_source("hpack_fixture_dynamic_name_continuation_malformed"),
    "hpack_fixture_dynamic_name_continuation_out_of_range" => veln_stdlib::prelude_source("hpack_fixture_dynamic_name_continuation_out_of_range"),
    "hpack_fixture_table_size_update_not_at_start" => veln_stdlib::prelude_source("hpack_fixture_table_size_update_not_at_start"),
    "hpack_fixture_table_size_update_trailing_bytes" => veln_stdlib::prelude_source("hpack_fixture_table_size_update_trailing_bytes"),
    "byte_read_u16_be" => veln_stdlib::prelude_source("byte_read_u16_be"),
    "byte_read_u24_be" => veln_stdlib::prelude_source("byte_read_u24_be"),
    "byte_read_u31_be" => veln_stdlib::prelude_source("byte_read_u31_be"),
    "byte_read_u32_be" => veln_stdlib::prelude_source("byte_read_u32_be"),
    "byte_read_u40_be" => veln_stdlib::prelude_source("byte_read_u40_be"),
    "byte_read_u48_be" => veln_stdlib::prelude_source("byte_read_u48_be"),
    "byte_read_u56_be" => veln_stdlib::prelude_source("byte_read_u56_be"),
    "byte_read_u64_be" => veln_stdlib::prelude_source("byte_read_u64_be"),
    "byte_read_u16_le" => veln_stdlib::prelude_source("byte_read_u16_le"),
    "byte_read_u24_le" => veln_stdlib::prelude_source("byte_read_u24_le"),
    "byte_read_u31_le" => veln_stdlib::prelude_source("byte_read_u31_le"),
    "byte_read_u32_le" => veln_stdlib::prelude_source("byte_read_u32_le"),
    "byte_read_u40_le" => veln_stdlib::prelude_source("byte_read_u40_le"),
    "byte_read_u48_le" => veln_stdlib::prelude_source("byte_read_u48_le"),
    "byte_read_u56_le" => veln_stdlib::prelude_source("byte_read_u56_le"),
    "byte_read_u64_le" => veln_stdlib::prelude_source("byte_read_u64_le"),
    "byte_write_u8_be" => veln_stdlib::prelude_source("byte_write_u8_be"),
    "byte_write_u16_be" => veln_stdlib::prelude_source("byte_write_u16_be"),
    "byte_write_u24_be" => veln_stdlib::prelude_source("byte_write_u24_be"),
    "byte_write_u31_be" => veln_stdlib::prelude_source("byte_write_u31_be"),
    "byte_write_u32_be" => veln_stdlib::prelude_source("byte_write_u32_be"),
    "byte_write_u40_be" => veln_stdlib::prelude_source("byte_write_u40_be"),
    "byte_write_u48_be" => veln_stdlib::prelude_source("byte_write_u48_be"),
    "byte_write_u56_be" => veln_stdlib::prelude_source("byte_write_u56_be"),
    "byte_write_u64_be" => veln_stdlib::prelude_source("byte_write_u64_be"),
    "byte_write_u16_le" => veln_stdlib::prelude_source("byte_write_u16_le"),
    "byte_write_u24_le" => veln_stdlib::prelude_source("byte_write_u24_le"),
    "byte_write_u31_le" => veln_stdlib::prelude_source("byte_write_u31_le"),
    "byte_write_u32_le" => veln_stdlib::prelude_source("byte_write_u32_le"),
    "byte_write_u40_le" => veln_stdlib::prelude_source("byte_write_u40_le"),
    "byte_write_u48_le" => veln_stdlib::prelude_source("byte_write_u48_le"),
    "byte_write_u56_le" => veln_stdlib::prelude_source("byte_write_u56_le"),
    "byte_write_u64_le" => veln_stdlib::prelude_source("byte_write_u64_le"),
    "byte_count" => veln_stdlib::prelude_source("byte_count"),
    "byte_count_to_int" => veln_stdlib::prelude_source("byte_count_to_int"),
    "byte_offset" => veln_stdlib::prelude_source("byte_offset"),
    "byte_offset_to_int" => veln_stdlib::prelude_source("byte_offset_to_int"),
    "stream_adapter_drain_actions" => veln_stdlib::prelude_source("stream_adapter_drain_actions"),
    "stream_adapter_accept_loop" => veln_stdlib::prelude_source("stream_adapter_accept_loop"),
    "stream_adapter_drain_actions_until_cancellable" => veln_stdlib::prelude_source("stream_adapter_drain_actions_until_cancellable"),
    "vec_fold" => veln_stdlib::prelude_source("vec_fold"),
    "vec_len" => veln_stdlib::prelude_source("vec_len"),
    "vec_is_empty" => veln_stdlib::prelude_source("vec_is_empty"),
    "vec_push" => veln_stdlib::prelude_source("vec_push"),
    "vec_concat" => veln_stdlib::prelude_source("vec_concat"),
    "vec_map" => veln_stdlib::prelude_source("vec_map"),
    "vec_filter" => veln_stdlib::prelude_source("vec_filter"),
    "vec_try_map" => veln_stdlib::prelude_source("vec_try_map"),
    "vec_try_map_with" => veln_stdlib::prelude_source("vec_try_map_with"),
    "list_nil" => veln_stdlib::prelude_source("list_nil"),
    "list_cons" => veln_stdlib::prelude_source("list_cons"),
    "list_is_empty" => veln_stdlib::prelude_source("list_is_empty"),
    "list_fold" => veln_stdlib::prelude_source("list_fold"),
    "list_reverse" => veln_stdlib::prelude_source("list_reverse"),
    "list_map" => veln_stdlib::prelude_source("list_map"),
    "list_filter" => veln_stdlib::prelude_source("list_filter"),
    "list_try_map" => veln_stdlib::prelude_source("list_try_map"),
    "dict_get" => veln_stdlib::prelude_source("dict_get"),
    "dict_contains" => veln_stdlib::prelude_source("dict_contains"),
    "dict_insert" => veln_stdlib::prelude_source("dict_insert"),
    "dict_remove" => veln_stdlib::prelude_source("dict_remove"),
    "dict_map" => veln_stdlib::prelude_source("dict_map"),
    "dict_map_with" => veln_stdlib::prelude_source("dict_map_with"),
    "dict_filter" => veln_stdlib::prelude_source("dict_filter"),
    "dict_filter_with" => veln_stdlib::prelude_source("dict_filter_with"),
    "dict_fold" => veln_stdlib::prelude_source("dict_fold"),
    "dict_fold_with" => veln_stdlib::prelude_source("dict_fold_with"),
    "dict_try_map" => veln_stdlib::prelude_source("dict_try_map"),
    "dict_try_map_with" => veln_stdlib::prelude_source("dict_try_map_with"),
    "option_map" => veln_stdlib::prelude_source("option_map"),
    "option_and_then" => veln_stdlib::prelude_source("option_and_then"),
    "option_unwrap_or" => veln_stdlib::prelude_source("option_unwrap_or"),
    "result_map" => veln_stdlib::prelude_source("result_map"),
    "result_map_err" => veln_stdlib::prelude_source("result_map_err"),
    "result_and_then" => veln_stdlib::prelude_source("result_and_then"),
    "string_split_once" => veln_stdlib::prelude_source("string_split_once"),
    "string_parse_int" => veln_stdlib::prelude_source("string_parse_int"),
    "int_to_string" => veln_stdlib::prelude_source("int_to_string"),
}

const fn runtime_symbol(
    module: &'static str,
    name: &'static str,
    effects: &'static [&'static str],
    lowering: &'static str,
) -> StandardSymbolDescriptor {
    StandardSymbolDescriptor {
        module: Some(module),
        name,
        kind: StandardSymbolKind::Runtime,
        effects,
        lowering: Some(lowering),
        signature: None,
        source: None,
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }
}

const fn runtime_symbol_with_signature(
    module: &'static str,
    name: &'static str,
    effects: &'static [&'static str],
    lowering: &'static str,
    signature: StandardSignature,
) -> StandardSymbolDescriptor {
    StandardSymbolDescriptor {
        module: Some(module),
        name,
        kind: StandardSymbolKind::Runtime,
        effects,
        lowering: Some(lowering),
        signature: Some(signature),
        source: None,
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }
}

const fn prelude_symbol_descriptor(name: &'static str) -> StandardSymbolDescriptor {
    StandardSymbolDescriptor {
        module: None,
        name,
        kind: StandardSymbolKind::Prelude,
        effects: PURE_EFFECTS,
        lowering: None,
        signature: None,
        source: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }
}

const fn source_prelude_symbol_descriptor(
    name: &'static str,
    source: veln_stdlib::StdlibSource,
) -> StandardSymbolDescriptor {
    StandardSymbolDescriptor {
        module: None,
        name,
        kind: StandardSymbolKind::Veln,
        effects: PURE_EFFECTS,
        lowering: None,
        signature: None,
        source: Some(source),
        stability: StandardSymbolStability::CompatibilityOnly,
    }
}

pub(crate) fn qualified_symbol(segments: &[String]) -> Option<&'static StandardSymbolDescriptor> {
    let [module, name] = segments else {
        return None;
    };
    QUALIFIED_SYMBOLS
        .iter()
        .find(|symbol| symbol.module == Some(module.as_str()) && symbol.name == name)
}

pub(crate) fn prelude_symbol(name: &str) -> Option<&'static StandardSymbolDescriptor> {
    prelude_symbols().find(|symbol| symbol.name == name)
}

fn prelude_symbols() -> impl Iterator<Item = &'static StandardSymbolDescriptor> {
    descriptor_only_prelude_symbols().chain(SOURCE_PRELUDE_SYMBOLS.iter())
}

fn descriptor_only_prelude_symbols() -> impl Iterator<Item = &'static StandardSymbolDescriptor> {
    FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS
        .iter()
        .chain(SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS.iter())
}

#[cfg(test)]
pub(crate) fn source_backed_prelude_symbols() -> &'static [StandardSymbolDescriptor] {
    SOURCE_PRELUDE_SYMBOLS
}

#[cfg(test)]
pub(crate) fn source_backed_prelude_names() -> impl Iterator<Item = &'static str> {
    source_backed_prelude_symbols()
        .iter()
        .map(|symbol| symbol.name)
}

#[cfg(test)]
pub(crate) fn source_backed_symbols() -> impl Iterator<Item = &'static StandardSymbolDescriptor> {
    source_backed_prelude_symbols()
        .iter()
        .chain(QUALIFIED_SYMBOLS)
        .filter(|symbol| symbol.source.is_some())
}

#[allow(dead_code)]
pub(crate) fn compiler_support_sources() -> impl Iterator<Item = veln_stdlib::StdlibSource> {
    [veln_stdlib::COMPILER_SUPPORT].into_iter()
}

pub(crate) fn effect_strings(symbol: &StandardSymbolDescriptor) -> Vec<String> {
    symbol
        .effects
        .iter()
        .map(|effect| (*effect).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn path(module: &str, name: &str) -> Vec<String> {
        vec![module.to_string(), name.to_string()]
    }

    #[test]
    fn descriptor_table_carries_runtime_effect_metadata() {
        let symbol = qualified_symbol(&path("stdio", "println")).expect("stdio descriptor");

        assert_eq!(symbol.kind, StandardSymbolKind::Runtime);
        assert_eq!(symbol.effects, ["stdio"]);
        assert_eq!(symbol.lowering, Some("runtime.stdio.println"));
        assert_eq!(symbol.source, None);
        assert_eq!(
            symbol.stability,
            StandardSymbolStability::RequiredForSelfHosting
        );
        assert_eq!(effect_strings(symbol), vec!["stdio"]);
    }

    #[test]
    fn descriptor_table_carries_prelude_purity_metadata() {
        let symbol = prelude_symbol("float_add").expect("prelude descriptor");

        assert_eq!(symbol.kind, StandardSymbolKind::Prelude);
        assert!(symbol.effects.is_empty());
        assert_eq!(symbol.lowering, None);
        assert_eq!(symbol.stability, StandardSymbolStability::CompatibilityOnly);
    }

    #[test]
    fn source_backed_prelude_descriptors_carry_metadata() {
        let mut entries = Vec::new();

        for name in SOURCE_PRELUDE_NAMES.iter().copied() {
            let symbol = prelude_symbol(name).expect("source-backed helper descriptor");
            let source = symbol.source.expect("source metadata");

            assert_eq!(symbol.kind, StandardSymbolKind::Veln);
            assert!(symbol.effects.is_empty());
            assert_eq!(symbol.lowering, None);
            assert_eq!(source.entry, symbol.name);
            assert!(
                !source.path.starts_with('/'),
                "source path should be repository relative"
            );
            assert!(source.text.contains(&format!("fn {name}")));
            entries.push(source.entry);
        }

        assert_eq!(entries, SOURCE_PRELUDE_NAMES);
    }

    #[test]
    fn descriptor_only_prelude_helpers_do_not_carry_source_metadata() {
        for symbol in descriptor_only_prelude_symbols() {
            assert_eq!(symbol.kind, StandardSymbolKind::Prelude);
            assert_eq!(symbol.lowering, None);
            assert!(symbol.effects.is_empty());
            assert_eq!(symbol.source, None);
        }
    }

    #[test]
    fn vec_fold_source_metadata_uses_prelude_source() {
        let symbol = prelude_symbol("vec_fold").expect("vec_fold descriptor");
        let source = symbol.source.expect("vec_fold source metadata");

        assert_eq!(symbol.kind, StandardSymbolKind::Veln);
        assert_eq!(source.path, "prelude.veln");
        assert!(source.text.contains("fn vec_fold("));
    }

    #[test]
    fn source_backed_step_helpers_are_not_prelude_descriptors() {
        for name in SOURCE_BACKED_PRIVATE_HELPERS {
            assert_eq!(prelude_symbol(name), None);
        }
    }

    #[test]
    fn deferred_dictionary_traversal_helpers_are_not_prelude_descriptors() {
        for name in ["dict_keys", "dict_values"] {
            assert_eq!(prelude_symbol(name), None, "{name}");
        }
    }

    #[test]
    fn no_descriptor_only_pure_helpers_remain_after_source_backed_migration() {
        assert_eq!(SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS.iter().next(), None);
    }

    #[test]
    fn source_backed_boundary_matches_current_prelude_split() {
        let source_backed = SOURCE_PRELUDE_SYMBOLS
            .iter()
            .map(|symbol| symbol.name)
            .collect::<Vec<_>>();
        let descriptor_only = descriptor_only_prelude_symbols()
            .map(|symbol| symbol.name)
            .collect::<Vec<_>>();

        assert_eq!(source_backed, SOURCE_PRELUDE_NAMES);
        assert_eq!(
            descriptor_only,
            [
                "float_negate",
                "float_add",
                "float_subtract",
                "float_multiply",
                "float_divide",
                "float_less",
                "float_less_equal",
                "float_greater",
                "float_greater_equal",
            ]
        );
    }

    #[test]
    fn source_backed_descriptors_have_valid_metadata() {
        let mut sources = BTreeSet::new();
        let mut count = 0;

        for symbol in prelude_symbols().chain(QUALIFIED_SYMBOLS.iter()) {
            if let Some(source) = symbol.source {
                count += 1;
                assert_eq!(symbol.kind, StandardSymbolKind::Veln);
                assert_eq!(symbol.effects, PURE_EFFECTS);
                assert_eq!(symbol.lowering, None);
                assert_eq!(source.entry, symbol.name);
                assert!(
                    !source.path.starts_with('/'),
                    "source path should be repository relative"
                );
                assert!(
                    source.text.contains(&format!("fn {}", source.entry)),
                    "embedded source should define {}",
                    source.entry
                );
                assert!(
                    sources.insert((source.path, source.entry)),
                    "duplicate source-backed entry {} in {}",
                    source.entry,
                    source.path
                );
            }
        }

        assert_eq!(
            count,
            SOURCE_PRELUDE_SYMBOLS.len(),
            "expected one source descriptor per source-backed prelude symbol"
        );
    }

    #[test]
    fn qualified_descriptors_have_unique_source_names() {
        let mut names = BTreeSet::new();

        for symbol in QUALIFIED_SYMBOLS {
            let module = symbol.module.expect("qualified symbol has a module");
            assert!(
                names.insert((module, symbol.name)),
                "duplicate qualified symbol {module}::{}",
                symbol.name
            );
        }
    }

    #[test]
    fn prelude_descriptors_have_unique_source_names() {
        let mut names = BTreeSet::new();

        for symbol in prelude_symbols() {
            assert_eq!(symbol.module, None);
            assert!(
                names.insert(symbol.name),
                "duplicate prelude symbol {}",
                symbol.name
            );
        }
    }
}
