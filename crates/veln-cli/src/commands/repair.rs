use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use veln_diagnostics::{Diagnostic, JsonValue, diagnostic_to_json};
use veln_project::Project;
use veln_source::{LineCol, SourceSpan};

use crate::commands::check::check_diagnostics;
use crate::diagnostics::{has_error, tool_info};

const APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE: &str = "safe_repair_candidate";
const APPLICATION_STATUS_UNAPPLIED: &str = "unapplied";

pub(crate) fn repair(
    json: bool,
    apply: bool,
    candidate_id: Option<String>,
    inputs: Vec<PathBuf>,
) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let project = Project::discover(root, &inputs).map_err(|error| error.to_string())?;
    let mut check_project = project.clone();
    let diagnostics = check_diagnostics(&mut check_project);
    let candidates = repair_candidates(&diagnostics);
    let outcome = if apply {
        apply_candidate(project, inputs, candidates, candidate_id)?
    } else {
        RepairOutcome::preview(candidates, candidate_id)
    };

    if json {
        println!("{}", outcome.to_json());
    } else {
        print_human(&outcome);
    }

    Ok(if outcome.exit_success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

#[derive(Clone, Debug)]
struct RepairCandidate {
    repair_id: String,
    source_candidate_id: String,
    name: String,
    application_policy: String,
    application_status: String,
    edit_summary: String,
    file: String,
    start: LineCol,
    end: LineCol,
    replacement: String,
    verification_command: Option<String>,
    source: JsonValue,
}

#[derive(Clone, Debug)]
struct RepairOutcome {
    mode: &'static str,
    status: &'static str,
    candidates: Vec<RepairCandidate>,
    selected: Option<RepairCandidate>,
    applied_edits: Vec<AppliedEdit>,
    verification: Verification,
    refusal_reason: Option<String>,
}

#[derive(Clone, Debug)]
struct AppliedEdit {
    file: String,
    start: LineCol,
    end: LineCol,
    replacement: String,
}

#[derive(Clone, Debug)]
struct Verification {
    status: &'static str,
    command: Option<String>,
    diagnostics: Vec<Diagnostic>,
}

impl RepairOutcome {
    fn preview(candidates: Vec<RepairCandidate>, requested_candidate_id: Option<String>) -> Self {
        let selected = requested_candidate_id
            .as_deref()
            .and_then(|id| unique_candidate(&candidates, id).ok().flatten().cloned());
        Self {
            mode: "preview",
            status: "preview",
            candidates,
            selected,
            applied_edits: Vec::new(),
            verification: Verification {
                status: "not_run",
                command: None,
                diagnostics: Vec::new(),
            },
            refusal_reason: None,
        }
    }

    fn refused(
        candidates: Vec<RepairCandidate>,
        selected: Option<RepairCandidate>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            mode: "apply",
            status: "refused",
            candidates,
            selected,
            applied_edits: Vec::new(),
            verification: Verification {
                status: "not_run",
                command: None,
                diagnostics: Vec::new(),
            },
            refusal_reason: Some(reason.into()),
        }
    }

    fn exit_success(&self) -> bool {
        self.status != "refused"
    }

    fn to_json(&self) -> String {
        let tool = tool_info();
        JsonValue::object([
            ("schema_version", JsonValue::Number(1)),
            (
                "tool",
                JsonValue::object([
                    ("name", JsonValue::string(tool.name)),
                    ("version", JsonValue::string(tool.version)),
                ]),
            ),
            ("command", JsonValue::string("repair")),
            ("mode", JsonValue::string(self.mode)),
            ("status", JsonValue::string(self.status)),
            (
                "selected_candidate",
                self.selected
                    .as_ref()
                    .map_or(JsonValue::Null, RepairCandidate::to_json),
            ),
            (
                "candidates",
                JsonValue::array(self.candidates.iter().map(RepairCandidate::to_json)),
            ),
            (
                "applied_edits",
                JsonValue::array(self.applied_edits.iter().map(AppliedEdit::to_json)),
            ),
            ("verification", self.verification.to_json()),
            (
                "summary",
                JsonValue::object([
                    (
                        "candidate_count",
                        JsonValue::Number(self.candidates.len() as i64),
                    ),
                    (
                        "applicable_count",
                        JsonValue::Number(
                            self.candidates
                                .iter()
                                .filter(|candidate| candidate.is_applicable())
                                .count() as i64,
                        ),
                    ),
                    (
                        "applied_count",
                        JsonValue::Number(self.applied_edits.len() as i64),
                    ),
                    (
                        "refusal_reason",
                        self.refusal_reason
                            .as_ref()
                            .map_or(JsonValue::Null, JsonValue::string),
                    ),
                ]),
            ),
        ])
        .to_json()
    }
}

impl RepairCandidate {
    fn is_applicable(&self) -> bool {
        self.application_policy == APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE
            && self.application_status == APPLICATION_STATUS_UNAPPLIED
    }

