//! Embedded Veln standard library sources.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdlibSource {
    pub path: &'static str,
    pub entry: &'static str,
    pub text: &'static str,
}

const CORE_PRELUDE_PATH: &str = "stdlib/core_prelude.veln";
const CORE_PRELUDE_TEXT: &str = include_str!("../veln/core_prelude.veln");

pub const fn core_prelude_source(entry: &'static str) -> StdlibSource {
    StdlibSource {
        path: CORE_PRELUDE_PATH,
        entry,
        text: CORE_PRELUDE_TEXT,
    }
}

pub static CORE_PRELUDE_VEC_LEN: StdlibSource = core_prelude_source("vec_len");

pub static CORE_PRELUDE_VEC_IS_EMPTY: StdlibSource = core_prelude_source("vec_is_empty");

pub static CORE_PRELUDE_VEC_PUSH: StdlibSource = core_prelude_source("vec_push");

pub static CORE_PRELUDE_VEC_CONCAT: StdlibSource = core_prelude_source("vec_concat");

pub static CORE_PRELUDE_VEC_MAP: StdlibSource = core_prelude_source("vec_map");

pub static CORE_PRELUDE_VEC_FILTER: StdlibSource = core_prelude_source("vec_filter");

pub static CORE_PRELUDE_VEC_TRY_MAP: StdlibSource = core_prelude_source("vec_try_map");

pub static CORE_PRELUDE_VEC_TRY_MAP_WITH: StdlibSource = core_prelude_source("vec_try_map_with");

pub static CORE_PRELUDE_OPTION_MAP: StdlibSource = core_prelude_source("option_map");

pub static CORE_PRELUDE_OPTION_AND_THEN: StdlibSource = core_prelude_source("option_and_then");

pub static CORE_PRELUDE_OPTION_UNWRAP_OR: StdlibSource = core_prelude_source("option_unwrap_or");

pub static CORE_PRELUDE_RESULT_MAP: StdlibSource = core_prelude_source("result_map");

pub static CORE_PRELUDE_RESULT_MAP_ERR: StdlibSource = core_prelude_source("result_map_err");

pub static CORE_PRELUDE_RESULT_AND_THEN: StdlibSource = core_prelude_source("result_and_then");

pub static CORE_PRELUDE_DICT_CONTAINS: StdlibSource = core_prelude_source("dict_contains");

pub static CORE_PRELUDE: StdlibSource = core_prelude_source("option_unwrap_or");

pub static COMPILER_SUPPORT: StdlibSource = StdlibSource {
    path: "stdlib/compiler_support.veln",
    entry: "load_source_text",
    text: include_str!("../veln/compiler_support.veln"),
};
