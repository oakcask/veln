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
        value.as_object().is_some_and(serde_json::Map::is_empty)
    }
}

pub(crate) const TOOLS: [ToolSchema; 2] = [
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
];

fn parse_schema(text: &'static str) -> Value {
    serde_json::from_str(text).expect("checked MCP schema should contain JSON")
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
    fn tool_inputs_accept_only_empty_objects() {
        for tool in TOOLS {
            assert!(tool.accepts_input(&serde_json::json!({})));
            assert!(!tool.accepts_input(&serde_json::json!({"unknown": true})));
            assert!(!tool.accepts_input(&serde_json::json!([])));
            assert!(!tool.accepts_input(&Value::Null));
        }
    }

    fn matches_schema(schema: &Value, value: &Value) -> bool {
        if let Some(variants) = schema.get("oneOf").and_then(Value::as_array) {
            return variants
                .iter()
                .filter(|variant| matches_schema(variant, value))
                .count()
                == 1;
        }

        match schema.get("type").and_then(Value::as_str) {
            Some("object") => matches_object_schema(schema, value),
            Some("array") => value.as_array().is_some_and(|values| {
                schema
                    .get("items")
                    .is_none_or(|items| values.iter().all(|item| matches_schema(items, item)))
            }),
            Some("string") => value.is_string(),
            Some("integer") => value.as_i64().is_some() || value.as_u64().is_some(),
            Some(_) => false,
            None => schema.get("const").is_none_or(|expected| expected == value),
        }
    }

    fn matches_object_schema(schema: &Value, value: &Value) -> bool {
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
        if schema.get("additionalProperties") == Some(&Value::Bool(false))
            && object.keys().any(|key| !properties.contains_key(key))
        {
            return false;
        }
        properties.iter().all(|(key, property)| {
            object
                .get(key)
                .is_none_or(|value| matches_schema(property, value))
        })
    }
}
