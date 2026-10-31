---
role: specification
authority: normative
update-when: The `veln check --json` diagnostic schema, human `veln check` rendering, stable check detail fields, check diagnostic producers, or source-visible runtime diagnostic projection changes.
specification-coverage: usage=#usage; behavior=#diagnostic-families; limits=#limits
---

# Check JSON And Diagnostics

This page specifies the shared diagnostic envelope used by `veln check --json`
and by static failures from `run --json`. Test reports reuse the diagnostic
object shape within their own [test envelope](test-json.md).

## Usage

Run `veln check --json [INPUTS ...]` when a consumer needs structured analysis
diagnostics. The command emits one JSON object on stdout. Use
[commands.md](commands.md) for input discovery and exit behavior; use
[run-json.md](run-json.md) and [test-json.md](test-json.md) for completed
execution records.

## Envelope and common fields

The envelope has these fields:

| Field | Contract |
| --- | --- |
| `schema_version` | Number `1`. |
| `tool` | Object with string `name` and `version`. |
| `status` | `error` if any diagnostic has severity `error`; `partial` if there is no error and at least one `hole` diagnostic; otherwise `ok`. |
| `diagnostics` | Ordered array of diagnostic objects. |
| `summary` | Object with `diagnostic_count`, `by_severity`, and `by_kind`; each map counts the corresponding strings. |

Each diagnostic has non-null string fields `id`, `severity`, `kind`, and
`message`, plus `span`, `details`, and `related`. `span` is `null` for a
spanless diagnostic or an object with `file`, `start`, and `end`. `file` is
the source path. Each position has one-based `line` and Unicode-scalar
`column`, and zero-based byte `offset`; the end position is exclusive.
`details` is the producer's JSON
value and may be `null`. `related` is always an array of producer-supplied
JSON values.

The primary message names the failed fact at its reported span. Causes,
provenance, repair hints, and other locations belong in `related` or
structured `details`. Producers omit detail keys when the fact is unavailable;
consumers must preserve diagnostic and related-note order.

## Diagnostic families

A known expression form outside the module immutable value finite-data
language reports `value.module_initializer_unsupported` at the smallest
unsupported expression. Its details contain `phase: "type_check"` and the
declaration `name`. Its first related entry has kind
`module_value_declaration` and the complete declaration span. Its second has
kind `permitted_initializers`, has no span, and names the admitted categories.
When a call has a callee that is not a name path, the complete call is the
smallest unsupported expression.
Unresolved names and call targets retain `name.unresolved`, and a constructor
with the wrong payload arity retains `core.constructor_arity_mismatch`; these
failures do not add `value.module_initializer_unsupported`.

A cyclic dependency component formed by selected same-module or imported
module-value references reports one `value.module_initializer_cycle` at the
deterministic reference that closes the cycle. Its details contain
`phase: "type_check"`, the referring declaration `name`, and the referenced
`target`. Related entries first list every participating declaration with kind
`cycle_declaration`, ordered by declaration identity and then source location.
They then list every dependency within the component with kind
`cycle_reference`, ordered by source identity, target identity, and reference
location. Every such entry carries the corresponding declaration or reference
span. A reference to a colliding module-value identity selects no declaration
and cannot create a cycle diagnostic from an arbitrary member of the
collision. The checked same-module
[`cycle human`](../../examples/specification/check/module-value-initializer-cycle-human/)
case covers the rendered primary location and related-note order; the checked
[`cycle JSON`](../../examples/specification/check/module-value-initializer-cycle-json/)
case covers the structured details and spans. The checked cross-module
[`human`](../../examples/specification/check/module-value-cross-module-cycle-human/),
[`reordered human`](../../examples/specification/check/module-value-cross-module-cycle-human-reordered/),
[`JSON`](../../examples/specification/check/module-value-cross-module-cycle-json/),
and
[`reordered JSON`](../../examples/specification/check/module-value-cross-module-cycle-json-reordered/)
cases additionally fix transitive quarantine and the same locations after
source and import reordering. The quarantine suppresses diagnostics that would
otherwise arise from direct and multi-hop dependents reading unavailable
module values. The checked bare-import cycle
[`human`](../../examples/specification/check/module-value-bare-imported-cycle-human/),
[`reordered human`](../../examples/specification/check/module-value-bare-imported-cycle-human-reordered/),
[`JSON`](../../examples/specification/check/module-value-bare-imported-cycle-json/),
and
[`reordered JSON`](../../examples/specification/check/module-value-bare-imported-cycle-json-reordered/)
cases fix the same primary and related locations when ordinary import exposure
selects an edge and an aliased package-distinct namesake remains healthy.

A function type in a module value annotation uses the ordinary effect-list
diagnostics. An unbound row tail reports `effect.row_unbound`, and an unknown
effect label reports `effect.unknown`. Their details identify the boundary as
`module_value_type`.

A public module value annotation that recursively names a private type reports
`type.public_annotation_private` at the written type occurrence. Its closed
details contain `boundary: "public_module_value"`, the declaration `name`,
`written_type`, and `resolved_type`. Related entries identify the
`private_type_declaration` when available and then the
`public_module_value_boundary`. The checked
[`human`](../../examples/specification/check/module-value-public-annotation-visibility-human/)
and
[`JSON`](../../examples/specification/check/module-value-public-annotation-visibility-json/)
cases fix the rendered and structured forms.
A private type alias produces this diagnostic even when its terminal target is
public. In that case, `resolved_type` and the `private_type_declaration` related
entry identify the written alias declaration. Nested aggregate occurrences use
the same rule. The checked private-alias
[`human`](../../examples/specification/check/module-value-private-alias-annotation-human/)
and
[`JSON`](../../examples/specification/check/module-value-private-alias-annotation-json/)
cases fix the direct and nested forms.
Annotation resolution uses the imports of the module that declares the value.
It retains the selected type's package and module identity when producing
private-declaration context. An exact written import path takes precedence over
a same-spelled type in the declaration owner's module. The imported type's
identity therefore controls whether the annotation is public; a bare type name
continues to select the owner's type. If the annotation does not resolve from
that owner, the checker instead reports `name.unresolved` at the written type.
An equal type spelling in the consuming package or another package does not
change that diagnostic into `type.public_annotation_private` or supply
private-declaration context. The checked imported-owner
[`human`](../../examples/specification/check/module-value-imported-annotation-owner-human/)
and
[`JSON`](../../examples/specification/check/module-value-imported-annotation-owner-json/)
cases cover package-distinct imports and their private declaration identities.
The checked owner-package fallback
[`human`](../../examples/specification/check/module-value-owner-package-fallback-human/)
and
[`JSON`](../../examples/specification/check/module-value-owner-package-fallback-json/)
cases cover this diagnostic boundary. The checked exact-import collision
[`human`](../../examples/specification/check/module-value-exact-import-owner-collision-human/)
and
[`JSON`](../../examples/specification/check/module-value-exact-import-owner-collision-json/)
cases cover the same-spelled owner boundary and the absence of derivative
unresolved or ambiguity diagnostics.

