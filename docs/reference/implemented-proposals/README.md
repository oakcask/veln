# Implemented Proposal Records

Status: implemented

This directory keeps completed proposal records after their observable behavior
has moved into `../../specification/` or checked examples. Use it for history,
completion evidence, and cleanup audits, not as the source for current
behavior.

## Read First

- Current behavior: [../../specification/README.md](../../specification/README.md).
- Planned or incomplete work: [../../proposals/README.md](../../proposals/README.md).

## Records

- Integer bitwise operators and flag vocabulary removal:
  [integer-bitwise-operators-and-flag-removal.md](integer-bitwise-operators-and-flag-removal.md).
- Binary and hexadecimal integer literals:
  [binary-and-hexadecimal-integer-literals.md](binary-and-hexadecimal-integer-literals.md).
- JVM backend migration:
  [jvm-bytecode-backend.md](jvm-bytecode-backend.md), then
  [jvm-bytecode-backend-full.md](jvm-bytecode-backend-full.md) only for
  original gate details.
- Shared project analysis:
  [project-analysis-pipeline.md](project-analysis-pipeline.md).
- Formatter stabilization:
  [formatter-stabilization.md](formatter-stabilization.md).
- Hash line comments:
  [hash-line-comments.md](hash-line-comments.md).
- Drop legacy slash comments:
  [drop-legacy-slash-comments.md](drop-legacy-slash-comments.md).
- First repair command boundary:
  [repair-command-first-boundary.md](repair-command-first-boundary.md).
- Repair command saved input freshness:
  [repair-command-saved-input-freshness.md](repair-command-saved-input-freshness.md).
- Repair command confirmation and override:
  [repair-command-confirmation-override.md](repair-command-confirmation-override.md).
- Agent test selection graph:
  [agent-test-selection-graph.md](agent-test-selection-graph.md).
- Agent module, package, and documentation model:
  [agent-module-package-docs.md](agent-module-package-docs.md).
- Self-hosting standard library:
  [self-hosting-standard-library.md](self-hosting-standard-library.md).
- Path runtime representation:
  [path-runtime-representation.md](path-runtime-representation.md).
- Iterative list helper runtime:
  [iterative-list-helper-runtime.md](iterative-list-helper-runtime.md).
- ADT generalization route:
  [adt-generalization-route.md](adt-generalization-route.md).
- User-defined ADT follow-ups:
  [user-defined-adts.md](user-defined-adts.md).
- User function tail recursion trampoline:
  [tail-recursion-trampoline.md](tail-recursion-trampoline.md).
- Type parameter angle brackets:
  [type-parameter-angle-brackets.md](type-parameter-angle-brackets.md).
- Canonical type argument delimiters:
  [canonical-type-argument-delimiters.md](canonical-type-argument-delimiters.md).
- Remove legacy type delimiters:
  [remove-legacy-type-delimiters.md](remove-legacy-type-delimiters.md).
- Local inference private helper call-site:
  [local-inference-private-helper-call-site.md](local-inference-private-helper-call-site.md).
- Local inference prelude callback argument:
  [local-inference-prelude-callback-argument.md](local-inference-prelude-callback-argument.md).
- Local inference dictionary callback aliases:
  [local-inference-dictionary-callback-aliases.md](local-inference-dictionary-callback-aliases.md).
- Local inference declared helper callback argument:
  [local-inference-declared-helper-callback-argument.md](local-inference-declared-helper-callback-argument.md).
- Local inference effectful declared-helper callback:
  [local-inference-effectful-declared-helper-callback.md](local-inference-effectful-declared-helper-callback.md).
- Local inference declared helper callback alias:
  [local-inference-declared-helper-callback-alias.md](local-inference-declared-helper-callback-alias.md).
- Local inference prelude callback fallback:
  [local-inference-prelude-callback-fallback.md](local-inference-prelude-callback-fallback.md).
- Local inference record field callback:
  [local-inference-record-field-callback.md](local-inference-record-field-callback.md).
- Local inference local callback binding:
  [local-inference-local-callback-binding.md](local-inference-local-callback-binding.md).
- Local inference local callback binding annotation elision:
  [local-inference-local-callback-binding-annotation-elision.md](local-inference-local-callback-binding-annotation-elision.md).
- Local inference direct return callback:
  [local-inference-direct-return-callback.md](local-inference-direct-return-callback.md).
- Local inference match-arm callback:
  [local-inference-match-arm-callback.md](local-inference-match-arm-callback.md).
- Local inference if-branch callback:
  [local-inference-if-branch-callback.md](local-inference-if-branch-callback.md).
- Local inference callback return expected type:
  [local-inference-callback-return-expected-type.md](local-inference-callback-return-expected-type.md).
- Local inference constructor payload callback:
  [local-inference-constructor-payload-callback.md](local-inference-constructor-payload-callback.md).
- Local inference collection callback element:
  [local-inference-collection-callback-element.md](local-inference-collection-callback-element.md).
- Local inference dictionary value callback:
  [local-inference-dictionary-value-callback.md](local-inference-dictionary-value-callback.md).
- Local inference variadic callback parameter:
  [local-inference-variadic-callback-parameter.md](local-inference-variadic-callback-parameter.md).
- Local inference non-empty collection initializer:
  [local-inference-non-empty-collection-initializer.md](local-inference-non-empty-collection-initializer.md).
- Local inference ADT constructor payload:
  [local-inference-adt-constructor-payload.md](local-inference-adt-constructor-payload.md).
- Local inference match scrutinee constructor pattern:
  [local-inference-match-scrutinee-constructor-pattern.md](local-inference-match-scrutinee-constructor-pattern.md).
- Local inference local pattern let:
  [local-inference-local-pattern-let.md](local-inference-local-pattern-let.md).
- Local inference if branch local let:
  [local-inference-if-branch-local-let.md](local-inference-if-branch-local-let.md).
- Local inference local let expected type paths:
  [local-inference-local-let-expected-type-paths.md](local-inference-local-let-expected-type-paths.md).
- Local inference nested initializer expected type:
  [local-inference-nested-initializer-expected-type.md](local-inference-nested-initializer-expected-type.md).
- Local inference hole expected-type flow:
  [local-inference-hole-expected-type-flow.md](local-inference-hole-expected-type-flow.md).
