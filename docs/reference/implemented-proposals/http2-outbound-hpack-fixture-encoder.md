# HTTP/2 Outbound HPACK Fixture Encoder

Status: implemented

This record preserves the completed outbound HPACK fixture encoder slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture module exposes a source-visible helper that maps
exact HPACK static table name/value pairs to indexed-field bytes when the
static entry has a fixed value. The encoder uses the same finite static table
metadata path used by source-visible static decode instead of a separate
one-fixture-per-index encode series. Checked exact pairs include request
pseudo-headers such as `:method: POST`, response pseudo-headers such as
`:status: 404`, ordinary empty-valued static entries such as `content-type:`,
and `accept-encoding: gzip, deflate`, while non-exact pairs such as
`:method: PUT` and `content-type: text/plain` stay on the fixture
encode-failure path. The header-list
encoder for the outbound protocol-core fixtures routes fixture-owned
static-indexed header lists through that helper before falling back to the
other fixture encoder paths. It also accepts raw
literal-without-indexing and literal-with-indexing header lists for supported
static-table names, checked Huffman-marked literal header lists for the
same fixture boundary, and the checked request and response pseudo-header
fixture lists used by outbound HEADERS and server-side `PUSH_PROMISE`
send-intents. The literal-without-indexing encoder also accepts ordinary
new-name fields when the name is lowercase, passes the existing HTTP
field-name token boundary, is not connection-specific, and the value is a
visible-ASCII raw string. It emits deterministic new-name HPACK literal bytes
without Huffman compression.

The same outbound boundary accepts a fixture-owned ordinary new-name
literal-never-indexed header list. It emits the checked `0x10` HPACK prefix
and raw `x-never: no` bytes without inserting that field into the bounded
dynamic table. A later dynamic-index probe for `x-never: no` therefore stays
on the HPACK fixture header-list encode failure path, while any earlier
inserted dynamic entry remains reusable from the returned state.

The same outbound boundary accepts a fixture-owned ordinary new-name
literal-with-indexing header list for a checked visible-ASCII field-name and
value pair. It emits the checked `0x40` HPACK prefix and raw `x-trace: ok`
bytes, returns an immutable encode state that inserts the field into the
bounded dynamic table, and later encodes the same header list from that state
as dynamic indexed byte `0xbe`. Invalid ordinary names remain on the HPACK
fixture header-list encode failure path before outbound HEADERS bytes are
emitted.

The fixture module also exposes a stateful encode transition. Callers create a
separate initial encode state, encode a supported literal-with-indexing header
list, and receive both the encoded header block and a new state whose bounded
dynamic table contains the inserted entry. A later matching header list encoded
from that returned state uses the checked dynamic indexed byte `0xbe` and
returns a state with the encode count advanced again. The stateless
`encode_header_list` wrapper remains as a compatibility path that delegates to
the stateful encoder with a fresh initial encode state.

Successful fixture encoding produces the opaque header-block `ByteChunk` that
the existing outbound send-intent path already accepts. HEADERS therefore
keeps the same single-frame and CONTINUATION splitting behavior based on the
peer-advertised maximum frame size. The checked stateful HEADERS case encodes
the HPACK header block before frame splitting: the first literal-with-indexing
block is split across HEADERS and CONTINUATION frames when the peer frame-size
limit is small, while the later matching header list from the returned encode
state is emitted as a single dynamic indexed HEADERS block. Server-side
`PUSH_PROMISE` uses the same stateful fixture encoder boundary: the first
literal-with-indexing promised header list can be split across `PUSH_PROMISE`
and CONTINUATION frames after the promised-stream id payload, and a later
matching `PUSH_PROMISE` header list encoded from the returned fixture state
uses the dynamic indexed byte `0xbe`.

