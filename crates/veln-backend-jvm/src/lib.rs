//! Typed IR to Java source and JVM execution support.

mod api;
mod emit;
mod java;

pub use api::{
    JavaBackendOptions, JavaProgram, JavaSourceFile, generate_java, generate_java_with_entry,
    generate_java_with_entry_args, generate_java_with_entry_args_options,
    generate_java_with_entry_options, generate_java_with_options,
};

#[cfg(test)]
mod tests;
