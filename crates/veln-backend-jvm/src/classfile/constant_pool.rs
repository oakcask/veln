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
        let Some(encoded) = encode_modified_utf8(value) else {
            if self.error.is_none() {
                self.error = Some(JvmBackendError::new(
                    "JVM class constant-pool string exceeds 65535 bytes",
                ));
            }
            return 1;
        };
        self.intern(CpKey::Utf8(value.to_string()), |key| match key {
            CpKey::Utf8(_) => CpEntry::Utf8(encoded),
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
            if let CpEntry::Utf8(bytes) = entry {
                checked_utf8_length(bytes).ok_or_else(|| {
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

fn encode_modified_utf8(value: &str) -> Option<Vec<u8>> {
    let mut encoded = Vec::with_capacity(value.len());
    for scalar in value.chars() {
        let scalar = u32::from(scalar);
        if scalar == 0 {
            extend_modified_utf8(&mut encoded, &[0xc0, 0x80])?;
        } else if scalar <= 0x7f {
            extend_modified_utf8(&mut encoded, &[scalar as u8])?;
        } else if scalar <= 0x7ff {
            extend_modified_utf8(
                &mut encoded,
                &[(0xc0 | (scalar >> 6)) as u8, (0x80 | (scalar & 0x3f)) as u8],
            )?;
        } else if scalar <= 0xffff {
            encode_modified_utf8_code_unit(&mut encoded, scalar as u16)?;
        } else {
            let supplementary = scalar - 0x1_0000;
            let high_surrogate = 0xd800 | (supplementary >> 10) as u16;
            let low_surrogate = 0xdc00 | (supplementary & 0x3ff) as u16;
            encode_modified_utf8_code_unit(&mut encoded, high_surrogate)?;
            encode_modified_utf8_code_unit(&mut encoded, low_surrogate)?;
        }
    }
    Some(encoded)
}

fn encode_modified_utf8_code_unit(encoded: &mut Vec<u8>, code_unit: u16) -> Option<()> {
    extend_modified_utf8(
        encoded,
        &[
            (0xe0 | (code_unit >> 12)) as u8,
            (0x80 | ((code_unit >> 6) & 0x3f)) as u8,
            (0x80 | (code_unit & 0x3f)) as u8,
        ],
    )
}

fn extend_modified_utf8(encoded: &mut Vec<u8>, bytes: &[u8]) -> Option<()> {
    if encoded.len().checked_add(bytes.len())? > usize::from(u16::MAX) {
        return None;
    }
    encoded.extend_from_slice(bytes);
    Some(())
}

fn checked_utf8_length(bytes: &[u8]) -> Option<u16> {
    u16::try_from(bytes.len()).ok()
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
    Utf8(Vec<u8>),
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
            Self::Utf8(bytes) => {
                out.push(1);
                write_u16(
                    out,
                    checked_utf8_length(bytes)
                        .expect("constant-pool UTF-8 lengths are checked before serialization"),
                );
                out.extend_from_slice(bytes);
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
            entries: vec![CpEntry::Utf8(vec![b'x'; usize::from(u16::MAX) + 1])],
            indexes: BTreeMap::new(),
            error: None,
        };
        let mut out = vec![1, 2, 3];

        let failure = pool.write(&mut out);

        assert!(failure.is_err());
        assert_eq!(out, vec![1, 2, 3]);
    }

    #[test]
    fn modified_utf8_encodes_null_and_supplementary_scalars() {
        assert_eq!(encode_modified_utf8("\0"), Some(vec![0xc0, 0x80]));
        assert_eq!(
            encode_modified_utf8("😀"),
            Some(vec![0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80])
        );
    }

    #[test]
    fn modified_utf8_encodes_each_scalar_width() {
        assert_eq!(
            encode_modified_utf8("A\u{80}\u{800}"),
            Some(vec![0x41, 0xc2, 0x80, 0xe0, 0xa0, 0x80])
        );
    }

    #[test]
    fn modified_utf8_accepts_exactly_65535_encoded_bytes() {
        let value = format!("{}a", "\0".repeat(32_767));

        let encoded = encode_modified_utf8(&value).expect("boundary value should be accepted");

        assert_eq!(value.len(), 32_768);
        assert_eq!(encoded.len(), usize::from(u16::MAX));
    }

    #[test]
    fn modified_utf8_rejects_65536_encoded_bytes() {
        let value = "\0".repeat(32_768);

        assert!(encode_modified_utf8(&value).is_none());
        assert_eq!(value.len(), 32_768);
    }

    #[test]
    fn constant_pool_serializes_the_validated_modified_utf8_bytes() {
        let mut pool = ConstantPool::new();
        pool.utf8("a\0😀");
        let mut out = Vec::new();

        pool.write(&mut out)
            .expect("constant pool should serialize");

        assert_eq!(
            out,
            vec![
                0, 2, 1, 0, 9, b'a', 0xc0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80,
            ]
        );
    }

    #[test]
    fn constant_pool_preserves_the_bounded_error_for_oversized_modified_utf8() {
        let mut pool = ConstantPool::new();
        let index = pool.utf8(&"\0".repeat(32_768));
        let mut out = vec![1, 2, 3];

        let failure = pool.write(&mut out);

        assert_eq!(index, 1);
        assert_eq!(
            failure
                .expect_err("oversized value should fail")
                .to_string(),
            "JVM class constant-pool string exceeds 65535 bytes"
        );
        assert_eq!(out, vec![1, 2, 3]);
    }

    #[test]
    fn oversized_modified_utf8_does_not_replace_an_earlier_pool_error() {
        let mut pool = ConstantPool::new();
        pool.entries
            .resize_with(usize::from(u16::MAX) - 1, || CpEntry::Padding);
        pool.integer(7);

        pool.utf8(&"\0".repeat(32_768));

        assert_eq!(
            pool.error.as_ref().map(ToString::to_string).as_deref(),
            Some("JVM class constant pool exceeds the classfile limit")
        );
    }
}
