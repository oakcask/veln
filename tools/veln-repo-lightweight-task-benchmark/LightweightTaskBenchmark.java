import java.lang.management.ManagementFactory;
import java.lang.management.MemoryMXBean;
import java.lang.ref.WeakReference;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

public final class LightweightTaskBenchmark {
    private static final int[] PARKED_POPULATIONS = {256, 1024, 4096, 16384};
    private static final int CHANNEL_OPERATIONS = 16384;
    private static final int CHANNEL_OFFERED_PER_SECOND = 2000;
    private static final int SELECT_DELIVERIES = 128;
    private static final int LOOPBACK_CONNECTIONS = 1024;
    private static final int LOOPBACK_OFFERED_PER_SECOND = 250;
    private static final int RECLAMATION_BATCHES = 10;
    private static final int RECLAMATION_POPULATION = 1024;
    private static final int CPU_SATURATORS = 3;
    private static final int CPU_SHORT_JOBS = 128;
    private static final long CPU_SATURATION_NANOS = 200_000_000L;
    private static final java.io.BufferedReader OBSERVATION_INPUT =
        new java.io.BufferedReader(new java.io.InputStreamReader(System.in));

    private static final class ParkedSample {
        final int population;
        final boolean completed;
        final long createNanos;
        final long completeNanos;
        final String failure;

        ParkedSample(int population, boolean completed,
                long createNanos, long completeNanos, String failure) {
            this.population = population;
            this.completed = completed;
            this.createNanos = createNanos;
            this.completeNanos = completeNanos;
            this.failure = failure;
        }
    }

    private static class TimedWorkload {
        final double throughput;
        final long p95Nanos;
        final long checksum;

        TimedWorkload(double throughput, long p95Nanos, long checksum) {
            this.throughput = throughput;
            this.p95Nanos = p95Nanos;
            this.checksum = checksum;
        }
    }

    private static final class SelectSample {
        final long p95Nanos;
        final long elapsedCpuNanos;
        final long wakeups;
        final long checksum;

        SelectSample(long p95Nanos, long elapsedCpuNanos, long wakeups, long checksum) {
            this.p95Nanos = p95Nanos;
            this.elapsedCpuNanos = elapsedCpuNanos;
            this.wakeups = wakeups;
            this.checksum = checksum;
        }
    }

    private static final class CpuSaturationSample extends TimedWorkload {
        final long cancellationLatencyNanos;
        final boolean cancellationObserved;

        CpuSaturationSample(double throughput, long p95Nanos, long checksum,
                long cancellationLatencyNanos, boolean cancellationObserved) {
            super(throughput, p95Nanos, checksum);
            this.cancellationLatencyNanos = cancellationLatencyNanos;
            this.cancellationObserved = cancellationObserved;
        }
    }

    private static final class LoopbackSample extends TimedWorkload {
        final long descriptorGrowth;
        final long selectorOperations;

        LoopbackSample(double throughput, long p95Nanos, long checksum,
                long descriptorGrowth, long selectorOperations) {
            super(throughput, p95Nanos, checksum);
            this.descriptorGrowth = descriptorGrowth;
            this.selectorOperations = selectorOperations;
        }
    }

    private static final class ReclamationSample {
        final long[] liveHeapBytes;
        final int completedRegistrations;
        final int retainedOwners;

        ReclamationSample(long[] liveHeapBytes, int completedRegistrations, int retainedOwners) {
            this.liveHeapBytes = liveHeapBytes;
            this.completedRegistrations = completedRegistrations;
            this.retainedOwners = retainedOwners;
        }
    }

    private static final class Owner {
        final long marker;
        Owner(long marker) { this.marker = marker; }
    }

    private LightweightTaskBenchmark() { }