- Local inference examples cleanup:
  [local-inference-examples-cleanup.md](local-inference-examples-cleanup.md).
- Local inference diagnostic details:
  [local-inference-diagnostic-details.md](local-inference-diagnostic-details.md).
- If else expression syntax:
  [if-else-expression-syntax.md](if-else-expression-syntax.md).
- Public member alias re-exports:
  [public-member-alias-reexports.md](public-member-alias-reexports.md).
- Implicit prelude and unqualified imports:
  [implicit-prelude-and-unqualified-imports.md](implicit-prelude-and-unqualified-imports.md).
- Package export manifest surface:
  [package-export-manifest-surface.md](package-export-manifest-surface.md).
- Package lockfile sources:
  [package-lockfile-sources.md](package-lockfile-sources.md).
- File based modules and packages:
  [file-based-modules-and-packages.md](file-based-modules-and-packages.md).
- Binary fixture helpers:
  [binary-fixture-helpers.md](binary-fixture-helpers.md).
- Binary fixture schema references:
  [binary-fixture-schema-references.md](binary-fixture-schema-references.md).
- Binary data outgoing chunk production:
  [binary-data-outgoing-chunk-production.md](binary-data-outgoing-chunk-production.md).
- Binary data source-visible `u56` byte helpers:
  [binary-data-u56-byte-helpers.md](binary-data-u56-byte-helpers.md).
- Binary data schema conversion boundary:
  [binary-data-schema-conversion-boundary.md](binary-data-schema-conversion-boundary.md).
- Binary data HPACK Huffman EOS diagnostic:
  [binary-data-hpack-huffman-eos-diagnostic.md](binary-data-hpack-huffman-eos-diagnostic.md).
- Binary data HPACK static index byte preview diagnostics:
  [binary-data-hpack-static-index-byte-preview-diagnostics.md](binary-data-hpack-static-index-byte-preview-diagnostics.md).
- Schema documentation references:
  [schema-documentation-references.md](schema-documentation-references.md).
- Remove schema map-to:
  [remove-schema-map-to.md](remove-schema-map-to.md).
- Binary schema `UInt40be` and `UInt40le` primitives:
  [binary-schema-u40-primitives.md](binary-schema-u40-primitives.md).
- Binary schema `UInt48be` and `UInt48le` primitives:
  [binary-schema-u48-primitives.md](binary-schema-u48-primitives.md).
- Binary schema direct visible big-endian width parity for `UInt16be`,
  `UInt24be`, `UInt31be`, `UInt32be`, `UInt56be`, and `UInt64be`:
  [binary-schema-big-endian-width-parity.md](binary-schema-big-endian-width-parity.md).
- Binary schema `UInt56be` and `UInt56le` primitives:
  [binary-schema-u56-primitives.md](binary-schema-u56-primitives.md).
- Binary schema direct visible little-endian width parity for `UInt56le` and
  `UInt64le`:
  [binary-schema-u56le-u64le-parity.md](binary-schema-u56le-u64le-parity.md).
- Lowercase schema primitives:
  [lowercase-schema-primitives.md](lowercase-schema-primitives.md).
- Binary schema `Flag40be`, `Flag40le`, `Flag56be`, and `Flag56le` bitsets:
  [binary-schema-flag40-and-flag56-bitsets.md](binary-schema-flag40-and-flag56-bitsets.md).
- Binary schema `Flag48be` and `Flag48le` bitsets:
  [binary-schema-flag48-bitsets.md](binary-schema-flag48-bitsets.md).
- Binary schema flag decode bindings:
  [binary-schema-flag-decode-bindings.md](binary-schema-flag-decode-bindings.md).
- Binary schema reserved-byte-prefix encode:
  [binary-schema-reserved-byte-prefix-encode.md](binary-schema-reserved-byte-prefix-encode.md).
- Binary schema general reserved byte prefixes:
  [binary-schema-general-reserved-byte-prefixes.md](binary-schema-general-reserved-byte-prefixes.md).
- Binary schema reserved fifteen-bit prefix:
  [binary-schema-reserved-fifteen-bit-prefix.md](binary-schema-reserved-fifteen-bit-prefix.md).
- Binary schema packed visible two-byte groups:
  [binary-schema-packed-visible-two-byte-groups.md](binary-schema-packed-visible-two-byte-groups.md).
- Binary schema packed visible three-byte groups:
  [binary-schema-packed-visible-three-byte-groups.md](binary-schema-packed-visible-three-byte-groups.md).
- Binary schema packed visible four-byte groups:
  [binary-schema-packed-visible-four-byte-groups.md](binary-schema-packed-visible-four-byte-groups.md).
- Binary schema packed visible five-byte groups:
  [binary-schema-packed-visible-five-byte-groups.md](binary-schema-packed-visible-five-byte-groups.md).
- Binary schema packed visible six-byte groups:
  [binary-schema-packed-visible-six-byte-groups.md](binary-schema-packed-visible-six-byte-groups.md).
- Binary schema packed visible seven-byte groups:
  [binary-schema-packed-visible-seven-byte-groups.md](binary-schema-packed-visible-seven-byte-groups.md).
- Binary schema packed visible eight-byte groups:
  [binary-schema-packed-visible-eight-byte-groups.md](binary-schema-packed-visible-eight-byte-groups.md).
- Binary schema one-byte reserved suffix:
  [binary-schema-one-byte-reserved-suffix.md](binary-schema-one-byte-reserved-suffix.md).
- Binary schema six-byte reserved suffix:
  [binary-schema-six-byte-reserved-suffix.md](binary-schema-six-byte-reserved-suffix.md).
- Binary schema wide reserved suffix groups:
  [binary-schema-wide-reserved-suffix-groups.md](binary-schema-wide-reserved-suffix-groups.md).
- Binary schema byte-visible reserved suffix:
  [binary-schema-byte-visible-reserved-suffix.md](binary-schema-byte-visible-reserved-suffix.md).
- Binary schema split reserved groups:
  [binary-schema-split-reserved-groups.md](binary-schema-split-reserved-groups.md).
- Binary schema general reserved bitfield layouts:
  [binary-schema-general-reserved-bitfield-layouts.md](binary-schema-general-reserved-bitfield-layouts.md).
- Binary schema seven-byte split reserved layouts:
  [binary-schema-seven-byte-split-reserved-layouts.md](binary-schema-seven-byte-split-reserved-layouts.md).
