use std::sync::OnceLock;

use serde_json::Value;

const WORKSPACE_PROJECTS_INPUT: &str =
    include_str!("../schemas/mcp/v1/workspace-projects-input.json");
const WORKSPACE_PROJECTS_RESULT: &str =
    include_str!("../schemas/mcp/v1/workspace-projects-result.json");
const REFRESH_WORKSPACE_INPUT: &str =
    include_str!("../schemas/mcp/v1/refresh-workspace-input.json");
const REFRESH_WORKSPACE_RESULT: &str =
    include_str!("../schemas/mcp/v1/refresh-workspace-result.json");
const CHECK_PROJECT_INPUT: &str = include_str!("../schemas/mcp/v1/check-project-input.json");
const CHECK_PROJECT_RESULT: &str = include_str!("../schemas/mcp/v1/check-project-result.json");
const DEFINITION_INPUT: &str = include_str!("../schemas/mcp/v1/definition-input.json");
const DEFINITION_RESULT: &str = include_str!("../schemas/mcp/v1/definition-result.json");
const REFERENCES_INPUT: &str = include_str!("../schemas/mcp/v1/references-input.json");
const REFERENCES_RESULT: &str = include_str!("../schemas/mcp/v1/references-result.json");
const SEARCH_DOCS_INPUT: &str = include_str!("../schemas/mcp/v1/search-docs-input.json");
const SEARCH_DOCS_RESULT: &str = include_str!("../schemas/mcp/v1/search-docs-result.json");
const READ_DOC_INPUT: &str = include_str!("../schemas/mcp/v1/read-doc-input.json");
const READ_DOC_RESULT: &str = include_str!("../schemas/mcp/v1/read-doc-result.json");

#[derive(Clone, Copy)]
pub(crate) struct ToolSchema {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    input: &'static str,
    result: &'static str,
}

impl ToolSchema {
    pub(crate) fn input_schema(self) -> Value {
        parse_schema(self.input)
    }

    pub(crate) fn result_schema(self) -> Value {
        parse_schema(self.result)
    }

    pub(crate) fn accepts_input(self, value: &Value) -> bool {
        let Some(object) = value.as_object() else {
            return false;
        };
        match self.name {
            "workspace_projects" | "refresh_workspace" => object.is_empty(),
            "check_project" => {
                object.keys().all(|key| key == "project" || key == "source")
                    && object.get("project").is_none_or(Value::is_string)
                    && object.get("source").is_none_or(Value::is_string)
            }
            "definition" | "references" => matches_schema(&self.input_schema(), value),
            "search_docs" => {
                matches_schema(&self.input_schema(), value)
                    && value["query"].as_str().is_some_and(|query| {
                        !veln_project::portable_normalized_case_fold(query)
                            .trim()
                            .is_empty()
                    })
            }
            "read_doc" => matches_schema(&self.input_schema(), value),
            _ => false,
        }
    }

    pub(crate) fn accepts_result(self, value: &Value) -> bool {
        matches_schema(&self.result_schema(), value)
    }
}

pub(crate) const TOOLS: [ToolSchema; 7] = [
    ToolSchema {
        name: "workspace_projects",
        description: "Return the current workspace project selection without refreshing it",
        input: WORKSPACE_PROJECTS_INPUT,
        result: WORKSPACE_PROJECTS_RESULT,
    },
    ToolSchema {
        name: "refresh_workspace",
        description: "Rediscover workspace projects and atomically replace the selection",
        input: REFRESH_WORKSPACE_INPUT,
        result: REFRESH_WORKSPACE_RESULT,
    },
    ToolSchema {
        name: "check_project",
        description: "Analyze one saved workspace project or anonymous Veln source",
        input: CHECK_PROJECT_INPUT,
        result: CHECK_PROJECT_RESULT,
    },
    ToolSchema {
        name: "definition",
        description: "Resolve a supported symbol in one saved workspace source",
        input: DEFINITION_INPUT,
        result: DEFINITION_RESULT,
    },
    ToolSchema {
        name: "references",
        description: "Return references for a supported symbol in one saved workspace source",
        input: REFERENCES_INPUT,
        result: REFERENCES_RESULT,
    },
    ToolSchema {
        name: "search_docs",
        description: "Search the checked Veln language reference topics",
        input: SEARCH_DOCS_INPUT,
        result: SEARCH_DOCS_RESULT,
    },
    ToolSchema {
        name: "read_doc",
        description: "Read one checked Veln language reference resource by exact URI",
        input: READ_DOC_INPUT,
        result: READ_DOC_RESULT,
    },
];

