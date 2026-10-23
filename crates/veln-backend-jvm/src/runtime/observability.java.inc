    private static final class ProductionMetricIndex {
        private final long limit;
        private final java.util.TreeSet<MetricSeriesIdentity> series;
        private long comparisonWork;

        private ProductionMetricIndex(long limit) {
            this.limit = Math.max(0L, limit);
            this.series = new java.util.TreeSet<MetricSeriesIdentity>(this::compareIdentities);
            this.comparisonWork = 0L;
        }

        private int compareIdentities(MetricSeriesIdentity left, MetricSeriesIdentity right) {
            if (comparisonWork < Long.MAX_VALUE) {
                comparisonWork += 1L;
            }
            return left.compareTo(right);
        }

        private synchronized boolean contains(Object candidate) {
            return series.contains(metricSeriesIdentity(candidate));
        }

        private synchronized void insert(Object candidate) {
            MetricSeriesIdentity identity = metricSeriesIdentity(candidate);
            if (series.contains(identity) || ((long) series.size()) < limit) {
                series.add(identity);
            }
        }

        private synchronized long work() {
            return comparisonWork;
        }

        private synchronized void resetWork() {
            comparisonWork = 0L;
        }
    }

    private static final class MetricSeriesIdentity implements Comparable<MetricSeriesIdentity> {
        private final int kind;
        private final String name;
        private final java.util.List<MetricAttributeMultiplicity> attributes;

        private MetricSeriesIdentity(
            int kind,
            String name,
            java.util.List<MetricAttributeMultiplicity> attributes
        ) {
            this.kind = kind;
            this.name = name;
            this.attributes = attributes;
        }

        @Override
        public int compareTo(MetricSeriesIdentity other) {
            int compared = Integer.compare(kind, other.kind);
            if (compared != 0) {
                return compared;
            }
            compared = name.compareTo(other.name);
            if (compared != 0) {
                return compared;
            }
            int shared = Math.min(attributes.size(), other.attributes.size());
            for (int index = 0; index < shared; index += 1) {
                compared = attributes.get(index).compareTo(other.attributes.get(index));
                if (compared != 0) {
                    return compared;
                }
            }
            return Integer.compare(attributes.size(), other.attributes.size());
        }
    }

    private static final class MetricAttribute implements Comparable<MetricAttribute> {
        private final String name;
        private final int scalarKind;
        private final Object value;

        private MetricAttribute(String name, int scalarKind, Object value) {
            this.name = name;
            this.scalarKind = scalarKind;
            this.value = value;
        }

        @Override
        public int compareTo(MetricAttribute other) {
            int compared = name.compareTo(other.name);
            if (compared != 0) {
                return compared;
            }
            compared = Integer.compare(scalarKind, other.scalarKind);
            if (compared != 0) {
                return compared;
            }
            if (scalarKind == 0) {
                return Boolean.compare(((Boolean) value).booleanValue(), ((Boolean) other.value).booleanValue());
            }
            if (scalarKind == 1) {
                return Long.compare(((Long) value).longValue(), ((Long) other.value).longValue());
            }
            return ((String) value).compareTo((String) other.value);
        }
    }

    private static final class MetricAttributeMultiplicity
        implements Comparable<MetricAttributeMultiplicity> {
        private final MetricAttribute attribute;
        private final long multiplicity;

        private MetricAttributeMultiplicity(MetricAttribute attribute, long multiplicity) {
            this.attribute = attribute;
            this.multiplicity = multiplicity;
        }

        @Override
        public int compareTo(MetricAttributeMultiplicity other) {
            int compared = attribute.compareTo(other.attribute);
            if (compared != 0) {
                return compared;
            }
            return Long.compare(multiplicity, other.multiplicity);
        }
    }

    private static MetricSeriesIdentity metricSeriesIdentity(Object candidate) {
        Adt series = (Adt) candidate;
        int kind;
        if (isAdt(series, "CounterSeries")) {
            kind = 0;
        } else if (isAdt(series, "HistogramSeries")) {
            kind = 1;
        } else {
            throw new IllegalStateException("expected metric series");
        }
        String name = asString(series.payloads[0]);
        java.util.TreeMap<MetricAttribute, Long> counts =
            new java.util.TreeMap<MetricAttribute, Long>();
        ListValue remaining = asListValue(series.payloads[1]);
        while (remaining.cons) {
            Adt attribute = asAdt(remaining.head, "Attribute");
            String attributeName = asString(attribute.payloads[0]);
            Adt scalar = (Adt) attribute.payloads[1];
            MetricAttribute key;
            if (isAdt(scalar, "BoolValue")) {
                key = new MetricAttribute(attributeName, 0, (Boolean) scalar.payloads[0]);
            } else if (isAdt(scalar, "IntValue")) {
                key = new MetricAttribute(attributeName, 1, (Long) scalar.payloads[0]);
            } else if (isAdt(scalar, "StringValue")) {
                key = new MetricAttribute(attributeName, 2, asString(scalar.payloads[0]));
            } else {
                throw new IllegalStateException("expected metric attribute scalar");
            }
            Long count = counts.get(key);
            counts.put(key, Long.valueOf(count == null ? 1L : count.longValue() + 1L));
            remaining = asListValue(remaining.tail);
        }
        java.util.ArrayList<MetricAttributeMultiplicity> normalized =
            new java.util.ArrayList<MetricAttributeMultiplicity>(counts.size());
        for (java.util.Map.Entry<MetricAttribute, Long> entry : counts.entrySet()) {
            normalized.add(new MetricAttributeMultiplicity(entry.getKey(), entry.getValue().longValue()));
        }
        return new MetricSeriesIdentity(kind, name, java.util.Collections.unmodifiableList(normalized));
    }

    private static ProductionMetricIndex asProductionMetricIndex(Object value) {
        return (ProductionMetricIndex) value;
    }

    public static Object productionMetricIndexNew(Object limit) {
        return new ProductionMetricIndex(asLong(limit));
    }

    public static Object productionMetricIndexContains(Object index, Object candidate) {
        return Boolean.valueOf(asProductionMetricIndex(index).contains(candidate));
    }

    public static Object productionMetricIndexInsert(Object index, Object candidate) {
        asProductionMetricIndex(index).insert(candidate);
        return UNIT;
    }

    public static Object productionMetricIndexWork(Object index) {
        return Long.valueOf(asProductionMetricIndex(index).work());
    }

    public static Object productionMetricIndexResetWork(Object index) {
        asProductionMetricIndex(index).resetWork();
        return UNIT;
    }

    private static final String OBSERVATION_EXPORTER_EFFECT =
        "std::host_effects::ObservationExporter";

    private static final class ExportAttributeFacts {
        long dropped;
        long truncated;
        long redacted;
    }

    private static String observationJsonString(String value) {
        StringBuilder output = new StringBuilder(value.length() + 8);
        output.append('"');
        for (int index = 0; index < value.length(); index += 1) {
            char character = value.charAt(index);
            switch (character) {
                case '"': output.append("\\\""); break;
                case '\\': output.append("\\\\"); break;
                case '\b': output.append("\\b"); break;
                case '\f': output.append("\\f"); break;
                case '\n': output.append("\\n"); break;
                case '\r': output.append("\\r"); break;
                case '\t': output.append("\\t"); break;
                default:
                    if (character < 0x20) {
                        output.append(String.format("\\u%04x", Integer.valueOf(character)));
                    } else {
                        output.append(character);
                    }
            }
        }
        return output.append('"').toString();
    }

    private static String observationOptionalString(Object value) {
        Option option = asOption(value);
        return option.some ? observationJsonString(asString(option.value)) : "null";
    }

    private static java.util.Set<String> observationRedactedNames(Object names) {
        java.util.HashSet<String> result = new java.util.HashSet<String>();
        ListValue remaining = asListValue(names);
        while (remaining.cons) {
            result.add(asString(remaining.head));
            remaining = asListValue(remaining.tail);
        }
        return result;
    }

    private static String observationAttributes(
        Object attributes,
        long maximumCount,
        long maximumStringSize,
        long maximumRedacted,
        java.util.Set<String> redactedNames,
        ExportAttributeFacts facts
    ) {
        StringBuilder output = new StringBuilder("[");
        ListValue remaining = asListValue(attributes);
        long retained = 0L;
        boolean first = true;
        while (remaining.cons) {
            Adt attribute = asAdt(remaining.head, "Attribute");
            remaining = asListValue(remaining.tail);
            if (retained >= maximumCount) {
                facts.dropped += 1L;
                continue;
            }
            retained += 1L;
            String name = asString(attribute.payloads[0]);
            Adt scalar = (Adt) attribute.payloads[1];
            String type;
            String value;
            if (redactedNames.contains(name) && facts.redacted < maximumRedacted) {
                type = "string";
                value = observationJsonString("[REDACTED]");
                facts.redacted += 1L;
            } else if (isAdt(scalar, "BoolValue")) {
                type = "bool";
                value = Boolean.TRUE.equals(scalar.payloads[0]) ? "true" : "false";
            } else if (isAdt(scalar, "IntValue")) {
                type = "int";
                value = Long.toString(asLong(scalar.payloads[0]));
            } else if (isAdt(scalar, "StringValue")) {
                type = "string";
                String text = asString(scalar.payloads[0]);
                int limit = (int) Math.min(Integer.MAX_VALUE, maximumStringSize);
                if (text.length() > limit) {
                    text = text.substring(0, limit);
                    facts.truncated += 1L;
                }
                value = observationJsonString(text);
            } else {
                throw new IllegalStateException("expected observation attribute scalar");
            }
            if (!first) output.append(',');
            first = false;
            output.append("{\"name\":").append(observationJsonString(name))
                .append(",\"type\":").append(observationJsonString(type))
                .append(",\"value\":").append(value).append('}');
        }
        return output.append(']').toString();
    }

    private static String observationSource(Object source) {
        String file = asString(recordField(source, "file"));
        try {
            java.nio.file.Path path = java.nio.file.Paths.get(file);
            boolean windowsAbsolute = file.matches("^[A-Za-z]:[\\\\/].*") || file.startsWith("\\\\");
            if (path.isAbsolute() || windowsAbsolute) {
                java.nio.file.Path name = path.getFileName();
                if (windowsAbsolute && !path.isAbsolute()) {
                    int separator = Math.max(file.lastIndexOf('/'), file.lastIndexOf('\\'));
                    file = separator < 0 ? "<unknown>" : file.substring(separator + 1);
                } else {
                    file = name == null ? "<unknown>" : name.toString();
                }
            }
        } catch (java.nio.file.InvalidPathException ignored) {
            file = "<invalid-source>";
        }
        return "{\"package\":" + observationJsonString(asString(recordField(source, "package")))
            + ",\"module\":" + observationJsonString(asString(recordField(source, "module")))
            + ",\"file\":" + observationJsonString(file)
            + ",\"start_line\":" + asLong(recordField(source, "start_line"))
            + ",\"start_column\":" + asLong(recordField(source, "start_column"))
            + ",\"end_line\":" + asLong(recordField(source, "end_line"))
            + ",\"end_column\":" + asLong(recordField(source, "end_column")) + "}";
    }

    private static String observationOptionalSource(Object source) {
        Option option = asOption(source);
        if (option.some) return observationSource(option.value);
        return "{\"package\":\"std\",\"module\":\"observe\",\"file\":\"<observe>\","
            + "\"start_line\":0,\"start_column\":0,\"end_line\":0,\"end_column\":0}";
    }

    private static String observationContext(Object context) {
        Option option = asOption(context);
        if (!option.some) return "null";
        Adt trace = asAdt(option.value, "TraceContext");
        return "{\"trace_id\":" + observationJsonString(asString(trace.payloads[0]))
            + ",\"span_id\":" + observationJsonString(asString(trace.payloads[1]))
            + ",\"sampled\":" + (Boolean.TRUE.equals(trace.payloads[2]) ? "true" : "false") + "}";
    }

    private static String observationLinks(Object links) {
        StringBuilder output = new StringBuilder("[");
        ListValue remaining = asListValue(links);
        boolean first = true;
        while (remaining.cons) {
            Adt trace = asAdt(remaining.head, "TraceContext");
            if (!first) output.append(',');
            first = false;
            output.append("{\"trace_id\":").append(observationJsonString(asString(trace.payloads[0])))
                .append(",\"span_id\":").append(observationJsonString(asString(trace.payloads[1])))
                .append(",\"sampled\":").append(Boolean.TRUE.equals(trace.payloads[2]) ? "true" : "false")
                .append('}');
            remaining = asListValue(remaining.tail);
        }
        return output.append(']').toString();
    }

    public static Object observationJsonLine(
        Object payload,
        Object wallTime,
        Object sequence,
        Object resource,
        Object scope,
        Object limits,
        Object failedExports,
        Object admittedMetrics,
        Object droppedMetrics
    ) {
        Adt exportPayload = (Adt) payload;
        Adt exportResource = asAdt(resource, "Resource");
        Adt exportScope = asAdt(scope, "InstrumentationScope");
        Adt exportLimits = asAdt(limits, "ExporterLimits");
        long maximumCount = Math.max(0L, asLong(exportLimits.payloads[0]));
        long maximumStringSize = Math.max(0L, asLong(exportLimits.payloads[1]));
        long maximumRedacted = Math.max(0L, asLong(exportLimits.payloads[2]));
        java.util.Set<String> redactedNames = observationRedactedNames(exportLimits.payloads[3]);
        ExportAttributeFacts facts = new ExportAttributeFacts();

        String kind;
        String name;
        String severity = "null";
        String trace = "null";
        String span = "null";
        String links = "[]";
        String source;
        String attributes;
        String data = "";
        if (isAdt(exportPayload, "ExportLog")) {
            Adt record = asAdt(exportPayload.payloads[0], "LogRecord");
            kind = "log";
            name = asString(record.payloads[0]);
            String severityName = ((Adt) record.payloads[1]).name;
            int severitySeparator = severityName.lastIndexOf("::");
            if (severitySeparator >= 0) severityName = severityName.substring(severitySeparator + 2);
            severity = observationJsonString(severityName.toLowerCase(java.util.Locale.ROOT));
            data = ",\"message\":" + observationJsonString(asString(record.payloads[2]));
            attributes = observationAttributes(record.payloads[3], maximumCount, maximumStringSize, maximumRedacted, redactedNames, facts);
            trace = observationContext(record.payloads[4]);
            source = observationSource(record.payloads[5]);
        } else if (isAdt(exportPayload, "ExportEvent")) {
            Adt record = asAdt(exportPayload.payloads[0], "EventRecord");
            kind = "event";
            name = asString(record.payloads[0]);
            attributes = observationAttributes(record.payloads[1], maximumCount, maximumStringSize, maximumRedacted, redactedNames, facts);
            trace = observationContext(record.payloads[2]);
            source = observationSource(record.payloads[3]);
        } else if (isAdt(exportPayload, "ExportCounter")) {
            Adt record = asAdt(exportPayload.payloads[0], "CounterIncrementRecord");
            kind = "counter";
            name = asString(record.payloads[0]);
            data = ",\"value\":" + asLong(record.payloads[1]);
            attributes = observationAttributes(record.payloads[2], maximumCount, maximumStringSize, maximumRedacted, redactedNames, facts);
            trace = observationContext(record.payloads[3]);
            source = observationSource(record.payloads[4]);
        } else if (isAdt(exportPayload, "ExportHistogram")) {
            Adt record = asAdt(exportPayload.payloads[0], "HistogramSampleRecord");
            kind = "histogram";
            name = asString(record.payloads[0]);
            data = ",\"value\":" + asLong(record.payloads[1]);
            attributes = observationAttributes(record.payloads[2], maximumCount, maximumStringSize, maximumRedacted, redactedNames, facts);
            trace = observationContext(record.payloads[3]);
            source = observationSource(record.payloads[4]);
        } else if (isAdt(exportPayload, "ExportSpanStart")) {
            kind = "span_start";
            name = asString(exportPayload.payloads[0]);
            String traceId = asString(exportPayload.payloads[1]);
            String spanId = asString(exportPayload.payloads[2]);
            span = "{\"trace_id\":" + observationJsonString(traceId)
                + ",\"span_id\":" + observationJsonString(spanId)
                + ",\"parent_span_id\":" + observationOptionalString(exportPayload.payloads[3]) + "}";
            links = observationLinks(exportPayload.payloads[4]);
            source = observationOptionalSource(exportPayload.payloads[5]);
            attributes = "[]";
        } else if (isAdt(exportPayload, "ExportSpanFinish")) {
            kind = "span_finish";
            name = asString(exportPayload.payloads[0]);
            span = "{\"trace_id\":" + observationJsonString(asString(exportPayload.payloads[1]))
                + ",\"span_id\":" + observationJsonString(asString(exportPayload.payloads[2]))
                + ",\"parent_span_id\":" + observationOptionalString(exportPayload.payloads[3]) + "}";
            source = observationOptionalSource(exportPayload.payloads[4]);
            attributes = "[]";
        } else {
            throw new IllegalStateException("expected observation export payload");
        }
        String resourceAttributes = observationAttributes(exportResource.payloads[1], maximumCount, maximumStringSize, maximumRedacted, redactedNames, facts);
        String line = "{\"schema_version\":1,\"kind\":" + observationJsonString(kind)
            + ",\"name\":" + observationJsonString(name)
            + ",\"wall_time\":{\"unix_seconds\":" + asLong(recordField(wallTime, "unix_seconds"))
            + ",\"nanosecond\":" + asLong(recordField(wallTime, "nanosecond")) + "}"
            + ",\"sequence\":" + asLong(sequence)
            + ",\"severity\":" + severity
            + ",\"trace\":" + trace
            + ",\"span\":" + span
            + ",\"links\":" + links
            + ",\"source\":" + source
            + ",\"task_id\":null"
            + ",\"attributes\":" + attributes
            + ",\"resource\":{\"name\":" + observationJsonString(asString(exportResource.payloads[0]))
            + ",\"attributes\":" + resourceAttributes + "}"
            + ",\"instrumentation_scope\":{\"name\":" + observationJsonString(asString(exportScope.payloads[0]))
            + ",\"version\":" + observationJsonString(asString(exportScope.payloads[1])) + "}"
            + ",\"cardinality\":{\"admitted_metric_observations\":" + asLong(admittedMetrics)
            + ",\"dropped_metric_observations\":" + asLong(droppedMetrics) + "}"
            + ",\"exporter_health\":{\"failed_exports\":" + asLong(failedExports) + "}"
            + ",\"attribute_facts\":{\"dropped\":" + facts.dropped
            + ",\"truncated\":" + facts.truncated + ",\"redacted\":" + facts.redacted + "}"
            + data + "}\n";
        return record(
            "line", line,
            "dropped", Long.valueOf(facts.dropped),
            "truncated", Long.valueOf(facts.truncated),
            "redacted", Long.valueOf(facts.redacted)
        );
    }

    public static Object observationExportWrite(Object destination, Object line, Object diagnostic) {
        Adt configured = (Adt) destination;
        String target;
        if (isAdt(configured, "StandardError") || isAdt(configured, "DiagnosticStandardError")) target = "stderr";
        else if (isAdt(configured, "FileDestination") || isAdt(configured, "DiagnosticFileDestination")) target = "file:" + asString(configured.payloads[0]);
        else if (isAdt(configured, "NoDiagnosticDestination")) return Boolean.TRUE;
        else throw new IllegalStateException("expected observation export destination");
        HandlerFrame adapter = findHandler(OBSERVATION_EXPORTER_EFFECT);
        if (adapter != null) {
            Object result = invokeHandler(adapter, "write", new Object[] { target, asString(line), diagnostic });
            return (Boolean) result;
        }
        try {
            if (target.equals("stderr")) {
                System.err.print(asString(line));
                System.err.flush();
                if (System.err.checkError()) return Boolean.FALSE;
            } else {
                java.nio.file.Files.write(
                    java.nio.file.Paths.get(target.substring("file:".length())),
                    asString(line).getBytes(java.nio.charset.StandardCharsets.UTF_8),
                    java.nio.file.StandardOpenOption.CREATE,
                    java.nio.file.StandardOpenOption.APPEND
                );
            }
            return Boolean.TRUE;
        } catch (java.io.IOException | RuntimeException error) {
            return Boolean.FALSE;
        }
    }

    public static Object observationExportIdentity(Object prefix, Object kind, Object sequence) {
        HandlerFrame adapter = findHandler(OBSERVATION_EXPORTER_EFFECT);
        if (adapter != null) {
            return invokeHandler(adapter, "identity", new Object[] { kind, sequence });
        }
        return asString(prefix) + "-" + asString(kind) + "-" + java.util.UUID.randomUUID();
    }
