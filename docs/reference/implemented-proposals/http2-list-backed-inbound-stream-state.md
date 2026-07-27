# HTTP/2 List-Backed Inbound Stream State

Status: implemented

This record preserves the completed inbound stream-state remainder from the
HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md` and the checked executable case
historical aggregate evidence.

## Completed Behavior

The ordinary HTTP/2 receive core stores tracked inbound streams in one
recursive list. Each entry owns its stream id, receive-window credit, priority
and content-length body-accounting facts, and lifecycle.

Stream lookup, peer admission, DATA debit, stream-level `WINDOW_UPDATE`,
`SETTINGS_INITIAL_WINDOW_SIZE` delta application, PRIORITY replacement,
`RST_STREAM`, end-of-stream transitions, GOAWAY drain checks, and concurrent
stream counting traverse or replace entries in that list. Accepted updates
preserve every unrelated entry, while rejected frames preserve the complete
input state.

The active concurrent-stream receive limit is compared with the current count
of open list entries. It is not a bound imposed by the representation.

## Evidence

- Historical aggregate evidence admits five
  concurrent peer-created streams and prints every list entry.
- The same case debits DATA on stream 3, refills stream 5, and records priority
  facts on stream 5 while preserving the other entries.
- The same case resets stream 3, admits stream 11, and shows that the windows,
  priorities, and lifecycles of unrelated entries remain unchanged.
- The same case rejects stream 13 with attempted count `6` and allowed count
  `5`, proving that rejection uses the current open-entry count.
