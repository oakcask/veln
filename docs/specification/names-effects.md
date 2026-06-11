# Names And Effects

This is the routing page for implemented name resolution, effect checking, and
compiler-known calls.

## Read First

- Namespaces, shadowing, duplicate checks, module ownership, external package
  imports, and manifest export checks:
  [names-effects-full.md](names-effects-full.md#name-resolution).
- Declaration effect spelling, effect labels, and effect inference:
  [names-effects-full.md](names-effects-full.md#effect-labels) and
  [names-effects-full.md](names-effects-full.md#concurrency-calls).
- Compiler-known calls:
  [stdio](names-effects-full.md#stdio-calls),
  [file-system](names-effects-full.md#file-system-calls),
  [process](names-effects-full.md#process-calls), and
  [concurrency](names-effects-full.md#concurrency-calls).
- Prelude helper signatures, value semantics, source-backed helper set, and
  descriptor-only helper boundary:
  [names-effects-full.md](names-effects-full.md#prelude-helpers).
- Pure byte vocabulary helpers for `Byte`, `ByteChunk`, `ByteCount`,
  `ByteOffset`, and compact hex fixture decoding:
  [names-effects-full.md](names-effects-full.md#helper-signatures).
- Descriptor-backed standard symbols, source metadata, and the
  compiler-support source-loading trial:
  [names-effects-full.md](names-effects-full.md#compiler-known-descriptor-table).

## Fast Routes

- Confirming source-backed versus descriptor-only status before proposal work:
  [names-effects-full.md](names-effects-full.md#source-backed-boundary).
- Checking self-hosting migration completion before new proposal work:
  [names-effects-full.md](names-effects-full.md#source-backed-boundary).
  The migration is complete when the descriptor-only pure-helper list is empty
  and all compiler-known pure helpers in that split are source-backed.
  Completion history:
  [../reference/implemented-proposals/self-hosting-standard-library.md](../reference/implemented-proposals/self-hosting-standard-library.md).
- Checking helper signatures before changing the prelude adapter:
  [names-effects-full.md](names-effects-full.md#helper-signatures).
- Checking standard symbol descriptor metadata:
  [names-effects-full.md](names-effects-full.md#compiler-known-descriptor-table).

## Read When

- Updating `name.*`, `module.*`, or `effect.*` diagnostics.
- Changing compiler-known calls, reachability, prelude helpers, or effect
  inference.
- Deciding whether a behavior belongs in the implemented reference or remains
  proposal rationale.

## Skip Unless Needed

- Do not open source-decision history before the implemented behavior in
  [names-effects-full.md](names-effects-full.md) answers the question.
- Use [diagnostics-json.md](diagnostics-json.md) only when the machine-readable
  shape of a diagnostic also changes.
