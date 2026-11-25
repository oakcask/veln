import java.lang.reflect.Method;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/** Separate consumer of the generated, erased provider, without Java generics. */
public final class Consumer {
    private static int checks;
    private static int callbackEffects;

    private static void equal(Object actual, Object expected) {
        if (!Objects.equals(actual, expected)) {
            throw new AssertionError("expected " + expected + ", received " + actual);
        }
        checks++;
    }

    private static void matrix(Object value) {
        equal(Provider.identity(value), value);
        Provider.Callable importedAlias = Provider::identity;
        Provider.Callable transitiveAlias = importedAlias;
        equal(transitiveAlias.call(value), value);
        equal(((Provider.Some) Provider.wrap(value)).value(), value);
        equal(Provider.apply(transitiveAlias, value), value);
        equal(Provider.repeat(value, 3L), value);
        equal(((Provider.Some) Provider.nested(value)).value(), List.of(value));
    }

    public static void main(String[] args) throws Exception {
        Provider.Callable callback = value -> value;
        Object[] values = {
            42L, 1.5D, true, "hello", Map.of("x", 42L),
            new Provider.Some(new Provider.Some("nested")), callback
        };
        for (Object value : values) matrix(value);
        for (int i = values.length - 1; i >= 0; i--) matrix(values[i]);

        // Interleave representations through one persistent alias.
        Provider.Callable alias = Provider::identity;
        equal(alias.call(42L), 42L);
        equal(alias.call("hello"), "hello");
        equal(alias.call(true), true);
        equal(((Provider.Callable) alias.call(callback)).call("callable"), "callable");
        Provider.Callable effectful = value -> { callbackEffects++; return value; };
        equal(Provider.apply(effectful, "effect"), "effect");
        equal(callbackEffects, 1);

        // Inspect actual class methods, rather than counting generated source text.
        for (String name : new String[] {"identity", "wrap", "apply", "repeat", "nested"}) {
            int count = 0;
            for (Method method : Provider.class.getDeclaredMethods()) {
                if (!method.getName().equals(name)) continue;
                count++;
                equal(method.getReturnType(), Object.class);
                equal(method.getTypeParameters().length, 0);
                for (Class<?> parameter : method.getParameterTypes()) equal(parameter, Object.class);
            }
            equal(count, 1);
        }
        System.out.println("JVM: " + checks + " assertions; 5 shared Object bodies; callback executed once");
    }
}
