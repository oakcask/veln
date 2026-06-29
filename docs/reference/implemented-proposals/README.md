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
- Local inference nested initializer expected type:
  [local-inference-nested-initializer-expected-type.md](local-inference-nested-initializer-expected-type.md).
- Local inference hole expected-type flow:
  [local-inference-hole-expected-type-flow.md](local-inference-hole-expected-type-flow.md).
- Local inference examples cleanup:
  [local-inference-examples-cleanup.md](local-inference-examples-cleanup.md).
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
- Schema documentation references:
  [schema-documentation-references.md](schema-documentation-references.md).
- Binary schema `UInt56be` and `UInt56le` primitives:
  [binary-schema-u56-primitives.md](binary-schema-u56-primitives.md).
- Binary schema `Flag40be`, `Flag40le`, `Flag56be`, and `Flag56le` bitsets:
  [binary-schema-flag40-and-flag56-bitsets.md](binary-schema-flag40-and-flag56-bitsets.md).
- Binary schema `Flag48be` and `Flag48le` bitsets:
  [binary-schema-flag48-bitsets.md](binary-schema-flag48-bitsets.md).
- Binary schema flag decode bindings:
  [binary-schema-flag-decode-bindings.md](binary-schema-flag-decode-bindings.md).
- Binary schema reserved-byte-prefix encode:
  [binary-schema-reserved-byte-prefix-encode.md](binary-schema-reserved-byte-prefix-encode.md).
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
- Binary schema repeat helper bindings:
  [binary-schema-repeat-schema-payload-helpers.md](binary-schema-repeat-schema-payload-helpers.md).
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
- Binary schema dispatch one-bit reserved payload helpers:
  [binary-schema-dispatch-one-bit-reserved-payload-helpers.md](binary-schema-dispatch-one-bit-reserved-payload-helpers.md).
- Binary schema dispatch reserved byte prefix payload helpers:
  [binary-schema-dispatch-reserved-byte-prefix-payload-helpers.md](binary-schema-dispatch-reserved-byte-prefix-payload-helpers.md).
- Binary schema `ByteView` payload multiple validation:
  [binary-schema-byteview-payload-multiple.md](binary-schema-byteview-payload-multiple.md).
- Binary schema dispatch payload helper boundary diagnostics:
  [binary-schema-dispatch-payload-helper-boundary-diagnostics.md](binary-schema-dispatch-payload-helper-boundary-diagnostics.md).
- Binary schema direction-specific dispatch payload helpers:
  [binary-schema-directional-dispatch-payload-helpers.md](binary-schema-directional-dispatch-payload-helpers.md).
- Binary schema mapped encode projection diagnostics:
  [binary-schema-mapped-encode-projection-diagnostics.md](binary-schema-mapped-encode-projection-diagnostics.md).
- Binary schema same-module recursive dispatch decode-only:
  [binary-schema-same-module-recursive-dispatch-decode-only.md](binary-schema-same-module-recursive-dispatch-decode-only.md).
- Binary schema mapping arithmetic encode:
  [binary-schema-mapping-arithmetic-encode.md](binary-schema-mapping-arithmetic-encode.md).
- Binary schema mapping converter varargs:
  [binary-schema-mapping-converter-varargs.md](binary-schema-mapping-converter-varargs.md).
- Binary schema imported converter bare inverse encode:
  [binary-schema-imported-converter-bare-inverse-encode.md](binary-schema-imported-converter-bare-inverse-encode.md).
- Codec generated helper boundary slices:
  [codec-generated-helper-boundary-slices.md](codec-generated-helper-boundary-slices.md).
- Codec hand-written encode resume:
  [codec-hand-written-encode-resume.md](codec-hand-written-encode-resume.md).
- Codec hand-written `NeedEnd` boundary:
  [codec-hand-written-need-end-boundary.md](codec-hand-written-need-end-boundary.md).
- Codec imported hand-written boundary:
  [codec-imported-hand-written-boundary.md](codec-imported-hand-written-boundary.md).
- Codec imported derived boundary:
  [codec-imported-derived-boundary.md](codec-imported-derived-boundary.md).
- Network adapter ownership boundary:
  [network-adapter-ownership-boundary.md](network-adapter-ownership-boundary.md).
- Network stream close boundary:
  [network-stream-close-boundary.md](network-stream-close-boundary.md).
