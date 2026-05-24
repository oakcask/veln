# First-Slice Implementation Memo

Date: 2026-05-24

This memo turns the current first-slice design decisions into an implementation
shape. It is a working plan, not a replacement for the decision records under
`docs/discussions/`.

## Goal

The first slice should deliver one small standard edit loop:

```text
veln fmt
veln check --json
veln run <entry>
veln test
```

The important boundary is that these commands share one project context and one
analysis pipeline. `check` is the primary agent repair-loop command. `run` and
`test` should reuse the same checked facts instead of inventing command-specific
parsing, discovery, or diagnostics.

## Architecture

```text
veln CLI
  -> project context and source discovery
  -> lexer and lossless parse tree
  -> surface AST
  -> semantic analysis tables
     - name_facts
     - type_facts
     - effect_facts
     - contract_facts
     - hole_facts
     - boundary_facts
  -> checked core
  -> typed IR
  -> JVM backend
     - Java source generation first
     - small JVM runtime library
```

Use Rust for the CLI, parser, formatter, checker, checked core, typed IR, and
initial JVM lowering. The JVM runtime support library may be Java or Kotlin, but
do not split out a separate Kotlin compiler backend before the first
`check`/`fmt`/`run`/`test` loop works.

The typed IR is the main boundary between target-independent Veln semantics and
target-specific execution. It must not expose JVM class names, descriptors,
stack locals, object identity, or runtime-library layout as language facts.

## Rust Module Shape

Start with a compact crate layout and keep the internal boundaries explicit:

```text
crates/
  veln-cli/          # commands, output modes, exit behavior
  veln-source/       # source files, spans, line indexes, relative paths
  veln-syntax/       # lexer, parser, lossless tree, formatting input
  veln-ast/          # arena-backed surface AST and NodeId handles
  veln-project/      # source discovery, module context, import roots
  veln-sema/         # name, type, effect, contract, hole analysis
  veln-core/         # checked core
  veln-ir/           # runtime-neutral typed IR
  veln-backend-jvm/  # typed IR to Java source
  veln-diagnostics/  # stable envelopes and detail payloads
  veln-test/         # test discovery, test JSON, captured events
```

If this is too much for the initial repository scaffold, keep fewer crates but
preserve these as Rust modules. The implementation risk is not the number of
crates; it is letting command behavior and backend details leak across phase
boundaries.

## Phase Responsibilities

- `fmt` uses the lossless parse tree and may consult the surface AST when the
  file recovers enough structure. It must not require type, contract, or effect
  analysis.
- `check` builds the surface AST when possible, then populates phase-specific
  analysis tables and emits stable-envelope diagnostics.
- `run` and `test` require checked core for the selected entry point or case.
  They are blocked by parse errors, validation errors, type errors, missing
  required public effects, failed required static contracts, or reachable
  holes.
- Runtime contract checks are lowered from validated `contract_facts`, not
  reparsed from source text.
- Every source-backed node that can appear in diagnostics has a session-stable
  `NodeId` and span.

## Standard Library Boundary

Split the standard library into frontend-known signatures and runtime/backend
implementations.

The frontend owns:

- primitive types: `Bool`, `Int`, `Float`, `String`, and `Unit`
- built-in parametric forms: `Option(T)`, `Result(T, E)`, `List(T)`, and
  `Dict(K, V)`
- record and function type modeling
- prelude function signatures
- purity and effect metadata
- the contract-call allowlist

The runtime or backend owns:

- concrete list and dictionary representation
- `Option` and `Result` value constructors
- stdio operation handlers
- contract failure objects
- captured output event production

First-slice prelude helpers:

```text
list_len
list_is_empty
list_push
list_concat
list_map
list_filter
list_fold
list_try_map

dict_get
dict_contains
dict_insert
dict_remove

option_map
option_and_then
option_unwrap_or

result_map
result_map_err
result_and_then
```

Treat prelude helper names as ordinary functions with compiler-known metadata.
They may lower to runtime calls or intrinsics, but the observable contract is
their name, type shape, value semantics, effect metadata, and diagnostic
behavior.

## Stdio Boundary

`stdio` is a built-in module, not a special statement form. The first
implementation provides exactly these output functions:

```veln
pub fn stdio::print(text: String) -> Unit effects [stdio]
pub fn stdio::println(text: String) -> Unit effects [stdio]
pub fn stdio::eprint(text: String) -> Unit effects [stdio]
pub fn stdio::eprintln(text: String) -> Unit effects [stdio]
```

Represent output internally as effect operations routed through implementation
owned handlers. `run` installs the process stdout/stderr handler. `test`
installs a capture handler that produces deterministic source-linked events.

## Typed IR

Keep the IR runtime-neutral and source-linked:

```text
ProgramIr
  modules
  functions
  builtin_refs

FunctionIr
  symbol
  params
  return_type
  effects
  body
  source_node_id

IrExpr
  literal
  local
  let
  call
  builtin_call
  record
  field
  list
  match
  result_propagate
  contract_check
```

`stdio::println` should be an IR builtin or effect operation, not a JVM-specific
method call. The JVM backend decides how to map it to the runtime library.

## First Vertical Slice

Start with the smallest executable program that exercises public effects,
built-in calls, `Result`, and JVM execution:

```veln
pub fn main() -> Result(Unit, AppError) effects [stdio]
  stdio::println("hello")
  Ok(())
end
```

The first implementation milestone is:

```text
veln fmt
veln check --json
veln run main
```

After that, add holes and fallible traversal to test the agent repair loop:

```veln
pub fn summarize(lines: List(String)) -> Result(String, ParseError) effects []
  let items = list_try_map(lines, parse_line)
  _
end
```

At this point `check --json` should report the hole expected type, local
bindings, and candidate-query hints.

## Implementation Order

1. Build `veln check --json` through lexer, parser, surface AST, `NodeId`,
   parse diagnostics, and JSON envelope.
2. Add public function boundary checking for `pub fn`, explicit return types,
   and explicit `effects [...]`.
3. Add the minimal type system: primitives, records, lists, `Option`, `Result`,
   local monomorphic inference, and hole expected-type flow.
4. Add name, effect, contract, and hole diagnostics, including missing public
   effect provenance.
5. Add `veln fmt` for the first-slice grammar.
6. Lower checked programs to checked core and typed IR.
7. Add the JVM backend with Java source generation and a small runtime library.
8. Add `veln run` with entry-point resolution and reachable-hole blocking.
9. Add `veln test` with shared static gates, explicit targets, `*_test.veln`
   discovery, same-file examples, and captured stdio events.

## Related Decisions

- [First Implementation Architecture](../discussions/2026-05-24-agent-language-spec-wall/result-first-implementation-architecture.md)
- [First Implementation Commands](../discussions/2026-05-24-agent-language-spec-wall/result-first-implementation-commands.md)
- [First-Slice Grammar](../discussions/2026-05-24-agent-language-spec-wall/result-first-slice-grammar.md)
- [AST Phase Boundary](../discussions/2026-05-24-agent-language-spec-wall/result-ast-phase-boundary.md)
- [AST Implementation Representation](../discussions/2026-05-24-agent-language-spec-wall/result-ast-implementation-representation.md)
- [Minimum Type System for Holes](../discussions/2026-05-24-agent-language-spec-wall/result-minimum-type-system-for-holes.md)
- [First-Slice Prelude Helpers](../discussions/2026-05-24-agent-language-spec-wall/result-first-slice-prelude-helpers.md)
- [Stdio API and Output Events](../discussions/2026-05-24-agent-language-spec-wall/result-stdio-api-and-output-events.md)
