use super::*;

#[derive(Default)]
pub(super) struct ConstantPool {
    entries: Vec<CpEntry>,
    indexes: BTreeMap<CpKey, u16>,
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
        let index = (self.entries.len() + 1) as u16;
        let entry = build(&key);
        self.entries.push(entry);
        if matches!(key, CpKey::Long(_) | CpKey::Double(_)) {
            self.entries.push(CpEntry::Padding);
        }
        self.indexes.insert(key, index);
        index
    }

    pub(super) fn write(&self, out: &mut Vec<u8>) {
        write_u16(out, (self.entries.len() + 1) as u16);
        for entry in &self.entries {
            match entry {
                CpEntry::Utf8(value) => {
                    out.push(1);
                    write_u16(out, value.len() as u16);
                    out.extend_from_slice(value.as_bytes());
                }
                CpEntry::Class(index) => {
                    out.push(7);
                    write_u16(out, *index);
                }
                CpEntry::String(index) => {
                    out.push(8);
                    write_u16(out, *index);
                }
                CpEntry::Fieldref {
                    class_index,
                    name_and_type,
                } => {
                    out.push(9);
                    write_u16(out, *class_index);
                    write_u16(out, *name_and_type);
                }
                CpEntry::Methodref {
                    class_index,
                    name_and_type,
                } => {
                    out.push(10);
                    write_u16(out, *class_index);
                    write_u16(out, *name_and_type);
                }
                CpEntry::NameAndType {
                    name_index,
                    descriptor_index,
                } => {
                    out.push(12);
                    write_u16(out, *name_index);
                    write_u16(out, *descriptor_index);
                }
                CpEntry::Long(value) => {
                    out.push(5);
                    out.extend_from_slice(&value.to_be_bytes());
                }
                CpEntry::Double(bits) => {
                    out.push(6);
                    out.extend_from_slice(&bits.to_be_bytes());
                }
                CpEntry::Padding => {}
            }
        }
    }
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
    Double(u64),
    Padding,
}

pub(super) fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(super) fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}
