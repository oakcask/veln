---
review-when: The handler clause syntax, migration boundary, acceptance evidence, or implementation status changes.
---

# Explicit Handler Operation Clauses

Status: implemented

## Summary

Replace handler provider references with explicit operation clauses. Each
clause binds the handled operation arguments and evaluates an ordinary Veln
expression. A clause that delegates to an external function writes an ordinary
function call with every argument visible at the call site.

The completed proposal removes the existing `operation = provider` syntax.
There is no permanent compatibility alias, feature flag, or formatter output
for that syntax.

Current handler behavior is specified in
[names-effects-full.md](../../specification/names-effects-full.md) and the
current source grammar is specified in
[source-surface-full.md](../../specification/source-surface-full.md).

## Motivation

The current syntax hides a calling convention:

```veln
handler console(prefix: String) handles Console effects [stdio]
	write = provide_write
end
```

The provider function must receive every handler context parameter before
every operation parameter. The assignment does not show those arguments. A
reader must combine three declarations to derive the provider signature.

The hidden parameter prefix makes an ordinary function look as if it has
handler-specific calling semantics. Signature errors are reported only after
the reader has applied that convention incorrectly. The convention also makes
handler context look like implicit mutable state even though the context is a
lexically captured value.

## Proposed Source Form

An operation clause has this form:

```veln
operation_name(parameter_names) => expression
```

The handler context parameters are in lexical scope in every clause
expression. The clause parameter names bind the operation arguments. Their
types come from the handled effect declaration.

The `=>` token follows the existing `match` arm convention. In both forms, the
left side selects and binds an input shape, and the right side evaluates the
result expression.

```veln
effect Console
	write(text: String) -> ()
	error(text: String) -> ()
end

fn print_with_prefix(prefix: String, text: String) -> () effects [stdio]
	stdio::println(prefix + text)
end

handler console(prefix: String) handles Console effects [stdio]
	write(text) => print_with_prefix(prefix, text)
	error(text) => print_with_prefix("ERROR: " + prefix, text)
end
```

External functions have no handler-specific signature. Each external call is
checked as an ordinary call. The source explicitly selects, orders, repeats,
or omits context and operation values in the same way as any other function
call.

The standard duplex-stream handler becomes:

```veln
pub handler net_stream(stream: NetStream) handles transport::DuplexStream effects [net]
	read_chunk() => read_stream_chunk(stream)
	write_chunks(chunks) => write_stream_chunks(stream, chunks)
end
```

## Grammar

The source grammar replaces `HandlerProvider` with
`HandlerOperationClause`:

```text
HandlerDecl            ::= "pub"? "handler" Name "(" ParamList? ")"
                           "handles" MemberPath Effects? NL
                           HandlerOperationClause+ "end" NL?
HandlerOperationClause ::= Name "(" HandlerOperationParams? ")" "=>" Expr NL
HandlerOperationParams ::= Name ("," Name)*
```

Clause parameters do not repeat type annotations. The effect operation is the
single source of their types. A clause parameter is a lexical binding and does
not have to reuse the parameter name from the effect declaration.

The old production is removed:

```text
HandlerProvider ::= Name "=" MemberPath NL
```

After proposal completion, source such as `write = provide_write` is a syntax
error. The parser must not reinterpret it as an operation clause.

## Static Semantics

For an effect operation with parameter types `O1, ..., On` and result type
`R`, a matching clause must bind exactly `n` distinct parameter names. The
checker assigns `O1, ..., On` to those bindings in source order. The checker
checks the clause expression with expected type `R`.

Handler context parameters keep their declared types and are available by
normal lexical name resolution. Referencing a context parameter does not add
an implicit argument to an external call.

A clause parameter may use the same name as a handler context parameter. The
clause parameter is the inner lexical binding and shadows that context
parameter throughout the clause expression. Other clauses keep access to the
context parameter unless they declare the same clause parameter name.

Each handler must contain exactly one clause for each operation in the handled
effect. A missing clause, duplicate clause, or clause for an unknown operation
is an error. A clause with the wrong parameter count or duplicate parameter
name is an error.