An available module value selected as a call target reports
`type.not_callable` at that target. Its details identify `call_target` as the
failed constraint and `module_value` as the selected non-callable kind. Related
context identifies the selected declaration. An unavailable same-module
identity still reserves its name, but a call through it does not emit
`type.not_callable`, select declaration context, or fall through to another
callable. Diagnostics from independently checked call arguments remain.
An unavailable imported module value exposes neither a bare nor a qualified
name. With no other eligible declaration, either read reports
`name.unresolved`. For a bare name or a qualified path that selects one target
module, a valid imported value with the same name remains selectable without
`name.ambiguous`, and a valid imported function remains callable without
`name.ambiguous` or `type.not_callable`. These results also apply when a module
value initializer contains the read. If a failed imported candidate initially
makes that initializer ambiguous, removing the failed candidate selects the
valid namesake. A dependent declaration that then satisfies its annotation is
available to importers, so their reads produce ordinary type results rather
than derivative unresolved or ambiguous diagnostics. If a declaration in a
cyclic candidate-dependency component recovers and then fails again, the
checker keeps every declaration in that component unavailable. It retains the
diagnostics from the recurring failure even though the final quarantined state
can no longer reproduce them. Reads from rejected declarations remain
unresolved, while declarations outside the component retain independently
valid facts. An intrinsic failure that remains continuously unavailable does
not reject otherwise valid declarations in its candidate component and does
not add recurring-failure diagnostics. Duplicate declarations, invalid-cased
declarations, and initializer cycles do not recover when imported availability
changes. Conversely, a public
function or function alias withheld by a signature-visibility failure does not
make a same-named imported module value ambiguous or unresolved. An available
public function or alias still makes the bare name ambiguous. A same-module
function or alias collision continues to withhold its module value after an
imported dependency causes that module's availability to be checked again; its
importers emit neither a derivative ambiguity nor `type.not_callable` for the
withheld value.
When module-value lookup reports `name.ambiguous`, each contributing import has
one `import_candidate` related entry at the import declaration. Entries are
ordered by resolved package, module, and candidate kind rather than source or
import order. A bare collision identifies whether each candidate is a module
value or function and gives its qualifying alias. A qualified exact-path
collision identifies every package-distinct selected import before member
visibility or availability is considered, so private, missing, and unavailable
members retain the same complete package-origin context. The package, resolved
module, candidate kind, message, and span remain present in JSON; human output
renders the same package-disambiguating messages. The checked imported-lookup
and exact-member ambiguity cases cover value reads, call targets, reversed
imports, and mixed function/module-value candidates.
The checked
invalid-import exposure
[`human`](../../examples/specification/check/module-value-invalid-import-exposure-human/)
and
[`JSON`](../../examples/specification/check/module-value-invalid-import-exposure-json/)
cases cover bare and qualified initializer reads, published and withheld
function collisions, owner rechecks, valid-namesake selection in both import
orders, and recurrent diagnostics from a rejected cyclic component.
An exact test companion that explicitly imports its target may read an
available private module value without `name.unresolved`. An unavailable
private value remains unresolved, and a module value that depends on that read
remains unpublished. The same read from an ordinary source or a companion for
another target also reports `name.unresolved`.
The checked companion initializer
[`human`](../../examples/specification/check/module-value-companion-private-access-human/)
and
[`JSON`](../../examples/specification/check/module-value-companion-private-access-json/)
cases cover access. The unavailable-target
[`human`](../../examples/specification/check/module-value-companion-private-unavailable-human/)
and
[`JSON`](../../examples/specification/check/module-value-companion-private-unavailable-json/)
cases cover invalid-target and dependent suppression, while the wrong-target
[`human`](../../examples/specification/check/module-value-companion-private-wrong-target-human/)
and
[`JSON`](../../examples/specification/check/module-value-companion-private-wrong-target-json/)
cases cover rejection.
Explicit type arguments on the call target preserve both outcomes, and the
primary span covers the complete type-applied callee. The checked typed-call
cases cover the selected declaration, details, related context, primary span,
and absence of an unresolved-call diagnostic in
[`human`](../../examples/specification/check/module-value-typed-call-human/) and
[`JSON`](../../examples/specification/check/module-value-typed-call-json/)
output. The checked
[`unavailable typed call`](../../examples/specification/check/module-value-unavailable-typed-call-json/)
case covers name reservation and independent argument diagnostics.

A public function alias whose target selects an available module value reports
`name.kind_mismatch` at the alias declaration. The primary message identifies
the target as a module value rather than a function. Its details contain
`phase: "name"`, the alias `node_id`, `expected_kind: "function"`,
`actual_kind: "module_value"`, and the written `target`. A target that is
missing, ambiguous, invalid, quarantined, or otherwise unavailable retains
`name.unresolved` instead. The checked
[`human`](../../examples/specification/check/module-value-function-alias-kind-human/)
and
[`JSON`](../../examples/specification/check/module-value-function-alias-kind-json/)
cases cover bare same-module and qualified written-import targets.