- Binary schema eight-byte split reserved layouts:
  [binary-schema-eight-byte-split-reserved-layouts.md](binary-schema-eight-byte-split-reserved-layouts.md).
- Binary schema wide reserved prefix groups:
  [binary-schema-wide-reserved-prefix-groups.md](binary-schema-wide-reserved-prefix-groups.md).
- Binary schema suffix reserved groups:
  [binary-schema-suffix-reserved-groups.md](binary-schema-suffix-reserved-groups.md).
- Binary schema reserved-bit mapping exposure:
  [binary-schema-reserved-bit-mapping-exposure.md](binary-schema-reserved-bit-mapping-exposure.md).
- Binary schema repeat helper bindings, including representation-only
  lowercase reserved repeat payloads and same-module recursive repeated nested
  payload helpers:
  [binary-schema-repeat-schema-payload-helpers.md](binary-schema-repeat-schema-payload-helpers.md).
- Binary schema repeat `ByteView(left_length - right_length)` helpers:
  [binary-schema-repeat-byteview-subtract-helpers.md](binary-schema-repeat-byteview-subtract-helpers.md).
- Binary schema direct nested decode bindings:
  [binary-schema-direct-nested-decode-bindings.md](binary-schema-direct-nested-decode-bindings.md).
- Binary schema anonymous record decode:
  [binary-schema-anonymous-record-decode.md](binary-schema-anonymous-record-decode.md).
- Binary schema sibling nested anonymous record decode:
  [binary-schema-sibling-nested-anonymous-record-decode.md](binary-schema-sibling-nested-anonymous-record-decode.md).
- Binary schema anonymous record encode:
  [binary-schema-anonymous-record-encode.md](binary-schema-anonymous-record-encode.md).
- Binary schema dispatch `ByteView(length_field)` payload helpers:
  [binary-schema-dispatch-byteview-payload-helpers.md](binary-schema-dispatch-byteview-payload-helpers.md).
- Binary schema dispatch `ByteView(left_length + right_length)` payload helpers:
  [binary-schema-dispatch-byteview-add-payload-helpers.md](binary-schema-dispatch-byteview-add-payload-helpers.md).
- Binary schema dispatch `ByteView(left_length - right_length)` payload helpers:
  [binary-schema-dispatch-byteview-subtract-payload-helpers.md](binary-schema-dispatch-byteview-subtract-payload-helpers.md).
- Binary schema dispatch `ByteView(left_length * right_length)` payload helpers:
  [binary-schema-dispatch-byteview-product-payload-helpers.md](binary-schema-dispatch-byteview-product-payload-helpers.md).
- Binary schema dispatch `ByteView(left_length / right_length)` payload helpers:
  [binary-schema-dispatch-byteview-quotient-payload-helpers.md](binary-schema-dispatch-byteview-quotient-payload-helpers.md).
- Binary schema imported dispatch `ByteView(left_length / right_length)` payload helpers:
  [binary-schema-imported-dispatch-byteview-quotient-payload-helpers.md](binary-schema-imported-dispatch-byteview-quotient-payload-helpers.md).
- Binary schema dispatch one-bit reserved payload helpers:
  [binary-schema-dispatch-one-bit-reserved-payload-helpers.md](binary-schema-dispatch-one-bit-reserved-payload-helpers.md).
- Binary schema dispatch lowercase one-bit reserved payload:
  [binary-schema-dispatch-lowercase-one-bit-reserved-payload.md](binary-schema-dispatch-lowercase-one-bit-reserved-payload.md).
- Binary schema dispatch lowercase two-bit reserved payload:
  [binary-schema-dispatch-lowercase-two-bit-reserved-payload.md](binary-schema-dispatch-lowercase-two-bit-reserved-payload.md).
- Binary schema dispatch lowercase subbyte reserved payloads:
  [binary-schema-dispatch-lowercase-subbyte-reserved-payloads.md](binary-schema-dispatch-lowercase-subbyte-reserved-payloads.md).
- Binary schema dispatch nonzero lowercase subbyte reserved payloads:
  [binary-schema-dispatch-nonzero-lowercase-subbyte-reserved-payloads.md](binary-schema-dispatch-nonzero-lowercase-subbyte-reserved-payloads.md).
- Binary schema dispatch reserved byte prefix payload helpers:
  [binary-schema-dispatch-reserved-byte-prefix-payload-helpers.md](binary-schema-dispatch-reserved-byte-prefix-payload-helpers.md).
- Binary schema dispatch nested repeat helpers:
  [binary-schema-dispatch-nested-repeat-helpers.md](binary-schema-dispatch-nested-repeat-helpers.md).
- Binary schema `ByteView` payload multiple validation:
  [binary-schema-byteview-payload-multiple.md](binary-schema-byteview-payload-multiple.md).
- Binary schema field reference diagnostics:
  [binary-schema-field-reference-diagnostics.md](binary-schema-field-reference-diagnostics.md).
- Binary schema dispatch payload helper boundary diagnostics:
  [binary-schema-dispatch-payload-helper-boundary-diagnostics.md](binary-schema-dispatch-payload-helper-boundary-diagnostics.md).
- Binary schema direction-specific dispatch payload helpers:
  [binary-schema-directional-dispatch-payload-helpers.md](binary-schema-directional-dispatch-payload-helpers.md).
- Binary schema mapped encode projection diagnostics:
  [binary-schema-mapped-encode-projection-diagnostics.md](binary-schema-mapped-encode-projection-diagnostics.md).
- Binary schema same-module recursive dispatch helpers:
  [binary-schema-same-module-recursive-dispatch-helpers.md](binary-schema-same-module-recursive-dispatch-helpers.md).
- Binary schema mapping arithmetic encode:
  [binary-schema-mapping-arithmetic-encode.md](binary-schema-mapping-arithmetic-encode.md).
- Binary schema mapping converter varargs:
  [binary-schema-mapping-converter-varargs.md](binary-schema-mapping-converter-varargs.md).
- Binary schema imported converter bare inverse encode:
  [binary-schema-imported-converter-bare-inverse-encode.md](binary-schema-imported-converter-bare-inverse-encode.md).
- Schema binary pattern boundary:
  [schema-binary-pattern-boundary.md](schema-binary-pattern-boundary.md).
