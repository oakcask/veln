use super::*;
use crate::outcome::ToolOutcome;

#[test]
fn shared_tool_outcome_renderer_preserves_success_and_failure_envelopes() {
    let success = render_tool_outcome(
        "definition",
        ToolOutcome::Success(json!({"definition": null})),
    );
    assert_eq!(
        success,
        json!({
            "content": [{"type": "text", "text": "{\"definition\":null}"}],
            "structuredContent": {"definition": null},
            "isError": false
        })
    );

    let failure = render_tool_outcome(
        "definition",
        ToolOutcome::DomainFailure {
            code: "invalid_position",
            message: "position is outside the selected source",
            details: json!({"source": "main.veln", "line": 4, "column": 1}),
        },
    );
    assert_eq!(
        failure,
        json!({
            "content": [{
                "type": "text",
                "text": "{\"code\":\"invalid_position\",\"details\":{\"column\":1,\"line\":4,\"source\":\"main.veln\"},\"message\":\"position is outside the selected source\"}"
            }],
            "structuredContent": {
                "code": "invalid_position",
                "message": "position is outside the selected source",
                "details": {"source": "main.veln", "line": 4, "column": 1}
            },
            "isError": true
        })
    );
}
