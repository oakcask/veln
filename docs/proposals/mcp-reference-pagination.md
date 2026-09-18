---
role: proposal
update-when: MCP reference pagination schemas, continuation lifetime, or planned acceptance evidence changes.
---

# MCP Reference Pagination

## Outcome And Readiness

Agents can consume bounded pages of saved-source references without losing or
repeating locations when files change between requests. This slice separates
pagination from the broader navigation inventory in
[Agent Language Services](agent-language-services.md).

The saved capture, reference collection, scope metadata, and workspace refresh
foundations already exist in `crates/veln-mcp/src/references.rs` and the server.
Current behavior is specified in [MCP Navigation](../specification/mcp.md).
The checked `references-input.json` currently accepts only source coordinates;
the adapter returns the entire collected array. Pagination is not implemented.
No pending syntax, rename, plugin, or additional symbol class is a dependency.

## Scope

Preserve the current supported-symbol, visibility, recovery, project ownership,
coordinate, stable-capture, and package-resource boundaries. Page only the
reference sites returned by the current service. Declarations and package
implementation bodies remain excluded. `include_declaration`, additional
navigation coverage, LSP pagination, and bounded analysis cost are non-goals.
The page bound limits response size, not the work needed to collect references.

## Request And Result Contract

| Input or result | Proposed contract |
| --- | --- |
| Initial request | Required `source`, `line`, and `column` retain their current rules. Optional integer `page_size` is 1 through 1,000, default 100. |
| Continuation request | Exactly one nonempty string field, `cursor`. Source coordinates and page size cannot accompany it. |
| Invalid shape | Unknown fields, `null`, mixed request forms, non-integer or out-of-range page sizes produce JSON-RPC invalid params without consuming a cursor. |
| Success | Existing `references` and `scope` fields plus optional string `next_cursor`. Omit `next_cursor` on an empty or final page; never return it as `null`. |
| Ordering | Sort the captured complete result by URI UTF-8 bytes, then numeric start line, start column, end line, end column before paging. Each nonfinal page has exactly the requested size. |
| Scope | Every page repeats the initial capture's scope metadata and selection generation. |
| Domain failure | Use the existing MCP tool-error `{code, message, details}` envelope, with no success-only references, scope, or next cursor. Cursor failure details are `{}`. |

Requests without a page size now default to a bounded response. Clients must
follow `next_cursor` to obtain complete results; existing small results retain
their shape. Checked input and result schemas and advertised tool schemas must
describe both request forms and the optional continuation result.

## Continuation State

A cursor is opaque and authenticated, bound to one server process, selection
generation, captured ordered result, page size, and next offset. Only one
outstanding cursor exists per retained result. Continuation reads retained
locations without recapturing files or admitting additional package resources.
At most 64 unfinished results are retained; admission of a 65th evicts the
oldest initially admitted unfinished result. Continuation does not change that
order. Terminal results release their retained state. There is no time expiry.

The following table is the acceptance authority for this planned slice.
Its evidence is not yet implemented.

| State and event | Observable outcome and next state | Planned evidence |
| --- | --- | --- |
| Initial capture succeeds with zero, exactly one page, or multiple pages. | Empty/final results have no cursor; otherwise a cursor identifies the next offset. Concatenation equals the independently expected ordered reference sites without omissions or duplicates. | MCP stdio fixture with exact locations across multiple files and pages, including an unsupported selection. |
| Current cursor is continued. | Return the next page and consume that cursor. A nonfinal page supplies a new cursor; the final page supplies none. | Server transition test and stdio round trip. |
| Cursor is malformed, tampered, from another server, from before restart, or already consumed. | `invalid_cursor`; other retained results and workspace selection are unchanged. Consumed-token rejection takes precedence over later refresh or eviction. | Server token and replay tests. |
| An unconsumed cursor's result was evicted or its generation was successfully refreshed. | `stale_snapshot`; no result is recaptured. Restoring identical file bytes does not revive it. | Capacity and refresh transition tests. |
| Files change after capture, without successful refresh. | Continuation returns the originally captured locations and scope, even if those locations no longer address the new file bytes. | Source-edit and source-removal continuation tests. |
| Refresh fails. | Selection and all live cursors remain usable. | Failed-refresh state-preservation test. |
| Initial capture or resource admission fails. | Preserve existing failure behavior; publish no cursor, consume no continuation capacity, and leave earlier cursors and retained resources usable. | Stable-capture and capacity failure tests with a prior live cursor. |
| Invalid continuation shape is followed by a valid use of the same cursor. | The valid use succeeds; schema failure did not consume state. | Schema and stdio failure/recovery cases. |

Tests cover page sizes 1 and 1,000, default 100, and rejection of 0, 1,001,
fractional values, and `null`. Numeric acceptance follows the existing exact
JSON integer semantics. A 65-result transition checks the specified eviction
victim, and replay after final consumption checks `invalid_cursor`.

## Verification And Completion

Implement the acceptance table in `crates/veln-mcp/src/server/tests/` and the
checked MCP stdio harness under `examples/specification/`. Extend schema tests
for advertised input and success/failure results. Keep existing reference
fixtures as regression evidence for symbol identity and scope; do not replace
them with assertions derived from the pagination implementation.

Run focused MCP tests through `bash scripts/agent-test -p veln-mcp` and the
repository specification-case runner for the added stdio cases. Completion
requires those cases, updated MCP specification and tool schemas, and removal
of this proposal and its Ready entry. Broader navigation and declaration
inclusion remain separate work.
