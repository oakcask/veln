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
        String value = format(asResult(result).value());
        FixtureHexDiagnostic fixtureHex = fixtureHexDiagnostics.get().remove(value);
        ByteDiagnostic byteDiagnostic = byteDiagnostics.get().remove(value);
        ProtocolDiagnostic protocolDiagnostic = protocolDiagnostics.get().remove(value);
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

    private static final ThreadLocal<java.util.Map<String, ProtocolDiagnostic>> protocolDiagnostics =
        ThreadLocal.withInitial(java.util.HashMap::new);

    private static void registerHttp2ProtocolClosedWithPendingDiagnostic(
        String message,
        long byteOffset,
        long pendingCount,
        String activeContinuation
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "http2.protocol.closed_with_pending",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("pending_count", pendingCount),
                    DiagnosticDetail.string("input_event", "end"),
                    DiagnosticDetail.string("active_continuation", activeContinuation)
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
        String activeContinuation
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
                    DiagnosticDetail.string("active_continuation", activeContinuation)
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
        String ruleProvenance
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
        String ruleProvenance
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
        String ruleProvenance
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
        String ruleProvenance
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
        String ruleProvenance
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
        String ruleProvenance
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
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
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
        String ruleProvenance
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
                    DiagnosticDetail.string("rule_provenance", ruleProvenance)
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
                    DiagnosticDetail.number("attempted_concurrent_stream_count", attemptedCount),
                    DiagnosticDetail.number("allowed_concurrent_stream_count", allowedCount),
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
        String peerLimitProvenance
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
                    DiagnosticDetail.string("peer_limit_provenance", peerLimitProvenance)
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
        String codecModule
    ) {
        protocolDiagnostics.get().put(
            message,
            new ProtocolDiagnostic(
                "hpack.fixture.unsupported_header_block",
                byteOffset,
                java.util.Arrays.asList(
                    DiagnosticDetail.number("observed_header_block_size", observedSize),
                    DiagnosticDetail.number("observed_first_byte", observedFirstByte),
                    DiagnosticDetail.string("expected_fixture", expectedFixture),
                    DiagnosticDetail.string("codec_module", codecModule)
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
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "schema.fixed_field_mismatch",
                byteOffset,
                java.util.Arrays.asList(
                    new ByteDiagnosticFieldPathSegment("schema", schemaName),
                    new ByteDiagnosticFieldPathSegment("field", fieldName)
                ),
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
