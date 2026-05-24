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

## Dependency Choices

Keep the first implementation dependency set small. Prefer ordinary Rust data
structures and local code until a crate directly protects a first-slice
boundary: CLI compatibility, source-relative paths, lossless syntax, stable
JSON, temporary backend artifacts, or golden diagnostics.

Use a hand-written lexer and parser for `veln-syntax`. The parser should be a
recursive-descent parser with a small Pratt parser for expressions, and it
should build a lossless syntax tree suitable for parse recovery and `fmt`.
This is a better first fit than a parser generator because Veln needs
source-backed partial programs, targeted recovery around `end`, newline-aware
grouping, contract-specific predicate parsing, and formatter input that
preserves comments and trivia.

Initial crate choices by module:

```text
veln-cli
  clap                 # command parsing for check, fmt, run, and test

veln-source
  camino               # UTF-8 project-relative paths
  text-size            # text offsets and ranges

veln-syntax
  rowan                # lossless syntax tree
  text-size            # shared offsets and spans

veln-ast
  none initially       # use Vec-backed arenas and typed NodeId newtypes

veln-project
  none initially       # use std::fs discovery until manifests/source roots exist

veln-sema
  none initially       # use BTreeMap or explicit sorting for deterministic facts
  indexmap later       # add only if insertion-ordered tables become useful

veln-core
  none initially

veln-ir
  serde as dev/test    # debug or snapshot JSON only; no stable external IR yet

veln-backend-jvm
  tempfile             # generated Java source and isolated javac/java work dirs

veln-diagnostics
  serde
  serde_json
  annotate-snippets optional  # human text rendering only, not JSON schema shape

veln-test
  serde
  serde_json
  insta as dev/test    # golden diagnostics and test JSON snapshots
```

Use `thiserror` for structured library error types when hand-written error
enums start to repeat. `anyhow` is acceptable inside the CLI command boundary
for top-level orchestration errors, but it should not cross into compiler
facts, diagnostics, checked core, typed IR, or test JSON records.

Do not start with `lalrpop`, `chumsky`, `logos`, or a JVM bytecode generation
crate.

- `lalrpop` is a maintained parser generator, but build-time generation and
  LR-oriented recovery are a poor fit for the first lossless, repair-oriented
  parser.
- `chumsky` has strong parser-combinator recovery, but its parser shape should
  not own Veln's lossless tree, formatter input, or recovery diagnostics in the
  first slice.
- `logos` is a good lexer generator, but the first lexer needs direct control
  over newline separators, grouping forms, trivia, invalid tokens, and recovery
  anchors.
- JVM bytecode emission should wait until generated Java source proves the IR
  boundary and runtime helper surface.

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

- primitive types: `Bool`, `Int`, `Float`, `String`, and `()`
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
pub fn stdio::print(text: String) -> () effects [stdio]
pub fn stdio::println(text: String) -> () effects [stdio]
pub fn stdio::eprint(text: String) -> () effects [stdio]
pub fn stdio::eprintln(text: String) -> () effects [stdio]
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
pub fn main() -> Result((), AppError) effects [stdio]
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

## Current Implementation Position

Review note: the current code has known gaps against this memo and the
design-wall decisions. Read
`../reviews/2026-05-24-first-slice-gap-review.md` before treating the completion
claims below as the current implementation status.

Status as of 2026-05-24: implementation order items 1, 2, 3, 4, 5, 6, 7, 8,
and 9 are complete for the first-slice gate.

Implemented so far:

- Rust workspace and first-slice crate scaffold.
- `veln check --json` command entry point.
- Source discovery for explicit paths and recursive `.veln` discovery.
- Initial hand-written lexer and parser for modules, imports, functions,
  contracts, effect lists, `let` lines, and tail-expression lines.
- Lossless syntax root that retains whitespace, comments, invalid tokens, and
  recovery trivia for formatter work.
- Structured expression syntax nodes for calls, paths, literals, lists,
  operators, unit, and holes.
- Initial surface AST lowering with deterministic session-local `NodeId`
  allocation for functions, parameters, contracts, body lines, expressions, and
  holes.
- Stable top-level diagnostic JSON envelope for parse diagnostics.
- Structured lossless syntax nodes for module declarations, use declarations,
  function declarations, signatures, contracts, body blocks, let statements,
  and expression lines.
