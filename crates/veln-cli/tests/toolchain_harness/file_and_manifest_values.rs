use super::*;

pub(super) fn jsonrpc_fixture_error(
    manifest_path: &Path,
    line_number: usize,
    message_index: usize,
    position: &str,
    fact: &str,
) -> ! {
    manifest_error(
        manifest_path,
        line_number,
        format!("JSON-RPC fixture message {message_index} at {position}: {fact}"),
    )
}

pub(super) fn read_case_text_file_path(
    path: &Path,
    line_number: usize,
    relative: &str,
    relative_path: &Path,
    context: Option<&str>,
) -> String {
    let base = path.parent().unwrap_or_else(|| Path::new(""));
    let resolved =
        resolve_case_file_reference(path, line_number, base, relative, relative_path, context);
    fs::read_to_string(&resolved).unwrap_or_else(|error| {
        manifest_error(
            path,
            line_number,
            format!(
                "failed to read case file `{relative}`{} as UTF-8: {error}",
                case_file_error_context(context)
            ),
        )
    })
}

pub(super) fn validate_case_file_reference(
    path: &Path,
    line_number: usize,
    relative: &str,
    context: Option<&str>,
) -> PathBuf {
    if relative.is_empty()
        || relative.starts_with('/')
        || relative.starts_with('\\')
        || relative.contains('\\')
        || relative
            .split('/')
            .any(|component| !is_portable_case_file_component(component))
    {
        manifest_error(
            path,
            line_number,
            format!(
                "case file reference `{relative}`{} must use portable relative components",
                case_file_error_context(context)
            ),
        );
    }
    PathBuf::from(relative)
}

pub(super) fn case_file_error_context(context: Option<&str>) -> String {
    context
        .map(|context| format!(" for {context}"))
        .unwrap_or_default()
}

pub(super) fn is_portable_case_file_component(component: &str) -> bool {
    if component.is_empty() || component == "." || component == ".." || component.ends_with('.') {
        return false;
    }
    if !component
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return false;
    }
    let stem = component.split('.').next().unwrap_or(component);
    !matches_reserved_windows_stem(stem)
}

pub(super) fn matches_reserved_windows_stem(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper.strip_prefix("COM").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || upper.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
}

pub(super) fn resolve_case_file_reference(
    manifest_path: &Path,
    line_number: usize,
    base: &Path,
    relative: &str,
    relative_path: &Path,
    context: Option<&str>,
) -> PathBuf {
    let mut current = base.to_path_buf();
    let mut traversed = PathBuf::new();
    let component_count = relative_path.components().count();
    for (index, component) in relative_path.components().enumerate() {
        let name = component.as_os_str();
        if !directory_contains_exact_entry(&current, name) {
            manifest_error(
                manifest_path,
                line_number,
                format!(
                    "case file `{relative}`{} must match fixture entry spelling exactly",
                    case_file_error_context(context)
                ),
            );
        }
        current.push(name);
        traversed.push(name);
        let metadata = fs::symlink_metadata(&current).unwrap_or_else(|error| {
            manifest_error(
                manifest_path,
                line_number,
                format!(
                    "failed to inspect case file `{relative}`{}: {error}",
                    case_file_error_context(context)
                ),
            )
        });
        if is_link_like_metadata(&metadata) {
            manifest_error(
                manifest_path,
                line_number,
                format!(
                    "case file `{relative}`{} must not traverse a link or reparse point",
                    case_file_error_context(context)
                ),
            );
        }
        let final_component = index + 1 == component_count;
        if final_component {
            if !metadata.is_file() {
                manifest_error(
                    manifest_path,
                    line_number,
                    format!(
                        "case file `{relative}`{} must be a regular file",
                        case_file_error_context(context)
                    ),
                );
            }
        } else if !metadata.is_dir() {
            manifest_error(
                manifest_path,
                line_number,
                format!(
                    "case file `{relative}`{} component `{}` must be a directory",
                    case_file_error_context(context),
                    traversed.display()
                ),
            );
        }
    }
    current
}

pub(super) fn directory_contains_exact_entry(dir: &Path, name: &std::ffi::OsStr) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .any(|entry| entry.file_name() == name)
}