fn parse_schema(text: &'static str) -> Value {
    serde_json::from_str(text).expect("checked MCP schema should contain JSON")
}

fn matches_schema(schema: &Value, value: &Value) -> bool {
    matches_schema_with_root(schema, schema, value)
}

fn matches_schema_with_root(root: &Value, schema: &Value, value: &Value) -> bool {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let Some(name) = reference.strip_prefix("#/$defs/") else {
            return false;
        };
        let Some(target) = root.get("$defs").and_then(|defs| defs.get(name)) else {
            return false;
        };
        return matches_schema_with_root(root, target, value);
    }
    if let Some(expected) = schema.get("const") {
        return expected == value;
    }
    if let Some(variants) = schema.get("enum").and_then(Value::as_array)
        && !variants.iter().any(|variant| variant == value)
    {
        return false;
    }
    if let Some(variants) = schema.get("oneOf").and_then(Value::as_array) {
        return variants
            .iter()
            .filter(|variant| matches_schema_with_root(root, variant, value))
            .count()
            == 1;
    }

    match schema.get("type").and_then(Value::as_str) {
        Some("object") => matches_object_schema(root, schema, value),
        Some("array") => value.as_array().is_some_and(|values| {
            schema.get("items").is_none_or(|items| {
                values
                    .iter()
                    .all(|item| matches_schema_with_root(root, items, item))
            })
        }),
        Some("string") => matches_string_schema(schema, value),
        Some("integer") => matches_integer_schema(schema, value),
        Some("boolean") => value.is_boolean(),
        Some("null") => value.is_null(),
        Some(_) => false,
        None => true,
    }
}

fn matches_string_schema(schema: &Value, value: &Value) -> bool {
    let Some(text) = value.as_str() else {
        return false;
    };
    let len = text.chars().count() as u64;
    if let Some(minimum) = schema.get("minLength").and_then(Value::as_u64)
        && len < minimum
    {
        return false;
    }
    if let Some(maximum) = schema.get("maxLength").and_then(Value::as_u64)
        && len > maximum
    {
        return false;
    }
    true
}

fn matches_integer_schema(schema: &Value, value: &Value) -> bool {
    let Some(number) = value.as_number() else {
        return false;
    };
    if !json_number_is_integer(&number.to_string()) {
        return false;
    }
    if let Some(minimum) = schema.get("minimum").and_then(Value::as_i64)
        && json_number_is_less_than_i64(&number.to_string(), minimum)
    {
        return false;
    }
    if let Some(maximum) = schema.get("maximum").and_then(Value::as_i64)
        && !json_number_is_less_than_i64(&number.to_string(), maximum + 1)
    {
        return false;
    }
    true
}

pub(crate) fn json_integer_usize(value: &Value) -> Option<usize> {
    let number = value.as_number()?;
    let text = number.to_string();
    if !json_number_is_integer(&text) {
        return None;
    }
    let parts = JsonNumberParts::parse(&text)?;
    if parts.negative {
        return None;
    }
    if parts.is_zero() {
        return Some(0);
    }
    let scale = parts.signed_scale()?;
    let mut digits = parts.digits;
    let trailing_zeros = if scale > 0 {
        digits.truncate(digits.len().checked_sub(scale as usize)?);
        0
    } else {
        scale.unsigned_abs() as usize
    };
    let normalized = digits.trim_start_matches('0');
    if normalized.is_empty() {
        return Some(0);
    }
    let maximum = usize::MAX.to_string();
    let total_len = normalized.len().checked_add(trailing_zeros)?;
    if total_len > maximum.len() {
        return None;
    }
    if total_len == maximum.len()
        && normalized
            .bytes()
            .chain(std::iter::repeat_n(b'0', trailing_zeros))
            .gt(maximum.bytes())
    {
        return None;
    }
    let mut canonical = String::with_capacity(total_len);
    canonical.push_str(normalized);
    canonical.extend(std::iter::repeat_n('0', trailing_zeros));
    canonical.parse().ok()
}