The checker infers clause effects from the clause expression. A clause must not
retain the effect handled by its enclosing handler. A public handler must
declare every other retained effect. A private handler keeps the existing
effect-inference behavior.

The type and effect rules for an external function call inside a clause are
the ordinary call rules. The handler checker does not construct or compare a
special provider function signature.

## Dynamic Semantics

Installing a handler evaluates its context arguments once, from left to right,
before it evaluates the handled body. Each captured result remains available
to every operation clause for that handler installation.

When the body performs an operation, the runtime binds the performed argument
values to the matching clause parameters and evaluates the clause expression.
The expression result is the operation result.

This proposal preserves deep handling, same-effect nested-handler shadowing,
task-local handler installation, early-return cleanup, and effect replacement
on the handled expression. It does not add mutable handler state.

## Diagnostics Contract

Public diagnostics use operation-clause terminology after migration. They do
not describe an external function as a provider.

| Current diagnostic | Completed-proposal disposition |
| --- | --- |
| `handler.missing_provider` | Replace with `handler.missing_operation_clause` |
| `handler.duplicate_provider` | Replace with `handler.duplicate_operation_clause` |
| `handler.provider_signature` | Remove; ordinary expression and call diagnostics cover the clause |
| `handler.provider_unknown` | Remove; ordinary name resolution covers the call target |
| `handler.recursive_provider` | Replace with `handler.recursive_operation_clause` |
| `handler.unknown_operation` | Keep, with its boundary and message changed to operation-clause terminology |

New handler diagnostics use `boundary = "handler_operation_clause"`. Their
structured details retain `handler`, `handled_effect`, and `operation` when
those values exist. They do not emit a `provider` field or a compatibility
alias for it.

An old-form clause receives a syntax diagnostic at the missing `(` boundary.
The diagnostic states that a handler operation clause must bind its operation
parameters and evaluate an expression. It must not suggest that the old form
remains supported.

## Acceptance Cases

Planned executable cases belong under `../../examples/specification/`. The
grammar, formatter, semantic, CLI, and runtime cases together are the primary
acceptance evidence. The examples in this proposal are planned syntax and are
not current executable evidence.

| Case | Input distinction | Required observation |
| --- | --- | --- |
| Explicit external call | A clause passes handler context and operation arguments to an external function | Check succeeds and the run result uses the explicitly written argument order |
| Direct expression | A clause computes its result without an external helper | Check succeeds and the expression result becomes the operation result |
| Multiple operations | One effect declares operations with zero, one, and multiple parameters | Exactly one explicit clause for each operation checks and runs |
| Clause delimiter | One handler uses `=>`; rejected variants use `=` after a parameter list | The `=>` form parses and each `=` form fails at the clause delimiter |
| Renamed bindings | Clause parameter names differ from effect parameter names | Check succeeds and types follow operation parameter order |
| Context shadowing | A clause parameter has the same name as a handler context parameter while another clause uses the context parameter | The first clause resolves the name to its operation argument and the other clause resolves it to the captured context value |
| Parameter count failure | A clause binds fewer or more parameters than its operation | Check fails at the clause parameter boundary |
| Duplicate binding failure | One clause repeats a parameter name | Check fails at the duplicate binding |
| Result failure | A clause expression does not produce the operation result type | Check fails at the clause expression with the operation result as its expected type |
| Ordinary call failure | An external call has an unknown target or incompatible arguments | The ordinary name or call diagnostic is emitted; no provider-signature diagnostic is emitted |
| Coverage failures | A handler has a missing, duplicate, or unknown operation clause | The matching operation-clause diagnostic is emitted with declaration-related context |
| Recursive clause failure | A clause expression retains its handled effect | `handler.recursive_operation_clause` is emitted |
| Public retained effect | A public clause calls an effectful function with and without the required handler effect declaration | The declared case succeeds and the missing-effect case fails |
| Context evaluation | Context expressions have observable effects and multiple clauses use their captured values | Context expressions run once, left to right, before the handled body |
| Existing handler semantics | Nested, repeated-operation, task-boundary, and early-return cases use the new clauses | Their existing observable results remain unchanged |
| Standard duplex handler | The standard transport handler delegates both operations explicitly | Static effect replacement and loopback runtime cases retain their current observations |
| Old syntax removal | Source contains `operation = function_path` | Parsing fails and no formatter, compiler, standard-library, or current-example path accepts it |
| Editor behavior | Rename, definition, references, semantic tokens, and formatting operate on calls inside clauses | Editor results match ordinary lexical bindings and function calls |