The stateful encoder also accepts bounded dynamic table-size update requests
for outbound HEADERS header blocks. It emits canonical checked fixture bytes
for the implemented HPACK integer boundary, including `0x3e` for table size
`30` and `0x3f 0x81 0x01` for table size `160`, and returns a new immutable
fixture state with the updated table capacity. Later outbound HEADERS
encoding from that reduced state observes the new capacity before deciding
whether a supported header list can reuse a dynamic indexed entry. The checked
dynamic-table eviction boundary emits `0x20` for a zero table-size update,
returns a state with zero capacity and no dynamic entries, and keeps repeated
`:method: PUT` literal-with-indexing HEADERS encodes on the literal path while
capacity remains zero. It emits `:method: PUT` as a literal again after a
table-size update to `30`, because the entry does not fit the reduced
capacity, and reuses the same entry as `0xbe` after a table-size update to
`42`, because it exactly fits that capacity. A requested
table-size update greater than the active peer-advertised
`SETTINGS_HEADER_TABLE_SIZE` returns a typed HPACK fixture encode failure
before the send-intent path emits header-block bytes.
The main protocol-core example also derives that peer-advertised capacity from
received SETTINGS frames: a lower received peer header-table-size value permits
a smaller outbound update, causes the following supported HEADERS fixture to
encode as a literal instead of reusing `0xbe`, and still allows an inbound
table-size update at the local receive-limit boundary. A higher received peer
header-table-size value permits the matching outbound update and lets a later
fixture HEADERS encode reuse the dynamic indexed entry.
The focused outbound table-size update case also routes the accepted returned
state through later split HEADERS and split server-side `PUSH_PROMISE`
encodes, and keeps the rejected over-peer-limit path on an empty HTTP/2
output chunk list.

The same outbound boundary accepts a fixture-owned dynamic-name
literal-with-indexing header list when the selected header name is already in
the bounded dynamic table. The checked `:path: /again` encode emits
`0x7e 0x06 "/again"`, inserts that fresh pair as the newest dynamic entry,
reuses it as `0xbe`, and keeps the older `:path: /target` entry reachable as
`0xbf`. The aggregate protocol-core example feeds the returned state through
both outbound HEADERS and server-side `PUSH_PROMISE` framing.

The outbound boundary also accepts a fixture-owned dynamic-name
literal-never-indexed header list when the selected header name is already in
the bounded dynamic table. The checked `:path: /secret` encode emits
`0x1f 0x2f 0x07 "/secret"` from the carried `:path` name, does not insert the
fresh value, and keeps the prior `:path: /target` entry reusable as `0xbe`
through outbound HEADERS framing. Encoding the never-indexed dynamic-name
fixture without a matching dynamic-table name remains the same focused HPACK
fixture dynamic-name failure path.

