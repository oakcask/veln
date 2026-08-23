use super::{Diagnostic, ProtocolDiagnosticContext, note_json, push_byte_preview_note};

impl ProtocolDiagnosticContext<'_> {
    pub(super) fn project_connection_rule(&self) -> Option<Diagnostic> {
        self.project_connection_lifecycle_rule()
            .or_else(|| self.project_frame_shape_rule())
    }

    fn project_connection_lifecycle_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.protocol.closed_with_pending" => self.project_closed_with_pending(),
            "http2.protocol.partial_preface" => self.project_partial_preface(),
            "http2.protocol.invalid_preface" => self.project_invalid_preface(),
            "http2.protocol.continuation_expected" => self.project_continuation_expected(),
            _ => None,
        }
    }

    fn project_closed_with_pending(&self) -> Option<Diagnostic> {
        let pending_count = self.number("pending_count")?;
        let active_continuation = self.string("active_continuation")?;
        let expected_stream = self.number("expected_stream_id")?;
        let started_kind = self.number("started_frame_kind")?;
        let started_offset = self.number("started_byte_offset")?;
        let accumulated = self.number("accumulated_header_block_bytes")?;
        let rule_provenance = self.string("rule_provenance")?;
        let mut diagnostic = self.diagnostic(format!(
            "input ended with pending bytes at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Input end arrived while {pending_count} byte(s) remained undecoded."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic.related.push(note_json(format!(
            "Active continuation state: {active_continuation}."
        )));
        if active_continuation != "none" {
            diagnostic.related.push(note_json(format!(
                "Pending header block started with frame kind {started_kind} at byte offset {started_offset} for stream {expected_stream}; accumulated {accumulated} header-block byte(s)."
            )));
            diagnostic
                .related
                .push(note_json(format!("Rule provenance: {rule_provenance}.")));
        }
        Some(diagnostic)
    }

    fn project_partial_preface(&self) -> Option<Diagnostic> {
        let pending_count = self.number("pending_count")?;
        let expected_count = self.number("expected_count")?;
        let mut diagnostic = self.diagnostic(format!(
            "input ended with partial client connection preface at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Input end arrived after {pending_count} of {expected_count} preface byte(s)."
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_preface(&self) -> Option<Diagnostic> {
        let expected_byte = self.number("expected_byte")?;
        let actual_byte = self.number("actual_byte")?;
        let matched_count = self.number("matched_prefix_count")?;
        let expected_count = self.number("expected_count")?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid client connection preface at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Observed byte {actual_byte}; expected byte {expected_byte} after {matched_count} of {expected_count} preface byte(s)."
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_continuation_expected(&self) -> Option<Diagnostic> {
        let actual_kind = self.number("actual_frame_kind")?;
        let actual_stream = self.number("actual_stream_id")?;
        let expected_stream = self.number("expected_stream_id")?;
        let started_kind = self.number("started_frame_kind")?;
        let started_offset = self.number("started_byte_offset")?;
        let active_continuation = self.string("active_continuation")?;
        let accumulated = self.number("accumulated_header_block_bytes")?;
        let rule_provenance = self.string("rule_provenance")?;
        let mut diagnostic = self.diagnostic(format!(
            "expected CONTINUATION frame at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Incoming frame kind {actual_kind} on stream {actual_stream} violated active continuation state `{active_continuation}`."
        )));
        diagnostic.related.push(note_json(format!(
            "Pending header block started with frame kind {started_kind} at byte offset {started_offset} for stream {expected_stream}; accumulated {accumulated} header-block byte(s)."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Rule provenance: {rule_provenance}.")));
        Some(diagnostic)
    }

    fn project_frame_shape_rule(&self) -> Option<Diagnostic> {
        self.project_frame_identity_rule()
            .or_else(|| self.project_frame_payload_rule())
            .or_else(|| self.project_settings_ack_rule())
    }

    fn project_frame_identity_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.protocol.initial_peer_settings_required" => {
                self.project_initial_peer_settings_required()
            }
            "http2.protocol.invalid_frame_kind" => self.project_invalid_frame_kind(),
            "http2.protocol.invalid_stream_id" => self.project_invalid_stream_id(),
            "http2.protocol.settings_not_allowed_for_endpoint" => {
                self.project_settings_not_allowed_for_endpoint()
            }
            "http2.protocol.peer_stream_id_not_increasing" => {
                self.project_peer_stream_id_not_increasing()
            }
            _ => None,
        }
    }

    fn project_initial_peer_settings_required(&self) -> Option<Diagnostic> {
        let actual_kind = self.number("actual_frame_kind")?;
        let actual_flags = self.number("actual_flags")?;
        let endpoint_role = self.string("endpoint_role")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "initial peer frame must be non-ACK SETTINGS at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {actual_kind} with flags {actual_flags} on {} {} cannot start a {endpoint_role} endpoint connection.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_frame_kind(&self) -> Option<Diagnostic> {
        let actual_kind = self.number("actual_frame_kind")?;
        let expected_kind = self.number("expected_frame_kind")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid frame kind at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {actual_kind} on {} {} did not match expected frame kind {expected_kind}.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_stream_id(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let required_domain = self.string("required_stream_id_domain")?;
        let endpoint_role = self.string("endpoint_role")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid stream id at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} requires {required_domain} for {endpoint_role}.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_settings_not_allowed_for_endpoint(&self) -> Option<Diagnostic> {
        let setting_identifier = self.number("setting_identifier")?;
        let setting_name = self.string("setting_name")?;
        let endpoint_role = self.string("endpoint_role")?;
        let frame_kind = self.number("frame_kind")?;
        let mut diagnostic = self.diagnostic(format!(
            "{setting_name} is not allowed for {endpoint_role} endpoints at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "{setting_name} ({setting_identifier}) appeared in frame kind {frame_kind}."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Endpoint role: {endpoint_role}.")));
        self.push_active_state(&mut diagnostic)?;
        let rule_provenance = self.string("rule_provenance")?;
        diagnostic
            .related
            .push(note_json(format!("Rule provenance: {rule_provenance}.")));
        Some(diagnostic)
    }

    fn project_peer_stream_id_not_increasing(&self) -> Option<Diagnostic> {
        let frame = self.frame_ref()?;
        let previous_stream_id = self.number("previous_peer_stream_id")?;
        let endpoint_role = self.string("endpoint_role")?;
        let mut diagnostic = self.diagnostic(format!(
            "peer-created stream id {} is not greater than {previous_stream_id} at byte offset {}",
            frame.stream_id, self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "The {endpoint_role} endpoint attempted to create idle stream {} after peer-created stream {previous_stream_id}.",
            frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        diagnostic.related.push(note_json(format!(
            "Use a new peer-created stream id greater than {previous_stream_id}."
        )));
        Some(diagnostic)
    }

    fn project_frame_payload_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.protocol.invalid_payload_length" => self.project_invalid_payload_length(),
            "http2.protocol.invalid_window_update_increment" => {
                self.project_invalid_window_update_increment()
            }
            "http2.protocol.invalid_data_padding" => self.project_invalid_data_padding(),
            "http2.protocol.content_length_mismatch" => self.project_content_length_mismatch(),
            _ => None,
        }
    }

    fn project_invalid_payload_length(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let observed_length = self.number("observed_payload_length")?;
        let expected_length = self.number("expected_payload_length")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid payload length at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} declared {observed_length} byte(s); expected {expected_length} byte(s).",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_window_update_increment(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let observed_increment = self.number("observed_window_increment")?;
        let accepted_min = self.number("accepted_min_window_increment")?;
        let accepted_max = self.number("accepted_max_window_increment")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid WINDOW_UPDATE increment at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} declared WINDOW_UPDATE increment {observed_increment}; accepted range is {accepted_min}..{accepted_max}.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_invalid_data_padding(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let pad_length = self.number("pad_length")?;
        let remaining_payload_length = self.number("remaining_payload_length")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "invalid DATA padding at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} declared pad length {pad_length} byte(s); remaining payload length is {remaining_payload_length} byte(s).",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_content_length_mismatch(&self) -> Option<Diagnostic> {
        let frame_kind = self.number("frame_kind")?;
        let expected_length = self.number("expected_content_length")?;
        let observed_length = self.number("observed_body_length")?;
        let active_state = self.string("active_state")?;
        let rule_provenance = self.string("rule_provenance")?;
        let frame = self.frame_ref()?;
        let no_content_status = match active_state.as_str() {
            "no-content-response-204" => Some("204"),
            "no-content-response-304" => Some("304"),
            _ => None,
        };
        let mut diagnostic = match no_content_status {
            Some(status) if rule_provenance == "rfc9110_no_content_response_body" => {
                let mut diagnostic = self.diagnostic(format!(
                    "response status {status} prohibits nonempty DATA at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} contributed {observed_length} DATA application byte(s); response status {status} permits no application content.",
                    frame.stream_ref, frame.stream_id
                )));
                diagnostic
            }
            _ => {
                let mut diagnostic = self.diagnostic(format!(
                    "content-length body length mismatch at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} observed {observed_length} DATA application byte(s); accepted content-length is {expected_length} byte(s).",
                    frame.stream_ref, frame.stream_id
                )));
                diagnostic
            }
        };
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }

    fn project_settings_ack_rule(&self) -> Option<Diagnostic> {
        if self.id != "http2.protocol.unexpected_settings_ack" {
            return None;
        }
        let frame_kind = self.number("frame_kind")?;
        let frame = self.frame_ref()?;
        let mut diagnostic = self.diagnostic(format!(
            "unexpected SETTINGS ACK at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} acknowledged local SETTINGS, but no local SETTINGS batch is outstanding.",
            frame.stream_ref, frame.stream_id
        )));
        self.push_preview_state_and_provenance(&mut diagnostic)?;
        Some(diagnostic)
    }
}
