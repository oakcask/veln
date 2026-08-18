---
role: proposal
update-when: The agent-language-services plugin clients, supported platforms, compatibility field identities, matrix-reference registry, closure phase boundary, or lifecycle-migration source-universe prerequisite changes.
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

An earlier implementation attempt exposed three defects in this proposal. It
invented literal-looking host, validator, contract, and digest values without
the checked artifacts that could establish them. Its raw-text validator could
accept hidden or displaced matrix and reference text. Its phase guard inferred
matrix presence from whole-document validity and Git rename classification.

More placeholder words, reference phrases, or preclassified rename fixtures
cannot close those defects. This proposal now separates finite planning
identity from later compatibility evidence. It also gives matrix references,
the closure phase, and permitted path transitions exact structural identities.

## Selection State

This proposal is ready. Complete it before selecting the frozen source
inventory PR from
[Agent Language Services Lifecycle Migration](agent-language-services-lifecycle-migration.md).

## Scope

Add one top-level client-platform membership table to
`agent-language-services.md`. The table has exactly the columns `Client` and
`Platform`. Each data row declares one exact client and platform pair.

The canonical matrix section starts with the exact source heading
`### Closed Client-Platform Matrix`. Its top-level blocks occur in this order:

1. the exact row-count paragraph;
2. the exact phase-identity paragraph;
3. the membership table with the headers `Client` and `Platform`;
4. the level-four heading `Compatibility Field Identities` and its one-column
   table with the header `Compatibility field`; and
5. the level-four heading `Matrix Reference Registry` and its registry table.

The section is the final level-three subsection under `## Agent Plugin` and
ends at `## Safety And Privacy`. No other table or phase paragraph appears in
that interval. The later-installer sentence becomes its own paragraph before
the matrix section. All four registered `Agent Plugin` paragraphs occur before
the matrix heading.

Add a separate top-level compatibility-field table. It contains exactly these
ordered field identities and no values:

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

Each future compatibility record selected by a membership row must contain
each of these eleven fields exactly once. The closed field table fixes that
record shape without supplying any field value.

The initial table contains exactly these two ordered client-platform keys:

1. `codex/x86_64-unknown-linux-gnu`
2. `claude-code/x86_64-unknown-linux-gnu`

The matrix section contains these exact source paragraphs:

```markdown
Closed client-platform row count: `2`.
Matrix closure phase: `agent-language-services-platform-matrix-closed`.
```

The closure PR may not select a different platform, add a third row, reorder
the keys, or use a range, wildcard, “all supported platforms,” or an unnamed
future row. A separate proposal must add another client or platform after this
closure completes.

This prerequisite closes membership and field identity, not compatibility
values. It may not invent a host build, schema revision, validator version,
contract revision, or integrity digest for an artifact that has not been
implemented and checked. The plugin implementation records exact values only
from checked client and validator artifacts. A patterned example,
syntactically valid digest, or unresolvable version label is not compatibility
evidence.

The registry has exactly the columns `Reference ID`, `Document`, `Heading`,
`Block`, `Label`, and `Destination`. The label is always
`Closed Client-Platform Matrix`. The registry contains these exact ordered
tuples:

