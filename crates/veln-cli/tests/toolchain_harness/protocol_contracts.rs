use super::*;

// These selectors apply only to protocol assertions/references. Ordinary JSON
// Pointer traversal keeps its strict array-index contract.
pub(super) fn protocol_pointer<'a>(
    value: &'a JsonValue,
    tokens: &[String],
) -> JsonPointerResult<'a> {
    let mut current = value;
    for token in tokens {
        if let JsonValue::Array(values) = current
            && let Some(selector) = token.strip_prefix('$')
            && let Some((field, expected)) = selector.split_once('=')
        {
            let mut matches = values.iter().filter(|value| {
                let selected = if field == "value" {
                    Some(*value)
                } else {
                    value.object_field(field)
                };
                selected.and_then(JsonValue::as_str) == Some(expected)
            });
            let Some(value) = matches.next() else {
                return JsonPointerResult::Missing;
            };
            if matches.next().is_some() {
                return JsonPointerResult::Invalid(format!(
                    "selector {token:?} matched multiple entries; select a unique identity"
                ));
            }
            current = value;
        } else {
            current = match json_pointer(current, std::slice::from_ref(token)) {
                JsonPointerResult::Found(value) => value,
                other => return other,
            };
        }
    }
    JsonPointerResult::Found(current)
}

pub(super) fn selected_value(result: JsonPointerResult<'_>) -> Result<&JsonValue, String> {
    match result {
        JsonPointerResult::Found(value) => Ok(value),
        JsonPointerResult::Missing => Err(
            "selected JSON path was not found; inspect the response and reference identity".into(),
        ),
        JsonPointerResult::Invalid(reason) => Err(format!("invalid traversal: {reason}")),
    }
}

fn reference_value<'a>(
    messages: &'a [JsonValue],
    reference: &JsonValue,
) -> Result<&'a JsonValue, String> {
    let id = reference
        .object_field("id")
        .ok_or("response reference needs id")?;
    let path = reference
        .object_field("path")
        .and_then(JsonValue::as_str)
        .ok_or("response reference needs string path")?;
    let tokens = response_pointer_tokens(path)?;
    let selected = || {
        let response = select_mcp_response(messages, id)?;
        selected_value(protocol_pointer(response, &tokens))
    };
    selected().map_err(|error| {
        format!(
            "response reference id {} path {path:?}: {error}",
            id.to_compact_string()
        )
    })
}

fn response_pointer_tokens(path: &str) -> Result<Vec<String>, String> {
    if path.is_empty() {
        return Ok(Vec::new());
    }
    if !path.starts_with('/') {
        return Err("response path must be an RFC 6901 pointer".into());
    }
    path[1..]
        .split('/')
        .map(|token| {
            let mut result = String::new();
            let mut chars = token.chars();
            while let Some(ch) = chars.next() {
                if ch == '~' {
                    result.push(match chars.next() {
                        Some('0') => '~',
                        Some('1') => '/',
                        _ => return Err("response path contains an invalid pointer escape".into()),
                    });
                } else {
                    result.push(ch);
                }
            }
            Ok(result)
        })
        .collect()
}

fn json_object_entries(value: &JsonValue) -> Option<&[(String, JsonValue)]> {
    if let JsonValue::Object(entries) = value {
        Some(entries)
    } else {
        None
    }
}

fn string_field<'a>(value: &'a JsonValue, key: &str) -> Result<&'a str, String> {
    value
        .object_field(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| format!("reference needs string {key}"))
}

pub(super) fn validate_response_reference(reference: &JsonValue) -> Result<(), String> {
    let entries = json_object_entries(reference).ok_or("$response must contain an object")?;
    let allowed = ["id", "path", "link", "replace"];
    let mut keys = BTreeSet::new();
    for (key, _) in entries {
        if !allowed.contains(&key.as_str()) || !keys.insert(key) {
            return Err(format!("unknown or duplicate $response field {key:?}"));
        }
    }
    let id = reference.object_field("id").ok_or("$response needs id")?;
    if !matches!(id, JsonValue::String(_) | JsonValue::Number(_))
        && !matches!(id, JsonValue::Decimal(raw) if is_json_integer_token(raw))
    {
        return Err("$response id must be a string or integer".into());
    }
    response_pointer_tokens(string_field(reference, "path")?)?;
    if reference.object_field("link").is_some() {
        string_field(reference, "link")?;
    }
    if let Some(replace) = reference.object_field("replace") {
        let values = replace
            .as_array()
            .ok_or("$response replace must be [from, to]")?;
        if values.len() != 2
            || values.iter().any(|value| value.as_str().is_none())
            || values[0].as_str() == Some("")
        {
            return Err("$response replace needs two strings and a nonempty from value".into());
        }
    }
    Ok(())
}

