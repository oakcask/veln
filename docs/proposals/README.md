---
role: routing
update-when: A proposal is added, moved, reclassified, completed, or removed.
---

# Proposals

The catalog in this directory contains only planned or incomplete work. Every
proposal page in this directory declares `role: proposal`. Proposal text is not
current language behavior unless the matching page under `../specification/`
also states it.

## Read First

- Current behavior: [Language Specification](../specification/README.md).

## Blocked

- Explicit import-alias casing is blocked until an owning proposal defines the
  syntax and lookup contract:
  [identifier-casing-explicit-import-aliases.md](identifier-casing-explicit-import-aliases.md).
- MCP rename casing mapping is blocked until an owning proposal defines the MCP
  rename tool contract:
  [identifier-casing-mcp-rename.md](identifier-casing-mcp-rename.md).
- The agent-language-services umbrella remains a planning inventory only for
  the unimplemented alias, transitive-dependency, recovery, casing-neutral,
  cross-adapter conformance, published-reference, and plugin work. Do not
  select the umbrella directly. Its independently ready MCP reference
  declaration-inclusion slice is listed above:
  [agent-language-services.md](agent-language-services.md).

## Selection Rule

Before implementing a proposal, compare it with the matching specification
page and executable cases. Select only a complete proposal page listed under
Ready. If Ready contains no suitable implementation target, report
that there is no target instead of selecting work from Blocked.

Do not select work that is already covered by the current specification or
only extends a numbered, width-based, arity-based, route-count, or diagnostic-id
sequence. Such work needs a concrete new capability.

## Proposal Shape

Express observable targets as structured acceptance cases, decision tables,
state-transition tables, executable models, or another directly verifiable
form when practical. Map those targets to the tests, fixtures, doctests,
benchmarks, or executable specifications that will verify implementation.
Keep prose for scope, rationale, non-goals, and constraints that cannot
reasonably be expressed in the primary verification medium. Do not describe
planned evidence as already passing.

State proposed behavior declaratively as an externally observable contract.
Keep internal algorithms and ordered procedures outside normative behavior
unless they are required design constraints. Use Simplified Technical English
style when those details need prose explanation.

## Update When

Promote observable behavior to executable evidence under
`../../examples/specification/` first when practical, then update the smallest
matching specification page. Delete completed, rejected, superseded, and
otherwise closed proposals and remove them from this catalog. Preserve only
durable rationale that remains useful independently of proposal completion
under `../reference/source-decisions/`.
