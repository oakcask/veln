---
role: proposal
update-when: The proposed net::write_all cumulative progress result, retry boundaries, invalid-reply behavior, or transport progress scope changes.
---

# Preserve Progress in Network Write-all Results

Return cumulative committed bytes from `net::write_all`, including on failure.
This is a Ready proposal independent of generic functions. A dedicated
`net::WriteAllOutcome` distinguishes exact failed progress from provider replies
whose total side effects cannot be established.

## Current boundary and purpose

[net::write](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/net.veln) returns either a count or a
failed committed prefix. `write_all` retries successful partial writes but
returns `Result<(), NetError>` and discards the prefix when a write fails.
It also does not retain the sum of earlier successful attempts. In contrast,
[io::write_all](https://github.com/oakcask/veln/blob/7b4b32ed68d8309abbdc0512f0f92caae80c3219/crates/veln-stdlib/veln/io.veln) explicitly retains total
progress. A caller that needs to account for bytes already sent cannot recover
that information from the network helper.

Progress is local commitment to the transport, not proof that a peer received,
processed, or durably stored data. A returned prefix alone does not make
reconnection or application-level retries safe.

## Proposed API and accumulation

```veln
net::write_all(stream: Stream, bytes: ByteChunk, deadline: Option<Deadline>, token: Option<CancelToken>) -> WriteAllOutcome effects [net::IO]
```

Keep the existing deadline/token arguments in this proposal. Optional-argument
naming symmetry is a separate concern. Add these public outcome cases:

- `Complete(count: ByteCount)` reports the full original input length.
- `Failed(committed: ByteCount, error: NetError)` reports exact cumulative progress.
- `InvalidReply(known_prefix: ByteCount, error: NetError)` reports only the
  validated lower bound when a provider reply cannot establish total progress.

All counts refer to the original input, not just the last low-level write.
The low-level `net::write` and its `WriteOutcome` remain unchanged.

| Event, after a known committed prefix `p` | Required result or transition |
| --- | --- |
| Empty original input | `Complete(0)` without invoking the provider. |
| Successful partial write of `n > 0` bytes | Advance to `p + n`; submit only the remaining suffix. |
| All original bytes committed | `Complete(original_length)`. |
| Provider fails after committing `n` more bytes | `Failed(p + n, original_error)`; no further write. |
| Provider reports zero successful progress with data remaining | `Failed(p, progress_error)`. |
| Cancellation or deadline failure with a valid prefix | Preserve cumulative prefix and the original error classification. |
| Negative or oversized provider count | Return `InvalidReply(p, invalid_provider_error)`; do not use the invalid count to slice or retry. |

For an invalid provider reply, the result case marks the prefix as a verified
lower bound. Callers do not parse an error message to discover that uncertainty.
The result must not imply that the provider made no additional side effects.
This proposal does not introduce exact progress information that the provider
never supplied.
Count accumulation is bounded by the original input length and must not wrap.
The helper preserves the same deadline and token across all attempts.

## Related work and alternatives

The inspected [Rust Write::write_all](https://doc.rust-lang.org/std/io/trait.Write.html#method.write_all)
returns only unit or an error and stops after failure. It is a valid simpler
convention; loss of progress is not universally an API defect. Veln's design
choice differs because the low-level network operation already reports failed
prefixes and its shared IO helper retains cumulative progress. Keeping those
facts available avoids making the higher-level helper less informative.

A new fallible wrapper returning unit would repeat the current information
loss. Reusing only the exact-count `WriteOutcome` would conflate a known prefix
with known total progress. A separate three-case outcome preserves this
distinction without a new generic progress framework or a dependency cycle
between the network and shared IO modules.

## Planned acceptance and migration

| Case | Required observation | Intended verification |
| --- | --- | --- |
| Input length six, writes of two then four | Submissions are six then four bytes; result is `Complete(6)`. | Stdlib net companions with scripted IO handler |
| Two committed, then failure after one more | `Failed(3, original_error)`; remaining bytes are never submitted. | Stdlib companions |
| Zero progress after a prefix | Failure retains the prefix; no retry loop. | Stdlib companions |
| Empty input or malformed provider count | No provider call for empty input; invalid counts neither cause slicing nor invent verified progress. | Stdlib companions |
| Deadline/token across retries | Same controls reach every attempt; failure preserves cumulative count. | Scoped Veln handler support |
| Real host write | Count/error translation preserves the host's known prefix. | Bounded backend host-integration test |

These are planned tests. Migrate transport adapters and HTTP/2 consumers from
`Ok(())` to `WriteAllOutcome::Complete(total)` and from `Err` to the two
failure cases. Add the qualified case names in examples where existing
`WriteOutcome` names could be ambiguous. The existing
`DuplexStream::write_chunks` unit/error boundary deliberately remains terminal
on failure; document that it cannot support byte-level resumption. Redesigning
multi-chunk transport progress is outside this proposal and must not be
advertised as solved by changing `net::write_all`.

No grammar or formatter rewrite is needed. Call/result migration is explicit;
ordinary formatting cannot choose an error recovery policy. Update the
[current networking specification](../specification/standard-library-networking.md)
and examples when behavior lands. Preserve test placement and real-host effect
rules, then retire this proposal and its catalog entry.
