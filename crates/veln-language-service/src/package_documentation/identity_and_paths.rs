use super::*;

pub(super) fn doc_digest(canonical_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(DOC_DOMAIN);
    hasher.update((canonical_bytes.len() as u64).to_be_bytes());
    hasher.update(canonical_bytes);
    lower_hex(hasher.finalize().as_slice())
}

pub(super) fn declaration_id(kind: &str, identity: &str) -> String {
    digest_hex(
        DECLARATION_ID_DOMAIN,
        &[kind.as_bytes(), identity.as_bytes()],
    )
}

pub(super) fn module_id(source_path: &str) -> String {
    digest_hex(MODULE_ID_DOMAIN, &[source_path.as_bytes()])
}

pub(super) fn digest_hex(domain: &[u8], parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        let len = u64::try_from((*part).len()).expect("digest transcript part length fits u64");
        hasher.update(len.to_be_bytes());
        hasher.update(part);
    }
    lower_hex(hasher.finalize().as_slice())
}

pub(super) fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn source_uri(identity: &str, digest: &str, source_path: &str) -> String {
    if identity.is_empty() || digest.is_empty() {
        return source_path.to_string();
    }
    let mut uri = String::from("veln-pkg:///");
    uri.push_str(&encoded_segment(identity));
    uri.push_str("/snapshot/");
    uri.push_str(digest);
    uri.push('/');
    for (index, segment) in source_path.split('/').enumerate() {
        if index > 0 {
            uri.push('/');
        }
        uri.push_str(&encoded_segment(segment));
    }
    uri
}

pub(super) fn is_package_relative_path(path: &str) -> bool {
    let path = std::path::Path::new(path);
    !path.is_absolute()
        && !path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
}

pub(super) fn is_test_source_path(path: &str) -> bool {
    classify_companion_source(path).is_some() || path.ends_with("_test.veln")
}

pub(super) fn manifest_field(fields: &[veln_project::ManifestField], key: &str) -> Option<String> {
    manifest_field_with_span(fields, key).map(|field| field.value.clone())
}

pub(super) fn manifest_field_with_span<'a>(
    fields: &'a [veln_project::ManifestField],
    key: &str,
) -> Option<&'a veln_project::ManifestField> {
    fields.iter().find(|field| field.key == key)
}

pub(super) fn manifest_list_field(
    fields: &[veln_project::ManifestField],
    key: &str,
) -> Vec<String> {
    manifest_field(fields, key)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}
