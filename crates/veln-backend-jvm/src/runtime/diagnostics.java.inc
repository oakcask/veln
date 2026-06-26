    public static void recordContractFailure(ContractFailure error) {
        String path = System.getenv("VELN_CONTRACT_ERRORS");
        if (path == null || path.isEmpty()) {
            return;
        }
        String line = "contract"
            + "\t" + error.clause
            + "\t" + hex(error.predicate)
            + "\t" + hex(error.function)
            + "\t" + error.blame
            + "\t" + hex(error.nodeId)
            + "\t" + hex(error.sourceFile)
            + "\t" + Integer.toString(error.startLine)
            + "\t" + Integer.toString(error.startColumn)
            + "\t" + Integer.toString(error.endLine)
            + "\t" + Integer.toString(error.endColumn)
            + System.lineSeparator();
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (java.io.IOException ioError) {
            throw new RuntimeException("failed to record contract error", ioError);
        }
    }

    public static void recordResultFailure(Object result) {
        String path = System.getenv("VELN_RESULT_ERRORS");
        if (path == null || path.isEmpty()) {
            return;
        }
        Object resultValue = asResult(result).value();
        String value = format(resultValue);
        registerEncodeErrorValueDiagnostic(resultValue, value);
        FixtureHexDiagnostic fixtureHex = fixtureHexDiagnostics.get().remove(value);
        RuntimeDiagnosticPayload runtimeDiagnostic = runtimeDiagnosticPayload(resultValue);
        ByteDiagnostic byteDiagnostic = runtimeDiagnostic == null
            ? byteDiagnostics.get().remove(value)
            : runtimeDiagnostic.byteDiagnostic;
        ValueDiagnostic valueDiagnostic = valueDiagnostics.get().remove(value);
        ProtocolDiagnostic protocolDiagnostic = runtimeDiagnostic == null
            ? protocolDiagnostics.get().remove(value)
            : runtimeDiagnostic.protocolDiagnostic;
        String line = "result"
            + "\t" + hex(value);
        if (fixtureHex != null) {
            line = line
                + "\tfixture_hex"
                + "\t" + fixtureHex.id
                + "\t" + Integer.toString(fixtureHex.textStart)
                + "\t" + Integer.toString(fixtureHex.textEnd)
                + "\t" + Integer.toString(fixtureHex.byteOffset)
                + "\t" + fixtureHex.nibble
                + "\t" + Integer.toString(fixtureHex.contextStart)
                + "\t" + Integer.toString(fixtureHex.contextEnd)
                + "\t" + hex(fixtureHex.context);
        }
        if (byteDiagnostic != null) {
            line = line
                + "\tbyte_diagnostic_v2"
                + "\t" + byteDiagnostic.id
                + "\t" + Long.toString(byteDiagnostic.byteOffset)
                + "\t" + Integer.toString(byteDiagnostic.fieldPath.size())
                + formatByteDiagnosticFieldPath(byteDiagnostic.fieldPath)
                + "\t" + Integer.toString(byteDiagnostic.details.size())
                + formatDiagnosticDetails(byteDiagnostic.details);
        }
        if (valueDiagnostic != null) {
            line = line
                + "\tvalue_diagnostic"
                + "\t" + valueDiagnostic.id
                + "\t" + Integer.toString(valueDiagnostic.fieldPath.size())
                + formatByteDiagnosticFieldPath(valueDiagnostic.fieldPath)
                + "\t" + Integer.toString(valueDiagnostic.details.size())
                + formatDiagnosticDetails(valueDiagnostic.details);
        }
        if (protocolDiagnostic != null) {
            line = line
                + "\tprotocol_diagnostic"
                + "\t" + protocolDiagnostic.id
                + "\t" + Long.toString(protocolDiagnostic.byteOffset)
                + "\t" + Integer.toString(protocolDiagnostic.details.size())
                + formatDiagnosticDetails(protocolDiagnostic.details);
        }
        line = line + System.lineSeparator();
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (java.io.IOException ioError) {
            throw new RuntimeException("failed to record result error", ioError);
        }
    }

    private static final ThreadLocal<java.util.Map<String, FixtureHexDiagnostic>> fixtureHexDiagnostics =
        ThreadLocal.withInitial(java.util.HashMap::new);

    private static final ThreadLocal<java.util.Map<String, ByteDiagnostic>> byteDiagnostics =
        ThreadLocal.withInitial(java.util.HashMap::new);

    private static final ThreadLocal<java.util.Map<String, ValueDiagnostic>> valueDiagnostics =
        ThreadLocal.withInitial(java.util.HashMap::new);

    private static final ThreadLocal<java.util.Map<String, ProtocolDiagnostic>> protocolDiagnostics =
        ThreadLocal.withInitial(java.util.HashMap::new);

    private static ByteDiagnostic byteDiagnosticForMessage(Object message) {
        if (!(message instanceof String)) {
            return null;
        }
        return byteDiagnostics.get().get(asString(message));
    }

    private static RuntimeDiagnosticPayload runtimeDiagnosticPayload(Object value) {
        if (!isAdtNamed(value, "RuntimeDiagnostic::RuntimeDiagnostic")) {
            return null;
        }
        Adt diagnostic = (Adt) value;
        String id = asString(diagnostic.payloads[0]);
        ByteDiagnostic byteDiagnostic = runtimeByteDiagnostic(id, diagnostic.payloads[2]);
        if (byteDiagnostic != null) {
            return new RuntimeDiagnosticPayload(byteDiagnostic, null);
        }
        ProtocolDiagnostic protocolDiagnostic =
            runtimeProtocolDiagnostic(id, diagnostic.payloads[2]);
        if (protocolDiagnostic != null) {
            return new RuntimeDiagnosticPayload(null, protocolDiagnostic);
        }
        return null;
    }

    private static ByteDiagnostic runtimeByteDiagnostic(String id, Object detail) {
        if (!isAdtNamed(detail, "RuntimeDiagnosticDetail::RuntimeByteDiagnostic")) {
            return null;
        }
        Adt byteDetail = (Adt) detail;
        long byteOffset = byteOffsetLong(byteDetail.payloads[0]);
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath =
            runtimeDiagnosticFieldPath(byteDetail.payloads[1]);
        if (fieldPath == null) {
            return null;
        }
        java.util.ArrayList<DiagnosticDetail> details =
            runtimeByteDiagnosticFacts(byteDetail.payloads[2]);
        if (details == null) {
            return null;
        }
        DiagnosticDetail preview = runtimeBytePreview(byteDetail.payloads[3]);
        if (preview != null) {
            details.add(preview);
        }
        return new ByteDiagnostic(id, byteOffset, fieldPath, details);
    }

    private static ProtocolDiagnostic runtimeProtocolDiagnostic(String id, Object detail) {
        ProtocolDiagnostic hpackDiagnostic = runtimeHpackFixtureDiagnostic(id, detail);
        if (hpackDiagnostic != null) {
            return hpackDiagnostic;
        }
        return runtimeHttp2ProtocolDiagnostic(id, detail);
    }

    private static ProtocolDiagnostic runtimeHpackFixtureDiagnostic(String id, Object detail) {
        if (isAdtNamed(detail, "RuntimeDiagnosticDetail::RuntimeHpackFixtureDiagnostic")) {
            return runtimeHpackFixtureCommonDiagnostic(id, (Adt) detail);
        }
        if (isAdtNamed(detail, "RuntimeDiagnosticDetail::RuntimeHpackFixtureDynamicIndexDiagnostic")) {
            return runtimeHpackFixtureDynamicIndexDiagnostic(id, (Adt) detail);
        }
        if (isAdtNamed(detail, "RuntimeDiagnosticDetail::RuntimeHpackFixtureTableSizeUpdateDiagnostic")) {
            return runtimeHpackFixtureTableSizeUpdateDiagnostic(id, (Adt) detail);
        }
        return null;
    }

    private static ProtocolDiagnostic runtimeHpackFixtureCommonDiagnostic(String id, Adt detail) {
        if (!id.equals("hpack.fixture.unsupported_header_block")
            && !id.equals("hpack.fixture.malformed_string_length")
            && !id.equals("hpack.fixture.malformed_raw_string_value")
            && !id.equals("hpack.fixture.malformed_huffman_padding")
            && !id.equals("hpack.fixture.huffman_eos_symbol")
            && !id.equals("hpack.fixture.huffman_non_visible_value")) {
            return null;
        }
        long byteOffset = asLong(detail.payloads[0]);
        long observedSize = asLong(detail.payloads[1]);
        long observedFirstByte = asLong(detail.payloads[2]);
        String expectedFixture = asString(detail.payloads[3]);
        String codecModule = asString(detail.payloads[4]);
        String nearbyContext = byteChunkPreviewTrace(detail.payloads[5], 8);
        return new ProtocolDiagnostic(
            id,
            byteOffset,
            java.util.Arrays.asList(
                DiagnosticDetail.number("observed_header_block_size", observedSize),
                DiagnosticDetail.number("observed_first_byte", observedFirstByte),
                DiagnosticDetail.string("expected_fixture", expectedFixture),
                DiagnosticDetail.string("codec_module", codecModule),
                DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
            )
        );
    }

    private static ProtocolDiagnostic runtimeHpackFixtureDynamicIndexDiagnostic(
        String id,
        Adt detail
    ) {
        if (!id.equals("hpack.fixture.dynamic_index_out_of_range")) {
            return null;
        }
        long byteOffset = asLong(detail.payloads[0]);
        long observedSize = asLong(detail.payloads[1]);
        long observedFirstByte = asLong(detail.payloads[2]);
        long requestedDynamicIndex = asLong(detail.payloads[3]);
        long dynamicTableEntryCount = asLong(detail.payloads[4]);
        String expectedFixture = asString(detail.payloads[5]);
        String codecModule = asString(detail.payloads[6]);
        String nearbyContext = byteChunkPreviewTrace(detail.payloads[7], 8);
        return new ProtocolDiagnostic(
            id,
            byteOffset,
            java.util.Arrays.asList(
                DiagnosticDetail.number("observed_header_block_size", observedSize),
                DiagnosticDetail.number("observed_first_byte", observedFirstByte),
                DiagnosticDetail.number("requested_dynamic_index", requestedDynamicIndex),
                DiagnosticDetail.number("dynamic_table_entry_count", dynamicTableEntryCount),
                DiagnosticDetail.string("expected_fixture", expectedFixture),
                DiagnosticDetail.string("codec_module", codecModule),
                DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
            )
        );
    }

    private static ProtocolDiagnostic runtimeHpackFixtureTableSizeUpdateDiagnostic(
        String id,
        Adt detail
    ) {
        if (!id.equals("hpack.fixture.table_size_update_not_at_start")) {
            return null;
        }
        long byteOffset = asLong(detail.payloads[0]);
        long observedSize = asLong(detail.payloads[1]);
        long observedFirstByte = asLong(detail.payloads[2]);
        long observedHeaderTableSize = asLong(detail.payloads[3]);
        long frameKind = asLong(detail.payloads[4]);
        long streamId = asLong(detail.payloads[5]);
        String activeState = asString(detail.payloads[6]);
        String expectedFixture = asString(detail.payloads[7]);
        String codecModule = asString(detail.payloads[8]);
        String nearbyContext = byteChunkPreviewTrace(detail.payloads[9], 8);
        return new ProtocolDiagnostic(
            id,
            byteOffset,
            java.util.Arrays.asList(
                DiagnosticDetail.number("observed_header_block_size", observedSize),
                DiagnosticDetail.number("observed_first_byte", observedFirstByte),
                DiagnosticDetail.number("observed_header_table_size", observedHeaderTableSize),
                DiagnosticDetail.number("frame_kind", frameKind),
                DiagnosticDetail.number("stream_id", streamId),
                DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                DiagnosticDetail.string("active_state", activeState),
                DiagnosticDetail.string("expected_fixture", expectedFixture),
                DiagnosticDetail.string("codec_module", codecModule),
                DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
            )
        );
    }

    private static ProtocolDiagnostic runtimeHttp2ProtocolDiagnostic(String id, Object detail) {
        if (isAdtNamed(detail, "RuntimeDiagnosticDetail::RuntimeHttp2ProtocolInvalidFrameKindDiagnostic")) {
            return runtimeHttp2ProtocolInvalidFrameKindDiagnostic(id, (Adt) detail);
        }
        if (isAdtNamed(detail, "RuntimeDiagnosticDetail::RuntimeHttp2PeerLimitFrameSizeDiagnostic")) {
            return runtimeHttp2PeerLimitFrameSizeDiagnostic(id, (Adt) detail);
        }
        if (isAdtNamed(detail, "RuntimeDiagnosticDetail::RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic")) {
            return runtimeHttp2ProtocolInvalidPayloadLengthDiagnostic(id, (Adt) detail);
        }
        if (isAdtNamed(detail, "RuntimeDiagnosticDetail::RuntimeHttp2ProtocolPriorityDependencyDiagnostic")) {
            return runtimeHttp2ProtocolPriorityDependencyDiagnostic(id, (Adt) detail);
        }
        if (isAdtNamed(detail, "RuntimeDiagnosticDetail::RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic")) {
            return runtimeHttp2ProtocolStreamAfterGoawayDiagnostic(id, (Adt) detail);
        }
        return null;
    }

    private static ProtocolDiagnostic runtimeHttp2ProtocolInvalidFrameKindDiagnostic(
        String id,
        Adt detail
    ) {
        if (!id.equals("http2.protocol.invalid_frame_kind")) {
            return null;
        }
        long byteOffset = asLong(detail.payloads[0]);
        long actualKind = asLong(detail.payloads[1]);
        long streamId = asLong(detail.payloads[2]);
        long expectedKind = asLong(detail.payloads[3]);
        String activeState = asString(detail.payloads[4]);
        String ruleProvenance = asString(detail.payloads[5]);
        String nearbyContext = byteChunkPreviewTrace(detail.payloads[6], 8);
        return new ProtocolDiagnostic(
            id,
            byteOffset,
            java.util.Arrays.asList(
                DiagnosticDetail.number("actual_frame_kind", actualKind),
                DiagnosticDetail.number("stream_id", streamId),
                DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                DiagnosticDetail.number("expected_frame_kind", expectedKind),
                DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                DiagnosticDetail.string("active_state", activeState),
                DiagnosticDetail.string("rule_provenance", ruleProvenance)
            )
        );
    }

    private static ProtocolDiagnostic runtimeHttp2PeerLimitFrameSizeDiagnostic(
        String id,
        Adt detail
    ) {
        if (!id.equals("http2.peer_limit.frame_size_exceeded")) {
            return null;
        }
        long byteOffset = asLong(detail.payloads[0]);
        long observedLength = asLong(detail.payloads[1]);
        long allowedLength = asLong(detail.payloads[2]);
        long frameKind = asLong(detail.payloads[3]);
        long streamId = asLong(detail.payloads[4]);
        String receiveLimitProvenance = asString(detail.payloads[5]);
        return new ProtocolDiagnostic(
            id,
            byteOffset,
            java.util.Arrays.asList(
                DiagnosticDetail.number("observed_payload_length", observedLength),
                DiagnosticDetail.number("allowed_max_frame_size", allowedLength),
                DiagnosticDetail.number("frame_kind", frameKind),
                DiagnosticDetail.number("stream_id", streamId),
                DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                DiagnosticDetail.string("receive_limit_provenance", receiveLimitProvenance)
            )
        );
    }

    private static ProtocolDiagnostic runtimeHttp2ProtocolInvalidPayloadLengthDiagnostic(
        String id,
        Adt detail
    ) {
        if (!id.equals("http2.protocol.invalid_payload_length")) {
            return null;
        }
        long byteOffset = asLong(detail.payloads[0]);
        long frameKind = asLong(detail.payloads[1]);
        long streamId = asLong(detail.payloads[2]);
        long observedLength = asLong(detail.payloads[3]);
        long expectedLength = asLong(detail.payloads[4]);
        String activeState = asString(detail.payloads[5]);
        String ruleProvenance = asString(detail.payloads[6]);
        String nearbyContext = byteChunkPreviewTrace(detail.payloads[7], 8);
        return new ProtocolDiagnostic(
            id,
            byteOffset,
            java.util.Arrays.asList(
                DiagnosticDetail.number("frame_kind", frameKind),
                DiagnosticDetail.number("stream_id", streamId),
                DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                DiagnosticDetail.number("observed_payload_length", observedLength),
                DiagnosticDetail.number("expected_payload_length", expectedLength),
                DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                DiagnosticDetail.string("active_state", activeState),
                DiagnosticDetail.string("rule_provenance", ruleProvenance)
            )
        );
    }

    private static ProtocolDiagnostic runtimeHttp2ProtocolPriorityDependencyDiagnostic(
        String id,
        Adt detail
    ) {
        if (!id.equals("http2.protocol.invalid_priority_dependency")) {
            return null;
        }
        long byteOffset = asLong(detail.payloads[0]);
        long streamId = asLong(detail.payloads[1]);
        long dependencyStreamId = asLong(detail.payloads[2]);
        String activeState = asString(detail.payloads[3]);
        String ruleProvenance = asString(detail.payloads[4]);
        String nearbyContext = byteChunkPreviewTrace(detail.payloads[5], 8);
        return new ProtocolDiagnostic(
            id,
            byteOffset,
            java.util.Arrays.asList(
                DiagnosticDetail.number("frame_kind", 2),
                DiagnosticDetail.number("stream_id", streamId),
                DiagnosticDetail.string("stream_ref", "stream"),
                DiagnosticDetail.number("dependency_stream_id", dependencyStreamId),
                DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                DiagnosticDetail.string("active_state", activeState),
                DiagnosticDetail.string("rule_provenance", ruleProvenance)
            )
        );
    }

    private static ProtocolDiagnostic runtimeHttp2ProtocolStreamAfterGoawayDiagnostic(
        String id,
        Adt detail
    ) {
        if (!id.equals("http2.protocol.stream_after_goaway")) {
            return null;
        }
        long byteOffset = asLong(detail.payloads[0]);
        long streamId = asLong(detail.payloads[1]);
        long lastStreamId = asLong(detail.payloads[2]);
        String shutdownState = asString(detail.payloads[3]);
        String endpointRole = asString(detail.payloads[4]);
        String ruleProvenance = asString(detail.payloads[5]);
        return new ProtocolDiagnostic(
            id,
            byteOffset,
            java.util.Arrays.asList(
                DiagnosticDetail.number("stream_id", streamId),
                DiagnosticDetail.string("stream_ref", "stream"),
                DiagnosticDetail.number("last_stream_id", lastStreamId),
                DiagnosticDetail.string("shutdown_state", shutdownState),
                DiagnosticDetail.string("endpoint_role", endpointRole),
                DiagnosticDetail.string("active_state", shutdownState),
                DiagnosticDetail.string("rule_provenance", ruleProvenance)
            )
        );
    }

    private static String byteChunkPreviewTrace(Object chunk, int maxPreviewBytes) {
        java.util.List<Object> bytes = byteChunkBytes(chunk);
        long count = bytes.size();
        long previewCount = Math.min(count, Math.max(0, maxPreviewBytes));
        StringBuilder out = new StringBuilder((int) (previewCount * 2));
        char[] digits = "0123456789abcdef".toCharArray();
        for (long index = 0; index < previewCount; index += 1) {
            int value = (int) byteValueLong(bytes.get((int) index));
            out.append(digits[value >>> 4]);
            out.append(digits[value & 0x0f]);
        }
        return hex(out.toString())
            + ":"
            + previewCount
            + ":"
            + count
            + ":"
            + Boolean.toString(count > previewCount);
    }

    private static java.util.List<ByteDiagnosticFieldPathSegment> runtimeDiagnosticFieldPath(
        Object value
    ) {
        java.util.ArrayList<ByteDiagnosticFieldPathSegment> path =
            new java.util.ArrayList<ByteDiagnosticFieldPathSegment>();
        Object cursor = value;
        while (true) {
            ListValue list = asListValue(cursor);
            if (!list.cons) {
                return path;
            }
            if (!isAdtNamed(list.head, "RuntimeDiagnosticFieldPathSegment::RuntimeDiagnosticFieldPathSegment")) {
                return null;
            }
            Adt segment = (Adt) list.head;
            path.add(new ByteDiagnosticFieldPathSegment(
                asString(segment.payloads[0]),
                asString(segment.payloads[1])
            ));
            cursor = list.tail;
        }
    }

    private static java.util.ArrayList<DiagnosticDetail> runtimeByteDiagnosticFacts(Object value) {
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        if (isAdtNamed(value, "RuntimeByteDiagnosticFacts::RuntimeByteCountFacts")) {
            Adt facts = (Adt) value;
            details.add(DiagnosticDetail.number("expected_count", byteCountLong(facts.payloads[0])));
            details.add(DiagnosticDetail.number("available_count", byteCountLong(facts.payloads[1])));
            details.add(DiagnosticDetail.string("readiness", asString(facts.payloads[2])));
            return details;
        }
        if (isAdtNamed(value, "RuntimeByteDiagnosticFacts::RuntimeByteRangeFacts")) {
            Adt facts = (Adt) value;
            details.add(DiagnosticDetail.number("requested_count", byteCountLong(facts.payloads[0])));
            details.add(DiagnosticDetail.number("available_count", byteCountLong(facts.payloads[1])));
            return details;
        }
        if (isAdtNamed(value, "RuntimeByteDiagnosticFacts::RuntimeByteReasonFacts")) {
            Adt facts = (Adt) value;
            details.add(DiagnosticDetail.string("reason", asString(facts.payloads[0])));
            return details;
        }
        return null;
    }

    private static DiagnosticDetail runtimeBytePreview(Object value) {
        if (isAdtNamed(value, "RuntimeBytePreview::NoRuntimeBytePreview")) {
            return null;
        }
        if (!isAdtNamed(value, "RuntimeBytePreview::RuntimeBytePreview")) {
            return null;
        }
        Adt preview = (Adt) value;
        String data = asString(preview.payloads[0]);
        long previewByteCount = byteCountLong(preview.payloads[1]);
        long totalByteCount = byteCountLong(preview.payloads[2]);
        boolean truncated = asBool(preview.payloads[3]);
        return DiagnosticDetail.bytePreviewV2(
            "byte_preview",
            hex(data)
                + ":"
                + Long.toString(previewByteCount)
                + ":"
                + Long.toString(totalByteCount)
                + ":"
                + Boolean.toString(truncated)
        );
    }

    private static void registerHttp2ProtocolClosedWithPendingDiagnostic(
        String message,
        long byteOffset,
        long pendingCount,
        String activeContinuation,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.closed_with_pending",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("pending_count", pendingCount),
                    DiagnosticDetail.string("input_event", "end"),
                    DiagnosticDetail.string("active_continuation", activeContinuation),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerHttp2ProtocolPartialPrefaceDiagnostic(
        String message,
        long byteOffset,
        long pendingCount,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.partial_preface",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("pending_count", pendingCount),
                    DiagnosticDetail.number("expected_count", 24),
                    DiagnosticDetail.string("input_event", "end"),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", "connection-preface"),
                    DiagnosticDetail.string("rule_provenance", "rfc9113_client_connection_preface")
                )
            )
        );
    }

    private static void registerHttp2ProtocolInvalidPrefaceDiagnostic(
        String message,
        long byteOffset,
        long expectedByte,
        long actualByte,
        long matchedCount,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.invalid_preface",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("expected_byte", expectedByte),
                    DiagnosticDetail.number("actual_byte", actualByte),
                    DiagnosticDetail.number("matched_prefix_count", matchedCount),
                    DiagnosticDetail.number("expected_count", 24),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", "connection-preface"),
                    DiagnosticDetail.string("rule_provenance", "rfc9113_client_connection_preface")
                )
            )
        );
    }

    private static void registerHttp2ProtocolContinuationExpectedDiagnostic(
        String message,
        long byteOffset,
        long actualKind,
        long actualStream,
        long expectedStream,
        long startedKind,
        long startedOffset,
        String activeContinuation,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.continuation_expected",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("actual_frame_kind", actualKind),
                    DiagnosticDetail.number("actual_stream_id", actualStream),
                    DiagnosticDetail.number("expected_stream_id", expectedStream),
                    DiagnosticDetail.number("started_frame_kind", startedKind),
                    DiagnosticDetail.number("started_byte_offset", startedOffset),
                    DiagnosticDetail.string("active_continuation", activeContinuation),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerHttp2ProtocolInvalidFrameKindDiagnostic(
        String message,
        long byteOffset,
        long actualKind,
        long streamId,
        long expectedKind,
        String activeState,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.invalid_frame_kind",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("actual_frame_kind", actualKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.number("expected_frame_kind", expectedKind),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolInvalidStreamIdDiagnostic(
        String message,
        long byteOffset,
        long frameKind,
        long streamId,
        String requiredDomain,
        String endpointRole,
        String activeState,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.invalid_stream_id",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.string("required_stream_id_domain", requiredDomain),
                    DiagnosticDetail.string("endpoint_role", endpointRole),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolInvalidPayloadLengthDiagnostic(
        String message,
        long byteOffset,
        long frameKind,
        long streamId,
        long observedLength,
        long expectedLength,
        String activeState,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.invalid_payload_length",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.number("observed_payload_length", observedLength),
                    DiagnosticDetail.number("expected_payload_length", expectedLength),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(
        String message,
        long byteOffset,
        long streamId,
        long observedIncrement,
        long acceptedMinIncrement,
        long acceptedMaxIncrement,
        String activeState,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.invalid_window_update_increment",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("frame_kind", 8),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.number("observed_window_increment", observedIncrement),
                    DiagnosticDetail.number("accepted_min_window_increment", acceptedMinIncrement),
                    DiagnosticDetail.number("accepted_max_window_increment", acceptedMaxIncrement),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolInvalidDataPaddingDiagnostic(
        String message,
        long byteOffset,
        long streamId,
        long padLength,
        long remainingPayloadLength,
        String activeState,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.invalid_data_padding",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("frame_kind", 0),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", "stream"),
                    DiagnosticDetail.number("pad_length", padLength),
                    DiagnosticDetail.number("remaining_payload_length", remainingPayloadLength),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolContentLengthMismatchDiagnostic(
        String message,
        long byteOffset,
        long frameKind,
        long streamId,
        long expectedLength,
        long observedLength,
        String activeState,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.content_length_mismatch",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.number("expected_content_length", expectedLength),
                    DiagnosticDetail.number("observed_body_length", observedLength),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolUnexpectedSettingsAckDiagnostic(
        String message,
        long byteOffset,
        String activeState,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.unexpected_settings_ack",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("frame_kind", 4),
                    DiagnosticDetail.number("stream_id", 0),
                    DiagnosticDetail.string("stream_ref", "connection"),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolInvalidRequestHeaderListDiagnostic(
        String message,
        long byteOffset,
        long frameKind,
        long streamId,
        String failedHeaderFact,
        String headerName,
        String decodedHeaderNames,
        String activeState,
        String ruleProvenance
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.invalid_request_header_list",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.string("failed_header_fact", failedHeaderFact),
                    DiagnosticDetail.string("header_name", headerName),
                    DiagnosticDetail.string("decoded_header_names", decodedHeaderNames),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolInvalidResponseHeaderListDiagnostic(
        String message,
        long byteOffset,
        long frameKind,
        long streamId,
        String failedHeaderFact,
        String headerName,
        String decodedHeaderNames,
        String activeState,
        String ruleProvenance
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.invalid_response_header_list",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.string("failed_header_fact", failedHeaderFact),
                    DiagnosticDetail.string("header_name", headerName),
                    DiagnosticDetail.string("decoded_header_names", decodedHeaderNames),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolInvalidPriorityDependencyDiagnostic(
        String message,
        long byteOffset,
        long streamId,
        long dependencyStreamId,
        String activeState,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.invalid_priority_dependency",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("frame_kind", 2),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", "stream"),
                    DiagnosticDetail.number("dependency_stream_id", dependencyStreamId),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2ProtocolStreamAfterGoawayDiagnostic(
        String message,
        long byteOffset,
        long streamId,
        long lastStreamId,
        String shutdownState,
        String endpointRole,
        String ruleProvenance
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.stream_after_goaway",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", "stream"),
                    DiagnosticDetail.number("last_stream_id", lastStreamId),
                    DiagnosticDetail.string("shutdown_state", shutdownState),
                    DiagnosticDetail.string("endpoint_role", endpointRole),
                    DiagnosticDetail.string("active_state", shutdownState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2PeerLimitFrameSizeExceededDiagnostic(
        String message,
        long byteOffset,
        long observedLength,
        long allowedLength,
        long frameKind,
        long streamId,
        String receiveLimitProvenance
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.peer_limit.frame_size_exceeded",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("observed_payload_length", observedLength),
                    DiagnosticDetail.number("allowed_max_frame_size", allowedLength),
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.string("receive_limit_provenance", receiveLimitProvenance)
                )
            )
        );
    }

    private static void registerHttp2PeerLimitHeaderListSizeExceededDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long allowedSize,
        long frameKind,
        long streamId,
        String receiveLimitProvenance,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.peer_limit.header_list_size_exceeded",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("observed_header_list_size", observedSize),
                    DiagnosticDetail.number("allowed_header_list_size", allowedSize),
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.string("receive_limit_provenance", receiveLimitProvenance),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerHttp2PeerLimitHeaderTableSizeExceededDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long allowedSize,
        long frameKind,
        long streamId,
        String receiveLimitProvenance,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.peer_limit.header_table_size_exceeded",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("observed_header_table_size", observedSize),
                    DiagnosticDetail.number("allowed_header_table_size", allowedSize),
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.string("receive_limit_provenance", receiveLimitProvenance),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerHttp2PeerLimitFlowControlWindowExceededDiagnostic(
        String message,
        long byteOffset,
        long observedLength,
        long allowedWindowCredit,
        long frameKind,
        long streamId,
        String activeState,
        String ruleProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.peer_limit.flow_control_window_exceeded",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("observed_payload_length", observedLength),
                    DiagnosticDetail.number("allowed_window_credit", allowedWindowCredit),
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerHttp2PeerLimitConcurrentStreamsExceededDiagnostic(
        String message,
        long byteOffset,
        long streamId,
        long attemptedCount,
        long allowedCount,
        String endpointRole,
        String activeState,
        String receiveLimitProvenance,
        String ruleProvenance
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.peer_limit.concurrent_streams_exceeded",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.number(
                        "current_open_peer_created_stream_count",
                        attemptedCount - 1
                    ),
                    DiagnosticDetail.number("attempted_concurrent_stream_count", attemptedCount),
                    DiagnosticDetail.number("allowed_concurrent_stream_count", allowedCount),
                    DiagnosticDetail.string("endpoint_role", endpointRole),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("receive_limit_provenance", receiveLimitProvenance),
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
                )
            )
        );
    }

    private static void registerHttp2PeerLimitSettingsValueOutOfRangeDiagnostic(
        String message,
        long byteOffset,
        long settingIdentifier,
        String settingName,
        long observedValue,
        long acceptedMinValue,
        long acceptedMaxValue,
        String peerLimitProvenance,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.peer_limit.settings_value_out_of_range",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("setting_identifier", settingIdentifier),
                    DiagnosticDetail.string("setting_name", settingName),
                    DiagnosticDetail.number("observed_value", observedValue),
                    DiagnosticDetail.number("accepted_min_value", acceptedMinValue),
                    DiagnosticDetail.number("accepted_max_value", acceptedMaxValue),
                    DiagnosticDetail.string("peer_limit_provenance", peerLimitProvenance),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerHpackFixtureUnsupportedHeaderBlockDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long observedFirstByte,
        String expectedFixture,
        String codecModule,
        String nearbyContext
    ) {
        registerHpackFixtureHeaderBlockDiagnostic(
            message,
            "hpack.fixture.unsupported_header_block",
            byteOffset,
            observedSize,
            observedFirstByte,
            expectedFixture,
            codecModule,
            nearbyContext
        );
    }

    private static void registerHpackFixtureMalformedStringLengthDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long observedFirstByte,
        String expectedFixture,
        String codecModule,
        String nearbyContext
    ) {
        registerHpackFixtureHeaderBlockDiagnostic(
            message,
            "hpack.fixture.malformed_string_length",
            byteOffset,
            observedSize,
            observedFirstByte,
            expectedFixture,
            codecModule,
            nearbyContext
        );
    }

    private static void registerHpackFixtureMalformedRawStringValueDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long observedFirstByte,
        String expectedFixture,
        String codecModule,
        String nearbyContext
    ) {
        registerHpackFixtureHeaderBlockDiagnostic(
            message,
            "hpack.fixture.malformed_raw_string_value",
            byteOffset,
            observedSize,
            observedFirstByte,
            expectedFixture,
            codecModule,
            nearbyContext
        );
    }

    private static void registerHpackFixtureMalformedHuffmanPaddingDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long observedFirstByte,
        String expectedFixture,
        String codecModule,
        String nearbyContext
    ) {
        registerHpackFixtureHeaderBlockDiagnostic(
            message,
            "hpack.fixture.malformed_huffman_padding",
            byteOffset,
            observedSize,
            observedFirstByte,
            expectedFixture,
            codecModule,
            nearbyContext
        );
    }

    private static void registerHpackFixtureHuffmanEosSymbolDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long observedFirstByte,
        String expectedFixture,
        String codecModule,
        String nearbyContext
    ) {
        registerHpackFixtureHeaderBlockDiagnostic(
            message,
            "hpack.fixture.huffman_eos_symbol",
            byteOffset,
            observedSize,
            observedFirstByte,
            expectedFixture,
            codecModule,
            nearbyContext
        );
    }

    private static void registerHpackFixtureHuffmanNonVisibleValueDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long observedFirstByte,
        String expectedFixture,
        String codecModule,
        String nearbyContext
    ) {
        registerHpackFixtureHeaderBlockDiagnostic(
            message,
            "hpack.fixture.huffman_non_visible_value",
            byteOffset,
            observedSize,
            observedFirstByte,
            expectedFixture,
            codecModule,
            nearbyContext
        );
    }

    private static void registerHpackFixtureDynamicIndexOutOfRangeDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long observedFirstByte,
        long requestedDynamicIndex,
        long dynamicTableEntryCount,
        String expectedFixture,
        String codecModule,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "hpack.fixture.dynamic_index_out_of_range",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("observed_header_block_size", observedSize),
                    DiagnosticDetail.number("observed_first_byte", observedFirstByte),
                    DiagnosticDetail.number("requested_dynamic_index", requestedDynamicIndex),
                    DiagnosticDetail.number("dynamic_table_entry_count", dynamicTableEntryCount),
                    DiagnosticDetail.string("expected_fixture", expectedFixture),
                    DiagnosticDetail.string("codec_module", codecModule),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerHpackFixtureTableSizeUpdateNotAtStartDiagnostic(
        String message,
        long byteOffset,
        long observedSize,
        long observedFirstByte,
        long observedUpdateSize,
        long frameKind,
        long streamId,
        String activeState,
        String expectedFixture,
        String codecModule,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "hpack.fixture.table_size_update_not_at_start",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("observed_header_block_size", observedSize),
                    DiagnosticDetail.number("observed_first_byte", observedFirstByte),
                    DiagnosticDetail.number("observed_header_table_size", observedUpdateSize),
                    DiagnosticDetail.number("frame_kind", frameKind),
                    DiagnosticDetail.number("stream_id", streamId),
                    DiagnosticDetail.string("stream_ref", streamId == 0 ? "connection" : "stream"),
                    DiagnosticDetail.string("active_state", activeState),
                    DiagnosticDetail.string("expected_fixture", expectedFixture),
                    DiagnosticDetail.string("codec_module", codecModule),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerHpackFixtureHeaderBlockDiagnostic(
        String message,
        String id,
        long byteOffset,
        long observedSize,
        long observedFirstByte,
        String expectedFixture,
        String codecModule,
        String nearbyContext
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                id,
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("observed_header_block_size", observedSize),
                    DiagnosticDetail.number("observed_first_byte", observedFirstByte),
                    DiagnosticDetail.string("expected_fixture", expectedFixture),
                    DiagnosticDetail.string("codec_module", codecModule),
                    DiagnosticDetail.bytePreviewV2("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerCodecIncompleteInputDiagnostic(
        String message,
        long byteOffset,
        long expectedCount,
        long availableCount
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "codec.incomplete_input",
                byteOffset,
                java.util.Collections.emptyList(),
                java.util.Arrays.asList(
                    DiagnosticDetail.number("expected_count", expectedCount),
                    DiagnosticDetail.number("available_count", availableCount),
                    DiagnosticDetail.string("readiness", "need_bytes")
                )
            )
        );
    }

    private static void registerCodecByteRangeOutOfBoundsDiagnostic(
        String message,
        long byteOffset,
        long requestedCount,
        long availableCount,
        String nearbyContext
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "codec.byte_range_out_of_bounds",
                byteOffset,
                java.util.Collections.emptyList(),
                java.util.Arrays.asList(
                    DiagnosticDetail.number("requested_count", requestedCount),
                    DiagnosticDetail.number("available_count", availableCount),
                    DiagnosticDetail.bytePreview("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerSchemaTruncatedFieldDiagnostic(
        String message,
        long byteOffset,
        String schemaName,
        String fieldName,
        long expectedCount,
        long availableCount,
        String nearbyContext
    ) {
        registerSchemaTruncatedFieldPathDiagnostic(
            message,
            byteOffset,
            schemaFieldPath(schemaName, fieldName),
            expectedCount,
            availableCount,
            nearbyContext
        );
    }

    private static void registerSchemaTruncatedFieldPathDiagnostic(
        String message,
        long byteOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
        long expectedCount,
        long availableCount,
        String nearbyContext
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.truncated_field",
                byteOffset,
                fieldPath,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("expected_count", expectedCount),
                    DiagnosticDetail.number("available_count", availableCount),
                    DiagnosticDetail.string("readiness", "need_bytes"),
                    DiagnosticDetail.bytePreview("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerSchemaFixedFieldMismatchDiagnostic(
        String message,
        long byteOffset,
        String schemaName,
        String fieldName,
        long expectedValue,
        long actualValue,
        String nearbyContext
    ) {
        registerSchemaFixedFieldMismatchPathDiagnostic(
            message,
            byteOffset,
            schemaFieldPath(schemaName, fieldName),
            expectedValue,
            actualValue,
            nearbyContext
        );
    }

    private static void registerSchemaFixedFieldMismatchPathDiagnostic(
        String message,
        long byteOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
        long expectedValue,
        long actualValue,
        String nearbyContext
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.fixed_field_mismatch",
                byteOffset,
                fieldPath,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("expected_value", expectedValue),
                    DiagnosticDetail.number("actual_value", actualValue),
                    DiagnosticDetail.bytePreview("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerSchemaReservedBitsMismatchDiagnostic(
        String message,
        long byteOffset,
        String schemaName,
        String fieldName,
        long bitWidth,
        long expectedValue,
        long actualValue,
        String nearbyContext
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.reserved_bits_mismatch",
                byteOffset,
                java.util.Arrays.asList(
                    new ByteDiagnosticFieldPathSegment("schema", schemaName),
                    new ByteDiagnosticFieldPathSegment("field", fieldName)
                ),
                java.util.Arrays.asList(
                    DiagnosticDetail.number("bit_width", bitWidth),
                    DiagnosticDetail.number("expected_value", expectedValue),
                    DiagnosticDetail.number("actual_value", actualValue),
                    DiagnosticDetail.bytePreview("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerSchemaLengthOutOfBoundsDiagnostic(
        String message,
        long byteOffset,
        String schemaName,
        String fieldName,
        long expectedCount,
        long availableCount,
        String nearbyContext
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.length_out_of_bounds",
                byteOffset,
                java.util.Arrays.asList(
                    new ByteDiagnosticFieldPathSegment("schema", schemaName),
                    new ByteDiagnosticFieldPathSegment("field", fieldName)
                ),
                java.util.Arrays.asList(
                    DiagnosticDetail.number("expected_count", expectedCount),
                    DiagnosticDetail.number("available_count", availableCount),
                    DiagnosticDetail.bytePreview("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerSchemaLengthOutOfBoundsPathDiagnostic(
        String message,
        long byteOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
        long expectedCount,
        long availableCount,
        String nearbyContext
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.length_out_of_bounds",
                byteOffset,
                fieldPath,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("expected_count", expectedCount),
                    DiagnosticDetail.number("available_count", availableCount),
                    DiagnosticDetail.bytePreview("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static void registerSchemaIntegerOutOfRangeDiagnostic(
        String message,
        long byteOffset,
        String schemaName,
        String fieldName,
        long byteWidth,
        long minValue,
        long maxValue,
        long actualValue,
        String nearbyContext
    ) {
        registerSchemaIntegerOutOfRangePathDiagnostic(
            message,
            byteOffset,
            schemaFieldPath(schemaName, fieldName),
            byteWidth,
            minValue,
            maxValue,
            actualValue,
            nearbyContext
        );
    }

    private static void registerSchemaIntegerOutOfRangePathDiagnostic(
        String message,
        long byteOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
        long byteWidth,
        long minValue,
        long maxValue,
        long actualValue,
        String nearbyContext
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.integer_out_of_range",
                byteOffset,
                fieldPath,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("byte_width", byteWidth),
                    DiagnosticDetail.number("min_value", minValue),
                    DiagnosticDetail.number("max_value", maxValue),
                    DiagnosticDetail.number("actual_value", actualValue),
                    DiagnosticDetail.bytePreview("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static java.util.List<ByteDiagnosticFieldPathSegment> schemaFieldPath(
        String schemaName,
        String fieldName
    ) {
        return java.util.Arrays.asList(
            new ByteDiagnosticFieldPathSegment("schema", schemaName),
            new ByteDiagnosticFieldPathSegment("field", fieldName)
        );
    }

    private static java.util.List<ByteDiagnosticFieldPathSegment> schemaRepeatedFieldPath(
        String schemaName,
        String fieldName,
        long index
    ) {
        return java.util.Arrays.asList(
            new ByteDiagnosticFieldPathSegment("schema", schemaName),
            new ByteDiagnosticFieldPathSegment("field", fieldName),
            new ByteDiagnosticFieldPathSegment("index", Long.toString(index))
        );
    }

    private static java.util.List<ByteDiagnosticFieldPathSegment> schemaFieldPathWithPrefix(
        java.util.List<ByteDiagnosticFieldPathSegment> prefix,
        String schemaName,
        String fieldName
    ) {
        if (prefix == null) {
            return schemaFieldPath(schemaName, fieldName);
        }
        java.util.ArrayList<ByteDiagnosticFieldPathSegment> path =
            new java.util.ArrayList<ByteDiagnosticFieldPathSegment>(prefix);
        path.add(new ByteDiagnosticFieldPathSegment("schema", schemaName));
        path.add(new ByteDiagnosticFieldPathSegment("field", fieldName));
        return path;
    }

    private static void registerSchemaValidationFailedDiagnostic(
        String message,
        long byteOffset,
        String schemaName,
        String fieldName,
        String predicate,
        long fieldValue,
        java.util.Map<String, Long> decodedValues,
        String nearbyContext
    ) {
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        details.add(DiagnosticDetail.string("predicate", predicate));
        details.add(DiagnosticDetail.number("field_value", fieldValue));
        details.add(DiagnosticDetail.string("decoded_values", formatDecodedSchemaValues(decodedValues)));
        for (java.util.Map.Entry<String, Long> entry : decodedValues.entrySet()) {
            details.add(DiagnosticDetail.number(entry.getKey(), entry.getValue().longValue()));
        }
        details.add(DiagnosticDetail.bytePreview("byte_preview", nearbyContext));
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.validation_failed",
                byteOffset,
                java.util.Arrays.asList(
                    new ByteDiagnosticFieldPathSegment("schema", schemaName),
                    new ByteDiagnosticFieldPathSegment("field", fieldName)
                ),
                details
            )
        );
    }

    private static void registerSchemaStructuralValidationFailedDiagnostic(
        String message,
        long byteOffset,
        String schemaName,
        String predicate,
        java.util.Map<String, Long> decodedValues,
        String nearbyContext
    ) {
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        details.add(DiagnosticDetail.string("predicate", predicate));
        details.add(DiagnosticDetail.string("decoded_values", formatDecodedSchemaValues(decodedValues)));
        for (java.util.Map.Entry<String, Long> entry : decodedValues.entrySet()) {
            details.add(DiagnosticDetail.number(entry.getKey(), entry.getValue().longValue()));
        }
        details.add(DiagnosticDetail.bytePreview("byte_preview", nearbyContext));
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.validation_failed",
                byteOffset,
                java.util.Arrays.asList(
                    new ByteDiagnosticFieldPathSegment("schema", schemaName)
                ),
                details
            )
        );
    }

    private static void registerSchemaMappingDivisionByZeroDiagnostic(
        String message,
        long byteOffset,
        String schemaName,
        String targetField,
        String nearbyContext
    ) {
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        details.add(DiagnosticDetail.string("target_field", targetField));
        details.add(DiagnosticDetail.string("operator", "/"));
        details.add(DiagnosticDetail.bytePreview("byte_preview", nearbyContext));
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.mapping_division_by_zero",
                byteOffset,
                java.util.Arrays.asList(
                    new ByteDiagnosticFieldPathSegment("schema", schemaName),
                    new ByteDiagnosticFieldPathSegment("field", targetField)
                ),
                details
            )
        );
    }

    private static void registerSchemaLengthDivisionByZeroDiagnostic(
        String message,
        long byteOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
        String expression,
        String divisorOperand,
        String nearbyContext
    ) {
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        details.add(DiagnosticDetail.string("length_expression", expression));
        details.add(DiagnosticDetail.string("divisor_operand", divisorOperand));
        details.add(DiagnosticDetail.string("operator", "/"));
        details.add(DiagnosticDetail.bytePreview("byte_preview", nearbyContext));
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.length_division_by_zero",
                byteOffset,
                fieldPath,
                details
            )
        );
    }

    private static void registerSchemaValueValidationFailedDiagnostic(
        String message,
        String schemaName,
        String fieldName,
        String predicate,
        Long fieldValue,
        java.util.Map<String, Long> suppliedValues
    ) {
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        details.add(DiagnosticDetail.string("predicate", predicate));
        if (fieldValue != null) {
            details.add(DiagnosticDetail.number("field_value", fieldValue.longValue()));
        }
        details.add(DiagnosticDetail.string("supplied_values", formatDecodedSchemaValues(suppliedValues)));
        for (java.util.Map.Entry<String, Long> entry : suppliedValues.entrySet()) {
            details.add(DiagnosticDetail.number(entry.getKey(), entry.getValue().longValue()));
        }
        java.util.ArrayList<ByteDiagnosticFieldPathSegment> path =
            new java.util.ArrayList<ByteDiagnosticFieldPathSegment>();
        path.add(new ByteDiagnosticFieldPathSegment("schema", schemaName));
        if (fieldName != null && !fieldName.isEmpty()) {
            path.add(new ByteDiagnosticFieldPathSegment("field", fieldName));
        }
        valueDiagnostics.get().put(
            message,
            new ValueDiagnostic("schema.validation_failed", path, details)
        );
    }

    private static void registerByteWriteValueDiagnostic(
        String message,
        String helperName,
        long suppliedValue,
        long minValue,
        long maxValue,
        long width,
        boolean littleEndian
    ) {
        valueDiagnostics.get().put(
            message,
            new ValueDiagnostic(
                "codec.byte_write_value_unrepresentable",
                java.util.Collections.emptyList(),
                java.util.Arrays.asList(
                    DiagnosticDetail.string("helper_name", helperName),
                    DiagnosticDetail.number("supplied_value", suppliedValue),
                    DiagnosticDetail.number("min_value", minValue),
                    DiagnosticDetail.number("max_value", maxValue),
                    DiagnosticDetail.number("width", width),
                    DiagnosticDetail.string("byte_order", littleEndian ? "little_endian" : "big_endian")
                )
            )
        );
    }

    private static void registerEncodeErrorValueDiagnostic(Object value, String message) {
        if (!isAdtNamed(value, "EncodeError::EncodeError")) {
            return;
        }
        if (valueDiagnostics.get().containsKey(message)) {
            return;
        }
        Adt error = asAdt(value, "EncodeError::EncodeError");
        String id = asString(error.payloads[0]);
        if (!isCommandEncodeDiagnosticId(id)) {
            return;
        }
        String fieldPath = asString(error.payloads[1]);
        String reason = asString(error.payloads[2]);
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        details.add(DiagnosticDetail.string("reason", reason));
        details.add(DiagnosticDetail.string("field_path_display", fieldPath));
        valueDiagnostics.get().put(
            message,
            new ValueDiagnostic(id, encodeErrorFieldPath(fieldPath), details)
        );
    }

    private static void registerSchemaByteViewEncodeLengthMismatchDiagnostic(
        Object error,
        Object view,
        long expectedCount,
        long actualCount,
        String lengthExpression
    ) {
        Adt encodeError = asAdt(error, "EncodeError::EncodeError");
        String id = asString(encodeError.payloads[0]);
        String fieldPath = asString(encodeError.payloads[1]);
        String reason = asString(encodeError.payloads[2]);
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        details.add(DiagnosticDetail.string("reason", reason));
        details.add(DiagnosticDetail.string("field_path_display", fieldPath));
        details.add(DiagnosticDetail.number("expected_count", expectedCount));
        details.add(DiagnosticDetail.number("actual_count", actualCount));
        details.add(DiagnosticDetail.string("length_expression", lengthExpression));
        details.add(DiagnosticDetail.number("byte_offset", byteViewOffset(view)));
        details.add(DiagnosticDetail.bytePreview("byte_preview", byteViewPreviewTrace(view, 8)));
        valueDiagnostics.get().put(
            format(error),
            new ValueDiagnostic(id, encodeErrorFieldPath(fieldPath), details)
        );
    }

    private static boolean isCommandEncodeDiagnosticId(String id) {
        return id.equals("codec.encode_value_unrepresentable")
            || id.equals("codec.encode_mapping_mismatch")
            || id.equals("codec.dispatch_unknown_tag")
            || id.equals("codec.dispatch_length_mismatch")
            || id.equals("codec.dispatch_mismatch");
    }

    public static void registerDecodeErrorByteDiagnostic(Object value, String message) {
        if (!isAdtNamed(value, "DecodeError::DecodeError")
            && !isAdtNamed(value, "DecodeError::DecodeErrorWithReason")) {
            return;
        }
        Adt error = (Adt) value;
        String id = asString(error.payloads[0]);
        long byteOffset = byteOffsetLong(error.payloads[1]);
        String fieldPath = asString(error.payloads[2]);
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        if (isAdtNamed(value, "DecodeError::DecodeErrorWithReason")) {
            String reason = asString(error.payloads[3]);
            details.add(DiagnosticDetail.string("reason", reason));
            mergeDecodeErrorReasonByteContext(details, reason);
        }
        details.add(DiagnosticDetail.string("field_path_display", fieldPath));
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(id, byteOffset, encodeErrorFieldPath(fieldPath), details)
        );
    }

    private static void mergeDecodeErrorReasonByteContext(
        java.util.ArrayList<DiagnosticDetail> details,
        String reason
    ) {
        ByteDiagnostic carried = byteDiagnostics.get().remove(reason);
        if (carried == null) {
            return;
        }
        details.add(DiagnosticDetail.number("local_byte_offset", carried.byteOffset));
        for (DiagnosticDetail detail : carried.details) {
            if (detail.key.equals("expected_count")
                || detail.key.equals("available_count")
                || detail.key.equals("byte_preview")) {
                details.add(detail);
            } else if (detail.key.equals("requested_count")
                && detail.valueKind.equals("number")) {
                details.add(new DiagnosticDetail(
                    "expected_count",
                    detail.valueKind,
                    detail.encodedValue
                ));
            }
        }
    }

    public static void registerDecodeNeedMoreByteDiagnostic(Object value, String message) {
        if (!isAdt(value, "DecodeStep::NeedMore")) {
            return;
        }
        Object readiness = adtPayload(value, 0);
        java.util.ArrayList<DiagnosticDetail> details =
            new java.util.ArrayList<DiagnosticDetail>();
        long byteOffset = 0L;
        if (isAdt(readiness, "DecodeReadiness::NeedBytes")) {
            long neededCount = byteCountLong(adtPayload(readiness, 0));
            byteOffset = neededCount;
            details.add(DiagnosticDetail.string("readiness", "need_bytes"));
            details.add(DiagnosticDetail.number("needed_count", neededCount));
        } else if (isAdt(readiness, "DecodeReadiness::NeedEnd")) {
            details.add(DiagnosticDetail.string("readiness", "need_end"));
        } else {
            return;
        }
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "codec.incomplete_input",
                byteOffset,
                new java.util.ArrayList<ByteDiagnosticFieldPathSegment>(),
                details
            )
        );
    }

    private static java.util.List<ByteDiagnosticFieldPathSegment> encodeErrorFieldPath(
        String fieldPath
    ) {
        java.util.ArrayList<ByteDiagnosticFieldPathSegment> path =
            new java.util.ArrayList<ByteDiagnosticFieldPathSegment>();
        if (fieldPath == null || fieldPath.isEmpty()) {
            return path;
        }
        String[] segments = fieldPath.split("\\.");
        for (int index = 0; index < segments.length; index += 1) {
            String segment = segments[index];
            if (segment.isEmpty()) {
                continue;
            }
            path.add(new ByteDiagnosticFieldPathSegment(index == 0 ? "schema" : "field", segment));
        }
        return path;
    }

    private static void registerSchemaDispatchUnknownTagDiagnostic(
        String message,
        long byteOffset,
        String schemaName,
        String fieldName,
        String tagField,
        long decodedTagValue,
        String expectedTags,
        String nearbyContext
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.dispatch_unknown_tag",
                byteOffset,
                java.util.Arrays.asList(
                    new ByteDiagnosticFieldPathSegment("schema", schemaName),
                    new ByteDiagnosticFieldPathSegment("field", fieldName)
                ),
                java.util.Arrays.asList(
                    DiagnosticDetail.string("tag_field", tagField),
                    DiagnosticDetail.number("decoded_tag_value", decodedTagValue),
                    DiagnosticDetail.string("expected_tags", expectedTags),
                    DiagnosticDetail.bytePreview("byte_preview", nearbyContext)
                )
            )
        );
    }

    private static String formatDecodedSchemaValues(java.util.Map<String, Long> decodedValues) {
        StringBuilder out = new StringBuilder();
        boolean first = true;
        for (java.util.Map.Entry<String, Long> entry : decodedValues.entrySet()) {
            if (!first) {
                out.append(", ");
            }
            out.append(entry.getKey()).append("=").append(entry.getValue());
            first = false;
        }
        return out.toString();
    }

    private static String formatByteDiagnosticFieldPath(
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath
    ) {
        String out = "";
        for (ByteDiagnosticFieldPathSegment segment : fieldPath) {
            out = out + "\t" + segment.kind + "\t" + hex(segment.name);
        }
        return out;
    }

    private static String byteDiagnosticFieldPathDisplay(ByteDiagnostic diagnostic) {
        String out = "";
        for (ByteDiagnosticFieldPathSegment segment : diagnostic.fieldPath) {
            if (out.isEmpty()) {
                out = segment.name;
            } else {
                out = out + "." + segment.name;
            }
        }
        return out;
    }

    private static String formatDiagnosticDetails(
        java.util.List<DiagnosticDetail> details
    ) {
        String out = "";
        for (DiagnosticDetail detail : details) {
            out = out + "\t" + detail.key + "\t" + detail.valueKind + "\t" + detail.encodedValue;
        }
        return out;
    }

    private static void registerFixtureHexDiagnostic(
        String message,
        String id,
        String input,
        int textStart,
        int textEnd,
        int byteOffset,
        String nibble
    ) {
        int contextStart = Math.max(0, textStart - 8);
        int contextEnd = Math.min(input.length(), textEnd + 8);
        fixtureHexDiagnostics.get().put(
            message,
            new FixtureHexDiagnostic(
                id,
                textStart,
                textEnd,
                byteOffset,
                nibble,
                contextStart,
                contextEnd,
                input.substring(contextStart, contextEnd)
            )
        );
    }

    private static final class FixtureHexDiagnostic {
        final String id;
        final int textStart;
        final int textEnd;
        final int byteOffset;
        final String nibble;
        final int contextStart;
        final int contextEnd;
        final String context;

        FixtureHexDiagnostic(
            String id,
            int textStart,
            int textEnd,
            int byteOffset,
            String nibble,
            int contextStart,
            int contextEnd,
            String context
        ) {
            this.id = id;
            this.textStart = textStart;
            this.textEnd = textEnd;
            this.byteOffset = byteOffset;
            this.nibble = nibble;
            this.contextStart = contextStart;
            this.contextEnd = contextEnd;
            this.context = context;
        }
    }

    private static final class ByteDiagnostic {
        final String id;
        final long byteOffset;
        final java.util.List<ByteDiagnosticFieldPathSegment> fieldPath;
        final java.util.List<DiagnosticDetail> details;

        ByteDiagnostic(
            String id,
            long byteOffset,
            java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
            java.util.List<DiagnosticDetail> details
        ) {
            this.id = id;
            this.byteOffset = byteOffset;
            this.fieldPath = fieldPath;
            this.details = details;
        }
    }

    private static final class ValueDiagnostic {
        final String id;
        final java.util.List<ByteDiagnosticFieldPathSegment> fieldPath;
        final java.util.List<DiagnosticDetail> details;

        ValueDiagnostic(
            String id,
            java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
            java.util.List<DiagnosticDetail> details
        ) {
            this.id = id;
            this.fieldPath = fieldPath;
            this.details = details;
        }
    }

    private static final class ProtocolDiagnostic {
        final String id;
        final long byteOffset;
        final java.util.List<DiagnosticDetail> details;

        ProtocolDiagnostic(
            String id,
            long byteOffset,
            java.util.List<DiagnosticDetail> details
        ) {
            this.id = id;
            this.byteOffset = byteOffset;
            this.details = details;
        }
    }

    private static final class RuntimeDiagnosticPayload {
        final ByteDiagnostic byteDiagnostic;
        final ProtocolDiagnostic protocolDiagnostic;

        RuntimeDiagnosticPayload(
            ByteDiagnostic byteDiagnostic,
            ProtocolDiagnostic protocolDiagnostic
        ) {
            this.byteDiagnostic = byteDiagnostic;
            this.protocolDiagnostic = protocolDiagnostic;
        }
    }

    private static final class DiagnosticDetail {
        final String key;
        final String valueKind;
        final String encodedValue;

        static DiagnosticDetail number(String key, long value) {
            return new DiagnosticDetail(key, "number", Long.toString(value));
        }

        static DiagnosticDetail string(String key, String value) {
            return new DiagnosticDetail(key, "string", hex(value));
        }

        static DiagnosticDetail bytePreview(String key, String value) {
            return new DiagnosticDetail(
                key,
                "byte_preview_v2",
                value
            );
        }

        static DiagnosticDetail bytePreviewV2(String key, String value) {
            return new DiagnosticDetail(key, "byte_preview_v2", value);
        }

        DiagnosticDetail(String key, String valueKind, String encodedValue) {
            this.key = key;
            this.valueKind = valueKind;
            this.encodedValue = encodedValue;
        }
    }

    private static final class ByteDiagnosticFieldPathSegment {
        final String kind;
        final String name;

        ByteDiagnosticFieldPathSegment(String kind, String name) {
            this.kind = kind;
            this.name = name;
        }
    }
