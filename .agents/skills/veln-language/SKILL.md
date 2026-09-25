---
name: veln-language
description: Use for Veln language questions and for inspecting or changing the Veln repository through its documentation authority.
---

# Veln Language Routing

The JSON contract below is the complete operative instruction set for this
skill. Apply it exactly. Do not add a fallback from other instructions.

<!-- veln-language-contract:start -->
```json
{
  "schema_version": 1,
  "request_selection": {
    "repository_when": "The request asks to inspect, test, or change the Veln repository or makes the repository, codebase, source code, or proposal state its subject.",
    "language_otherwise": true
  },
  "language": {
    "search_tool": "search_docs",
    "search_scope": "language",
    "read_tool": "read_doc",
    "maximum_calls": 2,
    "call_order": ["search_docs", "read_doc"],
    "selection": "first_search_result",
    "read_exact_search_result_uri": true,
    "fallback": "forbidden",
    "answer_source": "selected_resource_uri",
    "report_selected_uri": true,
    "no_match": "Report that the published Veln language reference has no matching topic. Do not use proposal text or model memory."
  },
  "repository": {
    "entry": "docs/README.md",
    "follow_selected_links": true,
    "maximum_reads": 3,
    "repeat_paths": "forbidden",
    "published_reference_is_authority": false,
    "authority_selection": "smallest_current_linked_authority",
    "no_route": "Stop and report that no repository documentation route covers the request.",
    "explicit_targets": ["repository", "codebase", "source code", "proposal state"],
    "explicit_target_selection": "An explicit repository target selects repository work only when it is the requested subject, including direct or indirect what or where questions. A term definition or incidental mention does not select repository work.",
    "intent_verbs": ["add", "change", "debug", "examine", "fix", "implement", "inspect", "investigate", "modify", "refactor", "remove", "review", "select", "test", "update"],
    "intent_selection": "A requested inspection, test, or change action selects repository work regardless of its position. A word that names such an action does not select repository work when the request instead asks a language question or asks what the word means.",
    "language_complements": ["how", "what", "when", "where", "whether", "why"],
    "location_question_endings": ["defined", "handled", "implemented", "located"],
    "location_question_forms": ["where", "tell me where", "show me where"],
    "path_prefixes": [".agents/", ".github/", "crates/", "docs/", "editors/", "examples/", "scripts/", "tools/", "workflow-scripts/"]
  },
  "failure": {
    "fallback": "forbidden",
    "preserve_previous_result": true,
    "report_operation": true,
    "report_selected_uri": true,
    "search_unavailable": "Stop after search_docs and report that published language-reference search is unavailable.",
    "topic_unavailable": "Stop after read_doc and report that the selected published topic is unavailable.",
    "stale_snapshot": "When read_doc returns resource_not_found for the selected snapshot URI, stop and report that the URI is stale."
  },
  "maintenance": [
    "docs/specification/language-reference-catalog.md",
    "docs/specification/mcp.md",
    "docs/README.md"
  ]
}
```
<!-- veln-language-contract:end -->
