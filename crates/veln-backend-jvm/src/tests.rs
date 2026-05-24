use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::*;
use veln_ast::lower_surface_ast;
use veln_ir::{IrExprKind, IrStmtKind, TypedProgram};
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
        "pub fn main(raw: String) -> Result((), AppError) effects [stdio]\n",
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
    assert!(program.contains("VelnRuntime.stdioPrintln(\"ok\", \"call-"));
    assert!(program.contains("return VelnRuntime.ok(VelnRuntime.UNIT);"));
    assert!(runtime.contains("public static final class Result"));
    assert!(runtime.contains("public static final class Option"));
    assert!(runtime.contains("public static java.util.Map<String, Object> record"));
    assert!(runtime.contains("public static java.util.List<Object> list"));
}

#[test]
fn generates_runtime_values_for_records_lists_and_options() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Result({message: String, values: List(String), maybe: Option(String), empty: Option(String)}, AppError) effects []\n",
        "  Ok({message: \"ok\", values: [\"a\", \"b\"], maybe: Some(\"x\"), empty: None})\n",
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
    assert!(program.contains("\"empty\", VelnRuntime.none()"));
}

#[test]
fn generates_runtime_lookup_for_record_field_access() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Int effects []\n",
        "  {count: 1}.count\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains(
        "return VelnRuntime.recordField(VelnRuntime.record(\"count\", Long.valueOf(1L)), \"count\");"
    ));
    assert!(runtime.contains("public static Object recordField(Object record, String field)"));
}

#[test]
fn generates_entry_runner_for_selected_function() {
    let ir = lower_to_ir(concat!(
        "pub fn other() -> () effects []\n",
        "  ()\n",
        "end\n",
        "pub fn chosen() -> Result((), AppError) effects []\n",
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
        "pub fn main() -> Result((), AppError) effects []\n",
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
fn generates_runtime_support_for_float_numeric_ops() {
    let ir = lower_to_ir(concat!(
        "pub fn main(left: Float, right: Float) -> {negated: Float, sum: Float, ordered: Bool} effects []\n",
        "  {negated: -left, sum: left + right, ordered: left <= right}\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("\"negated\", VelnRuntime.floatNegate(p_left)"));
    assert!(program.contains("\"sum\", VelnRuntime.floatAdd(p_left, p_right)"));
    assert!(program.contains("\"ordered\", VelnRuntime.floatLessEqual(p_left, p_right)"));
    assert!(runtime.contains("public static Object floatAdd(Object left, Object right)"));
    assert!(runtime.contains("public static Object floatLessEqual(Object left, Object right)"));
    assert!(!runtime.contains("requireFloatOperands"));
    assert!(runtime.contains("return Double.valueOf(asDouble(left) + asDouble(right));"));
    assert!(runtime.contains("return Boolean.valueOf(asDouble(left) <= asDouble(right));"));
}

#[test]
fn generated_float_comparison_allows_nan_operands() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Bool effects []\n",
        "  0.0 / 0.0 < 1.0\n",
        "end\n",
    ));
    let java = generate_java_with_entry(&ir, "main");
    let root = temp_dir("float-comparison-nan");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("VelnEntry.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(java.status.success());
}

#[test]
fn generated_float_arithmetic_allows_nan_operands() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Float effects []\n",
        "  0.0 / 0.0 + 1.0\n",
        "end\n",
    ));
    let java = generate_java_with_entry(&ir, "main");
    let root = temp_dir("float-arithmetic-nan");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("VelnEntry.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(java.status.success());
}

#[test]
fn generates_runtime_calls_for_prelude_helpers() {
    let ir = lower_to_ir(concat!(
        "pub fn main(items: List(Int), table: Dict(String, Int), mapper: fn(Int) -> String) -> {",
        "count: Int, pushed: List(Int), mapped: List(String), found: Option(Int), inserted: Dict(String, Int)",
        "} effects []\n",
        "  {count: list_len(items), pushed: list_push(items, 1), mapped: list_map(items, mapper), ",
        "found: dict_get(table, \"a\"), inserted: dict_insert(table, \"b\", 2)}\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("\"count\", VelnRuntime.listLen(p_items)"));
    assert!(program.contains("\"pushed\", VelnRuntime.listPush(p_items, Long.valueOf(1L))"));
    assert!(program.contains("\"mapped\", VelnRuntime.listMap(p_items, p_mapper)"));
    assert!(program.contains("\"found\", VelnRuntime.dictGet(p_table, \"a\")"));
    assert!(
        program.contains("\"inserted\", VelnRuntime.dictInsert(p_table, \"b\", Long.valueOf(2L))")
    );
    assert!(runtime.contains("public static Object listTryMap"));
    assert!(runtime.contains("public static Object resultAndThen"));
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
        "pub fn main(raw: String) -> Result((), AppError) effects [stdio]\n",
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