- Codec generated helper boundary slices:
  [codec-generated-helper-boundary-slices.md](codec-generated-helper-boundary-slices.md).
- Remove source codec declarations:
  [remove-source-codec-declarations.md](remove-source-codec-declarations.md).
- Schema helper public surface cleanup:
  [schema-helper-public-surface-cleanup.md](schema-helper-public-surface-cleanup.md).
- Format-neutral schema `Option` helpers:
  [format-neutral-schema-option-helpers.md](format-neutral-schema-option-helpers.md).
- Format-neutral schema top-level option-list helpers:
  [format-neutral-schema-option-list-helpers.md](format-neutral-schema-option-list-helpers.md).
- Format-neutral schema option dictionary helpers:
  [format-neutral-schema-option-dict-helpers.md](format-neutral-schema-option-dict-helpers.md).
- Format-neutral schema result helpers:
  [format-neutral-schema-result-helpers.md](format-neutral-schema-result-helpers.md).
- Format-neutral schema recursive result visible-shape helpers:
  [format-neutral-schema-result-visible-shapes.md](format-neutral-schema-result-visible-shapes.md).
- Format-neutral schema top-level list helpers:
  [format-neutral-schema-list-helpers.md](format-neutral-schema-list-helpers.md).
- Format-neutral schema nested record list helpers:
  [format-neutral-schema-nested-list-helpers.md](format-neutral-schema-nested-list-helpers.md).
- Format-neutral schema nested record option-list helpers:
  [format-neutral-schema-nested-option-list-helpers.md](format-neutral-schema-nested-option-list-helpers.md).
- Format-neutral schema string-keyed scalar dictionary helpers:
  [format-neutral-schema-dict-helpers.md](format-neutral-schema-dict-helpers.md).
- Format-neutral schema nested record dictionary helpers:
  [format-neutral-schema-nested-dict-helpers.md](format-neutral-schema-nested-dict-helpers.md).
- Format-neutral schema recursive container helpers:
  [format-neutral-schema-recursive-container-helpers.md](format-neutral-schema-recursive-container-helpers.md).
- Format-neutral schema source ADT helpers:
  [format-neutral-schema-source-adt-helpers.md](format-neutral-schema-source-adt-helpers.md).
- Format-neutral schema vec helpers:
  [format-neutral-schema-vec-helpers.md](format-neutral-schema-vec-helpers.md).
- Format-neutral schema scalar encode helpers:
  [format-neutral-schema-scalar-encode-helpers.md](format-neutral-schema-scalar-encode-helpers.md).
- Format-neutral schema `Option<scalar>` encode helpers:
  [format-neutral-schema-option-scalar-encode-helpers.md](format-neutral-schema-option-scalar-encode-helpers.md).
- Format-neutral schema `List<scalar>` encode helpers:
  [format-neutral-schema-list-scalar-encode-helpers.md](format-neutral-schema-list-scalar-encode-helpers.md).
- Format-neutral schema `List<Option<scalar>>` encode helpers:
  [format-neutral-schema-list-option-encode-helpers.md](format-neutral-schema-list-option-encode-helpers.md).
- Format-neutral schema `List<Option<List<scalar>>>` encode helpers:
  [format-neutral-schema-list-option-list-encode-helpers.md](format-neutral-schema-list-option-list-encode-helpers.md).
- Format-neutral schema `Vec<scalar>` encode helpers:
  [format-neutral-schema-vec-scalar-encode-helpers.md](format-neutral-schema-vec-scalar-encode-helpers.md).
- Format-neutral schema nested record `Vec<scalar>` encode helpers:
  [format-neutral-schema-nested-vec-scalar-encode-helpers.md](format-neutral-schema-nested-vec-scalar-encode-helpers.md).
- Format-neutral schema `Vec<Option<scalar>>` encode helpers:
  [format-neutral-schema-option-vec-encode-helpers.md](format-neutral-schema-option-vec-encode-helpers.md).
- Format-neutral schema bounded recursive `Vec<Vec<scalar>>` encode helpers:
  [format-neutral-schema-recursive-vec-scalar-encode-helpers.md](format-neutral-schema-recursive-vec-scalar-encode-helpers.md).
- Format-neutral schema `Dict<String, scalar>` encode helpers:
  [format-neutral-schema-dict-scalar-encode-helpers.md](format-neutral-schema-dict-scalar-encode-helpers.md).
- Format-neutral schema `Dict<String, Option<scalar>>` encode helpers:
  [format-neutral-schema-dict-option-scalar-encode-helpers.md](format-neutral-schema-dict-option-scalar-encode-helpers.md).
- Format-neutral schema `Dict<String, List<scalar>>` encode helpers:
  [format-neutral-schema-dict-list-scalar-encode-helpers.md](format-neutral-schema-dict-list-scalar-encode-helpers.md).
- Format-neutral schema `Dict<String, Vec<scalar>>` encode helpers:
  [format-neutral-schema-dict-vec-scalar-encode-helpers.md](format-neutral-schema-dict-vec-scalar-encode-helpers.md).
- Format-neutral schema `Dict<String, Vec<Option<scalar>>>` encode helpers:
  [format-neutral-schema-dict-vec-option-encode-helpers.md](format-neutral-schema-dict-vec-option-encode-helpers.md).
- Format-neutral schema `Option<Dict<String, scalar>>` encode helpers:
  [format-neutral-schema-option-dict-encode-helpers.md](format-neutral-schema-option-dict-encode-helpers.md).
- Format-neutral schema container encode helpers:
  [format-neutral-schema-container-encode-helpers.md](format-neutral-schema-container-encode-helpers.md).
- Format-neutral schema `Result<scalar, Option<scalar>>` encode helpers:
  [format-neutral-schema-result-option-encode-helpers.md](format-neutral-schema-result-option-encode-helpers.md).
- Format-neutral schema recursive `Result` encode helpers:
  [format-neutral-schema-recursive-result-encode-helpers.md](format-neutral-schema-recursive-result-encode-helpers.md).
- Format-neutral schema result-container encode helpers:
  [format-neutral-schema-result-container-encode-helpers.md](format-neutral-schema-result-container-encode-helpers.md).
- Format-neutral schema encode helper diagnostics:
  [format-neutral-schema-encode-helper-diagnostics.md](format-neutral-schema-encode-helper-diagnostics.md).