The executable source grammar must accept the new production and reject the
old production. Parser and formatter unit cases must cover zero-parameter and
multi-parameter clauses. Semantic cases must cover every static row above.

## Migration Plan

Implementation may use temporary internal commits in which both source forms
parse. Such coexistence is an implementation staging tool and is not a
completed language state.

1. Add operation-clause syntax, AST representation, formatting, lowering, and
   semantic checks.
2. Convert compiler tests, executable grammar cases, specification examples,
   standard-library handlers, and editor tests to operation clauses.
3. Replace public provider-specific diagnostics and structured fields with the
   operation-clause contract.
4. Remove the old parser production, provider-reference AST surface,
   provider-signature checking, provider-reference editor special cases, and
   formatter support.
5. Search all source, tests, current documentation, generated grammar, standard
   library, diagnostics, and editor code for residual old syntax and
   provider-reference terminology. Classify any residual occurrence as a
   historical explanation or remove it.
6. Promote the implemented behavior and executable evidence to the current
   specification. Move this proposal to the implemented-proposal records and
   remove it from the proposal catalog.

## Migration Map

| Old surface | New surface | Compatibility policy |
| --- | --- | --- |
| `operation = function_path` | `operation(parameters) => expression` | Remove |
| Implicit context-then-operation provider signature | Explicit ordinary calls in clause expressions | Remove |
| `HandlerProvider` grammar and public syntax model | `HandlerOperationClause` | Remove |
| Provider-specific signature and lookup diagnostics | Ordinary expression, call, and name diagnostics | Remove |
| Provider-specific missing, duplicate, and recursive terminology | Operation-clause diagnostics | Replace without aliases |
| LSP provider-reference special handling | Ordinary call and lexical-binding handling inside clauses | Remove |

Historical records may quote the old syntax when the surrounding text labels
it as former behavior. Current specifications, runnable examples, standard
library source, and user-facing editor behavior must not retain it.

## Non-Goals

- Do not add mutable handler state.
- Do not allow one handler declaration to handle multiple nominal effects.
- Do not change `perform` syntax.
- Do not change effect-row substitution or handler effect replacement.
- Do not change deep handling, nesting, task boundaries, or cleanup behavior.
- Do not add a permanent compatibility mode for the old handler syntax.
- Do not require an automatic source migration command.
- Do not require runtime closures when an implementation can lower clauses to
  equivalent internal functions.

## Planned Verification

Implementation must keep repository-relative checks for these surfaces:

```sh
bash scripts/agent-test -p veln-syntax
bash scripts/agent-test -p veln-sema
bash scripts/agent-test -p veln-lsp
bash scripts/agent-test -p veln-cli --test toolchain_harness
```

The implementation must also run the executable specification cases selected
for lexical handlers and the standard duplex-stream handler. The completion
record must name those cases rather than claiming coverage from crate tests
alone.

## Residual Provider Terminology Audit

Remaining `provider` hits are not source-surface provider references:

- Core, IR, JVM backend, and runtime handler-table names retain `providers`
  for the operation-to-function table that executes already-lowered handler
  clauses.
- This record uses `provider` to describe the removed syntax, diagnostic
  migration, and acceptance boundary.
- Other reference records and editor-support text use unrelated meanings such
  as extension semantic token providers or cryptography providers.
- Example string literals that print `provider` are ordinary program data.

## Completion Boundary

This proposal is complete only when all acceptance cases pass and the old
syntax is rejected. No public compatibility alias may remain.

Completion must update the executable and rendered source grammar, handler
typing and effect specification, diagnostics JSON specification, standard
duplex-stream example, and lexical-handler executable cases. A repository-wide
residual-name audit must classify every remaining provider-reference hit.

This completed record is retained only for history. Current behavior lives in
`../../specification/` and executable evidence.
