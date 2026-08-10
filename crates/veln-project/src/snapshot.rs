use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::portable::{default_case_fold, validate_source_path};
use crate::{LowerHexBytes, PortableSourcePathError};

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

/// One owned distribution source retained by a captured package snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedPackageSource {
    path: String,
    bytes: Vec<u8>,
}

impl CapturedPackageSource {
    /// Returns the deterministic package-relative path with `/` separators.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the exact bytes read from the source file.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// An immutable capture of one package's manifest and distribution sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedPackageSnapshot {
    manifest_bytes: Vec<u8>,
    sources: Vec<CapturedPackageSource>,
    digest: String,
}

impl CapturedPackageSnapshot {
    /// Returns the exact bytes read from the package-root `veln.toml`.
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    /// Returns the owned sources in package-relative UTF-8 byte order.
    pub fn sources(&self) -> &[CapturedPackageSource] {
        &self.sources
    }

    /// Returns the package snapshot digest computed from this capture.
    pub fn digest(&self) -> &str {
        &self.digest
    }
}

/// A filesystem package cannot be captured as a package snapshot.
#[derive(Debug)]
pub enum PackageSnapshotCaptureError {
    /// The package root does not contain a regular `veln.toml`.
    ManifestNotRegular(PathBuf),
    /// A discovered entry has no exact UTF-8 package-relative representation.
    UnrepresentablePath(PathBuf),
    /// A represented distribution source path is outside the portable domain.
    InvalidSourcePath {
        path: String,
        reason: PortableSourcePathError,
    },
    /// A distribution source does not contain valid UTF-8 source text.
    InvalidSourceText { path: String, valid_up_to: usize },
    /// Two exact source-path spellings collide under Unicode default case folding.
    SourcePathCollision { first: String, second: String },
    /// A discovered distribution source path is not a regular file.
    SourceNotRegular(PathBuf),
    /// A filesystem operation failed for the retained path.
    Filesystem { path: PathBuf, source: io::Error },
    /// The captured inputs cannot be encoded by the digest transcript.
    Digest(PackageSnapshotDigestError),
}

impl fmt::Display for PackageSnapshotCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManifestNotRegular(path) => write!(
                formatter,
                "package snapshot manifest is not a regular file: {}",
                path.display()
            ),
            Self::UnrepresentablePath(path) => write!(
                formatter,
                "package snapshot path is not valid UTF-8: {}",
                path.display()
            ),
            Self::InvalidSourcePath { path, reason } => {
                write!(
                    formatter,
                    "invalid package snapshot source path `{path}`: {reason}"
                )
            }
            Self::InvalidSourceText { path, valid_up_to } => write!(
                formatter,
                "package snapshot source `{path}` is not valid UTF-8 at byte {valid_up_to}"
            ),
            Self::SourcePathCollision { first, second } => write!(
                formatter,
                "package snapshot source paths `{first}` and `{second}` collide after Unicode default case folding"
            ),
            Self::SourceNotRegular(path) => write!(
                formatter,
                "package snapshot source is not a regular file: {}",
                path.display()
            ),
            Self::Filesystem { path, source } => {
                write!(formatter, "cannot capture {}: {source}", path.display())
            }
            Self::Digest(source) => write!(formatter, "cannot digest package snapshot: {source}"),
        }
    }
}

impl Error for PackageSnapshotCaptureError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Filesystem { source, .. } => Some(source),
            Self::Digest(source) => Some(source),
            Self::ManifestNotRegular(_)
            | Self::UnrepresentablePath(_)
            | Self::InvalidSourcePath { .. }
            | Self::InvalidSourceText { .. }
            | Self::SourcePathCollision { .. }
            | Self::SourceNotRegular(_) => None,
        }
    }
}

/// Captures a package root's regular manifest and owned distribution sources.
///
/// Symbolic links, `.git`, descendant package trees, and test sources are
/// excluded. Source bytes are retained exactly and sources are ordered by
/// their package-relative `/`-separated paths before the digest is computed.
pub fn capture_package_snapshot(
    root: &Path,
) -> Result<CapturedPackageSnapshot, PackageSnapshotCaptureError> {
    let manifest_path = root.join("veln.toml");
    let manifest_type = symlink_file_type(&manifest_path)?;
    if !manifest_type.is_file() {
        return Err(PackageSnapshotCaptureError::ManifestNotRegular(
            manifest_path,
        ));
    }
    let manifest_bytes = read_capture_file(&manifest_path)?;

    let mut sources = Vec::new();
    collect_package_sources(root, Path::new(""), &mut sources)?;
    captured_package_snapshot(manifest_bytes, sources)
}

