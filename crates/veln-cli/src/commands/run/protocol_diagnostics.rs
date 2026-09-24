mod connection;
mod hpack;
mod streams;

use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_test::TestFailure;

use super::{
    byte_offset_value, json_field, json_number, json_object, json_string, note_json,
    push_byte_preview_note,
};

pub(super) fn protocol_result_failure_diagnostic(failure: &TestFailure) -> Option<Diagnostic> {
    ProtocolDiagnosticContext::from_failure(failure)?.project()
}

struct ProtocolDiagnosticContext<'a> {
    source: &'a JsonValue,
    entries: &'a [(String, JsonValue)],
    id: String,
    byte_offset: i64,
}

impl<'a> ProtocolDiagnosticContext<'a> {
    fn from_failure(failure: &'a TestFailure) -> Option<Self> {
        let details = json_object(&failure.details)?;
        let source = json_field(details, "protocol_diagnostic")?;
        let entries = json_object(source)?;
        Some(Self {
            source,
            entries,
            id: json_string(entries, "id")?,
            byte_offset: byte_offset_value(entries)?,
        })
    }

    fn project(&self) -> Option<Diagnostic> {
        self.project_connection_rule()
            .or_else(|| self.project_header_list_rule())
            .or_else(|| self.project_stream_rule())
            .or_else(|| self.project_peer_limit_rule())
            .or_else(|| self.project_hpack_fixture_rule())
    }

    fn project_header_list_rule(&self) -> Option<Diagnostic> {
        let (decoded_label, message) = match self.id.as_str() {
            "http2.protocol.invalid_request_header_list" => (
                "request header",
                protocol_header_list_message(
                    "request header list",
                    &self.string("failed_header_fact")?,
                    &self.string("header_name")?,
                    self.byte_offset,
                ),
            ),
            "http2.protocol.invalid_response_header_list" => {
                let active_state = self.string("active_state")?;
                let subject = if active_state == "response-trailers" {
                    "response trailer list"
                } else {
                    "response header list"
                };
                let decoded_label = if active_state == "response-trailers" {
                    "response trailer"
                } else {
                    "response header"
                };
                (
                    decoded_label,
                    protocol_header_list_message(
                        subject,
                        &self.string("failed_header_fact")?,
                        &self.string("header_name")?,
                        self.byte_offset,
                    ),
                )
            }
            _ => return None,
        };
        let frame_kind = self.number("frame_kind")?;
        let decoded_header_names = self.string("decoded_header_names")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(message);
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} decoded {decoded_label} names: {decoded_header_names}.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn diagnostic(&self, message: String) -> Diagnostic {
        Diagnostic::new(
            self.id.clone(),
            Severity::Error,
            DiagnosticKind::Runtime,
            message,
            None,
            self.source.clone(),
        )
    }

    fn frame_ref(&self) -> Option<ProtocolFrameRef> {
        Some(ProtocolFrameRef {
            stream_id: self.number("stream_id")?,
            stream_ref: self.string("stream_ref")?,
        })
    }

    fn number(&self, key: &str) -> Option<i64> {
        json_number(self.entries, key)
    }

    fn string(&self, key: &str) -> Option<String> {
        json_string(self.entries, key)
    }

    fn push_preview_state_and_provenance(&self, diagnostic: &mut Diagnostic) -> Option<()> {
        push_byte_preview_note(diagnostic, self.entries);
        self.push_state_and_provenance(diagnostic)
    }

    fn push_state_and_provenance(&self, diagnostic: &mut Diagnostic) -> Option<()> {
        self.push_active_state(diagnostic)?;
        self.push_rule_provenance(diagnostic)
    }

    fn push_active_state(&self, diagnostic: &mut Diagnostic) -> Option<()> {
        diagnostic.related.push(note_json(format!(
            "Active protocol state: {}.",
            self.string("active_state")?
        )));
        Some(())
    }