pub(super) fn input_contains_response_directive(input: &str) -> bool {
    fn contains(value: &JsonValue) -> bool {
        match value {
            JsonValue::Object(entries) => entries
                .iter()
                .any(|(key, value)| key == "$response" || contains(value)),
            JsonValue::Array(values) => values.iter().any(contains),
            _ => false,
        }
    }
    if input.starts_with("Content-Length:") {
        decode_lsp_stdout(input).is_ok_and(|messages| messages.iter().any(contains))
    } else {
        input
            .lines()
            .any(|line| parse_json(line).is_ok_and(|value| contains(&value)))
    }
}

fn response_directive(entries: &[(String, JsonValue)]) -> Result<Option<&JsonValue>, String> {
    let Some((_, reference)) = entries.iter().find(|(key, _)| key == "$response") else {
        return Ok(None);
    };
    if entries.len() != 1 {
        return Err("$response directive must contain no other members".into());
    }
    validate_response_reference(reference)?;
    Ok(Some(reference))
}

pub(super) fn validate_response_directives(value: &JsonValue) -> Result<(), String> {
    match value {
        JsonValue::Object(entries) => {
            if response_directive(entries)?.is_none() {
                for (_, value) in entries {
                    validate_response_directives(value)?;
                }
            }
            Ok(())
        }
        JsonValue::Array(values) => {
            for value in values {
                validate_response_directives(value)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn response_link(text: &str, label: &str) -> Result<JsonValue, String> {
    let marker = format!("[{label}](");
    let links: Vec<_> = text
        .match_indices(&marker)
        .filter_map(|(start, _)| {
            text[start + marker.len()..]
                .split_once(')')
                .map(|(uri, _)| uri)
        })
        .collect();
    if links.len() != 1 {
        return Err(format!(
            "link {label:?} matched {} entries; select one published link",
            links.len()
        ));
    }
    Ok(JsonValue::String(links[0].into()))
}

fn replace_response_text(replacement: JsonValue, replace: &JsonValue) -> Result<JsonValue, String> {
    let pair = replace.as_array().expect("validated replacement");
    let from = pair[0].as_str().unwrap();
    let to = pair[1].as_str().unwrap();
    let text = replacement
        .as_str()
        .ok_or("replace reference must select a string")?;
    if text.matches(from).count() != 1 {
        return Err(format!(
            "replace source {from:?} must occur exactly once; inspect the published URI"
        ));
    }
    Ok(JsonValue::String(text.replacen(from, to, 1)))
}

fn response_replacement(
    messages: &[JsonValue],
    reference: &JsonValue,
) -> Result<JsonValue, String> {
    let mut replacement = reference_value(messages, reference)?.clone();
    if let Some(link) = reference.object_field("link") {
        let text = replacement
            .as_str()
            .ok_or("link reference must select Markdown text")?;
        replacement = response_link(text, link.as_str().expect("validated link"))?;
    }
    if let Some(replace) = reference.object_field("replace") {
        replacement = replace_response_text(replacement, replace)?;
    }
    Ok(replacement)
}

pub(super) fn expand_response_directives(
    value: &mut JsonValue,
    messages: &[JsonValue],
) -> Result<(), String> {
    match value {
        JsonValue::Object(entries) => {
            if let Some(reference) = response_directive(entries)? {
                *value = response_replacement(messages, reference)?;
            } else {
                for (_, value) in entries {
                    expand_response_directives(value, messages)?;
                }
            }
        }
        JsonValue::Array(values) => {
            for value in values {
                expand_response_directives(value, messages)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn validate_source_range(reference: &JsonValue) -> Result<(), String> {
    let entries = json_object_entries(reference).ok_or("source_range must be an object")?;
    let allowed = ["id", "path", "declaration", "symbol"];
    let mut keys = BTreeSet::new();
    for (key, _) in entries {
        if !allowed.contains(&key.as_str()) || !keys.insert(key) {
            return Err(format!("unknown or duplicate source_range field {key:?}"));
        }
    }
    if entries.len() != 4 || reference.object_field("id").is_none() {
        return Err("source_range needs id, path, declaration, and symbol".into());
    }
    let identity = JsonValue::Object(
        entries
            .iter()
            .filter(|(key, _)| key == "id" || key == "path")
            .cloned()
            .collect(),
    );
    validate_response_reference(&identity)?;
    let declaration = string_field(reference, "declaration")?;
    let symbol = string_field(reference, "symbol")?;
    if symbol.is_empty() || declaration.matches(symbol).count() != 1 {
        return Err(
            "source_range declaration must contain the nonempty symbol exactly once".into(),
        );
    }
    Ok(())
}

pub(super) fn source_range(
    messages: &[JsonValue],
    reference: &JsonValue,
    lsp: bool,
) -> Result<JsonValue, String> {
    let text = reference_value(messages, reference)?
        .as_str()
        .ok_or("source_range must reference source text")?;
    let declaration = string_field(reference, "declaration")?;
    let symbol = string_field(reference, "symbol")?;
    let positions: Vec<_> = text.match_indices(declaration).collect();
    if positions.len() != 1 {
        return Err(format!(
            "declaration {declaration:?} matched {} occurrences; inspect the returned source",
            positions.len()
        ));
    }
    let offset = positions[0].0 + declaration.find(symbol).expect("validated symbol");
    let before = &text[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count();
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let column = if lsp {
        text[line_start..offset].encode_utf16().count()
    } else {
        text[line_start..offset].chars().count()
    };
    let width = if lsp {
        symbol.encode_utf16().count()
    } else {
        symbol.chars().count()
    };
    let (base, coordinate) = if lsp { (0, "character") } else { (1, "column") };
    parse_json(&format!(
        r#"{{"start":{{"line":{},"{coordinate}":{}}},"end":{{"line":{},"{coordinate}":{}}}}}"#,
        line + base,
        column + base,
        line + base,
        column + base + width
    ))
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> JsonValue {
        parse_json(text).unwrap()
    }

    #[test]
    fn protocol_selectors_ignore_unrelated_entries_and_reject_ambiguous_names() {
        for input in [
            r#"[{"name":"rename","uri":"right"},{"name":"unrelated"}]"#,
            r#"[{"name":"added"},{"name":"unrelated"},{"name":"rename","uri":"right"}]"#,
        ] {
            let input = value(input);
            let selected = selected_value(protocol_pointer(
                &input,
                &["$name=rename".into(), "uri".into()],
            ))
            .unwrap();
            assert_eq!(selected.as_str(), Some("right"));
        }
        let duplicate = value(r#"[{"name":"rename"},{"name":"rename"}]"#);
        assert!(matches!(
            protocol_pointer(&duplicate, &["$name=rename".into()]),
            JsonPointerResult::Invalid(_)
        ));
        assert!(matches!(
            protocol_pointer(&duplicate, &["$name=missing".into()]),
            JsonPointerResult::Missing
        ));
        let strings = value(r#"["added","resource_capacity","other"]"#);
        assert_eq!(
            selected_value(protocol_pointer(
                &strings,
                &["$value=resource_capacity".into()]
            ))
            .unwrap()
            .as_str(),
            Some("resource_capacity")
        );
        assert!(matches!(
            json_pointer(&strings, &["$value=resource_capacity".into()]),
            JsonPointerResult::Invalid(_)
        ));
    }

    #[test]
    fn response_directives_preserve_json_types_escaping_and_exact_link_identity() {
        let responses = vec![value(
            r#"{"id":"list","result":{"resources":[{"name":"a/b","uri":"veln-pkg:///snapshot/new/é.veln"}],"number":1e0,"text":"- [function f](veln-doc:///snapshot/new/declaration/opaque)"}}"#,
        )];
        let mut request = value(
            r#"{"uri":{"$response":{"id":"list","path":"/result/resources/$name=a~1b/uri"}},"number":{"$response":{"id":"list","path":"/result/number"}},"link":{"$response":{"id":"list","path":"/result/text","link":"function f"}},"rejected":{"$response":{"id":"list","path":"/result/resources/$name=a~1b/uri","replace":["/é.veln","/hidden.veln"]}}}"#,
        );
        expand_response_directives(&mut request, &responses).unwrap();
        assert_eq!(
            request.object_field("uri").unwrap().as_str(),
            Some("veln-pkg:///snapshot/new/é.veln")
        );
        assert_eq!(
            request.object_field("number"),
            responses[0]
                .object_field("result")
                .unwrap()
                .object_field("number")
        );
        assert_eq!(
            request.object_field("link").unwrap().as_str(),
            Some("veln-doc:///snapshot/new/declaration/opaque")
        );
        assert_eq!(
            request.object_field("rejected").unwrap().as_str(),
            Some("veln-pkg:///snapshot/new/hidden.veln")
        );
        assert!(parse_json(&request.to_compact_string()).is_ok());
        for directive in [
            r#"{"$response":{"id":"missing","path":""}}"#,
            r#"{"$response":{"id":"list","path":"/result/missing"}}"#,
            r#"{"$response":{"id":"list","path":"/result/text","link":"missing"}}"#,
            r#"{"$response":{"id":"list","path":"/result/text","replace":["missing","next"]}}"#,
        ] {
            assert!(
                expand_response_directives(&mut value(directive), &responses).is_err(),
                "{directive}"
            );
        }
        let mut duplicate = responses.clone();
        duplicate.push(responses[0].clone());
        assert!(
            expand_response_directives(
                &mut value(r#"{"$response":{"id":"list","path":"/result"}}"#),
                &duplicate
            )
            .is_err()
        );
    }

    #[test]
    fn response_directives_reject_malformed_operands_before_execution() {
        for directive in [
            r#"{"$response":{"id":null,"path":""}}"#,
            r#"{"$response":{"id":1.5,"path":""}}"#,
            r#"{"$response":{"id":1,"path":"not/a/pointer"}}"#,
            r#"{"$response":{"id":1,"path":"/~2"}}"#,
            r#"{"$response":{"id":1,"path":"","extra":true}}"#,
            r#"{"$response":{"id":1,"id":1,"path":""}}"#,
            r#"{"$response":{"id":1,"path":"","replace":["","x"]}}"#,
            r#"{"$response":{"id":1,"path":""},"extra":true}"#,
        ] {
            assert!(
                validate_response_directives(&value(directive)).is_err(),
                "{directive}"
            );
        }
    }

    #[test]
    fn response_directive_validation_and_expansion_reject_the_same_nested_envelope() {
        let directive =
            value(r#"{"request":{"params":[{"$response":{"id":1,"path":""},"extra":true}]}}"#);
        let validation_error = validate_response_directives(&directive).unwrap_err();
        let expansion_error = expand_response_directives(&mut directive.clone(), &[]).unwrap_err();

        assert_eq!(
            validation_error,
            "$response directive must contain no other members"
        );
        assert_eq!(expansion_error, validation_error);
    }

    #[test]
    fn source_ranges_follow_inserted_lines_and_protocol_coordinate_units() {
        let reference =
            value(r#"{"id":1,"path":"/result","declaration":"pub fn café(","symbol":"café"}"#);
        validate_source_range(&reference).unwrap();
        for (text, line) in [
            ("😀 pub fn café()\r\n", 0),
            ("# added\r\n😀 pub fn café()\r\n", 1),
        ] {
            let responses = vec![JsonValue::Object(vec![
                ("id".into(), reference.object_field("id").unwrap().clone()),
                ("result".into(), JsonValue::String(text.into())),
            ])];
            let lsp = source_range(&responses, &reference, true).unwrap();
            assert_eq!(
                lsp,
                value(&format!(
                    r#"{{"start":{{"line":{line},"character":10}},"end":{{"line":{line},"character":14}}}}"#
                ))
            );
            let mcp = source_range(&responses, &reference, false).unwrap();
            assert_eq!(
                mcp,
                value(&format!(
                    r#"{{"start":{{"line":{},"column":10}},"end":{{"line":{},"column":14}}}}"#,
                    line + 1,
                    line + 1
                ))
            );
            let wrong =
                value(r#"{"start":{"line":99,"character":10},"end":{"line":99,"character":14}}"#);
            assert!(expect_json_value(&wrong, &lsp).is_err());
        }
        for text in ["unrelated", "pub fn café()\npub fn café()"] {
            let responses = vec![JsonValue::Object(vec![
                ("id".into(), reference.object_field("id").unwrap().clone()),
                ("result".into(), JsonValue::String(text.into())),
            ])];
            assert!(source_range(&responses, &reference, true).is_err());
        }
    }
}

#[test]
fn manifest_response_directives_fail_before_skipping_for_both_protocols() {
    for protocol in ["lsp", "mcp"] {
        let root = test_temp_root(&format!("{protocol}-response-directive"));
        let message = r#"{"jsonrpc":"2.0","id":1,"method":"read","params":{"uri":{"$res\u0070onse":{"id":1,"path":"bad"}}}}"#;
        let input = if protocol == "lsp" {
            format!("[{message}]")
        } else {
            message.to_owned()
        };
        assert!(input_contains_response_directive(message));
        fs::write(root.join("requests.json"), input).unwrap();
        let field = if protocol == "lsp" {
            "stdin_jsonrpc_file"
        } else {
            "stdin_file"
        };
        let manifest = format!(
            "command = [\"{protocol}\"]\n{field} = \"requests.json\"\nexit = 0\n[skip]\nplatforms = [\"linux\", \"macos\", \"windows\"]\nreason = \"must validate references before skipping\"\n"
        );
        let failure =
            std::panic::catch_unwind(|| parse_manifest(&root.join("case.toml"), &manifest))
                .expect_err("malformed reference must fail loading");
        assert!(panic_message(failure).contains("pointer"));
        fs::remove_dir_all(root).unwrap();
    }
}