- Golden-style parse diagnostic coverage for required `details` keys, recovery
  strategies, source-relative spans including EOF spans, and deterministic file
  ordering.
- `veln check --json` end-to-end coverage for valid input, parse recovery,
  missing `end`, malformed declarations, and invalid tokens.
- Public function boundary diagnostics for missing public parameter types,
  missing public return types, and missing explicit public `effects [...]`
  clauses, surfaced through the same `veln check --json` envelope.
- Item 2 review follow-up: public boundary diagnostics now use the
  `result-check-json-details-fields.md` vocabulary for the missing public
  effects id and the type constraint enum values.
- Exact `veln check --json` golden coverage for public boundary diagnostic
  spans, including the parameter span and function boundary span.
- Item 3 type-analysis slice: primitive type rendering for `Bool`, `Int`,
  `Float`, `String`, and `()`; parsed `Option(...)`, `Result(...)`,
  `List(...)`, `Dict(...)`, record, and function type forms; annotation
  validation diagnostics; local binding context; declared-return,
  local-annotation, call-argument, record-field, collection-element, and `?`
  propagation expected-type flow; `Ok(...)`, `Err(...)`, and `Some(...)`
  constructor typing in expected contexts; simple non-constructor call
  inference from local function signatures; return/assignment mismatch
  diagnostics; and `hole.unfilled` diagnostics with expected type, visible
  local bindings, candidate queries, and related expected-type origins.
- Hole-only checks now report top-level JSON `status: "partial"` while still
  using `severity: "hint"` for the hole diagnostic.
- Item 4 diagnostics slice: unresolved value and call-target names now report
  `name.unresolved` diagnostics through the stable JSON envelope; direct
  `stdio::print`, `stdio::println`, `stdio::eprint`, and
  `stdio::eprintln` calls infer the `stdio` effect; public functions whose
  declared effects omit an inferred public effect report
  `effect.missing_public` with declared effects, inferred effects, bounded
  call provenance, and related call-site spans; contract clauses validate the
  first-slice pure boolean subset and report contract diagnostics for
  non-boolean predicates, effectful predicates, and unsupported calls; contract
  predicate name-resolution failures route through `name.unresolved` with the
  `contract_predicate` namespace; and hole diagnostics now carry
  contract-derived and `satisfy` repair constraints with constraint-origin
  related entries.
- Human diagnostics should keep the primary message focused on the specific
  fact that failed, and use related notes for cause and provenance. Avoid
  wording that makes a contextual boundary rule sound like an unconditional
  source requirement. For example, a missing public effect should say that the
  public function uses an undeclared effect, then point at the call that
  required the effect.
- Item 5 formatter slice: `veln fmt [path ...]` discovers the same `.veln`
  input set as `check`, parses each file, refuses to write any file when parse
  diagnostics are present, and otherwise writes deterministic first-slice
  formatting from the syntax tree. The formatter normalizes module/use
  declarations, function signatures, public/private functions, contract clause
  indentation and keyword spelling, `let` lines, tail expressions, holes with
  `satisfy`, records, lists, calls, literals, paths, prefix/binary operators,
  and postfix `?`. Files containing comments are currently preserved
  byte-for-byte to avoid destructive comment movement before comment attachment
  is implemented.
- Item 6 lowering slice: `veln-core` now has source-linked checked core nodes
  for functions, params, contracts, let statements, expression statements, tail
  returns, literals, locals, records, lists, calls, `Result`/`Option`
  constructors, `try` propagation, and holes. `veln-ir` now has a
  runtime-neutral typed IR with function calls, stdio builtins, value calls,
  expression statements, returns, constructors, records, lists, and `try`.
  `veln-sema` exposes `lower_checked_surface_module`, which runs the existing
  diagnostics first, returns no core or IR when semantic errors are present,
  returns blocked checked core but no executable IR when reachable holes or
  unsupported executable forms remain, and returns typed IR only for complete
  checked core.
- Item 7 JVM backend slice: `veln-backend-jvm` now generates compile-ready
  Java source artifacts from typed IR without adding JVM class names,
  descriptors, or runtime-layout details to the IR. The backend emits a
  generated program class plus a small runtime class covering boxed first-slice
  values, `Result`, `Option`, records, lists, stdio builtin calls, integer and
  boolean operators, function/value calls, and `?` propagation through early
  `Err` return checks.
