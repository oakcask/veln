#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static IMPORT_CLASSIFICATION_WORK: Cell<usize> = const { Cell::new(0) };
    static IMPORT_CLASSIFICATION_BUILDS: Cell<usize> = const { Cell::new(0) };
    static RECOVERY_TARGET_WORK: Cell<RecoveryTargetWork> =
        const { Cell::new(RecoveryTargetWork {
            declared_identity_bytes: 0,
            candidate_bytes: 0,
            corrected_bytes: 0,
        }) };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RecoveryTargetWork {
    pub(crate) declared_identity_bytes: usize,
    pub(crate) candidate_bytes: usize,
    pub(crate) corrected_bytes: usize,
}

#[cfg(test)]
pub(super) fn record_import_classification_work() {
    IMPORT_CLASSIFICATION_WORK.set(IMPORT_CLASSIFICATION_WORK.get() + 1);
}

#[cfg(not(test))]
pub(super) fn record_import_classification_work() {}

#[cfg(test)]
pub(crate) fn reset_import_classification_work() {
    IMPORT_CLASSIFICATION_WORK.set(0);
    IMPORT_CLASSIFICATION_BUILDS.set(0);
    RECOVERY_TARGET_WORK.set(RecoveryTargetWork::default());
}

#[cfg(test)]
pub(crate) fn import_classification_work() -> usize {
    IMPORT_CLASSIFICATION_WORK.get()
}

#[cfg(test)]
pub(crate) fn import_classification_builds() -> usize {
    IMPORT_CLASSIFICATION_BUILDS.get()
}

#[cfg(test)]
pub(crate) fn recovery_target_work() -> RecoveryTargetWork {
    RECOVERY_TARGET_WORK.get()
}

#[cfg(test)]
pub(super) fn record_recovery_target_work(update: impl FnOnce(&mut RecoveryTargetWork)) {
    RECOVERY_TARGET_WORK.set({
        let mut work = RECOVERY_TARGET_WORK.get();
        update(&mut work);
        work
    });
}

#[cfg(not(test))]
pub(super) fn record_recovery_target_work(_update: impl FnOnce(&mut RecoveryTargetWork)) {}

#[cfg(test)]
pub(super) fn record_import_classification_build() {
    IMPORT_CLASSIFICATION_BUILDS.set(IMPORT_CLASSIFICATION_BUILDS.get() + 1);
}

#[cfg(not(test))]
pub(super) fn record_import_classification_build() {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NormalImportSelectionWork {
    pub(crate) builds: usize,
    pub(crate) build_import_visits: usize,
    pub(crate) build_index_entries: usize,
    pub(crate) lookups: usize,
    pub(crate) lookup_index_probes: usize,
}

#[cfg(test)]
thread_local! {
    static NORMAL_IMPORT_SELECTION_WORK: Cell<NormalImportSelectionWork> =
        const { Cell::new(NormalImportSelectionWork {
            builds: 0,
            build_import_visits: 0,
            build_index_entries: 0,
            lookups: 0,
            lookup_index_probes: 0,
        }) };
}

#[cfg(test)]
pub(super) fn record_normal_import_selection_work(
    update: impl FnOnce(&mut NormalImportSelectionWork),
) {
    NORMAL_IMPORT_SELECTION_WORK.set({
        let mut work = NORMAL_IMPORT_SELECTION_WORK.get();
        update(&mut work);
        work
    });
}

#[cfg(not(test))]
pub(super) fn record_normal_import_selection_work(
    _update: impl FnOnce(&mut NormalImportSelectionWork),
) {
}

#[cfg(test)]
pub(crate) fn reset_normal_import_selection_work() {
    NORMAL_IMPORT_SELECTION_WORK.set(NormalImportSelectionWork::default());
}

#[cfg(test)]
pub(crate) fn normal_import_selection_work() -> NormalImportSelectionWork {
    NORMAL_IMPORT_SELECTION_WORK.get()
}
