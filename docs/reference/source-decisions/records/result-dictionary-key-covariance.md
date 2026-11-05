---
role: reference
authority: supporting
update-when: Compiler-owned Dict key variance, dictionary lookup equality, persistent dictionary updates, or widened-dictionary companion coverage changes.
---

# Dictionary Key Covariance

## Decision

Keep the key parameter of the compiler-owned immutable `Dict<K, V>` constructor
covariant at assignment boundaries, as specified by the current
[assignment compatibility rules](../../../specification/types.md#compatibility-and-limits).
In particular, a dictionary with a refined key type may be assigned to a
concrete expected dictionary whose key type is a wider type.

Compiler-owned variance metadata records both `Dict` parameters as covariant.
Source-ADT variance analysis composes through that checked metadata
when a payload contains `Dict`. This metadata agrees with the established
assignment rule; it does not broaden context-free generic inference or change
dictionary runtime behavior.

This decision depends on the lookup and update properties below. A backend or
container design that does not preserve those properties must not inherit this
classification merely because it exposes dictionary-shaped operations.

## Safety basis

JVM dictionary lookup uses the stored map's ordinary key equality. The
`dictGet` implementation in
`crates/veln-backend-jvm/src/runtime/collections.java.inc` calls `containsKey`
and `get` on that map. A dictionary does not retain a key-consuming comparator
or another callback specialized to the original, narrower key type. Querying a
widened dictionary with a value from the wider key type therefore cannot invoke
a consumer that only accepts the narrower type.

The same runtime implements `dictInsert` and `dictRemove` by copying the stored
map into a `LinkedHashMap`, applying the operation to that copy, and passing
the copy to `freezeMap`. An operation through a widened dictionary returns a
new immutable dictionary. It does not mutate the original dictionary through
an alias.

Together, ordinary stored-key equality and persistent copy-and-freeze updates
make wider-key lookup, insertion, and removal safe for the compiler-owned
immutable `Dict` constructor. The assignment changes only the static view of
the value; it does not add a comparator, alter stored entries, or expose
alias-visible mutation.

## Evidence and limits

The `widened_dictionary_operations_preserve_original_entries_and_order` test
in `crates/veln-stdlib/veln/prelude.test.veln` widens refined key and value
types. It queries present and absent keys, inserts and removes through the
widened value, and checks that the original entries and their iteration order
remain unchanged.

This reasoning does not generalize to a container that retains a comparator or
another key-consuming callback specialized to its original key type. It also
does not generalize to a container that exposes mutation visible through an
alias. Such containers need their own variance analysis and remain invariant
unless an independent safety argument establishes another classification.
