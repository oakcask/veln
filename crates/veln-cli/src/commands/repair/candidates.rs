use std::fs;
use std::path::{Path, PathBuf};

use veln_diagnostics::{Diagnostic, JsonValue, parse_json_value};
use veln_source::{LineCol, SourceSpan};

use super::{APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE, APPLICATION_STATUS_UNAPPLIED};

#[derive(Clone, Debug)]
pub(super) struct RepairCandidate {
    pub(super) repair_id: String,
    pub(super) source_candidate_id: String,
    pub(super) name: String,
    pub(super) application_policy: String,
    pub(super) application_status: String,
    pub(super) edit_summary: String,
    pub(super) edits: Vec<RepairEdit>,
    pub(super) verification_command: Option<String>,
    pub(super) source: JsonValue,
    pub(super) input_repair_id: Option<String>,
    pub(super) requires_current_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RepairEdit {
    pub(super) file: String,
    pub(super) start: LineCol,
    pub(super) end: LineCol,
    pub(super) replacement: String,
}

impl RepairCandidate {
    pub(super) fn is_safe_unapplied(&self) -> bool {
        self.application_policy == APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE
            && self.application_status == APPLICATION_STATUS_UNAPPLIED
    }

    fn matches_id(&self, id: &str) -> bool {
        self.repair_id == id
            || self.source_candidate_id == id
            || self.input_repair_id.as_deref() == Some(id)
    }

    pub(super) fn blocking_obligations(&self) -> Vec<String> {
        object_string_array(&self.source, "blocking_obligations").unwrap_or_default()
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("repair_id", JsonValue::string(self.repair_id.clone())),
            (
                "source_candidate_id",
                JsonValue::string(self.source_candidate_id.clone()),
            ),
            ("name", JsonValue::string(self.name.clone())),
            (
                "application_policy",
                JsonValue::string(self.application_policy.clone()),
            ),
            (
                "application_status",
                JsonValue::string(self.application_status.clone()),
            ),
            ("edit_summary", JsonValue::string(self.edit_summary.clone())),
            (
                "edits",
                JsonValue::array(self.edits.iter().map(RepairEdit::to_json)),
            ),
            (
                "verification_command",
                self.verification_command
                    .as_ref()
                    .map_or(JsonValue::Null, JsonValue::string),
            ),
            ("source", self.source.clone()),
        ])
    }

    fn from_saved_advisory(index: usize, candidate: &JsonValue) -> Option<Self> {
        Self::from_advisory_with_freshness(index, candidate, CandidateFreshness::SavedInput)
    }

    fn from_advisory_with_freshness(
        index: usize,
        candidate: &JsonValue,
        freshness: CandidateFreshness,
    ) -> Option<Self> {
        Self::from_parts(
            index,
            CandidateParts {
                candidate,
                source_candidate_id: object_string(candidate, "candidate_id")?,
                input_repair_id: None,
                edits: replace_edits_from_edits(candidate)?,
                verification_command: object_value(candidate, "verification_hint")
                    .and_then(|hint| object_string(hint, "command")),
                source: candidate.clone(),
                requires_current_match: freshness.requires_current_match(),
            },
        )
    }

    fn from_saved_command(index: usize, candidate: &JsonValue) -> Option<Self> {
        Self::from_parts(
            index,
            CandidateParts {
                candidate,
                source_candidate_id: object_string(candidate, "source_candidate_id")?,
                input_repair_id: object_string(candidate, "repair_id"),
                edits: replace_edits_from_saved_command(candidate)?,
                verification_command: object_string(candidate, "verification_command"),
                source: object_value(candidate, "source")
                    .cloned()
                    .unwrap_or_else(|| candidate.clone()),
                requires_current_match: true,
            },
        )
    }

    fn from_parts(index: usize, parts: CandidateParts<'_>) -> Option<Self> {
        Some(Self {
            repair_id: format!("repair-{}", index + 1),
            source_candidate_id: parts.source_candidate_id.to_string(),
            name: object_string(parts.candidate, "name")
                .unwrap_or("")
                .to_string(),
            application_policy: object_string(parts.candidate, "application_policy")?.to_string(),
            application_status: object_string(parts.candidate, "application_status")?.to_string(),
            edit_summary: object_string(parts.candidate, "edit_summary")
                .unwrap_or("Replace source span")
                .to_string(),
            edits: parts.edits,
            verification_command: parts.verification_command.map(str::to_string),
            source: parts.source,
            input_repair_id: parts.input_repair_id.map(str::to_string),
            requires_current_match: parts.requires_current_match,
        })
    }
}

