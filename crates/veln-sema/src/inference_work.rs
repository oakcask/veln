#[cfg(test)]
thread_local! {
    static WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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
