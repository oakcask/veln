use std::collections::BTreeSet;

use crate::source_less_names::{
    InvalidStandardSymbolCase, InvalidStandardSymbolReason, SourceLessNameClass,
    validate_source_less_name,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StandardSymbolKind {
    Runtime,
    Prelude,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StandardSymbolDescriptor {
    pub(crate) module: Option<&'static str>,
    pub(crate) name: &'static str,
    pub(crate) name_class: SourceLessNameClass,
    pub(crate) kind: StandardSymbolKind,
    pub(crate) effects: &'static [&'static str],
    pub(crate) lowering: Option<&'static str>,
    pub(crate) signature: Option<StandardSignature>,
    pub(crate) stability: StandardSymbolStability,
}

#[derive(Debug)]
pub(crate) struct StandardSymbolRegistry {
    qualified: Vec<&'static StandardSymbolDescriptor>,
    prelude: Vec<&'static StandardSymbolDescriptor>,
    compiler_adapters: Vec<&'static StandardSymbolDescriptor>,
}

impl StandardSymbolRegistry {
    #[cfg(test)]
    pub(crate) fn qualified_symbols(&self) -> &[&'static StandardSymbolDescriptor] {
        &self.qualified
    }

    #[cfg(test)]
    pub(crate) fn prelude_symbols(&self) -> &[&'static StandardSymbolDescriptor] {
        &self.prelude
    }

    #[cfg(test)]
    pub(crate) fn compiler_adapter_symbols(&self) -> &[&'static StandardSymbolDescriptor] {
        &self.compiler_adapters
    }

    pub(crate) fn qualified_symbol(
        &self,
        segments: &[String],
    ) -> Option<&'static StandardSymbolDescriptor> {
        let [module, name] = segments else {
            return None;
        };
        self.qualified
            .iter()
            .copied()
            .find(|symbol| symbol.module == Some(module.as_str()) && symbol.name == name)
    }

    pub(crate) fn prelude_symbol(&self, name: &str) -> Option<&'static StandardSymbolDescriptor> {
        self.prelude
            .iter()
            .copied()
            .find(|symbol| symbol.name == name)
    }

    pub(crate) fn compiler_adapter_symbol(
        &self,
        name: &str,
    ) -> Option<&'static StandardSymbolDescriptor> {
        self.compiler_adapters
            .iter()
            .copied()
            .find(|symbol| symbol.name == name)
    }
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
    Named(&'static str),
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
const PATH_TYPE: StandardType = StandardType::Named("Path");
const FS_ERROR_TYPE: StandardType = StandardType::Named("FsError");
const PROCESS_ERROR_TYPE: StandardType = StandardType::Named("ProcessError");
const BYTE_CHUNK_TYPE: StandardType = StandardType::Named("ByteChunk");
const NET_STREAM_TYPE: StandardType = StandardType::Named("NetStream");
const ACCEPT_OUTCOME_TYPE: StandardType = StandardType::Named("AcceptOutcome");
const STREAM_READ_OUTCOME_TYPE: StandardType = StandardType::Named("StreamReadOutcome");
const STREAM_WRITE_OUTCOME_TYPE: StandardType = StandardType::Named("StreamWriteOutcome");
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
const PARAM_PATH: &[StandardType] = &[StandardType::Named("Path")];
const PARAM_PATH_STRING: &[StandardType] = &[StandardType::Named("Path"), StandardType::String];
const PARAM_BYTE_CHUNK: &[StandardType] = &[StandardType::Named("ByteChunk")];
const PARAM_STRING: &[StandardType] = &[StandardType::String];
const PARAM_NET_LISTENER: &[StandardType] = &[StandardType::Named("NetListener")];
const PARAM_NET_LISTENER_DEADLINE: &[StandardType] = &[
    StandardType::Named("NetListener"),
    StandardType::Named("Deadline"),
];
const PARAM_NET_LISTENER_DEADLINE_CANCEL_TOKEN: &[StandardType] = &[
    StandardType::Named("NetListener"),
    StandardType::Named("Deadline"),
    StandardType::Named("CancelToken"),
];
const PARAM_NET_STREAM: &[StandardType] = &[StandardType::Named("NetStream")];
const PARAM_NET_STREAM_DEADLINE: &[StandardType] = &[
    StandardType::Named("NetStream"),
    StandardType::Named("Deadline"),
];
const PARAM_NET_STREAM_DEADLINE_CANCEL_TOKEN: &[StandardType] = &[
    StandardType::Named("NetStream"),
    StandardType::Named("Deadline"),
    StandardType::Named("CancelToken"),
];
const PARAM_NET_STREAM_BYTE_CHUNK: &[StandardType] = &[
    StandardType::Named("NetStream"),
    StandardType::Named("ByteChunk"),
];
const PARAM_NET_STREAM_BYTE_CHUNK_DEADLINE: &[StandardType] = &[
    StandardType::Named("NetStream"),
    StandardType::Named("ByteChunk"),
    StandardType::Named("Deadline"),
];
const PARAM_NET_STREAM_BYTE_CHUNK_DEADLINE_CANCEL_TOKEN: &[StandardType] = &[
    StandardType::Named("NetStream"),
    StandardType::Named("ByteChunk"),
    StandardType::Named("Deadline"),
    StandardType::Named("CancelToken"),
];
const PARAM_NET_STREAM_BYTE_CHUNKS: &[StandardType] =
    &[StandardType::Named("NetStream"), LIST_BYTE_CHUNK_TYPE];
const PARAM_NET_STREAM_BYTE_CHUNKS_DEADLINE: &[StandardType] = &[
    StandardType::Named("NetStream"),
    LIST_BYTE_CHUNK_TYPE,
    StandardType::Named("Deadline"),
];
const PARAM_NET_STREAM_BYTE_CHUNKS_DEADLINE_CANCEL_TOKEN: &[StandardType] = &[
    StandardType::Named("NetStream"),
    LIST_BYTE_CHUNK_TYPE,
    StandardType::Named("Deadline"),
    StandardType::Named("CancelToken"),
];
const PARAM_INT: &[StandardType] = &[StandardType::Int];
const PARAM_DEADLINE: &[StandardType] = &[StandardType::Named("Deadline")];
const PARAM_CANCEL_OWNER: &[StandardType] = &[StandardType::Named("CancelOwner")];
const PARAM_CANCEL_TOKEN: &[StandardType] = &[StandardType::Named("CancelToken")];
const PARAM_DEADLINE_CANCEL_TOKEN: &[StandardType] = &[
    StandardType::Named("Deadline"),
    StandardType::Named("CancelToken"),
];
#[cfg(test)]
pub(crate) const DEFAULT_PRELUDE_BUILTIN_MODULE: &str = "prelude_builtin";
#[cfg(test)]
const STANDARD_PACKAGE_PRIVATE_HELPERS: &[&str] = &[
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

macro_rules! compiler_adapter_symbol_set {
    ($($name:literal),+ $(,)?) => {
        #[cfg(test)]
        const COMPILER_ADAPTER_NAMES: &[&str] = &[$($name),+];
        pub(crate) const COMPILER_ADAPTER_SYMBOLS: &[StandardSymbolDescriptor] = &[
            $(source_prelude_symbol_descriptor($name)),+
        ];
    };
}

pub(crate) const QUALIFIED_SYMBOLS: &[StandardSymbolDescriptor] = &[
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
            return_type: StandardType::Named("ByteChunk"),
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
            return_type: StandardType::Named("NetListener"),
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "connect",
        NET_EFFECTS,
        "runtime.net.connect",
        StandardSignature {
            params: PARAM_STRING,
            return_type: StandardType::Named("NetStream"),
        },
    ),
    runtime_symbol_with_signature(
        "net",
        "accept",
        NET_EFFECTS,
        "runtime.net.accept",
        StandardSignature {
            params: PARAM_NET_LISTENER,
            return_type: StandardType::Named("NetStream"),
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
            return_type: StandardType::Named("ByteChunk"),
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
            return_type: StandardType::Named("Deadline"),
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "deadline_at_ms",
        TIME_EFFECTS,
        "runtime.time.deadline_at_ms",
        StandardSignature {
            params: PARAM_INT,
            return_type: StandardType::Named("Deadline"),
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
            return_type: StandardType::Named("CancelToken"),
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "cancel_owner",
        TIME_EFFECTS,
        "runtime.time.cancel_owner",
        StandardSignature {
            params: &[],
            return_type: StandardType::Named("CancelOwner"),
        },
    ),
    runtime_symbol_with_signature(
        "time",
        "cancel_token_from",
        TIME_EFFECTS,
        "runtime.time.cancel_token_from",
        StandardSignature {
            params: PARAM_CANCEL_OWNER,
            return_type: StandardType::Named("CancelToken"),
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
            return_type: StandardType::Named("CancellableWaitOutcome"),
        },
    ),
];

pub(crate) const FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS: &[StandardSymbolDescriptor] = &[
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

pub(crate) const SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS: &[StandardSymbolDescriptor] = &[];

compiler_adapter_symbol_set! {
    "byte",
    "byte_to_int",
    "byte_chunk",
    "byte_chunk_count",
    "byte_append",
    "byte_chunk_from_hex",
    "byte_chunk_to_visible_ascii_string",
    "byte_chunk_from_visible_ascii_string",
    "byte_take",
    "byte_drop",
    "byte_view",
    "byte_view_to_chunk",
    "byte_view_count",
    "byte_view_take",
    "byte_view_drop",
    "byte_view_slice",
    "byte_chunks_empty",
    "byte_chunks_one",
    "byte_chunks_append",
    "byte_chunks_produce",
    "byte_read_u8_be",
    "byte_expect_fixed_u8_be",
    "byte_decode_http2_frame",
    "byte_decode_schema_width_sample",
    "byte_decode_schema_validation_sample",
    "http2_protocol_closed_with_pending",
    "http2_protocol_partial_preface",
    "http2_protocol_invalid_preface",
    "http2_protocol_initial_peer_settings_required",
    "http2_protocol_continuation_expected",
    "http2_protocol_invalid_frame_kind",
    "http2_protocol_invalid_stream_id",
    "http2_protocol_invalid_payload_length",
    "http2_protocol_invalid_payload_length_chunk",
    "http2_protocol_invalid_window_update_increment",
    "http2_protocol_invalid_data_padding",
    "http2_protocol_content_length_mismatch",
    "http2_protocol_invalid_request_header_list",
    "http2_protocol_invalid_response_header_list",
    "http2_protocol_unexpected_settings_ack",
    "http2_protocol_settings_not_allowed_for_endpoint",
    "http2_protocol_invalid_priority_dependency",
    "http2_protocol_stream_after_goaway",
    "http2_peer_limit_frame_size_exceeded",
    "http2_peer_limit_header_list_size_exceeded",
    "http2_peer_limit_header_table_size_exceeded",
    "http2_peer_limit_flow_control_window_exceeded",
    "http2_peer_limit_concurrent_streams_exceeded",
    "http2_peer_limit_settings_value_out_of_range",
    "hpack_fixture_unsupported_header_block",
    "hpack_fixture_unsupported_static_index",
    "hpack_fixture_malformed_string_length",
    "hpack_fixture_malformed_raw_string_value",
    "hpack_fixture_malformed_huffman_padding",
    "hpack_fixture_huffman_eos_symbol",
    "hpack_fixture_huffman_non_visible_value",
    "hpack_fixture_table_size_update_malformed",
    "hpack_fixture_dynamic_index_out_of_range",
    "hpack_fixture_dynamic_name_continuation_missing",
    "hpack_fixture_dynamic_name_continuation_malformed",
    "hpack_fixture_dynamic_name_continuation_out_of_range",
    "hpack_fixture_table_size_update_not_at_start",
    "hpack_fixture_table_size_update_trailing_bytes",
    "byte_read_u16_be",
    "byte_read_u24_be",
    "byte_read_u31_be",
    "byte_read_u32_be",
    "byte_read_u40_be",
    "byte_read_u48_be",
    "byte_read_u56_be",
    "byte_read_u64_be",
    "byte_read_u16_le",
    "byte_read_u24_le",
    "byte_read_u31_le",
    "byte_read_u32_le",
    "byte_read_u40_le",
    "byte_read_u48_le",
    "byte_read_u56_le",
    "byte_read_u64_le",
    "byte_write_u8_be",
    "byte_write_u16_be",
    "byte_write_u24_be",
    "byte_write_u31_be",
    "byte_write_u32_be",
    "byte_write_u40_be",
    "byte_write_u48_be",
    "byte_write_u56_be",
    "byte_write_u64_be",
    "byte_write_u16_le",
    "byte_write_u24_le",
    "byte_write_u31_le",
    "byte_write_u32_le",
    "byte_write_u40_le",
    "byte_write_u48_le",
    "byte_write_u56_le",
    "byte_write_u64_le",
    "byte_count",
    "byte_count_to_int",
    "byte_offset",
    "byte_offset_to_int",
    "stream_adapter_drain_actions",
    "stream_adapter_accept_loop",
    "stream_adapter_drain_actions_until_cancellable",
    "vec_fold",
    "vec_len",
    "vec_is_empty",
    "vec_push",
    "vec_concat",
    "vec_map",
    "vec_filter",
    "vec_try_map",
    "vec_try_map_with",
    "list_nil",
    "list_cons",
    "list_is_empty",
    "list_fold",
    "list_reverse",
    "list_map",
    "list_filter",
    "list_try_map",
    "dict_get",
    "dict_contains",
    "dict_insert",
    "dict_remove",
    "dict_map",
    "dict_map_with",
    "dict_filter",
    "dict_filter_with",
    "dict_fold",
    "dict_fold_with",
    "dict_try_map",
    "dict_try_map_with",
    "option_map",
    "option_and_then",
    "option_unwrap_or",
    "result_map",
    "result_map_err",
    "result_and_then",
    "string_split_once",
    "string_parse_int",
    "int_to_string",
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
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Runtime,
        effects,
        lowering: Some(lowering),
        signature: None,
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
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Runtime,
        effects,
        lowering: Some(lowering),
        signature: Some(signature),
        stability: StandardSymbolStability::RequiredForSelfHosting,
    }
}

const fn prelude_symbol_descriptor(name: &'static str) -> StandardSymbolDescriptor {
    StandardSymbolDescriptor {
        module: None,
        name,
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: PURE_EFFECTS,
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }
}

const fn source_prelude_symbol_descriptor(name: &'static str) -> StandardSymbolDescriptor {
    StandardSymbolDescriptor {
        module: None,
        name,
        name_class: SourceLessNameClass::Function,
        kind: StandardSymbolKind::Prelude,
        effects: PURE_EFFECTS,
        lowering: None,
        signature: None,
        stability: StandardSymbolStability::CompatibilityOnly,
    }
}

pub(crate) fn private_compiler_adapter_name(name: &str) -> bool {
    name == "byte_decode_http2_frame"
        || name.starts_with("http2_protocol_")
        || name.starts_with("http2_peer_limit_")
        || name.starts_with("hpack_fixture_")
}

#[cfg(test)]
fn prelude_symbols() -> impl Iterator<Item = &'static StandardSymbolDescriptor> {
    build_standard_symbol_registry(
        QUALIFIED_SYMBOLS,
        FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS,
        SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS,
        COMPILER_ADAPTER_SYMBOLS,
    )
    .expect("standard symbol registry")
    .prelude
    .into_iter()
}

#[cfg(test)]
fn compatibility_prelude_symbols() -> impl Iterator<Item = &'static StandardSymbolDescriptor> {
    FLOAT_COMPATIBILITY_PRELUDE_SYMBOLS
        .iter()
        .chain(SELF_HOSTING_CANDIDATE_PRELUDE_SYMBOLS.iter())
}

#[cfg(test)]
pub(crate) fn build_standard_symbol_registry(
    qualified: &'static [StandardSymbolDescriptor],
    compatibility_prelude: &'static [StandardSymbolDescriptor],
    self_hosting_prelude: &'static [StandardSymbolDescriptor],
    compiler_adapters: &'static [StandardSymbolDescriptor],
) -> Result<StandardSymbolRegistry, InvalidStandardSymbolCase> {
    build_standard_symbol_registry_with_modules(
        DEFAULT_PRELUDE_BUILTIN_MODULE,
        qualified,
        compatibility_prelude,
        self_hosting_prelude,
        compiler_adapters,
    )
}

pub(crate) fn build_standard_symbol_registry_with_modules(
    prelude_builtin_module: &'static str,
    qualified: &'static [StandardSymbolDescriptor],
    compatibility_prelude: &'static [StandardSymbolDescriptor],
    self_hosting_prelude: &'static [StandardSymbolDescriptor],
    compiler_adapters: &'static [StandardSymbolDescriptor],
) -> Result<StandardSymbolRegistry, InvalidStandardSymbolCase> {
    let mut registry = StandardSymbolRegistry {
        qualified: Vec::new(),
        prelude: Vec::new(),
        compiler_adapters: Vec::new(),
    };
    let mut qualified_keys = BTreeSet::new();
    let mut prelude_keys = BTreeSet::new();
    let mut compiler_adapter_keys = BTreeSet::new();

    for descriptor in qualified {
        validate_source_lookup_descriptor("runtime", descriptor)?;
        validate_qualified_lookup_key("runtime", descriptor, &mut qualified_keys)?;
        registry.qualified.push(descriptor);
    }
    for descriptor in compatibility_prelude
        .iter()
        .chain(self_hosting_prelude.iter())
    {
        validate_source_lookup_descriptor("prelude", descriptor)?;
        validate_prelude_lookup_key("prelude", descriptor, &mut prelude_keys)?;
        registry.prelude.push(descriptor);
    }
    for descriptor in compiler_adapters {
        validate_source_lookup_descriptor("compiler_adapter", descriptor)?;
        validate_prelude_builtin_lookup_key(
            prelude_builtin_module,
            "compiler_adapter",
            descriptor,
            &mut compiler_adapter_keys,
        )?;
        registry.compiler_adapters.push(descriptor);
        if !private_compiler_adapter_name(descriptor.name) {
            validate_prelude_lookup_key("compiler_adapter", descriptor, &mut prelude_keys)?;
            registry.prelude.push(descriptor);
        }
    }

    Ok(registry)
}

fn validate_qualified_lookup_key(
    provider: &'static str,
    descriptor: &StandardSymbolDescriptor,
    keys: &mut BTreeSet<(&'static str, &'static str)>,
) -> Result<(), InvalidStandardSymbolCase> {
    let Some(module) = descriptor.module else {
        return Err(InvalidStandardSymbolCase {
            provider,
            name: descriptor.name.to_string(),
            name_class: SourceLessNameClass::Module,
            reason: InvalidStandardSymbolReason::InvalidLookupKey,
        });
    };
    if keys.insert((module, descriptor.name)) {
        Ok(())
    } else {
        Err(InvalidStandardSymbolCase {
            provider,
            name: format!("{module}::{}", descriptor.name),
            name_class: descriptor.name_class,
            reason: InvalidStandardSymbolReason::DuplicateLookupKey,
        })
    }
}

fn validate_prelude_lookup_key(
    provider: &'static str,
    descriptor: &StandardSymbolDescriptor,
    keys: &mut BTreeSet<&'static str>,
) -> Result<(), InvalidStandardSymbolCase> {
    if descriptor.module.is_some() {
        return Err(InvalidStandardSymbolCase {
            provider,
            name: descriptor.name.to_string(),
            name_class: descriptor.name_class,
            reason: InvalidStandardSymbolReason::InvalidLookupKey,
        });
    }
    if !keys.insert(descriptor.name) {
        return Err(InvalidStandardSymbolCase {
            provider,
            name: descriptor.name.to_string(),
            name_class: descriptor.name_class,
            reason: InvalidStandardSymbolReason::DuplicateLookupKey,
        });
    }
    Ok(())
}

fn validate_prelude_builtin_lookup_key(
    prelude_builtin_module: &'static str,
    provider: &'static str,
    descriptor: &StandardSymbolDescriptor,
    keys: &mut BTreeSet<(&'static str, &'static str)>,
) -> Result<(), InvalidStandardSymbolCase> {
    validate_source_less_name(
        provider,
        prelude_builtin_module,
        SourceLessNameClass::Module,
    )?;
    if descriptor.module.is_some() {
        return Err(InvalidStandardSymbolCase {
            provider,
            name: format!("{prelude_builtin_module}::{}", descriptor.name),
            name_class: descriptor.name_class,
            reason: InvalidStandardSymbolReason::InvalidLookupKey,
        });
    }
    if !keys.insert((prelude_builtin_module, descriptor.name)) {
        return Err(InvalidStandardSymbolCase {
            provider,
            name: format!("{prelude_builtin_module}::{}", descriptor.name),
            name_class: descriptor.name_class,
            reason: InvalidStandardSymbolReason::DuplicateLookupKey,
        });
    }
    Ok(())
}

fn validate_source_lookup_descriptor(
    provider: &'static str,
    descriptor: &StandardSymbolDescriptor,
) -> Result<(), InvalidStandardSymbolCase> {
    if descriptor.name_class != SourceLessNameClass::Function {
        return Err(InvalidStandardSymbolCase {
            provider,
            name: descriptor.name.to_string(),
            name_class: SourceLessNameClass::Function,
            reason: InvalidStandardSymbolReason::InvalidLookupClass,
        });
    }
    if let Some(module) = descriptor.module {
        for segment in module.split("::") {
            validate_source_less_name(provider, segment, SourceLessNameClass::Module)?;
        }
    }
    validate_source_less_name(provider, descriptor.name, descriptor.name_class)
}

#[cfg(test)]
pub(crate) fn compiler_adapter_symbols() -> &'static [StandardSymbolDescriptor] {
    COMPILER_ADAPTER_SYMBOLS
}

#[cfg(test)]
pub(crate) fn compiler_adapter_names() -> impl Iterator<Item = &'static str> {
    compiler_adapter_symbols().iter().map(|symbol| symbol.name)
}

pub(crate) fn effect_strings(symbol: &StandardSymbolDescriptor) -> Vec<String> {
    symbol
        .effects
        .iter()
        .map(|effect| (*effect).to_string())
        .collect()
}

pub(crate) fn standard_function_link_name(module: Option<&str>, name: &str) -> String {
    let Some(module) = module.and_then(|module| module.strip_prefix("std::")) else {
        return name.to_string();
    };
    let module = module.replace("::", "$");
    format!("__veln_std${module}${name}")
}

#[cfg(test)]
#[path = "standard_symbols/tests.rs"]
mod tests;
