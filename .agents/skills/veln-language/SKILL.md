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
    "repository_when": "The request asks to inspect or change the Veln repository, its implementation, or its proposal state.",
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
    "intent_verbs": ["add", "change", "debug", "fix", "implement", "inspect", "modify", "refactor", "remove", "review", "select", "test", "update"],
    "intent_selection": "A leading inspection or change intent selects repository work unless the request asks how, what, when, where, whether, or why the Veln language behaves, or explicitly asks about language semantics.",
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
