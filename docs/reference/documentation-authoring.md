---
role: reference
authority: normative
update-when: The repository documentation classification, metadata, presentation, specification-writing, or verification policy changes.
---

# Documentation Authoring Policy

This document defines how Markdown under `docs/` is classified, written, and
maintained. Routing documents select the smallest relevant document. Skills
define the steps used to perform and verify documentation work. Neither should
duplicate the rules in this document.

This document governs documentation structure and presentation. A current
behavior page under `../specification/` remains authoritative for the product
behavior it specifies.

## Document Roles and Authority

Every added or modified Markdown document under `docs/` must declare one role.

| Role | Purpose | `authority:` | Exceptional `status:` |
| --- | --- | --- | --- |
| `routing` | Selects another document without defining behavior | Not allowed | `closed` or `superseded` |
| `specification` | Defines current behavior or a current project contract | `normative` is required | Not allowed |
| `proposal` | Defines planned or incomplete behavior | Not allowed | `closed`, `rejected`, or `superseded` only after leaving `docs/proposals/` |
| `reference` | Defines stable requirements, policy, rationale, or source support | `normative` or `supporting` is required | `superseded` |
| `review` | Records bounded findings, diagnostic evidence, or quality-gate results | `supporting` is optional | `superseded` |
| `implementation-record` | Preserves completed proposal history or completion evidence | `supporting` is optional | `superseded` |

Use `status:` only for an exceptional lifecycle state. Do not add it to an
active proposal, ordinary route, or current supporting record. Do not repeat
the status as a `Status:` label in the document body.

## Frontmatter

Put YAML frontmatter at the start of every added or modified Markdown document
under `docs/`. Declare:

- one `role:` value from the role table;
- one single-line `update-when:` value;
- `authority:` only as required or permitted by the role table;
- `status:` only as permitted by the role table.

An `update-when:` value must name an observable project-state change that can
make the document stale. Write it so an agent can select the document from the
value alone, without its title or body. Name the affected contract, command,
component, schema, artifact, evidence, or document set.

Do not use calendar schedules or vague triggers such as `periodically`,
`regularly`, `as needed`, `when necessary`, `always`, or `TBD`. Avoid phrases
such as `the documented behavior`, `its evidence`, `this record`, or `the
routed task` unless the same value first names the concrete referent.

Use the narrowest sufficient trigger. State related alternatives in the same
field when any one of them requires review. Do not add a second `update-when:`
field.

```markdown
---
role: specification
authority: normative
update-when: The CLI command output contract or its checked command-output fixtures change.
---
```

```markdown
---
role: routing
update-when: A CLI specification page is added, moved, reclassified, or removed.
---
```

## Placement and Lifecycle

- Use `docs/specification/` for current implemented language behavior.
- Use `docs/proposals/` only for active `role: proposal` targets that are not
  fully implemented.
- Remove or relocate rejected, superseded, implemented, and otherwise closed
  proposal pages from `docs/proposals/`.
- Use `docs/reference/source-decisions/` for implemented rationale and decision
  history.
- Use `docs/reference/implemented-proposals/` for completed proposal history
  and completion evidence, not current behavior.
- Keep implementation gaps, verification evidence, and correction lists in
  the matching proposal or reference page.
- When prose and executable evidence disagree, update the implementation,
  executable evidence, or prose together.

## Routing and Detail

Routing documents exist for discovery. They must tell readers what to read,
when to read it, and what to skip. They must not become a second authority for
the rules or behavior they route. They may define their own selection and
stopping behavior.

- Keep top-level and directory routing documents short.
- Route by task or subject to the smallest current page.
- Link to the authoritative document instead of copying its rules.
- Include only enough summary to distinguish one route from another.
- Prefer sections such as `Read First`, `Read When`, and `Skip Unless Needed`
  over long catalogs or historical narration.
- Put the most actionable route first.
- Link with relative paths.
- Do not route ordinary task work through old reviews or historical records.

Keep each non-routing document small enough that a reader can use one subject,
contract, or maintenance responsibility without loading unrelated material.
When a document covers several independently useful subjects, split those
subjects into clearly named pages and add or update the nearest directory README
to route among them. Add another directory level when it gives several related
pages a meaningful shared scope and keeps unrelated routes out of the parent
index. Let the parent index route to that directory README instead of listing
every descendant page.

Do not create or retain a summary page and a `*-full.md` counterpart that
describe the same scope at different lengths. That structure makes readers
compare two accounts and makes maintainers update the same facts twice. When a
documentation change updates either member of an existing same-scope pair,
retire the pair in the same change:

- If the content has one subject and authority, merge it into one document and
  delete the other file.
