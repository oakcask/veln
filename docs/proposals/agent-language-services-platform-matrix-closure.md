---
role: proposal
update-when: The agent-language-services canonical client-platform source block, compatibility field identities, registered matrix-reference selectors, artifact-backed compatibility evidence, or lifecycle-migration source-universe prerequisite changes.
---

# Agent Language Services Platform Matrix Closure

## Summary

Replace the undefined agent-plugin platform set with a literal closed matrix.
The lifecycle migration cannot freeze every client-platform cell while the
umbrella proposal names clients but quantifies over an unnamed set of
“supported platforms.”

This is a documentation-contract prerequisite. It does not implement a plugin,
run host validation, or claim that a client-platform cell already passes.

## Correction Boundary

An implementation attempt showed that the previous completion rule was
self-contradictory. It required exact host, schema, validator, contract, and
digest values while excluding the artifacts and host validation that could
establish those values. Copying literal-looking values into a proposal, a
validator, and its tests proves only textual agreement.

This proposal now closes client-platform membership and compatibility-field
identity only. Later plugin implementation must derive compatibility values
from checked client and validator artifacts. That work remains incomplete and
cannot use this documentation-only closure as evidence that a value is valid.

A later implementation attempt exposed a second contradiction. The validator
was asked to recognize visible semantic Markdown blocks and reject every
implicit platform-set synonym, while the same contract declared a finite
ten-reference registry and excluded arbitrary natural-language
classification. Mutation-driven additions to a private Markdown parser cannot
close that acceptance set. This corrected proposal instead defines one finite
source grammar and exact structural selectors. Markdown outside those selected
source blocks is not compatibility-set authority.

## Selection State

This proposal is ready. Complete it before selecting the frozen source
inventory PR from
[Agent Language Services Lifecycle Migration](agent-language-services-lifecycle-migration.md).

## Scope

Add one literal client-platform membership table to the canonical matrix source
block in `## Agent Plugin` of `agent-language-services.md`. The block starts
with the column-zero line `### Closed Client-Platform Matrix`. The `Client` and
`Platform` header, separator, and data rows follow after one empty line. No
other content occurs between that heading and table. Each data row declares one
exact client and platform pair.

Add the column-zero line `#### Compatibility Field Identities` after the
membership table and one empty line. The one-column `Field` table follows after
one empty line. It names the fields that every future `compatibility.toml`
record selected by a membership row must contain:

1. `client`
2. `platform`
3. `host-build`
4. `manifest-schema`
5. `validator-version`
6. `validator-integrity`
7. `veln-contract`
8. `mcp-contract`
9. `lsp-contract`
10. `language-service-contract`
11. `reference-schema-contract`

The canonical matrix source block occurs exactly once under the `## Agent
Plugin` owner. It is the contiguous source range from `### Closed
Client-Platform Matrix` through the `| reference-schema-contract |` row. No
other line occurs in that range except the headings, empty lines, table headers,
separators, and rows defined above.

The initial table contains exactly these two ordered client-platform keys:

1. `codex/x86_64-unknown-linux-gnu`
2. `claude-code/x86_64-unknown-linux-gnu`

The closure PR may not select a different platform, add a third row, reorder
the keys, or use a range, wildcard, “all supported platforms,” or an unnamed
future row. A separate proposal must add another client or platform after this
closure completes.

The compatibility-field table contains field identities only. This closure
must not choose a host build, schema revision, validator version, contract
revision, or integrity digest. Later plugin work records each exact value only
after an authoritative client or validator artifact and its validation result
exist.

Update these exact source blocks so each registered quantifier over
client-platform cells routes to the literal table. A paragraph selector is the
listed literal prefix at column zero and continues through the next empty line.
A table-row selector is the listed literal row prefix at column zero and ends
at that source line's final `|`.

