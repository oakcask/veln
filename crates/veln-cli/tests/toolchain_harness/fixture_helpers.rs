use super::*;

pub(super) fn assert_fixture_schema_error(schema: &str, field_path: Option<&str>, expected: &str) {
    let root = test_temp_root("fixture-schema-error");
    write_fixture_schema_sources(&root);
    let field_path = field_path
        .map(|value| format!("field_path = {value}"))
        .unwrap_or_default();
    let manifest = parse_manifest(
        Path::new("case.toml"),
        &format!(
            r#"
command = ["run", "--json", "main", "main.veln", "wire.veln", "facade.veln"]
exit = 0

[[binary_fixture]]
name = "schema-reference"
schema = "{schema}"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
{field_path}
"#
        ),
    );
    let panic = std::panic::catch_unwind(|| manifest.validate_fixture_schema_references(&root))
        .expect_err("schema reference should be rejected");
    let message = panic_message(panic);
    assert!(
        message.contains(expected),
        "expected panic to contain `{expected}`, got `{message}`"
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

pub(super) fn write_fixture_schema_sources(root: &Path) {
    fs::write(
        root.join("main.veln"),
        r#"
use wire
use facade

schema LocalPacket
	format binary

	length: UInt8
end
"#,
    )
    .expect("main source should be written");
    fs::write(
        root.join("wire.veln"),
        r#"
pub schema PublicPacket
	format binary

	length: UInt8
end

schema PrivatePacket
	format binary

	length: UInt8
end

pub fn make_packet() -> Int
	1
end

pub type PacketShape
	pub Packet(Int)
end

"#,
    )
    .expect("wire source should be written");
    fs::write(
        root.join("facade.veln"),
        r#"
use wire

pub schema AliasPacket = wire::PublicPacket
"#,
    )
    .expect("facade source should be written");
}

pub(super) fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&str>() {
        return message.to_string();
    }
    "non-string panic".to_string()
}

pub(super) fn test_temp_root(name: &str) -> PathBuf {
    let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "veln-toolchain-harness-test-{name}-{}-{nanos}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("test root should be created");
    root
}

#[cfg(windows)]
pub(super) fn fake_tool_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{name}.cmd"))
}

#[cfg(not(windows))]
pub(super) fn fake_tool_path(root: &Path, name: &str) -> PathBuf {
    root.join(name)
}

pub(super) fn parse_json_pointer(
    path: &Path,
    line_number: usize,
    assertion_name: &str,
    assertion_index: usize,
    pointer: &str,
) -> Vec<String> {
    if pointer.is_empty() {
        return Vec::new();
    }
    if !pointer.starts_with('/') {
        manifest_error(
            path,
            line_number,
            format!(
                "{assertion_name} {assertion_index} path `{pointer}` is not a JSON Pointer; nonempty pointers must start with `/`"
            ),
        );
    }
    pointer[1..]
        .split('/')
        .map(|token| {
            let mut decoded = String::new();
            let mut chars = token.chars();
            while let Some(ch) = chars.next() {
                if ch != '~' {
                    decoded.push(ch);
                    continue;
                }
                match chars.next() {
                    Some('0') => decoded.push('~'),
                    Some('1') => decoded.push('/'),
                    Some(escape) => manifest_error(
                        path,
                        line_number,
                        format!(
                            "{assertion_name} {assertion_index} path `{pointer}` has invalid JSON Pointer escape `~{escape}`"
                        ),
                    ),
                    None => manifest_error(
                        path,
                        line_number,
                        format!(
                            "{assertion_name} {assertion_index} path `{pointer}` has an incomplete JSON Pointer escape"
                        ),
                    ),
                }
            }
            decoded
        })
        .collect()
}