- Recursive format-neutral schema encode shapes:
  [recursive-format-neutral-schema-encode-shapes.md](recursive-format-neutral-schema-encode-shapes.md).
- Codec hand-written encode resume:
  [codec-hand-written-encode-resume.md](codec-hand-written-encode-resume.md).
- Codec hand-written `NeedEnd` boundary:
  [codec-hand-written-need-end-boundary.md](codec-hand-written-need-end-boundary.md).
- Codec imported hand-written boundary:
  [codec-imported-hand-written-boundary.md](codec-imported-hand-written-boundary.md).
- Codec imported derived boundary:
  [codec-imported-derived-boundary.md](codec-imported-derived-boundary.md).
- Codec consumed-count invalid diagnostics:
  [codec-consumed-count-invalid-diagnostics.md](codec-consumed-count-invalid-diagnostics.md).
- Codec sequence mismatch diagnostics:
  [codec-sequence-mismatch-diagnostics.md](codec-sequence-mismatch-diagnostics.md).
- Codec payload length mismatch diagnostics:
  [codec-payload-length-mismatch-diagnostics.md](codec-payload-length-mismatch-diagnostics.md).
- Codec padding mismatch diagnostics:
  [codec-padding-mismatch-diagnostics.md](codec-padding-mismatch-diagnostics.md).
- Codec integer out-of-range diagnostics:
  [codec-integer-out-of-range-diagnostics.md](codec-integer-out-of-range-diagnostics.md).
- Codec version mismatch diagnostics:
  [codec-version-mismatch-diagnostics.md](codec-version-mismatch-diagnostics.md).
- Codec tag mismatch diagnostics:
  [codec-tag-mismatch-diagnostics.md](codec-tag-mismatch-diagnostics.md).
- Network adapter ownership boundary:
  [network-adapter-ownership-boundary.md](network-adapter-ownership-boundary.md).
- Network stream close boundary:
  [network-stream-close-boundary.md](network-stream-close-boundary.md).
- Network stream shutdown write boundary:
  [network-stream-shutdown-write-boundary.md](network-stream-shutdown-write-boundary.md).
- Network stream shutdown read boundary:
  [network-stream-shutdown-read-boundary.md](network-stream-shutdown-read-boundary.md).
- Network stream state inspection:
  [network-stream-state-inspection.md](network-stream-state-inspection.md).
- Network listener close boundary:
  [network-listener-close-boundary.md](network-listener-close-boundary.md).
- Network client connect boundary:
  [network-client-connect-boundary.md](network-client-connect-boundary.md).
- Network write chunks boundary:
  [network-write-chunks-boundary.md](network-write-chunks-boundary.md).
- Network write until boundary:
  [network-write-until-boundary.md](network-write-until-boundary.md).
- Network write chunks until boundary:
  [network-write-chunks-until-boundary.md](network-write-chunks-until-boundary.md).
- Network write until cancellable boundary:
  [network-write-until-cancellable-boundary.md](network-write-until-cancellable-boundary.md).
- Network write chunks until cancellable boundary:
  [network-write-chunks-until-cancellable-boundary.md](network-write-chunks-until-cancellable-boundary.md).
- Network adapter cancellable write-drain:
  [network-adapter-cancellable-write-drain.md](network-adapter-cancellable-write-drain.md).
- Network monotonic clock boundary:
  [network-monotonic-clock-boundary.md](network-monotonic-clock-boundary.md).
- Network deadline at boundary:
  [network-deadline-at-boundary.md](network-deadline-at-boundary.md).
- Network adapter outbound write ordering:
  [network-adapter-outbound-write-ordering.md](network-adapter-outbound-write-ordering.md).
- Network HTTP/2 adapter core write boundary:
  [network-http2-adapter-core-write-boundary.md](network-http2-adapter-core-write-boundary.md).
- Network adapter clean shutdown:
  [network-adapter-clean-shutdown.md](network-adapter-clean-shutdown.md).
- Network cancel owner boundary:
  [network-cancel-owner-boundary.md](network-cancel-owner-boundary.md).
- Network cancel owner status:
  [network-cancel-owner-status.md](network-cancel-owner-status.md).
- Network production loopback lifecycle:
  [network-production-loopback-lifecycle.md](network-production-loopback-lifecycle.md).
- Network production listen/connect lifecycle:
  [network-production-listen-connect-lifecycle.md](network-production-listen-connect-lifecycle.md).
- Network production cancellable deadline lifecycle:
  [network-production-cancellable-deadline-lifecycle.md](network-production-cancellable-deadline-lifecycle.md).
- Network production owner-drain lifecycle:
  [network-production-owner-drain-lifecycle.md](network-production-owner-drain-lifecycle.md).
- Network production multi-chunk routing:
  [network-production-multi-chunk-routing.md](network-production-multi-chunk-routing.md).
  Includes the production multi-event adapter task-helper routing evidence and
  the production multi-chunk read-failure runtime boundary plus the
  per-stream task handler-failure lifecycle cleanup boundary.
- Network stream adapter routing helper:
  [network-stream-adapter-routing-helper.md](network-stream-adapter-routing-helper.md).
- Network adapter accept-loop helper:
  [network-adapter-accept-loop-helper.md](network-adapter-accept-loop-helper.md).
- Network production two-stream multi-cycle routing:
  [network-production-two-stream-multi-cycle-routing.md](network-production-two-stream-multi-cycle-routing.md).
- Network stream address metadata:
  [network-stream-address-metadata.md](network-stream-address-metadata.md).
- Network listener address metadata:
  [network-listener-address-metadata.md](network-listener-address-metadata.md).
- Network channel select-many routing:
  [network-channel-select-many-routing.md](network-channel-select-many-routing.md).
- Network channel select timeout result:
  [network-channel-select-timeout-result.md](network-channel-select-timeout-result.md).
- Network channel select timeout cancellable:
  [network-channel-select-timeout-cancellable.md](network-channel-select-timeout-cancellable.md).
- HTTP/2 stream domain values:
  [http2-stream-domain-values.md](http2-stream-domain-values.md).
- HTTP/2 peer-created stream id ordering:
  [http2-peer-created-stream-id-ordering.md](http2-peer-created-stream-id-ordering.md).
- HTTP/2 unknown frame preservation:
  [http2-unknown-frame-preservation.md](http2-unknown-frame-preservation.md).