    fn matches_id(&self, id: &str) -> bool {
        self.repair_id == id || self.source_candidate_id == id
    }

    fn span(&self) -> JsonValue {
        span_json(&self.file, self.start, self.end)
    }

    fn to_json(&self) -> JsonValue {
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
                "edit",
                JsonValue::object([
                    ("kind", JsonValue::string("replace")),
                    ("span", self.span()),
                    ("replacement", JsonValue::string(self.replacement.clone())),
                ]),
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
}

impl AppliedEdit {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string("replace")),
            ("span", span_json(&self.file, self.start, self.end)),
            ("replacement", JsonValue::string(self.replacement.clone())),
        ])
    }
}

impl Verification {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("status", JsonValue::string(self.status)),
            (
                "command",
                self.command
                    .as_ref()
                    .map_or(JsonValue::Null, JsonValue::string),
            ),
            (
                "diagnostics",
                JsonValue::array(self.diagnostics.iter().map(diagnostic_to_json)),
            ),
        ])
    }
}

fn print_human(outcome: &RepairOutcome) {
    match outcome.status {
        "preview" => {
            if outcome.candidates.is_empty() {
                println!("no repair candidates found");
                return;
            }
            for candidate in &outcome.candidates {
                println!(
                    "{}: {} at {}:{}:{} -> `{}` [{}]",
                    candidate.repair_id,
                    candidate.edit_summary,
                    candidate.file,
                    candidate.start.line,
                    candidate.start.column,
                    candidate.replacement,
                    candidate.application_policy
                );
            }
        }
        "applied" => {
            let Some(candidate) = &outcome.selected else {
                println!("repair applied");
                return;
            };
            println!(
                "applied {} at {}:{}:{} -> `{}`",
                candidate.repair_id,
                candidate.file,
                candidate.start.line,
                candidate.start.column,
                candidate.replacement
            );
            println!("verification passed");
        }
        "refused" => {
            println!(
                "repair refused: {}",
                outcome
                    .refusal_reason
                    .as_deref()
                    .unwrap_or("unknown reason")
            );
        }
        _ => {}
    }
}

fn apply_candidate(
    project: Project,
    inputs: Vec<PathBuf>,
    candidates: Vec<RepairCandidate>,
    requested_candidate_id: Option<String>,
) -> Result<RepairOutcome, String> {
    let applicable = candidates
        .iter()
        .filter(|candidate| candidate.is_applicable())
        .collect::<Vec<_>>();
    if applicable.is_empty() {
        return Ok(RepairOutcome::refused(
            candidates,
            None,
            "no safe unapplied repair candidates",
        ));
    }

    let selected = match requested_candidate_id.as_deref() {
        Some(id) => match unique_candidate(&candidates, id) {
            Ok(Some(candidate)) => candidate.clone(),
            Ok(None) => {
                return Ok(RepairOutcome::refused(
                    candidates,
                    None,
                    format!("candidate `{id}` was not found"),
                ));
            }
            Err(()) => {
                return Ok(RepairOutcome::refused(
                    candidates,
                    None,
                    format!("candidate `{id}` is ambiguous"),
                ));
            }
        },
        None if applicable.len() == 1 => applicable[0].clone(),
        None => {
            return Ok(RepairOutcome::refused(
                candidates,
                None,
                "multiple safe repair candidates; choose one with `--candidate`",
            ));
        }
    };

    if !selected.is_applicable() {
        return Ok(RepairOutcome::refused(
            candidates,
            Some(selected),
            "candidate is not safe to apply automatically",
        ));
    }

    let source_path = source_path(&project.root, &selected.file)?;
    let original = fs::read_to_string(&source_path).map_err(|error| error.to_string())?;
    let edit = match replace_span(&original, &selected) {
        Ok(edit) => edit,
        Err(reason) => return Ok(RepairOutcome::refused(candidates, Some(selected), reason)),
    };
    fs::write(&source_path, &edit.repaired).map_err(|error| error.to_string())?;

    let mut verify_project =
        Project::discover(project.root.clone(), &inputs).map_err(|error| error.to_string())?;
    let verification_diagnostics = check_diagnostics(&mut verify_project);
    if has_error(&verification_diagnostics) {
        fs::write(&source_path, original).map_err(|error| error.to_string())?;
        let mut outcome = RepairOutcome::refused(candidates, Some(selected), "verification failed");
        outcome.verification = Verification {
            status: "failed",
            command: outcome
                .selected
                .as_ref()
                .and_then(|candidate| candidate.verification_command.clone()),
            diagnostics: verification_diagnostics,
        };
        return Ok(outcome);
    }

    let applied_edit = AppliedEdit {
        file: selected.file.clone(),
        start: selected.start,
        end: edit.end,
        replacement: selected.replacement.clone(),
    };
    Ok(RepairOutcome {
        mode: "apply",
        status: "applied",
        candidates,
        selected: Some(selected.clone()),
        applied_edits: vec![applied_edit],
        verification: Verification {
            status: "passed",
            command: selected.verification_command.clone(),
            diagnostics: verification_diagnostics,
        },
        refusal_reason: None,
    })
}