impl RepairEdit {
    pub(super) fn is_satisfy_suffix_deletion(&self) -> bool {
        self.replacement.is_empty()
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string("replace")),
            ("span", span_json(&self.file, self.start, self.end)),
            ("replacement", JsonValue::string(self.replacement.clone())),
        ])
    }
}

struct CandidateParts<'a> {
    candidate: &'a JsonValue,
    source_candidate_id: &'a str,
    input_repair_id: Option<&'a str>,
    edits: Vec<RepairEdit>,
    verification_command: Option<&'a str>,
    source: JsonValue,
    requires_current_match: bool,
}

#[derive(Clone, Copy)]
enum CandidateFreshness {
    CurrentAnalysis,
    SavedInput,
}

impl CandidateFreshness {
    fn requires_current_match(self) -> bool {
        matches!(self, Self::SavedInput)
    }
}

pub(super) enum CandidateIdMatch<'a> {
    Missing,
    Unique(&'a RepairCandidate),
    Ambiguous,
}

pub(super) fn find_candidate_by_id<'a>(
    candidates: &'a [RepairCandidate],
    id: &str,
) -> CandidateIdMatch<'a> {
    let mut matches = candidates
        .iter()
        .filter(|candidate| candidate.matches_id(id))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return CandidateIdMatch::Ambiguous;
    }
    matches
        .pop()
        .map_or(CandidateIdMatch::Missing, CandidateIdMatch::Unique)
}

pub(super) fn safe_unapplied_candidates(candidates: &[RepairCandidate]) -> Vec<&RepairCandidate> {
    candidates
        .iter()
        .filter(|candidate| candidate.is_safe_unapplied())
        .collect()
}

pub(super) fn split_repair_inputs(inputs: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>) {
    inputs
        .iter()
        .cloned()
        .partition(|input| !is_saved_candidate_input(input))
}

fn is_saved_candidate_input(input: &Path) -> bool {
    input
        .extension()
        .is_some_and(|extension| extension == "json")
}

pub(super) fn repair_candidates_from_diagnostics(
    diagnostics: &[Diagnostic],
) -> Vec<RepairCandidate> {
    let mut candidates = Vec::new();
    for diagnostic in diagnostics {
        collect_advisory_candidates_from_details(
            &diagnostic.details,
            &mut candidates,
            CandidateFreshness::CurrentAnalysis,
        );
    }
    candidates
}

pub(super) fn repair_candidates_from_saved_inputs(
    root: &Path,
    inputs: &[PathBuf],
) -> Result<Vec<RepairCandidate>, String> {
    let mut candidates = Vec::new();
    for input in inputs {
        let path = if input.is_absolute() {
            input.clone()
        } else {
            root.join(input)
        };
        let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let value = parse_json_value(&source).map_err(|error| {
            format!(
                "invalid saved repair candidate input `{}`: {error}",
                input.display()
            )
        })?;
        collect_saved_repair_candidates(&value, &mut candidates);
    }
    Ok(candidates)
}

fn collect_saved_repair_candidates(value: &JsonValue, candidates: &mut Vec<RepairCandidate>) {
    match value {
        JsonValue::Array(values) => {
            for value in values {
                collect_saved_repair_candidates(value, candidates);
            }
        }
        JsonValue::Object(_) => {
            if let Some(candidate) = RepairCandidate::from_saved_command(candidates.len(), value) {
                candidates.push(candidate);
                return;
            }
            if let Some(candidate) = RepairCandidate::from_saved_advisory(candidates.len(), value) {
                candidates.push(candidate);
                return;
            }
            if let Some(values) = object_array(value, "candidates") {
                for value in values {
                    collect_saved_repair_candidates(value, candidates);
                }
            }
            if let Some(values) = object_array(value, "diagnostics") {
                for value in values {
                    collect_saved_diagnostic_candidates(value, candidates);
                }
            }
        }
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Decimal(_)
        | JsonValue::String(_) => {}
    }
}