    fn push_rule_provenance(&self, diagnostic: &mut Diagnostic) -> Option<()> {
        diagnostic.related.push(note_json(format!(
            "Rule provenance: {}.",
            self.string("rule_provenance")?
        )));
        Some(())
    }
}

struct ProtocolFrameRef {
    stream_id: i64,
    stream_ref: String,
}

const HEADER_LIST_FIXED_FACT_DETAILS: &[(&str, &str)] = &[
    (
        "protocol_on_non_connect_request",
        "contains :protocol on a non-CONNECT request",
    ),
    (
        "duplicate_protocol_pseudo_header",
        "contains duplicate :protocol",
    ),
    ("protocol_value_empty", "contains empty :protocol"),
    (
        "extended_connect_scheme_missing",
        "is missing required extended CONNECT :scheme",
    ),
    (
        "extended_connect_path_missing",
        "is missing required extended CONNECT :path",
    ),
    (
        "extended_connect_authority_missing",
        "is missing required extended CONNECT :authority",
    ),
    (
        "extended_connect_not_negotiated",
        "uses extended CONNECT before negotiation",
    ),
    (
        "connect_authority_missing",
        "is missing required CONNECT :authority",
    ),
    (
        "connect_authority_empty",
        "contains empty CONNECT :authority",
    ),
    (
        "connect_scheme_present",
        "contains forbidden CONNECT :scheme",
    ),
    ("connect_path_present", "contains forbidden CONNECT :path"),
    (
        "te_header_value_not_trailers",
        "contains te value other than trailers",
    ),
    ("method_value_empty", "contains empty :method"),
    (
        "scheme_value_not_http_or_https",
        "contains :scheme value other than http or https",
    ),
    ("path_value_empty", "contains empty :path"),
    ("authority_value_invalid", "contains invalid :authority"),
    ("content_length_invalid", "contains invalid content-length"),
    (
        "content_length_mismatch",
        "contains mismatched content-length values",
    ),
    (
        "switching_protocols_status_forbidden",
        "uses switching protocols status",
    ),
];

const HEADER_LIST_NAMED_FACT_DETAILS: &[(&str, &str, &str)] = &[
    ("missing_required_pseudo_header", "is missing ", ""),
    ("response_only_pseudo_header", "contains response-only ", ""),
    ("request_only_pseudo_header", "contains request-only ", ""),
    ("duplicate_pseudo_header", "contains duplicate ", ""),
    ("trailer_pseudo_header", "contains pseudo-header ", ""),
    (
        "pseudo_header_after_regular_header",
        "places ",
        " after a regular header",
    ),
    (
        "ordinary_header_name_not_lowercase",
        "contains uppercase ordinary header ",
        "",
    ),
    (
        "ordinary_header_name_invalid_token",
        "contains invalid ordinary header name ",
        "",
    ),
    (
        "connection_specific_header",
        "contains connection-specific header ",
        "",
    ),
];

pub(super) fn protocol_header_list_message(
    subject: &str,
    failed_fact: &str,
    header_name: &str,
    byte_offset: i64,
) -> String {
    if failed_fact == "informational_response_end_stream" {
        return format!("informational response ended the stream at byte offset {byte_offset}");
    }

    if let Some(detail) = HEADER_LIST_FIXED_FACT_DETAILS
        .iter()
        .find_map(|(fact, detail)| (*fact == failed_fact).then_some(*detail))
    {
        return format!("{subject} {detail} at byte offset {byte_offset}");
    }
    if let Some((prefix, suffix)) = HEADER_LIST_NAMED_FACT_DETAILS
        .iter()
        .find_map(|(fact, prefix, suffix)| (*fact == failed_fact).then_some((*prefix, *suffix)))
    {
        return format!("{subject} {prefix}{header_name}{suffix} at byte offset {byte_offset}");
    }
    format!("invalid {subject} at byte offset {byte_offset}")
}
