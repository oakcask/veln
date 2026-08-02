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

    #[test]
    fn http2_client_service_ordinary_failures_close_retained_stream() {
        let connection = stdlib_source("http2/connection.veln");
        assert!(connection.contains(
            "Ok(Err(failure)) => client_fail_close(stream, client_connection_task_failure(completed, failure))"
        ));
        assert!(connection
            .contains("Err(_) => client_fail_close(stream, Http2ClientServiceJoinFailure(completed, \"failed\"))"));
        assert!(connection
            .contains("Err(reason) => client_fail_close(stream, Http2ClientServiceCallbackFailure(completed, reason))"));

        let cleanup = function_body(connection, "client_fail_close");
        let close_offset = cleanup
            .find("net::close_stream(stream)")
            .expect("client failure cleanup should close the retained stream");
        let failure_offset = cleanup
            .find("Err(failure)")
            .expect("client failure cleanup should return the original failure");
        assert!(
            close_offset < failure_offset,
            "client failure cleanup should close before returning the ordinary failure"
        );
    }

    fn stdlib_source(path: &str) -> &'static str {
        package_bundle()
            .files
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| panic!("standard library source `{path}` should be bundled"))
            .text
    }

    fn function_body<'a>(source: &'a str, name: &str) -> &'a str {
        let signature = format!("fn {name}");
        let start = source
            .find(&signature)
            .unwrap_or_else(|| panic!("function `{name}` should exist"));
        let body_start = source[start..]
            .find('\n')
            .map(|offset| start + offset + 1)
            .expect("function signature should end with a newline");
        let body_end = source[body_start..]
            .find("\nend")
            .map(|offset| body_start + offset)
            .expect("function should end with `end`");
        &source[body_start..body_end]
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
