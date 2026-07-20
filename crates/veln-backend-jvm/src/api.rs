use veln_ir::TypedProgram;

use crate::classfile::ClassfileEmitter;
use crate::java::java_type_identifier;
use crate::model::{EntryArgType, JvmBackendOptions, JvmProgram, SanitizedOptions};

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
