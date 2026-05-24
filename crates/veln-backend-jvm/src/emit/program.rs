use std::collections::{BTreeMap, BTreeSet};

use veln_ir::TypedProgram;

use crate::api::{JavaProgram, JavaSourceFile, SanitizedOptions};
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

    pub(crate) fn emit_with_entry(&self, entry_function: &str) -> JavaProgram {
        let mut program = self.emit();
        program.sources.push(JavaSourceFile {
            path: "VelnEntry.java".to_string(),
            contents: self.emit_entry_class(entry_function),
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

    fn emit_entry_class(&self, entry_function: &str) -> String {
        let function_name = self.function_name(entry_function);
        format!(
            r#"public final class VelnEntry {{
    private VelnEntry() {{}}

    public static void main(String[] args) {{
        Object result = {program}.{function_name}();
        if ({runtime}.isErr(result)) {{
            System.err.println({runtime}.format(result));
            System.exit(1);
        }}
    }}
}}
"#,
            program = self.options.program_class,
            runtime = self.options.runtime_class,
            function_name = function_name,
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
            return new Result(true, value);
        }}

        public static Result err(Object value) {{
            return new Result(false, value);
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
            return new Option(true, value);
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
        return java.util.Collections.unmodifiableMap(map);
    }}

    public static java.util.List<Object> list(Object... values) {{
        return java.util.Collections.unmodifiableList(
            new java.util.ArrayList<Object>(java.util.Arrays.asList(values))
        );
    }}

    public static Object stdioPrint(Object value) {{
        System.out.print(format(value));
        return UNIT;
    }}

    public static Object stdioPrintln(Object value) {{
        System.out.println(format(value));
        return UNIT;
    }}

    public static Object stdioEprint(Object value) {{
        System.err.print(format(value));
        return UNIT;
    }}

    public static Object stdioEprintln(Object value) {{
        System.err.println(format(value));
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
        return Long.valueOf(-asLong(value));
    }}

    public static Object add(Object left, Object right) {{
        return Long.valueOf(asLong(left) + asLong(right));
    }}

    public static Object subtract(Object left, Object right) {{
        return Long.valueOf(asLong(left) - asLong(right));
    }}

    public static Object multiply(Object left, Object right) {{
        return Long.valueOf(asLong(left) * asLong(right));
    }}

    public static Object divide(Object left, Object right) {{
        return Long.valueOf(asLong(left) / asLong(right));
    }}

    public static Object equal(Object left, Object right) {{
        return Boolean.valueOf(java.util.Objects.equals(left, right));
    }}

    public static Object notEqual(Object left, Object right) {{
        return Boolean.valueOf(!java.util.Objects.equals(left, right));
    }}

    public static Object less(Object left, Object right) {{
        return Boolean.valueOf(asLong(left) < asLong(right));
    }}

    public static Object lessEqual(Object left, Object right) {{
        return Boolean.valueOf(asLong(left) <= asLong(right));
    }}

    public static Object greater(Object left, Object right) {{
        return Boolean.valueOf(asLong(left) > asLong(right));
    }}

    public static Object greaterEqual(Object left, Object right) {{
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

    private static boolean asBool(Object value) {{
        return ((Boolean) value).booleanValue();
    }}

    private static long asLong(Object value) {{
        return ((Number) value).longValue();
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
