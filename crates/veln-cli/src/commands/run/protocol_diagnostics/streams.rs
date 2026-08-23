use super::{Diagnostic, ProtocolDiagnosticContext, note_json, push_byte_preview_note};

impl ProtocolDiagnosticContext<'_> {
    pub(super) fn project_stream_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.protocol.invalid_priority_dependency" => {
                let frame_kind = self.number("frame_kind")?;
                let dependency_stream_id = self.number("dependency_stream_id")?;
                let frame = self.frame_ref()?;
                let mut diagnostic = self.diagnostic(format!(
                    "invalid PRIORITY dependency at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} declared itself as dependency stream {dependency_stream_id}.",
                    frame.stream_ref, frame.stream_id
                )));
                self.push_preview_state_and_provenance(&mut diagnostic)?;
                Some(diagnostic)
            }
            "http2.protocol.stream_after_goaway" => {
                let frame = self.frame_ref()?;
                let last_stream_id = self.number("last_stream_id")?;
                let shutdown_state = self.string("shutdown_state")?;
                let endpoint_role = self.string("endpoint_role")?;
                let mut diagnostic = self.diagnostic(format!(
                    "stream opened after graceful shutdown at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Peer opened {} {}; graceful shutdown recorded last stream id {last_stream_id}.",
                    frame.stream_ref, frame.stream_id
                )));
                diagnostic.related.push(note_json(format!(
                    "Active shutdown state: {shutdown_state}."
                )));
                diagnostic
                    .related
                    .push(note_json(format!("Endpoint role: {endpoint_role}.")));
                self.push_preview_state_and_provenance(&mut diagnostic)?;
                Some(diagnostic)
            }
            _ => None,
        }
    }

    pub(super) fn project_peer_limit_rule(&self) -> Option<Diagnostic> {
        self.project_peer_size_limit_rule()
            .or_else(|| self.project_peer_flow_limit_rule())
            .or_else(|| self.project_peer_settings_limit_rule())
    }

    fn project_peer_size_limit_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.peer_limit.frame_size_exceeded" => {
                let observed_length = self.number("observed_payload_length")?;
                let allowed_length = self.number("allowed_max_frame_size")?;
                let frame_kind = self.number("frame_kind")?;
                let frame = self.frame_ref()?;
                let provenance = self.string("receive_limit_provenance")?;
                let mut diagnostic = self.diagnostic(format!(
                    "frame payload length exceeds receive maximum at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} declared {observed_length} byte(s); active receive maximum is {allowed_length} byte(s).",
                    frame.stream_ref, frame.stream_id
                )));
                push_byte_preview_note(&mut diagnostic, self.entries);
                diagnostic.related.push(note_json(format!(
                    "Receive limit provenance: {provenance}."
                )));
                Some(diagnostic)
            }
            "http2.peer_limit.header_list_size_exceeded" => self.project_size_limit_rule(
                "observed_header_list_size",
                "allowed_header_list_size",
                "header list size exceeds receive maximum",
                "decoded header list size",
            ),
            "http2.peer_limit.header_table_size_exceeded" => self.project_size_limit_rule(
                "observed_header_table_size",
                "allowed_header_table_size",
                "header table size exceeds receive maximum",
                "requested HPACK header table size",
            ),
            _ => None,
        }
    }

    fn project_peer_flow_limit_rule(&self) -> Option<Diagnostic> {
        match self.id.as_str() {
            "http2.peer_limit.flow_control_window_exceeded" => {
                let observed_length = self.number("observed_payload_length")?;
                let allowed_credit = self.number("allowed_window_credit")?;
                let frame_kind = self.number("frame_kind")?;
                let frame = self.frame_ref()?;
                let mut diagnostic = self.diagnostic(format!(
                    "flow-control window exceeded at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Frame kind {frame_kind} on {} {} declared {observed_length} byte(s); available receive window credit is {allowed_credit} byte(s).",
                    frame.stream_ref, frame.stream_id
                )));
                self.push_preview_state_and_provenance(&mut diagnostic)?;
                Some(diagnostic)
            }
            "http2.peer_limit.concurrent_streams_exceeded" => {
                let frame = self.frame_ref()?;
                let current_count = self.number("current_open_peer_created_stream_count")?;
                let attempted_count = self.number("attempted_concurrent_stream_count")?;
                let allowed_count = self.number("allowed_concurrent_stream_count")?;
                let endpoint_role = self.string("endpoint_role")?;
                let limit_provenance = self.string("receive_limit_provenance")?;
                let rule_provenance = self.string("rule_provenance")?;
                let mut diagnostic = self.diagnostic(format!(
                    "concurrent stream receive limit exceeded at byte offset {}",
                    self.byte_offset
                ));
                diagnostic.related.push(note_json(format!(
                    "Opening {} {} would make {attempted_count} concurrent peer-created stream(s); {current_count} peer-created stream(s) are currently open and the active receive limit is {allowed_count}.",
                    frame.stream_ref, frame.stream_id
                )));
                push_byte_preview_note(&mut diagnostic, self.entries);
                self.push_active_state(&mut diagnostic)?;
                diagnostic
                    .related
                    .push(note_json(format!("Endpoint role: {endpoint_role}.")));
                diagnostic.related.push(note_json(format!(
                    "Receive limit provenance: {limit_provenance}."
                )));
                diagnostic
                    .related
                    .push(note_json(format!("Rule provenance: {rule_provenance}.")));
                Some(diagnostic)
            }
            _ => None,
        }
    }

    fn project_peer_settings_limit_rule(&self) -> Option<Diagnostic> {
        if self.id != "http2.peer_limit.settings_value_out_of_range" {
            return None;
        }
        let setting_identifier = self.number("setting_identifier")?;
        let setting_name = self.string("setting_name")?;
        let observed_value = self.number("observed_value")?;
        let accepted_min_value = self.number("accepted_min_value")?;
        let accepted_max_value = self.number("accepted_max_value")?;
        let provenance = self.string("peer_limit_provenance")?;
        let mut diagnostic = self.diagnostic(format!(
            "SETTINGS value outside accepted range at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "{setting_name} ({setting_identifier}) declared {observed_value}; accepted range is {accepted_min_value}..{accepted_max_value}."
        )));
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Peer limit provenance: {provenance}.")));
        Some(diagnostic)
    }

    fn project_size_limit_rule(
        &self,
        observed_key: &str,
        allowed_key: &str,
        message: &str,
        observed_label: &str,
    ) -> Option<Diagnostic> {
        let observed_size = self.number(observed_key)?;
        let allowed_size = self.number(allowed_key)?;
        let frame_kind = self.number("frame_kind")?;
        let frame = self.frame_ref()?;
        let limit_provenance = self.string("receive_limit_provenance")?;
        let rule_provenance = self.string("rule_provenance")?;
        let peer_outbound = limit_provenance == "peer_settings_item";
        let message = if peer_outbound {
            "header list size exceeds peer-advertised outbound maximum"
        } else {
            message
        };
        let maximum_label = if peer_outbound {
            "active peer-advertised outbound maximum"
        } else {
            "active receive maximum"
        };
        let provenance_label = if peer_outbound {
            "Peer setting provenance"
        } else {
            "Receive limit provenance"
        };
        let mut diagnostic =
            self.diagnostic(format!("{message} at byte offset {}", self.byte_offset));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} {observed_label} {observed_size}; {maximum_label} is {allowed_size}.",
            frame.stream_ref, frame.stream_id
        )));
        diagnostic.related.push(note_json(format!(
            "{provenance_label}: {limit_provenance}."
        )));
        diagnostic
            .related
            .push(note_json(format!("Rule provenance: {rule_provenance}.")));
        push_byte_preview_note(&mut diagnostic, self.entries);
        Some(diagnostic)
    }
}
