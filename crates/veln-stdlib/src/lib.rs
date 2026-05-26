//! Embedded Veln standard library sources.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdlibSource {
    pub path: &'static str,
    pub entry: &'static str,
    pub text: &'static str,
}

pub const CORE_PRELUDE: StdlibSource = StdlibSource {
    path: "stdlib/core_prelude.veln",
    entry: "option_unwrap_or",
    text: include_str!("../veln/core_prelude.veln"),
};

pub const COMPILER_SUPPORT: StdlibSource = StdlibSource {
    path: "stdlib/compiler_support.veln",
    entry: "load_source_text",
    text: include_str!("../veln/compiler_support.veln"),
};
