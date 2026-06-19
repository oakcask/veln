    private static final Object SELECT_PENDING = new Object();
    private static final Object SELECT_CLOSED = new Object();
    private static final java.util.concurrent.atomic.AtomicInteger SELECT_CURSOR =
        new java.util.concurrent.atomic.AtomicInteger(0);

    public static final class Channel {
        private final java.util.ArrayDeque<Object> queue;
        private final long capacity;
        private long waitingReceivers;
        private boolean hasRendezvousValue;
        private Object rendezvousValue;
        private boolean closed;

        private Channel(long capacity) {
            if (capacity < 0L || capacity > Integer.MAX_VALUE) {
                throw new IllegalArgumentException("channel capacity is out of range");
            }
            this.queue = new java.util.ArrayDeque<Object>();
            this.capacity = capacity;
            this.waitingReceivers = 0L;
            this.hasRendezvousValue = false;
            this.rendezvousValue = null;
            this.closed = false;
        }
    }

    public static final class Sender {
        private final Channel channel;

        private Sender(Channel channel) {
            this.channel = channel;
        }
    }

    public static final class Receiver {
        private final Channel channel;

        private Receiver(Channel channel) {
            this.channel = channel;
        }
    }

    public static final class Task {
        private final java.util.concurrent.FutureTask<Object> future;
        private final Thread thread;

        private Task(Fn fn) {
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object arg) {
            Object frozenArg = freezeBoundaryValue(arg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            Object frozenTwelfthArg = freezeBoundaryValue(twelfthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg, frozenTwelfthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            Object frozenTwelfthArg = freezeBoundaryValue(twelfthArg);
            Object frozenThirteenthArg = freezeBoundaryValue(thirteenthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg, frozenTwelfthArg, frozenThirteenthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            Object frozenTwelfthArg = freezeBoundaryValue(twelfthArg);
            Object frozenThirteenthArg = freezeBoundaryValue(thirteenthArg);
            Object frozenFourteenthArg = freezeBoundaryValue(fourteenthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg, frozenTwelfthArg, frozenThirteenthArg, frozenFourteenthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            Object frozenTwelfthArg = freezeBoundaryValue(twelfthArg);
            Object frozenThirteenthArg = freezeBoundaryValue(thirteenthArg);
            Object frozenFourteenthArg = freezeBoundaryValue(fourteenthArg);
            Object frozenFifteenthArg = freezeBoundaryValue(fifteenthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg, frozenTwelfthArg, frozenThirteenthArg, frozenFourteenthArg, frozenFifteenthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            Object frozenTwelfthArg = freezeBoundaryValue(twelfthArg);
            Object frozenThirteenthArg = freezeBoundaryValue(thirteenthArg);
            Object frozenFourteenthArg = freezeBoundaryValue(fourteenthArg);
            Object frozenFifteenthArg = freezeBoundaryValue(fifteenthArg);
            Object frozenSixteenthArg = freezeBoundaryValue(sixteenthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg, frozenTwelfthArg, frozenThirteenthArg, frozenFourteenthArg, frozenFifteenthArg, frozenSixteenthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg, Object seventeenthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            Object frozenTwelfthArg = freezeBoundaryValue(twelfthArg);
            Object frozenThirteenthArg = freezeBoundaryValue(thirteenthArg);
            Object frozenFourteenthArg = freezeBoundaryValue(fourteenthArg);
            Object frozenFifteenthArg = freezeBoundaryValue(fifteenthArg);
            Object frozenSixteenthArg = freezeBoundaryValue(sixteenthArg);
            Object frozenSeventeenthArg = freezeBoundaryValue(seventeenthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg, frozenTwelfthArg, frozenThirteenthArg, frozenFourteenthArg, frozenFifteenthArg, frozenSixteenthArg, frozenSeventeenthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg, Object seventeenthArg, Object eighteenthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            Object frozenTwelfthArg = freezeBoundaryValue(twelfthArg);
            Object frozenThirteenthArg = freezeBoundaryValue(thirteenthArg);
            Object frozenFourteenthArg = freezeBoundaryValue(fourteenthArg);
            Object frozenFifteenthArg = freezeBoundaryValue(fifteenthArg);
            Object frozenSixteenthArg = freezeBoundaryValue(sixteenthArg);
            Object frozenSeventeenthArg = freezeBoundaryValue(seventeenthArg);
            Object frozenEighteenthArg = freezeBoundaryValue(eighteenthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg, frozenTwelfthArg, frozenThirteenthArg, frozenFourteenthArg, frozenFifteenthArg, frozenSixteenthArg, frozenSeventeenthArg, frozenEighteenthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg, Object seventeenthArg, Object eighteenthArg, Object nineteenthArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            Object frozenTwelfthArg = freezeBoundaryValue(twelfthArg);
            Object frozenThirteenthArg = freezeBoundaryValue(thirteenthArg);
            Object frozenFourteenthArg = freezeBoundaryValue(fourteenthArg);
            Object frozenFifteenthArg = freezeBoundaryValue(fifteenthArg);
            Object frozenSixteenthArg = freezeBoundaryValue(sixteenthArg);
            Object frozenSeventeenthArg = freezeBoundaryValue(seventeenthArg);
            Object frozenEighteenthArg = freezeBoundaryValue(eighteenthArg);
            Object frozenNineteenthArg = freezeBoundaryValue(nineteenthArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg, frozenTwelfthArg, frozenThirteenthArg, frozenFourteenthArg, frozenFifteenthArg, frozenSixteenthArg, frozenSeventeenthArg, frozenEighteenthArg, frozenNineteenthArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }

        private Task(Fn fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg, Object seventeenthArg, Object eighteenthArg, Object nineteenthArg, Object twentiethArg) {
            Object frozenFirstArg = freezeBoundaryValue(firstArg);
            Object frozenSecondArg = freezeBoundaryValue(secondArg);
            Object frozenThirdArg = freezeBoundaryValue(thirdArg);
            Object frozenFourthArg = freezeBoundaryValue(fourthArg);
            Object frozenFifthArg = freezeBoundaryValue(fifthArg);
            Object frozenSixthArg = freezeBoundaryValue(sixthArg);
            Object frozenSeventhArg = freezeBoundaryValue(seventhArg);
            Object frozenEighthArg = freezeBoundaryValue(eighthArg);
            Object frozenNinthArg = freezeBoundaryValue(ninthArg);
            Object frozenTenthArg = freezeBoundaryValue(tenthArg);
            Object frozenEleventhArg = freezeBoundaryValue(eleventhArg);
            Object frozenTwelfthArg = freezeBoundaryValue(twelfthArg);
            Object frozenThirteenthArg = freezeBoundaryValue(thirteenthArg);
            Object frozenFourteenthArg = freezeBoundaryValue(fourteenthArg);
            Object frozenFifteenthArg = freezeBoundaryValue(fifteenthArg);
            Object frozenSixteenthArg = freezeBoundaryValue(sixteenthArg);
            Object frozenSeventeenthArg = freezeBoundaryValue(seventeenthArg);
            Object frozenEighteenthArg = freezeBoundaryValue(eighteenthArg);
            Object frozenNineteenthArg = freezeBoundaryValue(nineteenthArg);
            Object frozenTwentiethArg = freezeBoundaryValue(twentiethArg);
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeBoundaryValue(call(fn, frozenFirstArg, frozenSecondArg, frozenThirdArg, frozenFourthArg, frozenFifthArg, frozenSixthArg, frozenSeventhArg, frozenEighthArg, frozenNinthArg, frozenTenthArg, frozenEleventhArg, frozenTwelfthArg, frozenThirteenthArg, frozenFourteenthArg, frozenFifteenthArg, frozenSixteenthArg, frozenSeventeenthArg, frozenEighteenthArg, frozenNineteenthArg, frozenTwentiethArg))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }
    }
    public static Object channelBounded(Object capacity) {
        Channel channel = new Channel(asLong(capacity));
        return record("tx", new Sender(channel), "rx", new Receiver(channel));
    }

    public static Object channelClone(Object sender) {
        Sender tx = (Sender) sender;
        return new Sender(tx.channel);
    }

    public static Object channelSend(Object sender, Object value) {
        Sender tx = (Sender) sender;
        synchronized (tx.channel) {
            if (tx.channel.closed) {
                return err("closed");
            }
            if (tx.channel.capacity == 0L) {
                Object frozen = freezeBoundaryValue(value);
                while (!tx.channel.closed
                    && (tx.channel.waitingReceivers == 0L || tx.channel.hasRendezvousValue)) {
                    try {
                        tx.channel.wait();
                    } catch (InterruptedException interrupted) {
                        Thread.currentThread().interrupt();
                        return err("interrupted");
                    }
                }
                if (tx.channel.closed) {
                    return err("closed");
                }
                tx.channel.rendezvousValue = frozen;
                tx.channel.hasRendezvousValue = true;
                tx.channel.notifyAll();
                boolean interruptedAfterTransfer = false;
                while (tx.channel.hasRendezvousValue) {
                    try {
                        tx.channel.wait();
                    } catch (InterruptedException interrupted) {
                        interruptedAfterTransfer = true;
                    }
                }
                if (interruptedAfterTransfer) {
                    Thread.currentThread().interrupt();
                }
                return ok(UNIT);
            }
            while (!tx.channel.closed && tx.channel.queue.size() >= tx.channel.capacity) {
                try {
                    tx.channel.wait();
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    return err("interrupted");
                }
            }
            if (tx.channel.closed) {
                return err("closed");
            }
            tx.channel.queue.addLast(freezeBoundaryValue(value));
            tx.channel.notifyAll();
            return ok(UNIT);
        }
    }

    public static Object channelRecv(Object receiver) {
        Receiver rx = (Receiver) receiver;
        synchronized (rx.channel) {
            if (rx.channel.capacity == 0L) {
                rx.channel.waitingReceivers += 1L;
                rx.channel.notifyAll();
                boolean interruptedAfterTransfer = false;
                try {
                    while (!rx.channel.hasRendezvousValue && !rx.channel.closed) {
                        try {
                            rx.channel.wait();
                        } catch (InterruptedException interrupted) {
                            if (!rx.channel.hasRendezvousValue) {
                                Thread.currentThread().interrupt();
                                return none();
                            }
                            interruptedAfterTransfer = true;
                        }
                    }
                    if (!rx.channel.hasRendezvousValue) {
                        return none();
                    }
                    Object value = rx.channel.rendezvousValue;
                    rx.channel.rendezvousValue = null;
                    rx.channel.hasRendezvousValue = false;
                    rx.channel.notifyAll();
                    if (interruptedAfterTransfer) {
                        Thread.currentThread().interrupt();
                    }
                    return some(value);
                } finally {
                    rx.channel.waitingReceivers -= 1L;
                    rx.channel.notifyAll();
                }
            }
            while (rx.channel.queue.isEmpty() && !rx.channel.closed) {
                try {
                    rx.channel.wait();
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    return none();
                }
            }
            Object value = rx.channel.queue.pollFirst();
            if (value == null) {
                return none();
            }
            rx.channel.notifyAll();
            return some(value);
        }
    }

    public static Object channelSelect(Object leftReceiver, Object rightReceiver) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, -1L, true, false);
    }

    public static Object channelSelectPriority(Object leftReceiver, Object rightReceiver) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, -1L, false, false);
    }

    public static Object channelSelectManyPriority(Object receiverList) {
        Receiver[] receivers = channelReceiverArray(receiverList);
        return channelSelectReceiversWithTimeout(receivers, -1L, null, false, false);
    }

    public static Object channelSelectManyTimeout(Object receiverList, Object timeoutMillis) {
        Receiver[] receivers = channelReceiverArray(receiverList);
        return channelSelectReceiversWithTimeout(receivers, asLong(timeoutMillis), null, false, false);
    }

    public static Object channelSelectManyTimeoutResult(Object receiverList, Object timeoutMillis) {
        Receiver[] receivers = channelReceiverArray(receiverList);
        return channelSelectReceiversWithTimeout(receivers, asLong(timeoutMillis), null, false, true);
    }

    public static Object channelSelectManyTimeoutCancellable(
        Object receiverList,
        Object timeoutMillis,
        Object token
    ) {
        Receiver[] receivers = channelReceiverArray(receiverList);
        return channelSelectReceiversWithTimeout(
            receivers,
            asLong(timeoutMillis),
            asCancelToken(token),
            false,
            true
        );
    }

    public static Object channelSelectTimeout(
        Object leftReceiver,
        Object rightReceiver,
        Object timeoutMillis
    ) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, asLong(timeoutMillis), true, false);
    }

    public static Object channelSelectResult(Object leftReceiver, Object rightReceiver) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, -1L, true, true);
    }

    public static Object channelSelectPriorityResult(Object leftReceiver, Object rightReceiver) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, -1L, false, true);
    }

    public static Object channelSelectTimeoutResult(
        Object leftReceiver,
        Object rightReceiver,
        Object timeoutMillis
    ) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, asLong(timeoutMillis), true, true);
    }

    private static Object channelSelectWithTimeout(
        Object leftReceiver,
        Object rightReceiver,
        long timeoutMillis,
        boolean rotateTies,
        boolean reportInterrupt
    ) {
        Receiver left = (Receiver) leftReceiver;
        Receiver right = (Receiver) rightReceiver;
        Receiver[] receivers = new Receiver[] { left, right };
        return channelSelectReceiversWithTimeout(
            receivers,
            timeoutMillis,
            null,
            rotateTies,
            reportInterrupt
        );
    }

    private static Object channelSelectReceiversWithTimeout(
        Receiver[] receivers,
        long timeoutMillis,
        CancelToken cancelToken,
        boolean rotateTies,
        boolean reportInterrupt
    ) {
        if (receivers.length == 0) {
            throw new RuntimeFailure("channel select_many_priority failed: receiver list is empty");
        }
        boolean[] registered = new boolean[receivers.length];
        long startNanos = timeoutMillis >= 0L ? System.nanoTime() : 0L;
        long timeoutNanos = timeoutMillis >= 0L
            ? (timeoutMillis > Long.MAX_VALUE / 1000000L
                ? Long.MAX_VALUE
                : timeoutMillis * 1000000L)
            : -1L;
        try {
            for (int i = 0; i < receivers.length; i += 1) {
                Channel channel = receivers[i].channel;
                if (channel.capacity == 0L) {
                    synchronized (channel) {
                        channel.waitingReceivers += 1L;
                        registered[i] = true;
                        channel.notifyAll();
                    }
                }
            }
            int start = rotateTies
                ? Math.floorMod(SELECT_CURSOR.getAndIncrement(), receivers.length)
                : 0;
            while (true) {
                if (cancelToken != null && waitCancelled(cancelToken)) {
                    return err("cancelled");
                }
                boolean allClosed = true;
                for (int offset = 0; offset < receivers.length; offset += 1) {
                    int i = (start + offset) % receivers.length;
                    Object selected = channelSelectPoll(receivers[i], i);
                    if (selected == SELECT_PENDING) {
                        allClosed = false;
                    } else if (selected != SELECT_CLOSED) {
                        return channelSelectReturn(selected, reportInterrupt);
                    }
                }
                if (allClosed) {
                    return channelSelectReturn(none(), reportInterrupt);
                }
                if (timeoutNanos >= 0L && System.nanoTime() - startNanos >= timeoutNanos) {
                    return channelSelectReturn(none(), reportInterrupt);
                }
                try {
                    Thread.sleep(1L);
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    return reportInterrupt ? err("interrupted") : none();
                }
            }
        } finally {
            for (int i = 0; i < receivers.length; i += 1) {
                if (registered[i]) {
                    Channel channel = receivers[i].channel;
                    synchronized (channel) {
                        channel.waitingReceivers -= 1L;
                        channel.notifyAll();
                    }
                }
            }
        }
    }

    private static Receiver[] channelReceiverArray(Object receiverList) {
        java.util.ArrayList<Receiver> receivers = new java.util.ArrayList<Receiver>();
        ListValue current = asListValue(receiverList);
        while (current.cons) {
            receivers.add((Receiver) current.head);
            current = asListValue(current.tail);
        }
        return receivers.toArray(new Receiver[receivers.size()]);
    }

    private static Object channelSelectReturn(Object selected, boolean reportInterrupt) {
        return reportInterrupt ? ok(selected) : selected;
    }

    private static Object channelSelectPoll(Receiver rx, int index) {
        synchronized (rx.channel) {
            Object value;
            if (rx.channel.capacity == 0L) {
                if (rx.channel.hasRendezvousValue) {
                    value = rx.channel.rendezvousValue;
                    rx.channel.rendezvousValue = null;
                    rx.channel.hasRendezvousValue = false;
                    rx.channel.notifyAll();
                    return some(record("index", Long.valueOf(index), "value", value));
                }
                return rx.channel.closed ? SELECT_CLOSED : SELECT_PENDING;
            }
            value = rx.channel.queue.pollFirst();
            if (value != null) {
                rx.channel.notifyAll();
                return some(record("index", Long.valueOf(index), "value", value));
            }
            return rx.channel.closed ? SELECT_CLOSED : SELECT_PENDING;
        }
    }

    public static Object channelClose(Object sender) {
        Sender tx = (Sender) sender;
        synchronized (tx.channel) {
            tx.channel.closed = true;
            tx.channel.notifyAll();
        }
        return UNIT;
    }

    public static Object taskSpawn(Object fn) {
        return new Task((Fn) fn);
    }

    public static Object taskSpawnWith(Object fn, Object arg) {
        return new Task((Fn) fn, arg);
    }

    public static Object taskSpawnWith2(Object fn, Object firstArg, Object secondArg) {
        return new Task((Fn) fn, firstArg, secondArg);
    }

    public static Object taskSpawnWith3(Object fn, Object firstArg, Object secondArg, Object thirdArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg);
    }

    public static Object taskSpawnWith4(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg);
    }

    public static Object taskSpawnWith5(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg);
    }

    public static Object taskSpawnWith6(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg);
    }

    public static Object taskSpawnWith7(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg);
    }

    public static Object taskSpawnWith8(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg);
    }

    public static Object taskSpawnWith9(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg);
    }

    public static Object taskSpawnWith10(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg);
    }

    public static Object taskSpawnWith11(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg);
    }

    public static Object taskSpawnWith12(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg, twelfthArg);
    }

    public static Object taskSpawnWith13(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg, twelfthArg, thirteenthArg);
    }

    public static Object taskSpawnWith14(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg, twelfthArg, thirteenthArg, fourteenthArg);
    }

    public static Object taskSpawnWith15(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg, twelfthArg, thirteenthArg, fourteenthArg, fifteenthArg);
    }

    public static Object taskSpawnWith16(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg, twelfthArg, thirteenthArg, fourteenthArg, fifteenthArg, sixteenthArg);
    }

    public static Object taskSpawnWith17(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg, Object seventeenthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg, twelfthArg, thirteenthArg, fourteenthArg, fifteenthArg, sixteenthArg, seventeenthArg);
    }

    public static Object taskSpawnWith18(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg, Object seventeenthArg, Object eighteenthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg, twelfthArg, thirteenthArg, fourteenthArg, fifteenthArg, sixteenthArg, seventeenthArg, eighteenthArg);
    }

    public static Object taskSpawnWith19(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg, Object seventeenthArg, Object eighteenthArg, Object nineteenthArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg, twelfthArg, thirteenthArg, fourteenthArg, fifteenthArg, sixteenthArg, seventeenthArg, eighteenthArg, nineteenthArg);
    }

    public static Object taskSpawnWith20(Object fn, Object firstArg, Object secondArg, Object thirdArg, Object fourthArg, Object fifthArg, Object sixthArg, Object seventhArg, Object eighthArg, Object ninthArg, Object tenthArg, Object eleventhArg, Object twelfthArg, Object thirteenthArg, Object fourteenthArg, Object fifteenthArg, Object sixteenthArg, Object seventeenthArg, Object eighteenthArg, Object nineteenthArg, Object twentiethArg) {
        return new Task((Fn) fn, firstArg, secondArg, thirdArg, fourthArg, fifthArg, sixthArg, seventhArg, eighthArg, ninthArg, tenthArg, eleventhArg, twelfthArg, thirteenthArg, fourteenthArg, fifteenthArg, sixteenthArg, seventeenthArg, eighteenthArg, nineteenthArg, twentiethArg);
    }

    public static Object taskJoin(Object task) {
        Task handle = (Task) task;
        try {
            return ok(handle.future.get());
        } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
            return err("interrupted");
        } catch (java.util.concurrent.ExecutionException failed) {
            return err("failed");
        } catch (java.util.concurrent.CancellationException cancelled) {
            return err("cancelled");
        }
    }

    public static Object taskCancel(Object task) {
        Task handle = (Task) task;
        handle.future.cancel(true);
        handle.thread.interrupt();
        return UNIT;
    }
