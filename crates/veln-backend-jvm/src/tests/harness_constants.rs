pub(super) const RUNTIME_LIST_HARNESS: &str = r#"
public final class RuntimeListHarness {
    private static int foldCalls = 0;
    private static int tryCalls = 0;

    public static void main(String[] args) {
        Object values = VelnRuntime.listNil();
        for (int index = 0; index < 20000; index += 1) {
            values = VelnRuntime.listCons(Long.valueOf(1), values);
        }
        Object reversed = VelnRuntime.listReverse(values);
        Object total = VelnRuntime.listFold(reversed, Long.valueOf(0), new VelnRuntime.Fn() {
            public Object call(Object... args) {
                foldCalls += 1;
                return Long.valueOf(((Long) args[0]).longValue() + ((Long) args[1]).longValue());
            }
        });
        Object kept = VelnRuntime.listFilter(reversed, new VelnRuntime.Fn() {
            public Object call(Object... args) {
                return Boolean.TRUE;
            }
        });
        Object tried = VelnRuntime.listTryMap(
            VelnRuntime.listCons(
                Long.valueOf(1),
                VelnRuntime.listCons(Long.valueOf(2), VelnRuntime.listCons(Long.valueOf(3), VelnRuntime.listNil()))
            ),
            new VelnRuntime.Fn() {
                public Object call(Object... args) {
                    tryCalls += 1;
                    if (((Long) args[0]).longValue() == 2L) {
                        return VelnRuntime.err("stop");
                    }
                    return VelnRuntime.ok(args[0]);
                }
            }
        );
        System.out.println(
            total
                + ":"
                + foldCalls
                + ":"
                + VelnRuntime.listIsEmpty(VelnRuntime.listNil())
                + ":"
                + VelnRuntime.listIsEmpty(kept)
                + ":"
                + tryCalls
                + ":"
                + tried
        );
    }
}
"#;

pub(super) const RUNTIME_BYTE_HEX_HARNESS: &str = r#"
public final class RuntimeByteHexHarness {
    public static void main(String[] args) {
        System.out.println(VelnRuntime.byteChunkFromHex("0001ff"));
        System.out.println(VelnRuntime.byteChunkFromHex("00 01\nff\t10"));
        System.out.println(VelnRuntime.byteChunkFromHex("0x00"));
        System.out.println(VelnRuntime.byteChunkFromHex("00_1"));
        System.out.println(VelnRuntime.byteChunkFromHex("0 0"));
        System.out.println(VelnRuntime.byteChunkFromHex("00#comment"));
        System.out.println(VelnRuntime.byteChunkFromHex("00:01"));
        System.out.println(VelnRuntime.byteChunkFromHex("001"));
    }
}
"#;

