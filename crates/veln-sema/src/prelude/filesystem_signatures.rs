use veln_core::CoreType;

use crate::adt::type_operations as adt;
use crate::semantic_model::Type;

trait FilesystemPreludeType: Clone {
    fn named(name: &'static str) -> Self;
    fn source(name: &'static str, module: &'static str) -> Self;
    fn int() -> Self;
    fn path() -> Self;
    fn record(fields: Vec<(&'static str, Self)>) -> Self;
    fn result(value: Self, error: Self) -> Self;
}

impl FilesystemPreludeType for Type {
    fn named(name: &'static str) -> Self {
        Type::named(name, Vec::new())
    }

    fn source(name: &'static str, module: &'static str) -> Self {
        Type::resolved_named(name, format!("std::{module}::{name}"), Vec::new())
    }

    fn int() -> Self {
        Type::int()
    }

    fn path() -> Self {
        Type::path()
    }

    fn record(fields: Vec<(&'static str, Self)>) -> Self {
        Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name.to_string(), ty))
                .collect(),
        )
    }

    fn result(value: Self, error: Self) -> Self {
        adt::result_type(value, error)
    }
}

impl FilesystemPreludeType for CoreType {
    fn named(name: &'static str) -> Self {
        CoreType::named(name, Vec::new())
    }

    fn source(name: &'static str, _module: &'static str) -> Self {
        CoreType::named(name, Vec::new())
    }

    fn int() -> Self {
        CoreType::int()
    }

    fn path() -> Self {
        CoreType::named("Path", Vec::new())
    }

    fn record(fields: Vec<(&'static str, Self)>) -> Self {
        CoreType::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name.to_string(), ty))
                .collect(),
        )
    }

    fn result(value: Self, error: Self) -> Self {
        adt::core_result_type(value, error)
    }
}

pub(super) fn prelude_filesystem_io_signature(name: &str) -> Option<(Vec<Type>, Type)> {
    filesystem_io_signature(name)
}

pub(super) fn core_filesystem_io_signature(name: &str) -> Option<(Vec<CoreType>, CoreType)> {
    filesystem_io_signature(name)
}

fn filesystem_io_signature<T: FilesystemPreludeType>(name: &str) -> Option<(Vec<T>, T)> {
    filesystem_source_signature(name)
        .or_else(|| provider_factory_signature(name))
        .or_else(|| provider_resource_view_signature(name))
        .or_else(|| provider_slot_signature(name))
}

fn filesystem_source_signature<T: FilesystemPreludeType>(name: &str) -> Option<(Vec<T>, T)> {
    match name {
        "io_provider_scope_owner" => Some((vec![], T::named("IoOwner"))),
        "fs_system_open" => Some((vec![T::path()], io_result(T::named("FsMutableFilesystem")))),
        "fs_read_only" => Some((
            vec![T::named("FsMutableFilesystem")],
            T::named("FsFilesystem"),
        )),
        "io_read_exact" => Some((
            vec![T::named("IoReader"), T::named("ByteCount")],
            T::source("ExactOutcome", "io"),
        )),
        "io_write_all" => Some((
            vec![T::named("IoWriter"), T::named("ByteChunk")],
            T::source("WriteOutcome", "io"),
        )),
        "io_copy" => Some((
            vec![
                T::named("IoReader"),
                T::named("IoWriter"),
                T::named("ByteCount"),
            ],
            T::source("CopyOutcome", "io"),
        )),
        _ => None,
    }
}

fn provider_factory_signature<T: FilesystemPreludeType>(name: &str) -> Option<(Vec<T>, T)> {
    match name {
        "io_provider_filesystem" => Some((
            vec![T::named("IoOwner"), T::int()],
            io_result(T::named("FsFilesystem")),
        )),
        "io_provider_mutable_filesystem" => Some((
            vec![T::named("IoOwner"), T::int()],
            io_result(T::named("FsMutableFilesystem")),
        )),
        "io_provider_resource" => Some((
            vec![
                T::named("IoOwner"),
                T::int(),
                T::source("ViewFamily", "io_provider"),
            ],
            io_result(T::named("IoResource")),
        )),
        _ => None,
    }
}

fn provider_resource_view_signature<T: FilesystemPreludeType>(name: &str) -> Option<(Vec<T>, T)> {
    match name {
        "io_provider_read_file" => Some((
            vec![T::named("IoResource")],
            io_result(T::source("ReadFile", "fs")),
        )),
        "io_provider_write_file" => Some((
            vec![T::named("IoResource")],
            io_result(T::source("WriteFile", "fs")),
        )),
        "io_provider_update_file" => Some((
            vec![T::named("IoResource")],
            io_result(T::source("UpdateFile", "fs")),
        )),
        "io_provider_append_file" => Some((
            vec![T::named("IoResource")],
            io_result(T::source("AppendFile", "fs")),
        )),
        "io_provider_duplex" => Some((
            vec![T::named("IoResource")],
            io_result(T::record(vec![
                ("reader", T::named("IoReader")),
                ("writer", T::named("IoWriter")),
                ("closer", T::named("IoCloser")),
            ])),
        )),
        _ => None,
    }
}

fn provider_slot_signature<T: FilesystemPreludeType>(name: &str) -> Option<(Vec<T>, T)> {
    let capability = provider_slot_capability(name)?;
    Some((vec![T::named("IoOwner"), capability], io_result(T::int())))
}

fn provider_slot_capability<T: FilesystemPreludeType>(name: &str) -> Option<T> {
    match name {
        "io_provider_slot" => Some(T::named("IoResource")),
        "io_provider_filesystem_slot" => Some(T::named("FsFilesystem")),
        "io_provider_mutable_filesystem_slot" => Some(T::named("FsMutableFilesystem")),
        "io_provider_reader_slot" => Some(T::named("IoReader")),
        "io_provider_writer_slot" => Some(T::named("IoWriter")),
        "io_provider_seeker_slot" => Some(T::named("IoSeeker")),
        "io_provider_reader_at_slot" => Some(T::named("IoReaderAt")),
        "io_provider_writer_at_slot" => Some(T::named("IoWriterAt")),
        "io_provider_closer_slot" => Some(T::named("IoCloser")),
        "io_provider_syncer_slot" => Some(T::named("IoSyncer")),
        "io_provider_truncater_slot" => Some(T::named("FsTruncater")),
        _ => None,
    }
}

fn io_result<T: FilesystemPreludeType>(value: T) -> T {
    T::result(value, T::source("Error", "io"))
}
