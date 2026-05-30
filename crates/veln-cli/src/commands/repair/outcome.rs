use veln_diagnostics::{Diagnostic, JsonValue, diagnostic_to_json};

use crate::diagnostics::tool_info;

use super::candidates::{CandidateIdMatch, RepairCandidate, RepairEdit, find_candidate_by_id};
use super::{
    REPAIR_COMMAND, REPAIR_MODE_APPLY, REPAIR_MODE_PREVIEW, REPAIR_STATUS_APPLIED,
    REPAIR_STATUS_PREVIEW, REPAIR_STATUS_REFUSED, VERIFICATION_STATUS_NOT_RUN,
};

#[derive(Clone, Debug)]
pub(super) struct RepairOutcome {
    pub(super) mode: &'static str,
    pub(super) status: &'static str,
    pub(super) candidates: Vec<RepairCandidate>,
    pub(super) selected: Option<RepairCandidate>,
    pub(super) applied_edits: Vec<AppliedEdit>,
    pub(super) verification: Verification,
    pub(super) confirmation: Option<RepairConfirmation>,
    pub(super) override_record: Option<RepairOverride>,
    pub(super) refusal_reason: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct AppliedEdit {
    pub(super) edit: RepairEdit,
}

#[derive(Clone, Debug)]
pub(super) struct Verification {
    pub(super) status: &'static str,
    pub(super) command: Option<String>,
    pub(super) diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub(super) struct RepairConfirmation {
    confirmed_candidate_id: String,
    repair_id: String,
    source_candidate_id: String,
    override_requested: bool,
}

#[derive(Clone, Debug)]
pub(super) struct RepairOverride {
    application_policy: String,
    application_status: String,
    accepted_obligations: Vec<String>,
}

impl RepairOutcome {
    pub(super) fn preview(
        candidates: Vec<RepairCandidate>,
        requested_candidate_id: Option<String>,
    ) -> Self {
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

    pub(super) fn refused(
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

    pub(super) fn exit_success(&self) -> bool {
        self.status != REPAIR_STATUS_REFUSED
    }

    pub(super) fn to_json(&self) -> String {
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

impl AppliedEdit {
    fn to_json(&self) -> JsonValue {
        self.edit.to_json()
    }
}

impl Verification {
    pub(super) fn to_json(&self) -> JsonValue {
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
    pub(super) fn new(
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
    pub(super) fn from_candidate(candidate: &RepairCandidate) -> Self {
        Self {
            application_policy: candidate.application_policy.clone(),
            application_status: candidate.application_status.clone(),
            accepted_obligations: candidate.blocking_obligations(),
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

pub(super) fn print_human(outcome: &RepairOutcome) {
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