pub(super) const RUNTIME_RESULT_DIAGNOSTIC_TRACE_HARNESS: &str = r#"
public final class RuntimeResultDiagnosticTraceHarness {
    public static void main(String[] args) {
        Object schemaEncodeError = VelnRuntime.adt(
            "EncodeError::EncodeError",
            new Object[] {
                "schema.encode_value_unrepresentable",
                "Packet.value",
                "too large"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(schemaEncodeError));

        Object codecEncodeError = VelnRuntime.adt(
            "EncodeError::EncodeError",
            new Object[] {
                "codec.encode_value_unrepresentable",
                "Packet.value",
                "too large"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(codecEncodeError));

        Object decodeError = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.decode_failed",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(7) }),
                "Packet.value",
                "plain reason"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(decodeError));

        Object lengthMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.length_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(9) }),
                "Packet.payload",
                "expected_length=4; actual_length=3; reason=payload length did not match header length"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(lengthMismatch));

        Object plainLengthMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.length_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(10) }),
                "Packet.payload",
                "plain length mismatch"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(plainLengthMismatch));

        Object payloadLengthMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.payload_length_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(21) }),
                "Packet.payload",
                "expected_payload_length=8; actual_payload_length=5; reason=payload length did not match frame header"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(payloadLengthMismatch));

        Object plainPayloadLengthMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.payload_length_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(22) }),
                "Packet.payload",
                "plain payload length mismatch"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(plainPayloadLengthMismatch));

        Object paddingMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.padding_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(24) }),
                "Packet.padding",
                "expected_padding_length=2; actual_padding_length=5; reason=DATA padding did not match payload boundary"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(paddingMismatch));

        Object plainPaddingMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.padding_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(25) }),
                "Packet.padding",
                "plain padding mismatch"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(plainPaddingMismatch));

        Object integerOutOfRange = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.integer_out_of_range",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(17) }),
                "Packet.stream_id",
                "byte_width=4; min_value=0; max_value=2147483647; actual_value=2147483648; reason=decoded value exceeds signed integer range"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(integerOutOfRange));

        Object plainIntegerOutOfRange = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.integer_out_of_range",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(18) }),
                "Packet.stream_id",
                "plain integer conversion failure"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(plainIntegerOutOfRange));

        Object sequenceMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.sequence_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(13) }),
                "Packet.sequence",
                "expected_sequence=client_preface,settings; actual_sequence=settings; reason=frame sequence violated protocol state"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(sequenceMismatch));

        Object versionMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.version_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(3) }),
                "Packet.version",
                "expected_version=2; actual_version=1; reason=codec version is not supported"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(versionMismatch));

        Object tagMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.tag_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(14) }),
                "Packet.kind",
                "expected_tag=DATA; actual_tag=HEADERS; reason=dispatch tag did not match selected payload"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(tagMismatch));

        Object plainTagMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.tag_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(15) }),
                "Packet.kind",
                "plain tag mismatch"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(plainTagMismatch));

        Object magicMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.magic_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(18) }),
                "Packet.magic",
                "expected_magic=VELN; actual_magic=VEIN; reason=file magic did not match expected signature"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(magicMismatch));

        Object plainMagicMismatch = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.magic_mismatch",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(19) }),
                "Packet.magic",
                "plain magic mismatch"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(plainMagicMismatch));

        Object unsupportedFeature = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.unsupported_feature",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(27) }),
                "Packet.extension",
                "feature=dynamic_table_size_update; reason=dynamic table size updates are disabled for this profile"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(unsupportedFeature));

        Object plainUnsupportedFeature = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.unsupported_feature",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(28) }),
                "Packet.extension",
                "plain unsupported feature"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(plainUnsupportedFeature));

        Object trailingInput = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.trailing_input",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(5) }),
                "Packet.payload",
                "consumed_count=5; available_count=8; remaining_count=3; reason=packet decoder completed before the bounded input ended"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(trailingInput));

        Object malformedTrailingInput = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.trailing_input",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(6) }),
                "Packet.payload",
                "consumed_count=5; available_count=8; remaining_count=4; reason=inconsistent counts"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(malformedTrailingInput));

        Object bytes = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("0102030405")).value();
        Object view = ((VelnRuntime.Result) VelnRuntime.byteView(
            bytes,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(2) }),
            VelnRuntime.adt("ByteCount", new Object[] { Long.valueOf(3) })
        )).value();
        Object byteReadReason = ((VelnRuntime.Result) VelnRuntime.byteReadU32Be(view)).value();
        Object contextualDecodeError = VelnRuntime.adt(
            "DecodeError::DecodeErrorWithReason",
            new Object[] {
                "codec.invalid_input",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(42) }),
                "ManualPacket.checksum",
                byteReadReason
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(contextualDecodeError));

        Object plainDecodeError = VelnRuntime.adt(
            "DecodeError::DecodeError",
            new Object[] {
                "codec.consumed_count_invalid",
                VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(11) }),
                "Packet.count"
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(plainDecodeError));

        Object oversizedDecoded = VelnRuntime.adt(
            "DecodeStep::Decoded",
            new Object[] {
                Long.valueOf(7),
                VelnRuntime.adt("ByteCount", new Object[] { Long.valueOf(5) })
            }
        );
        Object oversizedInvalidStep = VelnRuntime.validateCodecDecodeStep(
            view,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(21) }),
            oversizedDecoded,
            "Packet.count"
        );
        VelnRuntime.recordResultFailure(
            VelnRuntime.Result.err(VelnRuntime.adtPayload(oversizedInvalidStep, 0))
        );

        Object negativeDecoded = VelnRuntime.adt(
            "DecodeStep::Decoded",
            new Object[] {
                Long.valueOf(7),
                VelnRuntime.adt("ByteCount", new Object[] { Long.valueOf(-1) })
            }
        );
        Object negativeInvalidStep = VelnRuntime.validateCodecDecodeStep(
            view,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(22) }),
            negativeDecoded,
            "Packet.count"
        );
        VelnRuntime.recordResultFailure(
            VelnRuntime.Result.err(VelnRuntime.adtPayload(negativeInvalidStep, 0))
        );

        Object needMore = VelnRuntime.adt(
            "DecodeStep::NeedMore",
            new Object[] {
                VelnRuntime.adt(
                    "DecodeReadiness::NeedBytes",
                    new Object[] {
                        VelnRuntime.adt("ByteCount", new Object[] { Long.valueOf(5) })
                    }
                )
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(needMore));

        Object needEnd = VelnRuntime.adt(
            "DecodeStep::NeedMore",
            new Object[] {
                VelnRuntime.adt("DecodeReadiness::NeedEnd", new Object[] {})
            }
        );
        VelnRuntime.recordResultFailure(VelnRuntime.Result.err(needEnd));
    }
}
"#;

