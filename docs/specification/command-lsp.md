---
role: specification
authority: normative
update-when: The veln lsp command startup or stdio boundary changes.
specification-coverage: usage=#lsp-command; behavior=#lsp-command; limits=#limits-and-errors
---

# LSP Command

`veln lsp` starts the editor server on stdin and stdout using JSON-RPC
framing. It accepts no source path arguments. Standard output is reserved for
protocol messages.

The server handles initialization, lifecycle shutdown, document open/change,
semantic tokens, definition, prepare-rename, and rename requests. It publishes
diagnostics for open documents and discovered workspace sources after workspace
identity is initialized. Unsaved open-document text is the source for semantic
tokens. A token request for an unopened document falls back to its file URI;
an unreadable file returns an empty token array.

The semantic-token legend and navigation boundaries are specified by
[editor-support.md](editor-support.md).

## Limits and errors

The command is a protocol endpoint rather than a source-analysis CLI. Clients
must use JSON-RPC framing and lifecycle messages; source paths and ordinary CLI
arguments are not accepted.

## References

Transport coverage is in `crates/veln-cli/tests/toolchain_harness/lsp_transport.rs`;
editor feature behavior is specified by [editor-support.md](editor-support.md).