| Reference ID | Owner heading | Exact source selector |
| --- | --- | --- |
| `plugin-server-lifecycle` | `## Agent Plugin` | `Each row in the` |
| `plugin-installer-boundary` | `## Agent Plugin` | `A later proposal may` |
| `plugin-compatibility-authority` | `## Agent Plugin` | `For every row in the` |
| `plugin-native-validation` | `## Agent Plugin` | `Every row in the` |
| `conformance-requirement-coverage` | `## Conformance Contract` | `The manifest contains one requirement ID and at least one planned evidence ID` |
| `conformance-capability-membership` | `## Conformance Contract` | `The v1 manifest closes the capability matrices that this proposal previously` |
| `q21-plugin-matrix` | `## Conformance Contract` | `| Q21 plugin matrix |` |
| `q22-gate-totality` | `## Conformance Contract` | `| Q22 gate totality |` |
| `umbrella-completion` | `## Conformance Contract` | `The proposal completes only` |
| `plugin-acceptance-completion` | `### Plugin` | `| Run the proposal completion gate. |` |

The validator's independent expected-reference fixture contains exactly these
ten reference IDs, owner headings, and source selectors. It does not derive
that expected set from the edited document. A separate proposal must register
a new selector before another source block can become compatibility-set
authority.

Each owner is a unique full heading path. `## Agent Plugin` and `## Conformance
Contract` are children of the document title. `### Plugin` is a child of the
unique `## Acceptance Model`. An owner interval begins after its heading and
ends before the next column-zero ATX heading of equal or smaller level. A
missing or duplicate owner heading fails.

Each selected source block contains this exact ordinary source link, with its own
reference ID substituted:

```markdown
[Closed Client-Platform Matrix](#closed-client-platform-matrix "matrix-ref:<reference-id>")
```

For a paragraph selector, the exact link occupies its own column-zero source
line in the selected paragraph. That line contains only the link and the comma
or period required by the surrounding sentence. For a table-row selector, the
exact ordinary link occurs after the selector on the same source line. That
line contains no comment delimiter, image marker, code-span delimiter, or
backslash escape. The selected block contains its own reference ID exactly
once. A matching link elsewhere does not satisfy that selector. The whole file
contains no unregistered `matrix-ref:` identity.

## Finite Validation Grammar

The validator reads the canonical source forms above. It does not implement a
general Markdown renderer and does not classify English synonyms.

- The owner headings, matrix headings, table delimiters, cells, reference
  selectors, and links use the exact column-zero source forms defined here.
- The canonical matrix source block must be inside `## Agent Plugin` and before
  the next level-two heading. The field table must be inside that matrix block.
- Only the selected matrix block defines client-platform membership and field
  identity. Text in comments, code, quotations, images, link metadata, or
  unrelated headings is not another authoritative matrix and need not be
  parsed as one.
- Only the ten selected source blocks quantify over the matrix. Reviewers must
  reject a proposed eleventh quantifier or synonym unless a separate proposal
  first adds its exact selector and reference identity.

This source grammar deliberately trades Markdown spelling flexibility for a
closed mechanically verifiable contract. The documentation remains ordinary
renderable Markdown, but the validator rejects changes to the canonical forms
instead of guessing whether alternate Markdown has the same meaning.

## Checked Matrix Contract

Add a documentation validator that extracts the canonical membership and
field-identity source block independently from the later frozen-source
inventory. The validator records the ordered client-platform keys, exact row
count, ordered field identities, and registered reference selectors. It
rejects:

- a missing or duplicate client-platform key;
- a canonical heading or table displaced from its owner or exact adjacency;
- an empty client or platform identifier;
- a range, wildcard, placeholder, or catch-all row;
- a missing, duplicate, reordered, or unexpected compatibility-field identity;
- a compatibility value or integrity digest added by this closure;
- a missing, duplicate, unknown, or wrongly selected `matrix-ref:` identity;
- a canonical membership row count other than the independent expected count
  of two.

