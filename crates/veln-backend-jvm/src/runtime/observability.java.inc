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
