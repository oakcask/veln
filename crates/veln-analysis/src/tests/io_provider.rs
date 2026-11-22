use super::*;

fn provider_project(source: &str) -> Project {
    Project {
        root: PathBuf::new(),
        manifest: None,
        files: vec![SourceFile::new("main.veln", source)],
    }
}

#[test]
fn io_provider_file_records_expose_exact_static_capabilities() {
    // Compiler contract: the standard adapter's named structural records expose
    // their proposed fields and preserve opaque view identities.
    let analysis = analyze_project(
        provider_project(
            r#"
use io from "std"
use fs from "std"
use io_provider as providers from "std"

pub fn read(file: fs::ReadFile, maximum: ByteCount) -> io::ReadOutcome effects [io::Read]
    io::read(file.reader, maximum)
end

pub fn write(file: fs::WriteFile, bytes: ByteChunk) -> io::WriteOutcome effects [io::Write]
    io::write(file.writer, bytes)
end

pub fn seek(file: fs::UpdateFile, origin: io::SeekOrigin) -> Result<ByteOffset, io::Error> effects [io::Seek]
    io::seek(file.seeker, origin)
end

pub fn close(file: fs::AppendFile) -> io::CloseOutcome effects [io::Close]
    io::close(file.closer)
end

pub fn native(text: String) -> Result<Path, io::Error>
    fs::native_path(text)
end

pub fn select(root: Path) -> Result<fs::MutableFilesystem, io::Error> effects [fs::Host, providers::Scope]
    fs::system(root)
end
"#,
        ),
        DoctestMode::Exclude,
    );
    assert!(
        analysis.checked_diagnostics().is_empty(),
        "{:?}",
        analysis.checked_diagnostics()
    );
}

#[test]
fn io_provider_type_boundary_rejects_capability_and_owner_upgrades() {
    for source in [
        "use io from \"std\"\npub fn bad(reader: io::Reader, bytes: ByteChunk) -> io::WriteOutcome effects [io::Write]\n  io::write(reader, bytes)\nend\n",
        "use io from \"std\"\nuse net from \"std\"\npub fn bad(stream: net::Stream, origin: io::SeekOrigin) -> Result<ByteOffset, io::Error> effects [io::Seek]\n  io::seek(stream, origin)\nend\n",
        "use io from \"std\"\nuse fs from \"std\"\nuse path as portable from \"std\"\npub fn bad(filesystem: fs::Filesystem, name: portable::Path) -> fs::MutationOutcome effects [fs::Mutate]\n  fs::remove_file(filesystem, name)\nend\n",
        "use io from \"std\"\nuse io_provider from \"std\"\npub fn bad() -> Result<io_provider::Resource, io::Error> effects [concurrency]\n  io_provider::resource(0, 0, io_provider::ViewFamily::Duplex)\nend\n",
        "use fs from \"std\"\nuse path as portable from \"std\"\npub fn bad(path: portable::Path) -> String\n  fs::display_native(path)\nend\n",
        "use io from \"std\"\nuse fs from \"std\"\nuse io_provider from \"std\"\nuse path as portable from \"std\"\npub fn bad(root: portable::Path) -> Result<fs::MutableFilesystem, io::Error> effects [fs::Host, io_provider::Scope]\n  fs::system(root)\nend\n",
    ] {
        let analysis = analyze_project(provider_project(source), DoctestMode::Exclude);
        let diagnostics = analysis.checked_diagnostics();
        assert_eq!(diagnostics.len(), 1, "{source}\n{diagnostics:?}");
        assert_eq!(
            diagnostics[0].id, "type.mismatch",
            "{source}\n{diagnostics:?}"
        );
    }
}

#[test]
fn io_provider_error_paths_keep_standard_identity_in_application_handlers() {
    let analysis = analyze_project(
        provider_project(
            r#"
use io from "std"
use path as portable from "std"

fn failed(path: portable::Path) -> Result<ByteOffset, io::Error>
    Err({operation: "seek", kind: io::ErrorKind::InvalidInput,
        path: Some(path), other_path: Some(path), network: None,
        commit: None, message: "invalid target"})
end

handler seeking(path: portable::Path) for io::Seek
    seek(resource, origin) => failed(path)
end
"#,
        ),
        DoctestMode::Exclude,
    );
    assert!(
        analysis.checked_diagnostics().is_empty(),
        "{:#?}",
        analysis.checked_diagnostics()
    );
}

#[test]
fn io_provider_import_alias_links_inferred_and_explicit_storage_types() {
    for argument in ["2", "false"] {
        let source = format!(
            r#"
use io_provider as cells from "std"

fn add(value: Int, amount: Int) -> {{state: Int, result: Int}}
    {{state: value + amount, result: value}}
end

pub fn main() -> Int effects [concurrency]
    let cell: cells::State<Int> = cells::state<Int>(40)
    let previous = cells::update(cell, {argument}, add)
    cells::update<Int, Int, Int>(cell, 0, add)
end
"#
        );
        let analysis = analyze_project(provider_project(&source), DoctestMode::Exclude);
        let diagnostics = analysis.checked_diagnostics();
        if argument == "2" {
            assert!(diagnostics.is_empty(), "{diagnostics:#?}");
            assert!(
                analysis
                    .lower_reachable_entry("main", FunctionKind::Function)
                    .lowered
                    .diagnostics
                    .is_empty()
            );
        } else {
            assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
            assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
        }
    }
}

