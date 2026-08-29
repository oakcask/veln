use super::*;
use crate::LowerHexBytes;
use sha2::{Digest, Sha256};

#[test]
fn package_snapshot_digest_matches_fixed_vectors() {
    let vectors = [
        (
            b"".as_slice(),
            Vec::new(),
            "f0030b92642915b495c426a5b5185676e0306219a52c448a94fb5e8dccc494ad",
        ),
        (
            b"[package]\nname = \"p\"\n".as_slice(),
            vec![
                PackageSnapshotSource::new("a.veln", b"a\n"),
                PackageSnapshotSource::new("z.veln", b"z\n"),
            ],
            "77150b975c9bb56aab9e9b3c8899a81907abc9db535fdfbb6276d40bff9fa878",
        ),
        (
            b"".as_slice(),
            vec![PackageSnapshotSource::new("src/λ.veln", "λ\n".as_bytes())],
            "f360e18455f6b7c90dd6c34cdec7a444082e003e44583dc8a7d99ae50cba713b",
        ),
    ];

    for (manifest, sources, expected) in vectors {
        let actual = package_snapshot_digest(manifest, &sources).expect("vector should hash");
        assert_eq!(actual, expected);
        assert_eq!(actual.len(), 64);
        assert!(
            actual
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        );
    }
}

#[test]
fn package_snapshot_digest_ignores_source_discovery_order() {
    let manifest = b"[package]\nname = \"p\"\n";
    let ordered = [
        PackageSnapshotSource::new("a.veln", b"a\n"),
        PackageSnapshotSource::new("z.veln", b"z\n"),
    ];
    let reversed = [ordered[1], ordered[0]];

    assert_eq!(
        package_snapshot_digest(manifest, &ordered),
        package_snapshot_digest(manifest, &reversed)
    );
}

#[test]
fn package_snapshot_digest_rejects_duplicate_paths() {
    let sources = [
        PackageSnapshotSource::new("same.veln", b"first"),
        PackageSnapshotSource::new("same.veln", b"second"),
    ];

    assert_eq!(
        package_snapshot_digest(b"", &sources),
        Err(PackageSnapshotDigestError::DuplicateSourcePath(
            "same.veln".to_string()
        ))
    );
}

#[test]
fn package_snapshot_digest_changes_for_each_transcript_mutation() {
    let baseline_sources = [PackageSnapshotSource::new("a.veln", b"A\n")];
    let baseline = package_snapshot_digest(b"manifest", &baseline_sources).unwrap();
    assert_eq!(baseline, transcript_digest(TranscriptMutation::None));

    let changed_inputs = [
        package_snapshot_digest(b"Manifest", &baseline_sources).unwrap(),
        package_snapshot_digest(b"manifest", &[PackageSnapshotSource::new("b.veln", b"A\n")])
            .unwrap(),
        package_snapshot_digest(b"manifest", &[PackageSnapshotSource::new("a.veln", b"B\n")])
            .unwrap(),
    ];
    for changed in changed_inputs {
        assert_ne!(baseline, changed);
    }

    let mutations = [
        TranscriptMutation::DomainByte,
        TranscriptMutation::ManifestTag,
        TranscriptMutation::SourceCountTag,
        TranscriptMutation::SourceTag,
        TranscriptMutation::LittleEndianLength,
    ];

    for mutation in mutations {
        assert_ne!(baseline, transcript_digest(mutation), "{mutation:?}");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TranscriptMutation {
    None,
    DomainByte,
    ManifestTag,
    SourceCountTag,
    SourceTag,
    LittleEndianLength,
}

fn transcript_digest(mutation: TranscriptMutation) -> String {
    let domain = if mutation == TranscriptMutation::DomainByte {
        b"veln-package-snapshot/v2\0".as_slice()
    } else {
        b"veln-package-snapshot/v1\0"
    };
    let manifest_tag = if mutation == TranscriptMutation::ManifestTag {
        0x04
    } else {
        0x01
    };
    let source_count_tag = if mutation == TranscriptMutation::SourceCountTag {
        0x05
    } else {
        0x02
    };
    let source_tag = if mutation == TranscriptMutation::SourceTag {
        0x06
    } else {
        0x03
    };
    let manifest = b"manifest";
    let path = "a.veln";
    let source = b"A\n";
    let length = |len: usize| u64::try_from(len).unwrap().to_be_bytes();
    let manifest_length = if mutation == TranscriptMutation::LittleEndianLength {
        u64::try_from(manifest.len()).unwrap().to_le_bytes()
    } else {
        length(manifest.len())
    };

    let mut transcript = Vec::new();
    transcript.extend_from_slice(domain);
    transcript.push(manifest_tag);
    transcript.extend_from_slice(&manifest_length);
    transcript.extend_from_slice(manifest);
    transcript.push(source_count_tag);
    transcript.extend_from_slice(&length(1));
    transcript.push(source_tag);
    transcript.extend_from_slice(&length(path.len()));
    transcript.extend_from_slice(path.as_bytes());
    transcript.extend_from_slice(&length(source.len()));
    transcript.extend_from_slice(source);

    format!("{:x}", LowerHexBytes(&Sha256::digest(transcript)))
}
