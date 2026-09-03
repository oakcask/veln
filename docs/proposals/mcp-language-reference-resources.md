---
role: proposal
update-when: The checked language-reference catalog, MCP resource protocol, Markdown rendering contract, or planned language-resource evidence changes.
---

# MCP Language Reference Resources

## Summary

Publish the checked language-reference catalog through the existing `veln mcp`
server. This slice gives MCP clients a deterministic index and one Markdown
resource per language topic without adding model-controlled documentation
tools or package and source resources.

## Scope

| Included | Excluded |
| --- | --- |
| MCP resource capability advertisement, listing, and reading. | `search_docs`, `read_doc`, or any other model-controlled documentation tool. |
| One language-reference index and every topic in the checked catalog. | Package documentation, standard-library documentation, and virtual source resources. |
| Deterministic Markdown rendered from the checked catalog artifact. | Offline and web output trees or a second hand-maintained content source. |
| Exact URI lookup and bounded complete reads. | Pagination, subscriptions, runtime regeneration, and historical catalog snapshots. |
| Protocol, renderer, and stdio evidence for success and failure. | Navigation expansion, rename, client plugins, and conformance-manifest completion. |

The server consumes the checked catalog and digest used by ordinary builds. It
does not execute the source grammar, inspect development documentation, or
analyze a workspace to list or read these resources.

## Resource Catalog

Initialization advertises `resources` with both `listChanged` and `subscribe`
set to `false`. The complete resource list has no continuation cursor and is
sorted by URI UTF-8 bytes.

The list contains exactly these resources for the checked digest:

- one index at
  `veln-doc:///language/snapshot/<digest>/index`; and
- one topic for each checked catalog topic at
  `veln-doc:///language/snapshot/<digest>/topic/<topic-id>`.

The index resource has name `language-index`, title `Veln Language Reference`,
and media type `text/markdown; charset=utf-8`. A topic resource uses its topic
identifier as its name, its catalog title as its title, its catalog summary as
its description, and the same media type. Resource metadata comes only from
the checked artifact and these renderer constants.

The first `resources/list` request omits `cursor`. The server accepts request
metadata according to the existing MCP request-metadata contract. A supplied
cursor, an unknown field, `null` parameters, or a non-object parameter is an
invalid-params protocol failure. The result omits `nextCursor`.

## Markdown Rendering

The index contains its title followed by one link and summary for every topic.
The topic order matches the checked catalog order. Each link uses the exact
topic resource URI returned by `resources/list`.

Each topic contains these catalog-owned values in this order:

1. the title and summary;
2. the supporting body paragraphs;
3. every selected grammar block, with its production name;
4. every selected example, with its display name and each displayed file's
   relative name and source text;
5. the keywords; and
6. links to the related topic resources.

Grammar and source blocks preserve the scalar text in the checked artifact.
Renderer-owned separators and Markdown decoration are fixed by checked golden
outputs. Rendering does not add repository paths, proposal material,
maintenance commands, timestamps, build paths, or compiler binary versions.

Each rendered resource is indivisible and contains at most 262,144 UTF-8
bytes. The language-reference freshness check rejects a checked catalog whose
rendered index or topic exceeds that limit. Ordinary MCP reads return complete
content and do not truncate or paginate it.

## Resource Reads And Failures

`resources/read` accepts one exact `uri` and existing MCP request metadata. A
successful read returns one text content entry whose URI equals the request,
whose media type equals the listed resource media type, and whose text equals
the deterministic rendered bytes decoded as UTF-8.

Lookup compares the complete URI spelling. It does not normalize percent
encoding, path segments, authority, query, fragment, digest, or topic
identifier. An unknown URI, a noncanonical spelling, a digest other than the
checked digest, or a topic identifier absent from the checked catalog returns
the MCP resource-not-found protocol error with structured domain code
`resource_not_found`. The failure returns no content. A missing URI, `null`, a
non-string URI, an unknown field, or non-object parameters instead returns
invalid params.

The resource set is available after initialization and stays unchanged until
server shutdown. Workspace refresh and project analysis do not change its
URIs or content.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Initialize the server. | The response advertises a non-subscribable, unchanged resource capability in addition to the existing tool capability. | MCP server initialization test and stdio specification case. |
| List resources without a cursor. | The result contains the index and every checked topic exactly once, sorted by URI bytes, with catalog-derived metadata and no `nextCursor`. | Catalog-to-resource table test and exact stdio output. |
| Read the index and one topic containing grammar and examples. | Each response contains one complete Markdown text entry with the requested URI and media type. The index routes to every topic, and the topic preserves the ordered catalog semantic content. | Renderer golden tests and MCP stdio reads. |
| Compare listed and related URIs with readable resources. | Every emitted URI is accepted unchanged by `resources/read`; every read resource appears in the list. | Bidirectional catalog and read-lookup test. |
| Read an unknown, noncanonical, wrong-digest, or unknown-topic URI. | The server returns `resource_not_found` with no content and does not fall back to a file or workspace lookup. | Table-driven URI rejection tests and one stdio failure case. |
| Supply malformed list or read parameters. | Cursor-bearing list input and missing, nullable, non-string, non-object, or unknown-field input fail as invalid params. | Protocol parameter-shape table. |
| Refresh the workspace and analyze a project between two reads. | The language resource list, URIs, metadata, and bytes remain identical. | Server lifecycle state-preservation test. |
| Render the largest resource at and above the byte limit. | A resource at 262,144 bytes is accepted; a larger resource fails freshness validation before publication. Successful reads are never partial. | Renderer boundary test and freshness rejection case. |
| Inspect every rendered resource. | Content is derived from the checked catalog and renderer constants and contains none of the excluded development or build provenance. | Bundle exclusion assertions shared with catalog validation. |
| Run an ordinary MCP build and the explicit language-reference freshness route. | The ordinary build and server consume checked inputs without SWI-Prolog. The freshness route detects catalog, digest, renderer, or size-limit drift. | Cargo consumer tests and the existing language-reference freshness CI route extended for rendered resources. |

## Completion

This proposal is complete when every acceptance row passes, the MCP
specification and executable examples state the resource behavior, and the
language-reference specification routes to the implemented Markdown renderer.
Move completion history under `../reference/implemented-proposals/`, remove
this page from the Ready catalog, and leave the umbrella proposal unselectable
until another finite slice is extracted.
