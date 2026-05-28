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
    pub(crate) source: Option<&'static veln_stdlib::StdlibSource>,
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

const DESCRIPTOR_PRELUDE_SYMBOLS: &[StandardSymbolDescriptor] = &[
    prelude_symbol_descriptor("float_negate"),
    prelude_symbol_descriptor("float_add"),
    prelude_symbol_descriptor("float_subtract"),
    prelude_symbol_descriptor("float_multiply"),
    prelude_symbol_descriptor("float_divide"),
    prelude_symbol_descriptor("float_less"),
    prelude_symbol_descriptor("float_less_equal"),
    prelude_symbol_descriptor("float_greater"),
    prelude_symbol_descriptor("float_greater_equal"),
    prelude_symbol_descriptor("string_split_once"),
    prelude_symbol_descriptor("string_parse_int"),
    prelude_symbol_descriptor("int_to_string"),
    prelude_symbol_descriptor("vec_len"),
    prelude_symbol_descriptor("vec_push"),
    prelude_symbol_descriptor("vec_concat"),
    prelude_symbol_descriptor("vec_map"),
    prelude_symbol_descriptor("vec_filter"),
    prelude_symbol_descriptor("vec_fold"),
    prelude_symbol_descriptor("vec_try_map"),
    prelude_symbol_descriptor("vec_try_map_with"),
    prelude_symbol_descriptor("dict_get"),
    prelude_symbol_descriptor("dict_contains"),
    prelude_symbol_descriptor("dict_insert"),
    prelude_symbol_descriptor("dict_remove"),
];

const SOURCE_PRELUDE_SYMBOLS: &[StandardSymbolDescriptor] = &[
    source_prelude_symbol_descriptor("vec_is_empty", &veln_stdlib::CORE_PRELUDE_VEC_IS_EMPTY),
    source_prelude_symbol_descriptor("option_map", &veln_stdlib::CORE_PRELUDE_OPTION_MAP),
    source_prelude_symbol_descriptor(
        "option_and_then",
        &veln_stdlib::CORE_PRELUDE_OPTION_AND_THEN,
    ),
    source_prelude_symbol_descriptor(
        "option_unwrap_or",
        &veln_stdlib::CORE_PRELUDE_OPTION_UNWRAP_OR,
    ),
    source_prelude_symbol_descriptor("result_map", &veln_stdlib::CORE_PRELUDE_RESULT_MAP),
    source_prelude_symbol_descriptor("result_map_err", &veln_stdlib::CORE_PRELUDE_RESULT_MAP_ERR),
    source_prelude_symbol_descriptor(
        "result_and_then",
        &veln_stdlib::CORE_PRELUDE_RESULT_AND_THEN,
    ),
];

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
    source: &'static veln_stdlib::StdlibSource,
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
    DESCRIPTOR_PRELUDE_SYMBOLS
        .iter()
        .chain(SOURCE_PRELUDE_SYMBOLS.iter())
}

#[cfg(test)]
pub(crate) fn source_backed_symbols() -> impl Iterator<Item = &'static StandardSymbolDescriptor> {
    SOURCE_PRELUDE_SYMBOLS
        .iter()
        .chain(QUALIFIED_SYMBOLS)
        .filter(|symbol| symbol.source.is_some())
}

#[allow(dead_code)]
pub(crate) fn compiler_support_sources() -> impl Iterator<Item = &'static veln_stdlib::StdlibSource>
{
    [&veln_stdlib::COMPILER_SUPPORT].into_iter()
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
        let symbol = prelude_symbol("vec_len").expect("prelude descriptor");

        assert_eq!(symbol.kind, StandardSymbolKind::Prelude);
        assert!(symbol.effects.is_empty());
        assert_eq!(symbol.lowering, None);
        assert_eq!(symbol.stability, StandardSymbolStability::CompatibilityOnly);
    }

    #[test]
    fn source_backed_prelude_descriptors_carry_metadata() {
        let expected: Vec<_> = SOURCE_PRELUDE_SYMBOLS
            .iter()
            .map(|symbol| symbol.name)
            .collect();
        let mut entries = Vec::new();

        for name in expected.iter().copied() {
            let symbol = prelude_symbol(name).expect("source-backed helper descriptor");
            let source = symbol.source.expect("source metadata");

            assert_eq!(symbol.kind, StandardSymbolKind::Veln);
            assert!(symbol.effects.is_empty());
            assert_eq!(symbol.lowering, None);
            assert_eq!(source.entry, symbol.name);
            assert_eq!(source.path, "stdlib/core_prelude.veln");
            assert!(source.text.contains(&format!("fn {name}")));
            entries.push(source.entry);
        }

        assert_eq!(entries, expected);
    }

    #[test]
    fn descriptor_only_prelude_helpers_do_not_carry_source_metadata() {
        let symbol = prelude_symbol("dict_get").expect("prelude descriptor");

        assert_eq!(symbol.kind, StandardSymbolKind::Prelude);
        assert_eq!(symbol.lowering, None);
        assert!(symbol.effects.is_empty());
        assert_eq!(symbol.source, None);
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
