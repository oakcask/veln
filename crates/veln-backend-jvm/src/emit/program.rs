use std::collections::{BTreeMap, BTreeSet};

use veln_ir::TypedProgram;

use crate::api::{EntryArgType, JavaProgram, JavaSourceFile, SanitizedOptions};
use crate::emit::function::FunctionEmitter;
use crate::java::{sanitize_identifier_text, unique_java_identifier};

pub(crate) struct ProgramEmitter<'a> {
    program: &'a TypedProgram,
    pub(crate) options: SanitizedOptions,
    function_names: BTreeMap<String, String>,
}

impl<'a> ProgramEmitter<'a> {
    pub(crate) fn new(program: &'a TypedProgram, options: SanitizedOptions) -> Self {
        let mut function_names = BTreeMap::new();
        let mut used_names = BTreeSet::new();
        for function in &program.functions {
            let name = unique_java_identifier(
                &format!("fn_{}", sanitize_identifier_text(&function.name)),
                &mut used_names,
            );
            function_names.insert(function.name.clone(), name);
        }
        Self {
            program,
            options,
            function_names,
        }
    }

    pub(crate) fn emit(&self) -> JavaProgram {
        JavaProgram {
            sources: vec![
                JavaSourceFile {
                    path: format!("{}.java", self.options.program_class),
                    contents: self.emit_program_class(),
                },
                JavaSourceFile {
                    path: format!("{}.java", self.options.runtime_class),
                    contents: self.emit_runtime_class(),
                },
            ],
        }
    }

    pub(crate) fn emit_with_entry(
        &self,
        entry_function: &str,
        entry_arg_types: &[EntryArgType],
    ) -> JavaProgram {
        let mut program = self.emit();
        program.sources.push(JavaSourceFile {
            path: "VelnEntry.java".to_string(),
            contents: self.emit_entry_class(entry_function, entry_arg_types),
        });
        program
    }

