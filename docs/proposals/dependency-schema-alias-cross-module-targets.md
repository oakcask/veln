---
role: proposal
update-when: Direct-dependency schema-alias target resolution across package modules, saved operation-reference scope, or cross-adapter acceptance evidence changes.
---

# Direct-Dependency Schema Alias Cross-Module Targets

## Outcome And Readiness

Let a consumer find operation references to a public schema alias whose direct
target is a public schema in another module of the same retained dependency.
Packages can expose a schema through a facade without losing alias navigation.

The saved dependency captures, alias identities, and operation-reference
adapters already exist. The current boundary is specified in
[MCP navigation](../specification/mcp.md) and
[editor navigation](../specification/editor-support.md).
The language-service test
`valid_cross_module_dependency_schema_alias_target_stays_outside_reference_slice`
in `crates/veln-language-service/src/tests/dependencies_schema_references.rs`
explicitly excludes this valid source form. The
`codec-schema-references` executable specification independently establishes
cross-module schema-alias syntax. No new import syntax, MCP tool, package
capture format, or alias-chain support is needed. This is an implementation
target independent of the remaining
[language-services inventory](agent-language-services.md).

## Planned Contract

Extend the existing direct-dependency schema-alias operation-reference
eligibility rule to a qualified direct target that resolves uniquely to a
public schema declaration in another module of the same retained package.
Resolution follows the package's ordinary schema namespace, imports,
visibility, and casing rules. Target lookup must not use the consumer's
imports. Alias chains remain unsupported.

The selected identity remains the package and alias declaration, not the
target schema. Selecting an eligible consumer `decode` or `encode` alias leaf
returns all resolved operation leaves for that alias in the selected project's
captured owned sources. Full written import paths and valid implicit leaf
aliases select the same identity. Existing same-module bare-target eligibility
and import precedence remain unchanged.

Results retain the existing sorting, deduplication, coordinates, project scope,
and saved-capture contract. Package declarations, package-internal uses, alias
target expressions, composition leaves, sibling aliases, and direct target
schema uses are excluded. LSP declaration inclusion does not add a package
declaration. MCP uses the existing checked `references` schemas. Definition,
rename, standard-library aliases, transitive dependencies, recovery navigation,
and pagination are outside this proposal.

## Acceptance Model

This table specifies planned evidence, not passing tests. In its positive
fixture, the dependency exports `core` and `facade`; `core` declares public
schema `Packet`, and `facade` imports `core` and declares
`pub schema Alias = core::Packet`. The consumer imports `facade` from that
dependency and uses `facade::Alias` in both operations. Checked source cases
must establish valid source independently of navigation expectations.

| Input or event | Required observation | Planned evidence |
| --- | --- | --- |
| Select either consumer operation leaf for the positive fixture. | Both selections return the identical exact URI/range set for the alias's decode and encode leaves. | Language-service identity case and paired MCP/LSP executable cases. |
| Place the target in a nested module and use its full imported path or valid implicit leaf alias in the facade. | Both valid spellings resolve to the same target; consumer operation references retain the facade alias identity. | Checked source cases and target-resolution matrix. |
| Put a sibling alias, direct target use, same-spelled alias from another dependency, and a same-spelled consumer symbol beside the positive uses. | Each selected alias returns only its own operation leaves. | Exact-set identity matrix in language-service tests. |
| Give the consumer an unrelated import with the target module's name. | It does not alter target resolution inside the dependency. | Package/consumer namespace isolation case. |
| Target a private, missing, wrong-kind, invalid-cased, ambiguous, or recovered declaration, or use a duplicate or recovered target import. | The alias is ineligible; MCP and LSP return successful empty reference results without fallback to a same-spelled schema. | Target eligibility matrix and protocol boundary cases. |
| Target another alias, another package, or a transitive dependency; select a standard-library alias. | Successful empty results preserve those unsupported boundaries. | Existing boundary cases plus cross-module variants where needed. |
| Use a private or non-exported facade alias, or an invalid consumer import. | No alias references are exposed. | Existing alias/import boundary matrix extended with qualified targets. |
| Add package-internal uses, another selected project, an unselected descendant project, and composition or alias-target leaves. | None enters the selected project's operation-reference set; selecting excluded leaves returns an empty set. | Scope matrix and paired MCP/LSP cases, including declaration inclusion. |
| Select an eligible same-module bare-target alias. | Its existing identity and reference results are unchanged. | Existing dependency schema-alias regression cases. |
| Change dependency inputs during saved capture until retries are exhausted. | MCP reports `snapshot_changed` without partial locations or scope; prior selection and retained resources remain intact. | MCP capture-failure test using the new qualified-target fixture. |

## Verification And Completion

Use the existing language-service dependency-schema tests, MCP server tests,
and paired executable cases under `examples/specification/` to verify the
table. Update the current cross-module empty-result assertions only for this
new eligibility boundary; keep alias-chain and other-package assertions.
Run focused Rust tests through `scripts/agent-test` and executable cases
through the repository's guarded specification harness. Audit analysis cost
under the performance-regression skill when changing target lookup.

This refinement deliberately adds no implementation or claimed executable
coverage. During implementation, add checked evidence first, then update the
smallest matching MCP and editor specification sections. Remove this proposal
and its Ready entry once those acceptance cases pass and the umbrella routes
this slice to the current specifications.
