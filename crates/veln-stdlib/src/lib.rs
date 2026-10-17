//! Embedded Veln standard library package.

pub const PACKAGE_NAME: &str = "std";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdlibFile {
    pub path: &'static str,
    pub text: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdlibLoweredFile {
    pub path: &'static str,
    pub module: &'static [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdlibPackage {
    pub manifest: &'static str,
    pub exports: &'static [&'static str],
    pub files: &'static [StdlibFile],
    pub lowered_files: &'static [StdlibLoweredFile],
}

include!(concat!(env!("OUT_DIR"), "/stdlib_bundle.rs"));

pub const fn package_bundle() -> StdlibPackage {
    StdlibPackage {
        manifest: MANIFEST,
        exports: EXPORTS,
        files: FILES,
        lowered_files: LOWERED_FILES,
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
                "host_effects.veln",
                "prelude.veln",
                "net.veln",
                "transport.veln",
                "transport/net.veln",
                "transport/network.veln",
                "observe.veln",
                "trace.veln",
                "traced.veln",
                "log.veln",
                "http2/frame.veln",
                "http2/diagnostic.veln",
                "http2/hpack.veln",
                "http2/hpack/diagnostic.veln",
                "http2/core.veln",
                "http2/connection.veln",
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
        assert!(paths.iter().all(|path| !path.ends_with(".test.veln")));
        assert_eq!(
            paths.iter().copied().collect::<BTreeSet<_>>().len(),
            paths.len()
        );
        assert_eq!(
            package
                .lowered_files
                .iter()
                .map(|file| file.path)
                .collect::<Vec<_>>(),
            paths
        );
    }

    #[test]
    fn distribution_source_collection_excludes_test_filename_classes() {
        let root = std::env::temp_dir().join(format!(
            "veln-stdlib-source-collection-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("nested")).expect("test source root should be created");
        fs::write(root.join("main.veln"), "pub fn main() -> ()\n  ()\nend\n")
            .expect("production source should be written");
        fs::write(
            root.join("main_test.veln"),
            "test integration() -> ()\nend\n",
        )
        .expect("integration test source should be written");
        fs::write(
            root.join("nested").join("main.test.veln"),
            "test companion() -> ()\nend\n",
        )
        .expect("companion test source should be written");

        let mut paths = Vec::new();
        collect_distribution_sources(&root, Path::new(""), &mut paths);
        let _ = fs::remove_dir_all(&root);
        paths.sort();

        assert_eq!(paths, vec!["main.veln"]);
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
                if is_distribution_source(&path) {
                    paths.push(path);
                }
            }
        }
    }

    fn is_distribution_source(path: &str) -> bool {
        path.ends_with(".veln") && !path.ends_with("_test.veln") && !path.ends_with(".test.veln")
    }
}