- Item 8 run slice: `veln run <entry> [path ...]` now shares source discovery,
  parse diagnostics, semantic diagnostics, checked-core lowering, typed IR,
  and the JVM backend. The command resolves a zero-argument entry function,
  blocks before user code execution on parse errors, semantic errors, missing
  public effects, reachable holes, or other checked-core blockers, writes Java
  artifacts to an isolated temporary build directory, invokes `javac`, then
  invokes `java` on a generated entry wrapper. The wrapper exits non-zero for
  `Err(...)` results, and the CLI forwards generated stdout/stderr. Missing
  JDK tools report clear `javac` or `java` messages.
- Item 9 test slice: `veln test [--json] [target ...]` now shares the same
  parse, semantic, checked-core, typed-IR, JVM backend, and JDK execution gates
  used by `run`. Without explicit targets it discovers selected cases from
  `*_test.veln` files; with explicit targets it treats zero-argument functions
  in the selected files as first-slice test cases, which is the current
  same-file example boundary. Each case runs through an entry wrapper in an
  isolated build directory. `--json` emits the required
  `veln-test-json/v0` run fields, selection metadata, deterministic summary
  counts, diagnostics, suite errors, per-case records, runtime failures, and
  captured stdout/stderr case events.

Item 3 review notes:

- Implementation order item 3 is complete for the first-slice gate. The
  checker now covers the minimum type forms, validates malformed annotations,
  flows expected types into holes through the gate's currently implemented
  expression forms, and reports type mismatches through the stable diagnostic
  envelope.
- The dictionary part of item 3 is complete as type-system support:
  `Dict(K, V)` annotations parse, validate arity, render deterministically,
  and can flow into hole diagnostics. Dictionary literal parsing and value
  construction are not implemented expression forms yet and should not be read
  as part of this item 3 completion claim.
- `hole.unfilled` includes the required details shape: `phase`, `node_id`,
  `label`, `expected_type`, `expected_type_source`, `constraints`,
  `local_bindings`, and `candidate_queries` are present. When an expected type
  is known, the checker emits a structured symbol candidate query, and related
  entries point at the closest available expected-type origin. Candidate
  ranking and concrete repair generation remain outside item 3.
- `status: "partial"` is appropriate for hole-only checks. The current
  top-level priority is also appropriate: parse and type errors keep
  `status: "error"` even if hole diagnostics exist.
- Suppressing semantic diagnostics for a file with parse diagnostics keeps
  parse golden output free of type-analysis noise. Multi-file CLI coverage now
  verifies that parse errors in one file do not hide semantic diagnostics in
  another file. This is sufficient for the current per-file analysis model.

Resolved item 2 review notes:

- The public/private boundary matches the first-slice policy:
  `pub fn` requires explicit parameter types, an explicit return type, and an
  explicit `effects [...]` clause; private helpers may omit these annotations
  without parse or semantic errors.
- Boundary diagnostics use the stable top-level JSON envelope, source-
  relative spans, session-local `node_id` values, and deterministic summary
  counts.
- Missing public effects now uses `id: "effect.missing_public"`, matching the
  details-field decision.
- Public missing-signature type diagnostics no longer use the unlisted
  `constraint: "public_signature"` value; missing parameter types use
  `assignable`, and missing return types use `return_value`.
- Boundary span assertions are exact in the CLI JSON fixtures.

Item 3 completion gate status:

- Complete: primitive coverage includes at least `Bool`, `Int`, `Float`,
  `String`, and `()`, with deterministic rendering in type and hole
  diagnostics.
- Complete: built-in compound type support covers records, homogeneous lists,
  dictionary type forms, function types, `Option(T)`, and `Result(T, E)`.
- Complete: annotated type syntax is validated enough that malformed
  annotations do not silently become `"unknown"` without a diagnostic.
- Complete: local monomorphic inference can use local function signatures for
  simple calls, rather than treating all non-constructor calls as unknown.
- Complete for currently implemented expression forms: expected-type flow
  reaches holes through declared returns, local annotations, call arguments,
  record fields, collection elements, and `?` propagation into compatible
  `Result` returns. Match branch expected-type flow remains later follow-up
  work because match expressions are not implemented yet; contract-derived
  repair constraints are covered by item 4.
