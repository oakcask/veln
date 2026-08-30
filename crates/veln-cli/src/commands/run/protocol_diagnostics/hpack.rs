use super::{Diagnostic, ProtocolDiagnosticContext, note_json, push_byte_preview_note};

struct HpackFixtureObservation {
    header_block_size: i64,
    first_byte: i64,
    expected_fixture: String,
    codec_module: String,
}

impl ProtocolDiagnosticContext<'_> {
    pub(super) fn project_hpack_fixture_rule(&self) -> Option<Diagnostic> {
        let message = match self.id.as_str() {
            "hpack.fixture.unsupported_header_block" => "unsupported HPACK fixture header block",
            "hpack.fixture.unsupported_static_index" => "unsupported HPACK static index",
            "hpack.static.unsupported_index" => "unsupported HPACK static index",
            "hpack.fixture.malformed_string_length" => "malformed HPACK string length",
            "hpack.fixture.malformed_raw_string_value" => "malformed HPACK raw string value",
            "hpack.fixture.malformed_huffman_padding" => "malformed HPACK Huffman padding",
            "hpack.fixture.huffman_eos_symbol" => "HPACK Huffman EOS used as decoded symbol",
            "hpack.fixture.huffman_non_visible_value" => {
                "HPACK Huffman decoded non-visible header value"
            }
            "hpack.fixture.table_size_update_malformed" => {
                "malformed HPACK table-size update integer"
            }
            "hpack.fixture.dynamic_index_out_of_range" => {
                return self.project_hpack_dynamic_index_rule();
            }
            "hpack.fixture.dynamic_name_continuation_missing"
            | "hpack.fixture.dynamic_name_continuation_malformed"
            | "hpack.fixture.dynamic_name_continuation_out_of_range" => {
                return self.project_hpack_dynamic_name_rule();
            }
            "hpack.fixture.table_size_update_not_at_start"
            | "hpack.fixture.table_size_update_trailing_bytes" => {
                return self.project_hpack_table_size_update_rule();
            }
            _ => return None,
        };
        self.project_hpack_fixture_message(message)
    }

    fn project_hpack_fixture_message(&self, message: &str) -> Option<Diagnostic> {
        let observation = self.hpack_fixture_observation()?;
        let mut diagnostic =
            self.diagnostic(format!("{message} at byte offset {}", self.byte_offset));
        if self.id == "hpack.static.unsupported_index" {
            diagnostic.related.push(note_json(format!(
                "HPACK static decoder `{}` observed header block size {} and first byte {}.",
                observation.codec_module, observation.header_block_size, observation.first_byte
            )));
        } else {
            diagnostic.related.push(note_json(format!(
                "HPACK fixture codec `{}` observed header block size {} and first byte {}.",
                observation.codec_module, observation.header_block_size, observation.first_byte
            )));
        }
        Some(self.finish_hpack_fixture_diagnostic(diagnostic, &observation.expected_fixture))
    }

    fn project_hpack_dynamic_index_rule(&self) -> Option<Diagnostic> {
        let observation = self.hpack_fixture_observation()?;
        let requested_index = self.number("requested_dynamic_index")?;
        let entry_count = self.number("dynamic_table_entry_count")?;
        let mut diagnostic = self.diagnostic(format!(
            "HPACK dynamic index out of range at byte offset {}",
            self.byte_offset
        ));
        diagnostic.related.push(note_json(format!(
            "HPACK dynamic index {requested_index} was requested, but the fixture dynamic table currently contains {entry_count} entry/entries."
        )));
        diagnostic.related.push(note_json(format!(
            "HPACK fixture codec `{}` observed header block size {} and first byte {}.",
            observation.codec_module, observation.header_block_size, observation.first_byte
        )));
        Some(self.finish_hpack_fixture_diagnostic(diagnostic, &observation.expected_fixture))
    }

    fn project_hpack_dynamic_name_rule(&self) -> Option<Diagnostic> {
        let observation = self.hpack_fixture_observation()?;
        let requested_index = self.number("requested_dynamic_index")?;
        let entry_count = self.number("dynamic_table_entry_count")?;
        let message = match self.id.as_str() {
            "hpack.fixture.dynamic_name_continuation_missing" => {
                "HPACK dynamic-name continuation is missing a fixture table entry"
            }
            "hpack.fixture.dynamic_name_continuation_malformed" => {
                "HPACK dynamic-name continuation is malformed"
            }
            "hpack.fixture.dynamic_name_continuation_out_of_range" => {
                "HPACK dynamic-name continuation is out of range"
            }
            _ => return None,
        };
        let mut diagnostic =
            self.diagnostic(format!("{message} at byte offset {}", self.byte_offset));
        diagnostic.related.push(note_json(format!(
            "HPACK dynamic-name continuation requested dynamic index {requested_index}, and the fixture dynamic table currently contains {entry_count} entry/entries."
        )));
        diagnostic.related.push(note_json(format!(
            "HPACK fixture codec `{}` observed header block size {} and first byte {}.",
            observation.codec_module, observation.header_block_size, observation.first_byte
        )));
        Some(self.finish_hpack_fixture_diagnostic(diagnostic, &observation.expected_fixture))
    }

    fn project_hpack_table_size_update_rule(&self) -> Option<Diagnostic> {
        let observation = self.hpack_fixture_observation()?;
        let observed_update_size = self.number("observed_header_table_size")?;
        let frame_kind = self.number("frame_kind")?;
        let frame = self.frame_ref()?;
        let active_state = self.string("active_state")?;
        let message = match self.id.as_str() {
            "hpack.fixture.table_size_update_not_at_start" => {
                "HPACK table-size update appears after a header field"
            }
            "hpack.fixture.table_size_update_trailing_bytes" => {
                "HPACK table-size update leaves trailing bytes"
            }
            _ => return None,
        };
        let fact = match self.id.as_str() {
            "hpack.fixture.table_size_update_not_at_start" => "after a decoded header field",
            "hpack.fixture.table_size_update_trailing_bytes" => {
                "before unexpected trailing header-block bytes"
            }
            _ => return None,
        };
        let mut diagnostic =
            self.diagnostic(format!("{message} at byte offset {}", self.byte_offset));
        diagnostic.related.push(note_json(format!(
            "Frame kind {frame_kind} on {} {} requested HPACK header table size {observed_update_size} {fact}.",
            frame.stream_ref, frame.stream_id
        )));
        diagnostic.related.push(note_json(format!(
            "HPACK fixture codec `{}` observed header block size {}, first byte {}, and active state {active_state}.",
            observation.codec_module, observation.header_block_size, observation.first_byte
        )));
        Some(self.finish_hpack_fixture_diagnostic(diagnostic, &observation.expected_fixture))
    }

    fn hpack_fixture_observation(&self) -> Option<HpackFixtureObservation> {
        Some(HpackFixtureObservation {
            header_block_size: self.number("observed_header_block_size")?,
            first_byte: self.number("observed_first_byte")?,
            expected_fixture: self.string("expected_fixture")?,
            codec_module: self.string("codec_module")?,
        })
    }

    fn finish_hpack_fixture_diagnostic(
        &self,
        mut diagnostic: Diagnostic,
        expected_fixture: &str,
    ) -> Diagnostic {
        push_byte_preview_note(&mut diagnostic, self.entries);
        diagnostic
            .related
            .push(note_json(format!("Expected {expected_fixture}.")));
        diagnostic
    }
}
