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
        description: "Search checked Veln language and retained package documentation",
        input: SEARCH_DOCS_INPUT,
        result: SEARCH_DOCS_RESULT,
    },
    ToolSchema {
        name: "read_doc",
        description: "Read one checked Veln language or retained package documentation resource by exact URI",
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
#[path = "schema/tests.rs"]
mod tests;
