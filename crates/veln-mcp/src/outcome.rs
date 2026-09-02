use serde_json::Value;

pub(crate) enum ToolOutcome {
    Success(Value),
    DomainFailure {
        code: &'static str,
        message: &'static str,
        details: Value,
    },
}

pub(crate) fn domain_failure(
    code: &'static str,
    message: &'static str,
    details: Value,
) -> ToolOutcome {
    ToolOutcome::DomainFailure {
        code,
        message,
        details,
    }
}