pub(super) const RUNTIME_BYTE_VIEW_HARNESS: &str = r#"
public final class RuntimeByteViewHarness {
    public static void main(String[] args) {
        Object chunk = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("00010203ff")).value();
        Object view = ((VelnRuntime.Result) VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(1))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(3))).value()
        )).value();
        Object wideView = ((VelnRuntime.Result) VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(1))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        Object two = ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(2))).value();
        Object viewPrefix = ((VelnRuntime.Result) VelnRuntime.byteViewTake(view, two)).value();
        Object viewSuffix = ((VelnRuntime.Result) VelnRuntime.byteViewDrop(view, two)).value();
        Object viewSlice = ((VelnRuntime.Result) VelnRuntime.byteViewSlice(
            wideView,
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(1))).value(),
            two
        )).value();
        Object outputChunks = VelnRuntime.byteChunksAppend(
            VelnRuntime.byteChunksOne(VelnRuntime.byteViewToChunk(viewPrefix)),
            VelnRuntime.byteChunksOne(VelnRuntime.byteViewToChunk(viewSuffix))
        );
        System.out.println(VelnRuntime.byteReadU8Be(view));
        System.out.println(VelnRuntime.byteReadU16Be(view));
        System.out.println(VelnRuntime.byteReadU24Be(view));
        System.out.println(VelnRuntime.byteReadU16Le(view));
        System.out.println(VelnRuntime.byteReadU24Le(view));
        System.out.println(VelnRuntime.byteViewCount(view));
        System.out.println(VelnRuntime.byteReadU16Be(viewPrefix));
        System.out.println(VelnRuntime.byteReadU8Be(viewSuffix));
        System.out.println(VelnRuntime.byteReadU16Be(viewSlice));
        System.out.println(outputChunks);
        System.out.println(VelnRuntime.byteViewDrop(view, ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()));
        System.out.println(VelnRuntime.byteReadU31Be(wideView));
        Object maxU31 = ((VelnRuntime.Result) VelnRuntime.byteWriteU31Be(Long.valueOf(2147483647))).value();
        Object maxU31View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU31,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU31Be(maxU31View));
        Object maxU32 = ((VelnRuntime.Result) VelnRuntime.byteWriteU32Be(Long.valueOf(4294967295L))).value();
        Object maxU32View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU32,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU32Be(maxU32View));
        Object maxU40 = ((VelnRuntime.Result) VelnRuntime.byteWriteU40Be(Long.valueOf(1099511627775L))).value();
        Object maxU40View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU40,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(5))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU40Be(maxU40View));
        Object maxU48 = ((VelnRuntime.Result) VelnRuntime.byteWriteU48Be(Long.valueOf(281474976710655L))).value();
        Object maxU48View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU48,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(6))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU48Be(maxU48View));
        Object maxU31Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU31Le(Long.valueOf(2147483647))).value();
        Object maxU31LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU31Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU31Le(maxU31LeView));
        Object maxU32Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU32Le(Long.valueOf(4294967295L))).value();
        Object maxU32LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU32Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU32Le(maxU32LeView));
        Object maxU40Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU40Le(Long.valueOf(1099511627775L))).value();
        Object maxU40LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU40Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(5))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU40Le(maxU40LeView));
        Object maxU48Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU48Le(Long.valueOf(281474976710655L))).value();
        Object maxU48LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU48Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(6))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU48Le(maxU48LeView));
        Object maxU64 = ((VelnRuntime.Result) VelnRuntime.byteWriteU64Be(Long.MAX_VALUE)).value();
        Object maxU64View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU64,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(8))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU64Be(maxU64View));
        Object maxU64Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU64Le(Long.MAX_VALUE)).value();
        Object maxU64LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU64Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(8))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU64Le(maxU64LeView));
        System.out.println(VelnRuntime.byteWriteU16Le(Long.valueOf(4660)));
        System.out.println(VelnRuntime.byteWriteU24Le(Long.valueOf(66051)));
        System.out.println(VelnRuntime.byteWriteU32Le(Long.valueOf(16909060)));
        System.out.println(VelnRuntime.byteReadU24Be(((VelnRuntime.Result) VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(2))).value()
        )).value()));
        System.out.println(VelnRuntime.byteReadU24Le(((VelnRuntime.Result) VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(2))).value()
        )).value()));
        System.out.println(VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(4))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(2))).value()
        ));
        System.out.println(VelnRuntime.byteWriteU8Be(Long.valueOf(256)));
        System.out.println(VelnRuntime.byteWriteU32Le(Long.valueOf(4294967296L)));
        System.out.println(VelnRuntime.byteReadU31Be(maxU32View));
        System.out.println(VelnRuntime.byteReadU31Le(maxU32LeView));
        Object overflowU64View = ((VelnRuntime.Result) VelnRuntime.byteView(
            ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("ffffffffffffffff")).value(),
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(8))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU64Be(overflowU64View));
        System.out.println(VelnRuntime.byteReadU64Le(overflowU64View));
        Object frame = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("000005010400000001")).value();
        Object frameView = ((VelnRuntime.Result) VelnRuntime.byteView(
            frame,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(9))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeHttp2FrameHeader(frameView));
        Object widthSample = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("1234deadbeef")).value();
        Object widthSampleView = ((VelnRuntime.Result) VelnRuntime.byteView(
            widthSample,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(6))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeSchemaWidthSample(widthSampleView));
        Object validationSample = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("00000504")).value();
        Object validationSampleView = ((VelnRuntime.Result) VelnRuntime.byteView(
            validationSample,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeSchemaValidationSample(validationSampleView));
        Object invalidValidationSample = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("00000506")).value();
        Object invalidValidationSampleView = ((VelnRuntime.Result) VelnRuntime.byteView(
            invalidValidationSample,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeSchemaValidationSample(invalidValidationSampleView));
        Object reservedFrame = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("000005010480000001")).value();
        Object reservedView = ((VelnRuntime.Result) VelnRuntime.byteView(
            reservedFrame,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(9))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeHttp2FrameHeader(reservedView));
    }
}
"#;

