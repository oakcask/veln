# Run JSON

`veln run --json` emits schema version `veln-run-json/v0` with:

- `command`: `run`
- `status`: `passed`, `failed`, or `error`
- `exit_code`: the captured Java process exit code, or `1` for tool errors
- `stdout`: captured user-program stdout
- `stderr`: captured user-program stderr
- `error`: `null` for passed runs, or a structured result, runtime, or runner
  error

Runtime contract failures use `error.kind: "contract"`. The error details use
`kind: "contract"` and `phase: "runtime"` and include:

- `clause`: `require`, `ensure`, or `invariant`
- `predicate`: the failed clause text
- `function`: the checked function boundary
- `blame`: `caller` for `require`, `implementation` for `ensure`, and either
  value for `invariant` depending on entry or return failure
- `node_id`: the contract node identifier
- `span`: the source span for the failed clause

An entry returning `Err(value)` uses `error.kind: "result"`. The error details
use `kind: "result"`, `phase: "runtime"`, and `value` with the rendered error
value. When the result value is a compact fixture hex failure from
`byte_chunk_from_hex(text)`, `details.fixture_hex` includes:

- `kind: "fixture_hex"`
- `id`: `fixture.hex.invalid_character` or `fixture.hex.odd_length`
- `fixture_text_span`: start and end offsets inside the fixture text
- `byte_offset`: the decoded `ByteOffset` kind and integer value
- `nibble_position`: `high` or `low`
- `nearby_context`: bounded fixture text around the failed span

When the result value is a closed-input fixed-width `ByteView` read
truncation, `details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "codec.incomplete_input"`
- `byte_offset`: the first missing decoded-stream `ByteOffset`
- `field_path`: schema-local path segment objects with `kind` and `name`;
  empty when no schema owns the read
- `expected_count`: the required byte count
- `available_count`: the byte count available in the view
- `readiness: "need_bytes"`

Other non-zero Java process exits use `error.kind: "runtime"` with
`details.phase: "runtime"`. JDK setup failures use `error.kind: "runner"` with
`details.phase: "tool"`.