fn collect_saved_diagnostic_candidates(value: &JsonValue, candidates: &mut Vec<RepairCandidate>) {
    let Some(details) = object_value(value, "details") else {
        return;
    };
    collect_advisory_candidates_from_details(details, candidates, CandidateFreshness::SavedInput);
}

fn collect_advisory_candidates_from_details(
    details: &JsonValue,
    candidates: &mut Vec<RepairCandidate>,
    freshness: CandidateFreshness,
) {
    let Some(queries) = object_array(details, "candidate_queries") else {
        return;
    };
    for query in queries {
        let Some(query_candidates) = object_array(query, "candidates") else {
            continue;
        };
        for candidate in query_candidates {
            if let Some(candidate) = RepairCandidate::from_advisory_with_freshness(
                candidates.len(),
                candidate,
                freshness,
            ) {
                candidates.push(candidate);
            }
        }
    }
}

pub(super) fn has_current_applicable_match(
    selected: &RepairCandidate,
    current_candidates: &[RepairCandidate],
) -> bool {
    let mut matched_non_empty_edit = false;
    for edit in &selected.edits {
        if edit.is_satisfy_suffix_deletion() {
            if !selected.edits.iter().any(|other| {
                !other.is_satisfy_suffix_deletion()
                    && other.file == edit.file
                    && other.end.offset == edit.start.offset
            }) {
                return false;
            }
            continue;
        }
        let has_current_match = current_candidates.iter().any(|candidate| {
            candidate.is_safe_unapplied()
                && candidate.source_candidate_id == selected.source_candidate_id
                && candidate
                    .edits
                    .iter()
                    .any(|current_edit| current_edit == edit)
        });
        if !has_current_match {
            return false;
        }
        matched_non_empty_edit = true;
    }
    matched_non_empty_edit
}

fn replace_edits_from_edits(candidate: &JsonValue) -> Option<Vec<RepairEdit>> {
    let edits = object_array(candidate, "edits")?;
    if edits.is_empty() {
        return None;
    }
    edits.iter().map(replace_edit).collect()
}

fn replace_edits_from_saved_command(candidate: &JsonValue) -> Option<Vec<RepairEdit>> {
    if let Some(edits) = replace_edits_from_edits(candidate) {
        return Some(edits);
    }
    object_value(candidate, "edit")
        .and_then(replace_edit)
        .map(|edit| vec![edit])
}

fn replace_edit(edit: &JsonValue) -> Option<RepairEdit> {
    if object_string(edit, "kind")? != "replace" {
        return None;
    }
    let span = object_value(edit, "span").and_then(source_span)?;
    Some(RepairEdit {
        file: span.file.as_str().to_string(),
        start: span.start,
        end: span.end,
        replacement: object_string(edit, "replacement")?.to_string(),
    })
}

fn source_span(value: &JsonValue) -> Option<SourceSpan> {
    Some(SourceSpan {
        file: object_string(value, "file")?.into(),
        start: line_col(object_value(value, "start")?)?,
        end: line_col(object_value(value, "end")?)?,
    })
}

fn line_col(value: &JsonValue) -> Option<LineCol> {
    Some(LineCol {
        line: object_number(value, "line")? as usize,
        column: object_number(value, "column")? as usize,
        offset: object_number(value, "offset")? as usize,
    })
}

fn span_json(file: &str, start: LineCol, end: LineCol) -> JsonValue {
    JsonValue::object([
        ("file", JsonValue::string(file)),
        ("start", line_col_json(start)),
        ("end", line_col_json(end)),
    ])
}

fn line_col_json(line_col: LineCol) -> JsonValue {
    JsonValue::object([
        ("line", JsonValue::Number(line_col.line as i64)),
        ("column", JsonValue::Number(line_col.column as i64)),
        ("offset", JsonValue::Number(line_col.offset as i64)),
    ])
}

fn object_value<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find_map(|(entry_key, value)| (entry_key == key).then_some(value))
}

