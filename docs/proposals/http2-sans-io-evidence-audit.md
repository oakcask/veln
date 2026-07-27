# HTTP/2 Retirement Evidence Audit

Status: proposed

Read this page only when migrating the assertion surface removed with the
historical `http2-protocol-core` fixture. Current HTTP/2 behavior starts in
[`http2.md`](../specification/http2.md).

The audit inventory is reconstructed by
[`check-http2-retirement-evidence`](../../scripts/check-http2-retirement-evidence)
from the parent of the fixture-retirement change:

| Historical item | Count | Exact retained evidence |
| --- | ---: | ---: |
| `require_*` invocation | 652 | 0 |
| stdout line | 2,044 | 0 |
| output table name and chunks | 315 | 0 |

These exact-match counts intentionally do not award evidence for a shared
caller name, helper name, first stdout token, diagnostic id, or output-table
prefix. Renamed or consolidated replacements need an item-level checked
mapping that compares the protected historical values with the retained
executable assertion.

Run the default checker for the gate. Run it with `--inventory` to obtain the
stable item keys and value hashes needed to build that mapping.