- If the content has independently useful subjects, move them into focused
  pages under the nearest meaningful hierarchy and delete both paired files.
- Update every internal link to the remaining document or focused pages. Do not
  retain an old path as a routing or compatibility document; link verification
  must identify internal references that still require migration.

Split or add a routing index when:

- an entry point is mostly a flat list of many files;
- one document contains multiple subjects that readers update independently;
- one file mixes current guidance with historical evidence;
- readers must open more than one long file before identifying the relevant
  content.

Choose boundaries by subject and ownership, not by a target line count. Do not
split a focused document merely to make it shorter. Do not create a trivial
one-file directory when a heading link provides the needed route.

## Presenting Information

Present enumerations as bulleted lists, numbered lists, or tables by default.

- Use bullets when item order does not matter.
- Use numbers when order or rank is significant.
- Use a table when readers need to compare the same fields across items.
- Keep an enumeration inline only when it is short and reading its items
  independently would not improve clarity.
- Split a long enumeration into named groups when one flat list remains hard to
  scan.
- Do not hide test cases, evidence, requirements, exceptions, or alternatives
  in a comma-separated prose paragraph.

## Behavior Specifications

Make a requirement normative only when it protects a durable user, product,
interoperability, safety, or maintenance outcome. Treat a review finding,
hypothetical bypass, rejected implementation, or one-time migration concern as
evidence to assess, not as a requirement by itself. Prefer deletion or
simplification when it resolves the underlying risk.

Specify behavior as an externally observable contract. State inputs, outcomes,
failures, visible state transitions, and invariants. Do not turn an incidental
implementation algorithm, data structure, or operation order into a normative
requirement. Explain why an internal detail is normative when compatibility,
safety, performance, or another explicit constraint requires it.

Do not make document layout, paragraph order, test organization, workflow step
names, or a particular change path normative unless an external compatibility
or safety constraint depends on that exact structure.

Use the strongest practical verification medium:

- use executable doctests for API examples and documented source behavior;
- use table-driven tests, checked fixtures, CLI cases, or executable examples
  for inputs, outputs, diagnostics, commands, and serialization;
- use executable grammar, schemas, and accepted and rejected fixtures for
  source surfaces;
- use transition tables with current state, event, guard, next state, outputs,
  and failures, mapped to tests, for stateful or protocol behavior;
- use decision or truth tables for material rule combinations;
- use benchmarks with a named workload, metric, comparison method, and noise
  policy for performance claims;
- use concise prose for rationale, scope, non-goals, provenance, and genuinely
  non-mechanical constraints.

Do not use a benchmark to specify functional behavior or claim a stable
threshold on an uncontrolled runner.

Use flowcharts or other diagrams only when they materially clarify
relationships. Treat them as supporting views unless they are generated from
the authoritative artifact. Otherwise, name the authoritative transition
table, tests, or executable model.

When an internal algorithm or ordered procedure needs prose explanation, use
Simplified Technical English style without claiming formal conformance:

- use one action or condition per sentence;
- use short sentences with an explicit subject and verb;
- use one term for one concept and define specialized terms;
- prefer active voice when the actor is relevant;
- put a condition before the action that depends on it;
- use numbered steps when order matters;
- use one imperative action per procedural step;
- state expected results when they are not obvious;
- avoid ambiguous pronouns, nested conditions, idioms, and informal metaphors.

Planned behavior must identify its acceptance model and intended verification.
Do not present planned evidence as implemented or passing. Do not invent exact
versions, digests, compatibility values, or expected outputs when no
independent authority can establish them. Specify the evidence that must
produce the value or keep the work explicitly incomplete.

Current behavior must route to checked evidence when practical. Keep prose
thin when executable evidence can own the detail. Name the authoritative
artifact when prose, tables, diagrams, generated pages, and tests describe the
same behavior. Copies of the same expected values can detect drift, but they do
not independently establish correctness. Prefer generating secondary views
from the authoritative artifact. Update all representations together when
generation is impractical, and treat disagreement as a defect.

Give one-time transition evidence an explicit retirement path. If only prose
can express a normative claim, state why stronger representation is not
practical and make the prose falsifiable.

## Verification Outcomes

A documentation change is ready when:

- every changed Markdown document has valid frontmatter for its role;
- every changed or moved relative link resolves;
- routing documents point to the smallest current authority without copying
  its rules;
- a change that updates either member of a same-scope summary/detail pair
  removes that pair and updates its internal links;
- split documents have subject-based boundaries and do not duplicate one
  authority;
- normative claims have observable acceptance conditions;
- prose and executable or mechanically checked evidence agree;
- planned evidence is not presented as implemented;
- long enumerations remain independently scannable.