fn json_number_is_integer(text: &str) -> bool {
    let Some(parts) = JsonNumberParts::parse(text) else {
        return false;
    };
    if parts.is_zero() || parts.exponent_at_least_fraction_len() {
        return true;
    }
    let Some(scale) = parts.fraction_scale() else {
        return false;
    };
    scale <= parts.digits.len()
        && parts.digits[parts.digits.len() - scale..]
            .bytes()
            .all(|byte| byte == b'0')
}

fn json_number_is_less_than_i64(text: &str, minimum: i64) -> bool {
    if minimum < 0 {
        return text.starts_with('-')
            && text
                .parse::<f64>()
                .is_ok_and(|value| value < minimum as f64);
    }
    let Some(parts) = JsonNumberParts::parse(text) else {
        return true;
    };
    if parts.negative {
        return true;
    }
    if parts.is_zero() {
        return 0 < minimum;
    }
    let scale = parts.signed_scale();
    if scale
        .is_none_or(|scale| scale < 0 && scale.unsigned_abs() as usize > minimum.to_string().len())
    {
        return false;
    }
    let Some(scale) = scale else {
        return true;
    };
    let mut digits = parts.digits;
    let trailing_zeros = if scale > 0 {
        digits.truncate(digits.len().saturating_sub(scale as usize));
        0
    } else {
        scale.unsigned_abs() as usize
    };
    let normalized = digits.trim_start_matches('0');
    if normalized.is_empty() {
        return 0 < minimum;
    }
    let minimum = minimum.to_string();
    let total_len = normalized.len() + trailing_zeros;
    total_len < minimum.len()
        || (total_len == minimum.len()
            && normalized
                .bytes()
                .chain(std::iter::repeat_n(b'0', trailing_zeros))
                .lt(minimum.bytes()))
}

struct JsonNumberParts {
    negative: bool,
    digits: String,
    fraction_len: usize,
    exponent: JsonExponent,
}

enum JsonExponent {
    Finite(i64),
    HugePositive,
    HugeNegative,
}

impl JsonNumberParts {
    fn parse(text: &str) -> Option<Self> {
        let (negative, text) = match text.strip_prefix('-') {
            Some(text) => (true, text),
            None => (false, text),
        };
        let (mantissa, exponent) = match text.find(['e', 'E']) {
            Some(index) => (&text[..index], JsonExponent::parse(&text[index + 1..])?),
            None => (text, JsonExponent::Finite(0)),
        };
        let (integer, fraction) = match mantissa.split_once('.') {
            Some((integer, fraction)) => (integer, fraction),
            None => (mantissa, ""),
        };
        if integer.is_empty()
            || !integer.bytes().all(|byte| byte.is_ascii_digit())
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        let mut digits = String::with_capacity(integer.len() + fraction.len());
        digits.push_str(integer);
        digits.push_str(fraction);
        Some(Self {
            negative,
            digits,
            fraction_len: fraction.len(),
            exponent,
        })
    }

    fn is_zero(&self) -> bool {
        self.digits.bytes().all(|byte| byte == b'0')
    }

    fn exponent_at_least_fraction_len(&self) -> bool {
        match self.exponent {
            JsonExponent::Finite(exponent) => exponent >= self.fraction_len as i64,
            JsonExponent::HugePositive => true,
            JsonExponent::HugeNegative => false,
        }
    }

