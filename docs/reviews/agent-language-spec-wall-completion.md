# Agent-Language Spec Wall Completion Review

Status: complete for accepted implementation targets and the current advisory
repair candidate boundary.

This review covers the no-target route after
[../proposals/target-queue.md](../proposals/target-queue.md) and
[../proposals/first-slice-follow-ups.md](../proposals/first-slice-follow-ups.md)
reported no accepted targets. The historical implementation target was the
repair-loop portion of the open design-wall material under
[../proposals/agent-language-spec-wall/README.md](../proposals/agent-language-spec-wall/README.md).
The completed implementation target is the advisory, machine-readable repair
candidate boundary before a dedicated repair command exists.

## Completion Check

- `../proposals/target-queue.md` reports no accepted targets.
- `../proposals/first-slice-follow-ups.md` reports no accepted first-slice
  follow-up targets.
- `../proposals/agent-language-spec-wall/open-questions.md` now routes the
  resolved first-slice questions to source-decision records and current
  language reference pages.
- Current implemented behavior routes through `../reference/language/`, with
  short topic pages for source surface, names and effects, contracts, holes,
  commands, JSON output, execution, and editor support.
- The remaining design-wall pages are explicitly exploratory. New work from
  those pages must first become a selected proposal target before changing
  current behavior.
- `../proposals/agent-language-spec-wall/repair-command.md` keeps the dedicated
  repair command, final candidate schema, multi-file edit representation,
  ranking model, and confirmation protocol as open command-level work.
- `../reference/language/commands.md` and
  `../reference/language/commands-full.md` state the implemented command
  boundary: no `veln repair` command exists, and repair edits appear only as
  advisory `check --json` diagnostic details.
- `../reference/language/holes.md` and
  `../reference/language/diagnostics-json.md` now route the implemented
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
history, not incomplete accepted work for the current target. Open command
details stay bounded by `../proposals/agent-language-spec-wall/repair-command.md`;
they do not reopen the target queue unless promoted as a new accepted target.

The prelude complexity decision is an intentional non-promise in the current
reference. `../reference/language/names-effects.md` documents value semantics,
source-order traversal, `Result` short-circuiting, and the absence of
asymptotic complexity guarantees, so the `accepted-proposal` status on that
record is not a blocking implementation target.

## Verification

Current repair-boundary review:

- `cargo fmt --check`
- `cargo test -p veln-cli cli::tests::help_parser_reports_unknown_topics`
- `cargo test -p veln-sema`
- `cargo test -p veln-cli --test check_json`

Earlier accepted-target review evidence:

- `rg` over `../proposals/` and `../reference/source-decisions/` for accepted
  and open status labels.
- `cargo test -p veln-test doctest`
- `cargo test -p veln-cli --test check_json negative_doctest`
- `cargo test -p veln-cli --test toolchain_harness`
