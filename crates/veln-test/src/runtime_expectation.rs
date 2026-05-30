use veln_diagnostics::JsonValue;
use veln_source::SourceSpan;

use crate::{TestCase, TestCaseStatus, TestFailure, json_object_field, source_span_to_json};

pub fn apply_runtime_result(case: &mut TestCase, actual_failure: Option<TestFailure>) {
    let Some(expected) = &case.expected_runtime_failure else {
        if let Some(failure) = actual_failure {
            case.status = TestCaseStatus::Failed;
            case.failure = Some(failure);
        } else {
            case.status = TestCaseStatus::Passed;
        }
        return;
    };

    match actual_failure {
        Some(failure) if expected.matches(&failure) => {
            case.status = TestCaseStatus::Passed;
            case.reason = None;
            case.failure = None;
        }
        Some(failure) => {
            case.status = TestCaseStatus::Failed;
            case.reason = Some("expected_runtime_failure".to_string());
            case.failure = Some(TestFailure::runtime_expectation(
                "runtime failure did not match expectation",
                expected,
                Some(failure),
            ));
        }
        None => {
            case.status = TestCaseStatus::Failed;
            case.reason = Some("expected_runtime_failure".to_string());
            case.failure = Some(TestFailure::runtime_expectation(
                "expected runtime failure did not occur",
                expected,
                None,
            ));
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExpectedRuntimeFailure {
    Contract(ExpectedContractFailure),
    Result(ExpectedResultFailure),
}

#[derive(Clone, Debug)]
pub struct ExpectedContractFailure {
    pub clause: String,
    pub predicate: String,
    pub function: Option<String>,
    pub blame: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct ExpectedResultFailure {
    pub value: String,
    pub span: SourceSpan,
}

impl ExpectedRuntimeFailure {
    fn matches(&self, failure: &TestFailure) -> bool {
        match self {
            ExpectedRuntimeFailure::Contract(expected) => {
                if failure.kind != "contract" {
                    return false;
                }
                json_object_field(&failure.details, "kind") == Some("contract")
                    && json_object_field(&failure.details, "phase") == Some("runtime")
                    && json_object_field(&failure.details, "clause")
                        == Some(expected.clause.as_str())
                    && json_object_field(&failure.details, "predicate")
                        == Some(expected.predicate.as_str())
                    && expected.function.as_deref().is_none_or(|function| {
                        json_object_field(&failure.details, "function") == Some(function)
                    })
                    && expected.blame.as_deref().is_none_or(|blame| {
                        json_object_field(&failure.details, "blame") == Some(blame)
                    })
            }
            ExpectedRuntimeFailure::Result(expected) => {
                failure.kind == "result"
                    && json_object_field(&failure.details, "kind") == Some("result")
                    && json_object_field(&failure.details, "phase") == Some("runtime")
                    && json_object_field(&failure.details, "value") == Some(expected.value.as_str())
            }
        }
    }

    pub(crate) fn to_json(&self) -> JsonValue {
        match self {
            ExpectedRuntimeFailure::Contract(expected) => {
                let mut fields = vec![
                    ("kind", JsonValue::string("contract")),
                    ("span", source_span_to_json(&expected.span)),
                    ("clause", JsonValue::string(expected.clause.clone())),
                    ("predicate", JsonValue::string(expected.predicate.clone())),
                ];
                if let Some(function) = &expected.function {
                    fields.push(("function", JsonValue::string(function.clone())));
                }
                if let Some(blame) = &expected.blame {
                    fields.push(("blame", JsonValue::string(blame.clone())));
                }
                JsonValue::object(fields)
            }
            ExpectedRuntimeFailure::Result(expected) => JsonValue::object([
                ("kind", JsonValue::string("result")),
                ("span", source_span_to_json(&expected.span)),
                ("value", JsonValue::string(expected.value.clone())),
            ]),
        }
    }
}
