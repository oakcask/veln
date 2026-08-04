//! Shared project analysis for Veln tools.

mod analysis;
mod diagnostics;
mod surface;

pub use analysis::{
    DoctestMode, ProjectAnalysis, ReachableEntryAnalysis, analyze_project,
    checked_project_diagnostics,
};
pub use diagnostics::parse_diagnostic_to_envelope;
pub use surface::{
    derive_source_module_path, load_embedded_standard_surface_module, load_surface_module,
    validate_manifest_dependencies, validate_manifest_exports,
};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use veln_diagnostics::{DiagnosticKind, JsonValue, Severity, diagnostic_to_json};
    use veln_project::Project;
    use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan};
    use veln_syntax::{
        ParseDiagnostic, ParseRepairCandidate, ParseRepairEdit, Recovery, RecoveryStrategy,
        UnexpectedToken,
    };

    use super::*;

    #[test]
    fn parse_diagnostic_conversion_preserves_complete_json_shape() {
        let diagnostic = ParseDiagnostic {
            id: "parse.expected_item",
            message: "expected a function or test declaration".to_string(),
            span: Some(span("main.veln", 2, 3, 4, 2, 4, 5)),
            parser_context: "module",
            unexpected: UnexpectedToken {
                kind: "At".to_string(),
                text: "@".to_string(),
            },
            expected: vec!["fn", "test"],
            recovery: Recovery {
                strategy: RecoveryStrategy::SynchronizeToAnchor,
                anchor: Some("fn".to_string()),
                dropped_token_count: 2,
            },
            repair_candidates: vec![ParseRepairCandidate {
                candidate_id: "parse.expected_item.replace".to_string(),
                name: "Replace token".to_string(),
                application_policy: "manual_review".to_string(),
                application_status: "available".to_string(),
                edit_summary: "Replace `@` with `fn`".to_string(),
                edits: vec![ParseRepairEdit {
                    span: span("main.veln", 2, 3, 4, 2, 4, 5),
                    replacement: "fn".to_string(),
                }],
            }],
        };

        let converted = parse_diagnostic_to_envelope(&diagnostic);

        assert_eq!(converted.kind, DiagnosticKind::Parse);
        assert_eq!(converted.severity, Severity::Error);
        assert_eq!(
            veln_diagnostics::diagnostic_to_json(&converted).to_json(),
            concat!(
                "{\"id\":\"parse.expected_item\",\"severity\":\"error\",\"kind\":\"parse\",",
                "\"message\":\"expected a function or test declaration\",",
                "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":2,\"column\":3,\"offset\":4},",
                "\"end\":{\"line\":2,\"column\":4,\"offset\":5}},",
                "\"details\":{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"module\",",
                "\"unexpected\":{\"kind\":\"At\",\"text\":\"@\"},\"expected\":[\"fn\",\"test\"],",
                "\"recovery\":{\"strategy\":\"synchronize_to_anchor\",\"anchor\":\"fn\",",
                "\"dropped_token_count\":2},\"candidate_queries\":[{\"query_id\":\"parse.expected_item\",",
                "\"candidates\":[{\"candidate_id\":\"parse.expected_item.replace\",",
                "\"name\":\"Replace token\",\"application_policy\":\"manual_review\",",
                "\"application_status\":\"available\",\"edit_summary\":\"Replace `@` with `fn`\",",
                "\"edits\":[{\"kind\":\"replace\",\"span\":{\"file\":\"main.veln\",",
                "\"start\":{\"line\":2,\"column\":3,\"offset\":4},",
                "\"end\":{\"line\":2,\"column\":4,\"offset\":5}},\"replacement\":\"fn\"}]}]}]},",
                "\"related\":[]}"
            )
        );
    }

    #[test]
    fn parse_diagnostic_conversion_preserves_contract_classification() {
        let diagnostic = diagnostic("contract_predicate", vec!["contract predicate"]);

        let converted = parse_diagnostic_to_envelope(&diagnostic);

        assert_eq!(converted.kind, DiagnosticKind::Contract);
        assert_eq!(
            converted.details.to_json(),
            concat!(
                "{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"contract_predicate\",",
                "\"unexpected\":{\"kind\":\"Invalid\",\"text\":\"?\"},",
                "\"expected\":[\"contract predicate\"],",
                "\"recovery\":{\"strategy\":\"none\",\"anchor\":null,\"dropped_token_count\":0}}"
            )
        );
    }

    #[test]
    fn integer_literal_conversion_preserves_related_note() {
        let diagnostic = diagnostic("integer_literal", vec!["decimal integer"]);

        let converted = parse_diagnostic_to_envelope(&diagnostic);

        assert_eq!(
            JsonValue::array(converted.related).to_json(),
            "[{\"message\":\"Accepted integer form: decimal integer.\"}]"
        );
    }

    #[test]
    fn shared_analysis_keeps_diagnostic_json_order_stable_across_projects() {
        let alpha = project(
            "src/alpha/shared.veln",
            concat!(
                "mod alpha.shared\n",
                "pub fn entry() -> Int\n",
                "  \"alpha-only\"\n",
                "end\n",
            ),
        );
        let beta = project(
            "src/beta/shared.veln",
            concat!(
                "mod beta.shared\n",
                "pub fn entry() -> Bool\n",
                "  1\n",
                "end\n",
            ),
        );
        let alpha_isolated = checked_diagnostic_json(alpha.clone());
        let beta_isolated = checked_diagnostic_json(beta.clone());

        assert_project_evidence(
            &alpha_isolated,
            "src/alpha/shared.veln",
            "alpha.shared",
            "expected `Int`, but found `String`",
        );
        assert_project_evidence(
            &beta_isolated,
            "src/beta/shared.veln",
            "beta.shared",
            "expected `Bool`, but found `Int`",
        );
        assert_no_project_leak(
            &alpha_isolated,
            "src/beta/shared.veln",
            "beta.shared",
            "expected `Bool`, but found `Int`",
        );
        assert_no_project_leak(
            &beta_isolated,
            "src/alpha/shared.veln",
            "alpha.shared",
            "expected `Int`, but found `String`",
        );

        for _ in 0..8 {
            assert_eq!(checked_diagnostic_json(alpha.clone()), alpha_isolated);
            assert_eq!(checked_diagnostic_json(beta.clone()), beta_isolated);
        }

        let handles = (0..16)
            .map(|index| {
                let project = if index % 2 == 0 {
                    alpha.clone()
                } else {
                    beta.clone()
                };
                thread::spawn(move || (index, checked_diagnostic_json(project)))
            })
            .collect::<Vec<_>>();

        for handle in handles {
            let (index, diagnostics) = handle.join().expect("analysis thread should not panic");
            if index % 2 == 0 {
                assert_eq!(diagnostics, alpha_isolated);
                assert_no_project_leak(
                    &diagnostics,
                    "src/beta/shared.veln",
                    "beta.shared",
                    "expected `Bool`, but found `Int`",
                );
            } else {
                assert_eq!(diagnostics, beta_isolated);
                assert_no_project_leak(
                    &diagnostics,
                    "src/alpha/shared.veln",
                    "alpha.shared",
                    "expected `Int`, but found `String`",
                );
            }
        }
    }

    #[test]
    fn rediscovered_project_analysis_uses_changed_source_text_and_manifest_data() {
        let temp = TempProject::new("analysis-rediscovery-isolation");
        temp.write(
            "src/main.veln",
            concat!("pub fn entry() -> Int\n", "  1\n", "end\n"),
        );
        temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]\n");

        let baseline = checked_discovered_diagnostic_json(&temp, &[]);

        assert!(baseline.is_empty(), "{baseline:#?}");

        temp.write(
            "src/main.veln",
            concat!("pub fn entry() -> Bool\n", "  1\n", "end\n"),
        );
        temp.write("veln.toml", "[lib]\nexports = [\"src/other.veln\"]\n");

        let changed = checked_discovered_diagnostic_json(&temp, &[]);

        assert_eq!(
            diagnostic_ids(&changed),
            ["manifest.missing_export", "type.mismatch"],
            "{changed:#?}"
        );
        assert!(
            changed
                .iter()
                .any(|diagnostic| diagnostic.contains("src/other.veln")),
            "{changed:#?}"
        );
        assert!(
            changed
                .iter()
                .any(|diagnostic| diagnostic.contains("expected `Bool`, but found `Int`")),
            "{changed:#?}"
        );

        temp.write(
            "src/main.veln",
            concat!("pub fn entry() -> Int\n", "  1\n", "end\n"),
        );
        temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]\n");

        let restored = checked_discovered_diagnostic_json(&temp, &[]);

        assert!(restored.is_empty(), "{restored:#?}");
    }

    #[test]
    fn shared_analysis_keeps_local_std_prefixed_application_modules_fresh() {
        let project = project(
            "std/helper.veln",
            concat!(
                "fn answer(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn entry() -> Int\n",
                "  answer(1)\n",
                "end\n",
            ),
        );

        let diagnostics = checked_diagnostic_json(project);

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }

    #[test]
    fn shared_analysis_keeps_embedded_standard_module_name_collisions_fresh() {
        let cache = crate::analysis::TestStandardEnvironmentCache::new();
        let project = project(
            "std/prelude.veln",
            concat!(
                "fn local_only(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn entry() -> Int\n",
                "  local_only(1)\n",
                "end\n",
            ),
        );

        let diagnostics = checked_diagnostic_json_with_cache(project, &cache);

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert_eq!(cache.standard_prepares(), 1);
        assert_eq!(cache.application_analyses(), 1);
    }

    #[test]
    fn shared_analysis_prepares_standard_once_and_rebuilds_each_application() {
        let cache = crate::analysis::TestStandardEnvironmentCache::new();
        let alpha = project(
            "src/shared.veln",
            concat!("pub fn entry() -> Int\n", "  1\n", "end\n",),
        );
        let beta = project(
            "std/helper.veln",
            concat!(
                "fn answer(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn entry() -> Bool\n",
                "  answer(1)\n",
                "end\n",
            ),
        );

        let alpha_expected = checked_diagnostic_json_with_cache(alpha.clone(), &cache);
        let beta_expected = checked_diagnostic_json_with_cache(beta.clone(), &cache);

        assert!(alpha_expected.is_empty(), "{alpha_expected:#?}");
        assert_eq!(diagnostic_ids(&beta_expected), ["type.mismatch"]);
        assert_eq!(cache.standard_prepares(), 1);
        assert_eq!(cache.application_analyses(), 2);

        let handles = thread::scope(|scope| {
            (0..12)
                .map(|index| {
                    let project = if index % 2 == 0 {
                        alpha.clone()
                    } else {
                        beta.clone()
                    };
                    let cache = &cache;
                    scope.spawn(move || (index, checked_diagnostic_json_with_cache(project, cache)))
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("analysis should not panic"))
                .collect::<Vec<_>>()
        });

        for (index, diagnostics) in handles {
            if index % 2 == 0 {
                assert_eq!(diagnostics, alpha_expected);
            } else {
                assert_eq!(diagnostics, beta_expected);
            }
        }
        assert_eq!(cache.standard_prepares(), 1);
        assert_eq!(cache.application_analyses(), 14);
    }

    fn checked_diagnostic_json(project: Project) -> Vec<String> {
        analyze_project(project, DoctestMode::Exclude)
            .checked_diagnostics()
            .iter()
            .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
            .collect()
    }

    fn checked_diagnostic_json_with_cache(
        project: Project,
        cache: &crate::analysis::TestStandardEnvironmentCache,
    ) -> Vec<String> {
        crate::analysis::analyze_project_with_test_standard_cache(
            project,
            DoctestMode::Exclude,
            cache,
        )
        .checked_diagnostics()
        .iter()
        .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
        .collect()
    }

    fn checked_discovered_diagnostic_json(temp: &TempProject, inputs: &[PathBuf]) -> Vec<String> {
        checked_diagnostic_json(
            Project::discover(temp.root().to_path_buf(), inputs)
                .expect("project discovery should succeed"),
        )
    }

    fn assert_project_evidence(
        diagnostics: &[String],
        source_path: &str,
        module_path: &str,
        type_message: &str,
    ) {
        assert_eq!(
            diagnostic_ids(diagnostics),
            ["module.source_mod", "type.mismatch"],
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains(source_path)),
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains(module_path)),
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains(type_message)),
            "{diagnostics:#?}"
        );
    }

    fn assert_no_project_leak(
        diagnostics: &[String],
        source_path: &str,
        module_path: &str,
        type_message: &str,
    ) {
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.contains(source_path)),
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.contains(module_path)),
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.contains(type_message)),
            "{diagnostics:#?}"
        );
    }

    fn diagnostic_ids(diagnostics: &[String]) -> Vec<&'static str> {
        diagnostics
            .iter()
            .map(|diagnostic| {
                if diagnostic.contains("\"id\":\"module.source_mod\"") {
                    "module.source_mod"
                } else if diagnostic.contains("\"id\":\"manifest.missing_export\"") {
                    "manifest.missing_export"
                } else if diagnostic.contains("\"id\":\"type.mismatch\"") {
                    "type.mismatch"
                } else {
                    "unexpected"
                }
            })
            .collect()
    }

    fn project(path: &str, text: &str) -> Project {
        Project {
            root: ".".into(),
            files: vec![SourceFile::new(path, text)],
            manifest: None,
        }
    }

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after Unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "veln-analysis-{name}-{}-{unique}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("temporary project root should be created");
            Self { root }
        }

        fn root(&self) -> &Path {
            &self.root
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("temporary project parent should be created");
            }
            fs::write(path, contents).expect("temporary project file should be written");
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn diagnostic(parser_context: &'static str, expected: Vec<&'static str>) -> ParseDiagnostic {
        ParseDiagnostic {
            id: "parse.invalid",
            message: "invalid syntax".to_string(),
            span: None,
            parser_context,
            unexpected: UnexpectedToken {
                kind: "Invalid".to_string(),
                text: "?".to_string(),
            },
            expected,
            recovery: Recovery {
                strategy: RecoveryStrategy::None,
                anchor: None,
                dropped_token_count: 0,
            },
            repair_candidates: Vec::new(),
        }
    }

    fn span(
        file: &str,
        start_line: usize,
        start_column: usize,
        start_offset: usize,
        end_line: usize,
        end_column: usize,
        end_offset: usize,
    ) -> SourceSpan {
        SourceSpan {
            file: SourcePath::new(file),
            start: LineCol {
                line: start_line,
                column: start_column,
                offset: start_offset,
            },
            end: LineCol {
                line: end_line,
                column: end_column,
                offset: end_offset,
            },
        }
    }
}
