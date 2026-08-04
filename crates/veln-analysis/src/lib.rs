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
    derive_source_module_path, load_surface_module, validate_manifest_dependencies,
    validate_manifest_exports,
};

#[cfg(test)]
mod tests {
    use std::thread;

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

    fn checked_diagnostic_json(project: Project) -> Vec<String> {
        analyze_project(project, DoctestMode::Exclude)
            .checked_diagnostics()
            .iter()
            .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
            .collect()
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