#[cfg(unix)]
pub(super) fn is_link_like_metadata(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
pub(super) fn is_link_like_metadata(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(any(unix, windows)))]
pub(super) fn is_link_like_metadata(_metadata: &fs::Metadata) -> bool {
    false
}

pub(super) fn parse_manifest_json_value(path: &Path, value: &ManifestValue<'_>) -> JsonValue {
    parse_manifest_json_value_allow_decimal(path, value)
}

pub(super) fn parse_manifest_mcp_json_value(path: &Path, value: &ManifestValue<'_>) -> JsonValue {
    parse_manifest_json_value_allow_decimal(path, value)
}

pub(super) fn parse_manifest_json_value_allow_decimal(
    path: &Path,
    value: &ManifestValue<'_>,
) -> JsonValue {
    if value.is_string() {
        JsonValue::String(parse_string(path, value))
    } else if value.raw() == "true" {
        JsonValue::Bool(true)
    } else if value.raw() == "false" {
        JsonValue::Bool(false)
    } else if value.raw() == "null" {
        JsonValue::Null
    } else if value.raw().starts_with('[') || value.raw().starts_with('{') {
        parse_json(value.raw()).unwrap_or_else(|error| {
            if value.is_unterminated() && error.missing_closing_delimiter {
                value.report_unterminated(path);
            }
            manifest_error(
                path,
                value.json_error_line(error.offset),
                format!("invalid json assertion value: {error}"),
            )
        })
    } else {
        parse_json(value.raw())
            .unwrap_or_else(|_| manifest_error(path, value.line(), "expected JSON value"))
    }
}

pub(super) fn is_json_integer_token(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let mut index = 0;
    if matches!(bytes.first(), Some(b'-')) {
        index = 1;
    }
    let Some(first) = bytes.get(index) else {
        return false;
    };
    match first {
        b'0' => index += 1,
        b'1'..=b'9' => {
            index += 1;
            while matches!(bytes.get(index), Some(b'0'..=b'9')) {
                index += 1;
            }
        }
        _ => return false,
    }
    index == bytes.len()
}

pub(super) fn parse_binary_fixture_hex(
    path: &Path,
    value: &ManifestValue<'_>,
) -> BinaryFixtureBytes {
    let line_number = value.line();
    let hex = parse_string(path, value);
    let bytes = decode_lowercase_hex(path, line_number, &hex);
    BinaryFixtureBytes { hex, bytes }
}

pub(super) fn parse_binary_fixture_hex_array(
    path: &Path,
    value: &ManifestValue<'_>,
) -> Vec<BinaryFixtureBytes> {
    let line_number = value.line();
    parse_string_array(path, value)
        .into_iter()
        .map(|hex| {
            let bytes = decode_lowercase_hex(path, line_number, &hex);
            BinaryFixtureBytes { hex, bytes }
        })
        .collect()
}

pub(super) fn decode_lowercase_hex(path: &Path, line_number: usize, hex: &str) -> Vec<u8> {
    let bytes = hex.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        manifest_error(
            path,
            line_number,
            "expected complete lowercase hex byte pairs",
        );
    }

    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.as_chunks::<2>().0 {
        let high = lowercase_hex_nibble(pair[0])
            .unwrap_or_else(|| manifest_error(path, line_number, "expected lowercase hex"));
        let low = lowercase_hex_nibble(pair[1])
            .unwrap_or_else(|| manifest_error(path, line_number, "expected lowercase hex"));
        decoded.push((high << 4) | low);
    }
    decoded
}

pub(super) fn lowercase_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

pub(super) fn parse_bool(path: &Path, value: &ManifestValue<'_>) -> bool {
    match value.raw() {
        "true" => true,
        "false" => false,
        _ => manifest_error(path, value.line(), "expected bool"),
    }
}

pub(super) fn parse_source_error_expectation(
    path: &Path,
    value: &ManifestValue<'_>,
) -> SourceErrorExpectation {
    let line_number = value.line();
    let value = parse_string(path, value);
    match value.as_str() {
        "expected" => SourceErrorExpectation::Expected,
        _ => manifest_error(
            path,
            line_number,
            format!("unknown source error expectation `{value}`"),
        ),
    }
}

