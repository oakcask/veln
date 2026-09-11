use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use veln_language_service::{
    PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE, PackageDocGeneratorContract, PackageDocResult,
    render_package_documentation,
};
use veln_project::{
    PackageIdentity, PackageSnapshotSource, capture_embedded_package_snapshot, parse_manifest_text,
};

pub const BUNDLE_SCHEMA_VERSION: u64 = 2;
pub const GENERATOR_CONTRACT: &str = "veln-mcp-package-documentation/v1";
pub const DIGEST_DOMAIN: &[u8] = b"veln-mcp-standard-library-package-doc-resources/v2\0";
pub const CHECKED_ARTIFACT: &str =
    include_str!("../generated/mcp-standard-library-package-doc-resources-v2.json");
pub const CHECKED_DIGEST: &str =
    include_str!("../generated/mcp-standard-library-package-doc-resources-v2.sha256");

const PACKAGE_IDENTITY: &str = "std";
const SNAPSHOT_MANIFEST_PATH: &str = "veln.toml";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleMetadata {
    pub schema_version: u64,
    pub generator_contract: String,
    pub package_identity: String,
    pub snapshot_digest: String,
    pub documentation_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedResource {
    pub uri: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub mime_type: String,
    pub text: String,
    pub listed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedBundle {
    pub metadata: BundleMetadata,
    pub resources: Vec<CheckedResource>,
    pub search_candidates: Vec<CheckedSearchCandidate>,
    pub declaration_locations: Vec<CheckedDeclarationLocation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedSearchCandidate {
    pub uri: String,
    pub identifier: String,
    pub title: String,
    pub name: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub signature: Option<String>,
    pub documentation: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedDeclarationLocation {
    pub source_uri: String,
    pub line: usize,
    pub column: usize,
    pub offset: usize,
    pub declaration_uri: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedBundle {
    pub bytes: String,
    pub digest: String,
    pub bundle: CheckedBundle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshnessMismatch {
    pub artifact_matches: bool,
    pub digest_matches: bool,
    pub generated_digest: String,
    pub checked_digest: String,
}

pub fn checked_artifact_bytes() -> &'static str {
    CHECKED_ARTIFACT
}

pub fn checked_artifact_digest() -> &'static str {
    CHECKED_DIGEST.trim()
}

pub fn checked_bundle() -> Result<CheckedBundle, String> {
    verify_checked_digest()?;
    parse_bundle(CHECKED_ARTIFACT)
}

pub fn checked_bundle_for_snapshot(snapshot_digest: &str) -> Result<CheckedBundle, String> {
    let bundle = checked_bundle()?;
    if bundle.metadata.snapshot_digest != snapshot_digest {
        return Err(format!(
            "regenerate the checked MCP standard-library package-documentation resources for snapshot `{snapshot_digest}`; checked snapshot is `{}`",
            bundle.metadata.snapshot_digest
        ));
    }
    Ok(bundle)
}

pub fn verify_checked_artifact() -> Result<(), String> {
    checked_bundle().map(|_| ())
}

fn verify_checked_digest() -> Result<(), String> {
    let expected = bundle_digest(CHECKED_ARTIFACT.as_bytes());
    if expected != checked_artifact_digest() {
        return Err(format!(
            "regenerate the checked MCP standard-library package-documentation resources; checked digest is {}, generated digest is {}",
            checked_artifact_digest(),
            expected
        ));
    }
    Ok(())
}

pub fn generate_checked_bundle() -> Result<GeneratedBundle, String> {
    let standard_library = veln_stdlib::package_bundle();
    let snapshot = capture_embedded_package_snapshot(
        standard_library.manifest.as_bytes(),
        standard_library
            .files
            .iter()
            .map(|file| PackageSnapshotSource::new(file.path, file.text.as_bytes())),
    )
    .map_err(|error| format!("capture the embedded standard-library package snapshot: {error}"))?;
    let manifest = parse_manifest_text(SNAPSHOT_MANIFEST_PATH, standard_library.manifest);
    let result = PackageDocResult::generate(
        &PackageIdentity::embedded_standard(),
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new(GENERATOR_CONTRACT),
    );
    let mut resources = render_package_documentation(&result);
    resources.sort_by(|left, right| left.uri.as_bytes().cmp(right.uri.as_bytes()));
    let search_candidates = result.search_candidates();
    let declaration_locations = result.declaration_locations().collect::<Vec<_>>();
    let value = json!({
        "declaration_locations": declaration_locations.into_iter().map(|location| json!({
            "column": location.column,
            "declaration_uri": location.declaration_uri,
            "line": location.line,
            "offset": location.offset,
            "source_uri": location.source_uri,
        })).collect::<Vec<_>>(),
        "documentation_digest": result.doc_digest(),
        "generator_contract": GENERATOR_CONTRACT,
        "package_identity": result.identity(),
        "resources": resources.into_iter().map(|resource| json!({
            "description": resource.description,
            "listed": resource.listed,
            "mime_type": resource.mime_type,
            "name": resource.name,
            "text": resource.text,
            "title": resource.title,
            "uri": resource.uri,
        })).collect::<Vec<_>>(),
        "search_candidates": search_candidates.into_iter().map(|candidate| json!({
            "documentation": candidate.documentation,
            "identifier": candidate.identifier,
            "keywords": candidate.keywords,
            "name": candidate.name,
            "signature": candidate.signature,
            "summary": candidate.summary,
            "title": candidate.title,
            "uri": candidate.uri,
        })).collect::<Vec<_>>(),
        "schema_version": BUNDLE_SCHEMA_VERSION,
        "snapshot_digest": result.snapshot_digest(),
    });
    let bytes = canonical_json(&value)?;
    let bundle = parse_bundle(&bytes)?;
    let digest = bundle_digest(bytes.as_bytes());
    Ok(GeneratedBundle {
        bytes,
        digest,
        bundle,
    })
}

pub fn verify_freshness() -> Result<(), FreshnessMismatch> {
    verify_freshness_against(CHECKED_ARTIFACT, checked_artifact_digest())
}

pub fn verify_freshness_against(
    checked_artifact: &str,
    checked_digest: &str,
) -> Result<(), FreshnessMismatch> {
    let generated = generate_checked_bundle().map_err(|message| FreshnessMismatch {
        artifact_matches: false,
        digest_matches: false,
        generated_digest: message,
        checked_digest: checked_digest.to_string(),
    })?;
    verify_generated_against(&generated, checked_artifact, checked_digest)
}

fn verify_generated_against(
    generated: &GeneratedBundle,
    checked_artifact: &str,
    checked_digest: &str,
) -> Result<(), FreshnessMismatch> {
    let artifact_matches = generated.bytes == checked_artifact;
    let digest_matches = generated.digest == checked_digest;
    if artifact_matches && digest_matches {
        Ok(())
    } else {
        Err(FreshnessMismatch {
            artifact_matches,
            digest_matches,
            generated_digest: generated.digest.clone(),
            checked_digest: checked_digest.to_string(),
        })
    }
}

pub fn write_checked_outputs(repo_root: &Path, generated: &GeneratedBundle) -> Result<(), String> {
    let output_dir = repo_root.join("tools/veln-repo-mcp-standard-library-docs/generated");
    fs::create_dir_all(&output_dir).map_err(|error| {
        format!("create the MCP standard-library package-documentation output directory: {error}")
    })?;
    fs::write(
        output_dir.join("mcp-standard-library-package-doc-resources-v2.json"),
        &generated.bytes,
    )
    .map_err(|error| {
        format!("write the checked MCP standard-library package-documentation resources: {error}")
    })?;
    fs::write(
        output_dir.join("mcp-standard-library-package-doc-resources-v2.sha256"),
        format!("{}\n", generated.digest),
    )
    .map_err(|error| {
        format!("write the checked MCP standard-library package-documentation digest: {error}")
    })?;
    Ok(())
}

pub fn bundle_digest(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(DIGEST_DOMAIN);
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
    hex_lower(&digest.finalize())
}

fn canonical_json(value: &Value) -> Result<String, String> {
    let mut out = serde_json::to_string(value).map_err(|error| {
        format!("serialize canonical MCP standard-library package-documentation JSON: {error}")
    })?;
    out.push('\n');
    Ok(out)
}

fn parse_bundle(bytes: &str) -> Result<CheckedBundle, String> {
    let value: Value = serde_json::from_str(bytes).map_err(|error| {
        format!("parse checked MCP standard-library package-documentation resources: {error}")
    })?;
    let object = value.as_object().ok_or_else(|| {
        "checked MCP standard-library package-documentation resources must be an object".to_string()
    })?;
    require_exact_keys(
        object.keys().map(String::as_str),
        [
            "declaration_locations",
            "documentation_digest",
            "generator_contract",
            "package_identity",
            "resources",
            "schema_version",
            "search_candidates",
            "snapshot_digest",
        ],
        "bundle",
    )?;
    let metadata = parse_bundle_metadata(&value)?;
    validate_metadata(&metadata)?;
    let resources = parse_bundle_collection(&value, "resources", parse_resource)?;
    validate_resources(&metadata, &resources)?;
    let search_candidates =
        parse_bundle_collection(&value, "search_candidates", parse_search_candidate)?;
    let declaration_locations =
        parse_bundle_collection(&value, "declaration_locations", parse_declaration_location)?;
    validate_indexes(
        &metadata,
        &resources,
        &search_candidates,
        &declaration_locations,
    )?;
    Ok(CheckedBundle {
        metadata,
        resources,
        search_candidates,
        declaration_locations,
    })
}

fn parse_bundle_metadata(value: &Value) -> Result<BundleMetadata, String> {
    Ok(BundleMetadata {
        schema_version: value["schema_version"].as_u64().ok_or_else(|| {
            "checked resource bundle schema_version must be an unsigned integer".to_string()
        })?,
        generator_contract: string_field(value, "generator_contract")?.to_string(),
        package_identity: string_field(value, "package_identity")?.to_string(),
        snapshot_digest: digest_field(value, "snapshot_digest")?.to_string(),
        documentation_digest: digest_field(value, "documentation_digest")?.to_string(),
    })
}

fn parse_bundle_collection<T>(
    value: &Value,
    field: &str,
    parse_item: fn(&Value) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    value[field]
        .as_array()
        .ok_or_else(|| format!("checked resource bundle {field} must be an array"))?
        .iter()
        .map(parse_item)
        .collect()
}

fn validate_indexes(
    metadata: &BundleMetadata,
    resources: &[CheckedResource],
    search_candidates: &[CheckedSearchCandidate],
    declaration_locations: &[CheckedDeclarationLocation],
) -> Result<(), String> {
    let status_only = resources.len() == 1 && resources[0].uri.ends_with("/status");
    if status_only {
        return if search_candidates.is_empty() && declaration_locations.is_empty() {
            Ok(())
        } else {
            Err("a checked status bundle must not contain documentation indexes".to_string())
        };
    }
    if search_candidates.is_empty() || declaration_locations.is_empty() {
        return Err(
            "checked resource bundle must contain search candidates and declaration locations"
                .to_string(),
        );
    }
    let resource_uris = resources
        .iter()
        .map(|resource| resource.uri.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(candidate) = search_candidates
        .iter()
        .find(|candidate| !resource_uris.contains(candidate.uri.as_str()))
    {
        return Err(format!(
            "checked search candidate URI `{}` must identify a checked resource",
            candidate.uri
        ));
    }
    let source_base = format!(
        "veln-pkg:///{}/snapshot/{}/",
        metadata.package_identity, metadata.snapshot_digest
    );
    if let Some(location) = declaration_locations.iter().find(|location| {
        !location.source_uri.starts_with(&source_base)
            || !resource_uris.contains(location.declaration_uri.as_str())
    }) {
        return Err(format!(
            "checked declaration location `{}` must identify the checked package and a checked resource",
            location.source_uri
        ));
    }
    Ok(())
}

fn parse_search_candidate(value: &Value) -> Result<CheckedSearchCandidate, String> {
    let object = value.as_object().ok_or_else(|| {
        "each checked MCP standard-library search candidate must be an object".to_string()
    })?;
    require_exact_keys(
        object.keys().map(String::as_str),
        [
            "documentation",
            "identifier",
            "keywords",
            "name",
            "signature",
            "summary",
            "title",
            "uri",
        ],
        "search candidate",
    )?;
    Ok(CheckedSearchCandidate {
        uri: string_field(value, "uri")?.to_string(),
        identifier: string_field(value, "identifier")?.to_string(),
        title: string_field(value, "title")?.to_string(),
        name: string_field(value, "name")?.to_string(),
        summary: string_field(value, "summary")?.to_string(),
        keywords: string_array_field(value, "keywords")?,
        signature: optional_string_field(value, "signature")?,
        documentation: string_array_field(value, "documentation")?,
    })
}

fn parse_declaration_location(value: &Value) -> Result<CheckedDeclarationLocation, String> {
    let object = value.as_object().ok_or_else(|| {
        "each checked MCP standard-library declaration location must be an object".to_string()
    })?;
    require_exact_keys(
        object.keys().map(String::as_str),
        ["column", "declaration_uri", "line", "offset", "source_uri"],
        "declaration location",
    )?;
    Ok(CheckedDeclarationLocation {
        source_uri: string_field(value, "source_uri")?.to_string(),
        line: usize_field(value, "line")?,
        column: usize_field(value, "column")?,
        offset: usize_field(value, "offset")?,
        declaration_uri: string_field(value, "declaration_uri")?.to_string(),
    })
}

fn validate_metadata(metadata: &BundleMetadata) -> Result<(), String> {
    if metadata.schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(format!(
            "checked resource bundle schema_version must be {BUNDLE_SCHEMA_VERSION}"
        ));
    }
    if metadata.generator_contract != GENERATOR_CONTRACT {
        return Err(format!(
            "checked resource bundle generator_contract must be `{GENERATOR_CONTRACT}`"
        ));
    }
    if metadata.package_identity != PACKAGE_IDENTITY {
        return Err(format!(
            "checked resource bundle package_identity must be `{PACKAGE_IDENTITY}`"
        ));
    }
    Ok(())
}

fn parse_resource(value: &Value) -> Result<CheckedResource, String> {
    let object = value.as_object().ok_or_else(|| {
        "each checked MCP standard-library package-documentation resource must be an object"
            .to_string()
    })?;
    require_exact_keys(
        object.keys().map(String::as_str),
        [
            "description",
            "listed",
            "mime_type",
            "name",
            "text",
            "title",
            "uri",
        ],
        "resource",
    )?;
    let description = match &value["description"] {
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        _ => return Err("checked resource description must be a string or null".to_string()),
    };
    Ok(CheckedResource {
        uri: string_field(value, "uri")?.to_string(),
        name: string_field(value, "name")?.to_string(),
        title: string_field(value, "title")?.to_string(),
        description,
        mime_type: string_field(value, "mime_type")?.to_string(),
        text: string_field(value, "text")?.to_string(),
        listed: value["listed"]
            .as_bool()
            .ok_or_else(|| "checked resource listed must be a boolean".to_string())?,
    })
}

fn validate_resources(
    metadata: &BundleMetadata,
    resources: &[CheckedResource],
) -> Result<(), String> {
    if resources.is_empty() {
        return Err("checked resource bundle must contain at least one resource".to_string());
    }
    let base = format!(
        "veln-doc:///package/{}/snapshot/{}/documentation/{}/",
        metadata.package_identity, metadata.snapshot_digest, metadata.documentation_digest
    );
    let listed = validate_resource_sequence(&base, resources)?;
    validate_listed_resource(&base, resources, &listed)
}

fn validate_resource_sequence<'a>(
    base: &str,
    resources: &'a [CheckedResource],
) -> Result<Vec<&'a CheckedResource>, String> {
    let mut previous_uri: Option<&str> = None;
    let mut listed = Vec::new();
    for resource in resources {
        validate_resource(base, resource)?;
        if previous_uri.is_some_and(|previous| previous.as_bytes() >= resource.uri.as_bytes()) {
            return Err("checked resources must be strictly sorted by URI bytes".to_string());
        }
        previous_uri = Some(&resource.uri);
        if resource.listed {
            listed.push(resource);
        }
    }
    Ok(listed)
}

fn validate_resource(base: &str, resource: &CheckedResource) -> Result<(), String> {
    if !resource.uri.starts_with(base) {
        return Err(format!(
            "checked resource URI `{}` must use the bundle identity and digests",
            resource.uri
        ));
    }
    if resource.mime_type != PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE {
        return Err(format!(
            "checked resource `{}` must use the package-documentation Markdown media type",
            resource.uri
        ));
    }
    if resource.name.is_empty() || resource.title.is_empty() || resource.text.is_empty() {
        return Err(format!(
            "checked resource `{}` must have non-empty name, title, and text",
            resource.uri
        ));
    }
    Ok(())
}

fn validate_listed_resource(
    base: &str,
    resources: &[CheckedResource],
    listed: &[&CheckedResource],
) -> Result<(), String> {
    if listed.len() != 1 {
        return Err("checked resource bundle must contain exactly one listed resource".to_string());
    }
    let listed_suffix = listed[0].uri.strip_prefix(base).unwrap_or_default();
    if listed_suffix != "index" && listed_suffix != "status" {
        return Err("the listed checked resource must be the package index or status".to_string());
    }
    if listed_suffix == "status" && resources.len() != 1 {
        return Err("a checked status resource must be the only published resource".to_string());
    }
    Ok(())
}

fn require_exact_keys<'a>(
    actual: impl IntoIterator<Item = &'a str>,
    expected: impl IntoIterator<Item = &'a str>,
    kind: &str,
) -> Result<(), String> {
    let actual = actual.into_iter().collect::<BTreeSet<_>>();
    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "checked resource {kind} keys must be closed; expected {expected:?}, found {actual:?}"
        ))
    }
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("checked resource bundle field `{field}` must be a string"))
}

fn optional_string_field(value: &Value, field: &str) -> Result<Option<String>, String> {
    match value.get(field) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        _ => Err(format!(
            "checked resource bundle field `{field}` must be a string or null"
        )),
    }
}

fn string_array_field(value: &Value, field: &str) -> Result<Vec<String>, String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("checked resource bundle field `{field}` must be an array"))?
        .iter()
        .map(|item| {
            item.as_str().map(str::to_string).ok_or_else(|| {
                format!("checked resource bundle field `{field}` must contain only strings")
            })
        })
        .collect()
}

fn usize_field(value: &Value, field: &str) -> Result<usize, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| {
            format!("checked resource bundle field `{field}` must be an unsigned integer")
        })
}

fn digest_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    let digest = string_field(value, field)?;
    if digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(digest)
    } else {
        Err(format!(
            "checked resource bundle field `{field}` must be 64 lowercase hexadecimal digits"
        ))
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests;