The validator runs through the existing documentation-validation workflow. Its
failure output names the invalid row, field identity, selector, reference, or
path; tells the maintainer to enumerate or restore the exact item; and explains
that the lifecycle inventory cannot prove finite coverage otherwise.

The closure diff guard is active when the base revision contains zero
column-zero `### Closed Client-Platform Matrix` headings in the `## Agent
Plugin` owner interval and the head revision contains at least one. Activation
does not depend on row, field, reference, uniqueness, or whole-document
validity. Those checks report independently. The guard retires after that
transition merges. The active guard permits exactly this path-operation
manifest, derived without Git rename or copy detection:

```text
M .github/workflows/workflow--test-scripts.yaml
M docs/proposals/README.md
M docs/proposals/agent-language-services-lifecycle-migration.md
D docs/proposals/agent-language-services-platform-matrix-closure.md
M docs/proposals/agent-language-services.md
M docs/reference/implemented-proposals/README.md
A docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md
A workflow-scripts/check-agent-language-platform-matrix.mjs
A workflow-scripts/check-agent-language-platform-matrix.test.mjs
```

Modified and deleted paths start as regular non-executable files. Added and
modified paths end as regular non-executable files. The guard rejects a missing
operation, extra path, different operation, executable-bit change, or Git type
change. It does not permit unrelated CI remediation. A range test proves that
an unrelated documentation change after closure does not inherit the closure
allowlist. Separate tests reject a protected-path move represented as deletion
and addition other than the required proposal-to-implementation-record archive
operation, and a regular file changed to another Git type.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Enumerate the intended plugin compatibility set. | The canonical matrix source block contains `codex/x86_64-unknown-linux-gnu` followed by `claude-code/x86_64-unknown-linux-gnu`, and no other row. | Independent expected-key fixture; owner, adjacency, header, row-count, order, and value mutations. |
| Preserve one compatibility contract shape per cell. | The canonical field source block contains the eleven ordered compatibility-field identities exactly once and contains no values. Every future record selected by a membership row uses that shape. | Owner, adjacency, header, missing, duplicate, reordered, unexpected, and value-bearing mutations. |
| Keep cell identity unique and literal. | Empty, duplicate, ranged, wildcard, placeholder, and catch-all client-platform keys fail. | Injected rejection cases for each invalid key class. |
| Close all registered references over the same set. | Each of the ten exact selectors resolves once under its owner and contains its own exact titled matrix link at the required terminal position. No unknown reference identity exists. | One missing-selector and one misplaced-link mutation per registry row, plus duplicate, unknown, wrong-title, and wrong-destination mutations. |
| Keep the prerequisite documentation-only. | The matrix-addition transition does not add plugin artifacts, executable MCP cases, harness changes, semantic baselines, or unrelated CI remediation, and its allowlist retires after merge. | Phase-aware diff-scope cases for the matrix transition, an extra path, a later unrelated docs change, a protected-path rename, and a Git type change. |

## Non-Goals

- Choosing or implementing the future plugin packaging mechanism.
- Claiming support for a client-platform cell before its pinned validation
  passes.
- Choosing compatibility values before checked client and validator artifacts
  establish them.
- Adding `compatibility.toml`, client manifests, MCP or LSP smoke tests, or an
  installer.
- Freezing the broader agent-language-services source inventory.
- Adding another client or platform after the exact two-cell set closes.
- Parsing arbitrary Markdown visibility or classifying natural-language
  synonyms for client-platform sets.
- Adding this planning prerequisite to the current MCP behavior specification.

## Completion Rule

This proposal completes only when all five acceptance rows pass and all ten
registered selectors contain their exact titled matrix links. The completed
record preserves both Correction Boundary findings. This closure changes a
finite input to planned plugin work, not implemented MCP behavior, so it does
not add or modify a page under `docs/specification/`. Move the completed record
out of `docs/proposals/` before selecting the lifecycle migration's
frozen-source-inventory PR.
