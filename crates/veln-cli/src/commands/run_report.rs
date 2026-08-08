use std::process::{ExitCode, ExitStatus};

use veln_diagnostics::JsonValue;
use veln_test::TestFailure;

pub(super) fn runtime_error_message(stderr: &str, status: ExitStatus) -> String {
    stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("run process exited with status {status}"))
}

pub(super) struct RunJsonReport {
    status: &'static str,
    exit_code: i32,
    stdout: String,
    stderr: String,
    error: Option<RunJsonError>,
}

impl RunJsonReport {
    pub(super) fn passed(exit_code: i32, stdout: String, stderr: String) -> Self {
        Self {
            status: "passed",
            exit_code,
            stdout,
            stderr,
            error: None,
        }
    }

    pub(super) fn failed(
        exit_code: i32,
        stdout: String,
        stderr: String,
        failure: TestFailure,
    ) -> Self {
        Self {
            status: "failed",
            exit_code,
            stdout,
            stderr,
            error: Some(RunJsonError::from_test_failure(failure)),
        }
    }

    pub(super) fn runtime_error(
        exit_code: i32,
        stdout: String,
        stderr: String,
        message: String,
    ) -> Self {
        let details = invalid_shift_runtime_details(&message)
            .unwrap_or_else(|| JsonValue::object([("phase", JsonValue::string("runtime"))]));
        Self {
            status: "failed",
            exit_code,
            stdout,
            stderr,
            error: Some(RunJsonError::runtime(message, details)),
        }
    }

    pub(super) fn runtime_transport_error(
        exit_code: i32,
        stdout: String,
        _stderr: String,
        failure: TransportFailureTrace,
    ) -> Self {
        let message = format!(
            "transport {} failed: {}",
            failure.operation.replace('_', " "),
            failure.category
        );
        Self {
            status: "failed",
            exit_code,
            stdout,
            stderr: format!("{message}\n"),
            error: Some(RunJsonError::runtime(message, failure.details())),
        }
    }

    pub(super) fn tool_error(message: String) -> Self {
        Self {
            status: "error",
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(RunJsonError::runner(message)),
        }
    }

    pub(super) fn exit_code(&self) -> ExitCode {
        if self.status == "passed" {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        }
    }

    pub(super) fn to_json(&self) -> String {
        JsonValue::object([
            ("schema_version", JsonValue::string("veln-run-json/v0")),
            ("command", JsonValue::string("run")),
            ("status", JsonValue::string(self.status)),
            ("exit_code", JsonValue::Number(self.exit_code.into())),
            ("stdout", JsonValue::string(self.stdout.clone())),
            ("stderr", JsonValue::string(self.stderr.clone())),
            (
                "error",
                self.error
                    .as_ref()
                    .map_or(JsonValue::Null, RunJsonError::to_json),
            ),
        ])
        .to_json()
    }
}

pub(super) struct TransportFailureTrace {
    operation: String,
    category: String,
    local_endpoint: Option<String>,
    peer_endpoint: Option<String>,
    listener_id: Option<String>,
    stream_id: Option<String>,
    lifecycle_phase: String,
    input_committed: Option<bool>,
    output_committed: Option<bool>,
    ownership_committed: Option<bool>,
    platform_cause: Option<String>,
}

impl TransportFailureTrace {
    fn details(&self) -> JsonValue {
        let mut entries = vec![
            ("phase".to_string(), JsonValue::string("runtime")),
            (
                "id".to_string(),
                JsonValue::string("runtime.transport_failure"),
            ),
            (
                "operation".to_string(),
                JsonValue::string(self.operation.clone()),
            ),
            (
                "category".to_string(),
                JsonValue::string(self.category.clone()),
            ),
            (
                "lifecycle_phase".to_string(),
                JsonValue::string(self.lifecycle_phase.clone()),
            ),
        ];
        push_optional_string(&mut entries, "local_endpoint", &self.local_endpoint);
        push_optional_string(&mut entries, "peer_endpoint", &self.peer_endpoint);
        push_optional_string(&mut entries, "listener_id", &self.listener_id);
        push_optional_string(&mut entries, "stream_id", &self.stream_id);
        push_optional_bool(&mut entries, "input_committed", self.input_committed);
        push_optional_bool(&mut entries, "output_committed", self.output_committed);
        push_optional_bool(
            &mut entries,
            "ownership_committed",
            self.ownership_committed,
        );
        push_optional_string(&mut entries, "platform_cause", &self.platform_cause);
        JsonValue::Object(entries)
    }
}

fn push_optional_string(entries: &mut Vec<(String, JsonValue)>, key: &str, value: &Option<String>) {
    if let Some(value) = value {
        entries.push((key.to_string(), JsonValue::string(value.clone())));
    }
}

fn push_optional_bool(entries: &mut Vec<(String, JsonValue)>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        entries.push((key.to_string(), JsonValue::Bool(value)));
    }
}

