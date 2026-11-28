# Three-node distributed string KVS

This example runs three independent Veln processes with fixed membership. Every
node accepts client requests over cleartext HTTP/2. Followers forward requests
to the elected Raft leader. Inter-node RPC bodies use the binary `Packet`
schema in [protocol.veln](protocol.veln); they contain no JSON or delimiter-based
message encoding. The JavaScript client is an independent implementation of
that wire format. The KVS, consensus, HTTP/2 transport and recovery code run in
Veln.

## Run

Use the repository's Rust and JDK prerequisites. From the repository root:

```sh
pnpm example:kvs
```

The launcher builds the CLI and starts node IDs 1, 2 and 3 on loopback ports
19001, 19002 and 19003. It stores separate journals in `.data/node1` through
`.data/node3` next to this README. Wait for election before issuing requests.
The launcher prints process group IDs for fault experiments. Ctrl-C stops all
remaining nodes. An individual node exiting leaves the other two running.

Use a second terminal, also at the repository root:

```sh
node examples/distributed_kvs/client.mjs 19001 status
node examples/distributed_kvs/client.mjs 19002 put '茶' 'hot 🍵' 7 1
node examples/distributed_kvs/client.mjs 19003 get '茶' 7 2
node examples/distributed_kvs/client.mjs 19001 delete '茶' 7 3
node examples/distributed_kvs/client.mjs 19002 get '茶' 7 4
```

A client reply has `kind: 6`. Its `a` field is the application status: `200`
means success, `404` means an absent key, and `503` means the request could not
be confirmed. The `value` field holds the string returned by a successful Get.
An empty string and an absent key are distinct. Status requests have `kind: 7`;
their `kind: 8` replies expose role in `a` (`0` follower, `1` candidate, `2`
leader), known leader ID in `b`, commit index in `c`, and log size in
`entry_term`. Status is a local diagnostic, not a linearizable KVS read.

Each logical operation, including a Get, needs a unique positive
`(CLIENT_ID, SEQUENCE)` pair. Retry an uncertain operation with the same pair
and the same operation, key and value. Start a new operation with a new pair.
Client IDs and sequence numbers fit unsigned 32-bit integers. These identities
are retained in the replicated state so a retried old Put cannot overwrite a
later value. The client does not retry automatically; after an election or a
`503`, retry the same logical operation against a reachable node. A timeout or
`503` does not prove that a write was not committed.

`pnpm example:kvs 20000 ./kvs-data` changes the base port and data root. Node
ports are always base plus node ID. `VELN_BIN` selects an existing CLI and skips
the build. Do not run two processes with the same node ID or share a node's data
directory. Reuse its directory when restarting a stopped node. To start one
node manually, run this from the example directory, using the matching base
port and its existing directory:

```sh
../../target/debug/veln run main . -- 1 19000 .data/node1
```

## Consistency and failure behavior