- Complete: `hole.unfilled` diagnostics include required details keys, visible
  local bindings, useful candidate-query records when an expected type is
  known, and `related` entries for the closest expected-type origin.
- Complete: `veln check --json` fixtures cover hole-only `partial`,
  parse-error `error`, type-error `error`, mixed error-plus-hole priority, and
  parse-file semantic suppression without hiding diagnostics from other files.

Item 4 completion gate status:

- Complete: unresolved bare names and unresolved call targets produce
  deterministic `name.unresolved` JSON diagnostics with symbol, namespace,
  resolution status, and empty candidate arrays. Contract predicate unresolved
  names use the same diagnostic id with `namespace: "contract_predicate"`.
- Complete: direct stdio calls are recognized as compiler-known effectful
  prelude calls, and public functions with insufficient `effects [...]`
  declarations report missing public `stdio` effects with bounded provenance.
  The details shape matches the required `effect.missing_public` fields in the
  check-JSON decision: `phase`, `node_id`, `effect`, `boundary`,
  `declared_effects`, `inferred_effects`, `provenance`, and
  `provenance_truncated`. The richer transitive-effect path fields
  (`hidden_frame_count`, `omitted_path_count`, and expanded path entries)
  remain follow-up for transitive helper inference; the current direct-stdio
  slice is not blocked by them.
- Complete for the first-slice contract subset: `require` and `ensure`
  predicates are checked for boolean shape, effectful stdio use, unsupported
  calls, and unresolved predicate names. Valid runtime contract discharge is
  still deferred.
- Complete: hole diagnostics include contract-derived repair constraints and
  `satisfy candidate => predicate` constraints, plus related entries pointing
  at constraint origins. The current `satisfy` implementation preserves the
  source suffix in parse and AST and exposes it in `hole.unfilled` constraints;
  stricter source-syntax diagnostics for missing candidate bindings, missing
  `=>`, candidate shadowing, and unused candidate bindings remain follow-up
  before formatter stabilization.

Item 5 completion gate status:

- Complete: `veln fmt [path ...]` is available as a CLI command and uses the
  existing project source discovery path.
- Complete: parse errors block writeback for the whole fmt invocation, and the
  command reports human parse diagnostics to stderr with exit code 1. This
  keeps invalid input from being destructively reformatted.
- Complete for the first-slice grammar currently parsed: golden CLI coverage
  verifies idempotence, public and private function signatures, a `require`
  contract clause, `let` and tail expression lines, `hole satisfy`, records,
  lists, calls, module/use headers, and multi-function spacing.
- Complete for comment safety: comment-bearing files are preserved
  byte-for-byte until formatter comment attachment is implemented.

Item 6 completion gate status:

- Complete: `veln-core` owns the first source-linked checked core
  representation rather than depending on `veln-sema`. Core nodes retain
  session-stable `NodeId` handles and source spans for functions, params,
  contracts, statements, record fields, and expressions.
- Complete: `veln-ir` owns the first runtime-neutral typed IR representation.
  It distinguishes normal functions, stdio builtins, and value calls without
  exposing JVM class names, descriptors, local slots, or runtime-library
  layout.
- Complete for the first-slice executable forms: lowering covers pure
  literals, locals, records, lists, prefix/binary operators, `let`, expression
  statements, tail returns, `Ok`, `Err`, `Some`, stdio builtin calls, ordinary
  local function calls, function-typed value calls, and postfix `?` as typed
  `try` propagation.
- Complete for safety: parse errors are still filtered before sema lowering by
  the command pipeline; semantic error diagnostics block both core and IR;
  reachable holes build blocked checked core but no executable typed IR; and
  constructor/call arity gaps are represented as checked-core blockers rather
  than runnable IR.
- Complete: focused sema tests cover runnable core-to-IR lowering with
  `parse(raw)?`, stdio, and `Ok(())`; hole-only lowering to blocked core with
  no IR; and type-error lowering that produces neither core nor IR.

Item 6 review notes:

- No blocking findings: item 6 is complete for the first-slice gate, and the
  current position is ready for item 7. Checked core is source-linked through
  `NodeId` and spans, while typed IR remains runtime-neutral through
  target-independent types, node ids, statements, expressions, and call-target
  categories.