pub(super) fn transport_failure_from_trace(trace: &str) -> Option<TransportFailureTrace> {
    let fields: Vec<&str> = trace
        .lines()
        .rev()
        .find(|line| line.starts_with("transport\t"))?
        .split('\t')
        .collect();
    if fields.len() != 12 {
        return None;
    }
    Some(TransportFailureTrace {
        operation: trace_string(fields[1])?,
        category: trace_string(fields[2])?,
        local_endpoint: trace_optional_string(fields[3])?,
        peer_endpoint: trace_optional_string(fields[4])?,
        listener_id: trace_optional_string(fields[5])?,
        stream_id: trace_optional_string(fields[6])?,
        lifecycle_phase: trace_string(fields[7])?,
        input_committed: trace_optional_bool(fields[8])?,
        output_committed: trace_optional_bool(fields[9])?,
        ownership_committed: trace_optional_bool(fields[10])?,
        platform_cause: trace_optional_string(fields[11])?,
    })
}

fn trace_optional_string(value: &str) -> Option<Option<String>> {
    if value == "-" {
        Some(None)
    } else {
        trace_string(value).map(Some)
    }
}

fn trace_string(value: &str) -> Option<String> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

fn trace_optional_bool(value: &str) -> Option<Option<bool>> {
    match value {
        "true" => Some(Some(true)),
        "false" => Some(Some(false)),
        "unknown" => Some(None),
        _ => None,
    }
}

struct RunJsonError {
    kind: String,
    message: String,
    details: JsonValue,
}

impl RunJsonError {
    fn from_test_failure(failure: TestFailure) -> Self {
        Self {
            kind: failure.kind,
            message: failure.message,
            details: failure.details,
        }
    }

    fn runtime(message: String, details: JsonValue) -> Self {
        Self {
            kind: "runtime".to_string(),
            message,
            details,
        }
    }

    fn runner(message: String) -> Self {
        Self {
            kind: "runner".to_string(),
            message,
            details: JsonValue::object([("phase", JsonValue::string("tool"))]),
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string(self.kind.clone())),
            ("message", JsonValue::string(self.message.clone())),
            ("details", self.details.clone()),
        ])
    }
}

fn invalid_shift_runtime_details(message: &str) -> Option<JsonValue> {
    let count_text = message.strip_prefix("invalid shift count ")?;
    let (count_text, operator_text) = count_text.split_once(" for operator `")?;
    let (operator, suffix) = operator_text.split_once('`')?;
    if suffix != "; expected a value between 0 and 63" {
        return None;
    }
    let count = count_text.parse::<i64>().ok()?;
    Some(JsonValue::object([
        ("phase", JsonValue::string("runtime")),
        ("id", JsonValue::string("runtime.invalid_shift_count")),
        ("operator", JsonValue::string(operator)),
        ("actual_count", JsonValue::Number(count)),
        ("minimum_count", JsonValue::Number(0)),
        ("maximum_count", JsonValue::Number(63)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace_hex(value: &str) -> String {
        value
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn transport_failure_trace_projects_known_context_and_commit_facts() {
        let trace = format!(
            "transport\t{}\t{}\t{}\t{}\t-\t{}\t{}\tfalse\tfalse\ttrue\t{}\n",
            trace_hex("shutdown"),
            trace_hex("event_record_failed"),
            trace_hex("127.0.0.1:0"),
            trace_hex("fixture-stream"),
            trace_hex("fixture-stream"),
            trace_hex("write_shutdown"),
            trace_hex("could not record VELN_NET_EVENTS"),
        );

        let failure = transport_failure_from_trace(&trace).expect("transport trace should parse");
        assert_eq!(failure.operation, "shutdown");
        assert_eq!(failure.category, "event_record_failed");
        assert_eq!(failure.local_endpoint.as_deref(), Some("127.0.0.1:0"));
        assert_eq!(failure.peer_endpoint.as_deref(), Some("fixture-stream"));
        assert_eq!(failure.listener_id, None);
        assert_eq!(failure.stream_id.as_deref(), Some("fixture-stream"));
        assert_eq!(failure.lifecycle_phase, "write_shutdown");
        assert_eq!(failure.input_committed, Some(false));
        assert_eq!(failure.output_committed, Some(false));
        assert_eq!(failure.ownership_committed, Some(true));
    }

    #[test]
    fn transport_failure_trace_preserves_unknown_facts_as_absent() {
        let trace = format!(
            "transport\t{}\t{}\t-\t-\t-\t-\t{}\tunknown\tunknown\tunknown\t{}\n",
            trace_hex("connect"),
            trace_hex("io_failure"),
            trace_hex("during_operation"),
            trace_hex("production socket failed"),
        );

        let failure = transport_failure_from_trace(&trace).expect("transport trace should parse");
        assert_eq!(failure.local_endpoint, None);
        assert_eq!(failure.peer_endpoint, None);
        assert_eq!(failure.input_committed, None);
        assert_eq!(failure.output_committed, None);
        assert_eq!(failure.ownership_committed, None);
        let details = failure.details().to_json();
        assert!(!details.contains("local_endpoint"));
        assert!(!details.contains("input_committed"));
    }

    #[test]
    fn transport_failure_trace_rejects_malformed_hex_context() {
        let trace = concat!(
            "transport\tzz\t696f5f6661696c757265\t-\t-\t-\t-\t",
            "647572696e675f6f7065726174696f6e\tunknown\tunknown\tunknown\t-\n",
        );

        assert!(transport_failure_from_trace(trace).is_none());
    }
}
