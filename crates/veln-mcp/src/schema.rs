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
            _ => false,
        }
    }

    pub(crate) fn accepts_result(self, value: &Value) -> bool {
        matches_schema(&self.result_schema(), value)
    }
}

pub(crate) const TOOLS: [ToolSchema; 3] = [
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
        Some("string") => value.is_string(),
        Some("integer") => matches_integer_schema(schema, value),
        Some("boolean") => value.is_boolean(),
        Some(_) => false,
        None => true,
    }
}

fn matches_integer_schema(schema: &Value, value: &Value) -> bool {
    let Some(number) = value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| value.try_into().ok()))
    else {
        return false;
    };
    if let Some(minimum) = schema.get("minimum").and_then(Value::as_i64)
        && number < minimum
    {
        return false;
    }
    true
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
    fn check_project_result_schema_closes_diagnostics_summary_and_analysis_modes() {
        let tool = tool("check_project").unwrap();
        let diagnostic = serde_json::json!({
            "id": "type.mismatch",
            "kind": "type",
            "severity": "error",
            "message": "type mismatch",
            "span": span(),
            "related": [{"message": "expected here", "span": span()}],
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

        let mut invalid_mode = project_success.clone();
        invalid_mode["analysis"]["mode"] = serde_json::json!("single_file");
        assert!(!tool.accepts_result(&invalid_mode));

        let mut invalid_extra = project_success.clone();
        invalid_extra["diagnostics"][0]["extra"] = serde_json::json!(true);
        assert!(!tool.accepts_result(&invalid_extra));

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
