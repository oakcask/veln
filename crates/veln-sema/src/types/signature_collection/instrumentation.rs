#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static TYPE_CANONICALIZATION_VISITS: Cell<usize> = const { Cell::new(0) };
    static VARIANT_CANONICALIZATION_LOOKUPS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_type_canonicalization_visits() {
    TYPE_CANONICALIZATION_VISITS.with(|visits| visits.set(0));
}

#[cfg(test)]
pub(crate) fn take_type_canonicalization_visits() -> usize {
    TYPE_CANONICALIZATION_VISITS.with(|visits| visits.replace(0))
}

#[cfg(test)]
pub(crate) fn reset_variant_canonicalization_lookups() {
    VARIANT_CANONICALIZATION_LOOKUPS.with(|lookups| lookups.set(0));
}

#[cfg(test)]
pub(crate) fn take_variant_canonicalization_lookups() -> usize {
    VARIANT_CANONICALIZATION_LOOKUPS.with(|lookups| lookups.replace(0))
}

pub(super) fn record_type_canonicalization_visit() {
    #[cfg(test)]
    TYPE_CANONICALIZATION_VISITS.with(|visits| visits.set(visits.get() + 1));
}

pub(super) fn record_variant_canonicalization_lookup() {
    #[cfg(test)]
    VARIANT_CANONICALIZATION_LOOKUPS.with(|lookups| lookups.set(lookups.get() + 1));
}
