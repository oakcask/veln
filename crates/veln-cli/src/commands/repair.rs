use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use veln_diagnostics::{Diagnostic, JsonValue, diagnostic_to_json, parse_json_value};
use veln_project::Project;
use veln_source::{LineCol, SourceSpan};

use crate::commands::check::check_diagnostics;
use crate::diagnostics::{has_error, tool_info};

const APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED: &str = "manual_review_required";
const APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE: &str = "safe_repair_candidate";
const APPLICATION_STATUS_UNAPPLIED: &str = "unapplied";
const REPAIR_COMMAND: &str = "repair";
const REPAIR_MODE_APPLY: &str = "apply";
const REPAIR_MODE_PREVIEW: &str = "preview";
const REPAIR_STATUS_APPLIED: &str = "applied";
const REPAIR_STATUS_PREVIEW: &str = "preview";
const REPAIR_STATUS_REFUSED: &str = "refused";
const VERIFICATION_STATUS_FAILED: &str = "failed";
const VERIFICATION_STATUS_NOT_RUN: &str = "not_run";
const VERIFICATION_STATUS_PASSED: &str = "passed";

const REFUSAL_CANDIDATE_NOT_AUTOMATIC: &str = "candidate is not safe to apply automatically";
const REFUSAL_MULTIPLE_SAFE_CANDIDATES: &str =
    "multiple safe repair candidates; choose one with `--candidate`";
const REFUSAL_NO_SAFE_CANDIDATES: &str = "no safe unapplied repair candidates";
const REFUSAL_OVERRIDE_REQUIRES_CONFIRMATION: &str = "override requires `--confirm <candidate_id>`";
const REFUSAL_OVERRIDE_UNSUPPORTED_POLICY: &str = "override candidate policy is not supported";
const REFUSAL_OVERRIDE_STATUS_NOT_UNAPPLIED: &str = "override candidate is not unapplied";
const REFUSAL_SAVED_CANDIDATE_NOT_CURRENT: &str = "saved candidate is not current";
const REFUSAL_VERIFICATION_FAILED: &str = "verification failed";

