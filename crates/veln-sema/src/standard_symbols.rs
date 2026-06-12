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
    pub(crate) source: Option<veln_stdlib::StdlibSource>,
    pub(crate) stability: StandardSymbolStability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StandardSymbolStability {
    RequiredForSelfHosting,
    CompatibilityOnly,
}

const STDIO_EFFECTS: &[&str] = &["stdio"];
const CONCURRENCY_EFFECTS: &[&str] = &["concurrency"];
const FS_EFFECTS: &[&str] = &["fs"];
const PROCESS_EFFECTS: &[&str] = &["process"];
const PURE_EFFECTS: &[&str] = &[];
#[cfg(test)]
const SOURCE_BACKED_PRIVATE_HELPERS: &[&str] = &[
    "vec_map_step",
    "vec_try_map_step",
    "vec_try_map_with_step",
    "list_reverse_step",
    "list_map_step",
    "list_filter_step",
    "list_try_map_step",
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
        "select_timeout",
        CONCURRENCY_EFFECTS,
        "runtime.channel.select_timeout",
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
    runtime_symbol("task", "join", CONCURRENCY_EFFECTS, "runtime.task.join"),
    runtime_symbol("task", "cancel", CONCURRENCY_EFFECTS, "runtime.task.cancel"),
    runtime_symbol(
        "fs",
        "read_to_string",
        FS_EFFECTS,
        "runtime.fs.read_to_string",
    ),
    runtime_symbol("fs", "write_string", FS_EFFECTS, "runtime.fs.write_string"),
    runtime_symbol("fs", "exists", FS_EFFECTS, "runtime.fs.exists"),
    runtime_symbol("fs", "read_dir", FS_EFFECTS, "runtime.fs.read_dir"),
    runtime_symbol("process", "args", PROCESS_EFFECTS, "runtime.process.args"),
    runtime_symbol("process", "env", PROCESS_EFFECTS, "runtime.process.env"),
    runtime_symbol("process", "cwd", PROCESS_EFFECTS, "runtime.process.cwd"),
    runtime_symbol("process", "exit", PROCESS_EFFECTS, "runtime.process.exit"),
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
    "byte_take" => veln_stdlib::prelude_source("byte_take"),
    "byte_drop" => veln_stdlib::prelude_source("byte_drop"),
    "byte_view" => veln_stdlib::prelude_source("byte_view"),
    "byte_view_to_chunk" => veln_stdlib::prelude_source("byte_view_to_chunk"),
    "byte_read_u8_be" => veln_stdlib::prelude_source("byte_read_u8_be"),
    "byte_expect_fixed_u8_be" => veln_stdlib::prelude_source("byte_expect_fixed_u8_be"),
    "byte_decode_http2_frame_header" => veln_stdlib::prelude_source("byte_decode_http2_frame_header"),
    "byte_decode_http2_frame" => veln_stdlib::prelude_source("byte_decode_http2_frame"),
    "byte_decode_schema_width_sample" => veln_stdlib::prelude_source("byte_decode_schema_width_sample"),
    "byte_decode_schema_validation_sample" => veln_stdlib::prelude_source("byte_decode_schema_validation_sample"),
    "http2_protocol_closed_with_pending" => veln_stdlib::prelude_source("http2_protocol_closed_with_pending"),
    "http2_protocol_continuation_expected" => veln_stdlib::prelude_source("http2_protocol_continuation_expected"),
    "http2_protocol_invalid_frame_kind" => veln_stdlib::prelude_source("http2_protocol_invalid_frame_kind"),
    "http2_peer_limit_frame_size_exceeded" => veln_stdlib::prelude_source("http2_peer_limit_frame_size_exceeded"),
    "byte_read_u16_be" => veln_stdlib::prelude_source("byte_read_u16_be"),
    "byte_read_u24_be" => veln_stdlib::prelude_source("byte_read_u24_be"),
    "byte_read_u31_be" => veln_stdlib::prelude_source("byte_read_u31_be"),
    "byte_read_u32_be" => veln_stdlib::prelude_source("byte_read_u32_be"),
    "byte_write_u8_be" => veln_stdlib::prelude_source("byte_write_u8_be"),
    "byte_write_u16_be" => veln_stdlib::prelude_source("byte_write_u16_be"),
    "byte_write_u24_be" => veln_stdlib::prelude_source("byte_write_u24_be"),
    "byte_write_u31_be" => veln_stdlib::prelude_source("byte_write_u31_be"),
    "byte_write_u32_be" => veln_stdlib::prelude_source("byte_write_u32_be"),
    "byte_count" => veln_stdlib::prelude_source("byte_count"),
    "byte_count_to_int" => veln_stdlib::prelude_source("byte_count_to_int"),
    "byte_offset" => veln_stdlib::prelude_source("byte_offset"),
    "byte_offset_to_int" => veln_stdlib::prelude_source("byte_offset_to_int"),
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
