use veln_diagnostics::JsonValue;

use crate::semantic_model::Type;

pub(crate) const CANDIDATE_STATUS_QUERY_ONLY: &str = "query_only";
pub(crate) const APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED: &str = "manual_review_required";
pub(crate) const APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE: &str = "safe_repair_candidate";
pub(crate) const APPLICATION_STATUS_UNAPPLIED: &str = "unapplied";
pub(crate) const SATISFY_STATUS_BLOCKED_UNTIL_DISCHARGED: &str = "blocked_until_discharged";
pub(crate) const SATISFY_STATUS_STATICALLY_SATISFIED: &str = "statically_satisfied";

const KNOWN_LIMIT_ADVISORY_UNAPPLIED: &str = "edit is advisory and unapplied";
const KNOWN_LIMIT_VERIFICATION_NOT_RUN: &str = "tests and examples have not been run";
const KNOWN_LIMIT_SATISFY_BLOCKED: &str =
    "satisfy predicate is not statically discharged by this candidate";
const OBLIGATION_MANUAL_REVIEW_REQUIRED: &str = "manual_review_required";
const OBLIGATION_SATISFY_BLOCKED: &str = "satisfy.blocked_until_discharged";
const OBLIGATION_VERIFICATION_NOT_RUN: &str = "verification.not_run";

pub(crate) fn application_policy(statically_satisfied: bool) -> &'static str {
    if statically_satisfied {
        APPLICATION_POLICY_SAFE_REPAIR_CANDIDATE
    } else {
        APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED
    }
}

pub(crate) fn candidate_satisfy_status(
    has_satisfy: bool,
    statically_satisfied: bool,
) -> Option<&'static str> {
    has_satisfy.then_some(if statically_satisfied {
        SATISFY_STATUS_STATICALLY_SATISFIED
    } else {
        SATISFY_STATUS_BLOCKED_UNTIL_DISCHARGED
    })
}

pub(crate) fn candidate_evidence(
    expected: &Type,
    actual: &Type,
    rank: usize,
    reason: &'static str,
    satisfy_status: Option<&'static str>,
) -> JsonValue {
    let mut evidence = vec![
        JsonValue::object([
            ("kind", JsonValue::string("type")),
            ("status", JsonValue::string("passed")),
            ("expected_type", JsonValue::string(expected.render())),
            ("candidate_type", JsonValue::string(actual.render())),
        ]),
        JsonValue::object([
            ("kind", JsonValue::string("ranking")),
            ("status", JsonValue::string("ranked")),
            ("rank", JsonValue::Number(rank as i64)),
            ("reason", JsonValue::string(reason)),
        ]),
        JsonValue::object([
            ("kind", JsonValue::string("verification")),
            ("status", JsonValue::string("not_run")),
        ]),
    ];
    if let Some(satisfy_status) = satisfy_status {
        let satisfy_reason = if satisfy_status == SATISFY_STATUS_STATICALLY_SATISFIED {
            reason
        } else {
            "not_statically_discharged"
        };
        evidence.push(JsonValue::object([
            ("kind", JsonValue::string("satisfy")),
            ("status", JsonValue::string(satisfy_status)),
            ("reason", JsonValue::string(satisfy_reason)),
        ]));
    }
    JsonValue::array(evidence)
}

pub(crate) fn candidate_known_limits(satisfy_status: Option<&'static str>) -> JsonValue {
    let mut limits = vec![
        JsonValue::string(KNOWN_LIMIT_ADVISORY_UNAPPLIED),
        JsonValue::string(KNOWN_LIMIT_VERIFICATION_NOT_RUN),
    ];
    if satisfy_status == Some(SATISFY_STATUS_BLOCKED_UNTIL_DISCHARGED) {
        limits.push(JsonValue::string(KNOWN_LIMIT_SATISFY_BLOCKED));
    }
    JsonValue::array(limits)
}

pub(crate) fn candidate_blocking_obligations(
    application_policy: &'static str,
    satisfy_status: Option<&'static str>,
) -> JsonValue {
    let mut obligations = Vec::new();
    if application_policy == APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED {
        obligations.push(JsonValue::string(OBLIGATION_MANUAL_REVIEW_REQUIRED));
    }
    if satisfy_status == Some(SATISFY_STATUS_BLOCKED_UNTIL_DISCHARGED) {
        obligations.push(JsonValue::string(OBLIGATION_SATISFY_BLOCKED));
    }
    obligations.push(JsonValue::string(OBLIGATION_VERIFICATION_NOT_RUN));
    JsonValue::array(obligations)
}