Every parse-clean module immutable value declaration also reports
`module.value_not_executable` at the complete declaration. The diagnostic has
kind `module` and severity `error`. Its `details` object contains
`phase: "module"`, `field: "module_value"`, and the declaration's `name`.
One related note says that checked core and typed IR preserve the declaration
while runtime materialization and code generation remain unavailable. This
diagnostic prevents `veln check` success; the
[source-surface specification](source-surface.md#module-immutable-value-declarations)
defines the accepted declaration syntax and current implementation boundary.

An unresolved call target that uses one of the
[removed compiler-known network spellings](name-resolution.md#removed-network-call-spellings)
reports one `name.unresolved` diagnostic. The primary message and `details`
retain the ordinary unresolved-call shape. When the source module has no
explicit import whose alias is `net`, the diagnostic adds these spanless
related entries in order:

| `kind` | `message` |
| --- | --- |
| `network_import_migration` | ``Import the exported network module with `use net from "std"`.`` |
| `network_effect_migration` | ``Declare the nominal `net::IO` effect for functions that use network operations.`` |
| `network_handler_migration` | ``At the application boundary, handle `net::IO` with `net::system()`.`` |

An unknown `net::...` spelling outside the removed set, an unresolved value,
or a module that already has a `net` import does not receive these migration
entries. The related context does not add a second diagnostic.

`veln check --json` reports an over-limit cleanup form as
`parse.cleanup_nesting_limit`; the parser error makes the envelope status
`error`. A `defer` token used where an expression is required reports
`parse.expected_expression` at the token, with expression parser context and
skip-token recovery. Recovery preserves the boundary of a following
declaration. An unterminated `begin` or `defer` reports
`parse.begin_missing_end` or `parse.defer_missing_end`. The missing cleanup
delimiter does not consume an enclosing declaration's closing `end`, an
enclosing `if` expression's following `else` branch, or an enclosing `match`
expression's following arm. It does not merge a following top-level
declaration into the failed body. If that declaration is a malformed public
module `let`, output retains both the cleanup diagnostic and
`parse.module_value_declaration`; recovery also preserves the next top-level
declaration.

A handler declaration with `handles` followed by another identifier that can
start its effect target reports `parse.expected_token` at the first `handles`.
Its message is `expected for`, its `details.expected` value is `["for"]`, and
its recovery strategy is `skip_token`; recovery treats the first `handles` as
the former separator and retains the following member path as the target. A
handler declaration that omits the separator reports the same diagnostic at
the effect target with `insert_token` recovery. That target can itself be the
ordinary identifier `handles` or a qualified path beginning with `handles::`.
Both recoveries retain the optional retained-effect list and the boundary of a
following declaration.

Malformed integer literals use `parse.integer_literal` with the complete
numeric candidate, parser context, accepted form, and non-cascading recovery;
related notes may identify the accepted digit set or prefix. Invalid literal
shift counts use `type.invalid_shift_count` with `operator`, `actual_count`,
`minimum_count`, and `maximum_count`. In ordinary expressions, contracts, and
hole predicates, the span is the count expression. In a schema field `where`
predicate or schema-level `validate` predicate, the span is the containing
predicate clause.

A remainder divisor written as an integer zero literal reports one
`type.remainder_by_zero` diagnostic with kind `type`, message
`remainder divisor is zero`, and details `operator: "%"` and `divisor: 0`.
Each such divisor in one predicate reports its own diagnostic. The primary span
selects that divisor expression. One related entry has `kind: "operation"`,
message `Remainder operation is here.`, and the span of the remainder operation
that uses that divisor.
Decimal-integer, binary, hexadecimal, parenthesized, and negated zero literals
receive this diagnostic. Computed expressions such as `1 - 1` remain runtime
checked. A non-`Int` zero such as `0.0` reports only its operand
`type.mismatch`. The same diagnostic rules apply in function and test contract
predicates and in hole `satisfy` predicates; schema validation predicates
reject `%` as unsupported syntax.

An invalid remainder operand preserves the diagnostic for the operand itself.
A missing field in a contract predicate uses `contract.field_missing`; the
same failure in `satisfy` uses `hole.satisfy_field_missing`. A prefixed
non-`Int` operand uses `type.mismatch`. A non-`Int` leaf nested in arithmetic
inside the operand also uses `type.mismatch`, and its primary span selects the
leaf instead of the enclosing arithmetic expression. When the operand appears
inside `%` in a call argument, that specific failure is selected before
checking whether the enclosing call result is a valid Boolean predicate. These
failures do not add a generic contract or `satisfy` type-mismatch or
unsupported-construct diagnostic.

An unresolved public type-alias target reports `name.unresolved` at that alias
declaration. Its details contain `phase: "name"`, the alias `node_id`,
`expected_kind: "type"`, and the written `target`. A function target reports
`name.kind_mismatch` at the alias declaration with the same fields and
`actual_kind: "function"`. These target diagnostics remain present when a
same-named alias declaration resolves successfully. The later declaration
independently reports `name.duplicate`, so source order does not suppress the
invalid declaration's target diagnostic.

Malformed variant-refinement-shaped type text uses
`parse.variant_refinement_type`. The
[malformed variant-refinement forms](source-surface.md#malformed-variant-refinement-forms)
define the rejected source shapes. The primary span selects the incomplete or
misplaced separator, segment, generic delimiter, or type argument. Parser
recovery retains the surrounding declaration and lossless source tree. This
diagnostic does not assert that a structurally valid base or final name resolves
semantically.

For a structurally complete refinement with an ASCII-lowercase final segment,
`parse.variant_refinement_type` remains first. Semantic recovery then reports
`name.invalid_case` on carriers classified for casing analysis. A unique
initial-only constructor repair suppresses lookup and
`type.variant_refinement_unknown` failures caused only by that casing, retains
the constructor identity, and permits independent arity, base, union,
visibility, and assignability diagnostics. A repair with no unique result
retains only independently selected identities and suppresses
constructor-dependent failures. Diagnostics keep written-alternative and
per-alternative check order, including when one parse diagnostic is emitted
during source loading and another is emitted after semantic recovery. Recovery
never validates the annotation. The same rules apply independently to every
union alternative and when the base and final segment both need unique
initial-only repairs. A non-generic lowercase qualified path remains ordinary
named-type text unless base resolution selects its refinement interpretation.
When a uniquely recovered lowercase refinement is a named generic argument,
its complete semantic type can instantiate the enclosing refinement base for
an independent union comparison. The output order is the parse diagnostic,
the final-segment casing diagnostic, and then `type.variant_union_base` on the
later outer alternative. That union diagnostic names the fully instantiated
outer bases in its closed details and relates the earliest outer base. It does
not add `type.invalid_annotation` solely because the recovered inner final was
lowercase. A missing, ambiguous, mixed-base, or otherwise incomplete nested
refinement does not instantiate an enclosing base for this comparison.
The parse diagnostic retains the annotation's declaration context. A recovered
test result uses `parser_context: "test_return"`; an ordinary function result
uses `parser_context: "function_return"`. The checked
[`immediate test-result`](../../examples/specification/check/adt-variant-refinement-test-return-immediate-recovery-json/)
and
[`delayed test-result`](../../examples/specification/check/adt-variant-refinement-test-return-delayed-recovery-json/)
outputs fix that distinction. The checked
combined-casing
[`human`](../../examples/specification/check/adt-variant-refinement-combined-case-recovery-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-combined-case-recovery-json/)
outputs fix parse-first ordering, both casing failures, retained constructor
identity, visibility, related context, and closed detail objects. The checked
[`human`](../../examples/specification/check/adt-variant-refinement-lowercase-final-recovery-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-lowercase-final-recovery-json/)
outputs fix the ordering, final-segment spans, retained identity, suppression,
closed existing detail objects, mixed exact and recovered union alternatives,
and a recovered generic non-ADT base with canonical declaration provenance.
The nested generic argument
[`human`](../../examples/specification/check/adt-variant-refinement-nested-lowercase-recovery-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-nested-lowercase-recovery-json/)
outputs fix the outer-alternative span, expected-base related span, diagnostic
order, suppression, and fully instantiated closed union-base details.
The focused ambiguous-repair
[`human`](../../examples/specification/check/adt-variant-refinement-lowercase-final-ambiguous-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-lowercase-final-ambiguous-json/)
outputs verify that a non-unique constructor repair retains only the base
identity for later union comparison.

An unresolved refinement base, with or without type arguments, uses
`name.unresolved` instead of `type.invalid_annotation`. Its primary span is the
written base path and its ordinary unresolved-name details identify the `type`
namespace. The alternative does not establish or participate in union-base
comparison, and its final segment does not receive constructor casing, lookup,
privacy, or assignability diagnostics. A declaration type parameter is instead
a known zero-arity identity: applying type arguments preserves that identity
and reports the ordinary invalid-annotation arity failure before suppressing
constructor-dependent checks. For a lowercase final, the recoverable parse
diagnostic remains first. The checked unresolved-base
[`human`](../../examples/specification/check/adt-variant-refinement-unresolved-generic-base-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-unresolved-generic-base-json/)
outputs fix the order, half-open spans, suppression, closed detail objects, and
the declaration-type-parameter arity boundary for lowercase and uppercase
finals.

A structurally valid refinement whose base resolves uniquely but cannot expose
a finite ADT variant set reports `type.variant_refinement_base`. The primary
span is the written base path and excludes its type arguments, `::`, and final
variant segment. Its closed details contain only `written_type` and `reason`.
`written_type` preserves the source spelling of the base and recursively
reconstructs its type arguments, including refinements nested in generic,
record, and function positions.

| `reason` | Failed fact | Related context |
| --- | --- | --- |
| `not_adt` | The resolved named type has no finite ADT variants. | A source-backed type uses `base_type_declaration` with the canonical target's `resolved_identity` and declaration span, including when the written base is a public alias or alias chain. A source-less type published by the type-syntax provider uses spanless `base_type_origin` with `resolved_identity` and `provider: "type_syntax"`. |
| `opaque` | The resolved compiler-known type intentionally hides its variants. | A spanless `base_type_origin` contains `resolved_identity` and `provider: "adt"`. |

When a final segment is unknown, resolves to a non-constructor, or names a
constructor owned by another ADT, the checker reports
`type.variant_refinement_unknown` at that final segment. This includes an
unknown name, a name that resolves as a non-constructor, and a constructor
owned by another ADT. Its closed details contain exactly `base_type` and
`variant`. Related entries identify the resolved ADT declaration and each
variant visible from the annotation, in declaration order. The first entry has
`kind: "base_type_declaration"` and contains `message`, `resolved_identity`,
and the declaration `span` when one exists. Each following entry has
`kind: "visible_variant_declaration"` and contains `message`, `variant`, the
same `resolved_identity`, and the variant declaration `span` when one exists.
All variants are visible within their declaring module. An ordinary foreign
module sees only public variants. An exact test companion with the required
explicit target import also sees the target module's private variants. A
source-less ADT uses the same entries without source spans when no declaration
locations exist.

The failed final segment retains the independently resolved base identity for
union-base comparison and declaration quarantine, whether the unknown final is
the first, middle, or last union alternative. Each repeated unknown final
reports at its own segment. Same-base alternatives before or after it do not
add `type.variant_union_base`; a later independently instantiated different
base still does. The failed final does not select a constructor. The checker
therefore suppresses privacy, assignability, and other failures that require a
selected constructor. A nested type argument that remains unresolved or
incomplete suppresses an enclosing unknown-final failure that would require
that argument to instantiate the base. A uniquely casing-recovered base keeps
the independent `name.invalid_case` before this diagnostic. A lowercase final
whose initial-only repair is ambiguous does not add this lookup diagnostic
because no constructor identity was selected.
The checked
[`human`](../../examples/specification/check/adt-variant-refinement-unknown-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-unknown-json/)
cases fix the message, final-segment span, closed details, and related-entry
order. The checked exact-companion
[`human`](../../examples/specification/check/adt-variant-refinement-unknown-companion-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-unknown-companion-json/)
cases fix the target-private variant projection and its related spans.

A variant refinement anywhere in a public function parameter or result, a
public module value annotation, or a payload of a public variant of a public
source ADT reports
`type.variant_refinement_private` when its uniquely resolved terminal ADT is
private or a selected variant is private. The written base can name the ADT
directly or resolve through a finite local or imported type-alias chain. The
check traverses named type arguments, function parameters and results, record
fields, and refinement unions. At a root annotation, a direct private base has a primary span on
the final identifier of the written base type and excludes preceding module
qualifiers. For example, `main::Secret::One` selects `Secret`, not
`main::Secret`. For a private variant of a public base, the primary span selects
the final variant segment. A record-field failure selects the complete written
refinement or refinement union at that field. The closed details contain
exactly `written_type`,
`base_type`, `variant`, `boundary`, and `exposure_path`. `boundary` is
`public_parameter`, `public_result`, `public_module_value`, or
`public_adt_payload`.
`exposure_path` starts with the boundary object. A parameter path contains
`kind: "parameter"` and its `name`; a result path contains `kind: "result"`.
A module-value path contains `kind: "module_value"` and its `name`.
Each traversed field then contributes `kind: "record_field"` and its `name`.
Named arguments and function positions contribute `type_argument` with a
zero-based `index`, `function_parameter` with a zero-based `index`, or
`function_result`. A refinement union contributes `union_alternative` with a
zero-based `index`. An ADT payload path starts with `kind: "adt_payload"` and
its `type`, `variant`, and `field`. Separate structural positions therefore
retain distinct deterministic paths.

For an alias-qualified failure at a root annotation, the primary span selects
the outermost alias identifier. At a record field, the complete refinement or
refinement-union span remains primary. For nested named type arguments and
function positions, the primary span selects the outer written structural type
occurrence. In every position, `written_type` retains the source-written
refinement. Related entries identify each `type_alias_declaration` from
outermost to innermost, then the
`private_type_declaration` or `private_variant_declaration`, and then the
`public_parameter_boundary`, `public_result_boundary`,
`public_module_value_boundary`, or `public_adt_payload_boundary`. A private
declaration entry uses the declaration
span from the selected ADT descriptor, including its generic arity. Another
same-module ADT with the same type and variant names but a different arity does
not supply that span. A union reports distinct inaccessible exposure paths in
written order. It removes paths with the same resolved base, canonical generic
arguments, variant, and alias declaration chain, and retains the first written
path. A direct path and each distinct alias chain remain separate. Canonical
generic arguments recursively use resolved type identities. Canonical identity
is used only when every nested type component resolves. Otherwise,
deduplication uses the complete written alternative, so distinct unresolved
arguments remain distinct while an exact written repeat retains its first
occurrence. The complete written alternative is reconstructed independently
of any bounded path display used for lookup. The declaration entry remains
present when the ADT comes from a reusable or base environment. A public alias
from an imported or reusable environment does not replace the terminal
declaration's visibility, identity, or span; its alias entry precedes the
private terminal entry under the same ordering rule.
If initial-only casing recovery uniquely selects the base ADT and the written
uppercase final segment resolves exactly, `name.invalid_case` precedes the
independent privacy diagnostic. The diagnostic's `written_type` retains the
source spelling. Ambiguous or unresolved base recovery does not produce a
privacy diagnostic. A type path nested in a root alternative's generic
arguments is checked as a distinct structural path rather than as another
direct root.
An exact final name shared by multiple variant declarations in the selected
ADT is also ambiguous for this privacy check. That alternative produces no
privacy diagnostic and does not by itself quarantine the public function; the
duplicate declaration retains its independent `name.duplicate` diagnostic.
Another union alternative with a unique private identity still produces its
privacy diagnostic and quarantines the function.
If the base has invalid generic arity but its ADT and final constructor
identities are unique, the existing arity diagnostic precedes the independent
privacy diagnostic. Union comparison and assignability diagnostics that
require an instantiated base remain suppressed.
An invalid annotation in a sibling named-type argument, function parameter or
result, nested type argument, or sibling alternative of the same root union
also precedes but does not suppress an independently provable privacy
diagnostic. Structural traversal retains this recovery. An effect row
does not stop traversal of a later structural child. An invalid square-bracket type position reports
`type.invalid_annotation` and does not make a record nested inside the brackets
a visibility path. A later direct record field still reports its independently
provable privacy diagnostic with its own `exposure_path`. A named type in a
field that does not form refinement syntax can share a source-text prefix with
a later refinement, but the later refinement keeps its own primary span,
`written_type`, and record-field `exposure_path`. Cyclic or ambiguous aliases
do not manufacture a visibility identity. Distinct written alias paths to one
terminal variant produce distinct diagnostics, while an exact repeated path
retains its first occurrence. Exact test-companion access does not change
public-signature eligibility.

Public-ADT publication quarantine is transitive over every resolved named ADT
dependency in a public payload. This includes ordinary named types, refinement
bases, named type arguments, and dependencies reached through transparent
aliases. If a dependent payload itself refines a quarantined ADT, it reports
`type.variant_refinement_private` at that written refinement. An ordinary named
reference extends the quarantine without producing another privacy diagnostic.
The closure is independent of declaration order, terminates for recursive type
graphs, and leaves an unrelated valid public ADT published. A public function
whose parameter or result contains an ADT in the closure is unpublished without
an additional privacy diagnostic at the ordinary reference. Existing
diagnostics remain in source-span order. The checked
[`human`](../../examples/specification/check/adt-variant-refinement-public-visibility-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-public-visibility-json/)
cases fix the human messages, half-open spans, details, related context, and
ordering. The record-field
[`human`](../../examples/specification/check/adt-variant-refinement-record-visibility-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-record-visibility-json/)
cases fix nested paths, union ordering, alias notes, and accepted structural
boundaries. The structural visibility
[`human`](../../examples/specification/check/adt-variant-refinement-structural-visibility-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-structural-visibility-json/)
cases fix named arguments, function positions, ADT payloads, union path
segments, and outer structural spans. Invalid-arity overlap in function and
named-argument positions is fixed by the focused
[`JSON overlap case`](../../examples/specification/check/adt-variant-refinement-structural-visibility-invalid-arity-overlap/).
Invalid-arity recovery at a direct union root retains each written alternative
index, as fixed by the focused
[`root-union JSON case`](../../examples/specification/check/adt-variant-refinement-invalid-arity-root-union-json/).
The publication-closure
[`human`](../../examples/specification/check/adt-variant-refinement-publication-closure-human/)
case fixes a single causal diagnostic and failed command output. Focused
semantic tests in
`crates/veln-sema/src/tests/variant_refinement_annotations_and_private_results/public_visibility.rs`
and its `publication_and_recovery.rs` support verify that the ordinary payload,
alias, parameter, and result dependencies remain unpublished. The alias-qualified
[`human`](../../examples/specification/check/adt-variant-refinement-alias-visibility-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-alias-visibility-json/)
cases fix alias primary spans, source-written details, and related-note order
for parameter and result roots.
The generic-union visibility
[`human`](../../examples/specification/check/adt-variant-refinement-generic-union-visibility-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-generic-union-visibility-json/)
cases verify complete alternative identity, exact duplicate removal, distinct
unresolved arguments, and nested unresolved arguments.
The long-alternative
[`human`](../../examples/specification/check/adt-variant-refinement-long-visibility-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-long-visibility-json/)
cases verify complete presentation beyond the bounded lookup display.
A failed base retains its resolved identity as semantic analysis state for
suppression and later analysis. This diagnostic recovery does not make the
invalid refinement selectable by language-service protocols. A source-less
failed base likewise retains semantic identity without a manufactured source
target. The failed occurrence does not emit variant, union,
visibility, or assignability diagnostics that require a refinable base.
When a qualified source type and a compiler-known type have the same leaf
name, the source module identity selects the source declaration. The base
diagnostic then uses that source identity and its declaration-related context;
the compiler-known descriptor does not make the qualified source path opaque
or ambiguous.
The checker continues with sibling alternatives and nested type positions. It
retains a name, constructor, or generic-arity diagnostic when that fact is
provable without treating the failed base as refinable. A failed refinement in
one record field, generic argument, or other structural child does not stop
that traversal. A failure in one generic argument does not suppress an
independently complete union-base comparison in a later sibling argument. An
exact or uniquely casing-recovered type application in a separate sibling
position still reports its independently provable generic-arity failure.
Within one refinement union, the earliest independently instantiated base
identity and canonical generic arguments establish the expected base. Each
later independently instantiated base is compared with it in written order. A
different ADT identity or different canonical generic arguments report
`type.variant_union_base` at the complete later alternative. An unknown final
segment does not prevent its independently resolved base from participating in
that comparison. An alternative without a comparable base does not establish
or clear the expected base. In particular, every nested named type must resolve
uniquely before the enclosing base can participate. An unresolved or ambiguous
nested name therefore does not establish the expected base. An in-scope
declaration type parameter is resolved in that declaration context and remains
eligible for comparison.
It remains distinct from a same-spelled nominal type. The type parameter keeps
its source spelling in diagnostic details, while the nominal type uses its
stable qualified identity when that qualification is needed to expose the
conflict. A compiler-known type and a source-declared nominal type with the
same leaf spelling also retain different identities. Diagnostic details keep
the compiler-known spelling concise and qualify the source type to expose the
conflict. Adding or renaming a source declaration whose name does not occur in
the union annotation leaves the expected-base selection and reported conflicts
unchanged. Its closed details contain exactly
`expected_base_type` and `actual_base_type`. The expected type is the earliest
independently instantiated base. The actual type is the base of the conflicting
alternative. These values use concise canonical names
when those names distinguish the bases. When equal concise names would hide a
different nominal identity, the conflicting position uses its stable qualified
identity. Each rendered base retains its complete generic argument list when
the conflicting base constructors have different arities. Nominal-identity
disambiguation recurses through variant refinements, record fields, and
function parameters and results inside an invariant generic argument. Matching
unit positions in that output use the source spelling `()`.
Record arguments compare fields by name rather than written field order, and
nested refinement arguments compare their selected variant sets rather than
written alternative order. These reorderings do not report a union-base
failure. If same-named record fields instead contain conflicting nominal
identities, `expected_base_type` and `actual_base_type` preserve the field order
written in their respective alternatives while qualifying the conflicting
identities.
One
`expected_variant_union_base` related
entry points to that earliest alternative and explains that it established the
union base. Each later conflict reports separately. An alternative that cannot
be instantiated does not participate in the comparison and does not suppress
an independently complete later pair. Its independently provable generic-arity
failure remains before the later union-base failure in written-alternative
order. A mixed-base union makes its annotation invalid. When it occurs in a
function, effect operation, handler, or ADT
constructor signature, every type exposed by that declaration is unknown to
its body and consumers. These boundaries suppress only diagnostics derived
from the rejected signature. Other independently provable declaration
diagnostics also remain.
When a nested type argument remains unresolved or incomplete, the checker still
reports an enclosing base-eligibility failure that can be proved from the
enclosing base identity alone. It suppresses an enclosing annotation or
final-segment failure that would require the incomplete child to form a valid
type.
The checker visits each written type-path occurrence once during recovery.
Repeated traversal of one source span does not duplicate its diagnostic, while
equal path text at distinct spans remains distinct and can report once at each
span.
When separate union alternatives independently produce failures, their
base diagnostics follow the alternatives' written order. Within each
alternative, the checker reports casing before base resolution, generic arity,
base eligibility, final-segment resolution, and visibility. Casing applies to
the complete alternative, including qualified type arguments. A base failure
from an earlier alternative therefore precedes a nested casing failure from a
later alternative, and that casing failure precedes the later alternative's
base failure. Diagnostics from unrelated checker families retain their
established relative order and can remain between the casing and base failures
of one alternative. The checked human and JSON cases below preserve the full
sequence around unrelated public-signature and casing diagnostics as well as
refinement failures.
When one alternative has an unknown final segment and its retained base also
differs from the union base, `type.variant_refinement_unknown` precedes
`type.variant_union_base`. The union-base diagnostic still uses the complete
alternative as its primary span. An unknown final with the established base
does not interrupt comparison of a later alternative. The checked
unknown-refinement human and JSON cases above preserve these recovery outcomes,
the diagnostic order, and both spans.
An independently provable generic-arity failure keeps
`type.invalid_annotation` instead of adding the base diagnostic. This applies
to exact and uniquely casing-recovered source-backed or source-less base types;
the invalid-annotation span covers the complete structural refinement spelling.
Unresolved, ambiguous, private, and wrong-kind bases also retain their existing
diagnostics.
The checker applies this contract to function and test parameters and results,
local annotations, ADT payloads, effect-operation parameters and results,
handler parameters, schema fields, and explicit call type arguments. A failure
in a declaration annotation prevents publication of usable typed state. An
invalid parameter supplies no expected argument types to callers and is unknown
inside its declaration body. An invalid result supplies no return expectation
to its body and no result type to callers. These quarantined types do not cause
derivative assignability diagnostics. The declaration-level rule applies to an
ordinary function, one effect operation, one handler, or one ADT constructor:
if any parameter, result, or payload annotation in that declaration has a base
failure, every parameter, result, or payload type exposed by that declaration is
unknown. Declaration identity and independently provable declaration
diagnostics remain available. A schema with such a field does not expose a
generated helper through the existing schema eligibility boundary. Invalid-name
function-value recovery and function aliases expose this same quarantined
callable signature; they do not restore sibling parameter or result types.
The same quarantine runs before private result inference when the source-backed
base declaration comes from the reusable standard environment. Its related
note retains the standard-library declaration span. An omitted result that
calls the invalid declaration does not inherit that declaration's written
result type. If no independent fact establishes a result, inference keeps it
unknown instead of producing a derivative mismatch in a later consumer.

Lowercase prospective bases without type arguments remain ordinary named-type
source text. Type arguments before the final `::Variant` make the refinement
structure explicit, so a lowercase generic base leaf is preserved for semantic
recovery. If changing only that preserved base leaf to the required case
identifies exactly one type, the checker reports `name.invalid_case`, retains
that identity, and performs the independent eligibility check on carriers
classified for casing analysis.
Explicit expression type arguments remain casing-silent, but their base
eligibility is still checked independently. A source type declaration with no
variants, a source-less non-ADT type, or a compiler-known opaque type therefore
also reports `type.variant_refinement_base`; a refinable ADT continues to
final-segment checking without an eligibility failure. If recovery does not
identify exactly one base, the checker does not manufacture an eligibility
diagnostic or retained identity. When casing and eligibility use the same base
span, the casing diagnostic precedes the eligibility diagnostic.
The checked
[`human`](../../examples/specification/check/adt-variant-refinement-base-diagnostics-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-base-diagnostics-json/)
cases cover source-backed, source-less, and opaque bases in declaration
annotations and explicit call arguments, related provenance, recovered-casing
overlap including a correct-arity generic base, occurrence-scoped downstream
suppression, declaration quarantine, and arity precedence.
The reusable-base and union-overlap
[`human`](../../examples/specification/check/adt-variant-refinement-base-overlap-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-base-overlap-json/)
cases cover standard-environment declaration provenance, quarantine before
omitted-result inference, and a failed union alternative before, between, and
after two successfully instantiated alternatives.
The focused union-base
[`human`](../../examples/specification/check/adt-variant-union-base-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-union-base-json/)
cases fix later-alternative spans, closed base details, earliest-alternative
context, repeated conflicts, recursively nested nominal-identity display,
invariant generic arguments, declaration-type-parameter and nominal-type name
collisions, compiler-known and source-nominal name collisions, isolation from
unrelated source declaration names, complete base rendering across different
generic arities, and arity recovery before an independent later conflict.
The focused
[`reordered JSON`](../../examples/specification/check/adt-variant-union-base-reordered-json/)
case fixes order-independent record and nested-refinement comparison plus
per-alternative record-field order in conflict details.
The checked nested-recovery
[`human`](../../examples/specification/check/adt-variant-refinement-nested-recovery-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-nested-recovery-json/)
cases cover independent enclosing base failures, nested generic-arity failures,
suppression of failures derived from an unresolved or incomplete child type,
one diagnostic per
written path occurrence, and independent diagnostics for equal paths at
distinct spans.
The qualified source-collision
[`human`](../../examples/specification/check/adt-variant-refinement-qualified-source-collision-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-qualified-source-collision-json/)
cases verify that qualified source declarations win over same-spelled
compiler-known type identities and retain source declaration provenance.
The checked nested-casing
[`human`](../../examples/specification/check/adt-variant-refinement-nested-casing-diagnostics-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-nested-casing-diagnostics-json/)
cases fix the per-alternative precedence when a non-ADT generic base contains a
uniquely recovered invalid-cased qualified type argument.
The checked declaration-quarantine
[`human`](../../examples/specification/check/adt-variant-refinement-declaration-quarantine-human/)
and
[`JSON`](../../examples/specification/check/adt-variant-refinement-declaration-quarantine-json/)
cases cover ordinary functions, effect-operation, handler, ADT-constructor, and
schema carriers, including retained independent diagnostics and absent
sibling-derived mismatches. For ordinary functions, invalid parameter or result
annotations quarantine sibling parameter types, return expectations, and named
result bindings used by contracts. Recovered invalid-name function values use
the same quarantined parameters and result. The cases reject derivative
`type.mismatch` and `contract.type_mismatch` diagnostics. The constructor coverage uses direct,
one-hop alias, and transitive-alias views of the same quarantined variant; every
view suppresses the derivative payload mismatch.

A value whose complete ADT variant set is not assignable at a direct local,
argument, branch, arm, result, or generated schema encode boundary reports
`type.variant_mismatch` at that value expression. The primary message states
the actual and expected types. The diagnostic renders the two sides
independently: an unambiguous
explicit alias spelling is preserved only on the side that supplied it, and an
expected alias never relabels an inferred actual type. Conflicting alias
provenance uses the target ADT's canonical display name, including when the
conflicting singleton or union refinements cover the complete ADT domain and
render as its base type. The same rule applies when direct and invariant nested
constructor payload contributions infer one type argument. The checked
[`alias diagnostic case`](../../examples/specification/check/adt-variant-refinement-alias-presentation-diagnostics-json/)
and
[`invariant payload diagnostic case`](../../examples/specification/check/adt-variant-refinement-invariant-alias-presentation-diagnostics-human/)
cover these presentation boundaries. Details contain `phase`, `node_id`, `actual_type`, `expected_type`,
declaration-ordered `expected_variants`, `excluded_variants`, and `constraint`.
`excluded_variants` is an object with `form` and `variants`. The `listed` form
contains the declaration-ordered excluded finite set. The
`all_except_expected` form uses an empty `variants` array and states that every
base-ADT variant outside `expected_variants` is excluded, without copying the
complete ADT declaration into each diagnostic. A `variant_exclusion` related
note renders that fact for human output. One `expected_type_origin` related
note identifies the declaration or local annotation that supplied the
expectation. When a compiler-known helper or generated schema encode helper
infers a parameter expectation from the call, the note instead identifies that
helper at the call site. A nested structural incompatibility with no truthful
finite variant exclusion reports ordinary `type.mismatch`. Alias display
differences do not create such an
invariance failure: when invariant arguments have the same resolved type, an
outer finite-set or base-to-refinement exclusion remains
`type.variant_mismatch`, with independent alias spellings in `actual_type` and
`expected_type`. A genuinely different resolved nested refinement reports
`type.mismatch`; its actual type preserves the inferred nested refinement. This
includes assigning an inferred named aggregate with a refined type argument to
the corresponding type that uses the base ADT. Record fields instead use
ordinary structural covariance, so an inferred singleton-refined field is
assignable to the corresponding base-typed field. The reverse record
assignment fails with `type.mismatch`. During aggregate inference, an
incompatible later contribution reports the aggregate position's accumulated
type as expected and preserves the later contribution's constructor refinement
as actual. The accumulated expected type contains only successfully typed
contributions. A contribution that already produced a diagnostic leaves that
type unchanged, does not produce a derivative aggregate mismatch, and cannot
change the expected type reported for a later contribution. A rejected call or
constructor expression follows this rule even when recovery can determine its
refined result type from the resolved declaration. The same rule applies when
one constructor payload uses a type parameter directly and another uses it
inside an invariant named type: the diagnostic reports the exact constraint
established by the other payload rather than widening the nested type. When one
payload contributes the same type parameter more than once, a conflict reports
the earlier occurrence's type as expected and the conflicting occurrence's
type as actual. None of that payload's occurrences update the accumulated type,
so a later payload is checked against only the constraints that were complete
before the rejected payload. If an earlier error leaves the value untyped,
`type.variant_mismatch` is omitted.

Generated schema encode boundaries additionally inspect corresponding fields
of anonymous records and corresponding arguments of `Option`, `List`, `Vec`,
`Dict`, and `Result`. When the boundary supplies the expected record type, a
list or vector literal checks each item with that schema expectation, a
dictionary literal checks each value with it, and `Some`, `Ok`, or `Err` checks
its payload with it. If every other inspected position is assignable, a base
ADT or excluded constructor at one of those nested refined positions reports
`type.variant_mismatch`; `actual_type`, `expected_type`, and the exclusion facts
describe that innermost ADT comparison. A missing record field, unrelated type,
unsupported container shape, or another non-variant mismatch makes the whole
comparison an ordinary `type.mismatch`. This recursive diagnostic selection is
specific to generated schema encode boundaries and follows a generated encode
helper through an immutable local binding. An ordinary function value with the
same visible type uses ordinary structural assignment and reports
`type.mismatch` for a nested incompatibility.
The [type assignment rules](types.md#compatibility-and-limits) own the record,
function-input, result, and effect directions. Their rejected directions and
outer rendered types are checked by the
[`complete structural assignment diagnostic case`](../../examples/specification/check/complete-type-structural-assignability-diagnostics/).
The same rules own call-bound effect-row constraints. A rejected constraint or
argument reports the ordinary `type.mismatch` for the failed compatibility
fact. The rejected call does not emit `effect.missing_public` from a tentative
row substitution, including when its rejected result is used later. An
accepted call still contributes its instantiated row to the public-boundary
diagnostic when another argument is an ordinary call, `perform` expression, or
pipeline. A pipeline into the row-polymorphic call is checked as its first
argument. An incompatible value propagated with `?`, an invalid operator
operand, or an invalid `if` condition or branch rejects the call without
publishing that tentative row. The
[`effect-row rejection case`](../../examples/specification/check/effect-row-call-rejection/)
checks those JSON outcomes for direct and nested payload failures,
local-binding and constructor-payload rejection, escaping-result recovery, and
an accepted ordinary-call argument. The
[`argument-consistency case`](../../examples/specification/check/effect-row-call-argument-consistency/)
requires accepted prefix, `perform`, and pipeline arguments to retain their row
effects through direct and private-helper boundaries. Rejected prefix, binary,
`?`, and `if` arguments emit type diagnostics without leaking tentative row
effects. The
[`constructor-record case`](../../examples/specification/check/effect-row-call-constructor-record-payload/)
requires `type.mismatch` without `effect.missing_public` from the rejected
row-bearing sibling, while its accepted control retains the concrete row. The
[`schema-argument case`](../../examples/specification/check/effect-row-call-schema-arguments/)
requires accepted `decode`, `encode`, and `validate` arguments to retain the row
diagnostics inferred through private helpers.

Format-neutral generated decode helpers retain refinements in their parameter
and successful `Result` types, but they use ordinary structural record
compatibility. Supplying a base ADT or an excluded constructor at a refined
nested position reports `type.mismatch`, as does assigning a base-typed decode
result to a refined result. These failures do not contain the
`expected_variants` or `excluded_variants` details of `type.variant_mismatch`.
The checked
[`format-neutral decode JSON case`](../../examples/specification/check/adt-variant-refinement-format-neutral-schema-decode-json/)
covers all three boundaries.

A refinement-bearing `format binary` field requires a decoder that can either
construct an admitted variant or inspect and validate the decoded ADT tag. The
generated binary decoder has neither capability for a resolved finite ADT, so
analysis reports `schema.variant_refinement_decode_unsupported`. The primary
span is the innermost unsupported refinement. For a union, that span covers the
complete union. The diagnostic does not replace an independently applicable
invalid base or unknown-variant diagnostic.

The diagnostic details object is closed and contains only `target_type`,
`base_type`, `expected_variants`, and `decoder`. `target_type` renders the
selected refinement, `base_type` renders its written ADT base,
`expected_variants` preserves annotation order, and `decoder` names the
generated decoder selected for the schema. One `decoder_provenance` related
entry identifies the selected decoder, its `generated_binary_schema`
provenance, and the `format binary` clause that selected it. The checked
[`human capability case`](../../examples/specification/check/adt-variant-refinement-decode-capability-human/)
and
[`JSON capability case`](../../examples/specification/check/adt-variant-refinement-decode-capability-json/)
cover singleton, union, nested selection, the closed details, and provenance.

A valid constructor arm for a stable match scrutinee refines the matched place
to the constructor singleton while checking the arm. A stable scrutinee is an
immutable binding written as a bare name or with redundant parentheses, or a
record-field path whose base ultimately resolves to an immutable parameter or
local binding. A binding and its transparent local aliases share one place. A
field path also shares its place with repeated access through transparent
aliases of the same resolved root. A different root or field sequence does not
share that refinement.

A transparent-alias group uses the feasible domain established by all valid
facts for its one immutable value. A wider annotation on one member does not
enlarge that domain. Valid constructor arms consume one residual variant.
Binding and `_` catch-alls give every participating reference the complete
residual set. A binding catch-all identifies the complete matched value, so a
nested match through that binding refines the original place and its aliases.
Passing any participating binding or field path to a parameter that requires
its arm refinement emits no `type.variant_mismatch`. Leaving an arm restores
the prior type of the binding or field path.

A valid same-ADT constructor excluded by the original refined domain reports
`type.match_impossible_variant` at the constructor's final name segment. Its
closed details contain only `scrutinee_type` and `arm_variant`.
`scrutinee_type` renders the alias group's effective feasible domain, including
an enclosing arm refinement, rather than a selected member's wider declared
type. Related entries identify the scrutinee occurrence and selected ADT
declaration. The checked widened-alias classifications are
[`adt-variant-refinement-match-alias-domain-diagnostics-human`](../../examples/specification/check/adt-variant-refinement-match-alias-domain-diagnostics-human/)
and
[`adt-variant-refinement-match-alias-domain-diagnostics-json`](../../examples/specification/check/adt-variant-refinement-match-alias-domain-diagnostics-json/).

A valid constructor or catch-all with no residual variant reports
`type.match_redundant_arm` at the complete arm pattern. Its closed details are
`scrutinee_type`, rendered `arm_pattern`, nullable `arm_variant`, and `reason`.
The reason is `preceding_catch_all` when any earlier valid catch-all exists,
including a catch-all that was itself redundant after constructor coverage.
The first valid catch-all takes precedence for every later redundant
constructor or catch-all. The reason is `duplicate_variant` when an earlier
constructor first covered the same variant and no valid catch-all intervened.
The reason is `complete_prior_coverage` only for a catch-all reached after
constructors completed the domain and before any earlier valid catch-all.
Ordered `preceding_arm` related entries select the first valid catch-all, first
duplicate arm, or domain-completing arms. The
checked human and JSON cases are
[`adt-variant-refinement-match-diagnostics-human`](../../examples/specification/check/adt-variant-refinement-match-diagnostics-human/)
and
[`adt-variant-refinement-match-diagnostics-json`](../../examples/specification/check/adt-variant-refinement-match-diagnostics-json/).

For stable bindings or record-field paths with a refined match domain, arm
validation precedes both classifications. An invalid-cased, unresolved,
inaccessible, wrong-ADT, wrong-generic-payload, wrong-arity, or malformed arm
retains its intrinsic diagnostic, contributes no coverage, and emits neither
derived classification. A pattern incompatible with a substituted generic
payload reports the ordinary `type.mismatch` at that payload pattern. An
unresolved constructor path reports `name.unresolved` at its final name segment
with `namespace: "constructor"`; the diagnostic symbol retains the complete
written path. If the rejected arm was the only arm for a remaining variant,
the match also reports the ordinary `type.match_non_exhaustive` diagnostic.
The invalid-casing recovery used by the ordinary exhaustiveness path when the
stable-place refined-coverage path is inactive can suppress its sole
derivative missing-case diagnostic; it does not validate the rejected arm.

On the stable-place refined-coverage path, the intrinsic wrong-arity
diagnostic is `type.constructor_pattern_arity` at the complete constructor
pattern. Its closed details contain only
`constructor`, `expected_payload_count`, and `actual_payload_count`, and it has
no related entries. The primary message states the expected and actual payload
counts. Supplied payload patterns and the arm body are still checked, but the
arm cannot satisfy exhaustiveness. The same rule applies when a nested
constructor does not resolve against its
expected payload ADT or has the wrong arity, and when a nested record pattern
names a missing or duplicate field. A literal or unit payload pattern whose
type differs from the expected constructor payload emits `type.mismatch` at
that payload pattern. Its details use
`expected_type_source: "constructor_payload"`,
`actual_type_source: "literal_pattern"`, and
`constraint: "constructor_payload_pattern"`. The invalid pattern also cannot
refine the scrutinee unless every payload binding is admitted to the arm scope.
When a stable binding or record-field path has a base-ADT domain, constructor
and catch-all arms still refine the same place and its aliases, but arm
validation and diagnostics retain the ordinary constructor path, payload-arity,
duplicate-arm, and exhaustiveness behavior. Such a match does not gain
`type.match_impossible_variant`,
`type.match_redundant_arm`, `name.unresolved`, or
`type.constructor_pattern_arity` from refined-domain diagnostics. Refined
coverage is inactive for a computed refined scrutinee, including a field access
rooted at a call, operator, or constructed value. It retains the ordinary match
diagnostics and does not refine bindings that contributed to the computed
value.
Invalid value-name casing, a duplicate payload binding,
parameter, or visible local, and a `callsite` binding that would shadow the
built-in call-site location keep their intrinsic casing, resolution, field,
arity, type, `name.duplicate`, or `name.callsite_shadow` diagnostic. An
impossible or redundant arm still checks its payload bindings, body, and
inherited expected result. Impossible arms retain the original scrutinee type;
redundant constructors use their original-domain singleton; redundant binding
catch-alls use the original scrutinee type. An incompatible use retains the
ordinary `type.variant_mismatch` behavior. The [type inference
rules](types.md#inference-rules) define the refinement scope and unsupported
scrutinee forms. The checked precedence and recovery cases are
[`adt-variant-refinement-match-precedence-human`](../../examples/specification/check/adt-variant-refinement-match-precedence-human/)
and
[`adt-variant-refinement-match-precedence-json`](../../examples/specification/check/adt-variant-refinement-match-precedence-json/).

An unannotated `if` or `match` whose result refinements can join under the
[type inference rules](types.md#inference-rules) emits no diagnostic. After a
finite refinement join has started, a resolved branch or arm result that cannot
join retains the ordinary `type.mismatch` contract against the base ADT rather
than introducing a control-flow-specific diagnostic. Recovery abandons the
partial finite join and remains at the base ADT; later compatible refinements
do not resume it. When the first typed result cannot start a finite refinement
join, later results retain the existing first-result expectation, mismatch,
and recovery behavior.

Source identifier casing uses `name.invalid_case` with `phase`, `origin`,
`occurrence`, `name`, `name_class`, `required_initial`, and
`observed_initial`. Qualified written paths add zero-based `segment_index`.
If a qualified constructor pattern has an invalid-cased module or type segment,
that pattern does not establish constructor coverage through final-segment
recovery. The match retains an independently provable
`type.match_non_exhaustive` diagnostic. The checked
[`human`](../../examples/specification/check/identifier-casing-qualified-use-paths-human/)
and
[`JSON`](../../examples/specification/check/identifier-casing-qualified-use-paths-json/)
cases cover invalid module, type, and constructor segments.
Source-derived module paths use `origin: "source_path"`,
`occurrence: "path_segment"`, `source_path`, `source_kind`, `segment`, and
`segment_index`; `source_kind` is `regular`, `export`, `companion`,
`doctest`, or `generated`. Selected regular and companion sources may report
casing diagnostics alongside parse errors. A lowercase initial followed by an
invalid module-identifier character uses
`module.invalid_source_path`.
When a selected documentation source cannot form a canonical virtual doctest
path, the same diagnostic reports the original documentation source. That
origin produces no generated doctest diagnostic path. Analysis continues for
the other selected sources, and `veln check --json` reports each rejected
origin once alongside their diagnostics. The record has kind `module`, the
original source span, and details containing
`phase: "module"`, `field: "module_identity"`, `source_path`, and the rejected
`segment`.
Recovery diagnostics do not create cascaded unresolved-name records.

An invalid explicit import alias uses `origin: source`, `occurrence:
declaration`, `name_class: module`, and `required_initial: ascii_lowercase`.
Its primary span is the alias token. An invalid import-target segment instead
uses `occurrence: path_segment`, includes its zero-based `segment_index`, and
spans that target token. A reserved `prelude` alias also uses the alias token.
A duplicate qualifier involving an explicit alias uses the later
explicit-alias token or later implicit-leaf token, according to which import is
reported; first-binding context appears as a related note. A duplicate target
import reports the later import's written target path even when package loading
has normalized different source spellings to the same package and module
identity. The first import's target path appears as related context. Equal
written paths in distinct package origins do not produce this duplicate-target
diagnostic. If initial-only casing correction gives an invalid target the same
module identity as another import target, the later import reports that same
duplicate-target shape and the earlier target remains its related context.
This ordering follows source order even when the invalid spelling is later;
the independent `name.invalid_case` record is retained.
Parser diagnostics precede casing diagnostics for one import; reserved and
duplicate facts follow casing, while target-resolution facts stay independent.
When duplicate targets or colliding qualifiers quarantine every import for a
qualified module-value path, the duplicate diagnostics remain and the read
reports `name.unresolved`. It does not report `name.ambiguous`, because a
quarantined import is not a lookup candidate. The checked
[`human`](../../examples/specification/check/module-value-quarantined-qualified-human/)
and
[`JSON`](../../examples/specification/check/module-value-quarantined-qualified-json/)
cases fix this boundary for an implicit qualifier and an explicit alias.
When eligible imports from distinct package origins have the same exact
written path, a qualified module-value read instead reports `name.ambiguous`.
The checker selects no target module before member lookup, so member presence,
visibility, or declaration availability cannot change the diagnostic to
`name.unresolved` or select one declaration. The result is independent of
import order. The checked
[`human`](../../examples/specification/check/module-value-ambiguous-exact-member-human/)
and
[`JSON`](../../examples/specification/check/module-value-ambiguous-exact-member-json/)
cases cover public, private, and unavailable members in both orders.
If the quarantined exact path equals the importing module's own spelling, the
same duplicate and unresolved diagnostics apply. The checker does not resolve
the qualified read to the same-module value, and it does not add
`name.ambiguous` or `type.mismatch`. The checked
[`human`](../../examples/specification/check/module-value-quarantined-exact-same-module-human/)
and
[`JSON`](../../examples/specification/check/module-value-quarantined-exact-same-module-json/)
cases cover both import orders.
When correcting an invalid target
segment or alias uniquely selects one public export in the import's target
package, the casing diagnostic remains and the recovered member does not add
`name.unresolved`. The importing source's owner package selects the import;
an equal module and declaration spelling owned by another package cannot
satisfy the recovery. This applies when the member path uses either the invalid
explicit alias or the quarantined import's full written target path. For a
public ADT or constructor,
independently provable `core.constructor_arity_mismatch` and
`type.match_non_exhaustive` records remain visible. Combined module sources
retain stable source and token order. The checked target-casing
[`human`](../../examples/specification/check/explicit-import-alias-target-casing-human/)
and
[`JSON`](../../examples/specification/check/explicit-import-alias-target-casing-json/)
cases fix the target token span, casing fields, and unresolved-member
suppression. The checked
[`explicit-import-alias-combined-order-human`](../../examples/specification/check/explicit-import-alias-combined-order-human/)
and
[`explicit-import-alias-combined-order-json`](../../examples/specification/check/explicit-import-alias-combined-order-json/)
cases fix the complete observable order in text and JSON output.
The checked
[`explicit-import-alias-independent-carrier-failures-human`](../../examples/specification/check/explicit-import-alias-independent-carrier-failures-human/)
and
[`explicit-import-alias-independent-carrier-failures-json`](../../examples/specification/check/explicit-import-alias-independent-carrier-failures-json/)
cases fix the independent constructor and match records.

Companion diagnostics expose `details.companion_path` and
`details.target_path` where applicable. Private companion effect and handler
wrong-target failures use `effect.private_companion_target` or
`handler.private_companion_target`; their details include companion and target
module fields plus `reason: "companion_target_mismatch"`. Effect failures
expose `companion_path`, `companion_target_module`, and `effect_module`; handler
failures expose `companion_path`, `companion_target_module`, and
`handler_module`. Public declarations use `module.companion_public_declaration`.
Exporting a test companion uses
`manifest.invalid_export` with `field: "lib.exports"`,
`reason: "test_companion"`, `source_path`, and `companion_path`.

Source-less compiler lookup failures are spanless
`toolchain.invalid_symbol_case` diagnostics with kind `toolchain` and details
`provider`, `name`, `name_class`, and `required_initial`. Invalid or duplicate
lookup keys and standard-symbol namespace mismatches use the same shape. They
remain toolchain failures rather than source `name.invalid_case`; human
messages identify invalid or duplicate lookup keys when applicable.

Schema diagnostics cover parse rejection, primitive kind checks, field
references, validation predicates, dispatch payload eligibility, explicit
schema operation path resolution, and generated helper availability. Mapping
clauses are rejected by the parser and therefore have no current schema
mapping diagnostic. If `format binary` appears after a schema validation
clause, that clause reports `parse.schema_validation_before_format`; a
format-neutral schema without a format clause may use the same validation
position.

Static failures for `validate Schema from value at offset` use
`schema.validate_expression`. The details include `phase: "body_analysis"`,
`node_id`, `schema_path`, `operation: "runtime_validate"`, and a stable
`reason` such as `unresolved_schema`, `wrong_kind`, `ambiguous_target`, or
`private_schema`; a resolved wrong-kind diagnostic also supplies
`resolved_kind`. The primary span covers the written schema path without the
`validate` keyword or either operand. `unresolved_schema` also reports an
ineligible reachable composition graph, including any binary schema or a
recursive source ADT that returns to an
active declaration with different instantiated arguments. A missing path in
`validate from value at offset` instead uses
`parse.schema_validate_expression` with `schema path` as its expected input.
The checked
[`schema-runtime-validation-targets-human`](../../examples/specification/check/schema-runtime-validation-targets-human/)
and
[`schema-runtime-validation-targets-json`](../../examples/specification/check/schema-runtime-validation-targets-json/)
cases fix the target spans, messages, stable reasons, and JSON fields. The
rejected
[`schema-runtime-validation-missing-path`](source-surface-fixtures/rejected/schema-runtime-validation-missing-path.veln)
fixture fixes the missing-path parse boundary. Runtime constructor rejection
uses `schema.variant_refinement_mismatch`, and a negative offset uses
`schema.validation_offset_invalid`. Their byte-diagnostic projection preserves
the exact `ByteOffset`, `reason`, and decoded JSON Pointer `field_path`. The
root segment has kind `schema` and the terminal declaration name; following
segments have kind `field`, including each containing composed-schema or
anonymous-record field, decimal list or vector index, and decoded dictionary
key before the rejected position. Source-ADT traversal adds the actual
constructor name followed by the declared payload name or zero-based
positional payload index as `field` segments. Direct and mutual recursion adds
those segments for every visited value occurrence, including repeated uses of
the same validation shape. The checked
[`schema-runtime-recursive-source-adt-validation`](../../examples/specification/run/schema-runtime-recursive-source-adt-validation/)
case fixes the resulting deep recursive path. The retained JSON Pointer display
escapes `~` as `~0` and `/` as `~1`. The checked
[`schema-runtime-refinement-validation-containers`](../../examples/specification/run/schema-runtime-refinement-validation-containers/)
case fixes collection indices, deterministic dictionary ordering, and escaped
dictionary keys. The checked
[`schema-runtime-refinement-validation-container-json`](../../examples/specification/run/schema-runtime-refinement-validation-container-json/)
and
[`schema-runtime-refinement-validation-container-human`](../../examples/specification/run/schema-runtime-refinement-validation-container-human/)
cases fix their structured and human diagnostic projections. Human rendering
identifies the failed fact at the offset and places the path and mismatch or
offset reason in notes.
Runtime predicate rejection uses `schema.validation_failed` at the supplied
offset. Its decoded JSON Pointer selects the containing field for a field
predicate and the current schema root for a schema-level predicate. The checked
[`schema-runtime-refinement-predicate-order`](../../examples/specification/run/schema-runtime-refinement-predicate-order/)
case fixes predicate precedence, paths, and reasons.
The checked
[`schema-runtime-source-adt-validation-json`](../../examples/specification/run/schema-runtime-source-adt-validation-json/)
and
[`schema-runtime-source-adt-validation-human`](../../examples/specification/run/schema-runtime-source-adt-validation-human/)
cases fix positional and named source-ADT payload projection.

Type inference diagnostics include:

- `type.local_inference_incomplete`: `slot_kind: "local_binding"`,
  `binding`, and the current `inferred_type`, including `unknown` when present.
- `type.private_inference_incomplete`: `boundary: "private_function"`,
  `slot_kind: "private_parameter"` with `parameter`, or
  `slot_kind: "private_return"`, plus `missing_fact` and `inferred_type`.
- `type.inference_ambiguous`: `slot_kind` is `constructor_type`,
  `empty_collection`, or `match_scrutinee`. Constructor ambiguity adds
  `constructor`, `inferred_type`, and `constraint: "constructor_type_context"`;
  empty collection ambiguity adds `collection`, `inferred_type`, and
  `constraint: "empty_collection_type_context"`; match scrutinee ambiguity
  adds `candidates` and `constraint: "match_constructor_pattern_domain"`.

Deferred-block restriction diagnostics use kind `type`,
`phase: "type_check"`, `boundary: "deferred_block"`, and one of these stable
identifier and reason pairs:

| Identifier | Reason | Failed fact |
| --- | --- | --- |
| `defer.propagation` | `result_propagation` | Postfix `?` occurs inside a deferred block. |
| `defer.non_unit` | `non_unit_result` | The deferred block result is not `()`. |
| `defer.nested` | `nested_defer` | A deferred block registers another deferred block. |

Each record has one `related` entry with `kind: "repair_hint"`, a repair
message, and the containing deferred-block span. The primary span remains the
specific propagation `?` token, non-unit block result, or nested `defer` keyword that
failed.

Call-site execution-gate diagnostics use kind `type` and
`phase: "core_lowering"`:

| Identifier | Stable details | Failed fact |
| --- | --- | --- |
| `core.callsite_entry_unsupported` | `entry`, `boundary: "run_entry"` | The selected run entry declares the `callsite` modifier, but no Veln call expression can supply its hidden location. |
| `core.callsite_contract_call_unsupported` | `node_id`, `reason: "callsite_contract_call_unsupported"`, `callee` | A runtime contract on an ordinary function calls a call-site-aware function without enclosing hidden context to forward. |

The entry and contract-call records include a related
`runtime_support` note that states the unavailable runtime boundary. These
diagnostics stop `veln run` before backend launch. Direct and indirect calls in
ordinary function bodies, forwarding through call-site-aware wrappers, passing
call-site-aware callbacks to runtime-backed consumers, and direct built-in
references or call-site-aware direct calls in runtime contracts on
call-site-aware functions do not produce these diagnostics. The
ordinary-function contract diagnostic represents a deliberate boundary: its
enclosing function has no hidden call-site context to forward.

A schema declaration used as an ordinary local annotation type reports
`type.schema_reference` with `schema` and `use_kind: "local_annotation"`.
Exact-width and lowercase schema primitives in the same position report
`schema.exact_width_primitive` or `schema.lowercase_primitive` with `primitive`
and `reason: "local_annotation"`. These producer rules apply inside nested
`begin` and `defer` bodies and inside a `begin` used as a handler operation
clause.

Handler effect diagnostics use `phase: "effect"`, `boundary`, `handler`,
`handled_effect`, nullable `operation`, and `reason`. Operation-clause
diagnostics use `boundary: "handler_operation_clause"` and do not emit a
`provider`. Unknown handled effects use
`reason: "unknown_handled_effect"` and related notes containing candidate
`effect` and `operations` declarations.

Advisory hole diagnostic fields, candidate eligibility, type-match ranking,
and `satisfy` behavior are specified by [holes.md](holes.md). Candidate
application-policy fields are specified by
[repair-candidates.md](repair-candidates.md). Runtime result projections are
specified by [run-json.md](run-json.md).

## Limits

The envelope reports the diagnostics produced for the selected analysis set;
it does not include diagnostics from unselected sources or unloaded
dependencies. A static diagnostic envelope has no captured program stdout or
stderr fields. Human rendering and JSON serialization consume the same ordered
diagnostic records. JSON preserves their structured fields, while human stderr
renders the corresponding primary messages and related notes. Diagnostic
details are extensible by family, so consumers must tolerate omitted and newly
added detail keys.

The envelope implementation is `crates/veln-diagnostics/src/envelope.rs`;
family producers and focused CLI assertions provide the executable contract.