Unsupported header names, unsupported values, and unsupported value encodings
return typed `HpackFixtureFailure` results from the HPACK fixture boundary.
Unsupported ordinary new-name fields stay on the same fixture header-list
encoding failure path before HEADERS or `PUSH_PROMISE` bytes are emitted.
The checked Huffman-marked single-NUL `:path` fixture value encodes to
`0x04 0x82 0xff 0xc7`, and the checked full-table single-byte
`hpack-byte-ff` `:path` fixture value encodes to
`0x04 0x84 0xff 0xff 0xfb 0xbf`. Later fixture label support accepts
multi-byte `hpack-bytes-*` Huffman-marked values; the checked
`hpack-bytes-00-ff` `:path` fixture value encodes to
`0x04 0x85 0xff 0xc7 0xff 0xff 0xdd`. Raw non-visible string values still
remain on the raw string encoding failure path. Those failures are not
projected as HTTP/2 protocol diagnostics by the outbound send-intent helpers.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks
  header-list encoding for static indexed `:method: GET`, `:method: POST`,
  `:path: /`, `:scheme: https`, `:status: 200`, `:status: 404`,
  `accept-encoding: gzip, deflate`, and `content-type:` into outbound HEADERS,
  raw literal
  `:path: /target` into outbound HEADERS split across CONTINUATION frames,
  stateful literal-with-indexing `:path: /target`
  encoding before HEADERS splitting, stateful dynamic indexed reuse as
  `0xbe`, ordinary literal-with-indexing `x-trace: ok` followed by dynamic
  indexed reuse as `0xbe`, outbound dynamic table-size update bytes `0x3e` and
  `0x3f 0x81 0x01`, zero table-size update byte `0x20`, a following literal
  HEADERS block that observes reduced dynamic-table capacity,
  zero-capacity `:method: PUT` insertion that is not retained,
  reduced-capacity `:method: PUT` insertion that is not retained at table size
  `30`, matching insertion that is retained and reused as `0xbe` at table
  size `42`, received lower and higher peer
  header-table-size SETTINGS values driving later outbound HPACK fixture
  capacity,
  dynamic-name literal-with-indexing `:path: /again` into outbound HEADERS,
  reuse of that inserted value as `0xbe`, retained older `:path: /target`
  reuse as `0xbf`, dynamic-name literal-never-indexed `:path: /secret` into
  outbound HEADERS, retained `:path: /target` reuse as `0xbe` after that
  never-indexed block, an
  over-peer-limit table-size update failure,
  raw new-name literal-without-indexing `x-demo: hello` into outbound
  HEADERS and server-side `PUSH_PROMISE`,
  raw new-name literal-never-indexed `x-never: no` into outbound HEADERS,
  the matching missing dynamic-index probe failure, retained dynamic indexed
  reuse as `0xbe` after that never-indexed block,
  Huffman-marked literal `:path: test` into outbound HEADERS,
  Huffman-marked literal `:path: hpack-byte-ff` into outbound HEADERS,
  Huffman-marked literal `:path: hpack-bytes-00-ff` into outbound HEADERS,
  Huffman-marked literal `:authority: abc.test` into outbound HEADERS, static
  indexed `:status: 200`, static indexed
  `accept-encoding: gzip, deflate`, `:method: POST`, and `content-type:`,
  and Huffman-marked literal `:status: 200` into server-side `PUSH_PROMISE`,
  stateful
  literal-with-indexing
  `:path: /target` encoding before `PUSH_PROMISE` splitting, stateful
  dynamic indexed reuse as `0xbe` in a later `PUSH_PROMISE`,
  dynamic-name literal-with-indexing `:path: /again`, newest reuse as `0xbe`,
  and retained older reuse as `0xbf` through later `PUSH_PROMISE`, one
  non-visible
  Huffman value encode failure, and one unsupported-header encode failure
  that remains an HPACK fixture result.
- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the same stateful encoder transition directly at the HPACK fixture boundary:
  direct exact static-indexed helper bytes for `:method: GET`,
  `:method: POST`, `:scheme: https`, `:status: 200`, `:status: 404`,
  `accept-encoding: gzip, deflate`, and `content-type:`, non-exact
  `:method: PUT` and `content-type: text/plain` encode failures,
  separate initial encode state, literal-with-indexing insertion, dynamic
  indexed reuse, encode-count advancement, stateless wrapper compatibility,
  accepted raw new-name literal-without-indexing bytes, rejected invalid
  ordinary new-name failure, accepted raw new-name literal-never-indexed
  bytes, the matching dynamic-index probe failure, retained dynamic indexed
  reuse after the never-indexed block, dynamic-name literal-without-indexing
  encode and retained reuse, dynamic-name literal-never-indexed encode,
  missing-name failure, retained reuse after the never-indexed dynamic-name
  block, dynamic-name literal-with-indexing insertion, newest reuse, retained
  older reuse, accepted outbound table-size update bytes, zero and reduced
  table capacity observed by later encodes, zero-capacity and
  reduced-capacity `:method: PUT` insertion and retention checks, and
  over-peer-limit table-size update failure.
- `../../../examples/specification/run/hpack-fixture-codec-json/` checks the
  direct static-indexed header-list encoder bytes for `:method: GET`,
  `:path: /`, `:scheme: https`, and `:status: 200`, plus unsupported
  header-name and header-value failures with expected fixture
  `fixture header list encoding`.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize the implemented outbound fixture encoder boundary and route
  readers to the checked example.
- `../../../examples/specification/run/http2-protocol-core-outbound-hpack-table-size-update-json/`
  focuses the outbound table-size update state handoff and rejected
  over-peer-limit path: the accepted state feeds split HEADERS and split
  server-side `PUSH_PROMISE`, and the rejected path emits an empty output
  chunk list.
