# Discussion Result: Runtime Contract Failure Reporting

## Picked Question

- How should runtime contract failures appear in `veln run` and `veln test`:
  process failures, structured runtime errors, test assertions, or a smaller
  first-slice combination?

## Decision

Use a structured runtime contract error as the common representation, then map
it to the command context.

For `veln run`, a runtime contract violation is a user-code execution failure.
The command should stop the selected execution, print a concise human-readable
contract failure, and exit non-zero. In JSON mode, it should emit one structured
runtime error record with `kind: "contract"` and the same blame vocabulary used
by `check` contract diagnostics.

For `veln test`, a runtime contract violation inside an individual test body or
executable example should be reported as a failed test case with the structured
contract error attached. It should not be reported as a compiler crash or test
harness crash. The test command exits non-zero if any test has such a failure,
but the runner may continue independent tests when the first implementation has
enough isolation to do so.

Contract failures that occur before a test case is selected, during discovery,
or in shared setup outside a specific test should be reported as suite-level
runtime errors rather than assertion-like test failures.

## Rationale

The first slice already decides that valid but statically unknown contracts
become runtime-required obligations. Reporting those failures only as generic
process termination would discard the reason the contract system exists:
explicit boundary obligations with repair-routing metadata.

Meyer's Design by Contract framing treats preconditions and postconditions as
runtime-checkable obligations at software boundaries, and even separates the
broken-contract topic from ordinary implementation mechanics. That supports
making a contract violation visible as a first-class user-code failure rather
than an internal tool error.

Findler and Felleisen's higher-order contract work is the stronger guide for
Veln's agent-oriented diagnostics: dynamic contract checking can preserve blame
information when the property is not statically known. Veln should therefore
carry `blame: "caller"` or `blame: "implementation"` through runtime reporting,
not just through `check --json`.

JML is also useful precedent because it explicitly positions one specification
language as input to multiple tools, including a runtime assertion checker,
static checkers, and documentation generators. Veln should follow the same
separation of concerns: `check` validates and classifies obligations, while
`run` and `test` enforce runtime-required obligations and report the observed
failure in their own command contexts.

For tests, the most useful repair-loop behavior is assertion-like failure when
the tested behavior violates a contract. A failed postcondition in a test tells
the agent where the implementation broke its advertised behavior; a failed
precondition tells it that the test or caller constructed an invalid call. In
both cases the test should fail with structured contract evidence, not obscure
the failure as a harness error.

## First-Slice Rule

- Runtime contract failures use `kind: "contract"` and `phase: "runtime"` in
  structured output.
- Runtime contract error details must include the failed clause identity, the
  clause kind (`require` or `ensure`), the failed clause text or structured
  predicate, the clause span, the function or boundary being checked, and
  `blame` when known.
- `require` failures keep `details.blame: "caller"` unless later evidence
  supports a narrower rule.
- `ensure` failures keep `details.blame: "implementation"` unless later
  evidence supports a narrower rule.
- `veln run` maps a runtime contract error to a non-zero process exit and one
  top-level runtime error record.
- `veln test` maps a runtime contract error inside a test case or executable
  example to a failed test result with the contract error embedded.
- `veln test` maps runtime contract errors outside a selected test case to a
  suite-level runtime error.
- Human output should name the failed clause and blame route. JSON output should
  keep the same stable envelope style as diagnostics where practical, while
  allowing test result records to wrap the contract error.

## Open Details

The exact exit code is not decided here. The first implementation only needs a
stable non-zero outcome for contract-failed `run` and contract-failed `test`.

The first test-output wrapper is resolved by
[Test JSON Shape](result-test-json-shape.md). This decision only requires that
the embedded contract error preserve the same blame and source identity as
contract diagnostics.

Observed runtime values may be useful in messages, but the first slice should
avoid making full value capture mandatory. Large values, secret-bearing values,
and non-printable values need a separate display policy.

## References

- Meyer, B. (1997). *Object-Oriented Software Construction* (2nd ed.).
  Prentice Hall PTR. https://archive.eiffel.com/doc/oosc/
- Findler, R. B., & Felleisen, M. (2002). Contracts for higher-order
  functions. *ICFP 2002*, 48-59.
  https://dblp.org/rec/conf/icfp/FindlerF02
- Leavens, G. T., Cheon, Y., Clifton, C., Ruby, C., & Cok, D. (2013).
  *JML Reference Manual: Introduction*.
  https://www.cs.ucf.edu/~leavens/JML/jmlrefman/jmlrefman_1.html

## Consequence

The first implementation gets one contract-failure model that works across
`check`, `run`, and `test`. Agents can route repairs from runtime evidence
using the same blame metadata they use for static diagnostics, while humans get
ordinary non-zero command results that fit shell and CI workflows.
