use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_analysis::{DoctestMode, checked_project_diagnostics};
use veln_project::Project;

mod application;
mod candidates;
mod editing;
mod outcome;

use application::{RepairApplyOptions, apply_candidate};
use candidates::{
    repair_candidates_from_diagnostics, repair_candidates_from_saved_inputs, split_repair_inputs,
};
use outcome::{RepairOutcome, print_human};

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
    let diagnostics = checked_project_diagnostics(project.clone(), DoctestMode::Include);
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