pub(super) const PUBLIC_LIST_HELPER_HARNESS: &str = r#"
public final class PublicListHelperHarness {
    public static void main(String[] args) {
        Object values = VelnRuntime.listNil();
        for (int index = 0; index < 20000; index += 1) {
            values = VelnRuntime.listCons(Long.valueOf(1), values);
        }
        VelnProgram.fn_consume(values);
    }
}
"#;

pub(super) const RUNTIME_PATH_HARNESS: &str = r#"
public final class RuntimePathHarness {
    public static void main(String[] args) {
        Object cwd = ((VelnRuntime.Result) VelnRuntime.processCwd()).value();
        System.out.println(VelnRuntime.fsExists(cwd));

        Object entries = ((VelnRuntime.Result) VelnRuntime.fsReadDir(cwd)).value();
        Object first = ((java.util.List<?>) entries).get(0);
        System.out.println(VelnRuntime.fsExists(first));
    }
}
"#;

pub(super) const RUNTIME_CHANNEL_SELECT_MANY_TIMEOUT_RESULT_HARNESS: &str = r#"
public final class RuntimeChannelSelectManyTimeoutResultHarness {
    public static void main(String[] args) {
        Object first = VelnRuntime.channelBounded(Long.valueOf(1));
        Object second = VelnRuntime.channelBounded(Long.valueOf(1));
        Object third = VelnRuntime.channelBounded(Long.valueOf(1));
        VelnRuntime.channelSend(VelnRuntime.recordField(second, "tx"), Long.valueOf(21));
        VelnRuntime.channelSend(VelnRuntime.recordField(third, "tx"), Long.valueOf(34));
        Object readyReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(first, "rx"),
            VelnRuntime.listCons(
                VelnRuntime.recordField(second, "rx"),
                VelnRuntime.listCons(VelnRuntime.recordField(third, "rx"), VelnRuntime.listNil())
            )
        );
        System.out.println(VelnRuntime.channelSelectManyTimeoutResult(readyReceivers, Long.valueOf(10)));

