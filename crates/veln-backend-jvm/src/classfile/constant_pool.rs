use super::*;

#[derive(Default)]
pub(super) struct ConstantPool {
    entries: Vec<CpEntry>,
    indexes: BTreeMap<CpKey, u16>,
    error: Option<JvmBackendError>,
}

impl ConstantPool {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn utf8(&mut self, value: &str) -> u16 {
        self.intern(CpKey::Utf8(value.to_string()), |key| match key {
            CpKey::Utf8(value) => CpEntry::Utf8(value.clone()),
            _ => unreachable!(),
        })
    }

    pub(super) fn class(&mut self, name: &str) -> u16 {
        let name_index = self.utf8(name);
        self.intern(CpKey::Class(name.to_string()), |_| {
            CpEntry::Class(name_index)
        })
    }

    pub(super) fn string(&mut self, value: &str) -> u16 {
        let utf8 = self.utf8(value);
        self.intern(CpKey::String(value.to_string()), |_| CpEntry::String(utf8))
    }

    pub(super) fn long(&mut self, value: i64) -> u16 {
        self.intern(CpKey::Long(value), |_| CpEntry::Long(value))
    }

    pub(super) fn integer(&mut self, value: i32) -> u16 {
        self.intern(CpKey::Integer(value), |_| CpEntry::Integer(value))
    }

    pub(super) fn double(&mut self, value: f64) -> u16 {
        self.intern(CpKey::Double(value.to_bits()), |_| {
            CpEntry::Double(value.to_bits())
        })
    }

    pub(super) fn name_and_type(&mut self, name: &str, descriptor: &str) -> u16 {
        let name_index = self.utf8(name);
        let descriptor_index = self.utf8(descriptor);
        self.intern(
            CpKey::NameAndType(name.to_string(), descriptor.to_string()),
            |_| CpEntry::NameAndType {
                name_index,
                descriptor_index,
            },
        )
    }

    pub(super) fn fieldref(&mut self, class: &str, name: &str, descriptor: &str) -> u16 {
        let class_index = self.class(class);
        let name_and_type = self.name_and_type(name, descriptor);
        self.intern(
            CpKey::Fieldref(class.to_string(), name.to_string(), descriptor.to_string()),
            |_| CpEntry::Fieldref {
                class_index,
                name_and_type,
            },
        )
    }

    pub(super) fn methodref(&mut self, class: &str, name: &str, descriptor: &str) -> u16 {
        let class_index = self.class(class);
        let name_and_type = self.name_and_type(name, descriptor);
        self.intern(
            CpKey::Methodref(class.to_string(), name.to_string(), descriptor.to_string()),
            |_| CpEntry::Methodref {
                class_index,
                name_and_type,
            },
        )
    }

    pub(super) fn intern<F>(&mut self, key: CpKey, build: F) -> u16
    where
        F: FnOnce(&CpKey) -> CpEntry,
    {
        if let Some(index) = self.indexes.get(&key) {
            return *index;
        }
        if self.error.is_some() {
            return 1;
        }
        let entry_slots = if matches!(key, CpKey::Long(_) | CpKey::Double(_)) {
            2
        } else {
            1
        };
        let Some(index) = checked_constant_pool_addition(self.entries.len(), entry_slots) else {
            self.error = Some(JvmBackendError::new(
                "JVM class constant pool exceeds the classfile limit",
            ));
            return 1;
        };
        let entry = build(&key);
        self.entries.push(entry);
        if entry_slots == 2 {
            self.entries.push(CpEntry::Padding);
        }
        self.indexes.insert(key, index);
        index
    }

    pub(super) fn write(&self, out: &mut Vec<u8>) -> Result<(), JvmBackendError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let count = checked_constant_pool_count(self.entries.len()).ok_or_else(|| {
            JvmBackendError::new("JVM class constant pool exceeds the classfile limit")
        })?;
        for entry in &self.entries {
            if let CpEntry::Utf8(value) = entry {
                checked_utf8_length(value).ok_or_else(|| {
                    JvmBackendError::new("JVM class constant-pool string exceeds 65535 bytes")
                })?;
            }
        }

        write_u16(out, count);
        for entry in &self.entries {
            entry.write(out);
        }
        Ok(())
    }
}

fn checked_constant_pool_addition(existing_slots: usize, added_slots: usize) -> Option<u16> {
    existing_slots
        .checked_add(added_slots)
        .and_then(|slots| slots.checked_add(1))
        .and_then(|count| u16::try_from(count).ok())?;
    existing_slots
        .checked_add(1)
        .and_then(|index| u16::try_from(index).ok())
}

