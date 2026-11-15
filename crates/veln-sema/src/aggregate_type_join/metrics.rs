#[cfg(test)]
use std::cell::Cell;

use super::Type;
#[cfg(test)]
use super::type_retained_units;

#[cfg(test)]
thread_local! {
    static FALLBACK_COMPARISONS: Cell<usize> = const { Cell::new(0) };
    static BOUNDED_FRAMES: Cell<usize> = const { Cell::new(0) };
    static FUNCTION_SCAN_NODES: Cell<usize> = const { Cell::new(0) };
    static CONSTRAINT_RECORD_FIELD_WORK: Cell<usize> = const { Cell::new(0) };
    static RELATION_CACHE_CURRENT_KEY_BYTES: Cell<usize> = const { Cell::new(0) };
    static BOUNDED_CANDIDATE_WORK: Cell<BoundedCandidateWork> =
        const { Cell::new(BoundedCandidateWork {
            candidate_iterations: 0,
            relation_key_nodes: 0,
            candidate_order_nodes: 0,
            result_materialization_units: 0,
            relation_cache_peak_entries: 0,
            relation_cache_peak_key_bytes: 0,
        }) };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BoundedCandidateWork {
    pub(crate) candidate_iterations: usize,
    pub(crate) relation_key_nodes: usize,
    pub(crate) candidate_order_nodes: usize,
    pub(crate) result_materialization_units: usize,
    pub(crate) relation_cache_peak_entries: usize,
    pub(crate) relation_cache_peak_key_bytes: usize,
}

#[inline(always)]
pub(super) fn record_bounded_candidate_iteration() {
    #[cfg(test)]
    BOUNDED_CANDIDATE_WORK.with(|work| {
        let mut current = work.get();
        current.candidate_iterations += 1;
        work.set(current);
    });
}

#[inline(always)]
pub(super) fn record_relation_key_node() {
    #[cfg(test)]
    BOUNDED_CANDIDATE_WORK.with(|work| {
        let mut current = work.get();
        current.relation_key_nodes += 1;
        work.set(current);
    });
}

#[inline(always)]
pub(super) fn record_candidate_order_node() {
    #[cfg(test)]
    BOUNDED_CANDIDATE_WORK.with(|work| {
        let mut current = work.get();
        current.candidate_order_nodes += 1;
        work.set(current);
    });
}

#[inline(always)]
pub(super) fn record_bounded_result_materialization(ty: &Type) {
    #[cfg(test)]
    BOUNDED_CANDIDATE_WORK.with(|work| {
        let mut current = work.get();
        current.result_materialization_units += type_retained_units(ty);
        work.set(current);
    });
    #[cfg(not(test))]
    let _ = ty;
}

#[inline(always)]
pub(super) fn record_relation_cache_insert(entries: usize, key_bytes: usize) {
    #[cfg(test)]
    RELATION_CACHE_CURRENT_KEY_BYTES.with(|current_key_bytes| {
        let key_bytes = current_key_bytes.get() + key_bytes;
        current_key_bytes.set(key_bytes);
        BOUNDED_CANDIDATE_WORK.with(|work| {
            let mut current = work.get();
            current.relation_cache_peak_entries = current.relation_cache_peak_entries.max(entries);
            current.relation_cache_peak_key_bytes =
                current.relation_cache_peak_key_bytes.max(key_bytes);
            work.set(current);
        });
    });
    #[cfg(not(test))]
    let _ = (entries, key_bytes);
}

pub(super) fn reset_relation_cache_current_key_bytes() {
    #[cfg(test)]
    RELATION_CACHE_CURRENT_KEY_BYTES.with(|current_key_bytes| current_key_bytes.set(0));
}

#[cfg(test)]
pub(crate) fn reset_bounded_candidate_work() {
    BOUNDED_CANDIDATE_WORK.with(|work| work.set(BoundedCandidateWork::default()));
    reset_relation_cache_current_key_bytes();
}

#[cfg(test)]
pub(crate) fn take_bounded_candidate_work() -> BoundedCandidateWork {
    BOUNDED_CANDIDATE_WORK.with(|work| work.replace(BoundedCandidateWork::default()))
}

#[cfg(test)]
pub(super) fn record_fallback_comparison() {
    FALLBACK_COMPARISONS.with(|comparisons| comparisons.set(comparisons.get() + 1));
}

#[cfg(not(test))]
pub(super) fn record_fallback_comparison() {}

#[cfg(test)]
pub(crate) fn reset_fallback_comparisons() {
    FALLBACK_COMPARISONS.with(|comparisons| comparisons.set(0));
}

#[cfg(test)]
pub(crate) fn take_fallback_comparisons() -> usize {
    FALLBACK_COMPARISONS.with(|comparisons| comparisons.replace(0))
}

#[cfg(test)]
pub(super) fn record_bounded_frame() {
    BOUNDED_FRAMES.with(|frames| frames.set(frames.get() + 1));
}

#[cfg(not(test))]
pub(super) fn record_bounded_frame() {}

#[cfg(test)]
pub(crate) fn reset_bounded_frames() {
    BOUNDED_FRAMES.with(|frames| frames.set(0));
}

#[cfg(test)]
pub(crate) fn take_bounded_frames() -> usize {
    BOUNDED_FRAMES.with(|frames| frames.replace(0))
}

#[cfg(test)]
pub(super) fn record_function_scan_node() {
    FUNCTION_SCAN_NODES.with(|nodes| nodes.set(nodes.get() + 1));
}

#[cfg(not(test))]
pub(super) fn record_function_scan_node() {}

#[cfg(test)]
pub(crate) fn reset_function_scan_nodes() {
    FUNCTION_SCAN_NODES.with(|nodes| nodes.set(0));
}

#[cfg(test)]
pub(crate) fn take_function_scan_nodes() -> usize {
    FUNCTION_SCAN_NODES.with(|nodes| nodes.replace(0))
}

#[inline(always)]
pub(super) fn record_constraint_record_field_work() {
    #[cfg(test)]
    CONSTRAINT_RECORD_FIELD_WORK.with(|work| work.set(work.get() + 1));
}

#[cfg(test)]
pub(crate) fn reset_constraint_record_field_work() {
    CONSTRAINT_RECORD_FIELD_WORK.with(|work| work.set(0));
}

#[cfg(test)]
pub(crate) fn take_constraint_record_field_work() -> usize {
    CONSTRAINT_RECORD_FIELD_WORK.with(|work| work.replace(0))
}

#[cfg(test)]
pub(crate) fn record_work(units: usize) {
    crate::inference_work::record(units);
}

#[cfg(not(test))]
pub(crate) fn record_work(_units: usize) {}

#[cfg(test)]
pub(crate) fn reset_work() {
    crate::inference_work::reset();
}

#[cfg(test)]
pub(crate) fn take_work() -> usize {
    crate::inference_work::take()
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DeepJoinWork {
    pub(crate) setup_type_nodes: usize,
    pub(crate) node_visits: usize,
    pub(crate) retained_nodes: usize,
    pub(crate) presentation_nodes: usize,
    pub(crate) materialized_nodes: usize,
}

#[cfg(test)]
thread_local! {
    static DEEP_JOIN_WORK: Cell<DeepJoinWork> = const {
        Cell::new(DeepJoinWork {
            setup_type_nodes: 0,
            node_visits: 0,
            retained_nodes: 0,
            presentation_nodes: 0,
            materialized_nodes: 0,
        })
    };
}

#[cfg(test)]
pub(crate) fn reset_deep_work() {
    DEEP_JOIN_WORK.set(DeepJoinWork::default());
}

#[cfg(test)]
pub(crate) fn take_deep_work() -> DeepJoinWork {
    DEEP_JOIN_WORK.replace(DeepJoinWork::default())
}

#[inline(always)]
pub(super) fn record_setup_type_node() {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.setup_type_nodes += 1;
        work
    });
}

#[inline(always)]
pub(super) fn record_node_visit() {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.node_visits += 1;
        work
    });
}

#[inline(always)]
pub(super) fn record_retained_nodes(nodes: usize) {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.retained_nodes = work.retained_nodes.max(nodes);
        work
    });
    #[cfg(not(test))]
    let _ = nodes;
}

#[inline(always)]
pub(super) fn record_presentation_work(nodes: usize) {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.presentation_nodes += nodes;
        work
    });
    #[cfg(not(test))]
    let _ = nodes;
}

#[inline(always)]
pub(super) fn record_materialization() {
    #[cfg(test)]
    DEEP_JOIN_WORK.update(|mut work| {
        work.materialized_nodes += 1;
        work
    });
}
