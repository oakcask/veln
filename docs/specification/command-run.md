---
role: specification
authority: normative
update-when: The veln run command selection, execution gate, entry argument, runtime diagnostic, stdout, or JSON behavior changes.
---

# Run Command

`run` uses the same source discovery rule as `check`. Parse-clean files are
combined into one surface module for entry resolution. It blocks before user
code execution on parse errors, a missing entry function, an entry argument
count mismatch, an entry parameter type that cannot be supplied from command
line text, selected-entry semantic errors, selected source module-header
casing errors, reachable holes, or checked-core blockers.

The entry must be a discovered function. Arguments after `--` are entry
arguments, not source inputs. Entry parameters may be declared as `String`,
`Int`, `Float`, or `Bool`. A final variadic entry parameter may use those same
element types, and extra command-line arguments are converted to that element
type and gathered into the entry binding as `List<T>`. `String` arguments are
passed through unchanged.
`Int` arguments parse as decimal signed integers, `Float` arguments parse as
JVM double-precision decimal text, and `Bool` arguments must be exactly `true`
or `false`. Non-variadic entries keep exact argument count behavior; variadic
entries require at least the fixed parameter count. The reachable program is
semantically checked, lowered to checked core, then typed IR, then JVM
classfile artifacts. Ordinary execution does not
write generated Java source or invoke a Java source compiler. Reachability
follows imported qualified calls by resolving the alias from selected-file `use`
declarations to the imported source module. A selected entry that imports and
reaches an invalid direct-dependency declaration reports the dependency
diagnostic and blocks backend execution. If the selected entry imports the
dependency but reaches only valid dependency declarations, unreachable
dependency diagnostics do not block `run`. A manifest dependency that the
selected entry does not import is not loaded for that invocation, so its
diagnostics are not reported. Semantic diagnostics in functions unreachable
from the selected entry do not block `run`.

`run` and `test` cache generated JVM classfile artifacts by backend content
below the selected Veln user cache root. On Unix other than macOS, the default
root is the `veln` child of an absolute, non-empty `XDG_CACHE_HOME`, or the
`veln` child of an absolute, non-empty `HOME/.cache` fallback. On macOS, it is
the `veln` child of an absolute, non-empty `HOME/Library/Caches`. On Windows,
it is the `veln` child of an absolute, non-empty `LOCALAPPDATA`.
`VELN_CACHE_DIR`, when set, must be non-empty and lexically absolute and names
the complete Veln cache root without an added `veln` component. Selection uses
native operating-system strings and does not canonicalize or normalize the
path.

A command that needs the cache checks Java launcher availability before cache
configuration. It checks cache configuration only after successful source
analysis, executable selection, and JVM program generation. `test` checks the
configuration once before any runnable test body starts. Empty or relative
overrides do not fall back to a host base. An unavailable host base or an
unusable selected root does not fall back to the package, working directory,
`target`, or a temporary directory. Commands that do not reach JVM execution
do not inspect cache configuration.