pub(crate) fn repair(
    json: bool,
    apply: bool,
    candidate_id: Option<String>,
    confirm_id: Option<String>,
    override_requested: bool,
    inputs: Vec<PathBuf>,
) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let (source_inputs, candidate_inputs) = split_repair_inputs(&inputs);
    let project =
        Project::discover(root.clone(), &source_inputs).map_err(|error| error.to_string())?;
    let diagnostics = check_diagnostics(project.clone());
    let current_candidates = repair_candidates_from_diagnostics(&diagnostics);
    let candidates = if candidate_inputs.is_empty() {
        current_candidates.clone()
    } else {
        repair_candidates_from_saved_inputs(&root, &candidate_inputs)?
    };
    let outcome = if apply {
        apply_candidate(
            project,
            source_inputs,
            candidates,
            RepairApplyOptions {
                requested_candidate_id: candidate_id,
                confirmed_candidate_id: confirm_id,
                override_requested,
            },
            &current_candidates,
        )?
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
    edits: Vec<RepairEdit>,
    verification_command: Option<String>,
    source: JsonValue,
    input_repair_id: Option<String>,
    requires_current_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RepairEdit {
    file: String,
    start: LineCol,
    end: LineCol,
    replacement: String,
}

#[derive(Clone, Debug)]
struct RepairOutcome {
    mode: &'static str,
    status: &'static str,
    candidates: Vec<RepairCandidate>,
    selected: Option<RepairCandidate>,
    applied_edits: Vec<AppliedEdit>,
    verification: Verification,
    confirmation: Option<RepairConfirmation>,
    override_record: Option<RepairOverride>,
    refusal_reason: Option<String>,
}

#[derive(Clone, Debug)]
struct AppliedEdit {
    edit: RepairEdit,
}

#[derive(Clone, Debug)]
struct Verification {
    status: &'static str,
    command: Option<String>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
struct RepairConfirmation {
    confirmed_candidate_id: String,
    repair_id: String,
    source_candidate_id: String,
    override_requested: bool,
}

#[derive(Clone, Debug)]
struct RepairOverride {
    application_policy: String,
    application_status: String,
    accepted_obligations: Vec<String>,
}

#[derive(Clone, Debug)]
struct RepairApplyOptions {
    requested_candidate_id: Option<String>,
    confirmed_candidate_id: Option<String>,
    override_requested: bool,
}

impl RepairOutcome {
    fn preview(candidates: Vec<RepairCandidate>, requested_candidate_id: Option<String>) -> Self {
        let selected = requested_candidate_id
            .as_deref()
            .and_then(|id| match find_candidate_by_id(&candidates, id) {
                CandidateIdMatch::Unique(candidate) => Some(candidate.clone()),
                CandidateIdMatch::Missing | CandidateIdMatch::Ambiguous => None,
            });
        Self {
            mode: REPAIR_MODE_PREVIEW,
            status: REPAIR_STATUS_PREVIEW,
            candidates,
            selected,
            applied_edits: Vec::new(),
            verification: Verification {
                status: VERIFICATION_STATUS_NOT_RUN,
                command: None,
                diagnostics: Vec::new(),
            },
            confirmation: None,
            override_record: None,
            refusal_reason: None,
        }
    }

    fn refused(
        candidates: Vec<RepairCandidate>,
        selected: Option<RepairCandidate>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            mode: REPAIR_MODE_APPLY,
            status: REPAIR_STATUS_REFUSED,
            candidates,
            selected,
            applied_edits: Vec::new(),
            verification: Verification {
                status: VERIFICATION_STATUS_NOT_RUN,
                command: None,
                diagnostics: Vec::new(),
            },
            confirmation: None,
            override_record: None,
            refusal_reason: Some(reason.into()),
        }
    }

    fn exit_success(&self) -> bool {
        self.status != REPAIR_STATUS_REFUSED
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
            ("command", JsonValue::string(REPAIR_COMMAND)),
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
                "confirmation",
                self.confirmation
                    .as_ref()
                    .map_or(JsonValue::Null, RepairConfirmation::to_json),
            ),
            (
                "override",
                self.override_record
                    .as_ref()
                    .map_or(JsonValue::Null, RepairOverride::to_json),
            ),
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
                                .filter(|candidate| candidate.is_safe_unapplied())
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
    fn is_safe_unapplied(&self) -> bool {
        self.application_policy == APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE
            && self.application_status == APPLICATION_STATUS_UNAPPLIED
    }

    fn matches_id(&self, id: &str) -> bool {
        self.repair_id == id
            || self.source_candidate_id == id
            || self.input_repair_id.as_deref() == Some(id)
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
}

impl RepairEdit {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string("replace")),
            ("span", span_json(&self.file, self.start, self.end)),
            ("replacement", JsonValue::string(self.replacement.clone())),
        ])
    }
}

impl AppliedEdit {
    fn to_json(&self) -> JsonValue {
        self.edit.to_json()
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

impl RepairConfirmation {
    fn new(
        confirmed_candidate_id: String,
        selected: &RepairCandidate,
        override_requested: bool,
    ) -> Self {
        Self {
            confirmed_candidate_id,
            repair_id: selected.repair_id.clone(),
            source_candidate_id: selected.source_candidate_id.clone(),
            override_requested,
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "confirmed_candidate_id",
                JsonValue::string(self.confirmed_candidate_id.clone()),
            ),
            ("repair_id", JsonValue::string(self.repair_id.clone())),
            (
                "source_candidate_id",
                JsonValue::string(self.source_candidate_id.clone()),
            ),
            ("override", JsonValue::Bool(self.override_requested)),
        ])
    }
}

impl RepairOverride {
    fn from_candidate(candidate: &RepairCandidate) -> Self {
        Self {
            application_policy: candidate.application_policy.clone(),
            application_status: candidate.application_status.clone(),
            accepted_obligations: object_string_array(&candidate.source, "blocking_obligations")
                .unwrap_or_default(),
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "application_policy",
                JsonValue::string(self.application_policy.clone()),
            ),
            (
                "application_status",
                JsonValue::string(self.application_status.clone()),
            ),
            (
                "accepted_obligations",
                JsonValue::array(
                    self.accepted_obligations
                        .iter()
                        .map(|obligation| JsonValue::string(obligation.clone())),
                ),
            ),
        ])
    }
}

