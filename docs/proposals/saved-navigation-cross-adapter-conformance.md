---
role: proposal
update-when: The saved LSP or MCP definition, references, coordinate, declaration-inclusion, pagination, or navigation-failure contract changes.
---

# Saved Navigation Cross-Adapter Conformance

## Outcome

Add paired executable evidence that LSP and MCP preserve the same shared
language-service navigation result for one saved workspace snapshot. The
evidence must compare normalized source identities and Unicode-scalar half-open
ranges rather than requiring the protocols to serialize the same wire shape.

This proposal covers conformance for behavior already specified by
[Editor Support](../specification/editor-support.md) and
[MCP Workspace Projects, Resources, And Navigation](../specification/mcp.md).
It does not add a symbol class or expand dependency traversal.

## Normalization Model

The paired harness must normalize each successful adapter response before it
compares results:

- Convert an LSP `file:` or `veln-pkg:` URI and its zero-based UTF-16 positions
  to the retained source identity and one-based Unicode-scalar half-open range
  used by the shared language service.
- Convert an MCP canonical URI and one-based Unicode-scalar range to the same
  source identity and range form.
- Compare definitions before protocol-specific envelopes are considered.
- Compare complete reference sets after collecting MCP continuation pages and
  after applying the same declaration-inclusion choice to both adapters.
- Preserve adapter-specific failures as failures. Do not normalize a rejected
  request to an empty successful result.

The normalization code is test support. It must not become a second production
implementation of symbol selection or reference discovery.

## Acceptance Cases

| Case | Observable acceptance | Required evidence |
| --- | --- | --- |
| Workspace definition | A selection at the same token through LSP and MCP normalizes to the same workspace definition identity and range. | One paired case with exact normalized equality. |
| Package definition | A direct-dependency or standard-library selection normalizes to the same package source identity and declaration range without materializing a new source path. | One paired `veln-pkg:` case with exact normalized equality. |
| References without declaration | The same saved selection and declaration-exclusion choice produce equal normalized reference sets. | One paired multi-source case that compares the complete sorted sets. |
| References with declaration and pagination | Declaration inclusion adds the same eligible workspace declaration, and collecting an MCP continuation page yields the same complete set as LSP. | One paired case whose MCP page size requires at least one continuation. |
| Coordinates | LF, CRLF, a non-BMP scalar before the selection, token-end selection, and end-position ranges preserve the documented half-open selection and output ranges. | A table-driven paired coordinate matrix using LSP UTF-16 and MCP Unicode-scalar positions. |
| Empty selection | A valid position that selects no supported symbol succeeds with no definition or an empty reference set in both adapters after normalization. | Paired definition and references assertions. |
| Invalid position | A position outside the retained source is rejected according to each protocol and produces no normalized navigation result. | Paired protocol-failure assertions that also prove no continuation cursor is created. |
| Unsupported declaration policy | A package selection that is ineligible for declaration inclusion keeps the same normalized references in both adapters. | A paired package reference case with declaration inclusion requested. |

## State Preservation

A rejected request must not mutate the saved snapshot or prior successful
result. After the invalid-position case, the harness must repeat one earlier
successful selection and observe the same normalized result. The failed MCP
request must not create a usable continuation cursor.

## Completion

This proposal is complete when the paired harness and every acceptance row are
checked by the repository test route, and the two current specification pages
identify that evidence as cross-adapter conformance coverage. Remove this page
and its Ready entry after those current authorities are updated.