/// Captures an immutable package snapshot from already embedded inputs.
///
/// This applies the same ordering, portable-path, UTF-8, collision, and digest
/// contracts as filesystem capture without materializing the inputs.
pub fn capture_embedded_package_snapshot<'a>(
    manifest_bytes: &[u8],
    sources: impl IntoIterator<Item = PackageSnapshotSource<'a>>,
) -> Result<CapturedPackageSnapshot, PackageSnapshotCaptureError> {
    captured_package_snapshot(
        manifest_bytes.to_vec(),
        sources
            .into_iter()
            .filter(|source| is_distribution_source(source.path))
            .map(|source| CapturedPackageSource {
                path: source.path.to_string(),
                bytes: source.bytes.to_vec(),
            })
            .collect(),
    )
}

fn captured_package_snapshot(
    manifest_bytes: Vec<u8>,
    mut sources: Vec<CapturedPackageSource>,
) -> Result<CapturedPackageSnapshot, PackageSnapshotCaptureError> {
    sources.sort_unstable_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    validate_captured_sources(&sources)?;

    let digest_sources = sources
        .iter()
        .map(|source| PackageSnapshotSource::new(&source.path, &source.bytes))
        .collect::<Vec<_>>();
    let digest = package_snapshot_digest(&manifest_bytes, &digest_sources)
        .map_err(PackageSnapshotCaptureError::Digest)?;

    Ok(CapturedPackageSnapshot {
        manifest_bytes,
        sources,
        digest,
    })
}

fn validate_captured_sources(
    sources: &[CapturedPackageSource],
) -> Result<(), PackageSnapshotCaptureError> {
    let mut folded_paths = BTreeMap::new();
    for source in sources {
        validate_source_path(&source.path).map_err(|reason| {
            PackageSnapshotCaptureError::InvalidSourcePath {
                path: source.path.clone(),
                reason,
            }
        })?;
        if let Err(error) = std::str::from_utf8(&source.bytes) {
            return Err(PackageSnapshotCaptureError::InvalidSourceText {
                path: source.path.clone(),
                valid_up_to: error.valid_up_to(),
            });
        }

        let folded = default_case_fold(&source.path);
        if let Some(first) = folded_paths.insert(folded, source.path.clone()) {
            return Err(PackageSnapshotCaptureError::SourcePathCollision {
                first,
                second: source.path.clone(),
            });
        }
    }
    Ok(())
}

fn collect_package_sources(
    root: &Path,
    relative_dir: &Path,
    sources: &mut Vec<CapturedPackageSource>,
) -> Result<(), PackageSnapshotCaptureError> {
    let directory = root.join(relative_dir);
    let entries =
        fs::read_dir(&directory).map_err(|source| filesystem_error(directory.clone(), source))?;
    for entry in entries {
        let entry = entry.map_err(|source| filesystem_error(directory.clone(), source))?;
        let file_name = entry.file_name();
        let relative_path = relative_dir.join(&file_name);
        let path = root.join(&relative_path);
        if file_name == OsStr::new(".git") {
            continue;
        }

        let file_type = entry
            .file_type()
            .map_err(|source| filesystem_error(path.clone(), source))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if has_regular_manifest(&path)? {
                continue;
            }
            collect_package_sources(root, &relative_path, sources)?;
        } else {
            if is_excluded_test_source_path(&relative_path) {
                continue;
            }
            let relative_utf8 = package_relative_path(&relative_path)?;
            if is_distribution_source(&relative_utf8) {
                if !file_type.is_file() {
                    return Err(PackageSnapshotCaptureError::SourceNotRegular(path));
                }
                sources.push(CapturedPackageSource {
                    path: relative_utf8,
                    bytes: read_capture_file(&path)?,
                });
            }
        }
    }
    Ok(())
}

fn package_relative_path(path: &Path) -> Result<String, PackageSnapshotCaptureError> {
    let mut normalized = String::new();
    for component in path.components() {
        let component = component
            .as_os_str()
            .to_str()
            .ok_or_else(|| PackageSnapshotCaptureError::UnrepresentablePath(path.to_path_buf()))?;
        if !normalized.is_empty() {
            normalized.push('/');
        }
        normalized.push_str(component);
    }
    Ok(normalized)
}

