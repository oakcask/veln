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
    "query_derivation": {
      "basis": "semantic_main_language_subject",
      "instruction": "Derive one broad English topic term that names the main Veln language concept requested. Translate a non-English request. Use a multi-word query only for an established compound concept such as borrow checker. Omit Veln, question framing, requested answer form, operations or details being asked about, and incidental concepts. Do not copy the request text as the query unless the request already consists only of the topic term.",
      "maximum_query_scalars": 64,
      "request_text_as_query": "forbidden_unless_topic_term_only",
      "no_subject": "Stop without a tool call and report that no bounded published language-reference query can be derived when no main language concept can be identified or the derived query would be empty or exceed 64 Unicode scalar values."
    },
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
    "explicit_path_reads": 2,
    "explicit_path_rule": "After reading docs/README.md, read an explicitly named normalized Markdown path under docs/ directly when it exists and is a current documentation authority. The explicit path is the terminal authority and does not need to be linked from docs/README.md.",
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
    "dispatch_order": ["search_unavailable", "topic_unavailable", "stale_snapshot", "other_search_failure", "other_topic_failure"],
    "search_unavailable": {
      "operation": "search_docs",
      "result": { "kind": "transport_error", "code": "tool_unavailable" },
      "instruction": "Stop after search_docs and report that published language-reference search is unavailable."
    },
    "topic_unavailable": {
      "operation": "read_doc",
      "result": { "kind": "transport_error", "code": "transport_unavailable" },
      "instruction": "Stop after read_doc and report that the selected published topic is unavailable."
    },
    "stale_snapshot": {
      "operation": "read_doc",
      "result": { "kind": "tool_error", "code": "resource_not_found", "selected_uri_must_match": true },
      "instruction": "Stop after read_doc and report that the selected snapshot URI is stale."
    },
    "other_search_failure": {
      "operation": "search_docs",
      "result": "Any failed or malformed result not matched by an earlier dispatch entry.",
      "instruction": "Stop after search_docs and report that published language-reference search failed."
    },
    "other_topic_failure": {
      "operation": "read_doc",
      "result": "Any failed or malformed result not matched by an earlier dispatch entry.",
      "instruction": "Stop after read_doc and report that the selected published topic could not be read."
    }
  },
  "maintenance": [
    "docs/specification/language-reference-catalog.md",
    "docs/specification/mcp.md",
    "docs/README.md"
  ]
}
```
<!-- veln-language-contract:end -->