- The safety boundary is appropriate for the next slice: semantic error
  diagnostics stop both checked core and typed IR; reachable holes, missing
  expressions, constructor arity gaps, and call arity gaps leave checked core
  in `Blocked` readiness; and blocked core is rejected before executable typed
  IR is produced.
- The lowering bridge is sufficient for the JVM backend to start: pure
  expressions, `let`, expression statements, tail returns, `Result` and
  `Option` constructors, stdio builtins, ordinary calls, function-value calls,
  records, lists, and postfix `?` all have target-independent core and IR
  shapes.
- Remaining fixture work is non-blocking for item 7, but should be added before
  claiming broader lowering stabilization: exact tests for function-typed
  value calls, blocked call/constructor arity cases, missing expression
  blockers, and item 8 entry-point reachable-hole handling.

Item 7 completion gate status:

- Complete: `veln-backend-jvm` exposes `generate_java` and
  `generate_java_with_options`, returning deterministic Java source artifacts
  for a generated program class and runtime class.
- Complete: Java source generation consumes only typed IR shapes and keeps
  target-specific choices inside the backend. The typed IR remains
  runtime-neutral and does not contain JVM class names, descriptors, local
  slots, or runtime-library layout.
- Complete for first-slice executable IR: generated Java covers function
  methods, boxed params and locals, returns, expression statements, function
  calls, stdio builtin calls, value calls through a runtime callable interface,
  `Ok`, `Err`, `Some`, records, lists, unit, literals, prefix and binary
  operators, and postfix `?` as an early return when the input result is
  `Err`.
- Complete: the runtime source provides minimal `Result`, `Option`, record,
  list, unit, stdio, callable, operator, and formatting helpers needed by the
  generated first-slice Java.
- Complete: focused backend tests cover result propagation, stdio generation,
  runtime source contents, record/list/option value generation, and `javac`
  compilation when a JDK is available.
- Deferred to item 8: command-line `veln run`, entry-point resolution,
  writing artifacts to a build directory, invoking `javac`/`java` as a user
  workflow, and reachable-hole blocking UX.

Item 7 review notes:

- No blocking findings: item 7 is complete for the first-slice gate, and the
  current position is ready for item 8.
- The typed IR to Java boundary is clean enough for this slice. JVM class
  names, method names, boxed `Object` representation, runtime helper names,
  temporary names, and Java source paths are chosen inside `veln-backend-jvm`
  rather than encoded in `veln-ir`.
- The generated `VelnProgram.java` and `VelnRuntime.java` artifacts are
  compile-ready source artifacts for item 8 to write and compile. They are not
  yet standalone `java VelnProgram` entry artifacts; item 8 must add
  entry-point resolution plus a runner or entry wrapper that calls the selected
  generated function.
- The runtime helpers are sufficient for the first-slice executable IR:
  minimal boxed `Result`, `Option`, records, lists, unit, stdio builtins,
  callable values, operators, formatting, and `?` early `Err` propagation are
  present. Broader runtime semantics such as contract discharge, dictionary
  values, match lowering, richer numeric behavior, and stable callable value
  construction remain later work.
- Focused backend tests are sufficient to proceed to item 8 because they cover
  Java emission shape and compile readiness when a JDK is available. Item 8
  should add end-to-end `run` fixtures for entry-point selection, argument
  handling, reachable-hole blocking, stdio capture/forwarding, and non-zero
  compiler/runtime failure reporting.

Item 8 completion gate status:

- Complete: `veln run <entry> [path ...]` is available and uses the existing
  project source discovery path. Omitting paths recursively discovers `.veln`
  files, matching `check` and `fmt`.
- Complete: static gates run before Java execution. Parse diagnostics,
  semantic error diagnostics including missing public effects, missing entries,
  unsupported parameterized entries, reachable holes, and checked-core blockers
  all stop execution before user code runs.
- Complete: executable typed IR is generated through the existing sema/core/IR
  boundary and passed to the JVM backend. The backend now emits a generated
  `VelnEntry` wrapper for the selected zero-argument entry instead of exposing
  backend naming details through the IR.
- Complete: `veln run` writes generated Java artifacts to an isolated temporary
  build directory, invokes `javac`, invokes `java`, forwards stdout/stderr, and
  preserves the Java process exit code for runtime failures. Missing `javac`
  and missing `java` are reported as explicit JDK setup errors.
