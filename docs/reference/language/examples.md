# Implemented Examples

Status: implemented

This file records complete examples that are expected to type-check and run
against the implemented language reference.

## Line-Item Order Summary

The comparison example is implemented in `examples/comparison/`. Its rationale
is recorded in
[Comparison Example Task](../source-decisions/records/result-comparison-example-task.md).

The pure API is:

```veln
summarize_order(lines: List(String), catalog: Dict(String, Int)) -> Result({item_count: Int, subtotal_cents: Int}, {kind: String, input: String})
```

Input lines use `sku,quantity` spelling. The implementation rejects malformed
rows, non-integer or non-positive quantities, and unknown SKUs. The command
wrapper keeps stdout in `main` and leaves parsing and summarization in pure
functions.

The example uses these implemented language features together:

- dictionary lookup with `dict_get`
- fallible list traversal with `list_try_map_with`
- summary accumulation with `list_fold`
- `Result` propagation
- record-shaped success and error values
- `stdio::println` for the wrapper
- a separate partial-program variant with a constrained typed hole
