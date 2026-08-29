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
