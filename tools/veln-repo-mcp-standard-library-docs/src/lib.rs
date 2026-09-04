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

pub const BUNDLE_SCHEMA_VERSION: u64 = 1;
pub const GENERATOR_CONTRACT: &str = "veln-mcp-package-documentation/v1";
pub const DIGEST_DOMAIN: &[u8] = b"veln-mcp-standard-library-package-doc-resources/v1\0";
pub const CHECKED_ARTIFACT: &str =
    include_str!("../generated/mcp-standard-library-package-doc-resources-v1.json");
pub const CHECKED_DIGEST: &str =
    include_str!("../generated/mcp-standard-library-package-doc-resources-v1.sha256");

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
    let value = json!({
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
        output_dir.join("mcp-standard-library-package-doc-resources-v1.json"),
        &generated.bytes,
    )
    .map_err(|error| {
        format!("write the checked MCP standard-library package-documentation resources: {error}")
    })?;
    fs::write(
        output_dir.join("mcp-standard-library-package-doc-resources-v1.sha256"),
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
            "documentation_digest",
            "generator_contract",
            "package_identity",
            "resources",
            "schema_version",
            "snapshot_digest",
        ],
        "bundle",
    )?;
    let metadata = BundleMetadata {
        schema_version: value["schema_version"].as_u64().ok_or_else(|| {
            "checked resource bundle schema_version must be an unsigned integer".to_string()
        })?,
        generator_contract: string_field(&value, "generator_contract")?.to_string(),
        package_identity: string_field(&value, "package_identity")?.to_string(),
        snapshot_digest: digest_field(&value, "snapshot_digest")?.to_string(),
        documentation_digest: digest_field(&value, "documentation_digest")?.to_string(),
    };
    validate_metadata(&metadata)?;
    let resources = value["resources"]
        .as_array()
        .ok_or_else(|| "checked resource bundle resources must be an array".to_string())?
        .iter()
        .map(parse_resource)
        .collect::<Result<Vec<_>, _>>()?;
    validate_resources(&metadata, &resources)?;
    Ok(CheckedBundle {
        metadata,
        resources,
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
    let mut previous_uri: Option<&str> = None;
    let mut uris = BTreeSet::new();
    let mut listed = Vec::new();
    for resource in resources {
        if !resource.uri.starts_with(&base) {
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
        if previous_uri.is_some_and(|previous| previous.as_bytes() >= resource.uri.as_bytes()) {
            return Err("checked resources must be strictly sorted by URI bytes".to_string());
        }
        previous_uri = Some(&resource.uri);
        if !uris.insert(resource.uri.as_str()) {
            return Err(format!(
                "checked resource URI `{}` is duplicated",
                resource.uri
            ));
        }
        if resource.listed {
            listed.push(resource);
        }
    }
    if listed.len() != 1 {
        return Err("checked resource bundle must contain exactly one listed resource".to_string());
    }
    let listed_suffix = listed[0].uri.strip_prefix(&base).unwrap_or_default();
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
