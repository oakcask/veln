---
role: proposal
update-when: Shared rename selection or conflict provenance, MCP saved-workspace navigation, MCP tool schemas, or the planned rename evidence changes.
---

# MCP Saved-Workspace Rename

## Summary

Add an MCP `rename` tool that computes workspace edits from the same immutable
saved-source capture used by MCP definition and references. The tool reuses the
shared selection, reference, identifier-class, and conflict model already used
by LSP. It returns edits for the client to apply; it does not write source files
or mutate the saved workspace.

This proposal owns the MCP rename contract and absorbs the former blocked
identifier-casing mapping. It does not create transport-specific casing or
conflict rules.

## Scope

| Included | Excluded |
| --- | --- |
| Manifest-project and anonymous-source capture through the existing saved-navigation boundary. | Open-document overlays, unsaved text, and client-applied filesystem mutation. |
| Workspace types, type aliases, constructors, functions, function aliases, test declarations, exact-companion private functions, value bindings, handler context parameters, operation-clause parameters, and the corresponding unambiguous recovery records supported by shared rename. | Schemas, effects, handlers, effect operations, module qualifiers, package declarations, and any other unsupported selection. |
| Exact workspace edit locations, replacement text, casing failures, and predictable conflict failures. | Prepare-rename, transitive-dependency navigation, package edits, and changes to shared name resolution. |
| Existing stable-capture, project-selection, path, and coordinate failures. | Reference pagination, cursor creation, package documentation, and package resource publication. |

The shared language service remains authoritative for symbol identity,
references, recovery linking, name class, predictable conflicts, and conflict
declaration provenance. The MCP adapter owns input validation, one-based
Unicode-scalar coordinates, canonical workspace edit URIs, result
serialization, and saved-capture failure behavior. It must preserve package
provenance in a conflict result without admitting the package as an MCP
resource.

## Tool Contract

The closed input schema requires `source`, positive integer `line` and
`column`, and non-empty string `new_name`. The source and coordinate fields use
the current MCP definition boundary: `source` is a workspace-relative saved
`.veln` file, and coordinates are one-based Unicode-scalar positions. Decimal
or exponent JSON spellings that denote an integer follow the existing
coordinate contract.

`new_name` is one complete identifier only when its first character is ASCII
alphabetic or `_` and every later character is ASCII alphanumeric or `_`.
Reserved words satisfy this lexical check, matching current LSP rename input
validation. A non-identifier returns the domain failure
`rename.invalid_name` with exactly `details: {requested_name}` and no edits.
Identifier-class validation then applies the shared rules:

- type and constructor replacements start with an ASCII uppercase letter;
- function and value-binding replacements start with an ASCII lowercase
  letter; and
- a recovery symbol uses the class retained for that declaration or binding.

A class-changing replacement returns `rename.invalid_case`. Its details contain
`symbol_class`, `requested_name`, and `required_initial`. A predictable
same-namespace or lexical conflict returns `rename.conflict`. Its details
contain `symbol_class`, `requested_name`, the conflicting declaration location,
and `affected_scope`. A module scope is exactly
`{kind: "module", name}`. A lexical scope is exactly
`{kind: "lexical", file, start_offset, end_offset}`: `file` is a
workspace-relative source path, and the offsets are zero-based UTF-8 byte
offsets into that saved source, with an exclusive end. Locations use one-based
Unicode-scalar half-open ranges. Workspace edit and workspace conflict
locations use canonical `file:` URIs. If the shared conflict is a direct
dependency or standard-library declaration, its location keeps the canonical
`veln-pkg:` URI supplied by the shared language service. Returning that URI
does not publish or admit the package resource. These are MCP domain failures,
not successful empty results, and neither failure returns edits.

The checked result schema is a closed `oneOf`. Success is exactly an object
with the required `edits` array. Each closed edit object requires string `uri`,
one-based Unicode-scalar half-open `range`, and string `new_text`; no other
field is allowed. A failure is exactly an object with required string `code`,
string `message`, and object `details`, with no success field. The
`rename.invalid_name`, `rename.invalid_case`, and `rename.conflict` detail
objects are closed to the fields defined above. `invalid_path`,
`invalid_position`, and `snapshot_changed` reuse their existing saved-navigation
codes and detail shapes.

A supported conflict-free selection returns every linked workspace declaration
and reference as an edit with `uri`, `range`, and `new_text`. Edits sort by URI
UTF-8 bytes and numeric start line, start column, end line, and end column.
Each location appears once. Replacing a name with itself follows the same
selection boundary and returns the selected symbol's complete edit set.
Exact MCP cases assert this ordering. Comparisons with the shared service and
LSP normalize locations to a set; they do not impose MCP ordering on LSP.

A valid position with no supported unambiguous workspace symbol succeeds with
`edits: []`. This includes package declarations, module segments, unsupported
symbol classes, incompatible roles, shadowed or out-of-scope occurrences, and
ambiguous recovery selections. It does not reinterpret the token as another
name class.

For a selected manifest project, the edit set may include any saved source in
that selected project and excludes sources owned by another project. For an
anonymous source, the edit set is limited to that captured file. Every edit is
computed from the same capture; the tool never mixes a later file version into
the result.

