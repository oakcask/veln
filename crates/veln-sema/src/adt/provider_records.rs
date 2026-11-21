use super::descriptors::AdtDescriptor;
use crate::semantic_model::Type;

// Structural records at the explicitly compiler-known provider boundary.
// As with WallTime, only declarations in the selected standard module receive
// this representation; an application namesake remains an ordinary source ADT.
pub(super) fn provider_record_type(descriptor: &AdtDescriptor) -> Option<Type> {
    if descriptor.package_name.as_deref() != Some("std") {
        return None;
    }
    let nominal = |module: &str, name: &str| {
        Type::resolved_named(name, format!("std::{module}::{name}"), Vec::new())
    };
    let view = |name| nominal("io", name);
    let record = |fields: Vec<(&str, Type)>| {
        Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name.to_string(), ty))
                .collect(),
        )
    };
    match (
        descriptor.module_name.as_deref()?,
        descriptor.type_name.as_str(),
    ) {
        ("std::io", "Error") => Some(record(vec![
            ("operation", Type::string()),
            ("kind", nominal("io", "ErrorKind")),
            ("path", Type::option(nominal("path", "Path"))),
            ("other_path", Type::option(nominal("path", "Path"))),
            ("network", Type::option(nominal("net", "NetError"))),
            ("commit", Type::option(nominal("io", "Commit"))),
            ("message", Type::string()),
        ])),
        ("std::fs", "ReadFile") => Some(record(vec![
            ("reader", view("Reader")),
            ("seeker", view("Seeker")),
            ("reader_at", view("ReaderAt")),
            ("closer", view("Closer")),
        ])),
        ("std::fs", "WriteFile") => Some(record(vec![
            ("writer", view("Writer")),
            ("seeker", view("Seeker")),
            ("writer_at", view("WriterAt")),
            ("closer", view("Closer")),
            ("syncer", view("Syncer")),
            ("truncater", nominal("fs", "Truncater")),
        ])),
        ("std::fs", "UpdateFile") => Some(record(vec![
            ("reader", view("Reader")),
            ("writer", view("Writer")),
            ("seeker", view("Seeker")),
            ("reader_at", view("ReaderAt")),
            ("writer_at", view("WriterAt")),
            ("closer", view("Closer")),
            ("syncer", view("Syncer")),
            ("truncater", nominal("fs", "Truncater")),
        ])),
        ("std::fs", "AppendFile") => Some(record(vec![
            ("writer", view("Writer")),
            ("closer", view("Closer")),
            ("syncer", view("Syncer")),
        ])),
        ("std::fs", "WriteMode" | "UpdateMode") => Some(record(vec![
            ("create", nominal("fs", "CreateMode")),
            ("truncate", Type::bool()),
        ])),
        ("std::fs", "Metadata") => Some(record(vec![
            ("kind", nominal("fs", "Kind")),
            ("length", Type::option(nominal("bytes", "ByteCount"))),
        ])),
        ("std::fs", "Entry") => Some(record(vec![
            ("name", nominal("path", "Path")),
            ("kind", nominal("fs", "Kind")),
        ])),
        _ => None,
    }
}
