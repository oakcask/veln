use veln_ir::TypedProgram;

use crate::classfile::ClassfileEmitter;
use crate::java::java_type_identifier;

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

pub fn generate_classfiles_with_entry(program: &TypedProgram, entry_function: &str) -> JvmProgram {
    generate_classfiles_with_entry_arg_types(program, entry_function, &[])
}

pub fn generate_classfiles_with_entry_arg_types(
    program: &TypedProgram,
    entry_function: &str,
    entry_arg_types: &[EntryArgType],
) -> JvmProgram {
    generate_classfiles_with_entry_arg_types_options(
        program,
        entry_function,
        entry_arg_types,
        &JvmBackendOptions::default(),
    )
}

pub fn generate_classfiles_with_entry_arg_types_options(
    program: &TypedProgram,
    entry_function: &str,
    entry_arg_types: &[EntryArgType],
    options: &JvmBackendOptions,
) -> JvmProgram {
    let options = SanitizedOptions {
        program_class: java_type_identifier(&options.program_class),
        runtime_class: "VelnRuntime".to_string(),
    };
    ClassfileEmitter::new(program, options).emit(entry_function, entry_arg_types)
}

#[derive(Clone, Debug)]
pub(crate) struct SanitizedOptions {
    pub(crate) program_class: String,
    pub(crate) runtime_class: String,
}
