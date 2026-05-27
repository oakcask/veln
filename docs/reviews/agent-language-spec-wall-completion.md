# Agent-Language Spec Wall Completion Review

Status: complete for selected implementation targets and the current advisory
repair candidate boundary.

This review covers the former no-target route. The historical implementation
target was the repair-loop portion of the design-wall material under
[../proposals/agent-language-spec-wall/README.md](../proposals/agent-language-spec-wall/README.md).
The completed implementation target is the advisory, machine-readable repair
candidate boundary before a dedicated repair command exists.

## Completion Check

- `../proposals/` now allows proposal pages to be chosen as proposal work when
  current specification behavior does not already cover them.
- `../proposals/agent-language-spec-wall/open-questions.md` now routes the
  resolved first-slice questions to source-decision records and current
  language specification pages.
- Current implemented behavior routes through `../specification/`, with
  short topic pages for source surface, names and effects, contracts, holes,
  commands, JSON output, execution, and editor support.
- The remaining design-wall pages are explicitly exploratory proposal work.
- `../proposals/agent-language-spec-wall/repair-command.md` keeps the dedicated
  repair command, final candidate schema, multi-file edit representation,
  ranking model, and confirmation protocol as proposed command-level work.
- `../specification/commands.md` and
  `../specification/commands-full.md` state the implemented command
  boundary: no `veln repair` command exists, and repair edits appear only as
  advisory `check --json` diagnostic details.
- `../specification/holes.md` and
  `../specification/diagnostics-json.md` now route the implemented
  advisory candidate record: concrete edits stay unapplied, candidates include
  target, edit summary, evidence, known limits, blocking obligations,
  verification hint, and `application_status: "unapplied"`, and
  `safe_repair_candidate` does not authorize automatic application.
- The implementation emits the required repair-candidate evidence for ranked
  typed-hole symbol candidates, including type evidence, ranking evidence,
  unrun verification evidence, satisfy evidence when applicable, and explicit
  verification blocking obligations.
- CLI parser coverage confirms both `veln repair` and `veln help repair` fail
  before project discovery with `unknown command`.

## Residual Scope

The design-wall directory still holds broad thesis, historical question
inventory, and future repair-command work. That is intentional proposal
history, not incomplete selected work for the current target. Command details
stay bounded by `../proposals/agent-language-spec-wall/repair-command.md`;
they do not change current behavior until implemented and promoted into the
language specification.

The prelude complexity decision is an intentional non-promise in the current
reference. `../specification/names-effects.md` documents value semantics,
source-order traversal, `Result` short-circuiting, and the absence of
asymptotic complexity guarantees, so the `proposed` status on that
record is not a blocking implementation target.

## Verification

Current repair-boundary review:

- `cargo fmt --check`
- `cargo test -p veln-cli cli::tests::help_parser_reports_unknown_topics`
- `cargo test -p veln-sema`
- `cargo test -p veln-cli --test check_json`

Earlier selected-target review evidence:

- `rg` over `../proposals/` and `../reference/source-decisions/` for proposal
  status labels.
- `cargo test -p veln-test doctest`
- `cargo test -p veln-cli --test check_json negative_doctest`
- `cargo test -p veln-cli --test toolchain_harness`
