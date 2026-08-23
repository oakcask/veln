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

fn protocol_header_list_message(
    subject: &str,
    failed_fact: &str,
    header_name: &str,
    byte_offset: i64,
) -> String {
    match failed_fact {
        "protocol_on_non_connect_request" => format!(
            "{subject} contains :protocol on a non-CONNECT request at byte offset {byte_offset}"
        ),
        "duplicate_protocol_pseudo_header" => {
            format!("{subject} contains duplicate :protocol at byte offset {byte_offset}")
        }
        "protocol_value_empty" => {
            format!("{subject} contains empty :protocol at byte offset {byte_offset}")
        }
        "extended_connect_scheme_missing" => format!(
            "{subject} is missing required extended CONNECT :scheme at byte offset {byte_offset}"
        ),
        "extended_connect_path_missing" => format!(
            "{subject} is missing required extended CONNECT :path at byte offset {byte_offset}"
        ),
        "extended_connect_authority_missing" => format!(
            "{subject} is missing required extended CONNECT :authority at byte offset {byte_offset}"
        ),
        "extended_connect_not_negotiated" => format!(
            "{subject} uses extended CONNECT before negotiation at byte offset {byte_offset}"
        ),
        "connect_authority_missing" => {
            format!("{subject} is missing required CONNECT :authority at byte offset {byte_offset}")
        }
        "connect_authority_empty" => {
            format!("{subject} contains empty CONNECT :authority at byte offset {byte_offset}")
        }
        "connect_scheme_present" => {
            format!("{subject} contains forbidden CONNECT :scheme at byte offset {byte_offset}")
        }
        "connect_path_present" => {
            format!("{subject} contains forbidden CONNECT :path at byte offset {byte_offset}")
        }
        "missing_required_pseudo_header" => {
            format!("{subject} is missing {header_name} at byte offset {byte_offset}")
        }
        "response_only_pseudo_header" => {
            format!("{subject} contains response-only {header_name} at byte offset {byte_offset}")
        }
        "request_only_pseudo_header" => {
            format!("{subject} contains request-only {header_name} at byte offset {byte_offset}")
        }
        "duplicate_pseudo_header" => {
            format!("{subject} contains duplicate {header_name} at byte offset {byte_offset}")
        }
        "trailer_pseudo_header" => {
            format!("{subject} contains pseudo-header {header_name} at byte offset {byte_offset}")
        }
        "pseudo_header_after_regular_header" => format!(
            "{subject} places {header_name} after a regular header at byte offset {byte_offset}"
        ),
        "ordinary_header_name_not_lowercase" => format!(
            "{subject} contains uppercase ordinary header {header_name} at byte offset {byte_offset}"
        ),
        "ordinary_header_name_invalid_token" => format!(
            "{subject} contains invalid ordinary header name {header_name} at byte offset {byte_offset}"
        ),
        "connection_specific_header" => format!(
            "{subject} contains connection-specific header {header_name} at byte offset {byte_offset}"
        ),
        "te_header_value_not_trailers" => {
            format!("{subject} contains te value other than trailers at byte offset {byte_offset}")
        }
        "method_value_empty" => {
            format!("{subject} contains empty :method at byte offset {byte_offset}")
        }
        "scheme_value_not_http_or_https" => format!(
            "{subject} contains :scheme value other than http or https at byte offset {byte_offset}"
        ),
        "path_value_empty" => {
            format!("{subject} contains empty :path at byte offset {byte_offset}")
        }
        "authority_value_invalid" => {
            format!("{subject} contains invalid :authority at byte offset {byte_offset}")
        }
        "content_length_invalid" => {
            format!("{subject} contains invalid content-length at byte offset {byte_offset}")
        }
        "content_length_mismatch" => format!(
            "{subject} contains mismatched content-length values at byte offset {byte_offset}"
        ),
        "switching_protocols_status_forbidden" => {
            format!("{subject} uses switching protocols status at byte offset {byte_offset}")
        }
        "informational_response_end_stream" => {
            format!("informational response ended the stream at byte offset {byte_offset}")
        }
        _ => format!("invalid {subject} at byte offset {byte_offset}"),
    }
}