- HTTP/2 header-block continuation state:
  [http2-header-block-continuation-state.md](http2-header-block-continuation-state.md).
- HTTP/2 HPACK static indexed fixture:
  [http2-hpack-authority-static-indexed-fixture.md](http2-hpack-authority-static-indexed-fixture.md).
- HTTP/2 HPACK static table decode and static-name literals:
  [http2-hpack-static-table-decode.md](http2-hpack-static-table-decode.md).
- HTTP/2 HPACK static-name Huffman literals:
  [http2-hpack-static-name-huffman-literals.md](http2-hpack-static-name-huffman-literals.md).
- HTTP/2 HPACK Huffman decode boundary:
  [http2-hpack-huffman-decode-boundary.md](http2-hpack-huffman-decode-boundary.md).
- HTTP/2 HPACK Huffman encode boundary:
  [http2-hpack-huffman-encode-boundary.md](http2-hpack-huffman-encode-boundary.md).
- HTTP/2 HPACK no-Huffman raw literal fixture:
  [http2-hpack-authority-literal-fixture.md](http2-hpack-authority-literal-fixture.md).
- HTTP/2 HPACK dynamic table fixture:
  [http2-hpack-dynamic-table-eviction-fixture.md](http2-hpack-dynamic-table-eviction-fixture.md).
- HTTP/2 HPACK dynamic table accounting core:
  [http2-hpack-dynamic-table-accounting-core.md](http2-hpack-dynamic-table-accounting-core.md).
- HTTP/2 HPACK static-name indexing core:
  [http2-hpack-static-name-indexing-core.md](http2-hpack-static-name-indexing-core.md).
- HTTP/2 HPACK dynamic index core:
  [http2-hpack-dynamic-index-core.md](http2-hpack-dynamic-index-core.md).
- HTTP/2 HPACK integer core:
  [http2-hpack-integer-core.md](http2-hpack-integer-core.md).
- HTTP/2 HPACK dynamic raw literal-name core:
  [http2-hpack-dynamic-raw-literal-name-core.md](http2-hpack-dynamic-raw-literal-name-core.md).
- HTTP/2 HPACK dynamic raw literal-name Huffman values:
  [http2-hpack-dynamic-raw-literal-name-huffman-values.md](http2-hpack-dynamic-raw-literal-name-huffman-values.md).
- HTTP/2 HPACK dynamic-name literal core:
  [http2-hpack-dynamic-name-literal-core.md](http2-hpack-dynamic-name-literal-core.md).
- HTTP/2 HPACK dynamic-name Huffman values:
  [http2-hpack-dynamic-name-huffman-values.md](http2-hpack-dynamic-name-huffman-values.md).
- HTTP/2 HPACK table-size receive policy:
  [http2-hpack-table-size-policy.md](http2-hpack-table-size-policy.md).
- HTTP/2 HPACK consecutive table-size updates:
  [http2-hpack-multiple-table-size-updates.md](http2-hpack-multiple-table-size-updates.md).
- HTTP/2 HPACK Huffman fixture:
  [http2-hpack-huffman-fixture.md](http2-hpack-huffman-fixture.md).
- HTTP/2 HPACK malformed Huffman padding diagnostic:
  [http2-hpack-huffman-padding-diagnostic.md](http2-hpack-huffman-padding-diagnostic.md).
- HTTP/2 HPACK focused Huffman diagnostics:
  [http2-hpack-huffman-focused-diagnostics.md](http2-hpack-huffman-focused-diagnostics.md).
- HTTP/2 HPACK multi-byte non-visible fixture:
  [http2-hpack-multibyte-non-visible-fixture.md](http2-hpack-multibyte-non-visible-fixture.md).
- HTTP/2 HPACK malformed string diagnostics:
  [http2-hpack-malformed-string-diagnostics.md](http2-hpack-malformed-string-diagnostics.md).
- HTTP/2 HPACK dynamic-name continuation diagnostics:
  [http2-hpack-dynamic-name-continuation-diagnostics.md](http2-hpack-dynamic-name-continuation-diagnostics.md).
- Runtime diagnostic HPACK fixture payloads:
  [runtime-diagnostic-hpack-fixture-payloads.md](runtime-diagnostic-hpack-fixture-payloads.md).
- Runtime diagnostic HPACK fixture helper payload:
  [runtime-diagnostic-hpack-helper-payload.md](runtime-diagnostic-hpack-helper-payload.md).
- Runtime diagnostic generated encode value payload:
  [runtime-diagnostic-encode-value-payload.md](runtime-diagnostic-encode-value-payload.md).
- Schema-owned encode value diagnostics:
  [schema-owned-encode-value-diagnostics.md](schema-owned-encode-value-diagnostics.md).
- Schema-owned dispatch value diagnostics:
  [schema-owned-dispatch-value-diagnostics.md](schema-owned-dispatch-value-diagnostics.md).
- Runtime diagnostic generated schema fixed-field payload:
  [runtime-diagnostic-schema-fixed-field-payload.md](runtime-diagnostic-schema-fixed-field-payload.md).
- Codec-owned decode invalid id diagnostics:
  [codec-owned-decode-invalid-id-diagnostics.md](codec-owned-decode-invalid-id-diagnostics.md).
- Codec magic mismatch diagnostics:
  [codec-magic-mismatch-diagnostics.md](codec-magic-mismatch-diagnostics.md).
- Codec unsupported feature diagnostics:
  [codec-unsupported-feature-diagnostics.md](codec-unsupported-feature-diagnostics.md).
- Runtime diagnostic HTTP/2 preface payloads:
  [runtime-diagnostic-http2-preface-payloads.md](runtime-diagnostic-http2-preface-payloads.md).
- Runtime diagnostic HTTP/2 SETTINGS ACK payload:
  [runtime-diagnostic-http2-settings-ack-payload.md](runtime-diagnostic-http2-settings-ack-payload.md).
- Runtime diagnostic HTTP/2 DATA, flow-control, and content-length payloads:
  [runtime-diagnostic-http2-data-flow-content-length-payloads.md](runtime-diagnostic-http2-data-flow-content-length-payloads.md).
- Runtime diagnostic HTTP/2 invalid frame-kind stream-state payload:
  [runtime-diagnostic-http2-invalid-frame-kind-stream-state-payload.md](runtime-diagnostic-http2-invalid-frame-kind-stream-state-payload.md).