## State And Failure Contract

The tool validates and computes edits without applying them. It builds a
read-only navigation snapshot from the stable capture and does not admit or
publish package source or documentation resources. Success, an empty
selection, and every failure preserve filesystem bytes, workspace roots,
selection generation, published diagnostics, the published resource set,
prior successful results, and all existing reference cursors. Rename does not
create or consume a cursor.

Invalid paths return `invalid_path`; out-of-range coordinates return
`invalid_position`; exhausted stable-capture retries return
`snapshot_changed`. These failures contain no edit array. Capture and analysis
failures publish no partial navigation resources. Because rename never admits
resources, retained package capacity cannot change a rename result.

## Verification Model

Each acceptance row names the strongest practical evidence for its observable
contract. Checked MCP stdio cases cover stable transport inputs and outputs.
MCP server transition tests cover state that requires an injected capture
change, a pre-existing cursor, or controlled resource capacity. Shared
language-service and LSP comparisons cover adapter parity only for identities
supported by both adapters; MCP-only type-alias behavior has direct shared
selection and server coverage instead. Schema tests independently close the
advertised protocol shapes.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Rename each supported parse-clean workspace symbol class in a selected project. | Return the declaration and exactly the linked references as sorted, unique workspace edits with the requested name. | Shared-language-service comparison and exact MCP stdio cases for types, type aliases, constructors, functions, function aliases, test declarations, exact-companion private functions, value bindings, and both handler parameter classes. |
| Rename a supported symbol in an anonymous source. | Return only edits in the captured source, with no project or package location. | Anonymous-source success and project-isolation cases. |
| Rename an unambiguous recovery declaration or linked recovery occurrence. | Return the retained declaration and in-scope linked references using the recovery symbol's name class. | Recovery cases for type, constructor, function, local, callable, handler context, and operation-clause binding identities. |
| Request the selected symbol's current name. | Return the same complete edit set as another valid same-class replacement. | Idempotent-name cases for top-level and lexical symbols. |
| Supply a non-empty replacement that violates the ASCII identifier rule. | Return `rename.invalid_name` with exactly the requested name in details, no edits, and unchanged state. | Input-boundary cases for punctuation-bearing, multi-token, non-ASCII, and digit-led replacements, plus a reserved-word lexical-acceptance case. |
| Change a type or constructor to lowercase, or a function or value binding to uppercase. | Return `rename.invalid_case` with the shared class and required-initial details and no edits. | Exact failure-schema cases paired with the shared and LSP casing matrix. |
| Choose a validly cased name that creates a predictable module or lexical conflict. | Return `rename.conflict` with the exact conflicting location and affected scope and no edits. A workspace conflict uses a canonical `file:` URI; a direct-dependency or standard-library conflict keeps its canonical `veln-pkg:` URI without publishing that resource. | Exact MCP stdio cases for workspace module and lexical conflicts; server cases for handler bindings, recovery symbols, and package-backed conflicts; shared-location comparisons for both URI provenances. |
| Select a schema, effect, handler, effect operation, module segment, package-backed occurrence, unsupported role, or ambiguous recovery occurrence in a workspace source. | Succeed with `edits: []`; do not map the selection to another class. | Supported-symbol boundary matrix over workspace declarations and workspace occurrences that resolve to direct-dependency or standard-library declarations. |
| Use a valid source with an invalid coordinate, or an invalid source path. | Return the existing `invalid_position` or `invalid_path` failure and no edits. | Checked schema and saved-navigation path/coordinate cases, including non-BMP and CRLF boundaries. |
| Change captured inputs throughout the bounded capture attempts. | Return `snapshot_changed`, publish no edits or resources, and leave prior state and cursors usable. | Injected capture-change transition followed by prior-resource, cursor, and valid-rename checks. |
| Rename a project symbol while its capture contains dependencies and the retained resource set is empty or full. | Return the same edits without publishing package resources or depending on retained capacity. | Resource-set transition cases around successful, empty, invalid-name, casing-failure, and conflict results. |
| Produce overlapping shared declaration and reference locations. | Emit each edit once in the defined MCP order while normalized shared and LSP edit sets remain equal. | Duplicate-location unit case, exact MCP order case, and normalized cross-adapter comparison. |
| Inspect `tools/list` and call the tool with an empty name, invalid field type, missing field, or extra field. | Advertise the closed input and result schemas; protocol-invalid input does not invoke rename or alter state. | Exact tool-list transcript, schema-validation tests, and follow-up valid call. |
| Apply no returned edits and issue definition or references again. | The server returns the original saved-snapshot result because rename itself performs no mutation. | Same-session MCP transition case. |

## Completion

This proposal is complete when every acceptance row has the planned evidence
named in that row and the evidence agrees across applicable adapter boundaries.
The MCP specification must then state the supported symbols, edit ordering,
conflict-location provenance, failure details, non-mutating behavior, capture
boundary, and state-preservation rules. Delete this page and its Ready catalog
entry after those current authorities cover the implemented behavior.
