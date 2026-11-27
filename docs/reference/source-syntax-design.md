---
role: reference
authority: normative
update-when: The Veln source-syntax design requirements for expression and block forms, token boundaries, lexical cleanup scopes, canonical formatting, or syntax review evidence change.
---

# Source Syntax Design Policy

This policy governs the design and review of Veln source syntax. It requires
explicit choices about expressions, bodies, and scopes so similar constructs
do not acquire conflicting rules through unrelated changes. It applies to
syntax proposals and implementation changes, including formatter changes
that alter the maintained source form.

The [language specification](../specification/README.md) owns implemented
behavior. This policy does not add syntax or make a proposed form available.
Existing forms need not be migrated merely to satisfy a new classification.
Each changed form must distinguish retained behavior from its proposed target.

## Classify the form

For each changed construct, identify what it accepts and how its extent is
recognized. Use these semantic categories; no document heading or metadata
field is required to encode them.

| Category | Design requirement |
| --- | --- |
| Single-expression form | Accept one expression. A newline by itself must not turn its operand into a sequence of statements. A block expression can be that one expression. |
| Block form | Accept body lines. Identify the opening syntax, branch separators if any, and the closing token. State whether an empty body is allowed and how it gets its result. |
| Paired expression and block forms | Specify how the parser selects the form, including newlines and comments, and whether the forms have equivalent scope and evaluation semantics. Support for a block does not require adding a corresponding expression form. |

Distinguish a single expression from a single physical line. A `begin`, `if`,
or `match` expression can span lines while retaining its own delimiters.
Distinguish a statement sequence from a new lexical or cleanup scope; state
the scope semantics instead of inferring them from line count.

For token-delimited bodies, leading indentation must not determine statement
ownership or scope. Demonstrate the boundaries with nested input whose leading
whitespace is removed. Keep meaningful newlines, comments, and literal content.
This does not remove the need to match nested delimiters.

Do not introduce a bare sequence after an expression-taking arrow merely by
putting a newline after it. A multiline sequence there uses an explicitly
delimited block expression. If a new construct requires another boundary
model, document its user benefit, its effect on reading without indentation,
and why an existing block expression does not meet the need. Review that
departure explicitly; a formatter preference alone is not its justification.

## Define scope and evaluation separately

Every changed form must answer the applicable questions below. Retained rules
can link to their current specification rather than duplicate it.

- Which bindings enter the body, which local bindings can be shadowed, and
  which bindings remain visible after it?
- Does the form introduce a lexical scope, a cleanup scope, both, or neither?
  Where do `defer` registrations belong and when does cleanup run?
- What is the result of an expression tail, a non-expression tail, or an empty
  body? What expected type and effects flow into and out of the form?
- Which operands evaluate immediately and which evaluate later? For deferred
  execution, distinguish captured bindings from argument evaluation at
  registration and argument evaluation at cleanup.
- How do normal completion, result propagation, runtime or contract failure,
  cancellation, and cleanup failure interact with scope exit?

For paired forms, a claim of equivalence must cover these observations, not
just the returned value. State any intentional semantic difference and how the
source makes it visible.

## Specify canonical source and preservation

Follow [Canonical Form](proposal-authoring.md#canonical-form)
for accepted input, canonical output, convergence, migration, comments, and
idempotence. Define both forms when a construct supports a pair.

Formatting must preserve explicit scope boundaries and cleanup timing. Do not
remove a `begin` solely because its body has one expression. A proposal for a
scope-changing conversion must independently define its semantic eligibility,
comment preservation, and verification. Manual semantic edits are distinct
from ordinary formatting.

## Review and enforcement

Syntax work is reviewable when representative examples establish:

| Concern | Review evidence |
| --- | --- |
| Classification and boundaries | Valid single-expression and block examples where supported, invalid mixtures, nesting, and leading-whitespace removal. Include comments around any newline that selects a form. |
| Scope and evaluation | An expected binding/result/effect outcome and an explicit cleanup or evaluation trace when timing is observable. Include a material failure path. |
| Canonical source | Before/after examples, comment attachment, explicit scope preservation, second-format stability, and a decision about old spellings. |
| Verification | A mapping to grammar fixtures, parser and semantic tests, formatter cases, and backend observations as applicable. Identify planned evidence as planned. |

Reviewers must request correction when a material choice is unspecified,
examples contradict the stated category, or the planned checks cannot detect
the stated boundary or scope violation. Equivalent explanations are allowed;
exact headings, row names, or matching phrases are not evidence of compliance.

This is a permanent design-review guard, with agent discovery through
`AGENTS.md` and author discovery through the documentation policy. Existing
frontmatter and link checks validate the routes, not semantic consistency.
No keyword linter or new CI job enforces these judgments. The residual risk is
that review misses a contradictory example or an untested scope boundary.

When a form is implemented, grammar fixtures and observable semantic/formatter
tests enforce its concrete rules. A future change to indentation semantics,
arrow boundaries, or scope-changing formatting requires reassessing the
corresponding policy decision with explicit evidence. Changes to syntax-test
infrastructure may justify additional mechanical enforcement if it can check
behavior rather than document wording.