    fn emit_program_class(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "public final class {} {{\n",
            self.options.program_class
        ));
        out.push_str(&format!(
            "    private {}() {{}}\n\n",
            self.options.program_class
        ));
        for (index, function) in self.program.functions.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            let mut function_emitter = FunctionEmitter::new(self, function);
            function_emitter.emit(&mut out);
        }
        out.push_str("}\n");
        out
    }

    fn emit_entry_class(&self, entry_function: &str, entry_arg_types: &[EntryArgType]) -> String {
        let function_name = self.function_name(entry_function);
        let args = entry_arg_types
            .iter()
            .enumerate()
            .map(|(index, ty)| entry_arg_value(*ty, index))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"public final class VelnEntry {{
    private VelnEntry() {{}}

    public static void main(String[] args) {{
        Object result = {program}.{function_name}({entry_args});
        if ({runtime}.isErr(result)) {{
            System.err.println({runtime}.format(result));
            System.exit(1);
        }}
    }}

    private static Object argInt(String text, String name) {{
        try {{
            return Long.valueOf(Long.parseLong(text));
        }} catch (NumberFormatException error) {{
            System.err.println("veln: invalid Int argument `" + name + "`: `" + text + "`");
            System.exit(1);
            return null;
        }}
    }}

    private static Object argFloat(String text, String name) {{
        try {{
            return Double.valueOf(Double.parseDouble(text));
        }} catch (NumberFormatException error) {{
            System.err.println("veln: invalid Float argument `" + name + "`: `" + text + "`");
            System.exit(1);
            return null;
        }}
    }}

    private static Object argBool(String text, String name) {{
        if ("true".equals(text)) {{
            return Boolean.TRUE;
        }}
        if ("false".equals(text)) {{
            return Boolean.FALSE;
        }}
        System.err.println("veln: invalid Bool argument `" + name + "`: `" + text + "`");
        System.exit(1);
        return null;
    }}
}}
"#,
            program = self.options.program_class,
            runtime = self.options.runtime_class,
            function_name = function_name,
            entry_args = args,
        )
    }

    fn emit_runtime_class(&self) -> String {
        let class = &self.options.runtime_class;
        format!(
            r#"public final class {class} {{
    public static final Unit UNIT = new Unit();

    private {class}() {{}}

    public static final class Unit {{
        private Unit() {{}}

        @Override
        public String toString() {{
            return "()";
        }}
    }}

    public static final class Result {{
        private final boolean ok;
        private final Object value;

        private Result(boolean ok, Object value) {{
            this.ok = ok;
            this.value = value;
        }}

        public static Result ok(Object value) {{
            return new Result(true, freezeValue(value));
        }}

        public static Result err(Object value) {{
            return new Result(false, freezeValue(value));
        }}

        public boolean isOk() {{
            return ok;
        }}

        public Object value() {{
            return value;
        }}

        @Override
        public String toString() {{
            return ok ? "Ok(" + format(value) + ")" : "Err(" + format(value) + ")";
        }}
    }}

    public static final class Option {{
        private final boolean some;
        private final Object value;

        private Option(boolean some, Object value) {{
            this.some = some;
            this.value = value;
        }}

        public static Option some(Object value) {{
            return new Option(true, freezeValue(value));
        }}

        public static Option none() {{
            return new Option(false, null);
        }}

        @Override
        public String toString() {{
            return some ? "Some(" + format(value) + ")" : "None";
        }}
    }}

    public interface Fn {{
        Object call(Object... args);
    }}

    public static Result ok(Object value) {{
        return Result.ok(value);
    }}

    public static Result err(Object value) {{
        return Result.err(value);
    }}

    public static Option some(Object value) {{
        return Option.some(value);
    }}

    public static Option none() {{
        return Option.none();
    }}

    public static boolean isErr(Object value) {{
        return value instanceof Result && !((Result) value).isOk();
    }}

    public static boolean isOk(Object value) {{
        return value instanceof Result && ((Result) value).isOk();
    }}

    public static Object resultValue(Object value) {{
        return asResult(value).value();
    }}

    public static boolean isSome(Object value) {{
        return value instanceof Option && ((Option) value).some;
    }}

    public static boolean isNone(Object value) {{
        return value instanceof Option && !((Option) value).some;
    }}

    public static Object optionValue(Object value) {{
        return asOption(value).value;
    }}

    public static Object unwrapOk(Object value) {{
        if (value instanceof Result) {{
            Result result = (Result) value;
            if (result.isOk()) {{
                return result.value();
            }}
        }}
        throw new IllegalStateException("expected Ok result");
    }}

    public static java.util.Map<String, Object> record(Object... entries) {{
        java.util.LinkedHashMap<String, Object> map = new java.util.LinkedHashMap<String, Object>();
        for (int index = 0; index + 1 < entries.length; index += 2) {{
            map.put((String) entries[index], entries[index + 1]);
        }}
        return freezeMap(map);
    }}

    public static Object recordField(Object record, String field) {{
        return asMap(record).get(field);
    }}

    public static boolean recordHasField(Object record, String field) {{
        return asMap(record).containsKey(field);
    }}

    public static java.util.List<Object> list(Object... values) {{
        return freezeList(new java.util.ArrayList<Object>(java.util.Arrays.asList(values)));
    }}

    public static java.util.Map<Object, Object> dict(Object... entries) {{
        java.util.LinkedHashMap<Object, Object> map = new java.util.LinkedHashMap<Object, Object>();
        for (int i = 0; i + 1 < entries.length; i += 2) {{
            map.put(entries[i], entries[i + 1]);
        }}
        return freezeMap(map);
    }}

    public static Object listLen(Object items) {{
        return Long.valueOf(asList(items).size());
    }}

    public static Object listIsEmpty(Object items) {{
        return Boolean.valueOf(asList(items).isEmpty());
    }}

    public static Object listPush(Object items, Object value) {{
        java.util.ArrayList<Object> copy = new java.util.ArrayList<Object>(asList(items));
        copy.add(value);
        return freezeList(copy);
    }}

    public static Object listConcat(Object left, Object right) {{
        java.util.ArrayList<Object> copy = new java.util.ArrayList<Object>(asList(left));
        copy.addAll(asList(right));
        return freezeList(copy);
    }}

    public static Object listMap(Object items, Object fn) {{
        java.util.ArrayList<Object> mapped = new java.util.ArrayList<Object>();
        for (Object item : asList(items)) {{
            mapped.add(call(fn, item));
        }}
        return freezeList(mapped);
    }}

    public static Object listFilter(Object items, Object fn) {{
        java.util.ArrayList<Object> filtered = new java.util.ArrayList<Object>();
        for (Object item : asList(items)) {{
            if (asBool(call(fn, item))) {{
                filtered.add(item);
            }}
        }}
        return freezeList(filtered);
    }}

    public static Object listFold(Object items, Object initial, Object fn) {{
        Object accumulator = initial;
        for (Object item : asList(items)) {{
            accumulator = call(fn, accumulator, item);
        }}
        return accumulator;
    }}

    public static Object listTryMap(Object items, Object fn) {{
        java.util.ArrayList<Object> mapped = new java.util.ArrayList<Object>();
        for (Object item : asList(items)) {{
            Object result = call(fn, item);
            if (isErr(result)) {{
                return result;
            }}
            mapped.add(unwrapOk(result));
        }}
        return ok(freezeList(mapped));
    }}

    public static Object dictGet(Object dict, Object key) {{
        java.util.Map<Object, Object> map = asMap(dict);
        if (map.containsKey(key)) {{
            return some(map.get(key));
        }}
        return none();
    }}

    public static Object dictContains(Object dict, Object key) {{
        return Boolean.valueOf(asMap(dict).containsKey(key));
    }}

    public static Object dictInsert(Object dict, Object key, Object value) {{
        java.util.LinkedHashMap<Object, Object> copy =
            new java.util.LinkedHashMap<Object, Object>(asMap(dict));
        copy.put(key, value);
        return freezeMap(copy);
    }}

    public static Object dictRemove(Object dict, Object key) {{
        java.util.LinkedHashMap<Object, Object> copy =
            new java.util.LinkedHashMap<Object, Object>(asMap(dict));
        copy.remove(key);
        return freezeMap(copy);
    }}

    private static java.util.List<Object> freezeList(java.util.List<Object> values) {{
        java.util.ArrayList<Object> frozen = new java.util.ArrayList<Object>(values.size());
        for (Object value : values) {{
            frozen.add(freezeValue(value));
        }}
        return java.util.Collections.unmodifiableList(frozen);
    }}

    private static <K, V> java.util.Map<K, V> freezeMap(java.util.Map<K, V> values) {{
        java.util.LinkedHashMap<K, V> frozen = new java.util.LinkedHashMap<K, V>();
        for (java.util.Map.Entry<K, V> entry : values.entrySet()) {{
            @SuppressWarnings("unchecked")
            K key = (K) freezeValue(entry.getKey());
            @SuppressWarnings("unchecked")
            V value = (V) freezeValue(entry.getValue());
            frozen.put(key, value);
        }}
        return java.util.Collections.unmodifiableMap(frozen);
    }}

    @SuppressWarnings("unchecked")
    private static Object freezeValue(Object value) {{
        if (value instanceof java.util.List) {{
            return freezeList((java.util.List<Object>) value);
        }}
        if (value instanceof java.util.Map) {{
            return freezeMap((java.util.Map<Object, Object>) value);
        }}
        return value;
    }}

    public static Object optionMap(Object option, Object fn) {{
        Option value = asOption(option);
        if (!value.some) {{
            return none();
        }}
        return some(call(fn, value.value));
    }}

    public static Object optionAndThen(Object option, Object fn) {{
        Option value = asOption(option);
        if (!value.some) {{
            return none();
        }}
        return call(fn, value.value);
    }}

    public static Object optionUnwrapOr(Object option, Object fallback) {{
        Option value = asOption(option);
        return value.some ? value.value : fallback;
    }}

    public static Object resultMap(Object result, Object fn) {{
        Result value = asResult(result);
        if (!value.isOk()) {{
            return value;
        }}
        return ok(call(fn, value.value()));
    }}

    public static Object resultMapErr(Object result, Object fn) {{
        Result value = asResult(result);
        if (value.isOk()) {{
            return value;
        }}
        return err(call(fn, value.value()));
    }}

    public static Object resultAndThen(Object result, Object fn) {{
        Result value = asResult(result);
        if (!value.isOk()) {{
            return value;
        }}
        return call(fn, value.value());
    }}

    public static Object floatNegate(Object value) {{
        return Double.valueOf(-asDouble(value));
    }}

    public static Object floatAdd(Object left, Object right) {{
        return Double.valueOf(asDouble(left) + asDouble(right));
    }}

    public static Object floatSubtract(Object left, Object right) {{
        return Double.valueOf(asDouble(left) - asDouble(right));
    }}

    public static Object floatMultiply(Object left, Object right) {{
        return Double.valueOf(asDouble(left) * asDouble(right));
    }}

    public static Object floatDivide(Object left, Object right) {{
        return Double.valueOf(asDouble(left) / asDouble(right));
    }}

    public static Object floatLess(Object left, Object right) {{
        return Boolean.valueOf(asDouble(left) < asDouble(right));
    }}

    public static Object floatLessEqual(Object left, Object right) {{
        return Boolean.valueOf(asDouble(left) <= asDouble(right));
    }}

    public static Object floatGreater(Object left, Object right) {{
        return Boolean.valueOf(asDouble(left) > asDouble(right));
    }}

    public static Object floatGreaterEqual(Object left, Object right) {{
        return Boolean.valueOf(asDouble(left) >= asDouble(right));
    }}

    private static int stdioSequence = 0;

    public static Object stdioPrint(Object value) {{
        return stdioPrint(value, null, null);
    }}

    public static Object stdioPrint(Object value, String nodeId, String sourceFile) {{
        String text = format(value);
        System.out.print(text);
        recordStdioEvent("stdout", "print", "none", text, nodeId, sourceFile);
        return UNIT;
    }}

    public static Object stdioPrintln(Object value) {{
        return stdioPrintln(value, null, null);
    }}

    public static Object stdioPrintln(Object value, String nodeId, String sourceFile) {{
        String text = format(value);
        System.out.print(text);
        System.out.print(System.lineSeparator());
        recordStdioEvent("stdout", "println", "newline", text, nodeId, sourceFile);
        return UNIT;
    }}

    public static Object stdioEprint(Object value) {{
        return stdioEprint(value, null, null);
    }}

    public static Object stdioEprint(Object value, String nodeId, String sourceFile) {{
        String text = format(value);
        System.err.print(text);
        recordStdioEvent("stderr", "eprint", "none", text, nodeId, sourceFile);
        return UNIT;
    }}

    public static Object stdioEprintln(Object value) {{
        return stdioEprintln(value, null, null);
    }}

    public static Object stdioEprintln(Object value, String nodeId, String sourceFile) {{
        String text = format(value);
        System.err.print(text);
        System.err.print(System.lineSeparator());
        recordStdioEvent("stderr", "eprintln", "newline", text, nodeId, sourceFile);
        return UNIT;
    }}

    public static Object call(Object fn, Object... args) {{
        if (fn instanceof Fn) {{
            return ((Fn) fn).call(args);
        }}
        throw new IllegalStateException("value is not callable");
    }}

    public static Object not(Object value) {{
        return Boolean.valueOf(!asBool(value));
    }}

    public static Object negate(Object value) {{
        if (isFloating(value)) {{
            return floatNegate(value);
        }}
        return Long.valueOf(-asLong(value));
    }}

    public static Object add(Object left, Object right) {{
        if (isFloating(left) || isFloating(right)) {{
            return floatAdd(left, right);
        }}
        return Long.valueOf(asLong(left) + asLong(right));
    }}

    public static Object subtract(Object left, Object right) {{
        if (isFloating(left) || isFloating(right)) {{
            return floatSubtract(left, right);
        }}
        return Long.valueOf(asLong(left) - asLong(right));
    }}

    public static Object multiply(Object left, Object right) {{
        if (isFloating(left) || isFloating(right)) {{
            return floatMultiply(left, right);
        }}
        return Long.valueOf(asLong(left) * asLong(right));
    }}

    public static Object divide(Object left, Object right) {{
        if (isFloating(left) || isFloating(right)) {{
            return floatDivide(left, right);
        }}
        return Long.valueOf(asLong(left) / asLong(right));
    }}

    public static Object equal(Object left, Object right) {{
        return Boolean.valueOf(java.util.Objects.equals(left, right));
    }}

    public static Object notEqual(Object left, Object right) {{
        return Boolean.valueOf(!java.util.Objects.equals(left, right));
    }}

    public static Object less(Object left, Object right) {{
        if (isFloating(left) || isFloating(right)) {{
            return floatLess(left, right);
        }}
        return Boolean.valueOf(asLong(left) < asLong(right));
    }}

    public static Object lessEqual(Object left, Object right) {{
        if (isFloating(left) || isFloating(right)) {{
            return floatLessEqual(left, right);
        }}
        return Boolean.valueOf(asLong(left) <= asLong(right));
    }}

    public static Object greater(Object left, Object right) {{
        if (isFloating(left) || isFloating(right)) {{
            return floatGreater(left, right);
        }}
        return Boolean.valueOf(asLong(left) > asLong(right));
    }}

    public static Object greaterEqual(Object left, Object right) {{
        if (isFloating(left) || isFloating(right)) {{
            return floatGreaterEqual(left, right);
        }}
        return Boolean.valueOf(asLong(left) >= asLong(right));
    }}

    public static Object and(Object left, Object right) {{
        return Boolean.valueOf(asBool(left) && asBool(right));
    }}

    public static Object or(Object left, Object right) {{
        return Boolean.valueOf(asBool(left) || asBool(right));
    }}

    public static Object pipe(Object left, Object right) {{
        return right;
    }}

    public static String format(Object value) {{
        if (value == UNIT) {{
            return "()";
        }}
        return String.valueOf(value);
    }}

    private static void recordStdioEvent(
        String stream,
        String operation,
        String terminator,
        String text,
        String nodeId,
        String sourceFile
    ) {{
        String path = System.getenv("VELN_STDIO_EVENTS");
        if (path == null || path.isEmpty()) {{
            return;
        }}
        stdioSequence += 1;
        String line = Integer.toString(stdioSequence)
            + "\t" + stream
            + "\t" + operation
            + "\t" + terminator
            + "\t" + (nodeId == null ? "" : nodeId)
            + "\t" + (sourceFile == null ? "" : sourceFile)
            + "\t" + hex(text)
            + System.lineSeparator();
        try {{
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        }} catch (java.io.IOException error) {{
            throw new RuntimeException("failed to record stdio event", error);
        }}
    }}

    private static String hex(String text) {{
        byte[] bytes = text.getBytes(java.nio.charset.StandardCharsets.UTF_8);
        char[] digits = "0123456789abcdef".toCharArray();
        char[] encoded = new char[bytes.length * 2];
        for (int index = 0; index < bytes.length; index += 1) {{
            int value = bytes[index] & 0xff;
            encoded[index * 2] = digits[value >>> 4];
            encoded[index * 2 + 1] = digits[value & 0x0f];
        }}
        return new String(encoded);
    }}

    private static boolean asBool(Object value) {{
        return ((Boolean) value).booleanValue();
    }}

    private static long asLong(Object value) {{
        return ((Number) value).longValue();
    }}

    private static double asDouble(Object value) {{
        return ((Number) value).doubleValue();
    }}

    private static boolean isFloating(Object value) {{
        return value instanceof Double || value instanceof Float;
    }}

    @SuppressWarnings("unchecked")
    private static java.util.List<Object> asList(Object value) {{
        return (java.util.List<Object>) value;
    }}

    @SuppressWarnings("unchecked")
    private static java.util.Map<Object, Object> asMap(Object value) {{
        return (java.util.Map<Object, Object>) value;
    }}

    private static Option asOption(Object value) {{
        return (Option) value;
    }}

    private static Result asResult(Object value) {{
        return (Result) value;
    }}
}}
"#
        )
    }

    pub(crate) fn function_name(&self, name: &str) -> String {
        self.function_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("fn_{}", sanitize_identifier_text(name)))
    }
}

fn entry_arg_value(ty: EntryArgType, index: usize) -> String {
    match ty {
        EntryArgType::String => format!("args[{index}]"),
        EntryArgType::Int => format!("argInt(args[{index}], \"{index}\")"),
        EntryArgType::Float => format!("argFloat(args[{index}], \"{index}\")"),
        EntryArgType::Bool => format!("argBool(args[{index}], \"{index}\")"),
    }
}