fn checked_constant_pool_count(entry_slots: usize) -> Option<u16> {
    entry_slots
        .checked_add(1)
        .and_then(|count| u16::try_from(count).ok())
}

fn checked_utf8_length(value: &str) -> Option<u16> {
    u16::try_from(value.len()).ok()
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CpKey {
    Utf8(String),
    Class(String),
    String(String),
    Fieldref(String, String, String),
    Methodref(String, String, String),
    NameAndType(String, String),
    Long(i64),
    Integer(i32),
    Double(u64),
}

pub(super) enum CpEntry {
    Utf8(String),
    Class(u16),
    String(u16),
    Fieldref {
        class_index: u16,
        name_and_type: u16,
    },
    Methodref {
        class_index: u16,
        name_and_type: u16,
    },
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    Long(i64),
    Integer(i32),
    Double(u64),
    Padding,
}

impl CpEntry {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Utf8(value) => {
                out.push(1);
                write_u16(
                    out,
                    checked_utf8_length(value)
                        .expect("constant-pool UTF-8 lengths are checked before serialization"),
                );
                out.extend_from_slice(value.as_bytes());
            }
            Self::Class(index) => {
                out.push(7);
                write_u16(out, *index);
            }
            Self::String(index) => {
                out.push(8);
                write_u16(out, *index);
            }
            Self::Fieldref {
                class_index,
                name_and_type,
            } => {
                out.push(9);
                write_u16(out, *class_index);
                write_u16(out, *name_and_type);
            }
            Self::Methodref {
                class_index,
                name_and_type,
            } => {
                out.push(10);
                write_u16(out, *class_index);
                write_u16(out, *name_and_type);
            }
            Self::NameAndType {
                name_index,
                descriptor_index,
            } => {
                out.push(12);
                write_u16(out, *name_index);
                write_u16(out, *descriptor_index);
            }
            Self::Long(value) => {
                out.push(5);
                out.extend_from_slice(&value.to_be_bytes());
            }
            Self::Integer(value) => {
                out.push(3);
                out.extend_from_slice(&value.to_be_bytes());
            }
            Self::Double(bits) => {
                out.push(6);
                out.extend_from_slice(&bits.to_be_bytes());
            }
            Self::Padding => {}
        }
    }
}

pub(super) fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(super) fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_rejects_an_entry_that_would_overflow_the_constant_pool_count() {
        let mut pool = ConstantPool::new();
        pool.entries
            .resize_with(usize::from(u16::MAX) - 1, || CpEntry::Padding);

        let index = pool.integer(7);

        assert_eq!(index, 1);
        assert_eq!(
            pool.error.as_ref().map(ToString::to_string).as_deref(),
            Some("JVM class constant pool exceeds the classfile limit")
        );
        assert_eq!(pool.entries.len(), usize::from(u16::MAX) - 1);
        assert!(!pool.indexes.contains_key(&CpKey::Integer(7)));
    }

    #[test]
    fn intern_accounts_for_two_slot_constants_before_mutating_the_pool() {
        let mut pool = ConstantPool::new();
        pool.entries
            .resize_with(usize::from(u16::MAX) - 2, || CpEntry::Padding);

        let index = pool.long(7);

        assert_eq!(index, 1);
        assert!(pool.error.is_some());
        assert_eq!(pool.entries.len(), usize::from(u16::MAX) - 2);
        assert!(!pool.indexes.contains_key(&CpKey::Long(7)));
    }

    #[test]
    fn write_rejects_an_unrepresentable_count_before_writing_bytes() {
        let mut pool = ConstantPool::new();
        pool.entries
            .resize_with(usize::from(u16::MAX), || CpEntry::Padding);
        let mut out = vec![1, 2, 3];

        let failure = pool.write(&mut out);

        assert!(failure.is_err());
        assert_eq!(out, vec![1, 2, 3]);
    }

    #[test]
    fn write_rejects_an_oversized_utf8_entry_before_writing_bytes() {
        let pool = ConstantPool {
            entries: vec![CpEntry::Utf8("x".repeat(usize::from(u16::MAX) + 1))],
            indexes: BTreeMap::new(),
            error: None,
        };
        let mut out = vec![1, 2, 3];

        let failure = pool.write(&mut out);

        assert!(failure.is_err());
        assert_eq!(out, vec![1, 2, 3]);
    }
}