| Reference ID | Document | Heading | Block | Label | Destination |
| --- | --- | --- | --- | --- | --- |
| `agent-plugin-server-lifecycle` | `agent-language-services.md` | `## Agent Plugin` | paragraph 3 | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `agent-plugin-installer-boundary` | `agent-language-services.md` | `## Agent Plugin` | paragraph 7 | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `agent-plugin-compatibility-authority` | `agent-language-services.md` | `## Agent Plugin` | paragraph 8 | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `agent-plugin-native-validation` | `agent-language-services.md` | `## Agent Plugin` | paragraph 9 | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `conformance-requirement-coverage` | `agent-language-services.md` | `## Conformance Contract` | paragraph 1 | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `conformance-capability-membership` | `agent-language-services.md` | `## Conformance Contract` | paragraph 2 | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `q21-plugin-matrix` | `agent-language-services.md` | `## Conformance Contract` | table row `Q21 plugin matrix` | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `q22-gate-totality` | `agent-language-services.md` | `## Conformance Contract` | table row `Q22 gate totality` | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `umbrella-completion` | `agent-language-services.md` | `## Conformance Contract` | paragraph 5 | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `plugin-acceptance-completion` | `agent-language-services.md` | `### Plugin` | table row `Run the proposal completion gate` | `Closed Client-Platform Matrix` | `#closed-client-platform-matrix` |
| `lifecycle-conformance-cells` | `agent-language-services-lifecycle-migration.md` | `## Preserved Finite Inputs` | list item 6 | `Closed Client-Platform Matrix` | `agent-language-services.md#closed-client-platform-matrix` |
| `lifecycle-compatibility-cells` | `agent-language-services-lifecycle-migration.md` | `## Preserved Finite Inputs` | list item 7 | `Closed Client-Platform Matrix` | `agent-language-services.md#closed-client-platform-matrix` |
| `lifecycle-prerequisite-acceptance` | `agent-language-services-lifecycle-migration.md` | `## Acceptance Model` | table row `Close the prerequisite client-platform set` | `Closed Client-Platform Matrix` | `agent-language-services.md#closed-client-platform-matrix` |

Each target block contains exactly this source link, using its tuple values:
`[<label>](<destination> "matrix-ref:<reference-id>")`. The link source does
not allow escapes, entities, alternate destination spelling, or omitted title.
The registry
row does not satisfy the target occurrence. The validator enumerates every
top-level link in the two documents whose title starts with `matrix-ref:` or
whose source destination equals one of the two registry destinations. That complete set must
equal the thirteen tuples. A block ordinal counts all top-level blocks of the
named kind under the named level-two heading in document order. Nested
level-three headings do not reset that count. Moving a link while preserving
the relative order of registered links therefore changes its tuple and fails.
An unregistered block has no authority to define or extend client-platform
membership, even if its prose uses another synonym for a client, host,
environment, or platform set. A separate proposal must revise the registry
before adding another authoritative location.

The phase identity marks that the closure transition has occurred. It does not
change when a later proposal revises matrix membership or repairs another
reference.

## Closed Source Grammar

The validator recognizes only column-zero Markdown blocks in the two scoped
documents. A lexical pass excludes fenced and indented code blocks, HTML blocks
and comments, block quotes, image text, link destinations, and link titles from
ordinary visible text. The grammar recognizes ATX headings, paragraphs, list
items, pipe tables with delimiter rows, inline code, and inline links only in
the exact forms required by this proposal. It does not use a raw substring or
regular-expression presence match as evidence that a block exists.

The scanner normalizes CRLF to LF and uses these states and productions:

- A fence starts with a column-zero run of at least three backticks or tildes.
  Only a column-zero run of the same character and at least the same length
  closes it. Fence contents do not create blocks.
- An HTML comment starts at `<!--` and ends at the next `-->`. A column-zero
  HTML tag starts an HTML block that ends at the next blank line. HTML contents
  do not create blocks. An unclosed comment or HTML block fails.
- A line with four leading spaces is indented code. A line whose first
  non-space character is `>` is a block quote. Neither creates a target block.
- A paragraph is a maximal run of nonblank column-zero text lines that are not
  another recognized block. A list item starts with exact column-zero `- ` and
  owns its following two-space continuation lines. Other indentation fails in
  a registered target block.
- A table is a maximal run of column-zero lines that start and end with `|`.
  Contract tables prohibit backslash escapes, entities, raw HTML, and code spans
  that contain `|`. Their headers, delimiters, cell counts, and cell order must
  match the independent expected manifest exactly.
- The inline scanner recognizes backtick code spans, images, links, backslash
  escapes, and entities. An unclosed inline construct fails. A required matrix
  link must be an ordinary link outside code and image syntax, with the exact
  unescaped label, destination, and quoted title from its registry tuple.