fn object_array<'a>(value: &'a JsonValue, key: &str) -> Option<&'a Vec<JsonValue>> {
    match object_value(value, key)? {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }
}

fn object_string_array(value: &JsonValue, key: &str) -> Option<Vec<String>> {
    object_array(value, key).map(|values| {
        values
            .iter()
            .filter_map(|value| match value {
                JsonValue::String(value) => Some(value.clone()),
                _ => None,
            })
            .collect()
    })
}

fn object_string<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    match object_value(value, key)? {
        JsonValue::String(value) => Some(value),
        _ => None,
    }
}

fn object_number(value: &JsonValue, key: &str) -> Option<i64> {
    match object_value(value, key)? {
        JsonValue::Number(value) => Some(*value),
        JsonValue::Decimal(value) => json_integer_token_value(value),
        _ => None,
    }
}

fn json_integer_token_value(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    let mut index = 0;
    if matches!(bytes.first(), Some(b'-')) {
        index = 1;
    }
    let first = bytes.get(index)?;
    match first {
        b'0' => index += 1,
        b'1'..=b'9' => {
            index += 1;
            while matches!(bytes.get(index), Some(b'0'..=b'9')) {
                index += 1;
            }
        }
        _ => return None,
    }
    if index != bytes.len() {
        return None;
    }
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(start: usize, end: usize) -> RepairCandidate {
        RepairCandidate {
            repair_id: "repair-1".to_string(),
            source_candidate_id: "symbol-1".to_string(),
            name: "value".to_string(),
            application_policy: APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE.to_string(),
            application_status: APPLICATION_STATUS_UNAPPLIED.to_string(),
            edit_summary: "Replace hole with `value`".to_string(),
            edits: vec![RepairEdit {
                file: "main.veln".to_string(),
                start: LineCol {
                    line: 1,
                    column: start + 1,
                    offset: start,
                },
                end: LineCol {
                    line: 1,
                    column: end + 1,
                    offset: end,
                },
                replacement: "value".to_string(),
            }],
            verification_command: None,
            source: JsonValue::Null,
            input_repair_id: None,
            requires_current_match: false,
        }
    }

    #[test]
    fn find_candidate_by_id_reports_ambiguous_source_candidate_ids() {
        let first = candidate(0, 6);
        let mut second = candidate(8, 14);
        second.repair_id = "repair-2".to_string();

        assert!(matches!(
            find_candidate_by_id(&[first, second], "symbol-1"),
            CandidateIdMatch::Ambiguous
        ));
    }

    #[test]
    fn repair_candidate_advisory_input_accepts_multiple_edits() {
        let span = span_json(
            "main.veln",
            LineCol {
                line: 2,
                column: 3,
                offset: 27,
            },
            LineCol {
                line: 2,
                column: 9,
                offset: 33,
            },
        );
        let candidate = JsonValue::object([
            ("candidate_id", JsonValue::string("symbol-1")),
            ("name", JsonValue::string("value")),
            (
                "application_policy",
                JsonValue::string(APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE),
            ),
            (
                "application_status",
                JsonValue::string(APPLICATION_STATUS_UNAPPLIED),
            ),
            (
                "edits",
                JsonValue::array([
                    JsonValue::object([
                        ("kind", JsonValue::string("replace")),
                        ("span", span.clone()),
                        ("replacement", JsonValue::string("value")),
                    ]),
                    JsonValue::object([
                        ("kind", JsonValue::string("replace")),
                        ("span", span),
                        ("replacement", JsonValue::string("other")),
                    ]),
                ]),
            ),
        ]);

        let candidate = RepairCandidate::from_advisory_with_freshness(
            0,
            &candidate,
            CandidateFreshness::CurrentAnalysis,
        )
        .expect("candidate should load");

        assert_eq!(candidate.edits.len(), 2);
    }

    #[test]
    fn saved_command_candidate_preserves_input_repair_id_for_selection() {
        let saved = JsonValue::object([
            ("repair_id", JsonValue::string("repair-7")),
            ("source_candidate_id", JsonValue::string("symbol-1")),
            ("name", JsonValue::string("value")),
            (
                "application_policy",
                JsonValue::string(APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE),
            ),
            (
                "application_status",
                JsonValue::string(APPLICATION_STATUS_UNAPPLIED),
            ),
            (
                "edit",
                JsonValue::object([
                    ("kind", JsonValue::string("replace")),
                    (
                        "span",
                        span_json(
                            "main.veln",
                            LineCol {
                                line: 2,
                                column: 3,
                                offset: 27,
                            },
                            LineCol {
                                line: 2,
                                column: 9,
                                offset: 33,
                            },
                        ),
                    ),
                    ("replacement", JsonValue::string("value")),
                ]),
            ),
        ]);
        let candidate = RepairCandidate::from_saved_command(0, &saved)
            .expect("saved command candidate should load");

        assert_eq!(candidate.repair_id, "repair-1");
        assert!(candidate.matches_id("repair-7"));
        assert!(candidate.requires_current_match);
        assert_eq!(candidate.edits.len(), 1);
    }

    #[test]
    fn saved_command_candidate_accepts_decimal_integer_span_tokens() {
        let saved = JsonValue::object([
            ("repair_id", JsonValue::string("repair-7")),
            ("source_candidate_id", JsonValue::string("symbol-1")),
            ("name", JsonValue::string("value")),
            (
                "application_policy",
                JsonValue::string(APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE),
            ),
            (
                "application_status",
                JsonValue::string(APPLICATION_STATUS_UNAPPLIED),
            ),
            (
                "edit",
                JsonValue::object([
                    ("kind", JsonValue::string("replace")),
                    (
                        "span",
                        JsonValue::object([
                            ("file", JsonValue::string("main.veln")),
                            (
                                "start",
                                JsonValue::object([
                                    ("line", JsonValue::Decimal("2".to_string())),
                                    ("column", JsonValue::Decimal("3".to_string())),
                                    ("offset", JsonValue::Decimal("27".to_string())),
                                ]),
                            ),
                            (
                                "end",
                                JsonValue::object([
                                    ("line", JsonValue::Decimal("2".to_string())),
                                    ("column", JsonValue::Decimal("9".to_string())),
                                    ("offset", JsonValue::Decimal("33".to_string())),
                                ]),
                            ),
                        ]),
                    ),
                    ("replacement", JsonValue::string("value")),
                ]),
            ),
        ]);

        let candidate = RepairCandidate::from_saved_command(0, &saved)
            .expect("saved command candidate should load");

        assert_eq!(candidate.edits[0].start.offset, 27);
        assert_eq!(candidate.edits[0].end.offset, 33);
    }

    #[test]
    fn saved_command_candidate_rejects_non_integer_decimal_span_tokens() {
        for token in ["1.0", "1e0"] {
            let saved = JsonValue::object([
                ("repair_id", JsonValue::string("repair-7")),
                ("source_candidate_id", JsonValue::string("symbol-1")),
                ("name", JsonValue::string("value")),
                (
                    "application_policy",
                    JsonValue::string(APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE),
                ),
                (
                    "application_status",
                    JsonValue::string(APPLICATION_STATUS_UNAPPLIED),
                ),
                (
                    "edit",
                    JsonValue::object([
                        ("kind", JsonValue::string("replace")),
                        (
                            "span",
                            JsonValue::object([
                                ("file", JsonValue::string("main.veln")),
                                (
                                    "start",
                                    JsonValue::object([
                                        ("line", JsonValue::Decimal(token.to_string())),
                                        ("column", JsonValue::Decimal("3".to_string())),
                                        ("offset", JsonValue::Decimal("27".to_string())),
                                    ]),
                                ),
                                (
                                    "end",
                                    JsonValue::object([
                                        ("line", JsonValue::Decimal("2".to_string())),
                                        ("column", JsonValue::Decimal("9".to_string())),
                                        ("offset", JsonValue::Decimal("33".to_string())),
                                    ]),
                                ),
                            ]),
                        ),
                        ("replacement", JsonValue::string("value")),
                    ]),
                ),
            ]);

            assert!(
                RepairCandidate::from_saved_command(0, &saved).is_none(),
                "non-integer decimal token {token} should not load"
            );
        }
    }
}
