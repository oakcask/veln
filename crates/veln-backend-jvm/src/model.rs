#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JvmProgram {
    pub classes: Vec<JvmClassFile>,
}

impl JvmProgram {
    pub fn class(&self, path: &str) -> Option<&[u8]> {
        self.classes
            .iter()
            .find(|class| class.path == path)
            .map(|class| class.contents.as_slice())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JvmClassFile {
    pub path: String,
    pub contents: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JvmBackendOptions {
    pub program_class: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryArgType {
    String,
    Int,
    Float,
    Bool,
    VariadicList {
        element: EntryArgScalar,
        count: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryArgScalar {
    String,
    Int,
    Float,
    Bool,
}

impl Default for JvmBackendOptions {
    fn default() -> Self {
        Self {
            program_class: "VelnProgram".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SanitizedOptions {
    pub(crate) program_class: String,
    pub(crate) runtime_class: String,
}
