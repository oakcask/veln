# Run JSON

`veln run --json` emits schema version `veln-run-json/v0` with:

- `command`: `run`
- `status`: `passed`, `failed`, or `error`
- `exit_code`: the captured Java process exit code, or `1` for tool errors
- `stdout`: captured user-program stdout
- `stderr`: captured user-program stderr
- `error`: `null` for passed runs, or a structured runtime or runner error

Runtime contract failures use `error.kind: "contract"`. The error details use
`kind: "contract"` and `phase: "runtime"` and include:

- `clause`: `require`, `ensure`, or `invariant`
- `predicate`: the failed clause text
- `function`: the checked function boundary
- `blame`: `caller` for `require`, `implementation` for `ensure`, and either
  value for `invariant` depending on entry or return failure
- `node_id`: the contract node identifier
- `span`: the source span for the failed clause

Other non-zero Java process exits use `error.kind: "runtime"` with
`details.phase: "runtime"`. JDK setup failures use `error.kind: "runner"` with
`details.phase: "tool"`.