- Complete: CLI fixtures cover reachable-hole blocking without requiring a
  JDK, missing entry reporting, missing `javac` reporting, and stdout/stderr
  forwarding when `javac` and `java` are available.
- Deferred beyond item 8: entry arguments, `veln test`, richer runtime
  contract discharge, dictionary values, match lowering, candidate ranking, and
  a persistent build cache. `veln test` is now covered by item 9 below.

Item 8 review notes:

- No blocking findings at item 8 review time: the implementation position was
  item 8 complete and ready to begin item 9.
- Entry resolution is sufficient for the first-slice minimum: `run` selects a
  named zero-argument function from the shared discovered source set and rejects
  missing or parameterized entries before compiling Java artifacts. Entry
  arguments remain explicitly deferred.
- Static gates are sufficient for this slice: parse errors, semantic errors,
  missing public effects, missing or parameterized entries, reachable holes, and
  checked-core/IR blockers stop before generated Java execution. The current
  command checks semantic errors across the discovered module before narrowing
  hole blocking to the entry-reachable module, which is conservative and may be
  relaxed later if selected-entry execution needs to tolerate unrelated broken
  helpers.
- Reachable-hole blocking matches the documented first-slice direct call graph
  scope. The current reachability graph follows selected entry plus direct
  function-name calls in expressions, allowing holes in unreachable functions.
  Broader conservative handling for future higher-order values, module
  initializers, imports, and ambiguous graph edges remains follow-up work.
- JVM execution behavior is reasonable for item 8: generated sources are written
  to an isolated temporary build directory, `javac` runs before `java`, process
  stdout/stderr are forwarded, Java runtime exit status is preserved, and
  missing `javac` or `java` produce explicit setup errors.
- Tests and docs are sufficient to proceed to item 9. Focused coverage includes
  reachable-hole blocking without a JDK, unreachable-hole execution when a JDK
  is available, missing entry errors, missing `javac`, stdout/stderr
  forwarding, entry wrapper generation, backend compilation, and sema/core/IR
  blocking behavior.

Item 5 review notes:

- No blocking findings: the current formatter is sufficient as the standard
  edit-loop formatter for the first-slice grammar because it shares project
  discovery with `check`, parses before writing, formats from structured syntax
  data, and is idempotent on the covered first-slice fixture.
- The parse-error behavior is appropriate for the gate: any parse diagnostic
  aborts writeback for the whole invocation, preserves both invalid and valid
  input files, reports human parse diagnostics to stderr, and exits with code
  1.
- Preserving any comment-bearing file byte-for-byte is compatible with item 5.
  It prevents destructive trivia movement while keeping the lossless tree's
  comment retention available for later comment attachment. This means comment
  files are deliberately no-op formatted until formatter stabilization.
- Fixture coverage is sufficient to start item 6, but it is not exhaustive
  formatter stabilization coverage. Before claiming a more complete formatter,
  add focused golden/idempotence fixtures for `ensure`, prefix and binary
  precedence, postfix `?`, nested records/lists/calls, multiple input files
  without parse errors, and comment attachment once comments stop being no-op
  preserved.

Item 9 completion gate status:

- Complete: `veln test [--json] [target ...]` is available. It reuses the
  existing project source discovery, parser, semantic diagnostics,
  checked-core lowering, typed IR, JVM backend, temporary artifact write,
  `javac`, and `java` execution path.
- Complete: explicit targets select zero-argument functions in the selected
  files. With no targets, selection is restricted to zero-argument functions in
  discovered `*_test.veln` files. Public and private zero-argument functions
  are both eligible test cases.
- Complete for the first-slice same-file example boundary: explicitly targeted
  non-`*_test.veln` files can be run as test files by selecting their
  zero-argument functions. Comment/docblock example extraction remains later
  work.
- Complete: static gates block before user code execution on parse errors,
  semantic errors, reachable holes, and checked-core blockers. Blocked test
  JSON keeps top-level diagnostics and marks already discovered cases as
  blocked.
- Complete: `veln test --json` emits the required run-level fields
  `schema_version`, `command`, `status`, `selection`, `summary`,
  `diagnostics`, `suite_errors`, and `cases`, and each case emits the required
  `id`, `name`, `kind`, `status`, `source`, `reason`, `failure`, `events`, and
  `diagnostics` fields.
