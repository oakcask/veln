use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use veln_project::{CapturedPackageSnapshot, PackageIdentity};

const URI_PREFIX: &str = "veln-pkg:///";

/// A canonical location for one source retained by a package snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualSourceEntry {
    uri: String,
    package_index: usize,
    source_index: usize,
}

impl VirtualSourceEntry {
    /// Returns the canonical `veln-pkg:` URI.
    pub fn uri(&self) -> &str {
        &self.uri
    }
}

/// An immutable catalog of captured package distribution sources.
#[derive(Clone, Debug)]
pub struct VirtualSourceCatalog {
    packages: Vec<(PackageIdentity, CapturedPackageSnapshot)>,
    entries: Vec<VirtualSourceEntry>,
    by_uri: BTreeMap<String, usize>,
    by_source: Vec<Vec<usize>>,
}

impl VirtualSourceCatalog {
    /// Builds canonical locations for every source in the supplied snapshots.
    pub fn new(
        packages: impl IntoIterator<Item = (PackageIdentity, CapturedPackageSnapshot)>,
    ) -> Result<Self, VirtualSourceCatalogError> {
        let packages = packages.into_iter().collect::<Vec<_>>();
        let mut entries = Vec::new();
        let mut by_uri = BTreeMap::new();
        let mut by_source = packages
            .iter()
            .map(|(_, snapshot)| Vec::with_capacity(snapshot.sources().len()))
            .collect::<Vec<_>>();

        for (package_index, (identity, snapshot)) in packages.iter().enumerate() {
            for (source_index, source) in snapshot.sources().iter().enumerate() {
                let uri = canonical_uri(identity, snapshot.digest(), source.path());
                let entry_index = entries.len();
                if by_uri.insert(uri.clone(), entry_index).is_some() {
                    return Err(VirtualSourceCatalogError::DuplicateUri(uri));
                }
                entries.push(VirtualSourceEntry {
                    uri,
                    package_index,
                    source_index,
                });
                by_source[package_index].push(entry_index);
            }
        }

        Ok(Self {
            packages,
            entries,
            by_uri,
            by_source,
        })
    }

    /// Lists every retained distribution source with its canonical URI.
    pub fn entries(&self) -> impl ExactSizeIterator<Item = &VirtualSourceEntry> {
        self.entries.iter()
    }

    /// Returns the retained entry for one package source by capture index.
    pub fn entry_for_source(
        &self,
        package_index: usize,
        source_index: usize,
    ) -> Option<&VirtualSourceEntry> {
        self.by_source
            .get(package_index)?
            .get(source_index)
            .and_then(|entry_index| self.entries.get(*entry_index))
    }

    /// Resolves an exact canonical URI to the captured source bytes.
    ///
    /// This lookup does not parse, decode, normalize, or access the filesystem.
    pub fn resolve(&self, uri: &str) -> Option<&[u8]> {
        let entry = self.entries.get(*self.by_uri.get(uri)?)?;
        self.packages
            .get(entry.package_index)?
            .1
            .sources()
            .get(entry.source_index)
            .map(|source| source.bytes())
    }
}

/// A set of captured snapshots cannot form an unambiguous catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VirtualSourceCatalogError {
    /// Two inputs produced the same canonical source URI.
    DuplicateUri(String),
}

impl fmt::Display for VirtualSourceCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateUri(uri) => write!(formatter, "duplicate virtual source URI `{uri}`"),
        }
    }
}

impl Error for VirtualSourceCatalogError {}

fn canonical_uri(identity: &PackageIdentity, digest: &str, source_path: &str) -> String {
    let mut uri = String::from(URI_PREFIX);
    encode_segment(identity.as_str(), &mut uri);
    uri.push_str("/snapshot/");
    uri.push_str(digest);
    uri.push('/');
    for (index, segment) in source_path.split('/').enumerate() {
        if index > 0 {
            uri.push('/');
        }
        encode_segment(segment, &mut uri);
    }
    uri
}

