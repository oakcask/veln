use std::fs;
use std::path::{Path, PathBuf};

use veln_project::{
    PackageIdentity, PackageSnapshotSource, capture_embedded_package_snapshot, parse_manifest_text,
};

use super::*;

fn generate(manifest: &str, sources: &[(&str, &str)]) -> PackageDocResult {
    let snapshot = capture_embedded_package_snapshot(
        manifest.as_bytes(),
        sources
            .iter()
            .map(|(path, text)| PackageSnapshotSource::new(path, text.as_bytes())),
    )
    .unwrap();
    let manifest = parse_manifest_text("veln.toml", manifest);
    let identity =
        PackageIdentity::new(manifest_field(&manifest.package.fields, "name").unwrap()).unwrap();
    PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-a"),
    )
}

fn generate_fixture(name: &str) -> PackageDocResult {
    let root = example_fixture_root(name);
    let manifest_text = fs::read_to_string(root.join("veln.toml")).unwrap();
    let mut source_texts = Vec::new();
    let mut source_paths = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "veln")
        })
        .collect::<Vec<_>>();
    source_paths.sort();
    for path in source_paths {
        let source_name = path.file_name().unwrap().to_string_lossy().to_string();
        source_texts.push((source_name, fs::read(path).unwrap()));
    }
    let snapshot = capture_embedded_package_snapshot(
        manifest_text.as_bytes(),
        source_texts
            .iter()
            .map(|(path, bytes)| PackageSnapshotSource::new(path, bytes.as_slice())),
    )
    .unwrap();
    let manifest = parse_manifest_text("veln.toml", &manifest_text);
    let identity =
        PackageIdentity::new(manifest_field(&manifest.package.fields, "name").unwrap()).unwrap();
    PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-a"),
    )
}

fn example_fixture_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/specification/doc")
        .join(name)
}

fn catalog_or_panic(result: &PackageDocResult) -> &PackageDocCatalog {
    result
        .catalog()
        .unwrap_or_else(|| panic!("successful catalog: {:?}", result.status().diagnostics))
}

fn generate_with_forced_declaration_id(
    manifest: &str,
    sources: &[(&str, &str)],
    id: &str,
) -> PackageDocResult {
    let snapshot = capture_embedded_package_snapshot(
        manifest.as_bytes(),
        sources
            .iter()
            .map(|(path, text)| PackageSnapshotSource::new(path, text.as_bytes())),
    )
    .unwrap();
    let manifest = parse_manifest_text("veln.toml", manifest);
    let identity =
        manifest_field(&manifest.package.fields, "name").unwrap_or_else(|| "demo".to_string());
    PackageDocBuilder::new(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-a"),
    )
    .with_forced_declaration_id(id)
    .generate()
}

mod catalog_identity;
mod doctest_gates;
mod doctest_publication;
mod navigation_and_manifest;