- Complete for first-slice captured output: case stdout/stderr are captured as
  deterministic `kind: "stdio"` events with stream, operation, text,
  terminator, sequence, node id, and source span. The current implementation
  records aggregate process stdout/stderr per case rather than one event per
  individual stdio call.
- Complete: focused CLI coverage includes static-gate blocked JSON without a
  JDK, missing `javac` JSON behavior, default `*_test.veln` discovery,
  passed and failed cases, captured stdout/stderr events, and explicit
  same-file target execution when a JDK is available.

Item 9 review notes:

- No blocking findings: implementation order items 1-9 are complete for the
  first-slice gate, and the standard edit loop now exists through `veln fmt`,
  `veln check --json`, `veln run <entry>`, and `veln test`.
- `veln test [--json] [target ...]` shares the same static gates and JVM
  execution path as `run`, supports explicit target files, defaults to
  discovered `*_test.veln` files, reports deterministic selection metadata,
  and emits the required run and case JSON fields.
- Same-file examples are complete only at the current first-slice boundary:
  an explicitly targeted non-test file contributes its zero-argument functions
  as cases. Parsed docblock/example extraction, expected-output examples, and
  automatic same-file example discovery remain follow-up work.
- Static gate behavior is acceptable for this slice. Parse and semantic errors
  block the suite before Java execution, reachable holes and checked-core
  blockers block selected cases, runtime failures become failed cases, and JDK
  setup failures are surfaced as runner errors in JSON.
- Captured stdio events satisfy the required event-key shape, use
  source-relative spans, and are deterministic for the current execution path.
  They are aggregate stdout/stderr events attached to the test function rather
  than per-stdio-operation events attached to the exact call site, so exact
  operation names, newline terminators, and call-site provenance should be
  implemented before claiming full conformance with the stdio event decision.
- Coverage is sufficient for the first-slice claim, but stabilization should
  add focused fixtures for missing `java` after `javac` succeeds, static-gate
  parse and semantic diagnostics in `veln test --json`, no-test discovery
  suite errors, explicit directory targets, multiple test files, and exact
  stdio event fields.

Next recommended implementation step: plan the next phase from a completed
first slice. Keep match expressions, dictionary literals, transitive effect
inference through undeclared helpers, full contract predicate parsing, strict
`satisfy` suffix validation, comment attachment, formatter stabilization
fixture expansion, broader lowering stabilization fixtures, entry arguments,
persistent build caching, per-stdio-call capture with exact call-site
provenance, docblock example extraction, runtime contract discharge, and
candidate ranking as later follow-up work.

## Related Decisions

- [First Implementation Architecture](../discussions/2026-05-24-agent-language-spec-wall/result-first-implementation-architecture.md)
- [First Implementation Commands](../discussions/2026-05-24-agent-language-spec-wall/result-first-implementation-commands.md)
- [First-Slice Grammar](../discussions/2026-05-24-agent-language-spec-wall/result-first-slice-grammar.md)
- [AST Phase Boundary](../discussions/2026-05-24-agent-language-spec-wall/result-ast-phase-boundary.md)
- [AST Implementation Representation](../discussions/2026-05-24-agent-language-spec-wall/result-ast-implementation-representation.md)
- [Minimum Type System for Holes](../discussions/2026-05-24-agent-language-spec-wall/result-minimum-type-system-for-holes.md)
- [First-Slice Prelude Helpers](../discussions/2026-05-24-agent-language-spec-wall/result-first-slice-prelude-helpers.md)
- [Stdio API and Output Events](../discussions/2026-05-24-agent-language-spec-wall/result-stdio-api-and-output-events.md)
- [Check JSON Details Fields](../discussions/2026-05-24-agent-language-spec-wall/result-check-json-details-fields.md)
- [Transitive Effect Diagnostics](../discussions/2026-05-24-agent-language-spec-wall/result-transitive-effect-diagnostics.md)
- [Contract Predicate Parsing](../discussions/2026-05-24-agent-language-spec-wall/result-contract-predicate-parsing.md)
- [Hole Satisfy Source Syntax](../discussions/2026-05-24-agent-language-spec-wall/result-hole-satisfy-source-syntax.md)
