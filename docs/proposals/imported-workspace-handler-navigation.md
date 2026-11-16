---
role: proposal
update-when: The planned imported workspace handler selection, reference identity, adapter projection, exclusions, or acceptance evidence changes.
---

# Imported Workspace Handler Navigation

Navigate a public workspace handler through a qualified handler application in
an importing module. The shared language service already supports same-module
handler definition and references, and it already resolves equivalent
workspace imports for effects and effect operations. This proposal extends
that existing identity to imported handler applications.

This proposal is ready for implementation. It changes one bounded navigation
capability. It does not change handler syntax, import resolution, compilation,
effect handling, or the general LSP and MCP navigation protocols.

## Scope And Identity

An eligible handler has one valid-cased, unrecovered public declaration in a
workspace module. Another workspace module imports that module with one
parse-clean, unambiguous `use` declaration. The written import can name the
full module path or use its unique implicit leaf alias. An exact full written
path takes precedence over a colliding implicit leaf alias, matching imported
effect navigation.

A complete `handle Body with module::handler(arguments)` application reached
through that import selects the declaration's module-and-name handler identity.
The handler-name leaf is the selected range. The module qualifier is not part
of the range. Selecting the declaration, a same-module bare application, or an
eligible imported application returns the same identity.

The reference set combines complete bare applications in saved workspace
sources that declare the handler's module with complete qualified applications
reached through eligible workspace imports. Equal handler spellings in another
module or symbol class do not join the identity.

## Observable Contract

Shared definition returns the public workspace handler declaration from the
declaration and every eligible application. Shared references return the
identity's deterministic workspace locations. LSP and MCP project that shared
result through their existing coordinate, ordering, declaration-inclusion,
pagination, stable-snapshot, and failure contracts.

The following selection decisions apply:

| Input | Result |
| --- | --- |
| Public handler declaration, same-module bare application, or eligible imported qualified application | The shared workspace handler identity, definition, and reference set. |
| Private handler reached through an import | Unsupported selection; existing valid-but-empty adapter result. |
| Ambiguous, recovered, missing, or multiply qualified import or handler path | Unsupported selection; no inferred identity from equal spelling. |
| Handler application with an incomplete or recovered argument list | Unsupported selection for the handler leaf. |
| Direct-dependency, standard-library, unresolved, or wrong-kind target | Unsupported selection; no workspace handler identity. |
| Duplicate or recovered handler declaration | Unsupported declaration and applications; unrelated valid identities remain available. |

Handlers remain unsupported for rename. An eligible imported handler does not
admit its owning effect, effect operations, context parameters, operation-clause
parameters, or module qualifier into the handler reference set. Comments and
strings do not contribute references. An unrelated parse error does not remove
a structurally complete, parse-clean handler application.

Adapter failures preserve state. An MCP capture, lookup, invalid-position, or
pagination failure publishes no partial locations, creates no cursor, and does
not replace the saved workspace, retained resources, prior successful results,
or existing cursors. An LSP request reads its accepted workspace snapshot and
does not admit rejected dependency or overlay state merely because the written
path has the same spelling.

## Acceptance And Verification

All evidence in this section is planned. Existing same-module handler and
imported-effect cases establish the reusable boundaries; they do not prove the
proposed handler import behavior.

| Case | Input | Required observation | Planned evidence |
| --- | --- | --- | --- |
| H1: shared selection | Select one public handler at its declaration, a same-module bare application, and qualified applications through a full import path and a unique implicit leaf alias. | Every selection returns the same declaration and combined reference set; every occurrence range covers only the handler leaf. | Focused language-service tests plus LSP definition and references tests. |
| H2: adapter agreement | Save the H1 workspace and request declaration-inclusive and declaration-excluding references through LSP and MCP. | After coordinate conversion, both adapters project the same shared locations. MCP preserves its ordering and pagination contract. | A checked MCP transcript and the saved-navigation cross-adapter conformance test. |
| H3: import resolution | Add colliding leaf aliases, an exact full path, recovered and ambiguous imports, and another module with an equal handler name. | The exact path wins where specified; ambiguous, recovered, and wrong-owner paths remain unsupported without contaminating the valid result. | Focused shared-selection and MCP negative tests. |
| H4: declaration and path rejection | Use a private, duplicate, recovered, dependency, standard-library, unresolved, wrong-kind, incomplete, or recovered handler target. | Definition is absent and references are empty for the unsupported selection; unrelated valid handler navigation remains unchanged. | Focused language-service and adapter rejection tables. |
| H5: state preservation | Keep a live MCP references cursor, then issue an imported-handler request whose capture or lookup fails. | The failure publishes no partial result and the existing cursor continues from its original captured set. Saved workspace and retained resources are unchanged. | Focused MCP server state tests using existing capture-failure support. |
| H6: source isolation | Place equal spellings in comments, strings, effect operations, handler parameters, and another module, plus an unrelated parse error. | Only resolved handler applications enter the shared result. | Focused language-service reference-index tests. |

The checked transcript belongs under `examples/specification/mcp/`. Adapter
and state-transition details belong in focused `veln-lsp` and `veln-mcp` tests.
Identity resolution and the complete positive and negative matrix belong in
`veln-language-service`; adapters must not implement a second handler scan.

## Related Work And Design Choice

[Veln PR #1647](https://github.com/oakcask/veln/pull/1647) added the current
same-module workspace handler identity, reference indexing, and LSP and MCP
projection. It deliberately rejected imported and qualified handler paths, so
it establishes the reusable identity and the current missing boundary rather
than evidence that this proposal is implemented.

[Veln PR #1673](https://github.com/oakcask/veln/pull/1673) added workspace
import resolution for public effect definition and references. Its full-path
precedence, unique implicit leaf alias, ambiguity, recovery, owner isolation,
and adapter projection rules apply to the same workspace import surface used
by handler paths. This proposal adopts those decisions instead of creating a
handler-specific import resolver.

The current
[Editor Support](../specification/editor-support.md#lsp-navigation-formatting-and-rename)
and [MCP](../specification/mcp.md#saved-workspace-navigation) specifications
remain authoritative for implemented same-module handler behavior and general
adapter state. Reusing their shared selection and projection boundaries keeps
the new capability consistent while leaving dependency and standard-library
handler navigation for a separate proposal.

## Non-Goals

- Do not change handler declaration, application, visibility, or effect rules.
- Do not add direct-dependency, transitive-dependency, or standard-library
  handler navigation.
- Do not add handler rename, completion, hover, signature help, or symbols.
- Do not broaden effect, effect-operation, module, or local-binding navigation.
- Do not add a new MCP location, cursor, ordering, or package-resource format.

## Implementation And Specification Route

Extend the shared handler selection and reference index with the existing
workspace import-resolution boundary. Project the shared result through the
existing LSP and MCP handler branches. No adapter-local import scan or new
handler identity is required.

Add executable evidence before updating current-behavior prose. When the
evidence passes, update the handler navigation sections of `editor-support.md`
and `mcp.md`, and name the checked transcript. Remove this slice from the
agent-language-services inventory, then remove this page and its Ready catalog
entry once the current specifications and executable evidence own the behavior.
