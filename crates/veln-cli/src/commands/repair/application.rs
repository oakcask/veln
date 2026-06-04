use std::fs;
use std::path::PathBuf;

use veln_analysis::{DoctestMode, checked_project_diagnostics};
use veln_project::Project;

use crate::diagnostics::has_error;

use super::candidates::{
    CandidateIdMatch, RepairCandidate, find_candidate_by_id, has_current_applicable_match,
    safe_unapplied_candidates,
};
use super::editing::build_edit_plan;
use super::outcome::{RepairConfirmation, RepairOutcome, RepairOverride, Verification};
use super::{
    APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED, APPLICATION_STATUS_UNAPPLIED,
    REFUSAL_CANDIDATE_NOT_AUTOMATIC, REFUSAL_MULTIPLE_SAFE_CANDIDATES, REFUSAL_NO_SAFE_CANDIDATES,
    REFUSAL_OVERRIDE_REQUIRES_CONFIRMATION, REFUSAL_OVERRIDE_STATUS_NOT_UNAPPLIED,
    REFUSAL_OVERRIDE_UNSUPPORTED_POLICY, REFUSAL_SAVED_CANDIDATE_NOT_CURRENT,
    REFUSAL_VERIFICATION_FAILED, REPAIR_MODE_APPLY, REPAIR_STATUS_APPLIED,
    VERIFICATION_STATUS_FAILED, VERIFICATION_STATUS_PASSED,
};

#[derive(Clone, Debug)]
pub(super) struct RepairApplyOptions {
    pub(super) requested_candidate_id: Option<String>,
    pub(super) confirmed_candidate_id: Option<String>,
    pub(super) override_requested: bool,
}

struct CandidateSelectionError {
    selected: Option<Box<RepairCandidate>>,
    reason: String,
}

pub(super) fn apply_candidate(
    project: Project,
    inputs: Vec<PathBuf>,
    candidates: Vec<RepairCandidate>,
    options: RepairApplyOptions,
    current_candidates: &[RepairCandidate],
) -> Result<RepairOutcome, String> {
    let selected = match select_candidate_for_apply(&candidates, &options) {
        Ok(selected) => selected,
        Err(error) => {
            return Ok(RepairOutcome::refused(
                candidates,
                error.selected.map(|selected| *selected),
                error.reason,
            ));
        }
    };

    if let Some(reason) = apply_authority_refusal(&selected, &options, current_candidates) {
        return Ok(RepairOutcome::refused(candidates, Some(selected), reason));
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
    let verification_diagnostics =
        checked_project_diagnostics(verify_project, DoctestMode::Include);
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

fn select_candidate_for_apply(
    candidates: &[RepairCandidate],
    options: &RepairApplyOptions,
) -> Result<RepairCandidate, CandidateSelectionError> {
    let selected = match options
        .requested_candidate_id
        .as_deref()
        .or(options.confirmed_candidate_id.as_deref())
    {
        Some(id) => find_selected_candidate(candidates, id)?,
        None => select_only_safe_candidate(candidates)?,
    }
    .clone();

    if options.override_requested && options.confirmed_candidate_id.is_none() {
        return Err(CandidateSelectionError {
            selected: Some(Box::new(selected)),
            reason: REFUSAL_OVERRIDE_REQUIRES_CONFIRMATION.to_string(),
        });
    }

    if let Some(confirmed_candidate_id) = options.confirmed_candidate_id.as_deref() {
        let confirmed = match find_candidate_by_id(candidates, confirmed_candidate_id) {
            CandidateIdMatch::Unique(candidate) => candidate,
            CandidateIdMatch::Missing => {
                return Err(CandidateSelectionError {
                    selected: Some(Box::new(selected)),
                    reason: format!("confirmed candidate `{confirmed_candidate_id}` was not found"),
                });
            }
            CandidateIdMatch::Ambiguous => {
                return Err(CandidateSelectionError {
                    selected: Some(Box::new(selected)),
                    reason: format!("confirmed candidate `{confirmed_candidate_id}` is ambiguous"),
                });
            }
        };
        if confirmed.repair_id != selected.repair_id {
            return Err(CandidateSelectionError {
                selected: Some(Box::new(selected)),
                reason: "confirmed candidate does not match selected candidate".to_string(),
            });
        }
    }

    Ok(selected)
}

fn select_only_safe_candidate(
    candidates: &[RepairCandidate],
) -> Result<&RepairCandidate, CandidateSelectionError> {
    let applicable = safe_unapplied_candidates(candidates);
    match applicable.as_slice() {
        [candidate] => Ok(candidate),
        [] => Err(CandidateSelectionError {
            selected: None,
            reason: REFUSAL_NO_SAFE_CANDIDATES.to_string(),
        }),
        _ => Err(CandidateSelectionError {
            selected: None,
            reason: REFUSAL_MULTIPLE_SAFE_CANDIDATES.to_string(),
        }),
    }
}

fn find_selected_candidate<'a>(
    candidates: &'a [RepairCandidate],
    id: &str,
) -> Result<&'a RepairCandidate, CandidateSelectionError> {
    match find_candidate_by_id(candidates, id) {
        CandidateIdMatch::Unique(candidate) => Ok(candidate),
        CandidateIdMatch::Missing => Err(CandidateSelectionError {
            selected: None,
            reason: format!("candidate `{id}` was not found"),
        }),
        CandidateIdMatch::Ambiguous => Err(CandidateSelectionError {
            selected: None,
            reason: format!("candidate `{id}` is ambiguous"),
        }),
    }
}

fn apply_authority_refusal(
    selected: &RepairCandidate,
    options: &RepairApplyOptions,
    current_candidates: &[RepairCandidate],
) -> Option<&'static str> {
    if !selected.is_safe_unapplied() && !options.override_requested {
        return Some(REFUSAL_CANDIDATE_NOT_AUTOMATIC);
    }
    if options.override_requested
        && !selected.is_safe_unapplied()
        && selected.application_policy != APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED
    {
        return Some(REFUSAL_OVERRIDE_UNSUPPORTED_POLICY);
    }
    if options.override_requested && selected.application_status != APPLICATION_STATUS_UNAPPLIED {
        return Some(REFUSAL_OVERRIDE_STATUS_NOT_UNAPPLIED);
    }
    if selected.requires_current_match
        && !options.override_requested
        && !has_current_applicable_match(selected, current_candidates)
    {
        return Some(REFUSAL_SAVED_CANDIDATE_NOT_CURRENT);
    }
    None
}