#[test]
fn io_provider_system_scope_retains_callback_effects() {
    for effects in ["fs, concurrency, stdio", "fs, concurrency"] {
        let source = format!(
            r#"
use fs from "std"
use io from "std"
fn observe(filesystem: fs::MutableFilesystem) -> Result<(), io::Error> effects [stdio]
    stdio::println("selected")
    Ok(())
end
pub fn select(root: Path) -> Result<(), io::Error> effects [{effects}]
    fs::with_system(root, observe)
end
"#
        );
        let analysis = analyze_project(provider_project(&source), DoctestMode::Exclude);
        let diagnostics = analysis.checked_diagnostics();
        if effects.contains("stdio") {
            assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        } else {
            assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
            assert_eq!(
                diagnostics[0].id, "effect.missing_public",
                "{diagnostics:#?}"
            );
            assert!(diagnostics[0].message.contains("stdio"), "{diagnostics:#?}");
        }
    }
}

#[test]
fn io_provider_explicit_concurrency_types_resolve_source_imports() {
    let analysis = analyze_project(
        provider_project(
            r#"
use fs from "std"
use io_provider as providers from "std"

fn worker(context: {cell: providers::State<Int>, filesystem: fs::Filesystem}) -> () effects [concurrency]
    ()
end

pub fn bridge(filesystem: fs::Filesystem, file: fs::ReadFile) -> () effects [concurrency]
    let files = channel::bounded<fs::ReadFile>(1)
    let _ = channel::send(files.tx, file)
    let views = channel::bounded<fs::Filesystem>(1)
    let _ = channel::send(views.tx, filesystem)
    let cell = providers::state(0)
    let job = task::spawn_with<(), {cell: providers::State<Int>, filesystem: fs::Filesystem}>(worker, {cell: cell, filesystem: filesystem})
    let _ = task::join(job)
    ()
end
"#,
        ),
        DoctestMode::Exclude,
    );
    assert!(
        analysis.checked_diagnostics().is_empty(),
        "{:#?}",
        analysis.checked_diagnostics()
    );
}

#[test]
fn io_provider_network_adapter_exposes_only_byte_views_and_retains_callback_effects() {
    let source = r#"
use io from "std"
use net from "std"
use io_provider as providers from "std"
fn body(views: {reader: io::Reader, writer: io::Writer, closer: io::Closer}) -> Result<(), io::Error> effects [io::Read, stdio]
    let _ = io::read(views.reader, byte_chunk_count(byte_chunk([])))
    stdio::println("selected byte reader")
    Ok(())
end
pub fn adapt(stream: net::Stream) -> Result<{reader: io::Reader, writer: io::Writer, closer: io::Closer}, io::Error> effects [providers::Scope, net::IO, concurrency]
    providers::network(stream)
end
pub fn use_adapter(stream: net::Stream) -> Result<(), io::Error> effects [net::IO, concurrency, stdio]
    providers::with_network(stream, body)
end
"#;
    let analysis = analyze_project(provider_project(source), DoctestMode::Exclude);
    assert!(
        analysis.checked_diagnostics().is_empty(),
        "{:?}",
        analysis.checked_diagnostics()
    );
    let missing_effect = source.replace(
        "effects [net::IO, concurrency, stdio]",
        "effects [net::IO, concurrency]",
    );
    let analysis = analyze_project(provider_project(&missing_effect), DoctestMode::Exclude);
    let diagnostics = analysis.checked_diagnostics();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    for bad in [
        "use io from \"std\"\nuse net from \"std\"\nuse io_provider as providers from \"std\"\npub fn bad() -> Result<{reader: io::Reader, writer: io::Writer, closer: io::Closer}, io::Error> effects [providers::Scope, net::IO, concurrency]\n  providers::network(1)\nend\n",
        "use io from \"std\"\npub fn bad(views: {reader: io::Reader, writer: io::Writer, closer: io::Closer}, origin: io::SeekOrigin) -> Result<ByteOffset, io::Error> effects [io::Seek]\n  io::seek(views.reader, origin)\nend\n",
    ] {
        let analysis = analyze_project(provider_project(bad), DoctestMode::Exclude);
        let diagnostics = analysis.checked_diagnostics();
        assert_eq!(diagnostics.len(), 1, "{bad}\n{diagnostics:?}");
        assert_eq!(diagnostics[0].id, "type.mismatch", "{bad}\n{diagnostics:?}");
    }
}
