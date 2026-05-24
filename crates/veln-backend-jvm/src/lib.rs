//! Typed IR to Java source and JVM execution support.

use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{BinaryOp, PrefixOp};
use veln_ir::{
    IrCallTarget, IrExpr, IrExprKind, IrFunction, IrRecordField, IrStmt, IrStmtKind, TypedProgram,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaProgram {
    pub sources: Vec<JavaSourceFile>,
}

impl JavaProgram {
    pub fn source(&self, path: &str) -> Option<&str> {
        self.sources
            .iter()
            .find(|source| source.path == path)
            .map(|source| source.contents.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaSourceFile {
    pub path: String,
    pub contents: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaBackendOptions {
    pub program_class: String,
    pub runtime_class: String,
}

impl Default for JavaBackendOptions {
    fn default() -> Self {
        Self {
            program_class: "VelnProgram".to_string(),
            runtime_class: "VelnRuntime".to_string(),
        }
    }
}

pub fn generate_java(program: &TypedProgram) -> JavaProgram {
    generate_java_with_options(program, &JavaBackendOptions::default())
}

pub fn generate_java_with_entry(program: &TypedProgram, entry_function: &str) -> JavaProgram {
    generate_java_with_entry_options(program, entry_function, &JavaBackendOptions::default())
}

pub fn generate_java_with_options(
    program: &TypedProgram,
    options: &JavaBackendOptions,
) -> JavaProgram {
    let options = SanitizedOptions {
        program_class: java_type_identifier(&options.program_class),
        runtime_class: java_type_identifier(&options.runtime_class),
    };
    let emitter = ProgramEmitter::new(program, options);
    emitter.emit()
}

pub fn generate_java_with_entry_options(
    program: &TypedProgram,
    entry_function: &str,
    options: &JavaBackendOptions,
) -> JavaProgram {
    let options = SanitizedOptions {
        program_class: java_type_identifier(&options.program_class),
        runtime_class: java_type_identifier(&options.runtime_class),
    };
    let emitter = ProgramEmitter::new(program, options);
    emitter.emit_with_entry(entry_function)
}

#[derive(Clone, Debug)]
struct SanitizedOptions {
    program_class: String,
    runtime_class: String,
}

struct ProgramEmitter<'a> {
    program: &'a TypedProgram,
    options: SanitizedOptions,
    function_names: BTreeMap<String, String>,
}

impl<'a> ProgramEmitter<'a> {
    fn new(program: &'a TypedProgram, options: SanitizedOptions) -> Self {
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

    fn emit(&self) -> JavaProgram {
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

    fn emit_with_entry(&self, entry_function: &str) -> JavaProgram {
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

    fn function_name(&self, name: &str) -> String {
        self.function_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("fn_{}", sanitize_identifier_text(name)))
    }
}

struct FunctionEmitter<'a, 'program> {
    program: &'a ProgramEmitter<'program>,
    function: &'a IrFunction,
    locals: BTreeMap<String, String>,
    used_names: BTreeSet<String>,
    temp_counter: usize,
}

impl<'a, 'program> FunctionEmitter<'a, 'program> {
    fn new(program: &'a ProgramEmitter<'program>, function: &'a IrFunction) -> Self {
        Self {
            program,
            function,
            locals: BTreeMap::new(),
            used_names: BTreeSet::new(),
            temp_counter: 0,
        }
    }

    fn emit(&mut self, out: &mut String) {
        let function_name = self.program.function_name(&self.function.name);
        let params = self
            .function
            .params
            .iter()
            .map(|param| {
                let java_name = self.bind_local(&param.name, "p");
                format!("Object {java_name}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("    static Object {function_name}({params}) {{\n"));
        for stmt in &self.function.body {
            self.emit_stmt(out, stmt);
        }
        if !matches!(
            self.function.body.last().map(|stmt| &stmt.kind),
            Some(IrStmtKind::Return { .. })
        ) {
            out.push_str(&format!(
                "        return {}.UNIT;\n",
                self.program.options.runtime_class
            ));
        }
        out.push_str("    }\n");
    }

    fn emit_stmt(&mut self, out: &mut String, stmt: &IrStmt) {
        match &stmt.kind {
            IrStmtKind::Let { name, value, .. } => {
                let java_value = self.emit_expr(value);
                self.emit_prelude(out, &java_value.prelude);
                let java_name = self.bind_local(name, "v");
                out.push_str(&format!(
                    "        Object {java_name} = {};\n",
                    java_value.code
                ));
            }
            IrStmtKind::Expr { value } => {
                let java_value = self.emit_expr(value);
                self.emit_prelude(out, &java_value.prelude);
                out.push_str(&format!("        {};\n", java_value.code));
            }
            IrStmtKind::Return { value } => {
                let java_value = self.emit_expr(value);
                self.emit_prelude(out, &java_value.prelude);
                out.push_str(&format!("        return {};\n", java_value.code));
            }
        }
    }

    fn emit_expr(&mut self, expr: &IrExpr) -> JavaExpr {
        match &expr.kind {
            IrExprKind::Local(name) => JavaExpr::simple(self.local_name(name)),
            IrExprKind::BoolLiteral(value) => JavaExpr::simple(if *value {
                "Boolean.TRUE".to_string()
            } else {
                "Boolean.FALSE".to_string()
            }),
            IrExprKind::StringLiteral(value) => {
                JavaExpr::simple(java_string(&veln_string_literal_value(value)))
            }
            IrExprKind::IntLiteral(value) => JavaExpr::simple(format!("Long.valueOf({value}L)")),
            IrExprKind::FloatLiteral(value) => {
                JavaExpr::simple(format!("Double.valueOf({value}D)"))
            }
            IrExprKind::Unit => {
                JavaExpr::simple(format!("{}.UNIT", self.program.options.runtime_class))
            }
            IrExprKind::ResultOk(value) => self.emit_unary_runtime("ok", value),
            IrExprKind::ResultErr(value) => self.emit_unary_runtime("err", value),
            IrExprKind::OptionSome(value) => self.emit_unary_runtime("some", value),
            IrExprKind::Call { target, args } => self.emit_call(target, args),
            IrExprKind::Try(value) => self.emit_try(value),
            IrExprKind::Record(fields) => self.emit_record(fields),
            IrExprKind::List(items) => self.emit_list(items),
            IrExprKind::Prefix { op, expr } => {
                let method = match op {
                    PrefixOp::Not => "not",
                    PrefixOp::Negate => "negate",
                };
                self.emit_unary_runtime(method, expr)
            }
            IrExprKind::Binary { op, left, right } => self.emit_binary(*op, left, right),
        }
    }

    fn emit_call(&mut self, target: &IrCallTarget, args: &[IrExpr]) -> JavaExpr {
        let mut prelude = Vec::new();
        let java_args = args
            .iter()
            .map(|arg| {
                let java_arg = self.emit_expr(arg);
                prelude.extend(java_arg.prelude);
                java_arg.code
            })
            .collect::<Vec<_>>();
        let code = match target {
            IrCallTarget::Function(name) => {
                format!(
                    "{}({})",
                    self.program.function_name(name),
                    java_args.join(", ")
                )
            }
            IrCallTarget::StdioBuiltin(name) => format!(
                "{}.{}({})",
                self.program.options.runtime_class,
                stdio_method(name),
                java_args.join(", ")
            ),
            IrCallTarget::Value(name) => {
                let mut all_args = Vec::with_capacity(java_args.len() + 1);
                all_args.push(self.local_name(name));
                all_args.extend(java_args);
                format!(
                    "{}.call({})",
                    self.program.options.runtime_class,
                    all_args.join(", ")
                )
            }
        };
        JavaExpr { prelude, code }
    }

    fn emit_try(&mut self, value: &IrExpr) -> JavaExpr {
        let java_value = self.emit_expr(value);
        let temp = self.next_temp("try");
        let mut prelude = java_value.prelude;
        prelude.push(format!("Object {temp} = {};", java_value.code));
        prelude.push(format!(
            "if ({}.isErr({temp})) {{",
            self.program.options.runtime_class
        ));
        prelude.push(format!("    return {temp};"));
        prelude.push("}".to_string());
        JavaExpr {
            prelude,
            code: format!("{}.unwrapOk({temp})", self.program.options.runtime_class),
        }
    }

    fn emit_record(&mut self, fields: &[IrRecordField]) -> JavaExpr {
        let mut prelude = Vec::new();
        let mut args = Vec::new();
        for field in fields {
            let value = self.emit_expr(&field.value);
            prelude.extend(value.prelude);
            args.push(java_string(&field.name));
            args.push(value.code);
        }
        JavaExpr {
            prelude,
            code: format!(
                "{}.record({})",
                self.program.options.runtime_class,
                args.join(", ")
            ),
        }
    }

    fn emit_list(&mut self, items: &[IrExpr]) -> JavaExpr {
        let mut prelude = Vec::new();
        let args = items
            .iter()
            .map(|item| {
                let value = self.emit_expr(item);
                prelude.extend(value.prelude);
                value.code
            })
            .collect::<Vec<_>>();
        JavaExpr {
            prelude,
            code: format!(
                "{}.list({})",
                self.program.options.runtime_class,
                args.join(", ")
            ),
        }
    }

    fn emit_binary(&mut self, op: BinaryOp, left: &IrExpr, right: &IrExpr) -> JavaExpr {
        let left = self.emit_expr(left);
        let right = self.emit_expr(right);
        let mut prelude = left.prelude;
        prelude.extend(right.prelude);
        JavaExpr {
            prelude,
            code: format!(
                "{}.{}({}, {})",
                self.program.options.runtime_class,
                binary_method(op),
                left.code,
                right.code
            ),
        }
    }

    fn emit_unary_runtime(&mut self, method: &str, value: &IrExpr) -> JavaExpr {
        let value = self.emit_expr(value);
        JavaExpr {
            prelude: value.prelude,
            code: format!(
                "{}.{method}({})",
                self.program.options.runtime_class, value.code
            ),
        }
    }

    fn emit_prelude(&self, out: &mut String, prelude: &[String]) {
        for line in prelude {
            out.push_str("        ");
            out.push_str(line);
            out.push('\n');
        }
    }

    fn bind_local(&mut self, source_name: &str, prefix: &str) -> String {
        let java_name = unique_java_identifier(
            &format!("{prefix}_{}", sanitize_identifier_text(source_name)),
            &mut self.used_names,
        );
        self.locals
            .insert(source_name.to_string(), java_name.clone());
        java_name
    }

    fn local_name(&self, source_name: &str) -> String {
        self.locals
            .get(source_name)
            .cloned()
            .unwrap_or_else(|| format!("v_{}", sanitize_identifier_text(source_name)))
    }

    fn next_temp(&mut self, label: &str) -> String {
        let name = format!("__{label}{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }
}

#[derive(Clone, Debug)]
struct JavaExpr {
    prelude: Vec<String>,
    code: String,
}

impl JavaExpr {
    fn simple(code: String) -> Self {
        Self {
            prelude: Vec::new(),
            code,
        }
    }
}

fn stdio_method(name: &str) -> &'static str {
    match name {
        "stdio::print" => "stdioPrint",
        "stdio::println" => "stdioPrintln",
        "stdio::eprint" => "stdioEprint",
        "stdio::eprintln" => "stdioEprintln",
        _ => "stdioPrintln",
    }
}

fn binary_method(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::PipeGreater => "pipe",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::Equal => "equal",
        BinaryOp::NotEqual => "notEqual",
        BinaryOp::Less => "less",
        BinaryOp::LessEqual => "lessEqual",
        BinaryOp::Greater => "greater",
        BinaryOp::GreaterEqual => "greaterEqual",
        BinaryOp::Add => "add",
        BinaryOp::Subtract => "subtract",
        BinaryOp::Multiply => "multiply",
        BinaryOp::Divide => "divide",
    }
}

fn java_type_identifier(name: &str) -> String {
    let sanitized = sanitize_identifier_text(name);
    if sanitized.is_empty() || is_java_keyword(&sanitized) {
        "VelnGenerated".to_string()
    } else {
        sanitized
    }
}

fn sanitize_identifier_text(text: &str) -> String {
    let mut output = String::new();
    for (index, character) in text.chars().enumerate() {
        let valid = character == '_' || character == '$' || character.is_ascii_alphanumeric();
        if !valid {
            output.push('_');
            continue;
        }
        if index == 0 && character.is_ascii_digit() {
            output.push('_');
        }
        output.push(character);
    }
    output
}

fn unique_java_identifier(base: &str, used_names: &mut BTreeSet<String>) -> String {
    let mut candidate = if base.is_empty() || is_java_keyword(base) {
        format!("_{base}")
    } else {
        base.to_string()
    };
    if candidate == "_" {
        candidate = "_value".to_string();
    }
    let original = candidate.clone();
    let mut suffix = 1;
    while used_names.contains(&candidate) || is_java_keyword(&candidate) {
        candidate = format!("{original}_{suffix}");
        suffix += 1;
    }
    used_names.insert(candidate.clone());
    candidate
}

fn is_java_keyword(value: &str) -> bool {
    java_keywords().iter().any(|keyword| *keyword == value)
}

fn java_keywords() -> &'static [&'static str] {
    &[
        "abstract",
        "assert",
        "boolean",
        "break",
        "byte",
        "case",
        "catch",
        "char",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "double",
        "else",
        "enum",
        "extends",
        "final",
        "finally",
        "float",
        "for",
        "goto",
        "if",
        "implements",
        "import",
        "instanceof",
        "int",
        "interface",
        "long",
        "native",
        "new",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "short",
        "static",
        "strictfp",
        "super",
        "switch",
        "synchronized",
        "this",
        "throw",
        "throws",
        "transient",
        "try",
        "void",
        "volatile",
        "while",
    ]
}

fn java_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn veln_string_literal_value(raw: &str) -> String {
    let Some(inner) = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return raw.to_string();
    };
    let mut output = String::new();
    let mut chars = inner.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('"') => output.push('"'),
            Some('\\') => output.push('\\'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use veln_ast::lower_surface_ast;
    use veln_sema::lower_checked_surface_module;
    use veln_source::SourceFile;
    use veln_syntax::parse;

    static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn generates_program_and_runtime_sources_for_result_try_and_stdio() {
        let ir = lower_to_ir(concat!(
            "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main(raw: String) -> Result(Unit, AppError) effects [stdio]\n",
            "  let value: Int = parse(raw)?\n",
            "  stdio::println(\"ok\")\n",
            "  Ok(())\n",
            "end\n",
        ));

        let java = generate_java(&ir);
        let program = java
            .source("VelnProgram.java")
            .expect("program source should exist");
        let runtime = java
            .source("VelnRuntime.java")
            .expect("runtime source should exist");

        assert!(program.contains("static Object fn_parse(Object p_raw)"));
        assert!(program.contains("static Object fn_main(Object p_raw)"));
        assert!(program.contains("Object __try0 = fn_parse(p_raw);"));
        assert!(program.contains("if (VelnRuntime.isErr(__try0))"));
        assert!(program.contains("Object v_value = VelnRuntime.unwrapOk(__try0);"));
        assert!(program.contains("VelnRuntime.stdioPrintln(\"ok\");"));
        assert!(program.contains("return VelnRuntime.ok(VelnRuntime.UNIT);"));
        assert!(runtime.contains("public static final class Result"));
        assert!(runtime.contains("public static final class Option"));
        assert!(runtime.contains("public static java.util.Map<String, Object> record"));
        assert!(runtime.contains("public static java.util.List<Object> list"));
    }

    #[test]
    fn generates_runtime_values_for_records_lists_and_options() {
        let ir = lower_to_ir(concat!(
            "pub fn main() -> Result({message: String, values: List(String), maybe: Option(String)}, AppError) effects []\n",
            "  Ok({message: \"ok\", values: [\"a\", \"b\"], maybe: Some(\"x\")})\n",
            "end\n",
        ));

        let java = generate_java(&ir);
        let program = java
            .source("VelnProgram.java")
            .expect("program source should exist");

        assert!(program.contains("VelnRuntime.record("));
        assert!(program.contains("\"message\", \"ok\""));
        assert!(program.contains("\"values\", VelnRuntime.list(\"a\", \"b\")"));
        assert!(program.contains("\"maybe\", VelnRuntime.some(\"x\")"));
    }

    #[test]
    fn generates_entry_runner_for_selected_function() {
        let ir = lower_to_ir(concat!(
            "pub fn other() -> Unit effects []\n",
            "  ()\n",
            "end\n",
            "pub fn chosen() -> Result(Unit, AppError) effects []\n",
            "  Ok(())\n",
            "end\n",
        ));

        let java = generate_java_with_entry(&ir, "chosen");
        let runner = java
            .source("VelnEntry.java")
            .expect("entry source should exist");

        assert!(runner.contains("Object result = VelnProgram.fn_chosen();"));
        assert!(runner.contains("if (VelnRuntime.isErr(result))"));
        assert!(runner.contains("System.exit(1);"));
    }

    #[test]
    fn sanitizes_custom_class_names_and_entry_references() {
        let ir = lower_to_ir(concat!(
            "pub fn main() -> Result(Unit, AppError) effects []\n",
            "  Ok(())\n",
            "end\n",
        ));

        let java = generate_java_with_entry_options(
            &ir,
            "main",
            &JavaBackendOptions {
                program_class: "9 bad-name".to_string(),
                runtime_class: "class".to_string(),
            },
        );
        let program = java
            .source("_9_bad_name.java")
            .expect("sanitized program source should exist");
        let runtime = java
            .source("VelnGenerated.java")
            .expect("fallback runtime source should exist");
        let runner = java
            .source("VelnEntry.java")
            .expect("entry source should exist");

        assert!(program.contains("public final class _9_bad_name"));
        assert!(program.contains("return VelnGenerated.ok(VelnGenerated.UNIT);"));
        assert!(runtime.contains("public final class VelnGenerated"));
        assert!(runner.contains("Object result = _9_bad_name.fn_main();"));
        assert!(runner.contains("if (VelnGenerated.isErr(result))"));
    }

    #[test]
    fn sanitizes_java_keywords_and_colliding_identifiers() {
        let mut ir = lower_to_ir(concat!(
            "fn add(left: Int, right: Int) -> Int effects []\n",
            "  let total: Int = left + right\n",
            "  total\n",
            "end\n",
        ));
        let function = &mut ir.functions[0];
        function.name = "class".to_string();
        function.params[0].name = "a-b".to_string();
        function.params[1].name = "a_b".to_string();
        if let IrStmtKind::Let { name, value, .. } = &mut function.body[0].kind {
            *name = "return".to_string();
            if let IrExprKind::Binary { left, right, .. } = &mut value.kind {
                left.kind = IrExprKind::Local("a-b".to_string());
                right.kind = IrExprKind::Local("a_b".to_string());
            }
        }
        if let IrStmtKind::Return { value } = &mut function.body[1].kind {
            value.kind = IrExprKind::Local("return".to_string());
        }

        let java = generate_java(&ir);
        let program = java
            .source("VelnProgram.java")
            .expect("program source should exist");

        assert!(program.contains("static Object fn_class(Object p_a_b, Object p_a_b_1)"));
        assert!(program.contains("Object v_return = VelnRuntime.add(p_a_b, p_a_b_1);"));
        assert!(program.contains("return v_return;"));
    }

    #[test]
    fn generates_runtime_calls_for_value_call_prefix_and_binary_ops() {
        let ir = lower_to_ir(concat!(
            "pub fn main(callback: fn(Int) -> Int, a: Int, b: Int, flag: Bool) -> {",
            "called: Int, negated: Int, inverted: Bool, add: Int, sub: Int, mul: Int, div: Int, ",
            "eq: Bool, ne: Bool, lt: Bool, le: Bool, gt: Bool, ge: Bool, anded: Bool, ored: Bool, piped: Int",
            "} effects []\n",
            "  {called: callback(1), negated: -a, inverted: not flag, add: a + b, sub: a - b, ",
            "mul: a * b, div: a / b, eq: a == b, ne: a != b, lt: a < b, le: a <= b, ",
            "gt: a > b, ge: a >= b, anded: flag and false, ored: flag or true, piped: a |> b}\n",
            "end\n",
        ));

        let java = generate_java(&ir);
        let program = java
            .source("VelnProgram.java")
            .expect("program source should exist");

        assert!(program.contains("\"called\", VelnRuntime.call(p_callback, Long.valueOf(1L))"));
        assert!(program.contains("\"negated\", VelnRuntime.negate(p_a)"));
        assert!(program.contains("\"inverted\", VelnRuntime.not(p_flag)"));
        assert!(program.contains("\"add\", VelnRuntime.add(p_a, p_b)"));
        assert!(program.contains("\"sub\", VelnRuntime.subtract(p_a, p_b)"));
        assert!(program.contains("\"mul\", VelnRuntime.multiply(p_a, p_b)"));
        assert!(program.contains("\"div\", VelnRuntime.divide(p_a, p_b)"));
        assert!(program.contains("\"eq\", VelnRuntime.equal(p_a, p_b)"));
        assert!(program.contains("\"ne\", VelnRuntime.notEqual(p_a, p_b)"));
        assert!(program.contains("\"lt\", VelnRuntime.less(p_a, p_b)"));
        assert!(program.contains("\"le\", VelnRuntime.lessEqual(p_a, p_b)"));
        assert!(program.contains("\"gt\", VelnRuntime.greater(p_a, p_b)"));
        assert!(program.contains("\"ge\", VelnRuntime.greaterEqual(p_a, p_b)"));
        assert!(program.contains("\"anded\", VelnRuntime.and(p_flag, Boolean.FALSE)"));
        assert!(program.contains("\"ored\", VelnRuntime.or(p_flag, Boolean.TRUE)"));
        assert!(program.contains("\"piped\", VelnRuntime.pipe(p_a, p_b)"));
    }

    #[test]
    fn escapes_string_literals_and_emits_result_errors() {
        let ir = lower_to_ir(concat!(
            "pub fn main() -> Result(String, String) effects []\n",
            "  Err(\"line\\n\\\"quoted\\\"\\\\tail\")\n",
            "end\n",
        ));

        let java = generate_java(&ir);
        let program = java
            .source("VelnProgram.java")
            .expect("program source should exist");

        assert!(program.contains("return VelnRuntime.err(\"line\\n\\\"quoted\\\"\\\\tail\");"));
    }

    #[test]
    fn generated_sources_compile_when_javac_is_available() {
        if Command::new("javac").arg("-version").output().is_err() {
            return;
        }

        let ir = lower_to_ir(concat!(
            "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main(raw: String) -> Result(Unit, AppError) effects [stdio]\n",
            "  let value: Int = parse(raw)?\n",
            "  stdio::println(\"ok\")\n",
            "  Ok(())\n",
            "end\n",
        ));
        let java = generate_java(&ir);
        let root = temp_dir("javac");
        for source in &java.sources {
            fs::write(root.join(&source.path), &source.contents)
                .expect("java source should be written");
        }

        let output = Command::new("javac")
            .arg("VelnProgram.java")
            .arg("VelnRuntime.java")
            .current_dir(&root)
            .output()
            .expect("javac should run");
        let _ = fs::remove_dir_all(&root);

        assert!(
            output.status.success(),
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn lower_to_ir(text: &str) -> TypedProgram {
        let source = SourceFile::new("main.veln", text);
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "parse diagnostics: {:#?}",
            parsed.diagnostics
        );
        let module = lower_surface_ast(&parsed.tree);
        let lowered = lower_checked_surface_module(&module);
        assert!(
            lowered.diagnostics.is_empty(),
            "semantic diagnostics: {:#?}",
            lowered.diagnostics
        );
        lowered.ir.expect("source should lower to typed IR")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-backend-jvm-{name}-{}-{nanos}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("test directory should be created");
        root
    }
}
