use veln_ir::TypedProgram;

use crate::emit::ProgramEmitter;
use crate::java::java_type_identifier;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaProgram {
    pub sources: Vec<JavaSourceFile>,
}

impl JavaProgram {
    pub fn source(&self, path: &str) -> Option<&str> {
        self.sources
            .iter()
            .find(|source| source.path == path)
            .map(|source| source.contents.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaSourceFile {
    pub path: String,
    pub contents: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaBackendOptions {
    pub program_class: String,
    pub runtime_class: String,
}

impl Default for JavaBackendOptions {
    fn default() -> Self {
        Self {
            program_class: "VelnProgram".to_string(),
            runtime_class: "VelnRuntime".to_string(),
        }
    }
}

pub fn generate_java(program: &TypedProgram) -> JavaProgram {
    generate_java_with_options(program, &JavaBackendOptions::default())
}

pub fn generate_java_with_entry(program: &TypedProgram, entry_function: &str) -> JavaProgram {
    generate_java_with_entry_args_options(
        program,
        entry_function,
        0,
        &JavaBackendOptions::default(),
    )
}

pub fn generate_java_with_entry_args(
    program: &TypedProgram,
    entry_function: &str,
    entry_arg_count: usize,
) -> JavaProgram {
    generate_java_with_entry_args_options(
        program,
        entry_function,
        entry_arg_count,
        &JavaBackendOptions::default(),
    )
}

pub fn generate_java_with_options(
    program: &TypedProgram,
    options: &JavaBackendOptions,
) -> JavaProgram {
    let options = SanitizedOptions {
        program_class: java_type_identifier(&options.program_class),
        runtime_class: java_type_identifier(&options.runtime_class),
    };
    let emitter = ProgramEmitter::new(program, options);
    emitter.emit()
}

pub fn generate_java_with_entry_options(
    program: &TypedProgram,
    entry_function: &str,
    options: &JavaBackendOptions,
) -> JavaProgram {
    generate_java_with_entry_args_options(program, entry_function, 0, options)
}

pub fn generate_java_with_entry_args_options(
    program: &TypedProgram,
    entry_function: &str,
    entry_arg_count: usize,
    options: &JavaBackendOptions,
) -> JavaProgram {
    let options = SanitizedOptions {
        program_class: java_type_identifier(&options.program_class),
        runtime_class: java_type_identifier(&options.runtime_class),
    };
    let emitter = ProgramEmitter::new(program, options);
    emitter.emit_with_entry(entry_function, entry_arg_count)
}

#[derive(Clone, Debug)]
pub(crate) struct SanitizedOptions {
    pub(crate) program_class: String,
    pub(crate) runtime_class: String,
}