fn is_distribution_source(path: &str) -> bool {
    path.ends_with(".veln") && !path.ends_with(".test.veln") && !path.ends_with("_test.veln")
}

fn is_excluded_test_source_path(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        is_excluded_test_source_bytes(path.as_os_str().as_bytes())
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let path = path.as_os_str().encode_wide().collect::<Vec<_>>();
        is_excluded_test_source_wide(&path)
    }

    #[cfg(not(any(unix, windows)))]
    {
        path.to_str()
            .is_some_and(|path| path.ends_with(".test.veln") || path.ends_with("_test.veln"))
    }
}

#[cfg(unix)]
fn is_excluded_test_source_bytes(path: &[u8]) -> bool {
    path.ends_with(b".test.veln") || path.ends_with(b"_test.veln")
}

#[cfg(windows)]
fn is_excluded_test_source_wide(path: &[u16]) -> bool {
    const COMPANION_SUFFIX: &[u16] = &[
        b'.' as u16,
        b't' as u16,
        b'e' as u16,
        b's' as u16,
        b't' as u16,
        b'.' as u16,
        b'v' as u16,
        b'e' as u16,
        b'l' as u16,
        b'n' as u16,
    ];
    const INTEGRATION_SUFFIX: &[u16] = &[
        b'_' as u16,
        b't' as u16,
        b'e' as u16,
        b's' as u16,
        b't' as u16,
        b'.' as u16,
        b'v' as u16,
        b'e' as u16,
        b'l' as u16,
        b'n' as u16,
    ];

    path.ends_with(COMPANION_SUFFIX) || path.ends_with(INTEGRATION_SUFFIX)
}

fn has_regular_manifest(directory: &Path) -> Result<bool, PackageSnapshotCaptureError> {
    let manifest = directory.join("veln.toml");
    match fs::symlink_metadata(&manifest) {
        Ok(metadata) => Ok(metadata.file_type().is_file()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(filesystem_error(manifest, source)),
    }
}

fn symlink_file_type(path: &Path) -> Result<fs::FileType, PackageSnapshotCaptureError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Err(
            PackageSnapshotCaptureError::ManifestNotRegular(path.to_path_buf()),
        ),
        Err(source) => Err(filesystem_error(path.to_path_buf(), source)),
    }
}

fn read_capture_file(path: &Path) -> Result<Vec<u8>, PackageSnapshotCaptureError> {
    fs::read(path).map_err(|source| filesystem_error(path.to_path_buf(), source))
}