- Runtime diagnostic HTTP/2 invalid stream id payload:
  [runtime-diagnostic-http2-invalid-stream-id-payload.md](runtime-diagnostic-http2-invalid-stream-id-payload.md).
- Runtime diagnostic HTTP/2 WINDOW_UPDATE payload:
  [runtime-diagnostic-http2-window-update-payload.md](runtime-diagnostic-http2-window-update-payload.md).
- Runtime diagnostic HTTP/2 PRIORITY dependency payload:
  [runtime-diagnostic-http2-priority-dependency-payload.md](runtime-diagnostic-http2-priority-dependency-payload.md).
- Runtime diagnostic HTTP/2 HPACK raw request-trailer payload:
  [runtime-diagnostic-http2-hpack-raw-request-trailer-payload.md](runtime-diagnostic-http2-hpack-raw-request-trailer-payload.md).
- Runtime diagnostic HTTP/2 HPACK raw request-trailer invalid-token payload:
  [runtime-diagnostic-http2-hpack-raw-request-trailer-token-payload.md](runtime-diagnostic-http2-hpack-raw-request-trailer-token-payload.md).
- HTTP/2 response trailer validation:
  [http2-response-trailer-validation.md](http2-response-trailer-validation.md).
- HTTP/2 content-length header validation:
  [http2-content-length-header-validation.md](http2-content-length-header-validation.md).
- HTTP/2 content-length body accounting:
  [http2-content-length-body-accounting.md](http2-content-length-body-accounting.md).
- Runtime diagnostic HTTP/2 closed-input helper payload:
  [runtime-diagnostic-http2-closed-helper-payload.md](runtime-diagnostic-http2-closed-helper-payload.md).
- Runtime diagnostic HTTP/2 partial preface helper payload:
  [runtime-diagnostic-http2-partial-preface-helper-payload.md](runtime-diagnostic-http2-partial-preface-helper-payload.md).
- Runtime diagnostic HTTP/2 invalid preface helper payload:
  [runtime-diagnostic-http2-invalid-preface-helper-payload.md](runtime-diagnostic-http2-invalid-preface-helper-payload.md).
- Runtime diagnostic HTTP/2 continuation-expected helper payload:
  [runtime-diagnostic-http2-continuation-helper-payload.md](runtime-diagnostic-http2-continuation-helper-payload.md).
- Runtime diagnostic HTTP/2 invalid frame-kind helper payload:
  [runtime-diagnostic-http2-invalid-frame-kind-helper-payload.md](runtime-diagnostic-http2-invalid-frame-kind-helper-payload.md).
- Runtime diagnostic HTTP/2 invalid stream id helper payload:
  [runtime-diagnostic-http2-invalid-stream-id-helper-payload.md](runtime-diagnostic-http2-invalid-stream-id-helper-payload.md).
- Runtime diagnostic HTTP/2 frame-size helper payload:
  [runtime-diagnostic-http2-frame-size-helper-payload.md](runtime-diagnostic-http2-frame-size-helper-payload.md).
- Runtime diagnostic HTTP/2 header-list helper payload:
  [runtime-diagnostic-http2-header-list-helper-payload.md](runtime-diagnostic-http2-header-list-helper-payload.md).
- Runtime diagnostic HTTP/2 header-list validation helper payload:
  [runtime-diagnostic-http2-header-list-validation-helper-payload.md](runtime-diagnostic-http2-header-list-validation-helper-payload.md).
- Runtime diagnostic HTTP/2 SETTINGS value helper payload:
  [runtime-diagnostic-http2-settings-value-helper-payload.md](runtime-diagnostic-http2-settings-value-helper-payload.md).
- Runtime diagnostic HTTP/2 header-table helper payload:
  [runtime-diagnostic-http2-header-table-helper-payload.md](runtime-diagnostic-http2-header-table-helper-payload.md).
- Runtime diagnostic HTTP/2 concurrent-streams helper payload:
  [runtime-diagnostic-http2-concurrent-streams-helper-payload.md](runtime-diagnostic-http2-concurrent-streams-helper-payload.md).
- Runtime diagnostic HTTP/2 payload-length helper payload:
  [runtime-diagnostic-http2-payload-length-helper-payload.md](runtime-diagnostic-http2-payload-length-helper-payload.md).
- Runtime diagnostic HTTP/2 helper payloads:
  [runtime-diagnostic-http2-helper-payloads.md](runtime-diagnostic-http2-helper-payloads.md).
- Runtime diagnostic HTTP/2 DATA padding and SETTINGS ACK helper payload:
  [runtime-diagnostic-http2-data-settings-helper-payload.md](runtime-diagnostic-http2-data-settings-helper-payload.md).
- Runtime diagnostic HTTP/2 side-table cleanup:
  [runtime-diagnostic-http2-side-table-cleanup.md](runtime-diagnostic-http2-side-table-cleanup.md).
- Runtime diagnostic test JSON payload:
  [runtime-diagnostic-test-json-payload.md](runtime-diagnostic-test-json-payload.md).
- Runtime diagnostic result value trace projection:
  [runtime-diagnostic-result-value-trace-projection.md](runtime-diagnostic-result-value-trace-projection.md).
- Runtime diagnostic payloads:
  [runtime-diagnostic-payload.md](runtime-diagnostic-payload.md).
- HTTP/2 HPACK string literal fixture:
  [http2-hpack-string-literal-fixture.md](http2-hpack-string-literal-fixture.md).
- HTTP/2 HPACK static name literal fixture:
  [http2-hpack-static-name-literal-fixture.md](http2-hpack-static-name-literal-fixture.md).
- HTTP/2 HPACK decoder foundation:
  [http2-hpack-decoder-foundation.md](http2-hpack-decoder-foundation.md).
- HTTP/2 outbound HPACK fixture encoder:
  [http2-outbound-hpack-fixture-encoder.md](http2-outbound-hpack-fixture-encoder.md).
- HTTP/2 outbound HPACK dynamic table eviction:
  [http2-outbound-hpack-dynamic-table-eviction.md](http2-outbound-hpack-dynamic-table-eviction.md).
- HTTP/2 outbound HPACK dynamic-name literal:
  [http2-outbound-hpack-dynamic-name-literal.md](http2-outbound-hpack-dynamic-name-literal.md).
