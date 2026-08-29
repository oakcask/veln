use super::*;

pub(super) fn fields_digest(fields: &BTreeMap<String, String>) -> String {
    let mut bytes = Vec::new();
    for (path, value) in fields {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(b'\n');
    }
    sha256(&bytes)
}
pub(super) fn aggregate_digest(cases: &BTreeMap<String, BTreeMap<String, String>>) -> String {
    let mut bytes = Vec::new();
    for (id, fields) in cases {
        bytes.extend_from_slice(id.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(fields_digest(fields).as_bytes());
        bytes.push(b'\n');
    }
    sha256(&bytes)
}
pub(super) fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(super) fn line(output: &mut String, kind: &str, value: &str) {
    output.push_str(kind);
    output.push('\t');
    output.push_str(value);
    output.push('\n');
}
pub(super) fn json_string(value: &str) -> String {
    format!("\"{}\"", escape_json_string(value))
}

pub(super) fn parse_json_string(value: &str, line_number: usize) -> Result<String, String> {
    parse_json(value)
        .map_err(|error| format!("baseline line {line_number} has invalid JSON string: {error}"))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("baseline line {line_number} value is not a JSON string"))
}