Any unknown construct in the matrix interval or in a registered target block
fails closed. The negative corpus covers fenced and indented code, HTML blocks
and comments, block quotes, paragraph and list continuation, escaped and
code-span pipes, code spans, image alt text, link destinations and titles,
backslash escapes, entities, and unclosed constructs.

The matrix interval starts after the unique exact level-three heading and ends
before the next heading of level three or less. The interval contains only the
ordered blocks listed in Scope. The membership delimiter is exactly
`| --- | --- |`. The one-column table delimiter is exactly `| --- |`. The
registry delimiter is exactly `| --- | --- | --- | --- | --- |`. A paragraph,
heading, table, phase identity, or link outside this grammar does not satisfy
the contract.

## Checked Matrix Contract

Add a documentation validator that parses the closed source grammar
independently from the later frozen-source inventory. The validator records the
ordered client-platform keys, exact row count, ordered field identities,
reference registry, and phase identity. It rejects:

- a missing or duplicate top-level matrix heading, membership table, field
  table, reference registry, or phase identity;
- a matrix, field table, registry, or phase identity that exists only in an
  HTML block or comment, fenced or indented code block, block quote, link
  metadata, or image text;
- a malformed Markdown table delimiter or an unexpected extra table in the
  matrix section;
- a missing or duplicate client-platform key;
- a client-platform key that does not equal the expected key at its expected
  index;
- a missing, duplicate, reordered, or unexpected compatibility field identity;
- a closure table that claims a compatibility value or integrity digest;
- a missing, duplicate, reordered, unexpected, displaced, hidden, or
  wrong-target registry row or matrix link at any registered location;
- a checked row-count value that differs from the table.

The production validator declares literal expectations for the two membership
keys, eleven field identities, thirteen registry tuples, one phase node,
canonical block layout, and nine path operations. It does not derive those
expectations from either proposal document.

The test file declares the same literal expectations independently. It does
not import them from production exports or derive them from a repository
document, registry, or path manifest. Mutation tests use standalone minimal
valid documents. A separate integration case validates the repository
documents. Paired mutations that delete or replace both a registry row and its
target block must fail.

The validator runs through the existing documentation-validation workflow.
Its failure output names the invalid row or reference, tells the maintainer to
enumerate or restore the exact cell, and explains that the lifecycle inventory
cannot prove finite coverage otherwise.

Phase presence is true only when the exact phase paragraph occurs at its
specified top-level location in the closed source grammar. The closure diff
guard is active exactly when phase presence is false in the base revision and
true in the head revision. This predicate is independent of matrix and
reference validity. An active guard runs even when head contract validation
also fails. When both revisions contain the phase paragraph, the guard is
inactive. A later reference repair or separately proposed membership revision
therefore does not reactivate it.

During the closure transition, the guard permits exactly this path-operation
manifest. It derives the canonical tree delta from
`git diff --raw --no-renames -z`. Git rename and copy similarity is not an
input. A moved path therefore appears as `D` for the old path and `A` for the
new path regardless of content similarity. A copied file appears only as `A`
for its destination because its source is unchanged.

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

The required mode pairs are exactly `100644` to `100644` for `M`, `000000` to
`100644` for `A`, and `100644` to `000000` for `D`. The guard rejects a missing
required operation, an additional operation, an operation assigned to the
wrong path, and every other mode pair.

Range tests use temporary Git histories. The permitted proposal-to-record move
must produce the same `D` and `A` operations for high- and low-similarity
content because rename detection is disabled. Tests also cover an unexpected
protected-path move, a copied file as an `A`, a regular file changed to a
symbolic link, an executable-bit change, and an unrelated documentation change
after closure.

## CI Range Contract

The documentation-validation workflow contains one exact step named
`Validate the closed agent language platform matrix`. The step runs this exact
command:

```text
node workflow-scripts/check-agent-language-platform-matrix.mjs
```

