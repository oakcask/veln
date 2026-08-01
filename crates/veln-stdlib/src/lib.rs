//! Embedded Veln standard library package.

pub const PACKAGE_NAME: &str = "std";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdlibFile {
    pub path: &'static str,
    pub text: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdlibPackage {
    pub manifest: &'static str,
    pub exports: &'static [&'static str],
    pub files: &'static [StdlibFile],
}

include!(concat!(env!("OUT_DIR"), "/stdlib_bundle.rs"));

pub const fn package_bundle() -> StdlibPackage {
    StdlibPackage {
        manifest: MANIFEST,
        exports: EXPORTS,
        files: FILES,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn bundle_has_standard_package_identity_and_public_modules() {
        let package = package_bundle();
        assert!(package.manifest.contains("name = \"std\""));
        assert_eq!(
            package.exports,
            [
                "prelude.veln",
                "transport.veln",
                "transport/net.veln",
                "http2/frame.veln",
                "http2/diagnostic.veln",
                "http2/hpack.veln",
                "http2/hpack/diagnostic.veln",
                "http2/core.veln",
            ]
        );
    }

    #[test]
    fn bundle_contains_each_distribution_source_once_and_no_tests() {
        let package = package_bundle();
        let paths = package
            .files
            .iter()
            .map(|file| file.path)
            .collect::<Vec<_>>();
        let mut expected = Vec::new();
        collect_distribution_sources(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("veln"),
            Path::new(""),
            &mut expected,
        );
        expected.sort();
        assert_eq!(paths, expected);
        assert!(paths.iter().all(|path| !path.ends_with("_test.veln")));
        assert_eq!(
            paths.iter().copied().collect::<BTreeSet<_>>().len(),
            paths.len()
        );
    }

    #[test]
    fn bundle_contains_private_foundation_modules_without_exporting_them() {
        let package = package_bundle();
        assert!(package.files.iter().any(|file| file.path == "bytes.veln"));
        assert!(
            package
                .files
                .iter()
                .any(|file| file.path == "diagnostic.veln")
        );
        assert!(!package.exports.contains(&"bytes.veln"));
        assert!(!package.exports.contains(&"diagnostic.veln"));
    }

    fn collect_distribution_sources(root: &Path, relative: &Path, paths: &mut Vec<String>) {
        let directory = root.join(relative);
        for entry in fs::read_dir(directory).expect("standard package directory should be readable")
        {
            let entry = entry.expect("standard package entry should be readable");
            let entry_relative = relative.join(entry.file_name());
            if entry.path().is_dir() && entry.file_name() == "target" {
                continue;
            }
            if entry.path().is_dir() {
                collect_distribution_sources(root, &entry_relative, paths);
            } else {
                let path = entry_relative.to_string_lossy().replace('\\', "/");
                if path.ends_with(".veln") && !path.ends_with("_test.veln") {
                    paths.push(path);
                }
            }
        }
    }
}
