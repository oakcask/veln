//! Typed IR to JVM backend artifacts.

mod api;
mod classfile;
mod emit;
mod java;

pub use api::{
    EntryArgType, JavaBackendOptions, JavaProgram, JavaSourceFile, JvmClassFile, JvmProgram,
    generate_classfiles_with_entry, generate_classfiles_with_entry_arg_types,
    generate_classfiles_with_entry_arg_types_options, generate_java, generate_java_with_entry,
    generate_java_with_entry_arg_types, generate_java_with_entry_arg_types_options,
    generate_java_with_entry_args, generate_java_with_entry_args_options,
    generate_java_with_entry_options, generate_java_with_options,
};

#[cfg(test)]
mod tests;
