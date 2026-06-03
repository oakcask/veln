//! Embedded Veln standard library sources.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdlibSource {
    pub path: &'static str,
    pub entry: &'static str,
    pub text: &'static str,
}

const PRELUDE_PATH: &str = "prelude.veln";
const PRELUDE_TEXT: &str = include_str!("../veln/prelude.veln");

pub const fn prelude_source(entry: &'static str) -> StdlibSource {
    StdlibSource {
        path: PRELUDE_PATH,
        entry,
        text: PRELUDE_TEXT,
    }
}

pub static COMPILER_SUPPORT: StdlibSource = StdlibSource {
    path: "compiler_support.veln",
    entry: "load_source_text",
    text: include_str!("../veln/compiler_support.veln"),
};