fn encode_segment(segment: &str, output: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            output.push(char::from(byte));
        } else {
            output.push('%');
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use veln_project::capture_package_snapshot;

    use super::*;

    static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn canonical_uris_round_trip_encoded_identity_and_source_segments() {
        let fixtures = [
            (
                "owner/package",
                "src/main file.veln",
                b"main\r\n".as_slice(),
                "owner%2Fpackage",
                "src/main%20file.veln",
            ),
            (
                "\u{5de5}\u{5177}/\u{5305}",
                "\u{30e9}\u{30a4}\u{30d6}\u{30e9}\u{30ea}/\u{03bb}.veln",
                "\u{03bb}\n".as_bytes(),
                "%E5%B7%A5%E5%85%B7%2F%E5%8C%85",
                "%E3%83%A9%E3%82%A4%E3%83%96%E3%83%A9%E3%83%AA/%CE%BB.veln",
            ),
        ];

        for (identity, path, bytes, encoded_identity, encoded_path) in fixtures {
            let root = TempPackage::new(&[(path, bytes)]);
            let snapshot = capture_package_snapshot(root.path()).unwrap();
            let digest = snapshot.digest().to_string();
            let catalog =
                VirtualSourceCatalog::new([(PackageIdentity::new(identity).unwrap(), snapshot)])
                    .unwrap();
            let entry = catalog.entries().next().unwrap();

            assert_eq!(
                entry.uri(),
                format!("veln-pkg:///{encoded_identity}/snapshot/{digest}/{encoded_path}")
            );
            assert_eq!(catalog.resolve(entry.uri()), Some(bytes));
        }
    }

    #[test]
    fn equivalent_path_vendor_and_mirror_snapshots_share_virtual_uris() {
        let sources = [
            ("src/main file.veln", b"main\r\n".as_slice()),
            ("src/nested/helper.veln", b"helper\n".as_slice()),
        ];
        let path = TempPackage::new(&sources);
        let vendor = TempPackage::new(&sources);
        let mirror = TempPackage::new(&sources);
        let identity = PackageIdentity::new("owner/package").unwrap();
        let catalogs = [path, vendor, mirror].map(|package| {
            VirtualSourceCatalog::new([(
                identity.clone(),
                capture_package_snapshot(package.path()).unwrap(),
            )])
            .unwrap()
        });
        let entries = catalogs.each_ref().map(|catalog| {
            catalog
                .entries()
                .map(|entry| {
                    (
                        entry.uri().to_string(),
                        catalog.resolve(entry.uri()).unwrap().to_vec(),
                    )
                })
                .collect::<Vec<_>>()
        });

        assert_eq!(entries[0], entries[1]);
        assert_eq!(entries[0], entries[2]);
        assert_eq!(entries[0].len(), 2);
        assert!(entries[0][0].0.contains("owner%2Fpackage"));
        assert!(entries[0][0].0.ends_with("/src/main%20file.veln"));
        assert_eq!(entries[0][0].1, b"main\r\n");
        assert!(entries[0][1].0.ends_with("/src/nested/helper.veln"));
        assert_eq!(entries[0][1].1, b"helper\n");
    }

    #[test]
    fn changed_snapshot_digest_changes_every_source_uri() {
        let root = TempPackage::new(&[("main.veln", b"before\n")]);
        let identity = PackageIdentity::new("package").unwrap();
        let before = capture_package_snapshot(root.path()).unwrap();
        fs::write(root.path().join("main.veln"), b"after\n").unwrap();
        let after = capture_package_snapshot(root.path()).unwrap();
        let before_catalog = VirtualSourceCatalog::new([(identity.clone(), before)]).unwrap();
        let after_catalog = VirtualSourceCatalog::new([(identity, after)]).unwrap();

        assert_ne!(
            before_catalog.entries().next().unwrap().uri(),
            after_catalog.entries().next().unwrap().uri()
        );
    }

    #[test]
    fn changed_manifest_digest_changes_every_source_uri() {
        let root = TempPackage::new(&[("main.veln", b"unchanged\n")]);
        let identity = PackageIdentity::new("package").unwrap();
        let before = capture_package_snapshot(root.path()).unwrap();
        fs::write(
            root.path().join("veln.toml"),
            b"[package]\nname = \"fixture\"\n\n",
        )
        .unwrap();
        let after = capture_package_snapshot(root.path()).unwrap();
        let before_catalog = VirtualSourceCatalog::new([(identity.clone(), before)]).unwrap();
        let after_catalog = VirtualSourceCatalog::new([(identity, after)]).unwrap();

        assert_ne!(
            before_catalog.entries().next().unwrap().uri(),
            after_catalog.entries().next().unwrap().uri()
        );
    }

    #[test]
    fn resolver_rejects_every_noncanonical_or_malformed_class() {
        let root = TempPackage::new(&[("dir/main.veln", b"main\n")]);
        let snapshot = capture_package_snapshot(root.path()).unwrap();
        let digest = snapshot.digest().to_string();
        let catalog =
            VirtualSourceCatalog::new([(PackageIdentity::new("owner/package").unwrap(), snapshot)])
                .unwrap();
        let canonical = catalog.entries().next().unwrap().uri().to_string();
        let invalid = [
            (
                "non-lowercase scheme",
                canonical.replacen("veln-pkg", "VELN-pkg", 1),
            ),
            (
                "nonempty authority",
                canonical.replacen(":///", "://host/", 1),
            ),
            ("userinfo", canonical.replacen(":///", "://user@host/", 1)),
            ("host", canonical.replacen(":///", "://host/", 1)),
            ("port", canonical.replacen(":///", "://host:80/", 1)),
            ("query", format!("{canonical}?read=true")),
            ("fragment", format!("{canonical}#source")),
            (
                "encoded unreserved",
                canonical.replacen("owner", "%6Fwner", 1),
            ),
            (
                "lowercase escape digits",
                canonical.replacen("%2F", "%2f", 1),
            ),
            (
                "decoded package separator",
                canonical.replacen("owner%2Fpackage", "owner/package", 1),
            ),
            (
                "encoded source separator",
                canonical.replacen("dir/main", "dir%2Fmain", 1),
            ),
            (
                "empty source segment",
                canonical.replacen("dir/main", "dir//main", 1),
            ),
            (
                "dot source segment",
                canonical.replacen("dir/main", "dir/./main", 1),
            ),
            (
                "dot-dot source segment",
                canonical.replacen("dir/main", "dir/../main", 1),
            ),
            (
                "encoded dot segment",
                canonical.replacen("dir/main", "dir/%2E/main", 1),
            ),
            ("malformed short escape", canonical.replacen("%2F", "%2", 1)),
            (
                "malformed escape digit",
                canonical.replacen("%2F", "%G0", 1),
            ),
            ("malformed UTF-8", canonical.replacen("owner", "%FFwner", 1)),
            ("short digest", canonical.replace(&digest, &digest[..63])),
            (
                "uppercase digest",
                canonical.replace(&digest, &digest.to_ascii_uppercase()),
            ),
            (
                "nonhex digest",
                canonical.replace(&digest, &format!("g{}", &digest[1..])),
            ),
        ];

        for (class, uri) in invalid {
            assert_eq!(catalog.resolve(&uri), None, "accepted {class}: {uri}");
        }
    }

    #[test]
    fn resolver_uses_exact_identity_digest_path_and_captured_bytes() {
        let first =
            TempPackage::new(&[("a.veln", b"\xEF\xBB\xBFa\r\n"), ("nested/b.veln", b"b\n")]);
        let second = TempPackage::new(&[("other.veln", b"other\n")]);
        let first_snapshot = capture_package_snapshot(first.path()).unwrap();
        let first_digest = first_snapshot.digest().to_string();
        let second_snapshot = capture_package_snapshot(second.path()).unwrap();
        let second_digest = second_snapshot.digest().to_string();
        let first_identity = PackageIdentity::new("first").unwrap();
        let second_identity = PackageIdentity::new("second").unwrap();
        let expected = [
            (&first_identity, &first_snapshot),
            (&second_identity, &second_snapshot),
        ]
        .into_iter()
        .flat_map(|(identity, snapshot)| {
            snapshot.sources().iter().map(|source| {
                (
                    canonical_uri(identity, snapshot.digest(), source.path()),
                    source.bytes().to_vec(),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
        let catalog = VirtualSourceCatalog::new([
            (first_identity, first_snapshot),
            (second_identity, second_snapshot),
        ])
        .unwrap();

        let listed = catalog.entries().collect::<Vec<_>>();
        let resolvable = listed
            .iter()
            .map(|entry| {
                (
                    entry.uri().to_string(),
                    catalog.resolve(entry.uri()).unwrap().to_vec(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(resolvable, expected);

        let canonical = listed
            .iter()
            .find(|entry| entry.uri().ends_with("/a.veln"))
            .unwrap()
            .uri();
        let mismatches = [
            canonical.replacen("/first/", "/unknown/", 1),
            canonical.replace(&first_digest, &second_digest),
            canonical.replacen("/a.veln", "/missing.veln", 1),
        ];
        for mismatch in mismatches {
            assert_eq!(catalog.resolve(&mismatch), None);
        }
        assert_eq!(
            catalog.resolve(canonical),
            Some(b"\xEF\xBB\xBFa\r\n".as_slice())
        );
    }

    #[test]
    fn source_entry_lookup_keeps_dense_index_for_many_sources() {
        let source_count = 20_001;
        let sources = (0..source_count)
            .map(|index| {
                (
                    format!("src/file-{index:05}.veln"),
                    format!("pub fn value_{index}() -> Int\n  {index}\nend\n").into_bytes(),
                )
            })
            .collect::<Vec<_>>();
        let root = TempPackage::new_owned(&sources);
        let snapshot = capture_package_snapshot(root.path()).unwrap();
        let catalog =
            VirtualSourceCatalog::new([(PackageIdentity::new("owner/package").unwrap(), snapshot)])
                .unwrap();

        assert_eq!(catalog.by_source.len(), 1);
        assert_eq!(catalog.by_source[0].len(), source_count);
        for source_index in 0..source_count {
            let entry = catalog.entry_for_source(0, source_index).unwrap();
            assert_eq!(entry.source_index, source_index);
            assert_eq!(
                catalog.entries[catalog.by_source[0][source_index]].uri(),
                entry.uri()
            );
        }
    }

    #[test]
    fn duplicate_canonical_sources_are_rejected() {
        let root = TempPackage::new(&[("main.veln", b"main\n")]);
        let snapshot = capture_package_snapshot(root.path()).unwrap();
        let result = VirtualSourceCatalog::new([
            (PackageIdentity::new("package").unwrap(), snapshot.clone()),
            (PackageIdentity::new("package").unwrap(), snapshot),
        ]);

        assert!(matches!(
            result,
            Err(VirtualSourceCatalogError::DuplicateUri(_))
        ));
    }

    struct TempPackage {
        root: PathBuf,
    }

    impl TempPackage {
        fn new(sources: &[(&str, &[u8])]) -> Self {
            Self::write_sources(sources.iter().map(|(path, bytes)| (*path, *bytes)))
        }

        fn new_owned(sources: &[(String, Vec<u8>)]) -> Self {
            Self::write_sources(
                sources
                    .iter()
                    .map(|(path, bytes)| (path.as_str(), &bytes[..])),
            )
        }

        fn write_sources<'a>(sources: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Self {
            let root = loop {
                let sequence = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
                let candidate = std::env::temp_dir().join(format!(
                    "veln-language-service-{}-{sequence}",
                    std::process::id()
                ));
                match fs::create_dir(&candidate) {
                    Ok(()) => break candidate,
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("failed to create temp package root: {error}"),
                }
            };
            fs::write(root.join("veln.toml"), b"[package]\nname = \"fixture\"\n").unwrap();
            for (path, bytes) in sources {
                let path = root.join(path);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(path, bytes).unwrap();
            }
            Self { root }
        }

        fn path(&self) -> &Path {
            &self.root
        }
    }

    impl Drop for TempPackage {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}
