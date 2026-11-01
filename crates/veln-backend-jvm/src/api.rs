use veln_ir::TypedProgram;

use crate::classfile::ClassfileEmitter;
use crate::java::java_type_identifier;
use crate::model::{
    EntryArgType, JvmBackendError, JvmBackendOptions, JvmProgram, SanitizedOptions,
};

pub fn generate_classfiles_with_entry(program: &TypedProgram, entry_function: &str) -> JvmProgram {
    try_generate_classfiles_with_entry(program, entry_function)
        .expect("typed programs must fit JVM classfile limits")
}

pub fn try_generate_classfiles_with_entry(
    program: &TypedProgram,
    entry_function: &str,
) -> Result<JvmProgram, JvmBackendError> {
    try_generate_classfiles_with_entry_arg_types(program, entry_function, &[])
}

pub fn generate_classfiles_with_test_entries(
    program: &TypedProgram,
    entry_functions: &[String],
) -> JvmProgram {
    try_generate_classfiles_with_test_entries(program, entry_functions)
        .expect("typed programs must fit JVM classfile limits")
}

pub fn try_generate_classfiles_with_test_entries(
    program: &TypedProgram,
    entry_functions: &[String],
) -> Result<JvmProgram, JvmBackendError> {
    let options = SanitizedOptions {
        program_class: java_type_identifier(&JvmBackendOptions::default().program_class),
        runtime_class: "VelnRuntime".to_string(),
    };
    ClassfileEmitter::new(program, options).emit_test_entries(entry_functions)
}

pub fn generate_classfiles_with_entry_arg_types(
    program: &TypedProgram,
    entry_function: &str,
    entry_arg_types: &[EntryArgType],
) -> JvmProgram {
    try_generate_classfiles_with_entry_arg_types(program, entry_function, entry_arg_types)
        .expect("typed programs must fit JVM classfile limits")
}

pub fn try_generate_classfiles_with_entry_arg_types(
    program: &TypedProgram,
    entry_function: &str,
    entry_arg_types: &[EntryArgType],
) -> Result<JvmProgram, JvmBackendError> {
    try_generate_classfiles_with_entry_arg_types_options(
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
    try_generate_classfiles_with_entry_arg_types_options(
        program,
        entry_function,
        entry_arg_types,
        options,
    )
    .expect("typed programs must fit JVM classfile limits")
}

pub fn try_generate_classfiles_with_entry_arg_types_options(
    program: &TypedProgram,
    entry_function: &str,
    entry_arg_types: &[EntryArgType],
    options: &JvmBackendOptions,
) -> Result<JvmProgram, JvmBackendError> {
    let options = SanitizedOptions {
        program_class: java_type_identifier(&options.program_class),
        runtime_class: "VelnRuntime".to_string(),
    };
    ClassfileEmitter::new(program, options).emit(entry_function, entry_arg_types)
}