Put, Get and Delete enter the same replicated log. The leader responds only
after the operation is committed and applied. A new leader appends a no-op in
its own term. Raft's log matching, election restriction and current-term commit
rule preserve committed prefixes across leader changes. Even a node that
still believes it is leader must obtain a majority before completing a new
read. The design follows [Raft, sections 5 and 8](https://raft.github.io/raft.pdf).

For distinct logical requests, a Get begun after a successful Put returns that
value or a later committed value. Concurrent requests can appear in either
order consistent with their real-time completion. Followers never answer a
KVS read from their local map. A retry identifies the original operation and
returns its cached result; it is not a new read of the current state.

The cluster continues after one process stops, provided the other two and the
client can communicate and scheduling and network delays permit election and
RPC completion. Leader loss causes a temporary interruption during election.
In a two-versus-one partition, only the majority side can complete new KVS
operations. An isolated old leader cannot return a stale value as a successful
new read. If fewer than two nodes are reachable, requests fail or time out.
The model excludes Byzantine peers and corrupted storage.

Before acknowledging a vote or changed log, the node completes a journal
record write. Recovery restores the term, vote and log, then rebuilds the map
and request results by applying the committed prefix learned through consensus.
An incomplete final record is discarded and overwritten; an invalid record
followed by another record causes recovery to fail. Node ID and base port are
checked when recovering a journal. They are configuration checks, not peer
authentication.

Durability covers process termination and restart with the filesystem still
available. The current host filesystem API does not provide the sync and
atomic-publication operations required here for a power-loss guarantee. This
example does not claim machine-power-loss durability. Removing a node's journal
and rejoining it under the same voting identity is unsupported.

## Wire format and implementation

Each connection carries one `POST /rpc` request and one response. Requests have
an `application/octet-stream` body; response bodies use the same binary packet
format. There is no TLS, discovery, cluster resizing or authentication. All
addresses are loopback. Connection reuse and multiple application streams per
connection are outside this example's transport scope.

The schema uses big-endian integers and validates the fixed magic `0x4b56` and
version `1`. The header occupies 58 octets:

| Offset | Field | Width |
| --- | --- | --- |
| 0 | magic | 2 |
| 2 | version | 1 |
| 3 | kind | 1 |
| 4 | term | 8 |
| 12 | node | 1 |
| 13 | a | 8 |
| 21 | b | 8 |
| 29 | c | 8 |
| 37 | entry_term | 8 |
| 45 | op | 1 |
| 46 | client | 4 |
| 50 | sequence | 4 |
| 54 | key_size | 2 |
| 56 | value_size | 2 |
| 58 | key, then value | declared octet counts |

Kinds `1`/`2` are vote request/reply, `3`/`4` are append request/reply, `5`/`6`
are client request/reply, and `7`/`8` are status request/reply. Kind `9` is a
journal record and is rejected on the network. For vote requests, `a` and `b`
carry last index and last term. For append requests they carry previous index
and previous term; `c` carries leader commit. A nonzero `entry_term` introduces
one entry; zero represents a heartbeat. Vote/append replies use `a = 1` for
acceptance; append replies carry matched index in `b`. Operations are `1` Put,
`2` Get, `3` Delete and internal `4` no-op. Client requests use `c` as the bounded
forwarding count. A local `307` reply asks the ingress node to forward once;
it is not an HTTP redirect.

Keys and values are Unicode scalar strings encoded as UTF-8, each at most
4096 octets. Empty strings, NUL, line breaks and leading BOM characters are
preserved without normalization. The decoder rejects invalid UTF-8, trailing
octets and inconsistent lengths. Because the current byte helpers do not
convert general Unicode strings, Veln stores canonical hexadecimal String
representations of these UTF-8 strings internally. This representation is
bijective and does not restrict the external strings to ASCII. Wire strings
remain raw UTF-8 octets, not hexadecimal text. Journal files contain hex text
of the schema-encoded records for use with the existing host text-file API.

[command.veln](command.veln) defines request identities, KVS operations,
log entries and application replies as named ADTs. A log entry contains its
term and command; a cached reply contains the application result rather than
a transport packet.

[state.veln](state.veln) groups the Raft state by lifetime and responsibility:

| Type | Contents |
| --- | --- |
| `State` | node ID, persistent state, volatile state, application state |
| `PersistentState` | term, optional vote, replicated log |
| `ReplicatedLog` | last index and typed log entries |
| `VolatileState` | commit index and role |
| `Role` | follower leader hint/deadline, candidate votes/deadline, or leader peer progress |
| `PeerProgress` | next index and matched index for one peer |
| `ApplicationState` | applied index, KVS values and cached application replies |

The constructors and focused access/update functions keep these shapes out of
function signatures. Role-specific data lives in its matching variant.
[raft.veln](raft.veln) owns pure consensus and application transitions over
`State`, typed peer requests and typed peer replies. Small local records still
name multi-result boundaries, such as a next state and reply.

[protocol.veln](protocol.veln) translates between these domain values and the
binary schema. Numeric tags and the overloaded wire fields `a`, `b` and `c`
stay inside that codec. The reply decoder receives the original request so
an empty successful Get remains distinct from a successful Put or Delete,
although their existing wire reply shapes overlap.

[server.veln](server.veln) owns one state loop, bounded event channels,
independent per-peer RPC tasks and at most 32 concurrent inbound connections.
Its pending requests, deliveries, runtime bookkeeping and task contexts have
small named types. It saves durable transitions before sending replies or new
RPCs. [http2_transport.veln](http2_transport.veln) composes standard HTTP/2 core
transitions with real network effects, complete writes and operation deadlines.
[storage.veln](storage.veln) saves and recovers only `PersistentState` through
typed journal deltas. Startup creates fresh volatile and application state from
that recovered state; consensus then supplies the committed prefix to apply.

The example retains all log entries, request identities and journal records.
It has no log compaction, snapshots, eviction or unlimited-workload guarantee.
Recovery and follower catch-up take longer as history grows. It is a teaching
example, not a production storage service.

## Verification

Build the CLI, then run the example's companion tests from its directory and
the real-process integration test from the repository root:

```sh
cargo build --locked -p veln-cli
cd examples/distributed_kvs
../../target/debug/veln test .
cd ../..
pnpm test:example:kvs
```

For agent runs, use the repository's guarded runner. For example, from the
example directory:

```sh
bash ../../scripts/agent-run ../../target/debug/veln test .
```

Companion tests cover a binary wire vector, invalid UTF-8 and framing,
maximum-size strings, vote exclusion, log ordering, leader replacement,
duplicate requests and durable-state reconstruction. Integration tests use
three real Veln processes and an independently encoded JavaScript client.
They cover all ingress nodes, malformed and fragmented HTTP/2 bodies, a paused
leader, a killed leader, partial-tail recovery, complete process restart,
majority loss and a small concurrent history checked against a sequential KVS
model. These scenarios corroborate the implementation; they are not an
exhaustive proof of Raft or an arbitrary-load availability guarantee.