fn repair_candidates(diagnostics: &[Diagnostic]) -> Vec<RepairCandidate> {
    let mut candidates = Vec::new();
    for diagnostic in diagnostics {
        let Some(queries) = object_array(&diagnostic.details, "candidate_queries") else {
            continue;
        };
        for query in queries {
            let Some(query_candidates) = object_array(query, "candidates") else {
                continue;
            };
            for candidate in query_candidates {
                if let Some(candidate) = RepairCandidate::from_details(candidates.len(), candidate)
                {
                    candidates.push(candidate);
                }
            }
        }
    }
    candidates
}

impl RepairCandidate {
    fn from_details(index: usize, candidate: &JsonValue) -> Option<Self> {
        let edits = object_array(candidate, "edits")?;
        let [edit] = edits.as_slice() else {
            return None;
        };
        let span = object_value(edit, "span").and_then(source_span)?;
        Some(Self {
            repair_id: format!("repair-{}", index + 1),
            source_candidate_id: object_string(candidate, "candidate_id")?.to_string(),
            name: object_string(candidate, "name").unwrap_or("").to_string(),
            application_policy: object_string(candidate, "application_policy")?.to_string(),
            application_status: object_string(candidate, "application_status")?.to_string(),
            edit_summary: object_string(candidate, "edit_summary")
                .unwrap_or("Replace source span")
                .to_string(),
            file: span.file.as_str().to_string(),
            start: span.start,
            end: span.end,
            replacement: object_string(edit, "replacement")?.to_string(),
            verification_command: object_value(candidate, "verification_hint")
                .and_then(|hint| object_string(hint, "command"))
                .map(str::to_string),
            source: candidate.clone(),
        })
    }
}

fn unique_candidate<'a>(
    candidates: &'a [RepairCandidate],
    id: &str,
) -> Result<Option<&'a RepairCandidate>, ()> {
    let mut matches = candidates
        .iter()
        .filter(|candidate| candidate.matches_id(id))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(());
    }
    Ok(matches.pop())
}

fn source_path(root: &Path, file: &str) -> Result<PathBuf, String> {
    let path = Path::new(file);
    if path.is_absolute() {
        return Err("repair target must be project-relative".to_string());
    }
    Ok(root.join(path))
}

#[derive(Debug)]
struct ResolvedEdit {
    repaired: String,
    end: LineCol,
}

fn replace_span(source: &str, candidate: &RepairCandidate) -> Result<ResolvedEdit, String> {
    let start = candidate.start.offset;
    let mut end = candidate.end.offset;
    if start >= end || end > source.len() {
        return Err("repair target span is stale".to_string());
    }
    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return Err("repair target span is not on character boundaries".to_string());
    }
    let target = &source[start..end];
    if !target.trim_start().starts_with('_') {
        return Err("repair target no longer names a hole".to_string());
    }
    if source[end..].starts_with(" satisfy ") {
        end += source[end..]
            .find('\n')
            .unwrap_or_else(|| source[end..].len());
    }

    let mut repaired = String::with_capacity(source.len() + candidate.replacement.len());
    repaired.push_str(&source[..start]);
    repaired.push_str(&candidate.replacement);
    repaired.push_str(&source[end..]);
    Ok(ResolvedEdit {
        repaired,
        end: line_col_at(source, end),
    })
}

fn line_col_at(source: &str, offset: usize) -> LineCol {
    let offset = offset.min(source.len());
    let mut line = 1;
    let mut line_start = 0;
    for (index, byte) in source.bytes().enumerate() {
        if index >= offset {
            break;
        }
        if byte == b'\n' {
            line += 1;
            line_start = index + 1;
        }
    }
    let column = source[line_start..offset].chars().count() + 1;
    LineCol {
        line,
        column,
        offset,
    }
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

fn object_string<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    match object_value(value, key)? {
        JsonValue::String(value) => Some(value),
        _ => None,
    }
}

fn object_number(value: &JsonValue, key: &str) -> Option<i64> {
    match object_value(value, key)? {
        JsonValue::Number(value) => Some(*value),
        _ => None,
    }
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
            verification_command: None,
            source: JsonValue::Null,
        }
    }

    #[test]
    fn replace_span_refuses_stale_targets() {
        let error = replace_span("value\n", &candidate(0, 5)).expect_err("target is not a hole");

        assert_eq!(error, "repair target no longer names a hole");
    }

    #[test]
    fn replace_span_refuses_out_of_bounds_targets() {
        let error = replace_span("_hole\n", &candidate(0, 20)).expect_err("target is stale");

        assert_eq!(error, "repair target span is stale");
    }
}
