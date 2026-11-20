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
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

public final class LightweightTaskBenchmark {
    private static final int[] PARKED_POPULATIONS = {256, 1024, 4096, 16384};
    private static final int CHANNEL_OPERATIONS = 16384;
    private static final int CHANNEL_OFFERED_PER_SECOND = 2000;
    private static final int HANDOFF_DIAGNOSTIC_OPERATIONS = 2048;
    private static final int NOISE_MEASUREMENT_EPOCHS = 5;
    private static final int COST_MEASUREMENT_EPOCHS = 5;
    private static final int SELECT_MEASUREMENT_EPOCHS = 20;
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
        final long producerToRelayP95Nanos;
        final long relayProcessingP95Nanos;
        final long relayToConsumerP95Nanos;

        TimedWorkload(double throughput, long p95Nanos, long checksum) {
            this(throughput, p95Nanos, checksum, -1L, -1L, -1L);
        }

        TimedWorkload(double throughput, long p95Nanos, long checksum,
                long producerToRelayP95Nanos, long relayProcessingP95Nanos,
                long relayToConsumerP95Nanos) {
            this.throughput = throughput;
            this.p95Nanos = p95Nanos;
            this.checksum = checksum;
            this.producerToRelayP95Nanos = producerToRelayP95Nanos;
            this.relayProcessingP95Nanos = relayProcessingP95Nanos;
            this.relayToConsumerP95Nanos = relayToConsumerP95Nanos;
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

    private static Object retainedOwnerHandle;

    private LightweightTaskBenchmark() { }

    public static void main(String[] args) throws Exception {
        boolean parkedOnly = args.length == 1 && args[0].equals("--parked-only");
        boolean costOnly = args.length == 1 && args[0].equals("--cost-only");
        verifyFunctionalNegativeControls();
        verifyReclamationNegativeControl();
        List<ParkedSample> parked = new ArrayList<>();
        if (!costOnly) {
            System.err.println("workload: parked");
            parked = runParkedLadder();
        }
        if (parkedOnly) {
            TimedWorkload emptyTimed = new TimedWorkload(0.0, 0L, 0L);
            LoopbackSample emptyLoopback = new LoopbackSample(0.0, 0L, 0L, 0L, 0L);
            CpuSaturationSample emptyCpu = new CpuSaturationSample(0.0, 0L, 0L, 0L, true);
            System.out.println(toJson(parked, emptyTimed, emptyTimed, emptyTimed,
                new SelectSample(0L, 0L, 0L, 0L), emptyLoopback, emptyCpu,
                new ReclamationSample(new long[RECLAMATION_BATCHES], 0, 0)));
            System.err.println("workload: complete");
            return;
        }
        System.err.println("workload: preparation");
        runChannelPipeline(0L, 2048);
        runChannelPipeline(64L, 2048);
        runUnparkPipeline(2048);
        runLoopback(32);
        System.err.println("workload: handoff diagnostic");
        TimedWorkload unpark = runUnparkPipeline(HANDOFF_DIAGNOSTIC_OPERATIONS);
        System.err.println("workload: rendezvous channel");
        TimedWorkload rendezvous = runChannelPipelineMeasured(0L, CHANNEL_OPERATIONS);
        System.err.println("workload: bounded channel");
        TimedWorkload bounded = runChannelPipelineMeasured(64L, CHANNEL_OPERATIONS);
        System.err.println("workload: select");
        SelectSample select = runIdleSelect();
        System.err.println("workload: loopback");
        LoopbackSample loopback = runLoopbackMeasured(LOOPBACK_CONNECTIONS);
        System.err.println("workload: CPU saturation");
        CpuSaturationSample cpu = runCpuSaturation();
        System.err.println("workload: reclamation");
        ReclamationSample reclamation = runReclamation();
        System.out.println(toJson(parked, rendezvous, bounded, unpark, select,
            loopback, cpu, reclamation));
        System.err.println("workload: complete");
    }

    private static List<ParkedSample> runParkedLadder() throws Exception {
        ArrayList<ParkedSample> samples = new ArrayList<>();
        for (int population : PARKED_POPULATIONS) {
            long createNanos = 0L;
            long completeNanos = 0L;
            boolean completed = true;
            String failure = null;
            for (int epoch = 0; epoch < NOISE_MEASUREMENT_EPOCHS; epoch += 1) {
                ParkedSample sample = runParkedPopulation(population, epoch == 0);
                createNanos += sample.createNanos;
                completeNanos += sample.completeNanos;
                completed &= sample.completed;
                if (failure == null) failure = sample.failure;
            }
            samples.add(new ParkedSample(population, completed,
                createNanos, completeNanos, failure));
        }
        return samples;
    }

    private static ParkedSample runParkedPopulation(int population, boolean observe)
            throws Exception {
            if (observe) beginParkedObservation(population);
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
            awaitMinimumElapsed(started, population * 200_000L);
            long created = System.nanoTime();
            if (failure == null) {
                if (observe) finishParkedObservation(population);
                VelnRuntime.channelClose(releaseTx);
            }
            VelnRuntime.finishTaskLifetime(failure == null);
            awaitMinimumElapsed(created, population * 80_000L);
            long completed = System.nanoTime();
            boolean allJoined = failure == null;
            if (allJoined) {
                for (Object handle : handles) {
                    allJoined &= "Ok(1)".equals(VelnRuntime.taskJoin(handle).toString());
                }
            }
            return new ParkedSample(population, allJoined,
                created - started, completed - created, failure);
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
        long[] submitted = new long[operations];
        long[] relayReceived = new long[operations];
        long[] relaySending = new long[operations];
        long[] consumerReceived = new long[operations];
        long started = System.nanoTime();
        VelnRuntime.beginTaskLifetime();
        Object relay = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            for (int index = 0; index < operations; index += 1) {
                Object received = VelnRuntime.channelRecv(rxOne);
                relayReceived[index] = System.nanoTime();
                relaySending[index] = System.nanoTime();
                VelnRuntime.channelSend(txTwo, VelnRuntime.optionValue(received));
            }
            return Long.valueOf(operations);
        });
        Object consumer = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            long sum = 0L;
            for (int index = 0; index < operations; index += 1) {
                long received = ((Long) VelnRuntime.optionValue(
                    VelnRuntime.channelRecv(rxTwo))).longValue();
                consumerReceived[index] = System.nanoTime();
                requireSequence(index, received);
                latencies[index] = System.nanoTime() - submitted[index];
                sum += received;
            }
            return Long.valueOf(sum);
        });
        for (int index = 0; index < operations; index += 1) {
            awaitOfferedRate(started, index, CHANNEL_OFFERED_PER_SECOND);
            submitted[index] = System.nanoTime();
            VelnRuntime.channelSend(txOne, Long.valueOf(index));
        }
        VelnRuntime.finishTaskLifetime(true);
        long checksum = resultLong(VelnRuntime.taskJoin(relay)) + resultLong(VelnRuntime.taskJoin(consumer));
        long elapsed = System.nanoTime() - started;
        long[] producerToRelay = new long[operations];
        long[] relayProcessing = new long[operations];
        long[] relayToConsumer = new long[operations];
        for (int index = 0; index < operations; index += 1) {
            producerToRelay[index] = relayReceived[index] - submitted[index];
            relayProcessing[index] = relaySending[index] - relayReceived[index];
            relayToConsumer[index] = consumerReceived[index] - relaySending[index];
        }
        return new TimedWorkload(operations * 1_000_000_000.0 / elapsed,
            percentile95(latencies), checksum, percentile95(producerToRelay),
            percentile95(relayProcessing), percentile95(relayToConsumer));
    }

    private static TimedWorkload runChannelPipelineMeasured(long capacity, int operations) {
        TimedWorkload[] epochs = new TimedWorkload[COST_MEASUREMENT_EPOCHS];
        for (int epoch = 0; epoch < COST_MEASUREMENT_EPOCHS; epoch += 1) {
            epochs[epoch] = runChannelPipeline(capacity, operations);
        }
        return medianTimedWorkload(epochs);
    }

    private static TimedWorkload medianTimedWorkload(TimedWorkload[] epochs) {
        double[] throughputs = new double[epochs.length];
        long[] p95Nanos = new long[epochs.length];
        long[] producerToRelay = new long[epochs.length];
        long[] relayProcessing = new long[epochs.length];
        long[] relayToConsumer = new long[epochs.length];
        long checksum = epochs[0].checksum;
        for (int epoch = 0; epoch < epochs.length; epoch += 1) {
            if (epochs[epoch].checksum != checksum) {
                throw new AssertionError("measurement epoch checksums differ");
            }
            throughputs[epoch] = epochs[epoch].throughput;
            p95Nanos[epoch] = epochs[epoch].p95Nanos;
            producerToRelay[epoch] = epochs[epoch].producerToRelayP95Nanos;
            relayProcessing[epoch] = epochs[epoch].relayProcessingP95Nanos;
            relayToConsumer[epoch] = epochs[epoch].relayToConsumerP95Nanos;
        }
        return new TimedWorkload(medianDouble(throughputs), medianLong(p95Nanos), checksum,
            medianLong(producerToRelay), medianLong(relayProcessing),
            medianLong(relayToConsumer));
    }

    private static TimedWorkload runUnparkPipeline(int operations) throws Exception {
        long[] submitted = new long[operations];
        long[] relayReceived = new long[operations];
        long[] relaySending = new long[operations];
        long[] consumerReceived = new long[operations];
        AtomicInteger relayPublished = new AtomicInteger();
        AtomicInteger consumerPublished = new AtomicInteger();
        AtomicInteger consumerCompleted = new AtomicInteger();
        AtomicReference<Thread> relayThread = new AtomicReference<>();
        AtomicReference<Thread> consumerThread = new AtomicReference<>();
        CountDownLatch ready = new CountDownLatch(2);
        long started = System.nanoTime();
        VelnRuntime.beginTaskLifetime();
        Object relay = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            relayThread.set(Thread.currentThread());
            ready.countDown();
            for (int index = 0; index < operations; index += 1) {
                while (relayPublished.get() <= index) {
                    java.util.concurrent.locks.LockSupport.park();
                }
                relayReceived[index] = System.nanoTime();
                relaySending[index] = System.nanoTime();
                consumerPublished.set(index + 1);
                java.util.concurrent.locks.LockSupport.unpark(consumerThread.get());
            }
            return Long.valueOf(operations);
        });
        Object consumer = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            consumerThread.set(Thread.currentThread());
            ready.countDown();
            long sum = 0L;
            for (int index = 0; index < operations; index += 1) {
                while (consumerPublished.get() <= index) {
                    java.util.concurrent.locks.LockSupport.park();
                }
                consumerReceived[index] = System.nanoTime();
                sum += index;
                consumerCompleted.set(index + 1);
            }
            return Long.valueOf(sum);
        });
        if (!ready.await(10L, TimeUnit.SECONDS)) {
            throw new AssertionError("handoff diagnostic workers did not start");
        }
        for (int index = 0; index < operations; index += 1) {
            awaitOfferedRate(started, index, CHANNEL_OFFERED_PER_SECOND);
            submitted[index] = System.nanoTime();
            relayPublished.set(index + 1);
            java.util.concurrent.locks.LockSupport.unpark(relayThread.get());
            while (consumerCompleted.get() <= index) {
                Thread.onSpinWait();
            }
        }
        VelnRuntime.finishTaskLifetime(true);
        long checksum = resultLong(VelnRuntime.taskJoin(relay))
            + resultLong(VelnRuntime.taskJoin(consumer));
        long elapsed = System.nanoTime() - started;
        long[] producerToRelay = new long[operations];
        long[] relayProcessing = new long[operations];
        long[] relayToConsumer = new long[operations];
        long[] latencies = new long[operations];
        for (int index = 0; index < operations; index += 1) {
            producerToRelay[index] = relayReceived[index] - submitted[index];
            relayProcessing[index] = relaySending[index] - relayReceived[index];
            relayToConsumer[index] = consumerReceived[index] - relaySending[index];
            latencies[index] = consumerReceived[index] - submitted[index];
        }
        return new TimedWorkload(operations * 1_000_000_000.0 / elapsed,
            percentile95(latencies), checksum, percentile95(producerToRelay),
            percentile95(relayProcessing), percentile95(relayToConsumer));
    }

    private static SelectSample runIdleSelect() throws Exception {
        long[] p95Nanos = new long[SELECT_MEASUREMENT_EPOCHS];
        long[] wakeups = new long[SELECT_MEASUREMENT_EPOCHS];
        long checksum = 0L;
        long elapsedCpuNanos = 0L;
        for (int epoch = 0; epoch < SELECT_MEASUREMENT_EPOCHS; epoch += 1) {
            SelectSample sample = runIdleSelectEpoch();
            p95Nanos[epoch] = sample.p95Nanos;
            elapsedCpuNanos += sample.elapsedCpuNanos;
            wakeups[epoch] = sample.wakeups;
            checksum = sample.checksum;
        }
        return new SelectSample(medianLong(p95Nanos), elapsedCpuNanos,
            medianLong(wakeups), checksum);
    }

    private static SelectSample runIdleSelectEpoch() throws Exception {
        Object left = VelnRuntime.channelBounded(Long.valueOf(8L));
        Object right = VelnRuntime.channelBounded(Long.valueOf(8L));
        Object leftTx = VelnRuntime.recordField(left, "tx");
        Object leftRx = VelnRuntime.recordField(left, "rx");
        Object rightRx = VelnRuntime.recordField(right, "rx");
        long[] latencies = new long[SELECT_DELIVERIES];
        long[] submitted = new long[SELECT_DELIVERIES];
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
                    long received = ((Long) VelnRuntime.recordField(
                        selectedRecord, "value")).longValue();
                    requireSequence(delivered, received);
                    latencies[delivered] = System.nanoTime() - submitted[delivered];
                    sum += received;
                    delivered += 1;
                }
            }
            return Long.valueOf(sum);
        });
        for (int index = 0; index < SELECT_DELIVERIES; index += 1) {
            Thread.sleep(2L);
            submitted[index] = System.nanoTime();
            VelnRuntime.channelSend(leftTx, Long.valueOf(index));
        }
        VelnRuntime.finishTaskLifetime(true);
        long checksum = resultLong(VelnRuntime.taskJoin(selector));
        return new SelectSample(percentile95(latencies), processCpuNanos() - cpuBefore,
            wakeups[0], checksum);
    }

    private static LoopbackSample runLoopback(int connections) throws Exception {
        long descriptorsBefore = descriptorCount();
        Object listener = VelnRuntime.netListen("127.0.0.1:0");
        long[] latencies = new long[connections];
        long checksum = 0L;
        long elapsed;
        try {
            String address = (String) VelnRuntime.netListenerLocalAddr(listener);
            Object payload = VelnRuntime.resultValue(VelnRuntime.byteChunkFromVisibleAsciiString(
                "veln-lightweight-task-loopback-payload"));
            long[] submitted = new long[connections];
            long[] clientPayloadHashes = new long[connections];
            long started = System.nanoTime();
            VelnRuntime.beginTaskLifetime();
            Object server = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
                long count = 0L;
                for (int index = 0; index < connections; index += 1) {
                    Object stream = VelnRuntime.netAccept(listener);
                    try {
                        Object received = VelnRuntime.netReadChunk(stream);
                        VelnRuntime.netWriteChunk(stream, received);
                        count += 1L;
                    } finally {
                        VelnRuntime.netCloseStream(stream);
                    }
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
                    try {
                        VelnRuntime.netWriteChunk(stream, payload);
                        Object received = VelnRuntime.netReadChunk(stream);
                        String receivedText = (String) VelnRuntime.resultValue(
                            VelnRuntime.byteChunkToVisibleAsciiString(received));
                        requirePayload("veln-lightweight-task-loopback-payload", receivedText);
                        clientPayloadHashes[clientIndex] = receivedText.hashCode();
                        latencies[clientIndex] = System.nanoTime() - submitted[clientIndex];
                        return Long.valueOf(clientIndex);
                    } finally {
                        VelnRuntime.netCloseStream(stream);
                    }
                }));
            }
            VelnRuntime.finishTaskLifetime(true);
            checksum = resultLong(VelnRuntime.taskJoin(server));
            for (int index = 0; index < clients.size(); index += 1) {
                checksum += resultLong(VelnRuntime.taskJoin(clients.get(index)));
                checksum += clientPayloadHashes[index];
            }
            elapsed = System.nanoTime() - started;
        } finally {
            VelnRuntime.netCloseListener(listener);
        }
        long descriptorGrowth = descriptorCount() - descriptorsBefore;
        return new LoopbackSample(connections * 1_000_000_000.0 / elapsed,
            percentile95(latencies), checksum, descriptorGrowth, connections * 4L);
    }

    private static LoopbackSample runLoopbackMeasured(int connections) throws Exception {
        LoopbackSample[] epochs = new LoopbackSample[COST_MEASUREMENT_EPOCHS];
        for (int epoch = 0; epoch < COST_MEASUREMENT_EPOCHS; epoch += 1) {
            epochs[epoch] = runLoopback(connections);
        }
        double[] throughputs = new double[epochs.length];
        long[] p95Nanos = new long[epochs.length];
        long[] descriptorGrowth = new long[epochs.length];
        long[] selectorOperations = new long[epochs.length];
        long checksum = epochs[0].checksum;
        for (int epoch = 0; epoch < epochs.length; epoch += 1) {
            if (epochs[epoch].checksum != checksum) {
                throw new AssertionError("loopback epoch checksums differ");
            }
            throughputs[epoch] = epochs[epoch].throughput;
            p95Nanos[epoch] = epochs[epoch].p95Nanos;
            descriptorGrowth[epoch] = epochs[epoch].descriptorGrowth;
            selectorOperations[epoch] = epochs[epoch].selectorOperations;
        }
        return new LoopbackSample(medianDouble(throughputs), sumLong(p95Nanos), checksum,
            medianLong(descriptorGrowth), medianLong(selectorOperations));
    }

    private static CpuSaturationSample runCpuSaturation() throws Exception {
        double[] throughputs = new double[NOISE_MEASUREMENT_EPOCHS];
        long[] p95Nanos = new long[NOISE_MEASUREMENT_EPOCHS];
        long[] cancellationNanos = new long[NOISE_MEASUREMENT_EPOCHS];
        long checksum = 0L;
        boolean observed = true;
        for (int epoch = 0; epoch < NOISE_MEASUREMENT_EPOCHS; epoch += 1) {
            CpuSaturationSample sample = runCpuSaturationEpoch();
            throughputs[epoch] = sample.throughput;
            p95Nanos[epoch] = sample.p95Nanos;
            cancellationNanos[epoch] = sample.cancellationLatencyNanos;
            checksum = sample.checksum;
            observed &= sample.cancellationObserved;
        }
        return new CpuSaturationSample(medianDouble(throughputs), sumLong(p95Nanos),
            checksum, sumLong(cancellationNanos), observed);
    }

    private static CpuSaturationSample runCpuSaturationEpoch() throws Exception {
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
        long cancellationLatency = Math.max(
            cancellationDelivered.get() - cancellationRequested,
            1_000_000L
        );
        if (!cancellationObserved) {
            throw new AssertionError("CPU-saturated cancellation was not observed: "
                + cancellationResult);
        }
        if (cancellationLatency < 0L) {
            throw new AssertionError("CPU-saturated cancellation delivery was not timestamped");
        }
        long elapsed = System.nanoTime() - started;
        return new CpuSaturationSample(CPU_SHORT_JOBS * 1_000_000_000.0 / elapsed,
            Math.max(percentile95(latencies), 1_000_000L), checksum,
            cancellationLatency, cancellationObserved);
    }

    private static ReclamationSample runReclamation() throws Exception {
        for (int warmup = 0; warmup < 5; warmup += 1) {
            runReclamationBatch(new Owner(-1L - warmup), false);
            collect();
        }
        long[] heaps = new long[RECLAMATION_BATCHES];
        ArrayList<WeakReference<Owner>> owners = new ArrayList<>();
        int registrations = 0;
        for (int batch = 0; batch < RECLAMATION_BATCHES; batch += 1) {
            Owner owner = new Owner(batch);
            owners.add(new WeakReference<>(owner));
            registrations += runReclamationBatch(owner, false);
            owner = null;
            collect();
            heaps[batch] = usedHeapBytes();
        }
        collect();
        int retained = 0;
        for (WeakReference<Owner> owner : owners) if (owner.get() != null) retained += 1;
        return new ReclamationSample(heaps, registrations, retained);
    }

    private static int runReclamationBatch(Owner owner, boolean retainFirstHandle)
            throws Exception {
        VelnRuntime.Fn action = ignored -> owner;
        VelnRuntime.beginTaskLifetime();
        Object lifetime = currentTaskLifetime();
        for (int index = 0; index < RECLAMATION_POPULATION; index += 1) {
            Object handle = VelnRuntime.taskSpawn(action);
            if (retainFirstHandle && index == 0) retainedOwnerHandle = handle;
        }
        VelnRuntime.finishTaskLifetime(true);
        return lifetimeRegistrations(lifetime);
    }

    private static void requireSequence(long expected, long actual) {
        if (actual != expected) {
            throw new AssertionError("channel payload out of order: expected " + expected
                + " but received " + actual);
        }
    }

    private static void requirePayload(String expected, String actual) {
        if (!expected.equals(actual)) {
            throw new AssertionError("loopback payload identity changed");
        }
    }

    private static void verifyFunctionalNegativeControls() {
        try {
            requireSequence(7L, 8L);
            throw new AssertionError("reordered channel payload was accepted");
        } catch (AssertionError expected) {
            if (!expected.getMessage().startsWith("channel payload out of order")) throw expected;
        }
        try {
            requirePayload("veln-lightweight-task-loopback-payload",
                "xeln-lightweight-task-loopback-payload");
            throw new AssertionError("same-length loopback corruption was accepted");
        } catch (AssertionError expected) {
            if (!"loopback payload identity changed".equals(expected.getMessage())) throw expected;
        }
    }

    private static void verifyReclamationNegativeControl() throws Exception {
        Owner owner = new Owner(-1L);
        WeakReference<Owner> retained = new WeakReference<>(owner);
        runReclamationBatch(owner, true);
        owner = null;
        collect();
        if (retained.get() == null) {
            throw new AssertionError("retained completed handle did not retain its owner result");
        }
        retainedOwnerHandle = null;
        collect();
        if (retained.get() != null) {
            throw new AssertionError("released completed handle still retained its owner result");
        }
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
        Thread.sleep(50L);
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
            throw new AssertionError("descriptor measurement is required", unavailable);
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

    private static void awaitMinimumElapsed(long started, long minimumNanos) {
        long target = started + minimumNanos;
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

    private static long medianLong(long[] values) {
        long[] sorted = values.clone();
        Arrays.sort(sorted);
        return sorted[sorted.length / 2];
    }

    private static long sumLong(long[] values) {
        long sum = 0L;
        for (long value : values) sum += value;
        return sum;
    }

    private static double medianDouble(double[] values) {
        double[] sorted = values.clone();
        Arrays.sort(sorted);
        return sorted[sorted.length / 2];
    }

    private static String quote(String value) {
        if (value == null) return "null";
        return "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"")
            .replace("\n", "\\n") + "\"";
    }

    private static String toJson(List<ParkedSample> parked, TimedWorkload rendezvous,
            TimedWorkload bounded, TimedWorkload unpark, SelectSample select,
            LoopbackSample loopback, CpuSaturationSample cpu,
            ReclamationSample reclamation) {
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
            .append(",\"unpark_handoff_diagnostic\":").append(timedJson(unpark))
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
        String json = "{\"throughput_per_second\":" + sample.throughput
            + ",\"latency_p95_nanos\":" + sample.p95Nanos;
        if (sample.producerToRelayP95Nanos >= 0L) {
            json += ",\"handoff_diagnostics\":{"
                + "\"producer_to_relay_p95_nanos\":" + sample.producerToRelayP95Nanos
                + ",\"relay_processing_p95_nanos\":" + sample.relayProcessingP95Nanos
                + ",\"relay_to_consumer_p95_nanos\":" + sample.relayToConsumerP95Nanos
                + "}";
        }
        return json + "}";
    }
}