    fn signed_scale(&self) -> Option<i64> {
        let JsonExponent::Finite(exponent) = self.exponent else {
            return None;
        };
        i64::try_from(self.fraction_len)
            .ok()
            .and_then(|fraction_len| fraction_len.checked_sub(exponent))
    }

    fn fraction_scale(&self) -> Option<usize> {
        let scale = self.signed_scale()?;
        if scale <= 0 {
            return Some(0);
        }
        usize::try_from(scale).ok()
    }
}

impl JsonExponent {
    fn parse(text: &str) -> Option<Self> {
        let (sign, digits) = match text.as_bytes().first() {
            Some(b'+') => (1, &text[1..]),
            Some(b'-') => (-1, &text[1..]),
            _ => (1, text),
        };
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        match text.parse::<i64>() {
            Ok(value) => Some(Self::Finite(value)),
            Err(_) if sign >= 0 => Some(Self::HugePositive),
            Err(_) => Some(Self::HugeNegative),
        }
    }
}

fn matches_object_schema(root: &Value, schema: &Value, value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if let Some(required) = schema.get("required").and_then(Value::as_array)
        && !required
            .iter()
            .filter_map(Value::as_str)
            .all(|field| object.contains_key(field))
    {
        return false;
    }
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    match schema.get("additionalProperties") {
        Some(Value::Bool(false)) if object.keys().any(|key| !properties.contains_key(key)) => {
            return false;
        }
        Some(additional_schema) if additional_schema.is_object() => {
            for (key, value) in object {
                if !properties.contains_key(key)
                    && !matches_schema_with_root(root, additional_schema, value)
                {
                    return false;
                }
            }
        }
        _ => {}
    }
    properties.iter().all(|(key, property)| {
        object
            .get(key)
            .is_none_or(|value| matches_schema_with_root(root, property, value))
    })
}

pub(crate) fn tool(name: &str) -> Option<ToolSchema> {
    TOOLS.into_iter().find(|tool| tool.name == name)
}

