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
    "semantic_basis": "Classify the requested action and subject by meaning. Do not decide from a closed vocabulary of verbs, question words, or sentence frames.",
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
    "semantic_selection": "A request for the agent to examine, validate, or alter Veln behavior or implementation selects repository work. A question whose subject is a compiler or parser implementation also selects repository work. A request for information about how a Veln language feature behaves selects language work. A repository, codebase, source-code, proposal-state, or repository-path subject selects repository work unless the phrase is incidental or being defined.",
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
