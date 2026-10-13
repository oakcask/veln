use super::*;

#[test]
fn surface_wire_discards_legacy_codec_declarations_without_shifting_following_fields() {
    let canonical = lower_source("fn main() -> ()\n  ()\nend\n");
    let mut legacy = encode_surface_module(&canonical);
    let empty_codec_slot = legacy_codec_slot_offset(&legacy);
    legacy.splice(
        empty_codec_slot..empty_codec_slot + 4,
        legacy_codec_declaration_slot(),
    );

    let decoded = decode_surface_module(&legacy).expect("legacy codec slot should decode");

    assert_eq!(decoded.functions.len(), 1);
    assert_eq!(decoded.functions[0].name.as_deref(), Some("main"));
    assert_eq!(
        encode_surface_module(&decoded),
        encode_surface_module(&canonical)
    );
}

fn legacy_codec_slot_offset(encoded: &[u8]) -> usize {
    let header_len = encoded
        .iter()
        .position(|byte| *byte == b'\n')
        .expect("wire header should end with a newline")
        + 1;
    let header_and_empty_prefix_fields = header_len + 1 + 6 * 4;
    assert_eq!(
        &encoded[header_and_empty_prefix_fields..header_and_empty_prefix_fields + 4],
        0_u32.to_le_bytes()
    );
    header_and_empty_prefix_fields
}

fn legacy_codec_declaration_slot() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_legacy_codec_identity(&mut bytes);
    push_legacy_codec_shape(&mut bytes);
    push_span(&mut bytes);
    bytes
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_span(bytes: &mut Vec<u8>) {
    push_string(bytes, "legacy.veln");
    for value in [1_u64, 1, 0, 4, 1, 3] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.push(0);
}

fn push_legacy_codec_identity(bytes: &mut Vec<u8>) {
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&7_u32.to_le_bytes());
    bytes.push(0);
    bytes.push(0);
    bytes.push(1);
    push_string(bytes, "LegacyCodec");
    bytes.push(1);
    push_string(bytes, "Packet");
}

fn push_legacy_codec_shape(bytes: &mut Vec<u8>) {
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.push(0);
    bytes.push(1);
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    bytes.push(0);
    bytes.push(0);
    push_span(bytes);
    bytes.extend_from_slice(&9_u32.to_le_bytes());
    bytes.push(1);
    bytes.push(1);
    bytes.push(1);
    push_string(bytes, "encode_packet");
    push_span(bytes);
}
