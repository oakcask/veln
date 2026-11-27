# CSV order analyzer

A local order desk with a Veln HTTP/2 backend and a small TypeScript browser UI.
Upload or paste CSV, then inspect accepted-row totals and all row errors. No
service account, database, or Node package installation is needed.

## Run

Requirements: Rust, JDK 25 or newer, and Node 26 or newer. From the repository
root, build the current CLI:

```sh
cargo build --locked -p veln-cli
```

Start the backend from its source directory in one terminal:

```sh
cd examples/csv_analyzer/backend
../../../target/debug/veln run main . -- 8082
```

The CLI captures program output until exit; it does not print a live readiness
message. In another terminal, from the repository root:

```sh
node examples/csv_analyzer/server.mjs
```

Open <http://127.0.0.1:8080>. The optional Node arguments are the web port and
backend port, for example `node examples/csv_analyzer/server.mjs 8090 8092`.
Pass the corresponding port to the Veln entry. Stop both processes with Ctrl-C.

## Try it

Click **Analyze order** with the initial input. The two valid rows total 900
cents and quantity 3. The row with quantity 0 is excluded and reported as line 4.
**Load sample** adds a second invalid row. Fix the rows and analyze again.
You can also select a local CSV file; selection fills the editable text area.
Nothing is written to disk by the application.

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
bash scripts/agent-run node --test examples/csv_analyzer/test.mjs
```

The tests launch the actual Veln backend on an ephemeral port, send HTTP/2
requests using Node's independent protocol implementation, and assert JSON
results, row failures, bounds, fragmented and padded DATA, CRLF, repeated
connections, idle deadlines, malformed input recovery, proxy cancellation, and
web proxy behavior. They also execute the served browser code
against a small DOM boundary to check submit, result rendering, file selection,
and error recovery. This is not a full browser layout or static type check.
The test owns its process group and temporary compiler cache and removes them
on completion or failure. `VELN_BIN` can select an already-built CLI.

The `test / csv example` workflow runs these checks for example, compiler,
runtime, standard-library, and build-tool changes on PRs and pushes to main.