- Network stream shutdown write boundary:
  [network-stream-shutdown-write-boundary.md](network-stream-shutdown-write-boundary.md).
- Network listener close boundary:
  [network-listener-close-boundary.md](network-listener-close-boundary.md).
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
- Network monotonic clock boundary:
  [network-monotonic-clock-boundary.md](network-monotonic-clock-boundary.md).
- Network deadline at boundary:
  [network-deadline-at-boundary.md](network-deadline-at-boundary.md).
- Network adapter outbound write ordering:
  [network-adapter-outbound-write-ordering.md](network-adapter-outbound-write-ordering.md).
- Network adapter clean shutdown:
  [network-adapter-clean-shutdown.md](network-adapter-clean-shutdown.md).
- Network cancel owner boundary:
  [network-cancel-owner-boundary.md](network-cancel-owner-boundary.md).
- Network cancel owner status:
  [network-cancel-owner-status.md](network-cancel-owner-status.md).
- Network production loopback lifecycle:
  [network-production-loopback-lifecycle.md](network-production-loopback-lifecycle.md).
- Network production cancellable deadline lifecycle:
  [network-production-cancellable-deadline-lifecycle.md](network-production-cancellable-deadline-lifecycle.md).
- Network production owner-drain lifecycle:
  [network-production-owner-drain-lifecycle.md](network-production-owner-drain-lifecycle.md).
- Network channel select-many routing:
  [network-channel-select-many-routing.md](network-channel-select-many-routing.md).
- Network channel select timeout result:
  [network-channel-select-timeout-result.md](network-channel-select-timeout-result.md).
- Network channel select timeout cancellable:
  [network-channel-select-timeout-cancellable.md](network-channel-select-timeout-cancellable.md).
- HTTP/2 unknown frame preservation:
  [http2-unknown-frame-preservation.md](http2-unknown-frame-preservation.md).
- HTTP/2 HPACK static indexed fixture:
  [http2-hpack-authority-static-indexed-fixture.md](http2-hpack-authority-static-indexed-fixture.md).
- HTTP/2 HPACK no-Huffman raw literal fixture:
  [http2-hpack-authority-literal-fixture.md](http2-hpack-authority-literal-fixture.md).
- HTTP/2 HPACK dynamic table fixture:
  [http2-hpack-dynamic-table-eviction-fixture.md](http2-hpack-dynamic-table-eviction-fixture.md).
- HTTP/2 HPACK table-size receive policy:
  [http2-hpack-table-size-policy.md](http2-hpack-table-size-policy.md).
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
- Runtime diagnostic generated schema fixed-field payload:
  [runtime-diagnostic-schema-fixed-field-payload.md](runtime-diagnostic-schema-fixed-field-payload.md).
- Codec-owned decode invalid id diagnostics:
  [codec-owned-decode-invalid-id-diagnostics.md](codec-owned-decode-invalid-id-diagnostics.md).
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
- HTTP/2 outbound HPACK dynamic-name literal:
  [http2-outbound-hpack-dynamic-name-literal.md](http2-outbound-hpack-dynamic-name-literal.md).
- HTTP/2 outbound PUSH_PROMISE enable-push setting:
  [http2-outbound-push-promise-enable-push-setting.md](http2-outbound-push-promise-enable-push-setting.md).
- HTTP/2 outbound DATA flow control:
  [http2-outbound-data-flow-control.md](http2-outbound-data-flow-control.md).
- HTTP/2 outbound DATA GOAWAY boundary:
  [http2-outbound-data-goaway-boundary.md](http2-outbound-data-goaway-boundary.md).
- HTTP/2 GOAWAY receive lifecycle:
  [http2-goaway-receive-lifecycle.md](http2-goaway-receive-lifecycle.md).
- HTTP/2 half-closed-by-peer outbound DATA:
  [http2-half-closed-by-peer-outbound-data.md](http2-half-closed-by-peer-outbound-data.md).
- HTTP/2 half-closed-local PRIORITY receive:
  [http2-half-closed-local-priority-receive.md](http2-half-closed-local-priority-receive.md).
- HTTP/2 client PUSH_PROMISE receive and promised response HEADERS admission:
  [http2-client-push-promise-receive.md](http2-client-push-promise-receive.md).
- HTTP/2 request header validation:
  [http2-request-header-validation.md](http2-request-header-validation.md).
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