- HTTP/2 outbound HPACK dynamic-name indexed literal:
  [http2-outbound-hpack-dynamic-name-indexed-literal.md](http2-outbound-hpack-dynamic-name-indexed-literal.md).
- HTTP/2 outbound HPACK dynamic-name never-indexed literal:
  [http2-outbound-hpack-dynamic-name-never-indexed-literal.md](http2-outbound-hpack-dynamic-name-never-indexed-literal.md).
- HTTP/2 outbound HPACK dynamic-name Huffman values:
  [http2-outbound-hpack-dynamic-name-huffman-values.md](http2-outbound-hpack-dynamic-name-huffman-values.md).
- HTTP/2 outbound HPACK Huffman literal names:
  [http2-outbound-hpack-huffman-literal-names.md](http2-outbound-hpack-huffman-literal-names.md).
- HTTP/2 outbound HPACK representation selection:
  [http2-outbound-hpack-representation-selection.md](http2-outbound-hpack-representation-selection.md).
- HTTP/2 production outbound HPACK header-list encoding:
  [http2-production-outbound-hpack-header-list-encoding.md](http2-production-outbound-hpack-header-list-encoding.md).
- HTTP/2 automatic outbound HPACK Huffman selection:
  [http2-automatic-outbound-hpack-huffman-selection.md](http2-automatic-outbound-hpack-huffman-selection.md).
- HTTP/2 outbound HPACK ordinary indexed literal:
  [http2-outbound-hpack-ordinary-indexed-literal.md](http2-outbound-hpack-ordinary-indexed-literal.md).
- HTTP/2 outbound HPACK static-name literal:
  [http2-outbound-hpack-static-name-literal.md](http2-outbound-hpack-static-name-literal.md).
- HTTP/2 outbound PUSH_PROMISE enable-push setting:
  [http2-outbound-push-promise-enable-push-setting.md](http2-outbound-push-promise-enable-push-setting.md).
- HTTP/2 outbound PUSH_PROMISE GOAWAY boundary:
  [http2-outbound-push-promise-goaway-boundary.md](http2-outbound-push-promise-goaway-boundary.md).
- HTTP/2 outbound promised stream id ordering:
  [http2-outbound-promised-stream-id-ordering.md](http2-outbound-promised-stream-id-ordering.md).
- HTTP/2 outbound local stream id ordering:
  [http2-outbound-local-stream-id-ordering.md](http2-outbound-local-stream-id-ordering.md).
- HTTP/2 outbound PRIORITY GOAWAY boundary:
  [http2-outbound-priority-goaway-boundary.md](http2-outbound-priority-goaway-boundary.md).
- HTTP/2 outbound PING request:
  [http2-outbound-ping-request.md](http2-outbound-ping-request.md).
- HTTP/2 SETTINGS ACK send state:
  [http2-settings-ack-send-state.md](http2-settings-ack-send-state.md).
- HTTP/2 SETTINGS item-length validation:
  [http2-settings-item-length-validation.md](http2-settings-item-length-validation.md).
- HTTP/2 client SETTINGS_ENABLE_PUSH rejection:
  [http2-client-settings-enable-push-rejection.md](http2-client-settings-enable-push-rejection.md).
- HTTP/2 local SETTINGS batch send:
  [http2-local-settings-batch-send.md](http2-local-settings-batch-send.md).
- HTTP/2 outbound DATA flow control:
  [http2-outbound-data-flow-control.md](http2-outbound-data-flow-control.md).
- HTTP/2 multi-stream outbound flow control:
  [http2-multi-stream-outbound-flow-control.md](http2-multi-stream-outbound-flow-control.md).
- HTTP/2 flow-control numeric domain types:
  [http2-flow-control-numeric-domain-types.md](http2-flow-control-numeric-domain-types.md).
- HTTP/2 outbound DATA GOAWAY boundary:
  [http2-outbound-data-goaway-boundary.md](http2-outbound-data-goaway-boundary.md).
- HTTP/2 outbound WINDOW_UPDATE GOAWAY boundary:
  [http2-outbound-window-update-goaway-boundary.md](http2-outbound-window-update-goaway-boundary.md).
- HTTP/2 GOAWAY receive lifecycle:
  [http2-goaway-receive-lifecycle.md](http2-goaway-receive-lifecycle.md).
- HTTP/2 GOAWAY drain completion:
  [http2-goaway-drain-completion.md](http2-goaway-drain-completion.md).
- HTTP/2 repeated outbound GOAWAY boundary:
  [http2-repeated-outbound-goaway-boundary.md](http2-repeated-outbound-goaway-boundary.md).
- HTTP/2 half-closed-by-peer outbound DATA:
  [http2-half-closed-by-peer-outbound-data.md](http2-half-closed-by-peer-outbound-data.md).
- HTTP/2 half-closed-local PRIORITY receive:
  [http2-half-closed-local-priority-receive.md](http2-half-closed-local-priority-receive.md).
- HTTP/2 client PUSH_PROMISE receive and promised response HEADERS admission:
  [http2-client-push-promise-receive.md](http2-client-push-promise-receive.md).
- HTTP/2 client promised stream id ordering:
  [http2-client-promised-stream-id-ordering.md](http2-client-promised-stream-id-ordering.md).
- HTTP/2 request header validation:
  [http2-request-header-validation.md](http2-request-header-validation.md).
- HTTP/2 CONNECT request header validation:
  [http2-connect-request-header-validation.md](http2-connect-request-header-validation.md).
- HTTP/2 extended CONNECT negotiation:
  [http2-extended-connect-negotiation.md](http2-extended-connect-negotiation.md).
- HTTP/2 response header validation:
  [http2-response-header-validation.md](http2-response-header-validation.md).
- HTTP/2 TE header validation:
  [http2-te-header-validation.md](http2-te-header-validation.md).
- HTTP/2 content-length header validation:
  [http2-content-length-header-validation.md](http2-content-length-header-validation.md).
- HTTP/2 content-length body accounting:
  [http2-content-length-body-accounting.md](http2-content-length-body-accounting.md).
- Function variadic arguments:
  [function-variadic-arguments.md](function-variadic-arguments.md).

## Skip Unless Needed

- Do not cite these records as current command, runtime, or language behavior.
- Return to `../../proposals/` for work that is absent or incomplete.
