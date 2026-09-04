use super::*;

#[test]
fn checked_bundle_has_closed_identity_and_publication_boundary() {
    let bundle = checked_bundle().unwrap();

    assert_eq!(bundle.metadata.schema_version, BUNDLE_SCHEMA_VERSION);
    assert_eq!(bundle.metadata.generator_contract, GENERATOR_CONTRACT);
    assert_eq!(bundle.metadata.package_identity, PACKAGE_IDENTITY);
    assert_eq!(
        bundle
            .resources
            .iter()
            .filter(|resource| resource.listed)
            .count(),
        1
    );
    assert!(
        bundle
            .resources
            .iter()
            .any(|resource| resource.uri.ends_with("/index") && resource.listed)
    );
    assert!(
        bundle
            .resources
            .iter()
            .any(|resource| resource.uri.contains("/module/") && !resource.listed)
    );
    assert!(
        bundle
            .resources
            .iter()
            .any(|resource| resource.uri.contains("/declaration/") && !resource.listed)
    );
}

#[test]
fn checked_digest_covers_exact_artifact_bytes() {
    assert_eq!(
        bundle_digest(checked_artifact_bytes().as_bytes()),
        checked_artifact_digest()
    );
    let mut changed = checked_artifact_bytes().as_bytes().to_vec();
    changed.push(b' ');
    assert_ne!(bundle_digest(&changed), checked_artifact_digest());
}

#[test]
fn snapshot_loader_rejects_stale_standard_library_resources() {
    let digest = checked_bundle().unwrap().metadata.snapshot_digest;
    assert!(checked_bundle_for_snapshot(&digest).is_ok());
    assert!(checked_bundle_for_snapshot(&"0".repeat(64)).is_err());
}

#[test]
fn parser_rejects_open_or_mismatched_bundle_shapes() {
    let mut value: Value = serde_json::from_str(checked_artifact_bytes()).unwrap();
    value["unknown"] = json!(true);
    assert!(parse_bundle(&canonical_json(&value).unwrap()).is_err());

    let mut value: Value = serde_json::from_str(checked_artifact_bytes()).unwrap();
    value["snapshot_digest"] = json!("0".repeat(64));
    assert!(parse_bundle(&canonical_json(&value).unwrap()).is_err());
}

#[test]
fn parser_rejects_resource_order_and_listed_boundary_drift() {
    let mut value: Value = serde_json::from_str(checked_artifact_bytes()).unwrap();
    value["resources"].as_array_mut().unwrap().reverse();
    assert!(parse_bundle(&canonical_json(&value).unwrap()).is_err());

    let mut value: Value = serde_json::from_str(checked_artifact_bytes()).unwrap();
    for resource in value["resources"].as_array_mut().unwrap() {
        resource["listed"] = json!(false);
    }
    assert!(parse_bundle(&canonical_json(&value).unwrap()).is_err());
}

#[test]
fn parser_accepts_one_listed_status_resource_without_partial_documentation() {
    let mut value: Value = serde_json::from_str(checked_artifact_bytes()).unwrap();
    let base = format!(
        "veln-doc:///package/{}/snapshot/{}/documentation/{}/status",
        value["package_identity"].as_str().unwrap(),
        value["snapshot_digest"].as_str().unwrap(),
        value["documentation_digest"].as_str().unwrap(),
    );
    value["resources"] = json!([{
        "description": null,
        "listed": true,
        "mime_type": PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE,
        "name": "std-documentation-status",
        "text": "# Package Documentation Status: std\n",
        "title": "Veln package documentation status: std",
        "uri": base,
    }]);

    let bundle = parse_bundle(&canonical_json(&value).unwrap()).unwrap();

    assert_eq!(bundle.resources.len(), 1);
    assert!(bundle.resources[0].listed);
    assert!(bundle.resources[0].uri.ends_with("/status"));
}

#[test]
fn stale_baselines_are_reported_without_accepting_one_matching_dimension() {
    let generated = GeneratedBundle {
        bytes: checked_artifact_bytes().to_string(),
        digest: checked_artifact_digest().to_string(),
        bundle: checked_bundle().unwrap(),
    };
    let changed_artifact = checked_artifact_bytes().replacen(
        "Veln package documentation: std",
        "Changed package documentation",
        1,
    );
    let mismatch =
        verify_generated_against(&generated, &changed_artifact, checked_artifact_digest())
            .expect_err("changed artifact should be stale");
    assert!(!mismatch.artifact_matches);
    assert!(mismatch.digest_matches);

    let mismatch = verify_generated_against(&generated, checked_artifact_bytes(), &"0".repeat(64))
        .expect_err("changed digest should be stale");
    assert!(mismatch.artifact_matches);
    assert!(!mismatch.digest_matches);
}
