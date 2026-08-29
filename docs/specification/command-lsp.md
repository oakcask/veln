---
role: specification
authority: normative
update-when: The veln lsp command startup or stdio boundary changes.
---

# LSP Command

`lsp` starts the editor language server over standard input and standard output
using JSON-RPC framing. It is intended for editor clients and does not take
source path arguments.

The server handles initialize, initialized, shutdown, exit, open-document,
change-document, full semantic-token, definition, prepare-rename, and rename
requests. It publishes diagnostics for open documents and for discovered
workspace sources when the client initializes workspace identity. It keeps the
latest open document text in memory and returns semantic tokens for unsaved
editor content. When a semantic-token request names a document that has not
been opened through the server, the server attempts to read the file URI from
disk; unreadable documents produce an empty token data array.

The semantic-token legend, token classes, LSP navigation support, and editor
feature boundaries are specified in [editor-support.md](editor-support.md).

