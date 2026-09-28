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
    "discovery_tool": "list_language_topics",
    "read_tool": "read_doc",
    "maximum_calls": 4,
    "maximum_topic_reads": 3,
    "call_order": ["list_language_topics", "read_doc"],
    "topic_selection": {
      "basis": "semantic_relevance_to_request",
      "instruction": "Inspect every listed title and summary. Select only the topics needed to answer the Veln language question, regardless of the request language or vocabulary. Discard topics that are not relevant. Select no more than three topics and do not derive an unlisted URI.",
      "selection_source": ["title", "summary"],
      "deduplicate_by": "uri",
      "no_relevant_topic": "Stop after list_language_topics and report that the published Veln language reference has no relevant topic."
    },
    "read_exact_listed_uris": true,
    "fallback": "forbidden",
    "answer_source": "successfully_read_selected_resource_uris",
    "report_selected_uris": true,
    "no_match": "Report that the published Veln language reference has no relevant topic. Do not use proposal text or model memory."
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
    "dispatch_order": ["listing_unavailable", "topic_unavailable", "stale_snapshot", "other_listing_failure", "other_topic_failure"],
    "listing_unavailable": {
      "operation": "list_language_topics",
      "result": { "kind": "transport_error", "code": "tool_unavailable" },
      "instruction": "Stop after list_language_topics and report that published language-topic discovery is unavailable."
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
    "other_listing_failure": {
      "operation": "list_language_topics",
      "result": "Any failed or malformed result not matched by an earlier dispatch entry.",
      "instruction": "Stop after list_language_topics and report that published language-topic discovery failed."
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
