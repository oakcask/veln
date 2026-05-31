//! Typed IR to JVM backend artifacts.
//!
//! The public facade exposes the direct bytecode backend API:
//!
//! ```
//! use veln_backend_jvm::{
//!     EntryArgType, JvmBackendOptions, generate_classfiles_with_entry_arg_types_options,
//! };
//!
//! let options = JvmBackendOptions::default();
//! let entry_arg_types = [EntryArgType::String];
//! let _ = (options, entry_arg_types, generate_classfiles_with_entry_arg_types_options);
//! ```
//!
//! The old Java source backend API is no longer exported by this crate:
//!
//! ```compile_fail
//! use veln_backend_jvm::{JavaBackendOptions, generate_java};
//! ```

mod api;
mod classfile;
mod java;
mod runtime;

pub use api::{
    EntryArgType, JvmBackendOptions, JvmClassFile, JvmProgram, generate_classfiles_with_entry,
    generate_classfiles_with_entry_arg_types, generate_classfiles_with_entry_arg_types_options,
};

#[cfg(test)]
mod tests;