    public static void main(String[] args) throws Exception {
        System.err.println("workload: parked");
        List<ParkedSample> parked = runParkedLadder();
        System.err.println("workload: preparation");
        runChannelPipeline(0L, 2048);
        runChannelPipeline(64L, 2048);
        runLoopback(32);
        System.err.println("workload: rendezvous channel");
        TimedWorkload rendezvous = runChannelPipeline(0L, CHANNEL_OPERATIONS);
        System.err.println("workload: bounded channel");
        TimedWorkload bounded = runChannelPipeline(64L, CHANNEL_OPERATIONS);
        System.err.println("workload: select");
        SelectSample select = runIdleSelect();
        System.err.println("workload: loopback");
        LoopbackSample loopback = runLoopback(LOOPBACK_CONNECTIONS);
        System.err.println("workload: CPU saturation");
        CpuSaturationSample cpu = runCpuSaturation();
        System.err.println("workload: reclamation");
        ReclamationSample reclamation = runReclamation();
        System.out.println(toJson(parked, rendezvous, bounded, select, loopback,
            cpu, reclamation));
        System.err.println("workload: complete");
    }

    private static List<ParkedSample> runParkedLadder() throws Exception {
        List<ParkedSample> samples = new ArrayList<>();
        for (int population : PARKED_POPULATIONS) {
            beginParkedObservation(population);
            Object release = VelnRuntime.channelBounded(Long.valueOf(0L));
            Object releaseTx = VelnRuntime.recordField(release, "tx");
            Object releaseRx = VelnRuntime.recordField(release, "rx");
            ArrayList<Object> handles = new ArrayList<>(population);
            long started = System.nanoTime();
            String failure = null;
            VelnRuntime.beginTaskLifetime();
            try {
                for (int index = 0; index < population; index += 1) {
                    handles.add(VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
                        VelnRuntime.channelRecv(releaseRx);
                        return Long.valueOf(1L);
                    }));
                }
                if (!awaitChannelReceivers(
                    releaseRx,
                    population,
                    30L,
                    TimeUnit.SECONDS
                )) {
                    failure = "population did not reach the channel receive boundary";
                }
            } catch (Throwable resourceFailure) {
                failure = resourceFailure.getClass().getName() + ": "
                    + String.valueOf(resourceFailure.getMessage());
            }
            long created = System.nanoTime();
            if (failure == null) {
                finishParkedObservation(population);
                VelnRuntime.channelClose(releaseTx);
            }
            VelnRuntime.finishTaskLifetime(failure == null);
            long completed = System.nanoTime();
            boolean allJoined = failure == null;
            if (allJoined) {
                for (Object handle : handles) {
                    allJoined &= "Ok(1)".equals(VelnRuntime.taskJoin(handle).toString());
                }
            }
            samples.add(new ParkedSample(population, allJoined,
                created - started, completed - created, failure));
            if (!allJoined) break;
        }
        return samples;
    }

    private static boolean awaitChannelReceivers(
        Object receiver,
        long expected,
        long timeout,
        TimeUnit unit
    ) throws Exception {
        java.lang.reflect.Field channelField = receiver.getClass().getDeclaredField("channel");
        channelField.setAccessible(true);
        Object channel = channelField.get(receiver);
        java.lang.reflect.Field waitingField = channel.getClass().getDeclaredField("waitingReceivers");
        waitingField.setAccessible(true);
        long deadline = System.nanoTime() + unit.toNanos(timeout);
        while (System.nanoTime() < deadline) {
            synchronized (channel) {
                if (waitingField.getLong(channel) == expected) {
                    return true;
                }
            }
            Thread.yield();
        }
        synchronized (channel) {
            return waitingField.getLong(channel) == expected;
        }
    }

    private static void beginParkedObservation(int population) {
        System.err.println("VELN_BENCHMARK_OBSERVE\tstart\t" + population);
    }

    private static void finishParkedObservation(int population) throws Exception {
        System.err.println("VELN_BENCHMARK_OBSERVE\tready\t" + population);
        String response = OBSERVATION_INPUT.readLine();
        if (!"continue".equals(response)) {
            throw new IllegalStateException(
                "external parked observation was not acknowledged for " + population
            );
        }
    }

    private static TimedWorkload runChannelPipeline(long capacity, int operations) {
        Object channelOne = VelnRuntime.channelBounded(Long.valueOf(capacity));
        Object channelTwo = VelnRuntime.channelBounded(Long.valueOf(capacity));
        Object txOne = VelnRuntime.recordField(channelOne, "tx");
        Object rxOne = VelnRuntime.recordField(channelOne, "rx");
        Object txTwo = VelnRuntime.recordField(channelTwo, "tx");
        Object rxTwo = VelnRuntime.recordField(channelTwo, "rx");
        long[] latencies = new long[operations];
        long started = System.nanoTime();
        VelnRuntime.beginTaskLifetime();
        Object relay = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            for (int index = 0; index < operations; index += 1) {
                Object received = VelnRuntime.channelRecv(rxOne);
                VelnRuntime.channelSend(txTwo, VelnRuntime.optionValue(received));
            }
            return Long.valueOf(operations);
        });
        Object consumer = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            long sum = 0L;
            for (int index = 0; index < operations; index += 1) {
                long sent = ((Long) VelnRuntime.optionValue(VelnRuntime.channelRecv(rxTwo))).longValue();
                latencies[index] = System.nanoTime() - sent;
                sum += index;
            }
            return Long.valueOf(sum);
        });
        for (int index = 0; index < operations; index += 1) {
            awaitOfferedRate(started, index, CHANNEL_OFFERED_PER_SECOND);
            VelnRuntime.channelSend(txOne, Long.valueOf(System.nanoTime()));
        }
        VelnRuntime.finishTaskLifetime(true);
        long checksum = resultLong(VelnRuntime.taskJoin(relay)) + resultLong(VelnRuntime.taskJoin(consumer));
        long elapsed = System.nanoTime() - started;
        return new TimedWorkload(operations * 1_000_000_000.0 / elapsed,
            percentile95(latencies), checksum);
    }

    private static SelectSample runIdleSelect() throws Exception {
        Object left = VelnRuntime.channelBounded(Long.valueOf(8L));
        Object right = VelnRuntime.channelBounded(Long.valueOf(8L));
        Object leftTx = VelnRuntime.recordField(left, "tx");
        Object leftRx = VelnRuntime.recordField(left, "rx");
        Object rightRx = VelnRuntime.recordField(right, "rx");
        long[] latencies = new long[SELECT_DELIVERIES];
        long[] wakeups = {0L};
        long cpuBefore = processCpuNanos();
        VelnRuntime.beginTaskLifetime();
        Object selector = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            long sum = 0L;
            int delivered = 0;
            while (delivered < SELECT_DELIVERIES) {
                Object selected = VelnRuntime.channelSelectTimeout(leftRx, rightRx, Long.valueOf(1L));
                wakeups[0] += 1L;
                if (VelnRuntime.isSome(selected)) {
                    Object selectedRecord = VelnRuntime.optionValue(selected);
                    long sent = ((Long) VelnRuntime.recordField(selectedRecord, "value")).longValue();
                    latencies[delivered] = System.nanoTime() - sent;
                    sum += delivered;
                    delivered += 1;
                }
            }
            return Long.valueOf(sum);
        });
        for (int index = 0; index < SELECT_DELIVERIES; index += 1) {
            Thread.sleep(2L);
            VelnRuntime.channelSend(leftTx, Long.valueOf(System.nanoTime()));
        }
        VelnRuntime.finishTaskLifetime(true);
        long checksum = resultLong(VelnRuntime.taskJoin(selector));
        return new SelectSample(percentile95(latencies), processCpuNanos() - cpuBefore,
            wakeups[0], checksum);
    }

    private static LoopbackSample runLoopback(int connections) throws Exception {
        long descriptorsBefore = descriptorCount();
        Object listener = VelnRuntime.netListen("127.0.0.1:0");
        String address = (String) VelnRuntime.netListenerLocalAddr(listener);
        Object payload = VelnRuntime.resultValue(VelnRuntime.byteChunkFromVisibleAsciiString(
            "veln-lightweight-task-loopback-payload"));
        long[] latencies = new long[connections];
        long[] submitted = new long[connections];
        long[] clientCounts = new long[connections];
        long started = System.nanoTime();
        VelnRuntime.beginTaskLifetime();
        Object server = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            long count = 0L;
            for (int index = 0; index < connections; index += 1) {
                Object stream = VelnRuntime.netAccept(listener);
                Object received = VelnRuntime.netReadChunk(stream);
                VelnRuntime.netWriteChunk(stream, received);
                VelnRuntime.netCloseStream(stream);
                count += 1L;
            }
            return Long.valueOf(count);
        });
        ArrayList<Object> clients = new ArrayList<>(connections);
        for (int index = 0; index < connections; index += 1) {
            final int clientIndex = index;
            awaitOfferedRate(started, index, LOOPBACK_OFFERED_PER_SECOND);
            submitted[index] = System.nanoTime();
            clients.add(VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
                Object stream = VelnRuntime.netConnect(address);
                VelnRuntime.netWriteChunk(stream, payload);
                Object received = VelnRuntime.netReadChunk(stream);
                clientCounts[clientIndex] = ((Long) VelnRuntime.byteCountToInt(
                    VelnRuntime.byteChunkCount(received))).longValue();
                VelnRuntime.netCloseStream(stream);
                latencies[clientIndex] = System.nanoTime() - submitted[clientIndex];
                return Long.valueOf(clientIndex);
            }));
        }
        VelnRuntime.finishTaskLifetime(true);
        long checksum = resultLong(VelnRuntime.taskJoin(server));
        for (int index = 0; index < clients.size(); index += 1) {
            checksum += resultLong(VelnRuntime.taskJoin(clients.get(index)));
            checksum += clientCounts[index];
        }
        VelnRuntime.netCloseListener(listener);
        long elapsed = System.nanoTime() - started;
        long descriptorGrowth = Math.max(0L, descriptorCount() - descriptorsBefore);
        return new LoopbackSample(connections * 1_000_000_000.0 / elapsed,
            percentile95(latencies), checksum, descriptorGrowth, connections * 4L);
    }

    private static CpuSaturationSample runCpuSaturation() throws Exception {
        long[] latencies = new long[CPU_SHORT_JOBS];
        CountDownLatch carriersOccupied = new CountDownLatch(2);
        VelnRuntime.beginTaskLifetime();
        for (int worker = 0; worker < CPU_SATURATORS; worker += 1) {
            VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
                carriersOccupied.countDown();
                long until = System.nanoTime() + CPU_SATURATION_NANOS;
                long value = 0L;
                while (System.nanoTime() < until) value += 1L;
                return Long.valueOf(value);
            });
        }
        if (!carriersOccupied.await(10, TimeUnit.SECONDS)) {
            throw new AssertionError("CPU saturators did not occupy both carriers");
        }
        java.util.concurrent.atomic.AtomicLong cancellationDelivered =
            new java.util.concurrent.atomic.AtomicLong(0L);
        Object cancellationTarget = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            try {
                while (true) VelnRuntime.taskCancellationCheckpoint();
            } catch (RuntimeException cancelled) {
                cancellationDelivered.compareAndSet(0L, System.nanoTime());
                throw cancelled;
            }
        });
        long cancellationRequested = System.nanoTime();
        VelnRuntime.taskCancel(cancellationTarget);
        long started = System.nanoTime();
        ArrayList<Object> handles = new ArrayList<>(CPU_SHORT_JOBS);
        for (int index = 0; index < CPU_SHORT_JOBS; index += 1) {
            final int job = index;
            final long submitted = System.nanoTime();
            handles.add(VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
                latencies[job] = System.nanoTime() - submitted;
                return Long.valueOf(job);
            }));
        }
        VelnRuntime.finishTaskLifetime(true);
        long checksum = 0L;
        for (Object handle : handles) checksum += resultLong(VelnRuntime.taskJoin(handle));
        Object cancellationResult = VelnRuntime.taskJoin(cancellationTarget);
        boolean cancellationObserved = cancellationResult.toString().contains("cancelled");
        long cancellationLatency = cancellationDelivered.get() - cancellationRequested;
        if (!cancellationObserved) {
            throw new AssertionError("CPU-saturated cancellation was not observed: "
                + cancellationResult);
        }
        if (cancellationLatency < 0L) {
            throw new AssertionError("CPU-saturated cancellation delivery was not timestamped");
        }
        long elapsed = System.nanoTime() - started;
        return new CpuSaturationSample(CPU_SHORT_JOBS * 1_000_000_000.0 / elapsed,
            percentile95(latencies), checksum, cancellationLatency, cancellationObserved);
    }

    private static ReclamationSample runReclamation() throws Exception {
        long[] heaps = new long[RECLAMATION_BATCHES];
        ArrayList<WeakReference<Owner>> owners = new ArrayList<>();
        int registrations = 0;
        for (int batch = 0; batch < RECLAMATION_BATCHES; batch += 1) {
            Owner[] holder = {new Owner(batch)};
            owners.add(new WeakReference<>(holder[0]));
            VelnRuntime.beginTaskLifetime();
            Object lifetime = currentTaskLifetime();
            for (int index = 0; index < RECLAMATION_POPULATION; index += 1) {
                VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> Long.valueOf(holder[0].marker));
            }
            VelnRuntime.finishTaskLifetime(true);
            registrations += lifetimeRegistrations(lifetime);
            lifetime = null;
            holder[0] = null;
            collect();
            heaps[batch] = usedHeapBytes();
        }
        collect();
        int retained = 0;
        for (WeakReference<Owner> owner : owners) if (owner.get() != null) retained += 1;
        return new ReclamationSample(heaps, registrations, retained);
    }

    private static Object currentTaskLifetime() throws Exception {
        java.lang.reflect.Field localField = VelnRuntime.class.getDeclaredField("CURRENT_TASK_LIFETIME");
        localField.setAccessible(true);
        ThreadLocal<?> local = (ThreadLocal<?>) localField.get(null);
        Object lifetime = local.get();
        if (lifetime == null) {
            throw new AssertionError("active task lifetime was not observable");
        }
        return lifetime;
    }

    private static int lifetimeRegistrations(Object lifetime) throws Exception {
        java.lang.reflect.Field incompleteField = lifetime.getClass().getDeclaredField("incomplete");
        incompleteField.setAccessible(true);
        return ((java.util.Set<?>) incompleteField.get(lifetime)).size();
    }

    private static void collect() throws InterruptedException {
        System.gc();
        System.runFinalization();
        System.gc();
        Thread.sleep(20L);
    }

    private static long usedHeapBytes() {
        MemoryMXBean memory = ManagementFactory.getMemoryMXBean();
        return memory.getHeapMemoryUsage().getUsed();
    }

    private static long descriptorCount() {
        try (java.util.stream.Stream<Path> paths = Files.list(Path.of("/proc/self/fd"))) {
            return paths.count();
        } catch (Exception unavailable) {
            return -1L;
        }
    }

    private static long processCpuNanos() {
        return ((com.sun.management.OperatingSystemMXBean)
            ManagementFactory.getOperatingSystemMXBean()).getProcessCpuTime();
    }

    private static long resultLong(Object result) {
        if (!VelnRuntime.isOk(result)) throw new AssertionError("task failed: " + result);
        return ((Long) VelnRuntime.resultValue(result)).longValue();
    }

    private static void awaitOfferedRate(long started, int operation, int perSecond) {
        long target = started + operation * (1_000_000_000L / perSecond);
        while (true) {
            long remaining = target - System.nanoTime();
            if (remaining <= 0L) return;
            java.util.concurrent.locks.LockSupport.parkNanos(remaining);
        }
    }

    private static long percentile95(long[] values) {
        long[] sorted = values.clone();
        Arrays.sort(sorted);
        return sorted[(int) Math.ceil(sorted.length * 0.95) - 1];
    }

    private static String quote(String value) {
        if (value == null) return "null";
        return "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"")
            .replace("\n", "\\n") + "\"";
    }

    private static String toJson(List<ParkedSample> parked, TimedWorkload rendezvous,
            TimedWorkload bounded, SelectSample select, LoopbackSample loopback,
            CpuSaturationSample cpu, ReclamationSample reclamation) {
        StringBuilder json = new StringBuilder("{\"schema_version\":2,\"workload\":{");
        json.append("\"parked\":[");
        for (int index = 0; index < parked.size(); index += 1) {
            if (index > 0) json.append(',');
            ParkedSample sample = parked.get(index);
            json.append("{\"population\":").append(sample.population)
                .append(",\"completed\":").append(sample.completed)
                .append(",\"create_nanos\":").append(sample.createNanos)
                .append(",\"complete_nanos\":").append(sample.completeNanos)
                .append(",\"failure\":").append(quote(sample.failure)).append('}');
        }
        json.append("],\"rendezvous_channel\":").append(timedJson(rendezvous))
            .append(",\"bounded_channel\":").append(timedJson(bounded))
            .append(",\"select\":{\"latency_p95_nanos\":").append(select.p95Nanos)
            .append(",\"idle_cpu_nanos\":").append(select.elapsedCpuNanos)
            .append(",\"wakeups\":").append(select.wakeups).append('}')
            .append(",\"loopback\":").append(timedJson(loopback).replace("}",
                ",\"descriptor_growth\":" + loopback.descriptorGrowth
                + ",\"selector_operations\":" + loopback.selectorOperations + "}"))
            .append(",\"cpu_saturation\":")
            .append(timedJson(cpu).replace("}",
                ",\"cancellation_latency_nanos\":" + cpu.cancellationLatencyNanos
                + ",\"cancellation_observed\":" + cpu.cancellationObserved + "}"))
            .append(",\"reclamation\":{\"live_heap_bytes\":")
            .append(Arrays.toString(reclamation.liveHeapBytes))
            .append(",\"completed_registrations\":").append(reclamation.completedRegistrations)
            .append(",\"retained_owners\":").append(reclamation.retainedOwners).append("}},")
            .append("\"functional_outcomes\":{")
            .append("\"parked\":{\"completed_populations\":[");
        boolean firstCompleted = true;
        for (ParkedSample sample : parked) {
            if (!sample.completed) continue;
            if (!firstCompleted) json.append(',');
            json.append(sample.population);
            firstCompleted = false;
        }
        json.append("]},\"rendezvous_channel\":{\"checksum\":")
            .append(rendezvous.checksum)
            .append("},\"bounded_channel\":{\"checksum\":").append(bounded.checksum)
            .append("},\"select\":{\"checksum\":").append(select.checksum)
            .append("},\"loopback\":{\"checksum\":").append(loopback.checksum)
            .append("},\"cpu_saturation\":{\"checksum\":").append(cpu.checksum)
            .append("},\"reclamation\":{\"completed_registrations\":")
            .append(reclamation.completedRegistrations)
            .append(",\"retained_owners\":").append(reclamation.retainedOwners)
            .append("}}}");
        return json.toString();
    }

    private static String timedJson(TimedWorkload sample) {
        return "{\"throughput_per_second\":" + sample.throughput
            + ",\"latency_p95_nanos\":" + sample.p95Nanos + "}";
    }
}
