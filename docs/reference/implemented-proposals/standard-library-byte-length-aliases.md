---
role: implementation-record
update-when: The public standard-library byte collection length aliases, prelude helper examples, standard-library source bundle, or package-documentation bundle changes.
---

# Standard-Library Byte Length Aliases

The standard prelude exports public function aliases for byte collection length
helpers:

```veln
pub fn byte_chunk_len = byte_chunk_count
pub fn byte_view_len = byte_view_count
```

Current behavior is specified by
[Prelude Helpers](../../specification/prelude-helpers.md).

Completion evidence:

- The `prelude-helpers` executable specification case checks bare and
  `prelude::`-qualified calls, function-value use, target-function
  compatibility, and unchanged `byte_chunk_count` and `byte_view_count`
  behavior.
- The standard-library package bundle and generated MCP package-documentation
  bundle include the aliases through the standard freshness checks.
