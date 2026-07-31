# Veln Design Notes

This directory routes durable design notes for the experimental Veln
implementation. Start here, open one route, and avoid full records until a
short page points to them. Use [navigation.md](navigation.md) only when the
first route is not obvious.

## Read First

- Current language behavior:
  [specification/README.md](specification/README.md).
- Planned or accepted proposal work:
  [proposals/README.md](proposals/README.md).
- Rationale, source support, and implemented proposal records:
  [reference/README.md](reference/README.md).

## Choose One Task

- Change implemented language behavior:
  [specification/topic-map.md](specification/topic-map.md).
- Promote proposal work into implemented behavior:
  [proposals/README.md](proposals/README.md), then the matching
  specification page.
- Update diagnostics, related notes, or command JSON behavior:
  [specification/diagnostics-json.md](specification/diagnostics-json.md)
  or [specification/json-output.md](specification/json-output.md).
- Check rationale behind current behavior:
  [specification/source-decisions.md](specification/source-decisions.md).
- Check source support or documentation maintenance routes:
  [navigation.md](navigation.md).

## Behavior Specification Rule

When a document specifies behavior, use the most directly verifiable practical
medium regardless of whether the document lives under `specification/`,
`proposals/`, or `reference/`. Prefer executable tests, doctests, checked
fixtures, executable specifications, measurable benchmarks, or structured
decision and state-transition tables. Use diagrams as derived or supporting
views when possible. Keep prose for routing, rationale, scope, and constraints
that cannot reasonably be expressed mechanically.

Specify behavior declaratively as an externally observable contract. Do not
prescribe internal algorithms, data structures, or operation order unless they
are required design constraints. When an internal algorithm or ordered
procedure needs prose explanation, use Simplified Technical English style:
write short sentences, put one action or condition in each sentence, and use
consistent terminology.

Planned behavior must identify its acceptance model and intended verification
without presenting not-yet-running evidence as implemented. Current behavior
must route to checked evidence when practical.

## Stop Rule

- Stop at the first short page that answers the task.
- Open `*-full.md` files and `result-*.md` records only when a short route
  names the relevant detail.
- Return here instead of scanning sibling directories when the route turns out
  to be proposal or reference work.

## Directory Map

- `specification/`: current implemented language behavior, kept as the latest
  specification only.
- `reference/`: durable rationale, source support, and completed proposal
  records.
- `proposals/`: planned or accepted targets not fully implemented.

## Skip Unless Needed

- Use the directory README files for status and placement routes instead of
  repeating those rules here.
- Do not read implemented proposal records before the current specification page
  and [reference/implemented-proposals/README.md](reference/implemented-proposals/README.md)
  route.