fn print_human(outcome: &RepairOutcome) {
    match outcome.status {
        REPAIR_STATUS_PREVIEW => {
            if outcome.candidates.is_empty() {
                println!("no repair candidates found");
                return;
            }
            for candidate in &outcome.candidates {
                let Some(edit) = candidate.edits.first() else {
                    continue;
                };
                println!(
                    "{}: {} at {}:{}:{} -> `{}` [{}]",
                    candidate.repair_id,
                    candidate.edit_summary,
                    edit.file,
                    edit.start.line,
                    edit.start.column,
                    edit.replacement,
                    candidate.application_policy
                );
            }
        }
        REPAIR_STATUS_APPLIED => {
            let Some(candidate) = &outcome.selected else {
                println!("repair applied");
                return;
            };
            let Some(edit) = candidate.edits.first() else {
                println!("repair applied");
                println!("verification passed");
                return;
            };
            println!(
                "applied {} at {}:{}:{} -> `{}`",
                candidate.repair_id,
                edit.file,
                edit.start.line,
                edit.start.column,
                edit.replacement
            );
            println!("verification passed");
        }
        REPAIR_STATUS_REFUSED => {
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
    options: RepairApplyOptions,
    current_candidates: &[RepairCandidate],
) -> Result<RepairOutcome, String> {
    let applicable = safe_unapplied_candidates(&candidates);

    let selection_id = options
        .requested_candidate_id
        .as_deref()
        .or(options.confirmed_candidate_id.as_deref());
    let selected = match selection_id {
        Some(id) => match find_candidate_by_id(&candidates, id) {
            CandidateIdMatch::Unique(candidate) => candidate.clone(),
            CandidateIdMatch::Missing => {
                return Ok(RepairOutcome::refused(
                    candidates,
                    None,
                    format!("candidate `{id}` was not found"),
                ));
            }
            CandidateIdMatch::Ambiguous => {
                return Ok(RepairOutcome::refused(
                    candidates,
                    None,
                    format!("candidate `{id}` is ambiguous"),
                ));
            }
        },
        None if applicable.len() == 1 => applicable[0].clone(),
        None if applicable.is_empty() => {
            return Ok(RepairOutcome::refused(
                candidates,
                None,
                REFUSAL_NO_SAFE_CANDIDATES,
            ));
        }
        None => {
            return Ok(RepairOutcome::refused(
                candidates,
                None,
                REFUSAL_MULTIPLE_SAFE_CANDIDATES,
            ));
        }
    };

    if options.override_requested && options.confirmed_candidate_id.is_none() {
        return Ok(RepairOutcome::refused(
            candidates,
            Some(selected),
            REFUSAL_OVERRIDE_REQUIRES_CONFIRMATION,
        ));
    }
    if let Some(confirmed_candidate_id) = options.confirmed_candidate_id.as_deref() {
        match find_candidate_by_id(&candidates, confirmed_candidate_id) {
            CandidateIdMatch::Unique(candidate) if candidate.repair_id == selected.repair_id => {}
            CandidateIdMatch::Unique(_) => {
                return Ok(RepairOutcome::refused(
                    candidates,
                    Some(selected),
                    "confirmed candidate does not match selected candidate",
                ));
            }
            CandidateIdMatch::Missing => {
                return Ok(RepairOutcome::refused(
                    candidates,
                    Some(selected),
                    format!("confirmed candidate `{confirmed_candidate_id}` was not found"),
                ));
            }
            CandidateIdMatch::Ambiguous => {
                return Ok(RepairOutcome::refused(
                    candidates,
                    Some(selected),
                    format!("confirmed candidate `{confirmed_candidate_id}` is ambiguous"),
                ));
            }
        }
    }

    if !selected.is_safe_unapplied() && !options.override_requested {
        return Ok(RepairOutcome::refused(
            candidates,
            Some(selected),
            REFUSAL_CANDIDATE_NOT_AUTOMATIC,
        ));
    }
    if options.override_requested
        && !selected.is_safe_unapplied()
        && selected.application_policy != APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED
    {
        return Ok(RepairOutcome::refused(
            candidates,
            Some(selected),
            REFUSAL_OVERRIDE_UNSUPPORTED_POLICY,
        ));
    }
    if options.override_requested && selected.application_status != APPLICATION_STATUS_UNAPPLIED {
        return Ok(RepairOutcome::refused(
            candidates,
            Some(selected),
            REFUSAL_OVERRIDE_STATUS_NOT_UNAPPLIED,
        ));
    }
    if selected.requires_current_match
        && !options.override_requested
        && !has_current_applicable_match(&selected, current_candidates)
    {
        return Ok(RepairOutcome::refused(
            candidates,
            Some(selected),
            REFUSAL_SAVED_CANDIDATE_NOT_CURRENT,
        ));
    }

    let edit_plan = match build_edit_plan(&project.root, &selected) {
        Ok(edit_plan) => edit_plan,
        Err(reason) => return Ok(RepairOutcome::refused(candidates, Some(selected), reason)),
    };
    for file_edit in &edit_plan.files {
        fs::write(&file_edit.path, &file_edit.repaired).map_err(|error| error.to_string())?;
    }

    let verify_project =
        Project::discover(project.root.clone(), &inputs).map_err(|error| error.to_string())?;
    let verification_diagnostics = check_diagnostics(verify_project);
    if has_error(&verification_diagnostics) {
        for file_edit in &edit_plan.files {
            fs::write(&file_edit.path, &file_edit.original).map_err(|error| error.to_string())?;
        }
        let mut outcome =
            RepairOutcome::refused(candidates, Some(selected), REFUSAL_VERIFICATION_FAILED);
        outcome.verification = Verification {
            status: VERIFICATION_STATUS_FAILED,
            command: outcome
                .selected
                .as_ref()
                .and_then(|candidate| candidate.verification_command.clone()),
            diagnostics: verification_diagnostics,
        };
        return Ok(outcome);
    }

    Ok(RepairOutcome {
        mode: REPAIR_MODE_APPLY,
        status: REPAIR_STATUS_APPLIED,
        candidates,
        selected: Some(selected.clone()),
        applied_edits: edit_plan.applied_edits,
        verification: Verification {
            status: VERIFICATION_STATUS_PASSED,
            command: selected.verification_command.clone(),
            diagnostics: verification_diagnostics,
        },
        confirmation: options
            .confirmed_candidate_id
            .map(|confirmed_candidate_id| {
                RepairConfirmation::new(
                    confirmed_candidate_id,
                    &selected,
                    options.override_requested,
                )
            }),
        override_record: options
            .override_requested
            .then(|| RepairOverride::from_candidate(&selected)),
        refusal_reason: None,
    })
}

fn safe_unapplied_candidates(candidates: &[RepairCandidate]) -> Vec<&RepairCandidate> {
    candidates
        .iter()
        .filter(|candidate| candidate.is_safe_unapplied())
        .collect()
}

fn repair_candidates_from_diagnostics(diagnostics: &[Diagnostic]) -> Vec<RepairCandidate> {
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
                if let Some(candidate) = RepairCandidate::from_advisory(candidates.len(), candidate)
                {
                    candidates.push(candidate);
                }
            }
        }
    }
    candidates
}

