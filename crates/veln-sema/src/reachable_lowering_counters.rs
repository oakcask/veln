use std::cell::Cell;

use veln_ast::Function;

thread_local! {
    static APPLICATION_BODY_CHECKS: Cell<usize> = const { Cell::new(0) };
    static APPLICATION_CORE_LOWERS: Cell<usize> = const { Cell::new(0) };
}

pub fn reset() {
    APPLICATION_BODY_CHECKS.set(0);
    APPLICATION_CORE_LOWERS.set(0);
}

pub fn application_body_checks() -> usize {
    APPLICATION_BODY_CHECKS.get()
}

pub fn application_core_lowers() -> usize {
    APPLICATION_CORE_LOWERS.get()
}

pub(crate) fn record_application_body_check(function: &Function) {
    if is_application_function(function) {
        APPLICATION_BODY_CHECKS.set(APPLICATION_BODY_CHECKS.get() + 1);
    }
}

pub(crate) fn record_application_core_lower(function: &Function) {
    if is_application_function(function) {
        APPLICATION_CORE_LOWERS.set(APPLICATION_CORE_LOWERS.get() + 1);
    }
}

fn is_application_function(function: &Function) -> bool {
    !function
        .module_name
        .as_deref()
        .is_some_and(|module| module.starts_with("std::"))
}
