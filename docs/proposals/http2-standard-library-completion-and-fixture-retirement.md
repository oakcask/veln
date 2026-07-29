# HTTP/2 Standard Library Completion and Fixture Retirement

Status: proposed

Current HTTP/2 core and HPACK behavior is implemented under
[`../specification/http2.md`](../specification/http2.md). This proposal tracks
only the remaining semantic evidence needed before the old
`http2-protocol-core` fixture route can be treated as fully retired.

## Completion Gate

The target is complete only when each historical helper invocation, exact
stdout line, and `output_chunk_list` row has row-specific executable evidence
that observes the owning public transition. The evidence must preserve the
endpoint role, starting state, concrete setup and input, selected branch,
result projection, emitted bytes, diagnostic precedence, post-state, and
failure atomicity for that row.

Production receive and send evidence must cross the public `std::http2::hpack`
codec and must not use fixture fallback decoding, canned HPACK encoding, or
`hpack.fixture.unsupported_header_block` compatibility.

## Current Gap

`scripts/check-http2-retirement-evidence` verifies the historical inventory,
artifact freshness, output byte preservation, and generated projection
consistency. It does not yet prove that every generated row invokes its owning
public transition with that row's concrete setup and input. Multiple
historical rows can still share one focused evidence target or assertion body.

Run `scripts/check-http2-retirement-evidence --semantic-gap-report` to inspect
that sharing. The report lists reused evidence targets and assertion bodies
with counts of distinct operations, roles, branches, starting states, and
inputs. A completed gate must drive those shared groups to row-specific
semantic evidence or justify a mechanically checked equivalence class that
contains the concrete row projections.

## Non-Goals

Do not restore reusable behavior to the retired broad fixture. Current
behavior remains owned by `std::http2::core`, `std::http2::hpack`, focused
standard-package tests, and executable specification cases.