impl RepairCandidate {
    fn from_advisory(index: usize, candidate: &JsonValue) -> Option<Self> {
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
                requires_current_match: false,
            },
        )
    }

    fn from_saved_advisory(index: usize, candidate: &JsonValue) -> Option<Self> {
        let mut candidate = Self::from_advisory(index, candidate)?;
        candidate.requires_current_match = true;
        Some(candidate)
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

struct CandidateParts<'a> {
    candidate: &'a JsonValue,
    source_candidate_id: &'a str,
    input_repair_id: Option<&'a str>,
    edits: Vec<RepairEdit>,
    verification_command: Option<&'a str>,
    source: JsonValue,
    requires_current_match: bool,
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

fn split_repair_inputs(inputs: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>) {
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

fn repair_candidates_from_saved_inputs(
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
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_) => {}
    }
}

fn collect_saved_diagnostic_candidates(value: &JsonValue, candidates: &mut Vec<RepairCandidate>) {
    let Some(details) = object_value(value, "details") else {
        return;
    };
    let Some(queries) = object_array(details, "candidate_queries") else {
        return;
    };
    for query in queries {
        let Some(query_candidates) = object_array(query, "candidates") else {
            continue;
        };
        for candidate in query_candidates {
            if let Some(candidate) =
                RepairCandidate::from_saved_advisory(candidates.len(), candidate)
            {
                candidates.push(candidate);
            }
        }
    }
}

