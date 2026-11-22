use super::{
    PURE_EFFECTS, SourceLessNameClass, StandardSymbolDescriptor, StandardSymbolKind,
    StandardSymbolStability,
};

pub(super) const fn source_prelude_symbol_descriptor(
    name: &'static str,
) -> StandardSymbolDescriptor {
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
    name.starts_with("observation_")
        || name.starts_with("production_metric_index_")
        || name.starts_with("net_system_")
        || name.starts_with("io_provider_")
        || name.starts_with("fs_system_")
        || name == "fs_read_only"
        || matches!(name, "io_read_exact" | "io_write_all" | "io_copy")
        || name == "string_contains_nul"
        || name == "string_nul"
        || name == "string_split_all"
        || name == "string_split_last"
        || name == "byte_decode_http2_frame"
        || name.starts_with("http2_protocol_")
        || name.starts_with("http2_peer_limit_")
        || name.starts_with("hpack_fixture_")
}
