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
    let owner = T::named("IoOwner");
    let resource = T::named("IoResource");
    let filesystem = T::named("FsFilesystem");
    let mutable_filesystem = T::named("FsMutableFilesystem");
    let reader = T::named("IoReader");
    let writer = T::named("IoWriter");
    let seeker = T::named("IoSeeker");
    let reader_at = T::named("IoReaderAt");
    let writer_at = T::named("IoWriterAt");
    let closer = T::named("IoCloser");
    let syncer = T::named("IoSyncer");
    let truncater = T::named("FsTruncater");
    let error = T::source("Error", "io");
    let result = |value| T::result(value, error.clone());
    let slot_result = || result(T::int());

    match name {
        "io_provider_scope_owner" => Some((vec![], owner)),
        "fs_system_open" => Some((vec![T::path()], result(mutable_filesystem))),
        "fs_read_only" => Some((vec![mutable_filesystem], filesystem)),
        "io_read_exact" => Some((
            vec![reader, T::named("ByteCount")],
            T::source("ExactOutcome", "io"),
        )),
        "io_write_all" => Some((
            vec![writer, T::named("ByteChunk")],
            T::source("WriteOutcome", "io"),
        )),
        "io_copy" => Some((
            vec![reader, writer, T::named("ByteCount")],
            T::source("CopyOutcome", "io"),
        )),
        "io_provider_filesystem" => Some((vec![owner, T::int()], result(filesystem))),
        "io_provider_mutable_filesystem" => {
            Some((vec![owner, T::int()], result(mutable_filesystem)))
        }
        "io_provider_resource" => Some((
            vec![owner, T::int(), T::source("ViewFamily", "io_provider")],
            result(resource),
        )),
        "io_provider_read_file" => Some((vec![resource], result(T::source("ReadFile", "fs")))),
        "io_provider_write_file" => Some((vec![resource], result(T::source("WriteFile", "fs")))),
        "io_provider_update_file" => Some((vec![resource], result(T::source("UpdateFile", "fs")))),
        "io_provider_append_file" => Some((vec![resource], result(T::source("AppendFile", "fs")))),
        "io_provider_duplex" => Some((
            vec![resource],
            result(T::record(vec![
                ("reader", reader),
                ("writer", writer),
                ("closer", closer),
            ])),
        )),
        "io_provider_slot" => Some((vec![owner, resource], slot_result())),
        "io_provider_filesystem_slot" => Some((vec![owner, filesystem], slot_result())),
        "io_provider_mutable_filesystem_slot" => {
            Some((vec![owner, mutable_filesystem], slot_result()))
        }
        "io_provider_reader_slot" => Some((vec![owner, reader], slot_result())),
        "io_provider_writer_slot" => Some((vec![owner, writer], slot_result())),
        "io_provider_seeker_slot" => Some((vec![owner, seeker], slot_result())),
        "io_provider_reader_at_slot" => Some((vec![owner, reader_at], slot_result())),
        "io_provider_writer_at_slot" => Some((vec![owner, writer_at], slot_result())),
        "io_provider_closer_slot" => Some((vec![owner, closer], slot_result())),
        "io_provider_syncer_slot" => Some((vec![owner, syncer], slot_result())),
        "io_provider_truncater_slot" => Some((vec![owner, truncater], slot_result())),
        _ => None,
    }
}