pub(super) fn parse_skip_platform(path: &Path, line_number: usize, value: &str) -> SkipPlatform {
    match value {
        "unix" => SkipPlatform::Unix,
        "windows" => SkipPlatform::Windows,
        "macos" => SkipPlatform::Macos,
        "linux" => SkipPlatform::Linux,
        _ => manifest_error(
            path,
            line_number,
            format!("unknown skip platform `{value}`"),
        ),
    }
}

pub(super) fn parse_tool_availability(path: &Path, value: &ManifestValue<'_>) -> ToolAvailability {
    let line_number = value.line();
    let value = parse_string(path, value);
    match value.as_str() {
        "missing" => ToolAvailability::Missing,
        "fake-success" => ToolAvailability::FakeSuccess,
        "fake-git-rev-parse" => ToolAvailability::FakeGitRevParse,
        "real" => ToolAvailability::Real,
        _ => manifest_error(
            path,
            line_number,
            format!("unknown tool availability `{value}`"),
        ),
    }
}

pub(super) fn parse_string_array(path: &Path, value: &ManifestValue<'_>) -> Vec<String> {
    value.parse_string_array(path)
}

pub(super) fn parse_string(path: &Path, value: &ManifestValue<'_>) -> String {
    value.parse_string(path)
}

pub(super) fn parse_string_with_context(
    path: &Path,
    value: &ManifestValue<'_>,
    context: &str,
) -> String {
    if !value.is_string() {
        manifest_error(path, value.line(), format!("{context}: expected string"));
    }
    parse_string(path, value)
}

pub(super) fn parse_i32(path: &Path, value: &ManifestValue<'_>) -> i32 {
    value
        .raw()
        .parse()
        .unwrap_or_else(|_| manifest_error(path, value.line(), "expected i32"))
}

pub(super) fn parse_i64(path: &Path, value: &ManifestValue<'_>) -> i64 {
    value
        .raw()
        .parse()
        .unwrap_or_else(|_| manifest_error(path, value.line(), "expected integer"))
}

pub(super) fn parse_positive_usize(path: &Path, value: &ManifestValue<'_>) -> usize {
    let parsed = value
        .raw()
        .parse()
        .unwrap_or_else(|_| manifest_error(path, value.line(), "expected positive integer"));
    if parsed == 0 {
        manifest_error(path, value.line(), "expected positive integer");
    }
    parsed
}

pub(super) fn parse_nonnegative_usize(path: &Path, value: &ManifestValue<'_>) -> usize {
    parse_nonnegative_usize_raw_with_context(path, value.line(), value.raw(), None)
}

pub(super) fn parse_nonnegative_usize_with_context(
    path: &Path,
    value: &ManifestValue<'_>,
    context: &str,
) -> usize {
    parse_nonnegative_usize_raw_with_context(path, value.line(), value.raw(), Some(context))
}

pub(super) fn parse_nonnegative_usize_raw_with_context(
    path: &Path,
    line_number: usize,
    raw: &str,
    context: Option<&str>,
) -> usize {
    if raw.starts_with('-') && is_json_integer_token(raw) {
        let message = "expected non-negative integer";
        let message = match context {
            Some(context) => format!("{context}: {message}"),
            None => message.to_string(),
        };
        manifest_error(path, line_number, message);
    }
    if !is_json_integer_token(raw) {
        let message = "expected integer";
        let message = match context {
            Some(context) => format!("{context}: {message}"),
            None => message.to_string(),
        };
        manifest_error(path, line_number, message);
    }
    raw.parse::<usize>().unwrap_or_else(|_| {
        let message = "expected non-negative integer within range";
        let message = match context {
            Some(context) => format!("{context}: {message}"),
            None => message.to_string(),
        };
        manifest_error(path, line_number, message)
    })
}

pub(super) fn manifest_error(
    path: &Path,
    line_number: usize,
    message: impl std::fmt::Display,
) -> ! {
    if line_number == 0 {
        panic!("{}: {message}", path.display());
    }
    panic!("{}:{line_number}: {message}", path.display());
}