The step passes
`AGENT_PLATFORM_MATRIX_BASE_SHA` from
`github.event.pull_request.base.sha || github.event.before` and
`AGENT_PLATFORM_MATRIX_HEAD_SHA` from `github.sha`.

The validator fails when either environment value is absent, is an all-zero
revision, or cannot be read from the checkout. For a pull request, the base is
the event's base commit and the head is the checked merge result. For a push to
the default branch, the base is the before commit and the head is the pushed
commit. The workflow keeps full Git history so both revisions and their tree
delta are available.

The test file contains an independent literal workflow-step expectation. It
validates the checked workflow file without importing step constants from the
production validator. A temporary-Git integration case invokes the production
entry point with base and head environment values and proves that the active
guard rejects an extra path in the actual revision range.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Enumerate the intended plugin compatibility set. | One top-level table contains `codex/x86_64-unknown-linux-gnu` followed by `claude-code/x86_64-unknown-linux-gnu` and no other membership. | Independent expected-key fixture, exact row count of two, duplicate-table mutation, and comment, fence, indented-code, block-quote, and malformed-delimiter mutations. |
| Preserve one compatibility contract shape per cell. | The closed field table contains the eleven ordered field identities exactly once and no compatibility values. Every future record selected by a membership row uses that exact shape. | Missing, duplicate, reordered, unexpected, and value-bearing field mutations. |
| Keep cell identity unique and literal. | A row is valid only when its complete client-platform key equals the expected key at its expected index. Every other key fails as unexpected. | Empty, duplicate, reordered, ranged, wildcard, placeholder, catch-all, and other unexpected-key mutations are representative negative fixtures. |
| Close all references over the same set. | The thirteen registered target blocks each carry their expected link title, structural ordinal, and destination, and the registry has no missing or additional tuple. | Per-location missing, duplicate, displaced, hidden, wrong-section, wrong-block, wrong-title, wrong-target, and same-kind title-swap mutations; missing, reordered, and unexpected registry rows; and paired registry-and-target deletion or replacement. |
| Keep validation independent of raw source coincidences. | Excluded or malformed source blocks cannot supply a matrix, registry, phase identity, or target reference. | Paired top-level and excluded-context cases for every block kind in the closed source grammar. |
| Retire the closure phase exactly once. | Only absence-to-presence of the exact phase node activates the closure guard, independently of contract validity. A later reference repair or separately proposed membership revision does not reactivate it. | Base/head phase table covering absent, closed-valid, closed-invalid-reference, invalid-head, and later-revision states. |
| Keep the prerequisite documentation-only. | The closure range has exactly the nine path operations and mode pairs in the path-operation manifest. No plugin artifact, executable MCP case, harness change, or semantic baseline enters the range. | Temporary-Git tree-delta cases for missing, extra, wrong-operation, moved, copied, symbolic-link, executable-bit, and post-closure documentation changes. |
| Run the guard on the authoritative CI range. | The registered workflow step supplies the pull-request base or push-before commit and checked head to the production validator. Missing or unreadable range inputs fail. | Independent workflow-step fixture and temporary-Git entry-point cases for a valid range, an extra path, missing inputs, an all-zero base, and unreadable revisions. |

## Non-Goals

- Choosing or implementing the future plugin packaging mechanism.
- Claiming support for a client-platform cell before its pinned validation
  passes.
- Choosing placeholder compatibility values before the client and validator
  artifacts exist.
- Adding `compatibility.toml`, client manifests, MCP or LSP smoke tests, or an
  installer.
- Freezing the broader agent-language-services source inventory.
- Adding another client or platform after the exact two-cell set closes.

## Completion Rule

This proposal completes only when all eight acceptance rows pass. The umbrella
proposal must contain the one top-level membership table, the one field table,
the one reference registry, the one phase identity, and exactly the thirteen
registered target references. The closure range must match the path-operation
manifest. Move the completed record out of `docs/proposals/` before selecting
the lifecycle migration's frozen-source-inventory PR.
