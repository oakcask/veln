---
role: reference
authority: normative
update-when: Proposal acceptance models, readiness and completion criteria, feasibility investigation selection, canonical source requirements, or related-work evidence rules change.
---

# Proposal Authoring Policy

This policy governs authoring and review of planned or incomplete behavior
under `docs/proposals/`. Apply the shared classification, metadata, and
[behavior-specification rules](documentation-authoring.md#behavior-specifications)
from the documentation authoring policy. The [proposal catalog](../proposals/README.md)
owns selection and lifecycle routing; the current language specification owns
implemented behavior.

## Acceptance Model

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

## Readiness and Completion

Ready means that a proposal can be selected for implementation. Its scope,
observable behavior, material design choices, acceptance conditions, and
intended verification must be sufficiently resolved to implement and review
the target without inventing its contract. Design In Progress identifies
unresolved design choices or dependencies that prevent that selection.

Keep implementation completion separate from readiness. Implementing the
feature, passing its acceptance tests, and updating current specifications and
executable examples are completion conditions. Defining acceptance cases and
verification routes does not require those checks to pass before Ready.
Do not add prototype, measurement, or passing-test prerequisites merely to
increase confidence or satisfy the requirement for verifiable behavior.

Pre-Ready feasibility evidence is justified only when an unresolved question
prevents choosing the public API, semantics, or scope. Name that question and
explain which design choice changes with the possible findings. Bound the
investigation to answering it; do not require the feature's implementation or
full acceptance suite as evidence that implementation may begin. For example,
whether an existing representation can preserve a required scalar value can
change the construction API; eventual execution of every numeric API case is
implementation acceptance.

When resolving that question needs code or measurement, define a separate,
independently complete investigation proposal under Ready with a bounded
question, expected observations, and a completion condition. It must be
selectable without implementing the parent feature. The parent remains Design
In Progress until the findings resolve its blocked choice. Investigation
completion does not automatically promote the parent or establish production
behavior; review the findings against the parent's contract first.

When authoring or reviewing readiness conditions, move implementation-only
checks to completion conditions and retain only decision-relevant feasibility
gates. An existing gate's presence is not its justification. Review these
boundaries semantically; headings, keywords, or a prescribed document layout
cannot establish compliance.

## Canonical Form

For source-syntax design and review, follow the
[Source Syntax Design Policy](source-syntax-design.md). Classify each changed
form as a single expression, a block, or a supported pair; explain token
boundaries, scope and cleanup behavior, and the evidence that will verify
those choices. A block does not imply that a corresponding expression form
must exist. Proposed syntax remains proposed until implementation evidence
supports promotion to the current specification.

A proposal that adds or changes language syntax must define its canonical
source form and the intended `veln fmt` behavior in the proposal itself. This
lets reviewers assess accepted syntax and the source users will maintain
together. Do not defer these decisions to implementation or later review.

Cover the following outcomes for the syntax in scope:

- **Canonical output:** identify the preferred spelling and layout with
  representative input/output examples. Distinguish accepted input forms from
  canonical output, including materially different single-item, multi-item,
  nested, and multiline forms where applicable.
- **Convergence and migration:** state which equivalent spellings converge to
  the same output. For affected existing spellings, decide whether ordinary
  formatting rewrites them automatically, conversion needs an explicit action,
  or they remain unchanged, and explain why. Define the eligibility and
  boundaries of any rewrite; accepting an old spelling does not by itself
  decide its formatter behavior.
- **Preservation and stability:** define the semantic invariants and comment
  attachments that formatting must preserve, and what happens when a rewrite
  cannot preserve them. Require identical output on a second format. Identify
  intentional exceptions to convergence and their observable conditions.
- **Acceptance evidence:** map representative before/after pairs, convergence,
  idempotence, preservation, and rewrite boundaries to planned formatter
  fixtures or checked CLI cases. Use semantic checks where formatting rewrites
  syntax in ways that can affect evaluation or scope. Keep planned checks
  distinct from evidence that already exists.

Apply these requirements only to affected forms. If a dimension is unaffected
or inapplicable, state why and link to the existing rule when one governs it.
Review the actual examples and rules for completeness and consistency with the
proposed grammar and semantics; a canonical-form heading or an idempotence
claim alone is insufficient. This is a semantic review requirement, not a
prescribed section layout.

## Related Work

When creating or substantively revising a proposal, include related work that
informs the design and its alternatives. Review relevant literature, web
materials, past Veln pull requests, and other open-source implementations as
appropriate to the subject. Do not require an entry from every source category
when it has no bearing on the proposal.

For each source used in a design claim, record:

- **Identity and locator:** a title or project name and a direct source link.
  Use a DOI or another stable identifier for literature when available. For a
  Veln pull request, include its number and link. For an implementation,
  identify the repository, revision or release, and relevant code or tests;
  prefer a permalink to the inspected revision.
- **Relevant finding:** the behavior, mechanism, result, or tradeoff supported
  by the inspected passage, discussion, code, or tests.
- **Applicability and limits:** how the source relates to Veln's constraints,
  including differences that limit transferring its conclusions.
- **Design consequence:** what the proposal adopts, adapts, rejects, or leaves
  open, and why. Link the source beside the decision or alternative it informs
  so readers can assess that reasoning without reconstructing a bibliography.

Read the relevant primary material before attributing a claim to it. Distinguish
source findings from the proposal author's inference. A past pull request is
evidence of discussion or a change at that revision; compare current-behavior
claims with the matching specification and implementation. An external
implementation is comparative evidence, not authority for Veln behavior.

If relevant sources cannot be found or inspected, state the search scope or
access limitation and the design questions that remain unsupported. Mark
uninspected candidate sources as follow-up rather than verified support. Do not
invent citations or treat a lack of search results as proof of novelty.

Review source support semantically: check that locators identify the material
used, findings agree with that material, and the proposal explains the resulting
choice and material differences. A source list or a heading alone does not
establish this. Keep acceptance conditions and planned verification explicit;
related work supports rationale but does not demonstrate that Veln implements
the proposed behavior.
