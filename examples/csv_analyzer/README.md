# CSV order analyzer

A local order desk with a Veln HTTP/2 backend and a small TypeScript browser UI.
Upload or paste CSV, then inspect accepted-row totals and all row errors.
No service account or database is needed. The launcher uses Node built-in modules.

## Run

Requirements: Rust, JDK 25 or newer, Node 26 or newer, and pnpm (the version
pinned in the repository's `packageManager` field). From the repository root:

```sh
pnpm example:csv
```

This builds the current Veln CLI, starts the backend, waits for its listening
log, and starts the browser-facing server. Open <http://127.0.0.1:8080> when
the launcher prints its URL. JSON logs stream to the same terminal.
Ctrl-C stops both servers, including the backend JVM, and removes the run's
temporary log and compiler cache. The launcher adds no Node dependencies;
pnpm may prepare the repository's existing workspace dependencies on first use.
If either server cannot start, the launcher exits and cleans up the other.

Choose other ports when the defaults are occupied:

```sh
pnpm example:csv --web-port 8090 --backend-port 8092
```

After building the CLI, `pnpm example:csv --skip-build` reuses it without
running Cargo. The default command always builds so compiler changes are
included. The launcher currently uses POSIX process groups for cleanup.

## Try it

Click **Analyze order** with the initial input. The two valid rows total 900
cents and quantity 3. The row with quantity 0 is excluded and reported as line 4.
**Load sample** adds a second invalid row. Fix the rows and analyze again.
You can also select a local CSV file; selection fills the editable text area.
CSV contents are not saved. Operational logs contain status and connection
metadata, without CSV rows or SKU values.

The supported CSV dialect has the exact header `sku,quantity,unit_cents`.
It accepts printable ASCII with LF or CRLF line endings, an optional final
newline, and exactly three unquoted columns per data row. SKU must be nonempty.
Quantity is an integer in `1..1000000`; unit cents is an integer in
`0..1000000`. Blank data rows and quoted fields produce row errors. This is
an intentionally bounded dialect, not a general CSV importer. Use at most
100 data rows and 8192 bytes, including the header and line endings.

`POST /api/analyze` accepts the CSV bytes and returns JSON:

```json
{"rows":3,"accepted":2,"quantity":3,"total_cents":900,"errors":[{"line":4,"code":"invalid_quantity"}]}
```

Totals include only accepted rows. All row errors are returned in input order;
line numbers include the header. Invalid encoding, header, line endings, or
row count return HTTP 400 with an `error` code. Oversized input returns 413.
The example uses integer cents; it does not assign a currency.

## Structured logs

The backend uses `log::info`, `log::warning`, and `log::error` from the standard
library inside an `observe::json_lines` handler. Each record has a timestamp,
sequence, source location, service identity, severity, and typed attributes.

| Message | Severity | Attributes |
| --- | --- | --- |
| `server.listening` | info | `address` |
| `request.completed` | info for success, warning for HTTP errors | `status`, `stream_id`, `response_bytes` |
| `connection.rejected` | warning | `reason` |
| `server.failed` | error | `reason` |

`request.completed` is emitted after the response bytes have been written.
HTTP 200 can still contain row validation errors; inspect the JSON response
for those details. Request bodies, SKU values, and raw request paths are not
logged. Attribute values are bounded by the exporter's configured limits.

The pnpm launcher streams the backend's file export to standard error and
removes that temporary file when it stops. To retain logs across runs, start
the backend manually from its source directory:

```sh
cd examples/csv_analyzer/backend
../../../target/debug/veln run main_with_log_file . -- 8082 server.jsonl
```

Start the frontend separately with `node examples/csv_analyzer/server.mjs`
from the repository root. Its optional positional arguments select the web
port and backend port. Follow the retained logs from the repository root:

```sh
tail -f examples/csv_analyzer/backend/server.jsonl
```

The file destination appends and flushes records while the server runs.
`server.jsonl` is ignored by Git. To choose another destination file, pass a
path relative to the backend's working directory instead.

For standard-error output, the original entry remains available from the
backend directory:

```sh
../../../target/debug/veln run main . -- 8082
```

The CLI captures process output until exit, so file output is useful for live
inspection with the current CLI. Exporter health is reported to standard error
when the application returns normally; forceful process termination may prevent
that summary. Export failures follow the standard handler's behavior: they
update exporter health and do not change the application result.

## How it fits together

```text
Browser TypeScript → Node HTTP server → HTTP/2 over loopback TCP → Veln
                         static files       CSV POST + JSON        validation
```

`backend/analyzer.veln` owns parsing, validation, and totals.
`backend/server.veln` uses standard HTTP/2 core and HPACK transitions with real
network effects. The current high-level HTTP/2 service accepts bodyless
requests, so this example supplies a small body-aware loop rather than changing
the standard API. It handles one request per connection and accepts connections
sequentially. A failed connection is closed before the next is accepted.

`dev.mjs` owns building, startup, log streaming, and process cleanup.
`server.mjs` serves a fixed asset list and proxies each analysis through a fresh
HTTP/2 session. It does not parse CSV or compute totals. Node's built-in
TypeScript stripping serves `web/app.ts` as JavaScript; it does not perform
static TypeScript type checking. There are no third-party Node dependencies.

Both listeners bind to loopback. The backend uses prior-knowledge cleartext
HTTP/2, without TLS or HTTP/1 upgrade. Its connection loop limits input to
64 frames, 16 KiB per frame, a 32 KiB wire budget, and a five-second deadline.
The Veln accept loop is recursive and is intended for short local sessions,
not an indefinitely running service. Protocol failures close the connection; the web proxy reports unavailable
backends as HTTP 502. This is a local example, not a deployment template.

## Regression checks

From the repository root after building the CLI:

```sh
pnpm test:example:csv
```

The launcher tests exercise the actual pnpm command, CLI build, startup, log
streaming, HTTP requests, Ctrl-C and SIGTERM cleanup, occupied ports, and missing
build tools. They check that both ports close after shutdown or startup failure.
The application tests launch the actual Veln backend on an ephemeral port, send HTTP/2
requests using Node's independent protocol implementation, and assert JSON
results, row failures, bounds, fragmented and padded DATA, CRLF, repeated
connections, idle deadlines, malformed input recovery, proxy cancellation, and
web proxy behavior. They assert live JSON Lines logs for startup, successful and
failed requests, and rejected connections, plus structured startup failure on
standard error and exclusion of CSV content. They also execute the served browser code
against a small DOM boundary to check submit, result rendering, file selection,
and error recovery. This is not a full browser layout or static type check.
The test owns its process group and temporary compiler cache and removes them
on completion or failure. `VELN_BIN` can select an already-built CLI.

The `test / csv example` workflow runs these checks for example, compiler,
runtime, standard-library, and build-tool changes on PRs and pushes to main.