On a cache miss the command writes the emitted classfiles into the cache; on a
cache hit it validates the manifest and cached classfiles before invoking
`java`. Invalid or incomplete cache entries are replaced instead of executed.
If an invalid entry cannot be removed, the command reports a cache error before
JVM startup and leaves the entry subject to full validation by later
invocations. If removal succeeds but preparation, prepared-entry validation,
or publication fails, the command leaves no published or partial replacement.
A later invocation observes a miss and can retry preparation and publication
below the same selected root; the failure does not select a fallback root.
When concurrent invocations prepare the same cache entry, each invocation uses
only a complete entry that validates against its own generated JVM program; an
invocation that loses publication to another writer revalidates the published
winner before using it. A writer that fails after another invocation publishes
a valid winner does not delete, replace, or invalidate that winner.
If an earlier process stops while it owns cache coordination, a later
invocation either uses a fully validated entry or reports a cache-coordination
error within an internal bound. The error occurs before JVM startup. Recovery
does not execute preparation remnants. The coordination representation,
waiting strategy, and duration are not command contracts. The fault-injected
cache evidence is in the `java::tests` unit tests. The process-level evidence
is `abandoned_jvm_cache_coordination_reaches_bounded_error_without_starting_java`
in the `toolchain_harness` test target.
Runtime trace files for command output remain isolated to the individual
command invocation. Human mode forwards process
stdout and stderr and returns the Java process status for ordinary runtime
failures. When a closed-input fixed-width `ByteView` read returns
`codec.incomplete_input`, human mode reports the missing byte at the decoded
byte offset as the primary diagnostic fact and puts pending readiness,
expected byte count, available byte count, and any available field path in
related notes. When a schema fixed-field check returns
`schema.fixed_field_mismatch`, human mode reports the fixed-field mismatch at
the decoded byte offset as the primary diagnostic fact and puts expected
value, actual value, bounded nearby byte preview, and field path in related
notes. The byte preview is rendered as lowercase hex byte pairs grouped with
spaces and includes the shown byte count, total diagnostic byte count, and
whether the preview was truncated.
When a source-visible `ByteView` range operation returns
`codec.byte_range_out_of_bounds`, human mode reports the failed range fact at
the requested byte offset and puts requested count, available count, and
bounded nearby byte preview in related notes. Checked byte write conversion
failures report `codec.byte_write_value_unrepresentable` and put the helper
name, supplied value, accepted range, width, byte order, and source-visible
`Err` value in related notes.
When binary schema frame decode returns `schema.length_out_of_bounds`, human
mode reports the failed payload boundary at the first missing byte offset and
puts expected payload count, available payload count, bounded nearby byte
preview, and field path in related notes.
When binary schema field-local validation returns `schema.validation_failed`,
human mode reports the failed validation fact at the owning field byte offset
and puts predicate text, decoded values, bounded nearby byte preview, and
field path in related notes.
When generated binary schema encode returns encode-time
`schema.validation_failed`, human mode reports the failed encode validation
fact and puts predicate text, supplied schema-local `Int` values, field path,
and the source-visible `EncodeError` value in related notes.
When a source-visible `EncodeError(...)` is returned directly from a run entry,
human mode uses the same focused encode diagnostic as the corresponding
generated encode or `EncodeStep::Invalid(EncodeError(...))` value and keeps the
rendered `EncodeError` value in related notes.
When generated length-bounded `ByteView` schema encode returns
`schema.encode_value_unrepresentable` for a count mismatch, human mode reports
the failed encode fact and puts the field path, mismatch reason, expected byte
count, actual `ByteView` count, byte offset, bounded nearby byte preview, and
the source-visible `EncodeError` value in related notes.
When binary schema decode returns `schema.integer_out_of_range`, human mode
reports the failed integer range fact at the field byte offset and puts byte
width, accepted range, actual value, bounded nearby byte preview, and field
path in related notes.
When a `veln run` entry returns
`DecodeError(id, byte_offset, field_path)`,
`DecodeErrorWithReason(id, byte_offset, field_path, reason)`,
`DecodeStep::Invalid(DecodeError(id, byte_offset, field_path))`, or
`DecodeStep::Invalid(DecodeErrorWithReason(id, byte_offset, field_path, reason))`,
human mode reports the failed decode fact at the contained byte offset and
puts field path plus the source-visible `DecodeError` value in related notes.
For `DecodeErrorWithReason`, the reason is also a related note. When an
attached reason is a byte-helper failure message with registered helper
context, human mode also puts local byte offset, expected and available byte
counts, and bounded nearby-byte preview in related notes, and `run --json`
keeps the same context in `details.byte_diagnostic`.
For `codec.checksum_mismatch`, human mode reports
`checksum mismatch at byte offset ...` and puts field path, expected checksum,
actual checksum, failure reason, and the source-visible `DecodeError` value in
related notes. `run --json` keeps the same checksum facts in
`details.byte_diagnostic.expected_checksum`, `actual_checksum`, and `reason`.
For `codec.length_mismatch`, human mode reports
`length mismatch at byte offset ...` and puts field path, expected length,
actual length, failure reason, and the source-visible `DecodeError` value in
related notes when the source-visible reason uses
`expected_length=<n>; actual_length=<n>; reason=<text>`. `run --json` keeps
the same length facts in `details.byte_diagnostic.expected_length`,
`actual_length`, and `reason`; plain reason strings keep only `reason`.
For `codec.payload_length_mismatch`, human mode reports
`payload length mismatch at byte offset ...` and puts field path, expected
payload length, actual payload length, failure reason, and the source-visible
`DecodeError` value in related notes when the source-visible reason uses
`expected_payload_length=<n>; actual_payload_length=<n>; reason=<text>`.
`run --json` keeps the same payload length facts in
`details.byte_diagnostic.expected_payload_length`, `actual_payload_length`,
and `reason`; plain reason strings keep only `reason`.
When an entry returns `DecodeStep::NeedMore(readiness)`, human mode reports
`codec.incomplete_input` at the closed-input byte boundary and puts readiness,
requested count when present, and the source-visible `DecodeStep` value in
related notes. `Decoded` entry values remain ordinary successful values.

With `--json`, `run` captures process stdout and stderr into the run JSON
record instead of forwarding them separately. Runtime contract failures are
reported as top-level structured runtime errors with contract details.

Missing `java` before class loading is reported as a JDK setup error.