fn has_current_applicable_match(
    selected: &RepairCandidate,
    current_candidates: &[RepairCandidate],
) -> bool {
    let mut matched_non_empty_edit = false;
    for edit in &selected.edits {
        if is_satisfy_suffix_deletion(edit) {
            if !selected.edits.iter().any(|other| {
                !is_satisfy_suffix_deletion(other)
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

enum CandidateIdMatch<'a> {
    Missing,
    Unique(&'a RepairCandidate),
    Ambiguous,
}

fn find_candidate_by_id<'a>(candidates: &'a [RepairCandidate], id: &str) -> CandidateIdMatch<'a> {
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

fn source_path(root: &Path, file: &str) -> Result<PathBuf, String> {
    let path = Path::new(file);
    if path.is_absolute() {
        return Err("repair target must be project-relative".to_string());
    }
    Ok(root.join(path))
}

#[derive(Debug)]
struct ResolvedEdit {
    start: usize,
    end: usize,
    applied: AppliedEdit,
}

#[derive(Debug)]
struct FileEdit {
    path: PathBuf,
    original: String,
    repaired: String,
}

#[derive(Debug)]
struct EditPlan {
    files: Vec<FileEdit>,
    applied_edits: Vec<AppliedEdit>,
}

struct PendingFileEdit {
    file: String,
    path: PathBuf,
    original: String,
    edits: Vec<ResolvedEdit>,
}

fn build_edit_plan(root: &Path, candidate: &RepairCandidate) -> Result<EditPlan, String> {
    if candidate.edits.is_empty() {
        return Err("unsupported edit shape".to_string());
    }

    let mut pending_files = Vec::<PendingFileEdit>::new();
    for edit in &candidate.edits {
        let pending_index = if let Some(index) = pending_files
            .iter()
            .position(|pending| pending.file == edit.file)
        {
            index
        } else {
            let path = source_path(root, &edit.file)?;
            let original = fs::read_to_string(&path).map_err(|error| error.to_string())?;
            pending_files.push(PendingFileEdit {
                file: edit.file.clone(),
                path,
                original,
                edits: Vec::new(),
            });
            pending_files.len() - 1
        };
        let has_explicit_followup = candidate
            .edits
            .iter()
            .any(|other| other.file == edit.file && other.start.offset == edit.end.offset);
        let resolved = resolve_edit_span(
            &pending_files[pending_index].original,
            edit,
            has_explicit_followup,
        )?;
        pending_files[pending_index].edits.push(resolved);
    }

    let mut files = Vec::new();
    let mut applied_edits = Vec::new();
    for mut pending in pending_files {
        pending.edits.sort_by_key(|edit| edit.start);
        for pair in pending.edits.windows(2) {
            if pair[0].end > pair[1].start {
                return Err("repair edits overlap".to_string());
            }
        }

        let mut repaired = pending.original.clone();
        for edit in pending.edits.iter().rev() {
            repaired.replace_range(edit.start..edit.end, &edit.applied.edit.replacement);
        }
        applied_edits.extend(pending.edits.into_iter().map(|edit| edit.applied));
        files.push(FileEdit {
            path: pending.path,
            original: pending.original,
            repaired,
        });
    }

    Ok(EditPlan {
        files,
        applied_edits,
    })
}

fn resolve_edit_span(
    source: &str,
    edit: &RepairEdit,
    has_explicit_followup: bool,
) -> Result<ResolvedEdit, String> {
    let start = edit.start.offset;
    let mut end = edit.end.offset;
    if start >= end || end > source.len() {
        return Err("repair target span is stale".to_string());
    }
    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return Err("repair target span is not on character boundaries".to_string());
    }
    let target = &source[start..end];
    if target.trim_start().starts_with('_') {
        if !has_explicit_followup && source[end..].starts_with(" satisfy ") {
            end += source[end..]
                .find('\n')
                .unwrap_or_else(|| source[end..].len());
        }
    } else if !(edit.replacement.is_empty() && is_satisfy_suffix_text(target)) {
        return Err("repair target no longer names a hole".to_string());
    }

    Ok(ResolvedEdit {
        start,
        end,
        applied: AppliedEdit {
            edit: RepairEdit {
                file: edit.file.clone(),
                start: edit.start,
                end: line_col_at(source, end),
                replacement: edit.replacement.clone(),
            },
        },
    })
}

fn is_satisfy_suffix_deletion(edit: &RepairEdit) -> bool {
    edit.replacement.is_empty()
}

fn is_satisfy_suffix_text(text: &str) -> bool {
    text.starts_with(" satisfy ") || text.starts_with("satisfy ")
}

#[cfg(test)]
fn replace_span(source: &str, candidate: &RepairCandidate) -> Result<LegacyResolvedEdit, String> {
    let edit = candidate
        .edits
        .first()
        .ok_or_else(|| "unsupported edit shape".to_string())?;
    let resolved = resolve_edit_span(source, edit, false)?;
    let mut repaired = source.to_string();
    repaired.replace_range(
        resolved.start..resolved.end,
        &resolved.applied.edit.replacement,
    );
    Ok(LegacyResolvedEdit {
        repaired,
        end: resolved.applied.edit.end,
    })
}

#[cfg(test)]
#[derive(Debug)]
struct LegacyResolvedEdit {
    repaired: String,
    end: LineCol,
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
    fn replace_span_refuses_stale_targets() {
        let error = replace_span("value\n", &candidate(0, 5)).expect_err("target is not a hole");

        assert_eq!(error, "repair target no longer names a hole");
    }

    #[test]
    fn replace_span_refuses_out_of_bounds_targets() {
        let error = replace_span("_hole\n", &candidate(0, 20)).expect_err("target is stale");

        assert_eq!(error, "repair target span is stale");
    }

    #[test]
    fn replace_span_removes_satisfy_suffix() {
        let edit = replace_span(
            "fn main(order: Int) -> Int\n  _value satisfy candidate => candidate == order\nend\n",
            &candidate(29, 35),
        )
        .expect("satisfy suffix should be included in the edit");

        assert_eq!(edit.repaired, "fn main(order: Int) -> Int\n  value\nend\n");
        assert_eq!(edit.end.line, 2);
        assert_eq!(edit.end.column, 49);
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

        let candidate =
            RepairCandidate::from_advisory(0, &candidate).expect("candidate should load");

        assert_eq!(candidate.edits.len(), 2);
    }

    #[test]
    fn build_edit_plan_refuses_overlapping_edits() {
        let root = std::env::temp_dir().join(format!("veln-repair-overlap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(root.join("main.veln"), "__hole\n").expect("source should be written");

        let mut candidate = candidate(0, 6);
        candidate.edits.push(RepairEdit {
            file: "main.veln".to_string(),
            start: LineCol {
                line: 1,
                column: 2,
                offset: 1,
            },
            end: LineCol {
                line: 1,
                column: 7,
                offset: 6,
            },
            replacement: "other".to_string(),
        });

        let error = build_edit_plan(&root, &candidate).expect_err("overlap should refuse");

        assert_eq!(error, "repair edits overlap");
        let _ = fs::remove_dir_all(&root);
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
}
