//! Embedded Veln standard library sources.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdlibSource {
    pub path: &'static str,
    pub entry: &'static str,
    pub text: &'static str,
}

const CORE_PRELUDE_PATH: &str = "stdlib/core_prelude.veln";
const CORE_PRELUDE_TEXT: &str = include_str!("../veln/core_prelude.veln");
const CORE_PRELUDE_VEC_FOLD_PATH: &str = "stdlib/core_prelude_vec_fold.veln";
const CORE_PRELUDE_VEC_FOLD_TEXT: &str = include_str!("../veln/core_prelude_vec_fold.veln");

pub const fn core_prelude_source(entry: &'static str) -> StdlibSource {
    StdlibSource {
        path: CORE_PRELUDE_PATH,
        entry,
        text: CORE_PRELUDE_TEXT,
    }
}

pub const fn core_prelude_vec_fold_source() -> StdlibSource {
    StdlibSource {
        path: CORE_PRELUDE_VEC_FOLD_PATH,
        entry: "vec_fold",
        text: CORE_PRELUDE_VEC_FOLD_TEXT,
    }
}

pub static COMPILER_SUPPORT: StdlibSource = StdlibSource {
    path: "stdlib/compiler_support.veln",
    entry: "load_source_text",
    text: include_str!("../veln/compiler_support.veln"),
};
