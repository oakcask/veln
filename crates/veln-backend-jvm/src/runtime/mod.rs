use veln_ast::BinaryOp;

macro_rules! runtime_method_table {
    ($name:ident, $fallback:expr, {$($surface:literal => $method:literal,)+}) => {
        pub(crate) fn $name(name: &str) -> &'static str {
            match name {
                $($surface => $method,)+
                _ => $fallback,
            }
        }
    };
}

runtime_method_table!(stdio_method, "stdioPrintln", {
    "stdio::print" => "stdioPrint",
    "stdio::println" => "stdioPrintln",
    "stdio::eprint" => "stdioEprint",
    "stdio::eprintln" => "stdioEprintln",
});

runtime_method_table!(prelude_method, "vecLen", {
    "float_negate" => "floatNegate",
    "float_add" => "floatAdd",
    "float_subtract" => "floatSubtract",
    "float_multiply" => "floatMultiply",
    "float_divide" => "floatDivide",
    "float_less" => "floatLess",
    "float_less_equal" => "floatLessEqual",
    "float_greater" => "floatGreater",
    "float_greater_equal" => "floatGreaterEqual",
    "byte" => "byteValue",
    "byte_to_int" => "byteToInt",
    "byte_chunk" => "byteChunk",
    "byte_chunk_count" => "byteChunkCount",
    "byte_append" => "byteAppend",
    "byte_chunk_from_hex" => "byteChunkFromHex",
    "byte_take" => "byteTake",
    "byte_drop" => "byteDrop",
    "byte_view" => "byteView",
    "byte_view_to_chunk" => "byteViewToChunk",
    "byte_read_u8_be" => "byteReadU8Be",
    "byte_expect_fixed_u8_be" => "byteExpectFixedU8Be",
    "byte_decode_http2_frame_header" => "byteDecodeHttp2FrameHeader",
    "byte_decode_http2_frame" => "byteDecodeHttp2Frame",
    "http2_protocol_closed_with_pending" => "http2ProtocolClosedWithPending",
    "http2_protocol_continuation_expected" => "http2ProtocolContinuationExpected",
    "byte_read_u16_be" => "byteReadU16Be",
    "byte_read_u24_be" => "byteReadU24Be",
    "byte_read_u31_be" => "byteReadU31Be",
    "byte_read_u32_be" => "byteReadU32Be",
    "byte_write_u8_be" => "byteWriteU8Be",
    "byte_write_u16_be" => "byteWriteU16Be",
    "byte_write_u24_be" => "byteWriteU24Be",
    "byte_write_u31_be" => "byteWriteU31Be",
    "byte_write_u32_be" => "byteWriteU32Be",
    "byte_count" => "byteCount",
    "byte_count_to_int" => "byteCountToInt",
    "byte_offset" => "byteOffset",
    "byte_offset_to_int" => "byteOffsetToInt",
    "string_split_once" => "stringSplitOnce",
    "string_parse_int" => "stringParseInt",
    "int_to_string" => "intToString",
    "vec_len" => "vecLen",
    "vec_is_empty" => "vecIsEmpty",
    "vec_push" => "vecPush",
    "vec_concat" => "vecConcat",
    "vec_map" => "vecMap",
    "vec_filter" => "vecFilter",
    "vec_fold" => "vecFold",
    "vec_try_map" => "vecTryMap",
    "vec_try_map_with" => "vecTryMapWith",
    "list_nil" => "listNil",
    "list_cons" => "listCons",
    "list_is_empty" => "listIsEmpty",
    "list_fold" => "listFold",
    "list_reverse" => "listReverse",
    "list_map" => "listMap",
    "list_filter" => "listFilter",
    "list_try_map" => "listTryMap",
    "dict_get" => "dictGet",
    "dict_contains" => "dictContains",
    "dict_insert" => "dictInsert",
    "dict_remove" => "dictRemove",
    "option_map" => "optionMap",
    "option_and_then" => "optionAndThen",
    "option_unwrap_or" => "optionUnwrapOr",
    "result_map" => "resultMap",
    "result_map_err" => "resultMapErr",
    "result_and_then" => "resultAndThen",
});

runtime_method_table!(concurrency_method, "channelRecv", {
    "channel::bounded" => "channelBounded",
    "channel::clone" => "channelClone",
    "channel::send" => "channelSend",
    "channel::recv" => "channelRecv",
    "channel::select" => "channelSelect",
    "channel::select_priority" => "channelSelectPriority",
    "channel::select_timeout" => "channelSelectTimeout",
    "channel::select_result" => "channelSelectResult",
    "channel::select_priority_result" => "channelSelectPriorityResult",
    "channel::select_timeout_result" => "channelSelectTimeoutResult",
    "channel::close" => "channelClose",
    "task::spawn" => "taskSpawn",
    "task::join" => "taskJoin",
    "task::cancel" => "taskCancel",
});

pub(crate) fn standard_library_method(name: &str) -> &'static str {
    match name {
        "fs::read_to_string" => "fsReadToString",
        "fs::write_string" => "fsWriteString",
        "fs::exists" => "fsExists",
        "fs::read_dir" => "fsReadDir",
        "process::args" => "processArgs",
        "process::env" => "processEnv",
        "process::cwd" => "processCwd",
        "process::exit" => "processExit",
        _ => panic!("unknown standard library builtin `{name}`"),
    }
}

pub(crate) fn binary_method(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::PipeGreater => "pipe",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::Equal => "equal",
        BinaryOp::NotEqual => "notEqual",
        BinaryOp::Less => "less",
        BinaryOp::LessEqual => "lessEqual",
        BinaryOp::Greater => "greater",
        BinaryOp::GreaterEqual => "greaterEqual",
        BinaryOp::Add => "add",
        BinaryOp::Subtract => "subtract",
        BinaryOp::Multiply => "multiply",
        BinaryOp::Divide => "divide",
    }
}
