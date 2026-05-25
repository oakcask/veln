# Discussion Result: Comparison Example Task

Status: accepted-proposal
Implementation: Veln support implemented; cross-language examples remain
outside this workspace.

## Picked Question

- Which first comparison example should be written across Ruby, Python,
  TypeScript, Elixir, Rust, and Veln?

## Decision

Use one canonical "line-item order summary" task for the first comparative
examples.

The task is a dependency-free command plus a pure core function:

```text
summarize_order(lines, catalog) -> Result<OrderSummary, OrderError>
```

`lines` is a list of strings in `sku,quantity` form. `catalog` maps `sku` to a
unit price in cents. The function parses every line, rejects malformed rows,
rejects non-positive or non-integer quantities, rejects unknown SKUs, and
returns an order summary with at least `item_count` and `subtotal_cents`.

Each language comparison should include:

- a complete implementation;
- tests for success, malformed input, bad quantity, and unknown SKU;
- one small command-line wrapper that prints the subtotal to stdout;
- for Veln only, a partial-program variant with a typed hole in the line parser
  and the expected `veln check --json` diagnostic shape.

## Rationale

The comparison should evaluate repair-loop shape, not crown a general-purpose
winner among languages. Nanz and Furia's Rosetta Code study is useful because it
uses common tasks to compare languages empirically, but Veln's first comparison
should be narrower: one fixed task, one stated purpose, and no broad productivity
claim beyond the observed example.

The task needs to be large enough to show syntax and tool behavior at program
structure scale. Stefik and Siebert treat programming-language syntax as an
empirical usability factor, and Lappi, Tirronen, and Itkonen's replication
notes that broader program structures may be more informative than isolated
word or symbol ratings. A whole tiny program therefore gives better evidence
than comparing isolated constructs such as function declarations or `if`
syntax.

The line-item task also exercises the cognitive dimensions that matter for an
agent-facing language. Green and Petre's framework pushes the comparison toward
viscosity, hidden dependencies, error-proneness, diffuseness, and closeness of
mapping. This task exposes all of those: a change to the error representation,
input grammar, or summary fields crosses parsing, types, tests, and output; a
repairing agent must understand where each dependency is recorded.

The selected task fits Veln's first-slice thesis. It uses records, lists,
dictionaries, `Option`-shaped lookup, `Result` propagation, fallible traversal,
contracts over the returned summary, a coarse `stdio` effect in the wrapper,
and structured diagnostics for the Veln hole variant. It does not require
filesystems, networking, time, randomness, user-defined ADTs, async execution,
package manifests, or performance-sensitive benchmarking.

## First-Slice Rules

- The comparison task is named `line-item order summary`.
- The Veln version should keep parsing and summarization in a pure function and
  put stdout in a separate wrapper with the `stdio` effect.
- The public pure function should return `Result` rather than throwing or
  exiting.
- The first Veln version should use record-shaped summaries and errors, not
  user-defined ADT declarations.
- The first Veln version should use `list_try_map` or an equivalent prelude
  helper for fallible traversal across input lines.
- The tests should include one successful two-line order, one malformed row,
  one non-positive quantity, and one unknown SKU.
- The comparison text must separate language surface observations from
  toolchain observations. For Veln, `veln check --json` diagnostics are part of
  the toolchain observation.
- The comparison must not claim benchmark-style speed, memory, or productivity
  conclusions from this single task.

## Open Details

The exact sample SKUs, output string, and record field spellings can be chosen
when the examples are written. They should remain identical across languages
unless a language needs an idiomatic wrapper around the same data.

The comparison can later add a second task for module boundaries or persistent
state. That should be a separate result, because this task intentionally avoids
filesystem and package-layout questions.

## References

- `nanz2015-rosetta-code`
- `green1996-cognitive-dimensions`
- `stefik2013-programming-language-syntax`
- `lappi2023-syntax-intuitiveness-replication`
- `barik2018-compiler-explanations`

## Consequence

The next comparison work has a bounded target. Agents can write examples in
Ruby, Python, TypeScript, Elixir, Rust, and Veln against the same parse,
validate, summarize, test, and stdout requirements, while Veln additionally
demonstrates typed-hole diagnostics and repair-loop evidence.
