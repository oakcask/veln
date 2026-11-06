#[cfg(test)]
thread_local! {
    static WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static COMPLETION_PAYLOAD_CONTEXT_TYPE_NODES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn record(units: usize) {
    WORK.with(|work| work.set(work.get() + units));
}

#[cfg(not(test))]
pub(crate) fn record(_units: usize) {}

#[cfg(test)]
pub(crate) fn reset() {
    WORK.with(|work| work.set(0));
}

#[cfg(test)]
pub(crate) fn take() -> usize {
    WORK.with(|work| work.replace(0))
}

#[cfg(test)]
#[inline(always)]
pub(crate) fn record_completion_payload_context_type_node() {
    COMPLETION_PAYLOAD_CONTEXT_TYPE_NODES.with(|work| work.set(work.get() + 1));
}

#[cfg(not(test))]
#[inline(always)]
pub(crate) fn record_completion_payload_context_type_node() {}

#[cfg(test)]
pub(crate) fn reset_completion_payload_context_type_nodes() {
    COMPLETION_PAYLOAD_CONTEXT_TYPE_NODES.with(|work| work.set(0));
}

#[cfg(test)]
pub(crate) fn take_completion_payload_context_type_nodes() -> usize {
    COMPLETION_PAYLOAD_CONTEXT_TYPE_NODES.with(|work| work.replace(0))
}
