# HTTP/2 Retirement Evidence Completion

Status: proposed

Current implemented HTTP/2 behavior starts in
[`../specification/http2.md`](../specification/http2.md). This proposal covers
only the remaining evidence work required to justify retirement of the broad
`http2-protocol-core` fixture.

## Goal

Make every historical helper invocation, exact stdout line, and output table
mechanically traceable to an equivalent execution through the public
`std::http2::core`, `std::http2::frame`, or `std::http2::hpack` boundary.
The retirement gate must reject evidence that merely reaches the same protocol
domain while omitting the historical endpoint role, starting state, concrete
input, result projection, diagnostic precedence, emitted bytes, or failure
atomicity.

## Implemented Foundation

The reusable connection, stream, HPACK, continuation, flow-control, shutdown,
and output-buffer behavior is already standard-owned. Production header-block
receive and send paths use the public HPACK codec. The broad fixture source and
case manifest have been removed, while its immutable inventory is retained in
[`retirement-evidence.tsv`](../../examples/specification/run/http2-protocol-core/retirement-evidence.tsv).

The current checker already:

- reconstructs 652 helper invocations, 2,044 stdout lines, and 315 output
  tables from the pinned pre-retirement revision;
- rejects missing, duplicate, unexpected, modified, or independently rebound
  inventory rows;
- verifies complete historical values and retained assertion-body hashes;
- requires references to checked cases or public HTTP/2 standard-package
  boundaries;
- rejects several known wrong-domain, success-for-failure, grouped-output,
  nested-DATA, and generic HPACK substitutions.

These checks preserve inventory integrity, but they do not yet prove semantic
equivalence for each retained row.

## Remaining Gaps

### Helper and stdout relevance

Relevance is still inferred mainly from protocol-domain tokens in a retained
test body. This permits many rows with different arguments and projections to
share evidence that checks only one representative transition. In particular,
132 helper rows bind to one outbound request-headers test, 137 helper rows bind
to one header-block decoder test, and 254 result stdout rows bind to one
receive-headers test.

The checker does not generally require the historical endpoint role, concrete
arguments, before-and-after values, diagnostic precedence, or complete result
projection to be present in the selected branch.

### Non-empty output provenance

The output manifest has one exact call for every historical table, but 273 of
the 275 non-empty rows supply their retained bytes to a shared validator. Frame
rows decode and rebuild their supplied payload, and HPACK rows decode and
re-encode supplied representations, without obtaining the original output from
the corresponding production send or receive transition.

Expected bytes therefore participate in constructing the observed value for
most rows. Exact reconstruction is useful codec coverage, but is not evidence
that the owning production transition emits those bytes.

### Empty output starting state

Several empty-output families use synthetic transitions with substituted
padding, frame-size limits, flow-control credits, SETTINGS sequences, GOAWAY
origins, payloads, header blocks, or content-length counts. Some grouped
SETTINGS rows omit valid items surrounding the failing item. Singleton
WINDOW_UPDATE rows share a generic draining state instead of preserving their
historical stream and GOAWAY boundaries.

### Documentation authority

The current specification and retired-route documentation describe the
retirement evidence more strongly than the executable checks support. The
fixture remains physically retired, but the semantic retirement gate is open
until the requirements below pass.

## Evidence Model

Keep the existing TSV as the immutable historical inventory. Add a structured
scenario manifest that describes how retained behavior is reproduced. Each
inventory row must name both a scenario and a projection within that scenario.

A scenario records:

- endpoint role;
- public constructors and ordered setup transitions;
- complete relevant connection, stream, HPACK, continuation, flow-control,
  SETTINGS, shutdown, and output starting state;
- the public operation under test and its concrete input;
- the expected accepted or rejected branch;
- all result fields relevant to the historical row;
- emitted bytes and output-buffer state;
- required unchanged projections after failure.

A projection records the exact historical fact supported by one scenario
result. Multiple historical rows may share a scenario when they are distinct
projections of the same execution, but every row must have its own checked
projection. Sharing a scenario must not allow one generic assertion to stand in
for different historical values.

Expected bytes and expected result fields are comparison-only data. The
generated execution must obtain observed bytes and state exclusively from the
production operation. The checker must reject a scenario that passes expected
output into the operation, a frame or HPACK reconstruction helper, or another
path used to derive the observed output.

## Generated Executable Evidence

Generate row-addressable standard-package tests from the structured scenario
manifest. Generated tests must:

1. construct initial state through the declared public setup sequence;
2. assert the complete declared starting projections;
3. invoke the owning production transition once;
4. select the expected branch without fallback acceptance;
5. compare every declared result and diagnostic projection;
6. compare production-emitted bytes with the retained expected bytes;
7. compare all required post-state projections, including unchanged state and
   output after rejection.

The generated source is checked in so reviewers can inspect the executable
assertions. A regeneration check fails when the manifest and generated source
differ. The generator must produce stable ordering and deterministic names
derived from inventory keys.

Focused hand-written tests remain useful specification evidence, but their
source text is not sufficient retirement evidence unless a structured scenario
executes and projects the retained row.

## Checker Requirements

Extend `scripts/check-http2-retirement-evidence` to enforce all of the
following:

- every historical row maps to an existing scenario and unique projection;
- every scenario uses a public production boundary appropriate for its
  historical owner;
- endpoint role, setup sequence, operation input, branch, diagnostic identity,
  result fields, output, and post-state requirements are explicit rather than
  inferred from broad source tokens;
- expected output is comparison-only and cannot be used to construct observed
  output;
- failed operations compare the complete declared immutable state and output;
- diagnostic-precedence scenarios contain all competing invalid conditions and
  compare the selected failure id and provenance;
- batch inputs retain their complete ordered item sequence;
- receive-versus-send and peer-versus-local GOAWAY origins are reproduced by
  setup transitions rather than direct substitution of the final lifecycle;
- no scenario or projection is unused, and no inventory row is unclassified;
- generated executable evidence is current and passes independently.

The checker should emit a coverage report grouped by transition family. The
report distinguishes inventory binding, structured scenario coverage,
production-derived output, failure atomicity, and diagnostic precedence. A
single aggregate covered count must not hide missing dimensions.

## Mutation Self-test

Expand the checker self-test with mutations that remove or substitute one
required dimension at a time. At minimum it must reject:

- the wrong endpoint role or GOAWAY origin;
- a direct final-state constructor replacing a required setup transition;
- changed stream ids, credits, limits, padding, payloads, or SETTINGS items;
- an accepted transition substituted for a rejected transition;
- the wrong diagnostic id or precedence winner;
- an omitted before-or-after state projection;
- expected bytes reused as observed bytes;
- a codec-only reconstruction substituted for the owning send transition;
- multiple retained rows collapsed into one generic projection;
- a failed HPACK representation-family check without the retained transition
  and complete caller-owned table projection.

Mutation coverage is required for every scenario family, not for every
individual inventory row. Schema validation and generated row projections then
apply those dimensions uniformly to all rows.

## Migration Order

1. Define and validate the structured scenario schema without weakening the
   existing inventory and hash checks.
2. Add generation and regeneration checks for row-addressable executable
   evidence.
3. Migrate helper and stdout rows by transition family, beginning with the
   heavily shared HEADERS and HPACK references.
4. Replace non-empty output reconstruction with owning production sends and
   receives.
5. Reconstruct empty-output setup sequences and complete failure-state
   projections for DATA, WINDOW_UPDATE, SETTINGS, HEADERS, PUSH_PROMISE,
   PRIORITY, GOAWAY, and content-length families.
6. Run the focused, standard-package, loader, performance, and workspace gates.
7. Align the specification and retired-route documentation with the completed
   executable evidence, then move this proposal back to implemented history.

## Completion Criteria

The proposal is complete only when:

- all 3,011 historical rows have structured scenario and projection coverage;
- every non-empty output row obtains observed bytes from its owning production
  transition;
- every rejected row compares its exact failure branch and all relevant
  unchanged state and output;
- all setup-sensitive rows reproduce their historical ordered transitions and
  concrete inputs;
- the mutation self-test demonstrates rejection of every required evidence
  dimension for each scenario family;
- the generated evidence passes independently, with zero missing, unused, or
  unclassified rows;
- current specification prose makes no claim stronger than executable evidence;
- guarded standard-package, focused protocol, loader, performance, and
  workspace verification pass without relaxed limits or material regression.

## Non-goals

- Reintroducing the retired broad fixture as executable production evidence.
- Adding another HPACK representation or sequence-extension codec boundary.
- Requiring one production execution for every stdout line when several lines
  are genuine projections of the same scenario.
- Treating source-text keywords, assertion-body hashes, or aggregate row counts
  alone as proof of semantic equivalence.