pub(crate) fn declarations() -> &'static Value {
    static DECLARATIONS: OnceLock<Value> = OnceLock::new();
    DECLARATIONS.get_or_init(|| {
        Value::Array(
            TOOLS
                .into_iter()
                .map(|tool| {
                    serde_json::json!({
                        "name": tool.name,
                        "description": tool.description,
                        "inputSchema": tool.input_schema(),
                        "outputSchema": tool.result_schema(),
                    })
                })
                .collect(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_tool_schemas_are_the_advertised_schemas() {
        let declarations = declarations().as_array().unwrap();
        assert_eq!(declarations.len(), TOOLS.len());
        for (declaration, tool) in declarations.iter().zip(TOOLS) {
            assert_eq!(declaration["name"], tool.name);
            assert_eq!(declaration["inputSchema"], tool.input_schema());
            assert_eq!(declaration["outputSchema"], tool.result_schema());
            assert_eq!(declaration["inputSchema"]["type"], "object");
            assert_eq!(declaration["outputSchema"]["type"], "object");
            assert_eq!(declaration["inputSchema"]["additionalProperties"], false);
        }
    }

    #[test]
    fn refresh_result_schema_accepts_success_and_domain_failure() {
        let schema = tool("refresh_workspace").unwrap().result_schema();
        assert!(matches_schema(
            &schema,
            &serde_json::json!({"generation": 1, "roots": ["alpha", "beta/deep"]})
        ));
        assert!(matches_schema(
            &schema,
            &serde_json::json!({
                "code": "generation_failed",
                "message": "workspace project discovery failed",
                "details": {}
            })
        ));
        assert!(!matches_schema(
            &schema,
            &serde_json::json!({"code": "generation_failed", "message": "missing details"})
        ));
    }

    #[test]
    fn empty_tool_inputs_accept_only_empty_objects() {
        for tool in [
            tool("workspace_projects").unwrap(),
            tool("refresh_workspace").unwrap(),
        ] {
            assert!(tool.accepts_input(&serde_json::json!({})));
            assert!(!tool.accepts_input(&serde_json::json!({"unknown": true})));
            assert!(!tool.accepts_input(&serde_json::json!([])));
            assert!(!tool.accepts_input(&Value::Null));
        }
    }

    #[test]
    fn check_project_input_accepts_only_declared_string_fields() {
        let tool = tool("check_project").unwrap();
        for value in [
            serde_json::json!({}),
            serde_json::json!({"project": "."}),
            serde_json::json!({"source": "main.veln"}),
            serde_json::json!({"project": ".", "source": "main.veln"}),
        ] {
            assert!(tool.accepts_input(&value), "{value}");
        }
        for value in [
            serde_json::json!({"unknown": true}),
            serde_json::json!({"project": null}),
            serde_json::json!({"source": null}),
            serde_json::json!({"project": []}),
            serde_json::json!(null),
        ] {
            assert!(!tool.accepts_input(&value), "{value}");
        }
    }

    #[test]
    fn definition_input_requires_closed_positive_coordinates() {
        let tool = tool("definition").unwrap();
        assert_position_input_schema(tool);
    }

    #[test]
    fn references_input_requires_closed_positive_coordinates() {
        let tool = tool("references").unwrap();
        assert_position_input_schema(tool);
    }

    fn assert_position_input_schema(tool: ToolSchema) {
        assert!(tool.accepts_input(&serde_json::json!({
            "source": "main.veln",
            "line": 1,
            "column": 1
        })));
        assert!(tool.accepts_input(&serde_json::json!({
            "source": "main.veln",
            "line": u64::MAX,
            "column": 1
        })));
        assert!(tool.accepts_input(&serde_json::json!({
            "source": "main.veln",
            "line": 1.0,
            "column": 1e0
        })));
        let above_u64 = serde_json::from_str(
            r#"{"source":"main.veln","line":18446744073709551616,"column":1}"#,
        )
        .unwrap();
        assert!(tool.accepts_input(&above_u64));
        let huge_positive_exponent = serde_json::from_str(
            r#"{"source":"main.veln","line":1e9223372036854775807,"column":1}"#,
        )
        .unwrap();
        assert!(tool.accepts_input(&huge_positive_exponent));
        let huge_negative_exponent = serde_json::from_str(
            r#"{"source":"main.veln","line":1e-9223372036854775808,"column":1}"#,
        )
        .unwrap();
        assert!(!tool.accepts_input(&huge_negative_exponent));
        let non_integer =
            serde_json::from_str(r#"{"source":"main.veln","line":6.0000000000000001,"column":1}"#)
                .unwrap();
        assert!(!tool.accepts_input(&non_integer));
        for value in [
            serde_json::json!({}),
            serde_json::json!({"source":"main.veln","line":1}),
            serde_json::json!({"source":"main.veln","line":0,"column":1}),
            serde_json::json!({"source":"main.veln","line":1,"column":-1}),
            serde_json::json!({"source":"main.veln","line":1.5,"column":1}),
            serde_json::json!({"source":null,"line":1,"column":1}),
            serde_json::json!({"source":"main.veln","line":1,"column":1,"extra":true}),
            serde_json::json!([]),
        ] {
            assert!(!tool.accepts_input(&value), "{value}");
        }
    }

    #[test]
    fn definition_result_accepts_empty_location_and_domain_failures() {
        let tool = tool("definition").unwrap();
        assert!(tool.accepts_result(&serde_json::json!({"definition": null})));
        for code in [
            "invalid_path",
            "invalid_position",
            "snapshot_changed",
            "resource_capacity",
        ] {
            let result = serde_json::json!({
                "code": code,
                "message": "failed",
                "details": {"source": "main.veln"}
            });
            assert!(tool.accepts_result(&result), "{result}");
        }
    }

    #[test]
    fn references_result_accepts_locations_scope_and_domain_failures() {
        let tool = tool("references").unwrap();
        let location = serde_json::json!({
            "uri": "file:///workspace/main.veln",
            "range": {
                "start": {"line": 2, "column": 3},
                "end": {"line": 2, "column": 9}
            }
        });
        assert!(tool.accepts_result(&serde_json::json!({
            "references": [location],
            "scope": {
                "mode": "project",
                "generation": 0,
                "project": ".",
                "project_wide": true
            }
        })));
        assert!(tool.accepts_result(&serde_json::json!({
            "references": [],
            "scope": {
                "mode": "single_file",
                "generation": 1,
                "project": ".",
                "source": "main.veln",
                "project_wide": false
            }
        })));
        for code in [
            "invalid_path",
            "invalid_position",
            "snapshot_changed",
            "resource_capacity",
        ] {
            let result = serde_json::json!({
                "code": code,
                "message": "failed",
                "details": {"source": "main.veln"}
            });
            assert!(tool.accepts_result(&result), "{result}");
        }
        assert!(!tool.accepts_result(&serde_json::json!({
            "code": "snapshot_changed",
            "message": "failed",
            "details": {},
            "references": []
        })));
    }

    #[test]
    fn search_docs_input_enforces_query_scope_and_limit_bounds() {
        let tool = tool("search_docs").unwrap();
        for value in [
            serde_json::json!({"query": "schema"}),
            serde_json::json!({"query": "schema", "scope": "language"}),
            serde_json::json!({"query": "schema", "limit": 1}),
            serde_json::json!({"query": "schema", "limit": 50}),
            serde_json::json!({"query": "schema", "limit": 5.0}),
        ] {
            assert!(tool.accepts_input(&value), "{value}");
        }
        let exponent_limit = serde_json::from_str(r#"{"query":"schema","limit":1e0}"#).unwrap();
        assert!(tool.accepts_input(&exponent_limit), "{exponent_limit}");
        for value in [
            serde_json::json!({}),
            serde_json::json!({"query": ""}),
            serde_json::json!({"query": " \t\n"}),
            serde_json::json!({"query": "x".repeat(257)}),
            serde_json::json!({"query": null}),
            serde_json::json!({"query": "schema", "scope": "all"}),
            serde_json::json!({"query": "schema", "limit": 0}),
            serde_json::json!({"query": "schema", "limit": 51}),
            serde_json::json!({"query": "schema", "limit": 1.5}),
            serde_json::json!({"query": "schema", "unknown": true}),
            serde_json::json!(null),
            serde_json::json!([]),
        ] {
            assert!(!tool.accepts_input(&value), "{value}");
        }
    }

    #[test]
    fn language_doc_result_schemas_accept_success_and_domain_failure() {
        let search = tool("search_docs").unwrap();
        assert!(search.accepts_result(&serde_json::json!({
            "scope": "language",
            "results": [{
                "uri": "veln-doc:///language/snapshot/d/topic/schemas",
                "title": "Schemas",
                "summary": "Schemas describe format-neutral and binary fields.",
                "excerpt": "Schemas",
                "prefix_truncated": false,
                "suffix_truncated": false
            }]
        })));
        assert!(!search.accepts_result(&serde_json::json!({
            "scope": "all",
            "results": []
        })));
        assert!(!search.accepts_result(&serde_json::json!({
            "scope": "language",
            "results": [{
                "uri": "uri",
                "title": "title",
                "summary": "summary",
                "excerpt": "x".repeat(161),
                "prefix_truncated": false,
                "suffix_truncated": false
            }]
        })));

        let read = tool("read_doc").unwrap();
        assert!(
            read.accepts_input(
                &serde_json::json!({"uri": "veln-doc:///language/snapshot/d/index"})
            )
        );
        assert!(!read.accepts_input(&serde_json::json!({"uri": null})));
        assert!(!read.accepts_input(&serde_json::json!({"uri": "uri", "unknown": true})));
        assert!(read.accepts_result(&serde_json::json!({
            "uri": "veln-doc:///language/snapshot/d/index",
            "name": "language-index",
            "title": "Veln Language Reference",
            "mimeType": "text/markdown; charset=utf-8",
            "text": "# Veln Language Reference\n"
        })));
        assert!(read.accepts_result(&serde_json::json!({
            "code": "resource_not_found",
            "message": "language reference resource not found",
            "details": {"uri": "missing"}
        })));
        assert!(!read.accepts_result(&serde_json::json!({
            "code": "generation_failed",
            "message": "failed",
            "details": {"uri": "missing"}
        })));
    }

    #[test]
    fn check_project_result_schema_closes_diagnostics_summary_and_analysis_modes() {
        let tool = tool("check_project").unwrap();
        let diagnostic = serde_json::json!({
            "id": "type.mismatch",
            "kind": "type",
            "severity": "error",
            "message": "type mismatch",
            "span": span(),
            "related": [
                {"message": "expected here", "span": span()},
                {"message": "Accepted integer form: 0 or 1."},
                {"kind": "repair_hint", "message": "Use a selected declaration.", "span": null},
                {
                    "kind": "effect_declaration",
                    "message": "Candidate effect is declared here.",
                    "effect": "net",
                    "operations": ["request"],
                    "span": span()
                },
                {
                    "message": "Field path: Packet.kind.",
                    "field_path": [
                        {"kind": "schema", "name": "Packet"},
                        {"kind": "field", "name": "kind"}
                    ]
                }
            ],
            "details": {"expected": "Int"}
        });
        let project_success = serde_json::json!({
            "schema_version": 1,
            "analysis": {
                "mode": "project",
                "generation": 0,
                "project": ".",
                "project_wide": true
            },
            "diagnostics": [diagnostic],
            "summary": {
                "diagnostic_count": 1,
                "by_severity": {"error": 1},
                "by_kind": {"type": 1}
            }
        });
        assert!(tool.accepts_result(&project_success));
        assert!(tool.accepts_result(&serde_json::json!({
            "schema_version": 1,
            "analysis": {
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "main.veln",
                "project_wide": false
            },
            "diagnostics": [],
            "summary": {
                "diagnostic_count": 0,
                "by_severity": {},
                "by_kind": {}
            }
        })));
        assert!(tool.accepts_result(&serde_json::json!({
            "code": "snapshot_changed",
            "message": "workspace files changed during capture",
            "details": {}
        })));
        assert!(tool.accepts_result(&serde_json::json!({
            "code": "resource_capacity",
            "message": "dependency source resource capacity exceeded",
            "details": {}
        })));

        let mut invalid_mode = project_success.clone();
        invalid_mode["analysis"]["mode"] = serde_json::json!("single_file");
        assert!(!tool.accepts_result(&invalid_mode));

        let mut invalid_extra = project_success.clone();
        invalid_extra["diagnostics"][0]["extra"] = serde_json::json!(true);
        assert!(!tool.accepts_result(&invalid_extra));

        let mut invalid_related_extra = project_success.clone();
        invalid_related_extra["diagnostics"][0]["related"][0]["unexpected"] =
            serde_json::json!(true);
        assert!(!tool.accepts_result(&invalid_related_extra));

        let mut invalid_related_path_extra = project_success.clone();
        invalid_related_path_extra["diagnostics"][0]["related"][4]["field_path"][0]["unexpected"] =
            serde_json::json!(true);
        assert!(!tool.accepts_result(&invalid_related_path_extra));

        let mut invalid_summary = project_success;
        invalid_summary["summary"]["by_kind"]["type"] = serde_json::json!("one");
        assert!(!tool.accepts_result(&invalid_summary));
    }

    fn matches_schema(schema: &Value, value: &Value) -> bool {
        super::matches_schema(schema, value)
    }

    fn span() -> Value {
        serde_json::json!({
            "file": "main.veln",
            "start": {"line": 1, "column": 1, "offset": 0},
            "end": {"line": 1, "column": 2, "offset": 1}
        })
    }
}
