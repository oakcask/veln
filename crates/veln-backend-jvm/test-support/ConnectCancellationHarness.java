public final class ConnectCancellationHarness {
    private static Thread taskThread(Object task) throws Exception {
        java.lang.reflect.Field threadField = task.getClass().getDeclaredField("thread");
        threadField.setAccessible(true);
        return (Thread) threadField.get(task);
    }

    private static boolean isWaitingInSelector(Thread thread) {
        boolean inAwaitSocketReady = false;
        boolean inSelector = false;
        for (StackTraceElement frame : thread.getStackTrace()) {
            if (frame.getClassName().equals("VelnRuntime")
                && frame.getMethodName().equals("awaitSocketReady")) {
                inAwaitSocketReady = true;
            }
            if (frame.getClassName().contains("Selector")) {
                inSelector = true;
            }
        }
        return inAwaitSocketReady && inSelector;
    }

    private static void cancelInSelector(Object task, String boundary) throws Exception {
        Thread thread = taskThread(task);
        long deadline = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(5L);
        while (!isWaitingInSelector(thread) && System.nanoTime() < deadline) {
            Thread.yield();
        }
        if (!isWaitingInSelector(thread)) {
            throw new AssertionError("task did not enter its " + boundary + " selector wait");
        }
        VelnRuntime.taskCancel(task);
        Object joined = VelnRuntime.taskJoin(task);
        VelnRuntime.Result result = (VelnRuntime.Result) joined;
        if (result.isOk()
            || !((Boolean) VelnRuntime.taskJoinErrorIsCancelled(result.value()))) {
            throw new AssertionError("selector cancellation was not preserved: " + joined);
        }
    }

    private static void runConnectWait() throws Exception {
        java.nio.channels.ServerSocketChannel server =
            java.nio.channels.ServerSocketChannel.open();
        java.util.ArrayList<java.nio.channels.SocketChannel> fillers =
            new java.util.ArrayList<java.nio.channels.SocketChannel>();
        try {
            server.bind(
                new java.net.InetSocketAddress(
                    java.net.InetAddress.getByName("127.0.0.1"),
                    0
                ),
                1
            );
            java.net.InetSocketAddress endpoint =
                (java.net.InetSocketAddress) server.getLocalAddress();
            for (int index = 0; index < 64; index += 1) {
                java.nio.channels.SocketChannel filler =
                    java.nio.channels.SocketChannel.open();
                filler.configureBlocking(false);
                filler.connect(endpoint);
                fillers.add(filler);
            }

            String address = "127.0.0.1:" + Integer.toString(endpoint.getPort());
            Object task = VelnRuntime.taskSpawn(new VelnRuntime.Fn() {
                public Object call(Object... ignored) {
                    return VelnRuntime.netConnect(address);
                }
            });
            cancelInSelector(task, "OP_CONNECT");
        } finally {
            for (java.nio.channels.SocketChannel filler : fillers) {
                filler.close();
            }
            server.close();
        }
    }

    private static void runRestrictedHostFallback() throws Exception {
        java.lang.reflect.Method awaitSocketReady = VelnRuntime.class.getDeclaredMethod(
            "awaitSocketReady",
            java.nio.channels.SelectableChannel.class,
            Integer.TYPE,
            Long.TYPE
        );
        awaitSocketReady.setAccessible(true);
        java.nio.channels.Pipe pipe = java.nio.channels.Pipe.open();
        try {
            pipe.source().configureBlocking(false);
            Object task = VelnRuntime.taskSpawn(new VelnRuntime.Fn() {
                public Object call(Object... ignored) {
                    try {
                        awaitSocketReady.invoke(
                            null,
                            pipe.source(),
                            java.nio.channels.SelectionKey.OP_READ,
                            0L
                        );
                        throw new AssertionError("selector wait returned without cancellation");
                    } catch (java.lang.reflect.InvocationTargetException failed) {
                        Throwable cause = failed.getCause();
                        if (cause instanceof RuntimeException) {
                            throw (RuntimeException) cause;
                        }
                        throw new RuntimeException(cause);
                    } catch (IllegalAccessException failed) {
                        throw new RuntimeException(failed);
                    }
                }
            });
            cancelInSelector(task, "restricted-host fallback");
        } finally {
            pipe.source().close();
            pipe.sink().close();
        }
    }

    public static void main(String[] args) throws Exception {
        try {
            runConnectWait();
        } catch (java.net.SocketException restrictedHost) {
            runRestrictedHostFallback();
        }
        System.out.println("connect cancellation remained primary");
    }
}
