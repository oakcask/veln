use std::error::Error;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::LowerHexBytes;

const PACKAGE_SNAPSHOT_DOMAIN: &[u8] = b"veln-package-snapshot/v1\0";

/// One source record in a package snapshot digest transcript.
///
/// `path` must be a normalized package-relative UTF-8 path. The digest API
/// preserves both the path and source bytes exactly and does not validate path
/// normalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackageSnapshotSource<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
}

impl<'a> PackageSnapshotSource<'a> {
    pub const fn new(path: &'a str, bytes: &'a [u8]) -> Self {
        Self { path, bytes }
    }
}

/// An input that cannot be encoded by the package snapshot v1 transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageSnapshotDigestError {
    /// More than one source used the same normalized UTF-8 path.
    DuplicateSourcePath(String),
    /// A byte length or the source count exceeded the transcript's u64 field.
    InputTooLarge,
}

impl fmt::Display for PackageSnapshotDigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSourcePath(path) => {
                write!(formatter, "duplicate package snapshot source path `{path}`")
            }
            Self::InputTooLarge => formatter.write_str("package snapshot input exceeds u64 limits"),
        }
    }
}

impl Error for PackageSnapshotDigestError {}

/// Computes the version-one package snapshot digest from exact captured bytes.
///
/// Sources are ordered by their normalized path UTF-8 bytes, so the input
/// slice order does not affect the result. The returned string contains 64
/// lowercase hexadecimal SHA-256 digits without a prefix.
pub fn package_snapshot_digest(
    manifest_bytes: &[u8],
    sources: &[PackageSnapshotSource<'_>],
) -> Result<String, PackageSnapshotDigestError> {
    let mut sources = sources.to_vec();
    sources.sort_unstable_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));

    for pair in sources.windows(2) {
        if pair[0].path == pair[1].path {
            return Err(PackageSnapshotDigestError::DuplicateSourcePath(
                pair[0].path.to_string(),
            ));
        }
    }

    let mut hash = Sha256::new();
    hash.update(PACKAGE_SNAPSHOT_DOMAIN);
    hash.update([0x01]);
    hash.update(encoded_len(manifest_bytes.len())?);
    hash.update(manifest_bytes);
    hash.update([0x02]);
    hash.update(encoded_len(sources.len())?);
    for source in sources {
        hash.update([0x03]);
        hash.update(encoded_len(source.path.len())?);
        hash.update(source.path.as_bytes());
        hash.update(encoded_len(source.bytes.len())?);
        hash.update(source.bytes);
    }

    let digest = hash.finalize();
    Ok(format!("{:x}", LowerHexBytes(&digest)))
}

fn encoded_len(len: usize) -> Result<[u8; 8], PackageSnapshotDigestError> {
    u64::try_from(len)
        .map(u64::to_be_bytes)
        .map_err(|_| PackageSnapshotDigestError::InputTooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            PACKAGE_SNAPSHOT_DOMAIN
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
}
