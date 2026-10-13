use super::*;

pub(super) struct LocalBindings {
    values: BTreeMap<String, u16>,
    changes: Vec<LocalBindingChange>,
    #[cfg(test)]
    peak_retained_entry_count: usize,
}

struct LocalBindingChange {
    name: String,
    previous: Option<u16>,
}

impl LocalBindings {
    pub(super) fn new(values: BTreeMap<String, u16>) -> Self {
        #[cfg(test)]
        let peak_retained_entry_count = values.len();
        Self {
            values,
            changes: Vec::new(),
            #[cfg(test)]
            peak_retained_entry_count,
        }
    }

    pub(super) fn mark(&self) -> usize {
        self.changes.len()
    }

    pub(super) fn insert(&mut self, name: String, slot: u16) {
        let previous = self.values.insert(name.clone(), slot);
        self.changes.push(LocalBindingChange { name, previous });
        #[cfg(test)]
        {
            self.peak_retained_entry_count = self
                .peak_retained_entry_count
                .max(self.values.len() + self.changes.len());
        }
    }

    pub(super) fn get(&self, name: &str) -> Option<&u16> {
        self.values.get(name)
    }

    pub(super) fn contains_key(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    pub(super) fn rollback(&mut self, mark: usize) {
        while self.changes.len() > mark {
            let change = self.changes.pop().expect("local binding change");
            if let Some(previous) = change.previous {
                self.values.insert(change.name, previous);
            } else {
                self.values.remove(&change.name);
            }
        }
    }

    #[cfg(test)]
    pub(super) fn peak_retained_entry_count(&self) -> usize {
        self.peak_retained_entry_count
    }
}