fn filesystem_error(path: PathBuf, source: io::Error) -> PackageSnapshotCaptureError {
    PackageSnapshotCaptureError::Filesystem { path, source }
}

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
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn package_snapshot_capture_owns_the_distribution_set_in_path_order() {
        let package = SnapshotFixture::new("distribution-set");
        package.write_bytes("veln.toml", b"manifest\r\n\xff");
        let cases = [
            ("z.veln", b"public".as_slice(), true),
            ("src/private.veln", b"private", true),
            ("generated/api.veln", b"generated", true),
            ("target/code.veln", b"target", true),
            ("src/unit.test.veln", b"companion", false),
            ("tests/api_test.veln", b"integration", false),
            ("notes.txt", b"other", false),
            ("nested/.git/hidden.veln", b"git", false),
            ("dependency/owned.veln", b"descendant", false),
        ];
        for (path, bytes, _) in cases {
            package.write_bytes(path, bytes);
        }
        package.write_bytes("dependency/veln.toml", b"nested manifest");

        let snapshot = capture_package_snapshot(package.root()).unwrap();
        let actual = snapshot
            .sources()
            .iter()
            .map(|source| (source.path(), source.bytes()))
            .collect::<Vec<_>>();

        assert_eq!(snapshot.manifest_bytes(), b"manifest\r\n\xff");
        assert_eq!(
            actual,
            vec![
                ("generated/api.veln", b"generated".as_slice()),
                ("src/private.veln", b"private".as_slice()),
                ("target/code.veln", b"target".as_slice()),
                ("z.veln", b"public".as_slice()),
            ]
        );
        assert!(snapshot.sources().iter().all(|source| {
            cases.iter().any(|(path, bytes, included)| {
                *included && *path == source.path() && *bytes == source.bytes()
            })
        }));
    }

    #[test]
    fn embedded_snapshot_matches_filesystem_snapshot_without_materialization() {
        let package = SnapshotFixture::new("embedded-equivalence");
        package.write_bytes("veln.toml", b"[package]\nname = \"std\"\n");
        package.write_bytes("nested/b.veln", b"pub fn b() -> Int\n  2\nend\n");
        package.write_bytes("a.veln", b"pub fn a() -> Int\r\n  1\r\nend\r\n");
        let filesystem = capture_package_snapshot(package.root()).unwrap();
        let embedded = capture_embedded_package_snapshot(
            filesystem.manifest_bytes(),
            filesystem
                .sources()
                .iter()
                .rev()
                .map(|source| PackageSnapshotSource::new(source.path(), source.bytes())),
        )
        .unwrap();

        assert_eq!(embedded, filesystem);
    }

    #[test]
    fn embedded_snapshot_applies_distribution_test_source_exclusions() {
        let snapshot = capture_embedded_package_snapshot(
            b"[package]\nname = \"demo\"\n",
            [
                PackageSnapshotSource::new("main.veln", b"pub fn main() -> Int\n\t1\nend\n"),
                PackageSnapshotSource::new("main.test.veln", b"not parsed"),
                PackageSnapshotSource::new("main_test.veln", b"not parsed"),
                PackageSnapshotSource::new("notes.txt", b"not captured"),
            ],
        )
        .unwrap();

        assert_eq!(snapshot.sources().len(), 1);
        assert_eq!(snapshot.sources()[0].path(), "main.veln");
    }

    #[test]
    fn embedded_snapshot_reuses_portable_source_validation() {
        let error = capture_embedded_package_snapshot(
            b"manifest",
            [PackageSnapshotSource::new("dir/../main.veln", b"main\n")],
        )
        .unwrap_err();

        assert!(matches!(
            error,
            PackageSnapshotCaptureError::InvalidSourcePath { path, .. }
                if path == "dir/../main.veln"
        ));
    }

    #[test]
    fn package_snapshot_capture_digest_uses_the_retained_exact_bytes() {
        let package = SnapshotFixture::new("digest-integration");
        package.write_bytes("veln.toml", b"manifest\0bytes");
        package.write_bytes("src/b.veln", b"b\r\n");
        package.write_bytes("src/a.veln", "a\0λ".as_bytes());

        let snapshot = capture_package_snapshot(package.root()).unwrap();
        let digest_sources = snapshot
            .sources()
            .iter()
            .map(|source| PackageSnapshotSource::new(source.path(), source.bytes()))
            .collect::<Vec<_>>();

        assert_eq!(
            snapshot.digest(),
            package_snapshot_digest(snapshot.manifest_bytes(), &digest_sources).unwrap()
        );

        let original = snapshot.digest().to_string();
        package.write_bytes("veln.toml", b"manifest\0byteS");
        assert_ne!(
            capture_package_snapshot(package.root()).unwrap().digest(),
            original
        );
        package.write_bytes("veln.toml", b"manifest\0bytes");
        package.write_bytes("src/a.veln", "A\0λ".as_bytes());
        assert_ne!(
            capture_package_snapshot(package.root()).unwrap().digest(),
            original
        );
        package.write_bytes("src/a.veln", "a\0λ".as_bytes());
        package.write_bytes("src/new.veln", b"new");
        assert_ne!(
            capture_package_snapshot(package.root()).unwrap().digest(),
            original
        );
    }

    #[test]
    fn package_snapshot_capture_digest_is_independent_of_physical_parent() {
        let first = SnapshotFixture::new("relocation-first");
        let second = SnapshotFixture::new("relocation-second");
        for package in [&first, &second] {
            package.write_bytes("veln.toml", b"same manifest");
            package.write_bytes("src/main.veln", b"same source");
        }

        assert_ne!(first.root(), second.root());
        assert_eq!(
            capture_package_snapshot(first.root()).unwrap(),
            capture_package_snapshot(second.root()).unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn package_snapshot_capture_excludes_all_symbolic_links() {
        use std::os::unix::fs::symlink;

        let package = SnapshotFixture::new("symbolic-links");
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes("real.veln", b"real");
        package.write_bytes("ignored.txt", b"\xff");
        package.write_bytes("outside/linked.veln", b"outside");
        fs::create_dir_all(package.path("links")).unwrap();
        symlink(package.path("real.veln"), package.path("linked.veln")).unwrap();
        symlink(package.path("ignored.txt"), package.path("invalid.veln")).unwrap();
        symlink(package.path("outside"), package.path("links/directory")).unwrap();

        let snapshot = capture_package_snapshot(package.root()).unwrap();
        let paths = snapshot
            .sources()
            .iter()
            .map(CapturedPackageSource::path)
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["outside/linked.veln", "real.veln"]);
    }

    #[test]
    fn package_snapshot_capture_requires_a_regular_manifest() {
        let missing = SnapshotFixture::new("missing-manifest");
        assert!(matches!(
            capture_package_snapshot(missing.root()),
            Err(PackageSnapshotCaptureError::ManifestNotRegular(_))
        ));

        let directory = SnapshotFixture::new("directory-manifest");
        fs::create_dir_all(directory.path("veln.toml")).unwrap();
        assert!(matches!(
            capture_package_snapshot(directory.root()),
            Err(PackageSnapshotCaptureError::ManifestNotRegular(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn package_snapshot_capture_rejects_a_symbolic_manifest() {
        use std::os::unix::fs::symlink;

        let package = SnapshotFixture::new("symbolic-manifest");
        package.write_bytes("manifest-target", b"manifest");
        symlink(package.path("manifest-target"), package.path("veln.toml")).unwrap();

        assert!(matches!(
            capture_package_snapshot(package.root()),
            Err(PackageSnapshotCaptureError::ManifestNotRegular(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn package_snapshot_capture_rejects_unrepresentable_entry_paths() {
        use std::os::unix::ffi::OsStringExt;

        let package = SnapshotFixture::new("non-utf8-path");
        package.write_bytes("veln.toml", b"manifest");
        let invalid_name = std::ffi::OsString::from_vec(b"invalid-\xff.veln".to_vec());
        fs::write(package.root().join(invalid_name), b"source").unwrap();

        assert!(matches!(
            capture_package_snapshot(package.root()),
            Err(PackageSnapshotCaptureError::UnrepresentablePath(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn package_snapshot_capture_excludes_unrepresentable_symbolic_links() {
        use std::os::unix::ffi::OsStringExt;
        use std::os::unix::fs::symlink;

        let package = SnapshotFixture::new("non-utf8-symlink");
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes("real.veln", b"real");
        let invalid_link = std::ffi::OsString::from_vec(b"invalid-\xff.veln".to_vec());
        symlink(package.path("real.veln"), package.root().join(invalid_link)).unwrap();

        let snapshot = capture_package_snapshot(package.root()).unwrap();
        let paths = snapshot
            .sources()
            .iter()
            .map(CapturedPackageSource::path)
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["real.veln"]);
    }

    #[cfg(unix)]
    #[test]
    fn package_snapshot_capture_excludes_unrepresentable_descendant_packages() {
        use std::os::unix::ffi::OsStringExt;

        let package = SnapshotFixture::new("non-utf8-descendant");
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes("outer.veln", b"outer");
        let invalid_dir = std::ffi::OsString::from_vec(b"dependency-\xff".to_vec());
        let nested_root = package.root().join(invalid_dir);
        fs::create_dir_all(&nested_root).unwrap();
        fs::write(nested_root.join("veln.toml"), b"nested manifest").unwrap();
        fs::write(nested_root.join("inner.veln"), b"inner").unwrap();

        let snapshot = capture_package_snapshot(package.root()).unwrap();
        let paths = snapshot
            .sources()
            .iter()
            .map(CapturedPackageSource::path)
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["outer.veln"]);
    }

    #[cfg(unix)]
    #[test]
    fn package_snapshot_capture_rejects_non_regular_distribution_sources() {
        let package = SnapshotFixture::new("non-regular-source");
        package.write_bytes("veln.toml", b"manifest");
        let fifo_path = package.path("fifo.veln");
        package.mkfifo("fifo.veln");

        assert!(matches!(
            capture_package_snapshot(package.root()),
            Err(PackageSnapshotCaptureError::SourceNotRegular(path)) if path == fifo_path
        ));
    }

    #[cfg(unix)]
    #[test]
    fn package_snapshot_capture_ignores_non_regular_non_sources() {
        let package = SnapshotFixture::new("non-regular-non-source");
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes("source.veln", b"source");
        package.mkfifo("fifo.txt");

        let snapshot = capture_package_snapshot(package.root()).unwrap();
        let paths = snapshot
            .sources()
            .iter()
            .map(CapturedPackageSource::path)
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["source.veln"]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn package_snapshot_capture_rejects_nonportable_source_paths() {
        let cases = [
            (
                "non-nfc",
                "cafe\u{301}.veln",
                PortableSourcePathError::NotNfc,
            ),
            (
                "control",
                "line\nfeed.veln",
                PortableSourcePathError::Control {
                    segment_index: 0,
                    character: '\n',
                },
            ),
            (
                "backslash",
                "dir\\source.veln",
                PortableSourcePathError::ForbiddenCharacter {
                    segment_index: 0,
                    character: '\\',
                },
            ),
            (
                "colon",
                "device:source.veln",
                PortableSourcePathError::ForbiddenCharacter {
                    segment_index: 0,
                    character: ':',
                },
            ),
            (
                "trailing-space",
                "dir /source.veln",
                PortableSourcePathError::TrailingSpace { segment_index: 0 },
            ),
            (
                "trailing-dot",
                "dir./source.veln",
                PortableSourcePathError::TrailingDot { segment_index: 0 },
            ),
            (
                "reserved-device",
                "NUL.veln",
                PortableSourcePathError::ReservedDevice { segment_index: 0 },
            ),
            (
                "reserved-device-stem-space",
                "NUL .veln",
                PortableSourcePathError::ReservedDevice { segment_index: 0 },
            ),
        ];

        for (label, path, reason) in cases {
            let package = SnapshotFixture::new(label);
            package.write_bytes("veln.toml", b"manifest");
            package.write_bytes(path, b"source");
            assert!(matches!(
                capture_package_snapshot(package.root()),
                Err(PackageSnapshotCaptureError::InvalidSourcePath {
                    path: actual_path,
                    reason: actual_reason,
                }) if actual_path == path && actual_reason == reason
            ));
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn package_snapshot_capture_reports_portable_failures_in_path_order() {
        let package = SnapshotFixture::new("portable-error-order");
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes("z:bad.veln", b"source");
        package.write_bytes("a:bad.veln", b"source");

        assert!(matches!(
            capture_package_snapshot(package.root()),
            Err(PackageSnapshotCaptureError::InvalidSourcePath { path, .. })
                if path == "a:bad.veln"
        ));
    }

    #[test]
    fn package_snapshot_capture_rejects_non_utf8_source_text() {
        let package = SnapshotFixture::new("non-utf8-source-text");
        package.write_bytes("veln.toml", b"manifest\xff");
        package.write_bytes("src/main.veln", b"valid\xff");

        assert!(matches!(
            capture_package_snapshot(package.root()),
            Err(PackageSnapshotCaptureError::InvalidSourceText {
                path,
                valid_up_to: 5,
            }) if path == "src/main.veln"
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn package_snapshot_capture_rejects_default_case_fold_collisions() {
        let package = SnapshotFixture::new("case-fold-collision");
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes("Straße.veln", b"first");
        package.write_bytes("STRASSE.veln", b"second");

        assert_eq!(
            capture_package_snapshot(package.root())
                .unwrap_err()
                .to_string(),
            "package snapshot source paths `STRASSE.veln` and `Straße.veln` collide after Unicode default case folding"
        );
        assert!(matches!(
            capture_package_snapshot(package.root()),
            Err(PackageSnapshotCaptureError::SourcePathCollision { first, second })
                if first == "STRASSE.veln" && second == "Straße.veln"
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn package_snapshot_capture_rejects_three_character_case_fold_collisions() {
        let package = SnapshotFixture::new("case-fold-three-collision");
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes("ffi.veln", b"first");
        package.write_bytes("ﬃ.veln", b"second");

        assert_eq!(
            capture_package_snapshot(package.root())
                .unwrap_err()
                .to_string(),
            "package snapshot source paths `ffi.veln` and `ﬃ.veln` collide after Unicode default case folding"
        );
        assert!(matches!(
            capture_package_snapshot(package.root()),
            Err(PackageSnapshotCaptureError::SourcePathCollision { first, second })
                if first == "ffi.veln" && second == "ﬃ.veln"
        ));
    }

    #[test]
    fn package_snapshot_capture_preserves_valid_unicode_and_source_bytes() {
        let package = SnapshotFixture::new("portable-exact-input");
        package.write_bytes("veln.toml", b"manifest\r\n\xff");
        package.write_bytes("src/café.veln", "λ\r\n".as_bytes());

        let snapshot = capture_package_snapshot(package.root()).unwrap();
        assert_eq!(snapshot.manifest_bytes(), b"manifest\r\n\xff");
        assert_eq!(snapshot.sources()[0].path(), "src/café.veln");
        assert_eq!(snapshot.sources()[0].bytes(), "λ\r\n".as_bytes());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn package_snapshot_capture_excludes_entries_before_portable_validation() {
        let package = SnapshotFixture::new("excluded-portability");
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes("kept.veln", b"kept");
        package.write_bytes("bad:/ignored.test.veln", b"\xff");
        package.write_bytes("bad./ignored_test.veln", b"\xff");
        package.write_bytes(".git/NUL.veln", b"\xff");
        package.write_bytes("dependency:/veln.toml", b"nested manifest");
        package.write_bytes("dependency:/NUL.veln", b"\xff");
        package.write_bytes_raw_relative(b"ignored-\xff.test.veln", b"\xff");
        package.write_bytes_raw_relative(b"ignored-\xff_test.veln", b"\xff");

        let snapshot = capture_package_snapshot(package.root()).unwrap();
        let paths = snapshot
            .sources()
            .iter()
            .map(CapturedPackageSource::path)
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["kept.veln"]);
    }

    #[cfg(unix)]
    #[test]
    fn unix_test_source_exclusion_uses_raw_path_bytes() {
        assert!(is_excluded_test_source_bytes(b"bad-\xff.test.veln"));
        assert!(is_excluded_test_source_bytes(b"bad-\xff_test.veln"));
        assert!(!is_excluded_test_source_bytes(b"bad-\xff.veln"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_test_source_exclusion_uses_raw_wide_code_units() {
        let mut companion = "bad-".encode_utf16().collect::<Vec<_>>();
        companion.push(0xD800);
        companion.extend(".test.veln".encode_utf16());
        assert!(is_excluded_test_source_wide(&companion));

        let mut integration = "bad-".encode_utf16().collect::<Vec<_>>();
        integration.push(0xD800);
        integration.extend("_test.veln".encode_utf16());
        assert!(is_excluded_test_source_wide(&integration));

        let mut retained = "bad-".encode_utf16().collect::<Vec<_>>();
        retained.push(0xD800);
        retained.extend(".veln".encode_utf16());
        assert!(!is_excluded_test_source_wide(&retained));
    }

    #[cfg(windows)]
    #[test]
    fn package_snapshot_capture_excludes_ill_formed_utf16_test_sources() {
        let package = SnapshotFixture::new("ill-formed-utf16-excluded");
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes("kept.veln", b"kept");
        package.write_bytes_raw_wide_relative(&ill_formed_wide_name(".test.veln"), b"\xff");
        package.write_bytes_raw_wide_relative(&ill_formed_wide_name("_test.veln"), b"\xff");

        let snapshot = capture_package_snapshot(package.root()).unwrap();
        let paths = snapshot
            .sources()
            .iter()
            .map(CapturedPackageSource::path)
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["kept.veln"]);
    }

    struct SnapshotFixture {
        root: PathBuf,
    }

    impl SnapshotFixture {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "veln-package-snapshot-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn root(&self) -> &Path {
            &self.root
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        fn write_bytes(&self, relative: &str, bytes: &[u8]) {
            let path = self.path(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, bytes).unwrap();
        }

        #[cfg(windows)]
        fn write_bytes_raw_wide_relative(&self, relative: &[u16], bytes: &[u8]) {
            use std::os::windows::ffi::OsStringExt;

            let path = self.root.join(std::ffi::OsString::from_wide(relative));
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, bytes).unwrap();
        }

        #[cfg(unix)]
        fn write_bytes_raw_relative(&self, relative: &[u8], bytes: &[u8]) {
            use std::os::unix::ffi::OsStrExt;

            let path = self.root.join(Path::new(OsStr::from_bytes(relative)));
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, bytes).unwrap();
        }

        #[cfg(unix)]
        fn mkfifo(&self, relative: &str) {
            let path = self.path(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let status = std::process::Command::new("mkfifo")
                .arg(&path)
                .status()
                .unwrap();
            assert!(status.success());
        }
    }

    #[cfg(windows)]
    fn ill_formed_wide_name(suffix: &str) -> Vec<u16> {
        let mut name = "ignored-".encode_utf16().collect::<Vec<_>>();
        name.push(0xD800);
        name.extend(suffix.encode_utf16());
        name
    }

    impl Drop for SnapshotFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
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
