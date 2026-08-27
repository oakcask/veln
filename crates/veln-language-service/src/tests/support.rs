use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use veln_project::{capture_package_snapshot, parse_manifest_text};
use veln_source::SourceSpan;

use super::*;

static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

    fn source(path: &str, text: &str) -> SourceFile {
        SourceFile::new(path, text)
    }

    fn query(
        sources: Vec<SourceFile>,
        source_path: &str,
        line: usize,
        column: usize,
    ) -> Option<NavigationResult> {
        navigate(
            &EffectiveProjectSnapshot::new(sources),
            SourcePosition {
                source: SourcePath::new(source_path),
                line,
                column,
            },
        )
    }

    fn assert_location(location: &NavigationLocation, path: &str, line: usize, column: usize) {
        assert_eq!(location.source, NavigationSource::Workspace);
        assert_eq!(location.span.file.as_str(), path);
        assert_eq!(
            (location.span.start.line, location.span.start.column),
            (line, column)
        );
    }

    fn locations(spans: &[SourceSpan]) -> Vec<(&str, usize, usize)> {
        spans
            .iter()
            .map(|span| (span.file.as_str(), span.start.line, span.start.column))
            .collect()
    }

    fn assert_rename_invalid_case(
        failure: RenameFailure,
        symbol_class: RenameNameClass,
        requested_name: &str,
        required_initial: RenameRequiredInitial,
    ) {
        assert_eq!(failure.code, "rename.invalid_case");
        assert_eq!(failure.symbol_class, symbol_class);
        assert_eq!(failure.requested_name, requested_name);
        assert_eq!(failure.required_initial, required_initial);
    }

    fn dependency_query(
        dependency: DirectDependencySnapshot,
        expression: &str,
    ) -> Option<NavigationResult> {
        let text =
            format!("use math from \"example/pkg\"\n\npub fn main() -> Int\n  {expression}\nend\n");
        navigate(
            &EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source("main.veln", &text)],
                vec![dependency],
            ),
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 10,
            },
        )
    }

    fn dependency_snapshot(
        identity: &str,
        sources: &[(&str, &str)],
        exports: impl IntoIterator<Item = &'static str>,
    ) -> DirectDependencySnapshot {
        let root = TempDependency::new(identity, sources);
        let identity = PackageIdentity::new(identity).unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let exports = exports
            .into_iter()
            .map(|export| format!("\"{export}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let manifest = parse_manifest_text(
            "veln.toml",
            &format!(
                "[package]\nname = \"{}\"\n\n[lib]\nexports = [{}]\n",
                identity.as_str(),
                exports,
            ),
        );
        DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest).unwrap()
    }

    fn standard_library_snapshot(
        sources: &[(&str, &str)],
        exports: impl IntoIterator<Item = &'static str>,
    ) -> DirectDependencySnapshot {
        let root = TempDependency::new("std", sources);
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let exports = exports
            .into_iter()
            .map(|export| format!("\"{export}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let manifest = parse_manifest_text(
            "veln.toml",
            &format!("[package]\nname = \"std\"\n\n[lib]\nexports = [{exports}]\n"),
        );
        DirectDependencySnapshot::from_validated_standard_library(snapshot, manifest).unwrap()
    }

    struct TempDependency {
        path: PathBuf,
    }

    impl TempDependency {
        fn new(identity: &str, sources: &[(&str, &str)]) -> Self {
            let id = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "veln-language-service-navigation-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            fs::write(
                path.join("veln.toml"),
                format!("[package]\nname = \"{identity}\"\n"),
            )
            .unwrap();
            for (relative, text) in sources {
                let source_path = path.join(relative);
                if let Some(parent) = source_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(source_path, text).unwrap();
            }
            Self { path }
        }
    }

    impl Drop for TempDependency {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