        Object timeoutFirst = VelnRuntime.channelBounded(Long.valueOf(1));
        Object timeoutSecond = VelnRuntime.channelBounded(Long.valueOf(1));
        Object timeoutReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(timeoutFirst, "rx"),
            VelnRuntime.listCons(VelnRuntime.recordField(timeoutSecond, "rx"), VelnRuntime.listNil())
        );
        System.out.println(VelnRuntime.channelSelectManyTimeoutResult(timeoutReceivers, Long.valueOf(0)));

        Object interruptFirst = VelnRuntime.channelBounded(Long.valueOf(1));
        Object interruptSecond = VelnRuntime.channelBounded(Long.valueOf(1));
        Object interruptReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(interruptFirst, "rx"),
            VelnRuntime.listCons(VelnRuntime.recordField(interruptSecond, "rx"), VelnRuntime.listNil())
        );
        Thread.currentThread().interrupt();
        System.out.println(VelnRuntime.channelSelectManyTimeoutResult(interruptReceivers, Long.valueOf(10000)));
        Thread.interrupted();

        Object cancelledToken = VelnRuntime.timeCancelToken();
        VelnRuntime.timeCancel(cancelledToken);
        Object cancelledReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(VelnRuntime.channelBounded(Long.valueOf(1)), "rx"),
            VelnRuntime.listCons(
                VelnRuntime.recordField(VelnRuntime.channelBounded(Long.valueOf(1)), "rx"),
                VelnRuntime.listNil()
            )
        );
        System.out.println(VelnRuntime.channelSelectManyTimeoutCancellable(
            cancelledReceivers,
            Long.valueOf(10000),
            cancelledToken
        ));

        final Object waitToken = VelnRuntime.timeCancelToken();
        Thread canceller = new Thread(new Runnable() {
            public void run() {
                try {
                    Thread.sleep(10L);
                } catch (InterruptedException error) {
                    Thread.currentThread().interrupt();
                }
                VelnRuntime.timeCancel(waitToken);
            }
        });
        Object waitingReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(VelnRuntime.channelBounded(Long.valueOf(1)), "rx"),
            VelnRuntime.listCons(
                VelnRuntime.recordField(VelnRuntime.channelBounded(Long.valueOf(1)), "rx"),
                VelnRuntime.listNil()
            )
        );
        canceller.start();
        System.out.println(VelnRuntime.channelSelectManyTimeoutCancellable(
            waitingReceivers,
            Long.valueOf(10000),
            waitToken
        ));
    }
}
"#;
